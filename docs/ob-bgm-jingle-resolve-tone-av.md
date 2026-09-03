# OB BGM jingle AV after adding `COLORS_Flow`

**Status:** E2 file + log + IDA (2026-09-02). Fault is HUD `BGM_Name` text, not `ResolveTone`.
**Date:** 2026-09-02
**Crashed binary:** `E:\OBHK0.3_v27\vsac27_Release.exe` (OB, in-scope)
**Log:** `E:\OBHK0.3_v27\EXVS2-debug-20260902-210725.log` (CrashReport stamp `210806`)
**Bank overlay:** `E:\OBHK0.3_v27\data\x64\mod\0x0C568109.fhm2d`
**Workspace bank:** `E:\XB\mod\090sound\bgm_ac27_update_02\BGM_AC27_UPDATE_02.nus3bank`

This is **not** the Audio Editor Save crash in
[`ob-tone-decode-av-from-save.md`](ob-tone-decode-av-from-save.md)
(`rva=0x11CE4ED` during `CSndBankData_FinalizeLoad`).
The process now reaches training / battle start. It dies when the unit BGM jingle
resolves a tone.

## Crash (game)

```
[GAME-LOG]0c568109          // pack opened ~210733
...
[GAME-LOG]cf58cccf
[GAME-LOG]add_pilot_mc
[GAME-LOG]add_pilot_mc
access violation: read at 0xF6DEFE0000, faulting rva=0x1F29AE7FD | callers: 0x5A89B <- 0x1CA4A1
access violation: read at 0x0,          faulting rva=0x1CA4A1   | callers: 0x3183A3 <- 0xA06D6E <- ...
```

| Item | Value |
|---|---|
| Nested first-chance RVA `…E7FD` | Ignore (same family as the SE load note) |
| Real fault | `rva=0x1CA4A1`, read at `0` |
| VA | `0x140000000 + 0x1CA4A1 = 0x141CA4A1` |
| Near caller `0xA06D6E` | Battle-start jingle path (`JingleProcWait` hook is `0x140A06E80`) |
| Same stack as | 2026-09-01 log when **audio had `COLORS_Flow` and the bank did not** |

Game-side lookup (POC, not re-decompiled here):

```
character_list cueHash
  -> bgm_table[record_id]
  -> group 6 + uppercase CRC32(cue label)
  -> CSndBankData_FindToneEntryIdByCommand(bank, crc)   // 0x1401B5560
  -> nus3::HBank.ResolveTone(toneEntryId)               // null -> AV
```

`add_pilot_mc` is a normal boot string (`0x141205028`), not the fault.

## What this run already proved

Pack `0x0C568109` **did load**. `[GAME-LOG]0c568109` is in the same session as the AV.
Startup `FinalizeLoad` did **not** die at `0x11CE4ED`, so the appended TONE record
walks `nus3_decode_tone_descriptor`.

Extract of the overlay after the 2026-09-02 pack:

| TONE | Size | Name field (len byte `0x19`) |
|---|---:|---|
| 0 | 12 | stub `00 00 00 00 01 00 00 00 ff 00 00 00` |
| 1 | 336 | `vstg_ac_title_in_2000_v3` + NUL (fills 25) |
| 2 | 336 | `vstg_ac_title_in_2000_v2` + NUL (fills 25) |
| 3 | 336 | `COLORS_Flow` + NUL + **13 extra NULs** to keep donor slot |

TONE 3: `flags0=0x8427FFFF`, `flags1=0x000C989F`, probe walk leftover `0`,
PACK `offset=0 size=8936` (same as donor v2/v3). DTON / GRP / BINF / PACK bytes
are vanilla. Only TONE grew (`3 -> 4` records, file `65944 -> 66288`).

`bgm_table` (`E:\XB\mod\090sound\bgm_table\0.bin`, 136 rows) group 6:

| cueHash (`record_id`) | cueLabelCrc | cue (uppercase) |
|---|---|---|
| `0xCCF5CF6D` | `0x298E86A2` | `VSTG_AC_TITLE_IN_2000_V3` |
| `0xBBF2FFFB` | `0x5E89B634` | `VSTG_AC_TITLE_IN_2000_V2` |
| `0x6D53718D` | `0x42858C78` | `COLORS_FLOW` |

`0x6D53718D = CRC32("BGM_CUEHASH|COLORS_FLOW")`.
`0x42858C78 = CRC32("COLORS_FLOW")`.
Vanilla labels in the bank are **lowercase**; the table stores uppercase CRC.
All three group-6 rows have `routeSelector=0`.

`.nus3audio` `BGM_AC27_UPDATE_02` (142 071 920 B):

| TNID | TNNM |
|---:|---|
| 0 | `11bfa604898243b28977ec383fc6a12c` (dead GUID, vanilla) |
| 1 | `vstg_ac_title_in_2000_v3` |
| 2 | `vstg_ac_title_in_2000_v2` |
| 3 | `COLORS_Flow` |

TNID `==` TONE index for the two vanilla live cues and for `COLORS_Flow`.

## IDA (2026-09-02, `vsac27_Release.exe`)

Fault RVA `0x1CA4A1` is **inside** `sub_1401CA440` (`0x1401CA440`), not a TONE
decoder. Instruction: `if ( *a2 )` — `a2` is a C-string title.

Call chain:

```
sub_140A06C80                    // battle-start jingle, string "/BGM_Name_mc"
  v6 = *(uint32*)(ctx + 64)      // cueHash
  v7 = BgmList_GetTitleWithNoteByCueHash(v6)   // 0x1409A9030
  sub_140318370(mc, "/BGM_Name_mc/BGM_Name_Text_mc/BGM_Name", v7)
    sub_1401CA440(textField, v7)
      if (*v7)  // AV when v7 == NULL
```

`BgmList_GetTitleWithNoteByCueHash`:

1. `LookupRecordIdByFieldValue(bgm_list, 0xA84A15F4, cueHash)`
2. Read kind-7 `titleWithNotePrefix` (`0x50223A04`) from that record
3. Deobfuscate into `byte_14211FFB0`
4. **On miss, `return 0`** — no empty-string fallback

Workspace `bgm_list.bin` (115 rows, 9127 B) had **zero** copies of
`0x6D53718D`. Overlay `data/x64/mod/0xC91627E8.fhm2d` was absent. That is
sufficient for this AV.

POC handbook “skip bgm_list → song plays, HUD blank” is **false** on this
jingle path. Missing HUD row is a hard crash.

### Name CRC (keep-slot padding is fine)

`HashUppercaseCueSourceString` (`0x1401C08E0`) used by
`CSndBankData_BuildSourceCrcToToneIdMap`:

```
len = 0;
if (*Src) { while (Src[len]) ++len; }   // C-string, stops at first NUL
copy; toupper each byte; CRC32
```

TONE 3 `COLORS_Flow\0` plus extra NULs in the `0x19` slot still hashes as
`CRC32("COLORS_FLOW") = 0x42858C78`. Do **not** shrink the length byte.

Do **not** shrink the length byte to `0x0C`. That is the editor-av path:
`flags1` bit 7 cleared, `skip_name` 16 instead of 28, load AV at `0x11CE4ED`.
Poison file:
`E:\XB\mod\090sound\bgm_ac27_update_02\BGM_AC27_UPDATE_02.nus3bank.editor-av`.

## Editor / packer implications

- Keep-slot (`flags0==0x8427FFFF`, donor `0x19`) stays mandatory for BGM load.
- A new BGM cue **must** have a `bgm_list` row whose `cueHash` (`0xA84A15F4`)
  equals `bgm_table.record_id`, with kind-7 `title` / `titleWithNotePrefix`.
- Pack that list as `0xC91627E8.fhm2d` into `data/x64/mod/`.
- BINF `unk1=3` on this bank matches vanilla TONE count coincidentally; SE banks
  show it is not a live-cue counter. Leave it.
- DTON (4 named templates) and GRP (mixer names) are not 1:1 with TONE count.
  Vanilla already has 4 DTON / 3 TONE. Do not clone DTON on BGM append.

## Fix applied (2026-09-02)

Inserted HUD row via `src-tauri/examples/add_bgm_list_colors_flow.rs`:

| Field | Value |
|---|---|
| `cueHash` `0xA84A15F4` | `0x6D53718D` |
| `title` | `COLORS Flow` |
| `titleWithNotePrefix` | `♪COLORS Flow` |
| `record_id` | `0xFDE810EA` (next unused, sorted) |
| `musicId` | 122 |
| `sourceGroupHash` | donor `0x169B49CD` |

Wrote `E:\XB\mod\012list\bgm_list\bgm_list.bin` (116 rows, 9186 B) and packed
`0xC91627E8.fhm2d` into `data/x64/mod/`.

## Files not to touch

- `*.nus3bank.editor-av` — shrunk name, load AV.
- `data/x64/mod/0x0C568109.fhm2d.editor-av` — packed poison.
- Vanilla overlay backup: `data/x64/mod/0x0C568109.fhm2d.vanilla` (65944 B bank inside).
