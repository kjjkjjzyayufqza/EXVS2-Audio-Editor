use std::fs;
use std::io::{Write, Cursor};
use super::{
    error::Nus3bankError,
    structures::{Nus3bankFile, AudioTrack},
    binary_utils::BinaryReader,
};

/// NUS3BANK file writer
pub struct Nus3bankWriter;

impl Nus3bankWriter {
    /// Write a NUS3BANK file to disk
    pub fn write_file<P: AsRef<std::path::Path>>(file: &Nus3bankFile, path: P) -> Result<(), Nus3bankError> {
        let mut output = Vec::new();
        Self::write_to_buffer(file, &mut output)?;
        
        fs::write(path, output)?;
        Ok(())
    }
    
    /// Write NUS3BANK data to a buffer
    pub fn write_to_buffer(file: &Nus3bankFile, output: &mut Vec<u8>) -> Result<(), Nus3bankError> {
        let mut cursor = Cursor::new(output);
        
        // Write main header
        cursor.write_all(b"NUS3")?;
        
        // Calculate and write total size (placeholder, will be updated)
        let total_size_pos = cursor.position();
        cursor.write_all(&[0u8; 4])?;
        
        // Write PROP section (minimal implementation)
        Self::write_prop_section(&mut cursor)?;
        
        // Write BINF section
        Self::write_binf_section(&mut cursor, &file.bank_info)?;
        
        // Write TONE section
        Self::write_tone_section(&mut cursor, &file.tracks)?;
        
        // Write PACK section
        Self::write_pack_section(&mut cursor, &file.tracks)?;
        
        // Update total size
        let total_size = cursor.position() as u32;
        let buffer = cursor.into_inner();
        
        // Write the total size at the correct position
        let size_bytes = BinaryReader::write_u32_le(total_size - 8); // Exclude NUS3 header
        buffer[total_size_pos as usize..total_size_pos as usize + 4].copy_from_slice(&size_bytes);
        
        Ok(())
    }
    
    /// Write PROP section (minimal implementation)
    fn write_prop_section<W: Write>(writer: &mut W) -> Result<(), Nus3bankError> {
        writer.write_all(b"PROP")?;
        writer.write_all(&BinaryReader::write_u32_le(8))?; // Section size
        writer.write_all(&[0u8; 8])?; // Minimal PROP data
        Ok(())
    }
    
    /// Write BINF section
    fn write_binf_section<W: Write>(writer: &mut W, bank_info: &super::structures::BankInfo) -> Result<(), Nus3bankError> {
        writer.write_all(b"BINF")?;
        
        // Calculate section size
        let string_data = BinaryReader::write_padded_string(&bank_info.bank_string);
        let section_size = 12 + string_data.len() as u32; // 3 u32s + string
        
        writer.write_all(&BinaryReader::write_u32_le(section_size))?;
        writer.write_all(&BinaryReader::write_u32_le(0))?; // Unknown1
        writer.write_all(&BinaryReader::write_u32_le(bank_info.bank_id))?;
        writer.write_all(&BinaryReader::write_u32_le(bank_info.bank_string.len() as u32))?;
        writer.write_all(&string_data)?;
        
        Ok(())
    }
    
    /// Write TONE section
    fn write_tone_section<W: Write>(writer: &mut W, tracks: &[AudioTrack]) -> Result<(), Nus3bankError> {
        writer.write_all(b"TONE")?;
        
        // Calculate section size
        let mut section_size = 8; // 2 u32s (unknown1 + track_count)
        for track in tracks {
            section_size += 16; // 4 u32s per track (id, name_length, size, pack_offset)
            let name_data = BinaryReader::write_padded_string(&track.name);
            section_size += name_data.len() as u32;
        }
        
        writer.write_all(&BinaryReader::write_u32_le(section_size))?;
        writer.write_all(&BinaryReader::write_u32_le(0))?; // Unknown1
        writer.write_all(&BinaryReader::write_u32_le(tracks.len() as u32))?;
        
        // Write tracks with recalculated offsets
        let mut current_pack_offset = 0u32;
        
        for track in tracks {
            writer.write_all(&BinaryReader::write_u32_le(track.numeric_id))?;
            writer.write_all(&BinaryReader::write_u32_le(track.name.len() as u32))?;
            
            let name_data = BinaryReader::write_padded_string(&track.name);
            writer.write_all(&name_data)?;
            
            writer.write_all(&BinaryReader::write_u32_le(track.size))?;
            writer.write_all(&BinaryReader::write_u32_le(current_pack_offset))?;
            
            // Update offset for next track
            current_pack_offset += track.size;
            // Add padding to align to 4 bytes
            let padding = BinaryReader::calculate_padding(track.size as usize);
            current_pack_offset += padding as u32;
        }
        
        Ok(())
    }
    
    /// Write PACK section
    fn write_pack_section<W: Write>(writer: &mut W, tracks: &[AudioTrack]) -> Result<(), Nus3bankError> {
        writer.write_all(b"PACK")?;
        
        // Calculate total PACK size
        let mut pack_size = 0u32;
        for track in tracks {
            pack_size += track.size;
            // Add padding
            let padding = BinaryReader::calculate_padding(track.size as usize);
            pack_size += padding as u32;
        }
        
        writer.write_all(&BinaryReader::write_u32_le(pack_size))?;
        
        // Write audio data for each track
        for track in tracks {
            if let Some(audio_data) = &track.audio_data {
                writer.write_all(audio_data)?;
                
                // Add padding
                let padding = BinaryReader::calculate_padding(audio_data.len());
                if padding > 0 {
                    writer.write_all(&vec![0u8; padding])?;
                }
            } else {
                return Err(Nus3bankError::InvalidFormat {
                    reason: format!("Track {} has no audio data", track.hex_id)
                });
            }
        }
        
        Ok(())
    }
}
