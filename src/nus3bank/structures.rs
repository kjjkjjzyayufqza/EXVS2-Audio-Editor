use super::error::Nus3bankError;

/// Main structure representing a complete NUS3BANK file
#[derive(Clone, Debug)]
pub struct Nus3bankFile {
    /// Bank metadata information
    pub bank_info: BankInfo,
    /// Collection of audio tracks in the bank
    pub tracks: Vec<AudioTrack>,
    /// Original file path
    pub file_path: String,
}

impl Nus3bankFile {
    /// Open and parse a NUS3BANK file
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Nus3bankError> {
        super::parser::Nus3bankParser::parse_file(path)
    }
    
    /// Save the NUS3BANK file to disk
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Nus3bankError> {
        super::writer::Nus3bankWriter::write_file(self, path)
    }
    
    /// Get track by hex ID
    pub fn get_track_by_hex_id(&self, hex_id: &str) -> Option<&AudioTrack> {
        self.tracks.iter().find(|t| t.hex_id == hex_id)
    }
    
    /// Get mutable track by hex ID
    pub fn get_track_by_hex_id_mut(&mut self, hex_id: &str) -> Option<&mut AudioTrack> {
        self.tracks.iter_mut().find(|t| t.hex_id == hex_id)
    }
    
    /// Replace track data by hex ID
    pub fn replace_track_data(&mut self, hex_id: &str, new_data: Vec<u8>) -> Result<(), Nus3bankError> {
        let track = self.get_track_by_hex_id_mut(hex_id)
            .ok_or_else(|| Nus3bankError::TrackNotFound { hex_id: hex_id.to_string() })?;
        
        track.audio_data = Some(new_data.clone());
        track.size = new_data.len() as u32;
        
        // Detect format (WAV only as per requirements)
        if new_data.starts_with(b"RIFF") {
            track.audio_format = AudioFormat::Wav;
        } else {
            track.audio_format = AudioFormat::Unknown;
        }
        
        Ok(())
    }
    
    /// Add new track to the bank
    pub fn add_track(&mut self, name: String, audio_data: Vec<u8>) -> Result<String, Nus3bankError> {
        // Validate input data
        if audio_data.is_empty() {
            return Err(Nus3bankError::InvalidFormat {
                reason: "Audio data cannot be empty".to_string()
            });
        }
        
        if name.is_empty() {
            return Err(Nus3bankError::InvalidFormat {
                reason: "Track name cannot be empty".to_string()
            });
        }
        
        // Check for existing track with same name
        if self.tracks.iter().any(|t| t.name == name) {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("Track with name '{}' already exists", name)
            });
        }
        
        // Generate new ID (find highest ID and add 1)
        let new_id = self.tracks.iter()
            .map(|t| t.numeric_id)
            .max()
            .unwrap_or(0) + 1;
        
        let hex_id = format!("0x{:x}", new_id);
        
        // Detect format (prefer WAV for compatibility)
        let audio_format = if audio_data.starts_with(b"RIFF") {
            AudioFormat::Wav
        } else {
            println!("Warning: Non-WAV format detected for track '{}'", name);
            AudioFormat::Unknown
        };
        
        let track = AudioTrack {
            index: self.tracks.len(),
            hex_id: hex_id.clone(),
            numeric_id: new_id,
            name,
            pack_offset: 0, // Will be recalculated when saving
            size: audio_data.len() as u32,
            metadata_offset: 0,
            metadata_size: 0,
            audio_data: Some(audio_data),
            audio_format,
            original_metadata: None, // New tracks don't have original metadata
        };
        
        self.tracks.push(track);
        self.bank_info.track_count = self.tracks.len() as u32;
        
        Ok(hex_id)
    }
    
    /// Remove track by hex ID
    pub fn remove_track(&mut self, hex_id: &str) -> Result<(), Nus3bankError> {
        let track = self.tracks.iter_mut()
            .find(|t| t.hex_id == hex_id)
            .ok_or_else(|| Nus3bankError::TrackNotFound { hex_id: hex_id.to_string() })?;
        
        // Mark track for removal by setting metadata_size to 0
        // This approach preserves track order and allows writer to filter correctly
        track.metadata_size = 0;
        track.size = 0;
        track.audio_data = None; // Clear audio data to save memory
        
        println!("Marked track {} for removal (metadata_size=0)", hex_id);
        
        // Update bank info to reflect new count (excluding removed tracks)
        let active_count = self.tracks.iter().filter(|t| t.metadata_size > 0).count();
        self.bank_info.track_count = active_count as u32;
        
        Ok(())
    }
}

/// Bank-level metadata extracted from BINF section /* "BINF": bank info (filename) */
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
/// Contains offsets for PROP, BINF, TONE, and PACK sections
#[derive(Clone, Debug)]
pub struct SectionOffsets {
    pub prop_offset: u32, /* "PROP": project info */
    pub binf_offset: u32, /* "BINF": bank info (filename) */
    pub tone_offset: u32, /* "TONE": stream info */
    pub pack_offset: u32, /* "PACK": audio streams */
}

impl Default for SectionOffsets {
    fn default() -> Self {
        Self {
            prop_offset: 0,
            binf_offset: 0,
            tone_offset: 0,
            pack_offset: 0,
        }
    }
}

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
    /// Offset within PACK section /* "PACK": audio streams */
    pub pack_offset: u32,
    /// Audio data size in bytes
    pub size: u32,
    /// TONE metadata offset /* "TONE": stream info */
    pub metadata_offset: u32,
    /// TONE metadata size /* "TONE": stream info */
    pub metadata_size: u32,
    /// Raw audio data (WAV format only)
    pub audio_data: Option<Vec<u8>>,
    /// Audio format type
    pub audio_format: AudioFormat,
    /// Original complete metadata block from TONE section (preserves all data)
    pub original_metadata: Option<Vec<u8>>,
}

impl AudioTrack {
    /// Generate filename for export
    pub fn filename(&self) -> String {
        format!("{}-{}.wav", self.hex_id, self.name)
    }
    
    /// Load audio data from PACK section /* "PACK": audio streams */
    pub fn load_audio_data(&mut self, pack_data: &[u8]) -> Result<(), Nus3bankError> {
        if self.pack_offset + self.size <= pack_data.len() as u32 {
            let start = self.pack_offset as usize;
            let end = start + self.size as usize;
            self.audio_data = Some(pack_data[start..end].to_vec());
            
            // Detect format (WAV only as per requirements)
            if let Some(data) = &self.audio_data {
                if data.starts_with(b"RIFF") {
                    self.audio_format = AudioFormat::Wav;
                }
            }
            
            Ok(())
        } else {
            Err(Nus3bankError::InvalidFormat {
                reason: format!("Track {} offset/size out of bounds", self.hex_id)
            })
        }
    }
}

/// Supported audio formats (WAV only as per requirements)
#[derive(Clone, Debug, PartialEq)]
pub enum AudioFormat {
    Wav,
    Unknown,
}
