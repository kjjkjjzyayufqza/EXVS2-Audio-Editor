# Plan: make NUS3BANK save OB-decoder-safe

**Repo:** `E:\research\EXVS2-Audio-Editor`  
**Owner note:** `docs/ob-tone-decode-av-from-save.md`  
**Engine contract:** `docs/NUS3BANK_OB_EXE_Runtime_Analysis.md` (`nus3_decode_tone_descriptor` @ `0x1411CDB10`)  
**Implemented 2026-08-29** in `ob_tone_decode.rs`, `parser.rs`, `structures.rs`, `writer.rs`. Remaining E3: regenerate `0xDFA2BBCE` with the fixed editor and load Rebellion in-game.

## Goal

`File → Save` (and `add_track` / `replace_track_data` / `remove_track`) must emit TONE/DTON bytes that `nus3_decode_tone_descriptor` can walk during `CSndBankData_FinalizeLoad`. The current add-on-stub path writes a bank that AVs at `0x1411CE4ED` (`mov edx, [rbx]`, mask bit 19).

## Current broken path

```
add_track("new_test_audio")
  → first live ToneMeta.clone()
  → rewrite_name_in_raw_meta()   // name at prefix+8, C# layout
  → save()
      build_tone_meta() dumps raw_meta (PACK off/size patched only)
      build_dton() re-encodes the Default row (0xEC → 0xE8)
```

Vanilla TONE 14 is a 12-byte stub. Add reuses that slot, clones TONE 0/13-class presence bits (bit 19 set), splices a shorter name, and leaves the donor tail. OB probe-decode then runs off the record.

## Constraints

- BANKTOC-only. No CNF banks in this work.
- Keep stub slots. OB looks up TONE by index; never compact the pointer table.
- Character SE banks keep a single DTON `Default` template. Do not append a DTON row per add.
- Tests must fail on the crashing `0xDFA2BBCE` file before the writer is “fixed”.
- Debug-only cargo. No `--release` unless asked.

## Phase 0 — freeze the crashing fixture (no writer change)

1. Add a testdata pointer or a tiny extracted blob:
   - full bank (optional, 540 KB): the two SE paths under `E:\XB\mod\090sound\`
   - required: TONE 14 vanilla 12-byte stub vs saved 328-byte `new_test_audio` record as a `#[test]` byte array
2. Write `src/nus3bank/ob_tone_decode.rs`: a probe-only port of `nus3_decode_tone_descriptor`.
   - Input: one TONE record (`raw_meta`).
   - Output: `Ok(bytes_consumed)` or `Err` with bit index / cursor.
   - Must implement: continuation words, bit 0 name, bit 1 type switch, bit 19 hash list (`count` then `count` times 8-byte step, dest would be `+0x14C`).
   - Other bits: consume using the same sizes as the EXE (port from `0x1411CDB10`, not from C#). Unknown bits that the EXE consumes must be listed in a table in this file; do not skip them.
3. Tests (RED):
   - vanilla TONE 13 probe `Ok`
   - vanilla TONE 14 stub skipped (`< 104` / 12 bytes) — decoder not called
   - saved `new_test_audio` TONE 14 probe `Err` (this is the product bug)
   - `original_se_bank_add_keeps_dton_and_slots` must call the probe on every live record after save and **fail today**

Stop when the new tests fail on current `main`. Do not patch the writer in this phase.

## Phase 1 — parse OB presence mask

Change `parse_tone_meta_block` so the name is found the way the EXE finds it:

```
hash, unk1,
stream = record[8..],
mask_words = read signed dwords from stream[4..] until word >= 0,
name at payload cursor (len-prefixed, align 4)
```

Keep the C# `hash+unk1+name` try as a fallback for synthetic / GVS fixtures (`sample_tone_raw_meta`). Prefer the OB layout when continuation words are present (`word < 0` at stream+4, or two mask dwords).

Store on `ToneMeta`:

- `descriptor_words: Vec<u32>` (same idea as DTON)
- `name_len_pos: usize` — byte offset of the name length **inside `raw_meta`**, after the mask
- `pack_offset_field_pos` / `pack_size_field_pos` computed from the payload cursor, not from `prefix+8+name`

Tests:

- vanilla 86-slot SE: 18 live names still parse
- TONE 13 name is `SE_CHR_GUNW_01ZERO_08` (or the real first live name), not a 159-byte garbage string from `0x9f`
- noop save of vanilla `0x421D77F3` stays MD5-identical, or TONE/DTON section bytes identical if header timestamps exist (assert those two sections)

## Phase 2 — name rewrite on the real name offset

`rewrite_name_in_raw_meta`:

- splice at `name_len_pos` from Phase 1, never `prefix_len + 8` unless that *is* the name
- keep mask words byte-for-byte
- keep the tail after the old aligned name
- shift `pack_*_field_pos` by the align delta only

After splice, run the Phase 0 probe. If it fails, return `Err` — do not save a bank the EXE cannot load.

`add_track` / `apply_new_cue_identity`:

- still allowed to clone a live donor **only if** probe passes after the name rewrite
- if the donor mask includes bits whose payload depends on the donor name length or PACK size, those fields must be rewritten (`patch_sample_clock` already exists for the clock block; extend the same idea to type dword / bit 19 list rather than hoping a tail copy works)
- prefer cloning a **one-shot, non-bit19** live cue when several donors exist; never clone a looping cue onto a one-shot add without `patch_sample_clock`

If no probe-safe clone exists, refuse the add with a clear error instead of writing `new_test_audio` into a stub.

## Phase 3 — DTON template byte identity

When `DtonSection::is_template()`:

- keep `raw_payload: Option<Vec<u8>>` like GRP
- `build_dton` writes that blob
- add/remove/replace must not rebuild `Default` or change entry size `0xEC`

Tests:

- vanilla DTON section bytes equal before/after noop save
- vanilla DTON section bytes equal before/after add-to-stub
- `0xEC` stays `0xEC`

Non-template (GVS per-cue DTON) stays on the current encode path; do not mix the two.

## Phase 4 — save-time gate

`Nus3bankWriter::write_file` after `build_tone`:

- for each live record, probe-decode
- on failure, return `Nus3bankError::InvalidFormat` with tone index + bit
- do not write a partial file

UI: surface that error on Save / Add. Do not toast success.

## Phase 5 — regenerate the Rebellion bank

After Phases 1–4 are green:

1. Open vanilla `0x421D77F3`.
2. Add `new_test_audio` (or the intended cue) through the fixed API.
3. Confirm every live TONE probes `Ok` and DTON is byte-identical to vanilla except TONE/PACK growth.
4. Replace `E:\XB\mod\090sound\wing_gundam_zero_rebellion_sound\SE_CHR_016GUNDMW_001WGZERO_001.nus3bank`.
5. In-game: load Rebellion + striker. Pre-register H = “FinalizeLoad does not AV”; P = training start reaches `AiContextProbe`; F = another `0x11CE4ED` / `0x11CE5F9`.

Do not hand-hex TONE 14 in the game tree as the “fix”. The writer must be able to emit that file.

## Files to touch (expected)

| File | Phase |
|---|---|
| `src/nus3bank/ob_tone_decode.rs` | 0 (new) |
| `src/nus3bank/mod.rs` | 0 export |
| `src/nus3bank/tests.rs` | 0–4 |
| `src/nus3bank/parser.rs` | 1 |
| `src/nus3bank/structures.rs` | 1–2 (`name_len_pos`, rewrite, add_track) |
| `src/nus3bank/writer.rs` | 3–4 |
| `src/nus3bank/error.rs` | 4 if a dedicated error helps |
| `NUS3BANK_Add_Audio_Implementation_Guide.md` | 5, after the code exists |
| `docs/NUS3BANK_DTON_Section_Analysis.md` | 5, replace the stale “DTON 未实现” banner |

## Acceptance

- `cargo test --lib nus3bank` (debug) is green.
- `original_se_bank_add_*` runs the OB probe on every live TONE after save.
- Saved `new_test_audio` on the 86-slot SE bank no longer matches the crashing TONE 14 hex (type dword 0 + bit 19 + 328-byte clone).
- Vanilla noop save does not shrink DTON `0xEC` → `0xE8`.
- In-game load of the regenerated `0xDFA2BBCE` is E3 (user run). Static tests cannot claim E3.

## Out of scope until the gate is green

- UI redesign of the add-audio modal
- CNF / BUS2 / MARK writers
- Replacing BNSF encode
- Changing `E:\TAURI_PROJECT` strikertable or MSC
