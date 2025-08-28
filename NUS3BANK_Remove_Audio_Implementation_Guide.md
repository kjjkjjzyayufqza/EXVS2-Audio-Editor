# NUS3BANK Audio Remove Implementation Guide

## Overview
实现从NUS3BANK文件中删除音频轨道的功能。删除操作需要重建TONE和PACK sections，并重新计算所有track的offsets。

## Current Implementation Status
- ✅ `structures.rs` - 已有 `remove_track()` 方法 (行89-105)
- ❌ `writer.rs` - 需要修改以处理删除track后的TONE section重建
- ❌ UI层 - 需要完善remove操作的保存流程

## Critical Changes Required

### 1. `src/nus3bank/writer.rs` - 修改TONE section处理

**问题**: 当前writer只是复制原始TONE section并更新pack offsets，但删除track后需要完全重建TONE section。

**需要修改的位置**:

#### 1.1 新增TONE section重建方法 (在第242行后添加)
```rust
/// Rebuild TONE section from current tracks
fn build_tone_section(tracks: &[AudioTrack]) -> Result<Vec<u8>, Nus3bankError> {
    let mut tone_data = Vec::new();
    
    // Track count (4 bytes)
    tone_data.extend_from_slice(&BinaryReader::write_u32_le(tracks.len() as u32));
    
    // Calculate metadata size for each track (following parser logic)
    let mut current_metadata_offset = 8 + (tracks.len() * 8); // Header + pointer table
    let mut track_metadata_blocks = Vec::new();
    
    for track in tracks {
        // Build metadata block following parser.rs line 320-374 format
        let mut metadata = Vec::new();
        
        // 6 bytes initial offset (parser.rs line 321)
        metadata.extend_from_slice(&[0u8; 6]);
        
        // temp_byte logic (parser.rs line 323-328)
        let temp_byte = if track.name.len() > 9 { 1u8 } else { 0u8 };
        metadata.push(temp_byte);
        
        if temp_byte > 9 || temp_byte == 0 {
            metadata.extend_from_slice(&[0u8; 5]);
        } else {
            metadata.push(0u8);
        }
        
        // String size and name (parser.rs line 330-338)
        let string_size = (track.name.len() + 1) as u8;
        metadata.push(string_size);
        metadata.extend_from_slice(track.name.as_bytes());
        metadata.push(0u8); // null terminator
        
        // Padding (parser.rs line 343-349)
        let padding = (string_size as usize + 1) % 4;
        if padding == 0 {
            metadata.extend_from_slice(&[0u8; 4]);
        } else {
            metadata.extend_from_slice(&vec![0u8; 4 - padding + 4]);
        }
        
        // Unknown value (usually 8) (parser.rs line 353)
        metadata.extend_from_slice(&BinaryReader::write_u32_le(8));
        
        // pack_offset and size (parser.rs line 356-357)
        metadata.extend_from_slice(&BinaryReader::write_u32_le(track.pack_offset));
        metadata.extend_from_slice(&BinaryReader::write_u32_le(track.size));
        
        track_metadata_blocks.push(metadata);
    }
    
    // Write pointer table (offset + metaSize pairs)
    for (i, metadata_block) in track_metadata_blocks.iter().enumerate() {
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

#### 1.2 修改 `write_file` 方法中的TONE处理 (替换行138-165)
```rust
b"TONE" => { /* "TONE": stream info */
    // For remove operations, rebuild TONE section completely
    let tone_new_start = new_file.len();
    new_file.extend_from_slice(b"TONE");
    
    // Build new TONE section
    let new_tone_data = Self::build_tone_section(&sorted_tracks)?;
    let new_tone_size = new_tone_data.len() as u32;
    
    new_file.extend_from_slice(&BinaryReader::write_u32_le(new_tone_size));
    new_file.extend_from_slice(&new_tone_data);
    
    // Skip original TONE section
    if cursor + 8 > original.len() { return Err(Nus3bankError::InvalidFormat { reason: format!("TONE header out of bounds at 0x{:X}", cursor) }); }
    let original_tone_size = read_u32_le(&original, cursor + 4)? as usize;
    cursor += 8 + original_tone_size;
}
```

#### 1.3 更新TOC中的TONE size (在PACK size更新后添加)
```rust
// Update TONE size inside TOC in the new buffer (if present)
let mut tone_entry_index: Option<usize> = None;
for (i, (magic, _)) in entries.iter().enumerate() {
    if magic == b"TONE" { tone_entry_index = Some(i); }
}

if let Some(i) = tone_entry_index {
    let new_tone_data = Self::build_tone_section(&sorted_tracks)?;
    let new_tone_size = new_tone_data.len() as u32;
    let size_pos = 8 /*NUS3+size*/ + 16 /*'BANKTOC '+toc_size+entry_count*/ + i*8 + 4;
    if size_pos + 4 <= new_file.len() {
        new_file[size_pos..size_pos+4].copy_from_slice(&BinaryReader::write_u32_le(new_tone_size));
    }
}
```

### 2. `src/nus3bank/structures.rs` - 优化remove_track方法

**当前问题**: remove_track方法只从Vec中删除track，但没有重置pack_offset。

**修改位置**: 行89-105，替换为:
```rust
/// Remove track by hex ID
pub fn remove_track(&mut self, hex_id: &str) -> Result<(), Nus3bankError> {
    let index = self.tracks.iter()
        .position(|t| t.hex_id == hex_id)
        .ok_or_else(|| Nus3bankError::TrackNotFound { hex_id: hex_id.to_string() })?;
    
    self.tracks.remove(index);
    
    // Update indices and reset pack_offsets (will be recalculated in writer)
    for (i, track) in self.tracks.iter_mut().enumerate() {
        track.index = i;
        track.pack_offset = 0; // Will be recalculated by writer
        track.metadata_offset = 0; // Will be recalculated by writer
        track.metadata_size = 0; // Will be recalculated by writer
    }
    
    self.bank_info.track_count = self.tracks.len() as u32;
    
    Ok(())
}
```

### 3. `src/nus3bank/replace.rs` - 添加remove操作支持

**如果不存在replace.rs文件，创建新文件**:
```rust
use super::structures::{Nus3bankFile, AudioTrack};
use super::error::Nus3bankError;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

// Store NUS3BANK replacement data
static REPLACEMENT_DATA: Lazy<Mutex<HashMap<String, ReplaceOperation>>> = 
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Clone)]
pub enum ReplaceOperation {
    Remove(String), // hex_id
    Replace(String, Vec<u8>), // hex_id, new_data
    Add(String, String, Vec<u8>), // name, generated_hex_id, data
}

pub struct Nus3bankReplacer;

impl Nus3bankReplacer {
    pub fn register_remove(hex_id: &str) -> Result<(), String> {
        if let Ok(mut data) = REPLACEMENT_DATA.lock() {
            data.insert(hex_id.to_string(), ReplaceOperation::Remove(hex_id.to_string()));
            Ok(())
        } else {
            Err("Failed to register remove operation".to_string())
        }
    }
    
    pub fn has_replacement_data() -> bool {
        if let Ok(data) = REPLACEMENT_DATA.lock() {
            !data.is_empty()
        } else {
            false
        }
    }
    
    pub fn get_replacement_count() -> usize {
        if let Ok(data) = REPLACEMENT_DATA.lock() {
            data.len()
        } else {
            0
        }
    }
    
    pub fn apply_to_file(file: &mut Nus3bankFile) -> Result<(), Nus3bankError> {
        if let Ok(data) = REPLACEMENT_DATA.lock() {
            for (_, operation) in data.iter() {
                match operation {
                    ReplaceOperation::Remove(hex_id) => {
                        file.remove_track(hex_id)?;
                    }
                    ReplaceOperation::Replace(hex_id, new_data) => {
                        file.replace_track_data(hex_id, new_data.clone())?;
                    }
                    ReplaceOperation::Add(name, _hex_id, data) => {
                        file.add_track(name.clone(), data.clone())?;
                    }
                }
            }
        }
        Ok(())
    }
    
    pub fn clear() {
        if let Ok(mut data) = REPLACEMENT_DATA.lock() {
            data.clear();
        }
    }
}
```

### 4. `src/ui/main_area/nus3audio_file_utils.rs` - 添加NUS3BANK remove支持

**修改位置**: 行37-50，在register_remove方法中添加NUS3BANK支持:
```rust
/// Register a file removal (in memory only)
pub fn register_remove(audio_info: &AudioFileInfo) -> Result<(), String> {
    let key = format!("{}:{}", audio_info.name, audio_info.id);

    // Check if this is a NUS3BANK file (hex ID format)
    if audio_info.id.starts_with("0x") {
        // Register with NUS3BANK replacer
        crate::nus3bank::replace::Nus3bankReplacer::register_remove(&audio_info.id)?;
    }

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
```

### 5. `src/ui/main_area/replace_utils.rs` - 修改unified保存方法

**修改位置**: 查找 `apply_replacements_and_save_unified` 方法，添加NUS3BANK remove处理:
```rust
// Apply NUS3BANK operations if any
if crate::nus3bank::replace::Nus3bankReplacer::has_replacement_data() {
    let mut nus3bank_file = crate::nus3bank::structures::Nus3bankFile::open(source_path)
        .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;
    
    crate::nus3bank::replace::Nus3bankReplacer::apply_to_file(&mut nus3bank_file)
        .map_err(|e| format!("Failed to apply NUS3BANK operations: {}", e))?;
    
    nus3bank_file.save(dest_path)
        .map_err(|e| format!("Failed to save NUS3BANK file: {}", e))?;
    
    crate::nus3bank::replace::Nus3bankReplacer::clear();
    return Ok(());
}
```

## Implementation Order

1. **创建 `src/nus3bank/replace.rs`** - 新文件
2. **修改 `src/nus3bank/mod.rs`** - 添加 `pub mod replace;`
3. **修改 `src/nus3bank/writer.rs`** - 添加TONE重建逻辑
4. **修改 `src/nus3bank/structures.rs`** - 优化remove_track方法
5. **修改 `src/ui/main_area/nus3audio_file_utils.rs`** - 添加NUS3BANK支持
6. **修改 `src/ui/main_area/replace_utils.rs`** - 集成NUS3BANK保存

## Key Technical Points

1. **TONE Section Structure**: 遵循parser.rs中解析的格式 (行250-378)
2. **Offset Calculation**: 删除track后所有后续track的offsets都需要重新计算
3. **TOC Update**: BANKTOC结构中的TONE和PACK size都需要更新
4. **Binary Format**: 严格遵循little-endian格式和padding对齐规则
5. **Error Handling**: 保持与现有错误处理模式一致

## Testing Requirements

1. 删除单个track后文件仍可正常解析
2. 删除多个track后track计数正确
3. 剩余track的audio data完整性
4. 文件大小和section sizes正确更新
