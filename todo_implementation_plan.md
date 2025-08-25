# NUS3BANK Implementation TODO List

## Phase 1: Core Binary Format Implementation (Critical Priority)

### Task 1.1: Create Binary Reading Utilities Module
**File**: `src/nus3bank/binary_utils.rs`
- [ ] Implement little-endian reading functions
  - [ ] `read_u8(reader)` - Read single byte
  - [ ] `read_u16_le(reader)` - Read 16-bit little-endian
  - [ ] `read_u32_le(reader)` - Read 32-bit little-endian (primary)
  - [ ] `read_f32_le(reader)` - Read 32-bit float little-endian
- [ ] Implement validation helpers
  - [ ] `assert_magic(reader, expected)` - Validate magic numbers
  - [ ] `calculate_padding(string_size)` - Calculate 4-byte alignment
- [ ] Add comprehensive error handling with custom error types

### Task 1.2: Design NUS3BANK Data Structures
**File**: `src/nus3bank/structures.rs`
- [ ] Create `Nus3bankFile` struct
  ```rust
  pub struct Nus3bankFile {
      pub bank_info: BankInfo,
      pub tracks: Vec<AudioTrack>,
      pub compressed: bool,
      pub decompressed_path: Option<PathBuf>,
  }
  ```
- [ ] Create `BankInfo` struct for file metadata
  ```rust
  pub struct BankInfo {
      pub bank_id: u32,
      pub bank_string: String,
      pub total_size: u32,
      pub track_count: u32,
      // Section offsets
      pub pack_offset: u32,
      pub tone_offset: u32,
      // ... other sections
  }
  ```
- [ ] Create `AudioTrack` struct for individual tracks
  ```rust
  pub struct AudioTrack {
      pub index: usize,           // Sequential index
      pub hex_id: String,         // "0x0", "0xb2", etc.
      pub numeric_id: u32,        // Actual hex value
      pub name: String,           // Track name
      pub pack_offset: u32,       // Offset within PACK section
      pub size: u32,              // Audio data size
      pub metadata_offset: u32,   // TONE metadata location
      pub metadata_size: u32,     // Metadata size
      pub audio_data: Option<Vec<u8>>, // Raw IDSP data (loaded on demand)
  }
  ```
- [ ] Create section structures (PROP, BINF, TONE, PACK, etc.)

### Task 1.3: Implement File Format Parsing
**File**: `src/nus3bank/parser.rs`
- [ ] Implement header validation and decompression
  ```rust
  pub fn parse_nus3bank(file_path: &Path) -> Result<Nus3bankFile, Nus3bankError>
  ```
- [ ] Implement section discovery and offset mapping
- [ ] Implement BINF section parsing (bank ID and string extraction)
- [ ] Implement TONE section parsing (complex metadata handling)
- [ ] Handle format variations (tempByte > 9 vs ≤ 9)
- [ ] Implement string alignment and padding logic
- [ ] Add comprehensive validation for each parsing step

## Phase 2: Export Functionality (High Priority)

### Task 2.1: Implement Audio Track Extraction
**File**: `src/nus3bank/export.rs`
- [ ] Extract audio tracks to IDSP files
- [ ] Generate filenames with pattern: `{hex_id}-{track_name}.idsp`
- [ ] Handle non-consecutive hex IDs correctly
- [ ] Implement batch export functionality
- [ ] Add progress reporting for large files

### Task 2.2: Integration with Existing Export Utils
**File**: `src/ui/main_area/export_utils.rs` (extend existing)
- [ ] Add NUS3BANK export support alongside NUS3AUDIO
- [ ] Reuse existing vgmstream conversion pipeline
- [ ] Maintain compatibility with current export UI

## Phase 3: Import and Replace Functionality (High Priority)

### Task 3.1: Implement File Loading
**File**: `src/nus3bank/import.rs`
- [ ] Load and validate NUS3BANK files
- [ ] Parse all tracks and metadata
- [ ] Integrate with existing `AudioFileInfo` structure
- [ ] Handle compressed files (zlib decompression)

### Task 3.2: Implement Replace Operations
**File**: `src/nus3bank/replace.rs`
- [ ] Replace individual tracks by hex ID
- [ ] Implement complete file reconstruction workflow
- [ ] Recalculate all offsets when size changes
- [ ] Update PACK section size header
- [ ] Update total file size header
- [ ] Validate data integrity after replacement

### Task 3.3: Integration with Existing Replace Utils
**File**: `src/ui/main_area/replace_utils.rs` (extend existing)
- [ ] Add NUS3BANK replace support
- [ ] Maintain in-memory replacement system compatibility
- [ ] Handle format conversion (WAV to IDSP if needed)

## Phase 4: Add/Remove Operations (Medium Priority)

### Task 4.1: Implement Track Addition
**File**: `src/nus3bank/add.rs`
- [ ] Add new tracks to NUS3BANK files
- [ ] Implement ID allocation strategy
- [ ] Handle metadata insertion in TONE section
- [ ] Update all size headers and offsets

### Task 4.2: Implement Track Removal
**File**: `src/nus3bank/remove.rs`
- [ ] Remove tracks by hex ID
- [ ] Compact PACK section after removal
- [ ] Update TONE section to remove metadata
- [ ] Recalculate all dependent offsets

## Phase 5: UI Integration (Medium Priority)

### Task 5.1: Update Audio File Info Structure
**File**: `src/ui/main_area/audio_file_info.rs`
- [ ] Add support for hex_id field
- [ ] Add bank_info field for NUS3BANK files
- [ ] Maintain backward compatibility with NUS3AUDIO

### Task 5.2: Update Main Area Components
**Files**: Various main_area modules
- [ ] Update file loading to detect NUS3BANK vs NUS3AUDIO
- [ ] Add NUS3BANK-specific UI elements (hex ID display)
- [ ] Update table rendering to show hex IDs
- [ ] Add bank information display

### Task 5.3: Update Audio Player Integration
**File**: `src/ui/audio_player/audio_player_component.rs`
- [ ] Support loading audio from NUS3BANK files
- [ ] Handle IDSP format audio data
- [ ] Maintain compatibility with existing audio backend

## Phase 6: Testing and Validation (Critical Priority)

### Task 6.1: Unit Tests
- [ ] Test binary reading utilities with sample data
- [ ] Test data structure parsing
- [ ] Test string alignment calculations
- [ ] Test offset calculations

### Task 6.2: Integration Tests
- [ ] Test complete file parsing with sample NUS3BANK
- [ ] Test export functionality
- [ ] Test replace operations
- [ ] Test add/remove operations
- [ ] Validate file integrity after modifications

### Task 6.3: Sample File Testing
- [ ] Use provided sample: `se_chr_001gundam_001gundam_001.nus3bank`
- [ ] Verify all 18 tracks extract correctly
- [ ] Verify hex ID pattern: 0x0-0x9, 0xf-0x15, 0xb2
- [ ] Test bank info extraction: ID=12, string="SE_CHR_001GUNDAM_001GUNDAM_001"

## Dependencies to Add

### Required Crates
```toml
# Add to Cargo.toml
flate2 = "1.0"      # For zlib decompression
byteorder = "1.4"   # For binary data reading
thiserror = "1.0"   # For custom error types
```

## Error Handling Strategy

### Custom Error Types
```rust
#[derive(Debug, thiserror::Error)]
pub enum Nus3bankError {
    #[error("Invalid magic number: expected {expected}, found {found}")]
    InvalidMagic { expected: String, found: String },
    
    #[error("Section validation failed: {section}")]
    SectionValidation { section: String },
    
    #[error("String alignment error: size {size}, padding {padding}")]
    StringAlignment { size: usize, padding: usize },
    
    #[error("File reconstruction failed: {reason}")]
    Reconstruction { reason: String },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

## Implementation Notes

### Critical Requirements
1. **Byte-perfect accuracy**: Binary format must match Python implementation exactly
2. **String alignment**: 4-byte padding calculations are critical for file integrity
3. **Offset management**: All offsets must be recalculated correctly when modifying files
4. **Memory efficiency**: Handle large files without excessive memory usage
5. **Error handling**: Comprehensive validation at each parsing step

### Testing Strategy
1. **Start with sample file**: Use provided `se_chr_001gundam_001gundam_001.nus3bank`
2. **Validate against known output**: Compare with Python implementation results
3. **Test edge cases**: Empty files, corrupted data, size limits
4. **Performance testing**: Large file handling, memory usage

### Success Criteria
1. **Export**: Successfully extract all 18 tracks from sample file
2. **Replace**: Modify track and reconstruct valid file
3. **Add/Remove**: Insert/delete tracks maintaining file integrity
4. **UI Integration**: Seamless operation within existing interface
5. **Performance**: Handle typical game archive sizes efficiently

## Risk Mitigation

### High-Risk Areas
1. **Binary format parsing**: Implement extensive validation
2. **File reconstruction**: Test with multiple scenarios
3. **Memory usage**: Monitor and optimize for large files
4. **UI integration**: Maintain backward compatibility

### Fallback Strategy
If implementation becomes too complex:
1. Create detailed progress documentation
2. Implement minimal viable product (export-only)
3. Leave comprehensive notes for next developer
4. Consider external library integration if available
