use super::audio_file_info::AudioFileInfo;
use nus3audio::{Nus3audioFile, AudioFile};
use std::fs;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

// Store temporary audio data for files that have been added, removed, or modified
static FILE_CHANGES: Lazy<Mutex<HashMap<String, FileChangeType>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

// Types of changes that can be made to files
pub enum FileChangeType {
    // Added audio file with ID, name, and data
    Add(String, String, Vec<u8>),
    // Removed audio file with ID and name
    Remove(String, String),
    // Replaced audio file with ID, name, and new data
    Replace(String, String, Vec<u8>),
}

/// Utility functions for NUS3AUDIO file operations
pub struct Nus3audioFileUtils;

impl Nus3audioFileUtils {
    /// Register a file addition (in memory only)
    pub fn register_add(audio_info: &AudioFileInfo, data: Vec<u8>) -> Result<(), String> {
        let key = format!("{}:{}", audio_info.name, audio_info.id);
        
        if let Ok(mut changes) = FILE_CHANGES.lock() {
            changes.insert(key, FileChangeType::Add(
                audio_info.id.clone(),
                audio_info.name.clone(),
                data
            ));
            Ok(())
        } else {
            Err("Failed to register file addition".to_string())
        }
    }
    
    /// Register a file removal (in memory only)
    pub fn register_remove(audio_info: &AudioFileInfo) -> Result<(), String> {
        let key = format!("{}:{}", audio_info.name, audio_info.id);
        
        if let Ok(mut changes) = FILE_CHANGES.lock() {
            changes.insert(key, FileChangeType::Remove(
                audio_info.id.clone(),
                audio_info.name.clone()
            ));
            Ok(())
        } else {
            Err("Failed to register file removal".to_string())
        }
    }
    
    /// Register a file replacement (in memory only)
    pub fn register_replace(audio_info: &AudioFileInfo, data: Vec<u8>) -> Result<(), String> {
        let key = format!("{}:{}", audio_info.name, audio_info.id);
        
        if let Ok(mut changes) = FILE_CHANGES.lock() {
            changes.insert(key, FileChangeType::Replace(
                audio_info.id.clone(),
                audio_info.name.clone(),
                data
            ));
            Ok(())
        } else {
            Err("Failed to register file replacement".to_string())
        }
    }
    
    /// Clear all pending changes
    pub fn clear_changes() {
        if let Ok(mut changes) = FILE_CHANGES.lock() {
            changes.clear();
            println!("Cleared all pending file changes");
        }
    }
    
    /// Save all pending changes to the file
    pub fn save_changes_to_file(file_path: &str) -> Result<(), String> {
        // Try to create a backup of the original file first
        let backup_path = format!("{}.backup", file_path);
        match fs::copy(file_path, &backup_path) {
            Ok(_) => println!("Created backup at {}", backup_path),
            Err(e) => println!("Warning: Failed to create backup: {}", e),
        }
        
        // Load the original NUS3AUDIO file
        let mut nus3_file = match Nus3audioFile::open(file_path) {
            Ok(file) => file,
            Err(e) => return Err(format!("Failed to open NUS3AUDIO file: {}", e)),
        };
        
        // Apply all changes
        if let Ok(changes) = FILE_CHANGES.lock() {
            // First process removals
            for (_, change_type) in changes.iter() {
                if let FileChangeType::Remove(id, name) = change_type {
                    // Find the index of the audio file to remove
                    if let Some(idx) = nus3_file.files.iter().position(|f| 
                        f.name == *name && f.id.to_string() == *id
                    ) {
                        // Remove the audio file
                        nus3_file.files.remove(idx);
                        println!("Removed audio file: {} (ID: {})", name, id);
                    }
                }
            }
            
            // Then process replacements
            for (_, change_type) in changes.iter() {
                if let FileChangeType::Replace(id, name, data) = change_type {
                    // Convert ID to u32
                    let id_val = match id.parse::<u32>() {
                        Ok(val) => val,
                        Err(_) => continue, // Skip if ID is invalid
                    };
                    
                    // Find the index of the audio file to replace
                    if let Some(idx) = nus3_file.files.iter().position(|f| 
                        f.name == *name && f.id.to_string() == *id
                    ) {
                        // Replace the audio file
                        nus3_file.files[idx] = AudioFile {
                            id: id_val,
                            name: name.clone(),
                            data: data.clone(),
                        };
                        println!("Replaced audio file: {} (ID: {})", name, id);
                    }
                }
            }
            
            // Finally process additions
            for (_, change_type) in changes.iter() {
                if let FileChangeType::Add(id, name, data) = change_type {
                    // Convert ID to u32
                    let id_val = match id.parse::<u32>() {
                        Ok(val) => val,
                        Err(_) => continue, // Skip if ID is invalid
                    };
                    
                    // Add the new audio file
                    nus3_file.files.push(AudioFile {
                        id: id_val,
                        name: name.clone(),
                        data: data.clone(),
                    });
                    println!("Added audio file: {} (ID: {})", name, id);
                }
            }
        }
        
        // Write the modified NUS3AUDIO data to a memory buffer
        let mut output_buffer = Vec::new();
        nus3_file.write(&mut output_buffer);
        
        // Write the buffer back to the original file
        match fs::write(file_path, output_buffer) {
            Ok(_) => {
                // Clear the changes after successful save
                Self::clear_changes();
                Ok(())
            },
            Err(e) => Err(format!("Failed to write updated file: {}", e)),
        }
    }
    
    /// Check if there are any pending changes
    pub fn has_pending_changes() -> bool {
        if let Ok(changes) = FILE_CHANGES.lock() {
            !changes.is_empty()
        } else {
            false
        }
    }
    
    /// Get the number of pending changes
    pub fn get_pending_changes_count() -> usize {
        if let Ok(changes) = FILE_CHANGES.lock() {
            changes.len()
        } else {
            0
        }
    }
    
    /// Register an audio file to be added to the NUS3AUDIO file
    pub fn register_add_audio(
        audio_info: &AudioFileInfo,
        audio_data: Vec<u8>,
    ) -> Result<(), String> {
        // Validate the ID
        let id = match audio_info.id.parse::<u32>() {
            Ok(val) => val,
            Err(_) => return Err("ID must be a valid number".to_string()),
        };
        
        // Create a key for the file change - use consistent format
        let key = format!("{}:{}", audio_info.name, audio_info.id);
        
        // Register the add operation
        if let Ok(mut changes) = FILE_CHANGES.lock() {
            changes.insert(
                key,
                FileChangeType::Add(
                    audio_info.id.clone(),
                    audio_info.name.clone(),
                    audio_data,
                ),
            );
            println!("Registered audio file to be added: {} (ID: {})", audio_info.name, audio_info.id);
            Ok(())
        } else {
            Err("Failed to register audio file addition".to_string())
        }
    }
    
    /// Get pending added audio data for a specific audio file
    pub fn get_pending_added_data(audio_name: &str, audio_id: &str) -> Option<Vec<u8>> {
        let key = format!("{}:{}", audio_name, audio_id);
        
        if let Ok(changes) = FILE_CHANGES.lock() {
            if let Some(FileChangeType::Add(_, _, data)) = changes.get(&key) {
                return Some(data.clone());
            }
        }
        
        None
    }
    
    /// Get all pending additions for external processing
    pub fn get_pending_additions() -> Vec<(String, String, Vec<u8>)> {
        let mut additions = Vec::new();
        
        if let Ok(changes) = FILE_CHANGES.lock() {
            for (_, change_type) in changes.iter() {
                if let FileChangeType::Add(id, name, data) = change_type {
                    additions.push((id.clone(), name.clone(), data.clone()));
                }
            }
        }
        
        additions
    }
    
    /// Get all pending removals for external processing
    pub fn get_pending_removals() -> Vec<(String, String)> {
        let mut removals = Vec::new();
        
        if let Ok(changes) = FILE_CHANGES.lock() {
            for (_, change_type) in changes.iter() {
                if let FileChangeType::Remove(id, name) = change_type {
                    removals.push((id.clone(), name.clone()));
                }
            }
        }
        
        removals
    }
    
    /// Get all currently valid IDs and names (considering pending changes)
    /// Returns (id, name) pairs that will exist after all pending changes are applied
    pub fn get_effective_audio_list(existing_files: Option<&Vec<AudioFileInfo>>) -> Vec<(String, String)> {
        let mut effective_list = Vec::new();
        
        // Start with existing files
        if let Some(files) = existing_files {
            for file in files {
                effective_list.push((file.id.clone(), file.name.clone()));
            }
        }
        
        if let Ok(changes) = FILE_CHANGES.lock() {
            // Collect all operations by ID and name to handle conflicts intelligently
            let mut operations_by_id: std::collections::HashMap<String, Vec<&FileChangeType>> = std::collections::HashMap::new();
            
            for (_, change_type) in changes.iter() {
                let key = match change_type {
                    FileChangeType::Add(id, name, _) => format!("{}:{}", id, name),
                    FileChangeType::Remove(id, name) => format!("{}:{}", id, name),
                    FileChangeType::Replace(id, name, _) => format!("{}:{}", id, name),
                };
                operations_by_id.entry(key).or_insert_with(Vec::new).push(change_type);
            }
            
            // Apply operations intelligently
            for (key, ops) in operations_by_id.iter() {
                let parts: Vec<&str> = key.split(':').collect();
                if parts.len() != 2 {
                    continue;
                }
                let id = parts[0];
                let name = parts[1];
                
                // Check what operations we have for this ID:name pair
                let has_remove = ops.iter().any(|op| matches!(op, FileChangeType::Remove(_, _)));
                let has_add = ops.iter().any(|op| matches!(op, FileChangeType::Add(_, _, _)));
                let has_replace = ops.iter().any(|op| matches!(op, FileChangeType::Replace(_, _, _)));
                
                // Remove from existing files if there's a remove operation
                if has_remove {
                    effective_list.retain(|(existing_id, existing_name)| {
                        !(existing_id == id && existing_name == name)
                    });
                }
                
                // Add to effective list if there's an add operation (regardless of remove)
                // This handles the case where user removes then adds the same ID
                if has_add {
                    // Only add if not already in the list (to avoid duplicates)
                    if !effective_list.iter().any(|(existing_id, existing_name)| existing_id == id && existing_name == name) {
                        effective_list.push((id.to_string(), name.to_string()));
                    }
                }
                
                // Replace operations don't change ID/name, just data, so no action needed for effective list
            }
        }
        
        effective_list
    }
    
    /// Check if an ID and name combination is available for a new audio file
    /// This excludes the current audio being added from conflict detection
    pub fn is_id_name_available(id: &str, name: &str, existing_files: Option<&Vec<AudioFileInfo>>) -> bool {
        let effective_list = Self::get_effective_audio_list(existing_files);
        
        // Check if this ID or name already exists in the effective list
        let id_exists = effective_list.iter().any(|(existing_id, _)| existing_id == id);
        let name_exists = effective_list.iter().any(|(_, existing_name)| existing_name == name);
        
        // Return true if both ID and name are available
        !id_exists && !name_exists
    }

} 