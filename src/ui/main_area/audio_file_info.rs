/// Structure to hold audio file information
#[derive(Clone, Debug)]
pub struct AudioFileInfo {
    pub name: String,
    pub id: String,
    pub size: usize,
    pub filename: String,
    pub file_type: String,
    // New fields for NUS3BANK support
    pub hex_id: Option<String>,        // Hex ID for NUS3BANK files
    pub is_nus3bank: bool,             // File type indicator
}

impl AudioFileInfo {
    /// Create AudioFileInfo for NUS3AUDIO file (backward compatibility)
    pub fn from_nus3audio(name: String, id: String, size: usize, filename: String, file_type: String) -> Self {
        Self {
            name,
            id,
            size,
            filename,
            file_type,
            hex_id: None,
            is_nus3bank: false,
        }
    }
    
    /// Create AudioFileInfo for NUS3BANK track
    /// Note: `id` stores the track's index (0-based) for subsong mapping
    pub fn from_nus3bank_track(name: String, index: u32, hex_id: String, size: usize, filename: String) -> Self {
        Self {
            name,
            id: index.to_string(),
            size,
            filename,
            file_type: "WAV".to_string(),
            hex_id: Some(hex_id),
            is_nus3bank: true,
        }
    }
    
    /// Get the effective ID (hex_id for NUS3BANK, id for NUS3AUDIO)
    pub fn effective_id(&self) -> &str {
        self.hex_id.as_ref().unwrap_or(&self.id)
    }
}
