use nus3audio::{Nus3audioFile, AudioFile};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use rfd::FileDialog;
use super::audio_file_info::AudioFileInfo;
use super::add_audio_modal::AddAudioModal;

/// Utility functions for adding new audio files
pub struct AddAudioUtils;

impl AddAudioUtils {
    /// Convert selected audio file to WAV format using vgmstream
    pub fn convert_to_wav(file_path: &str) -> Result<Vec<u8>, String> {
        // Path to vgmstream-cli.exe in tools directory
        let vgmstream_path = Path::new("tools").join("vgmstream-cli.exe");
        if !vgmstream_path.exists() {
            return Err(format!("vgmstream-cli not found at {:?}", vgmstream_path));
        }

        // Create a temporary output file path
        let temp_dir = std::env::temp_dir();
        let original_filename = Path::new(file_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let temp_filename = format!("temp_convert_{}.wav", original_filename);
        let temp_output_path = temp_dir.join(&temp_filename);
        let temp_output_path_str = temp_output_path.to_string_lossy().to_string();

        println!("Converting to WAV: {:?} -> {:?}", file_path, temp_output_path);

        // Run vgmstream-cli to convert audio to WAV
        let mut command = Command::new(&vgmstream_path);
        
        #[cfg(windows)]
        {
            use winapi::um::winbase::CREATE_NO_WINDOW;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        
        let result = command
            .args([
                "-o",
                &temp_output_path_str,
                file_path,
            ])
            .output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    // Read the temporary WAV file into memory
                    match fs::read(&temp_output_path) {
                        Ok(wav_data) => {
                            println!("Successfully converted to WAV: {} bytes", wav_data.len());
                            // Clean up the temporary file
                            let _ = fs::remove_file(&temp_output_path);
                            Ok(wav_data)
                        }
                        Err(e) => {
                            // Clean up the temporary file even if reading failed
                            let _ = fs::remove_file(&temp_output_path);
                            Err(format!("Failed to read converted WAV data: {}", e))
                        }
                    }
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    Err(format!("vgmstream-cli error: {}", error))
                }
            }
            Err(e) => Err(format!("Failed to run vgmstream-cli: {}", e)),
        }
    }

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
        // Check if file path exists
        let file_path = match &add_audio_modal.settings.file_path {
            Some(path) => path,
            None => return Err("No audio file path available".to_string()),
        };
        
        // Convert the audio file to WAV format using vgmstream
        let file_data = match Self::convert_to_wav(file_path) {
            Ok(wav_data) => wav_data,
            Err(e) => {
                println!("Warning: Failed to convert to WAV: {}", e);
                println!("Falling back to original file data");
                // Fall back to the original file data if conversion fails
                match &add_audio_modal.file_data {
                    Some(data) => data.clone(),
                    None => return Err("No audio file data available".to_string()),
                }
            }
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
        
        // Get the filename from the original file path
        let filename = Path::new(file_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        
        // Create a new AudioFileInfo for the UI
        let new_audio_info = AudioFileInfo {
            name,
            id: id_val.to_string(),
            size: file_data.len(),
            filename: format!("{}.wav", Path::new(&filename).file_stem().unwrap_or_default().to_string_lossy()),
            file_type: "WAV Audio".to_string(),
        };
        
        // Return the new AudioFileInfo and the converted WAV data
        Ok(new_audio_info)
    }
    
    /// Process the new audio file and save it to the nus3audio file
    /// 
    /// Deprecated: Use process_new_audio instead and then use Nus3audioFileUtils to register
    /// the addition to be applied later with save_changes_to_file.
    #[deprecated(
        since = "1.1.0",
        note = "Use process_new_audio instead and then use Nus3audioFileUtils to register the addition"
    )]
    pub fn process_new_audio_and_save(
        add_audio_modal: &AddAudioModal,
        original_file_path: &str,
    ) -> Result<AudioFileInfo, String> {
        // First create the new audio info
        let new_audio_info = Self::process_new_audio(add_audio_modal, original_file_path)?;
        
        // Check if file data exists
        let file_data = match &add_audio_modal.file_data {
            Some(data) => data,
            None => return Err("No audio file data available".to_string()),
        };
        
        // Convert ID to valid format expected by Nus3audioFile
        let id_val = match new_audio_info.id.parse::<u32>() {
            Ok(val) => val,
            Err(_) => return Err("ID must be a valid number".to_string()),
        };
        
        // Create a new AudioFile with the audio data
        let new_audio_file = AudioFile {
            id: id_val,
            name: new_audio_info.name.clone(),
            data: file_data.clone(),
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