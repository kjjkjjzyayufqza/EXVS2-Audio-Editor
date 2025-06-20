use super::audio_file_info::AudioFileInfo;
use nus3audio::{AudioFile, Nus3audioFile};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;

// Store temporary audio data for files that have been added, removed, or modified
static FILE_CHANGES: Lazy<Mutex<HashMap<String, FileChangeType>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// Types of changes that can be made to files
pub enum FileChangeType {
    // Added audio file with ID, name, and data
    Add(String, String, Vec<u8>),
    // Removed audio file with ID and name
    Remove(String, String),
}

/// Utility functions for NUS3AUDIO file operations
pub struct Nus3audioFileUtils;

impl Nus3audioFileUtils {
    /// Register a file addition (in memory only)
    pub fn register_add(audio_info: &AudioFileInfo, data: Vec<u8>) -> Result<(), String> {
        let key = format!("{}:{}", audio_info.name, audio_info.id);

        if let Ok(mut changes) = FILE_CHANGES.lock() {
            changes.insert(
                key,
                FileChangeType::Add(audio_info.id.clone(), audio_info.name.clone(), data),
            );
            Ok(())
        } else {
            Err("Failed to register file addition".to_string())
        }
    }

    /// Register a file removal (in memory only)
    pub fn register_remove(audio_info: &AudioFileInfo) -> Result<(), String> {
        let key = format!("{}:{}", audio_info.name, audio_info.id);

        if let Ok(mut changes) = FILE_CHANGES.lock() {
            changes.insert(
                key,
                FileChangeType::Remove(audio_info.id.clone(), audio_info.name.clone()),
            );
            Ok(())
        } else {
            Err("Failed to register file removal".to_string())
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
        match std::fs::copy(file_path, &backup_path) {
            Ok(_) => println!("Created backup at {}", backup_path),
            Err(e) => println!("Warning: Failed to create backup: {}", e),
        }

        // Use ReplaceUtils to apply all in-memory replacements and save the file (覆盖原文件)
        match super::replace_utils::ReplaceUtils::apply_replacements_and_save(file_path, file_path) {
            Ok(_) => {
                // 清空 FILE_CHANGES
                Self::clear_changes();
                Ok(())
            }
            Err(e) => Err(format!("Failed to write updated file: {}", e)),
        }
    }

    /// Check if there are any pending changes
    pub fn has_pending_changes() -> bool {
        // Check for pending changes in FILE_CHANGES
        let has_file_changes = if let Ok(changes) = FILE_CHANGES.lock() {
            !changes.is_empty()
        } else {
            false
        };
        
        // Check for replacement data in ReplaceUtils
        let has_replacements = super::replace_utils::ReplaceUtils::has_replacement_data();
        
        has_file_changes || has_replacements
    }

    /// Get the number of pending changes
    pub fn get_pending_changes_count() -> usize {
        // Count pending changes in FILE_CHANGES
        let file_changes_count = if let Ok(changes) = FILE_CHANGES.lock() {
            changes.len()
        } else {
            0
        };
        
        // Count replacement data in ReplaceUtils
        let replacements_count = super::replace_utils::ReplaceUtils::get_replacement_count();
        
        file_changes_count + replacements_count
    }

    /// Register an audio file to be added to the NUS3AUDIO file
    pub fn register_add_audio(
        audio_info: &AudioFileInfo,
        audio_data: Vec<u8>,
    ) -> Result<(), String> {
        // Validate the ID
        match audio_info.id.parse::<u32>() {
            Ok(val) => val,
            Err(_) => return Err("ID must be a valid number".to_string()),
        };

        // Create a key for the file change - use consistent format
        let key = format!("{}:{}", audio_info.name, audio_info.id);

        // Register the add operation
        if let Ok(mut changes) = FILE_CHANGES.lock() {
            changes.insert(
                key,
                FileChangeType::Add(audio_info.id.clone(), audio_info.name.clone(), audio_data),
            );
            println!(
                "Registered audio file to be added: {} (ID: {})",
                audio_info.name, audio_info.id
            );
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

    /// Get all currently valid IDs and names (considering pending changes)
    /// Returns (id, name) pairs that will exist after all pending changes are applied
    pub fn get_effective_audio_list(
        existing_files: Option<&Vec<AudioFileInfo>>,
    ) -> Vec<(String, String)> {
        let mut effective_list = Vec::new();

        // Start with existing files
        if let Some(files) = existing_files {
            for file in files {
                effective_list.push((file.id.clone(), file.name.clone()));
            }
        }

        if let Ok(changes) = FILE_CHANGES.lock() {
            // Collect all operations by ID and name to handle conflicts intelligently
            let mut operations_by_id: std::collections::HashMap<String, Vec<&FileChangeType>> =
                std::collections::HashMap::new();

            for (_, change_type) in changes.iter() {
                let key = match change_type {
                    FileChangeType::Add(id, name, _) => format!("{}:{}", id, name),
                    FileChangeType::Remove(id, name) => format!("{}:{}", id, name),
                };
                operations_by_id
                    .entry(key)
                    .or_insert_with(Vec::new)
                    .push(change_type);
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
                let has_remove = ops
                    .iter()
                    .any(|op| matches!(op, FileChangeType::Remove(_, _)));
                let has_add = ops
                    .iter()
                    .any(|op| matches!(op, FileChangeType::Add(_, _, _)));

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
                    if !effective_list.iter().any(|(existing_id, existing_name)| {
                        existing_id == id && existing_name == name
                    }) {
                        effective_list.push((id.to_string(), name.to_string()));
                    }
                }

                // Replace operations don't change ID/name, just data, so no action needed for effective list
            }
        }

        effective_list
    }
}
