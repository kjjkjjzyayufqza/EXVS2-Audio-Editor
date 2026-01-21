use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};

use super::{
    binary_utils::BinaryReader,
    error::Nus3bankError,
    structures::{
        BinfSection, DtonSection, GrpSection, JunkSection, Nus3bankFile, PackSection, PropSection,
        RawSection, TocEntry, ToneDes, ToneMeta, ToneSection, UnkvaluesPairOrder,
    },
};

/// NUS3BANK parser (BANKTOC-only), ported from `NUS3BANK.cs`.
pub struct Nus3bankParser;

impl Nus3bankParser {
    pub fn parse_file<P: AsRef<std::path::Path>>(path: P) -> Result<Nus3bankFile, Nus3bankError> {
        let file_path = path.as_ref().to_string_lossy().to_string();
        let file = File::open(&path)?;
        let mut reader = BufReader::new(file);

        let size = reader.get_ref().metadata().map(|m| m.len()).unwrap_or(0);
        if size < 0x20 {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("File too small: {} bytes", size),
            });
        }

        Self::parse_banktoc_only(&mut reader, file_path)
    }

    fn parse_banktoc_only<R: Read + Seek>(
        reader: &mut R,
        file_path: String,
    ) -> Result<Nus3bankFile, Nus3bankError> {
        BinaryReader::assert_magic(reader, b"NUS3")?;
        let _total_size = BinaryReader::read_u32_le(reader)?;

        let banktoc = BinaryReader::read_bytes(reader, 8)?;
        if banktoc.as_slice() != b"BANKTOC " {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!(
                    "BANKTOC header not found, got {:?}",
                    String::from_utf8_lossy(&banktoc)
                ),
            });
        }

        // C# semantics:
        // - `toc_size` counts bytes from offset 0x14 (entry_count field) to end of TOC region.
        // - sections begin at `0x14 + toc_size`.
        let toc_size = BinaryReader::read_u32_le(reader)?;
        let sec_count = BinaryReader::read_u32_le(reader)?;
        if sec_count == 0 || sec_count > 0x1000 {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("Unreasonable section count: {}", sec_count),
            });
        }

        let mut toc = Vec::with_capacity(sec_count as usize);
        for _ in 0..sec_count {
            let mut magic = [0u8; 4];
            reader.read_exact(&mut magic)?;
            let size = BinaryReader::read_u32_le(reader)?;
            toc.push(TocEntry { magic, size });
        }

        // headerSize = 0x14 + toc_size (C#)
        let mut header_pos = 0x14u64 + toc_size as u64;

        let mut prop: Option<PropSection> = None;
        let mut binf: Option<BinfSection> = None;
        let mut grp: Option<GrpSection> = None;
        let mut dton: Option<DtonSection> = None;
        let mut tone: Option<ToneSection> = None;
        let mut junk: Option<JunkSection> = None;
        let mut pack: Option<PackSection> = None;
        let mut unknown_sections: Vec<RawSection> = Vec::new();

        // Read each section using TOC ordering and sizes, matching `headerSize += size + 8`.
        for entry in &toc {
            reader.seek(SeekFrom::Start(header_pos))?;
            let section_bytes = Self::read_section_block(reader, entry.magic, entry.size)?;

            match &entry.magic[..] {
                b"PROP" => prop = Some(Self::parse_prop(&section_bytes)?),
                b"BINF" => binf = Some(Self::parse_binf(&section_bytes)?),
                b"GRP " => grp = Some(Self::parse_grp(&section_bytes)?),
                b"DTON" => dton = Some(Self::parse_dton(&section_bytes)?),
                b"TONE" => tone = Some(Self::parse_tone(&section_bytes)?),
                b"JUNK" => junk = Some(Self::parse_junk(&section_bytes)?),
                b"PACK" => pack = Some(Self::parse_pack(&section_bytes)?),
                _ => {
                    // Preserve unknown section payload bytes.
                    let mut cur = Cursor::new(section_bytes.as_slice());
                    cur.seek(SeekFrom::Start(8))?;
                    let data = BinaryReader::read_bytes(&mut cur, entry.size as usize)?;
                    unknown_sections.push(RawSection {
                        magic: entry.magic,
                        size: entry.size,
                        data,
                    });
                }
            }

            header_pos += 8u64 + entry.size as u64;
        }

        let mut tone = tone.ok_or_else(|| Nus3bankError::SectionValidation {
            section: "TONE section not found".to_string(),
        })?;
        let pack = pack.ok_or_else(|| Nus3bankError::SectionValidation {
            section: "PACK section not found".to_string(),
        })?;

        // Attach PACK payload to each tone meta using C# semantics:
        // payload_start = PACK_section_start + 8, and meta.offset is relative to payload_start.
        Self::attach_pack_payloads(&mut tone, &pack)?;

        let mut file = Nus3bankFile {
            toc,
            prop,
            binf,
            grp,
            dton,
            tone,
            junk,
            pack,
            unknown_sections,
            tracks: Vec::new(),
            file_path,
        };
        file.rebuild_tracks_view();
        Ok(file)
    }

    fn read_section_block<R: Read>(
        reader: &mut R,
        expected_magic: [u8; 4],
        expected_size: u32,
    ) -> Result<Vec<u8>, Nus3bankError> {
        let mut buf = vec![0u8; 8 + expected_size as usize];
        reader.read_exact(&mut buf)?;

        if buf[0..4] != expected_magic {
            return Err(Nus3bankError::InvalidMagic {
                expected: String::from_utf8_lossy(&expected_magic).to_string(),
                found: String::from_utf8_lossy(&buf[0..4]).to_string(),
            });
        }

        let actual_size = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        if actual_size != expected_size {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!(
                    "Section size mismatch for {:?}: TOC={}, header={}",
                    String::from_utf8_lossy(&expected_magic),
                    expected_size,
                    actual_size
                ),
            });
        }

        Ok(buf)
    }

    fn parse_prop(section: &[u8]) -> Result<PropSection, Nus3bankError> {
        let mut r = Cursor::new(section);
        BinaryReader::assert_magic(&mut r, b"PROP")?;
        let _section_size = BinaryReader::read_u32_le(&mut r)?;

        let _padding0 = BinaryReader::read_u32_le(&mut r)?;
        let unk1 = BinaryReader::read_i32_le(&mut r)?;
        let reserved_u16 = BinaryReader::read_u16_le(&mut r)?;
        let unk2 = BinaryReader::read_u16_le(&mut r)?;

        // project (u8 length includes null terminator)
        let project = BinaryReader::read_len_u8_string(&mut r)?;
        BinaryReader::align4(&mut r)?;

        // Some files use a minimal PROP layout that ends after `project`.
        let mut layout = super::structures::PropLayout::Minimal;
        let mut unk3: u16 = 0;
        let mut timestamp = String::new();

        let remaining = (section.len() as u64).saturating_sub(r.position()) as usize;
        if remaining >= 2 {
            layout = super::structures::PropLayout::Extended;
            unk3 = BinaryReader::read_u16_le(&mut r)?;
            BinaryReader::align4(&mut r)?;

            // Timestamp is optional; parse best-effort if present.
            let rem = (section.len() as u64).saturating_sub(r.position()) as usize;
            if rem >= 1 {
                let ts_len_with_null = BinaryReader::read_u8(&mut r)? as usize;
                if ts_len_with_null == 0 {
                    timestamp.clear();
                } else {
                    let need = ts_len_with_null; // content + null
                    let rem2 = (section.len() as u64).saturating_sub(r.position()) as usize;
                    if need <= rem2 {
                        let content =
                            BinaryReader::read_string_exact(&mut r, ts_len_with_null - 1)?;
                        BinaryReader::skip(&mut r, 1)?;
                        timestamp = content;
                    } else {
                        timestamp.clear();
                    }
                }
            }
        }

        Ok(PropSection {
            project,
            timestamp,
            unk1,
            reserved_u16,
            unk2,
            unk3,
            layout,
        })
    }

    fn parse_binf(section: &[u8]) -> Result<BinfSection, Nus3bankError> {
        let mut r = Cursor::new(section);
        BinaryReader::assert_magic(&mut r, b"BINF")?;
        let _section_size = BinaryReader::read_u32_le(&mut r)?;

        // C# `d.Skip(12)` from section start => after magic+size, there's a 4-byte reserved value.
        let reserved0 = BinaryReader::read_i32_le(&mut r)?;
        let unk1 = BinaryReader::read_i32_le(&mut r)?;

        let name_len = BinaryReader::read_u8(&mut r)? as usize;
        let name = if name_len > 0 {
            BinaryReader::read_string_exact(&mut r, name_len - 1)?
        } else {
            String::new()
        };
        // consume null terminator
        BinaryReader::skip(&mut r, 1)?;
        BinaryReader::align4(&mut r)?;
        let flag = BinaryReader::read_i32_le(&mut r)?;

        Ok(BinfSection {
            reserved0,
            unk1,
            name,
            flag,
        })
    }

    fn parse_grp(section: &[u8]) -> Result<GrpSection, Nus3bankError> {
        let mut r = Cursor::new(section);
        BinaryReader::assert_magic(&mut r, b"GRP ")?;
        let _section_size = BinaryReader::read_u32_le(&mut r)?;

        let count = BinaryReader::read_u32_le(&mut r)? as usize;
        let start = r.position();

        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let offset = BinaryReader::read_u32_le(&mut r)?;
            let size = BinaryReader::read_u32_le(&mut r)?;
            entries.push((offset, size));
        }

        let mut names = Vec::with_capacity(count);
        for (offset, size) in entries {
            let entry_start = start + offset as u64;
            // NOTE:
            // C# `NusGrp.Read` does NOT use the pointer-table `size` for bounds checking.
            // Real-world files may contain values that don't match the actual block length
            // (especially for the last entry), so we only validate `entry_start` and then
            // clamp the read window to the section end.
            if entry_start >= section.len() as u64 {
                return Err(Nus3bankError::InvalidFormat {
                    reason: "GRP entry offset out of bounds".to_string(),
                });
            }
            let section_end = section.len() as u64;
            let entry_end = if size == 0 {
                section_end
            } else {
                (entry_start + size as u64).min(section_end)
            };

            r.seek(SeekFrom::Start(entry_start))?;
            let _one = BinaryReader::read_i32_le(&mut r)?;

            // C# reads a signed byte here; it uses 0xFF for empty strings when rebuilding.
            // In practice we can safely parse as:
            // - 0xFF => empty string (still consume up to the next null if present)
            // - otherwise => length includes null terminator (len_with_null)
            let len_byte = BinaryReader::read_u8(&mut r)?;
            let name = if len_byte == 0xFF {
                // Read until null within the clamped window.
                let mut str_bytes = Vec::new();
                while r.position() < entry_end {
                    let b = BinaryReader::read_u8(&mut r)?;
                    if b == 0 {
                        break;
                    }
                    str_bytes.push(b);
                }
                String::from_utf8_lossy(&str_bytes).to_string()
            } else if len_byte == 0 {
                String::new()
            } else {
                let wanted = (len_byte as usize).saturating_sub(1);
                let remaining = (entry_end.saturating_sub(r.position())) as usize;
                let to_read = wanted.min(remaining);
                let content = BinaryReader::read_bytes(&mut r, to_read)?;
                // Consume null terminator if present (best-effort).
                if r.position() < entry_end {
                    let _ = BinaryReader::read_u8(&mut r)?;
                }
                String::from_utf8_lossy(&content).to_string()
            };

            names.push(name);
        }

        Ok(GrpSection { names })
    }

    pub(crate) fn parse_dton(section: &[u8]) -> Result<DtonSection, Nus3bankError> {
        if section.len() < 12 {
            return Err(Nus3bankError::InvalidFormat {
                reason: "DTON section too small".to_string(),
            });
        }

        // The section carries its own declared size (excluding the 8-byte header).
        // Use it to bound parsing, so trailing bytes do not affect offsets.
        let declared_size =
            u32::from_le_bytes([section[4], section[5], section[6], section[7]]) as usize;
        let declared_total = 8usize + declared_size;
        if declared_total > section.len() {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!(
                    "DTON section truncated: declared_total={} actual={}",
                    declared_total,
                    section.len()
                ),
            });
        }

        let section = &section[..declared_total];
        let mut r = Cursor::new(section);
        BinaryReader::assert_magic(&mut r, b"DTON")?;
        let _section_size = BinaryReader::read_u32_le(&mut r)?;

        let count = BinaryReader::read_u32_le(&mut r)? as usize;
        let start = r.position();

        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let offset = BinaryReader::read_u32_le(&mut r)?;
            let size = BinaryReader::read_u32_le(&mut r)?;
            entries.push((offset, size));
        }

        let mut tones = Vec::with_capacity(count);
        for (offset, size) in entries {
            let entry_start = start + offset as u64;
            let entry_end = entry_start
                .saturating_add(size as u64)
                .min(section.len() as u64);
            // NOTE:
            // Use the pointer-table `size` to bound parsing for this entry.
            if entry_start >= section.len() as u64 {
                return Err(Nus3bankError::InvalidFormat {
                    reason: "DTON entry offset out of bounds".to_string(),
                });
            }
            r.seek(SeekFrom::Start(entry_start))?;

            let hash = BinaryReader::read_i32_le(&mut r)?;
            let unk1 = BinaryReader::read_i32_le(&mut r)?;
            let name = BinaryReader::read_len_u8_string(&mut r)?;
            BinaryReader::align4(&mut r)?;

            let data_start = r.position();
            if data_start > entry_end {
                return Err(Nus3bankError::InvalidFormat {
                    reason: "DTON entry size too small for header".to_string(),
                });
            }

            let available = (entry_end - data_start) as usize;
            let float_bytes = available - (available % 4);
            let float_count = float_bytes / 4;

            let mut data = Vec::with_capacity(float_count);
            for _ in 0..float_count {
                if r.position() + 4 > entry_end {
                    break;
                }
                data.push(BinaryReader::read_f32_le(&mut r)?);
            }

            tones.push(ToneDes {
                hash,
                unk1,
                name,
                data,
            });
        }

        Ok(DtonSection { tones })
    }

    fn parse_tone(section: &[u8]) -> Result<ToneSection, Nus3bankError> {
        let mut r = Cursor::new(section);
        BinaryReader::assert_magic(&mut r, b"TONE")?;
        let _section_size = BinaryReader::read_u32_le(&mut r)?;

        let count = BinaryReader::read_u32_le(&mut r)? as usize;
        let start = r.position();

        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let offset = BinaryReader::read_u32_le(&mut r)?;
            let size = BinaryReader::read_u32_le(&mut r)?;
            entries.push((offset, size));
        }

        let mut tones = Vec::with_capacity(count);
        for (tone_idx, (offset, meta_size)) in entries.into_iter().enumerate() {
            let meta_start = start + offset as u64;
            let section_end = section.len() as u64;
            if meta_start >= section_end {
                return Err(Nus3bankError::InvalidFormat {
                    reason: format!("TONE meta offset out of bounds (index={})", tone_idx),
                });
            }
            // Some files may report a `meta_size` that slightly exceeds the remaining section bytes
            // (e.g. last entry). Clamp to section end to avoid hard failure; C# readers ignore this size.
            let meta_end = (meta_start + meta_size as u64).min(section_end);
            r.seek(SeekFrom::Start(meta_start))?;

            // Some BANKTOC files contain placeholder/stub TONE entries (very small `meta_size`),
            // which do not include the full ToneMeta structure. Treat them as removed/ignored.
            // Minimum full header up to `param` is ~100 bytes (depends on name length), so we use a
            // conservative cutoff and fall back to a minimal parse.
            if meta_size < 104 {
                let hash = BinaryReader::read_i32_le(&mut r)?;
                let unk1 = BinaryReader::read_i32_le(&mut r)?;
                let mut name_bytes = Vec::new();
                while r.position() < meta_end {
                    let b = BinaryReader::read_u8(&mut r)?;
                    if b == 0 {
                        break;
                    }
                    name_bytes.push(b);
                }
                let name = String::from_utf8_lossy(&name_bytes).to_string();

                tones.push(ToneMeta {
                    meta_prefix: Vec::new(),
                    hash,
                    unk1,
                    name,
                    reserved0: 0,
                    reserved8: 8,
                    offset: 0,
                    size: 0,
                    param: [0.0; 12],
                    offsets: Vec::new(),
                    unkvalues: Vec::new(),
                    unkvalues_pair_order: UnkvaluesPairOrder::IndexThenValue,
                    unkending: vec![-1],
                    end: Vec::new(),
                    payload: Vec::new(),
                    meta_size,
                    removed: true,
                });
                continue;
            }

            let meta_slice = &section[meta_start as usize..meta_end as usize];
            let mut meta = Self::parse_tone_meta_block(meta_slice, tone_idx)?;
            meta.meta_size = meta_size;
            tones.push(meta);
        }

        Ok(ToneSection { tones })
    }

    fn parse_tone_meta_block(meta: &[u8], tone_idx: usize) -> Result<ToneMeta, Nus3bankError> {
        fn align4_pos(pos: u64) -> u64 {
            (pos + 3) & !3
        }

        fn try_parse(meta: &[u8], tone_idx: usize, prefix_len: usize) -> Result<ToneMeta, Nus3bankError> {
            let mut c = Cursor::new(meta);

            let meta_prefix = if prefix_len == 8 {
                BinaryReader::read_bytes(&mut c, 8)?
            } else {
                Vec::new()
            };

            let hash = BinaryReader::read_i32_le(&mut c)?;
            let unk1 = BinaryReader::read_i32_le(&mut c)?;
            let name_len = BinaryReader::read_u8(&mut c)? as usize;
            if name_len == 0 || (c.position() + name_len as u64) > meta.len() as u64 {
                return Err(Nus3bankError::InvalidFormat {
                    reason: format!("Invalid TONE name_len (index={})", tone_idx),
                });
            }
            let name = BinaryReader::read_string_exact(&mut c, name_len - 1)?;
            BinaryReader::skip(&mut c, 1)?;
            c.seek(SeekFrom::Start(align4_pos(c.position())))?;

            let reserved0 = BinaryReader::read_i32_le(&mut c)?;
            let reserved8 = BinaryReader::read_i32_le(&mut c)?;
            let offset = BinaryReader::read_i32_le(&mut c)?;
            let size = BinaryReader::read_i32_le(&mut c)?;

            let mut param = [0.0f32; 12];
            for i in 0..12 {
                param[i] = BinaryReader::read_f32_le(&mut c)?;
            }

            let offsets_count = BinaryReader::read_i32_le(&mut c)?;
            if offsets_count < 0 || offsets_count > 1_000_000 {
                return Err(Nus3bankError::InvalidFormat {
                    reason: format!("Invalid offsets_count: {} (index={})", offsets_count, tone_idx),
                });
            }
            let needed_offsets_bytes = (offsets_count as u64) * 4;
            if c.position() + needed_offsets_bytes + 4 > meta.len() as u64 {
                return Err(Nus3bankError::InvalidFormat {
                    reason: format!("Offsets table exceeds meta bounds (index={})", tone_idx),
                });
            }
            let mut offsets = Vec::with_capacity(offsets_count as usize);
            for _ in 0..offsets_count {
                offsets.push(BinaryReader::read_i32_le(&mut c)?);
            }

            let unkvalues_count = BinaryReader::read_i32_le(&mut c)?;
            if unkvalues_count < 0 || unkvalues_count > 1_000_000 {
                return Err(Nus3bankError::InvalidFormat {
                    reason: format!("Invalid unkvalues_count: {} (index={})", unkvalues_count, tone_idx),
                });
            }

            let pairs_start = c.position();
            let mut unkvalues_pair_order = UnkvaluesPairOrder::IndexThenValue;
            if unkvalues_count > 0 && (meta.len() as u64).saturating_sub(pairs_start) >= 8 {
                let peek = BinaryReader::read_bytes(&mut c, 8)?;
                c.seek(SeekFrom::Start(pairs_start))?;

                let a_idx = i32::from_le_bytes([peek[0], peek[1], peek[2], peek[3]]);
                let a_val = f32::from_le_bytes([peek[4], peek[5], peek[6], peek[7]]);
                let b_val = f32::from_le_bytes([peek[0], peek[1], peek[2], peek[3]]);
                let b_idx = i32::from_le_bytes([peek[4], peek[5], peek[6], peek[7]]);

                let idx_ok = |x: i32| x == -1 || (0..=1_000_000).contains(&x);
                let val_ok = |x: f32| x.is_finite();
                let a_ok = idx_ok(a_idx) && val_ok(a_val);
                let b_ok = idx_ok(b_idx) && val_ok(b_val);

                unkvalues_pair_order = match (a_ok, b_ok) {
                    (true, false) => UnkvaluesPairOrder::IndexThenValue,
                    (false, true) => UnkvaluesPairOrder::ValueThenIndex,
                    (true, true) => UnkvaluesPairOrder::IndexThenValue,
                    (false, false) => {
                        return Err(Nus3bankError::InvalidFormat {
                            reason: format!("Unable to determine unkvalues pair order (index={})", tone_idx),
                        });
                    }
                };
            }

            let mut unkvalues: Vec<f32> = Vec::new();
            for _ in 0..unkvalues_count {
                if (meta.len() as u64).saturating_sub(c.position()) < 8 {
                    break;
                }
                let (idx, v) = match unkvalues_pair_order {
                    UnkvaluesPairOrder::IndexThenValue => {
                        let idx = BinaryReader::read_i32_le(&mut c)?;
                        if idx == -1 {
                            break;
                        }
                        let v = BinaryReader::read_f32_le(&mut c)?;
                        (idx, v)
                    }
                    UnkvaluesPairOrder::ValueThenIndex => {
                        let v = BinaryReader::read_f32_le(&mut c)?;
                        let idx = BinaryReader::read_i32_le(&mut c)?;
                        if idx == -1 {
                            break;
                        }
                        (idx, v)
                    }
                };

                if idx < 0 {
                    return Err(Nus3bankError::InvalidFormat {
                        reason: format!("unkvalues index out of range (index={})", tone_idx),
                    });
                }
                let idx = idx as usize;
                if idx > 1_000_000 {
                    return Err(Nus3bankError::InvalidFormat {
                        reason: format!("unkvalues index too large (index={})", tone_idx),
                    });
                }
                if idx >= unkvalues.len() {
                    unkvalues.resize(idx + 1, 0.0);
                }
                unkvalues[idx] = v;
            }

            let mut unkending = Vec::new();
            let mut found_term = false;
            while (meta.len() as u64).saturating_sub(c.position()) >= 4 {
                let v = BinaryReader::read_i32_le(&mut c)?;
                unkending.push(v);
                if v == -1 {
                    found_term = true;
                    break;
                }
            }
            if !found_term {
                unkending.push(-1);
            }

            let remaining_bytes = (meta.len() as u64).saturating_sub(c.position()) as usize;
            let mut end = Vec::new();
            if remaining_bytes % 4 == 0 {
                while (meta.len() as u64).saturating_sub(c.position()) >= 4 {
                    end.push(BinaryReader::read_i32_le(&mut c)?);
                }
            }

            Ok(ToneMeta {
                meta_prefix,
                hash,
                unk1,
                name,
                reserved0,
                reserved8,
                offset,
                size,
                param,
                offsets,
                unkvalues,
                unkvalues_pair_order,
                unkending,
                end,
                payload: Vec::new(),
                meta_size: meta.len() as u32,
                removed: false,
            })
        }

        let a = try_parse(meta, tone_idx, 0);
        let b = try_parse(meta, tone_idx, 8);

        match (a, b) {
            (Ok(x), Ok(y)) => {
                let score = |t: &ToneMeta| -> i32 {
                    let mut s = 0;
                    if t.reserved8 == 8 {
                        s += 2;
                    }
                    if !t.name.is_empty() {
                        s += 2;
                    }
                    if t.offset >= 0 && t.size >= 0 {
                        s += 1;
                    }
                    if t.offsets.len() <= 0x1000 {
                        s += 1;
                    }
                    if t.unkvalues.len() <= 0x1000 {
                        s += 1;
                    }
                    if !t.meta_prefix.is_empty() {
                        s += 1;
                    }
                    s
                };
                if score(&y) > score(&x) {
                    Ok(y)
                } else {
                    Ok(x)
                }
            }
            (Ok(x), Err(_)) => Ok(x),
            (Err(_), Ok(y)) => Ok(y),
            (Err(_ea), Err(_eb)) => {
                // Fallback for unsupported/unknown meta layouts:
                // - Do not guess field semantics
                // - Preserve raw bytes in `meta_prefix` for debug inspection
                Ok(ToneMeta {
                    meta_prefix: meta.to_vec(),
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
                    end: Vec::new(),
                    payload: Vec::new(),
                    meta_size: meta.len() as u32,
                    removed: true,
                })
            }
        }
    }

    fn parse_junk(section: &[u8]) -> Result<JunkSection, Nus3bankError> {
        let mut r = Cursor::new(section);
        BinaryReader::assert_magic(&mut r, b"JUNK")?;
        let size = BinaryReader::read_u32_le(&mut r)? as usize;
        // Some files use JUNK size 4, others use 8 (and potentially other small values).
        // Preserve payload bytes as-is; the section slice boundary already prevents over-read.
        let data = BinaryReader::read_bytes(&mut r, size)?;
        Ok(JunkSection { data })
    }

    fn parse_pack(section: &[u8]) -> Result<PackSection, Nus3bankError> {
        let mut r = Cursor::new(section);
        BinaryReader::assert_magic(&mut r, b"PACK")?;
        let size = BinaryReader::read_u32_le(&mut r)? as usize;
        if size > 200_000_000 {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("PACK too large: {} bytes", size),
            });
        }
        let data = BinaryReader::read_bytes(&mut r, size)?;
        Ok(PackSection { data })
    }

    fn attach_pack_payloads(
        tone: &mut ToneSection,
        pack: &PackSection,
    ) -> Result<(), Nus3bankError> {
        for t in tone.tones.iter_mut() {
            if t.offset < 0 || t.size < 0 {
                t.payload.clear();
                continue;
            }
            let start = t.offset as usize;
            let end = start + t.size as usize;
            if end > pack.data.len() {
                return Err(Nus3bankError::InvalidFormat {
                    reason: "TONE pack offset/size out of bounds".to_string(),
                });
            }
            t.payload = pack.data[start..end].to_vec();
        }
        Ok(())
    }
}
