use super::add_audio_modal::AddAudioModal;
use super::audio_file_info::AudioFileInfo;
use super::replace_utils::ReplaceUtils;
use crate::{Locale, localized};
use rfd::FileDialog;
use std::fs;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

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

        println!(
            "Converting to WAV: {:?} -> {:?}",
            file_path, temp_output_path
        );

        // Run vgmstream-cli to convert audio to WAV
        let mut command = Command::new(&vgmstream_path);

        #[cfg(windows)]
        {
            use winapi::um::winbase::CREATE_NO_WINDOW;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        println!(
            "Running command: {:?} -o {} {}",
            vgmstream_path, temp_output_path_str, file_path
        );

        let result = command
            .args(["-o", &temp_output_path_str, file_path])
            .output();
        println!("vgmstream-cli command result: {:?}", result);

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
        locale: Locale,
    ) -> Result<(), String> {
        // Open a file dialog to select the audio file
        let result: Option<std::path::PathBuf> = FileDialog::new()
            .add_filter(
                localized::audio_files_filter(),
                &["wav", "mp3", "flac", "ogg", "lopus", "idsp", "bin"],
            )
            .add_filter(localized::all_files_filter(), &["*"])
            .set_title(localized::select_audio_file_to_add())
            .pick_file();

        if result.is_none() {
            return Err(localized::no_file_selected().to_string());
        }

        // Get selected file path
        let selected_path = result.unwrap();
        let path_str = selected_path.to_string_lossy().to_string();

        // Open the modal with the selected file
        add_audio_modal.open_with_file(&path_str, existing_audio_files, locale);

        Ok(())
    }

    fn apply_gain_to_wav_bytes(wav_data: Vec<u8>, gain_db: f32) -> Result<Vec<u8>, String> {
        if gain_db.abs() < f32::EPSILON {
            return Ok(wav_data);
        }

        let temp_dir = std::env::temp_dir();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let temp_path = temp_dir.join(format!("add_gain_src_{}.wav", nonce));
        fs::write(&temp_path, &wav_data)
            .map_err(|e| format!("Failed to write temp WAV for gain: {}", e))?;

        let gained_path = match ReplaceUtils::apply_wav_gain(&temp_path, gain_db) {
            Ok(path) => path,
            Err(e) => {
                let _ = fs::remove_file(&temp_path);
                return Err(e);
            }
        };

        let gained = fs::read(&gained_path)
            .map_err(|e| format!("Failed to read gained WAV: {}", e));
        let _ = fs::remove_file(&temp_path);
        if gained_path != temp_path {
            let _ = fs::remove_file(&gained_path);
        }
        gained
    }

    /// Process the new audio file after the modal is confirmed
    /// The is_nus3bank flag will be determined by the caller based on the current file type
    pub fn process_new_audio(
        add_audio_modal: &AddAudioModal,
        is_nus3bank: bool,
        _locale: Locale,
    ) -> Result<(AudioFileInfo, Vec<u8>), String> {
        // Check if file path exists
        let file_path = match &add_audio_modal.settings.file_path {
            Some(path) => path,
            None => return Err(localized::no_audio_path().to_string()),
        };

        let gain_db = add_audio_modal.settings.gain_db;

        // Convert the audio file to WAV format using vgmstream
        let file_data = match Self::convert_to_wav(file_path) {
            Ok(wav_data) => wav_data,
            Err(e) => {
                if gain_db.abs() > f32::EPSILON {
                    return Err(format!(
                        "Cannot apply gain: WAV conversion failed: {}",
                        e
                    ));
                }
                println!("Warning: Failed to convert to WAV: {}", e);
                println!("Falling back to original file data");
                match &add_audio_modal.file_data {
                    Some(data) => data.clone(),
                    None => return Err(localized::no_audio_data().to_string()),
                }
            }
        };

        let file_data = Self::apply_gain_to_wav_bytes(file_data, gain_db)?;

        // Get name and ID from settings
        let name = add_audio_modal.settings.name.clone();
        let id = add_audio_modal.settings.id.clone();

        if name.is_empty() || id.is_empty() {
            return Err(localized::name_and_id_required().to_string());
        }

        // Convert ID to valid format expected by Nus3audioFile
        let id_val = match id.parse::<u32>() {
            Ok(val) => val,
            Err(_) => return Err(localized::id_must_be_valid_number().to_string()),
        };

        // Get the filename from the original file path
        let filename = Path::new(file_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // NUS3BANK hex_id is reserved later by Nus3bankReplacer::register_add.
        let new_audio_info = AudioFileInfo {
            name,
            id: id_val.to_string(),
            size: file_data.len(),
            filename: format!(
                "{}.wav",
                Path::new(&filename)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
            ),
            file_type: "WAV Audio".to_string(),
            hex_id: None,
            is_nus3bank,
        };

        Ok((new_audio_info, file_data))
    }
}
