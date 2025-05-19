use egui::{Context, Window, Button, RichText, Color32};

pub struct ConfirmModal {
    pub open: bool,
    pub title: String,
    pub message: String,
    pub confirmed: bool,
    pub cancelled: bool,
}

impl Default for ConfirmModal {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfirmModal {
    pub fn new() -> Self {
        Self {
            open: false,
            title: "Confirm".to_string(),
            message: "Are you sure you want to perform this action?".to_string(),
            confirmed: false,
            cancelled: false,
        }
    }
    
    /// Open the confirm dialog
    pub fn open(&mut self, title: &str, message: &str) {
        self.title = title.to_string();
        self.message = message.to_string();
        self.open = true;
        self.confirmed = false;
        self.cancelled = false;
    }
    
    /// Close the confirm dialog
    pub fn close(&mut self) {
        self.open = false;
    }
    
    /// Reset the confirmed and cancelled state
    pub fn reset_state(&mut self) {
        self.confirmed = false;
        self.cancelled = false;
    }
    
    /// Show the confirm dialog
    pub fn show(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }
        
        Window::new(&self.title)
            .min_width(300.0)
            .min_height(150.0)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.label(&self.message);
                    ui.add_space(20.0);
                    
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Confirm button - red warning color
                            if ui.add(Button::new(
                                RichText::new("Confirm")
                                    .color(Color32::from_rgb(255, 255, 255))
                            ).fill(Color32::from_rgb(220, 50, 50))).clicked() {
                                self.confirmed = true;
                                self.open = false;
                            }
                            
                            ui.add_space(10.0);
                            
                            // Cancel button
                            if ui.button("Cancel").clicked() {
                                self.cancelled = true;
                                self.open = false;
                            }
                        });
                    });
                });
            });
    }
} 