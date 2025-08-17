use super::audio_file_info::AudioFileInfo;
use nus3audio::Nus3audioFile;
use std::fs;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

// Cache for indexing patterns to avoid re-analyzing the same file multiple times
static INDEXING_PATTERN_CACHE: Lazy<Mutex<HashMap<String, bool>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

/// Utility functions for exporting audio files
pub struct ExportUtils;

impl ExportUtils {
    /// Determine the correct vgmstream index based on the nus3audio file's indexing pattern
    /// 
    /// This function analyzes the nus3audio file to detect whether it uses:
    /// - 0-based indexing (0,1,2,3...) -> needs +1 conversion for vgmstream
    /// - 1-based indexing (1,2,3,4...) -> direct mapping to vgmstream
    /// 
    /// Uses caching to avoid re-analyzing the same file multiple times.
    fn get_vgmstream_index(
        audio_file_id: &str,
        original_file_path: &str,
    ) -> Result<String, String> {
        // Parse the audio file ID
        let id_num = audio_file_id.parse::<u32>()
            .map_err(|_| format!("Invalid audio file ID: {}", audio_file_id))?;
        
        // Check cache first
        let cache_key = original_file_path.to_string();
        let starts_from_zero = if let Ok(cache) = INDEXING_PATTERN_CACHE.lock() {
            if let Some(&cached_pattern) = cache.get(&cache_key) {
                println!("Using cached indexing pattern for {}: starts_from_zero={}", original_file_path, cached_pattern);
                cached_pattern
            } else {
                // Cache miss, need to analyze the file
                drop(cache); // Release the lock before file operations
                
                // Load the nus3audio file to analyze the indexing pattern
                let nus3_file = Nus3audioFile::open(original_file_path)
                    .map_err(|e| format!("Failed to open nus3audio file: {}", e))?;
                
                if nus3_file.files.is_empty() {
                    return Err("No audio files found in nus3audio file".to_string());
                }
                
                // Collect all IDs and sort them
                let mut all_ids: Vec<u32> = nus3_file.files.iter().map(|f| f.id).collect();
                all_ids.sort();
                
                // Determine the indexing pattern
                let pattern = all_ids[0] == 0;
                
                println!("Analyzed indexing pattern for {}: IDs={:?}, starts_from_zero={}", 
                        original_file_path, all_ids, pattern);
                
                // Cache the result
                if let Ok(mut cache) = INDEXING_PATTERN_CACHE.lock() {
                    cache.insert(cache_key, pattern);
                }
                
                pattern
            }
        } else {
            // Fallback if cache lock fails - analyze without caching
            println!("Warning: Failed to access indexing pattern cache, analyzing without caching");
            
            let nus3_file = Nus3audioFile::open(original_file_path)
                .map_err(|e| format!("Failed to open nus3audio file: {}", e))?;
            
            if nus3_file.files.is_empty() {
                return Err("No audio files found in nus3audio file".to_string());
            }
            
            let mut all_ids: Vec<u32> = nus3_file.files.iter().map(|f| f.id).collect();
            all_ids.sort();
            all_ids[0] == 0
        };
        
        if starts_from_zero {
            // 0-based indexing: convert to 1-based for vgmstream
            // 0 -> 1, 1 -> 2, 2 -> 3, etc.
            let vgmstream_index = id_num + 1;
            println!("0-based indexing detected: {} -> {}", id_num, vgmstream_index);
            Ok(vgmstream_index.to_string())
        } else {
            // 1-based indexing: direct mapping
            // 1 -> 1, 2 -> 2, 3 -> 3, etc.
            println!("1-based indexing detected: {} -> {}", id_num, id_num);
            Ok(id_num.to_string())
        }
    }

    /// Convert audio to WAV format using vgmstream-cli and return the data in memory
    pub fn convert_to_wav_in_memory(
        audio_file_info: &AudioFileInfo,
        original_file_path: &str,
    ) -> Result<Vec<u8>, String> {
        // Path to vgmstream-cli.exe in tools directory
        let vgmstream_path = Path::new("tools").join("vgmstream-cli.exe");

        // Create a temporary output file path
        let temp_dir = std::env::temp_dir();
        let temp_filename = format!("temp_convert_{}.wav", audio_file_info.id);
        let temp_output_path = temp_dir.join(&temp_filename);
        let temp_output_path_str = temp_output_path.to_string_lossy().to_string();

        // Run vgmstream-cli to convert audio to WAV
        let mut command = Command::new(&vgmstream_path);

        #[cfg(windows)]
        {
            use winapi::um::winbase::CREATE_NO_WINDOW;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        // Get the correct vgmstream index using intelligent detection
        let vgmstream_index = Self::get_vgmstream_index(&audio_file_info.id, original_file_path)?;
        
        println!("Original ID: {}, Detected vgmstream index: {}", audio_file_info.id, vgmstream_index);
        println!("Temp output path: {:?}", temp_output_path);
        
        let result = command
            .args([
                "-i",
                "-o",
                &temp_output_path_str,
                "-s",
                &vgmstream_index,
                original_file_path,
            ])
            .output();

        //print the debug args
        println!("Exporting command: {:?}", result);

        match result {
            Ok(output) => {
                if output.status.success() {
                    // Read the temporary WAV file into memory
                    match fs::read(&temp_output_path) {
                        Ok(wav_data) => {
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

    /// Export audio data to a WAV file with custom output directory using vgmstream-cli
    pub fn export_to_wav_with_custom_dir(
        audio_file_info: &AudioFileInfo,
        original_file_path: &str,
        output_dir: &str,
    ) -> Result<String, String> {
        // Create output file path in the custom directory
        let output_dir_path = Path::new(output_dir);
        let output_filename = format!("{}.wav", audio_file_info.name);
        let output_path = output_dir_path.join(output_filename);
        let output_path_str = output_path.to_string_lossy().to_string();

        // Path to vgmstream-cli.exe in tools directory
        let vgmstream_path = Path::new("tools").join("vgmstream-cli.exe");

        // Run vgmstream-cli to convert audio to WAV
        let mut command = Command::new(vgmstream_path);

        #[cfg(windows)]
        {
            use winapi::um::winbase::CREATE_NO_WINDOW;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        // Get the correct vgmstream index using intelligent detection
        let vgmstream_index = Self::get_vgmstream_index(&audio_file_info.id, original_file_path)?;
        
        println!("Original ID: {}, Detected vgmstream index: {}", audio_file_info.id, vgmstream_index);

        let result = command
            .args([
                "-i",
                "-o",
                &output_path_str,
                "-s",
                &vgmstream_index,
                original_file_path,
            ])
            .output();

        println!("Exporting command: {:?}", result);
        match result {
            Ok(output) => {
                if output.status.success() {
                    println!("Successfully exported WAV file to: {:?}", output_path);
                    Ok(output_path_str)
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    Err(format!("vgmstream-cli error: {}", error))
                }
            }
            Err(e) => Err(format!("Failed to run vgmstream-cli: {}", e)),
        }
    }

    /// Export all audio files in a file to WAV files with custom output directory using vgmstream-cli
    pub fn export_all_to_wav(
        original_file_path: &str,
        output_dir: &str,
    ) -> Result<Vec<String>, String> {
        // Path to vgmstream-cli.exe in tools directory
        let vgmstream_path = Path::new("tools").join("vgmstream-cli.exe");

        // First, load the nus3audio file to get audio file information
        let nus3audio_file = match Nus3audioFile::open(original_file_path) {
            Ok(file) => file,
            Err(e) => return Err(format!("Failed to load nus3audio file: {}", e)),
        };

        println!(
            "Loaded nus3audio file with {} audio files",
            nus3audio_file.files.len()
        );

        let mut exported_paths = Vec::new();
        let output_dir_path = Path::new(output_dir);

        // Export each audio file directly using vgmstream-cli
        for audio_file in nus3audio_file.files.iter() {
            // Get the name for this audio file
            let audio_name = if audio_file.name.is_empty() {
                format!("audio_{}", audio_file.id)
            } else {
                audio_file.name.clone()
            };

            // Create output file path with the audio file name
            let output_filename = format!("{}.wav", audio_name);
            let output_path = output_dir_path.join(output_filename);
            let output_path_str = output_path.to_string_lossy().to_string();

            // Convert to WAV using vgmstream-cli with the subsong index
            let mut command = Command::new(&vgmstream_path);

            #[cfg(windows)]
            {
                use winapi::um::winbase::CREATE_NO_WINDOW;
                command.creation_flags(CREATE_NO_WINDOW);
            }

            // Get the correct vgmstream index using intelligent detection
            let vgmstream_index = match Self::get_vgmstream_index(&audio_file.id.to_string(), original_file_path) {
                Ok(index) => index,
                Err(e) => {
                    return Err(format!("Failed to determine vgmstream index for audio file {}: {}", audio_file.id, e));
                }
            };
            
            println!("Original ID: {}, Detected vgmstream index: {}", audio_file.id, vgmstream_index);

            let result = command
                .args([
                    "-o",
                    &output_path_str,
                    "-s",
                    &vgmstream_index,
                    original_file_path,
                ])
                .output();

            match result {
                Ok(output) => {
                    if output.status.success() {
                        println!("Successfully exported WAV file to: {:?}", output_path);
                        exported_paths.push(output_path_str);
                    } else {
                        let error = String::from_utf8_lossy(&output.stderr);
                        return Err(format!(
                            "vgmstream-cli error on audio file {}: {}",
                            audio_file.id, error
                        ));
                    }
                }
                Err(e) => {
                    return Err(format!(
                        "Failed to run vgmstream-cli for audio file {}: {}",
                        audio_file.id, e
                    ));
                }
            }
        }

        Ok(exported_paths)
    }
}
