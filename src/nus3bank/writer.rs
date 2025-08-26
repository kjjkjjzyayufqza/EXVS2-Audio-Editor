use std::fs;
use super::{
    error::Nus3bankError,
    structures::Nus3bankFile,
    binary_utils::BinaryReader,
};

/// NUS3BANK file writer
pub struct Nus3bankWriter;

impl Nus3bankWriter {
    /// Write a NUS3BANK file by updating the original bytes:
    /// - Recalculate PACK content and size
    /// - Update TONE metadata packOffset/size for each valid track
    /// - Update PACK size in BANKTOC (if present) and file total size
    pub fn write_file<P: AsRef<std::path::Path>>(file: &Nus3bankFile, path: P) -> Result<(), Nus3bankError> {
        // Read original file bytes to preserve unknown sections and exact ordering
        let original = fs::read(&file.file_path)?;

        // Small helpers
        let read_u32_le = |buf: &[u8], off: usize| -> Result<u32, Nus3bankError> {
            if off + 4 > buf.len() { return Err(Nus3bankError::InvalidFormat { reason: "u32 read out of bounds".to_string() }); }
            Ok(u32::from_le_bytes([buf[off], buf[off+1], buf[off+2], buf[off+3]]))
        };

        // Validate header basics
        if &original[0..4] != b"NUS3" {
            return Err(Nus3bankError::InvalidMagic { expected: "NUS3".to_string(), found: String::from_utf8_lossy(&original[0..4]).to_string() });
        }

        // Parse BANKTOC header
        if &original[8..12] != b"BANK" || &original[12..16] != b"TOC " {
            return Err(Nus3bankError::InvalidFormat { reason: "BANKTOC header not found".to_string() });
        }
        let toc_size = read_u32_le(&original, 16)? as usize;
        let entry_count = read_u32_le(&original, 20)? as usize;
        let entries_start = 24usize;
        let entries_end = entries_start + entry_count * 8;
        if entries_end > original.len() { return Err(Nus3bankError::InvalidFormat { reason: "TOC entries out of bounds".to_string() }); }

        // Collect entries (magic, size)
        let mut entries: Vec<([u8;4], u32)> = Vec::with_capacity(entry_count);
        let mut pack_entry_index: Option<usize> = None;
        for i in 0..entry_count {
            let base = entries_start + i*8;
            let magic = [original[base], original[base+1], original[base+2], original[base+3]];
            let size = read_u32_le(&original, base+4)?;
            if &magic == b"PACK" { pack_entry_index = Some(i); }
            entries.push((magic, size));
        }

        // Section stream starts right after TOC region (Python: 0x14 + toc_size)
        let sections_start = 20 + toc_size; // absolute offset from file start
        if sections_start > original.len() { return Err(Nus3bankError::InvalidFormat { reason: "Sections start out of bounds".to_string() }); }

        // First pass: locate original PACK section precisely by scanning entries and compute its slice
        let mut cursor_scan = sections_start;
        let mut original_pack_slice: Option<&[u8]> = None;
        let mut old_pack_size: Option<usize> = None;
        for (magic, size) in entries.iter() {
            match &magic[..] {
                b"PACK" => {
                    if cursor_scan + 8 > original.len() { return Err(Nus3bankError::InvalidFormat { reason: "PACK header out of bounds".to_string() }); }
                    let pack_size = read_u32_le(&original, cursor_scan + 4)? as usize;
                    if cursor_scan + 8 + pack_size > original.len() { return Err(Nus3bankError::InvalidFormat { reason: "PACK data out of bounds".to_string() }); }
                    original_pack_slice = Some(&original[cursor_scan + 8 .. cursor_scan + 8 + pack_size]);
                    old_pack_size = Some(pack_size);
                    // advance scan cursor as in second pass for consistency
                    cursor_scan += 8 + *size as usize;
                }
                _ => {
                    // advance over this section in original stream
                    if cursor_scan + 8 > original.len() { return Err(Nus3bankError::InvalidFormat { reason: "Section header out of bounds during scan".to_string() }); }
                    let sec_size = read_u32_le(&original, cursor_scan + 4)? as usize;
                    if cursor_scan + 8 + sec_size > original.len() { return Err(Nus3bankError::InvalidFormat { reason: "Section body out of bounds during scan".to_string() }); }
                    cursor_scan += 8 + sec_size;
                }
            }
        }
        let original_pack_slice = original_pack_slice.ok_or_else(|| Nus3bankError::InvalidFormat { reason: "PACK section not found during scan".to_string() })?;
        let old_pack_size = old_pack_size.unwrap();

        // Build new PACK from current tracks (fallback to original PACK bytes when missing)
        let mut sorted_tracks = file.tracks.clone();
        sorted_tracks.sort_by_key(|t| t.numeric_id);

        let mut new_pack: Vec<u8> = Vec::new();
        let mut new_pack_mapping: Vec<(u32, u32, u32)> = Vec::with_capacity(sorted_tracks.len());
        for t in &sorted_tracks {
            if t.metadata_size <= 0x0c { continue; }
            let data: Vec<u8> = if let Some(ref audio) = t.audio_data {
                audio.clone()
            } else if t.pack_offset < 0xffff_ffff && (t.pack_offset as usize + t.size as usize) <= original_pack_slice.len() {
                let start = t.pack_offset as usize;
                let end = start + t.size as usize;
                original_pack_slice[start..end].to_vec()
            } else {
                Vec::new()
            };
            let current_off = new_pack.len() as u32;
            new_pack.extend_from_slice(&data);
            let pad = BinaryReader::calculate_padding(data.len());
            if pad > 0 { new_pack.extend(std::iter::repeat(0u8).take(pad)); }
            new_pack_mapping.push((t.numeric_id, current_off, data.len() as u32));
        }
        let new_pack_size = new_pack.len() as u32;

        // Start constructing new file
        let mut new_file: Vec<u8> = Vec::with_capacity(original.len() - old_pack_size + new_pack.len());
        // NUS3 + placeholder size
        new_file.extend_from_slice(b"NUS3");
        new_file.extend_from_slice(&[0u8;4]);
        // Copy BANKTOC region ONLY (from original offset 8 through sections_start)
        // Note: copying up to entries_end may include the first section header/body,
        // which would later be written again when iterating sections, causing duplication.
        new_file.extend_from_slice(&original[8..sections_start]);

        // Update PACK size inside TOC in the new buffer (if present)
        if let Some(i) = pack_entry_index {
            let size_pos = 8 /*NUS3+size*/ + 16 /*'BANKTOC '+toc_size+entry_count*/ + i*8 + 4;
            if size_pos + 4 > new_file.len() { return Err(Nus3bankError::InvalidFormat { reason: "PACK size field out of bounds in TOC".to_string() }); }
            new_file[size_pos..size_pos+4].copy_from_slice(&BinaryReader::write_u32_le(new_pack_size));
        }

        // Now iterate sections in order according to TOC and reconstruct stream
        let mut cursor = sections_start;
        for (magic, size) in entries.iter() {
            let magic_str = String::from_utf8_lossy(magic).to_string();
            match &magic[..] {
                b"PACK" => {
                    // Write PACK header and data
                    new_file.extend_from_slice(b"PACK");
                    new_file.extend_from_slice(&BinaryReader::write_u32_le(new_pack_size));
                    new_file.extend_from_slice(&new_pack);
                    // Advance original cursor by old PACK header + data
                    cursor += 8 + *size as usize;
                }
                b"TONE" => {
                    // Copy TONE section as-is from original
                    if cursor + 8 > original.len() { return Err(Nus3bankError::InvalidFormat { reason: format!("TONE header out of bounds at 0x{:X}", cursor) }); }
                    let tone_size = read_u32_le(&original, cursor + 4)? as usize;
                    if cursor + 8 + tone_size > original.len() { return Err(Nus3bankError::InvalidFormat { reason: "TONE body out of bounds".to_string() }); }
                    // Record where TONE will start in the new buffer before copying
                    let tone_new_start = new_file.len();
                    // Copy header+body
                    new_file.extend_from_slice(&original[cursor..cursor + 8 + tone_size]);

                    // Patch each track's packOffset/size inside TONE in-place in new buffer
                    for t in &sorted_tracks {
                        if t.metadata_size <= 0x0c { continue; }
                        if let Some((_, new_off, new_sz)) = new_pack_mapping.iter().find(|(id, _, _)| *id == t.numeric_id) {
                            // Find pack fields absolute position in ORIGINAL file
                            if let Some(pos_orig) = Self::pack_fields_position(&original, t.metadata_offset as usize) {
                                // Map to NEW file using the delta between new TONE start and original TONE start
                                let delta = tone_new_start as isize - cursor as isize;
                                let pos_new = (pos_orig as isize + delta) as usize;
                                if pos_new + 8 <= new_file.len() {
                                    new_file[pos_new..pos_new+4].copy_from_slice(&BinaryReader::write_u32_le(*new_off));
                                    new_file[pos_new+4..pos_new+8].copy_from_slice(&BinaryReader::write_u32_le(*new_sz));
                                }
                            }
                        }
                    }
                    cursor += 8 + tone_size;
                }
                _ => {
                    // Copy unknown/other section as-is
                    if cursor + 8 > original.len() { return Err(Nus3bankError::InvalidFormat { reason: format!("Section '{}' header out of bounds at 0x{:X}", magic_str, cursor) }); }
                    let sec_size = read_u32_le(&original, cursor + 4)? as usize;
                    if cursor + 8 + sec_size > original.len() { return Err(Nus3bankError::InvalidFormat { reason: format!("Section '{}' body out of bounds", magic_str) }); }
                    new_file.extend_from_slice(&original[cursor..cursor + 8 + sec_size]);
                    cursor += 8 + sec_size;
                }
            }
        }

        // Update total size (exclude first 8 bytes per format)
        let total_size_le = BinaryReader::write_u32_le(new_file.len() as u32 - 8);
        new_file[4..8].copy_from_slice(&total_size_le);

        // Persist
        fs::write(path, new_file)?;
        Ok(())
    }

    /// Compute absolute position of packOffset field within a track's metadata blob.
    /// Returns byte index into the file where the 4-byte packOffset is stored.
    fn pack_fields_position(file_bytes: &[u8], metadata_offset: usize) -> Option<usize> {
        if metadata_offset + 8 >= file_bytes.len() { return None; }
        let mut pos = metadata_offset + 6;

        // Emulate Python logic precisely:
        // reader at (offset + 6), read one byte (advance by 1), then seek +5 or +1
        let temp_byte = *file_bytes.get(pos)?;
        pos = pos.checked_add(1)?; // account for the consumed temp_byte
        if temp_byte > 9 || temp_byte == 0 { pos = pos.checked_add(5)?; } else { pos = pos.checked_add(1)?; }

        let string_size = *file_bytes.get(pos)? as usize;
        pos += 1;

        // name (string_size-1) + null terminator
        pos = pos.checked_add(string_size.saturating_sub(1))?;
        pos = pos.checked_add(1)?;

        // padding alignment branch from Python implementation
        let padding = (string_size + 1) % 4;
        if padding == 0 { pos = pos.checked_add(4)?; } else { pos = pos.checked_add((4 - padding + 4) as usize)?; }

        // unknown 4 bytes before packOffset
        pos = pos.checked_add(4)?;

        // pos now points to packOffset
        if pos + 8 <= file_bytes.len() { Some(pos) } else { None }
    }

    /// If file uses BANKTOC, find the position of PACK entry size within the TOC list.
    fn banktoc_pack_size_position(file_bytes: &[u8]) -> Option<usize> {
        if file_bytes.len() < 24 { return None; }

        // Check for 'NUS3' and 'BANK' 'TOC '
        if &file_bytes[0..4] != b"NUS3" { return None; }
        if &file_bytes[8..12] != b"BANK" { return None; }
        if &file_bytes[12..16] != b"TOC " { return None; }

        // toc_size and entry_count
        let entry_count = u32::from_le_bytes([
            file_bytes[20], file_bytes[21], file_bytes[22], file_bytes[23]
        ]) as usize;

        // Entries start at offset 24; each entry is 8 bytes: [magic(4)][size(4)]
        let mut off = 24usize;
        for _ in 0..entry_count {
            if off + 8 > file_bytes.len() { return None; }
            let magic = &file_bytes[off..off + 4];
            if magic == b"PACK" {
                return Some(off + 4);
            }
            off += 8;
        }
        None
    }
}
