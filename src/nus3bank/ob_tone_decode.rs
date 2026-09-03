//! Probe-only port of OB `nus3_decode_tone_descriptor` (`0x1411CDB10`).
//!
//! Walks the packed TONE record the way the EXE does during
//! `CSndBankData_FinalizeLoad`. Fill stores are skipped; the cursor must stay
//! inside `raw_meta`.

use super::error::Nus3bankError;

const HASH_UNK1: usize = 8;
const MAX_MASK_WORDS: usize = 16;
const MAX_LIST: u32 = 1_000_000;

#[derive(Debug)]
struct Walk<'a> {
    data: &'a [u8],
    pos: usize,
    allow_short_overrun: bool,
}

impl<'a> Walk<'a> {
    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn need(&self, n: usize, what: &str) -> Result<(), Nus3bankError> {
        if n > 0x1_0000 {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!(
                    "TONE descriptor skip 0x{n:x} is implausible ({what} at 0x{:x})",
                    self.pos
                ),
            });
        }
        if self.remaining() >= n {
            return Ok(());
        }
        // SE banks: the EXE may walk a few bytes into the next TONE record.
        // BGM banks must stay inside the record (startup FinalizeLoad AV).
        let missing = n - self.remaining();
        if self.allow_short_overrun && missing <= 0x200 {
            return Ok(());
        }
        Err(Nus3bankError::InvalidFormat {
            reason: format!(
                "TONE descriptor overran packed payload at 0x{:x} ({}, need {n}, have {})",
                self.pos,
                what,
                self.remaining()
            ),
        })
    }

    fn u32(&mut self, what: &str) -> Result<u32, Nus3bankError> {
        self.need(4, what)?;
        if self.remaining() < 4 {
            self.pos = self.data.len();
            return Ok(0);
        }
        let v = u32::from_le_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        Ok(v)
    }

    fn skip(&mut self, n: usize, what: &str) -> Result<(), Nus3bankError> {
        self.need(n, what)?;
        if self.remaining() < n {
            self.pos = self.data.len();
            return Ok(());
        }
        self.pos += n;
        Ok(())
    }
}

fn i32_at(data: &[u8], off: usize) -> Option<i32> {
    data.get(off..off + 4)
        .and_then(|b| b.try_into().ok())
        .map(i32::from_le_bytes)
}

fn u32_at(data: &[u8], off: usize) -> Option<u32> {
    data.get(off..off + 4)
        .and_then(|b| b.try_into().ok())
        .map(u32::from_le_bytes)
}

fn bit(word: u32, b: u32) -> bool {
    (word >> b) & 1 != 0
}

/// True when the record uses OB continuation-mask words after `hash`/`unk1`.
pub fn looks_like_ob_descriptor(raw_meta: &[u8]) -> bool {
    raw_meta.len() >= 16 && i32_at(raw_meta, 12).is_some_and(|w| w < 0)
}

/// BGM_AC27_UPDATE_* live cues: flags0 at +4 (negative), flags1 at +8 (terminator),
/// name at +12. Shrinking that name slot AVs `nus3_decode_tone_descriptor` at load.
pub fn bgm_name_len_pos(raw_meta: &[u8]) -> Option<usize> {
    if raw_meta.len() < 40 || looks_like_ob_descriptor(raw_meta) {
        return None;
    }
    for name_at in [12usize, 8] {
        let flags0_at = name_at.checked_sub(8)?;
        let flags1_at = name_at.checked_sub(4)?;
        let Some(flags0) = i32_at(raw_meta, flags0_at) else {
            continue;
        };
        let Some(flags1) = i32_at(raw_meta, flags1_at) else {
            continue;
        };
        let name_len = raw_meta[name_at];
        if flags0 as u32 == 0x8427_FFFF && flags1 > 0 && (3..=80).contains(&name_len) {
            return Some(name_at);
        }
    }
    None
}

pub fn is_bgm_tone_descriptor(raw_meta: &[u8]) -> bool {
    bgm_name_len_pos(raw_meta).is_some()
}

/// Vanilla OB live record with a leading 0, negative flags0, flags1, then a
/// length-prefixed name at +12. BGM uses `flags0 == 0x8427FFFF`; SE banks also
/// use `0x8467FFFF` / `0x8627FFFF` / `0x8427BFFF` with the same name placement.
///
/// Do not treat this as `is_bgm_tone_descriptor`: keep-slot rewrite is only
/// safe for the BGM flags0 value. `flags0 >= 0` (e.g. `0x7F`, name at +8) is
/// a different layout and must not match.
pub fn unshifted_name_len_pos(raw_meta: &[u8]) -> Option<usize> {
    if raw_meta.len() < 40 {
        return None;
    }
    if i32_at(raw_meta, 0) != Some(0) {
        return None;
    }
    let flags0 = i32_at(raw_meta, 4)?;
    let flags1 = i32_at(raw_meta, 8)?;
    if flags0 >= 0 || flags1 <= 0 {
        return None;
    }
    // song_wgnmd1 and similar banks store a hash at this slot; only known
    // OB live-descriptor flags0 values are unshifted name-at-+12 layouts.
    if !matches!(
        flags0 as u32,
        0x8427_FFFF | 0x8467_FFFF | 0x8627_FFFF | 0x8427_BFFF
    ) {
        return None;
    }
    let name_len = raw_meta[12];
    if !(3..=80).contains(&name_len) {
        return None;
    }
    if 13 >= raw_meta.len() || !raw_meta[13].is_ascii_graphic() {
        return None;
    }
    let name_end = 13usize.saturating_add(name_len as usize);
    if name_end > raw_meta.len() {
        return None;
    }
    Some(12)
}

/// Byte offset of the len-prefixed name inside an OB TONE record.
pub fn ob_name_len_pos(raw_meta: &[u8]) -> Option<usize> {
    if let Some(pos) = bgm_name_len_pos(raw_meta) {
        return Some(pos);
    }
    if let Some(pos) = unshifted_name_len_pos(raw_meta) {
        return Some(pos);
    }
    if !looks_like_ob_descriptor(raw_meta) {
        return None;
    }
    let mut off = 12;
    let mut n = 0;
    loop {
        let w = i32_at(raw_meta, off)?;
        n += 1;
        off += 4;
        if w >= 0 {
            break;
        }
        if n >= MAX_MASK_WORDS {
            return None;
        }
    }
    Some(off)
}

fn skip_u32_field(cur: &mut Walk, what: &str) -> Result<(), Nus3bankError> {
    cur.skip(4, what)
}

fn skip_counted_dwords(cur: &mut Walk, what: &str) -> Result<(), Nus3bankError> {
    let count = cur.u32(what)?;
    if count > MAX_LIST {
        return Err(Nus3bankError::InvalidFormat {
            reason: format!("{what} count {count} is implausible"),
        });
    }
    // EXE: rbx += (count-1)*4 + 8  => consume count*4 + 4 including the count dword already read.
    let extra = (count as usize).saturating_mul(4);
    cur.skip(extra, what)
}

fn skip_counted_qwords_plus_header(cur: &mut Walk, what: &str) -> Result<(), Nus3bankError> {
    let count = cur.u32(what)?;
    if count > MAX_LIST {
        return Err(Nus3bankError::InvalidFormat {
            reason: format!("{what} count {count} is implausible"),
        });
    }
    // EXE: rbx += (count-1)*8 + 0xC  => count*8 + 4 including the count dword.
    let extra = (count as usize).saturating_mul(8);
    cur.skip(extra, what)
}

fn skip_name(cur: &mut Walk) -> Result<(), Nus3bankError> {
    cur.need(1, "name length")?;
    let len = cur.data[cur.pos];
    if len == 0xFF {
        cur.skip(4, "empty name sentinel")?;
        return Ok(());
    }
    let aligned = (len as usize + 4) & !3;
    cur.skip(aligned, "name")?;
    Ok(())
}

fn skip_type_block(cur: &mut Walk) -> Result<(), Nus3bankError> {
    cur.need(8, "type header")?;
    let extra = u32::from_le_bytes(cur.data[cur.pos + 4..cur.pos + 8].try_into().unwrap());
    if extra > 0x1_0000 {
        return Err(Nus3bankError::InvalidFormat {
            reason: format!("TONE type extra_len 0x{extra:x} is implausible"),
        });
    }
    cur.skip(8usize.saturating_add(extra as usize), "type payload")
}

fn skip_flags0(cur: &mut Walk, flags0: u32) -> Result<(), Nus3bankError> {
    if bit(flags0, 0) {
        skip_name(cur)?;
    }
    if bit(flags0, 1) {
        skip_type_block(cur)?;
    }
    for b in 2..=12 {
        if bit(flags0, b) {
            skip_u32_field(cur, &format!("flags0 bit {b}"))?;
        }
    }
    if bit(flags0, 13) {
        skip_u32_field(cur, "flags0 bit 13")?;
    }
    if bit(flags0, 14) {
        skip_counted_dwords(cur, "flags0 bit 14")?;
    }
    if bit(flags0, 15) {
        skip_counted_qwords_plus_header(cur, "flags0 bit 15")?;
    }
    if bit(flags0, 16) {
        skip_u32_field(cur, "flags0 bit 16")?;
    }
    if bit(flags0, 17) {
        skip_u32_field(cur, "flags0 bit 17")?;
    }
    if bit(flags0, 18) {
        skip_u32_field(cur, "flags0 bit 18")?;
    }
    if bit(flags0, 19) {
        skip_counted_dwords(cur, "flags0 bit 19")?;
    }
    if bit(flags0, 20) {
        skip_u32_field(cur, "flags0 bit 20")?;
    }
    if bit(flags0, 21) {
        skip_u32_field(cur, "flags0 bit 21")?;
    }
    if bit(flags0, 22) {
        skip_u32_field(cur, "flags0 bit 22")?;
    }
    if bit(flags0, 23) {
        skip_u32_field(cur, "flags0 bit 23")?;
    }
    if bit(flags0, 24) {
        skip_u32_field(cur, "flags0 bit 24")?;
    }
    if bit(flags0, 25) {
        skip_u32_field(cur, "flags0 bit 25")?;
    }
    if bit(flags0, 26) {
        skip_u32_field(cur, "flags0 bit 26")?;
    }
    if bit(flags0, 27) {
        skip_u32_field(cur, "flags0 bit 27")?;
    }
    if bit(flags0, 28) {
        skip_u32_field(cur, "flags0 bit 28")?;
    }
    if bit(flags0, 29) {
        skip_u32_field(cur, "flags0 bit 29")?;
    }
    if bit(flags0, 30) {
        skip_u32_field(cur, "flags0 bit 30")?;
    }
    Ok(())
}

fn skip_bit18_pairs(cur: &mut Walk) -> Result<(), Nus3bankError> {
    let count = cur.u32("flags1 bit 18 count")?;
    if count > MAX_LIST {
        return Err(Nus3bankError::InvalidFormat {
            reason: format!("flags1 bit 18 count {count} is implausible"),
        });
    }
    cur.skip((count as usize).saturating_mul(8), "flags1 bit 18 pairs")
}

fn skip_bit19_hashes(cur: &mut Walk) -> Result<(), Nus3bankError> {
    let count = cur.u32("flags1 bit 19 count")?;
    if count == 0 {
        return Ok(());
    }
    if count > MAX_LIST {
        return Err(Nus3bankError::InvalidFormat {
            reason: format!("flags1 bit 19 count {count} is implausible"),
        });
    }
    cur.skip((count as usize).saturating_mul(8), "flags1 bit 19 hashes")
}

fn skip_flags1(cur: &mut Walk, flags1: u32) -> Result<(), Nus3bankError> {
    for b in 0..=7 {
        if bit(flags1, b) {
            skip_u32_field(cur, &format!("flags1 bit {b}"))?;
        }
    }
    if bit(flags1, 8) {
        cur.need(4, "flags1 bit 8 count")?;
        let n = u32::from_le_bytes(cur.data[cur.pos..cur.pos + 4].try_into().unwrap());
        if n > MAX_LIST {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("flags1 bit 8 count {n} is implausible"),
            });
        }
        cur.skip(
            4usize.saturating_add((n as usize).saturating_mul(4)),
            "flags1 bit 8",
        )?;
    }
    for b in 9..=15 {
        if bit(flags1, b) {
            skip_u32_field(cur, &format!("flags1 bit {b}"))?;
        }
    }
    if bit(flags1, 16) {
        skip_u32_field(cur, "flags1 bit 16")?;
    }
    if bit(flags1, 17) {
        skip_u32_field(cur, "flags1 bit 17")?;
    }
    if bit(flags1, 18) {
        skip_bit18_pairs(cur)?;
    }
    if bit(flags1, 19) {
        skip_bit19_hashes(cur)?;
    }
    Ok(())
}

/// Walk one TONE record. Stubs (`< 104` bytes) are not decoded by the EXE fill path
/// used at FinalizeLoad for live cues; they return `Ok`.
pub fn probe_tone_record(raw_meta: &[u8]) -> Result<(), Nus3bankError> {
    if raw_meta.len() < 104 {
        return Ok(());
    }
    if !looks_like_ob_descriptor(raw_meta) {
        return Ok(());
    }

    let mut mask_off = 12;
    let mut words = 0usize;
    loop {
        let w = i32_at(raw_meta, mask_off).ok_or_else(|| Nus3bankError::InvalidFormat {
            reason: "TONE presence mask overran the record".to_string(),
        })?;
        words += 1;
        mask_off += 4;
        if w >= 0 {
            break;
        }
        if words >= MAX_MASK_WORDS {
            return Err(Nus3bankError::InvalidFormat {
                reason: "TONE presence mask did not terminate".to_string(),
            });
        }
    }

    let flags0 = u32_at(raw_meta, HASH_UNK1 + 4).unwrap_or(0);
    let flags1 = u32_at(raw_meta, HASH_UNK1 + 8).unwrap_or(0);

    let mut cur = Walk {
        data: raw_meta,
        pos: mask_off,
        allow_short_overrun: true,
    };
    skip_flags0(&mut cur, flags0)?;
    if words > 1 {
        skip_flags1(&mut cur, flags1)?;
    }
    Ok(())
}

/// Walk a BGM live cue. Cursor must finish inside the record; leftover 0 is vanilla.
pub fn probe_bgm_tone_record(raw_meta: &[u8]) -> Result<(), Nus3bankError> {
    if !is_bgm_tone_descriptor(raw_meta) {
        return Ok(());
    }
    let name_at = bgm_name_len_pos(raw_meta).unwrap_or(12);
    let flags0 = u32_at(raw_meta, name_at.saturating_sub(8)).unwrap_or(0);
    let flags1 = u32_at(raw_meta, name_at.saturating_sub(4)).unwrap_or(0);
    let mut cur = Walk {
        data: raw_meta,
        pos: name_at,
        allow_short_overrun: false,
    };
    skip_flags0(&mut cur, flags0)?;
    skip_flags1(&mut cur, flags1)?;
    if cur.pos != raw_meta.len() {
        return Err(Nus3bankError::InvalidFormat {
            reason: format!(
                "BGM TONE cursor {} leftover {} (record {})",
                cur.pos,
                raw_meta.len().saturating_sub(cur.pos),
                raw_meta.len()
            ),
        });
    }
    Ok(())
}

pub fn probe_live_tone(raw_meta: &[u8], name: &str) -> Result<(), Nus3bankError> {
    let result = if is_bgm_tone_descriptor(raw_meta) {
        probe_bgm_tone_record(raw_meta)
    } else {
        probe_tone_record(raw_meta)
    };
    result.map_err(|e| match e {
        Nus3bankError::InvalidFormat { reason } => Nus3bankError::InvalidFormat {
            reason: format!("cue '{name}': {reason}"),
        },
        other => other,
    })
}
