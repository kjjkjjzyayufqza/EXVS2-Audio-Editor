use egui::{Align, Align2, Color32, Context, Layout, RichText, Ui};
use egui_phosphor::regular;

use super::main_area_core::MainArea;

impl MainArea {
    /// Display the main editing area
    pub fn show(&mut self, ctx: &Context) {
        // Show the loop settings modal if open
        self.loop_settings_modal.show(ctx);
        
        // Show the add audio modal if open
        self.add_audio_modal.show(ctx);
        
        // Show the confirm modal if open
        self.confirm_modal.show(ctx);

        // Show the GRP list modal if open
        self.grp_list_modal.show(ctx);

        // Show the DTON tones modal if open
        self.dton_tones_modal.show(ctx);

        // Show the PROP edit modal if open
        self.prop_edit_modal.show(ctx);
        
        egui::CentralPanel::default()
            .frame(egui::Frame::new()
                .fill(ctx.style().visuals.window_fill) // 使用視窗背景色（深灰色）
                .inner_margin(egui::Margin::same(0)))
            .show(ctx, |ui| {
                self.render(ui);
            });
    }

    /// Render the main area content
    pub fn render(&mut self, ui: &mut Ui) {
        // First, clean up expired toast messages
        self.toast_messages.retain(|toast| !toast.has_expired());
        let available_height = ui.available_height();
        let available_width = ui.available_width();

        // Render toast messages at the top (overlay)
        self.render_toasts(ui);

        if let Some(selected) = self.selected_file.clone() {
            ui.vertical(|ui| {
                // 1. Header Area - Professional look with breadcrumbs/file info
                self.render_header(ui, &selected);

                // 2. Toolbar Area - Search and Output path
                self.render_toolbar(ui);

                // 3. Main Content Area - The Table
                if let Some(_audio_files) = &self.audio_files {
                    // Get filtered and sorted audio files
                    let filtered_audio_files = self.filtered_audio_files();
                    let files_count = filtered_audio_files.len();

                    // Render the table with audio files
                    self.render_audio_table(
                        ui, 
                        filtered_audio_files, 
                        files_count, 
                        available_height - 120.0, // Adjust for header and toolbar
                        available_width
                    );
                } else if let Some(error) = &self.error_message {
                    ui.centered_and_justified(|ui| {
                        ui.colored_label(Color32::RED, error);
                    });
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add(egui::Spinner::new());
                            ui.add_space(10.0);
                            ui.label("Loading audio file info...");
                        });
                    });
                }
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(regular::FILE_PLUS.to_string())
                            .size(48.0)
                            .color(ui.visuals().weak_text_color())
                    );
                    ui.add_space(10.0);
                    ui.heading("No file selected");
                    ui.label("Please select a file from the list on the left to start editing");
                });
            });
        }
    }

    /// Render header with file information
    fn render_header(&mut self, ui: &mut Ui, selected: &str) {
        egui::Frame::new()
            .fill(ui.visuals().panel_fill) // 使用面板背景色
            .inner_margin(egui::Margin::symmetric(16, 12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // App Icon / Title
                    ui.label(
                        RichText::new(regular::WAVEFORM.to_string())
                            .size(24.0)
                            .color(Color32::from_rgb(100, 150, 255))
                    );
                    ui.heading("Audio Editor");
                    
                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(20.0);

                    // Current File Info
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Currently editing:").weak().size(11.0));
                            
                            // Display filename with ellipsis if too long
                            let display_name = if selected.len() > 80 {
                                format!(
                                    "{}...{}",
                                    &selected[0..40],
                                    &selected[selected.len() - 37..]
                                )
                            } else {
                                selected.to_string()
                            };

                            ui.label(
                                RichText::new(display_name)
                                    .color(ui.visuals().strong_text_color())
                                    .strong()
                            ).on_hover_text(selected);
                        });

                        if let Some(count) = self.file_count {
                            ui.label(
                                RichText::new(format!("{} audio files found", count))
                                    .weak()
                                    .size(11.0)
                            );
                        }
                    });
                    
                    // Right-aligned status/actions can go here
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button(RichText::new(format!("{} Refresh", regular::ARROWS_CLOCKWISE))).clicked() {
                            if let Some(path) = self.selected_file.clone() {
                                self.update_selected_file(Some(path));
                            }
                        }
                    });
                });
            });
    }

    /// Render toolbar with search and output path
    fn render_toolbar(&mut self, ui: &mut Ui) {
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(16, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Search box (Left aligned)
                    ui.label(RichText::new(regular::MAGNIFYING_GLASS.to_string()).size(16.0));
                    self.render_search_box_compact(ui);
                    
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(12.0);

                    // Output Path (Middle/Right)
                    ui.label(RichText::new(regular::FOLDER_OPEN.to_string()).size(16.0));
                    self.render_output_path_compact(ui);
                    
                    // Advanced search toggle
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let text = if self.show_advanced_search { "Simple View" } else { "Advanced Search" };
                        let icon = if self.show_advanced_search { regular::CARET_UP } else { regular::CARET_DOWN };
                        
                        if ui.button(format!("{} {}", icon, text)).clicked() {
                            self.show_advanced_search = !self.show_advanced_search;
                        }
                    });
                });

                if self.show_advanced_search {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.add_space(24.0); // Align with search icon
                        ui.label("Search in:");
                        egui::ComboBox::from_id_salt("search_column_toolbar")
                            .selected_text(self.search_column.display_name())
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                for column in super::search_column::SearchColumn::all_columns() {
                                    ui.selectable_value(
                                        &mut self.search_column,
                                        column,
                                        column.display_name()
                                    );
                                }
                            });
                        
                        ui.add_space(10.0);
                        ui.small("Tip: Use 'KB' or 'MB' for size search");
                    });
                }
            });
            
        ui.add_space(4.0);
        ui.separator();
    }
    
    /// Render toast notifications
    pub fn render_toasts(&self, ui: &mut Ui) {
        if self.toast_messages.is_empty() {
            return;
        }
        
        // Calculate spacing from top
        let available_rect = ui.ctx().available_rect();
        let spacing = available_rect.height() * 0.08;
        let toast_offset = available_rect.height() * 0.06;
        
        // Show toast messages
        for (i, toast) in self.toast_messages.iter().enumerate() {
            // Create a toast window at the top center of the screen
            let window_id = egui::Id::new("toast_message").with(i);
            let pos = [0.0, spacing + (i as f32 * toast_offset)];
            
            egui::containers::Window::new("Toast")
                .id(window_id)
                .title_bar(false)
                .resizable(false)
                .movable(false)
                .anchor(Align2::CENTER_TOP, pos)
                .default_size([
                    available_rect.width() * 0.4,
                    available_rect.height() * 0.06,
                ])
                .show(ui.ctx(), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.colored_label(toast.color, &toast.message);
                    });
                });
        }
    }
}
