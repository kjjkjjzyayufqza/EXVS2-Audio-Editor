use egui::{Context, ViewportCommand};

/// Top menu panel component
pub struct TopPanel;

impl TopPanel {
    /// Display the top menu panel
    pub fn show(ctx: &Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // Don't show Quit button in web environment
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(ViewportCommand::Close);
                        }
                    });
                }
                
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        // Can display an about dialog here
                    }
                });
            });
        });
    }
}
