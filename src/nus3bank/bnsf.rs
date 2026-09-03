//! Convert PCM WAV to Namco BNSF/IS14 (G.722.1 Annex C) for EXVS2 character SE.
//!
//! **NUS3BANK only.** Call this from `Nus3bankReplacer::apply_to_file`.
//! `.nus3audio` payloads stay RIFF/WAVE; never run this encoder on that path.
//!
//! Vanilla unit SE banks store PACK payloads as BNSF+IS14, 48 kHz, mono,
//! 120-byte frames / 640 samples, 48 kbps, no encryption (`sfmt.flags = 0`).
//! The editor keeps WAV in memory for preview; this module is used on NUS3BANK save.

use super::error::Nus3bankError;
use hound::{SampleFormat, WavReader};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

const SAMPLE_RATE: u32 = 48_000;
const FRAME_SAMPLES: usize = 640;
const BYTES_PER_FRAME: usize = 120;
const BITRATE: &str = "48000";
const BANDWIDTH: &str = "14000";

pub fn is_bnsf_is14(data: &[u8]) -> bool {
    data.len() >= 12 && data.starts_with(b"BNSF") && &data[8..12] == b"IS14"
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BnsfClock {
    pub n_samples: i32,
    pub loop_start: i32,
    pub loop_end: i32,
    pub loop_flag: i32,
}

/// True when the BNSF container has a `loop` chunk (vanilla looping SE).
pub fn bnsf_has_loop_chunk(data: &[u8]) -> bool {
    parse_bnsf_clock(data).is_some_and(|c| c.loop_flag != 0)
}

fn be_u32(data: &[u8], at: usize) -> Option<u32> {
    data.get(at..at + 4)
        .and_then(|b| b.try_into().ok())
        .map(u32::from_be_bytes)
}

/// Read sfmt sample count and optional loop chunk from a BNSF/IS14 payload.
pub fn parse_bnsf_clock(data: &[u8]) -> Option<BnsfClock> {
    if !is_bnsf_is14(data) {
        return None;
    }
    let mut p = 12usize;
    let mut n_samples: Option<i32> = None;
    let mut loop_start = 0i32;
    let mut loop_end = 0i32;
    let mut loop_flag = 0i32;
    while p + 8 <= data.len() {
        let chunk = data.get(p..p + 4)?;
        let size = be_u32(data, p + 4)? as usize;
        let body = p + 8;
        if chunk == b"sfmt" && size >= 20 {
            n_samples = be_u32(data, body + 8).map(|v| v as i32);
        } else if chunk == b"loop" && size >= 8 {
            loop_start = be_u32(data, body).map(|v| v as i32)?;
            loop_end = be_u32(data, body + 4).map(|v| v as i32)?;
            loop_flag = 1;
        } else if chunk == b"sdat" {
            break;
        }
        p = p.saturating_add(8).saturating_add(size);
    }
    Some(BnsfClock {
        n_samples: n_samples?,
        loop_start,
        loop_end,
        loop_flag,
    })
}

fn is_riff_wav(data: &[u8]) -> bool {
    data.len() >= 12 && data.starts_with(b"RIFF") && data.get(8..12) == Some(&b"WAVE"[..])
}

/// Pass through existing BNSF/IS14. Encode RIFF/WAVE. Reject anything else.
pub fn ensure_bnsf_is14(data: Vec<u8>) -> Result<Vec<u8>, Nus3bankError> {
    if is_bnsf_is14(&data) {
        return Ok(data);
    }
    if is_riff_wav(&data) {
        return match encode_nonempty_wav_payload(data.clone())? {
            Some(bnsf) => Ok(bnsf),
            None => Ok(data),
        };
    }
    Err(Nus3bankError::InvalidFormat {
        reason: "NUS3BANK audio must be BNSF/IS14 or WAV (got neither)".to_string(),
    })
}

/// Encode an already-written PACK WAV if it has samples. Empty WAV headers are left as-is.
pub fn encode_nonempty_wav_payload(data: Vec<u8>) -> Result<Option<Vec<u8>>, Nus3bankError> {
    if is_bnsf_is14(&data) || !is_riff_wav(&data) {
        return Ok(None);
    }
    let (pcm, rate, _) = wav_to_mono_i16(&data)?;
    if pcm.is_empty() || rate == 0 {
        return Ok(None);
    }
    Ok(Some(wav_to_bnsf_is14(&data)?))
}

pub fn wav_to_bnsf_is14(wav: &[u8]) -> Result<Vec<u8>, Nus3bankError> {
    let (pcm, src_rate, smpl) = wav_to_mono_i16(wav)?;
    if src_rate == 0 {
        return Err(Nus3bankError::InvalidFormat {
            reason: "WAV sample rate is 0".to_string(),
        });
    }
    let pcm = resample_linear(&pcm, src_rate, SAMPLE_RATE);
    let n_samples = pcm.len() as u32;
    if n_samples == 0 {
        return Err(Nus3bankError::InvalidFormat {
            reason: "WAV has no samples".to_string(),
        });
    }
    let loop_info = smpl.map(|(start, end)| {
        let scale = SAMPLE_RATE as u64;
        let start = ((start as u64) * scale / src_rate as u64) as u32;
        let end = ((end as u64) * scale / src_rate as u64) as u32;
        let start = start.min(n_samples.saturating_sub(1));
        let end = end.clamp(start + 1, n_samples);
        (align_down(start, FRAME_SAMPLES as u32), end)
    });

    let mut padded = pcm;
    while padded.len() % FRAME_SAMPLES != 0 {
        padded.push(0);
    }

    let sdat = encode_is14_frames(&padded)?;
    if sdat.len() % BYTES_PER_FRAME != 0 {
        return Err(Nus3bankError::InvalidFormat {
            reason: format!(
                "IS14 encoder returned {} bytes (not a multiple of {})",
                sdat.len(),
                BYTES_PER_FRAME
            ),
        });
    }

    Ok(wrap_bnsf(&sdat, n_samples, loop_info))
}

fn align_down(value: u32, align: u32) -> u32 {
    if align == 0 {
        value
    } else {
        value - (value % align)
    }
}

fn wav_to_mono_i16(wav: &[u8]) -> Result<(Vec<i16>, u32, Option<(u32, u32)>), Nus3bankError> {
    let reader = WavReader::new(Cursor::new(wav)).map_err(|e| Nus3bankError::InvalidFormat {
        reason: format!("Failed to parse WAV: {e}"),
    })?;
    let spec = reader.spec();
    let channels = spec.channels.max(1) as usize;
    let rate = spec.sample_rate;
    let interleaved: Vec<i16> = match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Int, 16) => reader
            .into_samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| Nus3bankError::InvalidFormat {
                reason: format!("Failed to read WAV samples: {e}"),
            })?,
        (SampleFormat::Int, 8) => reader
            .into_samples::<i8>()
            .map(|s| s.map(|v| (v as i16) << 8))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| Nus3bankError::InvalidFormat {
                reason: format!("Failed to read WAV samples: {e}"),
            })?,
        (SampleFormat::Float, 32) => reader
            .into_samples::<f32>()
            .map(|s| s.map(|v| (v.clamp(-1.0, 1.0) * 32767.0).round() as i16))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| Nus3bankError::InvalidFormat {
                reason: format!("Failed to read WAV samples: {e}"),
            })?,
        (fmt, bits) => {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("Unsupported WAV format {fmt:?} {bits}-bit"),
            });
        }
    };
    if interleaved.len() % channels != 0 {
        return Err(Nus3bankError::InvalidFormat {
            reason: "WAV sample count is not divisible by channel count".to_string(),
        });
    }
    let mono = if channels == 1 {
        interleaved
    } else {
        interleaved
            .chunks_exact(channels)
            .map(|frame| {
                let sum: i32 = frame.iter().map(|s| *s as i32).sum();
                (sum / channels as i32) as i16
            })
            .collect()
    };
    Ok((mono, rate, parse_smpl_loop(wav)))
}

fn parse_smpl_loop(wav: &[u8]) -> Option<(u32, u32)> {
    if wav.len() < 12 || !wav.starts_with(b"RIFF") || &wav[8..12] != b"WAVE" {
        return None;
    }
    let mut p = 12usize;
    while p + 8 <= wav.len() {
        let chunk = &wav[p..p + 4];
        let size = u32::from_le_bytes(wav[p + 4..p + 8].try_into().ok()?) as usize;
        let body = p + 8;
        if chunk == b"smpl" && body + 36 + 24 <= wav.len() && size >= 60 {
            let nloops = u32::from_le_bytes(wav[body + 28..body + 32].try_into().ok()?);
            if nloops == 0 {
                return None;
            }
            let loop0 = body + 36;
            let start = u32::from_le_bytes(wav[loop0 + 8..loop0 + 12].try_into().ok()?);
            let end = u32::from_le_bytes(wav[loop0 + 12..loop0 + 16].try_into().ok()?);
            if end > start {
                return Some((start, end));
            }
            return None;
        }
        p = body + size;
        if size % 2 == 1 {
            p += 1;
        }
    }
    None
}

fn resample_linear(samples: &[i16], from: u32, to: u32) -> Vec<i16> {
    if samples.is_empty() || from == 0 || from == to {
        return samples.to_vec();
    }
    let out_len = ((samples.len() as u64) * (to as u64) / (from as u64)).max(1) as usize;
    let last = samples.len() - 1;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let pos = (i as f64) * (from as f64) / (to as f64);
        let i0 = (pos.floor() as usize).min(last);
        let i1 = (i0 + 1).min(last);
        let frac = pos - (i0 as f64);
        let a = samples[i0] as f64;
        let b = samples[i1] as f64;
        out.push((a + (b - a) * frac).round() as i16);
    }
    out
}

fn encode_is14_frames(pcm: &[i16]) -> Result<Vec<u8>, Nus3bankError> {
    let encode_exe = find_encode_exe()?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = std::env::temp_dir();
    let pcm_path = tmp.join(format!("exvs2_is14_{nonce}.pcm"));
    let out_path = tmp.join(format!("exvs2_is14_{nonce}.is14"));

    let mut bytes = Vec::with_capacity(pcm.len() * 2);
    for s in pcm {
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    fs::write(&pcm_path, &bytes).map_err(|e| Nus3bankError::InvalidFormat {
        reason: format!("Failed to write temp PCM for IS14 encode: {e}"),
    })?;

    let mut command = Command::new(&encode_exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use winapi::um::winbase::CREATE_NO_WINDOW;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let output = command
        .args([
            "0",
            &pcm_path.to_string_lossy(),
            &out_path.to_string_lossy(),
            BITRATE,
            BANDWIDTH,
        ])
        .output()
        .map_err(|e| {
            let _ = fs::remove_file(&pcm_path);
            Nus3bankError::InvalidFormat {
                reason: format!("Failed to run IS14 encoder {}: {e}", encode_exe.display()),
            }
        })?;

    let result = if output.status.success() {
        fs::read(&out_path).map_err(|e| Nus3bankError::InvalidFormat {
            reason: format!("Failed to read IS14 encoder output: {e}"),
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(Nus3bankError::InvalidFormat {
            reason: format!("IS14 encoder failed: {stderr} {stdout}"),
        })
    };

    let _ = fs::remove_file(&pcm_path);
    let _ = fs::remove_file(&out_path);
    result
}

fn wrap_bnsf(sdat: &[u8], n_samples: u32, loop_info: Option<(u32, u32)>) -> Vec<u8> {
    let data_size = sdat.len() as u32;
    let extra_loop = if loop_info.is_some() { 16 } else { 0 };
    let size_field = data_size + 40 + extra_loop;
    let mut out = Vec::with_capacity(sdat.len() + 48 + extra_loop as usize);
    out.extend_from_slice(b"BNSF");
    out.extend_from_slice(&size_field.to_be_bytes());
    out.extend_from_slice(b"IS14");
    out.extend_from_slice(b"sfmt");
    out.extend_from_slice(&0x14u32.to_be_bytes());
    out.extend_from_slice(&1u32.to_be_bytes());
    out.extend_from_slice(&SAMPLE_RATE.to_be_bytes());
    out.extend_from_slice(&n_samples.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&(BYTES_PER_FRAME as u16).to_be_bytes());
    out.extend_from_slice(&(FRAME_SAMPLES as u16).to_be_bytes());
    if let Some((start, end)) = loop_info {
        let end = end.max(start + 1);
        out.extend_from_slice(b"loop");
        out.extend_from_slice(&8u32.to_be_bytes());
        out.extend_from_slice(&start.to_be_bytes());
        out.extend_from_slice(&(end.saturating_sub(1)).to_be_bytes());
    }
    out.extend_from_slice(b"sdat");
    out.extend_from_slice(&data_size.to_be_bytes());
    out.extend_from_slice(sdat);
    out
}

fn find_encode_exe() -> Result<PathBuf, Nus3bankError> {
    let name = "encode.exe";
    let mut candidates = vec![PathBuf::from("tools").join(name)];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("tools").join(name));
        }
    }
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tools").join(name));
    if let Some(found) = candidates.into_iter().find(|p| p.is_file()) {
        return Ok(found);
    }
    Err(Nus3bankError::InvalidFormat {
        reason: format!(
            "IS14 encoder not found (looked for tools/{}). Cannot write BNSF.",
            name
        ),
    })
}

pub fn encode_exe_available() -> bool {
    find_encode_exe().is_ok()
}

#[cfg(test)]
pub fn write_minimal_wav_sine(path: &Path, sample_rate: u32, n_samples: u32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..n_samples {
        let t = i as f32 / sample_rate as f32;
        let s = (t * 440.0 * std::f32::consts::TAU).sin() * 16000.0;
        writer.write_sample(s as i16).unwrap();
    }
    writer.finalize().unwrap();
}
