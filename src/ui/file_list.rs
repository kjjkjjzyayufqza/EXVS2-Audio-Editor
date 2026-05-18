use crate::localized;
use egui::{Align, Button, Color32, CursorIcon, Layout, Rect, RichText, ScrollArea, Ui};
use egui_phosphor::regular;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::main_area::ConfirmModal;

/// File item structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileItem {
    pub path: String,
    pub name: String,
    pub is_selected: bool,
}

/// File list component
#[derive(Default, Deserialize, Serialize)]
pub struct FileList {
    pub files: Vec<FileItem>,
    pub selected_file: Option<String>,
    #[serde(skip)]
    pub search_query: String,
    #[serde(skip)]
    pub confirm_clear_modal: ConfirmModal,
}

impl FileList {
    /// Create a new file list
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a file to the list
    pub fn add_file(&mut self, path: String) {
        // Avoid duplicates
        if self.files.iter().any(|f| f.path == path) {
            return;
        }

        let path_obj = PathBuf::from(&path);
        let name = path_obj
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown file")
            .to_string();

        self.files.push(FileItem {
            path: path.clone(),
            name,
            is_selected: false,
        });

        // Automatically select the newly added file
        self.selected_file = Some(path);
        self.update_selection();
    }

    /// Remove a file from the list
    pub fn remove_file(&mut self, path: &str) {
        // Find index of file to remove
        let index = self.files.iter().position(|f| f.path == path);

        if let Some(idx) = index {
            // Remove file from list
            self.files.remove(idx);

            // Update selection if the removed file was selected
            if let Some(selected) = &self.selected_file {
                if selected == path {
                    // Select another file if available
                    self.selected_file = self.files.first().map(|f| f.path.clone());
                }
            }

            self.update_selection();
        }
    }

    /// Clear all files from the list
    pub fn clear_all(&mut self) {
        self.files.clear();
        self.selected_file = None;
    }

    /// Update selection state
    fn update_selection(&mut self) {
        for file in &mut self.files {
            if let Some(selected) = &self.selected_file {
                file.is_selected = &file.path == selected;
            } else {
                file.is_selected = false;
            }
        }
    }

    /// Get filtered files based on search query
    fn filtered_files(&self) -> Vec<&FileItem> {
        if self.search_query.is_empty() {
            // If no search query, return all files
            return self.files.iter().collect();
        }

        let query = self.search_query.to_lowercase();
        self.files
            .iter()
            .filter(|file| {
                file.name.to_lowercase().contains(&query)
                    || file.path.to_lowercase().contains(&query)
            })
            .collect()
    }

    /// Display the file list
    pub fn show(&mut self, ui: &mut Ui) -> bool {
        let mut file_changed = false;
        let mut action_path = None;
        let mut is_remove_action = false;

        // Show confirm modal
        self.confirm_clear_modal.show(ui.ctx());
        
        // Handle confirm modal result
        if self.confirm_clear_modal.confirmed {
            self.clear_all();
            file_changed = true;
            self.confirm_clear_modal.reset_state();
        } else if self.confirm_clear_modal.cancelled {
            self.confirm_clear_modal.reset_state();
        }

        ui.vertical(|ui| {
            // Header
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading(format!("{} {}", regular::FILES, localized::files_heading()));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Add Button (Always visible at the top for better UX)
                    let add_btn =
                        Button::new(RichText::new(regular::PLUS_CIRCLE).size(20.0)).frame(false);
                    if ui.add(add_btn).on_hover_text(localized::add_files_tooltip()).clicked() {
                        if let Some(paths) = rfd::FileDialog::new()
                            .set_title(localized::select_audio_files_title())
                            .add_filter(localized::audio_files_filter(), &["nus3audio", "nus3bank", "wav", "mp3"])
                            .pick_files()
                        {
                            for path in paths {
                                let path_str = path.to_string_lossy().to_string();
                                self.add_file(path_str);
                            }
                            file_changed = true;
                        }
                    }

                    if !self.files.is_empty() {
                        let clear_btn = Button::new(
                            RichText::new(regular::TRASH).color(Color32::from_rgb(255, 100, 100)),
                        )
                        .frame(false);
                        if ui.add(clear_btn).on_hover_text(localized::clear_all_files_title()).clicked() {
                            self.confirm_clear_modal.open(
                                localized::clear_all_files_title(),
                                &localized::clear_all_files_confirm(self.files.len())
                            );
                        }

                        ui.label(RichText::new(format!("{}", self.files.len())).weak());
                    }
                });
            });
            ui.add_space(8.0);

            // Search box with improved UX - fixed width to prevent expansion
            ui.set_width(ui.available_width());
            egui::Frame::new()
                .fill(ui.visuals().extreme_bg_color)
                .corner_radius(4.0)
                .inner_margin(4.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(4.0);
                        ui.label(RichText::new(regular::MAGNIFYING_GLASS).weak());

                        let available_width = ui.available_width() - 30.0; // Reserve space for clear button
                        let _response = ui.add_sized(
                            [available_width, 20.0],
                            egui::TextEdit::singleline(&mut self.search_query)
                                .hint_text(localized::search_files_hint())
                                .frame(false),
                        );

                        if !self.search_query.is_empty() {
                            if ui
                                .button(regular::X)
                                .on_hover_text(localized::clear_search_tooltip())
                                .clicked()
                            {
                                self.search_query.clear();
                            }
                        }
                    });
                });
            ui.add_space(8.0);

            // File List
            if self.files.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(RichText::new(regular::FILE_DASHED).size(32.0).weak());
                    ui.add_space(8.0);
                    ui.label(RichText::new(localized::no_files_added()).weak());
                    ui.add_space(20.0);
                });
            } else {
                let filtered = self.filtered_files();

                if filtered.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(RichText::new(localized::no_matching_files()).weak());
                    });
                } else {
                    let row_height = 32.0;
                    ScrollArea::vertical().auto_shrink([false, true]).show_rows(
                        ui,
                        row_height,
                        filtered.len(),
                        |ui, row_range| {
                            for i in row_range {
                                let file = filtered[i];
                                let is_selected = file.is_selected;

                                ui.scope(|ui| {
                                    if is_selected {
                                        ui.visuals_mut().widgets.inactive.bg_fill =
                                            ui.visuals().selection.bg_fill;
                                        ui.visuals_mut().widgets.hovered.bg_fill =
                                            ui.visuals().selection.bg_fill;
                                    }

                                    let row_width = ui.available_width();
                                    let (_id, rect) =
                                        ui.allocate_space(egui::vec2(row_width, row_height));

                                    let ptr_pos = ui
                                        .ctx()
                                        .input(|i| i.pointer.hover_pos())
                                        .or(ui.ctx().pointer_interact_pos());

                                    let hovered_row = ptr_pos
                                        .map(|p| rect.contains(p))
                                        .unwrap_or(false);

                                    let row_pick = ui.interact(
                                        rect,
                                        ui.id().with(("file_list_row_pick", file.path.as_str())),
                                        egui::Sense::CLICK | egui::Sense::HOVER,
                                    );

                                    let painter = ui.painter();
                                    let rounding = 4.0;

                                    let mut label_rect = Rect::NOTHING;
                                    let mut remove_clicked = false;

                                    if hovered_row || is_selected {
                                        let bg_color = if is_selected {
                                            ui.visuals().selection.bg_fill
                                        } else {
                                            ui.visuals().widgets.hovered.bg_fill.gamma_multiply(0.3)
                                        };
                                        painter.rect_filled(rect, rounding, bg_color);
                                    }

                                    ui.scope_builder(
                                        egui::UiBuilder::new()
                                            .max_rect(rect)
                                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                                        |ui| {
                                            ui.add_space(8.0);

                                            let icon = if file.path.to_lowercase().ends_with(".nus3audio")
                                                || file.path.to_lowercase().ends_with(".nus3bank")
                                            {
                                                regular::MUSIC_NOTES
                                            } else {
                                                regular::FILE
                                            };
                                            ui.label(RichText::new(icon).weak());

                                            let text_color = if is_selected {
                                                ui.visuals().selection.stroke.color
                                            } else {
                                                ui.visuals().widgets.inactive.text_color()
                                            };

                                            ui.style_mut().wrap_mode =
                                                Some(egui::TextWrapMode::Truncate);

                                            let name_label_response = ui.add(
                                                egui::Label::new(
                                                    RichText::new(&file.name).color(text_color),
                                                )
                                                .truncate()
                                                .selectable(true),
                                            );
                                            label_rect = name_label_response.rect;
                                            name_label_response.on_hover_text(&file.path);

                                            ui.with_layout(
                                                Layout::right_to_left(Align::Center),
                                                |ui| {
                                                    ui.add_space(4.0);
                                                    if hovered_row || is_selected {
                                                        let remove_btn = Button::new(
                                                            RichText::new(regular::X).size(12.0),
                                                        )
                                                        .frame(false);
                                                        if ui
                                                            .add(remove_btn)
                                                            .on_hover_text(
                                                                localized::remove_from_list_tooltip(),
                                                            )
                                                            .clicked()
                                                        {
                                                            remove_clicked = true;
                                                        }
                                                    }
                                                },
                                            );
                                        },
                                    );

                                    if hovered_row {
                                        if let Some(p) = ptr_pos {
                                            if rect.contains(p)
                                                && label_rect.is_positive()
                                                && label_rect.expand(2.0).contains(p)
                                            {
                                                ui.ctx().set_cursor_icon(CursorIcon::Text);
                                            } else if rect.contains(p) {
                                                ui.ctx()
                                                    .set_cursor_icon(CursorIcon::PointingHand);
                                            }
                                        }
                                    }

                                    if remove_clicked {
                                        action_path = Some(file.path.clone());
                                        is_remove_action = true;
                                        file_changed = true;
                                    } else if row_pick.clicked() {
                                        let clicked_on_filename = ui
                                            .ctx()
                                            .pointer_interact_pos()
                                            .map(|p| {
                                                label_rect.is_positive()
                                                    && label_rect.expand(2.0).contains(p)
                                            })
                                            .unwrap_or(false);

                                        if !clicked_on_filename {
                                            action_path = Some(file.path.clone());
                                            is_remove_action = false;
                                            file_changed = true;
                                        }
                                    }

                                    row_pick.on_hover_text(&file.path);
                                });
                            }
                        },
                    );
                }
            }
        });

        // Process actions outside the UI loops
        if let Some(path) = action_path {
            if is_remove_action {
                self.remove_file(&path);
            } else {
                self.selected_file = Some(path);
                self.update_selection();
            }
        }

        file_changed
    }
}
