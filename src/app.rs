/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    selected_file: Option<String>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            selected_file: None,
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
        // Top panel with just the File menu
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                }
                // Removed theme toggle and other buttons
            });
        });

        // Central panel with just a Select File button
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0); // Add some space at the top

                if ui.button("Select File").clicked() {
                    // Open file dialog
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        let path_str = path.to_string_lossy().to_string();
                        println!("Selected file: {}", path_str);
                        self.selected_file = Some(path_str);
                    }
                }

                // If a file was selected, show its path
                if let Some(path) = &self.selected_file {
                    ui.add_space(20.0);
                    ui.label(format!("Selected: {}", path));
                }
            });
        });
    }
}
