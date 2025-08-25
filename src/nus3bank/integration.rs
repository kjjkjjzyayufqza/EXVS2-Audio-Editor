//! Integration helpers for existing systems
//! 
//! This module provides helper functions and types to integrate NUS3BANK support
//! with the existing UI and audio processing systems.

use super::{
    structures::{Nus3bankFile, AudioTrack, BankInfo},
};
use crate::ui::main_area::AudioFileInfo;

/// Integration helpers for NUS3BANK support
pub struct Nus3bankIntegration;

impl Nus3bankIntegration {
    /// Convert NUS3BANK AudioTrack to AudioFileInfo for UI compatibility
    pub fn track_to_audio_file_info(track: &AudioTrack, _bank_info: &BankInfo) -> AudioFileInfo {
        AudioFileInfo::from_nus3bank_track(
            track.name.clone(),
            track.numeric_id,
            track.hex_id.clone(),
            track.size as usize,
            track.filename(),
        )
    }
    
    /// Convert all tracks to AudioFileInfo vector
    pub fn tracks_to_audio_file_infos(tracks: &[AudioTrack], bank_info: &BankInfo) -> Vec<AudioFileInfo> {
        tracks.iter()
            .map(|track| Self::track_to_audio_file_info(track, bank_info))
            .collect()
    }
    
    /// Check if a file path is a NUS3BANK file
    pub fn is_nus3bank_file(file_path: &str) -> bool {
        file_path.to_lowercase().ends_with(".nus3bank")
    }
    
    /// Load NUS3BANK file and convert to AudioFileInfo vector for UI
    pub fn load_nus3bank_as_audio_infos(file_path: &str) -> Result<Vec<AudioFileInfo>, String> {
        let nus3bank_file = Nus3bankFile::open(file_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;
        
        Ok(Self::tracks_to_audio_file_infos(&nus3bank_file.tracks, &nus3bank_file.bank_info))
    }
    
    /// Get track count from NUS3BANK file
    pub fn get_track_count(file_path: &str) -> Result<usize, String> {
        let nus3bank_file = Nus3bankFile::open(file_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;
        
        Ok(nus3bank_file.tracks.len())
    }
}
