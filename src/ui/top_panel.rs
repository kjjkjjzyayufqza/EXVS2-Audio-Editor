use crate::Locale;
use crate::localized;
use crate::ui::main_area::Nus3audioFileUtils;
use crate::ui::update_log;
use egui::{Context, Id, Ui};
use once_cell::sync::Lazy;
use std::path::Path;
use std::sync::Mutex;

#[derive(Clone)]
struct ModalAction {
    dest: String,
    staged: String,
    /// Reload the open file after a successful elevated save.
    reload: bool,
}

// Modal dialog information
#[derive(Clone, Default)]
struct ModalInfo {
    open: bool,
    title: String,
    message: String,
    is_error: bool,
    /// Optional clickable links shown under the message (label, url)
    links: Vec<(String, String)>,
    /// Present when the user can retry the write with administrator rights.
    action: Option<ModalAction>,
    /// Full file written beside the user temp dir. Deleted on dismiss.
    staged_cleanup: Option<String>,
}

// Using Lazy and Mutex for thread-safe access to modal info
static MODAL_INFO: Lazy<Mutex<ModalInfo>> = Lazy::new(|| Mutex::new(ModalInfo::default()));

// Helper functions to manage the modal state
fn discard_staged(modal: &mut ModalInfo) {
    if let Some(path) = modal.staged_cleanup.take() {
        let _ = std::fs::remove_file(path);
    }
    modal.action = None;
}

fn show_modal(title: impl AsRef<str>, message: impl AsRef<str>, is_error: bool) {
    if let Ok(mut modal) = MODAL_INFO.lock() {
        discard_staged(&mut modal);
        modal.open = true;
        modal.title = title.as_ref().to_string();
        modal.message = message.as_ref().to_string();
        modal.is_error = is_error;
        modal.links.clear();
    }
}

fn show_save_denied(path: String, message: String, staged: String, reload: bool) {
    if let Ok(mut modal) = MODAL_INFO.lock() {
        discard_staged(&mut modal);
        modal.open = true;
        modal.title = localized::save_failed_title();
        modal.message = message;
        modal.is_error = true;
        modal.links.clear();
        modal.staged_cleanup = Some(staged.clone());
        #[cfg(windows)]
        {
            modal.action = Some(ModalAction {
                dest: path,
                staged,
                reload,
            });
        }
        #[cfg(not(windows))]
        {
            let _ = (path, reload);
        }
    }
}

fn set_modal_message(message: impl AsRef<str>) {
    if let Ok(mut modal) = MODAL_INFO.lock() {
        modal.open = true;
        modal.is_error = true;
        modal.message = message.as_ref().to_string();
    }
}

fn present_save_error(err: crate::save_io::SaveWriteError, reload: bool) {
    match err {
        crate::save_io::SaveWriteError::Busy { path, os } => {
            show_modal(
                localized::save_failed_title(),
                localized::save_failed_busy(&path, &os),
                true,
            );
        }
        crate::save_io::SaveWriteError::Denied { path, os, staged } => {
            show_save_denied(
                path.clone(),
                localized::save_failed_denied(&path, &os),
                staged.display().to_string(),
                reload,
            );
        }
        crate::save_io::SaveWriteError::Other { detail } => {
            show_modal(
                localized::save_failed_title(),
                localized::save_failed_detail(&detail),
                true,
            );
        }
    }
}

fn run_elevated_save(action: ModalAction, app: Option<&mut crate::TemplateApp>) {
    match crate::save_io::elevate_replace(Path::new(&action.dest), Path::new(&action.staged)) {
        Ok(()) => {
            let _ = std::fs::remove_file(&action.staged);
            if let Ok(mut modal) = MODAL_INFO.lock() {
                modal.staged_cleanup = None;
                modal.action = None;
            }
            if action.reload {
                Nus3audioFileUtils::clear_changes();
                crate::ui::main_area::ExportUtils::clear_wav_cache_for_file(&action.dest);
                crate::ui::main_area::ReplaceUtils::clear_saved_bank_pending(&action.dest);
                if let Some(app) = app {
                    app.main_area_mut().force_reload_selected_file();
                }
            }
            show_modal(
                localized::save_successful_title(),
                localized::save_elevate_ok(&action.dest),
                false,
            );
        }
        Err(crate::save_io::ElevateError::Cancelled) => {
            set_modal_message(localized::save_elevate_cancelled());
        }
        Err(crate::save_io::ElevateError::Unsupported) => {
            set_modal_message(localized::save_elevate_unsupported());
        }
        Err(crate::save_io::ElevateError::Failed(msg)) => {
            set_modal_message(localized::save_elevate_failed(&msg));
        }
    }
}

fn show_modal_with_links(
    title: impl AsRef<str>,
    message: impl AsRef<str>,
    links: Vec<(String, String)>,
    is_error: bool,
) {
    if let Ok(mut modal) = MODAL_INFO.lock() {
        discard_staged(&mut modal);
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
        let mut elevate_request: Option<ModalAction> = None;
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
            let title = if modal.is_error {
                egui::RichText::new(&modal.title)
                    .strong()
                    .color(ctx.global_style().visuals.error_fg_color)
            } else {
                egui::RichText::new(&modal.title).strong()
            };
            egui::Window::new(title)
                .id(Id::new("top_panel_modal"))
                .collapsible(false)
                .resizable(true)
                .movable(true)
                .default_width(480.0)
                .min_width(420.0)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(screen_center)
                .show(&ctx, |ui| {
                    ui.set_min_width(400.0);
                    let mut body = egui::RichText::new(&modal.message).size(14.0);
                    if modal.is_error {
                        body = body.color(ui.visuals().error_fg_color);
                    }
                    egui::ScrollArea::vertical()
                        .id_salt("top_panel_modal_body")
                        .max_height(240.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(body).wrap_mode(egui::TextWrapMode::Wrap),
                            );
                        });

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
                    ui.separator();
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(egui::Button::new(localized::ok()).min_size(egui::vec2(96.0, 40.0)))
                                .clicked()
                            {
                                should_close_modal = true;
                            }
                            if modal.action.is_some()
                                && ui
                                    .add(
                                        egui::Button::new(localized::save_elevate_btn())
                                            .min_size(egui::vec2(180.0, 40.0)),
                                    )
                                    .clicked()
                            {
                                elevate_request = modal.action.clone();
                            }
                        });
                    });
                });
        }

        // Update modal state after the window is displayed
        if let Some(action) = elevate_request {
            run_elevated_save(action, app.as_deref_mut());
        }

        if should_close_modal {
            if let Ok(mut modal) = MODAL_INFO.lock() {
                discard_staged(&mut modal);
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
                                let saved_count =
                                    Nus3audioFileUtils::get_pending_changes_count();
                                match Nus3audioFileUtils::save_changes_to_file(&file_path) {
                                    Ok(backup_warning) => {
                                        println!("Changes saved successfully to: {}", file_path);
                                        let mut body = if saved_count == 0 {
                                            localized::save_success_written(&file_path)
                                        } else {
                                            localized::save_success_body(saved_count, &file_path)
                                        };
                                        if let Some(err) = backup_warning {
                                            body.push_str(&localized::save_backup_failed(&err));
                                        }
                                        show_modal(
                                            localized::save_successful_title(),
                                            &body,
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
                                        present_save_error(e, true);
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
                eprintln!("File save fail: {e}");
                present_save_error(e, false);
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
