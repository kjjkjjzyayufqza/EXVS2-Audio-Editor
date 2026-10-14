//! Shrink NUS3AUDIO track payloads to high-quality VBR MP3.
//!
//! NUS3BANK is intentionally untouched: those saves go through the BNSF encoder.

use super::audio_file_info::AudioFileInfo;
use super::nus3audio_file_utils::Nus3audioFileUtils;
use super::replace_utils::ReplaceUtils;
use hound::{SampleFormat, WavReader};
use nus3audio::Nus3audioFile;
use std::io::Cursor;
use std::path::Path;
use std::time::Instant;

#[cfg(not(target_arch = "wasm32"))]
use mp3lame_encoder::{Bitrate, Builder, DualPcm, Encoder, FlushGap, MonoPcm, Quality, VbrMode};

/// Result of one shrink pass. `updated` entries replace the matching UI rows.
pub struct CompressReport {
    pub compressed: usize,
    pub skipped: usize,
    pub failed: usize,
    pub errors: Vec<String>,
    pub updated: Vec<AudioFileInfo>,
}

/// Compress `tracks` inside a `.nus3audio` container. Each track is decoded at most once.
pub fn compress_tracks(nus3_path: &str, tracks: &[AudioFileInfo]) -> CompressReport {
    let mut report = CompressReport {
        compressed: 0,
        skipped: 0,
        failed: 0,
        errors: Vec::new(),
        updated: Vec::new(),
    };

    if !nus3_path.to_ascii_lowercase().ends_with(".nus3audio") {
        report.failed = tracks.len().max(1);
        report
            .errors
            .push("MP3 shrink applies only to .nus3audio".to_string());
        return report;
    }

    let opened = Nus3audioFile::open(nus3_path).ok();
    let started = Instant::now();

    for info in tracks {
        if info.is_nus3bank {
            report.skipped += 1;
            continue;
        }

        let source = match track_source_bytes(info, opened.as_ref()) {
            Ok(bytes) => bytes,
            Err(err) => {
                report.failed += 1;
                report.errors.push(format!("{}: {err}", info.name));
                continue;
            }
        };

        match shrink_one(info, &source) {
            Ok(Some(updated)) => {
                println!(
                    "[PERF] MP3 shrink {}: {} -> {} bytes",
                    info.name, source.len(), updated.size
                );
                report.updated.push(updated);
                report.compressed += 1;
            }
            Ok(None) => {
                report.skipped += 1;
            }
            Err(err) => {
                report.failed += 1;
                report.errors.push(format!("{}: {err}", info.name));
            }
        }
    }

    println!(
        "[PERF] MP3 shrink pass: {}ms compressed={} skipped={} failed={}",
        started.elapsed().as_millis(),
        report.compressed,
        report.skipped,
        report.failed
    );
    report
}

fn track_source_bytes(
    info: &AudioFileInfo,
    opened: Option<&Nus3audioFile>,
) -> Result<Vec<u8>, String> {
    if let Some(bytes) = ReplaceUtils::get_replacement_data_unified(info) {
        return Ok(bytes);
    }
    if let Some(bytes) = Nus3audioFileUtils::get_pending_added_data(&info.name, &info.id) {
        return Ok(bytes);
    }
    let file = opened.ok_or_else(|| format!("track '{}' is not in the open file", info.name))?;
    file.files
        .iter()
        .find(|entry| entry.name == info.name && entry.id.to_string() == info.id)
        .map(|entry| entry.data.clone())
        .ok_or_else(|| format!("track '{}' was not found", info.name))
}

/// `Ok(None)` means the track was left unchanged (already MP3, empty, or not smaller).
fn shrink_one(info: &AudioFileInfo, source: &[u8]) -> Result<Option<AudioFileInfo>, String> {
    if source.len() < 64 || is_mp3(source) || is_empty_wav(source) {
        return Ok(None);
    }

    let wav = if ReplaceUtils::is_standard_pcm16_wav(source) {
        source.to_vec()
    } else {
        ReplaceUtils::convert_audio_bytes_to_pcm_wav(source)?
    };
    let (sample_rate, channels, pcm) = read_pcm16_wav(&wav)?;
    if pcm.is_empty() || sample_rate == 0 || channels == 0 {
        return Ok(None);
    }

    let mp3 = encode_pcm16(&pcm, sample_rate, channels)?;
    if mp3.len() >= source.len() {
        return Ok(None);
    }

    let filename = mp3_filename(&info.filename);
    let updated = if Nus3audioFileUtils::has_pending_add(&info.name, &info.id) {
        let updated = ReplaceUtils::store_replacement_bytes(
            info,
            mp3.clone(),
            filename,
            "MP3".to_string(),
        )?;
        Nus3audioFileUtils::replace_pending_add_data(&info.name, &info.id, mp3);
        updated
    } else {
        ReplaceUtils::store_replacement_bytes(info, mp3, filename, "MP3".to_string())?
    };
    Ok(Some(updated))
}

fn mp3_filename(current: &str) -> String {
    let stem = Path::new(current)
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("audio");
    format!("{stem}.mp3")
}

fn is_mp3(data: &[u8]) -> bool {
    if data.len() >= 3 && &data[0..3] == b"ID3" {
        return true;
    }
    data.len() >= 2 && data[0] == 0xFF && (data[1] & 0xE0) == 0xE0
}

fn is_empty_wav(data: &[u8]) -> bool {
    if data.len() < 44 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return false;
    }
    let mut offset = 12usize;
    while offset + 8 <= data.len() {
        let id = &data[offset..offset + 4];
        let size = u32::from_le_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as usize;
        if id == b"data" {
            return size == 0;
        }
        offset = offset.saturating_add(8).saturating_add(size);
        if size % 2 != 0 {
            offset = offset.saturating_add(1);
        }
    }
    false
}

fn read_pcm16_wav(data: &[u8]) -> Result<(u32, u16, Vec<i16>), String> {
    let mut reader = WavReader::new(Cursor::new(data)).map_err(|err| format!("WAV read: {err}"))?;
    let spec = reader.spec();
    let samples = match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Int, 16) => reader
            .samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("PCM read: {err}"))?,
        (SampleFormat::Float, 32) => reader
            .samples::<f32>()
            .map(|sample| {
                sample.map(|value| (value.clamp(-1.0, 1.0) * 32767.0).round() as i16)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("PCM read: {err}"))?,
        _ => {
            return Err(format!(
                "unsupported WAV {:?} {}-bit",
                spec.sample_format, spec.bits_per_sample
            ));
        }
    };
    Ok((spec.sample_rate, spec.channels, samples))
}

#[cfg(not(target_arch = "wasm32"))]
fn encode_pcm16(samples: &[i16], sample_rate: u32, channels: u16) -> Result<Vec<u8>, String> {
    if samples.is_empty() || channels == 0 {
        return Err("no PCM samples to encode".to_string());
    }

    let out_channels: u8 = if channels == 1 { 1 } else { 2 };
    let mut encoder = Builder::new()
        .ok_or_else(|| "failed to create the MP3 encoder".to_string())?
        .with_num_channels(out_channels)
        .map_err(|err| format!("MP3 channels: {err}"))?
        .with_sample_rate(sample_rate)
        .map_err(|err| format!("MP3 sample rate {sample_rate}: {err}"))?
        .with_brate(Bitrate::Kbps320)
        .map_err(|err| format!("MP3 bitrate: {err}"))?
        .with_quality(Quality::VeryNice)
        .map_err(|err| format!("MP3 quality: {err}"))?
        .with_vbr_mode(VbrMode::Mtrh)
        .map_err(|err| format!("MP3 VBR: {err}"))?
        .with_vbr_quality(Quality::Best)
        .map_err(|err| format!("MP3 VBR quality: {err}"))?
        .with_to_write_vbr_tag(false)
        .map_err(|err| format!("MP3 tag: {err}"))?
        .build()
        .map_err(|err| format!("MP3 init: {err}"))?;

    let mut out = Vec::new();
    // One MPEG frame is 1152 samples. Encode in batches so the scratch buffer stays small.
    const FRAMES: usize = 1152 * 64;
    if out_channels == 1 {
        let mut offset = 0usize;
        while offset < samples.len() {
            let end = (offset + FRAMES).min(samples.len());
            let chunk = &samples[offset..end];
            write_encoded(&mut encoder, MonoPcm(chunk), chunk.len(), &mut out)?;
            offset = end;
        }
    } else {
        let owned;
        let interleaved: &[i16] = if channels == 2 {
            let even = samples.len() - (samples.len() % 2);
            &samples[..even]
        } else {
            owned = downmix_stereo(samples, channels as usize);
            owned.as_slice()
        };
        let (left, right) = split_stereo(interleaved);
        let frames = left.len();
        let mut frame = 0usize;
        while frame < frames {
            let count = FRAMES.min(frames - frame);
            let end = frame + count;
            write_encoded(
                &mut encoder,
                DualPcm {
                    left: &left[frame..end],
                    right: &right[frame..end],
                },
                count,
                &mut out,
            )?;
            frame = end;
        }
    }
    write_flush(&mut encoder, &mut out)?;
    if out.is_empty() {
        return Err("MP3 encoder produced no bytes".to_string());
    }
    Ok(out)
}

#[cfg(target_arch = "wasm32")]
fn encode_pcm16(_samples: &[i16], _sample_rate: u32, _channels: u16) -> Result<Vec<u8>, String> {
    Err("MP3 shrink is available in the native build only".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn split_stereo(interleaved: &[i16]) -> (Vec<i16>, Vec<i16>) {
    let frames = interleaved.len() / 2;
    let mut left = Vec::with_capacity(frames);
    let mut right = Vec::with_capacity(frames);
    let mut index = 0usize;
    while index + 1 < interleaved.len() {
        left.push(interleaved[index]);
        right.push(interleaved[index + 1]);
        index += 2;
    }
    (left, right)
}

fn downmix_stereo(samples: &[i16], channels: usize) -> Vec<i16> {
    if channels == 0 {
        return Vec::new();
    }
    let frames = samples.len() / channels;
    let mut out = Vec::with_capacity(frames.saturating_mul(2));
    for frame in 0..frames {
        let base = frame * channels;
        let left = samples.get(base).copied().unwrap_or(0);
        let right = samples.get(base + 1).copied().unwrap_or(left);
        out.push(left);
        out.push(right);
    }
    out
}

#[cfg(not(target_arch = "wasm32"))]
fn write_encoded<I>(
    encoder: &mut Encoder,
    input: I,
    samples_per_channel: usize,
    out: &mut Vec<u8>,
) -> Result<(), String>
where
    I: mp3lame_encoder::EncoderInput,
{
    let need = mp3lame_encoder::max_required_buffer_size(samples_per_channel);
    out.reserve(need);
    let written = encoder
        .encode(input, out.spare_capacity_mut())
        .map_err(|err| format!("MP3 encode: {err}"))?;
    commit_spare(out, written)
}

#[cfg(not(target_arch = "wasm32"))]
fn write_flush(encoder: &mut Encoder, out: &mut Vec<u8>) -> Result<(), String> {
    let need = mp3lame_encoder::max_required_buffer_size(1152);
    out.reserve(need);
    let written = encoder
        .flush::<FlushGap>(out.spare_capacity_mut())
        .map_err(|err| format!("MP3 flush: {err}"))?;
    commit_spare(out, written)
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(unsafe_code)]
fn commit_spare(buf: &mut Vec<u8>, written: usize) -> Result<(), String> {
    let spare = buf.capacity().saturating_sub(buf.len());
    if written > spare {
        return Err(format!(
            "MP3 encoder wrote {written} bytes into {spare} bytes of spare capacity"
        ));
    }
    // SAFETY: `Encoder::encode` / `flush` initialized the first `written` spare bytes.
    unsafe {
        buf.set_len(buf.len() + written);
    }
    Ok(())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::encode_pcm16;

    #[test]
    fn vbr_mp3_is_smaller_than_pcm_and_starts_with_a_frame() {
        let rate = 48_000u32;
        let pcm: Vec<i16> = (0..rate as usize)
            .map(|i| {
                let t = i as f32 / rate as f32;
                (t * 440.0 * std::f32::consts::TAU).sin() * 12_000.0
            })
            .map(|sample| sample.round() as i16)
            .collect();
        let mp3 = encode_pcm16(&pcm, rate, 1).expect("encode");
        assert!(mp3.len() > 64, "mp3 len {}", mp3.len());
        assert!(
            mp3.len() < pcm.len() * 2,
            "mp3 {} not smaller than pcm {}",
            mp3.len(),
            pcm.len() * 2
        );
        assert_eq!(mp3[0], 0xFF, "expected an MPEG frame sync");
    }
}
