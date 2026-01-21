# gvs_nus3bank_bug.md

## Summary

When exporting/rebuilding `.nus3bank`, some existing (old) sound entries may appear as `RIFF/WAVE` but still fail to play in-game.
This is **not** necessarily a PACK/TONE offset issue. The root cause can be that those "WAV" payloads are **not standard PCM WAV**.

In our case, many legacy sounds are stored as `WAVEFORMATEXTENSIBLE (0xFFFE)` with a **non-standard SubFormat GUID**, while the game expects
simple `WAVEFORMATEX PCM (formatTag = 1)` (typically `fmt` chunk size = 16, PCM16).

The track you replaced (MP3 -> WAV) played correctly because the converter produced a standard PCM WAV.

## Symptoms

- After converting/rebuilding, only the replaced track plays in-game.
- Other tracks show `audio_format: "Wav"` in debug JSON but do not play in-game.
- The editor can still export/play some of these WAVs locally, but the game refuses to decode them.

## How to Detect

Inspect the embedded audio payload in `PACK`:

- If it starts with `RIFF....WAVE`, it is a WAV container.
- Parse the `fmt ` chunk:
  - **Playable (expected)**: `wFormatTag = 1` (PCM), `fmt_len = 16`, `bits = 16` (commonly).
  - **Often unplayable in-game**: `wFormatTag = 65534 (0xFFFE)` (WAVEFORMATEXTENSIBLE), `fmt_len = 52`,
    and the SubFormat GUID is not the standard PCM/IEEE-float GUID.

Example (observed):

- Replaced track:
  - `wFormatTag = 1`, `fmt_len = 16`, `channels = 2`, `sample_rate = 44100`, `bits = 16`
- Legacy tracks:
  - `wFormatTag = 65534`, `fmt_len = 52`, `channels = 1`, `sample_rate = 48000`, `bits = 0`
  - SubFormat GUID (non-standard): `47e142d2-36ba-4d8d-88fc-61654f8c836c`

## Why the Game Fails

Many games implement a strict WAV decoder that only accepts a small subset of WAV:

- PCM (formatTag = 1)
- A fixed `fmt` structure size (usually 16)
- Supported bit depth (commonly 16-bit)

If the payload uses WAVEFORMATEXTENSIBLE or a custom SubFormat, the game may reject it even though the file is technically a WAV container.

## Workarounds

### Recommended workflow

- Convert legacy sounds to **PCM16 WAV** before inserting into `.nus3bank`.
- Prefer a common sample rate used by the title (often 48000 Hz) to reduce edge cases.

### In-editor debug tool

We added a debug action that converts all audio entries of the currently opened `.nus3bank` into standard PCM16 WAV **in memory**
(already-PCM16 entries are skipped). You can then save/export to write the normalized audio back into the bank.

