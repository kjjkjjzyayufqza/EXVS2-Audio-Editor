use crate::TemplateApp;
use egui::{Context, ViewportCommand};
use nus3audio::Nus3audioFile;
use std::fs::File;
use std::io::Write;

/// Top menu panel component
pub struct TopPanel;

impl TopPanel {
    /// Display the top menu panel
    pub fn show(ctx: &Context, app: Option<&mut crate::TemplateApp>) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // Don't show Quit button in web environment
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Save .nus3audio").clicked() {
                            // Save current nus3audio file
                            // Use defer to avoid borrowing issues with egui
                            ui.ctx().request_repaint();

                            // Initialize file path
                            let mut selected_file_path = None;

                            // Get the selected file path from app if available
                            if let Some(app) = &app {
                                // Get the selected file path using the main_area accessor
                                let main_area = app.main_area();
                                if let Some(path) = &main_area.selected_file {
                                    selected_file_path = Some(path.to_string());
                                }
                            }

                            if selected_file_path.is_none() {
                                println!("No file selected to save");
                                return;
                            }

                            let is_web = cfg!(target_arch = "wasm32");
                            if !is_web {
                                // Native platform: use file dialog
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("NUS3AUDIO", &["nus3audio"])
                                    .set_file_name("output.nus3audio")
                                    .save_file()
                                {
                                    // Get path as string
                                    let path_str = path.to_string_lossy().to_string();

                                    // Execute save operation with selected file path
                                    if let Some(original_path) = selected_file_path {
                                        TopPanel::save_nus3audio_file(&original_path, &path_str);
                                    }
                                }
                            }
                        }
                    });
                }

                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        // Can display an about dialog here
                    }
                });
            });
        });
    }

    /// Save current audio files to a new nus3audio file
    fn save_nus3audio_file(original_path: &str, save_path: &str) {
        // Open the original nus3audio file
        match Nus3audioFile::open(original_path) {
            Ok(nus3_file) => {
                // Create a buffer to write the file
                let mut buffer = Vec::new();

                // Write nus3audio data to buffer
                nus3_file.write(&mut buffer);

                // Save buffer to file
                match File::create(save_path) {
                    Ok(mut file) => {
                        match file.write_all(&buffer) {
                            Ok(_) => {
                                println!("File save success: {}", save_path);

                                // Show a native message dialog for better user feedback
                                let _ = rfd::MessageDialog::new()
                                    .set_title("Save success")
                                    .set_description(&format!(
                                        "NUS3AUDIO has been success write to:\n{}",
                                        save_path
                                    ))
                                    .set_buttons(rfd::MessageButtons::Ok)
                                    .set_level(rfd::MessageLevel::Info)
                                    .show();
                            }
                            Err(e) => {
                                eprintln!("File save fail: {}", e);

                                // Show error dialog
                                let _ = rfd::MessageDialog::new()
                                    .set_title("Save Failed")
                                    .set_description(&format!("Can't write file: {}", e))
                                    .set_buttons(rfd::MessageButtons::Ok)
                                    .set_level(rfd::MessageLevel::Error)
                                    .show();
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Cre: {}", e);

                        // Show error dialog
                        let _ = rfd::MessageDialog::new()
                            .set_title("Save Failed")
                            .set_description(&format!("Can't write file: {}", e))
                            .set_buttons(rfd::MessageButtons::Ok)
                            .set_level(rfd::MessageLevel::Error)
                            .show();
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to open original nus3audio file: {}", e);

                // Show error dialog
                let _ = rfd::MessageDialog::new()
                    .set_title("Open Failed")
                    .set_description(&format!("Unable to open original NUS3AUDIO file: {}", e))
                    .set_buttons(rfd::MessageButtons::Ok)
                    .set_level(rfd::MessageLevel::Error)
                    .show();
            }
        }
    }
}
