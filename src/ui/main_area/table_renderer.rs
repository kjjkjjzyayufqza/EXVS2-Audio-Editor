use egui::{
    Button, Color32, Grid, Layout, Rect, RichText, ScrollArea, Stroke, TextWrapMode, Ui, Vec2, Direction,
};
use std::collections::HashSet;
use super::audio_file_info::AudioFileInfo;

use super::sort_column::SortColumn;

/// Table renderer for displaying audio files
pub struct TableRenderer;

impl TableRenderer {
/// Render table UI with callbacks for export, play and replace buttons
    pub fn render_table(
        ui: &mut Ui,
        audio_files: &[AudioFileInfo],
        selected_rows: &mut HashSet<usize>,
        persistent_selected: &mut HashSet<String>,
        striped: bool,
        clickable: bool,
        show_grid_lines: bool,
        available_height: f32,
        available_width: f32,
        on_export_clicked: &mut dyn FnMut(usize),
        on_play_clicked: &mut dyn FnMut(usize),
        on_replace_clicked: &mut dyn FnMut(usize),
        on_remove_clicked: &mut dyn FnMut(usize),
        sort_column: &mut SortColumn,
        sort_ascending: &mut bool,
    ) {
        // Responsive design: detect narrow width
        let is_narrow = available_width < 1100.0;
        let is_ultra_narrow = available_width < 700.0;

        // Set row and header height based on available space
        let header_height = (available_height * 0.04).max(35.0);
        let mut row_height = (available_height * 0.050).max(40.0);

        // Adjust row height for ultra narrow view to accommodate stacked buttons
        if is_ultra_narrow {
            row_height = row_height * 1.8;
        } else if is_narrow {
            row_height = row_height * 1.2;
        }

        // Define column widths as proportions of the available width
        // We aggressively compress other columns to maximize the action area
        let (
            col_width_checkbox,
            col_width_name,
            col_width_id,
            col_width_size,
            col_width_filename,
            col_width_type,
            col_action,
        ) = if is_ultra_narrow {
            (
                available_width * 0.014,
                available_width * 0.15,
                available_width * 0.08,
                available_width * 0.08,
                available_width * 0.15,
                available_width * 0.07,
                available_width * 0.42, // Maximum space for actions
            )
        } else if is_narrow {
            (
                available_width * 0.014,
                available_width * 0.18,
                available_width * 0.10,
                available_width * 0.08,
                available_width * 0.18,
                available_width * 0.08,
                available_width * 0.34,
            )
        } else {
            (
                available_width * 0.014,
                available_width * 0.22,
                available_width * 0.12,
                available_width * 0.1,
                available_width * 0.22,
                available_width * 0.1,
                available_width * 0.2,
            )
        };

        // Header text size
        let heading_size = 17.0;

        // Create header
        let header_bg_color = if ui.visuals().dark_mode {
            Color32::from_rgb(50, 50, 60)
        } else {
            Color32::from_rgb(220, 220, 230)
        };

        let header_rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(
            Rect::from_min_size(header_rect.min, Vec2::new(header_rect.width(), header_height)),
            0.0,
            header_bg_color,
        );

        Grid::new("table_header")
            .num_columns(7)
            .spacing([5.0, 0.0])
            .show(ui, |ui| {
                // Header with sort indicators and clickable functionality
                // Column 0: Selection header with Select All checkbox for current filtered view (centered)
                {
                    let all_selected = audio_files.iter().all(|f| {
                        let key = format!("{}:{}", f.name, f.id);
                        persistent_selected.contains(&key)
                    });
                    let mut header_checked = all_selected;
                    let (_id, cell_rect) = ui.allocate_space(Vec2::new(col_width_checkbox, header_height));
                    ui.scope_builder(egui::UiBuilder::new().max_rect(cell_rect).layout(Layout::centered_and_justified(Direction::LeftToRight)), |ui| {
                        let resp = ui.add(egui::Checkbox::new(&mut header_checked, ""));
                        if resp.changed() {
                            if header_checked {
                                for f in audio_files.iter() {
                                    persistent_selected.insert(format!("{}:{}", f.name, f.id));
                                }
                            } else {
                                for f in audio_files.iter() {
                                    persistent_selected.remove(&format!("{}:{}", f.name, f.id));
                                }
                            }
                        }
                        resp.on_hover_text("Select/Deselect all (filtered)");
                    });
                }
                
                // Name column header
                let name_sort_icon = if *sort_column == SortColumn::Name {
                    if *sort_ascending {
                        format!(" {}", egui_phosphor::regular::ARROW_UP)
                    } else {
                        format!(" {}", egui_phosphor::regular::ARROW_DOWN)
                    }
                } else {
                    "".to_string()
                };
                let name_text = RichText::new(format!("Name{}", name_sort_icon)).size(heading_size).strong();
                
                if ui.add_sized(
                    [col_width_name, header_height],
                    Button::new(name_text).fill(header_bg_color)
                ).clicked() {
                    if *sort_column == SortColumn::Name {
                        *sort_ascending = !*sort_ascending;
                    } else {
                        *sort_column = SortColumn::Name;
                        *sort_ascending = true;
                    }
                };

                // ID column header
                let id_sort_icon = if *sort_column == SortColumn::Id {
                    if *sort_ascending {
                        format!(" {}", egui_phosphor::regular::ARROW_UP)
                    } else {
                        format!(" {}", egui_phosphor::regular::ARROW_DOWN)
                    }
                } else {
                    "".to_string()
                };
                let id_text = RichText::new(format!("ID{}", id_sort_icon)).size(heading_size).strong();
                
                if ui.add_sized(
                    [col_width_id, header_height],
                    Button::new(id_text).fill(header_bg_color)
                ).clicked() {
                    if *sort_column == SortColumn::Id {
                        *sort_ascending = !*sort_ascending;
                    } else {
                        *sort_column = SortColumn::Id;
                        *sort_ascending = true;
                    }
                };

                // Size column header
                let size_sort_icon = if *sort_column == SortColumn::Size {
                    if *sort_ascending {
                        format!(" {}", egui_phosphor::regular::ARROW_UP)
                    } else {
                        format!(" {}", egui_phosphor::regular::ARROW_DOWN)
                    }
                } else {
                    "".to_string()
                };
                let size_text = RichText::new(format!("Size{}", size_sort_icon)).size(heading_size).strong();
                
                if ui.add_sized(
                    [col_width_size, header_height],
                    Button::new(size_text).fill(header_bg_color)
                ).clicked() {
                    if *sort_column == SortColumn::Size {
                        *sort_ascending = !*sort_ascending;
                    } else {
                        *sort_column = SortColumn::Size;
                        *sort_ascending = true;
                    }
                };

                // Filename column header
                let filename_sort_icon = if *sort_column == SortColumn::Filename {
                    if *sort_ascending {
                        format!(" {}", egui_phosphor::regular::ARROW_UP)
                    } else {
                        format!(" {}", egui_phosphor::regular::ARROW_DOWN)
                    }
                } else {
                    "".to_string()
                };
                let filename_text = RichText::new(format!("Filename{}", filename_sort_icon)).size(heading_size).strong();
                
                if ui.add_sized(
                    [col_width_filename, header_height],
                    Button::new(filename_text).fill(header_bg_color)
                ).clicked() {
                    if *sort_column == SortColumn::Filename {
                        *sort_ascending = !*sort_ascending;
                    } else {
                        *sort_column = SortColumn::Filename;
                        *sort_ascending = true;
                    }
                };

                // Type column header
                let type_sort_icon = if *sort_column == SortColumn::Type {
                    if *sort_ascending {
                        format!(" {}", egui_phosphor::regular::ARROW_UP)
                    } else {
                        format!(" {}", egui_phosphor::regular::ARROW_DOWN)
                    }
                } else {
                    "".to_string()
                };
                let type_text = RichText::new(format!("Type{}", type_sort_icon)).size(heading_size).strong();
                
                if ui.add_sized(
                    [col_width_type, header_height],
                    Button::new(type_text).fill(header_bg_color)
                ).clicked() {
                    if *sort_column == SortColumn::Type {
                        *sort_ascending = !*sort_ascending;
                    } else {
                        *sort_column = SortColumn::Type;
                        *sort_ascending = true;
                    }
                };

                // Action column header - consistent button style
                ui.allocate_ui_with_layout(
                    Vec2::new(col_action, header_height),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.add(
                            egui::Button::new(RichText::new("Action").size(heading_size).strong())
                                .fill(header_bg_color)
                        );
                    }
                );
                ui.end_row();
            });

        // Create table content
        let text_size = 16.0;
        // let row_height = ui.spacing().interact_size.y; // if you are adding buttons instead of labels.
        ui.set_min_height(available_height / 3.0); // Adjusted for header and spacing

        ScrollArea::vertical().show_rows(ui, row_height, audio_files.len(), |ui, row_range| {
            Grid::new("table_content")
                .num_columns(7)
                .spacing([5.0, 2.0])
                .show(ui, |ui| {
                    for row_index in row_range {
                        let file = &audio_files[row_index];
                        let key = format!("{}:{}", file.name, file.id);
                        let is_persist_selected = persistent_selected.contains(&key);
                        let is_row_selected = selected_rows.contains(&row_index);
                        let is_selected = is_persist_selected || is_row_selected;

                        // Striped background
                        if striped && row_index % 2 == 1 {
                            let row_rect = ui.available_rect_before_wrap();
                            ui.painter().rect_filled(
                                Rect::from_min_size(
                                    row_rect.min,
                                    Vec2::new(row_rect.width(), row_height),
                                ),
                                0.0,
                                ui.visuals().faint_bg_color,
                            );
                        }

                        // Highlight selected row
                        if is_selected {
                            let row_rect = ui.available_rect_before_wrap();
                            ui.painter().rect_filled(
                                Rect::from_min_size(
                                    row_rect.min,
                                    Vec2::new(row_rect.width(), row_height),
                                ),
                                0.0,
                                ui.visuals().selection.bg_fill,
                            );
                        }

                        // Create a responsive area that includes the entire row
                        let row_rect = ui.available_rect_before_wrap();
                        let sense = if clickable {
                            egui::Sense::click()
                        } else {
                            egui::Sense::hover()
                        };
                        let row_response = ui.interact(
                            Rect::from_min_size(
                                row_rect.min,
                                Vec2::new(row_rect.width(), row_height),
                            ),
                            ui.id().with(row_index),
                            sense,
                        );

                        // Handle row click events: toggle row selection only (checkbox controls persistent selection)
                        if row_response.clicked() && clickable {
                            if selected_rows.contains(&row_index) {
                                selected_rows.remove(&row_index);
                            } else {
                                selected_rows.insert(row_index);
                            }
                        }

                        // Column 0: Checkbox (centered)
                        {
                            let mut checked = is_persist_selected;
                            let (_id, cell_rect) = ui.allocate_space(Vec2::new(col_width_checkbox, row_height));
                            ui.scope_builder(egui::UiBuilder::new().max_rect(cell_rect).layout(Layout::centered_and_justified(Direction::LeftToRight)), |ui| {
                                let resp = ui.add(egui::Checkbox::new(&mut checked, ""));
                                if resp.changed() {
                                    if checked {
                                        persistent_selected.insert(key.clone());
                                    } else {
                                        persistent_selected.remove(&key);
                                    }
                                }
                            });
                        }

                        // Column 1: Name - with text clipping
                        ui.scope(|ui| {
                            ui.style_mut().wrap_mode = Some(TextWrapMode::Truncate);
                            let text = RichText::new(&file.name).size(text_size);
                            ui.add_sized([col_width_name, row_height], egui::Label::new(text))
                                .on_hover_text(&file.name);
                        });

                        // Column 2: ID - with text clipping and ellipsis
                        ui.scope(|ui| {
                            ui.style_mut().wrap_mode = Some(TextWrapMode::Truncate);
                            let text = RichText::new(if file.id.len() > 20 {
                                format!("{}...", &file.id[0..17])
                            } else {
                                file.id.clone()
                            })
                            .size(text_size);
                            ui.add_sized([col_width_id, row_height], egui::Label::new(text))
                                .on_hover_text(&file.id);
                        });

                        // Column 3: Size
                        let size_text = if file.size < 1024 {
                            format!("{} B", file.size)
                        } else if file.size < 1024 * 1024 {
                            format!("{:.1} KB", file.size as f32 / 1024.0)
                        } else {
                            format!("{:.1} MB", file.size as f32 / (1024.0 * 1024.0))
                        };

                        ui.add_sized(
                            [col_width_size, row_height],
                            egui::Label::new(RichText::new(size_text).size(text_size)),
                        );

                        // Column 4: Filename - with text clipping
                        ui.scope(|ui| {
                            ui.style_mut().wrap_mode = Some(TextWrapMode::Truncate);
                            let text = RichText::new(&file.filename).size(text_size);
                            ui.add_sized([col_width_filename, row_height], egui::Label::new(text))
                                .on_hover_text(&file.filename);
                        });

                        // Column 5: Type
                        ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);

                        // Set different colors based on file type
                        let type_text = match file.file_type.as_str() {
                            "OPUS Audio" => RichText::new(&file.file_type)
                                .size(text_size)
                                .color(Color32::from_rgb(100, 200, 100)), // Green
                            "IDSP Audio" => RichText::new(&file.file_type)
                                .size(text_size)
                                .color(Color32::from_rgb(100, 150, 255)), // Blue
                            _ => RichText::new(&file.file_type)
                                .size(text_size)
                                .color(Color32::from_rgb(200, 150, 100)), // Yellow/Brown
                        };

                        ui.add_sized([col_width_type, row_height], egui::Label::new(type_text));
                        
                        // Column 6: Actions - responsive buttons with overflow menu, centered in the cell
                        let (_id, cell_rect) = ui.allocate_space(Vec2::new(col_action, row_height));

                        ui.scope_builder(
                            egui::UiBuilder::new()
                                .max_rect(cell_rect)
                                .layout(Layout::left_to_right(egui::Align::Center)),
                            |ui| {
                                if is_ultra_narrow {
                                    // 2x2 Grid for ultra-narrow screens
                                    Grid::new(format!("actions_grid_{}", row_index))
                                        .num_columns(2)
                                        .spacing([4.0, 4.0])
                                        .show(ui, |ui| {
                                            // Row 1: Play and Export
                                            let play_btn = Button::new(
                                                RichText::new(egui_phosphor::regular::PLAY.to_string())
                                                    .size(text_size)
                                                    .color(Color32::from_rgb(100, 255, 150)),
                                            );
                                            if ui.add(play_btn).on_hover_text("Play").clicked() {
                                                on_play_clicked(row_index);
                                            }

                                            let export_btn = Button::new(
                                                RichText::new(egui_phosphor::regular::DOWNLOAD_SIMPLE.to_string())
                                                    .size(text_size),
                                            );
                                            if ui.add(export_btn).on_hover_text("Export").clicked() {
                                                on_export_clicked(row_index);
                                            }
                                            ui.end_row();

                                            // Row 2: Replace and Remove
                                            let replace_btn = Button::new(
                                                RichText::new(egui_phosphor::regular::SWAP.to_string())
                                                    .size(text_size)
                                                    .color(Color32::from_rgb(255, 180, 100)),
                                            );
                                            if ui.add(replace_btn).on_hover_text("Replace").clicked() {
                                                on_replace_clicked(row_index);
                                            }

                                            let remove_btn = Button::new(
                                                RichText::new(egui_phosphor::regular::TRASH.to_string())
                                                    .size(text_size)
                                                    .color(Color32::from_rgb(255, 100, 100)),
                                            );
                                            if ui.add(remove_btn).on_hover_text("Remove").clicked() {
                                                on_remove_clicked(row_index);
                                            }
                                            ui.end_row();
                                        });
                                } else {
                                    ui.horizontal(|button_ui| {
                                        let available_button_width = cell_rect.width();
                                        let spacing = 5.0;

                                        // Spacing helper to keep consistent gaps
                                        let mut is_first = true;
                                        let mut add_spacing = |ui: &mut egui::Ui| {
                                            if !is_first {
                                                ui.add_space(spacing);
                                            }
                                            is_first = false;
                                        };

                                        // Always show Play as icon-only (highest priority)
                                        add_spacing(button_ui);
                                        let play_button = button_ui.add(
                                            Button::new(
                                                RichText::new(egui_phosphor::regular::PLAY.to_string())
                                                    .size(text_size)
                                                    .color(Color32::from_rgb(100, 255, 150)),
                                            ),
                                        );
                                        if play_button.clicked() {
                                            on_play_clicked(row_index);
                                        }

                                        // Track remaining width with simple estimates so we can reserve for overflow menu
                                        let mut remaining_width = available_button_width - play_button.rect.width();

                                        // Estimated widths (px) for planning only; actual draw uses real sizes
                                        let est_icon = 30.0;
                                        let est_more = 30.0; // More (⋯) button

                                        // Planning flags
                                        let mut show_export = false;
                                        let mut show_replace = false;
                                        let mut show_remove = false;
                                        let mut overflow_export = false;
                                        let mut overflow_replace = false;
                                        let mut overflow_remove = false;
                                        let mut reserved_more = false;

                                        // Helper to reserve space for the overflow button once
                                        let mut ensure_more_reserved = |remaining: &mut f32| {
                                            if !reserved_more {
                                                if *remaining >= spacing + est_more {
                                                    *remaining -= spacing + est_more;
                                                }
                                                reserved_more = true;
                                            }
                                        };

                                        // Decide Export placement (icon only)
                                        if remaining_width >= spacing + est_icon {
                                            show_export = true;
                                            remaining_width -= spacing + est_icon;
                                        } else {
                                            overflow_export = true;
                                            ensure_more_reserved(&mut remaining_width);
                                        }

                                        // Decide Replace placement (icon only)
                                        if remaining_width >= spacing + est_icon {
                                            show_replace = true;
                                            remaining_width -= spacing + est_icon;
                                        } else {
                                            overflow_replace = true;
                                            ensure_more_reserved(&mut remaining_width);
                                        }

                                        // Decide Remove placement (icon only)
                                        if remaining_width >= spacing + est_icon {
                                            show_remove = true;
                                        } else {
                                            overflow_remove = true;
                                            ensure_more_reserved(&mut remaining_width);
                                        }

                                        // Draw Export (if inline)
                                        if show_export {
                                            add_spacing(button_ui);
                                            let export_text = RichText::new(
                                                egui_phosphor::regular::DOWNLOAD_SIMPLE.to_string(),
                                            )
                                            .size(text_size);
                                            let export_button = button_ui
                                                .add(Button::new(export_text))
                                                .on_hover_text("Export");
                                            if export_button.clicked() {
                                                on_export_clicked(row_index);
                                            }
                                        }

                                        // Draw Replace (if inline)
                                        if show_replace {
                                            add_spacing(button_ui);
                                            let replace_text = RichText::new(
                                                egui_phosphor::regular::SWAP.to_string(),
                                            )
                                            .size(text_size)
                                            .color(Color32::from_rgb(255, 180, 100));
                                            let replace_button = button_ui
                                                .add(Button::new(replace_text))
                                                .on_hover_text("Replace");
                                            if replace_button.clicked() {
                                                on_replace_clicked(row_index);
                                            }
                                        }

                                        // Draw Remove (if inline)
                                        if show_remove {
                                            add_spacing(button_ui);
                                            let remove_text = RichText::new(
                                                egui_phosphor::regular::TRASH.to_string(),
                                            )
                                            .size(text_size)
                                            .color(Color32::from_rgb(255, 100, 100));
                                            let remove_button = button_ui
                                                .add(Button::new(remove_text))
                                                .on_hover_text("Remove");
                                            if remove_button.clicked() {
                                                on_remove_clicked(row_index);
                                            }
                                        }

                                        // Overflow menu for actions that did not fit
                                        if overflow_export || overflow_replace || overflow_remove {
                                            add_spacing(button_ui);
                                            let more_label = RichText::new("⋯").size(text_size);
                                            let _ = button_ui.menu_button(more_label, |ui| {
                                                if overflow_export {
                                                    if ui.button("Export").clicked() {
                                                        on_export_clicked(row_index);
                                                        ui.close();
                                                    }
                                                }
                                                if overflow_replace {
                                                    if ui.button("Replace").clicked() {
                                                        on_replace_clicked(row_index);
                                                        ui.close();
                                                    }
                                                }
                                                if overflow_remove {
                                                    if ui.button("Remove").clicked() {
                                                        on_remove_clicked(row_index);
                                                        ui.close();
                                                    }
                                                }
                                            });
                                        }
                                    });
                                }
                            },
                        );

                        ui.end_row();

                        // Add grid lines
                        if show_grid_lines && row_index < audio_files.len() - 1 {
                            let line_start = row_rect.min + Vec2::new(0.0, row_height);
                            let line_end = line_start + Vec2::new(row_rect.width(), 0.0);
                            ui.painter().line_segment(
                                [line_start, line_end],
                                Stroke::new(
                                    0.5,
                                    ui.visuals().widgets.noninteractive.bg_stroke.color,
                                ),
                            );
                        }
                    }
                });
        });
    }
}
