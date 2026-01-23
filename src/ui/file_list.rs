use egui::{Align, Button, Color32, Frame, Layout, RichText, ScrollArea, Ui, Vec2};
use egui_phosphor::regular;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

        ui.vertical(|ui| {
            // Header
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading(format!("{} Files", regular::FILES));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Add Button (Always visible at the top for better UX)
                    let add_btn =
                        Button::new(RichText::new(regular::PLUS_CIRCLE).size(20.0)).frame(false);
                    if ui.add(add_btn).on_hover_text("Add Files").clicked() {
                        if let Some(paths) = rfd::FileDialog::new()
                            .set_title("Select Audio Files")
                            .add_filter("Audio Files", &["nus3audio", "wav", "mp3"])
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
                        if ui.add(clear_btn).on_hover_text("Clear All Files").clicked() {
                            self.clear_all();
                            file_changed = true;
                        }

                        ui.label(RichText::new(format!("{}", self.files.len())).weak());
                    }
                });
            });
            ui.add_space(8.0);

            // Search box with improved UX
            egui::Frame::none()
                .fill(ui.visuals().extreme_bg_color)
                .corner_radius(4.0)
                .inner_margin(4.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(4.0);
                        ui.label(RichText::new(regular::MAGNIFYING_GLASS).weak());

                        let _response = ui.add(
                            egui::TextEdit::singleline(&mut self.search_query)
                                .desired_width(ui.available_width())
                                .hint_text("Search files...")
                                .frame(false),
                        );

                        if !self.search_query.is_empty() {
                            if ui
                                .button(regular::X)
                                .on_hover_text("Clear Search")
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
                    ui.label(RichText::new("No files added").weak());
                    ui.add_space(20.0);
                });
            } else {
                let filtered = self.filtered_files();

                if filtered.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(RichText::new("No matching files").weak());
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

                                    // Allocate the specific space for this row to ensure height matches
                                    let row_width = ui.available_width();
                                    let (id, rect) =
                                        ui.allocate_space(egui::vec2(row_width, row_height));
                                    let response = ui.interact(rect, id, egui::Sense::click());

                                    if response.clicked() {
                                        action_path = Some(file.path.clone());
                                        is_remove_action = false;
                                        file_changed = true;
                                    }

                                    // Custom rendering for list item
                                    let painter = ui.painter();
                                    let rounding = 4.0;

                                    if response.hovered() || is_selected {
                                        let bg_color = if is_selected {
                                            ui.visuals().selection.bg_fill
                                        } else {
                                            ui.visuals().widgets.hovered.bg_fill.gamma_multiply(0.3)
                                        };
                                        painter.rect_filled(rect, rounding, bg_color);
                                    }

                                    // Layout content within the allocated rect with vertical centering
                                    let ui_builder = egui::UiBuilder::new()
                                        .max_rect(rect)
                                        .layout(egui::Layout::left_to_right(egui::Align::Center));
                                    ui.scope_builder(ui_builder, |ui| {
                                        ui.add_space(8.0);

                                        // File icon
                                        let icon =
                                            if file.path.to_lowercase().ends_with(".nus3audio") {
                                                regular::MUSIC_NOTES
                                            } else {
                                                regular::FILE
                                            };
                                        ui.label(RichText::new(icon).weak());

                                        // Filename
                                        ui.style_mut().wrap_mode =
                                            Some(egui::TextWrapMode::Truncate);
                                        let text_color = if is_selected {
                                            ui.visuals().selection.stroke.color
                                        } else {
                                            ui.visuals().widgets.inactive.text_color()
                                        };
                                        ui.label(RichText::new(&file.name).color(text_color));

                                        // Remove button (only show on hover or if selected)
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                ui.add_space(4.0);
                                                if response.hovered() || is_selected {
                                                    let remove_btn = Button::new(
                                                        RichText::new(regular::X).size(12.0),
                                                    )
                                                    .frame(false);
                                                    if ui
                                                        .add(remove_btn)
                                                        .on_hover_text("Remove from list")
                                                        .clicked()
                                                    {
                                                        action_path = Some(file.path.clone());
                                                        is_remove_action = true;
                                                        file_changed = true;
                                                    }
                                                }
                                            },
                                        );
                                    });

                                    response.on_hover_text(&file.path);
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
