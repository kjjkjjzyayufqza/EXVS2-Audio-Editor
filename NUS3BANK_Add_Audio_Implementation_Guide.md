# NUS3BANK Add Audio Implementation Guide

## Overview

This document describes the implementation for adding new audio tracks to NUS3BANK files, including proper updates to both TONE and PACK sections.

## Key Components

### 1. Data Flow Architecture

```
UI Layer -> Nus3audioFileUtils -> ReplaceUtils -> Nus3bankReplacer -> Nus3bankFile -> Writer
```

### 2. Key Format Consistency

- **NUS3BANK files**: Use `hex_id:name` format for all operations
- **NUS3AUDIO files**: Use `name:id` format for all operations

## Implementation Details

### 1. `src/nus3bank/structures.rs` - Add Track Method

The `add_track` method handles adding new tracks to the NUS3BANK structure:

```rust
/// Add new track to the bank
pub fn add_track(&mut self, name: String, audio_data: Vec<u8>) -> Result<String, Nus3bankError> {
    // Generate new ID (find highest ID and add 1)
    let new_id = self.tracks.iter()
        .map(|t| t.numeric_id)
        .max()
        .unwrap_or(0) + 1;
    
    let hex_id = format!("0x{:x}", new_id);
    
    // Detect format
    let audio_format = if audio_data.starts_with(b"RIFF") {
        AudioFormat::Wav
    } else {
        AudioFormat::Unknown
    };
    
    let track = AudioTrack {
        index: self.tracks.len(),
        hex_id: hex_id.clone(),
        numeric_id: new_id,
        name,
        pack_offset: 0, // Will be recalculated when saving
        size: audio_data.len() as u32,
        metadata_offset: 0,
        metadata_size: 0, // Will be calculated during TONE rebuild
        audio_data: Some(audio_data),
        audio_format,
    };
    
    self.tracks.push(track);
    self.bank_info.track_count = self.tracks.len() as u32;
    
    Ok(hex_id)
}
```

**Key Points**:
- Auto-generates hex_id based on highest existing ID + 1
- Sets `metadata_size = 0` initially (calculated during TONE rebuild)
- Updates `bank_info.track_count`

### 2. `src/nus3bank/replace.rs` - Add Operation Support

```rust
#[derive(Clone)]
pub enum ReplaceOperation {
    Remove(String), // hex_id
    Replace(String, Vec<u8>), // hex_id, new_data
    Add(String, String, Vec<u8>), // name, generated_hex_id, data
}

impl Nus3bankReplacer {
    /// Register an add operation for a track
    pub fn register_add(name: &str, audio_data: Vec<u8>) -> Result<String, String> {
        // Generate temporary hex_id for tracking
        let temp_hex_id = format!("0x{:x}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap().as_secs() & 0xFFFF);
        
        if let Ok(mut data) = REPLACEMENT_DATA.lock() {
            data.insert(
                temp_hex_id.clone(), 
                ReplaceOperation::Add(name.to_string(), temp_hex_id.clone(), audio_data)
            );
            Ok(temp_hex_id)
        } else {
            Err("Failed to register add operation".to_string())
        }
    }
    
    pub fn apply_to_file(file: &mut Nus3bankFile) -> Result<(), Nus3bankError> {
        if let Ok(data) = REPLACEMENT_DATA.lock() {
            for (_, operation) in data.iter() {
                match operation {
                    ReplaceOperation::Add(name, _hex_id, data) => {
                        file.add_track(name.clone(), data.clone())?;
                    }
                    ReplaceOperation::Replace(hex_id, new_data) => {
                        file.replace_track_data(hex_id, new_data.clone())?;
                    }
                    ReplaceOperation::Remove(hex_id) => {
                        file.remove_track(hex_id)?;
                    }
                }
            }
        }
        Ok(())
    }
}
```

### 3. `src/nus3bank/writer.rs` - PACK and TONE Rebuilding

#### PACK Section Rebuild

```rust
// Build new PACK from current tracks (fallback to original PACK bytes when missing)
let mut sorted_tracks = file.tracks.clone();
sorted_tracks.sort_by_key(|t| t.numeric_id);

// Rebuild PACK and update track pack_offsets
let mut new_pack: Vec<u8> = Vec::new();
let mut updated_tracks = Vec::new();

for track in &sorted_tracks {
    // Skip tracks with very small metadata (indicates removal or corruption)
    if track.metadata_size <= 0x0c { 
        println!("Skipping track {} with metadata_size <= 0x0c", track.hex_id);
        continue; 
    }
    
    // Get audio data from memory or original file
    let data: Vec<u8> = if let Some(ref audio) = track.audio_data {
        audio.clone() // New track data from add operations
    } else {
        // Load from original file for existing tracks
        // ... existing logic
    };
    
    // Create updated track with new pack_offset
    let mut updated_track = track.clone();
    updated_track.pack_offset = new_pack.len() as u32;
    updated_track.size = data.len() as u32;
    
    // Add data to new PACK
    new_pack.extend_from_slice(&data);
    let pad = BinaryReader::calculate_padding(data.len());
    if pad > 0 { 
        new_pack.extend(std::iter::repeat(0u8).take(pad)); 
    }
    
    updated_tracks.push(updated_track);
}
```

#### TONE Section Rebuild

```rust
fn build_tone_section(tracks: &[super::structures::AudioTrack]) -> Result<Vec<u8>, Nus3bankError> {
    let mut tone_data = Vec::new();
    let mut track_metadata_blocks = Vec::new();
    
    // Filter out removed tracks and include new tracks
    let valid_tracks: Vec<&super::structures::AudioTrack> = tracks.iter()
        .filter(|track| {
            // Include new tracks (metadata_size = 0 but has audio_data)
            track.audio_data.is_some() || track.metadata_size > 0x0c
        })
        .collect();
    
    println!("Building TONE section with {} valid tracks out of {} total", valid_tracks.len(), tracks.len());
    
    // Build metadata blocks for each valid track
    for track in &valid_tracks {
        let mut metadata = Vec::new();
        
        // Standard TONE metadata structure
        metadata.extend_from_slice(&[0u8; 6]); // Initial padding
        
        let temp_byte = if track.name.len() > 9 { 1u8 } else { 0u8 };
        metadata.push(temp_byte);
        
        if temp_byte > 9 || temp_byte == 0 {
            metadata.extend_from_slice(&[0u8; 5]);
        } else {
            metadata.push(0u8);
        }
        
        // Track name with null terminator
        let string_size = (track.name.len() + 1) as u8;
        metadata.push(string_size);
        metadata.extend_from_slice(track.name.as_bytes());
        metadata.push(0u8); // Null terminator
        
        // Padding alignment
        let padding = (string_size as usize + 1) % 4;
        if padding == 0 {
            metadata.extend_from_slice(&[0u8; 4]);
        } else {
            metadata.extend_from_slice(&vec![0u8; 4 - padding + 4]);
        }
        
        // Standard values
        metadata.extend_from_slice(&BinaryReader::write_u32_le(8)); // Unknown value
        metadata.extend_from_slice(&BinaryReader::write_u32_le(track.pack_offset));
        metadata.extend_from_slice(&BinaryReader::write_u32_le(track.size));
        
        track_metadata_blocks.push(metadata);
    }
    
    // Build TONE structure: [track_count][pointer_table][metadata_blocks]
    let pointer_table_size = valid_tracks.len() * 8;
    let metadata_start_offset = 4 + pointer_table_size;
    
    // Write track count
    tone_data.extend_from_slice(&BinaryReader::write_u32_le(valid_tracks.len() as u32));
    
    // Write pointer table
    let mut current_metadata_offset = metadata_start_offset;
    for metadata_block in &track_metadata_blocks {
        let relative_offset = current_metadata_offset as u32;
        let meta_size = metadata_block.len() as u32;
        
        tone_data.extend_from_slice(&BinaryReader::write_u32_le(relative_offset));
        tone_data.extend_from_slice(&BinaryReader::write_u32_le(meta_size));
        
        current_metadata_offset += metadata_block.len();
    }
    
    // Append all metadata blocks
    for metadata_block in track_metadata_blocks {
        tone_data.extend_from_slice(&metadata_block);
    }
    
    Ok(tone_data)
}
```

### 4. `src/ui/main_area/nus3audio_file_utils.rs` - UI Integration

```rust
/// Register an audio file to be added to the NUS3BANK file
pub fn register_add_nus3bank(
    audio_info: &AudioFileInfo,
    audio_data: Vec<u8>,
) -> Result<(), String> {
    // For NUS3BANK files, register with Nus3bankReplacer
    if audio_info.is_nus3bank {
        return crate::nus3bank::replace::Nus3bankReplacer::register_add(
            &audio_info.name,
            audio_data
        ).map(|_| ());
    }
    
    // Fallback to existing NUS3AUDIO logic
    Self::register_add_audio(audio_info, audio_data)
}
```

### 5. `src/ui/main_area/replace_utils.rs` - Unified Save Logic

```rust
pub fn apply_replacements_and_save_unified(
    original_file_path: &str,
    save_path: &str,
) -> Result<(), String> {
    if original_file_path.to_lowercase().ends_with(".nus3bank") {
        // Handle NUS3BANK files with add operations
        
        // Apply NUS3BANK operations if any
        if crate::nus3bank::replace::Nus3bankReplacer::has_replacement_data() {
            let mut nus3bank_file = crate::nus3bank::structures::Nus3bankFile::open(original_file_path)
                .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;
            
            // This will handle Add, Replace, and Remove operations
            crate::nus3bank::replace::Nus3bankReplacer::apply_to_file(&mut nus3bank_file)
                .map_err(|e| format!("Failed to apply NUS3BANK operations: {}", e))?;
            
            // Save with rebuilt TONE and PACK sections
            nus3bank_file.save(save_path)
                .map_err(|e| format!("Failed to save NUS3BANK file: {}", e))?;
            
            crate::nus3bank::replace::Nus3bankReplacer::clear();
            return Ok(());
        }
        
        // Fallback for empty operations
        std::fs::copy(original_file_path, save_path)
            .map_err(|e| format!("Failed to copy file: {}", e))?;
        Ok(())
    } else {
        // Handle NUS3AUDIO files (existing implementation)
        Self::apply_replacements_and_save(original_file_path, save_path)
    }
}
```

## Critical Implementation Points

### 1. Track ID Generation
- Use highest existing `numeric_id + 1` to avoid conflicts
- Convert to hex format: `format!("0x{:x}", new_id)`

### 2. TONE Section Updates
- Filter tracks to include new ones: `track.audio_data.is_some() || track.metadata_size > 0x0c`
- Rebuild complete TONE structure with updated track count
- Recalculate all pointer table offsets

### 3. PACK Section Updates
- Append new audio data with proper padding alignment
- Update `pack_offset` for all tracks based on new PACK layout
- Sort tracks by `numeric_id` to maintain consistent ordering

### 4. File Structure Integrity
- Update BANKTOC entries for both TONE and PACK section sizes
- Recalculate total file size
- Maintain proper section alignment and padding

### 5. Error Handling
- Validate audio format (prefer WAV)
- Check for duplicate track names/IDs
- Ensure proper memory cleanup on failure

## Data Validation

### Track Validation
```rust
// Validate new track data
if audio_data.is_empty() {
    return Err("Audio data cannot be empty".to_string());
}

if name.is_empty() {
    return Err("Track name cannot be empty".to_string());
}

// Check for existing track with same name
if self.tracks.iter().any(|t| t.name == name) {
    return Err(format!("Track with name '{}' already exists", name));
}
```

### Format Validation
```rust
// Prefer WAV format for compatibility
let audio_format = if audio_data.starts_with(b"RIFF") {
    AudioFormat::Wav
} else {
    println!("Warning: Non-WAV format detected for track '{}'", name);
    AudioFormat::Unknown
};
```

## Memory Management

- Use `Vec<u8>` for audio data storage in memory
- Clear temporary data after successful save operations
- Implement proper cleanup in error cases to prevent memory leaks
