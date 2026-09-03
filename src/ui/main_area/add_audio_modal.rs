use super::audio_file_info::AudioFileInfo;
use crate::ui::audio_player::PreviewTransport;
use crate::ui::waveform::{
    DEFAULT_PEAK_BINS, WaveformAction, WaveformOptions, WaveformPeaks, WaveformWidget,
};
use crate::{Locale, localized};
use egui::{Color32, Context, CornerRadius, Frame, RichText, ScrollArea, Ui, Window};
use egui_phosphor::regular;
use hound;
use mp3_duration;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

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
    /// Gain in decibels to apply after import
    pub gain_db: f32,
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
    /// Independent preview player for the chosen source file
    preview: PreviewTransport,
    /// Cached sound-wave peaks for the source file
    waveform: Option<WaveformPeaks>,
    /// Background peak job
    waveform_rx: Option<Receiver<(u64, WaveformPeaks)>>,
    waveform_generation: u64,
    waveform_loading: bool,
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
            preview: PreviewTransport::new(),
            waveform: None,
            waveform_rx: None,
            waveform_generation: 0,
            waveform_loading: false,
        }
    }

    /// Get the actual duration of an audio file by decoding it
    fn get_actual_audio_duration(&self, file_path: &str) -> Option<f32> {
        let path_lower = file_path.to_lowercase();

        if path_lower.ends_with(".wav") {
            match hound::WavReader::open(file_path) {
                Ok(reader) => {
                    let spec = reader.spec();
                    let samples = reader.duration();
                    if spec.sample_rate > 0 {
                        let duration_secs = samples as f32 / spec.sample_rate as f32;
                        println!("Got duration from WAV header: {:.2}s", duration_secs);
                        return Some(duration_secs);
                    }
                }
                Err(e) => {
                    println!("Failed to read WAV header: {}", e);
                }
            }
        }

        if path_lower.ends_with(".mp3") {
            match mp3_duration::from_path(file_path) {
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

        None
    }

    /// Open the modal with a selected audio file
    pub fn open_with_file(
        &mut self,
        file_path: &str,
        existing_audio_files: Option<Vec<AudioFileInfo>>,
        _locale: Locale,
    ) {
        println!("Opening add audio modal with file: {}", file_path);

        self.existing_audio_files = existing_audio_files;
        self.settings.file_path = Some(file_path.to_string());
        self.settings.gain_db = 0.0;
        self.error = None;
        self.waveform = None;
        self.waveform_loading = true;
        self.waveform_generation = self.waveform_generation.wrapping_add(1);
        let wave_gen = self.waveform_generation;
        self.waveform_rx = Some(WaveformPeaks::spawn_load(
            PathBuf::from(file_path),
            DEFAULT_PEAK_BINS,
            wave_gen,
        ));

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
                let effective_audio_list = Nus3audioFileUtils::get_effective_audio_list(
                    self.existing_audio_files.as_ref(),
                );

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
                        println!("Using estimated duration for new audio: {:.2}s", estimated);
                        estimated
                    }
                };

                self.settings.estimated_duration = duration;
            }
            Err(e) => {
                println!("Failed to read audio file: {}", e);
                self.error = Some(localized::failed_read_audio(&e));
                self.file_data = None;
            }
        }

        if self.file_data.is_some()
            && let Some(path) = self.settings.file_path.clone()
        {
            match self.preview.load(&path) {
                Ok(()) => {
                    self.preview.set_gain_db(0.0);
                    if self.preview.duration() > 0.0 {
                        self.settings.estimated_duration = self.preview.duration();
                    }
                }
                Err(e) => {
                    log::error!("Failed to load add-audio preview: {e}");
                }
            }
        }

        self.open = true;
        self.confirmed = false;
    }

    /// Close the modal
    pub fn close(&mut self) {
        self.preview.stop();
        self.waveform_rx = None;
        self.waveform_loading = false;
        self.open = false;
        // Clear file data to free memory
        self.file_data = None;
    }

    /// Dismiss without confirming (Escape / window close).
    pub fn dismiss_without_confirm(&mut self) {
        self.confirmed = false;
        self.close();
    }

    /// Keyboard: play/pause the dialog preview.
    pub fn toggle_preview_play(&mut self) {
        self.preview.toggle_play();
    }

    /// Keyboard: stop preview without unloading.
    pub fn stop_preview(&mut self) {
        self.preview.stop_to_start();
    }

    /// Reset the confirmed flag
    pub fn reset_confirmed(&mut self) {
        self.confirmed = false;
    }

    fn poll_waveform(&mut self) {
        let Some(rx) = self.waveform_rx.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok((wave_gen, peaks)) => {
                if wave_gen != self.waveform_generation {
                    return;
                }
                self.waveform_rx = None;
                self.waveform_loading = false;
                if !peaks.is_empty() {
                    if peaks.duration_secs > 0.0 {
                        self.settings.estimated_duration = peaks.duration_secs;
                        self.preview.set_duration_if_unknown(peaks.duration_secs);
                    }
                    println!(
                        "Add modal waveform ready: {} bins, {:.2}s",
                        peaks.peaks.len(),
                        peaks.duration_secs
                    );
                    self.waveform = Some(peaks);
                } else {
                    println!("Add modal waveform empty after async load");
                    self.waveform = None;
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.waveform_rx = None;
                self.waveform_loading = false;
            }
        }
    }

    /// Show the modal window
    pub fn show(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        self.poll_waveform();
        self.preview.tick(None, None, false);
        ctx.request_repaint();

        let available_rect = ctx.content_rect();
        let min_width = (available_rect.width() * 0.58).clamp(520.0, 960.0);
        let min_height = (available_rect.height() * 0.62).clamp(480.0, 780.0);
        let mut open = self.open;
        Window::new(localized::add_new_audio_title())
            .id(egui::Id::new("add_audio_modal"))
            .open(&mut open)
            .min_width(min_width)
            .default_width(min_width)
            .min_height(min_height)
            .resizable(true)
            .movable(true)
            .collapsible(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(available_rect.center())
            .show(ctx, |ui| {
                self.render_content(ui);
            });

        if !open && self.open {
            self.dismiss_without_confirm();
        }
    }

    /// Render modal content
    fn render_content(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(10.0);
            ui.heading(localized::add_new_audio_title());
            ui.add_space(10.0);
        });

        if let Some(error) = &self.error {
            ui.label(localized::error_label());
            ui.colored_label(egui::Color32::RED, error);
            ui.add_space(10.0);
            ui.separator();
        }

        // If we have file data, show form
        if self.file_data.is_some() {
            ScrollArea::vertical().show(ui, |ui| {
                // File information
                ui.vertical_centered(|ui| {
                    ui.heading(localized::file_information());
                    ui.add_space(10.0);
                });

                // Show file path
                if let Some(file_path) = &self.settings.file_path {
                    ui.horizontal(|ui| {
                        ui.label(localized::selected_file_label());
                        ui.label(file_path);
                    });
                }

                ui.add_space(10.0);

                // Duration (estimated or actual)
                ui.horizontal(|ui| {
                    ui.label(localized::duration_label());
                    ui.label(localized::seconds_fmt(self.settings.estimated_duration));
                });

                ui.add_space(10.0);
                self.render_preview_transport(ui);

                ui.add_space(8.0);
                self.render_waveform(ui);

                ui.add_space(10.0);
                self.render_gain_controls(ui);

                ui.add_space(20.0);
                ui.separator();
                ui.add_space(10.0);

                // Audio metadata input fields
                ui.vertical_centered(|ui| {
                    ui.heading(localized::audio_metadata());
                    ui.add_space(10.0);
                });

                // Name input
                ui.horizontal(|ui| {
                    ui.label(localized::name_label());
                    ui.text_edit_singleline(&mut self.settings.name);
                });

                // Show error if name already exists (check effective audio list)
                let name_exists = if !self.settings.name.is_empty() {
                    use super::nus3audio_file_utils::Nus3audioFileUtils;
                    let effective_audio_list = Nus3audioFileUtils::get_effective_audio_list(
                        self.existing_audio_files.as_ref(),
                    );
                    effective_audio_list
                        .iter()
                        .any(|(_, name)| *name == self.settings.name)
                } else {
                    false
                };

                if name_exists {
                    ui.colored_label(egui::Color32::RED, localized::name_exists_error());
                }

                // ID input
                ui.horizontal(|ui| {
                    ui.label(localized::id_label());
                    ui.text_edit_singleline(&mut self.settings.id);
                });

                // Show error if ID already exists (check effective audio list)
                let id_exists = if !self.settings.id.is_empty() {
                    use super::nus3audio_file_utils::Nus3audioFileUtils;
                    let effective_audio_list = Nus3audioFileUtils::get_effective_audio_list(
                        self.existing_audio_files.as_ref(),
                    );
                    effective_audio_list
                        .iter()
                        .any(|(id, _)| *id == self.settings.id)
                } else {
                    false
                };

                if id_exists {
                    ui.colored_label(egui::Color32::RED, localized::id_exists_error());
                }

                ui.add_space(20.0);
            });

            ui.separator();
            ui.add_space(10.0);

            // Control buttons
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(localized::cancel()).clicked() {
                        self.dismiss_without_confirm();
                    }

                    // Disable confirm button if there are validation errors
                    let name_exists = if !self.settings.name.is_empty() {
                        use super::nus3audio_file_utils::Nus3audioFileUtils;
                        let effective_audio_list = Nus3audioFileUtils::get_effective_audio_list(
                            self.existing_audio_files.as_ref(),
                        );
                        effective_audio_list
                            .iter()
                            .any(|(_, name)| *name == self.settings.name)
                    } else {
                        false
                    };

                    let id_exists = if !self.settings.id.is_empty() {
                        use super::nus3audio_file_utils::Nus3audioFileUtils;
                        let effective_audio_list = Nus3audioFileUtils::get_effective_audio_list(
                            self.existing_audio_files.as_ref(),
                        );
                        effective_audio_list
                            .iter()
                            .any(|(id, _)| *id == self.settings.id)
                    } else {
                        false
                    };

                    let has_validation_errors = name_exists
                        || id_exists
                        || self.settings.name.is_empty()
                        || self.settings.id.is_empty();

                    if ui
                        .add_enabled(
                            !has_validation_errors,
                            egui::Button::new(localized::confirm()),
                        )
                        .clicked()
                    {
                        self.preview.stop();
                        self.confirmed = true;
                        self.open = false;
                    }
                });
            });
        } else {
            // No file data
            ui.label(localized::no_audio_loaded());

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(10.0);

            // Just show cancel button
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(localized::cancel()).clicked() {
                        self.dismiss_without_confirm();
                    }
                });
            });
        }
    }

    fn render_preview_transport(&mut self, ui: &mut Ui) {
        Frame::new()
            .inner_margin(egui::Margin::symmetric(10, 8))
            .corner_radius(CornerRadius::same(8))
            .fill(Color32::from_rgb(20, 20, 22))
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;

                    let play_icon = if self.preview.is_playing() {
                        regular::PAUSE_CIRCLE
                    } else {
                        regular::PLAY_CIRCLE
                    };
                    let play_color = if self.preview.is_loaded() {
                        if self.preview.is_playing() {
                            Color32::from_rgb(255, 200, 100)
                        } else {
                            Color32::from_rgb(100, 220, 150)
                        }
                    } else {
                        ui.visuals().weak_text_color()
                    };

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(play_icon.to_string())
                                    .size(28.0)
                                    .color(play_color),
                            )
                            .frame(false),
                        )
                        .on_hover_text(if self.preview.is_playing() {
                            localized::pause_tooltip()
                        } else {
                            localized::play_tooltip_player()
                        })
                        .clicked()
                        && self.preview.is_loaded()
                    {
                        self.preview.toggle_play();
                    }

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(regular::STOP_CIRCLE.to_string())
                                    .size(22.0)
                                    .color(Color32::from_rgb(220, 100, 100)),
                            )
                            .frame(false),
                        )
                        .on_hover_text(localized::stop_playback_tooltip())
                        .clicked()
                        && self.preview.is_loaded()
                    {
                        self.preview.stop_to_start();
                    }

                    let pos = format_mmss(self.preview.position());
                    let dur = format_mmss(
                        self.preview
                            .duration()
                            .max(self.settings.estimated_duration),
                    );
                    ui.label(
                        RichText::new(format!("{pos} / {dur}"))
                            .monospace()
                            .size(12.0)
                            .color(ui.visuals().weak_text_color()),
                    );

                    ui.add_space(8.0);

                    let (vol_icon, vol_color) = if self.preview.volume() <= 0.001 {
                        (regular::SPEAKER_X, Color32::from_gray(150))
                    } else if self.preview.volume() < 0.33 {
                        (regular::SPEAKER_LOW, Color32::from_rgb(100, 150, 255))
                    } else {
                        (regular::SPEAKER_HIGH, Color32::from_rgb(100, 150, 255))
                    };
                    ui.label(
                        RichText::new(vol_icon.to_string())
                            .size(16.0)
                            .color(vol_color),
                    );

                    let mut vol_pct = self.preview.volume() * 100.0;
                    ui.spacing_mut().slider_width = 110.0;
                    if ui
                        .add(
                            egui::Slider::new(&mut vol_pct, 0.0..=100.0)
                                .show_value(false)
                                .trailing_fill(true),
                        )
                        .changed()
                    {
                        self.preview.set_volume(vol_pct / 100.0);
                    }

                    if !self.preview.is_loaded() {
                        ui.label(
                            RichText::new(localized::preview_unavailable())
                                .size(11.0)
                                .color(Color32::from_rgb(220, 120, 100)),
                        );
                    }
                });
            });
    }

    fn render_waveform(&mut self, ui: &mut Ui) {
        let wave_opts = WaveformOptions {
            height: 120.0,
            playhead_secs: if self.preview.is_loaded() {
                Some(self.preview.position())
            } else {
                None
            },
            loop_start_secs: None,
            loop_end_secs: None,
            show_loop: false,
            interactive_loop: false,
            interactive_seek: self.preview.is_loaded(),
            duration_override: Some(
                self.settings
                    .estimated_duration
                    .max(self.preview.duration()),
            ),
            loading: self.waveform_loading && self.waveform.is_none(),
        };

        let (_resp, action) = WaveformWidget::show(
            ui,
            "add_audio_waveform",
            self.waveform.as_ref(),
            &wave_opts,
        );

        match action {
            WaveformAction::Scrub(t) => {
                self.preview.scrub(t);
            }
            WaveformAction::Seek(t) => {
                self.preview.seek(t);
            }
            WaveformAction::LoopStart(_)
            | WaveformAction::LoopEnd(_)
            | WaveformAction::None => {}
        }

        ui.add_space(4.0);
        if self.waveform_loading && self.waveform.is_none() {
            ui.label(
                RichText::new(localized::waveform_loading())
                    .size(11.0)
                    .color(ui.visuals().weak_text_color()),
            );
        } else if self.waveform.is_none() {
            ui.label(
                RichText::new(localized::waveform_preview_unavailable())
                    .size(11.0)
                    .italics()
                    .color(ui.visuals().weak_text_color()),
            );
        }
    }

    fn render_gain_controls(&mut self, ui: &mut Ui) {
        ui.label(RichText::new(localized::gain_heading()).strong().size(14.0));
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.label(localized::gain_db_label());
            let mut gain_value = self.settings.gain_db;
            let gain_resp = ui.add(
                egui::Slider::new(&mut gain_value, -24.0..=24.0)
                    .suffix(" dB")
                    .trailing_fill(true),
            );
            if gain_resp.changed() || gain_resp.drag_stopped() {
                self.settings.gain_db = gain_value;
                self.preview.set_gain_db(gain_value);
            }
        });

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if ui.button("-6 dB").clicked() {
                self.settings.gain_db = -6.0;
                self.preview.set_gain_db(-6.0);
            }
            if ui.button("+6 dB").clicked() {
                self.settings.gain_db = 6.0;
                self.preview.set_gain_db(6.0);
            }
            if ui.button(localized::reset_gain()).clicked() {
                self.settings.gain_db = 0.0;
                self.preview.set_gain_db(0.0);
            }
        });

        let linear_factor = 10f32.powf(self.settings.gain_db / 20.0);
        ui.label(
            RichText::new(localized::linear_factor(linear_factor))
                .size(11.0)
                .color(ui.visuals().weak_text_color()),
        );
        ui.label(
            RichText::new(localized::gain_live_preview_hint())
                .size(11.0)
                .italics()
                .color(ui.visuals().weak_text_color()),
        );
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

fn format_mmss(secs: f32) -> String {
    let s = secs.max(0.0);
    let m = (s / 60.0).floor() as u32;
    let r = (s % 60.0).floor() as u32;
    format!("{m:02}:{r:02}")
}
