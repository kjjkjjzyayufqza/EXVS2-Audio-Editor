use std::path::PathBuf;

use super::replace::{Nus3bankReplacer, ReplaceOperation};
use super::structures::{
    BinfSection, DtonSection, GrpSection, JunkSection, Nus3bankFile, PropLayout, PropSection,
    TocEntry, ToneDes, ToneMeta, ToneSection, UnkvaluesPairOrder,
};

fn unique_temp_path(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    p.push(format!("exvs2_audio_editor_test_{}_{}", name, nonce));
    p
}

fn minimal_wav_bytes() -> Vec<u8> {
    // Minimal 44-byte WAV header (PCM mono 8000Hz 16-bit, 0 data)
    vec![
        0x52, 0x49, 0x46, 0x46, 0x24, 0x00, 0x00, 0x00, 0x57, 0x41, 0x56, 0x45, 0x66, 0x6d, 0x74,
        0x20, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x40, 0x1f, 0x00, 0x00, 0x80, 0x3e,
        0x00, 0x00, 0x02, 0x00, 0x10, 0x00, 0x64, 0x61, 0x74, 0x61, 0x00, 0x00, 0x00, 0x00,
    ]
}

fn make_sample_file() -> Nus3bankFile {
    let toc = vec![
        TocEntry {
            magic: *b"PROP",
            size: 0,
        },
        TocEntry {
            magic: *b"BINF",
            size: 0,
        },
        TocEntry {
            magic: *b"GRP ",
            size: 0,
        },
        TocEntry {
            magic: *b"DTON",
            size: 0,
        },
        TocEntry {
            magic: *b"TONE",
            size: 0,
        },
        TocEntry {
            magic: *b"JUNK",
            size: 0,
        },
        TocEntry {
            magic: *b"PACK",
            size: 0,
        },
    ];

    let prop = PropSection {
        project: "DefaultProject".to_string(),
        timestamp: "2014/10/06 03:02:28".to_string(),
        unk1: 0xF1,
        reserved_u16: 0,
        unk2: 0x3,
        unk3: 0x8,
        layout: PropLayout::Extended,
        leading_u32: 0,
        presence_mask: 0,
        version: 0x0003_0000,
        bit5_u32: None,
        bit6_u32: None,
    };

    let binf = BinfSection {
        reserved0: 0,
        unk1: 3,
        name: "snd_bgm_CRS01_Menu".to_string(),
        flag: 0x05,
    };

    let grp = GrpSection {
        names: vec!["group_a".to_string(), "group_b".to_string()],
        raw_payload: None,
    };

    let dton = DtonSection::default();

    let wav_a = minimal_wav_bytes();
    let wav_b = {
        let mut w = minimal_wav_bytes();
        w.push(0x00);
        w.push(0x00);
        w
    };

    let tone0 = ToneMeta {
        meta_prefix: Vec::new(),
        raw_meta: Vec::new(),
        pack_offset_field_pos: None,
        pack_size_field_pos: None,
        name_len_pos: None,
        descriptor_words: Vec::new(),
        hash: 0x1111,
        unk1: 0,
        name: "track_a".to_string(),
        reserved0: 0,
        reserved8: 8,
        offset: 0,
        size: wav_a.len() as i32,
        param: [0.0; 12],
        offsets: vec![1, 2, 3],
        unkvalues: vec![0.1, 0.2, 0.3],
        unkvalues_pair_order: UnkvaluesPairOrder::IndexThenValue,
        unkending: vec![7, 8, -1],
        end: vec![0, 0, 0],
        payload: wav_a,
        meta_size: 0,
        removed: false,
    };

    let tone1 = ToneMeta {
        meta_prefix: Vec::new(),
        raw_meta: Vec::new(),
        pack_offset_field_pos: None,
        pack_size_field_pos: None,
        name_len_pos: None,
        descriptor_words: Vec::new(),
        hash: 0x2222,
        unk1: 0,
        name: "track_b".to_string(),
        reserved0: 0,
        reserved8: 8,
        offset: 0,
        size: wav_b.len() as i32,
        param: [1.0; 12],
        offsets: vec![],
        unkvalues: vec![],
        unkvalues_pair_order: UnkvaluesPairOrder::IndexThenValue,
        unkending: vec![-1],
        end: vec![0, 0, 0],
        payload: wav_b,
        meta_size: 0,
        removed: false,
    };

    let tone = ToneSection {
        tones: vec![tone0, tone1],
    };

    Nus3bankFile {
        toc,
        prop: Some(prop),
        binf: Some(binf),
        grp: Some(grp),
        dton: Some(dton),
        tone,
        junk: Some(JunkSection {
            data: vec![0, 0, 0, 0],
        }),
        pack: Default::default(),
        unknown_sections: Vec::new(),
        tracks: Vec::new(),
        file_path: "in_memory".to_string(),
    }
}

fn declared_section_total_len(section: &[u8]) -> usize {
    assert!(section.len() >= 8);
    let declared_size =
        u32::from_le_bytes([section[4], section[5], section[6], section[7]]) as usize;
    8 + declared_size
}

fn dton_expected_float_counts(bytes: &[u8]) -> Vec<usize> {
    assert!(bytes.len() >= 12);
    let count = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let start = 12;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let base = 12 + i * 8;
        let off = u32::from_le_bytes([
            bytes[base],
            bytes[base + 1],
            bytes[base + 2],
            bytes[base + 3],
        ]) as usize;
        let sz = u32::from_le_bytes([
            bytes[base + 4],
            bytes[base + 5],
            bytes[base + 6],
            bytes[base + 7],
        ]) as usize;
        let entry_start = start + off;
        let entry_end = (entry_start + sz).min(bytes.len());
        assert!(entry_start + 9 <= entry_end);

        let name_len_with_null = bytes[entry_start + 8] as usize;
        let after_name = entry_start + 9 + name_len_with_null.saturating_sub(1) + 1;
        let aligned = (after_name + 3) & !3;
        let header_len = aligned.saturating_sub(entry_start);

        let available = entry_end.saturating_sub(entry_start + header_len);
        out.push(available / 4);
    }
    out
}

fn build_banktoc_file(section_payloads: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let sec_count = section_payloads.len() as u32;
    let toc_size = 4u32 + sec_count * 8;
    let mut out = Vec::new();
    out.extend_from_slice(b"NUS3");
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(b"BANKTOC ");
    out.extend_from_slice(&toc_size.to_le_bytes());
    out.extend_from_slice(&sec_count.to_le_bytes());

    for (magic, payload) in section_payloads {
        out.extend_from_slice(magic);
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    }

    for (magic, payload) in section_payloads {
        out.extend_from_slice(magic);
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        out.extend_from_slice(payload);
    }

    let total_size = out.len().saturating_sub(8) as u32;
    out[4..8].copy_from_slice(&total_size.to_le_bytes());
    out
}

fn write_len_string(out: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    let len = (bytes.len() + 1).min(255) as u8;
    out.push(len);
    out.extend_from_slice(&bytes[..(len as usize).saturating_sub(1)]);
    out.push(0);
    while out.len() % 4 != 0 {
        out.push(0);
    }
}

// This fixture contains only the original TONE section, no song audio.
fn legacy_song_bank(payload: &[u8]) -> Vec<u8> {
    let mut tone = include_bytes!("../../tests/fixtures/song_wgnmd1-tone.bin")[8..].to_vec();
    // PACK size in the original metadata; all other bytes reproduce the reported bug.
    tone[0x34..0x38].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    build_banktoc_file(&[(*b"TONE", tone), (*b"PACK", payload.to_vec())])
}

#[test]
fn legacy_song_header_survives_unknown_parameter_layout() {
    let payload = b"BNSFtest payload";
    let input = unique_temp_path("legacy_song.nus3bank");
    let output = unique_temp_path("legacy_song_roundtrip.nus3bank");
    std::fs::write(&input, legacy_song_bank(payload)).unwrap();
    let file = Nus3bankFile::open(&input).unwrap();
    assert_eq!(file.tracks.len(), 1);
    let track = &file.tracks[0];
    assert_eq!(track.name, "song_wgnmd1");
    assert_eq!(track.size as usize, payload.len());
    assert_eq!(track.pack_offset, 0);
    assert_eq!(track.audio_data.as_deref(), Some(payload.as_slice()));
    let original_meta = file.tone.tones[0].raw_meta.clone();
    assert!(file.tone.tones[0].meta_prefix.is_empty());
    file.save(&output).unwrap();
    let roundtrip = Nus3bankFile::open(&output).unwrap();
    assert_eq!(roundtrip.tone.tones[0].raw_meta, original_meta);
    assert_eq!(
        roundtrip.tracks[0].audio_data.as_deref(),
        Some(payload.as_slice())
    );
    std::fs::remove_file(input).unwrap();
    std::fs::remove_file(output).unwrap();
}

#[test]
fn legacy_song_rejects_pack_range_outside_file() {
    let input = unique_temp_path("legacy_song_bad_range.nus3bank");
    let mut bytes = legacy_song_bank(b"BNSF");
    // Two-entry TOC is 40 bytes; TONE size field is at section + 0x3c.
    bytes[100..104].copy_from_slice(&3439408u32.to_le_bytes());
    std::fs::write(&input, bytes).unwrap();
    let err = Nus3bankFile::open(&input).unwrap_err();
    assert!(err.to_string().contains("pack offset/size out of bounds"));
    std::fs::remove_file(input).unwrap();
}

#[test]
fn prefixed_tone_layout_still_parses() {
    let mut file = make_sample_file();
    file.tone.tones[0].meta_prefix = vec![0; 8];
    let input = unique_temp_path("prefixed_tone.nus3bank");
    file.save(&input).unwrap();
    let parsed = Nus3bankFile::open(&input).unwrap();
    assert_eq!(parsed.tracks[0].name, "track_a");
    assert_eq!(parsed.tone.tones[0].meta_prefix, vec![0; 8]);
    std::fs::remove_file(input).unwrap();
}

#[test]
fn export_names_cannot_contain_nul_or_escape_output_directory() {
    use super::export::wav_filename;
    assert_eq!(wav_filename("song_wgnmd1", "audio"), "song_wgnmd1.wav");
    assert_eq!(wav_filename("a\0b/c\\d:e?f", "audio"), "a_b_c_d_e_f.wav");
    assert_eq!(wav_filename("NUL", "audio"), "_NUL.wav");
    assert_eq!(wav_filename("com1.foo", "audio"), "_com1.foo.wav");
    assert_eq!(wav_filename(".. ", "audio"), "audio.wav");
}

#[test]
#[cfg(windows)]
fn batch_export_decodes_pcm_instead_of_copying_payload() {
    let input = unique_temp_path("export_pcm").with_extension("nus3bank");
    let output_dir = unique_temp_path("export_pcm");
    std::fs::create_dir(&output_dir).unwrap();
    let mut wav = minimal_wav_bytes();
    wav[4..8].copy_from_slice(&38u32.to_le_bytes());
    wav[28..32].copy_from_slice(&8000u32.to_le_bytes());
    wav[32..34].copy_from_slice(&1u16.to_le_bytes());
    wav[34..36].copy_from_slice(&8u16.to_le_bytes());
    wav[40..44].copy_from_slice(&2u32.to_le_bytes());
    wav.extend_from_slice(&[128, 129]);
    std::fs::write(&input, legacy_song_bank(&wav)).unwrap();
    let paths = super::export::Nus3bankExporter::export_all_tracks(
        input.to_str().unwrap(),
        output_dir.to_str().unwrap(),
    )
    .unwrap();
    assert_eq!(paths.len(), 1);
    let reader = hound::WavReader::open(&paths[0]).unwrap();
    assert_eq!(reader.spec().sample_rate, 8000);
    assert_eq!(reader.spec().bits_per_sample, 16);
    assert_eq!(reader.duration(), 2);
    drop(reader);
    std::fs::remove_file(&paths[0]).unwrap();
    // A failed decode must not be reported as a successful empty export.
    std::fs::write(&input, legacy_song_bank(b"not audio")).unwrap();
    assert!(
        super::export::Nus3bankExporter::export_all_tracks(
            input.to_str().unwrap(),
            output_dir.to_str().unwrap(),
        )
        .is_err()
    );
    std::fs::remove_file(input).unwrap();
    std::fs::remove_dir(output_dir).unwrap();
}

#[test]
#[cfg(windows)]
fn supplied_song_parses_and_exports_if_present() {
    let input = PathBuf::from("../song_wgnmd1.nus3bank");
    if !input.exists() {
        return;
    }
    let file = Nus3bankFile::open(&input).unwrap();
    assert_eq!(file.tracks.len(), 1);
    assert_eq!(file.tracks[0].name, "song_wgnmd1");
    assert_eq!(file.tracks[0].size, 3439408);
    assert!(
        file.tracks[0]
            .audio_data
            .as_ref()
            .unwrap()
            .starts_with(b"BNSF")
    );
    let output_dir = unique_temp_path("supplied_song_export");
    std::fs::create_dir(&output_dir).unwrap();
    let paths = super::export::Nus3bankExporter::export_all_tracks(
        input.to_str().unwrap(),
        output_dir.to_str().unwrap(),
    )
    .unwrap();
    assert_eq!(paths.len(), 1);
    let reader = hound::WavReader::open(&paths[0]).unwrap();
    assert_eq!(reader.spec().sample_rate, 48000);
    assert_eq!(reader.spec().channels, 2);
    assert_eq!(reader.spec().bits_per_sample, 16);
    assert_eq!(reader.duration(), 5158694);
    drop(reader);
    std::fs::remove_file(&paths[0]).unwrap();
    std::fs::remove_dir(output_dir).unwrap();
}

fn bitmask_prop_payload() -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&0u32.to_le_bytes());
    payload.extend_from_slice(&0x0000_00F1u32.to_le_bytes());
    payload.extend_from_slice(&0x0003_0001u32.to_le_bytes());
    write_len_string(&mut payload, "BitmaskProject");
    payload.extend_from_slice(&0x1122_3344u32.to_le_bytes());
    payload.extend_from_slice(&0x5566_7788u32.to_le_bytes());
    write_len_string(&mut payload, "2026/05/18 12:34:56");
    payload
}

fn dton_payload_with_raw_tail() -> Vec<u8> {
    let mut entry = Vec::new();
    entry.extend_from_slice(&0x1234_i32.to_le_bytes());
    entry.extend_from_slice(&0x5678_i32.to_le_bytes());
    write_len_string(&mut entry, "RawTail");
    entry.extend_from_slice(&[0x81, 0x00, 0x00, 0x80]);
    entry.extend_from_slice(&[0x04, 0x00, 0x00, 0x00]);
    entry.extend_from_slice(&[0xAA, 0xBB]);

    let mut payload = Vec::new();
    payload.extend_from_slice(&1u32.to_le_bytes());
    payload.extend_from_slice(&12u32.to_le_bytes());
    payload.extend_from_slice(&(entry.len() as u32).to_le_bytes());
    payload.extend_from_slice(&0u32.to_le_bytes());
    payload.extend_from_slice(&entry);
    payload
}

#[test]
fn parse_real_file_smoke_if_present() {
    let p = std::path::Path::new("se_chr_001gundam_001gundam_001.nus3bank");
    if !p.exists() {
        return;
    }

    let parsed = Nus3bankFile::open(p).unwrap();
    assert!(parsed.tone.tones.len() > 0);
}

#[test]
fn parse_dton_1_bin_extract() {
    let bytes: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/dton_1.bin"));
    assert_eq!(&bytes[0..4], b"DTON");
    assert_eq!(declared_section_total_len(bytes), bytes.len());

    let dton = super::parser::Nus3bankParser::parse_dton(bytes).unwrap();
    assert_eq!(dton.tones.len(), 1);
    assert_eq!(dton.tones[0].name, "Default");
    assert_eq!(dton.tones[0].unk1, 123456);
    let expected = dton_expected_float_counts(bytes);
    assert_eq!(dton.tones[0].data.len(), expected[0]);
}

#[test]
fn parse_dton_2_bin_extract() {
    let bytes: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/dton_2.bin"));
    assert_eq!(&bytes[0..4], b"DTON");
    assert_eq!(declared_section_total_len(bytes), bytes.len());

    let dton = super::parser::Nus3bankParser::parse_dton(bytes).unwrap();
    assert!(!dton.tones.is_empty());
    assert!(dton.tones.iter().any(|t| t.name == "Default"));
    let expected = dton_expected_float_counts(bytes);
    assert_eq!(dton.tones.len(), expected.len());
    for (i, t) in dton.tones.iter().enumerate() {
        assert_eq!(t.data.len(), expected[i]);
    }
}

#[test]
fn parse_dton_3_bin_extract() {
    let bytes: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/dton_3.bin"));
    assert_eq!(&bytes[0..4], b"DTON");
    assert_eq!(declared_section_total_len(bytes), bytes.len());

    let dton = super::parser::Nus3bankParser::parse_dton(bytes).unwrap();
    assert!(!dton.tones.is_empty());
    assert!(dton.tones.iter().any(|t| t.name == "Default"));
    let expected = dton_expected_float_counts(bytes);
    assert_eq!(dton.tones.len(), expected.len());
    for (i, t) in dton.tones.iter().enumerate() {
        assert_eq!(t.data.len(), expected[i]);
    }
}

#[test]
fn parse_bitmask_prop_preserves_engine_layout() {
    let bytes = build_banktoc_file(&[
        (*b"PROP", bitmask_prop_payload()),
        (*b"TONE", 0u32.to_le_bytes().to_vec()),
        (*b"PACK", Vec::new()),
    ]);
    let path = unique_temp_path("bitmask_prop.nus3bank");
    std::fs::write(&path, bytes).unwrap();

    let parsed = Nus3bankFile::open(&path).unwrap();
    let prop = parsed.prop.as_ref().unwrap();
    assert_eq!(prop.layout, PropLayout::Bitmask);
    assert_eq!(prop.leading_u32, 0);
    assert_eq!(prop.presence_mask, 0xF1);
    assert_eq!(prop.version, 0x0003_0001);
    assert_eq!(prop.project, "BitmaskProject");
    assert_eq!(prop.bit5_u32, Some(0x1122_3344));
    assert_eq!(prop.bit6_u32, Some(0x5566_7788));
    assert_eq!(prop.timestamp, "2026/05/18 12:34:56");

    let out_path = unique_temp_path("bitmask_prop_roundtrip.nus3bank");
    parsed.save(&out_path).unwrap();
    let reparsed = Nus3bankFile::open(&out_path).unwrap();
    let prop = reparsed.prop.as_ref().unwrap();
    assert_eq!(prop.layout, PropLayout::Bitmask);
    assert_eq!(prop.presence_mask, 0xF1);
    assert_eq!(prop.version, 0x0003_0001);
    assert_eq!(prop.project, "BitmaskProject");
    assert_eq!(prop.timestamp, "2026/05/18 12:34:56");
}

#[test]
fn parse_dton_preserves_raw_descriptor_bytes() {
    let payload = dton_payload_with_raw_tail();
    let mut section = Vec::new();
    section.extend_from_slice(b"DTON");
    section.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    section.extend_from_slice(&payload);

    let dton = super::parser::Nus3bankParser::parse_dton(&section).unwrap();
    let tone = &dton.tones[0];
    assert_eq!(tone.name, "RawTail");
    assert_eq!(
        tone.raw_data,
        vec![0x81, 0x00, 0x00, 0x80, 0x04, 0x00, 0x00, 0x00, 0xAA, 0xBB]
    );
    assert_eq!(tone.descriptor_words, vec![0x8000_0081, 0x0000_0004]);
    assert_eq!(tone.data.len(), 2);
}

#[test]
fn remove_track_preserves_tone_indices() {
    let mut file = make_sample_file();
    file.rebuild_tracks_view();

    let out_path = unique_temp_path("remove_stable_in.nus3bank");
    file.save(&out_path).unwrap();

    let mut parsed = Nus3bankFile::open(&out_path).unwrap();
    parsed.remove_track("0x0").unwrap();

    let out_path2 = unique_temp_path("remove_stable_out.nus3bank");
    parsed.save(&out_path2).unwrap();

    let reparsed = Nus3bankFile::open(&out_path2).unwrap();
    assert_eq!(reparsed.tone.tones.len(), 2);
    assert!(reparsed.tone.tones[0].removed);
    assert_eq!(reparsed.tracks.len(), 1);
    assert_eq!(reparsed.tracks[0].hex_id, "0x1");
    assert_eq!(reparsed.tracks[0].numeric_id, 1);
    assert_eq!(reparsed.tracks[0].name, "track_b");
}

#[test]
fn add_track_clones_dton_descriptor() {
    let mut file = make_sample_file();
    file.dton = Some(DtonSection {
        raw_payload: None,
        tones: vec![
            ToneDes {
                hash: 0x1111,
                unk1: 1,
                name: "track_a".to_string(),
                data: vec![1.0, 2.0],
                raw_data: vec![0, 0, 0x80, 0x3F, 0, 0, 0, 0x40],
                descriptor_words: Vec::new(),
            },
            ToneDes {
                hash: 0x2222,
                unk1: 2,
                name: "track_b".to_string(),
                data: vec![3.0, 4.0],
                raw_data: vec![0, 0, 0x40, 0x40, 0, 0, 0x80, 0x40],
                descriptor_words: Vec::new(),
            },
        ],
    });
    file.rebuild_tracks_view();

    let new_id = file
        .add_track("track_c".to_string(), minimal_wav_bytes())
        .unwrap();
    assert_eq!(new_id, "0x2");
    let dton = file.dton.as_ref().unwrap();
    assert_eq!(dton.tones.len(), 3);
    assert_eq!(dton.tones[2].name, "track_c");
    assert_eq!(dton.tones[2].raw_data, dton.tones[0].raw_data);
    assert_eq!(dton.tones[2].data, dton.tones[0].data);
}

#[test]
fn parse_sample2_smoke_if_present() {
    let p = std::path::Path::new("sample2.nus3bank");
    if !p.exists() {
        return;
    }

    let parsed = Nus3bankFile::open(p).unwrap();
    assert!(parsed.tone.tones.len() > 0);
}

#[test]
fn real_game_logic_files_parse_and_roundtrip_if_present() {
    let paths = [
        "se_chr_654gexvs2_003glfunl_001.nus3bank",
        "gvs_rx78.nus3bank",
    ];

    for path in paths {
        let p = std::path::Path::new(path);
        if !p.exists() {
            continue;
        }

        let parsed = Nus3bankFile::open(p).unwrap();
        assert!(!parsed.tone.tones.is_empty(), "{path} has no TONE entries");
        assert!(!parsed.tracks.is_empty(), "{path} has no playable tracks");

        if let Some(prop) = parsed.prop.as_ref() {
            if prop.layout == PropLayout::Bitmask {
                assert_eq!(prop.version & 0xFFFF_0000, 0x0003_0000);
                assert_ne!(prop.presence_mask & 1, 0);
            }
        }

        if let Some(dton) = parsed.dton.as_ref() {
            for tone in &dton.tones {
                assert!(tone.raw_data.len() >= tone.data.len() * 4);
                if !tone.raw_data.is_empty() {
                    assert!(!tone.descriptor_words.is_empty());
                }
            }
        }

        let track_ids = parsed
            .tracks
            .iter()
            .map(|track| (track.hex_id.clone(), track.numeric_id, track.tone_index))
            .collect::<Vec<_>>();
        let tone_count = parsed.tone.tones.len();
        let dton_count = parsed.dton.as_ref().map(|d| d.tones.len());
        let grp_count = parsed.grp.as_ref().map(|g| g.names.len());

        let out_path = unique_temp_path(&format!(
            "real_roundtrip_{}",
            path.replace(['\\', '/', '.'], "_")
        ));
        parsed.save(&out_path).unwrap();
        let reparsed = Nus3bankFile::open(&out_path).unwrap();

        assert_eq!(
            reparsed.tone.tones.len(),
            tone_count,
            "{path} TONE count changed"
        );
        assert_eq!(
            reparsed.dton.as_ref().map(|d| d.tones.len()),
            dton_count,
            "{path} DTON count changed"
        );
        assert_eq!(
            reparsed.grp.as_ref().map(|g| g.names.len()),
            grp_count,
            "{path} GRP count changed"
        );

        let reparsed_track_ids = reparsed
            .tracks
            .iter()
            .map(|track| (track.hex_id.clone(), track.numeric_id, track.tone_index))
            .collect::<Vec<_>>();
        assert_eq!(reparsed_track_ids, track_ids, "{path} track ids changed");
    }
}

#[test]
fn debug_json_is_valid_json() {
    let mut file = make_sample_file();
    file.rebuild_tracks_view();

    let opt = super::debug_json::DebugJsonOptions {
        include_pack_preview: true,
        include_tone_payload_preview: true,
        include_unknown_section_preview: true,
        max_preview_bytes: 256,
    };

    let s = file.to_debug_json_string(&opt).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert!(v.is_object());
}

#[test]
fn roundtrip_sections_and_tracks() {
    let mut file = make_sample_file();
    file.rebuild_tracks_view();

    let out_path = unique_temp_path("roundtrip.nus3bank");
    file.save(&out_path).unwrap();

    let parsed = Nus3bankFile::open(&out_path).unwrap();
    assert_eq!(parsed.prop.as_ref().unwrap().project, "DefaultProject");
    assert_eq!(parsed.binf.as_ref().unwrap().unk1, 3);
    assert_eq!(parsed.binf.as_ref().unwrap().flag, 0x05);
    assert_eq!(parsed.tracks.len(), 2);
    assert_eq!(parsed.tracks[0].name, "track_a");
    assert!(
        parsed.tracks[0]
            .audio_data
            .as_ref()
            .unwrap()
            .starts_with(b"RIFF")
    );
    assert_eq!(parsed.tone.tones.len(), 2);
    assert_eq!(parsed.tone.tones[0].name, "track_a");
    assert_eq!(parsed.tone.tones[1].name, "track_b");
}

#[test]
fn mutate_replace_and_save_updates_payload() {
    let mut file = make_sample_file();
    file.rebuild_tracks_view();

    let out_path = unique_temp_path("replace_in.nus3bank");
    file.save(&out_path).unwrap();

    let mut parsed = Nus3bankFile::open(&out_path).unwrap();
    let mut new_wav = minimal_wav_bytes();
    new_wav.extend_from_slice(b"ABCD");
    parsed.replace_track_data("0x0", new_wav.clone()).unwrap();

    let out_path2 = unique_temp_path("replace_out.nus3bank");
    parsed.save(&out_path2).unwrap();

    let reparsed = Nus3bankFile::open(&out_path2).unwrap();
    assert_eq!(reparsed.tracks.len(), 2);
    assert_eq!(reparsed.tracks[0].audio_data.as_ref().unwrap(), &new_wav);
}

#[test]
fn mutate_remove_and_save_filters_track() {
    let mut file = make_sample_file();
    file.rebuild_tracks_view();

    let out_path = unique_temp_path("remove_in.nus3bank");
    file.save(&out_path).unwrap();

    let mut parsed = Nus3bankFile::open(&out_path).unwrap();
    parsed.remove_track("0x0").unwrap();

    let out_path2 = unique_temp_path("remove_out.nus3bank");
    parsed.save(&out_path2).unwrap();

    let reparsed = Nus3bankFile::open(&out_path2).unwrap();
    assert_eq!(reparsed.tracks.len(), 1);
    assert_eq!(reparsed.tracks[0].name, "track_b");
}

#[test]
fn mutate_add_and_save_appends_track() {
    let mut file = make_sample_file();
    file.rebuild_tracks_view();

    let out_path = unique_temp_path("add_in.nus3bank");
    file.save(&out_path).unwrap();

    let mut parsed = Nus3bankFile::open(&out_path).unwrap();
    let new_id = parsed
        .add_track("track_c".to_string(), minimal_wav_bytes())
        .unwrap();
    assert_eq!(new_id, "0x2");

    let out_path2 = unique_temp_path("add_out.nus3bank");
    parsed.save(&out_path2).unwrap();

    let reparsed = Nus3bankFile::open(&out_path2).unwrap();
    assert_eq!(reparsed.tracks.len(), 3);
    assert_eq!(reparsed.tracks[2].name, "track_c");
}

#[test]
fn add_track_template_dton_does_not_grow() {
    let mut file = make_sample_file();
    file.dton = Some(DtonSection {
        raw_payload: None,
        tones: vec![ToneDes {
            hash: 1,
            unk1: 123456,
            name: "Default".to_string(),
            data: Vec::new(),
            raw_data: vec![1, 2, 3, 4],
            descriptor_words: Vec::new(),
        }],
    });
    file.rebuild_tracks_view();
    file.add_track("track_c".to_string(), minimal_wav_bytes())
        .unwrap();
    let dton = file.dton.as_ref().unwrap();
    assert_eq!(dton.tones.len(), 1);
    assert_eq!(dton.tones[0].name, "Default");
    assert_eq!(dton.tones[0].raw_data, vec![1, 2, 3, 4]);
    assert!(file.tracks.iter().any(|t| t.name == "track_c"));
}

fn sample_tone_raw_meta(name: &str, pack_off: i32, pack_sz: i32) -> (Vec<u8>, usize, usize) {
    let mut raw = Vec::new();
    raw.extend_from_slice(&0x1111i32.to_le_bytes());
    raw.extend_from_slice(&0i32.to_le_bytes());
    let name_bytes = name.as_bytes();
    raw.push((name_bytes.len() + 1) as u8);
    raw.extend_from_slice(name_bytes);
    raw.push(0);
    while raw.len() % 4 != 0 {
        raw.push(0);
    }
    raw.extend_from_slice(&0i32.to_le_bytes());
    raw.extend_from_slice(&8i32.to_le_bytes());
    let off_pos = raw.len();
    raw.extend_from_slice(&pack_off.to_le_bytes());
    let sz_pos = raw.len();
    raw.extend_from_slice(&pack_sz.to_le_bytes());
    raw.extend_from_slice(&[0xAA; 32]);
    (raw, off_pos, sz_pos)
}

#[test]
fn rewrite_name_keeps_descriptor_tail_and_shifts_pack_fields() {
    let (raw, off_pos, sz_pos) =
        sample_tone_raw_meta("SE_CHR_GUNW_01ZERO_BUSTER_SHOT", 32352, 18664);
    let tail = raw[raw.len() - 32..].to_vec();
    let mut tone = ToneMeta {
        meta_prefix: Vec::new(),
        raw_meta: raw,
        pack_offset_field_pos: Some(off_pos),
        pack_size_field_pos: Some(sz_pos),
        name_len_pos: Some(8),
        descriptor_words: Vec::new(),
        hash: 0x1111,
        unk1: 0,
        name: "SE_CHR_GUNW_01ZERO_BUSTER_SHOT".to_string(),
        reserved0: 0,
        reserved8: 8,
        offset: 32352,
        size: 18664,
        param: [0.0; 12],
        offsets: Vec::new(),
        unkvalues: Vec::new(),
        unkvalues_pair_order: UnkvaluesPairOrder::IndexThenValue,
        unkending: vec![-1],
        end: Vec::new(),
        payload: minimal_wav_bytes(),
        meta_size: 0,
        removed: false,
    };
    tone.rewrite_name_in_raw_meta("new_test_audio").unwrap();
    assert_eq!(tone.name, "new_test_audio");
    assert!(tone.raw_meta.len() > 40);
    assert_eq!(&tone.raw_meta[tone.raw_meta.len() - 32..], &tail);
    let off = i32::from_le_bytes(
        tone.raw_meta[tone.pack_offset_field_pos.unwrap()..][..4]
            .try_into()
            .unwrap(),
    );
    let sz = i32::from_le_bytes(
        tone.raw_meta[tone.pack_size_field_pos.unwrap()..][..4]
            .try_into()
            .unwrap(),
    );
    assert_eq!(off, 32352);
    assert_eq!(sz, 18664);

    tone.rewrite_name_in_raw_meta("SE_CHR_016GUNDMW_001WGZERO_001_01")
        .unwrap();
    assert_eq!(&tone.raw_meta[tone.raw_meta.len() - 32..], &tail);
}

fn make_stub_tone() -> ToneMeta {
    ToneMeta {
        meta_prefix: Vec::new(),
        raw_meta: vec![0u8; 32],
        pack_offset_field_pos: None,
        pack_size_field_pos: None,
        name_len_pos: None,
        descriptor_words: Vec::new(),
        hash: 0,
        unk1: 0,
        name: String::new(),
        reserved0: 0,
        reserved8: 8,
        offset: 0,
        size: 0,
        param: [0.0; 12],
        offsets: Vec::new(),
        unkvalues: Vec::new(),
        unkvalues_pair_order: UnkvaluesPairOrder::IndexThenValue,
        unkending: vec![-1],
        end: Vec::new(),
        payload: Vec::new(),
        meta_size: 32,
        removed: true,
    }
}

#[test]
fn sparse_template_bank_add_roundtrip() {
    let mut file = make_sample_file();
    file.dton = Some(DtonSection {
        raw_payload: None,
        tones: vec![ToneDes {
            hash: 1,
            unk1: 123456,
            name: "Default".to_string(),
            data: Vec::new(),
            raw_data: vec![7u8; 200],
            descriptor_words: Vec::new(),
        }],
    });
    file.tone.tones.insert(1, make_stub_tone());
    file.tone.tones.push(make_stub_tone());
    file.tone.tones[0].offset = 0;
    file.tone.tones[2].offset = 16;
    file.rebuild_tracks_view();
    assert_eq!(file.tone.tones.len(), 4);

    let out = unique_temp_path("sparse_in.nus3bank");
    file.save(&out).unwrap();
    let mut parsed = Nus3bankFile::open(&out).unwrap();
    let slots = parsed.tone.tones.len();
    parsed
        .add_track("new_test_audio".to_string(), minimal_wav_bytes())
        .unwrap();
    let out2 = unique_temp_path("sparse_out.nus3bank");
    parsed.save(&out2).unwrap();
    let reparsed = Nus3bankFile::open(&out2).unwrap();
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&out2);

    assert_eq!(reparsed.tone.tones.len(), slots);
    assert_eq!(reparsed.dton.as_ref().unwrap().tones.len(), 1);
    assert_eq!(reparsed.dton.as_ref().unwrap().tones[0].name, "Default");
    assert_eq!(reparsed.dton.as_ref().unwrap().tones[0].raw_data.len(), 200);
    assert!(reparsed.tracks.iter().any(|t| t.name == "new_test_audio"));
    let added = reparsed
        .tone
        .tones
        .iter()
        .find(|t| t.name == "new_test_audio")
        .expect("added cue missing");
    assert!(
        !added.raw_meta.is_empty(),
        "added cue must keep a TONE descriptor blob"
    );
    assert!(
        added.pack_offset_field_pos.is_some() && added.pack_size_field_pos.is_some(),
        "added cue must keep patchable PACK fields"
    );
}

#[test]
fn add_track_reuses_removed_slot() {
    let mut file = make_sample_file();
    file.rebuild_tracks_view();
    file.remove_track("0x0").unwrap();
    assert_eq!(file.tone.tones.len(), 2);
    assert!(file.tone.tones[0].removed);

    let id = file
        .add_track("track_c".to_string(), minimal_wav_bytes())
        .unwrap();
    assert_eq!(id, "0x0");
    assert_eq!(file.tone.tones.len(), 2);
    assert!(!file.tone.tones[0].removed);
    assert_eq!(file.tone.tones[0].name, "track_c");
}

fn original_se_bank_path() -> Option<PathBuf> {
    let p = PathBuf::from(
        r"E:\XB\mod\090sound\016gundmw_001wgzero_001\SE_CHR_016GUNDMW_001WGZERO_001.nus3bank",
    );
    p.exists().then_some(p)
}

fn bundled_se_bank_path() -> Option<PathBuf> {
    let p =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("se_chr_654gexvs2_003glfunl_001.nus3bank");
    p.exists().then_some(p)
}

fn live_named_meta_lens(file: &Nus3bankFile) -> Vec<(usize, String, usize)> {
    file.tone
        .tones
        .iter()
        .enumerate()
        .filter(|(_, t)| !t.removed && !t.name.is_empty())
        .map(|(i, t)| (i, t.name.clone(), t.raw_meta.len()))
        .collect()
}

fn assert_live_pack_16_aligned(file: &Nus3bankFile) {
    for t in file
        .tone
        .tones
        .iter()
        .filter(|t| !t.removed && t.size > 0 && t.offset >= 0)
    {
        assert_eq!(
            t.offset % 16,
            0,
            "PACK offset {} for {} is not 16-aligned",
            t.offset,
            t.name
        );
    }
}

#[test]
fn original_se_bank_noop_save_preserves_layout() {
    let Some(src) = original_se_bank_path() else {
        eprintln!("skip: original Wing Zero SE bank not found");
        return;
    };
    let parsed = Nus3bankFile::open(&src).unwrap();
    assert_eq!(parsed.tone.tones.len(), 86);
    assert_eq!(parsed.dton.as_ref().unwrap().tones.len(), 1);
    assert_eq!(parsed.dton.as_ref().unwrap().tones[0].name, "Default");
    let default_raw = parsed.dton.as_ref().unwrap().tones[0].raw_data.len();
    let before = live_named_meta_lens(&parsed);
    assert_eq!(before.len(), 18);

    let out = unique_temp_path("se_noop.nus3bank");
    parsed.save(&out).unwrap();
    let reparsed = Nus3bankFile::open(&out).unwrap();
    let _ = std::fs::remove_file(&out);

    assert_eq!(reparsed.tone.tones.len(), 86);
    assert_eq!(reparsed.dton.as_ref().unwrap().tones.len(), 1);
    assert_eq!(reparsed.dton.as_ref().unwrap().tones[0].name, "Default");
    assert_eq!(
        reparsed.dton.as_ref().unwrap().tones[0].raw_data.len(),
        default_raw
    );
    assert_eq!(live_named_meta_lens(&reparsed), before);
    assert_live_pack_16_aligned(&reparsed);
    assert_eq!(
        reparsed.dton.as_ref().unwrap().raw_payload,
        parsed.dton.as_ref().unwrap().raw_payload,
        "template DTON payload must round-trip"
    );
    for t in &reparsed.tone.tones {
        if t.removed {
            continue;
        }
        super::ob_tone_decode::probe_live_tone(&t.raw_meta, &t.name)
            .unwrap_or_else(|e| panic!("probe failed after noop save: {e}"));
    }
}

#[test]
fn original_se_bank_add_keeps_dton_and_slots() {
    let Some(src) = original_se_bank_path() else {
        eprintln!("skip: original Wing Zero SE bank not found");
        return;
    };
    let mut parsed = Nus3bankFile::open(&src).unwrap();
    let before = live_named_meta_lens(&parsed);
    let default_raw = parsed.dton.as_ref().unwrap().tones[0].raw_data.len();

    parsed
        .add_track("new_test_audio".to_string(), minimal_wav_bytes())
        .unwrap();
    let out = unique_temp_path("se_add.nus3bank");
    parsed.save(&out).unwrap();
    let reparsed = Nus3bankFile::open(&out).unwrap();
    let _ = std::fs::remove_file(&out);

    assert_eq!(reparsed.tone.tones.len(), 86);
    assert_eq!(reparsed.dton.as_ref().unwrap().tones.len(), 1);
    assert_eq!(reparsed.dton.as_ref().unwrap().tones[0].name, "Default");
    assert_eq!(
        reparsed.dton.as_ref().unwrap().tones[0].raw_data.len(),
        default_raw
    );
    assert!(reparsed.tracks.iter().any(|t| t.name == "new_test_audio"));
    let after = live_named_meta_lens(&reparsed);
    for (idx, name, len) in &before {
        let found = after.iter().find(|(i, n, _)| i == idx && n == name);
        assert!(found.is_some(), "missing original cue {} at {}", name, idx);
        assert_eq!(found.unwrap().2, *len, "raw_meta size changed for {}", name);
    }
    assert_live_pack_16_aligned(&reparsed);
    for t in &reparsed.tone.tones {
        if t.removed {
            continue;
        }
        super::ob_tone_decode::probe_live_tone(&t.raw_meta, &t.name)
            .unwrap_or_else(|e| panic!("probe failed after add: {e}"));
    }
    let added = reparsed
        .tone
        .tones
        .iter()
        .find(|t| t.name == "new_test_audio")
        .expect("added cue missing");
    assert_eq!(
        added.raw_meta.get(added.name_len_pos.unwrap()),
        Some(&((added.name.len() + 1) as u8))
    );
}

#[test]
fn original_se_bank_remove_keeps_slot_count() {
    let Some(src) = original_se_bank_path() else {
        eprintln!("skip: original Wing Zero SE bank not found");
        return;
    };
    let mut parsed = Nus3bankFile::open(&src).unwrap();
    let hex = parsed.tracks[0].hex_id.clone();
    parsed.remove_track(&hex).unwrap();
    let out = unique_temp_path("se_remove.nus3bank");
    parsed.save(&out).unwrap();
    let reparsed = Nus3bankFile::open(&out).unwrap();
    let _ = std::fs::remove_file(&out);

    assert_eq!(reparsed.tone.tones.len(), 86);
    assert_eq!(reparsed.dton.as_ref().unwrap().tones.len(), 1);
    assert_eq!(reparsed.tracks.len(), 17);
}

#[test]
fn original_se_bank_replace_keeps_identity() {
    let Some(src) = original_se_bank_path() else {
        eprintln!("skip: original Wing Zero SE bank not found");
        return;
    };
    let mut parsed = Nus3bankFile::open(&src).unwrap();
    let hex = parsed.tracks[0].hex_id.clone();
    let name = parsed.tracks[0].name.clone();
    let mut wav = minimal_wav_bytes();
    wav.extend_from_slice(&[1, 2, 3, 4]);
    parsed.replace_track_data(&hex, wav.clone()).unwrap();
    let out = unique_temp_path("se_replace.nus3bank");
    parsed.save(&out).unwrap();
    let reparsed = Nus3bankFile::open(&out).unwrap();
    let _ = std::fs::remove_file(&out);

    assert_eq!(reparsed.tone.tones.len(), 86);
    assert_eq!(reparsed.dton.as_ref().unwrap().tones.len(), 1);
    let track = reparsed.get_track_by_hex_id(&hex).unwrap();
    assert_eq!(track.name, name);
    assert_eq!(track.audio_data.as_ref().unwrap(), &wav);
    assert_live_pack_16_aligned(&reparsed);
}

#[test]
fn unshifted_name_pos_accepts_se_flags0_variants_not_bgm() {
    fn header(flags0: u32, flags1: u32, name: &str) -> Vec<u8> {
        let mut raw = vec![0u8; 120];
        raw[4..8].copy_from_slice(&flags0.to_le_bytes());
        raw[8..12].copy_from_slice(&flags1.to_le_bytes());
        let nlen = (name.len() + 1) as u8;
        raw[12] = nlen;
        raw[13..13 + name.len()].copy_from_slice(name.as_bytes());
        raw[13 + name.len()] = 0;
        raw
    }

    let bgm = header(0x8427_FFFF, 0x000C_989F, "COLORS_Flow");
    assert_eq!(
        super::ob_tone_decode::unshifted_name_len_pos(&bgm),
        Some(12)
    );
    assert!(super::ob_tone_decode::is_bgm_tone_descriptor(&bgm));

    let se_link = header(0x8627_FFFF, 0x000C_981F, "SE_CHR_654_link01");
    assert_eq!(
        super::ob_tone_decode::unshifted_name_len_pos(&se_link),
        Some(12)
    );
    assert_eq!(super::ob_tone_decode::ob_name_len_pos(&se_link), Some(12));
    assert!(!super::ob_tone_decode::is_bgm_tone_descriptor(&se_link));

    let se_bfff = header(0x8427_BFFF, 0x000C_981F, "SE_CHR_654_08");
    assert_eq!(
        super::ob_tone_decode::unshifted_name_len_pos(&se_bfff),
        Some(12)
    );
    assert!(!super::ob_tone_decode::is_bgm_tone_descriptor(&se_bfff));

    // flags0 >= 0: name lives at +8 (se_chr_654 0x7F family), not +12.
    let name_at_8 = header(0x0000_007F, 0x5F45_5322, "XXXXXXXX");
    assert_eq!(
        super::ob_tone_decode::unshifted_name_len_pos(&name_at_8),
        None
    );

    // song_wgnmd1 stores a hash here, not an OB live-descriptor flags0.
    let song_hash = header(0x84F7_BFFF, 0x000C_981F, "song_wgnmd1");
    assert_eq!(
        super::ob_tone_decode::unshifted_name_len_pos(&song_hash),
        None
    );
}

#[test]
fn bundled_se_bank_opens_unshifted_flag_variants() {
    let Some(src) = bundled_se_bank_path() else {
        eprintln!("skip: bundled se_chr_654gexvs2 bank not found");
        return;
    };
    let parsed = Nus3bankFile::open(&src).unwrap();
    assert_eq!(parsed.tone.tones.len(), 483);
    let live: Vec<_> = parsed
        .tone
        .tones
        .iter()
        .filter(|t| !t.removed && t.offset >= 0 && t.size > 0)
        .collect();
    assert!(
        live.len() >= 70,
        "expected many live cues, got {}",
        live.len()
    );
    assert!(
        live.iter()
            .any(|t| t.name.contains("link01") && t.name_len_pos == Some(12)),
        "0x8627FFFF cues must parse name at +12"
    );
    for t in &live {
        assert!(
            (t.offset as u64) + (t.size as u64) <= parsed.pack.data.len() as u64,
            "live cue {} PACK {}+{} overruns {}",
            t.name,
            t.offset,
            t.size,
            parsed.pack.data.len()
        );
    }
}

#[test]
fn bundled_se_bank_add_does_not_grow_dton_or_slots() {
    let Some(src) = bundled_se_bank_path() else {
        eprintln!("skip: bundled se_chr_654gexvs2 bank not found");
        return;
    };
    let mut parsed = Nus3bankFile::open(&src).unwrap();
    let slots = parsed.tone.tones.len();
    let dton_len = parsed.dton.as_ref().map(|d| d.tones.len()).unwrap_or(0);
    assert!(dton_len >= 1);
    parsed
        .add_track("new_test_audio".to_string(), minimal_wav_bytes())
        .unwrap();
    let out = unique_temp_path("bundled_se_add.nus3bank");
    parsed.save(&out).unwrap();
    let reparsed = Nus3bankFile::open(&out).unwrap();
    let _ = std::fs::remove_file(&out);

    assert_eq!(reparsed.tone.tones.len(), slots);
    assert_eq!(
        reparsed.dton.as_ref().map(|d| d.tones.len()).unwrap_or(0),
        dton_len
    );
    assert!(reparsed.tracks.iter().any(|t| t.name == "new_test_audio"));
    assert_live_pack_16_aligned(&reparsed);
}

#[test]
fn add_track_at_honors_reserved_index_out_of_order() {
    let mut file = make_sample_file();
    file.rebuild_tracks_view();

    let later = file
        .add_track_at(3, "later".to_string(), minimal_wav_bytes())
        .unwrap();
    assert_eq!(later, "0x3");
    assert!(file.tone.tones[2].removed);

    let earlier = file
        .add_track_at(2, "earlier".to_string(), minimal_wav_bytes())
        .unwrap();
    assert_eq!(earlier, "0x2");
    assert_eq!(file.tone.tones[2].name, "earlier");
    assert_eq!(file.tone.tones[3].name, "later");
    assert!(!file.tone.tones[2].removed);
    assert!(!file.tone.tones[3].removed);
}

#[test]
fn add_track_at_rejects_occupied_slot() {
    let mut file = make_sample_file();
    file.rebuild_tracks_view();
    let err = file
        .add_track_at(0, "taken".to_string(), minimal_wav_bytes())
        .unwrap_err();
    assert!(
        err.to_string().contains("already occupied"),
        "unexpected error: {err}"
    );
}

#[test]
fn register_add_uses_real_tone_slot_not_temp_id() {
    let mut file = make_sample_file();
    file.rebuild_tracks_view();
    let path = unique_temp_path("register_add_slot.nus3bank");
    file.save(&path).unwrap();
    let path_str = path.to_string_lossy().to_string();
    Nus3bankReplacer::clear_for_file(&path_str);

    let hex =
        Nus3bankReplacer::register_add(&path_str, "new_test_audio", minimal_wav_bytes()).unwrap();
    assert_eq!(hex, "0x2");
    assert!(!hex.contains("8000"));

    let ops = Nus3bankReplacer::operations_for_file(&path_str);
    assert_eq!(ops.len(), 1);
    match &ops[0] {
        ReplaceOperation::Add(name, hid, _) => {
            assert_eq!(name, "new_test_audio");
            assert_eq!(hid, "0x2");
        }
        other => panic!("expected Add, got {other:?}"),
    }

    Nus3bankReplacer::clear_for_file(&path_str);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn replace_pending_add_merges_instead_of_replace_op() {
    let mut file = make_sample_file();
    file.rebuild_tracks_view();
    let path = unique_temp_path("replace_pending_add.nus3bank");
    file.save(&path).unwrap();
    let path_str = path.to_string_lossy().to_string();
    Nus3bankReplacer::clear_for_file(&path_str);

    let hex =
        Nus3bankReplacer::register_add(&path_str, "new_test_audio", minimal_wav_bytes()).unwrap();
    let mut gained = minimal_wav_bytes();
    gained.extend_from_slice(&[9, 9, 9, 9]);
    Nus3bankReplacer::replace_track_in_memory(&path_str, &hex, gained.clone()).unwrap();

    let ops = Nus3bankReplacer::operations_for_file(&path_str);
    assert_eq!(ops.len(), 1, "replace must merge into the pending Add");
    match &ops[0] {
        ReplaceOperation::Add(name, hid, data) => {
            assert_eq!(name, "new_test_audio");
            assert_eq!(hid, &hex);
            assert_eq!(data, &gained);
        }
        other => panic!("expected merged Add, got {other:?}"),
    }

    Nus3bankReplacer::clear_for_file(&path_str);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn apply_add_then_replace_does_not_fail_track_not_found() {
    let mut file = make_sample_file();
    file.rebuild_tracks_view();
    let path = unique_temp_path("add_then_replace_save.nus3bank");
    file.save(&path).unwrap();
    let path_str = path.to_string_lossy().to_string();
    Nus3bankReplacer::clear_for_file(&path_str);

    let hex =
        Nus3bankReplacer::register_add(&path_str, "new_test_audio", minimal_wav_bytes()).unwrap();
    let mut gained = minimal_wav_bytes();
    gained.extend_from_slice(&[1, 2, 3, 4]);
    // UI save path also feeds replacement bytes after skipping ADD_ keys.
    Nus3bankReplacer::replace_track_in_memory(&path_str, &hex, gained.clone()).unwrap();

    let mut parsed = Nus3bankFile::open(&path).unwrap();
    Nus3bankReplacer::apply_to_file(&path_str, &mut parsed).unwrap();

    let track = parsed
        .get_track_by_hex_id(&hex)
        .expect("added track should exist after save");
    assert_eq!(track.name, "new_test_audio");
    assert_eq!(track.audio_data.as_ref().unwrap(), &gained);

    Nus3bankReplacer::clear_for_file(&path_str);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn two_pending_adds_keep_reserved_slots_when_applied_by_name() {
    let mut file = make_sample_file();
    file.rebuild_tracks_view();
    let path = unique_temp_path("two_pending_adds.nus3bank");
    file.save(&path).unwrap();
    let path_str = path.to_string_lossy().to_string();
    Nus3bankReplacer::clear_for_file(&path_str);

    let zebra = Nus3bankReplacer::register_add(&path_str, "zebra", minimal_wav_bytes()).unwrap();
    let apple = Nus3bankReplacer::register_add(&path_str, "apple", minimal_wav_bytes()).unwrap();
    assert_eq!(zebra, "0x2");
    assert_eq!(apple, "0x3");

    let mut parsed = Nus3bankFile::open(&path).unwrap();
    Nus3bankReplacer::apply_to_file(&path_str, &mut parsed).unwrap();

    assert_eq!(parsed.get_track_by_hex_id("0x2").unwrap().name, "zebra");
    assert_eq!(parsed.get_track_by_hex_id("0x3").unwrap().name, "apple");

    Nus3bankReplacer::clear_for_file(&path_str);
    let _ = std::fs::remove_file(&path);
}

fn parse_bnsf_sfmt(payload: &[u8]) -> (u16, u32, u32, u16, u16) {
    assert!(payload.starts_with(b"BNSF"));
    assert_eq!(&payload[8..12], b"IS14");
    assert_eq!(&payload[12..16], b"sfmt");
    let flags_ch = u32::from_be_bytes(payload[20..24].try_into().unwrap());
    let rate = u32::from_be_bytes(payload[24..28].try_into().unwrap());
    let nsamp = u32::from_be_bytes(payload[28..32].try_into().unwrap());
    let bsz = u16::from_be_bytes(payload[36..38].try_into().unwrap());
    let bsam = u16::from_be_bytes(payload[38..40].try_into().unwrap());
    ((flags_ch & 0xFFFF) as u16, rate, nsamp, bsz, bsam)
}

#[test]
fn wav_save_encodes_bnsf_is14() {
    if !super::bnsf::encode_exe_available() {
        panic!("tools/encode.exe is required to write EXVS2 character SE");
    }
    let wav_path = unique_temp_path("bnsf_src.wav");
    super::bnsf::write_minimal_wav_sine(&wav_path, 44100, 22050);
    let wav = std::fs::read(&wav_path).unwrap();
    let bnsf = super::bnsf::wav_to_bnsf_is14(&wav).unwrap();
    let _ = std::fs::remove_file(&wav_path);

    assert!(
        super::bnsf::is_bnsf_is14(&bnsf),
        "payload must be BNSF/IS14"
    );
    let (ch, rate, nsamp, bsz, bsam) = parse_bnsf_sfmt(&bnsf);
    assert_eq!(ch, 1);
    assert_eq!(rate, 48000);
    assert!(nsamp > 0);
    assert_eq!(bsz, 120);
    assert_eq!(bsam, 640);
    assert_ne!(&bnsf[..4], b"RIFF");
}

#[test]
fn apply_add_wav_writes_bnsf_not_riff() {
    if !super::bnsf::encode_exe_available() {
        panic!("tools/encode.exe is required to write EXVS2 character SE");
    }
    let mut file = make_sample_file();
    file.rebuild_tracks_view();
    let path = unique_temp_path("add_wav_to_bnsf.nus3bank");
    file.save(&path).unwrap();
    let path_str = path.to_string_lossy().to_string();
    Nus3bankReplacer::clear_for_file(&path_str);

    let wav_path = unique_temp_path("add_src.wav");
    super::bnsf::write_minimal_wav_sine(&wav_path, 48000, 640 * 4);
    let wav = std::fs::read(&wav_path).unwrap();
    let hex = Nus3bankReplacer::register_add(&path_str, "new_test_audio", wav).unwrap();

    let mut parsed = Nus3bankFile::open(&path).unwrap();
    Nus3bankReplacer::apply_to_file(&path_str, &mut parsed).unwrap();
    let track = parsed.get_track_by_hex_id(&hex).unwrap();
    let payload = track.audio_data.as_ref().expect("added payload");
    assert!(
        super::bnsf::is_bnsf_is14(payload),
        "saved PACK must be BNSF/IS14, got {:?}",
        &payload[..payload.len().min(12)]
    );

    Nus3bankReplacer::clear_for_file(&path_str);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wav_path);
}

#[test]
fn nus3bank_save_converts_existing_wav_pack_payload() {
    if !super::bnsf::encode_exe_available() {
        panic!("tools/encode.exe is required to write EXVS2 character SE");
    }
    let mut file = make_sample_file();
    file.rebuild_tracks_view();
    let hex = file.tracks[0].hex_id.clone();
    let wav_path = unique_temp_path("leftover_src.wav");
    super::bnsf::write_minimal_wav_sine(&wav_path, 48000, 640 * 4);
    let wav = std::fs::read(&wav_path).unwrap();
    file.replace_track_data(&hex, wav).unwrap();
    assert!(
        file.get_track_by_hex_id(&hex)
            .unwrap()
            .audio_data
            .as_ref()
            .unwrap()
            .starts_with(b"RIFF")
    );
    let path = unique_temp_path("leftover_wav.nus3bank");
    file.save(&path).unwrap();
    let path_str = path.to_string_lossy().to_string();
    Nus3bankReplacer::clear_for_file(&path_str);

    let mut parsed = Nus3bankFile::open(&path).unwrap();
    Nus3bankReplacer::apply_to_file(&path_str, &mut parsed).unwrap();
    parsed.save(&path).unwrap();

    let reparsed = Nus3bankFile::open(&path).unwrap();
    let payload = reparsed
        .get_track_by_hex_id(&hex)
        .unwrap()
        .audio_data
        .as_ref()
        .unwrap();
    assert!(
        super::bnsf::is_bnsf_is14(payload),
        "open+save must convert leftover WAV, got {:?}",
        &payload[..payload.len().min(12)]
    );

    Nus3bankReplacer::clear_for_file(&path_str);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wav_path);
}

fn looping_clock_raw_meta() -> Vec<u8> {
    let mut v = vec![0u8; 220];
    v.extend_from_slice(&[0x80, 0xBB, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00]);
    v.extend_from_slice(&172148i32.to_le_bytes());
    v.extend_from_slice(&112640i32.to_le_bytes());
    v.extend_from_slice(&172147i32.to_le_bytes());
    v.extend_from_slice(&1i32.to_le_bytes());
    v.extend_from_slice(&0i32.to_le_bytes());
    v.extend_from_slice(&0i32.to_le_bytes());
    v.extend_from_slice(&(-1i32).to_le_bytes());
    v.extend_from_slice(&4i32.to_le_bytes());
    v
}

fn tone_with_raw_meta(raw_meta: Vec<u8>) -> ToneMeta {
    ToneMeta {
        meta_prefix: Vec::new(),
        raw_meta,
        pack_offset_field_pos: None,
        pack_size_field_pos: None,
        name_len_pos: None,
        descriptor_words: Vec::new(),
        hash: 0,
        unk1: 0x000C_989F,
        name: "new_test_audio".to_string(),
        reserved0: 0,
        reserved8: 8,
        offset: 0,
        size: 0,
        param: [0.0; 12],
        offsets: Vec::new(),
        unkvalues: Vec::new(),
        unkvalues_pair_order: UnkvaluesPairOrder::IndexThenValue,
        unkending: vec![-1],
        end: Vec::new(),
        payload: Vec::new(),
        meta_size: 0,
        removed: false,
    }
}

#[test]
fn patch_sample_clock_overwrites_cloned_loop_window() {
    let mut tone = tone_with_raw_meta(looping_clock_raw_meta());
    assert!(tone.patch_sample_clock(36362, 0, 0, 0));
    let pos = 220 + 8;
    assert_eq!(
        i32::from_le_bytes(tone.raw_meta[pos..pos + 4].try_into().unwrap()),
        36362
    );
    assert_eq!(
        i32::from_le_bytes(tone.raw_meta[pos + 4..pos + 8].try_into().unwrap()),
        0
    );
    assert_eq!(
        i32::from_le_bytes(tone.raw_meta[pos + 8..pos + 12].try_into().unwrap()),
        0
    );
    assert_eq!(
        i32::from_le_bytes(tone.raw_meta[pos + 12..pos + 16].try_into().unwrap()),
        0
    );
    assert_eq!(
        tone.clock_pad_count(),
        Some(1),
        "one-shot must drop the extra looping pad"
    );
    let term = pos + 16 + 4;
    assert_eq!(
        i32::from_le_bytes(tone.raw_meta[term..term + 4].try_into().unwrap()),
        -1
    );
    assert_eq!(
        i32::from_le_bytes(tone.raw_meta[term + 4..term + 8].try_into().unwrap()),
        4
    );
}

#[test]
fn patch_sample_clock_repairs_oneshot_values_with_looping_pads() {
    let mut raw = vec![0u8; 220];
    raw.extend_from_slice(&[0x80, 0xBB, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00]);
    raw.extend_from_slice(&36362i32.to_le_bytes());
    raw.extend_from_slice(&0i32.to_le_bytes());
    raw.extend_from_slice(&0i32.to_le_bytes());
    raw.extend_from_slice(&0i32.to_le_bytes());
    raw.extend_from_slice(&0i32.to_le_bytes());
    raw.extend_from_slice(&0i32.to_le_bytes());
    raw.extend_from_slice(&(-1i32).to_le_bytes());
    let mut tone = tone_with_raw_meta(raw);
    assert_eq!(tone.clock_pad_count(), Some(2));
    assert!(tone.patch_sample_clock(36362, 0, 0, 0));
    assert_eq!(tone.clock_pad_count(), Some(1));
    assert_eq!(tone.sample_clock_from_raw_meta(), Some((36362, 0, 0, 0)));
}

#[test]
fn bank_needs_save_repair_when_cloned_loop_clock_remains() {
    if !super::bnsf::encode_exe_available() {
        panic!("tools/encode.exe is required to write EXVS2 character SE");
    }
    let mut file = make_sample_file();
    file.rebuild_tracks_view();
    let wav_path = unique_temp_path("repair_detect.wav");
    super::bnsf::write_minimal_wav_sine(&wav_path, 48000, 640 * 4);
    let wav = std::fs::read(&wav_path).unwrap();
    let bnsf = super::bnsf::wav_to_bnsf_is14(&wav).unwrap();
    let clock = super::bnsf::parse_bnsf_clock(&bnsf).unwrap();
    file.replace_track_data(&file.tracks[0].hex_id.clone(), bnsf)
        .unwrap();
    let mut raw = vec![0u8; 220];
    raw.extend_from_slice(&[0x80, 0xBB, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00]);
    raw.extend_from_slice(&172148i32.to_le_bytes());
    raw.extend_from_slice(&112640i32.to_le_bytes());
    raw.extend_from_slice(&172147i32.to_le_bytes());
    raw.extend_from_slice(&1i32.to_le_bytes());
    file.tone.tones[0].raw_meta = raw;
    file.tone.tones[0].unk1 = 0x000C_981F;
    assert!(
        crate::nus3bank::replace::Nus3bankReplacer::bank_needs_save_repair(&file),
        "cloned loop window must count as save-repair even with no pending add"
    );
    assert_eq!(clock.loop_flag, 0);
    let _ = std::fs::remove_file(&wav_path);
}

fn handmade_oneshot_bnsf(n_samples: u32) -> Vec<u8> {
    let sdat = vec![0u8; 120];
    let size_field = sdat.len() as u32 + 40;
    let mut out = Vec::new();
    out.extend_from_slice(b"BNSF");
    out.extend_from_slice(&size_field.to_be_bytes());
    out.extend_from_slice(b"IS14");
    out.extend_from_slice(b"sfmt");
    out.extend_from_slice(&0x14u32.to_be_bytes());
    out.extend_from_slice(&1u32.to_be_bytes());
    out.extend_from_slice(&48000u32.to_be_bytes());
    out.extend_from_slice(&n_samples.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&120u16.to_be_bytes());
    out.extend_from_slice(&640u16.to_be_bytes());
    out.extend_from_slice(b"sdat");
    out.extend_from_slice(&(sdat.len() as u32).to_be_bytes());
    out.extend_from_slice(&sdat);
    out
}

#[test]
fn bank_needs_save_repair_when_oneshot_keeps_looping_clock_record() {
    let mut file = make_sample_file();
    file.rebuild_tracks_view();
    let bnsf = handmade_oneshot_bnsf(36362);
    file.replace_track_data(&file.tracks[0].hex_id.clone(), bnsf)
        .unwrap();
    let mut raw = vec![0u8; 220];
    raw.extend_from_slice(&[0x80, 0xBB, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00]);
    raw.extend_from_slice(&36362i32.to_le_bytes());
    raw.extend_from_slice(&0i32.to_le_bytes());
    raw.extend_from_slice(&0i32.to_le_bytes());
    raw.extend_from_slice(&0i32.to_le_bytes());
    raw.extend_from_slice(&0i32.to_le_bytes());
    raw.extend_from_slice(&0i32.to_le_bytes());
    raw.extend_from_slice(&(-1i32).to_le_bytes());
    file.tone.tones[0].raw_meta = raw;
    file.tone.tones[0].unk1 = 0x000C_981F;
    assert!(
        crate::nus3bank::replace::Nus3bankReplacer::bank_needs_save_repair(&file),
        "one-shot values with looping pad count must still enable Save"
    );
}

#[test]
fn one_shot_bnsf_clears_cloned_loop_unk1_bit() {
    if !super::bnsf::encode_exe_available() {
        panic!("tools/encode.exe is required to write EXVS2 character SE");
    }
    let wav_path = unique_temp_path("oneshot.wav");
    super::bnsf::write_minimal_wav_sine(&wav_path, 48000, 640 * 4);
    let wav = std::fs::read(&wav_path).unwrap();
    let bnsf = super::bnsf::wav_to_bnsf_is14(&wav).unwrap();
    assert!(!super::bnsf::bnsf_has_loop_chunk(&bnsf));

    let mut file = make_sample_file();
    file.rebuild_tracks_view();
    let hex = file.tracks[0].hex_id.clone();
    file.replace_track_data(&hex, bnsf).unwrap();
    file.tone.tones[0].set_unk1(0x000C_989F);
    assert_ne!(file.tone.tones[0].unk1 & 0x80, 0);

    let path = unique_temp_path("oneshot_unk1.nus3bank");
    file.save(&path).unwrap();
    let path_str = path.to_string_lossy().to_string();
    Nus3bankReplacer::clear_for_file(&path_str);

    let mut parsed = Nus3bankFile::open(&path).unwrap();
    parsed.tone.tones[0].set_unk1(0x000C_989F);
    Nus3bankReplacer::apply_to_file(&path_str, &mut parsed).unwrap();
    assert_eq!(
        parsed.tone.tones[0].unk1 & 0x80,
        0,
        "one-shot BNSF must drop loop bit 0x80, unk1=0x{:x}",
        parsed.tone.tones[0].unk1 as u32
    );

    parsed.save(&path).unwrap();
    let reparsed = Nus3bankFile::open(&path).unwrap();
    assert_eq!(reparsed.tone.tones[0].unk1 & 0x80, 0);
    let clock = super::bnsf::parse_bnsf_clock(reparsed.tone.tones[0].payload.as_slice()).unwrap();
    let raw = &reparsed.tone.tones[0].raw_meta;
    let marker = [0x80u8, 0xBB, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00];
    if let Some(pos) = raw.windows(8).position(|w| w == marker) {
        let nsamp = i32::from_le_bytes(raw[pos + 8..pos + 12].try_into().unwrap());
        let lstart = i32::from_le_bytes(raw[pos + 12..pos + 16].try_into().unwrap());
        let lend = i32::from_le_bytes(raw[pos + 16..pos + 20].try_into().unwrap());
        let lflag = i32::from_le_bytes(raw[pos + 20..pos + 24].try_into().unwrap());
        assert_eq!(nsamp, clock.n_samples);
        assert_eq!(lstart, 0);
        assert_eq!(lend, 0);
        assert_eq!(lflag, 0);
    }

    Nus3bankReplacer::clear_for_file(&path_str);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wav_path);
}

fn decode_hex(hex: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(hex.len() / 2);
    let bytes = hex.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        let hi = hex_nibble(bytes[i]);
        let lo = hex_nibble(bytes[i + 1]);
        out.push((hi << 4) | lo);
        i += 2;
    }
    out
}

fn hex_nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => panic!("bad hex"),
    }
}

#[test]
fn crashing_editor_tone14_fails_ob_probe() {
    let raw = decode_hex(
        "00000000ffffffff00000000ffff67841f980c000f6e65775f746573745f617564696f000000000008000000003c0600e81a00000000004000000000000000000000803f0000803f0000b4c2000000000000803f000000001400000000000000030000000500000067080000ca0900006d08000002000000130a00000a000000000000000000b4c20100000000000000020000000000b4c2030000000000b4c2040000000000b4c2050000000000b4c2060000000000b4c2070000000000b4c2080000000000b4c2090000000000b4c2090000000000803f0000803f0000000001000000ffffffff80bb0000010000000a8e000000000000000000000000000000000000ffffffff04000000ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff0400000000000000ffffffff00000000ffffffff00000000ffffffff",
    );
    assert_eq!(raw.len(), 328);
    assert!(super::ob_tone_decode::looks_like_ob_descriptor(&raw));
    assert_eq!(raw[20], 0x0f, "old editor stored strlen without the NUL");
    let name_end = 20 + ((0x0fu8 as usize + 4) & !3);
    assert_eq!(
        &raw[name_end..name_end + 8],
        &[0, 0, 0, 0, 8, 0, 0, 0],
        "old splice left type=0 extra_len=8 instead of reserved0/reserved8"
    );
}

#[test]
fn vanilla_tone13_probes_and_renames_without_breaking_mask() {
    let raw = decode_hex(
        "00000000ffffffff00000000ffff67849f980c001653455f4348525f47554e575f30315a45524f5f30380000000000000800000030000600c83b00000000c03f00000000000000000000803f0000803f0000b4c2000000000000803f000000001400000000000000030000000400000067080000130a00006d080000020000000a000000000000000000b4c20100000000000000020000000000b4c2030000000000b4c2040000000000b4c2050000000000b4c2060000000000b4c2070000000000b4c2080000000000b4c2090000000000b4c2090000000000803f0000803f0000000001000000ffffffff80bb000001000000ef3c0100006e0000ee3c0100010000000000000000000000ffffffff04000000ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff0400000000000000ffffffff00000000ffffffff00000000ffffffff",
    );
    assert_eq!(raw.len(), 336);
    super::ob_tone_decode::probe_tone_record(&raw).unwrap();
    let name_at = super::ob_tone_decode::ob_name_len_pos(&raw).unwrap();
    assert_eq!(raw[name_at] as usize, "SE_CHR_GUNW_01ZERO_08".len() + 1);
    let mut tone = tone_with_raw_meta(raw);
    tone.name_len_pos = Some(name_at);
    tone.rewrite_name_in_raw_meta("new_test_audio").unwrap();
    assert_eq!(tone.name, "new_test_audio");
    super::ob_tone_decode::probe_live_tone(&tone.raw_meta, &tone.name).unwrap();
    let new_at = tone.name_len_pos.unwrap();
    assert_eq!(tone.raw_meta[new_at] as usize, "new_test_audio".len() + 1);
}

#[test]
fn vanilla_live_tone_probes_ok_if_present() {
    let Some(src) = original_se_bank_path() else {
        eprintln!("skip: original Wing Zero SE bank not found");
        return;
    };
    let parsed = Nus3bankFile::open(&src).unwrap();
    let live = parsed
        .tone
        .tones
        .iter()
        .find(|t| !t.removed && t.raw_meta.len() >= 104)
        .expect("expected a live OB tone");
    assert!(
        live.name_len_pos.is_some(),
        "OB parser must record the real name offset"
    );
    let pos = live.name_len_pos.unwrap();
    assert_eq!(live.raw_meta[pos] as usize, live.name.len() + 1);
    super::ob_tone_decode::probe_live_tone(&live.raw_meta, &live.name).unwrap();
}

fn bgm_update_02_backup_path() -> Option<PathBuf> {
    let p =
        PathBuf::from(r"E:\XB\mod\090sound\bgm_ac27_update_02\BGM_AC27_UPDATE_02.nus3bank.backup");
    p.exists().then_some(p)
}

#[test]
fn bgm_update_02_live_cues_are_bgm_descriptors() {
    let Some(src) = bgm_update_02_backup_path() else {
        eprintln!("skip: BGM_AC27_UPDATE_02 backup not found");
        return;
    };
    let parsed = Nus3bankFile::open(&src).unwrap();
    assert_eq!(parsed.tone.tones.len(), 3);
    assert!(parsed.tone.tones[0].removed || parsed.tone.tones[0].raw_meta.len() < 104);
    for t in parsed.tone.tones.iter().filter(|t| !t.removed) {
        let raw = &t.raw_meta;
        assert!(
            super::ob_tone_decode::is_bgm_tone_descriptor(raw),
            "live cue {} should use the BGM flags0/flags1/name layout (len={} b12={:02x} d4={:08x} d8={:08x} d12={:08x})",
            t.name,
            raw.len(),
            raw.get(12).copied().unwrap_or(0),
            u32::from_le_bytes(
                raw.get(4..8)
                    .unwrap_or(&[0; 4])
                    .try_into()
                    .unwrap_or([0; 4])
            ),
            u32::from_le_bytes(
                raw.get(8..12)
                    .unwrap_or(&[0; 4])
                    .try_into()
                    .unwrap_or([0; 4])
            ),
            u32::from_le_bytes(
                raw.get(12..16)
                    .unwrap_or(&[0; 4])
                    .try_into()
                    .unwrap_or([0; 4])
            ),
        );
        assert_eq!(t.name_len_pos, Some(12));
        super::ob_tone_decode::probe_live_tone(&t.raw_meta, &t.name).unwrap();
    }
}

#[test]
fn bgm_update_02_add_appends_index_3_keeps_name_slot() {
    let Some(src) = bgm_update_02_backup_path() else {
        eprintln!("skip: BGM_AC27_UPDATE_02 backup not found");
        return;
    };
    let mut parsed = Nus3bankFile::open(&src).unwrap();
    let donor_len = parsed.tone.tones[2].raw_meta.len();
    let donor_len_byte = parsed.tone.tones[2].raw_meta[12];
    let dton_before = parsed
        .dton
        .as_ref()
        .map(|d| d.tones.iter().map(|t| t.name.clone()).collect::<Vec<_>>());
    let id = parsed
        .add_track("COLORS_Flow".to_string(), minimal_wav_bytes())
        .unwrap();
    assert_eq!(id, "0x3");
    assert_eq!(parsed.tone.tones.len(), 4);
    assert!(parsed.tone.tones[0].raw_meta.len() < 104);
    assert_eq!(parsed.tone.tones[1].name, "vstg_ac_title_in_2000_v3");
    assert_eq!(parsed.tone.tones[2].name, "vstg_ac_title_in_2000_v2");
    let added = &parsed.tone.tones[3];
    assert_eq!(added.name, "COLORS_Flow");
    assert_eq!(added.raw_meta.len(), donor_len);
    assert_eq!(added.raw_meta[12], donor_len_byte);
    assert!(added.raw_meta[13..].starts_with(b"COLORS_Flow\0"));
    super::ob_tone_decode::probe_live_tone(&added.raw_meta, &added.name).unwrap();
    let dton_after = parsed
        .dton
        .as_ref()
        .map(|d| d.tones.iter().map(|t| t.name.clone()).collect::<Vec<_>>());
    assert_eq!(dton_before, dton_after);

    let out = unique_temp_path("bgm_add_out.nus3bank");
    parsed.save(&out).unwrap();
    let reparsed = Nus3bankFile::open(&out).unwrap();
    let _ = std::fs::remove_file(&out);
    assert_eq!(reparsed.tone.tones.len(), 4);
    assert_eq!(reparsed.tone.tones[3].name, "COLORS_Flow");
    assert_eq!(reparsed.tone.tones[3].raw_meta[12], donor_len_byte);
    super::ob_tone_decode::probe_live_tone(
        &reparsed.tone.tones[3].raw_meta,
        &reparsed.tone.tones[3].name,
    )
    .unwrap();
}

#[test]
fn bgm_editor_av_tone0_fails_probe() {
    let src = PathBuf::from(
        r"E:\XB\mod\090sound\bgm_ac27_update_02\BGM_AC27_UPDATE_02.nus3bank.editor-av",
    );
    if !src.exists() {
        eprintln!("skip: editor-av BGM bank not found");
        return;
    }
    let parsed = Nus3bankFile::open(&src).unwrap();
    let raw = &parsed.tone.tones[0].raw_meta;
    assert_eq!(parsed.tone.tones[0].name, "COLORS_Flow");
    assert!(
        super::ob_tone_decode::probe_bgm_tone_record(raw).is_err(),
        "Audio Editor Add+Save BGM cue must fail the strict probe"
    );
}
