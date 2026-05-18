use crate::localized;
use crate::Locale;
use crate::ui::main_area::Nus3audioFileUtils;
use crate::version_check;
use egui::{Context, Id};
use once_cell::sync::Lazy;
use std::sync::Mutex;

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
static MODAL_INFO: Lazy<Mutex<ModalInfo>> = Lazy::new(|| Mutex::new(ModalInfo::default()));

// Helper functions to manage the modal state
fn show_modal(title: impl AsRef<str>, message: impl AsRef<str>, is_error: bool) {
    if let Ok(mut modal) = MODAL_INFO.lock() {
        modal.open = true;
        modal.title = title.as_ref().to_string();
        modal.message = message.as_ref().to_string();
        modal.is_error = is_error;
        modal.has_link = false;
        modal.link_text = String::new();
        modal.link_url = String::new();
    }
}

fn show_modal_with_link(
    title: impl AsRef<str>,
    message: impl AsRef<str>,
    link_text: impl AsRef<str>,
    link_url: impl AsRef<str>,
    is_error: bool,
) {
    if let Ok(mut modal) = MODAL_INFO.lock() {
        modal.open = true;
        modal.title = title.as_ref().to_string();
        modal.message = message.as_ref().to_string();
        modal.is_error = is_error;
        modal.has_link = true;
        modal.link_text = link_text.as_ref().to_string();
        modal.link_url = link_url.as_ref().to_string();
    }
}

/// Top menu panel component
pub struct TopPanel;

impl TopPanel {
    /// Display the top menu panel
    pub fn show(ctx: &Context, mut app: Option<&mut crate::TemplateApp>) {
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

                    if ui.button(localized::ok()).clicked() {
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
            egui::MenuBar::new().ui(ui, |ui| {
                // Don't show Quit button in web environment
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button(localized::file_menu(), |ui| {
                        if ui.button(localized::save_changes()).clicked() {
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
                                show_modal(
                                    localized::save_failed_title(),
                                    localized::no_file_selected_save(),
                                    true,
                                );
                                return;
                            }

                            // Check if there are any pending changes
                            if !Nus3audioFileUtils::has_pending_changes() {
                                println!("No pending changes to save");
                                show_modal(
                                    localized::no_changes_title(),
                                    localized::no_pending_changes(),
                                    false,
                                );
                                return;
                            }

                            // Save changes to the current file
                            if let Some(file_path) = selected_file_path {
                                match Nus3audioFileUtils::save_changes_to_file(&file_path) {
                                    Ok(_) => {
                                        println!("Changes saved successfully to: {}", file_path);
                                        show_modal(
                                            localized::save_successful_title(),
                                            &localized::save_success_body(
                                                Nus3audioFileUtils::get_pending_changes_count(),
                                                &file_path,
                                            ),
                                            false,
                                        );

                                        // Update UI if needed
                                        if let Some(app_mut) = app.as_mut() {
                                            // Force reload after save since the on-disk file changed
                                            let main_area = app_mut.main_area_mut();
                                            main_area.force_reload_selected_file();
                                        }
                                    }
                                    Err(e) => {
                                        println!("Failed to save changes: {}", e);
                                        show_modal(
                                            localized::save_failed_title(),
                                            &localized::save_failed_msg(&e.to_string()),
                                            true,
                                        );
                                    }
                                }
                            }
                        }

                        // Dynamic save button based on file type
                        let (save_button_text, file_extension, file_filter) = {
                            let mut selected_file_path = None;
                            if let Some(app_ref) = app.as_ref() {
                                let main_area = app_ref.main_area();
                                if let Some(path) = &main_area.selected_file {
                                    selected_file_path = Some(path.to_string());
                                }
                            }
                            if let Some(ref path) = selected_file_path {
                                if path.to_lowercase().ends_with(".nus3bank") {
                                    (localized::save_nus3bank(), "nus3bank", "NUS3BANK")
                                } else {
                                    (localized::save_nus3audio(), "nus3audio", "NUS3AUDIO")
                                }
                            } else {
                                (localized::save_file_generic(), "nus3audio", "NUS3AUDIO")
                            }
                        };

                        if ui.button(save_button_text).clicked() {
                            // Save current audio file (NUS3AUDIO or NUS3BANK)
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
                                show_modal(
                                    localized::save_failed_title(),
                                    localized::no_file_selected_save_as(),
                                    true,
                                );
                                return;
                            }

                            let is_web = cfg!(target_arch = "wasm32");
                            if !is_web {
                                // Native platform: use file dialog
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter(file_filter, &[file_extension])
                                    .set_file_name(&format!("output.{}", file_extension))
                                    .save_file()
                                {
                                    // Get path as string
                                    let path_str = path.to_string_lossy().to_string();

                                    // Execute save operation with selected file path
                                    if let Some(original_path) = selected_file_path {
                                        // Save using unified method (supports both file types)
                                        TopPanel::save_nus3audio_file(ui.ctx(), &original_path, &path_str);
                                    }
                                }
                            }
                        }
                    });
                }

                ui.menu_button(localized::settings_menu(), |ui| {
                    if ui.button(localized::reset_layout()).clicked() {
                        TopPanel::reset_layout(ctx);
                        show_modal(localized::layout_reset_title(), localized::layout_reset_msg(), false);
                        ui.close();
                    }

                    ui.separator();
                    ui.label(localized::language());
                    if let Some(app) = app.as_mut() {
                        ui.radio_value(&mut app.locale, Locale::En, localized::language_english());
                        ui.radio_value(&mut app.locale, Locale::Zh, localized::language_chinese());
                    } else {
                        ui.label(localized::language_english());
                        ui.label(localized::language_chinese());
                    }
                });

                ui.menu_button(localized::help_menu(), |ui| {
                    if ui.button(localized::about()).clicked() {
                        show_modal_with_link(
                            localized::about_title(),
                            &localized::about_body(env!("CARGO_PKG_VERSION")),
                            localized::source_link_label(),
                            "https://github.com/kjjkjjzyayufqza/EXVS2-Audio-Editor",
                            false,
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
            show_modal_with_link(
                localized::update_available_title(),
                &localized::update_available_body(&current_version, &latest_version),
                localized::download_latest(),
                &download_url,
                false,
            );

            // Mark that we've shown the notice
            if let Ok(mut shown) = SHOWN_UPDATE_NOTICE.lock() {
                *shown = true;
            }
        }
    }

    /// Save current audio files to a new file (supports both NUS3AUDIO and NUS3BANK)
    fn save_nus3audio_file(_ctx: &Context, original_path: &str, save_path: &str) {
        // Use unified method to support both NUS3AUDIO and NUS3BANK files
        match crate::ui::main_area::ReplaceUtils::apply_replacements_and_save_unified(
            original_path,
            save_path,
        ) {
            Ok(_) => {
                println!("File save success: {}", save_path);

                // Show success modal dialog
                show_modal(
                    localized::save_success_export_title(),
                    &localized::save_success_export_body(save_path),
                    false,
                );
            }
            Err(e) => {
                eprintln!("File save fail: {}", e);

                // Show error dialog
                show_modal(
                    localized::save_failed_title(),
                    &localized::save_failed_export(&e.to_string()),
                    true,
                );
            }
        }
    }

    /// Reset resizable panel layout to defaults
    fn reset_layout(ctx: &Context) {
        ctx.memory_mut(|mem| {
            mem.data.remove::<egui::panel::PanelState>(Id::new("file_list_panel"));
            mem.data
                .remove::<egui::panel::PanelState>(Id::new("audio_player_panel"));
        });
        ctx.request_repaint();
    }
}
