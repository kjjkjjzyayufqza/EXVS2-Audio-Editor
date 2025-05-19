use nus3audio::{Nus3audioFile, AudioFile};
use std::fs;
use std::path::{Path, PathBuf};
use rfd::FileDialog;
use super::audio_file_info::AudioFileInfo;
use super::add_audio_modal::AddAudioModal;

/// Utility functions for adding new audio files
pub struct AddAudioUtils;

impl AddAudioUtils {
    /// Show file dialog to select a new audio file and open the add audio modal
    pub fn add_with_file_dialog(
        add_audio_modal: &mut AddAudioModal,
        existing_audio_files: Option<Vec<AudioFileInfo>>,
    ) -> Result<(), String> {
        // Open a file dialog to select the audio file
        let result = FileDialog::new()
            .add_filter("Audio Files", &["wav", "mp3", "ogg", "lopus", "idsp", "bin"])
            .add_filter("All Files", &["*"])
            .set_title("Select Audio File to Add")
            .pick_file();
        
        if result.is_none() {
            return Err("No file selected".to_string());
        }
        
        // Get selected file path
        let selected_path = result.unwrap();
        let path_str = selected_path.to_string_lossy().to_string();
        
        // Open the modal with the selected file
        add_audio_modal.open_with_file(&path_str, existing_audio_files);
        
        Ok(())
    }
    
    /// Process the new audio file after the modal is confirmed
    pub fn process_new_audio(
        add_audio_modal: &AddAudioModal,
        original_file_path: &str,
    ) -> Result<AudioFileInfo, String> {
        // Check if file data exists
        let file_data = match &add_audio_modal.file_data {
            Some(data) => data,
            None => return Err("No audio file data available".to_string()),
        };
        
        // Get name and ID from settings
        let name = add_audio_modal.settings.name.clone();
        let id = add_audio_modal.settings.id.clone();
        
        if name.is_empty() || id.is_empty() {
            return Err("Name and ID cannot be empty".to_string());
        }
        
        // Convert ID to valid format expected by Nus3audioFile
        let id_val = match id.parse::<u32>() {
            Ok(val) => val,
            Err(_) => return Err("ID must be a valid number".to_string()),
        };
        
        // Create a new AudioFile with the audio data
        let new_audio_file = AudioFile {
            id: id_val,
            name: name.clone(),
            data: file_data.clone(),
        };
        
        // Get the filename from the original file path
        let filename = match &add_audio_modal.settings.file_path {
            Some(path) => {
                Path::new(path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            },
            None => "unknown.wav".to_string(),
        };
        
        // Detect file type based on header
        let file_type = if file_data.len() >= 4 {
            match &file_data[..4] {
                b"OPUS" => "OPUS Audio",
                b"IDSP" => "IDSP Audio",
                b"RIFF" => "WAV Audio",
                b"BNSF" => "BNSF Audio",
                _ => "Unknown",
            }
        } else {
            "Unknown"
        };
        
        // Create a new AudioFileInfo for the UI
        let new_audio_info = AudioFileInfo {
            name,
            id: id_val.to_string(),
            size: file_data.len(),
            filename,
            file_type: file_type.to_string(),
        };
        
        // Load the existing NUS3AUDIO file
        let mut nus3_file = match Nus3audioFile::open(original_file_path) {
            Ok(file) => file,
            Err(e) => return Err(format!("Failed to open NUS3AUDIO file: {}", e)),
        };
        
        // Add the new audio file to the NUS3AUDIO file
        nus3_file.files.push(new_audio_file);
        
        // Create memory buffer for writing the updated NUS3AUDIO file
        let mut output_buffer = Vec::new();
        
        // Write the modified NUS3AUDIO data to memory buffer
        nus3_file.write(&mut output_buffer);
        
        // Write the buffer back to the original file (in-place update)
        match fs::write(original_file_path, output_buffer) {
            Ok(_) => Ok(new_audio_info),
            Err(e) => Err(format!("Failed to write updated file: {}", e)),
        }
    }
} 