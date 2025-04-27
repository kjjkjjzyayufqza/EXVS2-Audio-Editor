use nus3audio::{Nus3audioFile, AudioFile};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use rfd::FileDialog;
use super::audio_file_info::AudioFileInfo;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

// 使用静态HashMap存储替换后的音频数据
// 键是"文件路径:音频名称"，值是替换后的音频数据
static REPLACED_AUDIO_DATA: Lazy<Mutex<HashMap<String, Vec<u8>>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

/// Utility functions for replacing audio files
pub struct ReplaceUtils;

impl ReplaceUtils {
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
        
        // Store the replacement data in our static HashMap
        let key = format!("{}:{}", audio_file_info.name, audio_file_info.id);
        if let Ok(mut map) = REPLACED_AUDIO_DATA.lock() {
            map.insert(key, replacement_data.clone());
        }
        
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
    
    /// Process audio file with vgmstream-cli to add loop points
    pub fn process_with_vgmstream(
        file_path: &Path
    ) -> Result<PathBuf, String> {
        // Path to vgmstream-cli.exe in tools directory
        let vgmstream_path = Path::new("tools").join("vgmstream-cli.exe");
        if !vgmstream_path.exists() {
            return Err(format!("vgmstream-cli not found at {:?}", vgmstream_path));
        }
        
        // Create a temporary output file path
        let temp_dir = std::env::temp_dir();
        let original_filename = file_path.file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let temp_filename = format!("looping_{}", original_filename);
        let temp_output_path = temp_dir.join(&temp_filename);
        let temp_output_path_str = temp_output_path.to_string_lossy().to_string();
        
        println!("Processing with vgmstream: {:?} -> {:?}", file_path, temp_output_path);
        
        // Run vgmstream-cli to convert audio with loop points
        // -L: Loop the file forever
        // -E: Force end-to-end looping
        // -o: Output file path
        let result = Command::new(&vgmstream_path)
            .args(&[
                "-i",
                "-L",
                "-o",
                &temp_output_path_str,
                file_path.to_string_lossy().as_ref(),
            ])
            .output();
            
        match result {
            Ok(output) => {
                if output.status.success() {
                    println!("Successfully processed file with vgmstream: {:?}", temp_output_path);
                    Ok(temp_output_path)
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    Err(format!("vgmstream-cli error: {}", error))
                }
            }
            Err(e) => Err(format!("Failed to run vgmstream-cli: {}", e)),
        }
    }

    /// Show file dialog to select replacement audio file and replace the target audio in memory
    /// with added loop points
    pub fn replace_with_file_dialog(
        audio_file_info: &AudioFileInfo
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
        
        // Clone file_path for comparison later
        let original_path = file_path.clone();
        
        // Process the selected file with vgmstream to add loop points
        let processed_path = match Self::process_with_vgmstream(&file_path) {
            Ok(path) => path,
            Err(e) => {
                println!("Warning: Failed to process file with vgmstream: {}", e);
                println!("Falling back to original file");
                // Fall back to the original file if processing fails
                original_path.clone()
            }
        };
        
        // Replace the audio file with the processed file in memory only
        let result = Self::replace_in_memory(audio_file_info, processed_path.to_str().unwrap());
        
        // Clean up temporary file if it's different from the original
        if processed_path != original_path && processed_path.exists() {
            let _ = fs::remove_file(&processed_path);
        }
        
        result
    }
    
    /// Get the replacement audio data for a specific audio file
    pub fn get_replacement_data(audio_name: &str, audio_id: &str) -> Option<Vec<u8>> {
        let key = format!("{}:{}", audio_name, audio_id);
        if let Ok(map) = REPLACED_AUDIO_DATA.lock() {
            map.get(&key).cloned()
        } else {
            None
        }
    }
    
    /// Clear all replacement data from memory
    pub fn clear_replacements() {
        if let Ok(mut map) = REPLACED_AUDIO_DATA.lock() {
            map.clear();
            println!("Cleared all audio replacements from memory");
        }
    }

    /// Apply all in-memory replacements to a NUS3AUDIO file and save it
    pub fn apply_replacements_and_save(
        original_file_path: &str,
        save_path: &str,
    ) -> Result<(), String> {
        // Load the original NUS3AUDIO file
        let mut nus3_file = match Nus3audioFile::open(original_file_path) {
            Ok(file) => file,
            Err(e) => return Err(format!("Failed to open NUS3AUDIO file: {}", e)),
        };
        
        // Apply all replacements from our static HashMap
        if let Ok(map) = REPLACED_AUDIO_DATA.lock() {
            for (key, replacement_data) in map.iter() {
                // Parse the key to get audio name and id
                let parts: Vec<&str> = key.split(':').collect();
                if parts.len() != 2 {
                    continue;
                }
                
                let audio_name = parts[0];
                
                // Find the target audio file by name
                let target_index = match nus3_file.files.iter().position(|f| f.name == audio_name) {
                    Some(index) => index,
                    None => continue, // Skip if not found
                };
                
                // Replace the audio data while preserving the ID and name
                let id = nus3_file.files[target_index].id;
                let name = nus3_file.files[target_index].name.clone();
                
                // Create a new AudioFile with the replacement data
                let new_audio_file = AudioFile {
                    id,
                    name,
                    data: replacement_data.clone(),
                };
                
                // Replace the old file with the new one
                nus3_file.files[target_index] = new_audio_file;
            }
        }
        
        // Create memory buffer for writing the updated NUS3AUDIO file
        let mut output_buffer = Vec::new();
        
        // Write the modified NUS3AUDIO data to memory buffer
        nus3_file.write(&mut output_buffer);
        
        // Write the buffer to the save file
        match fs::write(save_path, output_buffer) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to write updated file: {}", e)),
        }
    }
}
