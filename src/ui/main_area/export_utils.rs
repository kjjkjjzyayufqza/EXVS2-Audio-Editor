use nus3audio::Nus3audioFile;
use std::fs;
use std::path::Path;
use super::audio_file_info::AudioFileInfo;

/// Utility functions for exporting audio files
pub struct ExportUtils;

impl ExportUtils {
    /// Export audio data to a WAV file
    pub fn export_to_wav(
        audio_file_info: &AudioFileInfo,
        original_file_path: &str,
    ) -> Result<String, String> {
        // Load the NUS3AUDIO file
        let nus3_file = match Nus3audioFile::open(original_file_path) {
            Ok(file) => file,
            Err(e) => return Err(format!("Failed to open NUS3AUDIO file: {}", e)),
        };

        // Find the audio file by name
        let audio_file = nus3_file.files.iter().find(|f| f.name == audio_file_info.name);
        let audio_file = match audio_file {
            Some(file) => file,
            None => return Err("Audio file not found in NUS3AUDIO file".to_string()),
        };

        // Create output file path (same directory as original file with .wav extension)
        let original_path = Path::new(original_file_path);
        let parent_dir = match original_path.parent() {
            Some(dir) => dir,
            None => return Err("Failed to get parent directory".to_string()),
        };
        
        let output_filename = format!("{}.wav", audio_file_info.name);
        let output_path = parent_dir.join(output_filename);
        
        // Write audio data to WAV file
        match fs::write(&output_path, &audio_file.data) {
            Ok(_) => {
                println!("Successfully exported WAV file to: {:?}", output_path);
                Ok(output_path.to_string_lossy().to_string())
            },
            Err(e) => Err(format!("Failed to write WAV file: {}", e)),
        }
    }
}
