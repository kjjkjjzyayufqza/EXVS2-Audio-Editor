use super::structures::Nus3bankFile;
use std::fs;

/// NUS3BANK export utilities
pub struct Nus3bankExporter;

impl Nus3bankExporter {
    /// Export a single track to WAV file
    pub fn export_track(
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
            fs::write(&output_path, audio_data)
                .map_err(|e| format!("Failed to write audio file: {}", e))?;
        } else {
            return Err(format!("Audio data not loaded for track '{}' ({}). The track may be corrupted or the file may not have been parsed correctly.", track.name, track.hex_id));
        }
        
        Ok(output_path)
    }
    
    /// Batch export all tracks from NUS3BANK
    pub fn export_all_tracks(
        file_path: &str,
        output_dir: &str,
    ) -> Result<Vec<String>, String> {
        let nus3bank_file = Nus3bankFile::open(file_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;
        
        let mut exported_files = Vec::new();
        
        for track in &nus3bank_file.tracks {
            match Self::export_track(file_path, &track.hex_id, output_dir) {
                Ok(path) => exported_files.push(path),
                Err(e) => log::warn!("Failed to export track {}: {}", track.hex_id, e),
            }
        }
        
        Ok(exported_files)
    }
    
    /// Export track to memory (for integration with existing systems)
    pub fn export_track_to_memory(
        file_path: &str,
        hex_id: &str,
    ) -> Result<Vec<u8>, String> {
        let nus3bank_file = Nus3bankFile::open(file_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;
        
        let track = nus3bank_file.get_track_by_hex_id(hex_id)
            .ok_or_else(|| format!("Track with hex ID {} not found", hex_id))?;
        
        track.audio_data.clone()
            .ok_or_else(|| format!("Audio data not loaded for track '{}' ({}). The track may be corrupted or the file may not have been parsed correctly.", track.name, track.hex_id))
    }
}
