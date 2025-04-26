use egui::{
    Align, Button, Color32, Grid, Rect, RichText, Rounding, ScrollArea, Stroke, TextWrapMode, Ui, Vec2,
};
use std::collections::HashSet;
use super::audio_file_info::AudioFileInfo;

/// Table renderer for displaying audio files
pub struct TableRenderer;

impl TableRenderer {
/// Render table UI with callbacks for export and play buttons
    pub fn render_table(
        ui: &mut Ui,
        audio_files: &[AudioFileInfo],
        selected_rows: &mut HashSet<usize>,
        striped: bool,
        clickable: bool,
        show_grid_lines: bool,
        available_height: f32,
        available_width: f32,
        on_export_clicked: &mut dyn FnMut(usize),
        on_play_clicked: &mut dyn FnMut(usize),
    ) {
        // Set row height and text style
        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);

        ui.set_height(text_height * 2.0); // Set height to twice the text height

        // Define column width with minimum sizes
        let col_width_name = available_width / 5.0; // Adjusted for better fit
        let col_width_id = available_width / 8.0; // Increased for long IDs
        let col_width_size = available_width / 8.0;
        let col_width_filename = available_width / 5.0;
        let col_width_type = available_width / 8.0;
        let col_action = available_width
            - col_width_name
            - col_width_id
            - col_width_size
            - col_width_filename
            - col_width_type;

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
            Rect::from_min_size(header_rect.min, Vec2::new(header_rect.width(), 35.0)),
            0.0,
            header_bg_color,
        );

        Grid::new("table_header")
            .num_columns(6)
            .spacing([5.0, 0.0])
            .show(ui, |ui| {
                // Header
                ui.add_sized(
                    [col_width_name, 35.0],
                    egui::Label::new(RichText::new("Name").size(heading_size).strong()),
                )
                .on_hover_text("Audio file name");

                ui.add_sized(
                    [col_width_id, 35.0],
                    egui::Label::new(RichText::new("ID").size(heading_size).strong()),
                )
                .on_hover_text("Audio file ID");

                ui.add_sized(
                    [col_width_size, 35.0],
                    egui::Label::new(RichText::new("Size").size(heading_size).strong()),
                )
                .on_hover_text("File size in bytes");

                ui.add_sized(
                    [col_width_filename, 35.0],
                    egui::Label::new(RichText::new("Filename").size(heading_size).strong()),
                )
                .on_hover_text("Audio filename");

                ui.add_sized(
                    [col_width_type, 35.0],
                    egui::Label::new(RichText::new("Type").size(heading_size).strong()),
                )
                .on_hover_text("Audio file type");

                ui.add_sized(
                    [col_action, 35.0],
                    egui::Label::new(RichText::new("Action").size(heading_size).strong()),
                )
                .on_hover_text("Action");
                ui.end_row();
            });

        // Create table content
        let row_height = text_height * 2.0;
        let text_size = 16.0;
        // let row_height = ui.spacing().interact_size.y; // if you are adding buttons instead of labels.
        ui.set_min_height(available_height - 240.0); // Adjusted for header and spacing

        ScrollArea::vertical().show_rows(ui, row_height, audio_files.len(), |ui, row_range| {
            Grid::new("table_content")
                .num_columns(6)
                .spacing([5.0, 2.0])
                .show(ui, |ui| {
                    for row_index in row_range {
                        let file = &audio_files[row_index];
                        let is_selected = selected_rows.contains(&row_index);

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

                        // Handle row click events
                        if row_response.clicked() && clickable {
                            if selected_rows.contains(&row_index) {
                                selected_rows.remove(&row_index);
                            } else {
                                selected_rows.insert(row_index);
                            }
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
                        
                        // Column 6: Actions - Add Play and Export buttons
                        ui.horizontal(|ui| {
                            // Play button
                            if ui.add_sized(
                                [30.0, 20.0],
                                Button::new(RichText::new("▶").size(text_size).color(Color32::from_rgb(100, 255, 150)))
                            ).clicked() {
                                // Call the callback to play audio
                                on_play_clicked(row_index);
                            }
                            
                            ui.add_space(5.0);
                            
                            // Export button
                            if ui.add_sized(
                                [70.0, 20.0],
                                Button::new(RichText::new("Export").size(text_size))
                            ).clicked() {
                                // Call the callback to handle the export
                                on_export_clicked(row_index);
                            }
                        });

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
