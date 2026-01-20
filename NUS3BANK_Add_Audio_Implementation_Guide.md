# NUS3BANK Add Audio Implementation Guide (Current Rust Implementation)

## Overview

本文件描述 **EXVS2 Audio Editor 当前的 Rust NUS3BANK 实现**如何添加音频，以及在写回时如何同时更新 `TONE` 与 `PACK`。

实现以 `NUS3BANK.cs` 作为主要参考来源，但为了适配 EXVS2 实际样本文件，解析层做了若干 **确定性兼容**（例如：`PROP` 精简版、`JUNK` 可变长度、`TONE` meta 变体、`unkvalues` pair 顺序差异等）。

## Implementation Status

- **Supported mode**: BANKTOC-only (`NUS3` + `BANKTOC `)
- **Sections parsed**: `PROP`, `BINF`, `GRP `, `DTON`, `TONE`, `JUNK`, `PACK`
- **Write-back**: Rebuilds `PACK` and `TONE` from the in-memory model; preserves unknown sections as raw bytes
- **Validation**: Unit tests + smoke tests for real sample files + debug JSON serialization test

## Key Differences vs `NUS3BANK.cs` (Observed in Real Files)

- **PROP layout**:
  - Some files contain a minimal `PROP` that ends after `project` and does not include `unk3/timestamp`.
  - The 2-byte field skipped by C# may be non-zero and must be preserved.
- **JUNK size**:
  - Not always 4; some files use 8 (and potentially other small sizes). We preserve payload bytes as-is.
- **TONE meta variability**:
  - Some entries are stub/placeholder metas with very small `meta_size`.
  - Some metas include an 8-byte prefix before the normal meta layout.
  - Some files store `unkvalues` pairs as `(value:f32, idx:i32)` instead of `(idx:i32, value:f32)`.
  - Some short meta layouts are not fully understood; for debug/export we preserve raw bytes without guessing semantics.

## Public API (Add/Replace/Remove)

All operations are performed on `Nus3bankFile` and persisted via `save()`.

### Replace audio payload

```rust
let mut bank = crate::nus3bank::structures::Nus3bankFile::open("input.nus3bank")?;
bank.replace_track_data("0x0", new_audio_bytes)?;
bank.save("output.nus3bank")?;
```

### Add audio track

```rust
let mut bank = crate::nus3bank::structures::Nus3bankFile::open("input.nus3bank")?;
let new_hex_id = bank.add_track("new_track_name".to_string(), new_audio_bytes)?;
bank.save("output.nus3bank")?;
```

### Remove audio track

```rust
let mut bank = crate::nus3bank::structures::Nus3bankFile::open("input.nus3bank")?;
bank.remove_track("0x0")?;
bank.save("output.nus3bank")?;
```

## How Save Updates `TONE` and `PACK`

- **PACK**:
  - Rebuilt as a concatenation of all active `ToneMeta.payload` in order.
  - Each payload is padded to 4-byte alignment.
  - Each `ToneMeta.offset/size` is updated to match the new PACK layout.
- **TONE**:
  - Rebuilt from the active `ToneMeta` list.
  - `unkvalues` pairs are written using the per-meta recorded order (`IndexThenValue` or `ValueThenIndex`).
  - If a meta had an 8-byte `meta_prefix`, it is preserved during rebuild.

## Debug System: Export NUS3BANK as JSON

为了方便你检查每个字段与变体差异，提供 `--debug-json` 导出。

### Export JSON

```bash
cargo run -- --debug-json input.nus3bank
```

Output file:

- `input.nus3bank.json`

Or specify an output path:

```bash
cargo run -- --debug-json input.nus3bank out.json
```

### What JSON contains

- **toc**: section order and sizes
- **sections**: parsed structs (`prop/binf/grp/dton/tone/junk/pack/unknown`)
- **tracks**: UI-facing derived list (`hex_id`, `name`, `pack_offset`, `size`, etc.)
- **tone.tones**:
  - `meta_prefix_len` and optional base64 preview
  - `unkvalues_pair_order`
  - `removed=true` for stub/unsupported meta layouts

## Limitations and Safety Notes

- Only BANKTOC-only NUS3BANK variants are supported.
- For some rare `TONE` meta layouts (short metas not matching the main structure), the parser does **not** guess field meanings:
  - The raw bytes are preserved for inspection in JSON (`meta_prefix`) and the entry is marked `removed=true`.
  - Editing and saving such files may drop/alter those unknown metas depending on how the writer rebuilds active tones.

