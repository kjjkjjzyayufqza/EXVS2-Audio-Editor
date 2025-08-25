# NUS3BANK Rust Implementation Guide

## Overview

This document provides a comprehensive guide for implementing NUS3BANK file system support in Rust for the EXVS2 Audio Editor. The implementation leverages existing codebase patterns from NUS3AUDIO handling and focuses on WAV format audio processing only, avoiding complex compression schemes.

## Current Codebase Analysis

### Existing Structure
The current codebase already handles NUS3AUDIO files through the `nus3audio` crate (version 1.2.0) with the following key components:

- **AudioFileInfo**: Core structure for audio file metadata
- **Nus3audioFile**: Main file handling structure from external crate
- **ReplaceUtils**: In-memory replacement system
- **ExportUtils**: Audio conversion and export functionality
- **Nus3audioFileUtils**: Utility functions for file operations

### Current File Detection Pattern
```rust
// From main_area_filtering.rs
if file_name.to_lowercase().ends_with(".nus3audio")
    || file_name.to_lowercase().ends_with(".nus3bank")
{
    match Nus3audioFile::open(file_name) {
        // Current implementation treats .nus3bank as .nus3audio
    }
}
```

## NUS3BANK Data Structures

### Core Structures

#### 1. Nus3bankFile (Primary Container)
```rust
use std::path::PathBuf;

/// Main structure representing a complete NUS3BANK file
#[derive(Clone, Debug)]
pub struct Nus3bankFile {
    /// Bank metadata information
    pub bank_info: BankInfo,
    /// Collection of audio tracks in the bank
    pub tracks: Vec<AudioTrack>,
    /// Whether the original file was compressed
    pub compressed: bool,
    /// Path to decompressed file (if applicable)
    pub decompressed_path: Option<PathBuf>,
    /// Original file path
    pub file_path: String,
}

impl Nus3bankFile {
    /// Open and parse a NUS3BANK file
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Nus3bankError> {
        // Implementation details in parser module
    }
    
    /// Save the NUS3BANK file to disk
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Nus3bankError> {
        // Implementation details in writer module
    }
    
    /// Get track by hex ID
    pub fn get_track_by_hex_id(&self, hex_id: &str) -> Option<&AudioTrack> {
        self.tracks.iter().find(|t| t.hex_id == hex_id)
    }
    
    /// Replace track data by hex ID
    pub fn replace_track_data(&mut self, hex_id: &str, new_data: Vec<u8>) -> Result<(), Nus3bankError> {
        // Implementation details in replace module
    }
}
```

#### 2. BankInfo (Metadata Container)
```rust
/// Bank-level metadata extracted from BINF section
#[derive(Clone, Debug)]
pub struct BankInfo {
    /// Bank ID (numeric identifier)
    pub bank_id: u32,
    /// Bank string name
    pub bank_string: String,
    /// Total file size
    pub total_size: u32,
    /// Number of tracks in bank
    pub track_count: u32,
    /// Section offsets for reconstruction
    pub section_offsets: SectionOffsets,
}

/// Section offset information for file reconstruction
#[derive(Clone, Debug)]
pub struct SectionOffsets {
    pub prop_offset: u32,
    pub binf_offset: u32,
    pub tone_offset: u32,
    pub pack_offset: u32,
}
```

#### 3. AudioTrack (Individual Track Data)
```rust
/// Individual audio track within a NUS3BANK file
#[derive(Clone, Debug)]
pub struct AudioTrack {
    /// Sequential index (0-based)
    pub index: usize,
    /// Hex ID string representation ("0x0", "0xb2", etc.)
    pub hex_id: String,
    /// Numeric ID value
    pub numeric_id: u32,
    /// Track name
    pub name: String,
    /// Offset within PACK section
    pub pack_offset: u32,
    /// Audio data size in bytes
    pub size: u32,
    /// TONE metadata offset
    pub metadata_offset: u32,
    /// TONE metadata size
    pub metadata_size: u32,
    /// Raw audio data (WAV format only)
    pub audio_data: Option<Vec<u8>>,
    /// Audio format type
    pub audio_format: AudioFormat,
}

/// Supported audio formats (WAV only as per requirements)
#[derive(Clone, Debug)]
pub enum AudioFormat {
    Wav,
    Unknown,
}

impl AudioTrack {
    /// Generate filename for export
    pub fn filename(&self) -> String {
        format!("{}-{}.wav", self.hex_id, self.name)
    }
    
    /// Load audio data from PACK section
    pub fn load_audio_data(&mut self, pack_data: &[u8]) -> Result<(), Nus3bankError> {
        // Extract audio data from PACK section at specified offset
    }
}
```

## Integration with Existing AudioFileInfo

### Extended AudioFileInfo Structure
```rust
/// Enhanced structure to support both NUS3AUDIO and NUS3BANK files
#[derive(Clone, Debug)]
pub struct AudioFileInfo {
    pub name: String,
    pub id: String,
    pub size: usize,
    pub filename: String,
    pub file_type: String,
    // New fields for NUS3BANK support
    pub hex_id: Option<String>,        // Hex ID for NUS3BANK files
    pub bank_info: Option<BankInfo>,   // Bank metadata for NUS3BANK files
    pub is_nus3bank: bool,             // File type indicator
}

impl AudioFileInfo {
    /// Create from NUS3BANK AudioTrack
    pub fn from_nus3bank_track(track: &AudioTrack, bank_info: &BankInfo) -> Self {
        Self {
            name: track.name.clone(),
            id: track.numeric_id.to_string(),
            size: track.size as usize,
            filename: track.filename(),
            file_type: "WAV".to_string(),
            hex_id: Some(track.hex_id.clone()),
            bank_info: Some(bank_info.clone()),
            is_nus3bank: true,
        }
    }
    
    /// Create from existing NUS3AUDIO (backward compatibility)
    pub fn from_nus3audio(audio_file: &nus3audio::AudioFile) -> Self {
        Self {
            name: audio_file.name.clone(),
            id: audio_file.id.to_string(),
            size: audio_file.data.len(),
            filename: audio_file.filename(),
            file_type: detect_audio_format(&audio_file.data),
            hex_id: None,
            bank_info: None,
            is_nus3bank: false,
        }
    }
}
```

## Binary Format Implementation

### Binary Reading Utilities
```rust
use std::io::{Read, Seek, SeekFrom, Result as IoResult};

/// Binary reading utilities for NUS3BANK format
pub struct BinaryReader;

impl BinaryReader {
    /// Read single byte
    pub fn read_u8<R: Read>(reader: &mut R) -> IoResult<u8> {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf)?;
        Ok(buf[0])
    }
    
    /// Read 16-bit little-endian unsigned integer
    pub fn read_u16_le<R: Read>(reader: &mut R) -> IoResult<u16> {
        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }
    
    /// Read 32-bit little-endian unsigned integer
    pub fn read_u32_le<R: Read>(reader: &mut R) -> IoResult<u32> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }
    
    /// Read 32-bit little-endian float
    pub fn read_f32_le<R: Read>(reader: &mut R) -> IoResult<f32> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        Ok(f32::from_le_bytes(buf))
    }
    
    /// Validate magic number
    pub fn assert_magic<R: Read>(reader: &mut R, expected: &[u8]) -> Result<(), Nus3bankError> {
        let mut buffer = vec![0u8; expected.len()];
        reader.read_exact(&mut buffer)?;
        
        if buffer != expected {
            return Err(Nus3bankError::InvalidMagic {
                expected: String::from_utf8_lossy(expected).to_string(),
                found: String::from_utf8_lossy(&buffer).to_string(),
            });
        }
        Ok(())
    }
    
    /// Calculate 4-byte alignment padding
    pub fn calculate_padding(size: usize) -> usize {
        (4 - (size % 4)) % 4
    }
    
    /// Read null-terminated string with padding
    pub fn read_padded_string<R: Read>(reader: &mut R, length: usize) -> IoResult<String> {
        let mut buffer = vec![0u8; length];
        reader.read_exact(&mut buffer)?;
        
        // Find null terminator
        let end = buffer.iter().position(|&b| b == 0).unwrap_or(length);
        Ok(String::from_utf8_lossy(&buffer[..end]).to_string())
    }
}
```

### Error Handling
```rust
use std::fmt;

/// NUS3BANK specific error types
#[derive(Debug)]
pub enum Nus3bankError {
    InvalidMagic { expected: String, found: String },
    SectionValidation { section: String },
    StringAlignment { size: usize, padding: usize },
    Reconstruction { reason: String },
    TrackNotFound { hex_id: String },
    InvalidHexId { hex_id: String },
    Io(std::io::Error),
    Utf8(std::string::FromUtf8Error),
}

impl fmt::Display for Nus3bankError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Nus3bankError::InvalidMagic { expected, found } => {
                write!(f, "Invalid magic number: expected {}, found {}", expected, found)
            }
            Nus3bankError::SectionValidation { section } => {
                write!(f, "Section validation failed: {}", section)
            }
            Nus3bankError::StringAlignment { size, padding } => {
                write!(f, "String alignment error: size {}, padding {}", size, padding)
            }
            Nus3bankError::Reconstruction { reason } => {
                write!(f, "File reconstruction failed: {}", reason)
            }
            Nus3bankError::TrackNotFound { hex_id } => {
                write!(f, "Track not found: {}", hex_id)
            }
            Nus3bankError::InvalidHexId { hex_id } => {
                write!(f, "Invalid hex ID format: {}", hex_id)
            }
            Nus3bankError::Io(err) => write!(f, "IO error: {}", err),
            Nus3bankError::Utf8(err) => write!(f, "UTF-8 conversion error: {}", err),
        }
    }
}

impl std::error::Error for Nus3bankError {}

impl From<std::io::Error> for Nus3bankError {
    fn from(err: std::io::Error) -> Self {
        Nus3bankError::Io(err)
    }
}

impl From<std::string::FromUtf8Error> for Nus3bankError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        Nus3bankError::Utf8(err)
    }
}
```

## File Parser Implementation

### Main Parser Structure
```rust
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};

/// NUS3BANK file parser
pub struct Nus3bankParser;

impl Nus3bankParser {
    /// Parse a NUS3BANK file from path
    pub fn parse_file<P: AsRef<std::path::Path>>(path: P) -> Result<Nus3bankFile, Nus3bankError> {
        let file = File::open(&path)?;
        let mut reader = BufReader::new(file);
        
        // Check if file is compressed (zlib header)
        let mut header = [0u8; 2];
        reader.read_exact(&mut header)?;
        reader.seek(SeekFrom::Start(0))?;
        
        let compressed = header == [0x78, 0x9C] || header == [0x78, 0x01] || header == [0x78, 0xDA];
        
        if compressed {
            // Skip compressed files as per requirements
            return Err(Nus3bankError::SectionValidation { 
                section: "Compressed files not supported in WAV-only mode".to_string() 
            });
        }
        
        Self::parse_uncompressed(reader, path.as_ref().to_string_lossy().to_string())
    }
    
    /// Parse uncompressed NUS3BANK data
    fn parse_uncompressed<R: Read + Seek>(mut reader: R, file_path: String) -> Result<Nus3bankFile, Nus3bankError> {
        // Validate main header
        BinaryReader::assert_magic(&mut reader, b"NUS3")?;
        let total_size = BinaryReader::read_u32_le(&mut reader)?;
        
        // Parse sections
        let mut bank_info = None;
        let mut tracks = Vec::new();
        let mut section_offsets = SectionOffsets {
            prop_offset: 0,
            binf_offset: 0,
            tone_offset: 0,
            pack_offset: 0,
        };
        
        // Read sections sequentially
        loop {
            match Self::read_section_magic(&mut reader) {
                Ok(section_magic) => {
                    match &section_magic[..] {
                        b"PROP" => {
                            let current_pos = Self::get_current_position(&mut reader)?;
                            section_offsets.prop_offset = current_pos - 4;
                            Self::parse_prop_section(&mut reader)?;
                        },
                        b"BINF" => {
                            let current_pos = Self::get_current_position(&mut reader)?;
                            section_offsets.binf_offset = current_pos - 4;
                            bank_info = Some(Self::parse_binf_section(&mut reader)?);
                        },
                        b"TONE" => {
                            let current_pos = Self::get_current_position(&mut reader)?;
                            section_offsets.tone_offset = current_pos - 4;
                            tracks = Self::parse_tone_section(&mut reader)?;
                        },
                        b"PACK" => {
                            let current_pos = Self::get_current_position(&mut reader)?;
                            section_offsets.pack_offset = current_pos - 4;
                            Self::parse_pack_section(&mut reader, &mut tracks)?;
                        },
                        _ => {
                            // Skip unknown sections
                            let section_size = BinaryReader::read_u32_le(&mut reader)?;
                            let mut skip_buf = vec![0u8; section_size as usize];
                            reader.read_exact(&mut skip_buf)?;
                        }
                    }
                },
                Err(_) => break, // End of file or no more sections
            }
        }
        
        let bank_info = bank_info.ok_or_else(|| Nus3bankError::SectionValidation {
            section: "BINF section not found".to_string()
        })?;
        
        let mut final_bank_info = bank_info;
        final_bank_info.section_offsets = section_offsets;
        final_bank_info.total_size = total_size;
        final_bank_info.track_count = tracks.len() as u32;
        
        Ok(Nus3bankFile {
            bank_info: final_bank_info,
            tracks,
            compressed: false,
            decompressed_path: None,
            file_path,
        })
    }
    
    /// Read section magic bytes
    fn read_section_magic<R: Read>(reader: &mut R) -> Result<[u8; 4], Nus3bankError> {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        Ok(magic)
    }
    
    /// Get current position (helper for readers without Seek)
    fn get_current_position<R: Read + Seek>(reader: &mut R) -> Result<u32, Nus3bankError> {
        Ok(reader.stream_position()? as u32)
    }
    
    /// Parse PROP section (properties)
    fn parse_prop_section<R: Read>(reader: &mut R) -> Result<(), Nus3bankError> {
        let section_size = BinaryReader::read_u32_le(reader)?;
        // Skip PROP section content for now (not critical for WAV processing)
        let mut skip_buf = vec![0u8; section_size as usize];
        reader.read_exact(&mut skip_buf)?;
        Ok(())
    }
    
    /// Parse BINF section (bank information)
    fn parse_binf_section<R: Read>(reader: &mut R) -> Result<BankInfo, Nus3bankError> {
        let section_size = BinaryReader::read_u32_le(reader)?;
        let _unknown1 = BinaryReader::read_u32_le(reader)?;
        let bank_id = BinaryReader::read_u32_le(reader)?;
        let string_length = BinaryReader::read_u32_le(reader)?;
        
        let bank_string = BinaryReader::read_padded_string(reader, string_length as usize)?;
        
        // Skip padding
        let padding = BinaryReader::calculate_padding(string_length as usize);
        if padding > 0 {
            let mut padding_buf = vec![0u8; padding];
            reader.read_exact(&mut padding_buf)?;
        }
        
        Ok(BankInfo {
            bank_id,
            bank_string,
            total_size: 0, // Will be set by caller
            track_count: 0, // Will be set by caller
            section_offsets: SectionOffsets {
                prop_offset: 0,
                binf_offset: 0,
                tone_offset: 0,
                pack_offset: 0,
            },
        })
    }
    
    /// Parse TONE section (track metadata)
    fn parse_tone_section<R: Read>(reader: &mut R) -> Result<Vec<AudioTrack>, Nus3bankError> {
        let section_size = BinaryReader::read_u32_le(reader)?;
        
        let _unknown1 = BinaryReader::read_u32_le(reader)?;
        let track_count = BinaryReader::read_u32_le(reader)?;
        
        let mut tracks = Vec::new();
        
        for i in 0..track_count {
            let numeric_id = BinaryReader::read_u32_le(reader)?;
            let hex_id = format!("0x{:x}", numeric_id);
            
            let name_length = BinaryReader::read_u32_le(reader)?;
            let name = BinaryReader::read_padded_string(reader, name_length as usize)?;
            
                    // Skip name padding
        let padding = BinaryReader::calculate_padding(name_length as usize);
        if padding > 0 {
            let mut padding_buf = vec![0u8; padding];
            reader.read_exact(&mut padding_buf)?;
        }
            
            let size = BinaryReader::read_u32_le(reader)?;
            let pack_offset = BinaryReader::read_u32_le(reader)?;
            
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
        
        // Read entire PACK section into memory
        let mut pack_data = vec![0u8; section_size as usize];
        reader.read_exact(&mut pack_data)?;
        
        // Load audio data for each track
        for track in tracks.iter_mut() {
            if track.pack_offset + track.size <= section_size {
                let start = track.pack_offset as usize;
                let end = start + track.size as usize;
                track.audio_data = Some(pack_data[start..end].to_vec());
                
                // Detect format (WAV only as per requirements)
                if track.audio_data.as_ref().unwrap().starts_with(b"RIFF") {
                    track.audio_format = AudioFormat::Wav;
                }
            }
        }
        
        Ok(())
    }
}
```

## Integration with Existing Systems

### File Detection and Loading
```rust
// Extension to main_area_filtering.rs
impl MainAreaFiltering {
    pub fn update_selected_file(&mut self, file_path: Option<String>) {
        // Clear any previously replaced audio data in memory
        ReplaceUtils::clear_replacements();
        
        self.selected_file = file_path;
        self.file_count = None;
        self.audio_files = None;
        self.error_message = None;

        // If file is selected, determine type and load accordingly
        if let Some(file_name) = &self.selected_file {
            if file_name.to_lowercase().ends_with(".nus3audio") {
                self.load_nus3audio_file(file_name);
            } else if file_name.to_lowercase().ends_with(".nus3bank") {
                self.load_nus3bank_file(file_name);
            }
        }
    }
    
    /// Load NUS3AUDIO file (existing implementation)
    fn load_nus3audio_file(&mut self, file_name: &str) {
        match Nus3audioFile::open(file_name) {
            Ok(nus3_file) => {
                self.file_count = Some(nus3_file.files.len());
                let mut audio_files = Vec::new();

                for audio_file in nus3_file.files.iter() {
                    audio_files.push(AudioFileInfo::from_nus3audio(audio_file));
                }

                self.audio_files = Some(audio_files);
            }
            Err(e) => {
                self.error_message = Some(format!("Error loading NUS3AUDIO file: {}", e));
            }
        }
    }
    
    /// Load NUS3BANK file (new implementation)
    fn load_nus3bank_file(&mut self, file_name: &str) {
        match Nus3bankFile::open(file_name) {
            Ok(nus3bank_file) => {
                self.file_count = Some(nus3bank_file.tracks.len());
                let mut audio_files = Vec::new();

                for track in nus3bank_file.tracks.iter() {
                    audio_files.push(AudioFileInfo::from_nus3bank_track(track, &nus3bank_file.bank_info));
                }

                self.audio_files = Some(audio_files);
            }
            Err(e) => {
                self.error_message = Some(format!("Error loading NUS3BANK file: {}", e));
            }
        }
    }
}
```

### Export Utilities Extension
```rust
// Extension to export_utils.rs
impl ExportUtils {
    /// Export NUS3BANK track to WAV
    pub fn export_nus3bank_track(
        file_path: &str,
        hex_id: &str,
        output_dir: &str,
    ) -> Result<String, String> {
        let nus3bank_file = Nus3bankFile::open(file_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;
        
        let track = nus3bank_file.get_track_by_hex_id(hex_id)
            .ok_or_else(|| format!("Track with hex ID {} not found", hex_id))?;
        
        let output_path = format!("{}/{}", output_dir, track.filename());
        
        if let Some(audio_data) = &track.audio_data {
            std::fs::write(&output_path, audio_data)
                .map_err(|e| format!("Failed to write audio file: {}", e))?;
        } else {
            return Err("Audio data not loaded for track".to_string());
        }
        
        Ok(output_path)
    }
    
    /// Batch export all tracks from NUS3BANK
    pub fn export_all_nus3bank_tracks(
        file_path: &str,
        output_dir: &str,
    ) -> Result<Vec<String>, String> {
        let nus3bank_file = Nus3bankFile::open(file_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;
        
        let mut exported_files = Vec::new();
        
        for track in &nus3bank_file.tracks {
            match Self::export_nus3bank_track(file_path, &track.hex_id, output_dir) {
                Ok(path) => exported_files.push(path),
                Err(e) => log::warn!("Failed to export track {}: {}", track.hex_id, e),
            }
        }
        
        Ok(exported_files)
    }
}
```

### Replace Utilities Extension
```rust
// Extension to replace_utils.rs
impl ReplaceUtils {
    /// Replace track in NUS3BANK file
    pub fn replace_nus3bank_track(
        file_path: &str,
        hex_id: &str,
        new_audio_data: Vec<u8>,
    ) -> Result<(), String> {
        // Store replacement in memory (similar to existing pattern)
        let key = format!("{}:{}", hex_id, file_path);
        
        if let Ok(mut map) = REPLACED_AUDIO_DATA.lock() {
            map.insert(key, new_audio_data);
            println!("Stored replacement data for NUS3BANK track: {}", hex_id);
            Ok(())
        } else {
            Err("Failed to acquire lock on replacement data".to_string())
        }
    }
    
    /// Apply replacements to NUS3BANK file and save
    pub fn save_nus3bank_with_replacements(
        original_path: &str,
        output_path: &str,
    ) -> Result<(), String> {
        let mut nus3bank_file = Nus3bankFile::open(original_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;
        
        // Apply all replacements from memory
        if let Ok(map) = REPLACED_AUDIO_DATA.lock() {
            for (key, replacement_data) in map.iter() {
                if key.ends_with(original_path) {
                    let hex_id = key.split(':').next().unwrap_or("");
                    if let Err(e) = nus3bank_file.replace_track_data(hex_id, replacement_data.clone()) {
                        log::warn!("Failed to replace track {}: {}", hex_id, e);
                    }
                }
            }
        }
        
        nus3bank_file.save(output_path)
            .map_err(|e| format!("Failed to save NUS3BANK file: {}", e))?;
        
        Ok(())
    }
}
```

## Audio Player Integration

### Player Component Extension
```rust
// Extension to audio_player_component.rs
impl AudioPlayerComponent {
    /// Load audio from NUS3BANK file
    fn load_nus3bank_audio(
        &mut self,
        file_info: &AudioFileInfo,
        file_path: &str,
    ) -> Result<AudioFile, String> {
        // Check for replacement data first
        let key = format!("{}:{}", file_info.hex_id.as_ref().unwrap_or(&file_info.id), file_path);
        
        let audio_data = if let Ok(map) = REPLACED_AUDIO_DATA.lock() {
            if let Some(replacement_data) = map.get(&key) {
                log::info!("Using replacement audio data for: {}", file_info.name);
                replacement_data.clone()
            } else {
                self.load_original_nus3bank_audio(file_info, file_path)?
            }
        } else {
            self.load_original_nus3bank_audio(file_info, file_path)?
        };
        
        Ok(AudioFile {
            file_path: file_path.to_string(),
            data: audio_data,
            name: file_info.name.clone(),
            file_type: file_info.file_type.clone(),
            id: file_info.hex_id.as_ref().unwrap_or(&file_info.id).clone(),
            #[cfg(target_arch = "wasm32")]
            temp_url: None,
        })
    }
    
    /// Load original audio data from NUS3BANK
    fn load_original_nus3bank_audio(
        &self,
        file_info: &AudioFileInfo,
        file_path: &str,
    ) -> Result<Vec<u8>, String> {
        let nus3bank_file = Nus3bankFile::open(file_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;
        
        let hex_id = file_info.hex_id.as_ref().unwrap_or(&file_info.id);
        let track = nus3bank_file.get_track_by_hex_id(hex_id)
            .ok_or_else(|| format!("Track with hex ID {} not found", hex_id))?;
        
        track.audio_data.clone()
            .ok_or_else(|| "Audio data not loaded for track".to_string())
    }
}
```

## Cargo.toml Dependencies

### No Additional Dependencies Required
All functionality is implemented using Rust standard library only:
- `std::io` for file I/O operations
- `std::fmt` for error display implementation
- `std::error::Error` trait for error handling
- Built-in little-endian conversion functions (`u32::from_le_bytes`, etc.)

## Module Structure

### Recommended File Organization
```
src/
├── nus3bank/
│   ├── mod.rs              # Module exports and public API
│   ├── structures.rs       # Data structures (Nus3bankFile, BankInfo, AudioTrack)
│   ├── parser.rs           # File parsing implementation
│   ├── writer.rs           # File writing/reconstruction
│   ├── binary_utils.rs     # Binary reading utilities
│   ├── error.rs            # Error types and handling
│   ├── export.rs           # Export functionality
│   ├── replace.rs          # Replace operations
│   └── integration.rs      # Integration helpers for existing systems
├── ui/
│   └── main_area/
│       ├── audio_file_info.rs    # Enhanced AudioFileInfo
│       ├── main_area_filtering.rs # Enhanced file detection
│       ├── export_utils.rs       # Enhanced export utilities
│       └── replace_utils.rs      # Enhanced replace utilities
```

### Module Exports (mod.rs)
```rust
//! NUS3BANK file format support for EXVS2 Audio Editor
//! 
//! This module provides comprehensive support for reading, writing, and manipulating
//! NUS3BANK audio archive files with focus on WAV format audio only.

pub mod structures;
pub mod parser;
pub mod writer;
pub mod binary_utils;
pub mod error;
pub mod export;
pub mod replace;
pub mod integration;

// Re-export main types
pub use structures::{Nus3bankFile, BankInfo, AudioTrack, AudioFormat};
pub use parser::Nus3bankParser;
pub use error::Nus3bankError;
pub use export::Nus3bankExporter;
pub use replace::Nus3bankReplacer;

/// Module version and compatibility information
pub const VERSION: &str = "1.0.0";
pub const SUPPORTED_FORMATS: &[&str] = &["WAV"];
```

## Implementation Phases

### Phase 1: Core Implementation (Priority: Critical)
1. **Binary utilities** (`binary_utils.rs`)
   - Little-endian reading functions
   - Magic number validation
   - String padding calculations
   - Error handling setup

2. **Data structures** (`structures.rs`)
   - Complete type definitions
   - Basic method implementations
   - Integration with existing AudioFileInfo

3. **Parser implementation** (`parser.rs`)
   - Header validation
   - Section parsing (PROP, BINF, TONE, PACK)
   - Track extraction
   - WAV format detection

### Phase 2: Integration (Priority: High)
1. **File detection enhancement**
   - Modify `main_area_filtering.rs`
   - Add NUS3BANK vs NUS3AUDIO differentiation
   - Maintain backward compatibility

2. **Export system integration**
   - Extend `export_utils.rs`
   - Add NUS3BANK export support
   - Reuse existing vgmstream pipeline

3. **Replace system integration**
   - Extend `replace_utils.rs`
   - Add NUS3BANK replacement support
   - Maintain in-memory replacement pattern

### Phase 3: Advanced Features (Priority: Medium)
1. **File writing** (`writer.rs`)
   - Complete file reconstruction
   - Offset recalculation
   - Size header updates

2. **Advanced operations**
   - Track addition/removal
   - Batch operations
   - Validation and integrity checks

### Phase 4: UI Enhancement (Priority: Low)
1. **Table display**
   - Hex ID column addition
   - Bank information display
   - Format-specific UI elements

2. **Audio player enhancement**
   - NUS3BANK-specific loading
   - Format handling improvements
