use super::error::Nus3bankError;
use super::structures::Nus3bankFile;
use once_cell::sync::Lazy;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Mutex;

// Store NUS3BANK replacement data, scoped by normalized file path.
pub static REPLACEMENT_DATA: Lazy<Mutex<HashMap<String, HashMap<String, ReplaceOperation>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Debug)]
pub enum ReplaceOperation {
    Remove(String),               // hex_id
    Replace(String, Vec<u8>),     // hex_id, new_data
    Add(String, String, Vec<u8>), // name, reserved_hex_id, data
}

/// NUS3BANK replace utilities
pub struct Nus3bankReplacer;

impl Nus3bankReplacer {
    fn normalize_file_key(file_path: &str) -> String {
        #[cfg(windows)]
        {
            file_path.replace('\\', "/").to_ascii_lowercase()
        }
        #[cfg(not(windows))]
        {
            file_path.to_string()
        }
    }

    fn parse_hex_index(hex_id: &str) -> Option<usize> {
        hex_id
            .strip_prefix("0x")
            .or_else(|| hex_id.strip_prefix("0X"))
            .and_then(|s| usize::from_str_radix(s, 16).ok())
    }

    fn normalize_hex_id(hex_id: &str) -> String {
        Self::parse_hex_index(hex_id)
            .map(|i| format!("0x{:x}", i))
            .unwrap_or_else(|| hex_id.to_string())
    }

    fn hex_id_sort_key(hex_id: &str) -> (u32, String) {
        let parsed = Self::parse_hex_index(hex_id)
            .and_then(|i| u32::try_from(i).ok())
            .unwrap_or(u32::MAX);
        (parsed, hex_id.to_string())
    }

    fn op_priority(op: &ReplaceOperation) -> u8 {
        match op {
            ReplaceOperation::Remove(_) => 0,
            ReplaceOperation::Replace(_, _) => 1,
            ReplaceOperation::Add(_, _, _) => 2,
        }
    }

    fn cmp_ops(a: &ReplaceOperation, b: &ReplaceOperation) -> Ordering {
        let pa = Self::op_priority(a);
        let pb = Self::op_priority(b);
        if pa != pb {
            return pa.cmp(&pb);
        }

        match (a, b) {
            (ReplaceOperation::Remove(ha), ReplaceOperation::Remove(hb)) => {
                Self::hex_id_sort_key(ha).cmp(&Self::hex_id_sort_key(hb))
            }
            (ReplaceOperation::Replace(ha, _), ReplaceOperation::Replace(hb, _)) => {
                Self::hex_id_sort_key(ha).cmp(&Self::hex_id_sort_key(hb))
            }
            (ReplaceOperation::Add(na, ha, _), ReplaceOperation::Add(nb, hb, _)) => na
                .cmp(nb)
                .then_with(|| Self::hex_id_sort_key(ha).cmp(&Self::hex_id_sort_key(hb))),
            _ => Ordering::Equal,
        }
    }

    fn next_add_index(file: &Nus3bankFile, ops: &[ReplaceOperation]) -> usize {
        let tones = &file.tone.tones;
        let mut removed: Vec<bool> = tones.iter().map(|t| t.removed).collect();
        let mut sorted = ops.to_vec();
        sorted.sort_by(Self::cmp_ops);
        for op in sorted {
            match op {
                ReplaceOperation::Remove(hex) => {
                    if let Some(idx) = Self::parse_hex_index(&hex) {
                        while removed.len() <= idx {
                            removed.push(true);
                        }
                        removed[idx] = true;
                    }
                }
                ReplaceOperation::Replace(_, _) => {}
                ReplaceOperation::Add(_, hex, _) => {
                    if let Some(idx) = Self::parse_hex_index(&hex) {
                        while removed.len() <= idx {
                            removed.push(true);
                        }
                        removed[idx] = false;
                    }
                }
            }
        }
        if file
            .binf
            .as_ref()
            .is_some_and(|b| b.name.to_ascii_uppercase().starts_with("BGM"))
        {
            return removed.len();
        }
        removed.iter().position(|&r| r).unwrap_or(removed.len())
    }

    fn peek_next_add_hex_id_locked(
        file: &Nus3bankFile,
        pending: Option<&HashMap<String, ReplaceOperation>>,
    ) -> String {
        let ops: Vec<ReplaceOperation> = pending
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default();
        let idx = Self::next_add_index(file, &ops);
        format!("0x{:x}", idx)
    }

    /// Register a remove operation for a track
    pub fn register_remove(file_path: &str, hex_id: &str) -> Result<(), String> {
        let file_key = Self::normalize_file_key(file_path);
        let hex_id = Self::normalize_hex_id(hex_id);
        if let Ok(mut data) = REPLACEMENT_DATA.lock() {
            let per_file = data.entry(file_key).or_insert_with(HashMap::new);
            per_file.insert(
                hex_id.clone(),
                ReplaceOperation::Remove(hex_id.clone()),
            );
            Ok(())
        } else {
            Err("Failed to register remove operation".to_string())
        }
    }

    /// Register an add operation for a track.
    ///
    /// Reserves the same TONE slot `add_track` / `add_track_at` will use on save
    /// (first unused stub, then append), excluding slots already taken by pending
    /// adds and including slots freed by pending removes.
    pub fn register_add(
        file_path: &str,
        name: &str,
        audio_data: Vec<u8>,
    ) -> Result<String, String> {
        if audio_data.is_empty() {
            return Err("Audio data cannot be empty".to_string());
        }

        if name.is_empty() {
            return Err("Track name cannot be empty".to_string());
        }

        let parsed = Nus3bankFile::open(file_path)
            .map_err(|e| format!("Failed to open NUS3BANK to reserve add slot: {}", e))?;

        let file_key = Self::normalize_file_key(file_path);
        if let Ok(mut data) = REPLACEMENT_DATA.lock() {
            let per_file = data.entry(file_key).or_insert_with(HashMap::new);
            let reserved_hex = Self::peek_next_add_hex_id_locked(&parsed, Some(per_file));
            per_file.insert(
                reserved_hex.clone(),
                ReplaceOperation::Add(name.to_string(), reserved_hex.clone(), audio_data),
            );
            println!(
                "Registered add operation for track '{}' with reserved_id {}",
                name, reserved_hex
            );
            Ok(reserved_hex)
        } else {
            Err("Failed to register add operation".to_string())
        }
    }

    /// Replace track in memory only (does not modify the actual file on disk).
    ///
    /// If `hex_id` is a pending Add, update that Add's audio bytes instead of
    /// creating a Replace op that would fail on save (`Track not found`).
    pub fn replace_track_in_memory(
        file_path: &str,
        hex_id: &str,
        new_audio_data: Vec<u8>,
    ) -> Result<(), String> {
        let file_key = Self::normalize_file_key(file_path);
        let hex_id = Self::normalize_hex_id(hex_id);
        if let Ok(mut data) = REPLACEMENT_DATA.lock() {
            let per_file = data.entry(file_key).or_insert_with(HashMap::new);

            if let Some(op) = per_file.get_mut(&hex_id) {
                match op {
                    ReplaceOperation::Add(_, _, data) => {
                        *data = new_audio_data;
                        println!(
                            "Updated pending ADD audio for NUS3BANK track: {}",
                            hex_id
                        );
                        return Ok(());
                    }
                    ReplaceOperation::Replace(_, data) => {
                        *data = new_audio_data;
                        println!("Stored replacement data for NUS3BANK track: {}", hex_id);
                        return Ok(());
                    }
                    ReplaceOperation::Remove(_) => {
                        return Err(format!(
                            "Cannot replace track {}: it is pending removal",
                            hex_id
                        ));
                    }
                }
            }

            let add_key = per_file.iter().find_map(|(k, op)| match op {
                ReplaceOperation::Add(_, hid, _) if Self::normalize_hex_id(hid) == hex_id => {
                    Some(k.clone())
                }
                _ => None,
            });
            if let Some(key) = add_key {
                if let Some(ReplaceOperation::Add(_, _, data)) = per_file.get_mut(&key) {
                    *data = new_audio_data;
                    println!(
                        "Updated pending ADD audio for NUS3BANK track: {}",
                        hex_id
                    );
                    return Ok(());
                }
            }

            per_file.insert(
                hex_id.clone(),
                ReplaceOperation::Replace(hex_id.clone(), new_audio_data),
            );
            println!("Stored replacement data for NUS3BANK track: {}", hex_id);
            Ok(())
        } else {
            Err("Failed to acquire lock on replacement data".to_string())
        }
    }

    /// Check if there are any replacement data stored
    pub fn has_replacement_data() -> bool {
        if let Ok(data) = REPLACEMENT_DATA.lock() {
            data.values().any(|m| !m.is_empty())
        } else {
            false
        }
    }

    /// Get the number of replacement data stored
    pub fn get_replacement_count() -> usize {
        if let Ok(data) = REPLACEMENT_DATA.lock() {
            data.values().map(|m| m.len()).sum()
        } else {
            0
        }
    }

    /// Apply all operations to a file
    pub fn apply_to_file(file_path: &str, file: &mut Nus3bankFile) -> Result<(), Nus3bankError> {
        let file_key = Self::normalize_file_key(file_path);
        let mut ops: Vec<ReplaceOperation> = if let Ok(data) = REPLACEMENT_DATA.lock() {
            data.get(&file_key)
                .map(|m| m.values().cloned().collect::<Vec<_>>())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Deterministic application order:
        // - Remove first (lowest risk of offset conflicts)
        // - Replace next (does not change entry count)
        // - Add last (changes entry count and PACK layout)
        ops.sort_by(Self::cmp_ops);

        for operation in ops {
            match operation {
                ReplaceOperation::Remove(hex_id) => {
                    println!("Applying remove operation for track: {}", hex_id);
                    file.remove_track(&hex_id)?;
                }
                ReplaceOperation::Replace(hex_id, new_data) => {
                    println!("Applying replace operation for track: {}", hex_id);
                    // NUS3BANK PACK only. NUS3AUDIO save must not reach this function.
                    let new_data = super::bnsf::ensure_bnsf_is14(new_data)?;
                    file.replace_track_data(&hex_id, new_data)?;
                }
                ReplaceOperation::Add(name, reserved_hex_id, audio_data) => {
                    let idx = Self::parse_hex_index(&reserved_hex_id).ok_or_else(|| {
                        Nus3bankError::InvalidHexId {
                            hex_id: reserved_hex_id.clone(),
                        }
                    })?;
                    println!(
                        "Applying add operation for track: {} at {}",
                        name, reserved_hex_id
                    );
                    let audio_data = super::bnsf::ensure_bnsf_is14(audio_data)?;
                    let new_hex_id = file.add_track_at(idx, name.clone(), audio_data)?;
                    println!(
                        "Successfully added track '{}' with ID: {}",
                        name, new_hex_id
                    );
                }
            }
        }

        // Open+Save with no pending add/replace still had leftover RIFF in PACK.
        Self::encode_leftover_wav_payloads(file)?;
        Self::sync_tone_bnsf_clock(file);

        Ok(())
    }

    fn encode_leftover_wav_payloads(file: &mut Nus3bankFile) -> Result<(), Nus3bankError> {
        let hex_ids: Vec<String> = file.tracks.iter().map(|t| t.hex_id.clone()).collect();

        for hex_id in hex_ids {
            let Some(payload) = file
                .get_track_by_hex_id(&hex_id)
                .and_then(|t| t.audio_data.clone())
            else {
                continue;
            };
            if let Some(bnsf) = super::bnsf::encode_nonempty_wav_payload(payload)? {
                println!(
                    "Converting leftover WAV PACK payload to BNSF/IS14 for {}",
                    hex_id
                );
                file.replace_track_data(&hex_id, bnsf)?;
            }
        }
        Ok(())
    }

    /// Match TONE unk1 bit 0x80 and the sample-clock ints to the BNSF payload.
    /// Add clones the first live cue (often a looping shot); one-shot custom
    /// audio then keeps the donor's 172k sample loop window and the game plays silence.
    fn sync_tone_bnsf_clock(file: &mut Nus3bankFile) {
        const LOOP_BIT: i32 = 0x80;
        for tone in &mut file.tone.tones {
            if tone.removed {
                continue;
            }
            let Some(clock) = super::bnsf::parse_bnsf_clock(&tone.payload) else {
                continue;
            };
            let new_unk1 = if clock.loop_flag != 0 {
                tone.unk1 | LOOP_BIT
            } else {
                tone.unk1 & !LOOP_BIT
            };
            if new_unk1 != tone.unk1 {
                println!(
                    "TONE unk1 {} 0x{:x} -> 0x{:x} (bnsf_loop={})",
                    tone.name, tone.unk1 as u32, new_unk1 as u32, clock.loop_flag != 0
                );
                tone.set_unk1(new_unk1);
            }
            if tone.patch_sample_clock(
                clock.n_samples,
                clock.loop_start,
                clock.loop_end,
                clock.loop_flag,
            ) {
                println!(
                    "TONE clock {} nsamp={} loop={}..{} flag={}",
                    tone.name, clock.n_samples, clock.loop_start, clock.loop_end, clock.loop_flag
                );
            }
        }
    }

    /// True when an on-disk NUS3BANK still has WAV PACK bytes or cloned loop clocks.
    /// Used so File→Save is enabled even with no pending add/replace.
    pub fn bank_needs_save_repair(file: &Nus3bankFile) -> bool {
        for tone in &file.tone.tones {
            if tone.removed {
                continue;
            }
            if super::bnsf::encode_nonempty_wav_payload(tone.payload.clone())
                .ok()
                .flatten()
                .is_some()
            {
                return true;
            }
            let Some(clock) = super::bnsf::parse_bnsf_clock(&tone.payload) else {
                continue;
            };
            let has_loop_bit = (tone.unk1 & 0x80) != 0;
            if has_loop_bit != (clock.loop_flag != 0) {
                return true;
            }
            if let Some((nsamp, lstart, lend, lflag)) = tone.sample_clock_from_raw_meta() {
                if nsamp != clock.n_samples
                    || lstart != clock.loop_start
                    || lend != clock.loop_end
                    || lflag != clock.loop_flag
                {
                    return true;
                }
            }
            if !tone.clock_layout_matches_loop_flag(clock.loop_flag) {
                return true;
            }
        }
        false
    }

    /// Clear replacement data for a specific file.
    pub fn clear_for_file(file_path: &str) {
        let file_key = Self::normalize_file_key(file_path);
        if let Ok(mut data) = REPLACEMENT_DATA.lock() {
            data.remove(&file_key);
        }
    }

    /// Clear all NUS3BANK replacement data from memory
    pub fn clear_replacements() {
        if let Ok(mut data) = REPLACEMENT_DATA.lock() {
            data.clear();
        }
        println!("Cleared all NUS3BANK audio replacements from memory");
    }

    /// Apply all in-memory replacements to a NUS3BANK file and save it
    pub fn apply_replacements_and_save(
        original_path: &str,
        output_path: &str,
    ) -> Result<(), String> {
        let mut nus3bank_file = Nus3bankFile::open(original_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;

        Self::apply_to_file(original_path, &mut nus3bank_file)
            .map_err(|e| format!("Failed to apply NUS3BANK operations: {}", e))?;

        nus3bank_file
            .save(output_path)
            .map_err(|e| format!("Failed to save NUS3BANK file: {}", e))?;

        Self::clear_for_file(original_path);

        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn operations_for_file(file_path: &str) -> Vec<ReplaceOperation> {
        let file_key = Self::normalize_file_key(file_path);
        if let Ok(data) = REPLACEMENT_DATA.lock() {
            data.get(&file_key)
                .map(|m| m.values().cloned().collect())
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    }
}
