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

### 3. 基于Python脚本的Writer重写方案

参考`nus3append.py`脚本的逻辑，我们需要采用以下策略：

**重要说明**：Python脚本只实现了ADD操作，REMOVE操作需要额外设计。Python脚本的核心策略是：
- 完全重写文件，保留原始数据的特定部分
- 精确的offset和size计算公式
- 使用现有track的metadata作为模板

#### 3.1 创建新的`TONEBuilder`结构：

```rust
pub struct TONEBuilder;

impl TONEBuilder {
    /// 基于现有tracks和模板重新生成TONE section
    /// 参考nus3append.py的逻辑：使用现有track作为metadata模板
    pub fn build_tone_section(tracks: &[AudioTrack], original_tone_data: &[u8]) -> Vec<u8> {
        let mut tone_data = Vec::new();
        
        let track_count = tracks.len() as u32;
        
        // 写入新的track count
        tone_data.extend_from_slice(&BinaryReader::write_u32_le(track_count));
        
        // 计算metadata的起始位置（相对于TONE section data开始）
        let metadata_start_offset = 4 + (track_count * 8); // 4(count) + pointer_table
        let mut current_metadata_offset = metadata_start_offset;
        
        let mut metadata_blocks = Vec::new();
        let mut pointers = Vec::new();
        
        // 为每个track构建metadata
        for track in tracks {
            let metadata = if track.audio_data.is_some() {
                // 新添加的track，使用模板生成metadata
                Self::build_new_track_metadata(track, original_tone_data)?
            } else {
                // 现有track，保持原有metadata但更新offset信息
                Self::update_existing_track_metadata(track, original_tone_data)?
            };
            
            let metadata_size = metadata.len() as u32;
            
            // 记录pointer (相对于TONE section data开始的偏移)
            pointers.push((current_metadata_offset, metadata_size));
            metadata_blocks.push(metadata);
            
            current_metadata_offset += metadata_size;
        }
        
        // 写入pointer table，完全按照Python脚本第94-99行的逻辑
        for (i, (offset, size)) in pointers.iter().enumerate() {
            if i < tracks.len() - 1 || tracks[i].audio_data.is_none() {
                // 现有track的offset需要+8（Python脚本第96行: curToneOffset + 8）
                tone_data.extend_from_slice(&BinaryReader::write_u32_le(offset + 8));
            } else {
                // 新添加track的offset（Python脚本第100行: lastToneOffset + lastToneSize + 8）
                tone_data.extend_from_slice(&BinaryReader::write_u32_le(*offset));
            }
            tone_data.extend_from_slice(&BinaryReader::write_u32_le(*size));
        }
        
        // 写入所有metadata blocks
        for metadata in metadata_blocks {
            tone_data.extend_from_slice(&metadata);
        }
        
        tone_data
    }
    
    /// 为新track生成metadata，完全按照nus3append.py的精确逻辑
    /// Python脚本第103-116行的实现
    fn build_new_track_metadata(track: &AudioTrack, original_tone_data: &[u8]) -> Result<Vec<u8>, Nus3bankError> {
        let mut metadata = Vec::new();
        
        // Step 1: 添加comparable_pre_meta_data (12 bytes)
        // Python脚本第73-74行: nus3bank.seek(comparable_offset - 0xD); comparable_pre_meta_data = nus3bank.read(0xC)
        if let Some(pre_meta) = Self::extract_comparable_pre_meta_data(original_tone_data)? {
            metadata.extend_from_slice(&pre_meta);
        } else {
            // fallback: 使用默认的12字节
            metadata.extend_from_slice(&[0; 12]);
        }
        
        // Step 2: 添加track name长度和名称
        // Python脚本第104-105行
        let name_bytes = track.name.as_bytes();
        metadata.push((name_bytes.len() + 1) as u8); // len(append_name) + 1
        metadata.extend_from_slice(name_bytes);       // bytes(append_name, 'utf8')
        
        // Step 3: 添加null terminator（Python脚本隐含在第105行）
        metadata.push(0);
        
        // Step 4: 添加padding，完全按照Python脚本第106-111行的逻辑
        let mut counter = name_bytes.len() + 1;
        if counter % 4 == 0 {
            // if counter % 4 == 0: nus3bank.write(struct.pack(b"<I", 0))
            metadata.extend_from_slice(&[0; 4]);
        }
        while counter % 4 != 0 {
            // while counter % 4 != 0: nus3bank.write(struct.pack(b"B", 0)); counter += 1
            metadata.push(0);
            counter += 1;
        }
        
        // Step 5: 添加固定值序列，完全按照Python脚本第112-115行
        metadata.extend_from_slice(&[0; 4]);         // struct.pack(b"<I", 0)
        metadata.extend_from_slice(&BinaryReader::write_u32_le(0x8)); // struct.pack(b"<I", 0x8)
        metadata.extend_from_slice(&BinaryReader::write_u32_le(0));   // struct.pack(b"<I", 0) - pack_offset placeholder
        metadata.extend_from_slice(&BinaryReader::write_u32_le(0x22E8)); // struct.pack(b"<I", 0x22E8)
        
        // Step 6: 添加comparable_meta_data
        // Python脚本第116行: nus3bank.write(comparable_meta_data)
        if let Some(template_metadata) = Self::extract_comparable_meta_data(original_tone_data)? {
            metadata.extend_from_slice(&template_metadata);
        }
        
        Ok(metadata)
    }
    
    /// 提取comparable_pre_meta_data (12字节)
    /// 对应Python脚本第73-74行的逻辑
    fn extract_comparable_pre_meta_data(original_tone_data: &[u8]) -> Result<Option<Vec<u8>>, Nus3bankError> {
        // 找到第一个track name的位置，然后向前13字节(0xD)读取12字节
        // 这需要实现comparable track的查找逻辑
        // TODO: 实现更精确的查找逻辑
        if original_tone_data.len() >= 24 { // 基本的安全检查
            // 简化实现：从第一个track的metadata开始位置推算
            let first_track_offset = BinaryReader::read_u32_le_at(original_tone_data, 4)? as usize;
            if first_track_offset >= 13 && first_track_offset + 12 <= original_tone_data.len() {
                return Ok(Some(original_tone_data[first_track_offset - 13..first_track_offset - 1].to_vec()));
            }
        }
        Ok(None)
    }
    
    /// 提取comparable_meta_data（0x22E8之后的部分）
    /// 对应Python脚本第65-72行的get_sub_meta_size和comparable_meta_data提取
    fn extract_comparable_meta_data(original_tone_data: &[u8]) -> Result<Option<Vec<u8>>, Nus3bankError> {
        // 找到第一个0x22E8 magic value的位置
        for i in 0..original_tone_data.len().saturating_sub(4) {
            if &original_tone_data[i..i+4] == &0x22E8u32.to_le_bytes() {
                // 找到下一个0x22E8或文件结束
                for j in (i+4)..original_tone_data.len().saturating_sub(4) {
                    if &original_tone_data[j..j+4] == &0x22E8u32.to_le_bytes() {
                        return Ok(Some(original_tone_data[i+4..j].to_vec()));
                    }
                }
                // 如果没找到下一个，返回到section末尾
                return Ok(Some(original_tone_data[i+4..].to_vec()));
            }
        }
        Ok(None)
    }
    
    /// 更新现有track的metadata，主要是pack_offset
    fn update_existing_track_metadata(track: &AudioTrack, original_tone_data: &[u8]) -> Result<Vec<u8>, Nus3bankError> {
        // 找到此track的原始metadata
        if let Some(original_metadata) = Self::find_track_metadata(track, original_tone_data)? {
            let mut metadata = original_metadata;
            
            // 更新pack_offset（需要找到metadata中pack_offset的位置并更新）
            // 这通常在name + padding之后的固定位置
            if let Some(offset_pos) = Self::find_pack_offset_position(&metadata, &track.name) {
                let new_offset_bytes = BinaryReader::write_u32_le(track.pack_offset);
                metadata[offset_pos..offset_pos+4].copy_from_slice(&new_offset_bytes);
            }
            
            Ok(metadata)
        } else {
            // 如果找不到原始metadata，作为新track处理
            Self::build_new_track_metadata(track, original_tone_data)
        }
    }
}
```

#### 3.2 重写`src/nus3bank/writer.rs`的主要逻辑：

参考Python脚本的文件重建策略：保留所有section的原始数据，只重建TONE和PACK section，并正确更新文件大小。

```rust
impl Nus3bankWriter {
    pub fn write_file<P: AsRef<std::path::Path>>(file: &Nus3bankFile, path: P) -> Result<(), Nus3bankError> {
        // 1. 重新构建PACK section并更新track的pack_offset
        let mut tracks_with_updated_offsets = file.tracks.clone();
        let new_pack = Self::build_pack_section(&mut tracks_with_updated_offsets)?;
        
        // 2. 重新构建TONE section（需要原始TONE数据作为模板）
        let original_tone_data = file.get_original_tone_data()
            .ok_or_else(|| Nus3bankError::InvalidFormat("Missing original TONE data for template".to_string()))?;
        let new_tone = TONEBuilder::build_tone_section(&tracks_with_updated_offsets, &original_tone_data)?;
        
        // 3. 参考nus3append.py的逻辑构建完整文件
        let mut new_file = Vec::new();
        
        // NUS3 header
        new_file.extend_from_slice(b"NUS3");
        new_file.extend_from_slice(&[0; 4]); // placeholder for total size
        
        // BANKTOC header  
        new_file.extend_from_slice(b"BANK");
        new_file.extend_from_slice(b"TOC ");
        
        // 计算各section的大小（过滤掉空的sections）
        let mut sections = Vec::new();
        
        if let Some(prop_data) = &file.bank_info.prop_data {
            sections.push((b"PROP".as_slice(), prop_data.len()));
        }
        
        // BINF section always exists
        let binf_data = Self::build_binf_section(&file.bank_info)?;
        sections.push((b"BINF".as_slice(), binf_data.len()));
        
        // GRP section - 直接复制原始数据（TODO: 未来可能需要处理）
        if let Some(grp_data) = &file.bank_info.grp_data {
            sections.push((b"GRP ".as_slice(), grp_data.len()));
        }
        
        // DTON section - 直接复制原始数据（TODO: 未来可能需要处理）
        if let Some(dton_data) = &file.bank_info.dton_data {
            sections.push((b"DTON".as_slice(), dton_data.len()));
        }
        
        // TONE section - 重新生成
        sections.push((b"TONE".as_slice(), new_tone.len()));
        
        // JUNK section - 直接复制原始数据（TODO: 未来可能需要处理）
        if let Some(junk_data) = &file.bank_info.junk_data {
            sections.push((b"JUNK".as_slice(), junk_data.len()));
        }
        
        // PACK section - 重新生成
        sections.push((b"PACK".as_slice(), new_pack.len()));
        
        // 写入TOC header
        let toc_size = sections.len() * 8;
        new_file.extend_from_slice(&BinaryReader::write_u32_le(toc_size as u32));
        new_file.extend_from_slice(&BinaryReader::write_u32_le(sections.len() as u32));
        
        // 写入TOC entries
        for (magic, size) in &sections {
            new_file.extend_from_slice(magic);
            new_file.extend_from_slice(&BinaryReader::write_u32_le(*size as u32));
        }
        
        // 写入各个sections的数据
        for (magic, _) in &sections {
            match magic {
                b"PROP" => {
                    new_file.extend_from_slice(b"PROP");
                    if let Some(data) = &file.bank_info.prop_data {
                        new_file.extend_from_slice(&BinaryReader::write_u32_le(data.len() as u32));
                        new_file.extend_from_slice(data);
                    }
                },
                b"BINF" => {
                    new_file.extend_from_slice(b"BINF");
                    new_file.extend_from_slice(&BinaryReader::write_u32_le(binf_data.len() as u32));
                    new_file.extend_from_slice(&binf_data);
                },
                b"GRP " => {
                    // TODO: 直接复制原始数据，未来可能需要更智能的处理
                    if let Some(data) = &file.bank_info.grp_data {
                        new_file.extend_from_slice(b"GRP ");
                        new_file.extend_from_slice(&BinaryReader::write_u32_le(data.len() as u32));
                        new_file.extend_from_slice(data);
                    }
                },
                b"DTON" => {
                    // TODO: 直接复制原始数据，未来可能需要更智能的处理
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
                    // TODO: 直接复制原始数据，未来可能需要更智能的处理
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
        
        // 更新total size，完全按照Python脚本第86行的逻辑
        // Python: size + newToneSize + 8
        let original_size = file.bank_info.total_size;
        let new_tone_size_increase = new_tone.len() as u32 - file.get_original_tone_data().unwrap().len() as u32;
        let total_size = original_size + new_tone_size_increase + 8;
        new_file[4..8].copy_from_slice(&BinaryReader::write_u32_le(total_size));
        
        // 写入文件
        fs::write(path, new_file)?;
        Ok(())
    }
    
    /// 重新构建PACK section，参考nus3append.py不重新排序的逻辑
    /// 按照现有track的顺序重建，确保正确的offset计算
    fn build_pack_section(tracks: &mut [AudioTrack]) -> Result<Vec<u8>, Nus3bankError> {
        let mut pack_data = Vec::new();
        
        // 不排序，保持原有顺序（这很重要，因为TONE section中的pointer顺序需要对应）
        for track in tracks.iter_mut() {
            if let Some(audio_data) = &track.audio_data {
                // 更新track的pack_offset为当前position
                track.pack_offset = pack_data.len() as u32;
                
                // 添加音频数据到PACK
                pack_data.extend_from_slice(audio_data);
                
                // 根据需要添加padding以保持对齐
                // 注意：这里可能需要根据具体的NUS3BANK规范调整padding策略
                let padding = BinaryReader::calculate_padding(audio_data.len());
                if padding > 0 {
                    pack_data.extend(std::iter::repeat(0u8).take(padding));
                }
            } else {
                // 对于没有新audio_data的track，保持原有的pack_offset
                // 这种情况下，原始音频数据应该已经包含在某个地方
                // 这里可能需要从原始PACK section中复制数据
                // TODO: 实现从原始PACK section复制现有track数据的逻辑
            }
        }
        
        Ok(pack_data)
    }
    
    /// 构建BINF section（保持现有逻辑）
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

### 4. Python脚本的关键计算公式

参考nus3append.py第78-79行，新track metadata的大小计算：
```rust
/// 计算新track metadata的大小，完全按照Python脚本第78-79行
/// newToneSize = comparable_size + 28 + len(append_name) + 1
/// newToneSize += 4 - ((len(append_name) + 1) % 4)
fn calculate_new_tone_size(track_name: &str, comparable_meta_size: u32) -> u32 {
    let name_len = track_name.len();
    let mut new_tone_size = comparable_meta_size + 28 + name_len as u32 + 1;
    new_tone_size += 4 - ((name_len + 1) % 4) as u32;
    new_tone_size
}
```

### 5. 基于Python脚本的Add/Remove方法重写

#### 5.1 参考nus3append.py重写`add_track`方法：

```rust
pub fn add_track(&mut self, name: String, audio_data: Vec<u8>) -> Result<String, Nus3bankError> {
    // 验证输入
    if name.is_empty() {
        return Err(Nus3bankError::InvalidParameter("Track name cannot be empty".to_string()));
    }
    
    if audio_data.is_empty() {
        return Err(Nus3bankError::InvalidParameter("Audio data cannot be empty".to_string()));
    }
    
    // 生成新ID（参考Python脚本的逻辑：新track添加到最后）
    let new_id = self.tracks.iter()
        .map(|t| t.numeric_id)
        .max()
        .unwrap_or(0) + 1;
    
    let hex_id = format!("0x{:x}", new_id);
    
    // 创建新track
    // 注意：pack_offset和metadata相关字段将在保存时重新计算
    // 这符合nus3append.py中只在写入时计算偏移的策略
    let track = AudioTrack {
        index: self.tracks.len(),
        hex_id: hex_id.clone(),
        numeric_id: new_id,
        name,
        pack_offset: 0, // 将在write_file时重新计算
        size: audio_data.len() as u32,
        metadata_offset: 0, // 将在build_tone_section时重新计算
        metadata_size: 0,   // 将在build_tone_section时重新计算
        audio_data: Some(audio_data),
        audio_format: if audio_data.starts_with(b"RIFF") { 
            AudioFormat::Wav 
        } else { 
            AudioFormat::Unknown 
        },
    };
    
    // 添加到tracks列表（参考Python脚本：添加到最后）
    self.tracks.push(track);
    
    // 更新track count（对应Python脚本中的toneCount + 1）
    self.bank_info.track_count = self.tracks.len() as u32;
    
    Ok(hex_id)
}

/// Remove track by hex_id
/// 注意：Python脚本nus3append.py只实现了ADD操作，REMOVE操作需要额外设计
/// 删除track后需要重新构建TONE和PACK sections，并重新计算所有offset
pub fn remove_track(&mut self, hex_id: &str) -> Result<bool, Nus3bankError> {
    // 找到要删除的track index
    let track_index = self.tracks.iter()
        .position(|t| t.hex_id == hex_id);
    
    if let Some(index) = track_index {
        // 删除track
        self.tracks.remove(index);
        
        // 重新计算所有track的index
        for (i, track) in self.tracks.iter_mut().enumerate() {
            track.index = i;
        }
        
        // 更新track count
        self.bank_info.track_count = self.tracks.len() as u32;
        
        Ok(true)
    } else {
        Ok(false)
    }
}
```

### 5. 需要在structures.rs中添加的辅助方法

```rust
impl Nus3bankFile {
    /// 获取原始TONE section数据，用作新track metadata的模板
    /// 这是参考nus3append.py实现的关键方法
    pub fn get_original_tone_data(&self) -> Option<&Vec<u8>> {
        self.bank_info.original_tone_data.as_ref()
    }
    
    /// 设置原始TONE section数据（在解析时调用）
    pub fn set_original_tone_data(&mut self, tone_data: Vec<u8>) {
        self.bank_info.original_tone_data = Some(tone_data);
    }
}

impl BankInfo {
    /// 在BankInfo中添加original_tone_data字段来保存原始TONE数据
    pub original_tone_data: Option<Vec<u8>>,
}
```

### 6. Parser修改要点

在`src/nus3bank/parser.rs`的TONE section解析中：

```rust
b"TONE" => {
    section_offsets.tone_offset = current_pos;
    let tone_data = Self::parse_tone_section(reader, expected_size)?;
    
    // 同时保存原始数据作为模板（关键改动）
    reader.seek(SeekFrom::Start(current_pos as u64))?;
    let raw_tone_data = Self::read_raw_section(reader, expected_size)?;
    bank_info.as_mut().unwrap().original_tone_data = Some(raw_tone_data);
    
    // 解析后的tracks数据
    tracks.extend(tone_data);
},
```
