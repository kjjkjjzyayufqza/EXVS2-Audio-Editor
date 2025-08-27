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
        
        let reader = BufReader::new(file);
        
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
        if section_size < 12 || section_size > 10000 {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("Invalid BINF section size: {} bytes (expected 12-10000)", section_size)
            });
        }
        
        let unknown1 = BinaryReader::read_u32_le(reader)?;
        let bank_id = BinaryReader::read_u32_le(reader)?;
        
        println!("BINF details - Unknown1: 0x{:08X}, Bank ID: {}", unknown1, bank_id);
        
        // Read the remaining bytes to find the null-terminated string
        let remaining_bytes = section_size.saturating_sub(8); // 8 = 4 (unknown1) + 4 (bank_id)
        let mut remaining_data = vec![0u8; remaining_bytes as usize];
        match reader.read_exact(&mut remaining_data) {
            Ok(_) => {
                // Find null terminator to extract string
                let string_end = remaining_data.iter().position(|&b| b == 0).unwrap_or(remaining_data.len());
                let bank_string = String::from_utf8_lossy(&remaining_data[..string_end]).to_string();
                println!("Bank string: '{}'", bank_string);
                
                Ok(BankInfo {
                    bank_id,
                    bank_string,
                    total_size: 0, // Will be set by caller
                    track_count: 0, // Will be set by caller
                    section_offsets: SectionOffsets::default(),
                })
            }
            Err(e) => {
                eprintln!("Failed to read remaining BINF data: {}", e);
                // Return with fallback values
                Ok(BankInfo {
                    bank_id,
                    bank_string: String::from("unknown_bank"),
                    total_size: 0,
                    track_count: 0,
                    section_offsets: SectionOffsets::default(),
                })
            }
        }
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
        if size_to_use < 12 || size_to_use > 10000 {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("Invalid BINF section size: {} bytes (expected 12-10000)", size_to_use)
            });
        }
        
        let unknown1 = BinaryReader::read_u32_le(reader)?;
        let bank_id = BinaryReader::read_u32_le(reader)?;
        
        println!("BINF details - Unknown1: 0x{:08X}, Bank ID: {}", unknown1, bank_id);
        
        // Read the remaining bytes to find the null-terminated string
        let remaining_bytes = size_to_use.saturating_sub(8); // 8 = 4 (unknown1) + 4 (bank_id)
        let mut remaining_data = vec![0u8; remaining_bytes as usize];
        match reader.read_exact(&mut remaining_data) {
            Ok(_) => {
                // Find null terminator to extract string
                let string_end = remaining_data.iter().position(|&b| b == 0).unwrap_or(remaining_data.len());
                let bank_string = String::from_utf8_lossy(&remaining_data[..string_end]).to_string();
                println!("Bank string: '{}'", bank_string);
                
                Ok(BankInfo {
                    bank_id,
                    bank_string,
                    total_size: 0, // Will be set by caller
                    track_count: 0, // Will be set by caller
                    section_offsets: SectionOffsets::default(),
                })
            }
            Err(e) => {
                eprintln!("Failed to read remaining BINF data: {}", e);
                // Return with fallback values
                Ok(BankInfo {
                    bank_id,
                    bank_string: String::from("unknown_bank"),
                    total_size: 0,
                    track_count: 0,
                    section_offsets: SectionOffsets::default(),
                })
            }
        }
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
    fn parse_tone_section<R: Read + Seek>(reader: &mut R) -> Result<Vec<AudioTrack>, Nus3bankError> {
        let section_size = BinaryReader::read_u32_le(reader)?;
        println!("TONE section size: {} bytes", section_size);
        
        let track_count = BinaryReader::read_u32_le(reader)?;
        
        println!("Number of tracks: {}", track_count);
        
        // Basic validation for track count
        if track_count == 0 {
            return Err(Nus3bankError::InvalidFormat {
                reason: "Track count cannot be zero".to_string()
            });
        }
        
        // Reasonable upper limit validation
        if track_count > 100000 {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("Track count {} exceeds reasonable limit of 100,000", track_count)
            });
        }
        
        // Correct validation based on Python implementation:
        // TONE section contains: 4 bytes (section_size) + 4 bytes (track_count) + track_count * 8 bytes (offset+metaSize pairs)
        // The actual track metadata is stored at the offsets, not directly in TONE section
        let min_bytes_needed = 8 + (track_count as u64) * 8; // 8 bytes header + 8 bytes per track pointer
        if min_bytes_needed > section_size as u64 {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("Track count {} requires at least {} bytes for pointer table but section only has {} bytes", 
                    track_count, min_bytes_needed, section_size)
            });
        }

        
        // Store the TONE magic offset for calculating absolute positions
        // After reading section_size (4) and track_count (4), current position = TONE_magic_start + 12
        // We want TONE_magic_start, so subtract 12
        let tone_magic_offset = BinaryReader::get_current_position(reader)? - 12;
        
        // First, read the pointer table (offset + metaSize pairs) as in Python implementation
        let mut track_pointers = Vec::new();
        for i in 0..track_count {
            let relative_offset = BinaryReader::read_u32_le(reader)?;
            let meta_size = BinaryReader::read_u32_le(reader)?;
            // Calculate absolute offset (Python: offset = readu32le(nus3) + toneOffset + 8)
            let absolute_offset = relative_offset + tone_magic_offset + 8;
            track_pointers.push((absolute_offset, meta_size));
            println!("Track {} pointer: relative_offset={}, absolute_offset={}, metaSize={}", 
                     i, relative_offset, absolute_offset, meta_size);
        }
        
        let mut tracks = Vec::new();
        let mut track_index = 0; // Use separate index for tracks array (like Python 'i' variable)
        
        // Now process each track by seeking to its metadata location (following Python implementation)
        for (original_index, (metadata_offset, meta_size)) in track_pointers.iter().enumerate() {
            // Skip tracks with insufficient metadata (as done in Python: if tones[i].metaSize <= 0xc: continue)
            if *meta_size <= 0xc {
                println!("Skipping track {} due to insufficient metaSize: {}", original_index, meta_size);
                continue;
            }
            
            // Seek to the track's metadata location
            reader.seek(SeekFrom::Start(*metadata_offset as u64))?;
            
            // Read track metadata following Python logic
            // Python code: nus3.seek(tones[i].offset+6)
            reader.seek(SeekFrom::Current(6))?;
            
            let temp_byte = BinaryReader::read_u8(reader)?;
            if temp_byte > 9 || temp_byte == 0 {
                reader.seek(SeekFrom::Current(5))?;
            } else {
                reader.seek(SeekFrom::Current(1))?;
            }
            
            let string_size = BinaryReader::read_u8(reader)?;
            
            // Read track name
            let mut name_bytes = vec![0u8; (string_size - 1) as usize];
            reader.read_exact(&mut name_bytes)?;
            let name = String::from_utf8_lossy(&name_bytes).to_string();
            
            // Skip null terminator
            reader.seek(SeekFrom::Current(1))?;
            
            // Display the original pointer table index as the track ID (matches expected hex ids)
            println!("\t0x{:x}:{}", original_index, name);
            
            // Handle padding (Python logic)
            let padding = (string_size as usize + 1) % 4;
            if padding == 0 {
                reader.seek(SeekFrom::Current(4))?;
            } else {
                reader.seek(SeekFrom::Current((4 - padding + 4) as i64))?;
            }
            
            // Read the 4-byte value (usually 8) before packOffset and size
            // This corresponds to the commented assert in Python: assert readu32le(nus3) == 8
            let unknown_value = BinaryReader::read_u32_le(reader)?;
            println!("Track {}: unknown_value before offsets = {}", original_index, unknown_value);
            
            let pack_offset = BinaryReader::read_u32_le(reader)?;
            let size = BinaryReader::read_u32_le(reader)?;
            
            println!("Track {}: pack_offset={}, size={}", original_index, pack_offset, size);
            
            tracks.push(AudioTrack {
                index: track_index, // Keep sequential index for UI/ordering
                hex_id: format!("0x{:x}", original_index), // Use original pointer index as stable hex ID
                numeric_id: original_index as u32, // Use original pointer index as numeric ID
                name,
                pack_offset,
                size,
                metadata_offset: *metadata_offset,
                metadata_size: *meta_size,
                audio_data: None,
                audio_format: AudioFormat::Unknown,
            });
            
            track_index += 1; // Increment track_index for next valid track
        }
        
        Ok(tracks)
    }
    
    /// Parse PACK section (audio data)
    fn parse_pack_section<R: Read>(reader: &mut R, tracks: &mut Vec<AudioTrack>) -> Result<(), Nus3bankError> {
        // Kept for backward compatibility when TONE is already parsed before PACK
        let pack_data = Self::read_pack_section(reader)?;
        Self::attach_pack_data_to_tracks(&pack_data, tracks)
    }

    /// Read PACK section bytes into memory (data only, excluding the 8-byte header already consumed)
    fn read_pack_section<R: Read>(reader: &mut R) -> Result<Vec<u8>, Nus3bankError> {
        let section_size = BinaryReader::read_u32_le(reader)?;
        println!("PACK section size: {} bytes", section_size);

        if section_size > 100_000_000 {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("PACK section too large: {} bytes", section_size),
            });
        }

        let mut pack_data = vec![0u8; section_size as usize];
        match reader.read_exact(&mut pack_data) {
            Ok(_) => println!("Successfully read PACK section"),
            Err(e) => {
                eprintln!("Failed to read PACK section: {}", e);
                return Err(Nus3bankError::Io(e));
            }
        }

        Ok(pack_data)
    }

    /// Attach PACK data to parsed tracks
    fn attach_pack_data_to_tracks(pack_data: &[u8], tracks: &mut Vec<AudioTrack>) -> Result<(), Nus3bankError> {
        let section_size = pack_data.len() as u32;
        for track in tracks.iter_mut() {
            if track.pack_offset < 0xffff_ffff {
                if track.pack_offset + track.size <= section_size {
                    let start = track.pack_offset as usize;
                    let end = start + track.size as usize;

                    if track.size > 0 && track.size <= section_size {
                        let audio_data = pack_data[start..end].to_vec();

                        if audio_data.starts_with(b"RIFF") {
                            track.audio_format = AudioFormat::Wav;
                            println!(
                                "Track '{}' ({}): WAV format detected, size: {} bytes",
                                track.name, track.hex_id, track.size
                            );
                        } else {
                            track.audio_format = AudioFormat::Unknown;
                            println!(
                                "Track '{}' ({}): Unknown format (first 4 bytes: {:02X?}), size: {} bytes",
                                track.name,
                                track.hex_id,
                                &audio_data[..4.min(audio_data.len())],
                                track.size
                            );
                        }

                        track.audio_data = Some(audio_data);
                        println!(
                            "Track '{}' ({}): Audio data loaded successfully, size: {} bytes",
                            track.name, track.hex_id, track.size
                        );
                    } else {
                        eprintln!(
                            "Track '{}' ({}): Invalid size: {} bytes",
                            track.name, track.hex_id, track.size
                        );
                        track.size = 0;
                        track.audio_data = None;
                    }
                } else {
                    eprintln!(
                        "Track '{}' ({}): Invalid offset/size combination (offset={}, size={}, section_size={})",
                        track.name,
                        track.hex_id,
                        track.pack_offset,
                        track.size,
                        section_size
                    );
                    track.size = 0;
                    track.audio_data = None;
                }
            } else {
                println!(
                    "Track '{}' ({}): Skipping due to invalid pack_offset: 0x{:x}",
                    track.name, track.hex_id, track.pack_offset
                );
                track.size = 0;
                track.audio_data = None;
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
        // Defer PACK data until tracks are parsed to ensure audio_data can be attached regardless of section order
        let mut deferred_pack_data: Option<Vec<u8>> = None;
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
            
            // In BANKTOC structure, sections are laid out consecutively:
            // [magic][size][data][magic][size][data]...
            // We need to verify the magic matches, then read the size
            let current_pos = BinaryReader::get_current_position(reader)?;
            let actual_magic = BinaryReader::read_section_magic(reader)?;
            if actual_magic != magic {
                eprintln!("Warning: Section magic mismatch for {}: expected {:?}, got {:?}", 
                         section_name, String::from_utf8_lossy(&magic), String::from_utf8_lossy(&actual_magic));
            }
            
            match &magic[..] {
                b"PROP" => {
                    section_offsets.prop_offset = current_pos;
                    Self::parse_prop_section(reader)?;
                },
                b"BINF" => {
                    section_offsets.binf_offset = current_pos;
                    *bank_info = Some(Self::parse_binf_section_with_size(reader, expected_size)?);
                },
                b"TONE" => {
                    section_offsets.tone_offset = current_pos;
                    *tracks = Self::parse_tone_section(reader)?;
                    // If PACK data was read earlier, attach it now
                    if let Some(pack_data) = deferred_pack_data.take() {
                        Self::attach_pack_data_to_tracks(&pack_data, tracks)?;
                    }
                },
                b"PACK" => {
                    section_offsets.pack_offset = current_pos;
                    // Read PACK data now, but attach to tracks only when they are available
                    let pack_data = Self::read_pack_section(reader)?;
                    if tracks.is_empty() {
                        deferred_pack_data = Some(pack_data);
                    } else {
                        Self::attach_pack_data_to_tracks(&pack_data, tracks)?;
                    }
                },
                _ => {
                    // Skip unknown sections
                    println!("Skipping unknown section '{}' of size {} bytes", section_name, expected_size);
                    let actual_size = BinaryReader::read_u32_le(reader)?;
                    if actual_size != expected_size {
                        eprintln!("Warning: Section size mismatch for {}: expected {}, got {}", section_name, expected_size, actual_size);
                        // Use the actual size from the file, not the TOC
                    }
                    let size_to_skip = actual_size.min(expected_size); // Use the smaller size for safety
                    if size_to_skip > 0 && size_to_skip < 10_000_000 { // Increased limit for large sections
                        let mut skip_buf = vec![0u8; size_to_skip as usize];
                        reader.read_exact(&mut skip_buf)?;
                    } else {
                        eprintln!("Invalid section size for '{}': {}", section_name, size_to_skip);
                        break;
                    }
                }
            }
        }
        // Finalize: if PACK was encountered before TONE, attach now
        if let Some(pack_data) = deferred_pack_data.take() {
            if !tracks.is_empty() {
                Self::attach_pack_data_to_tracks(&pack_data, tracks)?;
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
        // Defer PACK data until tracks are parsed (section order may vary)
        let mut deferred_pack_data: Option<Vec<u8>> = None;
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
                    // If PACK data was read earlier, attach it now
                    if let Some(pack_data) = deferred_pack_data.take() {
                        Self::attach_pack_data_to_tracks(&pack_data, tracks)?;
                    }
                },
                b"PACK" => {
                    let current_pos = BinaryReader::get_current_position(reader)? - 4;
                    section_offsets.pack_offset = current_pos;
                    // Read PACK bytes; attach later if tracks not parsed yet
                    let pack_data = Self::read_pack_section(reader)?;
                    if tracks.is_empty() {
                        deferred_pack_data = Some(pack_data);
                    } else {
                        Self::attach_pack_data_to_tracks(&pack_data, tracks)?;
                    }
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
        // Finalize: if PACK was encountered before TONE, attach now
        if let Some(pack_data) = deferred_pack_data.take() {
            if !tracks.is_empty() {
                Self::attach_pack_data_to_tracks(&pack_data, tracks)?;
            }
        }
        
        Ok(())
    }
}
