# NUS3BANK Audio Remove/Add Implementation Guide

## 当前实现状态分析

### ✅ 已实现功能
- 基础的`add_track()`、`remove_track()`、`replace_track_data()`方法
- PROP、BINF、TONE、PACK sections的解析
- BANKTOC结构的writer实现（部分）
- PACK section的重建和track offset更新

### ❌ 关键问题和缺失功能

1. **缺失Sections支持**：
   - GRP (Group) section - 未处理
   - DTON section - 未处理  
   - JUNK section - 未处理

2. **Writer实现不完整**：
   - 只支持BANKTOC结构，不支持标准section结构
   - TONE section处理有严重缺陷
   - Section size更新机制不完整

3. **TONE Section重大问题**：
   - 当前只是复制原TONE数据，不会重新生成
   - 添加track时不会在TONE中添加metadata
   - 删除track时不会从TONE中移除metadata
   - Track count不会正确更新

## NUS3BANK格式详解

基于您提供的例子：
```
TOC entry 0: PROP -> 20 bytes
TOC entry 1: BINF -> 44 bytes
TOC entry 2: GRP  -> 55056 bytes
TOC entry 3: DTON -> 248 bytes
TOC entry 4: TONE -> 9468 bytes
TOC entry 5: JUNK -> 4 bytes
TOC entry 6: PACK -> 308560 bytes
```

### File Structure:
```
NUS3BANK File:
├── NUS3 Header (4 bytes: "NUS3")
├── Total Size (4 bytes)
├── BANK Header (4 bytes: "BANK")  
├── TOC Header (4 bytes: "TOC ")
├── TOC Size (4 bytes)
├── Entry Count (4 bytes)
├── TOC Entries (8 bytes each: magic + size)
└── Sections Data:
    ├── PROP Section
    ├── BINF Section  
    ├── GRP Section    ⚠️ 未实现
    ├── DTON Section   ⚠️ 未实现
    ├── TONE Section
    ├── JUNK Section   ⚠️ 未实现
    └── PACK Section
```

## 实现方案

### 1. 扩展Parser支持所有Sections

#### 1.1 修改`src/nus3bank/parser.rs`

在`parse_banktoc_structure`和`parse_standard_sections`中添加：

```rust
// 在match语句中添加
b"GRP " => {
    section_offsets.grp_offset = current_pos;
    // 存储原始GRP数据，暂时不解析
    let grp_data = Self::read_raw_section(reader, expected_size)?;
    bank_info.as_mut().unwrap().grp_data = Some(grp_data);
},
b"DTON" => {
    section_offsets.dton_offset = current_pos;
    // 存储原始DTON数据
    let dton_data = Self::read_raw_section(reader, expected_size)?;
    bank_info.as_mut().unwrap().dton_data = Some(dton_data);
},
b"JUNK" => {
    section_offsets.junk_offset = current_pos;
    // 存储原始JUNK数据
    let junk_data = Self::read_raw_section(reader, expected_size)?;
    bank_info.as_mut().unwrap().junk_data = Some(junk_data);
},
```

#### 1.2 添加新的helper方法：

```rust
/// Read raw section data without parsing
fn read_raw_section<R: Read>(reader: &mut R, expected_size: u32) -> Result<Vec<u8>, Nus3bankError> {
    let section_size = BinaryReader::read_u32_le(reader)?;
    if section_size != expected_size {
        eprintln!("Warning: Section size mismatch");
    }
    
    let mut data = vec![0u8; section_size as usize];
    reader.read_exact(&mut data)?;
    Ok(data)
}
```

### 2. 扩展数据结构

#### 2.1 修改`src/nus3bank/structures.rs`

```rust
#[derive(Clone, Debug)]
pub struct SectionOffsets {
    pub prop_offset: u32,
    pub binf_offset: u32,
    pub grp_offset: u32,    // 新增
    pub dton_offset: u32,   // 新增
    pub tone_offset: u32,
    pub junk_offset: u32,   // 新增
    pub pack_offset: u32,
}

#[derive(Clone, Debug)]
pub struct BankInfo {
    pub bank_id: u32,
    pub bank_string: String,
    pub total_size: u32,
    pub track_count: u32,
    pub section_offsets: SectionOffsets,
    // 新增原始section数据存储
    pub prop_data: Option<Vec<u8>>,
    pub grp_data: Option<Vec<u8>>,
    pub dton_data: Option<Vec<u8>>,
    pub junk_data: Option<Vec<u8>>,
}
```

### 3. 完全重写Writer

#### 3.1 创建新的`TONEBuilder`结构：

```rust
pub struct TONEBuilder;

impl TONEBuilder {
    /// 从tracks重新生成完整的TONE section
    pub fn build_tone_section(tracks: &[AudioTrack]) -> Vec<u8> {
        let mut tone_data = Vec::new();
        
        // TONE header
        let track_count = tracks.len() as u32;
        tone_data.extend_from_slice(&BinaryReader::write_u32_le(track_count));
        
        // 计算pointer table的大小
        let pointer_table_size = track_count * 8; // offset + metaSize pairs
        let metadata_start = 8 + pointer_table_size; // 8 = 4(section_size) + 4(track_count)
        
        let mut current_metadata_offset = metadata_start;
        let mut metadata_blocks = Vec::new();
        let mut pointers = Vec::new();
        
        // 为每个track生成metadata并记录pointer
        for track in tracks {
            let metadata = Self::build_track_metadata(track);
            let metadata_size = metadata.len() as u32;
            
            // 记录pointer (相对偏移)
            pointers.push((current_metadata_offset, metadata_size));
            metadata_blocks.push(metadata);
            
            current_metadata_offset += metadata_size;
        }
        
        // 写入pointer table
        for (offset, size) in pointers {
            tone_data.extend_from_slice(&BinaryReader::write_u32_le(offset));
            tone_data.extend_from_slice(&BinaryReader::write_u32_le(size));
        }
        
        // 写入所有metadata blocks
        for metadata in metadata_blocks {
            tone_data.extend_from_slice(&metadata);
        }
        
        tone_data
    }
    
    /// 为单个track生成metadata block
    fn build_track_metadata(track: &AudioTrack) -> Vec<u8> {
        let mut metadata = Vec::new();
        
        // 按照parser.rs中的逆向逻辑构建metadata
        // 这里需要根据parser中的解析逻辑来构建
        
        // 6 bytes unknown header
        metadata.extend_from_slice(&[0; 6]);
        
        // temp_byte逻辑（需要根据具体情况设定）
        metadata.push(1); // temp_byte
        
        // string_size + name + null terminator
        let name_bytes = track.name.as_bytes();
        let string_size = (name_bytes.len() + 1) as u8;
        metadata.push(string_size);
        metadata.extend_from_slice(name_bytes);
        metadata.push(0); // null terminator
        
        // padding calculation
        let padding = (string_size as usize + 1) % 4;
        if padding == 0 {
            metadata.extend_from_slice(&[0; 4]);
        } else {
            metadata.extend_from_slice(&vec![0; 4 - padding + 4]);
        }
        
        // unknown value (usually 8)
        metadata.extend_from_slice(&BinaryReader::write_u32_le(8));
        
        // pack_offset and size
        metadata.extend_from_slice(&BinaryReader::write_u32_le(track.pack_offset));
        metadata.extend_from_slice(&BinaryReader::write_u32_le(track.size));
        
        metadata
    }
}
```

#### 3.2 重写`src/nus3bank/writer.rs`的主要逻辑：

```rust
impl Nus3bankWriter {
    pub fn write_file<P: AsRef<std::path::Path>>(file: &Nus3bankFile, path: P) -> Result<(), Nus3bankError> {
        // 1. 重新构建PACK section
        let new_pack = Self::build_pack_section(&file.tracks)?;
        
        // 2. 重新构建TONE section
        let new_tone = TONEBuilder::build_tone_section(&file.tracks);
        
        // 3. 构建完整文件
        let mut new_file = Vec::new();
        
        // NUS3 header
        new_file.extend_from_slice(b"NUS3");
        new_file.extend_from_slice(&[0; 4]); // placeholder for total size
        
        // BANKTOC header
        new_file.extend_from_slice(b"BANK");
        new_file.extend_from_slice(b"TOC ");
        
        // 计算新的section sizes
        let sections = [
            (b"PROP", file.bank_info.prop_data.as_ref().map(|d| d.len()).unwrap_or(0)),
            (b"BINF", Self::calculate_binf_size(&file.bank_info)),
            (b"GRP ", file.bank_info.grp_data.as_ref().map(|d| d.len()).unwrap_or(0)),
            (b"DTON", file.bank_info.dton_data.as_ref().map(|d| d.len()).unwrap_or(0)),
            (b"TONE", new_tone.len()),
            (b"JUNK", file.bank_info.junk_data.as_ref().map(|d| d.len()).unwrap_or(0)),
            (b"PACK", new_pack.len()),
        ];
        
        // 写入TOC
        let toc_size = sections.len() * 8;
        new_file.extend_from_slice(&BinaryReader::write_u32_le(toc_size as u32));
        new_file.extend_from_slice(&BinaryReader::write_u32_le(sections.len() as u32));
        
        // 写入TOC entries
        for (magic, size) in &sections {
            new_file.extend_from_slice(magic);
            new_file.extend_from_slice(&BinaryReader::write_u32_le(*size as u32));
        }
        
        // 写入各个sections
        for (magic, _) in &sections {
            match &magic[..] {
                b"PROP" => {
                    new_file.extend_from_slice(b"PROP");
                    if let Some(data) = &file.bank_info.prop_data {
                        new_file.extend_from_slice(&BinaryReader::write_u32_le(data.len() as u32));
                        new_file.extend_from_slice(data);
                    }
                },
                b"BINF" => {
                    new_file.extend_from_slice(b"BINF");
                    let binf_data = Self::build_binf_section(&file.bank_info)?;
                    new_file.extend_from_slice(&BinaryReader::write_u32_le(binf_data.len() as u32));
                    new_file.extend_from_slice(&binf_data);
                },
                b"GRP " => {
                    if let Some(data) = &file.bank_info.grp_data {
                        new_file.extend_from_slice(b"GRP ");
                        new_file.extend_from_slice(&BinaryReader::write_u32_le(data.len() as u32));
                        new_file.extend_from_slice(data);
                    }
                },
                b"DTON" => {
                    if let Some(data) = &file.bank_info.dton_data {
                        new_file.extend_from_slice(b"DTON");
                        new_file.extend_from_slice(&BinaryReader::write_u32_le(data.len() as u32));
                        new_file.extend_from_slice(data);
                    }
                },
                b"TONE" => {
                    new_file.extend_from_slice(b"TONE");
                    new_file.extend_from_slice(&BinaryReader::write_u32_le(new_tone.len() as u32));
                    new_file.extend_from_slice(&new_tone);
                },
                b"JUNK" => {
                    if let Some(data) = &file.bank_info.junk_data {
                        new_file.extend_from_slice(b"JUNK");
                        new_file.extend_from_slice(&BinaryReader::write_u32_le(data.len() as u32));
                        new_file.extend_from_slice(data);
                    }
                },
                b"PACK" => {
                    new_file.extend_from_slice(b"PACK");
                    new_file.extend_from_slice(&BinaryReader::write_u32_le(new_pack.len() as u32));
                    new_file.extend_from_slice(&new_pack);
                },
                _ => {}
            }
        }
        
        // 更新total size
        let total_size = new_file.len() as u32 - 8;
        new_file[4..8].copy_from_slice(&BinaryReader::write_u32_le(total_size));
        
        // 写入文件
        fs::write(path, new_file)?;
        Ok(())
    }
    
    /// 重新构建PACK section，确保正确的offset计算
    fn build_pack_section(tracks: &[AudioTrack]) -> Result<Vec<u8>, Nus3bankError> {
        let mut pack_data = Vec::new();
        
        // 按numeric_id排序以保持一致性
        let mut sorted_tracks = tracks.to_vec();
        sorted_tracks.sort_by_key(|t| t.numeric_id);
        
        for track in &mut sorted_tracks {
            if let Some(audio_data) = &track.audio_data {
                // 更新track的pack_offset
                track.pack_offset = pack_data.len() as u32;
                
                // 添加音频数据
                pack_data.extend_from_slice(audio_data);
                
                // 添加padding以保持对齐
                let padding = BinaryReader::calculate_padding(audio_data.len());
                if padding > 0 {
                    pack_data.extend(std::iter::repeat(0u8).take(padding));
                }
            }
        }
        
        Ok(pack_data)
    }
    
    /// 构建BINF section
    fn build_binf_section(bank_info: &BankInfo) -> Result<Vec<u8>, Nus3bankError> {
        let mut binf_data = Vec::new();
        
        // unknown1 (通常是0)
        binf_data.extend_from_slice(&BinaryReader::write_u32_le(0));
        
        // bank_id
        binf_data.extend_from_slice(&BinaryReader::write_u32_le(bank_info.bank_id));
        
        // bank_string + null terminator
        binf_data.extend_from_slice(bank_info.bank_string.as_bytes());
        binf_data.push(0);
        
        // padding到4字节对齐
        let padding = (binf_data.len() % 4);
        if padding != 0 {
            binf_data.extend(std::iter::repeat(0u8).take(4 - padding));
        }
        
        Ok(binf_data)
    }
}
```

### 4. 修改Add/Remove方法

#### 4.1 增强`add_track`方法：

```rust
pub fn add_track(&mut self, name: String, audio_data: Vec<u8>) -> Result<String, Nus3bankError> {
    // 生成新ID
    let new_id = self.tracks.iter()
        .map(|t| t.numeric_id)
        .max()
        .unwrap_or(0) + 1;
    
    let hex_id = format!("0x{:x}", new_id);
    
    // 创建新track，pack_offset将在保存时重新计算
    let track = AudioTrack {
        index: self.tracks.len(),
        hex_id: hex_id.clone(),
        numeric_id: new_id,
        name,
        pack_offset: 0, // 将在save时重新计算
        size: audio_data.len() as u32,
        metadata_offset: 0, // 将在save时重新计算
        metadata_size: 0,   // 将在save时重新计算
        audio_data: Some(audio_data),
        audio_format: if audio_data.starts_with(b"RIFF") { 
            AudioFormat::Wav 
        } else { 
            AudioFormat::Unknown 
        },
    };
    
    self.tracks.push(track);
    self.bank_info.track_count = self.tracks.len() as u32;
    
    Ok(hex_id)
}
```