# NUS3BANK DTON Section Analysis

## Overview
DTON section是NUS3BANK格式中的一个中等大小section，从您提供的示例中显示该section包含248字节的数据。其名称可能与"Data Tone"或类似概念相关。

## Current Implementation Status
- ❌ **未实现**: 当前代码完全跳过DTON section
- ❌ **数据丢失**: DTON section数据在add/remove操作时会丢失  
- ❌ **格式不完整**: 生成的NUS3BANK文件缺少DTON section

## Section Analysis

### Basic Information
```
Section Name: DTON (4 bytes: "DTON")
Section Size: 248 bytes (from example)
Position: TOC entry 3 (after PROP, BINF, GRP)
Size Characteristics: 中等固定大小，可能包含结构化数据
```

### Possible Data Content (需要实际hex分析确认)

#### Hypothesis 1: Audio Tone/Pitch Information
DTON可能代表"Data Tone"，包含音频音调相关信息：
```
Possible Structure:
├── Tone Count (4 bytes)
├── Global Tone Settings (variable)
├── Per-Track Tone Data:
│   ├── Track ID (4 bytes)
│   ├── Base Pitch (4 bytes, float?)
│   ├── Pitch Modulation (4 bytes)
│   └── Tone Parameters (variable)
└── Padding/Reserved (align to section size)
```

#### Hypothesis 2: Audio Timing/Duration Information
可能包含音频时序相关的数据：
```
Possible Structure:
├── Timing Parameters (global)
├── Track Duration Data:
│   ├── Track ID (4 bytes)
│   ├── Duration (4 bytes, milliseconds?)
│   ├── Loop Points (8 bytes, start/end?)
│   └── Timing Flags (4 bytes)
└── Additional Timing Metadata
```

#### Hypothesis 3: Audio Decoding/Processing Parameters
可能包含音频解码或处理的配置信息：
```
Possible Structure:
├── Decoder Configuration (global)
├── Per-Track Processing Data:
│   ├── Track ID (4 bytes)
│   ├── Processing Flags (4 bytes)
│   ├── Quality Settings (4 bytes)
│   └── Format Parameters (variable)
└── Reserved/Padding
```

## Data Size Analysis

### 248 bytes的可能分布：
- **Header**: 16-32 bytes (section metadata)
- **Per-Track Data**: 如果与tracks相关，248字节可以容纳:
  - 约15-20个tracks，每个16字节数据
  - 约10-12个tracks，每个20字节数据
  - 约8-10个tracks，每个24字节数据
- **Footer/Padding**: 8-16 bytes

## Relationship with Other Sections

### 与TONE Section的关系
- **TONE**: 包含track metadata和在PACK中的位置信息
- **DTON**: 可能包含与TONE配套的其他音频参数
- **可能的互补关系**: TONE负责track基本信息，DTON负责音频特性参数

### 与Track Operations的关系
如果DTON包含per-track数据：
- **Add Track**: 需要在DTON中添加对应的数据项
- **Remove Track**: 需要从DTON中移除对应的数据项
- **Track ID一致性**: 必须保持与TONE section中track IDs的一致性

## Impact on Add/Remove Operations

### 🚨 Critical Issues

1. **Track Reference Integrity**: 如果DTON包含track references，add/remove操作必须同步更新
2. **Data Consistency**: DTON数据必须与TONE section中的tracks保持一致
3. **Size Management**: 固定大小的DTON可能对track数量有限制

### Required Actions for Implementation

#### Phase 1: Data Preservation (Immediate)
```rust
// 在parser.rs中添加DTON section数据保存
b"DTON" => {
    section_offsets.dton_offset = current_pos;
    let dton_data = Self::read_raw_section(reader, expected_size)?;
    // 保存原始数据以便在writer中使用
    bank_info.dton_data = Some(dton_data);
},
```

#### Phase 2: Structure Analysis (Research)
```rust
// 分析DTON section的结构
fn analyze_dton_structure(dton_data: &[u8]) -> Result<DTONInfo, Nus3bankError> {
    // 1. 查找4字节对齐的数据模式
    // 2. 识别可能的计数器
    // 3. 查找与track IDs匹配的数值
    // 4. 分析数据重复模式
}
```

#### Phase 3: Smart Updates (Future)
如果确认DTON包含track-specific数据：
```rust
// 在add_track时更新DTON
fn update_dton_for_new_track(dton_data: &mut Vec<u8>, track_id: u32, default_params: &DTONParams) -> Result<(), Nus3bankError> {
    // 解析现有DTON数据
    // 添加新track的DTON entry
    // 重新序列化DTON数据
}

// 在remove_track时更新DTON
fn update_dton_for_removed_track(dton_data: &mut Vec<u8>, track_id: u32) -> Result<(), Nus3bankError> {
    // 查找并移除对应track的DTON entry
    // 重新计算数据大小和偏移
}
```

## Research Methods

### 1. Hex Pattern Analysis
```bash
# 分析DTON section的hex模式
xxd -s +<dton_offset> -l 248 nus3bank_file.nus3bank

# 查找重复的4字节模式
hexdump -C nus3bank_file.nus3bank | grep -A 20 -B 5 "DTON"
```

### 2. Track Correlation Analysis
```rust
// 分析DTON数据与track数量的关系
fn analyze_track_correlation(tracks: &[AudioTrack], dton_data: &[u8]) {
    println!("Track count: {}", tracks.len());
    println!("DTON size: {}", dton_data.len());
    
    // 查找DTON中是否有与track IDs匹配的数值
    for track in tracks {
        let id_bytes = track.numeric_id.to_le_bytes();
        if let Some(pos) = find_bytes_in_data(dton_data, &id_bytes) {
            println!("Found track ID {} at DTON offset {}", track.numeric_id, pos);
        }
    }
}
```

### 3. Multiple File Comparison
- 比较不同track数量的NUS3BANK文件的DTON sections
- 分析DTON size是否随track数量变化
- 查找DTON数据的固定部分和可变部分

## Implementation Strategy

### Conservative Approach (推荐)
1. **保存原始数据**: 完整保存DTON section数据
2. **只读模式**: 在确认格式之前不修改DTON数据
3. **完整性验证**: 确保写回的DTON数据与原始数据一致

### Aggressive Approach (风险较高)
1. **假设track关联**: 假设DTON包含per-track数据
2. **尝试解析**: 基于假设进行数据解析
3. **动态更新**: 在add/remove操作时更新DTON

## Data Structure Definitions (基于research结果)

```rust
// 待研究确认的DTON数据结构
#[derive(Clone, Debug)]
pub struct DTONSection {
    pub header: DTONHeader,
    pub track_data: Vec<DTONTrackData>,
    pub footer: Option<Vec<u8>>, // padding或其他数据
}

#[derive(Clone, Debug)]
pub struct DTONHeader {
    pub unknown1: u32,
    pub track_count: u32, // 如果确认存在
    pub global_params: Vec<u8>, // 其他全局参数
}

#[derive(Clone, Debug)]
pub struct DTONTrackData {
    pub track_id: u32, // 如果确认存在
    pub params: Vec<u8>, // track-specific参数
}
```

## Test Plan

### 测试用例1: 数据完整性
```rust
#[test]
fn test_dton_data_preservation() {
    let original_file = load_nus3bank("test.nus3bank");
    let saved_file = save_and_reload(original_file);
    assert_eq!(original_file.bank_info.dton_data, saved_file.bank_info.dton_data);
}
```

### 测试用例2: Track操作兼容性
```rust
#[test]
fn test_dton_track_operations() {
    let mut file = load_nus3bank("test.nus3bank");
    let original_dton = file.bank_info.dton_data.clone();
    
    // 添加track
    file.add_track("test".to_string(), vec![0; 1000]);
    
    // 如果DTON不包含track-specific数据，应该保持不变
    // 如果包含，需要验证更新的正确性
    // assert_eq!(file.bank_info.dton_data, original_dton); // 或其他验证逻辑
}
```

## Implementation Priority

### High Priority (Must-have)
- [x] 保存原始DTON数据
- [ ] 在writer中正确写回DTON section
- [ ] 验证DTON数据完整性

### Medium Priority (Should-have)
- [ ] Hex分析确定DTON结构
- [ ] 确认是否包含track references
- [ ] 基本的DTON数据解析

### Low Priority (Nice-to-have)
- [ ] 完整的DTON format解析
- [ ] 智能更新DTON在track操作时
- [ ] DTON数据的语义理解

## Risk Assessment

### 风险1: 数据格式复杂导致解析困难
**概率**: 中等  
**影响**: 中等  
**缓解策略**: 采用保守策略，保存原始数据

### 风险2: DTON与tracks强关联导致add/remove失败
**概率**: 高 (如果DTON包含track data)  
**影响**: 高  
**缓解策略**: 详细的hex分析和多文件比较

### 风险3: 固定大小限制导致功能受限
**概率**: 低  
**影响**: 中等  
**缓解策略**: 分析size模式，确认是否为固定大小

这个文档将根据实际的hex分析结果进行更新。
