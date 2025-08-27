# NUS3BANK GRP Section Analysis

## Overview
GRP section是NUS3BANK格式中的一个重要section，从您提供的示例中显示该section包含55056字节的数据，是除了PACK section之外最大的section。

## Current Implementation Status
- ❌ **未实现**: 当前代码完全跳过GRP section
- ❌ **数据丢失**: GRP section数据在add/remove操作时会丢失
- ❌ **格式不完整**: 生成的NUS3BANK文件缺少GRP section

## Section Analysis

### Basic Information
```
Section Name: GRP (4 bytes: "GRP ")
Section Size: 55056 bytes (from example)
Position: TOC entry 2 (after PROP and BINF)
```

### Possible Data Content (需要实际hex分析确认)

#### Hypothesis 1: Audio Group Definitions
GRP可能代表"Group"，用于定义audio tracks的分组关系：
```
Possible Structure:
├── Group Count (4 bytes)
├── Group Definitions:
│   ├── Group ID (4 bytes)
│   ├── Group Name Length (4 bytes)
│   ├── Group Name (variable)
│   ├── Track Count in Group (4 bytes)
│   └── Track IDs List (4 bytes each)
└── Additional Metadata
```

#### Hypothesis 2: Track Organization Metadata
可能包含track的逻辑组织信息：
- Track显示顺序
- Track分类信息
- UI显示相关的元数据

#### Hypothesis 3: Audio Processing Parameters
可能包含音频处理相关的全局参数：
- 音频采样率配置
- 音频格式转换参数
- 音频效果链配置

## Impact on Add/Remove Operations

### 🚨 Critical Issues

1. **Data Integrity**: 如果不保存GRP section数据，修改后的文件可能无法正常工作
2. **Track References**: GRP可能包含对track IDs的引用，添加/删除tracks时需要更新
3. **Group Consistency**: 删除tracks时可能需要更新相关的group信息

### Required Actions for Implementation

#### Phase 1: Data Preservation (Immediate)
```rust
// 在parser.rs中添加GRP section数据保存
b"GRP " => {
    section_offsets.grp_offset = current_pos;
    let grp_data = Self::read_raw_section(reader, expected_size)?;
    // 保存原始数据以便在writer中使用
    bank_info.grp_data = Some(grp_data);
},
```

#### Phase 2: Data Analysis (Research)
1. **Hex Dump Analysis**: 分析实际GRP section的hex数据
2. **Pattern Recognition**: 识别数据中的模式和结构
3. **Track Correlation**: 分析GRP数据与track IDs的关联性

#### Phase 3: Smart Updates (Future)
如果发现GRP包含track references：
```rust
// 在add_track时可能需要更新GRP
pub fn add_track_with_grp_update(&mut self, name: String, audio_data: Vec<u8>) -> Result<String, Nus3bankError> {
    let hex_id = self.add_track(name, audio_data)?;
    
    // 检查是否需要更新GRP section
    if let Some(ref mut grp_data) = self.bank_info.grp_data {
        Self::update_grp_for_new_track(grp_data, &hex_id)?;
    }
    
    Ok(hex_id)
}
```

## Research Methods

### 1. Hex Analysis Tools
```bash
# 使用hex editor分析GRP section
hexdump -C nus3bank_file.nus3bank | grep -A 50 -B 5 "GRP"
```

### 2. Pattern Identification
- 查找重复的4字节模式 (可能是IDs)
- 查找字符串模式 (可能是names)
- 查找计数器模式 (可能是counts)

### 3. Correlation Analysis
- 比较不同NUS3BANK文件的GRP sections
- 分析GRP size与track count的关系
- 查找GRP与TONE section的数据关联

## Implementation Priority

### High Priority (Must-have)
- [x] 保存原始GRP数据以防止数据丢失
- [ ] 在writer中正确写回GRP section
- [ ] 在structures.rs中添加grp_data字段

### Medium Priority (Should-have)
- [ ] 基础的hex dump分析
- [ ] 识别GRP的基本结构
- [ ] 确定是否包含track references

### Low Priority (Nice-to-have)
- [ ] 完全解析GRP格式
- [ ] 智能更新GRP在track修改时
- [ ] GRP数据的完整性验证

## Risks and Mitigation

### Risk 1: 数据格式错误导致文件损坏
**Mitigation**: 始终保持原始GRP数据的完整性，仅在确认格式后进行修改

### Risk 2: Track ID references不一致
**Mitigation**: 在确认GRP包含track references之前，不进行任何修改

### Risk 3: 兼容性问题
**Mitigation**: 保持与原始格式的完全兼容性，避免引入新的数据

## Next Steps

1. **立即实施数据保存机制**
2. **收集更多样本文件进行分析**
3. **执行hex dump分析以确定数据结构**
4. **基于分析结果制定详细的解析策略**

## Test Cases

### 测试用例1: 数据保存完整性
- 解析包含GRP的NUS3BANK文件
- 验证GRP数据被正确保存
- 写回文件并验证GRP section完整性

### 测试用例2: Add/Remove操作兼容性
- 添加新track后验证GRP section保持不变
- 删除track后验证文件仍可正常解析
- 比较操作前后的GRP section是否一致

这个分析文档将随着我们对GRP section的了解加深而不断更新。
