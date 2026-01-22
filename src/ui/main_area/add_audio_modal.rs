use super::audio_file_info::AudioFileInfo;
use egui::{Context, ScrollArea, Ui, Window};
use std::fs;
use std::path::Path;
use rodio::{Decoder, Source};
use std::io::Cursor;
use mp3_duration;

/// Structure to hold new audio file settings
#[derive(Clone, Debug, Default)]
pub struct AddAudioSettings {
    /// Custom name for the new audio file
    pub name: String,
    /// Custom ID for the new audio file
    pub id: String,
    /// Estimated duration of the audio file (in seconds)
    pub estimated_duration: f32,
    /// Selected file path
    pub file_path: Option<String>,
}

/// Modal window for adding new audio files
pub struct AddAudioModal {
    /// Is the modal open
    pub open: bool,
    /// Settings for the new audio
    pub settings: AddAudioSettings,
    /// Whether settings were confirmed by the user
    pub confirmed: bool,
    /// Audio file data loaded from disk
    pub file_data: Option<Vec<u8>>,
    /// Error message (if any)
    pub error: Option<String>,
    /// Existing audio files (to check for duplicates)
    pub existing_audio_files: Option<Vec<AudioFileInfo>>,
}

impl Default for AddAudioModal {
    fn default() -> Self {
        Self::new()
    }
}

impl AddAudioModal {
    /// Create a new add audio modal
    pub fn new() -> Self {
        Self {
            open: false,
            settings: AddAudioSettings::default(),
            confirmed: false,
            file_data: None,
            error: None,
            existing_audio_files: None,
        }
    }

    /// Get the actual duration of an audio file by decoding it
    fn get_actual_audio_duration(&self, file_path: &str) -> Option<f32> {
        // Return early if no file path exists
        let path = file_path;

        // Read the file
        let file_data = match std::fs::read(path) {
            Ok(data) => {
                println!("Read {} bytes from audio file", data.len());
                data
            }
            Err(e) => {
                println!("Failed to read audio file {:?}: {}", path, e);
                return None;
            }
        };

        // Try to decode the audio with rodio to get its duration
        let cursor = Cursor::new(file_data.clone());

        // Try with rodio first
        match Decoder::new(cursor) {
            Ok(decoder) => {
                let duration_secs = decoder.total_duration().map(|d| d.as_secs_f32());
                
                if let Some(duration) = duration_secs {
                    println!("Got duration from rodio: {:.2}s", duration);
                    return Some(duration);
                }
            }
            Err(e) => {
                println!("Rodio error (will try mp3_duration next): {:?}", e);
            }
        }

        // Try with mp3_duration as a fallback for MP3 files
        if path.to_lowercase().ends_with(".mp3") {
            match mp3_duration::from_path(path) {
                Ok(duration) => {
                    let secs = duration.as_secs_f32();
                    println!("Got duration from mp3_duration: {:.2}s", secs);
                    return Some(secs);
                }
                Err(e) => {
                    println!("mp3_duration error: {:?}", e);
                }
            }
        }

        // If both methods failed, return None
        None
    }

    /// Open the modal with a selected audio file
    pub fn open_with_file(&mut self, file_path: &str, existing_audio_files: Option<Vec<AudioFileInfo>>) {
        println!("Opening add audio modal with file: {}", file_path);
        
        self.existing_audio_files = existing_audio_files;
        self.settings.file_path = Some(file_path.to_string());
        self.error = None;
        
        // Try to read the file data
        match fs::read(file_path) {
            Ok(data) => {
                self.file_data = Some(data);
                
                // Set a default name based on the filename
                let default_name = Path::new(file_path)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                
                self.settings.name = default_name;
                
                // Generate a unique ID considering all effective audio files (after pending changes)
                use super::nus3audio_file_utils::Nus3audioFileUtils;
                let effective_audio_list = Nus3audioFileUtils::get_effective_audio_list(self.existing_audio_files.as_ref());
                
                let mut max_id = 0;
                for (id_str, _) in effective_audio_list {
                    if let Ok(id) = id_str.parse::<i32>() {
                        if id > max_id {
                            max_id = id;
                        }
                    }
                }
                
                // Set the new ID to be max_id + 1, or 1000 if no existing files
                self.settings.id = if max_id > 0 {
                    (max_id + 1).to_string()
                } else {
                    "1000".to_string()
                };
                
                // Get file duration
                let duration = match self.get_actual_audio_duration(file_path) {
                    Some(actual_duration) => {
                        println!(
                            "Using actual duration for new audio: {:.2}s",
                            actual_duration
                        );
                        actual_duration
                    }
                    None => {
                        // Fall back to estimation based on file size
                        let file_size = self.file_data.as_ref().unwrap().len();
                        let estimated = Self::estimate_duration_from_size(file_size);
                        println!(
                            "Using estimated duration for new audio: {:.2}s",
                            estimated
                        );
                        estimated
                    }
                };
                
                self.settings.estimated_duration = duration;
            }
            Err(e) => {
                println!("Failed to read audio file: {}", e);
                self.error = Some(format!("Failed to read audio file: {}", e));
                self.file_data = None;
            }
        }
        
        self.open = true;
        self.confirmed = false;
    }

    /// Close the modal
    pub fn close(&mut self) {
        self.open = false;
        // Clear file data to free memory
        self.file_data = None;
    }

    /// Reset the confirmed flag
    pub fn reset_confirmed(&mut self) {
        self.confirmed = false;
    }

    /// Show the modal window
    pub fn show(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }
        let available_rect = ctx.available_rect();
        let min_width = available_rect.width() * 0.5;
        let min_height = available_rect.height() * 0.5;

        Window::new("Add New Audio File")
            .min_width(min_width)
            .min_height(min_height)
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                self.render_content(ui);
            });
    }

    /// Render modal content
    fn render_content(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(10.0);
            ui.heading("Add New Audio File");
            ui.add_space(10.0);
        });

        if let Some(error) = &self.error {
            ui.label("Error:");
            ui.colored_label(egui::Color32::RED, error);
            ui.add_space(10.0);
            ui.separator();
        }

        // If we have file data, show form
        if self.file_data.is_some() {
            ScrollArea::vertical().show(ui, |ui| {
                // File information
                ui.vertical_centered(|ui| {
                    ui.heading("File Information");
                    ui.add_space(10.0);
                });

                // Show file path
                if let Some(file_path) = &self.settings.file_path {
                    ui.horizontal(|ui| {
                        ui.label("Selected File:");
                        ui.label(file_path);
                    });
                }

                ui.add_space(10.0);

                // Duration (estimated or actual)
                ui.horizontal(|ui| {
                    ui.label("Duration:");
                    ui.label(format!("{:.2} seconds", self.settings.estimated_duration));
                });

                ui.add_space(20.0);
                ui.separator();
                ui.add_space(10.0);

                // Audio metadata input fields
                ui.vertical_centered(|ui| {
                    ui.heading("Audio Metadata");
                    ui.add_space(10.0);
                });

                // Name input
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.settings.name);
                });

                // Show error if name already exists (check effective audio list)
                let name_exists = if !self.settings.name.is_empty() {
                    use super::nus3audio_file_utils::Nus3audioFileUtils;
                    let effective_audio_list = Nus3audioFileUtils::get_effective_audio_list(self.existing_audio_files.as_ref());
                    effective_audio_list.iter().any(|(_, name)| *name == self.settings.name)
                } else {
                    false
                };

                if name_exists {
                    ui.colored_label(egui::Color32::RED, "Error: Name already exists!");
                }

                // ID input
                ui.horizontal(|ui| {
                    ui.label("ID:");
                    ui.text_edit_singleline(&mut self.settings.id);
                });

                // Show error if ID already exists (check effective audio list)
                let id_exists = if !self.settings.id.is_empty() {
                    use super::nus3audio_file_utils::Nus3audioFileUtils;
                    let effective_audio_list = Nus3audioFileUtils::get_effective_audio_list(self.existing_audio_files.as_ref());
                    effective_audio_list.iter().any(|(id, _)| *id == self.settings.id)
                } else {
                    false
                };

                if id_exists {
                    ui.colored_label(egui::Color32::RED, "Error: ID already exists!");
                }

                ui.add_space(20.0);
            });

            ui.separator();
            ui.add_space(10.0);

            // Control buttons
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        self.open = false;
                    }

                    // Disable confirm button if there are validation errors
                    let name_exists = if !self.settings.name.is_empty() {
                        use super::nus3audio_file_utils::Nus3audioFileUtils;
                        let effective_audio_list = Nus3audioFileUtils::get_effective_audio_list(self.existing_audio_files.as_ref());
                        effective_audio_list.iter().any(|(_, name)| *name == self.settings.name)
                    } else {
                        false
                    };
                    
                    let id_exists = if !self.settings.id.is_empty() {
                        use super::nus3audio_file_utils::Nus3audioFileUtils;
                        let effective_audio_list = Nus3audioFileUtils::get_effective_audio_list(self.existing_audio_files.as_ref());
                        effective_audio_list.iter().any(|(id, _)| *id == self.settings.id)
                    } else {
                        false
                    };

                    let has_validation_errors = name_exists || id_exists || 
                                               self.settings.name.is_empty() || 
                                               self.settings.id.is_empty();

                    if ui.add_enabled(!has_validation_errors, egui::Button::new("Confirm")).clicked() {
                        self.confirmed = true;
                        self.open = false;
                    }
                });
            });
        } else {
            // No file data
            ui.label("No audio file loaded. Please select a valid audio file.");
            
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(10.0);
            
            // Just show cancel button
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        self.open = false;
                    }
                });
            });
        }
    }

    /// Estimate audio duration from file size (rough approximation)
    fn estimate_duration_from_size(size_bytes: usize) -> f32 {
        // Very rough estimate: Assuming ~16KB per second for compressed audio
        // This would vary greatly by format and compression
        let bytes_per_second = 16000.0;
        let estimated_seconds = size_bytes as f32 / bytes_per_second;

        // Clamp to reasonable values (at least 1 second, at most 10 minutes)
        estimated_seconds.max(1.0).min(600.0)
    }
} 