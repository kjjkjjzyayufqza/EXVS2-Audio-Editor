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
            map.get(&key).cloned()
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
    
    /// Clear all NUS3BANK replacement data from memory
    pub fn clear_replacements() {
        if let Ok(mut map) = REPLACED_NUS3BANK_DATA.lock() {
            map.clear();
            println!("Cleared all NUS3BANK audio replacements from memory");
        }
    }
    
    /// Apply all in-memory replacements to a NUS3BANK file and save it
    pub fn apply_replacements_and_save(
        original_path: &str,
        output_path: &str,
    ) -> Result<(), String> {
        let mut nus3bank_file = Nus3bankFile::open(original_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;
        
        // Apply all replacements from memory
        if let Ok(map) = REPLACED_NUS3BANK_DATA.lock() {
            for (key, replacement_data) in map.iter() {
                if key.ends_with(original_path) {
                    let hex_id = key.split(':').next().unwrap_or("");
                    if let Err(e) = nus3bank_file.replace_track_data(hex_id, replacement_data.clone()) {
                        log::warn!("Failed to replace track {}: {}", hex_id, e);
                    }
                }
            }
        }
        
        nus3bank_file.save(output_path)
            .map_err(|e| format!("Failed to save NUS3BANK file: {}", e))?;
        
        Ok(())
    }
    
    /// Add new track to NUS3BANK file in memory
    pub fn add_track_in_memory(
        file_path: &str,
        track_name: String,
        audio_data: Vec<u8>,
    ) -> Result<String, String> {
        // For now, we'll use a simple implementation that requires file reload
        // In a more sophisticated implementation, we could maintain the entire file state in memory
        let mut nus3bank_file = Nus3bankFile::open(file_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;
        
        let hex_id = nus3bank_file.add_track(track_name, audio_data)
            .map_err(|e| format!("Failed to add track: {}", e))?;
        
        // For now, we'll save immediately. In a production implementation,
        // this would be handled differently to maintain consistency with in-memory operations
        nus3bank_file.save(file_path)
            .map_err(|e| format!("Failed to save updated NUS3BANK file: {}", e))?;
        
        Ok(hex_id)
    }
    
    /// Remove track from NUS3BANK file in memory
    pub fn remove_track_in_memory(
        file_path: &str,
        hex_id: &str,
    ) -> Result<(), String> {
        // Similar to add_track_in_memory, this is a simplified implementation
        let mut nus3bank_file = Nus3bankFile::open(file_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;
        
        nus3bank_file.remove_track(hex_id)
            .map_err(|e| format!("Failed to remove track: {}", e))?;
        
        nus3bank_file.save(file_path)
            .map_err(|e| format!("Failed to save updated NUS3BANK file: {}", e))?;
        
        Ok(())
    }
}
