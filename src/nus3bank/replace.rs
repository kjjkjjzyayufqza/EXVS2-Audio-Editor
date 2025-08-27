use super::structures::Nus3bankFile;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

// Store NUS3BANK replacement data in memory (similar to existing NUS3AUDIO pattern)
static REPLACED_NUS3BANK_DATA: Lazy<Mutex<HashMap<String, Vec<u8>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// NUS3BANK replace utilities
pub struct Nus3bankReplacer;

impl Nus3bankReplacer {
    /// Replace track in memory only (does not modify the actual file on disk)
    pub fn replace_track_in_memory(
        file_path: &str,
        hex_id: &str,
        new_audio_data: Vec<u8>,
    ) -> Result<(), String> {
        // Store replacement in memory (similar to existing pattern)
        let key = format!("{}:{}", hex_id, file_path);

        if let Ok(mut map) = REPLACED_NUS3BANK_DATA.lock() {
            map.insert(key, new_audio_data);
            println!("Stored replacement data for NUS3BANK track: {}", hex_id);
            Ok(())
        } else {
            Err("Failed to acquire lock on replacement data".to_string())
        }
    }

    /// Get replacement data for a track
    pub fn get_replacement_data(file_path: &str, hex_id: &str) -> Option<Vec<u8>> {
        let key = format!("{}:{}", hex_id, file_path);
        if let Ok(map) = REPLACED_NUS3BANK_DATA.lock() {
            if let Some(v) = map.get(&key) {
                return Some(v.clone());
            }
            // Fallback: try case-insensitive and suffix path match to be robust across UI sources
            for (k, v) in map.iter() {
                let mut parts = k.splitn(2, ':');
                let k_hex = parts.next().unwrap_or("");
                let k_path = parts.next().unwrap_or("");
                if k_hex == hex_id
                    && (k_path == file_path
                        || k_path.eq_ignore_ascii_case(file_path)
                        || k_path.ends_with(file_path))
                {
                    return Some(v.clone());
                }
            }
            None
        } else {
            None
        }
    }

    /// Check if there are any NUS3BANK replacement data stored
    pub fn has_replacement_data() -> bool {
        if let Ok(map) = REPLACED_NUS3BANK_DATA.lock() {
            !map.is_empty()
        } else {
            false
        }
    }

    /// Get the number of NUS3BANK replacement data stored
    pub fn get_replacement_count() -> usize {
        if let Ok(map) = REPLACED_NUS3BANK_DATA.lock() {
            map.len()
        } else {
            0
        }
    }

    /// Clear all NUS3BANK replacement data from memory
    pub fn clear_replacements() {
        if let Ok(mut map) = REPLACED_NUS3BANK_DATA.lock() {
            map.clear();
            println!("Cleared all NUS3BANK audio replacements from memory");
        }
    }

    /// Clear only replacements for a specific file path
    pub fn clear_replacements_for_file(file_path: &str) {
        if let Ok(mut map) = REPLACED_NUS3BANK_DATA.lock() {
            let file_path_lower = file_path.to_ascii_lowercase();
            let keys: Vec<String> = map.keys().cloned().collect();
            for k in keys {
                if let Some((k_hex, k_path)) = k.split_once(':') {
                    if k_path == file_path
                        || k_path.eq_ignore_ascii_case(file_path)
                        || k_path.to_ascii_lowercase().ends_with(&file_path_lower)
                    {
                        let _ = k_hex; // silence unused warning in some builds
                        map.remove(&k);
                    }
                }
            }
        }
    }

    /// Apply all in-memory replacements to a NUS3BANK file and save it
    pub fn apply_replacements_and_save(
        original_path: &str,
        output_path: &str,
    ) -> Result<(), String> {
        let mut nus3bank_file = Nus3bankFile::open(original_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;

        // Apply replacements by iterating known tracks and fetching exact matches from the map
        let mut applied_count = 0usize;
        for track in nus3bank_file.tracks.clone() {
            if let Some(replacement) = Self::get_replacement_data(original_path, &track.hex_id) {
                if let Err(e) = nus3bank_file.replace_track_data(&track.hex_id, replacement) {
                    log::warn!("Failed to replace track {}: {}", track.hex_id, e);
                } else {
                    applied_count += 1;
                }
            }
        }

        println!("Applied {} NUS3BANK replacements", applied_count);

        nus3bank_file
            .save(output_path)
            .map_err(|e| format!("Failed to save NUS3BANK file: {}", e))?;

        // Clear only replacements for this file so pending state matches UI
        Self::clear_replacements_for_file(original_path);

        Ok(())
    }
}
