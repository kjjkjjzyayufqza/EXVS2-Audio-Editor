use egui::{Frame, CornerRadius, Stroke, Ui};

use super::main_area_core::MainArea;
use super::search_column::SearchColumn;

impl MainArea {
    /// Render search box
    pub fn render_search_box(&mut self, ui: &mut Ui) {
        Frame::group(ui.style())
            .stroke(Stroke::new(1.0, ui.visuals().widgets.active.bg_fill))
            .corner_radius(CornerRadius::same(5))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.heading("Search");
                    
                    // Toggle advanced search
                    if ui.button(if self.show_advanced_search { "Basic" } else { "Advanced" }).clicked() {
                        self.show_advanced_search = !self.show_advanced_search;
                    }
                });
                ui.add_space(5.0);
                
                // Basic search - always visible
                ui.horizontal_wrapped(|ui| {
                    ui.label("Search:");
                    if ui.text_edit_singleline(&mut self.search_query).changed() {
                        // Search query changed - will be applied automatically
                    }
                    if !self.search_query.is_empty() && ui.button("✖").clicked() {
                        self.search_query.clear();
                    }
                });
                
                // Advanced search options
                if self.show_advanced_search {
                    ui.add_space(5.0);
                    
                    // Column selection
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Search in:");
                        egui::ComboBox::from_id_salt("search_column")
                            .selected_text(self.search_column.display_name())
                            .show_ui(ui, |ui| {
                                for column in SearchColumn::all_columns() {
                                    ui.selectable_value(
                                        &mut self.search_column,
                                        column,
                                        column.display_name()
                                    );
                                }
                            });
                    });
                    
                    // Search tips
                    ui.add_space(5.0);
                    ui.small("Tip: For size column, you can search by 'KB', 'MB', etc.");
                }
            });
    }
}
