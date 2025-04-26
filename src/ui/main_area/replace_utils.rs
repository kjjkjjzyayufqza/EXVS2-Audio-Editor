use nus3audio::{Nus3audioFile, AudioFile};
use std::fs;
use std::path::Path;
use rfd::FileDialog;
use super::audio_file_info::AudioFileInfo;

/// Utility functions for replacing audio files
pub struct ReplaceUtils;

impl ReplaceUtils {
    /// Replace an audio file with data from a new file (modifies the actual file on disk)
    pub fn replace_audio(
        audio_file_info: &AudioFileInfo,
        original_file_path: &str,
        replacement_file_path: &str,
    ) -> Result<(), String> {
        // Load the original NUS3AUDIO file
        let mut nus3_file = match Nus3audioFile::open(original_file_path) {
            Ok(file) => file,
            Err(e) => return Err(format!("Failed to open NUS3AUDIO file: {}", e)),
        };

        // Find the target audio file by name
        let target_index = nus3_file.files.iter().position(|f| f.name == audio_file_info.name);
        let target_index = match target_index {
            Some(index) => index,
            None => return Err("Audio file not found in NUS3AUDIO file".to_string()),
        };

        // Load the replacement file data
        let replacement_data = match fs::read(replacement_file_path) {
            Ok(data) => data,
            Err(e) => return Err(format!("Failed to read replacement file: {}", e)),
        };

        // Replace the audio data while preserving the ID and name
        let id = nus3_file.files[target_index].id;
        let name = nus3_file.files[target_index].name.clone();
        
        // Create a new AudioFile with the replacement data
        let new_audio_file = AudioFile {
            id,
            name,
            data: replacement_data,
        };
        
        // Replace the old file with the new one
        nus3_file.files[target_index] = new_audio_file;
        
        // Create a backup of the original file
        let original_path = Path::new(original_file_path);
        let backup_path = original_path.with_extension(format!(
            "{}.bak",
            original_path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
        ));

        // Create memory buffer for writing the updated NUS3AUDIO file
        let mut output_buffer = Vec::new();
        
        // Write the modified NUS3AUDIO data to memory buffer
        nus3_file.write(&mut output_buffer);
        
        // Write the buffer to the original file
        match fs::write(original_file_path, output_buffer) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to write updated file: {}", e)),
        }
    }
    
    /// Replace audio data in memory only (does not modify the actual file on disk)
    pub fn replace_in_memory(
        audio_file_info: &AudioFileInfo,
        replacement_file_path: &str,
    ) -> Result<AudioFileInfo, String> {
        // Load the replacement file data
        let replacement_data = match fs::read(replacement_file_path) {
            Ok(data) => data,
            Err(e) => return Err(format!("Failed to read replacement file: {}", e)),
        };
        
        // Create a new AudioFileInfo with the replacement data
        let new_audio_info = AudioFileInfo {
            name: audio_file_info.name.clone(),
            id: audio_file_info.id.clone(),
            size: replacement_data.len(),
            filename: Path::new(replacement_file_path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            file_type: audio_file_info.file_type.clone(),
        };
        
        Ok(new_audio_info)
    }
    
    /// Show file dialog to select replacement audio file and replace the target audio in memory
    pub fn replace_with_file_dialog(
        audio_file_info: &AudioFileInfo,
        original_file_path: &str,
    ) -> Result<AudioFileInfo, String> {
        // Open a file dialog to select the replacement audio file
        let file_path = match FileDialog::new()
            .add_filter("Audio Files", &["wav", "mp3", "ogg", "lopus", "idsp", "bin"])
            .add_filter("All Files", &["*"])
            .set_title("Select Replacement Audio File")
            .pick_file()
        {
            Some(path) => path,
            None => return Err("No file selected".to_string()),
        };
        
        // Replace the audio file with the selected file in memory only
        Self::replace_in_memory(audio_file_info, file_path.to_str().unwrap())
    }
}
