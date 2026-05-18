use super::error::Nus3bankError;

/// A single BANKTOC entry: section magic and section data size (excluding the 8-byte section header).
#[derive(Clone, Debug)]
pub struct TocEntry {
    pub magic: [u8; 4],
    pub size: u32,
}

#[derive(Clone, Debug)]
pub struct RawSection {
    pub magic: [u8; 4],
    pub size: u32,
    pub data: Vec<u8>,
}

/// PROP section (C# `NusProp`)
#[derive(Clone, Debug)]
pub struct PropSection {
    pub project: String,
    pub timestamp: String,
    pub unk1: i32,
    /// 2 bytes skipped by C# (`d.Skip(2)`), but real files may not be zero.
    pub reserved_u16: u16,
    pub unk2: u16,
    pub unk3: u16,
    /// Controls how PROP is rebuilt to preserve original layout.
    pub layout: PropLayout,
    /// First payload u32 before the OB bitmask. Legacy layouts keep this as their first padding u32.
    pub leading_u32: u32,
    /// OB PROP presence mask at section offset +0x0C.
    pub presence_mask: u32,
    /// OB PROP version field. Major version must be 3 for bitmask PROP.
    pub version: u32,
    /// Optional OB PROP bit 5 u32 field.
    pub bit5_u32: Option<u32>,
    /// Optional OB PROP bit 6 u32 field.
    pub bit6_u32: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropLayout {
    /// Minimal PROP that ends after `project` (no `unk3`/`timestamp`).
    Minimal,
    /// Extended PROP with `unk3` and `timestamp` (C# `NusProp` behavior).
    Extended,
    /// OB engine PROP layout driven by a u32 presence mask.
    Bitmask,
}

/// BINF section (C# `NusBinf`)
#[derive(Clone, Debug)]
pub struct BinfSection {
    pub reserved0: i32,
    pub unk1: i32,
    pub name: String,
    pub flag: i32,
}

/// GRP section (C# `NusGrp`)
#[derive(Clone, Debug, Default)]
pub struct GrpSection {
    pub names: Vec<String>,
}

/// DTON section (C# `NusDton`)
#[derive(Clone, Debug, Default)]
pub struct DtonSection {
    pub tones: Vec<ToneDes>,
}

#[derive(Clone, Debug)]
pub struct ToneDes {
    pub hash: i32,
    pub unk1: i32,
    pub name: String,
    pub data: Vec<f32>, // Length varies; bounded by DTON entry size
    /// Exact descriptor bytes after the name/alignment header.
    pub raw_data: Vec<u8>,
    /// Descriptor presence words read until the first word with no high continuation bit.
    pub descriptor_words: Vec<u32>,
}

/// TONE section (C# `NUS_TONE`)
#[derive(Clone, Debug, Default)]
pub struct ToneSection {
    pub tones: Vec<ToneMeta>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnkvaluesPairOrder {
    IndexThenValue,
    ValueThenIndex,
}

#[derive(Clone, Debug)]
pub struct ToneMeta {
    /// Optional 8-byte prefix found in some BANKTOC variants before the normal ToneMeta fields.
    pub meta_prefix: Vec<u8>,
    /// Exact original TONE metadata block for runtime-compatible patching.
    pub raw_meta: Vec<u8>,
    /// Byte position of the PACK offset field inside `raw_meta`.
    pub pack_offset_field_pos: Option<usize>,
    /// Byte position of the PACK size field inside `raw_meta`.
    pub pack_size_field_pos: Option<usize>,
    pub hash: i32,
    pub unk1: i32,
    pub name: String,
    pub reserved0: i32,
    pub reserved8: i32,
    pub offset: i32,
    pub size: i32,
    pub param: [f32; 12],
    pub offsets: Vec<i32>,
    pub unkvalues: Vec<f32>,
    pub unkvalues_pair_order: UnkvaluesPairOrder,
    pub unkending: Vec<i32>,
    pub end: Vec<i32>,
    pub payload: Vec<u8>,
    pub meta_size: u32,
    pub removed: bool,
}

#[derive(Clone, Debug)]
pub struct JunkSection {
    /// JUNK payload bytes (size varies across files).
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct PackSection {
    /// PACK payload bytes (excluding the section header). This is optional because we rebuild from `ToneMeta.payload`.
    pub data: Vec<u8>,
}

/// Supported audio formats (WAV-focused UI, but payload bytes may be non-WAV).
#[derive(Clone, Debug, PartialEq)]
pub enum AudioFormat {
    Wav,
    Unknown,
}

/// UI-facing audio track view derived from `ToneMeta`.
#[derive(Clone, Debug)]
pub struct AudioTrack {
    /// Sequential index in the `TONE` list (0-based)
    pub index: usize,
    /// Hex string representation of the track index ("0x0", "0xb2", etc.)
    pub hex_id: String,
    /// Numeric ID value (same as `index` for BANKTOC-only mode)
    pub numeric_id: u32,
    pub name: String,
    /// Offset within PACK payload (no section header)
    pub pack_offset: u32,
    /// Audio payload size in bytes
    pub size: u32,
    /// Metadata size from the TONE pointer table (used for filtering/round-trip)
    pub metadata_size: u32,
    pub audio_data: Option<Vec<u8>>,
    pub audio_format: AudioFormat,
    /// Index into `ToneSection.tones`
    pub tone_index: usize,
}

impl AudioTrack {
    pub fn filename(&self) -> String {
        format!("{}-{}.wav", self.hex_id, self.name)
    }
}

/// Main structure representing a complete NUS3BANK file (BANKTOC-only mode).
#[derive(Clone, Debug)]
pub struct Nus3bankFile {
    pub toc: Vec<TocEntry>,
    pub prop: Option<PropSection>,
    pub binf: Option<BinfSection>,
    pub grp: Option<GrpSection>,
    pub dton: Option<DtonSection>,
    pub tone: ToneSection,
    pub junk: Option<JunkSection>,
    pub pack: PackSection,
    pub unknown_sections: Vec<RawSection>,
    /// Flattened UI track list derived from `tone`
    pub tracks: Vec<AudioTrack>,
    pub file_path: String,
}

impl Nus3bankFile {
    /// Open and parse a NUS3BANK file (BANKTOC-only).
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Nus3bankError> {
        super::parser::Nus3bankParser::parse_file(path)
    }

    /// Save the NUS3BANK file to disk.
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Nus3bankError> {
        super::writer::Nus3bankWriter::write_file(self, path)
    }

    pub fn get_track_by_hex_id(&self, hex_id: &str) -> Option<&AudioTrack> {
        self.tracks.iter().find(|t| t.hex_id == hex_id)
    }

    pub fn get_track_by_hex_id_mut(&mut self, hex_id: &str) -> Option<&mut AudioTrack> {
        self.tracks.iter_mut().find(|t| t.hex_id == hex_id)
    }

    pub fn replace_track_data(
        &mut self,
        hex_id: &str,
        new_data: Vec<u8>,
    ) -> Result<(), Nus3bankError> {
        if new_data.is_empty() {
            return Err(Nus3bankError::InvalidFormat {
                reason: "Audio data cannot be empty".to_string(),
            });
        }

        let track_pos = self
            .tracks
            .iter()
            .position(|t| t.hex_id == hex_id)
            .ok_or_else(|| Nus3bankError::TrackNotFound {
                hex_id: hex_id.to_string(),
            })?;
        let tone_index = self.tracks[track_pos].tone_index;

        {
            let tone = self.tone.tones.get_mut(tone_index).ok_or_else(|| {
                Nus3bankError::InvalidFormat {
                    reason: format!("Tone index out of bounds for track {}", hex_id),
                }
            })?;
            tone.payload = new_data.clone();
            tone.size = new_data.len() as i32;
        }

        {
            let track =
                self.tracks
                    .get_mut(track_pos)
                    .ok_or_else(|| Nus3bankError::InvalidFormat {
                        reason: "Track index out of bounds".to_string(),
                    })?;
            track.audio_data = Some(new_data.clone());
            track.size = new_data.len() as u32;
            track.audio_format = if new_data.starts_with(b"RIFF") {
                AudioFormat::Wav
            } else {
                AudioFormat::Unknown
            };
        }

        Ok(())
    }

    pub fn add_track(
        &mut self,
        name: String,
        audio_data: Vec<u8>,
    ) -> Result<String, Nus3bankError> {
        if audio_data.is_empty() {
            return Err(Nus3bankError::InvalidFormat {
                reason: "Audio data cannot be empty".to_string(),
            });
        }
        if name.is_empty() {
            return Err(Nus3bankError::InvalidFormat {
                reason: "Track name cannot be empty".to_string(),
            });
        }

        // Prevent duplicate names (matches existing UI expectations).
        if self.tracks.iter().any(|t| t.name == name) {
            return Err(Nus3bankError::InvalidFormat {
                reason: format!("Track with name '{}' already exists", name),
            });
        }

        // New track ID is the next index.
        let new_index = self.tone.tones.len();
        let hex_id = format!("0x{:x}", new_index as u32);

        // Use an existing tone as a template when available to keep metadata shape compatible.
        let template = self.tone.tones.iter().find(|t| !t.removed).cloned();
        let from_template = template.is_some();
        let mut new_tone = if let Some(t) = template {
            let mut cloned = t;
            cloned.name = name.clone();
            cloned.raw_meta.clear();
            cloned.pack_offset_field_pos = None;
            cloned.pack_size_field_pos = None;
            cloned.payload = audio_data.clone();
            cloned.size = audio_data.len() as i32;
            cloned.offset = 0;
            cloned.meta_size = 0;
            cloned.removed = false;
            cloned
        } else {
            ToneMeta {
                meta_prefix: Vec::new(),
                raw_meta: Vec::new(),
                pack_offset_field_pos: None,
                pack_size_field_pos: None,
                hash: 0,
                unk1: 0,
                name: name.clone(),
                reserved0: 0,
                reserved8: 8,
                offset: 0,
                size: audio_data.len() as i32,
                param: [0.0; 12],
                offsets: Vec::new(),
                unkvalues: Vec::new(),
                unkvalues_pair_order: UnkvaluesPairOrder::IndexThenValue,
                unkending: vec![-1],
                end: vec![0, 0, 0],
                payload: audio_data.clone(),
                meta_size: 0,
                removed: false,
            }
        };

        // If we didn't have a template, fall back to the C#-derived end-length rule.
        // If we used a template, preserve its `end` layout to maximize compatibility.
        if !from_template {
            let end_len = 3 + (((((new_tone.unk1 as u32) >> 8) & 0xFF) as usize) + 3) / 4;
            if new_tone.end.len() != end_len {
                new_tone.end.resize(end_len, 0);
            }
        }

        self.tone.tones.push(new_tone);

        if let Some(dton) = self.dton.as_mut() {
            let template_dton = if let Some(track) = self
                .tracks
                .iter()
                .find(|t| !self.tone.tones[t.tone_index].removed)
            {
                dton.tones.get(track.tone_index).cloned()
            } else {
                dton.tones.iter().next().cloned()
            };

            let mut new_dton = template_dton.unwrap_or_else(|| ToneDes {
                hash: 0,
                unk1: 0,
                name: name.clone(),
                data: Vec::new(),
                raw_data: Vec::new(),
                descriptor_words: Vec::new(),
            });
            new_dton.name = name.clone();
            dton.tones.push(new_dton);
        }

        // Rebuild UI track list (offsets will be recalculated during save).
        self.rebuild_tracks_view();

        Ok(hex_id)
    }

    pub fn remove_track(&mut self, hex_id: &str) -> Result<(), Nus3bankError> {
        let track = self
            .get_track_by_hex_id(hex_id)
            .ok_or_else(|| Nus3bankError::TrackNotFound {
                hex_id: hex_id.to_string(),
            })?
            .clone();

        let tone = self.tone.tones.get_mut(track.tone_index).ok_or_else(|| {
            Nus3bankError::InvalidFormat {
                reason: format!("Tone index out of bounds for track {}", hex_id),
            }
        })?;

        tone.removed = true;
        tone.payload.clear();
        tone.size = 0;

        // Keep the entry but mark it removed; the writer will filter removed tones.
        self.rebuild_tracks_view();

        Ok(())
    }

    pub(crate) fn rebuild_tracks_view(&mut self) {
        let mut tracks = Vec::new();
        for (i, tone) in self.tone.tones.iter().enumerate() {
            if tone.removed {
                continue;
            }

            let hex_id = format!("0x{:x}", i as u32);
            let audio_data = if tone.payload.is_empty() {
                None
            } else {
                Some(tone.payload.clone())
            };

            let audio_format = if tone.payload.starts_with(b"RIFF") {
                AudioFormat::Wav
            } else {
                AudioFormat::Unknown
            };

            tracks.push(AudioTrack {
                index: tracks.len(),
                hex_id,
                numeric_id: i as u32,
                name: tone.name.clone(),
                pack_offset: tone.offset.max(0) as u32,
                size: tone.size.max(0) as u32,
                metadata_size: tone.meta_size,
                audio_data,
                audio_format,
                tone_index: i,
            });
        }
        self.tracks = tracks;
    }
}
