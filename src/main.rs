#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    // Debug utility: export NUS3BANK as JSON and exit.
    // Usage:
    //   exvs2_audio_editor --debug-json <input.nus3bank> [output.json]
    //
    // Debug utility: normalize all embedded audio to standard PCM16 WAV and save.
    // Usage:
    //   exvs2_audio_editor --debug-convert-all-to-wav <input.nus3bank> [output.nus3bank]
    //   exvs2_audio_editor --debug-convert-all-to-wav <input.nus3bank> --output <output.nus3bank>
    // If output is omitted, the input file is overwritten.
    {
        #[cfg(windows)]
        use std::os::windows::process::CommandExt;
        use std::path::{Path, PathBuf};
        use std::process::Command;

        fn is_standard_pcm16_wav(data: &[u8]) -> bool {
            if data.len() < 12 {
                return false;
            }
            if &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
                return false;
            }

            let mut p = 12usize;
            while p + 8 <= data.len() {
                let cid = &data[p..p + 4];
                p += 4;
                let clen =
                    u32::from_le_bytes([data[p], data[p + 1], data[p + 2], data[p + 3]]) as usize;
                p += 4;

                if p + clen > data.len() {
                    return false;
                }

                if cid == b"fmt " {
                    if clen != 16 {
                        return false;
                    }
                    let w_format_tag = u16::from_le_bytes([data[p], data[p + 1]]);
                    let bits_per_sample = u16::from_le_bytes([data[p + 14], data[p + 15]]);
                    return w_format_tag == 1 && bits_per_sample == 16;
                }

                p += clen;
                if clen % 2 != 0 {
                    p += 1;
                }
            }

            false
        }

        fn convert_audio_bytes_to_pcm_wav(data: &[u8]) -> Result<Vec<u8>, String> {
            // Convert an embedded audio payload into a standard PCM WAV using vgmstream-cli.
            // This is used to normalize legacy WAV payloads that the game cannot decode
            // (e.g. WAVEFORMATEXTENSIBLE with a custom SubFormat GUID).
            let vgmstream_path = Path::new("tools").join("vgmstream-cli.exe");
            if !vgmstream_path.exists() {
                return Err(format!("vgmstream-cli not found at {:?}", vgmstream_path));
            }

            let temp_dir = std::env::temp_dir();
            let input_path = temp_dir.join("nus3bank_cli_in.wav");
            let output_path = temp_dir.join("nus3bank_cli_out_pcm.wav");

            std::fs::write(&input_path, data)
                .map_err(|e| format!("Failed to write temp input audio: {}", e))?;

            let mut command = Command::new(&vgmstream_path);
            #[cfg(windows)]
            {
                use winapi::um::winbase::CREATE_NO_WINDOW;
                command.creation_flags(CREATE_NO_WINDOW);
            }

            // -i: ignore any looping information (decode once)
            // -o: output WAV path
            let result = command
                .args([
                    "-i",
                    "-o",
                    &output_path.to_string_lossy(),
                    &input_path.to_string_lossy(),
                ])
                .output()
                .map_err(|e| format!("Failed to run vgmstream-cli: {}", e))?;

            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                let _ = std::fs::remove_file(&input_path);
                let _ = std::fs::remove_file(&output_path);
                return Err(format!("vgmstream-cli error: {}", stderr));
            }

            let wav_data = std::fs::read(&output_path)
                .map_err(|e| format!("Failed to read converted WAV data: {}", e))?;

            let _ = std::fs::remove_file(&input_path);
            let _ = std::fs::remove_file(&output_path);

            Ok(wav_data)
        }

        let argv: Vec<String> = std::env::args().skip(1).collect();
        let mut i = 0usize;
        while i < argv.len() {
            let a = &argv[i];

            if a == "--debug-json" {
                let input = argv
                    .get(i + 1)
                    .cloned()
                    .expect("Missing input path for --debug-json");
                let output = argv
                    .get(i + 2)
                    .cloned()
                    .unwrap_or_else(|| format!("{input}.json"));

                let file =
                    match exvs2_audio_editor::nus3bank::structures::Nus3bankFile::open(&input) {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("Error loading NUS3BANK file: {e:?}");
                            std::process::exit(1);
                        }
                    };
                let opt = exvs2_audio_editor::nus3bank::debug_json::DebugJsonOptions::default();
                if let Err(e) = exvs2_audio_editor::nus3bank::debug_json::write_debug_json_file(
                    &file, &opt, &output,
                ) {
                    eprintln!("Error writing debug JSON: {e:?}");
                    std::process::exit(1);
                }
                return Ok(());
            }

            if a == "--debug-convert-all-to-wav" || a == "--debug-convert-all-wav" {
                let input = argv
                    .get(i + 1)
                    .cloned()
                    .expect("Missing input path for --debug-convert-all-to-wav");

                let mut output: Option<PathBuf> = None;
                // Support:
                // - positional output: --debug-convert-all-to-wav in out
                // - named output:      --debug-convert-all-to-wav in --output out
                if let Some(next) = argv.get(i + 2) {
                    if next == "--output" {
                        output = argv.get(i + 3).map(|s| PathBuf::from(s));
                    } else if !next.starts_with("--") {
                        output = Some(PathBuf::from(next));
                    }
                }
                let output = output.unwrap_or_else(|| PathBuf::from(&input));

                let mut file =
                    match exvs2_audio_editor::nus3bank::structures::Nus3bankFile::open(&input) {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("Error loading NUS3BANK file: {e:?}");
                            std::process::exit(1);
                        }
                    };

                let mut converted = 0usize;
                let mut skipped = 0usize;
                let mut failed = 0usize;

                for (idx, tone) in file.tone.tones.iter_mut().enumerate() {
                    if tone.removed {
                        continue;
                    }

                    let hex_id = format!("0x{:x}", idx as u32);
                    let source = tone.payload.clone();
                    if is_standard_pcm16_wav(&source) {
                        skipped += 1;
                        continue;
                    }

                    match convert_audio_bytes_to_pcm_wav(&source) {
                        Ok(wav) => {
                            tone.payload = wav;
                            converted += 1;
                        }
                        Err(e) => {
                            failed += 1;
                            eprintln!("Convert failed for {}: {}", hex_id, e);
                        }
                    }
                }

                if let Err(e) = file.save(&output) {
                    eprintln!("Error saving NUS3BANK file: {e:?}");
                    std::process::exit(1);
                }

                println!(
                    "Debug convert done: converted={}, skipped={}, failed={}, output={}",
                    converted,
                    skipped,
                    failed,
                    output.to_string_lossy()
                );
                return Ok(());
            }

            i += 1;
        }
    }

    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 600.0])
            .with_min_inner_size([1000.0, 600.0])
            .with_icon(
                // NOTE: Adding an icon is optional
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "EXVS2 Audio Editor",
        native_options,
        Box::new(|cc| Ok(Box::new(exvs2_audio_editor::TemplateApp::new(cc)))),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Box::new(exvs2_audio_editor::TemplateApp::new(cc)).into()),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
