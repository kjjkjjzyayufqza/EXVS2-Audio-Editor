use std::fs;

use super::{
    binary_utils::BinaryReader,
    error::Nus3bankError,
    structures::{
        BinfSection, DtonSection, GrpSection, Nus3bankFile, PropLayout, PropSection, RawSection,
        TocEntry, ToneMeta,
    },
};

/// NUS3BANK writer (BANKTOC-only), ported from `NUS3BANK.cs` and extended with PACK rebuild.
pub struct Nus3bankWriter;

impl Nus3bankWriter {
    pub fn write_file<P: AsRef<std::path::Path>>(file: &Nus3bankFile, path: P) -> Result<(), Nus3bankError> {
        // Build active tones (skip removed).
        let mut active_tones: Vec<ToneMeta> = file
            .tone
            .tones
            .iter()
            .filter(|t| !t.removed)
            .cloned()
            .collect();

        // Rebuild PACK and update each tone's offset/size (offset is relative to PACK payload start).
        let pack_payload = Self::build_pack_payload(&mut active_tones);

        // Rebuild sections in TOC order.
        let toc = file.toc.clone();
        let mut section_payloads: Vec<([u8; 4], Vec<u8>)> = Vec::with_capacity(toc.len());
        for TocEntry { magic, .. } in &toc {
            let payload = match &magic[..] {
                b"PROP" => {
                    let prop = file.prop.as_ref().ok_or_else(|| Nus3bankError::SectionValidation {
                        section: "PROP section missing".to_string(),
                    })?;
                    Self::build_prop(prop)
                }
                b"BINF" => {
                    let binf = file.binf.as_ref().ok_or_else(|| Nus3bankError::SectionValidation {
                        section: "BINF section missing".to_string(),
                    })?;
                    Self::build_binf(binf)
                }
                b"GRP " => {
                    let grp = file.grp.as_ref().ok_or_else(|| Nus3bankError::SectionValidation {
                        section: "GRP section missing".to_string(),
                    })?;
                    Self::build_grp(grp)
                }
                b"DTON" => {
                    let dton = file.dton.as_ref().ok_or_else(|| Nus3bankError::SectionValidation {
                        section: "DTON section missing".to_string(),
                    })?;
                    Self::build_dton(dton)
                }
                b"TONE" => Self::build_tone(&active_tones)?,
                b"JUNK" => {
                    let junk = file.junk.as_ref().ok_or_else(|| Nus3bankError::SectionValidation {
                        section: "JUNK section missing".to_string(),
                    })?;
                    junk.data.clone()
                }
                b"PACK" => pack_payload.clone(),
                _ => {
                    let raw = Self::find_unknown_section(file, *magic)?;
                    raw.data.clone()
                }
            };
            section_payloads.push((*magic, payload));
        }

        // Build file header + BANKTOC.
        let sec_count = toc.len() as u32;
        let toc_size = 4u32 + sec_count * 8u32;

        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"NUS3");
        out.extend_from_slice(&[0u8; 4]); // placeholder total size
        out.extend_from_slice(b"BANKTOC ");
        out.extend_from_slice(&BinaryReader::write_u32_le(toc_size));
        out.extend_from_slice(&BinaryReader::write_u32_le(sec_count));

        for (magic, payload) in &section_payloads {
            out.extend_from_slice(magic);
            out.extend_from_slice(&BinaryReader::write_u32_le(payload.len() as u32));
        }

        // Write section stream: [magic][size][payload]
        for (magic, payload) in &section_payloads {
            out.extend_from_slice(magic);
            out.extend_from_slice(&BinaryReader::write_u32_le(payload.len() as u32));
            out.extend_from_slice(payload);
        }

        // Update total size (format: file length - 8)
        let total_size = out.len().saturating_sub(8) as u32;
        out[4..8].copy_from_slice(&BinaryReader::write_u32_le(total_size));

        fs::write(path, out)?;
        Ok(())
    }

    fn find_unknown_section(file: &Nus3bankFile, magic: [u8; 4]) -> Result<&RawSection, Nus3bankError> {
        file.unknown_sections.iter().find(|s| s.magic == magic).ok_or_else(|| {
            Nus3bankError::SectionValidation {
                section: format!("Unknown section {:?} missing", String::from_utf8_lossy(&magic)),
            }
        })
    }

    fn build_pack_payload(tones: &mut [ToneMeta]) -> Vec<u8> {
        let mut pack: Vec<u8> = Vec::new();
        for t in tones.iter_mut() {
            t.offset = pack.len() as i32;
            t.size = t.payload.len() as i32;
            pack.extend_from_slice(&t.payload);
            let pad = BinaryReader::calculate_padding(pack.len());
            if pad > 0 {
                pack.extend(std::iter::repeat(0u8).take(pad));
            }
        }
        pack
    }

    fn build_prop(prop: &PropSection) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&BinaryReader::write_u32_le(0)); // padding
        b.extend_from_slice(&BinaryReader::write_i32_le(prop.unk1));
        b.extend_from_slice(&BinaryReader::write_u16_le(prop.reserved_u16));
        b.extend_from_slice(&BinaryReader::write_u16_le(prop.unk2));

        let project_bytes = prop.project.as_bytes();
        let project_len = (project_bytes.len() + 1).min(255) as u8;
        b.push(project_len);
        b.extend_from_slice(&project_bytes[..(project_len as usize).saturating_sub(1)]);
        b.push(0); // null terminator
        while b.len() % 4 != 0 {
            b.push(0);
        }

        if prop.layout == PropLayout::Minimal {
            return b;
        }

        b.extend_from_slice(&BinaryReader::write_u16_le(prop.unk3));
        while b.len() % 4 != 0 {
            b.push(0);
        }

        let ts_bytes = prop.timestamp.as_bytes();
        let ts_len = (ts_bytes.len() + 1).min(255) as u8;
        b.push(ts_len);
        b.extend_from_slice(&ts_bytes[..(ts_len as usize).saturating_sub(1)]);
        b.push(0); // null terminator
        while b.len() % 4 != 0 {
            b.push(0);
        }

        b
    }

    fn build_binf(binf: &BinfSection) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&BinaryReader::write_i32_le(binf.reserved0));
        b.extend_from_slice(&BinaryReader::write_i32_le(binf.unk1));
        let name_bytes = binf.name.as_bytes();
        let name_len = (name_bytes.len() + 1).min(255) as u8;
        b.push(name_len);
        b.extend_from_slice(&name_bytes[..(name_len as usize).saturating_sub(1)]);
        b.push(0);
        while b.len() % 4 != 0 {
            b.push(0);
        }
        b.extend_from_slice(&BinaryReader::write_i32_le(binf.flag));
        b
    }

    fn build_grp(grp: &GrpSection) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&BinaryReader::write_u32_le(grp.names.len() as u32));

        // Pointer table starts immediately after count.
        let pointer_table_size = grp.names.len() * 8;
        let data_start = (pointer_table_size + 4) as u32; // +4 for the trailing zero u32

        let mut data = Vec::new();
        let mut offsets_and_sizes: Vec<(u32, u32)> = Vec::with_capacity(grp.names.len());

        for (i, name) in grp.names.iter().enumerate() {
            let entry_offset = data_start + data.len() as u32;
            let mut entry = Vec::new();
            entry.extend_from_slice(&BinaryReader::write_i32_le(1));
            if name.is_empty() {
                entry.push(0xFF);
            } else {
                let len = (name.as_bytes().len() + 1).min(255) as u8;
                entry.push(len);
                entry.extend_from_slice(&name.as_bytes()[..(len as usize).saturating_sub(1)]);
            }
            entry.push(0);
            while entry.len() % 4 != 0 {
                entry.push(0);
            }
            if i + 1 != grp.names.len() {
                entry.extend_from_slice(&BinaryReader::write_i32_le(0));
            }
            let entry_size = entry.len() as u32;
            offsets_and_sizes.push((entry_offset, entry_size));
            data.extend_from_slice(&entry);
        }

        for (off, sz) in offsets_and_sizes {
            payload.extend_from_slice(&BinaryReader::write_u32_le(off));
            payload.extend_from_slice(&BinaryReader::write_u32_le(sz));
        }
        payload.extend_from_slice(&BinaryReader::write_u32_le(0));
        payload.extend_from_slice(&data);
        payload
    }

    fn build_dton(dton: &DtonSection) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&BinaryReader::write_u32_le(dton.tones.len() as u32));

        let pointer_table_size = dton.tones.len() * 8;
        let data_start = (pointer_table_size + 4) as u32;

        let mut data = Vec::new();
        let mut offsets_and_sizes: Vec<(u32, u32)> = Vec::with_capacity(dton.tones.len());

        for tone in &dton.tones {
            let entry_offset = data_start + data.len() as u32;
            let mut entry = Vec::new();
            entry.extend_from_slice(&BinaryReader::write_i32_le(tone.hash));
            entry.extend_from_slice(&BinaryReader::write_i32_le(tone.unk1));
            let name_bytes = tone.name.as_bytes();
            let name_len = (name_bytes.len() + 1).min(255) as u8;
            entry.push(name_len);
            entry.extend_from_slice(&name_bytes[..(name_len as usize).saturating_sub(1)]);
            entry.push(0);
            while entry.len() % 4 != 0 {
                entry.push(0);
            }
            for f in &tone.data {
                entry.extend_from_slice(&BinaryReader::write_f32_le(*f));
            }
            // Follow C# behavior: ensure there's always 4 bytes after each entry.
            entry.extend_from_slice(&BinaryReader::write_u32_le(0));

            let entry_size = entry.len() as u32;
            offsets_and_sizes.push((entry_offset, entry_size));
            data.extend_from_slice(&entry);
        }

        for (off, sz) in offsets_and_sizes {
            payload.extend_from_slice(&BinaryReader::write_u32_le(off));
            payload.extend_from_slice(&BinaryReader::write_u32_le(sz));
        }
        payload.extend_from_slice(&BinaryReader::write_u32_le(0));
        payload.extend_from_slice(&data);
        payload
    }

    fn build_tone(tones: &[ToneMeta]) -> Result<Vec<u8>, Nus3bankError> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&BinaryReader::write_u32_le(tones.len() as u32));

        let pointer_table_size = tones.len() * 8;
        let data_start = (pointer_table_size + 4) as u32;

        let mut data = Vec::new();
        let mut offsets_and_sizes: Vec<(u32, u32)> = Vec::with_capacity(tones.len());

        for t in tones {
            let entry_offset = data_start + data.len() as u32;
            let entry = Self::build_tone_meta(t)?;
            offsets_and_sizes.push((entry_offset, entry.len() as u32));
            data.extend_from_slice(&entry);
        }

        for (off, sz) in offsets_and_sizes {
            payload.extend_from_slice(&BinaryReader::write_u32_le(off));
            payload.extend_from_slice(&BinaryReader::write_u32_le(sz));
        }
        payload.extend_from_slice(&BinaryReader::write_u32_le(0));
        payload.extend_from_slice(&data);
        Ok(payload)
    }

    fn build_tone_meta(t: &ToneMeta) -> Result<Vec<u8>, Nus3bankError> {
        let mut b = Vec::new();
        if !t.meta_prefix.is_empty() {
            b.extend_from_slice(&t.meta_prefix);
        }
        b.extend_from_slice(&BinaryReader::write_i32_le(t.hash));
        b.extend_from_slice(&BinaryReader::write_i32_le(t.unk1));

        let name_bytes = t.name.as_bytes();
        let name_len = (name_bytes.len() + 1).min(255) as u8;
        b.push(name_len);
        b.extend_from_slice(&name_bytes[..(name_len as usize).saturating_sub(1)]);
        b.push(0);
        while b.len() % 4 != 0 {
            b.push(0);
        }

        b.extend_from_slice(&BinaryReader::write_i32_le(t.reserved0));
        b.extend_from_slice(&BinaryReader::write_i32_le(t.reserved8));
        b.extend_from_slice(&BinaryReader::write_i32_le(t.offset));
        b.extend_from_slice(&BinaryReader::write_i32_le(t.size));

        for f in &t.param {
            b.extend_from_slice(&BinaryReader::write_f32_le(*f));
        }

        b.extend_from_slice(&BinaryReader::write_i32_le(t.offsets.len() as i32));
        for v in &t.offsets {
            b.extend_from_slice(&BinaryReader::write_i32_le(*v));
        }

        b.extend_from_slice(&BinaryReader::write_i32_le(t.unkvalues.len() as i32));
        for (i, v) in t.unkvalues.iter().enumerate() {
            match t.unkvalues_pair_order {
                super::structures::UnkvaluesPairOrder::IndexThenValue => {
                    b.extend_from_slice(&BinaryReader::write_i32_le(i as i32));
                    b.extend_from_slice(&BinaryReader::write_f32_le(*v));
                }
                super::structures::UnkvaluesPairOrder::ValueThenIndex => {
                    b.extend_from_slice(&BinaryReader::write_f32_le(*v));
                    b.extend_from_slice(&BinaryReader::write_i32_le(i as i32));
                }
            }
        }

        let mut unkending = t.unkending.clone();
        if unkending.last().copied() != Some(-1) {
            unkending.push(-1);
        }
        for v in &unkending {
            b.extend_from_slice(&BinaryReader::write_i32_le(*v));
        }

        // Preserve `end[]` exactly as parsed from the meta block. Some real files do not follow
        // the C# `end_len` rule derived from `unk1`.
        for v in &t.end {
            b.extend_from_slice(&BinaryReader::write_i32_le(*v));
        }

        Ok(b)
    }
}
