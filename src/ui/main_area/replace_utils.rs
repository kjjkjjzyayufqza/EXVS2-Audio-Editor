use nus3audio::{Nus3audioFile, AudioFile};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use rfd::FileDialog;
use super::audio_file_info::AudioFileInfo;
use super::loop_settings_modal::LoopSettingsModal;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

// 使用静态HashMap存储替换后的音频数据
// 键是"文件路径:音频名称"，值是替换后的音频数据
static REPLACED_AUDIO_DATA: Lazy<Mutex<HashMap<String, Vec<u8>>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

// 使用静态HashMap存储循环设置
// 键是"文件路径:音频名称"，值是(loop_start, loop_end, use_custom_loop)
static LOOP_SETTINGS: Lazy<Mutex<HashMap<String, (Option<f32>, Option<f32>, bool)>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

// 使用静态HashMap存储用户选择的替换文件路径
// 键是"文件路径:音频名称"，值是用户选择的替换文件路径
static REPLACEMENT_FILE_PATHS: Lazy<Mutex<HashMap<String, PathBuf>>> = Lazy::new(|| {
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
        
        // Create the key once
        let key = format!("{}:{}", audio_file_info.name, audio_file_info.id);
        
        // Store the replacement data in our static HashMap
        {
            let map_result = REPLACED_AUDIO_DATA.lock();
            if let Ok(mut map) = map_result {
                map.insert(key.clone(), replacement_data.clone());
            }
        }
        
        // Store the replacement file path
        {
            let path_buf = Path::new(replacement_file_path).to_path_buf();
            let map_result = REPLACEMENT_FILE_PATHS.lock();
            if let Ok(mut map) = map_result {
                map.insert(key, path_buf);
            }
        }
        
        // Get the filename for the new AudioFileInfo
        let filename = Path::new(replacement_file_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
            
        // Create a new AudioFileInfo with the replacement data
        let new_audio_info = AudioFileInfo {
            name: audio_file_info.name.clone(),
            id: audio_file_info.id.clone(),
            size: replacement_data.len(),
            filename,
            file_type: audio_file_info.file_type.clone(),
        };
        
        Ok(new_audio_info)
    }
    
    /// Process audio file with vgmstream-cli to add loop points
    pub fn process_with_vgmstream(
        file_path: &Path,
        loop_start: Option<f32>,
        loop_end: Option<f32>,
        use_custom_loop: bool
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
        // -i: Print metadata information
        // -L: Loop the file forever
        // -o: Output file path
        // -l / -f: Set loop start/end points if custom loop is enabled
        let mut command = Command::new(&vgmstream_path);
        
        #[cfg(windows)]
        {
            use winapi::um::winbase::CREATE_NO_WINDOW;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        
        // Start building arguments
        let mut args: Vec<String> = vec!["-i".to_string()];
        
        if use_custom_loop {
            if let Some(start) = loop_start {
                // Convert seconds to samples (assuming 48000Hz as a common sample rate)
                // This is a rough estimate - could be improved with actual sample rate detection
                let sample_rate = 48000.0;
                let start_sample = (start * sample_rate) as i32;
                args.push("-l".to_string());
                args.push(start_sample.to_string());
                
                if let Some(end) = loop_end {
                    let end_sample = (end * sample_rate) as i32;
                    args.push("-F".to_string());
                    args.push(end_sample.to_string());
                } else {
                    // If no end point specified, use -L for standard looping
                    args.push("-L".to_string());
                }
            } else {
                // If custom loop is enabled but no start point, just use standard looping
                args.push("-L".to_string());
            }
        } else {
            // Standard looping from beginning to end
            args.push("-L".to_string());
        }
        
        // Add output file path and input file path
        args.push("-o".to_string());
        args.push(temp_output_path_str);
        
        // 将路径转换为字符串，并将其所有权移入args向量
        let file_path_str = file_path.to_string_lossy().into_owned();
        args.push(file_path_str);
        
        // 将Vec<String>转换为Vec<&str>以传递给command.args()
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        
        let result = command.args(args_ref).output();
            
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

    /// Show file dialog to select replacement audio file and open the loop settings modal
    /// Does not replace anything in memory yet - this happens after loop settings are confirmed
    pub fn replace_with_file_dialog(
        audio_file_info: &AudioFileInfo, 
        loop_settings_modal: &mut LoopSettingsModal
    ) -> Result<AudioFileInfo, String> {
        // Open a file dialog to select the replacement audio file
        let result = FileDialog::new()
            .add_filter("Audio Files", &["wav", "mp3", "ogg", "lopus", "idsp", "bin"])
            .add_filter("All Files", &["*"])
            .set_title("Select Replacement Audio File")
            .pick_file();
        
        if result.is_none() {
            return Err("No file selected".to_string());
        }
        
        // Get selected file path
        // Make a clone so we own the path (prevents borrowing errors)
        let selected_path = result.unwrap().clone();
        
        // Extract filename safely as a string
        let mut filename = String::from("unknown");
        if let Some(name) = selected_path.file_name() {
            if let Some(name_str) = name.to_str() {
                filename = name_str.to_string();
            }
        }
        
        // Create key for hashmaps - Use the original audio name and ID
        let map_key = format!("{}:{}", audio_file_info.name, audio_file_info.id);
        
        // Store file path only - no audio data is replaced yet
        let replacement_path = selected_path.clone();
        if let Ok(mut map) = REPLACEMENT_FILE_PATHS.lock() {
            map.insert(map_key.clone(), replacement_path);
        }
        
        // Initialize with empty loop settings
        let empty_loop_settings = (None, None, false);
        if let Ok(mut settings) = LOOP_SETTINGS.lock() {
            settings.insert(map_key, empty_loop_settings);
        }
        
        // 创建一个新的AudioFileInfo，但保持原始的name和id
        // 只更新filename字段为新选择的文件
        let new_audio_info = AudioFileInfo {
            name: audio_file_info.name.clone(), // 保持原始name
            id: audio_file_info.id.clone(),     // 保持原始id
            size: audio_file_info.size,         // 保持原始大小
            filename,                 // 显示新文件名
            file_type: audio_file_info.file_type.clone(),
        };
        
        // 打开modal并传递新选择的音频信息
        loop_settings_modal.open_with_audio(new_audio_info.clone(), selected_path.to_str().unwrap_or(""));
        
        Ok(new_audio_info)
    }
    /// Process the replacement after loop settings are confirmed
    pub fn process_replacement_with_loop_settings(
        audio_file_info: &AudioFileInfo,
        file_path: Option<&Path>,
        loop_start: Option<f32>,
        loop_end: Option<f32>,
        use_custom_loop: bool
    ) -> Result<AudioFileInfo, String> {
        // 打印调试信息
        println!("Attempting to process replacement for: {} (ID: {})", audio_file_info.name, audio_file_info.id);
        
        // Create key for hashmaps - Use the original audio name and ID
        let key = format!("{}:{}", audio_file_info.name, audio_file_info.id);
        println!("Using hashmap key: {}", key);
        
        // Get the file path from the provided path or from the stored paths
        let actual_file_path = if let Some(path) = file_path {
            println!("Using provided file path: {:?}", path);
            path.to_path_buf()
        } else {
            // 打印存储的所有文件路径键，用于诊断
            if let Ok(map) = REPLACEMENT_FILE_PATHS.lock() {
                println!("Available replacement files in storage:");
                for (k, v) in map.iter() {
                    println!("  Key: {}, Path: {:?}", k, v);
                }
            }
            
            if let Ok(map) = REPLACEMENT_FILE_PATHS.lock() {
                if let Some(path) = map.get(&key) {
                    println!("Found stored file path: {:?}", path);
                    path.clone()
                } else {
                    return Err(format!("No replacement file path found for key: {}", key));
                }
            } else {
                return Err("Failed to access replacement file paths".to_string());
            }
        };
        
        println!("Using actual file path: {:?}", actual_file_path);
        
        // Process the selected file with vgmstream to add loop points
        let processed_path = match Self::process_with_vgmstream(&actual_file_path, loop_start, loop_end, use_custom_loop) {
            Ok(path) => path,
            Err(e) => {
                println!("Warning: Failed to process file with vgmstream: {}", e);
                println!("Falling back to original file");
                // Fall back to the original file if processing fails
                actual_file_path.clone()
            }
        };
        
        // Replace the audio file with the processed file in memory only
        let result = Self::replace_in_memory(audio_file_info, processed_path.to_str().unwrap());
        
        // Store loop settings
        if let Ok(mut settings) = LOOP_SETTINGS.lock() {
            settings.insert(key, (loop_start, loop_end, use_custom_loop));
        }
        
        // Clean up temporary file if it's different from the original
        if processed_path != actual_file_path && processed_path.exists() {
            // 使用 let 绑定来延长临时值的生命周期
            let remove_result = fs::remove_file(&processed_path);
            if let Err(e) = remove_result {
                println!("Warning: Failed to remove temporary file: {}", e);
            }
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

    /// Check if there are any replacement data stored
    pub fn has_replacement_data() -> bool {
        if let Ok(map) = REPLACED_AUDIO_DATA.lock() {
            !map.is_empty()
        } else {
            false
        }
    }

    /// Get the number of replacement data stored
    pub fn get_replacement_count() -> usize {
        if let Ok(map) = REPLACED_AUDIO_DATA.lock() {
            map.len()
        } else {
            0
        }
    }

    /// Get a reference to the loop settings map
    pub fn get_loop_settings() -> Result<std::sync::MutexGuard<'static, HashMap<String, (Option<f32>, Option<f32>, bool)>>, String> {
        if let Ok(settings) = LOOP_SETTINGS.lock() {
            Ok(settings)
        } else {
            Err("Failed to access loop settings".to_string())
        }
    }

    /// Clear all replacement data from memory
    pub fn clear_replacements() {
        if let Ok(mut map) = REPLACED_AUDIO_DATA.lock() {
            map.clear();
            println!("Cleared all audio replacements from memory");
        }
        
        if let Ok(mut settings) = LOOP_SETTINGS.lock() {
            settings.clear();
            println!("Cleared all loop settings from memory");
        }
        
        if let Ok(mut paths) = REPLACEMENT_FILE_PATHS.lock() {
            paths.clear();
            println!("Cleared all replacement file paths from memory");
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
        
        // Also apply all pending additions from Nus3audioFileUtils
        use super::nus3audio_file_utils::Nus3audioFileUtils;
        let pending_additions = Nus3audioFileUtils::get_pending_additions();
        for (id, name, data) in pending_additions {
            // Convert ID to u32
            let id_val = match id.parse::<u32>() {
                Ok(val) => val,
                Err(_) => continue, // Skip if ID is invalid
            };
            
            // Add the new audio file
            nus3_file.files.push(AudioFile {
                id: id_val,
                name: name.clone(),
                data: data.clone(),
            });
            println!("Added audio file: {} (ID: {})", name, id);
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
