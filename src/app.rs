use crate::ui::{TopPanel, FileList};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    file_list: FileList,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            file_list: FileList::new(),
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Set theme to dark
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Display top menu panel
        TopPanel::show(ctx);

        // First create the side panel with file list
        egui::SidePanel::left("file_list_panel")
            .resizable(true)
            .min_width(200.0)
            .default_width(350.0)
            .show(ctx, |ui| {
                // Display file list component
                if self.file_list.show(ui) {
                    // If a file is selected, handle it here
                    if let Some(selected) = &self.file_list.selected_file {
                        println!("Processing file: {}", selected);
                        // Add file processing logic here
                    }
                }
            });

        // Then create the central panel for the main editing area
        // This ensures it doesn't overlap with SidePanel as they are separate containers
        egui::CentralPanel::default().show(ctx, |ui| {
            // Main editing area
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                
                // Add other UI components here
                ui.heading("Audio Editor");
                
                if let Some(selected) = &self.file_list.selected_file {
                    ui.label(format!("Currently editing: {}", selected));
                    
                    // Add audio editor controls here
                    ui.add_space(20.0);
                    ui.label("Audio waveform display area");
                    
                    // Simulate audio waveform area
                    let rect = egui::Rect::from_min_size(
                        ui.cursor().min,
                        egui::vec2(ui.available_width(), 200.0),
                    );
                    ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(80, 80, 80));
                } else {
                    ui.label("Please select a file from the list on the left to edit");
                }
            });
        });
    }
}
