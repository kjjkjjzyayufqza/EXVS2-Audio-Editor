use egui::{RichText, Ui, Color32};
use egui_phosphor::regular;

use super::main_area_core::MainArea;

impl MainArea {
    /// Render search box in a compact way for the toolbar
    pub fn render_search_box_compact(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("Search audio files...")
                    .desired_width(250.0)
            );
            let _ = response; // Avoid unused variable warning
            
            if !self.search_query.is_empty() {
                if ui.button(RichText::new(regular::X.to_string()).color(Color32::GRAY)).on_hover_text("Clear search").clicked() {
                    self.search_query.clear();
                }
            }
        });
    }

    /// Render search box (deprecated/legacy)
    pub fn render_search_box(&mut self, ui: &mut Ui) {
        self.render_search_box_compact(ui);
    }
}
