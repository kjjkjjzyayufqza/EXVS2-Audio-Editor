use crate::ui::{TopPanel, FileList, MainArea};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Remove skip attribute to persist file list between sessions
    file_list: FileList,
    #[serde(skip)]
    main_area: MainArea,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            file_list: FileList::new(),
            main_area: MainArea::new(),
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
                        // Update main area with selected file
                        self.main_area.update_selected_file(Some(selected.clone()));
                    }
                }
            });

        // Display the main editing area
        self.main_area.show(ctx);
    }
}
