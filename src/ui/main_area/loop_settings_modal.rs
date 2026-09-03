use super::audio_file_info::AudioFileInfo;
use crate::localized;
use crate::ui::audio_player::PreviewTransport;
use crate::ui::waveform::{
    DEFAULT_PEAK_BINS, WaveformAction, WaveformOptions, WaveformPeaks, WaveformWidget,
};
use egui::{Color32, Context, CornerRadius, Frame, RichText, ScrollArea, Ui, Window};
use egui_phosphor::regular;
use hound;
use mp3_duration;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

/// Structure to hold loop settings
#[derive(Clone, Debug)]
pub struct LoopSettings {
    /// Loop start point in seconds
    pub loop_start: Option<f32>,
    /// Loop end point in seconds
    pub loop_end: Option<f32>,
    /// Whether to use the custom loop points
    pub use_custom_loop: bool,
    /// Whether to enable loop functionality
    pub enable_loop: bool,
    /// Estimated duration of the audio file (in seconds)
    pub estimated_duration: f32,
    /// Gain in decibels to apply after import
    pub gain_db: f32,
}

impl Default for LoopSettings {
    fn default() -> Self {
        Self {
            loop_start: None,
            loop_end: None,
            use_custom_loop: false,
            enable_loop: true,
            estimated_duration: 0.0,
            gain_db: 0.0,
        }
    }
}

/// Modal window for loop settings (replace / add flow)
pub struct LoopSettingsModal {
    /// Is the modal open
    pub open: bool,
    /// Audio file info
    pub audio_info: Option<AudioFileInfo>,
    /// Loop settings
    pub settings: LoopSettings,
    /// Whether settings were changed and confirmed by the user
    pub confirmed: bool,
    /// Path of the selected replacement / source audio
    source_path: Option<String>,
    /// Cached sound-wave peaks for the source file
    waveform: Option<WaveformPeaks>,
    /// Background peak job
    waveform_rx: Option<Receiver<(u64, WaveformPeaks)>>,
    waveform_generation: u64,
    waveform_loading: bool,
    /// Independent preview player
    preview: PreviewTransport,
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
            source_path: None,
            waveform: None,
            waveform_rx: None,
            waveform_generation: 0,
            waveform_loading: false,
            preview: PreviewTransport::new(),
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
                        return Some(samples as f32 / spec.sample_rate as f32);
                    }
                }
                Err(e) => {
                    println!("Failed to read WAV header: {e}");
                }
            }
        }

        if path_lower.ends_with(".mp3") {
            match mp3_duration::from_path(file_path) {
                Ok(duration) => return Some(duration.as_secs_f32()),
                Err(e) => {
                    println!("Failed to get mp3 duration: {e}");
                }
            }
        }

        None
    }

    /// Open the modal with audio info (non-blocking: wave loads async, preview ready for play)
    pub fn open_with_audio(&mut self, audio_info: AudioFileInfo, file_path: &str) {
        println!(
            "Opening loop settings modal for audio: {} (ID: {})",
            audio_info.name, audio_info.id
        );
        println!("Selected replacement file: {file_path}");

        self.audio_info = Some(audio_info.clone());
        self.source_path = Some(file_path.to_string());
        self.waveform = None;
        self.waveform_loading = true;
        self.waveform_generation = self.waveform_generation.wrapping_add(1);
        let wave_gen = self.waveform_generation;
        self.waveform_rx = Some(WaveformPeaks::spawn_load(
            PathBuf::from(file_path),
            DEFAULT_PEAK_BINS,
            wave_gen,
        ));

        let duration = match self.get_actual_audio_duration(file_path) {
            Some(d) => d,
            None => Self::estimate_duration_from_size(audio_info.size),
        };

        self.settings = LoopSettings {
            loop_start: None,
            loop_end: None,
            use_custom_loop: false,
            enable_loop: true,
            estimated_duration: duration,
            gain_db: 0.0,
        };

        // Load preview audio immediately (paused). Kira/symphonia handles common formats.
        match self.preview.load(file_path) {
            Ok(()) => {
                if self.preview.duration() > 0.0 {
                    self.settings.estimated_duration = self.preview.duration();
                }
                println!(
                    "Preview loaded: duration={:.2}s path={}",
                    self.preview.duration(),
                    file_path
                );
            }
            Err(e) => {
                log::error!("Failed to load preview audio: {e}");
                println!("Failed to load preview audio: {e}");
            }
        }

        self.open = true;
        self.confirmed = false;
    }

    /// Close the modal and stop preview
    pub fn close(&mut self) {
        self.preview.stop();
        self.waveform_rx = None;
        self.waveform_loading = false;
        self.open = false;
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

    /// Keyboard: stop preview without unloading (next play resumes the buffer).
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
                        // Prefer real PCM duration for wave / loop scrubbing
                        self.settings.estimated_duration = peaks.duration_secs;
                        self.preview.set_duration_if_unknown(peaks.duration_secs);
                    }
                    println!(
                        "Loop modal waveform ready: {} bins, {:.2}s",
                        peaks.peaks.len(),
                        peaks.duration_secs
                    );
                    self.waveform = Some(peaks);
                } else {
                    println!("Loop modal waveform empty after async load");
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

        // Tick preview; always repaint while modal is open so playhead + async wave update
        let use_custom = self.settings.enable_loop && self.settings.use_custom_loop;
        self.preview
            .tick(self.settings.loop_start, self.settings.loop_end, use_custom);
        // Keep UI live: position counter, wave worker, loading shimmer
        ctx.request_repaint();

        if let Some(audio_info) = &self.audio_info {
            let title = localized::loop_settings_title(&audio_info.name);
            let available_rect = ctx.content_rect();
            let min_width = (available_rect.width() * 0.58).clamp(520.0, 960.0);
            let min_height = (available_rect.height() * 0.62).clamp(480.0, 780.0);

            let mut open = self.open;
            // Do NOT use .anchor() — anchoring makes the window immovable.
            Window::new(&title)
                .id(egui::Id::new("loop_settings_modal"))
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
                // User closed via window X
                self.close();
            }
        }
    }

    /// Render modal content
    fn render_content(&mut self, ui: &mut Ui) {
        let Some(audio_info) = self.audio_info.clone() else {
            return;
        };

        // Prefer live backend duration
        if self.preview.is_loaded() && self.preview.duration() > 0.0 {
            self.settings.estimated_duration = self.preview.duration();
        }

        // Header strip
        Frame::new()
            .inner_margin(egui::Margin::symmetric(12, 10))
            .corner_radius(CornerRadius::same(8))
            .fill(Color32::from_rgb(22, 22, 24))
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(regular::WAVEFORM.to_string())
                            .size(18.0)
                            .color(Color32::from_rgb(100, 150, 255)),
                    );
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.set_max_width((ui.available_width() - 8.0).max(80.0));
                        ui.label(
                            RichText::new(&audio_info.name)
                                .strong()
                                .size(15.0)
                                .color(ui.visuals().strong_text_color()),
                        );
                        let path_label = self
                            .source_path
                            .as_deref()
                            .map(|p| {
                                std::path::Path::new(p)
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or(p)
                                    .to_string()
                            })
                            .unwrap_or_else(|| audio_info.filename.clone());
                        ui.label(
                            RichText::new(format!(
                                "{}  ·  {:.2}s",
                                path_label, self.settings.estimated_duration
                            ))
                            .size(11.0)
                            .color(ui.visuals().weak_text_color()),
                        );
                    });
                });
            });

        ui.add_space(8.0);

        // Preview transport bar (play / stop / volume) — works even before wave finishes
        self.render_preview_transport(ui);

        ui.add_space(8.0);

        let show_loop = self.settings.enable_loop && self.settings.use_custom_loop;
        if show_loop {
            if self.settings.loop_start.is_none() {
                self.settings.loop_start = Some(0.0);
            }
            if self.settings.loop_end.is_none() {
                self.settings.loop_end = Some(self.settings.estimated_duration);
            }
        }

        let wave_opts = WaveformOptions {
            height: 120.0,
            playhead_secs: if self.preview.is_loaded() {
                Some(self.preview.position())
            } else {
                None
            },
            loop_start_secs: self.settings.loop_start,
            loop_end_secs: self.settings.loop_end,
            show_loop,
            interactive_loop: show_loop,
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
            "loop_settings_waveform",
            self.waveform.as_ref(),
            &wave_opts,
        );

        match action {
            WaveformAction::LoopStart(t) => {
                let end = self
                    .settings
                    .loop_end
                    .unwrap_or(self.settings.estimated_duration);
                let t = t.clamp(0.0, end);
                self.settings.loop_start = Some(t);
                self.settings.use_custom_loop = true;
                self.settings.enable_loop = true;
            }
            WaveformAction::LoopEnd(t) => {
                let start = self.settings.loop_start.unwrap_or(0.0);
                let t = t.clamp(start, self.settings.estimated_duration.max(start));
                self.settings.loop_end = Some(t);
                self.settings.use_custom_loop = true;
                self.settings.enable_loop = true;
            }
            // Scrub: update playhead UI; throttle actual seeks inside preview
            WaveformAction::Scrub(t) => {
                self.preview.scrub(t);
            }
            WaveformAction::Seek(t) => {
                self.preview.seek(t);
            }
            WaveformAction::None => {}
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
        } else if show_loop {
            ui.label(
                RichText::new(localized::waveform_drag_loop_hint())
                    .size(11.0)
                    .color(ui.visuals().weak_text_color()),
            );
        }

        ui.add_space(10.0);

        ScrollArea::vertical()
            .id_salt("loop_settings_scroll")
            .max_height((ui.available_height() - 56.0).max(120.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(localized::loop_settings_heading())
                        .strong()
                        .size(14.0),
                );
                ui.add_space(6.0);

                ui.checkbox(&mut self.settings.enable_loop, localized::enable_loop());
                ui.add_space(4.0);

                if self.settings.enable_loop {
                    ui.checkbox(
                        &mut self.settings.use_custom_loop,
                        localized::use_custom_loop(),
                    );
                } else {
                    self.settings.use_custom_loop = false;
                    ui.add_enabled_ui(false, |ui| {
                        ui.checkbox(
                            &mut self.settings.use_custom_loop,
                            localized::use_custom_loop(),
                        );
                    });
                }

                if self.settings.enable_loop && self.settings.use_custom_loop {
                    ui.add_space(8.0);
                    let max_dur = self.settings.estimated_duration.max(0.01);

                    ui.horizontal(|ui| {
                        ui.label(localized::loop_start_sec());
                        let mut start_value = self.settings.loop_start.unwrap_or(0.0);
                        if ui
                            .add(
                                egui::DragValue::new(&mut start_value)
                                    .speed(0.05)
                                    .range(0.0..=max_dur)
                                    .suffix(" s")
                                    .min_decimals(2)
                                    .max_decimals(3),
                            )
                            .changed()
                        {
                            self.settings.loop_start = Some(start_value);
                            if let Some(end) = self.settings.loop_end {
                                if start_value > end {
                                    self.settings.loop_end = Some(start_value);
                                }
                            }
                        }
                        if ui
                            .small_button(localized::loop_set_from_playhead())
                            .on_hover_text(localized::loop_set_start_hint())
                            .clicked()
                        {
                            let t = self.preview.position().clamp(0.0, max_dur);
                            self.settings.loop_start = Some(t);
                            if let Some(end) = self.settings.loop_end {
                                if t > end {
                                    self.settings.loop_end = Some(t);
                                }
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label(localized::loop_end_sec());
                        let mut end_value = self
                            .settings
                            .loop_end
                            .unwrap_or(self.settings.estimated_duration);
                        let min_end = self.settings.loop_start.unwrap_or(0.0);
                        if ui
                            .add(
                                egui::DragValue::new(&mut end_value)
                                    .speed(0.05)
                                    .range(min_end..=max_dur)
                                    .suffix(" s")
                                    .min_decimals(2)
                                    .max_decimals(3),
                            )
                            .changed()
                        {
                            self.settings.loop_end = Some(end_value);
                        }
                        if ui
                            .small_button(localized::loop_set_from_playhead())
                            .on_hover_text(localized::loop_set_end_hint())
                            .clicked()
                        {
                            let t = self.preview.position().clamp(min_end, max_dur);
                            self.settings.loop_end = Some(t);
                        }
                    });

                    let loop_duration = match (self.settings.loop_start, self.settings.loop_end) {
                        (Some(start), Some(end)) => (end - start).max(0.0),
                        _ => self.settings.estimated_duration,
                    };

                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(localized::loop_duration_sec(loop_duration))
                            .color(Color32::from_rgb(232, 176, 72)),
                    );

                    ui.horizontal(|ui| {
                        if ui.button(localized::loop_preview_ab()).clicked() {
                            let start = self.settings.loop_start.unwrap_or(0.0);
                            self.preview.seek(start);
                            if !self.preview.is_playing() {
                                self.preview.toggle_play();
                            }
                        }
                    });
                } else if self.settings.enable_loop {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(localized::loop_full_track())
                            .color(ui.visuals().weak_text_color()),
                    );
                } else {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(localized::loop_disabled())
                            .color(ui.visuals().weak_text_color()),
                    );
                }

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(10.0);

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
                    // Live preview: every drag frame + final release
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

                ui.add_space(8.0);
            });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                if ui
                    .add_sized([96.0, 28.0], egui::Button::new(localized::confirm()))
                    .clicked()
                {
                    self.preview.stop();
                    self.confirmed = true;
                    self.open = false;
                }
                if ui
                    .add_sized([96.0, 28.0], egui::Button::new(localized::cancel()))
                    .clicked()
                {
                    self.close();
                }
            });
        });
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

                    // Volume
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

    fn estimate_duration_from_size(size_bytes: usize) -> f32 {
        let bytes_per_second = 16000.0;
        let estimated_seconds = size_bytes as f32 / bytes_per_second;
        estimated_seconds.max(1.0).min(600.0)
    }
}

fn format_mmss(secs: f32) -> String {
    let s = secs.max(0.0);
    let m = (s / 60.0).floor() as u32;
    let r = (s % 60.0).floor() as u32;
    format!("{m:02}:{r:02}")
}
