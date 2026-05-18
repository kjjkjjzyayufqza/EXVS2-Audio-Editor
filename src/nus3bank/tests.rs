use std::path::PathBuf;

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
    assert!(parsed.tracks[0]
        .audio_data
        .as_ref()
        .unwrap()
        .starts_with(b"RIFF"));
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
