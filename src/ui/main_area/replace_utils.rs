use super::audio_file_info::AudioFileInfo;
use super::loop_settings_modal::LoopSettingsModal;
use crate::nus3bank::{replace::Nus3bankReplacer};
use hound;
use nus3audio::{AudioFile, Nus3audioFile};
use once_cell::sync::Lazy;
use rfd::FileDialog;
use std::collections::HashMap;
use std::fs;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

// 使用静态HashMap存储替换后的音频数据
// 键是"文件路径:音频名称"，值是替换后的音频数据
static REPLACED_AUDIO_DATA: Lazy<Mutex<HashMap<String, Vec<u8>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// 使用静态HashMap存储循环设置
// 键是"文件路径:音频名称"，值是(loop_start, loop_end, use_custom_loop)
static LOOP_SETTINGS: Lazy<Mutex<HashMap<String, (Option<f32>, Option<f32>, bool)>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// 使用静态HashMap存储用户选择的替换文件路径
// 键是"文件路径:音频名称"，值是用户选择的替换文件路径
static REPLACEMENT_FILE_PATHS: Lazy<Mutex<HashMap<String, PathBuf>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Utility functions for replacing audio files
pub struct ReplaceUtils;

impl ReplaceUtils {
    /// Replace audio data in memory only (does not modify the actual file on disk)
    /// Supports both NUS3AUDIO and NUS3BANK files
    pub fn replace_in_memory(
        audio_file_info: &AudioFileInfo,
        replacement_file_path: &str,
    ) -> Result<AudioFileInfo, String> {
        // Load the replacement file data
        let replacement_data = match fs::read(replacement_file_path) {
            Ok(data) => data,
            Err(e) => return Err(format!("Failed to read replacement file: {}", e)),
        };

        // Create the key based on file type
        let key = if audio_file_info.is_nus3bank {
            // For NUS3BANK, use hex_id:name format
            format!("{}:{}", audio_file_info.hex_id.as_ref().unwrap_or(&audio_file_info.id), audio_file_info.name)
        } else {
            // For NUS3AUDIO, use original name:id format
            format!("{}:{}", audio_file_info.name, audio_file_info.id)
        };

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
            hex_id: audio_file_info.hex_id.clone(),
            is_nus3bank: audio_file_info.is_nus3bank,
        };

        Ok(new_audio_info)
    }

    /// Process audio file with vgmstream-cli to add loop points
    pub fn process_with_vgmstream(
        file_path: &Path,
        loop_start: Option<f32>,
        loop_end: Option<f32>,
        use_custom_loop: bool,
        enable_loop: bool,
    ) -> Result<PathBuf, String> {
        // Path to vgmstream-cli.exe in tools directory
        let vgmstream_path = Path::new("tools").join("vgmstream-cli.exe");
        if !vgmstream_path.exists() {
            return Err(format!("vgmstream-cli not found at {:?}", vgmstream_path));
        }

        // Create a temporary output file path
        let temp_dir = std::env::temp_dir();
        let stem = file_path.file_stem().unwrap_or_default().to_string_lossy();
        let temp_filename = format!("looping_{}.wav", stem);
        let temp_output_path = temp_dir.join(&temp_filename);
        let temp_output_path_str = temp_output_path.to_string_lossy().to_string();

        println!(
            "Processing with vgmstream: {:?} -> {:?}",
            file_path, temp_output_path
        );

        // Usage: vgmstream-cli [-o <outfile.wav>] [options] <infile> ...
        // Options:
        //     -o <outfile.wav>: name of output .wav file, default <infile>.wav
        //        <outfile> wildcards can be ?s=subsong, ?n=stream name, ?f=infile
        //     -m: print metadata only, don't decode
        //     -i: ignore looping information and play the whole stream once
        //     -l N.n: loop count, default 2.0
        //     -f N.n: fade time in seconds after N loops, default 10.0
        //     -d N.n: fade delay in seconds, default 0.0
        //     -F: don't fade after N loops and play the rest of the stream
        //     -e: set end-to-end looping (if file has no loop points)
        //     -E: force end-to-end looping even if file has real loop points
        //     -s N: select subsong N, if the format supports multiple subsongs
        //     -S N: select end subsong N (set 0 for 'all')
        //     -p: output to stdout (for piping into another program)
        //     -P: output to stdout even if stdout is a terminal
        //     -c: loop forever (continuously) to stdout
        //     -L: append a smpl chunk and create a looping wav
        //     -w: allow .wav in original sample format rather than mixing to PCM16
        //     -V: print version info and supported extensions as JSON
        //     -I: print requested file info as JSON
        //     -h: print all commands
        let mut command = Command::new(&vgmstream_path);

        #[cfg(windows)]
        {
            use winapi::um::winbase::CREATE_NO_WINDOW;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        // Start building arguments
        let mut args: Vec<String> = vec!["-i".to_string()];

        // Add loop functionality only if enabled
        if enable_loop {
            // Enable loop from beginning to end
            args.push("-e".to_string());

            // Add smpl chunk
            args.push("-L".to_string());
        }

        // Add output file path and input file path
        args.push("-o".to_string());
        args.push(temp_output_path_str);

        // 将路径转换为字符串，并将其所有权移入args向量
        let file_path_str = file_path.to_string_lossy().into_owned();
        args.push(file_path_str);

        println!(
            "Running command: {:?} {}",
            vgmstream_path,
            args.join(" ")
        );

        // 将Vec<String>转换为Vec<&str>以传递给command.args()
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        let result = command.args(args_ref).output();
        println!("vgmstream-cli command result: {:?}", result);

        match result {
            Ok(output) => {
                if output.status.success() {
                    println!(
                        "Successfully processed file with vgmstream: {:?}",
                        temp_output_path
                    );
                    
                    // Apply custom loop points if specified (after vgmstream processing)
                    if use_custom_loop {
                        if let Some(start) = loop_start {
                            // Get the sample rate from the processed WAV file
                            let sample_rate = Self::get_wav_sample_rate(&temp_output_path)?;
                            let start_sample = (start * sample_rate as f32) as u32;
                            
                            let end_sample = if let Some(end) = loop_end {
                                (end * sample_rate as f32) as u32
                            } else {
                                // If no end specified, use the total samples
                                Self::get_wav_total_samples(&temp_output_path)?
                            };
                            
                            // Modify the WAV file's smpl chunk with custom loop points
                            Self::modify_wav_smpl_chunk(&temp_output_path, start_sample, end_sample)?;
                            
                            println!("Applied custom loop points: start={} samples, end={} samples", start_sample, end_sample);
                        }
                    }
                    
                    Ok(temp_output_path)
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    Err(format!("vgmstream-cli error: {}", error))
                }
            }
            Err(e) => Err(format!("Failed to run vgmstream-cli: {}", e)),
        }
    }

    /// Get stored replacement file path for a specific audio key (name:id)
    pub fn get_replacement_path(audio_name: &str, audio_id: &str) -> Option<PathBuf> {
        let key = format!("{}:{}", audio_name, audio_id);
        if let Ok(map) = REPLACEMENT_FILE_PATHS.lock() {
            map.get(&key).cloned()
        } else {
            None
        }
    }

    /// Apply gain in decibels to a WAV file and write to a new temporary WAV file
    fn apply_wav_gain(input_path: &Path, gain_db: f32) -> Result<PathBuf, String> {
        if gain_db.abs() < std::f32::EPSILON {
            return Ok(input_path.to_path_buf());
        }

        let gain = 10f32.powf(gain_db / 20.0);

        // Open reader
        let mut reader = hound::WavReader::open(input_path)
            .map_err(|e| format!("Failed to open WAV for gain: {}", e))?;
        let spec = reader.spec();

        // Prepare output path
        let parent_dir: PathBuf = input_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::temp_dir());
        let out_path = parent_dir.join(format!(
            "gain_{}",
            input_path.file_name().unwrap_or_default().to_string_lossy()
        ));

        let mut writer = hound::WavWriter::create(&out_path, spec)
            .map_err(|e| format!("Failed to create output WAV: {}", e))?;

        match (spec.sample_format, spec.bits_per_sample) {
            (hound::SampleFormat::Int, 16) => {
                for s in reader.samples::<i16>() {
                    let v = s.map_err(|e| format!("Read sample error: {}", e))? as f32 / 32768.0;
                    let scaled = (v * gain).clamp(-1.0, 1.0);
                    let out = (scaled * 32767.0).round() as i16;
                    writer
                        .write_sample(out)
                        .map_err(|e| format!("Write sample error: {}", e))?;
                }
            }
            (hound::SampleFormat::Float, 32) => {
                for s in reader.samples::<f32>() {
                    let v = s.map_err(|e| format!("Read sample error: {}", e))?;
                    let out = (v * gain).clamp(-1.0, 1.0);
                    writer
                        .write_sample(out)
                        .map_err(|e| format!("Write sample error: {}", e))?;
                }
            }
            _ => {
                return Err(format!(
                    "Unsupported WAV format: {:?} {}-bit",
                    spec.sample_format, spec.bits_per_sample
                ));
            }
        }

        writer
            .finalize()
            .map_err(|e| format!("Finalize WAV error: {}", e))?;
        Ok(out_path)
    }

    /// Show file dialog to select replacement audio file and open the loop settings modal
    /// Does not replace anything in memory yet - this happens after loop settings are confirmed
    pub fn replace_with_file_dialog(
        audio_file_info: &AudioFileInfo,
        loop_settings_modal: &mut LoopSettingsModal,
    ) -> Result<AudioFileInfo, String> {
        // Open a file dialog to select the replacement audio file
        let result = FileDialog::new()
            .add_filter(
                "Audio Files",
                &["wav", "mp3", "flac", "ogg", "lopus", "idsp", "bin"],
            )
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
            filename,                           // 显示新文件名
            file_type: audio_file_info.file_type.clone(),
            hex_id: audio_file_info.hex_id.clone(),
            is_nus3bank: audio_file_info.is_nus3bank,
        };

        // 打开modal并传递新选择的音频信息
        loop_settings_modal
            .open_with_audio(new_audio_info.clone(), selected_path.to_str().unwrap_or(""));

        Ok(new_audio_info)
    }
    /// Process the replacement after loop settings are confirmed
    pub fn process_replacement_with_loop_settings(
        audio_file_info: &AudioFileInfo,
        file_path: Option<&Path>,
        loop_start: Option<f32>,
        loop_end: Option<f32>,
        use_custom_loop: bool,
        enable_loop: bool,
        gain_db: f32,
    ) -> Result<AudioFileInfo, String> {
        // 打印调试信息
        println!(
            "Attempting to process replacement for: {} (ID: {})",
            audio_file_info.name, audio_file_info.id
        );

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

        // Apply gain first if requested
        let gain_processed_path = if gain_db.abs() > std::f32::EPSILON {
            match Self::apply_wav_gain(&actual_file_path, gain_db) {
                Ok(p) => {
                    println!("Successfully applied gain to file: {:?}", p);
                    p
                },
                Err(e) => {
                    println!(
                        "Warning: Failed to apply gain: {}. Using original file.",
                        e
                    );
                    actual_file_path.clone()
                }
            }
        } else {
            actual_file_path.clone()
        };

        // Then process the gain-adjusted file with vgmstream to add loop points
        let final_path = match Self::process_with_vgmstream(
            &gain_processed_path,
            loop_start,
            loop_end,
            use_custom_loop,
            enable_loop,
        ) {
            Ok(path) => path,
            Err(e) => {
                println!("Warning: Failed to process file with vgmstream: {}", e);
                println!("Falling back to gain-processed file");
                // Fall back to the gain-processed file if vgmstream processing fails
                gain_processed_path.clone()
            }
        };

        // Replace the audio file with the final processed file (gain-applied then vgmstream-processed) in memory only
        let result = Self::replace_in_memory(audio_file_info, final_path.to_str().unwrap());

        // Store loop settings
        if let Ok(mut settings) = LOOP_SETTINGS.lock() {
            settings.insert(key, (loop_start, loop_end, use_custom_loop));
        }

        // Clean up temporary files if they are different from the original
        if gain_processed_path != actual_file_path && gain_processed_path.exists() {
            let _ = fs::remove_file(&gain_processed_path);
            println!("Cleaned up temporary gain file: {:?}", gain_processed_path);
        }
        if final_path != gain_processed_path && final_path != actual_file_path && final_path.exists() {
            let _ = fs::remove_file(&final_path);
            println!("Cleaned up temporary vgmstream file: {:?}", final_path);
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

    /// Get the replacement audio data for a specific audio file (unified for both file types)
    pub fn get_replacement_data_unified(audio_file_info: &AudioFileInfo) -> Option<Vec<u8>> {
        // Create the correct key based on file type
        let key = if audio_file_info.is_nus3bank {
            // For NUS3BANK, use hex_id:name format
            format!("{}:{}", audio_file_info.hex_id.as_ref().unwrap_or(&audio_file_info.id), audio_file_info.name)
        } else {
            // For NUS3AUDIO, use original name:id format
            format!("{}:{}", audio_file_info.name, audio_file_info.id)
        };
        
        // Also try with ADD_ prefix for NUS3BANK files (for newly added audio)
        let add_key = if audio_file_info.is_nus3bank {
            format!("ADD_{}:{}", audio_file_info.hex_id.as_ref().unwrap_or(&audio_file_info.id), audio_file_info.name)
        } else {
            key.clone()
        };
        
        println!("Looking for replacement data with key: {} or {}", key, add_key);
        if let Ok(map) = REPLACED_AUDIO_DATA.lock() {
            // Try regular key first, then ADD_ prefixed key
            let result = map.get(&key).cloned().or_else(|| map.get(&add_key).cloned());
            if result.is_some() {
                println!("Found replacement data for audio: {}", audio_file_info.name);
            } else {
                println!("No replacement data found for keys: {} or {}", key, add_key);
                println!("Available keys in replacement data:");
                for stored_key in map.keys() {
                    println!("  - {}", stored_key);
                }
            }
            result
        } else {
            println!("Failed to access replacement data map");
            None
        }
    }

    /// Store audio data for playback (used by NUS3BANK add operations)
    pub fn store_audio_data_for_playback(key: String, audio_data: Vec<u8>) -> Result<(), String> {
        if let Ok(mut map) = REPLACED_AUDIO_DATA.lock() {
            map.insert(key, audio_data);
            Ok(())
        } else {
            Err("Failed to store audio data for playback".to_string())
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
    pub fn get_loop_settings() -> Result<
        std::sync::MutexGuard<'static, HashMap<String, (Option<f32>, Option<f32>, bool)>>,
        String,
    > {
        if let Ok(settings) = LOOP_SETTINGS.lock() {
            Ok(settings)
        } else {
            Err("Failed to access loop settings".to_string())
        }
    }

    /// Clear all replacement data from memory (unified for both file types)
    pub fn clear_replacements() {
        // Clear NUS3AUDIO replacements
        if let Ok(mut map) = REPLACED_AUDIO_DATA.lock() {
            map.clear();
            println!("Cleared all NUS3AUDIO replacements from memory");
        }

        if let Ok(mut settings) = LOOP_SETTINGS.lock() {
            settings.clear();
            println!("Cleared all loop settings from memory");
        }

        if let Ok(mut paths) = REPLACEMENT_FILE_PATHS.lock() {
            paths.clear();
            println!("Cleared all replacement file paths from memory");
        }
        
        // Clear NUS3BANK replacements
        Nus3bankReplacer::clear_replacements();
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

    /// Replace target audio in memory with an empty WAV buffer, preserving name and id
    pub fn replace_with_empty_wav_in_memory(
        audio_file_info: &AudioFileInfo,
        _nus3_path: &str,
    ) -> Result<AudioFileInfo, String> {
        // A minimal valid 44-byte WAV header with 0 data bytes (PCM mono 8kHz 16-bit)
        // This is sufficient for representing an empty/near-empty WAV for our in-memory replacement use-case
        const EMPTY_WAV_HEADER: [u8; 44] = [
            0x52, 0x49, 0x46, 0x46, // 'RIFF'
            0x24, 0x00, 0x00, 0x00, // Chunk size = 36 + data_size (0)
            0x57, 0x41, 0x56, 0x45, // 'WAVE'
            0x66, 0x6d, 0x74, 0x20, // 'fmt '
            0x10, 0x00, 0x00, 0x00, // Subchunk1Size = 16
            0x01, 0x00, // AudioFormat = PCM
            0x01, 0x00, // NumChannels = 1
            0x40, 0x1f, 0x00, 0x00, // SampleRate = 8000
            0x80, 0x3e, 0x00, 0x00, // ByteRate = SampleRate * NumChannels * BitsPerSample/8
            0x02, 0x00, // BlockAlign = NumChannels * BitsPerSample/8
            0x10, 0x00, // BitsPerSample = 16
            0x64, 0x61, 0x74, 0x61, // 'data'
            0x00, 0x00, 0x00, 0x00, // Subchunk2Size = 0
        ];

        let replacement_data = EMPTY_WAV_HEADER.to_vec();

        // Store into in-memory replacement map using consistent key format
        let key = if audio_file_info.is_nus3bank {
            // For NUS3BANK, use hex_id:name format (consistent with replace_in_memory)
            format!("{}:{}", audio_file_info.hex_id.as_ref().unwrap_or(&audio_file_info.id), audio_file_info.name)
        } else {
            // For NUS3AUDIO, use original name:id format
            format!("{}:{}", audio_file_info.name, audio_file_info.id)
        };
        if let Ok(mut map) = REPLACED_AUDIO_DATA.lock() {
            map.insert(key, replacement_data.clone());
        }

        // Create a new AudioFileInfo reflecting the empty wav size and filename
        let new_audio_info = AudioFileInfo {
            name: audio_file_info.name.clone(),
            id: audio_file_info.id.clone(),
            size: replacement_data.len(),
            filename: format!("{}_empty.wav", audio_file_info.filename),
            file_type: "WAV Audio".to_string(),
            hex_id: audio_file_info.hex_id.clone(),
            is_nus3bank: audio_file_info.is_nus3bank,
        };

        Ok(new_audio_info)
    }

    /// Get the sample rate from a WAV file
    fn get_wav_sample_rate(wav_path: &Path) -> Result<u32, String> {
        let data = std::fs::read(wav_path)
            .map_err(|e| format!("Failed to read WAV file: {}", e))?;
        
        // Check for RIFF header (52 49 46 46)
        if data.len() < 44 || &data[0..4] != b"RIFF" {
            return Err("Invalid WAV file: missing RIFF header".to_string());
        }
        
        // Check for WAVE format
        if &data[8..12] != b"WAVE" {
            return Err("Invalid WAV file: not WAVE format".to_string());
        }
        
        // Find fmt chunk and extract sample rate
        let mut offset = 12;
        while offset + 8 <= data.len() {
            let chunk_id = &data[offset..offset + 4];
            let chunk_size = u32::from_le_bytes([
                data[offset + 4], data[offset + 5], 
                data[offset + 6], data[offset + 7]
            ]);
            
            if chunk_id == b"fmt " {
                if offset + 8 + 24 <= data.len() {
                    // Sample rate is at offset 24 in fmt chunk (offset + 8 + 24 - 8 = offset + 24)
                    let sample_rate = u32::from_le_bytes([
                        data[offset + 8 + 4], data[offset + 8 + 5],
                        data[offset + 8 + 6], data[offset + 8 + 7]
                    ]);
                    return Ok(sample_rate);
                }
                break;
            }
            
            offset += 8 + chunk_size as usize;
            // Ensure 16-bit alignment
            if chunk_size % 2 != 0 {
                offset += 1;
            }
        }
        
        Err("Could not find fmt chunk in WAV file".to_string())
    }

    /// Get the total samples from a WAV file
    fn get_wav_total_samples(wav_path: &Path) -> Result<u32, String> {
        let data = std::fs::read(wav_path)
            .map_err(|e| format!("Failed to read WAV file: {}", e))?;
        
        // Check for RIFF header
        if data.len() < 44 || &data[0..4] != b"RIFF" {
            return Err("Invalid WAV file: missing RIFF header".to_string());
        }
        
        let mut fmt_chunk_info: Option<(u16, u32, u16)> = None; // (channels, sample_rate, bits_per_sample)
        let mut data_chunk_size: Option<u32> = None;
        
        // Parse chunks to find fmt and data
        let mut offset = 12; // Skip RIFF header
        while offset + 8 <= data.len() {
            let chunk_id = &data[offset..offset + 4];
            let chunk_size = u32::from_le_bytes([
                data[offset + 4], data[offset + 5], 
                data[offset + 6], data[offset + 7]
            ]);
            
            if chunk_id == b"fmt " && offset + 8 + 16 <= data.len() {
                let channels = u16::from_le_bytes([data[offset + 8 + 2], data[offset + 8 + 3]]);
                let sample_rate = u32::from_le_bytes([
                    data[offset + 8 + 4], data[offset + 8 + 5],
                    data[offset + 8 + 6], data[offset + 8 + 7]
                ]);
                let bits_per_sample = u16::from_le_bytes([data[offset + 8 + 14], data[offset + 8 + 15]]);
                fmt_chunk_info = Some((channels, sample_rate, bits_per_sample));
            } else if chunk_id == b"data" {
                data_chunk_size = Some(chunk_size);
            }
            
            offset += 8 + chunk_size as usize;
            if chunk_size % 2 != 0 {
                offset += 1;
            }
        }
        
        match (fmt_chunk_info, data_chunk_size) {
            (Some((channels, _sample_rate, bits_per_sample)), Some(data_size)) => {
                let bytes_per_sample = (bits_per_sample / 8) as u32;
                let total_samples = data_size / (channels as u32 * bytes_per_sample);
                Ok(total_samples)
            }
            _ => Err("Could not find required chunks to calculate total samples".to_string())
        }
    }

    /// Modify the smpl chunk in a WAV file to set custom loop points
    fn modify_wav_smpl_chunk(wav_path: &Path, start_sample: u32, end_sample: u32) -> Result<(), String> {
        let mut data = std::fs::read(wav_path)
            .map_err(|e| format!("Failed to read WAV file: {}", e))?;
        
        // Check for RIFF header
        if data.len() < 12 || &data[0..4] != b"RIFF" {
            return Err("Invalid WAV file: missing RIFF header".to_string());
        }
        
        // Find smpl chunk at 0x24 offset
        if data.len() < 0x24 + 4 {
            return Err("WAV file too small to contain smpl chunk".to_string());
        }
        
        // Check if smpl chunk exists at expected position (0x24)
        if &data[0x24..0x24 + 4] != b"smpl" {
            return Err("smpl chunk not found at expected position 0x24".to_string());
        }
        
        // Verify we have enough space for the loop points
        if data.len() < 0x58 + 8 {
            return Err("WAV file too small to contain loop point data".to_string());
        }
        
        // Write start_sample at 0x58
        let start_bytes = start_sample.to_le_bytes();
        data[0x58] = start_bytes[0];
        data[0x59] = start_bytes[1];
        data[0x5A] = start_bytes[2];
        data[0x5B] = start_bytes[3];
        
        // Write end_sample at 0x5C
        let end_bytes = end_sample.to_le_bytes();
        data[0x5C] = end_bytes[0];
        data[0x5D] = end_bytes[1];
        data[0x5E] = end_bytes[2];
        data[0x5F] = end_bytes[3];
        
        // Save the modified WAV file
        std::fs::write(wav_path, &data)
            .map_err(|e| format!("Failed to write modified WAV file: {}", e))?;
        
        println!("Successfully modified smpl chunk: loop start={}, end={}", start_sample, end_sample);
        Ok(())
    }
    
    /// Apply all in-memory replacements and save (unified for both file types)
    pub fn apply_replacements_and_save_unified(
        original_file_path: &str,
        save_path: &str,
    ) -> Result<(), String> {
        if original_file_path.to_lowercase().ends_with(".nus3bank") {
            // Handle NUS3BANK files
            // Bridge UI in-memory replacements into Nus3bankReplacer cache
            // Handle both "hex_id:name" and "name:hex_id" key formats
            // Skip ADD_ prefixed keys (handled by Add operations)
            if let Ok(map) = REPLACED_AUDIO_DATA.lock() {
                for (key, replacement_data) in map.iter() {
                    // Skip ADD_ prefixed keys (these are handled by Add operations)
                    if key.starts_with("ADD_") {
                        println!("Skipping ADD_ prefixed key: {}", key);
                        continue;
                    }
                    
                    let parts: Vec<&str> = key.split(':').collect();
                    if parts.len() != 2 { continue; }
                    let left = parts[0];
                    let right = parts[1];
                    
                    // Check both possible hex_id positions
                    let hex_id = if left.starts_with("0x") {
                        left
                    } else if right.starts_with("0x") {
                        right
                    } else {
                        continue; // Skip if no hex_id found
                    };
                    
                    // Feed into Nus3bankReplacer using the current file path scope
                    let _ = Nus3bankReplacer::replace_track_in_memory(
                        original_file_path,
                        hex_id,
                        replacement_data.clone(),
                    );
                }
            }

            // Apply NUS3BANK operations if any
            if crate::nus3bank::replace::Nus3bankReplacer::has_replacement_data() {
                let mut nus3bank_file = crate::nus3bank::structures::Nus3bankFile::open(original_file_path)
                    .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;
                
                crate::nus3bank::replace::Nus3bankReplacer::apply_to_file(&mut nus3bank_file)
                    .map_err(|e| format!("Failed to apply NUS3BANK operations: {}", e))?;
                
                nus3bank_file.save(save_path)
                    .map_err(|e| format!("Failed to save NUS3BANK file: {}", e))?;
                
                crate::nus3bank::replace::Nus3bankReplacer::clear();
                return Ok(());
            }

            Nus3bankReplacer::apply_replacements_and_save(original_file_path, save_path)
        } else {
            // Handle NUS3AUDIO files (original implementation)
            Self::apply_replacements_and_save(original_file_path, save_path)
        }
    }
    
    /// Replace audio data in memory for NUS3BANK files
    pub fn replace_nus3bank_in_memory(
        file_path: &str,
        audio_file_info: &AudioFileInfo,
        replacement_file_path: &str,
    ) -> Result<(), String> {
        // Load the replacement file data
        let replacement_data = fs::read(replacement_file_path)
            .map_err(|e| format!("Failed to read replacement file: {}", e))?;
        
        let hex_id = audio_file_info.hex_id.as_ref()
            .ok_or_else(|| "No hex ID found for NUS3BANK track".to_string())?;
        
        Nus3bankReplacer::replace_track_in_memory(file_path, hex_id, replacement_data)
    }
}
