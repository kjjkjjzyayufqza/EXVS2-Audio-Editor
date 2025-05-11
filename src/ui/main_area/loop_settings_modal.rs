use super::audio_file_info::AudioFileInfo;
use super::replace_utils::ReplaceUtils;
use egui::{Context, ScrollArea, Ui, Window};
use rodio::{Decoder, Source};
use std::io::Cursor;
use mp3_duration;

/// Structure to hold loop settings
#[derive(Clone, Debug)]
pub struct LoopSettings {
    /// Loop start point in seconds
    pub loop_start: Option<f32>,
    /// Loop end point in seconds
    pub loop_end: Option<f32>,
    /// Whether to use the custom loop points
    pub use_custom_loop: bool,
    /// Estimated duration of the audio file (in seconds)
    pub estimated_duration: f32,
}

impl Default for LoopSettings {
    fn default() -> Self {
        Self {
            loop_start: None,
            loop_end: None,
            use_custom_loop: false,
            estimated_duration: 0.0,
        }
    }
}

/// Modal window for loop settings
pub struct LoopSettingsModal {
    /// Is the modal open
    pub open: bool,
    /// Audio file info
    pub audio_info: Option<AudioFileInfo>,
    /// Loop settings
    pub settings: LoopSettings,
    /// Whether settings were changed and confirmed by the user
    pub confirmed: bool,
}

impl Default for LoopSettingsModal {
    fn default() -> Self {
        Self::new()
    }
}

impl LoopSettingsModal {
    /// Create a new loop settings modal
    pub fn new() -> Self {
        Self {
            open: false,
            audio_info: None,
            settings: LoopSettings::default(),
            confirmed: false,
        }
    }

    /// Get the actual duration of an audio file by decoding it
    fn get_actual_audio_duration(&self, file_path: &str) -> Option<f32> {
        // Return early if no replacement file exists
        let path = file_path;

        // Read the file
        let file_data = match std::fs::read(&path) {
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
        match Decoder::new(Cursor::new(file_data)) {
            Ok(decoder) => {
                if let Some(duration) = decoder.total_duration() {
                    let duration_secs = duration.as_secs_f32();
                    println!("Decoded audio duration: {:.2}s", duration_secs);
                    Some(duration_secs)
                } else {
                    println!("Could not determine audio duration from rodio decoder, trying mp3_duration");
                    
                    // Try mp3_duration if rodio couldn't determine the duration
                    match mp3_duration::from_path(&path) {
                        Ok(duration) => {
                            let duration_secs = duration.as_secs_f32();
                            println!("MP3 duration: {:.2}s", duration_secs);
                            Some(duration_secs)
                        }
                        Err(e) => {
                            println!("Failed to get mp3 duration: {}", e);
                            // Return 0 as fallback
                            Some(0.0)
                        }
                    }
                }
            }
            Err(e) => {
                println!("Failed to decode with rodio: {}, trying mp3_duration", e);
                
                // Try mp3_duration if rodio failed
                match mp3_duration::from_path(&path) {
                    Ok(duration) => {
                        let duration_secs = duration.as_secs_f32();
                        println!("MP3 duration: {:.2}s", duration_secs);
                        Some(duration_secs)
                    }
                    Err(e) => {
                        println!("Failed to get mp3 duration: {}", e);
                        // Return 0 as fallback
                        Some(0.0)
                    }
                }
            }
        }
    }

    /// Open the modal with audio info
    pub fn open_with_audio(&mut self, audio_info: AudioFileInfo, file_path: &str) {
        println!("Opening loop settings modal for audio: {} (ID: {})", audio_info.name, audio_info.id);
        println!("Selected replacement file: {}", file_path);
        
        self.audio_info = Some(audio_info.clone());
        // First try to get the actual duration from the audio file
        let duration = match self.get_actual_audio_duration(file_path) {
            Some(actual_duration) => {
                println!(
                    "Using actual duration for {}: {:.2}s",
                    audio_info.name, actual_duration
                );
                actual_duration
            }
            None => {
                // Fall back to estimation if we couldn't get the actual duration
                let estimated = Self::estimate_duration_from_size(audio_info.size);
                println!(
                    "Using estimated duration for {}: {:.2}s",
                    audio_info.name, estimated
                );
                estimated
            }
        };

        self.settings = LoopSettings {
            loop_start: None,
            loop_end: None,
            use_custom_loop: false,
            estimated_duration: duration,
        };

        self.open = true;
        self.confirmed = false;
    }

    /// Close the modal
    pub fn close(&mut self) {
        self.open = false;
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

        if let Some(audio_info) = &self.audio_info {
            let title = format!("Loop Settings - {}", audio_info.name);

            Window::new(&title)
                .min_width(400.0)
                .min_height(300.0)
                .resizable(true)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    self.render_content(ui);
                });
        }
    }

    /// Render modal content
    fn render_content(&mut self, ui: &mut Ui) {
        if let Some(audio_info) = &self.audio_info {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                ui.heading("Audio Information");
                ui.add_space(10.0);
            });

            // Audio information section - simplified to only show name
            ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("audio_info_grid")
                    .num_columns(2)
                    .spacing([10.0, 10.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Name:");
                        ui.label(&audio_info.name);
                        ui.end_row();
                    });

                ui.add_space(20.0);

                // Loop settings section
                ui.vertical_centered(|ui| {
                    ui.heading("Loop Settings");
                    ui.add_space(10.0);
                });

                ui.checkbox(&mut self.settings.use_custom_loop, "Use custom loop points");

                if self.settings.use_custom_loop {
                    ui.add_space(10.0);

                    // Loop start input
                    ui.horizontal(|ui| {
                        ui.label("Loop Start (seconds):");
                        let mut start_value = self.settings.loop_start.unwrap_or(0.0);
                        if ui
                            .add(
                                egui::DragValue::new(&mut start_value)
                                    .speed(0.1)
                                    .range(0.0..=self.settings.estimated_duration)
                                    .suffix("s"),
                            )
                            .changed()
                        {
                            self.settings.loop_start = Some(start_value);

                            // Ensure loop_start <= loop_end if loop_end is set
                            if let Some(end) = self.settings.loop_end {
                                if start_value > end {
                                    self.settings.loop_end = Some(start_value);
                                }
                            }
                        }
                    });

                    // Loop end input
                    ui.horizontal(|ui| {
                        ui.label("Loop End (seconds):");
                        let mut end_value = self
                            .settings
                            .loop_end
                            .unwrap_or(self.settings.estimated_duration);
                        if ui
                            .add(
                                egui::DragValue::new(&mut end_value)
                                    .speed(0.1)
                                    .range(
                                        self.settings.loop_start.unwrap_or(0.0)
                                            ..=self.settings.estimated_duration,
                                    )
                                    .suffix("s"),
                            )
                            .changed()
                        {
                            self.settings.loop_end = Some(end_value);
                        }
                    });

                    // Show loop duration
                    let loop_duration = match (self.settings.loop_start, self.settings.loop_end) {
                        (Some(start), Some(end)) => end - start,
                        _ => self.settings.estimated_duration,
                    };

                    ui.add_space(10.0);
                    ui.label(format!("Loop Duration: {:.2} seconds", loop_duration));
                } else {
                    ui.label("Audio will loop from beginning to end");
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

                    if ui.button("Confirm").clicked() {
                        self.confirmed = true;
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
