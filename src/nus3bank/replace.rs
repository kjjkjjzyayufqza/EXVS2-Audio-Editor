use super::structures::Nus3bankFile;
use super::error::Nus3bankError;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

// Store NUS3BANK replacement data, scoped by normalized file path.
pub static REPLACEMENT_DATA: Lazy<Mutex<HashMap<String, HashMap<String, ReplaceOperation>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static TEMP_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Clone)]
pub enum ReplaceOperation {
    Remove(String), // hex_id
    Replace(String, Vec<u8>), // hex_id, new_data
    Add(String, String, Vec<u8>), // name, generated_hex_id, data
}

/// NUS3BANK replace utilities
pub struct Nus3bankReplacer;

impl Nus3bankReplacer {
    fn normalize_file_key(file_path: &str) -> String {
        #[cfg(windows)]
        {
            file_path.replace('\\', "/").to_ascii_lowercase()
        }
        #[cfg(not(windows))]
        {
            file_path.to_string()
        }
    }

    fn hex_id_sort_key(hex_id: &str) -> (u32, String) {
        let parsed = hex_id
            .strip_prefix("0x")
            .and_then(|s| u32::from_str_radix(s, 16).ok())
            .unwrap_or(u32::MAX);
        (parsed, hex_id.to_string())
    }

    /// Register a remove operation for a track
    pub fn register_remove(file_path: &str, hex_id: &str) -> Result<(), String> {
        let file_key = Self::normalize_file_key(file_path);
        if let Ok(mut data) = REPLACEMENT_DATA.lock() {
            let per_file = data.entry(file_key).or_insert_with(HashMap::new);
            per_file.insert(hex_id.to_string(), ReplaceOperation::Remove(hex_id.to_string()));
            Ok(())
        } else {
            Err("Failed to register remove operation".to_string())
        }
    }
    
    /// Register an add operation for a track
    pub fn register_add(file_path: &str, name: &str, audio_data: Vec<u8>) -> Result<String, String> {
        // Validate input data
        if audio_data.is_empty() {
            return Err("Audio data cannot be empty".to_string());
        }
        
        if name.is_empty() {
            return Err("Track name cannot be empty".to_string());
        }
        
        // Generate temporary hex_id for tracking (will be replaced with proper ID during add_track).
        // Use a monotonic counter for determinism (important for reproducible exports).
        let n = TEMP_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_hex_id = format!("0x{:x}", 0x8000_0000u32.wrapping_add(n));
        
        if let Ok(mut data) = REPLACEMENT_DATA.lock() {
            let file_key = Self::normalize_file_key(file_path);
            let per_file = data.entry(file_key).or_insert_with(HashMap::new);
            per_file.insert(
                temp_hex_id.clone(),
                ReplaceOperation::Add(name.to_string(), temp_hex_id.clone(), audio_data),
            );
            println!(
                "Registered add operation for track '{}' with temp_id {}",
                name, temp_hex_id
            );
            Ok(temp_hex_id)
        } else {
            Err("Failed to register add operation".to_string())
        }
    }
    
    /// Replace track in memory only (does not modify the actual file on disk)
    pub fn replace_track_in_memory(
        file_path: &str,
        hex_id: &str,
        new_audio_data: Vec<u8>,
    ) -> Result<(), String> {
        let file_key = Self::normalize_file_key(file_path);
        if let Ok(mut data) = REPLACEMENT_DATA.lock() {
            let per_file = data.entry(file_key).or_insert_with(HashMap::new);
            per_file.insert(
                hex_id.to_string(),
                ReplaceOperation::Replace(hex_id.to_string(), new_audio_data),
            );
            println!("Stored replacement data for NUS3BANK track: {}", hex_id);
            Ok(())
        } else {
            Err("Failed to acquire lock on replacement data".to_string())
        }
    }

    /// Check if there are any replacement data stored
    pub fn has_replacement_data() -> bool {
        if let Ok(data) = REPLACEMENT_DATA.lock() {
            data.values().any(|m| !m.is_empty())
        } else {
            false
        }
    }
    
    /// Get the number of replacement data stored
    pub fn get_replacement_count() -> usize {
        if let Ok(data) = REPLACEMENT_DATA.lock() {
            data.values().map(|m| m.len()).sum()
        } else {
            0
        }
    }
    
    /// Apply all operations to a file
    pub fn apply_to_file(file_path: &str, file: &mut Nus3bankFile) -> Result<(), Nus3bankError> {
        let file_key = Self::normalize_file_key(file_path);
        let mut ops: Vec<ReplaceOperation> = if let Ok(data) = REPLACEMENT_DATA.lock() {
            data.get(&file_key)
                .map(|m| m.values().cloned().collect::<Vec<_>>())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Deterministic application order:
        // - Remove first (lowest risk of offset conflicts)
        // - Replace next (does not change entry count)
        // - Add last (changes entry count and PACK layout)
        ops.sort_by(|a, b| {
            fn prio(op: &ReplaceOperation) -> u8 {
                match op {
                    ReplaceOperation::Remove(_) => 0,
                    ReplaceOperation::Replace(_, _) => 1,
                    ReplaceOperation::Add(_, _, _) => 2,
                }
            }
            let pa = prio(a);
            let pb = prio(b);
            if pa != pb {
                return pa.cmp(&pb);
            }

            match (a, b) {
                (ReplaceOperation::Remove(ha), ReplaceOperation::Remove(hb)) => {
                    Nus3bankReplacer::hex_id_sort_key(ha).cmp(&Nus3bankReplacer::hex_id_sort_key(hb))
                }
                (ReplaceOperation::Replace(ha, _), ReplaceOperation::Replace(hb, _)) => {
                    Nus3bankReplacer::hex_id_sort_key(ha).cmp(&Nus3bankReplacer::hex_id_sort_key(hb))
                }
                (ReplaceOperation::Add(na, ha, _), ReplaceOperation::Add(nb, hb, _)) => {
                    na.cmp(nb)
                        .then_with(|| Nus3bankReplacer::hex_id_sort_key(ha).cmp(&Nus3bankReplacer::hex_id_sort_key(hb)))
                }
                // Different types but same priority should not happen.
                _ => std::cmp::Ordering::Equal,
            }
        });

        for operation in ops {
            match operation {
                ReplaceOperation::Remove(hex_id) => {
                    println!("Applying remove operation for track: {}", hex_id);
                    file.remove_track(&hex_id)?;
                }
                ReplaceOperation::Replace(hex_id, new_data) => {
                    println!("Applying replace operation for track: {}", hex_id);
                    file.replace_track_data(&hex_id, new_data)?;
                }
                ReplaceOperation::Add(name, _temp_hex_id, audio_data) => {
                    println!("Applying add operation for track: {}", name);
                    let new_hex_id = file.add_track(name.clone(), audio_data)?;
                    println!("Successfully added track '{}' with ID: {}", name, new_hex_id);
                }
            }
        }

        Ok(())
    }
    
    /// Clear replacement data for a specific file.
    pub fn clear_for_file(file_path: &str) {
        let file_key = Self::normalize_file_key(file_path);
        if let Ok(mut data) = REPLACEMENT_DATA.lock() {
            data.remove(&file_key);
        }
    }

    /// Clear all NUS3BANK replacement data from memory
    pub fn clear_replacements() {
        if let Ok(mut data) = REPLACEMENT_DATA.lock() {
            data.clear();
        }
        println!("Cleared all NUS3BANK audio replacements from memory");
    }

    /// Apply all in-memory replacements to a NUS3BANK file and save it
    pub fn apply_replacements_and_save(
        original_path: &str,
        output_path: &str,
    ) -> Result<(), String> {
        let mut nus3bank_file = Nus3bankFile::open(original_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;

        Self::apply_to_file(original_path, &mut nus3bank_file)
            .map_err(|e| format!("Failed to apply NUS3BANK operations: {}", e))?;

        nus3bank_file
            .save(output_path)
            .map_err(|e| format!("Failed to save NUS3BANK file: {}", e))?;

        Self::clear_for_file(original_path);

        Ok(())
    }
}
