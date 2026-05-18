# NUS3BANK OB EXE Runtime Analysis

> Target IDB: `E:\OBHK0.3_v27\vsac27_Release.exe.i64`  
> Binary: `vsac27_Release.exe`  
> MD5: `07b22ba77c630c0d56f8be5d493a5a5a`  
> SHA-256: `cae3636aa4870d356eb25837badeb83482a71e290961442decf13cf87e1baa76`  
> Analysis date: 2026-05-18  
> Scope: OB EXE NUS3BANK runtime analysis.

## Short Conclusion

The OB EXE is suitable for continuing the NUS3BANK investigation. It contains the runtime concepts needed for GRP, DTON, PROP, tone resolution, group binding, and playback command analysis:

- BANKTOC section discovery for `PROP`, `BINF`, `GRP `, `DTON`, `TONE`, `PACK`, and `MARK`.
- PROP presence-mask parsing and version validation.
- A 352-byte runtime tone object initialized from a `DefaultTone` template.
- A continuation-bit tone descriptor decoder that applies serialized fields into the runtime tone object.
- Runtime group lookup/binding for pause, mute, volume-option, voice, hit, muzzle, and player groups.
- Game-side request paths that resolve tones and queue playback.

## Confirmed OB EXE Addresses

| OB EXE Address | IDA Name Applied | Role |
|---|---|---|
| `0x1411CEC60` | `nus3_parse_banktoc_bank` | Main `NUS3` + `BANKTOC` parser for base banks. |
| `0x1411D1DA0` | `nus3_parse_extended_cnf_bank` | Extended `CNF ` parser for routing/control sections such as `BUS2`, `GRP `, `DUCK`, `PROP`. |
| `0x1411CDB10` | `nus3_decode_tone_descriptor` | Decodes continuation presence-mask words and fills the 352-byte runtime tone object. |
| `0x1411B7490` | `nus3_init_default_tone_object` | Copies the 352-byte `DefaultTone` template from `0x14181F740`. |
| `0x1411B7E50` | `nus3_load_bank_and_bind_globals` | Creates/finalizes the bank object and binds global bus/group handles. |
| `0x1401B78E0` | `game_bind_sound_volume_groups` | Game-side binding for option/stem/voice/hit/muzzle/player volume groups. |
| `0x1401B8E20` | `game_set_volume_offset_group` | Stores a volume offset and binds `Volume_Offset_Group`. |
| `0x1401B8E50` | `nus3_bind_group_handle_by_name` | Creates an `HGroup` wrapper and resolves a group by name. |
| `0x1411B33D0` | `nus3_handle_set_group_by_name` | Handle wrapper for group-name lookup/binding. |
| `0x1411B1B70` | `nus3_handle_attach_refcounted_object` | Shared ref-counted handle attach helper. |
| `0x1401B4870` | `CSndBankData_RequestToneByEntryId` | Resolves a tone by entry id and can queue playback work. |
| `0x1411B1E20` | `nus3_hbank_get_tone_by_index` | `HBank` wrapper that resolves an `HTone` by direct tone index. |
| `0x1411B1F00` | `nus3_hbank_get_tone_by_entry_id` | `HBank` wrapper that resolves an `HTone` by bank-local entry id. |
| `0x1411B20F0` | `nus3_hbank_get_tone_by_entry_with_variant` | Resolves `HTone` using entry id plus variant/link parameters. |
| `0x1411B21F0` | `nus3_hbank_get_tone_by_entry_with_variant_alt` | Alternate variant-aware tone resolver. |
| `0x1411B7830` | `nus3_find_bank_and_get_tone_variant_alt` | Looks up a loaded bank by runtime id, then resolves a tone variant. |
| `0x1411B7A40` | `nus3_find_bank_and_get_tone_variant` | Looks up a loaded bank by runtime id, then resolves a tone variant. |
| `0x1411B6A70` | `nus3_enqueue_tone_command_u32` | Queues a 32-byte tone command packet with a caller-provided fourcc and u32 argument. |
| `0x1411B6670` | `nus3_enqueue_tone_volume_command` | Queues a `volu` tone-volume command packet. |
| `0x1411D7C50` | `nus3_ring_buffer_push_command` | Pushes command records into the NUS3 circular command buffer. |
| `0x1411B89A0` | `nus3_runtime_initialize` | Initializes NUS3 globals, mutexes, object lists, and the command ring buffer. |
| `0x1411B4040` | `nus3_update_and_drain_command_queue` | Runtime update tick; drains queued commands and updates active objects. |
| `0x1411B8640` | `nus3_runtime_shutdown` | Stops the NUS3 runtime and releases global objects/lists/mutexes. |
| `0x1411B50C0` | `nus3_dispatch_global_command` | Dispatches global command packets that do not carry an object pointer. |
| `0x1411C33A0` | `nus3_dispatch_runtime_object_command` | Object-level command dispatcher for `kyon`, `stop`, `volu`, pause/mute/pan/pitch/send/group controls. |
| `0x1411CB570` | `nus3_dispatch_basic_tone_command` | Basic tone/object command dispatcher. |
| `0x1411CB830` | `nus3_dispatch_extended_tone_command` | Extended tone/object command dispatcher. |
| `0x1411B2890` | `nus3_htone_enqueue_keyon_command` | Validates an `HTone` handle and queues `kyon`. |
| `0x1401B9570` | `game_enqueue_keyon_for_active_tones` | Iterates active game tone slots and queues `kyon` for valid handles. |
| `0x1411C8480` | `nus3_runtime_object_keyon_common` | Common object key-on handler reached from `kyon`. |
| `0x1411BE7C0` | `nus3_runtime_object_keyon_with_state_reset` | Key-on wrapper that resets extra runtime state before common key-on handling. |
| `0x1411C0C10` | `nus3_runtime_object_keyon_alt` | Alternate object key-on handler with different object offsets. |

IDA comments were added for the confirmed functions, and the IDB was saved after renaming.

## Section Loading Model

`nus3_parse_banktoc_bank` validates:

- `NUS3` at the file header.
- `BANK` / `TOC ` in the BANKTOC area.
- Section entries are walked in TOC order, and payload pointers are stored into the bank object.

Confirmed storage offsets in the OB EXE:

| Section | Magic | Runtime Pointer Offset |
|---|---:|---:|
| `PROP` | `0x504F5250` | `bank + 0x60` |
| `BINF` | `0x464E4942` | `bank + 0x68` |
| `GRP ` | `0x20505247` | `bank + 0x70` |
| `DTON` | `0x4E4F5444` | `bank + 0x78` |
| `TONE` | `0x454E4F54` | `bank + 0x80` |
| `PACK` | `0x4B434150` | `bank + 0x88` |
| `MARK` | `0x4B52414D` | `bank + 0x90` |

After discovery, the parser requires `PACK`. It then parses `PROP`, `BINF`, allocates arrays for DTON/TONE objects, and builds runtime descriptors.

## PROP Findings

The OB EXE parses PROP with a presence bitmask:

- `PROP + 0x0C` is a `u32` presence bitmask.
- Field data starts at `PROP + 0x10`.
- Bit 0 is the format version.
- The engine rejects the bank unless `(version & 0xFFFF0000) == 0x00030000`.
- Failure returns `0x80000018`.
- Bit 4 is a length-prefixed project string.
- Bits 5 and 6 are two `u32` fields stored in the bank object.
- Bit 7 is a length-prefixed timestamp string.

Rust impact:

- The current fixed-layout `PropSection` is not the engine's actual model.
- Rebuilding PROP should preserve or explicitly reconstruct the presence bitmask and field order.
- Version must remain `0x0003xxxx`; invalid major versions should be rejected with an explicit error.

## DTON and Tone Descriptor Findings

The key OB EXE function is `nus3_decode_tone_descriptor` at `0x1411CDB10`.

Important behavior:

- It scans descriptor words until it finds a non-negative word.
- The high bit is the continuation flag.
- The serialized fields are then read sequentially according to the set bits.
- The destination object has a base runtime size of `0x160` bytes.
- Variable-size data is appended after the fixed runtime object.
- `nus3_init_default_tone_object` initializes this object from the template at `0x14181F740`.

Confirmed default values in the OB EXE template:

| Runtime Offset | Value | Meaning |
|---:|---:|---|
| `+0x00` | pointer to `DefaultTone` | Name pointer. |
| `+0x10` | `-1` | Audio type / reference type default. |
| `+0x2C` | `1.0f` | Volume-like field. |
| `+0x30` | `1.0f` | Secondary volume/dry-level-like field. |
| `+0x34` | `-90.0f` | Mute threshold / dB floor-like field. |
| `+0x3C` | `1.0f` | Pitch-rate-like field. |
| `+0x44` | `20` | Priority-like field. |
| `+0x4C` | `1000` | Voice-limit-like field. |
| `+0x78` | `-1` plus bias on parsed values | Primary group index field. |
| `+0x90` | `1.0f` | Send-level-like field. |

The OB EXE uses a group-index bias:

- One parsed field writes `runtime_tone + 0x78 = serialized_value + 0x10000000`.
- Additional group-like arrays can also add `0x10000000` to group ids.

Rust impact:

- The current Rust DTON parser treats the entry tail as a flat `Vec<f32>`. That is useful for preservation/debugging, but it is not semantically accurate.
- Values such as `2.8025969e-44`, `4.2038954e-45`, and `null` in debug JSON are likely integer fields or invalid float interpretations.
- A correct editor should parse DTON as:
  1. Entry prefix fields already used by current files (`hash`, `unk1`, name), where applicable.
  2. Descriptor/presence words.
  3. Typed fields controlled by the descriptor bits.
  4. Raw bytes preserved for fields that are still not named.

## GRP Runtime Model

The OB EXE converts GRP entries into runtime group handles and linked-list nodes. Relevant runtime patterns:

- Group node name pointer is read from `node + 0x08`.
- Runtime group id is read from `node + 0x10`.
- Next pointer is read from `node + 0x18`.
- Refcount is manipulated at `node + 0x38`.

`nus3_load_bank_and_bind_globals` binds global groups by name:

| Group Name | Storage / Purpose |
|---|---|
| `Pause` | Pause group compatibility name. |
| `System_TitlePause` | Title pause group. |
| `nus3_TitlePause` | Title pause group. |
| `nus3_GamePause` | Game pause group. |
| `HUD_Mute` | HUD mute group. |
| `System_BgmMute` | BGM mute group. |
| `nus3_BgmMute` | BGM mute group. |

The OB EXE also has game-side volume group binding in `game_bind_sound_volume_groups`:

| Group Pattern | Purpose |
|---|---|
| `VOL_OPTION_BGM` | User BGM volume. |
| `VOL_OPTION_SE` | User SE volume. |
| `VOL_OPTION_VOICE_INGAME` | User in-game voice volume. |
| `VOL_OPTION_VOICE_OUTGAME` | User out-game voice volume. |
| `VOL_BGM_STEM1` ... `VOL_BGM_STEM5` | BGM stem volume. |
| `VOL_BGM_SUB`, `VOL_SE_SUB` | Submix groups. |
| `VOL_VOICE_INGAME_SUB`, `VOL_VOICE_OUTGAME_SUB` | Voice submix groups. |
| `VOL_BATTLE_SE_SUB` | Battle SE submix. |
| `VOL_MOVE_LAND_SE_SUB` | Movement/landing SE submix. |
| `HIT_SE_%02d` | Hit sound group family. |
| `HIT_MARK_SE_%02d` | Hit marker sound group family. |
| `MUZZLE_SE_%02d` | Muzzle sound group family. |
| `PLAYER_%02d_%02d` | Player-slot group family. |
| `Volume_Offset_Group` | Global volume offset group. |

Rust impact:

- GRP order matters because DTON/runtime tones reference groups by index/id.
- Empty GRP names should be preserved; they may intentionally occupy stable indices.
- Adding or removing entries must not renumber existing group indices unless all DTON/TONE references are updated.

## Read, Load, Play Flow

Observed flow in the OB EXE:

1. The NUS3BANK bytes are parsed by `nus3_parse_banktoc_bank`.
2. Section pointers are stored in the bank object.
3. PROP validates the bank version.
4. DTON/TONE counts are read and runtime pointer arrays are allocated.
5. `nus3_init_default_tone_object` initializes a default tone object.
6. `nus3_decode_tone_descriptor` applies per-entry serialized fields into that runtime tone object.
7. PACK offsets are converted into runtime audio-data pointers and sizes.
8. GRP and BUS names are resolved into ref-counted `HGroup` / `HBus` handles.
9. Game code binds option groups and stores handles for later volume/pause/mute updates.
10. `CSndBankData_RequestToneByEntryId` resolves a bank-local entry id to an `HTone` and can queue playback.

For playback, the important reverse-engineering target is not only the raw `PACK` bytes. The actual runtime tone includes:

- Audio payload pointer and size.
- Group assignments.
- Bus sends.
- Volume, pitch, priority, voice limit, and mute/pause behavior.
- Optional game-side bindings such as option-volume groups.

## Tone Resolution and Playback Request Flow

`CSndBankData_RequestToneByEntryId` is currently the clearest game-side bridge from bank data to playback. Its high-level behavior:

1. Check whether the sound bank data uses an internal hash/list path.
2. Resolve the requested bank-local entry id.
3. If only a handle is requested, return an `HTone` through one of the `HBank` tone resolver wrappers.
4. If `play_arg` is non-zero, resolve an `HTone` and enqueue a tone command.

Two resolution families are visible:

| Resolver | Vtable Slot | Meaning |
|---|---:|---|
| `nus3_hbank_get_tone_by_index` | `bank_vtbl + 0x120` | Direct tone-index lookup. |
| `nus3_hbank_get_tone_by_entry_id` | `bank_vtbl + 0x140` | Entry-id lookup. |
| `nus3_hbank_get_tone_by_entry_with_variant` | `bank_vtbl + 0x128` | Entry-id lookup with extra variant/link parameters. |
| `nus3_hbank_get_tone_by_entry_with_variant_alt` | `bank_vtbl + 0x148` | Alternate variant-aware lookup. |
| `nus3_find_bank_and_get_tone_variant` | `loaded_bank_vtbl + 0x130` | Global loaded-bank lookup, then variant resolution. |
| `nus3_find_bank_and_get_tone_variant_alt` | `loaded_bank_vtbl + 0x150` | Alternate global loaded-bank lookup, then variant resolution. |

The playback request path does not directly decode audio at the call site. It resolves an `HTone`, then queues a command:

- `nus3_enqueue_tone_command_u32` builds a 32-byte command packet with a caller-provided fourcc.
- `CSndBankData_RequestToneByEntryId` uses fourcc `0x6E6F796B`, which is `kyon` in little-endian byte order.
- `nus3_htone_enqueue_keyon_command` and `game_enqueue_keyon_for_active_tones` also enqueue `kyon`.
- `nus3_enqueue_tone_volume_command` uses fourcc `0x756C6F76`, which is `volu` in little-endian byte order.
- `nus3_ring_buffer_push_command` writes command records with record magic `0x70716E62` (`bnqp`) and wrap marker `0x64716E62` (`bnqd`).
- The command queue increments a counter at `tone + 0x3C`, which looks like a queued-command or in-flight reference counter.

### Command Queue Lifecycle

`nus3_runtime_initialize` allocates and initializes the command ring buffer:

- Global command queue pointer: `qword_14241EA08`.
- Allocation size: `131112` bytes.
- Ring capacity: `0x20000` bytes.
- Queue tag at offset `+0x00`: `0x75716E62` (`bnqu`).
- Buffer pointer at offset `+0x08`: allocation base + `0x28`.
- Capacity at offset `+0x10`: `0x20000`.
- Write offset at offset `+0x14`, read offset at offset `+0x18`, wrap flag at offset `+0x1C`.

`nus3_update_and_drain_command_queue` is the confirmed queue consumer:

1. It locks `qword_142486968`.
2. It loops while the queued-command counter is non-zero and read offset differs from write offset.
3. If the current record magic is `bnqd`, it wraps read offset back to zero.
4. It expects each normal record to start with `bnqp`.
5. The command payload starts at `record + 0x08`.
6. If `payload + 0x08` contains an object pointer, it calls that object's vtable `+0x20` command dispatcher, then decrements the object's `+0x3C` queued-command counter.
7. If the object pointer is null, it dispatches through `nus3_dispatch_global_command`.
8. It advances the read offset by `align4(payload_size) + 0x08`.

The object-level dispatcher confirms the playback/control semantics:

| FourCC | Meaning | Dispatcher Behavior |
|---|---|---|
| `kyon` / `0x6E6F796B` | Key-on / play request. | Calls object vtable `+0x38` in `nus3_dispatch_runtime_object_command`; basic dispatcher uses vtable `+0x38` with one argument. |
| `stop` / `0x706F7473` | Stop request. | Calls object vtable `+0x48` in the runtime-object dispatcher. |
| `volu` / `0x756C6F76` | Volume command. | Calls object vtable `+0xB0` in the runtime-object dispatcher; basic dispatcher calls vtable `+0x60`. |
| `paus` / `0x73756170` | Pause control. | Calls object-specific pause handler. |
| `mute` / `0x6574756D` | Mute control. | Calls object-specific mute handler. |

Following the `kyon` vtable slot finds three key-on handlers:

| Handler | Runtime Behavior |
|---|---|
| `nus3_runtime_object_keyon_common` | Checks active/blocked flags, clears a runtime counter, optionally converts the command argument into `a2 * 1000 / update_rate`, stores reciprocal timing, then sets play-trigger flags. |
| `nus3_runtime_object_keyon_with_state_reset` | Clears extra runtime state, sets a two-byte state flag, then enters the common key-on handler. |
| `nus3_runtime_object_keyon_alt` | Performs the same key-on/timing pattern with alternate object offsets. |

This means the confirmed `kyon` handler is a runtime-state trigger. The actual voice/audio creation should be in the later object update path that observes these flags, so the next playback target is the per-object update vtable slot rather than the queue writer itself.

`nus3_runtime_shutdown` signals runtime shutdown, drains/stops active state, waits for update counters, clears the runtime-active flag, and releases global objects/lists/mutexes.

Rust impact:

- A valid edit must preserve the ability to resolve the same bank-local entry id to an `HTone`.
- The actual start/play action is asynchronous through a `kyon` command in the NUS3 command ring buffer.
- Runtime object pointers in queued packets mean playback behavior depends on the resolved tone object's vtable and runtime fields, not just on the serialized bank bytes.
- If TONE/DTON entries are reordered or removed, game-side entry-id lookups can resolve the wrong `HTone`.
- Replacement is safer than add/remove because it keeps entry ids and group routing stable.

## Rust Modification Guidance

Recommended effective reverse-modification strategy:

1. Preserve all unknown bytes and all section order exactly when possible.
2. Treat `PACK` replacement as safe only when the target `TONE`/DTON descriptor still points to the same entry and size is rebuilt consistently.
3. Do not rebuild `GRP` by template unless the target file truly lacks a GRP section. Prefer editing names in place while preserving count and indices.
4. Do not remove DTON entries when removing audio. Mark/remove the matching TONE/PACK payload consistently, but keep descriptor arrays stable until group/tone references are fully understood.
5. Add a typed DTON model gradually:
   - `volume`
   - `volume_2`
   - `mute_threshold_db`
   - `pan`
   - `pitch_rate`
   - `priority`
   - `voice_limit`
   - `primary_group_index`
   - `send_level`
6. Keep raw descriptor bytes beside typed fields so the writer can round-trip unsupported bits without guessing.
7. Validate `PROP` version and bitmask layout instead of silently accepting impossible fixed-layout data.
8. When adding audio, clone a compatible existing TONE/DTON entry from the same bank category rather than synthesizing one from global defaults.

## Open Follow-Up Targets

These are the next useful OB EXE analysis targets:

| Target | Reason |
|---|---|
| `CSndBankData_RequestToneByEntryId` callers | Identify which game systems call playback and what ids they pass. |
| `BgmController_PlayCueHash` | Understand BGM-specific cue/hash to tone resolution. |
| `game_bind_sound_volume_groups` internals | Map every stored handle offset to UI/game option semantics. |
| `nus3_decode_tone_descriptor` bit table | Produce a complete bit-to-runtime-offset map for Rust typed parsing. |
| `nus3_parse_extended_cnf_bank` users | Decide whether CNF-style extended sections affect the target files. |
| Runtime object update after `kyon` | Map how key-on trigger flags create the actual voice and bind PACK audio data. |

## Current Working Judgment

Use the OB EXE for the next GRP/DTON/PROP deep dive. It has enough information for:

- How the game reads NUS3BANK.
- How it loads runtime tone/group/bus objects.
- How group names affect pause, mute, volume options, and routing.
- How playback requests resolve tones.
- How resolved tones enter the NUS3 command queue for playback/control.
- How the Rust editor should avoid invalid rewrites and move toward typed DTON editing.
