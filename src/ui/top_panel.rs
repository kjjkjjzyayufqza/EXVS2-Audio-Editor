use crate::version_check;
use egui::Context;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use crate::ui::main_area::Nus3audioFileUtils;

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

// Using Lazy and Mutex for thread-safe access to modal info
static MODAL_INFO: Lazy<Mutex<ModalInfo>> = Lazy::new(|| {
    Mutex::new(ModalInfo::default())
});

// Helper functions to manage the modal state
fn show_modal(title: &str, message: &str, is_error: bool) {
    if let Ok(mut modal) = MODAL_INFO.lock() {
        modal.open = true;
        modal.title = title.to_string();
        modal.message = message.to_string();
        modal.is_error = is_error;
        modal.has_link = false;
        modal.link_text = String::new();
        modal.link_url = String::new();
    }
}

fn show_modal_with_link(title: &str, message: &str, link_text: &str, link_url: &str, is_error: bool) {
    if let Ok(mut modal) = MODAL_INFO.lock() {
        modal.open = true;
        modal.title = title.to_string();
        modal.message = message.to_string();
        modal.is_error = is_error;
        modal.has_link = true;
        modal.link_text = link_text.to_string();
        modal.link_url = link_url.to_string();
    }
}

/// Top menu panel component
pub struct TopPanel;

impl TopPanel {
    /// Display the top menu panel
    pub fn show(ctx: &Context, mut app: Option<&mut crate::TemplateApp>) {
        // Check for version updates
        TopPanel::check_for_updates(ctx);
        
        // Show modal dialog if needed
        let mut should_close_modal = false;
        let modal_data = if let Ok(modal) = MODAL_INFO.lock() {
            if modal.open {
                Some(modal.clone())
            } else {
                None
            }
        } else {
            None
        };
        
        if let Some(modal) = modal_data {
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
        
        // Update modal state after the window is displayed
        if should_close_modal {
            if let Ok(mut modal) = MODAL_INFO.lock() {
                modal.open = false;
            }
        }
        
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // Don't show Quit button in web environment
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Save Changes").clicked() {
                            // Save pending changes to the current nus3audio file
                            ui.ctx().request_repaint();

                            // Initialize file path
                            let mut selected_file_path = None;

                            // Extract path first without moving app
                            {
                                if let Some(app_ref) = app.as_ref() {
                                    let main_area = app_ref.main_area();
                                    if let Some(path) = &main_area.selected_file {
                                        selected_file_path = Some(path.to_string());
                                    }
                                }
                            }

                            if selected_file_path.is_none() {
                                println!("No file selected to save changes");
                                // Show error dialog
                                show_modal(
                                    "Save Failed",
                                    "No file selected to save changes to",
                                    true,
                                );
                                return;
                            }

                            // Check if there are any pending changes
                            if !Nus3audioFileUtils::has_pending_changes() {
                                println!("No pending changes to save");
                                // Show info dialog
                                show_modal(
                                    "No Changes",
                                    "There are no pending changes to save",
                                    false,
                                );
                                return;
                            }

                            // Save changes to the current file
                            if let Some(file_path) = selected_file_path {
                                match Nus3audioFileUtils::save_changes_to_file(&file_path) {
                                    Ok(_) => {
                                        println!("Changes saved successfully to: {}", file_path);
                                        
                                        // Show success dialog
                                        show_modal(
                                            "Save Successful",
                                            &format!("Successfully saved {} changes to:\n{}", 
                                                Nus3audioFileUtils::get_pending_changes_count(),
                                                file_path),
                                            false,
                                        );
                                        
                                        // Update UI if needed
                                        if let Some(app_mut) = app.as_mut() {
                                            // Refresh the file view by reloading it
                                            let main_area = app_mut.main_area_mut();
                                            main_area.update_selected_file(Some(file_path.clone()));
                                        }
                                    },
                                    Err(e) => {
                                        println!("Failed to save changes: {}", e);
                                        
                                        // Show error dialog
                                        show_modal(
                                            "Save Failed",
                                            &format!("Failed to save changes: {}", e),
                                            true,
                                        );
                                    }
                                }
                            }
                        }
                        
                        if ui.button("Save .nus3audio").clicked() {
                            // Save current nus3audio file
                            // Use defer to avoid borrowing issues with egui
                            ui.ctx().request_repaint();

                            // Initialize file path
                            let mut selected_file_path = None;

                            // Get the selected file path from app if available
                            if let Some(app_ref) = app.as_ref() {
                                // Get the selected file path using the main_area accessor
                                let main_area = app_ref.main_area();
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
                            &format!("EXVS2 Audio Editor\n\nVersion: {}\n\nA tool for editing audio files in EXVS2 game.", 
                                env!("CARGO_PKG_VERSION")),
                            "Source: https://github.com/kjjkjjzyayufqza/EXVS2-Audio-Editor",
                            "https://github.com/kjjkjjzyayufqza/EXVS2-Audio-Editor",
                            false
                        );
                    }
                });
            });
        });
    }

    /// Check for updates and show notification if a new version is available
    fn check_for_updates(_ctx: &Context) {
        // Only show update notice once per session
        static SHOWN_UPDATE_NOTICE: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));
        
        // If we've already shown the notice, don't do anything else
        if let Ok(shown) = SHOWN_UPDATE_NOTICE.lock() {
            if *shown {
                return;
            }
        }
        
        // Get version check result
        let version_result = version_check::get_version_check_result();
        
        // Try to lock the mutex
        let check_result = match version_result.try_lock() {
            Ok(guard) => {
                // Check if we have a result
                if let Some(result) = &*guard {
                    // Check if there's a new version
                    if result.has_new_version {
                        let current = result.current_version.clone();
                        let latest = result.latest_version.clone();
                        let url = result.download_url.clone();
                        
                        // Return the data we need
                        Some((current, latest, url))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Err(_) => None, // Couldn't lock the mutex
        };
        
        // Show the update notice if we have the data
        if let Some((current_version, latest_version, download_url)) = check_result {
            // Show update available notice
            show_modal_with_link(
                "Update Available",
                &format!("A new version of EXVS2 Audio Editor is available!\n\nCurrent version: {}\nLatest version: {}\n\nClick the link below to download:",
                    current_version, latest_version),
                "Download latest version",
                &download_url,
                false
            );
            
            // Mark that we've shown the notice
            if let Ok(mut shown) = SHOWN_UPDATE_NOTICE.lock() {
                *shown = true;
            }
        }
    }
    
    /// Save current audio files to a new file (supports both NUS3AUDIO and NUS3BANK)
    fn save_nus3audio_file(original_path: &str, save_path: &str) {
        // Use unified method to support both NUS3AUDIO and NUS3BANK files
        match crate::ui::main_area::ReplaceUtils::apply_replacements_and_save_unified(original_path, save_path) {
            Ok(_) => {
                println!("File save success: {}", save_path);
                
                // Show success modal dialog
                show_modal(
                    "Save success", 
                    &format!("Audio file has been successfully saved to:\n{}", save_path),
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
