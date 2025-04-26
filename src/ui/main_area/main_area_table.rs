use egui::{Color32, Frame, Rounding, Stroke, Ui};
use std::time::Duration;

use super::{
    audio_file_info::AudioFileInfo, export_utils::ExportUtils, main_area_core::MainArea,
    replace_utils::ReplaceUtils, table_renderer::TableRenderer,
};

impl MainArea {
    /// Render the audio file table and handle export/play actions
    pub fn render_audio_table(
        &mut self,
        ui: &mut Ui,
        filtered_audio_files: Vec<AudioFileInfo>,
        files_count: usize,
        available_height: f32,
        available_width: f32,
    ) {
        // Use these variables to capture action information outside the immediate UI context
        // This way we can perform actions after all UI rendering is done to avoid multiple self borrowing
        struct ActionData {
            export_index: Option<usize>,
            play_index: Option<usize>,
            replace_index: Option<usize>,
            export_all: bool,
        }

        let mut action_data = ActionData {
            export_index: None,
            play_index: None,
            replace_index: None,
            export_all: false,
        };

        // First, render the UI
        Frame::group(ui.style())
            .stroke(Stroke::new(1.0, ui.visuals().faint_bg_color))
            .show(ui, |ui| {
                // Margins
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        // Table header
                        ui.heading("Audio File List");

                        // Capture Export All button click, don't act on it yet
                        if ui.button("Export All").clicked() {
                            action_data.export_all = true;
                        }

                        // File count display
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if !self.search_query.is_empty() {
                                ui.label(format!(
                                    "Found: {} / {} files",
                                    files_count,
                                    self.file_count.unwrap_or(0)
                                ));
                            } else {
                                ui.label(format!("Total: {} files", files_count));
                            }
                        });

                        ui.add_space(5.0);

                        // Empty results message
                        if !self.search_query.is_empty() && filtered_audio_files.is_empty() {
                            ui.add_space(8.0);
                            ui.label("No audio files match the search criteria.");
                        }

                        // The actual table rendering - capture actions but don't execute them yet
                        TableRenderer::render_table(
                            ui,
                            &filtered_audio_files,
                            &mut self.selected_rows,
                            self.striped,
                            self.clickable,
                            self.show_grid_lines,
                            available_height,
                            available_width,
                            &mut |index| {
                                action_data.export_index = Some(index);
                            },
                            &mut |index| {
                                action_data.play_index = Some(index);
                            },
                            &mut |index| {
                                action_data.replace_index = Some(index);
                            },
                            &mut self.sort_column,
                            &mut self.sort_ascending,
                        );

                        ui.add_space(8.0);
                    });
                    ui.add_space(8.0);
                });
                ui.add_space(8.0);
            });

        // Collect toast messages to add - we'll add them all at once to avoid multiple self.add_toast calls
        let mut toasts_to_add = Vec::new();

        // Process all actions and collect toast messages

        // Handle "Export All" action if clicked
        if action_data.export_all {
            let selected_file = self.selected_file.clone();
            let output_path = self.output_path.clone();

            if let Some(file_path) = &selected_file {
                if let Some(output_dir) = &output_path {
                    // Use ExportUtils to export all files
                    match ExportUtils::export_all_to_wav(file_path, output_dir) {
                        Ok(paths) => {
                            toasts_to_add.push((
                                format!(
                                    "Successfully exported {} files to: {}",
                                    paths.len(),
                                    output_dir
                                ),
                                Color32::GREEN,
                            ));
                        }
                        Err(e) => {
                            toasts_to_add.push((format!("Export failed: {}", e), Color32::RED));
                        }
                    }
                } else {
                    toasts_to_add.push((
                        "No output directory set. Please set an output directory.".to_string(),
                        Color32::GOLD,
                    ));
                }
            } else {
                toasts_to_add.push(("No file selected".to_string(), Color32::RED));
            }
        }

        // Handle "Export" action for a specific file if clicked
        if let Some(idx) = action_data.export_index {
            if idx < filtered_audio_files.len() {
                let audio_info = &filtered_audio_files[idx];
                let selected_file = self.selected_file.clone();
                let output_path = self.output_path.clone();

                if let Some(file_path) = &selected_file {
                    if let Some(output_dir) = &output_path {
                        match ExportUtils::export_to_wav_with_custom_dir(
                            audio_info, file_path, output_dir,
                        ) {
                            Ok(path) => {
                                toasts_to_add.push((
                                    format!("Successfully exported to: {}", path),
                                    Color32::GREEN,
                                ));
                            }
                            Err(e) => {
                                toasts_to_add.push((format!("Export failed: {}", e), Color32::RED));
                            }
                        }
                    } else {
                        toasts_to_add.push((
                            "No output directory set. Please set an output directory.".to_string(),
                            Color32::GOLD,
                        ));
                    }
                } else {
                    toasts_to_add.push(("No file selected".to_string(), Color32::RED));
                }
            }
        }

        // Handle "Play" action if clicked
        let mut play_started = false;
        if let Some(idx) = action_data.play_index {
            if idx < filtered_audio_files.len() {
                let audio_info = &filtered_audio_files[idx];
                let audio_name = audio_info.name.clone();
                let file_path = self.selected_file.clone();

                if let Some(path) = &file_path {
                    if let Some(audio_player) = &mut self.audio_player {
                        match audio_player.load_audio(audio_info, path) {
                            Ok(()) => {
                                // Start playing
                                let state = audio_player.get_audio_state();
                                let mut state = state.lock().unwrap();
                                if !state.is_playing {
                                    state.toggle_play();
                                    play_started = true;
                                }

                                toasts_to_add
                                    .push((format!("Now playing: {}", audio_name), Color32::GREEN));
                            }
                            Err(e) => {
                                toasts_to_add
                                    .push((format!("Failed to load audio: {}", e), Color32::RED));
                            }
                        }
                    } else {
                        toasts_to_add
                            .push(("Audio player is not initialized".to_string(), Color32::RED));
                    }
                } else {
                    toasts_to_add.push(("No file selected".to_string(), Color32::RED));
                }
            }
        }
        
        // Handle "Replace" action if clicked
        if let Some(idx) = action_data.replace_index {
            if idx < filtered_audio_files.len() {
                let audio_info = &filtered_audio_files[idx];
                let selected_file = self.selected_file.clone();

                if let Some(file_path) = &selected_file {
                    // Use ReplaceUtils to replace the audio file in memory
                    match ReplaceUtils::replace_with_file_dialog(audio_info, file_path) {
                        Ok(new_audio_info) => {
                            // Update the audio file in memory if we have audio_files
                            if let Some(ref mut audio_files) = self.audio_files {
                                // Find the original audio file in the full list
                                if let Some(original_idx) = audio_files.iter().position(|f| f.name == audio_info.name && f.id == audio_info.id) {
                                    // Replace with the new audio info
                                    audio_files[original_idx] = new_audio_info.clone();
                                    
                                    toasts_to_add.push((
                                        format!("Successfully replaced audio in memory: {}", audio_info.name),
                                        Color32::GREEN,
                                    ));
                                } else {
                                    toasts_to_add.push((
                                        format!("Could not find original audio file in memory: {}", audio_info.name),
                                        Color32::RED,
                                    ));
                                }
                            } else {
                                toasts_to_add.push((
                                    "Audio files not loaded in memory".to_string(),
                                    Color32::RED,
                                ));
                            }
                        }
                        Err(e) => {
                            toasts_to_add.push((format!("Replace failed: {}", e), Color32::RED));
                        }
                    }
                } else {
                    toasts_to_add.push(("No file selected".to_string(), Color32::RED));
                }
            }
        }

        // Add all collected toast messages at once
        for (message, color) in toasts_to_add {
            self.add_toast(message, color);
        }
    }
}
