use super::audio_file_info::AudioFileInfo;
use nus3audio::Nus3audioFile;
use crate::nus3bank::Nus3bankExporter;
use std::fs;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use once_cell::sync::Lazy;

// Cache for indexing patterns to avoid re-analyzing the same file multiple times
static INDEXING_PATTERN_CACHE: Lazy<Mutex<HashMap<String, bool>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

// Cache for converted WAV temp files: "file_path::name::id" -> temp_wav_path
// Avoids re-running vgmstream-cli for the same track within a session.
static WAV_CONVERSION_CACHE: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

/// Utility functions for exporting audio files
pub struct ExportUtils;

/// Loop range reported by vgmstream (`-m`), including smpl-backed WAVs.
#[derive(Clone, Copy, Debug)]
pub struct VgmstreamLoopInfo {
    pub loop_start_secs: f32,
    pub loop_end_secs: f32,
}

impl ExportUtils {
    /// Pre-populate the indexing pattern cache from already-loaded track IDs.
    /// Call this after `Nus3audioFile::open()` so the first `play` does not re-read the file.
    pub fn prime_indexing_cache(file_path: &str, track_ids: &[u32]) {
        if track_ids.is_empty() {
            return;
        }
        let starts_from_zero = track_ids.iter().min().copied().unwrap_or(1) == 0;
        if let Ok(mut cache) = INDEXING_PATTERN_CACHE.lock() {
            cache.insert(file_path.to_string(), starts_from_zero);
        }
        println!("[PERF] primed indexing cache for {} ({} tracks, starts_from_zero={})", file_path, track_ids.len(), starts_from_zero);
    }

    fn build_temp_audio_path(base_name: &str, extension: &str) -> PathBuf {
        let temp_dir = std::env::temp_dir();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let pid = std::process::id();
        let filename = format!("{}_{}_{}.{}", base_name, pid, timestamp, extension);
        temp_dir.join(filename)
    }

    fn detect_audio_extension(data: &[u8]) -> &'static str {
        if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WAVE" {
            return "wav";
        }
        if data.len() >= 4 && &data[0..4] == b"OggS" {
            return "ogg";
        }
        if data.len() >= 4 && &data[0..4] == b"fLaC" {
            return "flac";
        }
        if data.len() >= 3 && &data[0..3] == b"ID3" {
            return "mp3";
        }
        if data.len() >= 2 && data[0] == 0xFF && (data[1] & 0xE0) == 0xE0 {
            return "mp3";
        }
        "bin"
    }

    /// Write audio bytes into a temporary file and return the file path
    pub fn write_temp_audio_bytes(
        audio_file_info: &AudioFileInfo,
        audio_bytes: &[u8],
        tag: &str,
    ) -> Result<String, String> {
        let extension = Self::detect_audio_extension(audio_bytes);
        let base_name = format!("temp_audio_{}_{}", audio_file_info.id, tag);
        let temp_output_path = Self::build_temp_audio_path(&base_name, extension);
        let temp_output_path_str = temp_output_path.to_string_lossy().to_string();
        fs::write(&temp_output_path, audio_bytes)
            .map_err(|e| format!("Failed to write temporary audio file: {}", e))?;
        Ok(temp_output_path_str)
    }
    /// Determine the correct vgmstream index based on the nus3audio file's indexing pattern
    /// 
    /// This function analyzes the nus3audio file to detect whether it uses:
    /// - 0-based indexing (0,1,2,3...) -> needs +1 conversion for vgmstream
    /// - 1-based indexing (1,2,3,4...) -> direct mapping to vgmstream
    /// 
    /// Uses caching to avoid re-analyzing the same file multiple times.
    fn get_vgmstream_index(
        audio_file_id: &str,
        original_file_path: &str,
    ) -> Result<String, String> {
        // Parse the audio file ID
        let id_num = audio_file_id.parse::<u32>()
            .map_err(|_| format!("Invalid audio file ID: {}", audio_file_id))?;
        
        // Check cache first
        let cache_key = original_file_path.to_string();
        let starts_from_zero = if let Ok(cache) = INDEXING_PATTERN_CACHE.lock() {
            if let Some(&cached_pattern) = cache.get(&cache_key) {
                println!("Using cached indexing pattern for {}: starts_from_zero={}", original_file_path, cached_pattern);
                cached_pattern
            } else {
                // Cache miss, need to analyze the file
                drop(cache); // Release the lock before file operations
                
                // Load the nus3audio file to analyze the indexing pattern
                let nus3_file = Nus3audioFile::open(original_file_path)
                    .map_err(|e| format!("Failed to open nus3audio file: {}", e))?;
                
                if nus3_file.files.is_empty() {
                    return Err("No audio files found in nus3audio file".to_string());
                }
                
                // Collect all IDs and sort them
                let mut all_ids: Vec<u32> = nus3_file.files.iter().map(|f| f.id).collect();
                all_ids.sort();
                
                // Determine the indexing pattern
                let pattern = all_ids[0] == 0;
                
                println!("Analyzed indexing pattern for {}: IDs={:?}, starts_from_zero={}", 
                        original_file_path, all_ids, pattern);
                
                // Cache the result
                if let Ok(mut cache) = INDEXING_PATTERN_CACHE.lock() {
                    cache.insert(cache_key, pattern);
                }
                
                pattern
            }
        } else {
            // Fallback if cache lock fails - analyze without caching
            println!("Warning: Failed to access indexing pattern cache, analyzing without caching");
            
            let nus3_file = Nus3audioFile::open(original_file_path)
                .map_err(|e| format!("Failed to open nus3audio file: {}", e))?;
            
            if nus3_file.files.is_empty() {
                return Err("No audio files found in nus3audio file".to_string());
            }
            
            let mut all_ids: Vec<u32> = nus3_file.files.iter().map(|f| f.id).collect();
            all_ids.sort();
            all_ids[0] == 0
        };
        
        if starts_from_zero {
            // 0-based indexing: convert to 1-based for vgmstream
            // 0 -> 1, 1 -> 2, 2 -> 3, etc.
            let vgmstream_index = id_num + 1;
            println!("0-based indexing detected: {} -> {}", id_num, vgmstream_index);
            Ok(vgmstream_index.to_string())
        } else {
            // 1-based indexing: direct mapping
            // 1 -> 1, 2 -> 2, 3 -> 3, etc.
            println!("1-based indexing detected: {} -> {}", id_num, id_num);
            Ok(id_num.to_string())
        }
    }

    /// Invalidate WAV conversion cache for all tracks belonging to a specific file.
    /// Call this after saving the file to disk so stale conversions are not replayed.
    pub fn clear_wav_cache_for_file(file_path: &str) {
        if let Ok(mut cache) = WAV_CONVERSION_CACHE.lock() {
            cache.retain(|key, _| !key.starts_with(file_path));
            println!("[PERF] WAV cache cleared for: {}", file_path);
        }
    }

    /// Query loop points via vgmstream-cli metadata (`-m`).
    /// For NUS3AUDIO/NUS3BANK multi-stream files pass `stream_index` (1-based string).
    /// For a standalone WAV produced with `-L` (smpl), pass `stream_index = None`.
    pub fn query_vgmstream_loop_info(
        input_path: &str,
        stream_index: Option<&str>,
    ) -> Option<VgmstreamLoopInfo> {
        let vgmstream_path = Path::new("tools").join("vgmstream-cli.exe");
        if !vgmstream_path.exists() {
            println!("[LOOP] vgmstream-cli not found at {:?}", vgmstream_path);
            return None;
        }

        let mut command = Command::new(&vgmstream_path);
        #[cfg(windows)]
        {
            use winapi::um::winbase::CREATE_NO_WINDOW;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let mut args: Vec<String> = vec!["-m".to_string()];
        if let Some(s) = stream_index {
            args.push("-s".to_string());
            args.push(s.to_string());
        }
        args.push(input_path.to_string());

        let t0 = Instant::now();
        let output = match command.args(&args).output() {
            Ok(o) => o,
            Err(e) => {
                println!("[LOOP] vgmstream -m failed to run: {e}");
                return None;
            }
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let text = format!("{stdout}\n{stderr}");
        println!(
            "[LOOP] vgmstream -m ({}ms) index={:?} path={}",
            t0.elapsed().as_millis(),
            stream_index,
            input_path
        );

        let info = Self::parse_vgmstream_metadata_loop(&text);
        if let Some(ref lp) = info {
            println!(
                "[LOOP] vgmstream loop: {:.3}s .. {:.3}s",
                lp.loop_start_secs, lp.loop_end_secs
            );
        } else {
            println!("[LOOP] vgmstream: no loop points in metadata");
        }
        info
    }

    /// Parse `vgmstream-cli -m` text for loop start/end (seconds).
    fn parse_vgmstream_metadata_loop(text: &str) -> Option<VgmstreamLoopInfo> {
        let lower = text.to_ascii_lowercase();
        // Explicitly disabled
        if lower.contains("looping: disabled") || lower.contains("looping: no") {
            // Still try parse in case start/end are present for display
        }

        let mut start_secs: Option<f32> = None;
        let mut end_secs: Option<f32> = None;
        let mut sample_rate: Option<f32> = None;
        let mut start_samples: Option<f32> = None;
        let mut end_samples: Option<f32> = None;

        for line in text.lines() {
            let line_trim = line.trim();
            let line_l = line_trim.to_ascii_lowercase();

            if line_l.starts_with("sample rate:") {
                // "sample rate: 48000 Hz"
                if let Some(num) = line_trim
                    .split(':')
                    .nth(1)
                    .and_then(|s| s.split_whitespace().next())
                    .and_then(|s| s.parse::<f32>().ok())
                {
                    sample_rate = Some(num);
                }
            }

            // Prefer seconds in parentheses: "loop start: 123 samples (0:01.234 seconds)"
            if line_l.contains("loop start") {
                if let Some(s) = Self::parse_seconds_from_vgm_line(line_trim) {
                    start_secs = Some(s);
                }
                if let Some(n) = Self::parse_samples_from_vgm_line(line_trim) {
                    start_samples = Some(n);
                }
            }
            if line_l.contains("loop end") {
                if let Some(s) = Self::parse_seconds_from_vgm_line(line_trim) {
                    end_secs = Some(s);
                }
                if let Some(n) = Self::parse_samples_from_vgm_line(line_trim) {
                    end_samples = Some(n);
                }
            }
        }

        // Fall back to samples / sample_rate
        if start_secs.is_none() {
            if let (Some(sr), Some(n)) = (sample_rate, start_samples) {
                if sr > 0.0 {
                    start_secs = Some(n / sr);
                }
            }
        }
        if end_secs.is_none() {
            if let (Some(sr), Some(n)) = (sample_rate, end_samples) {
                if sr > 0.0 {
                    end_secs = Some(n / sr);
                }
            }
        }

        let start = start_secs?;
        let end = end_secs?;
        if end <= start {
            return None;
        }
        // Ignore trivial full-zero
        if end < 0.01 {
            return None;
        }
        Some(VgmstreamLoopInfo {
            loop_start_secs: start.max(0.0),
            loop_end_secs: end,
        })
    }

    /// Extract "N.NNN seconds" from a vgmstream metadata line.
    fn parse_seconds_from_vgm_line(line: &str) -> Option<f32> {
        // Patterns:
        //   (0:01.234 seconds)
        //   (1.234 seconds)
        //   1.234 seconds
        let lower = line.to_ascii_lowercase();
        if let Some(idx) = lower.find("seconds") {
            let before = &line[..idx];
            // Take last token-ish number before "seconds", may be "0:01.234" or "1.234"
            let token = before
                .rsplit(|c: char| c == '(' || c == ' ' || c == '\t')
                .find(|t| !t.is_empty() && t.chars().any(|c| c.is_ascii_digit()))?;
            if let Some(colon) = token.find(':') {
                // m:ss.xxx
                let mins: f32 = token[..colon].parse().ok()?;
                let secs: f32 = token[colon + 1..].trim_end_matches(')').parse().ok()?;
                return Some(mins * 60.0 + secs);
            }
            return token.trim_end_matches(')').parse::<f32>().ok();
        }
        None
    }

    fn parse_samples_from_vgm_line(line: &str) -> Option<f32> {
        // "loop start: 12345 samples (...)"
        let lower = line.to_ascii_lowercase();
        let idx = lower.find("samples")?;
        let before = &line[..idx];
        let token = before
            .rsplit(|c: char| c == ':' || c == ' ' || c == '\t')
            .find(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()))?;
        token.parse::<f32>().ok()
    }

    /// Resolve stream index string for vgmstream `-s` (NUS3AUDIO / NUS3BANK).
    pub fn vgmstream_stream_index_for(
        audio_file_info: &AudioFileInfo,
        original_file_path: &str,
    ) -> Option<String> {
        if audio_file_info.is_nus3bank {
            let id_num = audio_file_info.id.parse::<u32>().ok()?;
            Some((id_num + 1).to_string())
        } else {
            Self::get_vgmstream_index(&audio_file_info.id, original_file_path).ok()
        }
    }

    /// Convert audio to WAV format using vgmstream-cli and return the temp file path
    /// Supports both NUS3AUDIO and NUS3BANK files
    pub fn convert_to_wav_temp_path(
        audio_file_info: &AudioFileInfo,
        original_file_path: &str,
    ) -> Result<String, String> {
        // Check if this is a NUS3BANK file
        if audio_file_info.is_nus3bank {
            return Self::convert_nus3bank_to_wav_temp_path(audio_file_info, original_file_path);
        }

        // Check WAV conversion cache before running vgmstream
        let cache_key = format!("{}::{}::{}", original_file_path, audio_file_info.name, audio_file_info.id);
        if let Ok(cache) = WAV_CONVERSION_CACHE.lock() {
            if let Some(cached_path) = cache.get(&cache_key) {
                if Path::new(cached_path).exists() {
                    println!("[PERF] NUS3AUDIO WAV cache hit: {} -> {}", audio_file_info.name, cached_path);
                    return Ok(cached_path.clone());
                }
            }
        }

        // Original NUS3AUDIO implementation
        // Path to vgmstream-cli.exe in tools directory
        let vgmstream_path = Path::new("tools").join("vgmstream-cli.exe");

        // Create a temporary output file path
        let temp_output_path = Self::build_temp_audio_path(
            &format!("temp_convert_{}", audio_file_info.id),
            "wav",
        );
        let temp_output_path_str = temp_output_path.to_string_lossy().to_string();

        // Run vgmstream-cli to convert audio to WAV
        let mut command = Command::new(&vgmstream_path);

        #[cfg(windows)]
        {
            use winapi::um::winbase::CREATE_NO_WINDOW;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        // Get the correct vgmstream index using intelligent detection
        let t_index = Instant::now();
        let vgmstream_index = Self::get_vgmstream_index(&audio_file_info.id, original_file_path)?;
        println!("[PERF] NUS3AUDIO get_vgmstream_index: {}ms (index={})", t_index.elapsed().as_millis(), vgmstream_index);

        // Build args vector so we can print full command before execution
        let args_vec: Vec<String> = vec![
            "-i".to_string(),
            "-o".to_string(),
            temp_output_path_str.clone(),
            "-s".to_string(),
            vgmstream_index.clone(),
            original_file_path.to_string(),
        ];

        let t_vgm = Instant::now();
        let result = command
            .args(&args_vec)
            .output();
        println!("[PERF] NUS3AUDIO vgmstream-cli: {}ms", t_vgm.elapsed().as_millis());

        match result {
            Ok(output) => {
                if output.status.success() {
                    // Store converted WAV in cache for reuse
                    if let Ok(mut cache) = WAV_CONVERSION_CACHE.lock() {
                        cache.insert(cache_key, temp_output_path_str.clone());
                    }
                    Ok(temp_output_path_str)
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    Err(format!("vgmstream-cli error: {}", error))
                }
            }
            Err(e) => Err(format!("Failed to run vgmstream-cli: {}", e)),
        }
    }

    /// Export audio data to a WAV file with custom output directory using vgmstream-cli
    pub fn export_to_wav_with_custom_dir(
        audio_file_info: &AudioFileInfo,
        original_file_path: &str,
        output_dir: &str,
    ) -> Result<String, String> {
        // Create output file path in the custom directory
        let output_dir_path = Path::new(output_dir);
        let output_filename = crate::nus3bank::export::wav_filename(&audio_file_info.name, "audio");
        let output_path = output_dir_path.join(output_filename);
        let output_path_str = output_path.to_string_lossy().to_string();

        // Path to vgmstream-cli.exe in tools directory
        let vgmstream_path = Path::new("tools").join("vgmstream-cli.exe");

        // Run vgmstream-cli to convert audio to WAV
        let mut command = Command::new(&vgmstream_path);

        #[cfg(windows)]
        {
            use winapi::um::winbase::CREATE_NO_WINDOW;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        // Get the correct vgmstream index using intelligent detection
        let vgmstream_index = Self::get_vgmstream_index(&audio_file_info.id, original_file_path)?;
        
        // println!("Original ID: {}, Detected vgmstream index: {}", audio_file_info.id, vgmstream_index);

        let args_vec: Vec<String> = vec![
            "-i".to_string(),
            "-o".to_string(),
            output_path_str.clone(),
            "-s".to_string(),
            vgmstream_index.clone(),
            original_file_path.to_string(),
        ];
        // println!(
        //     "Running command: {:?} {}",
        //     vgmstream_path,
        //     args_vec.join(" ")
        // );

        let result = command
            .args(&args_vec)
            .output();

        // println!("Exporting command result: {:?}", result);
        match result {
            Ok(output) => {
                if output.status.success() {
                    // println!("Successfully exported WAV file to: {:?}", output_path);
                    Ok(output_path_str)
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    Err(format!("vgmstream-cli error: {}", error))
                }
            }
            Err(e) => Err(format!("Failed to run vgmstream-cli: {}", e)),
        }
    }

    /// Export all audio files in a file to WAV files with custom output directory using vgmstream-cli
    pub fn export_all_to_wav(
        original_file_path: &str,
        output_dir: &str,
    ) -> Result<Vec<String>, String> {
        // Path to vgmstream-cli.exe in tools directory
        let vgmstream_path = Path::new("tools").join("vgmstream-cli.exe");

        // First, load the nus3audio file to get audio file information
        let nus3audio_file = match Nus3audioFile::open(original_file_path) {
            Ok(file) => file,
            Err(e) => return Err(format!("Failed to load nus3audio file: {}", e)),
        };

        // println!(
        //     "Loaded nus3audio file with {} audio files",
        //     nus3audio_file.files.len()
        // );

        let mut exported_paths = Vec::new();
        let output_dir_path = Path::new(output_dir);

        // Export each audio file directly using vgmstream-cli
        for audio_file in nus3audio_file.files.iter() {
            // Get the name for this audio file
            let audio_name = if audio_file.name.is_empty() {
                format!("audio_{}", audio_file.id)
            } else {
                audio_file.name.clone()
            };

            // Create output file path with the audio file name
            let output_filename = crate::nus3bank::export::wav_filename(&audio_name, "audio");
            let output_path = output_dir_path.join(output_filename);
            let output_path_str = output_path.to_string_lossy().to_string();

            // Convert to WAV using vgmstream-cli with the subsong index
            let mut command = Command::new(&vgmstream_path);

            #[cfg(windows)]
            {
                use winapi::um::winbase::CREATE_NO_WINDOW;
                command.creation_flags(CREATE_NO_WINDOW);
            }

            // Get the correct vgmstream index using intelligent detection
            let vgmstream_index = match Self::get_vgmstream_index(&audio_file.id.to_string(), original_file_path) {
                Ok(index) => index,
                Err(e) => {
                    return Err(format!("Failed to determine vgmstream index for audio file {}: {}", audio_file.id, e));
                }
            };
            
            // println!("Original ID: {}, Detected vgmstream index: {}", audio_file.id, vgmstream_index);

            let args_vec: Vec<String> = vec![
                "-o".to_string(),
                output_path_str.clone(),
                "-s".to_string(),
                vgmstream_index.clone(),
                original_file_path.to_string(),
            ];
            // println!(
            //     "Running command: {:?} {}",
            //     vgmstream_path,
            //     args_vec.join(" ")
            // );

            let result = command
                .args(&args_vec)
                .output();

            match result {
                Ok(output) => {
                    if output.status.success() {
                        // println!("Successfully exported WAV file to: {:?}", output_path);
                        exported_paths.push(output_path_str);
                    } else {
                        let error = String::from_utf8_lossy(&output.stderr);
                        return Err(format!(
                            "vgmstream-cli error on audio file {}: {}",
                            audio_file.id, error
                        ));
                    }
                }
                Err(e) => {
                    return Err(format!(
                        "Failed to run vgmstream-cli for audio file {}: {}",
                        audio_file.id, e
                    ));
                }
            }
        }

        Ok(exported_paths)
    }
    
    /// Convert NUS3BANK track to WAV format and return the temp file path
    fn convert_nus3bank_to_wav_temp_path(
        audio_file_info: &AudioFileInfo,
        original_file_path: &str,
    ) -> Result<String, String> {
        // Check WAV conversion cache before running vgmstream
        let cache_key = format!("{}::{}::{}", original_file_path, audio_file_info.name, audio_file_info.id);
        if let Ok(cache) = WAV_CONVERSION_CACHE.lock() {
            if let Some(cached_path) = cache.get(&cache_key) {
                if Path::new(cached_path).exists() {
                    println!("[PERF] NUS3BANK WAV cache hit: {} -> {}", audio_file_info.name, cached_path);
                    return Ok(cached_path.clone());
                }
            }
        }

        // Use vgmstream-cli to decode specific subsong into a temporary WAV
        // Compute subsong index for vgmstream (1-based). Our UI id is 0-based.
        let id_num = audio_file_info.id.parse::<u32>()
            .map_err(|_| format!("Invalid audio file ID: {}", audio_file_info.id))?;
        let vgmstream_index = id_num + 1;

        // Path to vgmstream-cli.exe in tools directory
        let vgmstream_path = Path::new("tools").join("vgmstream-cli.exe");

        // Create a temporary output file path
        let temp_output_path = Self::build_temp_audio_path(
            &format!("temp_convert_bank_{}", vgmstream_index),
            "wav",
        );
        let temp_output_path_str = temp_output_path.to_string_lossy().to_string();

        // Run vgmstream-cli to convert audio to WAV
        let mut command = Command::new(&vgmstream_path);

        #[cfg(windows)]
        {
            use winapi::um::winbase::CREATE_NO_WINDOW;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let args_vec: Vec<String> = vec![
            "-o".to_string(),
            temp_output_path_str.clone(),
            "-s".to_string(),
            vgmstream_index.to_string(),
            original_file_path.to_string(),
        ];

        let t_vgm = Instant::now();
        let result = command
            .args(&args_vec)
            .output();
        println!("[PERF] NUS3BANK vgmstream-cli (subsong {}): {}ms", vgmstream_index, t_vgm.elapsed().as_millis());

        match result {
            Ok(output) => {
                if output.status.success() {
                    // Store converted WAV in cache for reuse
                    if let Ok(mut cache) = WAV_CONVERSION_CACHE.lock() {
                        cache.insert(cache_key, temp_output_path_str.clone());
                    }
                    Ok(temp_output_path_str)
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    Err(format!("vgmstream-cli error: {}", error))
                }
            }
            Err(e) => Err(format!("Failed to run vgmstream-cli: {}", e)),
        }
    }

    /// Export NUS3BANK track to WAV file with custom output directory
    pub fn export_nus3bank_to_wav_with_custom_dir(
        audio_file_info: &AudioFileInfo,
        original_file_path: &str,
        output_dir: &str,
    ) -> Result<String, String> {
        // Compute subsong index for vgmstream (1-based). Our UI id is 0-based.
        let id_num = audio_file_info.id.parse::<u32>()
            .map_err(|_| format!("Invalid audio file ID: {}", audio_file_info.id))?;
        let vgmstream_index = id_num + 1;

        // Create output file path in the custom directory
        let output_dir_path = Path::new(output_dir);
        let output_filename = crate::nus3bank::export::wav_filename(&audio_file_info.name, "audio");
        let output_path = output_dir_path.join(output_filename);
        let output_path_str = output_path.to_string_lossy().to_string();

        // Path to vgmstream-cli.exe in tools directory
        let vgmstream_path = Path::new("tools").join("vgmstream-cli.exe");

        // Run vgmstream-cli to convert audio to WAV
        let mut command = Command::new(&vgmstream_path);

        #[cfg(windows)]
        {
            use winapi::um::winbase::CREATE_NO_WINDOW;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let args_vec: Vec<String> = vec![
            "-o".to_string(),
            output_path_str.clone(),
            "-s".to_string(),
            vgmstream_index.to_string(),
            original_file_path.to_string(),
        ];
        // println!(
        //     "Running command: {:?} {}",
        //     vgmstream_path,
        //     args_vec.join(" ")
        // );

        let result = command
            .args(&args_vec)
            .output();

        // println!("Exporting command (NUS3BANK) result: {:?}", result);
        match result {
            Ok(output) => {
                if output.status.success() {
                    // println!("Successfully exported WAV file to: {:?}", output_path);
                    Ok(output_path_str)
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    Err(format!("vgmstream-cli error: {}", error))
                }
            }
            Err(e) => Err(format!("Failed to run vgmstream-cli: {}", e)),
        }
    }
    
    /// Export all tracks from NUS3BANK file
    pub fn export_all_nus3bank_to_wav(
        original_file_path: &str,
        output_dir: &str,
    ) -> Result<Vec<String>, String> {
        Nus3bankExporter::export_all_tracks(original_file_path, output_dir)
    }
    
    /// Unified export method that works with both NUS3AUDIO and NUS3BANK files
    pub fn export_to_wav_with_custom_dir_unified(
        audio_file_info: &AudioFileInfo,
        original_file_path: &str,
        output_dir: &str,
    ) -> Result<String, String> {
        if audio_file_info.is_nus3bank {
            Self::export_nus3bank_to_wav_with_custom_dir(audio_file_info, original_file_path, output_dir)
        } else {
            Self::export_to_wav_with_custom_dir(audio_file_info, original_file_path, output_dir)
        }
    }
    
    /// Unified export all method that works with both file types
    pub fn export_all_to_wav_unified(
        original_file_path: &str,
        output_dir: &str,
    ) -> Result<Vec<String>, String> {
        if original_file_path.to_lowercase().ends_with(".nus3bank") {
            Self::export_all_nus3bank_to_wav(original_file_path, output_dir)
        } else {
            Self::export_all_to_wav(original_file_path, output_dir)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    #[cfg(windows)]
    fn nus3bank_single_export_sanitizes_name_if_present() {
        let input = "../song_wgnmd1.nus3bank";
        if !Path::new(input).exists() { return; }
        let output_dir = ExportUtils::build_temp_audio_path("single_export_test", "dir");
        fs::create_dir(&output_dir).unwrap();
        let info = AudioFileInfo::from_nus3bank_track(
            "song\0wgnmd1".to_string(), 0, "0x0".to_string(), 3439408,
            "unused.wav".to_string(),
        );
        let path = ExportUtils::export_to_wav_with_custom_dir_unified(
            &info, input, output_dir.to_str().unwrap(),
        ).unwrap();
        assert_eq!(Path::new(&path).file_name().unwrap(), "song_wgnmd1.wav");
        let reader = hound::WavReader::open(&path).unwrap();
        assert_eq!(reader.spec().sample_rate, 48000);
        assert_eq!(reader.duration(), 5158694);
        drop(reader);
        fs::remove_file(path).unwrap();
        fs::remove_dir(output_dir).unwrap();
    }

    fn touch(path: &str) {
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(b"dummy").unwrap();
    }

    // Unique prefix avoids collisions when tests run in parallel.
    fn cache_key(tag: &str) -> String {
        format!("test_file_{}::track::0", tag)
    }

    #[test]
    fn wav_cache_hit_returns_existing_file() {
        let tmp = std::env::temp_dir().join("eu_cache_hit_test.wav");
        let tmp_str = tmp.to_str().unwrap().to_string();
        touch(&tmp_str);

        {
            let mut c = WAV_CONVERSION_CACHE.lock().unwrap();
            c.insert(cache_key("hit"), tmp_str.clone());
        }

        let found = WAV_CONVERSION_CACHE
            .lock()
            .unwrap()
            .get(&cache_key("hit"))
            .filter(|p| Path::new(p).exists())
            .cloned();

        assert_eq!(found, Some(tmp_str));
        WAV_CONVERSION_CACHE.lock().unwrap().remove(&cache_key("hit"));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn wav_cache_miss_when_file_deleted() {
        let tmp_str = std::env::temp_dir()
            .join("eu_cache_miss_test.wav")
            .to_str()
            .unwrap()
            .to_string();
        // File intentionally NOT created

        {
            let mut c = WAV_CONVERSION_CACHE.lock().unwrap();
            c.insert(cache_key("miss"), tmp_str.clone());
        }

        let found = WAV_CONVERSION_CACHE
            .lock()
            .unwrap()
            .get(&cache_key("miss"))
            .filter(|p| Path::new(p).exists())
            .cloned();

        assert_eq!(found, None, "stale cache entry must miss when file is absent");
        WAV_CONVERSION_CACHE.lock().unwrap().remove(&cache_key("miss"));
    }

    #[test]
    fn clear_wav_cache_for_file_removes_only_matching_entries() {
        {
            let mut c = WAV_CONVERSION_CACHE.lock().unwrap();
            c.insert("myfile.nus3audio::track_a::0".to_string(), "/tmp/a.wav".to_string());
            c.insert("myfile.nus3audio::track_b::1".to_string(), "/tmp/b.wav".to_string());
            c.insert("otherfile.nus3audio::track_c::0".to_string(), "/tmp/c.wav".to_string());
        }

        ExportUtils::clear_wav_cache_for_file("myfile.nus3audio");

        let c = WAV_CONVERSION_CACHE.lock().unwrap();
        assert!(!c.contains_key("myfile.nus3audio::track_a::0"), "cleared");
        assert!(!c.contains_key("myfile.nus3audio::track_b::1"), "cleared");
        assert!(c.contains_key("otherfile.nus3audio::track_c::0"), "unrelated entry must survive");
        drop(c);

        WAV_CONVERSION_CACHE.lock().unwrap().remove("otherfile.nus3audio::track_c::0");
    }

    #[test]
    fn prime_indexing_cache_zero_based() {
        ExportUtils::prime_indexing_cache("test_zero_idx.nus3audio", &[0, 1, 2]);
        assert_eq!(
            INDEXING_PATTERN_CACHE.lock().unwrap().get("test_zero_idx.nus3audio").copied(),
            Some(true)
        );
    }

    #[test]
    fn prime_indexing_cache_one_based() {
        ExportUtils::prime_indexing_cache("test_one_idx.nus3audio", &[1, 2, 3]);
        assert_eq!(
            INDEXING_PATTERN_CACHE.lock().unwrap().get("test_one_idx.nus3audio").copied(),
            Some(false)
        );
    }

    #[test]
    fn parse_loop_seconds_from_vgmstream_m() {
        let text = r#"
sample rate: 48000 Hz
channels: 2
looping: Yes
loop start: 48000 samples (0:01.000 seconds)
loop end: 144000 samples (0:03.000 seconds)
stream total samples: 200000 (0:04.166 seconds)
"#;
        let info = ExportUtils::parse_vgmstream_metadata_loop(text).expect("loop");
        assert!((info.loop_start_secs - 1.0).abs() < 0.01);
        assert!((info.loop_end_secs - 3.0).abs() < 0.01);
    }

    #[test]
    fn parse_loop_from_samples_only() {
        let text = r#"
sample rate: 44100 Hz
looping: yes
loop start: 44100 samples
loop end: 88200 samples
"#;
        let info = ExportUtils::parse_vgmstream_metadata_loop(text).expect("loop");
        assert!((info.loop_start_secs - 1.0).abs() < 0.01);
        assert!((info.loop_end_secs - 2.0).abs() < 0.01);
    }
}
