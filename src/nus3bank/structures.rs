use super::error::Nus3bankError;

/// A single BANKTOC entry: section magic and section data size (excluding the 8-byte section header).
#[derive(Clone, Debug)]
pub struct TocEntry {
    pub magic: [u8; 4],
    pub size: u32,
}

#[derive(Clone, Debug)]
pub struct RawSection {
    pub magic: [u8; 4],
    pub size: u32,
    pub data: Vec<u8>,
}

/// PROP section (C# `NusProp`)
#[derive(Clone, Debug)]
pub struct PropSection {
    pub project: String,
    pub timestamp: String,
    pub unk1: i32,
    /// 2 bytes skipped by C# (`d.Skip(2)`), but real files may not be zero.
    pub reserved_u16: u16,
    pub unk2: u16,
    pub unk3: u16,
    /// Controls how PROP is rebuilt to preserve original layout.
    pub layout: PropLayout,
    /// First payload u32 before the OB bitmask. Legacy layouts keep this as their first padding u32.
    pub leading_u32: u32,
    /// OB PROP presence mask at section offset +0x0C.
    pub presence_mask: u32,
    /// OB PROP version field. Major version must be 3 for bitmask PROP.
    pub version: u32,
    /// Optional OB PROP bit 5 u32 field.
    pub bit5_u32: Option<u32>,
    /// Optional OB PROP bit 6 u32 field.
    pub bit6_u32: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropLayout {
    /// Minimal PROP that ends after `project` (no `unk3`/`timestamp`).
    Minimal,
    /// Extended PROP with `unk3` and `timestamp` (C# `NusProp` behavior).
    Extended,
    /// OB engine PROP layout driven by a u32 presence mask.
    Bitmask,
}

/// BINF section (C# `NusBinf`)
#[derive(Clone, Debug)]
pub struct BinfSection {
    pub reserved0: i32,
    pub unk1: i32,
    pub name: String,
    pub flag: i32,
}

/// GRP section (C# `NusGrp`)
#[derive(Clone, Debug, Default)]
pub struct GrpSection {
    pub names: Vec<String>,
    /// Original GRP payload (excluding the 8-byte section header). Used for
    /// byte-identical write-back when the UI has not edited group names.
    pub raw_payload: Option<Vec<u8>>,
}

/// DTON section (C# `NusDton`)
#[derive(Clone, Debug, Default)]
pub struct DtonSection {
    pub tones: Vec<ToneDes>,
    /// Original DTON payload excluding the 8-byte section header.
    /// Template banks write this back byte-for-byte until the UI edits DTON.
    pub raw_payload: Option<Vec<u8>>,
}

impl DtonSection {
    /// EXVS2 character SE banks store one `Default` descriptor for the whole
    /// bank. GVS / per-tone banks store one DTON row per live cue.
    pub fn is_template(&self, tones: &[ToneMeta]) -> bool {
        if self.tones.is_empty() {
            return false;
        }
        let named_live = tones
            .iter()
            .filter(|t| !t.removed && !t.name.is_empty())
            .count();
        if self.tones.len() == named_live && named_live > 1 {
            return false;
        }
        if self.tones.len() == 1 {
            return true;
        }
        self.tones
            .iter()
            .any(|row| row.name.eq_ignore_ascii_case("Default"))
            && self.tones.len() < named_live
    }
}

#[derive(Clone, Debug)]
pub struct ToneDes {
    pub hash: i32,
    pub unk1: i32,
    pub name: String,
    pub data: Vec<f32>, // Length varies; bounded by DTON entry size
    /// Exact descriptor bytes after the name/alignment header.
    pub raw_data: Vec<u8>,
    /// Descriptor presence words read until the first word with no high continuation bit.
    pub descriptor_words: Vec<u32>,
}

/// TONE section (C# `NUS_TONE`)
#[derive(Clone, Debug, Default)]
pub struct ToneSection {
    pub tones: Vec<ToneMeta>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnkvaluesPairOrder {
    IndexThenValue,
    ValueThenIndex,
}

#[derive(Clone, Debug)]
pub struct ToneMeta {
    /// Optional 8-byte prefix found in some BANKTOC variants before the normal ToneMeta fields.
    pub meta_prefix: Vec<u8>,
    /// Exact original TONE metadata block for runtime-compatible patching.
    pub raw_meta: Vec<u8>,
    /// Byte position of the PACK offset field inside `raw_meta`.
    pub pack_offset_field_pos: Option<usize>,
    /// Byte position of the PACK size field inside `raw_meta`.
    pub pack_size_field_pos: Option<usize>,
    /// Byte position of the len-prefixed name inside `raw_meta`.
    pub name_len_pos: Option<usize>,
    /// OB presence-mask words (empty for C# / field-built records).
    pub descriptor_words: Vec<u32>,
    pub hash: i32,
    pub unk1: i32,
    pub name: String,
    pub reserved0: i32,
    pub reserved8: i32,
    pub offset: i32,
    pub size: i32,
    pub param: [f32; 12],
    pub offsets: Vec<i32>,
    pub unkvalues: Vec<f32>,
    pub unkvalues_pair_order: UnkvaluesPairOrder,
    pub unkending: Vec<i32>,
    pub end: Vec<i32>,
    pub payload: Vec<u8>,
    pub meta_size: u32,
    pub removed: bool,
}

impl ToneMeta {
    /// Rewrite the len-prefixed name inside `raw_meta` and keep the descriptor
    /// tail bytes. Pack offset/size field positions are shifted by the name
    /// length delta.
    pub fn rewrite_name_in_raw_meta(&mut self, new_name: &str) -> Result<(), Nus3bankError> {
        if self.raw_meta.is_empty() {
            self.name = new_name.to_string();
            return Ok(());
        }
        let prefix_len = self.meta_prefix.len();
        let name_len_pos = if let Some(pos) = super::ob_tone_decode::bgm_name_len_pos(&self.raw_meta)
        {
            pos
        } else {
            self.name_len_pos.unwrap_or(prefix_len + 8)
        };
        if self.raw_meta.len() <= name_len_pos {
            return Err(Nus3bankError::InvalidFormat {
                reason: "TONE raw_meta too small to rewrite name".to_string(),
            });
        }
        let old_name_len = self.raw_meta[name_len_pos] as usize;
        if old_name_len == 0 {
            return Err(Nus3bankError::InvalidFormat {
                reason: "TONE raw_meta name_len is 0".to_string(),
            });
        }
        let old_name_end = name_len_pos + 1 + old_name_len;
        if old_name_end > self.raw_meta.len() {
            return Err(Nus3bankError::InvalidFormat {
                reason: "TONE raw_meta name overruns the block".to_string(),
            });
        }
        let old_aligned = (old_name_end + 3) & !3;
        if old_aligned > self.raw_meta.len() {
            return Err(Nus3bankError::InvalidFormat {
                reason: "TONE raw_meta name alignment overruns the block".to_string(),
            });
        }

        let name_bytes = new_name.as_bytes();
        if super::ob_tone_decode::is_bgm_tone_descriptor(&self.raw_meta) {
            // EXE skip_name size is (len_byte + 4) & !3. Keep the donor length byte
            // so flags0/flags1 payload after the name stays on the vanilla cursor.
            let slot = (old_name_len + 4) & !3;
            if name_bytes.len() + 1 > old_name_len {
                return Err(Nus3bankError::InvalidFormat {
                    reason: format!(
                        "BGM cue name '{}' is longer than the donor name slot ({old_name_len})",
                        new_name
                    ),
                });
            }
            let mut field = Vec::with_capacity(slot);
            field.push(old_name_len as u8);
            field.extend_from_slice(name_bytes);
            field.push(0);
            field.resize(old_name_len + 1, 0);
            field.resize(slot, 0);
            self.raw_meta[name_len_pos..name_len_pos + slot].copy_from_slice(&field);
            self.name_len_pos = Some(name_len_pos);
            self.name = new_name.to_string();
            self.meta_size = self.raw_meta.len() as u32;
            super::ob_tone_decode::probe_live_tone(&self.raw_meta, &self.name)?;
            return Ok(());
        }

        let new_name_len = (name_bytes.len() + 1).min(255);
        let stored_name = &name_bytes[..new_name_len.saturating_sub(1)];
        let new_name_end = name_len_pos + 1 + new_name_len;
        let new_aligned = (new_name_end + 3) & !3;

        let mut out = Vec::with_capacity(
            prefix_len + 8 + 1 + new_name_len + 3 + (self.raw_meta.len() - old_aligned),
        );
        out.extend_from_slice(&self.raw_meta[..name_len_pos]);
        out.push(new_name_len as u8);
        out.extend_from_slice(stored_name);
        out.push(0);
        while out.len() < new_aligned {
            out.push(0);
        }
        out.extend_from_slice(&self.raw_meta[old_aligned..]);

        let delta = new_aligned as isize - old_aligned as isize;
        let shift = |pos: Option<usize>| -> Option<usize> {
            pos.and_then(|p| {
                let n = p as isize + delta;
                if n >= 0 { Some(n as usize) } else { None }
            })
        };
        self.pack_offset_field_pos = shift(self.pack_offset_field_pos);
        self.pack_size_field_pos = shift(self.pack_size_field_pos);
        self.name_len_pos = Some(name_len_pos);
        self.raw_meta = out;
        self.name = String::from_utf8_lossy(stored_name).into_owned();
        self.meta_size = self.raw_meta.len() as u32;
        if super::ob_tone_decode::looks_like_ob_descriptor(&self.raw_meta)
            || super::ob_tone_decode::is_bgm_tone_descriptor(&self.raw_meta)
        {
            super::ob_tone_decode::probe_live_tone(&self.raw_meta, &self.name)?;
        }
        Ok(())
    }

    pub fn apply_new_cue_identity(
        &mut self,
        name: String,
        payload: Vec<u8>,
    ) -> Result<(), Nus3bankError> {
        if !self.raw_meta.is_empty() {
            self.rewrite_name_in_raw_meta(&name)?;
        } else {
            self.name = name;
        }
        self.payload = payload;
        self.size = self.payload.len() as i32;
        self.offset = 0;
        self.removed = false;
        self.meta_size = self.raw_meta.len() as u32;
        Ok(())
    }

    /// Vanilla one-shot cues use unk1 without bit 0x80; looping cues set it.
    /// Must patch `raw_meta` too: the writer prefers the original blob.
    pub fn set_unk1(&mut self, unk1: i32) {
        self.unk1 = unk1;
        let pos = self.meta_prefix.len() + 4;
        if self.raw_meta.len() >= pos + 4 {
            self.raw_meta[pos..pos + 4].copy_from_slice(&unk1.to_le_bytes());
        }
    }

    /// Patch the TONE clock block `48000, channels=1, n_samples, loopStart, loopEnd, loopFlag`.
    /// Cloned looping templates leave the donor's sample count/loop here; the game
    /// then seeks past the real BNSF length and plays silence.
    ///
    /// Looping records also keep two 0 pads before the `-1` terminator; vanilla
    /// one-shots have one. Writing only the four clock ints leaves that extra
    /// 0 in place and File→Save then thinks the bank is clean.
    pub fn patch_sample_clock(
        &mut self,
        n_samples: i32,
        loop_start: i32,
        loop_end: i32,
        loop_flag: i32,
    ) -> bool {
        if self.raw_meta.len() < 24 {
            return false;
        }
        let Some(pos) = clock_marker_pos(&self.raw_meta) else {
            return false;
        };
        let start = pos + 8;
        if start + 16 > self.raw_meta.len() {
            return false;
        }
        let mut changed = false;
        let write_i32 = |raw: &mut [u8], at: usize, value: i32| {
            raw[at..at + 4].copy_from_slice(&value.to_le_bytes());
        };
        let current = (
            i32::from_le_bytes(self.raw_meta[start..start + 4].try_into().unwrap()),
            i32::from_le_bytes(self.raw_meta[start + 4..start + 8].try_into().unwrap()),
            i32::from_le_bytes(self.raw_meta[start + 8..start + 12].try_into().unwrap()),
            i32::from_le_bytes(self.raw_meta[start + 12..start + 16].try_into().unwrap()),
        );
        if current != (n_samples, loop_start, loop_end, loop_flag) {
            write_i32(&mut self.raw_meta, start, n_samples);
            write_i32(&mut self.raw_meta, start + 4, loop_start);
            write_i32(&mut self.raw_meta, start + 8, loop_end);
            write_i32(&mut self.raw_meta, start + 12, loop_flag);
            changed = true;
        }
        let want_pads = expected_clock_pads(loop_flag);
        let pads_start = start + 16;
        if let Some(term) = terminator_after(&self.raw_meta, pads_start) {
            let have_pads = (term - pads_start) / 4;
            if have_pads != want_pads {
                let mut out = Vec::with_capacity(
                    pads_start + want_pads * 4 + (self.raw_meta.len() - term),
                );
                out.extend_from_slice(&self.raw_meta[..pads_start]);
                for _ in 0..want_pads {
                    out.extend_from_slice(&0i32.to_le_bytes());
                }
                out.extend_from_slice(&self.raw_meta[term..]);
                self.raw_meta = out;
                self.meta_size = self.raw_meta.len() as u32;
                changed = true;
            }
        }
        changed
    }

    pub fn sample_clock_from_raw_meta(&self) -> Option<(i32, i32, i32, i32)> {
        let pos = clock_marker_pos(&self.raw_meta)?;
        let start = pos + 8;
        if start + 16 > self.raw_meta.len() {
            return None;
        }
        Some((
            i32::from_le_bytes(self.raw_meta[start..start + 4].try_into().ok()?),
            i32::from_le_bytes(self.raw_meta[start + 4..start + 8].try_into().ok()?),
            i32::from_le_bytes(self.raw_meta[start + 8..start + 12].try_into().ok()?),
            i32::from_le_bytes(self.raw_meta[start + 12..start + 16].try_into().ok()?),
        ))
    }

    /// 0-pads between the 4-int clock tuple and the `-1` terminator.
    /// Vanilla one-shot = 1, looping = 2.
    pub fn clock_pad_count(&self) -> Option<usize> {
        let pos = clock_marker_pos(&self.raw_meta)?;
        let pads_start = pos + 8 + 16;
        let term = terminator_after(&self.raw_meta, pads_start)?;
        Some((term - pads_start) / 4)
    }

    pub fn clock_layout_matches_loop_flag(&self, loop_flag: i32) -> bool {
        self.clock_pad_count() == Some(expected_clock_pads(loop_flag))
    }
}

fn clock_marker_pos(raw: &[u8]) -> Option<usize> {
    const MARKER: [u8; 8] = [0x80, 0xBB, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00];
    raw.windows(8).position(|w| w == MARKER)
}

fn expected_clock_pads(loop_flag: i32) -> usize {
    if loop_flag != 0 {
        2
    } else {
        1
    }
}

fn terminator_after(raw: &[u8], pos: usize) -> Option<usize> {
    let mut p = pos;
    while p + 4 <= raw.len() {
        let v = i32::from_le_bytes(raw.get(p..p + 4)?.try_into().ok()?);
        if v == -1 {
            return Some(p);
        }
        p += 4;
    }
    None
}

/// 12-byte empty TONE stub (hash + unk1 + padding). Matches the parser's
/// `< 104` placeholder path so remove/add never leave a zero-length live cue.
pub fn empty_tone_stub_raw_meta() -> Vec<u8> {
    vec![0u8; 12]
}

pub fn empty_tone_stub() -> ToneMeta {
    let raw_meta = empty_tone_stub_raw_meta();
    ToneMeta {
        meta_prefix: Vec::new(),
        raw_meta: raw_meta.clone(),
        pack_offset_field_pos: None,
        pack_size_field_pos: None,
        name_len_pos: None,
        descriptor_words: Vec::new(),
        hash: 0,
        unk1: 0,
        name: String::new(),
        reserved0: 0,
        reserved8: 8,
        offset: 0,
        size: 0,
        param: [0.0; 12],
        offsets: Vec::new(),
        unkvalues: Vec::new(),
        unkvalues_pair_order: UnkvaluesPairOrder::IndexThenValue,
        unkending: vec![-1],
        end: vec![0, 0, 0],
        payload: Vec::new(),
        meta_size: raw_meta.len() as u32,
        removed: true,
    }
}

pub fn detect_pack_alignment(tones: &[ToneMeta]) -> usize {
    let offs: Vec<i32> = tones
        .iter()
        .filter(|t| !t.removed && t.size > 0 && t.offset >= 0)
        .map(|t| t.offset)
        .collect();
    if !offs.is_empty() && offs.iter().all(|offset| offset % 16 == 0) {
        16
    } else {
        4
    }
}

#[derive(Clone, Debug)]
pub struct JunkSection {
    /// JUNK payload bytes (size varies across files).
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct PackSection {
    /// PACK payload bytes (excluding the section header). This is optional because we rebuild from `ToneMeta.payload`.
    pub data: Vec<u8>,
}

/// Supported audio formats (WAV-focused UI, but payload bytes may be non-WAV).
#[derive(Clone, Debug, PartialEq)]
pub enum AudioFormat {
    Wav,
    Unknown,
}

/// UI-facing audio track view derived from `ToneMeta`.
#[derive(Clone, Debug)]
pub struct AudioTrack {
    /// Sequential index in the `TONE` list (0-based)
    pub index: usize,
    /// Hex string representation of the track index ("0x0", "0xb2", etc.)
    pub hex_id: String,
    /// Numeric ID value (same as `index` for BANKTOC-only mode)
    pub numeric_id: u32,
    pub name: String,
    /// Offset within PACK payload (no section header)
    pub pack_offset: u32,
    /// Audio payload size in bytes
    pub size: u32,
    /// Metadata size from the TONE pointer table (used for filtering/round-trip)
    pub metadata_size: u32,
    pub audio_data: Option<Vec<u8>>,
    pub audio_format: AudioFormat,
    /// Index into `ToneSection.tones`
    pub tone_index: usize,
}

impl AudioTrack {
    pub fn filename(&self) -> String {
        format!("{}-{}.wav", self.hex_id, self.name)
    }
}

/// Main structure representing a complete NUS3BANK file (BANKTOC-only mode).
#[derive(Clone, Debug)]
pub struct Nus3bankFile {
    pub toc: Vec<TocEntry>,
    pub prop: Option<PropSection>,
    pub binf: Option<BinfSection>,
    pub grp: Option<GrpSection>,
    pub dton: Option<DtonSection>,
    pub tone: ToneSection,
    pub junk: Option<JunkSection>,
    pub pack: PackSection,
    pub unknown_sections: Vec<RawSection>,
    /// Flattened UI track list derived from `tone`
    pub tracks: Vec<AudioTrack>,
    pub file_path: String,
}

impl Nus3bankFile {
    /// Open and parse a NUS3BANK file (BANKTOC-only).
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Nus3bankError> {
        super::parser::Nus3bankParser::parse_file(path)
    }

    /// Save the NUS3BANK file to disk.
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Nus3bankError> {
        super::writer::Nus3bankWriter::write_file(self, path)
    }

    pub fn get_track_by_hex_id(&self, hex_id: &str) -> Option<&AudioTrack> {
        self.tracks.iter().find(|t| t.hex_id == hex_id)
    }

    pub fn get_track_by_hex_id_mut(&mut self, hex_id: &str) -> Option<&mut AudioTrack> {
        self.tracks.iter_mut().find(|t| t.hex_id == hex_id)
    }

    pub fn replace_track_data(
        &mut self,
        hex_id: &str,
        new_data: Vec<u8>,
    ) -> Result<(), Nus3bankError> {
        if new_data.is_empty() {
            return Err(Nus3bankError::InvalidFormat {
                reason: "Audio data cannot be empty".to_string(),
            });
        }

        let track_pos = self
            .tracks
            .iter()
            .position(|t| t.hex_id == hex_id)
            .ok_or_else(|| Nus3bankError::TrackNotFound {
                hex_id: hex_id.to_string(),
            })?;
        let tone_index = self.tracks[track_pos].tone_index;

        {
            let tone = self.tone.tones.get_mut(tone_index).ok_or_else(|| {
                Nus3bankError::InvalidFormat {
                    reason: format!("Tone index out of bounds for track {}", hex_id),
                }
            })?;
            tone.payload = new_data.clone();
            tone.size = new_data.len() as i32;
        }

        {
            let track =
                self.tracks
                    .get_mut(track_pos)
                    .ok_or_else(|| Nus3bankError::InvalidFormat {
                        reason: "Track index out of bounds".to_string(),
                    })?;
            track.audio_data = Some(new_data.clone());
            track.size = new_data.len() as u32;
            track.audio_format = if new_data.starts_with(b"RIFF") {
                AudioFormat::Wav
            } else {
                AudioFormat::Unknown
            };
        }

        Ok(())
    }
}

fn bank_appends_new_cues(binf: Option<&BinfSection>) -> bool {
    binf.is_some_and(|b| b.name.to_ascii_uppercase().starts_with("BGM"))
}

/// Next TONE index for Add. BGM banks keep the 12-byte stub at 0 and append.
pub fn next_live_add_index(tones: &[ToneMeta], binf: Option<&BinfSection>) -> usize {
    if bank_appends_new_cues(binf) {
        return tones.len();
    }
    tones
        .iter()
        .position(|t| t.removed)
        .unwrap_or(tones.len())
}

impl Nus3bankFile {
    pub fn add_track(
        &mut self,
        name: String,
        audio_data: Vec<u8>,
    ) -> Result<String, Nus3bankError> {
        let new_index = next_live_add_index(&self.tone.tones, self.binf.as_ref());
        self.add_track_at(new_index, name, audio_data)
    }

    /// Insert a live cue at a reserved TONE index.
    ///
    /// Reuses a removed stub at `reserved_index`, appends when
    /// `reserved_index == tones.len()`, or pads intermediate stubs when applying
    /// pending adds out of index order.
    pub fn add_track_at(
        &mut self,
        reserved_index: usize,
        name: String,
        audio_data: Vec<u8>,
    ) -> Result<String, Nus3bankError> {
        if audio_data.is_empty() {
            return Err(Nus3bankError::InvalidFormat {
                reason: "Audio data cannot be empty".to_string(),
            });
        }
        if name.is_empty() {
            return Err(Nus3bankError::InvalidFormat {
                reason: "Track name cannot be empty".to_string(),
            });
        }

        // Prevent duplicate names (matches existing UI expectations).
        if self.tracks.iter().any(|t| t.name == name) {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("Track with name '{}' already exists", name),
            });
        }

        if reserved_index > self.tone.tones.len().saturating_add(1024) {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!(
                    "Reserved TONE index {} is out of range",
                    reserved_index
                ),
            });
        }
        if reserved_index < self.tone.tones.len() && !self.tone.tones[reserved_index].removed {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("TONE slot 0x{:x} is already occupied", reserved_index),
            });
        }

        let dton_is_template = self
            .dton
            .as_ref()
            .map(|dton| dton.is_template(&self.tone.tones))
            .unwrap_or(false);
        let bgm_bank = bank_appends_new_cues(self.binf.as_ref());

        let existed = reserved_index < self.tone.tones.len();
        let hex_id = format!("0x{:x}", reserved_index as u32);

        let template = self.tone.tones.iter().find(|t| !t.removed).cloned();
        let from_template = template.is_some();
        let mut new_tone = if let Some(t) = template {
            let mut cloned = t;
            cloned.apply_new_cue_identity(name.clone(), audio_data.clone())?;
            if super::ob_tone_decode::looks_like_ob_descriptor(&cloned.raw_meta)
                || super::ob_tone_decode::is_bgm_tone_descriptor(&cloned.raw_meta)
            {
                super::ob_tone_decode::probe_live_tone(&cloned.raw_meta, &cloned.name)?;
            }
            cloned
        } else {
            ToneMeta {
                meta_prefix: Vec::new(),
                raw_meta: Vec::new(),
                pack_offset_field_pos: None,
                pack_size_field_pos: None,
                name_len_pos: None,
                descriptor_words: Vec::new(),
                hash: 0,
                unk1: 0,
                name: name.clone(),
                reserved0: 0,
                reserved8: 8,
                offset: 0,
                size: audio_data.len() as i32,
                param: [0.0; 12],
                offsets: Vec::new(),
                unkvalues: Vec::new(),
                unkvalues_pair_order: UnkvaluesPairOrder::IndexThenValue,
                unkending: vec![-1],
                end: vec![0, 0, 0],
                payload: audio_data.clone(),
                meta_size: 0,
                removed: false,
            }
        };

        if !from_template {
            let end_len = 3 + (((((new_tone.unk1 as u32) >> 8) & 0xFF) as usize) + 3) / 4;
            if new_tone.end.len() != end_len {
                new_tone.end.resize(end_len, 0);
            }
        }

        while self.tone.tones.len() < reserved_index {
            self.tone.tones.push(empty_tone_stub());
        }
        if reserved_index == self.tone.tones.len() {
            self.tone.tones.push(new_tone);
        } else {
            self.tone.tones[reserved_index] = new_tone;
        }

        if !dton_is_template && !bgm_bank {
            if let Some(dton) = self.dton.as_mut() {
                dton.raw_payload = None;
                let template_dton = dton
                    .tones
                    .iter()
                    .find(|row| !row.name.eq_ignore_ascii_case("Default"))
                    .cloned()
                    .or_else(|| dton.tones.first().cloned());

                let mut new_dton = template_dton.unwrap_or_else(|| ToneDes {
                    hash: 0,
                    unk1: 0,
                    name: name.clone(),
                    data: Vec::new(),
                    raw_data: Vec::new(),
                    descriptor_words: Vec::new(),
                });
                new_dton.name = name.clone();
                if existed && reserved_index < dton.tones.len() {
                    dton.tones[reserved_index] = new_dton;
                } else {
                    dton.tones.push(new_dton);
                }
            }
        }

        self.rebuild_tracks_view();

        Ok(hex_id)
    }

    pub fn remove_track(&mut self, hex_id: &str) -> Result<(), Nus3bankError> {
        let track = self
            .get_track_by_hex_id(hex_id)
            .ok_or_else(|| Nus3bankError::TrackNotFound {
                hex_id: hex_id.to_string(),
            })?
            .clone();

        let stub_raw = self
            .tone
            .tones
            .iter()
            .enumerate()
            .filter(|(i, t)| {
                *i != track.tone_index
                    && t.removed
                    && !t.raw_meta.is_empty()
                    && t.raw_meta.len() < 104
            })
            .min_by_key(|(_, t)| t.raw_meta.len())
            .map(|(_, t)| (t.raw_meta.clone(), t.hash, t.unk1, t.meta_prefix.clone()))
            .unwrap_or_else(|| (empty_tone_stub_raw_meta(), 0, 0, Vec::new()));

        let tone = self.tone.tones.get_mut(track.tone_index).ok_or_else(|| {
            Nus3bankError::InvalidFormat {
                reason: format!("Tone index out of bounds for track {}", hex_id),
            }
        })?;

        tone.removed = true;
        tone.payload.clear();
        tone.size = 0;
        tone.offset = 0;
        tone.name.clear();
        let (raw, hash, unk1, prefix) = stub_raw;
        tone.raw_meta = raw;
        tone.hash = hash;
        tone.unk1 = unk1;
        tone.meta_prefix = prefix;
        tone.pack_offset_field_pos = None;
        tone.pack_size_field_pos = None;
        tone.meta_size = tone.raw_meta.len() as u32;

        // Keep the TONE slot. Do not compact the pointer table; OB looks up by index.
        self.rebuild_tracks_view();

        Ok(())
    }

    pub(crate) fn rebuild_tracks_view(&mut self) {
        let mut tracks = Vec::new();
        for (i, tone) in self.tone.tones.iter().enumerate() {
            if tone.removed {
                continue;
            }

            let hex_id = format!("0x{:x}", i as u32);
            let audio_data = if tone.payload.is_empty() {
                None
            } else {
                Some(tone.payload.clone())
            };

            let audio_format = if tone.payload.starts_with(b"RIFF") {
                AudioFormat::Wav
            } else {
                AudioFormat::Unknown
            };

            tracks.push(AudioTrack {
                index: tracks.len(),
                hex_id,
                numeric_id: i as u32,
                name: tone.name.clone(),
                pack_offset: tone.offset.max(0) as u32,
                size: tone.size.max(0) as u32,
                metadata_size: tone.meta_size,
                audio_data,
                audio_format,
                tone_index: i,
            });
        }
        self.tracks = tracks;
    }
}
