use crate::ui::{FileList, MainArea, TopPanel};
use crate::version_check;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Remove skip attribute to persist file list between sessions
    file_list: FileList,
    // Remove skip attribute to persist main area settings (like output path) between sessions
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
    /// Get a reference to the main area
    pub fn main_area(&self) -> &MainArea {
        &self.main_area
    }
    
    /// Get a mutable reference to the main area
    pub fn main_area_mut(&mut self) -> &mut MainArea {
        &mut self.main_area
    }
    
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        // Start version check in background
        version_check::check_for_updates_async();

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            let mut app: Self = eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
            // Make sure audio player is initialized after deserialization
            app.main_area.ensure_audio_player_initialized();
            return app;
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
        // 自定義暗色主題，避免純黑色
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(32, 32, 32);   // 標題與播放器背景
        visuals.window_fill = egui::Color32::from_rgb(45, 45, 45);  // 主編輯區域背景
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(32, 32, 32);
        ctx.set_visuals(visuals);
        // Display top menu panel
        TopPanel::show(ctx, Some(self));

        // Display audio player (if initialized)
        if let Some(audio_player) = &mut self.main_area.audio_player {
            let action = audio_player.show(ctx);
            self.main_area.handle_audio_player_action(action);
            self.main_area.sync_audio_settings_from_player();
        }

        let available_rect = ctx.available_rect();
        let side_panel_width = available_rect.width() * 0.20;
        let side_panel_min_width = available_rect.width() * 0.14;

        // First create the side panel with file list
        egui::SidePanel::left("file_list_panel")
            .resizable(true)
            .min_width(side_panel_min_width)
            .default_width(side_panel_width)
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
