use egui::{Color32, Ui, RichText};
use egui_phosphor::regular;

use super::main_area_core::MainArea;

impl MainArea {
    /// Render output path selection in a compact way for the toolbar
    pub fn render_output_path_compact(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let path_text = match &self.output_path {
                Some(path) => {
                    // Shorten path if too long
                    if path.len() > 30 {
                        let mut shortened = String::new();
                        if let Some(last_part) = std::path::Path::new(path).file_name() {
                            if let Some(last_str) = last_part.to_str() {
                                shortened = last_str.to_string();
                            }
                        }
                        if shortened.is_empty() {
                            format!("{}...", &path[0..27])
                        } else {
                            shortened
                        }
                    } else {
                        path.clone()
                    }
                },
                None => "Output folder not set".to_string(),
            };
            
            let color = if self.output_path.is_none() {
                Color32::from_rgb(255, 200, 100) // Warning color
            } else {
                ui.visuals().strong_text_color()
            };

            ui.label(RichText::new("Export to:").weak().size(11.0));
            
            let label_resp = ui.label(RichText::new(path_text).color(color));
            if let Some(path) = &self.output_path {
                label_resp.on_hover_text(path);
            } else {
                label_resp.on_hover_text("Please select a folder where exported files will be saved");
            }
            
            if ui.button(format!("{} Browse", regular::FOLDER_OPEN)).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Select Output Directory")
                    .set_directory(self.output_path.clone().unwrap_or_else(|| ".".to_string()))
                    .pick_folder() 
                {
                    if let Some(path_str) = path.to_str() {
                        self.output_path = Some(path_str.to_string());
                    }
                }
            }

            if self.output_path.is_some() {
                if ui.button(RichText::new(regular::X.to_string()).color(Color32::GRAY)).on_hover_text("Clear output path").clicked() {
                    self.output_path = None;
                }
            }
        });
    }

    /// Render output path selection (deprecated/legacy)
    pub fn render_output_path(&mut self, ui: &mut Ui) {
        self.render_output_path_compact(ui);
    }
}
