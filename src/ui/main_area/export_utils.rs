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
    
    /// Export audio data to a WAV file with custom output directory
    pub fn export_to_wav_with_custom_dir(
        audio_file_info: &AudioFileInfo,
        original_file_path: &str,
        output_dir: &str,
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

        // Create output file path in the custom directory
        let output_dir_path = Path::new(output_dir);
        let output_filename = format!("{}.wav", audio_file_info.name);
        let output_path = output_dir_path.join(output_filename);
        
        // Write audio data to WAV file
        match fs::write(&output_path, &audio_file.data) {
            Ok(_) => {
                println!("Successfully exported WAV file to: {:?}", output_path);
                Ok(output_path.to_string_lossy().to_string())
            },
            Err(e) => Err(format!("Failed to write WAV file: {}", e)),
        }
    }
    
    /// Export all audio files in a NUS3AUDIO file to WAV files with custom output directory
    pub fn export_all_to_wav(
        original_file_path: &str,
        output_dir: &str,
    ) -> Result<Vec<String>, String> {
        // Load the NUS3AUDIO file
        let nus3_file = match Nus3audioFile::open(original_file_path) {
            Ok(file) => file,
            Err(e) => return Err(format!("Failed to open NUS3AUDIO file: {}", e)),
        };

        let mut exported_paths = Vec::new();
        let output_dir_path = Path::new(output_dir);
        
        // Export each audio file
        for audio_file in &nus3_file.files {
            let output_filename = format!("{}.wav", audio_file.name);
            let output_path = output_dir_path.join(&output_filename);
            
            match fs::write(&output_path, &audio_file.data) {
                Ok(_) => {
                    println!("Successfully exported WAV file to: {:?}", output_path);
                    exported_paths.push(output_path.to_string_lossy().to_string());
                },
                Err(e) => {
                    return Err(format!("Failed to write WAV file {}: {}", output_filename, e));
                }
            }
        }
        
        Ok(exported_paths)
    }
}
