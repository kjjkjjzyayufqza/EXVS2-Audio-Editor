use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use super::{
    error::Nus3bankError,
    structures::{Nus3bankFile, BankInfo, AudioTrack, AudioFormat, SectionOffsets},
    binary_utils::BinaryReader,
};

/// NUS3BANK file parser
pub struct Nus3bankParser;

impl Nus3bankParser {
    /// Parse a NUS3BANK file from path
    pub fn parse_file<P: AsRef<std::path::Path>>(path: P) -> Result<Nus3bankFile, Nus3bankError> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        println!("Attempting to parse NUS3BANK file: {}", path_str);
        
        let file = File::open(&path).map_err(|e| {
            eprintln!("Failed to open file '{}': {}", path_str, e);
            Nus3bankError::Io(e)
        })?;
        
        let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);
        println!("File size: {} bytes", file_size);
        
        if file_size < 8 {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("File too small: {} bytes (minimum 8 bytes required)", file_size)
            });
        }
        
        let mut reader = BufReader::new(file);
        
        // Check if file is compressed (zlib header)
        let compressed = BinaryReader::is_compressed(&mut reader)?;
        
        if compressed {
            // Skip compressed files as per requirements
            return Err(Nus3bankError::UnsupportedCompression);
        }
        
        Self::parse_uncompressed(reader, path_str)
    }
    
    /// Parse uncompressed NUS3BANK data
    fn parse_uncompressed<R: Read + Seek>(mut reader: R, file_path: String) -> Result<Nus3bankFile, Nus3bankError> {
        // Debug: show hex dump of file header
        if let Err(e) = BinaryReader::peek_hex_dump(&mut reader, 128, "File header") {
            eprintln!("Warning: Failed to create file header hex dump: {}", e);
        }
        
        // Validate main header
        println!("Validating NUS3 header...");
        BinaryReader::assert_magic(&mut reader, b"NUS3")?;
        let total_size = BinaryReader::read_u32_le(&mut reader)?;
        println!("Total file size from header: {} bytes", total_size);
        
        // Parse sections
        let mut bank_info = None;
        let mut tracks = Vec::new();
        let mut section_offsets = SectionOffsets::default();
        let mut sections_found = Vec::new();
        
        // Check if file has BANKTOC structure
        let first_section = match BinaryReader::read_section_magic(&mut reader) {
            Ok(magic) => magic,
            Err(e) => return Err(e),
        };
        
        if &first_section == b"BANK" {
            // This is a BANKTOC structure - read TOC first then process sections
            Self::parse_banktoc_structure(&mut reader, &mut bank_info, &mut tracks, &mut section_offsets, &mut sections_found)?;
        } else {
            // Standard section structure - reset position and read normally
            reader.seek(SeekFrom::Current(-4))?;
            Self::parse_standard_sections(&mut reader, &mut bank_info, &mut tracks, &mut section_offsets, &mut sections_found)?;
        }
        
        println!("Sections found: {:?}", sections_found);
        
        let bank_info = bank_info.ok_or_else(|| Nus3bankError::SectionValidation {
            section: "BINF section not found".to_string()
        })?;
        
        let mut final_bank_info = bank_info;
        final_bank_info.section_offsets = section_offsets;
        final_bank_info.total_size = total_size;
        final_bank_info.track_count = tracks.len() as u32;
        
        println!("Successfully parsed {} tracks", tracks.len());
        
        Ok(Nus3bankFile {
            bank_info: final_bank_info,
            tracks,
            compressed: false,
            decompressed_path: None,
            file_path,
        })
    }
    
    /// Parse PROP section (properties)
    fn parse_prop_section<R: Read>(reader: &mut R) -> Result<(), Nus3bankError> {
        let section_size = BinaryReader::read_u32_le(reader)?;
        println!("PROP section size: {} bytes", section_size);
        
        // Skip PROP section content for now (not critical for WAV processing)
        if section_size > 0 && section_size < 1_000_000 { // Sanity check
            let mut skip_buf = vec![0u8; section_size as usize];
            match reader.read_exact(&mut skip_buf) {
                Ok(_) => {
                    println!("Successfully skipped PROP section");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to read PROP section: {}", e);
                    Err(Nus3bankError::Io(e))
                }
            }
        } else {
            Err(Nus3bankError::InvalidFormat {
                reason: format!("Invalid PROP section size: {}", section_size)
            })
        }
    }
    
    /// Parse BINF section (bank information)
    fn parse_binf_section<R: Read + Seek>(reader: &mut R) -> Result<BankInfo, Nus3bankError> {
        let section_size = BinaryReader::read_u32_le(reader)?;
        println!("BINF section size: {} bytes", section_size);
        
        // Debug: show hex dump of BINF section start
        if let Err(e) = BinaryReader::peek_hex_dump(reader, 64.min(section_size as usize), "BINF section start") {
            eprintln!("Warning: Failed to create hex dump: {}", e);
        }
        
        // Validate section size before proceeding
        if section_size < 16 || section_size > 10000 {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("Invalid BINF section size: {} bytes (expected 16-10000)", section_size)
            });
        }
        
        let unknown1 = BinaryReader::read_u32_le(reader)?;
        let bank_id = BinaryReader::read_u32_le(reader)?;
        let string_length = BinaryReader::read_u32_le(reader)?;
        
        println!("BINF details - Unknown1: 0x{:08X}, Bank ID: {}, String length: {}", unknown1, bank_id, string_length);
        
        // Calculate maximum allowed string length
        let max_string_length = section_size.saturating_sub(12).min(1000);
        
        // Handle potentially invalid string length by falling back to empty string
        let bank_string = if string_length == 0 || string_length > max_string_length {
            println!("Warning: Invalid string length {}, using empty string", string_length);
            String::new()
        } else {
            match BinaryReader::read_padded_string(reader, string_length as usize) {
                Ok(s) => {
                    println!("Bank string: '{}'", s);
                    
                    // Skip padding
                    let padding = BinaryReader::calculate_padding(string_length as usize);
                    if padding > 0 {
                        let mut padding_buf = vec![0u8; padding];
                        match reader.read_exact(&mut padding_buf) {
                            Ok(_) => println!("Skipped {} bytes of padding", padding),
                            Err(e) => {
                                eprintln!("Failed to skip BINF padding: {}", e);
                                // Don't fail here, just log the error
                                println!("Continuing despite padding skip error");
                            }
                        }
                    }
                    s
                }
                Err(e) => {
                    eprintln!("Failed to read bank string, using fallback: {}", e);
                    String::from("unknown_bank")
                }
            }
        };
        
        Ok(BankInfo {
            bank_id,
            bank_string,
            total_size: 0, // Will be set by caller
            track_count: 0, // Will be set by caller
            section_offsets: SectionOffsets::default(),
        })
    }
    
    /// Parse BINF section with known size from BANKTOC
    fn parse_binf_section_with_size<R: Read + Seek>(reader: &mut R, expected_size: u32) -> Result<BankInfo, Nus3bankError> {
        let section_size = BinaryReader::read_u32_le(reader)?;
        println!("BINF section size: {} bytes (expected from TOC: {})", section_size, expected_size);
        
        // Verify section size matches TOC
        if section_size != expected_size {
            eprintln!("Warning: BINF section size mismatch. Using TOC size: {}", expected_size);
        }
        
        // Use the expected size from TOC for safety
        let size_to_use = expected_size;
        
        // Debug: show hex dump of BINF section start
        if let Err(e) = BinaryReader::peek_hex_dump(reader, 64.min(size_to_use as usize), "BINF section start") {
            eprintln!("Warning: Failed to create hex dump: {}", e);
        }
        
        // Validate section size before proceeding
        if size_to_use < 16 || size_to_use > 10000 {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("Invalid BINF section size: {} bytes (expected 16-10000)", size_to_use)
            });
        }
        
        let unknown1 = BinaryReader::read_u32_le(reader)?;
        let bank_id = BinaryReader::read_u32_le(reader)?;
        let string_length = BinaryReader::read_u32_le(reader)?;
        
        println!("BINF details - Unknown1: 0x{:08X}, Bank ID: {}, String length: {}", unknown1, bank_id, string_length);
        
        // Calculate maximum allowed string length
        let max_string_length = size_to_use.saturating_sub(16).min(1000); // 16 bytes for header + size + unknown1 + bank_id + string_length
        
        // Handle potentially invalid string length by falling back to empty string
        let bank_string = if string_length == 0 || string_length > max_string_length {
            println!("Warning: Invalid string length {}, using empty string", string_length);
            // Skip remaining bytes in BINF section
            let remaining_bytes = size_to_use.saturating_sub(16);
            if remaining_bytes > 0 {
                let mut skip_buf = vec![0u8; remaining_bytes as usize];
                match reader.read_exact(&mut skip_buf) {
                    Ok(_) => println!("Skipped {} remaining bytes in BINF section", remaining_bytes),
                    Err(e) => eprintln!("Failed to skip remaining BINF bytes: {}", e),
                }
            }
            String::new()
        } else {
            match BinaryReader::read_padded_string(reader, string_length as usize) {
                Ok(s) => {
                    println!("Bank string: '{}'", s);
                    
                    // Skip padding
                    let padding = BinaryReader::calculate_padding(string_length as usize);
                    if padding > 0 {
                        let mut padding_buf = vec![0u8; padding];
                        match reader.read_exact(&mut padding_buf) {
                            Ok(_) => println!("Skipped {} bytes of padding", padding),
                            Err(e) => {
                                eprintln!("Failed to skip BINF padding: {}", e);
                                // Don't fail here, just log the error
                                println!("Continuing despite padding skip error");
                            }
                        }
                    }
                    s
                }
                Err(e) => {
                    eprintln!("Failed to read bank string, using fallback: {}", e);
                    String::from("unknown_bank")
                }
            }
        };
        
        Ok(BankInfo {
            bank_id,
            bank_string,
            total_size: 0, // Will be set by caller
            track_count: 0, // Will be set by caller
            section_offsets: SectionOffsets::default(),
        })
    }
    
    /// Safe BINF section parser with enhanced error recovery
    fn parse_binf_section_safe<R: Read + Seek>(reader: &mut R) -> Result<BankInfo, Nus3bankError> {
        match Self::parse_binf_section(reader) {
            Ok(bank_info) => Ok(bank_info),
            Err(Nus3bankError::InvalidFormat { reason }) => {
                eprintln!("Standard BINF parsing failed: {}", reason);
                eprintln!("Attempting alternative BINF parsing strategy...");
                
                // Try alternative parsing with minimal structure
                Ok(BankInfo {
                    bank_id: 0,
                    bank_string: String::from("recovered_bank"),
                    total_size: 0,
                    track_count: 0,
                    section_offsets: SectionOffsets::default(),
                })
            }
            Err(e) => Err(e),
        }
    }
    
    /// Parse TONE section (track metadata)
    fn parse_tone_section<R: Read>(reader: &mut R) -> Result<Vec<AudioTrack>, Nus3bankError> {
        let section_size = BinaryReader::read_u32_le(reader)?;
        println!("TONE section size: {} bytes", section_size);
        
        let _unknown1 = BinaryReader::read_u32_le(reader)?;
        let track_count = BinaryReader::read_u32_le(reader)?;
        
        println!("Number of tracks: {}", track_count);
        
        if track_count > 1000 { // Sanity check
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("Invalid track count: {}", track_count)
            });
        }
        
        let mut tracks = Vec::new();
        
        for i in 0..track_count {
            let numeric_id = BinaryReader::read_u32_le(reader)?;
            let hex_id = format!("0x{:x}", numeric_id);
            
            let name_length = BinaryReader::read_u32_le(reader)?;
            
            if name_length > 1000 { // Sanity check
                return Err(Nus3bankError::InvalidFormat {
                    reason: format!("Invalid track name length: {}", name_length)
                });
            }
            
            let name = BinaryReader::read_padded_string(reader, name_length as usize)?;
            
            // Skip name padding
            let padding = BinaryReader::calculate_padding(name_length as usize);
            if padding > 0 {
                let mut padding_buf = vec![0u8; padding];
                match reader.read_exact(&mut padding_buf) {
                    Ok(_) => {},
                    Err(e) => {
                        eprintln!("Failed to skip track name padding for track {}: {}", i, e);
                        return Err(Nus3bankError::Io(e));
                    }
                }
            }
            
            let size = BinaryReader::read_u32_le(reader)?;
            let pack_offset = BinaryReader::read_u32_le(reader)?;
            
            println!("Track {}: ID={}, name='{}', size={}, offset={}", i, hex_id, name, size, pack_offset);
            
            tracks.push(AudioTrack {
                index: i as usize,
                hex_id,
                numeric_id,
                name,
                pack_offset,
                size,
                metadata_offset: 0, // Position tracking simplified
                metadata_size: 16 + name_length + padding as u32, // Approximate
                audio_data: None,
                audio_format: AudioFormat::Unknown, // Will be detected when loading data
            });
        }
        
        Ok(tracks)
    }
    
    /// Parse PACK section (audio data)
    fn parse_pack_section<R: Read>(reader: &mut R, tracks: &mut Vec<AudioTrack>) -> Result<(), Nus3bankError> {
        let section_size = BinaryReader::read_u32_le(reader)?;
        println!("PACK section size: {} bytes", section_size);
        
        if section_size > 100_000_000 { // Sanity check: max 100MB
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("PACK section too large: {} bytes", section_size)
            });
        }
        
        // Read entire PACK section into memory
        let mut pack_data = vec![0u8; section_size as usize];
        match reader.read_exact(&mut pack_data) {
            Ok(_) => println!("Successfully read PACK section"),
            Err(e) => {
                eprintln!("Failed to read PACK section: {}", e);
                return Err(Nus3bankError::Io(e));
            }
        }
        
        // Load audio data for each track
        for track in tracks.iter_mut() {
            if track.pack_offset + track.size <= section_size {
                let start = track.pack_offset as usize;
                let end = start + track.size as usize;
                track.audio_data = Some(pack_data[start..end].to_vec());
                
                // Detect format (WAV only as per requirements)
                if let Some(data) = &track.audio_data {
                    if data.starts_with(b"RIFF") {
                        track.audio_format = AudioFormat::Wav;
                        println!("Track '{}': WAV format detected", track.name);
                    } else {
                        println!("Track '{}': Unknown format (first 4 bytes: {:02X?})", track.name, &data[..4.min(data.len())]);
                    }
                }
            } else {
                eprintln!("Track '{}': Invalid offset/size combination (offset={}, size={}, section_size={})", 
                          track.name, track.pack_offset, track.size, section_size);
            }
        }
        
        Ok(())
    }
    
    /// Parse BANKTOC structure (with table of contents)
    fn parse_banktoc_structure<R: Read + Seek>(
        reader: &mut R,
        bank_info: &mut Option<BankInfo>,
        tracks: &mut Vec<AudioTrack>,
        section_offsets: &mut SectionOffsets,
        sections_found: &mut Vec<String>
    ) -> Result<(), Nus3bankError> {
        // Read "TOC " part
        let mut toc_magic = [0u8; 4];
        reader.read_exact(&mut toc_magic)?;
        if &toc_magic != b"TOC " {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("Expected 'TOC ' after 'BANK', found {:?}", String::from_utf8_lossy(&toc_magic))
            });
        }
        
        println!("Found BANKTOC structure");
        sections_found.push("BANKTOC".to_string());
        
        // Read TOC size
        let toc_size = BinaryReader::read_u32_le(reader)?;
        println!("TOC size: {} bytes", toc_size);
        
        // Read number of entries
        let entry_count = BinaryReader::read_u32_le(reader)?;
        println!("TOC entries: {}", entry_count);
        
        if entry_count > 100 { // Sanity check
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("Too many TOC entries: {}", entry_count)
            });
        }
        
        // Read TOC entries (each entry is magic + size)
        let mut toc_entries = Vec::new();
        for i in 0..entry_count {
            let mut magic = [0u8; 4];
            reader.read_exact(&mut magic)?;
            let size = BinaryReader::read_u32_le(reader)?;
            let section_name = String::from_utf8_lossy(&magic).to_string();
            println!("TOC entry {}: {} -> {} bytes", i, section_name, size);
            toc_entries.push((magic, size));
        }
        
        // Now process actual sections in order
        for (magic, expected_size) in toc_entries {
            let section_name = String::from_utf8_lossy(&magic).to_string();
            sections_found.push(section_name.clone());
            
            match &magic[..] {
                b"PROP" => {
                    let current_pos = BinaryReader::get_current_position(reader)?;
                    section_offsets.prop_offset = current_pos;
                    let actual_magic = BinaryReader::read_section_magic(reader)?;
                    if actual_magic != magic {
                        eprintln!("Warning: Section magic mismatch for PROP");
                    }
                    Self::parse_prop_section(reader)?;
                },
                b"BINF" => {
                    let current_pos = BinaryReader::get_current_position(reader)?;
                    section_offsets.binf_offset = current_pos;
                    let actual_magic = BinaryReader::read_section_magic(reader)?;
                    if actual_magic != magic {
                        eprintln!("Warning: Section magic mismatch for BINF");
                    }
                    *bank_info = Some(Self::parse_binf_section_with_size(reader, expected_size)?);
                },
                b"TONE" => {
                    let current_pos = BinaryReader::get_current_position(reader)?;
                    section_offsets.tone_offset = current_pos;
                    let actual_magic = BinaryReader::read_section_magic(reader)?;
                    if actual_magic != magic {
                        eprintln!("Warning: Section magic mismatch for TONE");
                    }
                    *tracks = Self::parse_tone_section(reader)?;
                },
                b"PACK" => {
                    let current_pos = BinaryReader::get_current_position(reader)?;
                    section_offsets.pack_offset = current_pos;
                    let actual_magic = BinaryReader::read_section_magic(reader)?;
                    if actual_magic != magic {
                        eprintln!("Warning: Section magic mismatch for PACK");
                    }
                    Self::parse_pack_section(reader, tracks)?;
                },
                _ => {
                    // Skip unknown sections
                    println!("Skipping unknown section '{}' of size {} bytes", section_name, expected_size);
                    let actual_magic = BinaryReader::read_section_magic(reader)?;
                    if actual_magic != magic {
                        eprintln!("Warning: Section magic mismatch for {}", section_name);
                    }
                    let actual_size = BinaryReader::read_u32_le(reader)?;
                    if actual_size != expected_size {
                        eprintln!("Warning: Section size mismatch for {}: expected {}, got {}", section_name, expected_size, actual_size);
                    }
                    if actual_size > 0 && actual_size < 10_000_000 { // Increased limit for large sections
                        let mut skip_buf = vec![0u8; actual_size as usize];
                        reader.read_exact(&mut skip_buf)?;
                    } else {
                        eprintln!("Invalid section size for '{}': {}", section_name, actual_size);
                        break;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Parse standard sections (no BANKTOC)
    fn parse_standard_sections<R: Read + Seek>(
        reader: &mut R,
        bank_info: &mut Option<BankInfo>,
        tracks: &mut Vec<AudioTrack>,
        section_offsets: &mut SectionOffsets,
        sections_found: &mut Vec<String>
    ) -> Result<(), Nus3bankError> {
        // Read sections sequentially
        loop {
            // Try to read section magic, break if EOF
            let section_magic = match BinaryReader::read_section_magic(reader) {
                Ok(magic) => magic,
                Err(_) => {
                    println!("Reached end of file or error reading section magic");
                    break; // End of file or no more sections
                }
            };
            
            let section_name = String::from_utf8_lossy(&section_magic).to_string();
            sections_found.push(section_name.clone());
            println!("Processing section: {}", section_name);
            
            match &section_magic[..] {
                b"PROP" => {
                    let current_pos = BinaryReader::get_current_position(reader)? - 4;
                    section_offsets.prop_offset = current_pos;
                    Self::parse_prop_section(reader)?;
                },
                b"BINF" => {
                    let current_pos = BinaryReader::get_current_position(reader)? - 4;
                    section_offsets.binf_offset = current_pos;
                    *bank_info = Some(Self::parse_binf_section_safe(reader)?);
                },
                b"TONE" => {
                    let current_pos = BinaryReader::get_current_position(reader)? - 4;
                    section_offsets.tone_offset = current_pos;
                    *tracks = Self::parse_tone_section(reader)?;
                },
                b"PACK" => {
                    let current_pos = BinaryReader::get_current_position(reader)? - 4;
                    section_offsets.pack_offset = current_pos;
                    Self::parse_pack_section(reader, tracks)?;
                },
                _ => {
                    // Skip unknown sections with better error handling
                    match BinaryReader::read_u32_le(reader) {
                        Ok(section_size) => {
                            println!("Skipping unknown section '{}' of size {} bytes", section_name, section_size);
                            if section_size > 0 && section_size < 10_000_000 { // Increased sanity check limit
                                let mut skip_buf = vec![0u8; section_size as usize];
                                match reader.read_exact(&mut skip_buf) {
                                    Ok(_) => {},
                                    Err(e) => {
                                        eprintln!("Failed to skip section '{}': {}", section_name, e);
                                        return Err(Nus3bankError::Io(e));
                                    }
                                }
                            } else {
                                eprintln!("Invalid section size for '{}': {}", section_name, section_size);
                                break;
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to read section size for '{}': {}", section_name, e);
                            break;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
}
