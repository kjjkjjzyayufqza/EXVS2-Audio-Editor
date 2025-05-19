use egui::{
    Align2, Color32, Context, Rect, Vec2, Ui
};

use super::main_area_core::MainArea;

impl MainArea {
    /// Display the main editing area
    pub fn show(&mut self, ctx: &Context) {
        // Show the loop settings modal if open
        self.loop_settings_modal.show(ctx);
        
        // Show the add audio modal if open
        self.add_audio_modal.show(ctx);
        
        // Show the confirm modal if open
        self.confirm_modal.show(ctx);
        
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render(ui);
        });
    }

    /// Render the main area content
    pub fn render(&mut self, ui: &mut Ui) {
        // First, clean up expired toast messages
        self.toast_messages.retain(|toast| !toast.has_expired());
        let available_height = ui.available_height();
        let available_width = ui.available_width();

        ui.vertical_centered(|ui| {
            ui.add_space(10.0); // Reduced space to allow more content

            ui.heading("Audio Editor");
            
            // Render toast messages at the top
            self.render_toasts(ui);

            if let Some(selected) = &self.selected_file {
                // Display filename with ellipsis if too long
                let display_name = if selected.len() > 60 {
                    format!(
                        "{}...{}",
                        &selected[0..30],
                        &selected[selected.len() - 27..]
                    )
                } else {
                    selected.clone()
                };

                ui.label(format!("Currently editing: {}", display_name))
                    .on_hover_text(selected);

                ui.add_space(10.0); // Reduced space
                ui.heading("NUS3AUDIO Info Display");

                // Display file info if available
                if let Some(_audio_files) = &self.audio_files {
                    ui.add_space(10.0);

                    // Display file count
                    if let Some(count) = self.file_count {
                        ui.label(format!("Number of audio files: {}", count));
                        ui.add_space(5.0);
                    }

                    // Add search box before the table
                    self.render_search_box(ui);
                    ui.add_space(10.0);
                    
                    // Add output path selection
                    self.render_output_path(ui);
                    ui.add_space(10.0);

                    // Get filtered and sorted audio files
                    let filtered_audio_files = self.filtered_audio_files();
                    let files_count = filtered_audio_files.len();

                    // Render the table with audio files
                    self.render_audio_table(
                        ui, 
                        filtered_audio_files, 
                        files_count, 
                        available_height, 
                        available_width
                    );
                } else if let Some(error) = &self.error_message {
                    ui.add_space(10.0);
                    ui.colored_label(Color32::RED, error);
                } else {
                    println!("aaaa {}", selected);
                    let rect = Rect::from_min_size(
                        ui.cursor().min,
                        Vec2::new(ui.available_width(), 200.0),
                    );
                    ui.painter()
                        .rect_filled(rect, 4.0, Color32::from_rgb(80, 80, 80));
                    ui.add_space(200.0); // Add space to account for the rect

                    if selected.to_lowercase().ends_with(".nus3audio")
                        || selected.to_lowercase().ends_with(".nus3bank")
                    {
                        ui.label("Loading NUS3AUDIO file info...");
                    } else {
                        ui.label("Selected file is not a NUS3AUDIO file.");
                    }
                }
            } else {
                ui.label("Please select a file from the list on the left to edit");
            }
        });
    }
    
    /// Render toast notifications
    pub fn render_toasts(&self, ui: &mut Ui) {
        if self.toast_messages.is_empty() {
            return;
        }
        
        // Calculate spacing from top
        let spacing = 50.0;
        
        // Show toast messages
        for (i, toast) in self.toast_messages.iter().enumerate() {
            // Create a toast window at the top center of the screen
            let window_id = egui::Id::new("toast_message").with(i);
            let pos = [0.0, spacing + (i as f32 * 60.0)];
            
            egui::containers::Window::new("Toast")
                .id(window_id)
                .title_bar(false)
                .resizable(false)
                .movable(false)
                .anchor(Align2::CENTER_TOP, pos)
                .default_size([300.0, 40.0])
                .show(ui.ctx(), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.colored_label(toast.color, &toast.message);
                    });
                });
        }
    }
}
