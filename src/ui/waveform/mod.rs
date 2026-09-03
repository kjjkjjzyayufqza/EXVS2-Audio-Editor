//! Sound-wave visualization (min/max peak bars) with optional loop markers.
//! Drawn with egui::Painter — not a scientific plot chart.
//!
//! Peak extraction is optimized for large WAVs: sparse sample access (not a full
//! sample-by-sample scan), and can be spawned on a background thread.

use egui::{
    Color32, CursorIcon, FontId, Id, Pos2, Rect, Response, Sense, Shape, Stroke, TextStyle, Ui,
    pos2, vec2,
};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// Default bin count for UI waveforms (enough detail, cheap to draw).
pub const DEFAULT_PEAK_BINS: usize = 1024;

/// Cached peak envelope for a track (resolution independent of display width).
#[derive(Clone, Debug, Default)]
pub struct WaveformPeaks {
    /// Per-bin (min, max) amplitudes in `-1.0..=1.0`.
    pub peaks: Vec<(f32, f32)>,
    /// Total duration in seconds.
    pub duration_secs: f32,
    /// Sample rate (Hz).
    pub sample_rate: u32,
}

impl WaveformPeaks {
    pub fn is_empty(&self) -> bool {
        self.peaks.is_empty() || self.duration_secs <= 0.0
    }

    /// Fast peaks from a WAV path via sparse PCM seeks (does not load whole file).
    pub fn from_wav_path(path: impl AsRef<Path>, max_bins: usize) -> Result<Self, String> {
        use std::io::{Read, Seek, SeekFrom};

        let path = path.as_ref();
        let mut file = std::fs::File::open(path)
            .map_err(|e| format!("Failed to open WAV for waveform: {e}"))?;

        // Parse header from a prefix (fmt/data rarely need more than a few KB; allow 1MB junk).
        let mut header = vec![0u8; 1024 * 1024];
        let n = file
            .read(&mut header)
            .map_err(|e| format!("Failed to read WAV header: {e}"))?;
        header.truncate(n);
        let info = parse_wav_pcm(&header)?;

        if info.total_frames == 0 || info.sample_rate == 0 {
            return Ok(Self {
                peaks: Vec::new(),
                duration_secs: 0.0,
                sample_rate: info.sample_rate,
            });
        }

        let bins = max_bins.clamp(32, 4096).min(info.total_frames).max(1);
        let frames_per_bin = ((info.total_frames as f64 / bins as f64).ceil() as usize).max(1);
        let samples_per_bin = 32usize;
        let mut peaks = vec![(0.0f32, 0.0f32); bins];
        let mut frame_buf = vec![0u8; info.block_align.max(4)];

        for bin in 0..bins {
            let frame0 = bin * frames_per_bin;
            if frame0 >= info.total_frames {
                break;
            }
            let frame1 = ((bin + 1) * frames_per_bin).min(info.total_frames);
            let span = (frame1 - frame0).max(1);
            let step = (span / samples_per_bin).max(1);

            let mut min_v = 0.0f32;
            let mut max_v = 0.0f32;
            let mut any = false;

            let mut sample_frame = |frame: usize| {
                let off = info.data_offset as u64 + (frame as u64) * (info.block_align as u64);
                if file.seek(SeekFrom::Start(off)).is_err() {
                    return;
                }
                if file.read_exact(&mut frame_buf[..info.block_align]).is_err() {
                    return;
                }
                if let Some(sample) = mono_from_frame_bytes(&info, &frame_buf[..info.block_align]) {
                    if !any {
                        min_v = sample;
                        max_v = sample;
                        any = true;
                    } else {
                        min_v = min_v.min(sample);
                        max_v = max_v.max(sample);
                    }
                }
            };

            let mut f = frame0;
            while f < frame1 {
                sample_frame(f);
                f += step;
            }
            if frame1 > frame0 + 1 {
                sample_frame(frame1 - 1);
            }

            if any {
                min_v = min_v.clamp(-1.0, 1.0);
                max_v = max_v.clamp(-1.0, 1.0);
                if (max_v - min_v).abs() < 0.01 {
                    min_v = min_v.min(-0.01);
                    max_v = max_v.max(0.01);
                }
                peaks[bin] = (min_v, max_v);
            } else {
                peaks[bin] = (-0.01, 0.01);
            }
        }

        let duration_secs = info.total_frames as f32 / info.sample_rate as f32;
        Ok(Self {
            peaks,
            duration_secs,
            sample_rate: info.sample_rate,
        })
    }

    /// Parse RIFF/WAVE bytes and build peaks with sparse sampling.
    pub fn from_wav_bytes(bytes: &[u8], max_bins: usize) -> Result<Self, String> {
        let info = parse_wav_pcm(bytes)?;
        if info.total_frames == 0 || info.sample_rate == 0 {
            return Ok(Self {
                peaks: Vec::new(),
                duration_secs: 0.0,
                sample_rate: info.sample_rate,
            });
        }

        let bins = max_bins.clamp(32, 4096).min(info.total_frames).max(1);
        let frames_per_bin = ((info.total_frames as f64 / bins as f64).ceil() as usize).max(1);
        // Cap samples examined per bin — keeps long tracks near O(bins * 48).
        let samples_per_bin = 48usize;

        let mut peaks = vec![(0.0f32, 0.0f32); bins];

        for bin in 0..bins {
            let frame0 = bin * frames_per_bin;
            if frame0 >= info.total_frames {
                break;
            }
            let frame1 = ((bin + 1) * frames_per_bin).min(info.total_frames);
            let span = (frame1 - frame0).max(1);
            let step = (span / samples_per_bin).max(1);

            let mut min_v = 0.0f32;
            let mut max_v = 0.0f32;
            let mut any = false;

            let mut f = frame0;
            while f < frame1 {
                if let Some(sample) = read_mono_frame(&info, bytes, f) {
                    if !any {
                        min_v = sample;
                        max_v = sample;
                        any = true;
                    } else {
                        min_v = min_v.min(sample);
                        max_v = max_v.max(sample);
                    }
                }
                f += step;
            }

            // Always include last frame of the bin for edge accuracy
            if frame1 > frame0 + 1 {
                if let Some(sample) = read_mono_frame(&info, bytes, frame1 - 1) {
                    if !any {
                        min_v = sample;
                        max_v = sample;
                        any = true;
                    } else {
                        min_v = min_v.min(sample);
                        max_v = max_v.max(sample);
                    }
                }
            }

            if any {
                min_v = min_v.clamp(-1.0, 1.0);
                max_v = max_v.clamp(-1.0, 1.0);
                if (max_v - min_v).abs() < 0.01 {
                    min_v = min_v.min(-0.01);
                    max_v = max_v.max(0.01);
                }
                peaks[bin] = (min_v, max_v);
            } else {
                peaks[bin] = (-0.01, 0.01);
            }
        }

        let duration_secs = info.total_frames as f32 / info.sample_rate as f32;
        Ok(Self {
            peaks,
            duration_secs,
            sample_rate: info.sample_rate,
        })
    }

    /// Try WAV path; returns empty peaks on failure.
    /// Order: sparse seek parse → hound (small files only) → optional vgmstream decode.
    pub fn try_from_audio_path(path: impl AsRef<Path>, max_bins: usize) -> Self {
        let path = path.as_ref();
        let t0 = std::time::Instant::now();
        match Self::from_wav_path(path, max_bins) {
            Ok(peaks) if !peaks.is_empty() => {
                println!(
                    "[PERF] waveform sparse OK: {} bins, {:.2}s in {}ms ({})",
                    peaks.peaks.len(),
                    peaks.duration_secs,
                    t0.elapsed().as_millis(),
                    path.display()
                );
                return peaks;
            }
            Ok(_) => {
                println!(
                    "[PERF] waveform sparse empty in {}ms ({})",
                    t0.elapsed().as_millis(),
                    path.display()
                );
            }
            Err(e) => {
                println!(
                    "[PERF] waveform sparse failed in {}ms: {} ({})",
                    t0.elapsed().as_millis(),
                    e,
                    path.display()
                );
            }
        }

        // Hound full-scan is expensive — only for modest files
        let file_len = std::fs::metadata(path).map(|m| m.len()).unwrap_or(u64::MAX);
        if file_len <= 12 * 1024 * 1024 {
            let t1 = std::time::Instant::now();
            match Self::from_wav_via_hound(path, max_bins) {
                Ok(peaks) if !peaks.is_empty() => {
                    println!(
                        "[PERF] waveform hound OK: {} bins in {}ms",
                        peaks.peaks.len(),
                        t1.elapsed().as_millis()
                    );
                    return peaks;
                }
                Ok(_) => {}
                Err(e) => {
                    println!("[PERF] waveform hound failed: {e}");
                }
            }
        } else {
            println!(
                "[PERF] waveform skip hound (file too large: {} MB)",
                file_len / (1024 * 1024)
            );
        }

        let t2 = std::time::Instant::now();
        if let Some(peaks) = Self::from_via_vgmstream(path, max_bins) {
            if !peaks.is_empty() {
                println!(
                    "[PERF] waveform vgmstream OK: {} bins in {}ms",
                    peaks.peaks.len(),
                    t2.elapsed().as_millis()
                );
                return peaks;
            }
        }

        println!(
            "[PERF] waveform UNAVAILABLE for {} (total {}ms)",
            path.display(),
            t0.elapsed().as_millis()
        );
        Self::default()
    }

    /// Stream via hound (handles more WAV variants than the sparse parser).
    fn from_wav_via_hound(path: &Path, max_bins: usize) -> Result<Self, String> {
        use hound::{SampleFormat, WavReader};

        let mut reader = WavReader::open(path).map_err(|e| format!("hound open failed: {e}"))?;
        let spec = reader.spec();
        let channels = spec.channels.max(1) as usize;
        let sample_rate = spec.sample_rate.max(1);
        let total_frames = reader.duration() as usize;
        if total_frames == 0 {
            return Ok(Self {
                peaks: Vec::new(),
                duration_secs: 0.0,
                sample_rate,
            });
        }

        let bins = max_bins.clamp(32, 4096).min(total_frames).max(1);
        let frames_per_bin = ((total_frames as f64 / bins as f64).ceil() as usize).max(1);
        let samples_per_bin = 48usize;
        let step = (frames_per_bin / samples_per_bin).max(1);

        let mut peaks = vec![(0.0f32, 0.0f32); bins];
        let mut frame = 0usize;
        let mut ch = 0usize;
        let mut mono_acc = 0.0f32;
        let mut seeded = vec![false; bins];
        let mut push_seeded = |frame: usize, sample: f32| {
            let bin = (frame / frames_per_bin).min(bins - 1);
            let local = frame % frames_per_bin;
            if local % step != 0 && local + 1 != frames_per_bin {
                return;
            }
            if !seeded[bin] {
                peaks[bin] = (sample, sample);
                seeded[bin] = true;
            } else {
                peaks[bin].0 = peaks[bin].0.min(sample);
                peaks[bin].1 = peaks[bin].1.max(sample);
            }
        };

        match (spec.sample_format, spec.bits_per_sample) {
            (SampleFormat::Int, 16) => {
                for s in reader.samples::<i16>() {
                    let v = s.map_err(|e| format!("sample: {e}"))? as f32 / 32768.0;
                    mono_acc += v;
                    ch += 1;
                    if ch >= channels {
                        push_seeded(frame, mono_acc / channels as f32);
                        frame += 1;
                        ch = 0;
                        mono_acc = 0.0;
                    }
                }
            }
            (SampleFormat::Float, _) => {
                for s in reader.samples::<f32>() {
                    let v = s.map_err(|e| format!("sample: {e}"))?;
                    mono_acc += v;
                    ch += 1;
                    if ch >= channels {
                        push_seeded(frame, mono_acc / channels as f32);
                        frame += 1;
                        ch = 0;
                        mono_acc = 0.0;
                    }
                }
            }
            (SampleFormat::Int, 8) => {
                for s in reader.samples::<i8>() {
                    let v = s.map_err(|e| format!("sample: {e}"))? as f32 / 128.0;
                    mono_acc += v;
                    ch += 1;
                    if ch >= channels {
                        push_seeded(frame, mono_acc / channels as f32);
                        frame += 1;
                        ch = 0;
                        mono_acc = 0.0;
                    }
                }
            }
            (SampleFormat::Int, bits @ (24 | 32)) => {
                for s in reader.samples::<i32>() {
                    let raw = s.map_err(|e| format!("sample: {e}"))?;
                    let v = if bits == 24 {
                        raw as f32 / 8_388_608.0
                    } else {
                        raw as f32 / 2_147_483_648.0
                    };
                    mono_acc += v;
                    ch += 1;
                    if ch >= channels {
                        push_seeded(frame, mono_acc / channels as f32);
                        frame += 1;
                        ch = 0;
                        mono_acc = 0.0;
                    }
                }
            }
            _ => {
                return Err(format!(
                    "hound unsupported format {:?} {}-bit",
                    spec.sample_format, spec.bits_per_sample
                ));
            }
        }

        for (i, entry) in peaks.iter_mut().enumerate() {
            if !seeded[i] {
                *entry = (-0.01, 0.01);
            } else {
                entry.0 = entry.0.clamp(-1.0, 1.0);
                entry.1 = entry.1.clamp(-1.0, 1.0);
                if (entry.1 - entry.0).abs() < 0.01 {
                    entry.0 = entry.0.min(-0.01);
                    entry.1 = entry.1.max(0.01);
                }
            }
        }

        Ok(Self {
            peaks,
            duration_secs: total_frames as f32 / sample_rate as f32,
            sample_rate,
        })
    }

    /// Decode arbitrary audio to a temp WAV via vgmstream-cli, then extract peaks.
    fn from_via_vgmstream(path: &Path, max_bins: usize) -> Option<Self> {
        let candidates = [
            PathBuf::from("tools").join("vgmstream-cli.exe"),
            std::env::current_exe()
                .ok()
                .and_then(|p| {
                    p.parent()
                        .map(|d| d.join("tools").join("vgmstream-cli.exe"))
                })
                .unwrap_or_default(),
        ];
        let vgm = candidates.into_iter().find(|p| p.exists())?;
        Self::run_vgmstream_peaks(&vgm, path, max_bins)
    }

    fn run_vgmstream_peaks(vgm: &Path, path: &Path, max_bins: usize) -> Option<Self> {
        use std::process::Command;
        let temp =
            std::env::temp_dir().join(format!("exvs2_wave_preview_{}.wav", std::process::id()));
        let mut cmd = Command::new(vgm);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let out = cmd
            .args([
                "-o",
                temp.to_str()?,
                "-i", // ignore loops, decode once
                path.to_str()?,
            ])
            .output()
            .ok()?;
        if !out.status.success() || !temp.exists() {
            let _ = std::fs::remove_file(&temp);
            return None;
        }
        let peaks = Self::from_wav_path(&temp, max_bins)
            .ok()
            .or_else(|| Self::from_wav_via_hound(&temp, max_bins).ok());
        let _ = std::fs::remove_file(&temp);
        peaks
    }

    /// Spawn background peak extraction. Returns a receiver of `(generation, peaks)`.
    /// Caller should ignore results whose generation does not match the latest request.
    pub fn spawn_load(
        path: PathBuf,
        max_bins: usize,
        generation: u64,
    ) -> Receiver<(u64, WaveformPeaks)> {
        let (tx, rx): (Sender<(u64, WaveformPeaks)>, Receiver<(u64, WaveformPeaks)>) =
            mpsc::channel();
        println!(
            "[PERF] waveform worker spawn gen={generation} path={}",
            path.display()
        );
        match thread::Builder::new()
            .name("waveform-peaks".into())
            .spawn(move || {
                let peaks = Self::try_from_audio_path(&path, max_bins);
                println!(
                    "[PERF] waveform worker done gen={generation} empty={} bins={}",
                    peaks.is_empty(),
                    peaks.peaks.len()
                );
                let _ = tx.send((generation, peaks));
            }) {
            Ok(_) => {}
            Err(e) => {
                println!("[PERF] waveform worker SPAWN FAILED: {e}");
                // Dropping tx without send → receiver sees Disconnected
            }
        }
        rx
    }
}

#[derive(Clone, Copy)]
struct WavPcmInfo {
    sample_rate: u32,
    channels: usize,
    bits_per_sample: u16,
    is_float: bool,
    data_offset: usize,
    data_size: usize,
    total_frames: usize,
    block_align: usize,
}

fn parse_wav_pcm(bytes: &[u8]) -> Result<WavPcmInfo, String> {
    if bytes.len() < 44 {
        return Err("WAV too short".into());
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("Not a RIFF/WAVE file".into());
    }

    let mut offset = 12usize;
    let mut fmt_audio_format: u16 = 0;
    let mut channels: u16 = 0;
    let mut sample_rate: u32 = 0;
    let mut bits_per_sample: u16 = 0;
    let mut block_align: u16 = 0;
    let mut data_offset = 0usize;
    let mut data_size = 0usize;
    let mut found_fmt = false;
    let mut found_data = false;

    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes([
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]) as usize;
        let body = offset + 8;
        let next = body.saturating_add(size);
        // Chunks are word-aligned
        let next_aligned = next + (next % 2);

        if id == b"fmt " && body + 16 <= bytes.len() {
            fmt_audio_format = u16::from_le_bytes([bytes[body], bytes[body + 1]]);
            channels = u16::from_le_bytes([bytes[body + 2], bytes[body + 3]]);
            sample_rate = u32::from_le_bytes([
                bytes[body + 4],
                bytes[body + 5],
                bytes[body + 6],
                bytes[body + 7],
            ]);
            block_align = u16::from_le_bytes([bytes[body + 12], bytes[body + 13]]);
            bits_per_sample = u16::from_le_bytes([bytes[body + 14], bytes[body + 15]]);
            found_fmt = true;
        } else if id == b"data" {
            data_offset = body;
            // Use the RIFF chunk size as declared — do NOT clamp to the header buffer.
            // Callers that only read a header prefix still need the full PCM frame count.
            data_size = size;
            found_data = true;
            break;
        }

        if next_aligned <= offset {
            break;
        }
        offset = next_aligned;
    }

    if !found_fmt || !found_data {
        return Err("Missing fmt/data chunk".into());
    }

    // 1 = PCM, 3 = IEEE float, 0xFFFE = extensible (treat as PCM/float via bits)
    let is_float = fmt_audio_format == 3 || (fmt_audio_format == 0xFFFE && bits_per_sample == 32);
    let is_pcm = fmt_audio_format == 1 || fmt_audio_format == 0xFFFE || is_float;
    if !is_pcm {
        return Err(format!("Unsupported WAV format tag: {fmt_audio_format}"));
    }
    if channels == 0 || sample_rate == 0 || bits_per_sample == 0 {
        return Err("Invalid WAV format fields".into());
    }

    let channels = channels as usize;
    let bytes_per_sample = (bits_per_sample as usize / 8).max(1);
    let block = if block_align as usize >= bytes_per_sample {
        block_align as usize
    } else {
        bytes_per_sample * channels
    };
    let total_frames = data_size / block.max(1);

    Ok(WavPcmInfo {
        sample_rate,
        channels,
        bits_per_sample,
        is_float,
        data_offset,
        data_size,
        total_frames,
        block_align: block,
    })
}

fn read_mono_frame(info: &WavPcmInfo, bytes: &[u8], frame: usize) -> Option<f32> {
    let frame_off = info.data_offset + frame * info.block_align;
    if frame_off + info.block_align > bytes.len()
        || frame_off + info.block_align > info.data_offset + info.data_size
    {
        return None;
    }
    mono_from_frame_bytes(info, &bytes[frame_off..frame_off + info.block_align])
}

fn mono_from_frame_bytes(info: &WavPcmInfo, frame: &[u8]) -> Option<f32> {
    if frame.len() < info.block_align {
        return None;
    }
    let mut acc = 0.0f32;
    let bps = (info.bits_per_sample / 8) as usize;
    for ch in 0..info.channels {
        let off = ch * bps;
        let s = match (info.is_float, info.bits_per_sample) {
            (true, 32) if off + 4 <= frame.len() => {
                f32::from_le_bytes([frame[off], frame[off + 1], frame[off + 2], frame[off + 3]])
            }
            (false, 8) => {
                let u = frame[off] as i16 - 128;
                u as f32 / 128.0
            }
            (false, 16) if off + 2 <= frame.len() => {
                i16::from_le_bytes([frame[off], frame[off + 1]]) as f32 / 32768.0
            }
            (false, 24) if off + 3 <= frame.len() => {
                let v = i32::from_le_bytes([frame[off], frame[off + 1], frame[off + 2], 0]);
                let v = if v & 0x0080_0000 != 0 {
                    v | !0x00FF_FFFF
                } else {
                    v
                };
                v as f32 / 8_388_608.0
            }
            (false, 32) if off + 4 <= frame.len() => {
                i32::from_le_bytes([frame[off], frame[off + 1], frame[off + 2], frame[off + 3]])
                    as f32
                    / 2_147_483_648.0
            }
            _ => return None,
        };
        acc += s;
    }
    Some(acc / info.channels as f32)
}

/// Visual / interaction options for one draw pass.
#[derive(Clone, Debug)]
pub struct WaveformOptions {
    pub height: f32,
    pub playhead_secs: Option<f32>,
    pub loop_start_secs: Option<f32>,
    pub loop_end_secs: Option<f32>,
    pub show_loop: bool,
    /// Allow dragging loop handles (start/end circles).
    pub interactive_loop: bool,
    /// Click / drag on the wave seeks.
    pub interactive_seek: bool,
    pub duration_override: Option<f32>,
    /// Show a subtle loading shimmer when peaks are not ready yet.
    pub loading: bool,
}

impl Default for WaveformOptions {
    fn default() -> Self {
        Self {
            height: 72.0,
            playhead_secs: None,
            loop_start_secs: None,
            loop_end_secs: None,
            show_loop: false,
            interactive_loop: false,
            interactive_seek: true,
            duration_override: None,
            loading: false,
        }
    }
}

/// Result of pointer interaction with the waveform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WaveformAction {
    None,
    /// Continuous scrub while the pointer is held (UI only / throttled seek — do NOT hard-seek every frame).
    Scrub(f32),
    /// Single commit: click or drag released. Safe to apply to the audio backend once.
    Seek(f32),
    LoopStart(f32),
    LoopEnd(f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DragTarget {
    None,
    Seek,
    LoopStart,
    LoopEnd,
}

/// Professional sound-wave widget (vertical peak bars + loop region + circled handles).
pub struct WaveformWidget;

impl WaveformWidget {
    pub fn show(
        ui: &mut Ui,
        id: impl Into<Id>,
        peaks: Option<&WaveformPeaks>,
        options: &WaveformOptions,
    ) -> (Response, WaveformAction) {
        let id = id.into();
        let available_width = ui.available_width().max(32.0);
        let height = options.height.clamp(40.0, 220.0);
        let desired = vec2(available_width, height);

        let (response, painter) = ui.allocate_painter(desired, Sense::click_and_drag());
        let rect = response.rect;
        let painter = painter.with_clip_rect(rect);

        let duration = options
            .duration_override
            .or_else(|| peaks.map(|p| p.duration_secs))
            .unwrap_or(0.0)
            .max(0.0);

        let bg = if ui.visuals().dark_mode {
            Color32::from_rgb(18, 18, 20)
        } else {
            Color32::from_rgb(236, 236, 240)
        };
        let border = ui.visuals().widgets.noninteractive.bg_stroke.color;
        let wave_color = Color32::from_rgb(92, 148, 210);
        let wave_played = Color32::from_rgb(120, 188, 240);
        let wave_outside_loop = Color32::from_rgba_unmultiplied(92, 148, 210, 70);
        let center_line = Color32::from_rgba_unmultiplied(255, 255, 255, 18);
        let loop_fill = Color32::from_rgba_unmultiplied(232, 176, 72, 55);
        let loop_stroke = Color32::from_rgb(232, 176, 72);
        let playhead_color = Color32::from_rgb(255, 210, 120);
        let dim_overlay = Color32::from_rgba_unmultiplied(0, 0, 0, 90);

        painter.rect_filled(rect, 6.0, bg);
        painter.rect_stroke(
            rect,
            6.0,
            Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );

        let inner = rect.shrink2(vec2(6.0, 8.0));
        if inner.width() < 4.0 || inner.height() < 4.0 {
            return (response, WaveformAction::None);
        }

        let mid_y = inner.center().y;
        let half_h = inner.height() * 0.46;

        painter.line_segment(
            [pos2(inner.left(), mid_y), pos2(inner.right(), mid_y)],
            Stroke::new(1.0, center_line),
        );

        // Resolve loop range for drawing
        let (loop_start, loop_end) = if options.show_loop && duration > 0.0 {
            let start = options.loop_start_secs.unwrap_or(0.0).clamp(0.0, duration);
            let end = options
                .loop_end_secs
                .unwrap_or(duration)
                .clamp(0.0, duration);
            if start <= end {
                (start, end)
            } else {
                (end, start)
            }
        } else {
            (0.0, 0.0)
        };
        let loop_x0 = if options.show_loop && duration > 0.0 {
            Some(time_to_x(loop_start, duration, inner))
        } else {
            None
        };
        let loop_x1 = if options.show_loop && duration > 0.0 {
            Some(time_to_x(loop_end, duration, inner))
        } else {
            None
        };

        // Dim regions outside the loop so the loop range reads as "active"
        if let (Some(x0), Some(x1)) = (loop_x0, loop_x1) {
            if x0 > inner.left() + 1.0 {
                painter.rect_filled(
                    Rect::from_min_max(pos2(inner.left(), inner.top()), pos2(x0, inner.bottom())),
                    0.0,
                    dim_overlay,
                );
            }
            if x1 < inner.right() - 1.0 {
                painter.rect_filled(
                    Rect::from_min_max(pos2(x1, inner.top()), pos2(inner.right(), inner.bottom())),
                    0.0,
                    dim_overlay,
                );
            }
            let loop_rect = Rect::from_min_max(
                pos2(x0, inner.top()),
                pos2(x1.max(x0 + 1.0), inner.bottom()),
            );
            painter.rect_filled(loop_rect, 0.0, loop_fill);
        }

        let play_x = options
            .playhead_secs
            .filter(|_| duration > 0.0)
            .map(|t| time_to_x(t, duration, inner));

        // Sound-wave bars
        if let Some(peaks) = peaks.filter(|p| !p.is_empty()) {
            let n = peaks.peaks.len();
            let cols = (inner.width().floor() as usize).max(1);
            let mut shapes = Vec::with_capacity(cols);

            for col in 0..cols {
                let t0 = col as f32 / cols as f32;
                let t1 = (col + 1) as f32 / cols as f32;
                let i0 = (t0 * n as f32).floor() as usize;
                let i1 = ((t1 * n as f32).ceil() as usize).min(n).max(i0 + 1);

                let mut min_v = 0.0f32;
                let mut max_v = 0.0f32;
                for i in i0..i1 {
                    let (a, b) = peaks.peaks[i];
                    min_v = min_v.min(a);
                    max_v = max_v.max(b);
                }

                let x = inner.left() + col as f32 + 0.5;
                let y1 = mid_y - max_v * half_h;
                let y2 = mid_y - min_v * half_h;

                let in_loop = match (loop_x0, loop_x1) {
                    (Some(a), Some(b)) => x >= a && x <= b,
                    _ => true,
                };
                let played = play_x.map(|px| x <= px).unwrap_or(false);

                let color = if !in_loop {
                    wave_outside_loop
                } else if played {
                    wave_played
                } else {
                    wave_color
                };

                shapes.push(Shape::line_segment(
                    [pos2(x, y1), pos2(x, y2)],
                    Stroke::new(1.0, color),
                ));
            }
            painter.extend(shapes);
        } else if options.loading {
            // Lightweight loading bars
            let t = ui.input(|i| i.time) as f32;
            let muted =
                Color32::from_rgba_unmultiplied(wave_color.r(), wave_color.g(), wave_color.b(), 80);
            for i in 0..24 {
                let phase = (t * 3.0 + i as f32 * 0.35).sin().abs();
                let x = inner.left() + (i as f32 + 0.5) / 24.0 * inner.width();
                let h = half_h * (0.15 + 0.55 * phase);
                painter.line_segment(
                    [pos2(x, mid_y - h), pos2(x, mid_y + h)],
                    Stroke::new(2.0, muted),
                );
            }
        } else {
            let muted =
                Color32::from_rgba_unmultiplied(wave_color.r(), wave_color.g(), wave_color.b(), 50);
            for i in 0..12 {
                let x = inner.left() + (i as f32 + 0.5) / 12.0 * inner.width();
                let h = half_h * (0.12 + 0.08 * ((i % 3) as f32));
                painter.line_segment(
                    [pos2(x, mid_y - h), pos2(x, mid_y + h)],
                    Stroke::new(1.5, muted),
                );
            }
        }

        // Loop markers + circled handles + A/B labels
        if let (Some(x0), Some(x1)) = (loop_x0, loop_x1) {
            painter.line_segment(
                [pos2(x0, inner.top()), pos2(x0, inner.bottom())],
                Stroke::new(2.0, loop_stroke),
            );
            painter.line_segment(
                [pos2(x1, inner.top()), pos2(x1, inner.bottom())],
                Stroke::new(2.0, loop_stroke),
            );

            draw_loop_handle(&painter, pos2(x0, inner.top() + 8.0), loop_stroke, true);
            draw_loop_handle(&painter, pos2(x1, inner.top() + 8.0), loop_stroke, false);

            let font = TextStyle::Small.resolve(ui.style());
            painter.text(
                pos2(x0 + 8.0, inner.top() + 2.0),
                egui::Align2::LEFT_TOP,
                "A",
                font.clone(),
                loop_stroke,
            );
            painter.text(
                pos2(x1 - 8.0, inner.top() + 2.0),
                egui::Align2::RIGHT_TOP,
                "B",
                font,
                loop_stroke,
            );

            // Loop length label under region
            if duration > 0.0 {
                let mid_x = (x0 + x1) * 0.5;
                let len = (loop_end - loop_start).max(0.0);
                painter.text(
                    pos2(mid_x, inner.bottom() - 2.0),
                    egui::Align2::CENTER_BOTTOM,
                    format!("LOOP {len:.2}s"),
                    FontId::proportional(10.0),
                    loop_stroke,
                );
            }
        }

        // Playhead
        if let Some(px) = play_x {
            painter.line_segment(
                [pos2(px, inner.top()), pos2(px, inner.bottom())],
                Stroke::new(1.5, playhead_color),
            );
            let tip = pos2(px, inner.top());
            painter.add(Shape::convex_polygon(
                vec![
                    pos2(tip.x - 5.0, tip.y),
                    pos2(tip.x + 5.0, tip.y),
                    pos2(tip.x, tip.y + 7.0),
                ],
                playhead_color,
                Stroke::NONE,
            ));
        }

        // Interaction — never emit hard Seek on every dragged frame (that spams the audio device).
        let mut action = WaveformAction::None;
        let drag_id = id.with("drag_target");
        let mut drag_target = ui.ctx().data_mut(|d| {
            d.get_temp::<DragTarget>(drag_id)
                .unwrap_or(DragTarget::None)
        });

        if response.drag_started() {
            if let Some(pos) = response.interact_pointer_pos() {
                drag_target = pick_drag_target(
                    pos,
                    loop_x0,
                    loop_x1,
                    options.interactive_loop,
                    options.interactive_seek,
                );
            }
        }

        let pointer_time = response.interact_pointer_pos().map(|pos| {
            if duration > 0.0 {
                x_to_time(pos.x, duration, inner)
            } else {
                0.0
            }
        });

        // Commit once on release or click; scrub (preview) only while dragging.
        if let Some(t) = pointer_time {
            let target = if response.drag_started()
                || response.dragged()
                || response.drag_stopped()
                || response.clicked()
            {
                if drag_target == DragTarget::None {
                    pick_drag_target(
                        response.interact_pointer_pos().unwrap(),
                        loop_x0,
                        loop_x1,
                        options.interactive_loop,
                        options.interactive_seek,
                    )
                } else {
                    drag_target
                }
            } else {
                drag_target
            };

            let commit = response.clicked() || response.drag_stopped();
            let scrubbing = response.dragged() || response.drag_started();

            match target {
                DragTarget::LoopStart if options.interactive_loop && (scrubbing || commit) => {
                    action = WaveformAction::LoopStart(t);
                    ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                }
                DragTarget::LoopEnd if options.interactive_loop && (scrubbing || commit) => {
                    action = WaveformAction::LoopEnd(t);
                    ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                }
                DragTarget::Seek if options.interactive_seek => {
                    if commit {
                        action = WaveformAction::Seek(t);
                    } else if scrubbing {
                        action = WaveformAction::Scrub(t);
                    }
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                _ => {
                    // Click on empty wave without an active drag target → seek
                    if options.interactive_seek && response.clicked() {
                        action = WaveformAction::Seek(t);
                    }
                }
            }
        }

        if response.drag_stopped() {
            drag_target = DragTarget::None;
        }

        if let Some(pos) = response.hover_pos() {
            match pick_drag_target(
                pos,
                loop_x0,
                loop_x1,
                options.interactive_loop,
                options.interactive_seek,
            ) {
                DragTarget::LoopStart | DragTarget::LoopEnd => {
                    ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                }
                DragTarget::Seek => {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                DragTarget::None => {}
            }
        }

        ui.ctx().data_mut(|d| d.insert_temp(drag_id, drag_target));

        (response, action)
    }
}

fn draw_loop_handle(painter: &egui::Painter, center: Pos2, color: Color32, is_start: bool) {
    painter.circle_filled(center, 7.0, Color32::from_rgb(24, 24, 26));
    painter.circle_stroke(center, 7.0, Stroke::new(2.0, color));
    painter.circle_filled(center, 3.0, color);
    let notch_x = if is_start {
        center.x + 4.0
    } else {
        center.x - 4.0
    };
    painter.line_segment(
        [pos2(center.x, center.y), pos2(notch_x, center.y)],
        Stroke::new(1.4, color),
    );
}

fn time_to_x(time: f32, duration: f32, inner: Rect) -> f32 {
    if duration <= 0.0 {
        return inner.left();
    }
    let t = (time / duration).clamp(0.0, 1.0);
    inner.left() + t * inner.width()
}

fn x_to_time(x: f32, duration: f32, inner: Rect) -> f32 {
    if duration <= 0.0 || inner.width() <= 0.0 {
        return 0.0;
    }
    let t = ((x - inner.left()) / inner.width()).clamp(0.0, 1.0);
    t * duration
}

fn pick_drag_target(
    pos: Pos2,
    loop_start_x: Option<f32>,
    loop_end_x: Option<f32>,
    interactive_loop: bool,
    interactive_seek: bool,
) -> DragTarget {
    const HIT: f32 = 12.0;
    if interactive_loop {
        if let Some(x0) = loop_start_x {
            if (pos.x - x0).abs() <= HIT {
                return DragTarget::LoopStart;
            }
        }
        if let Some(x1) = loop_end_x {
            if (pos.x - x1).abs() <= HIT {
                return DragTarget::LoopEnd;
            }
        }
    }
    if interactive_seek {
        DragTarget::Seek
    } else {
        DragTarget::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hound::{SampleFormat, WavSpec, WavWriter};
    use std::f32::consts::PI;

    fn write_sine_wav(path: &Path, secs: f32, freq: f32) {
        let spec = WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(path, spec).unwrap();
        let n = (secs * 44100.0) as usize;
        for i in 0..n {
            let t = i as f32 / 44100.0;
            let s = (2.0 * PI * freq * t).sin();
            writer
                .write_sample((s * 0.5 * i16::MAX as f32) as i16)
                .unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn peaks_from_sine_are_nonempty() {
        let path = std::env::temp_dir().join("waveform_peaks_test.wav");
        write_sine_wav(&path, 0.25, 440.0);
        let peaks = WaveformPeaks::from_wav_path(&path, 128).unwrap();
        assert!(!peaks.is_empty());
        assert!(peaks.duration_secs > 0.2);
        assert!(peaks.peaks.len() >= 32);
        let max_amp = peaks
            .peaks
            .iter()
            .map(|(_, mx)| mx.abs())
            .fold(0.0f32, f32::max);
        assert!(max_amp > 0.1, "expected audible peak amplitude");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn sparse_peaks_are_fast_enough_for_large_wav() {
        let path = std::env::temp_dir().join("waveform_large_test.wav");
        // ~10s mono 16-bit — full scan would still be fine, but sparse must succeed
        write_sine_wav(&path, 10.0, 220.0);
        let t0 = std::time::Instant::now();
        let peaks = WaveformPeaks::from_wav_path(&path, 1024).unwrap();
        let ms = t0.elapsed().as_millis();
        assert!(!peaks.is_empty());
        assert!(ms < 500, "sparse peak extract too slow: {ms}ms");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn real_temp_convert_wav_if_present() {
        // Optional: exercise a real vgmstream convert product left by the app (~55MB / 5min).
        let dir = std::env::temp_dir();
        let Some(entry) = std::fs::read_dir(&dir).ok().and_then(|rd| {
            rd.filter_map(|e| e.ok()).find(|e| {
                let n = e.file_name().to_string_lossy().into_owned();
                n.starts_with("temp_convert_") && n.ends_with(".wav")
            })
        }) else {
            return;
        };
        let path = entry.path();
        let len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if len < 1_000_000 {
            return;
        }
        let t0 = std::time::Instant::now();
        let peaks = WaveformPeaks::from_wav_path(&path, 1024).expect("peaks from real temp wav");
        let ms = t0.elapsed().as_millis();
        println!(
            "real temp wav {} bytes -> {} bins {:.2}s in {}ms",
            len,
            peaks.peaks.len(),
            peaks.duration_secs,
            ms
        );
        assert!(!peaks.is_empty());
        assert!(peaks.duration_secs > 1.0);
        assert!(ms < 5000, "real large wav peak extract too slow: {ms}ms");
    }
}
