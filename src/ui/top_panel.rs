use crate::Locale;
use crate::localized;
use crate::ui::main_area::Nus3audioFileUtils;
use crate::ui::update_log;
use egui::{Context, Id, Ui};
use once_cell::sync::Lazy;
use std::sync::Mutex;

// Modal dialog information
#[derive(Clone, Default)]
struct ModalInfo {
    open: bool,
    title: String,
    message: String,
    is_error: bool,
    /// Optional clickable links shown under the message (label, url)
    links: Vec<(String, String)>,
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
        modal.links.clear();
    }
}

fn show_modal_with_links(
    title: impl AsRef<str>,
    message: impl AsRef<str>,
    links: Vec<(String, String)>,
    is_error: bool,
) {
    if let Ok(mut modal) = MODAL_INFO.lock() {
        modal.open = true;
        modal.title = title.as_ref().to_string();
        modal.message = message.as_ref().to_string();
        modal.is_error = is_error;
        modal.links = links;
    }
}

/// Top menu panel component
pub struct TopPanel;

impl TopPanel {
    /// Display the top menu panel
    pub fn show(ui: &mut Ui, mut app: Option<&mut crate::TemplateApp>) {
        let ctx = ui.ctx().clone();
        // Version notice + history windows
        update_log::show_windows(&ctx);

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
            // Fixed id keeps drag position across different modal titles.
            // Do NOT use .anchor() — it makes the window immovable.
            let screen_center = ctx.content_rect().center();
            egui::Window::new(&modal.title)
                .id(Id::new("top_panel_modal"))
                .collapsible(false)
                .resizable(true)
                .movable(true)
                .default_width(440.0)
                .min_width(400.0)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(screen_center)
                .show(&ctx, |ui| {
                    ui.set_min_width(380.0);
                    ui.label(egui::RichText::new(&modal.message).size(14.0));

                    if !modal.links.is_empty() {
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(6.0);
                        for (link_text, link_url) in &modal.links {
                            ui.horizontal(|ui| {
                                ui.label(format!("{link_text}:"));
                                ui.hyperlink_to(link_url.as_str(), link_url.as_str());
                            });
                        }
                    }

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(localized::ok()).clicked() {
                                should_close_modal = true;
                            }
                        });
                    });
                });
        }

        // Update modal state after the window is displayed
        if should_close_modal {
            if let Ok(mut modal) = MODAL_INFO.lock() {
                modal.open = false;
            }
        }

        egui::Panel::top("top_panel").show(ui, |ui| {
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

                            if let Some(file_path) = selected_file_path {
                                // Pending add/replace, or a NUS3BANK that still needs
                                // WAV→BNSF / cloned-loop-clock repair.
                                if !Nus3audioFileUtils::has_pending_changes_for_path(&file_path) {
                                    println!("No pending changes to save");
                                    show_modal(
                                        localized::no_changes_title(),
                                        localized::no_pending_changes(),
                                        false,
                                    );
                                    return;
                                }
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
                                        TopPanel::save_nus3audio_file(
                                            ui.ctx(),
                                            &original_path,
                                            &path_str,
                                        );
                                    }
                                }
                            }
                        }
                    });
                }

                ui.menu_button(localized::settings_menu(), |ui| {
                    if ui.button(localized::reset_layout()).clicked() {
                        TopPanel::reset_layout(&ctx);
                        show_modal(
                            localized::layout_reset_title(),
                            localized::layout_reset_msg(),
                            false,
                        );
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
                        show_modal_with_links(
                            localized::about_title(),
                            &localized::about_body(env!("CARGO_PKG_VERSION")),
                            vec![
                                (
                                    localized::source_link_label(),
                                    "https://github.com/kjjkjjzyayufqza/EXVS2-Audio-Editor"
                                        .to_string(),
                                ),
                                (
                                    localized::buy_me_a_coffee_label(),
                                    "https://www.buymeacoffee.com/kjjkjjzyayx".to_string(),
                                ),
                            ],
                            false,
                        );
                    }
                    if ui.button(localized::update_history_menu()).clicked() {
                        update_log::open_history();
                        ui.close();
                    }
                });
            });
        });
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
            mem.data
                .remove::<egui::PanelState>(Id::new("file_list_panel"));
            mem.data
                .remove::<egui::PanelState>(Id::new("audio_player_panel"));
        });
        ctx.request_repaint();
    }
}
