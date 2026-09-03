# OB `nus3_decode_tone_descriptor` AV from Audio Editor save

**Status:** E2 file + IDA; in-game AV is E3 for this exact bank  
**Date:** 2026-08-29  
**Crashed binary:** `E:\OBHK0.3_v27\vsac27_Release.exe` (OB, in-scope)  
**Crashed bank:** `E:\XB\mod\090sound\wing_gundam_zero_rebellion_sound\SE_CHR_016GUNDMW_001WGZERO_001.nus3bank` (`0xDFA2BBCE`)  
**Clean control:** `E:\XB\mod\090sound\016gundmw_001wgzero_001\SE_CHR_016GUNDMW_001WGZERO_001.nus3bank` (`0x421D77F3`)

This note is the owner write-up for the load-time access violation. The game did not crash in MSC, model, or strikertable. It crashed while `CSndBankData_FinalizeLoad` instantiated TONE descriptors written by this editor.

Related later crash (bank **did** load, jingle `ResolveTone` null): 
[`ob-bgm-jingle-resolve-tone-av.md`](ob-bgm-jingle-resolve-tone-av.md)
(`rva=0x1CA4A1`, pack `0x0C568109` present in `[GAME-LOG]`). Do not treat that
as this FinalizeLoad AV, and do not shrink the BGM donor name slot to “fix” it.

## Crash (game)

Log: `E:\OBHK0.3_v27\EXVS2-debug-20260829-144859.log`

```
access violation: read at 0x19F7C96EF04
faulting rva=0x11CE4ED
callers: 0x11CE595 <- 0x11CE7E4 <- 0x1B647E <- 0x11C9D2D
      <- 0x1B57D4 <- 0x1B5627 <- 0x1B46AE <- 0x1C10AA <- …
```

The first CrashReport line (`faulting rva=0x63993E7FD`) is a nested AV while walking a garbage RIP. Ignore it. `EXVS2-fault.log` has many `…E7FD` first-chance lines of the same kind.

| RVA | Function | What it was doing |
|---|---|---|
| `0x11CE4ED` | `nus3_decode_tone_descriptor` | `mov edx, [rbx]`: presence-mask **bit 19** reads a hash-count from the packed cursor |
| `0x11CE595` | `sub_1411CE550` | Probe pass (`a3=0`) after `call nus3_decode_tone_descriptor` |
| `0x11CE7E4` | `sub_1411CE7A0` | Get-or-create tone by index |
| `0x11C9D2D` | `sub_1411C9CE0` | HBank `vtable+0x180`. Implementation: `vtable+0x1B0(this, 11, index, &obj)` — **11 is TONE chunk type** |
| `0x1B57D4` | `CSndBankData_BuildSourceCrcToToneIdMap` | Enumerates every cue, hashes the source string, maps CRC → tone id |
| `0x1B46AE` | `CSndBankData_FinalizeLoad` | After `LoadHBankHandle`, always builds that map |

`rbx=0x19F7C96EF04` is not a one-dword overread. The packed cursor had already walked off the descriptor. Bit 19 then treated unmapped memory as a count.

Same family, older faults in `EXVS2-fault.log`: `rva=0x11CE5F9` (`mov r8, [[bank+0x38]+idx*8]`) — probe succeeded, then the DTON template index was bad. Same load path, second death.

Engine decoder contract (OB `0x1411CDB10`):

1. Packed stream starts at TONE record + 8.
2. At stream+4, read signed dwords until the first non-negative word (continuation mask).
3. Payload cursor is immediately after those words.
4. Each set bit consumes a typed field from the cursor into a 352-byte runtime tone object.
5. Bit 19 of the dword at `rsi+4` (second mask word when the mask is two dwords) consumes `count` then up to four hashes stored at `tone+0x14C` with `+0x10000000`.

The editor must emit a stream this function can walk without leaving the record.

## File diff (editor output vs vanilla)

| | Vanilla `0x421D77F3` | Saved `0xDFA2BBCE` |
|---|---|---|
| Size | 539400 | **546620** |
| MD5 | `5ba221c2669885fa0b593a3aec5d0475` | **`b95f2bf2977cb7ac4060f172bc95b980`** |
| TONE count | 86 | 86 |
| TONE section | 7648 | **7964 (+316)** |
| GRP / BINF / PROP | identical | identical |
| DTON count | 1 (`Default`) | 1 (`Default`) |
| DTON entry size at +16 | `0xEC` | **`0xE8` (−4)** |
| TONE 14 | 12-byte stub | **328-byte live cue `new_test_audio`** |

TONE 14 is the first removed stub in the vanilla pointer table. `Nus3bankFile::add_track` reuses the first `removed` slot. The added name in tests and in this bank is `new_test_audio`. This is the editor add path, not a hand hex-edit.

Vanilla TONE 14 (12 bytes):

```
00000000 ffffffff 00000000
```

Saved TONE 14 starts as a clone of a live donor (TONE 13 `SE_CHR_GUNW_01ZERO_08` family) with the name spliced:

- presence `flags0=0x8467FFFF`, `flags1` bit 19 **set** (the crashing bit)
- `flags1` vs donor: `0x000C981F` vs `0x000C989F` (bit 7 cleared)
- first dword after the aligned name: donor `8`, saved **`0`**
- record length 328 vs donor 336 (−8, the name-align delta)

Name `new_test_audio` is 15 bytes. Donor name `SE_CHR_GUNW_01ZERO_08` is 21 bytes. Align-4 name field shrinks by 8. The writer keeps the donor tail and the donor mask, including bit 19, then patches PACK offset/size. The engine still walks the donor mask against a cursor that is no longer on the donor layout.

## Root causes in this repo

### 1. `parse_tone_meta_block` does not know the OB presence mask

`src/nus3bank/parser.rs` `try_parse` only tries:

- `hash + unk1 + name`
- `8-byte prefix + hash + unk1 + name`

OB live records are:

```
hash (4)
unk1 (4)
stream+0 (4)          // skipped by the decoder
continuation words    // signed dwords until first >= 0
payload: name, type, pack off/size, params, bit19 hashes, …
```

Scoring two wrong layouts can still “succeed” and yield a name plus `pack_offset_field_pos`. That is enough for the UI and for current tests. It is not enough for the EXE.

### 2. `add_track` clones the first live `raw_meta`

`src/nus3bank/structures.rs`:

```rust
let template = self.tone.tones.iter().find(|t| !t.removed).cloned();
cloned.apply_new_cue_identity(name, audio_data)?;
```

`apply_new_cue_identity` only calls `rewrite_name_in_raw_meta` and replaces PACK bytes. Every presence bit, type dword, unkvalues table, and bit-19 hash list is inherited from an unrelated cue.

### 3. `rewrite_name_in_raw_meta` assumes C# layout

It takes `name_len_pos = meta_prefix.len() + 8`. That matches the synthetic helper `sample_tone_raw_meta` in `tests.rs`. It does not skip continuation words. On a real OB blob the splice either hits the mask or inserts padding in front of `reserved8`, which is how the saved type dword becomes `0`.

The writer then dumps `raw_meta` with only PACK offset/size patched (`writer.rs` `build_tone_meta`). There is no “does `nus3_decode_tone_descriptor` finish inside this record?” check.

### 4. Tests never run the engine decoder

`original_se_bank_add_keeps_dton_and_slots` asserts:

- TONE slot count stays 86
- DTON row count stays 1
- original cue names/sizes unchanged
- `new_test_audio` exists
- PACK offsets 16-aligned

The crashing `0xDFA2BBCE` satisfies those asserts. The test is green on a bank that AVs in `FinalizeLoad`.

### 5. DTON template is rebuilt, not preserved

GRP already keeps `raw_payload`. DTON does not. `build_dton` re-encodes name + `raw_data`. Vanilla `Default` entry size `0xEC` becomes `0xE8` after save. That is a 4-byte shrink of the only DTON template the engine copies at `bank+0x38`. It is the likely sibling of the historical `0x11CE5F9` faults.

## What this editor must guarantee on save

For every TONE record with `meta_size >= 104` (live):

1. Probe-decode with the OB cursor rules finishes with `cursor <= record_end`.
2. Fill-decode does not need more bytes than the record.
3. Bit 19 count, if the bit is set, is `<= 4` and `cursor + 4 + count*8 <= record_end`.
4. After a name-only edit, every presence bit still matches the payload that follows the new name.

For DTON when `is_template()` is true:

5. Section bytes round-trip (prefer `raw_payload`, same pattern as GRP).

Stub records (`< 104` bytes / 12-byte empty) must stay stubs. Do not promote them by cloning a live mask unless the cloned stream is rewritten to a known-good live layout and then probe-decoded.

## Non-goals

- Do not “fix” this by changing strikertable, outgame, or MSC in `E:\TAURI_PROJECT`.
- Do not target a later EXE than OB.
- Do not treat C# `NUS3BANK.cs` as the OB decoder. The EXE at `0x1411CDB10` is the contract.
