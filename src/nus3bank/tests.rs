use std::path::PathBuf;

use super::structures::{
    BinfSection, DtonSection, GrpSection, JunkSection, Nus3bankFile, PropLayout, PropSection,
    TocEntry, ToneMeta, ToneSection, UnkvaluesPairOrder,
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
        0x52, 0x49, 0x46, 0x46, 0x24, 0x00, 0x00, 0x00, 0x57, 0x41, 0x56, 0x45, 0x66, 0x6d,
        0x74, 0x20, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x40, 0x1f, 0x00, 0x00,
        0x80, 0x3e, 0x00, 0x00, 0x02, 0x00, 0x10, 0x00, 0x64, 0x61, 0x74, 0x61, 0x00, 0x00,
        0x00, 0x00,
    ]
}

fn make_sample_file() -> Nus3bankFile {
    let toc = vec![
        TocEntry { magic: *b"PROP", size: 0 },
        TocEntry { magic: *b"BINF", size: 0 },
        TocEntry { magic: *b"GRP ", size: 0 },
        TocEntry { magic: *b"DTON", size: 0 },
        TocEntry { magic: *b"TONE", size: 0 },
        TocEntry { magic: *b"JUNK", size: 0 },
        TocEntry { magic: *b"PACK", size: 0 },
    ];

    let prop = PropSection {
        project: "DefaultProject".to_string(),
        timestamp: "2014/10/06 03:02:28".to_string(),
        unk1: 0xF1,
        reserved_u16: 0,
        unk2: 0x3,
        unk3: 0x8,
        layout: PropLayout::Extended,
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
        junk: Some(JunkSection { data: vec![0, 0, 0, 0] }),
        pack: Default::default(),
        unknown_sections: Vec::new(),
        tracks: Vec::new(),
        file_path: "in_memory".to_string(),
    }
}

fn declared_section_total_len(section: &[u8]) -> usize {
    assert!(section.len() >= 8);
    let declared_size = u32::from_le_bytes([section[4], section[5], section[6], section[7]]) as usize;
    8 + declared_size
}

fn dton_expected_float_counts(bytes: &[u8]) -> Vec<usize> {
    assert!(bytes.len() >= 12);
    let count = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let start = 12;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let base = 12 + i * 8;
        let off = u32::from_le_bytes([bytes[base], bytes[base + 1], bytes[base + 2], bytes[base + 3]]) as usize;
        let sz = u32::from_le_bytes([bytes[base + 4], bytes[base + 5], bytes[base + 6], bytes[base + 7]]) as usize;
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
fn parse_sample2_smoke_if_present() {
    let p = std::path::Path::new("sample2.nus3bank");
    if !p.exists() {
        return;
    }

    let parsed = Nus3bankFile::open(p).unwrap();
    assert!(parsed.tone.tones.len() > 0);
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
    assert!(parsed.tracks[0].audio_data.as_ref().unwrap().starts_with(b"RIFF"));
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
    let new_id = parsed.add_track("track_c".to_string(), minimal_wav_bytes()).unwrap();
    assert_eq!(new_id, "0x2");

    let out_path2 = unique_temp_path("add_out.nus3bank");
    parsed.save(&out_path2).unwrap();

    let reparsed = Nus3bankFile::open(&out_path2).unwrap();
    assert_eq!(reparsed.tracks.len(), 3);
    assert_eq!(reparsed.tracks[2].name, "track_c");
}

