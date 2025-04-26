use std::fs;
use std::path::Path;
use std::process::Command;
use super::audio_file_info::AudioFileInfo;

/// Utility functions for exporting audio files
pub struct ExportUtils;

impl ExportUtils {
    /// Export audio data to a WAV file using vgmstream-cli
    pub fn export_to_wav(
        audio_file_info: &AudioFileInfo,
        original_file_path: &str,
    ) -> Result<String, String> {
        // Create output file path (same directory as original file with .wav extension)
        let original_path = Path::new(original_file_path);
        let parent_dir = match original_path.parent() {
            Some(dir) => dir,
            None => return Err("Failed to get parent directory".to_string()),
        };
        
        let output_filename = format!("{}.wav", audio_file_info.name);
        let output_path = parent_dir.join(output_filename);
        let output_path_str = output_path.to_string_lossy().to_string();
        
        // Path to vgmstream-cli.exe in tools directory
        let vgmstream_path = Path::new("tools").join("vgmstream-cli.exe");
        
        // Run vgmstream-cli to convert audio to WAV
        let result = Command::new(vgmstream_path)
            .args(&["-o", &output_path_str, "-s", &audio_file_info.id, original_file_path])
            .output();
            
        match result {
            Ok(output) => {
                if output.status.success() {
                    println!("Successfully exported WAV file to: {:?}", output_path);
                    Ok(output_path_str)
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    Err(format!("vgmstream-cli error: {}", error))
                }
            },
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
        let result = Command::new(vgmstream_path)
            .args(&["-o", &output_path_str, "-s", &audio_file_info.id, original_file_path])
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
            },
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
        
        // First, get information about the file to know how many subsongs it has
        let info_result = Command::new(&vgmstream_path)
            .args(&["-m", original_file_path])
            .output();
        
        let info_output = match info_result {
            Ok(output) => {
                if output.status.success() {
                    String::from_utf8_lossy(&output.stdout).to_string()
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    return Err(format!("vgmstream-cli info error: {}", error));
                }
            },
            Err(e) => return Err(format!("Failed to run vgmstream-cli for info: {}", e)),
        };
        
        // Parse subsong count from the output
        let subsong_count = if info_output.contains("subsongs: ") {
            let subsongs_line = info_output
                .lines()
                .find(|line| line.contains("subsongs: "))
                .unwrap_or("subsongs: 1");
            
            let count_str = subsongs_line
                .split("subsongs: ")
                .nth(1)
                .unwrap_or("1")
                .trim();
                
            count_str.parse::<usize>().unwrap_or(1)
        } else {
            1 // Default to 1 if no subsong info found
        };
        
        let mut exported_paths = Vec::new();
        let output_dir_path = Path::new(output_dir);
        
        // Export each subsong
        for subsong_id in 1..=subsong_count {
            let subsong_id_str = subsong_id.to_string();
            let output_filename = format!("subsong_{}.wav", subsong_id);
            let output_path = output_dir_path.join(&output_filename);
            let output_path_str = output_path.to_string_lossy().to_string();
            
            let result = Command::new(&vgmstream_path)
                .args(&["-o", &output_path_str, "-s", &subsong_id_str, original_file_path])
                .output();
                
            match result {
                Ok(output) => {
                    if output.status.success() {
                        println!("Successfully exported WAV file to: {:?}", output_path);
                        exported_paths.push(output_path_str);
                    } else {
                        let error = String::from_utf8_lossy(&output.stderr);
                        return Err(format!("vgmstream-cli error on subsong {}: {}", subsong_id, error));
                    }
                },
                Err(e) => {
                    return Err(format!("Failed to run vgmstream-cli for subsong {}: {}", subsong_id, e));
                }
            }
        }
        
        Ok(exported_paths)
    }
}
