use super::audio_state::{AudioState, LoopMode};
use egui::{Align, Color32, CornerRadius, Frame, Layout, RichText, Ui, widgets::Slider};
use egui_phosphor::regular;
use std::sync::{Arc, Mutex};

/// Audio player controls component
pub struct AudioControls {
    /// Reference to the audio state
    audio_state: Arc<Mutex<AudioState>>,
}

impl AudioControls {
    /// Create a new audio controls component
    pub fn new(audio_state: Arc<Mutex<AudioState>>) -> Self {
        Self { audio_state }
    }

    /// Render the audio controls UI
    pub fn render(&mut self, ui: &mut Ui) {
        // Get a clone of the audio state to avoid holding the mutex lock during UI rendering
        let state_copy = {
            let state = self.audio_state.lock().unwrap();

            // Request continuous repainting if we're playing to update timers
            if state.is_playing {
                ui.ctx().request_repaint();
            }

            state.clone()
        };

        // Check if there's an audio file loaded
        let has_audio = state_copy.current_audio.is_some();

        let available_width = ui.available_width();

        // Music player style frame
        Frame::new()
            .inner_margin(12.0)
            .fill(ui.visuals().window_fill)
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .corner_radius(CornerRadius::same(12))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Top Row: Info and Volume
                    ui.horizontal(|ui| {
                        // Info Section (Left)
                        ui.allocate_ui_with_layout(
                            egui::vec2(available_width * 0.5, 45.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                if let Some(audio) = &state_copy.current_audio {
                                    ui.horizontal(|ui| {
                                        // Track Icon
                                        let icon = match audio.file_type.as_str() {
                                            "OPUS Audio" => regular::MUSIC_NOTE,
                                            "IDSP Audio" => regular::HEADPHONES,
                                            _ => regular::FILE_AUDIO,
                                        };

                                        let type_color = match audio.file_type.as_str() {
                                            "OPUS Audio" => Color32::from_rgb(100, 200, 100),
                                            "IDSP Audio" => Color32::from_rgb(100, 150, 255),
                                            _ => Color32::from_rgb(200, 150, 100),
                                        };

                                        ui.label(
                                            RichText::new(icon.to_string())
                                                .size(24.0)
                                                .color(type_color),
                                        );

                                        ui.vertical(|ui| {
                                            ui.label(
                                                RichText::new(&audio.name)
                                                    .color(ui.visuals().strong_text_color())
                                                    .size(15.0)
                                                    .strong(),
                                            );

                                            ui.label(
                                                RichText::new(&audio.file_type)
                                                    .color(type_color)
                                                    .size(11.0),
                                            );
                                        });
                                    });
                                } else {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(regular::MUSIC_NOTES_PLUS.to_string())
                                                .size(24.0)
                                                .color(ui.visuals().weak_text_color()),
                                        );
                                        ui.label(
                                            RichText::new("No track selected")
                                                .color(ui.visuals().weak_text_color())
                                                .italics()
                                                .size(14.0),
                                        );
                                    });
                                }
                            },
                        );

                        // Volume Section (Right)
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            self.render_volume_controls(ui, &state_copy);
                        });
                    });

                    ui.add_space(6.0);

                    // Middle Row: Progress Slider
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(state_copy.format_position())
                                .monospace()
                                .size(11.0)
                                .color(ui.visuals().weak_text_color()),
                        );

                        let mut progress = state_copy.progress();
                        let slider_width = ui.available_width() - 50.0;

                        // Custom styled slider
                        ui.spacing_mut().slider_width = slider_width;
                        let slider_response = ui.add(
                            Slider::new(&mut progress, 0.0..=1.0)
                                .show_value(false)
                                .text(""),
                        );

                        if slider_response.drag_stopped() && has_audio {
                            let mut state = self.audio_state.lock().unwrap();
                            let new_position = progress * state.total_duration;
                            state.set_position(new_position);
                        }

                        ui.label(
                            RichText::new(state_copy.format_duration())
                                .monospace()
                                .size(11.0)
                                .color(ui.visuals().weak_text_color()),
                        );
                    });

                    ui.add_space(6.0);

                    // Bottom Row: Playback Controls
                    ui.vertical_centered(|ui| {
                        ui.horizontal_centered(|ui| {
                            let accent_color = Color32::from_rgb(100, 150, 255);
                            ui.spacing_mut().item_spacing.x = 20.0;

                            // Shuffle Button
                            let shuffle_color = if state_copy.shuffle {
                                accent_color
                            } else {
                                ui.visuals().widgets.noninteractive.fg_stroke.color
                            };
                            let shuffle_btn = ui.add(
                                egui::Button::new(
                                    RichText::new(regular::SHUFFLE.to_string())
                                        .size(18.0)
                                        .color(shuffle_color),
                                )
                                .frame(false),
                            );
                            if shuffle_btn.on_hover_text("Shuffle").clicked() {
                                self.audio_state.lock().unwrap().toggle_shuffle();
                            }

                            // Previous Button
                            let prev_btn = ui.add(
                                egui::Button::new(
                                    RichText::new(regular::SKIP_BACK.to_string())
                                        .size(22.0),
                                )
                                .frame(false),
                            );
                            if prev_btn.on_hover_text("Previous Track").clicked() {
                                self.audio_state.lock().unwrap().previous_track();
                            }

                            // Play/Pause Button
                            let play_icon = if state_copy.is_playing {
                                regular::PAUSE_CIRCLE
                            } else {
                                regular::PLAY_CIRCLE
                            };
                            let play_color = if has_audio {
                                if state_copy.is_playing {
                                    Color32::from_rgb(255, 200, 100)
                                } else {
                                    Color32::from_rgb(100, 255, 150)
                                }
                            } else {
                                ui.visuals()
                                    .widgets
                                    .noninteractive
                                    .fg_stroke
                                    .color
                                    .gamma_multiply(0.5)
                            };

                            let play_btn = ui.add(
                                egui::Button::new(
                                    RichText::new(play_icon.to_string())
                                        .size(38.0)
                                        .color(play_color),
                                )
                                .frame(false),
                            );
                            if play_btn
                                .on_hover_text(if state_copy.is_playing {
                                    "Pause"
                                } else {
                                    "Play"
                                })
                                .clicked()
                                && has_audio
                            {
                                self.audio_state.lock().unwrap().toggle_play();
                            }

                            // Next Button
                            let next_btn = ui.add(
                                egui::Button::new(
                                    RichText::new(regular::SKIP_FORWARD.to_string())
                                        .size(22.0),
                                )
                                .frame(false),
                            );
                            if next_btn.on_hover_text("Next Track").clicked() {
                                self.audio_state.lock().unwrap().next_track();
                            }

                            // Loop Button
                            let (loop_icon, loop_color, loop_text) =
                                match state_copy.loop_mode {
                                    LoopMode::None => (
                                        regular::REPEAT,
                                        ui.visuals().widgets.noninteractive.fg_stroke.color,
                                        "Loop: Off",
                                    ),
                                    LoopMode::All => {
                                        (regular::REPEAT, accent_color, "Loop: All")
                                    }
                                    LoopMode::Single => {
                                        (regular::REPEAT_ONCE, accent_color, "Loop: One")
                                    }
                                };

                            let loop_btn = ui.add(
                                egui::Button::new(
                                    RichText::new(loop_icon.to_string())
                                        .size(18.0)
                                        .color(loop_color),
                                )
                                .frame(false),
                            );
                            if loop_btn.on_hover_text(loop_text).clicked() {
                                self.audio_state.lock().unwrap().next_loop_mode();
                            }

                            // Stop Button
                            let stop_btn = ui.add(
                                egui::Button::new(
                                    RichText::new(regular::STOP_CIRCLE.to_string())
                                        .size(22.0)
                                        .color(Color32::from_rgb(255, 100, 100)),
                                )
                                .frame(false),
                            );
                            if stop_btn.on_hover_text("Stop Playback").clicked()
                                && has_audio
                            {
                                self.audio_state.lock().unwrap().stop();
                            }
                        });
                    });
                });
            });
    }

    fn render_volume_controls(&mut self, ui: &mut Ui, state_copy: &AudioState) {
        let mut volume = state_copy.volume * 100.0;

        ui.horizontal(|ui| {
            let (volume_icon, volume_color) = if state_copy.is_muted || state_copy.volume <= 0.0 {
                (regular::SPEAKER_X, Color32::from_gray(150))
            } else if state_copy.volume < 0.33 {
                (regular::SPEAKER_LOW, Color32::from_rgb(100, 150, 255))
            } else if state_copy.volume < 0.66 {
                (regular::SPEAKER_HIGH, Color32::from_rgb(100, 150, 255))
            } else {
                (regular::SPEAKER_HIGH, Color32::from_rgb(100, 150, 255))
            };

            let volume_btn = ui.add(
                egui::Button::new(
                    RichText::new(volume_icon.to_string())
                        .size(18.0)
                        .color(volume_color),
                )
                .frame(false),
            );

            if volume_btn.clicked() {
                self.audio_state.lock().unwrap().toggle_mute();
            }

            let slider_response = ui.add(
                Slider::new(&mut volume, 0.0..=100.0)
                    .show_value(false)
                    .text(""),
            );

            if slider_response.changed() {
                self.audio_state.lock().unwrap().set_volume(volume / 100.0);
            }
        });
    }
}
