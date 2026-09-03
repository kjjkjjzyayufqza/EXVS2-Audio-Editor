use super::structures::Nus3bankFile;
use std::path::Path;
use std::process::Command;

/// Produce a single Windows-safe filename without altering the bank's track name.
pub(crate) fn wav_filename(name: &str, fallback: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let stem = sanitized.trim().trim_end_matches(['.', ' ']);
    let stem = if stem.is_empty() { fallback } else { stem };
    let base = stem.split('.').next().unwrap_or(stem).to_ascii_uppercase();
    let reserved = matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            base.strip_prefix(prefix).is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2"
                        | "3"
                        | "4"
                        | "5"
                        | "6"
                        | "7"
                        | "8"
                        | "9"
                        | "\u{00B9}"
                        | "\u{00B2}"
                        | "\u{00B3}"
                )
            })
        });
    format!("{}{}.wav", if reserved { "_" } else { "" }, stem)
}

/// NUS3BANK export utilities
pub struct Nus3bankExporter;

impl Nus3bankExporter {
    /// Export a single track to WAV file
    pub fn export_track(file_path: &str, hex_id: &str, output_dir: &str) -> Result<String, String> {
        let nus3bank_file = Nus3bankFile::open(file_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;

        let track = nus3bank_file
            .get_track_by_hex_id(hex_id)
            .ok_or_else(|| format!("Track with hex ID {} not found", hex_id))?;

        Self::export_parsed_track(file_path, track, output_dir)
    }

    fn export_parsed_track(
        file_path: &str,
        track: &super::structures::AudioTrack,
        output_dir: &str,
    ) -> Result<String, String> {
        let filename = wav_filename(&format!("{}-{}", track.hex_id, track.name), "audio");
        let output_path = Path::new(output_dir).join(filename);
        let mut command = Command::new(Path::new("tools").join("vgmstream-cli.exe"));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(winapi::um::winbase::CREATE_NO_WINDOW);
        }
        let output = command
            .arg("-i")
            .arg("-o")
            .arg(&output_path)
            .arg("-s")
            .arg((track.index + 1).to_string())
            .arg(file_path)
            .output()
            .map_err(|e| format!("Failed to run vgmstream-cli: {}", e))?;
        if !output.status.success() {
            return Err(format!(
                "vgmstream-cli error: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        if !output_path.is_file() {
            return Err("vgmstream-cli did not create the WAV file".to_string());
        }
        Ok(output_path.to_string_lossy().into_owned())
    }

    /// Batch export all tracks from NUS3BANK
    pub fn export_all_tracks(file_path: &str, output_dir: &str) -> Result<Vec<String>, String> {
        let nus3bank_file = Nus3bankFile::open(file_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;

        let mut exported_files = Vec::new();
        let mut failures = Vec::new();

        for track in &nus3bank_file.tracks {
            match Self::export_parsed_track(file_path, track, output_dir) {
                Ok(path) => exported_files.push(path),
                Err(e) => failures.push(format!("{}: {}", track.hex_id, e)),
            }
        }
        if !failures.is_empty() {
            return Err(format!(
                "Exported {} tracks; failed: {}",
                exported_files.len(),
                failures.join("; ")
            ));
        }

        Ok(exported_files)
    }

    /// Export track to memory (for integration with existing systems)
    pub fn export_track_to_memory(file_path: &str, hex_id: &str) -> Result<Vec<u8>, String> {
        let nus3bank_file = Nus3bankFile::open(file_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;

        let track = nus3bank_file
            .get_track_by_hex_id(hex_id)
            .ok_or_else(|| format!("Track with hex ID {} not found", hex_id))?;

        track.audio_data.clone()
            .ok_or_else(|| format!("Audio data not loaded for track '{}' ({}). The track may be corrupted or the file may not have been parsed correctly.", track.name, track.hex_id))
    }
}
