use super::structures::Nus3bankFile;
use super::error::Nus3bankError;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

// Store NUS3BANK replacement data
pub static REPLACEMENT_DATA: Lazy<Mutex<HashMap<String, ReplaceOperation>>> = 
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Clone)]
pub enum ReplaceOperation {
    Remove(String), // hex_id
    Replace(String, Vec<u8>), // hex_id, new_data
    Add(String, String, Vec<u8>), // name, generated_hex_id, data
}

/// NUS3BANK replace utilities
pub struct Nus3bankReplacer;

impl Nus3bankReplacer {
    /// Register a remove operation for a track
    pub fn register_remove(hex_id: &str) -> Result<(), String> {
        if let Ok(mut data) = REPLACEMENT_DATA.lock() {
            data.insert(hex_id.to_string(), ReplaceOperation::Remove(hex_id.to_string()));
            Ok(())
        } else {
            Err("Failed to register remove operation".to_string())
        }
    }
    
    /// Register an add operation for a track
    pub fn register_add(name: &str, audio_data: Vec<u8>) -> Result<String, String> {
        // Validate input data
        if audio_data.is_empty() {
            return Err("Audio data cannot be empty".to_string());
        }
        
        if name.is_empty() {
            return Err("Track name cannot be empty".to_string());
        }
        
        // Generate temporary hex_id for tracking (will be replaced with proper ID during add_track)
        let temp_hex_id = format!("0x{:x}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap().as_secs() & 0xFFFF);
        
        if let Ok(mut data) = REPLACEMENT_DATA.lock() {
            data.insert(
                temp_hex_id.clone(), 
                ReplaceOperation::Add(name.to_string(), temp_hex_id.clone(), audio_data)
            );
            println!("Registered add operation for track '{}' with temp_id {}", name, temp_hex_id);
            Ok(temp_hex_id)
        } else {
            Err("Failed to register add operation".to_string())
        }
    }
    
    /// Replace track in memory only (does not modify the actual file on disk)
    pub fn replace_track_in_memory(
        _file_path: &str,
        hex_id: &str,
        new_audio_data: Vec<u8>,
    ) -> Result<(), String> {
        if let Ok(mut data) = REPLACEMENT_DATA.lock() {
            data.insert(hex_id.to_string(), ReplaceOperation::Replace(hex_id.to_string(), new_audio_data));
            println!("Stored replacement data for NUS3BANK track: {}", hex_id);
            Ok(())
        } else {
            Err("Failed to acquire lock on replacement data".to_string())
        }
    }

    /// Check if there are any replacement data stored
    pub fn has_replacement_data() -> bool {
        if let Ok(data) = REPLACEMENT_DATA.lock() {
            !data.is_empty()
        } else {
            false
        }
    }
    
    /// Get the number of replacement data stored
    pub fn get_replacement_count() -> usize {
        if let Ok(data) = REPLACEMENT_DATA.lock() {
            data.len()
        } else {
            0
        }
    }
    
    /// Apply all operations to a file
    pub fn apply_to_file(file: &mut Nus3bankFile) -> Result<(), Nus3bankError> {
        if let Ok(data) = REPLACEMENT_DATA.lock() {
            for (_, operation) in data.iter() {
                match operation {
                    ReplaceOperation::Remove(hex_id) => {
                        println!("Applying remove operation for track: {}", hex_id);
                        file.remove_track(hex_id)?;
                    }
                    ReplaceOperation::Replace(hex_id, new_data) => {
                        println!("Applying replace operation for track: {}", hex_id);
                        file.replace_track_data(hex_id, new_data.clone())?;
                    }
                    ReplaceOperation::Add(name, _temp_hex_id, audio_data) => {
                        println!("Applying add operation for track: {}", name);
                        let new_hex_id = file.add_track(name.clone(), audio_data.clone())?;
                        println!("Successfully added track '{}' with ID: {}", name, new_hex_id);
                    }
                }
            }
        }
        Ok(())
    }
    
    /// Clear all replacement data from memory
    pub fn clear() {
        if let Ok(mut data) = REPLACEMENT_DATA.lock() {
            data.clear();
        }
    }

    /// Clear all NUS3BANK replacement data from memory
    pub fn clear_replacements() {
        Self::clear();
        println!("Cleared all NUS3BANK audio replacements from memory");
    }

    /// Apply all in-memory replacements to a NUS3BANK file and save it
    pub fn apply_replacements_and_save(
        original_path: &str,
        output_path: &str,
    ) -> Result<(), String> {
        let mut nus3bank_file = Nus3bankFile::open(original_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;

        Self::apply_to_file(&mut nus3bank_file)
            .map_err(|e| format!("Failed to apply NUS3BANK operations: {}", e))?;

        nus3bank_file
            .save(output_path)
            .map_err(|e| format!("Failed to save NUS3BANK file: {}", e))?;

        Self::clear();

        Ok(())
    }
}
