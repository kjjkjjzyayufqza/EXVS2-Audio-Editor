use super::audio_file_info::AudioFileInfo;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::process::Command;

/// Utility functions for exporting audio files
pub struct ExportUtils;

impl ExportUtils {
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
        let result = Command::new(&vgmstream_path)
            .args(&[
                "-o",
                &temp_output_path_str,
                "-s",
                &(audio_file_info.id.parse::<i32>().unwrap() + 1).to_string(), // start with index 1
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
        let result = Command::new(vgmstream_path)
            .args(&[
                "-o",
                &output_path_str,
                "-s",
                &(audio_file_info.id.parse::<i32>().unwrap() + 1).to_string(), // start with index 1
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
            }
            Err(e) => return Err(format!("Failed to run vgmstream-cli for info: {}", e)),
        };
        println!("vgmstream-cli output: {}", info_output);
        // Parse subsong count from the output
        let subsong_count = if info_output.contains("stream count: ") {
            let subsongs_line = info_output
                .lines()
                .find(|line| line.contains("stream count: "))
                .unwrap_or("stream count: 1");

            let count_str = subsongs_line
                .split("stream count: ")
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
            let output_path = output_dir_path;
            let output_path_str = output_path.to_string_lossy().to_string();

            let result = Command::new(&vgmstream_path)
                .args(&[
                    "-o",
                    &output_path_str,
                    "-s",
                    &subsong_id_str,
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
                            "vgmstream-cli error on subsong {}: {}",
                            subsong_id, error
                        ));
                    }
                }
                Err(e) => {
                    return Err(format!(
                        "Failed to run vgmstream-cli for subsong {}: {}",
                        subsong_id, e
                    ));
                }
            }
        }

        Ok(exported_paths)
    }
}
