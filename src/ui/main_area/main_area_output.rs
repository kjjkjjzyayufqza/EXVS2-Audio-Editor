use egui::{Color32, CornerRadius, Frame, Stroke, Ui};

use super::main_area_core::MainArea;

impl MainArea {
    /// Render output path selection
    pub fn render_output_path(&mut self, ui: &mut Ui) {
        Frame::group(ui.style())
            .stroke(Stroke::new(1.0, ui.visuals().widgets.active.bg_fill))
            .corner_radius(CornerRadius::same(5))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Output Path");
                });
                ui.add_space(5.0);
                
                // Current output path display
                ui.horizontal(|ui| {
                    ui.label("Path:");
                    let path_text = match &self.output_path {
                        Some(path) => {
                            // Shorten path if too long
                            if path.len() > 40 {
                                let mut shortened = String::new();
                                if let Some(last_part) = std::path::Path::new(path).file_name() {
                                    if let Some(last_str) = last_part.to_str() {
                                        shortened = last_str.to_string();
                                    }
                                }
                                if shortened.is_empty() {
                                    format!("{}...", &path[0..37])
                                } else {
                                    shortened
                                }
                            } else {
                                path.clone()
                            }
                        },
                        None => "Not set".to_string(),
                    };
                    
                    // Display path with full path as hover text
                    let label = ui.label(path_text);
                    if let Some(path) = &self.output_path {
                        label.on_hover_text(path);
                    }
                    
                    // Select folder button
                    if ui.button("Select folder").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title("Select Output Directory")
                            .set_directory(self.output_path.clone().unwrap_or_else(|| ".".to_string()))
                            .pick_folder() 
                        {
                            if let Some(path_str) = path.to_str() {
                                self.output_path = Some(path_str.to_string());
                                // Save the path to config immediately to ensure it's persisted
                                // This would be ideal, but for simplicity, we'll rely on app shutdown saving
                            }
                        }
                    }
                    // Clear button if path is set
                    if self.output_path.is_some() && ui.button("✖").clicked() {
                        self.output_path = None;
                    }
                });
                // Help text and warnings
                ui.add_space(5.0);
                // Show warning if output path is not set
                if self.output_path.is_none() {
                    ui.colored_label(Color32::GOLD, 
                        "No output directory set. Please set an output directory in the Output Path section.");
                }
            });
    }
}
