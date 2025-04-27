use super::audio_file_info::AudioFileInfo;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::process::Command;
use nus3audio::Nus3audioFile;

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
        
        // First, load the nus3audio file to get audio file information
        let nus3audio_file = match Nus3audioFile::open(original_file_path) {
            Ok(file) => file,
            Err(e) => return Err(format!("Failed to load nus3audio file: {}", e)),
        };
        
        println!("Loaded nus3audio file with {} audio files", nus3audio_file.files.len());
        
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
            let result = Command::new(&vgmstream_path)
                .args(&[
                    "-o",
                    &output_path_str,
                    "-s",
                    &(audio_file.id + 1).to_string(), // vgmstream uses 1-based indexing
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
