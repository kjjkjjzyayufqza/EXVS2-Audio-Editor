use base64::Engine as _;
use serde_json::{json, Value};

use super::structures::{AudioTrack, Nus3bankFile, RawSection, TocEntry, ToneMeta};

/// Options to control debug JSON output.
#[derive(Clone, Debug)]
pub struct DebugJsonOptions {
    /// Maximum number of bytes to include per payload preview (base64).
    pub max_preview_bytes: usize,
    /// Include previews for PACK section data.
    pub include_pack_preview: bool,
    /// Include previews for ToneMeta payload data.
    pub include_tone_payload_preview: bool,
    /// Include previews for unknown section payloads.
    pub include_unknown_section_preview: bool,
}

impl Default for DebugJsonOptions {
    fn default() -> Self {
        Self {
            max_preview_bytes: 4096,
            include_pack_preview: false,
            include_tone_payload_preview: false,
            include_unknown_section_preview: false,
        }
    }
}

fn magic_to_string(magic: &[u8; 4]) -> String {
    String::from_utf8_lossy(magic).to_string()
}

fn bytes_preview_base64(bytes: &[u8], max_bytes: usize) -> Value {
    let take_n = bytes.len().min(max_bytes);
    let truncated = take_n < bytes.len();
    let prefix = &bytes[..take_n];
    let b64 = base64::engine::general_purpose::STANDARD.encode(prefix);
    json!({
        "len": bytes.len(),
        "preview_len": take_n,
        "preview_base64": b64,
        "truncated": truncated,
    })
}

fn toc_entry_json(e: &TocEntry) -> Value {
    json!({
        "magic": magic_to_string(&e.magic),
        "size": e.size,
    })
}

fn raw_section_json(s: &RawSection, opt: &DebugJsonOptions) -> Value {
    let mut v = json!({
        "magic": magic_to_string(&s.magic),
        "size_from_toc": s.size,
        "data_len": s.data.len(),
    });

    if opt.include_unknown_section_preview {
        v["data_preview"] = bytes_preview_base64(&s.data, opt.max_preview_bytes);
    }

    v
}

fn audio_track_json(t: &AudioTrack) -> Value {
    json!({
        "index": t.index,
        "hex_id": t.hex_id,
        "numeric_id": t.numeric_id,
        "name": t.name,
        "pack_offset": t.pack_offset,
        "size": t.size,
        "metadata_size": t.metadata_size,
        "tone_index": t.tone_index,
        "audio_format": format!("{:?}", t.audio_format),
        "audio_data_len": t.audio_data.as_ref().map(|d| d.len()).unwrap_or(0),
    })
}

fn tone_meta_json(t: &ToneMeta, opt: &DebugJsonOptions) -> Value {
    let mut v = json!({
        "meta_prefix_len": t.meta_prefix.len(),
        "hash": t.hash,
        "unk1": t.unk1,
        "name": t.name,
        "reserved0": t.reserved0,
        "reserved8": t.reserved8,
        "offset": t.offset,
        "size": t.size,
        "param": t.param,
        "offsets": t.offsets,
        "unkvalues": t.unkvalues,
        "unkvalues_pair_order": format!("{:?}", t.unkvalues_pair_order),
        "unkending": t.unkending,
        "end": t.end,
        "meta_size": t.meta_size,
        "removed": t.removed,
        "payload_len": t.payload.len(),
    });

    if !t.meta_prefix.is_empty() {
        v["meta_prefix"] = bytes_preview_base64(&t.meta_prefix, opt.max_preview_bytes);
    }

    if opt.include_tone_payload_preview {
        v["payload_preview"] = bytes_preview_base64(&t.payload, opt.max_preview_bytes);
    }

    v
}

impl Nus3bankFile {
    /// Convert the parsed file into a JSON value for debugging/inspection.
    pub fn to_debug_json_value(&self, opt: &DebugJsonOptions) -> Value {
        let toc = self.toc.iter().map(toc_entry_json).collect::<Vec<_>>();

        let prop = self.prop.as_ref().map(|p| {
            json!({
                "project": p.project,
                "timestamp": p.timestamp,
                "unk1": p.unk1,
                "reserved_u16": p.reserved_u16,
                "unk2": p.unk2,
                "unk3": p.unk3,
                "layout": format!("{:?}", p.layout),
            })
        });

        let binf = self.binf.as_ref().map(|b| {
            json!({
                "reserved0": b.reserved0,
                "unk1": b.unk1,
                "name": b.name,
                "flag": b.flag,
            })
        });

        let grp = self.grp.as_ref().map(|g| json!({ "names": g.names }));

        let dton = self.dton.as_ref().map(|d| {
            json!({
                "tones": d.tones.iter().map(|td| json!({
                    "hash": td.hash,
                    "unk1": td.unk1,
                    "name": td.name,
                    "data_len": td.data.len(),
                    "data": td.data,
                })).collect::<Vec<_>>()
            })
        });

        let tone = json!({
            "count": self.tone.tones.len(),
            "tones": self.tone.tones.iter().map(|t| tone_meta_json(t, opt)).collect::<Vec<_>>(),
        });

        let mut pack = json!({
            "data_len": self.pack.data.len(),
        });
        if opt.include_pack_preview {
            pack["data_preview"] = bytes_preview_base64(&self.pack.data, opt.max_preview_bytes);
        }

        let junk = self.junk.as_ref().map(|j| {
            json!({
                "data_len": j.data.len(),
                "data_preview": bytes_preview_base64(&j.data, opt.max_preview_bytes),
            })
        });

        let unknown_sections = self
            .unknown_sections
            .iter()
            .map(|s| raw_section_json(s, opt))
            .collect::<Vec<_>>();

        let tracks = self.tracks.iter().map(audio_track_json).collect::<Vec<_>>();

        json!({
            "toc": toc,
            "sections": {
                "prop": prop,
                "binf": binf,
                "grp": grp,
                "dton": dton,
                "tone": tone,
                "junk": junk,
                "pack": pack,
                "unknown": unknown_sections,
            },
            "tracks": tracks,
        })
    }

    /// Convert the parsed file into a pretty-printed JSON string for debugging/inspection.
    pub fn to_debug_json_string(&self, opt: &DebugJsonOptions) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.to_debug_json_value(opt))
    }
}

/// Write debug JSON to disk.
pub fn write_debug_json_file<P: AsRef<std::path::Path>>(
    file: &Nus3bankFile,
    opt: &DebugJsonOptions,
    out_path: P,
) -> Result<(), super::error::Nus3bankError> {
    let s = file
        .to_debug_json_string(opt)
        .map_err(|e| super::error::Nus3bankError::Reconstruction {
            reason: format!("Failed to serialize debug JSON: {e}"),
        })?;
    std::fs::write(out_path, s)?;
    Ok(())
}

