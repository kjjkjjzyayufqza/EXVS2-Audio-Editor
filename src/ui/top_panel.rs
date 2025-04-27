use crate::TemplateApp;
use crate::ui::main_area::ReplaceUtils;
use egui::{Context, ViewportCommand};
use nus3audio::Nus3audioFile;
use std::fs::File;
use std::io::Write;
use std::cell::RefCell;
use std::sync::Once;

// Modal dialog information
#[derive(Clone, Default)]
struct ModalInfo {
    open: bool,
    title: String,
    message: String,
    is_error: bool,
    has_link: bool,
    link_text: String,
    link_url: String,
}

// Use a simple type to store our modal state
static mut MODAL_INFO: Option<ModalInfo> = None;
static INIT: Once = Once::new();

// Helper functions to manage the modal state
fn init_modal() {
    INIT.call_once(|| {
        unsafe {
            MODAL_INFO = Some(ModalInfo::default());
        }
    });
}

fn show_modal(title: &str, message: &str, is_error: bool) {
    init_modal();
    unsafe {
        if let Some(modal) = &mut MODAL_INFO {
            modal.open = true;
            modal.title = title.to_string();
            modal.message = message.to_string();
            modal.is_error = is_error;
            modal.has_link = false;
            modal.link_text = String::new();
            modal.link_url = String::new();
        }
    }
}

fn show_modal_with_link(title: &str, message: &str, link_text: &str, link_url: &str, is_error: bool) {
    init_modal();
    unsafe {
        if let Some(modal) = &mut MODAL_INFO {
            modal.open = true;
            modal.title = title.to_string();
            modal.message = message.to_string();
            modal.is_error = is_error;
            modal.has_link = true;
            modal.link_text = link_text.to_string();
            modal.link_url = link_url.to_string();
        }
    }
}

/// Top menu panel component
pub struct TopPanel;

impl TopPanel {
    /// Display the top menu panel
    pub fn show(ctx: &Context, app: Option<&mut crate::TemplateApp>) {
        init_modal();
        
        // Show modal dialog if needed
        let mut should_close_modal = false;
        
        unsafe {
            if let Some(modal) = &MODAL_INFO {
                if modal.open {
                    egui::Window::new(&modal.title)
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .show(ctx, |ui| {
                            ui.label(&modal.message);
                            
                            if modal.has_link {
                                ui.hyperlink_to(&modal.link_text, &modal.link_url);
                            }
                            
                            ui.add_space(8.0);
                            
                            if ui.button("OK").clicked() {
                                should_close_modal = true;
                            }
                        });
                }
            }
        }
        
        // Update modal state after the window is displayed
        if should_close_modal {
            unsafe {
                if let Some(modal) = &mut MODAL_INFO {
                    modal.open = false;
                }
            }
        }
        
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
                            if let Some(current_app) = &app {
                                // Get the selected file path using the main_area accessor
                                let main_area = current_app.main_area();
                                if let Some(path) = &main_area.selected_file {
                                    selected_file_path = Some(path.to_string());
                                }
                            }

                            if selected_file_path.is_none() {
                                println!("No file selected to save");
                                // Show error dialog
                                show_modal(
                                    "Save Failed",
                                    "No file selected to save",
                                    true,
                                );
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
                                        // Pass the app instance to save_nus3audio_file
                                        TopPanel::save_nus3audio_file(&original_path, &path_str);
                                    }
                                }
                            }
                        }
                    });
                }

                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        // Show about modal with project information
                        show_modal_with_link(
                            "About EXVS2 Audio Editor",
                            "EXVS2 Audio Editor\n\nA tool for editing audio files in EXVS2 game.",
                            "Source: https://github.com/kjjkjjzyayufqza/EXVS2-Audio-Editor",
                            "https://github.com/kjjkjjzyayufqza/EXVS2-Audio-Editor",
                            false
                        );
                    }
                });
            });
        });
    }

    /// Save current audio files to a new nus3audio file
    fn save_nus3audio_file(original_path: &str, save_path: &str) {
        // Use ReplaceUtils to apply all in-memory replacements and save the file
        match crate::ui::main_area::ReplaceUtils::apply_replacements_and_save(original_path, save_path) {
            Ok(_) => {
                println!("File save success: {}", save_path);
                
                // Show success modal dialog
                show_modal(
                    "Save success", 
                    &format!("NUS3AUDIO has been success write to:\n{}", save_path),
                    false
                );
            }
            Err(e) => {
                eprintln!("File save fail: {}", e);
                
                // Show error dialog
                show_modal(
                    "Save Failed", 
                    &format!("Failed to save file: {}", e),
                    true
                );
            }
        }
    }
}
