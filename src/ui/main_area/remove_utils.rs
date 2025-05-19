use nus3audio::Nus3audioFile;
use std::fs;
use super::audio_file_info::AudioFileInfo;

/// Utility functions for removing audio files
pub struct RemoveUtils;

impl RemoveUtils {
    /// Remove audio file in memory only (does not modify the actual file on disk)
    pub fn remove_in_memory(
        audio_file_info: &AudioFileInfo,
    ) -> Result<(), String> {
        // This function only removes the audio from memory
        // It does not modify the actual file on disk
        
        // Just return success as the actual removal is handled by the caller
        Ok(())
    }

    /// Remove audio file from nus3audio file and save changes
    pub fn remove_audio(
        audio_file_info: &AudioFileInfo,
        original_file_path: &str,
    ) -> Result<(), String> {
        // Load the existing NUS3AUDIO file
        let mut nus3_file = match Nus3audioFile::open(original_file_path) {
            Ok(file) => file,
            Err(e) => return Err(format!("Failed to open NUS3AUDIO file: {}", e)),
        };
        
        // Find the index of the audio file to remove
        let target_index = match nus3_file.files.iter().position(|f| 
            f.name == audio_file_info.name && 
            f.id.to_string() == audio_file_info.id
        ) {
            Some(index) => index,
            None => return Err(format!("Audio file '{}' not found in NUS3AUDIO file", audio_file_info.name)),
        };
        
        // Remove the audio file from the collection
        nus3_file.files.remove(target_index);
        
        // Create memory buffer for writing the updated NUS3AUDIO file
        let mut output_buffer = Vec::new();
        
        // Write the modified NUS3AUDIO data to memory buffer
        nus3_file.write(&mut output_buffer);
        
        // Write the buffer back to the original file (in-place update)
        match fs::write(original_file_path, output_buffer) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to write updated file: {}", e)),
        }
    }
} 