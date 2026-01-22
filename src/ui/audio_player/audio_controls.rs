use super::audio_state::AudioState;
use egui::{widgets::Slider, Color32, Frame, RichText, CornerRadius, Stroke, Ui};
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
        let available_height = ui.available_height();
        let is_narrow = available_width <= available_height * 1.2;
        let horizontal_gap = available_width * 0.02;
        let vertical_gap = available_height * 0.04;

        // Frame around the controls
        Frame::group(ui.style())
            .corner_radius(CornerRadius::same(6))
            .stroke(Stroke::new(
                1.0,
                ui.visuals().widgets.noninteractive.bg_stroke.color,
            ))
            .show(ui, |ui| {
                let render_volume_controls = |ui: &mut Ui, slider_width: f32, slider_height: f32| {
                    // Volume slider
                    let mut volume = state_copy.volume * 100.0; // Convert 0-1 to 0-100 for display
                    if ui
                        .add_sized(
                            [slider_width, slider_height],
                            Slider::new(&mut volume, 0.0..=100.0)
                                .text("")
                                .show_value(false),
                        )
                        .changed()
                    {
                        let mut state = self.audio_state.lock().unwrap();
                        state.set_volume(volume / 100.0); // Convert back to 0-1 for storage
                    }

                    // Volume button with phosphor icon
                    let (volume_icon, _icon_name) =
                        if state_copy.is_muted || state_copy.volume <= 0.0 {
                            (regular::SPEAKER_NONE, "SPEAKER_NONE")
                        } else if state_copy.volume < 0.33 {
                            (regular::SPEAKER_LOW, "SPEAKER_LOW")
                        } else if state_copy.volume < 0.66 {
                            (regular::SPEAKER_HIGH, "SPEAKER_HIGH")
                        } else {
                            (regular::SPEAKER_HIGH, "SPEAKER_HIGH")
                        };

                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new(volume_icon.to_string()).size(16.0),
                        ))
                        .clicked()
                    {
                        let mut state = self.audio_state.lock().unwrap();
                        state.toggle_mute();
                    }
                };

                let slider_height = available_height * if is_narrow { 0.12 } else { 0.1 };
                let volume_slider_width = available_width * if is_narrow { 0.5 } else { 0.25 };

                ui.add_space(vertical_gap);

                if is_narrow {
                    ui.vertical(|ui| {
                        // Audio file name display (if any)
                        if let Some(audio) = &state_copy.current_audio {
                            ui.label(
                                RichText::new(&audio.name)
                                    .color(ui.visuals().strong_text_color())
                                    .size(16.0),
                            );

                            ui.add_space(vertical_gap);

                            // Audio type label with color
                            let type_color = match audio.file_type.as_str() {
                                "OPUS Audio" => Color32::from_rgb(100, 200, 100),
                                "IDSP Audio" => Color32::from_rgb(100, 150, 255),
                                _ => Color32::from_rgb(200, 150, 100),
                            };

                            ui.label(RichText::new(&audio.file_type).color(type_color).size(14.0));
                        } else {
                            ui.label(
                                RichText::new("No audio file loaded")
                                    .color(ui.visuals().weak_text_color()),
                            );
                        }
                    });

                    ui.add_space(vertical_gap);

                    // Progress row (time + slider + duration)
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(state_copy.format_position())
                                .monospace()
                                .size(14.0),
                        );
                        ui.add_space(horizontal_gap);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(state_copy.format_duration())
                                    .monospace()
                                    .size(14.0),
                            );
                        });
                    });

                    ui.add_space(vertical_gap);

                    let mut progress = state_copy.progress();
                    let slider_width = ui.available_width();
                    let slider_response = ui.add_sized(
                        [slider_width, slider_height],
                        Slider::new(&mut progress, 0.0..=1.0)
                            .show_value(false)
                            .text(""),
                    );

                    if slider_response.drag_stopped() && has_audio {
                        let mut state: std::sync::MutexGuard<'_, AudioState> =
                            self.audio_state.lock().unwrap();
                        let new_position = progress * state.total_duration;
                        state.set_position(new_position);
                    }

                    ui.add_space(vertical_gap);

                    ui.horizontal_wrapped(|ui| {
                        // Play/pause button with phosphor icons
                        let (play_icon, _icon_name, play_color) = if state_copy.is_playing {
                            (
                                regular::PAUSE_CIRCLE,
                                "PAUSE_CIRCLE",
                                Color32::from_rgb(255, 200, 100),
                            )
                        } else {
                            (
                                regular::PLAY_CIRCLE,
                                "PLAY_CIRCLE",
                                Color32::from_rgb(100, 255, 150),
                            )
                        };

                        let play_button_color = if has_audio {
                            play_color
                        } else {
                            Color32::from_gray(150)
                        };

                        let rich_text = egui::RichText::new(play_icon.to_string())
                            .size(24.0)
                            .color(play_button_color);

                        if ui.add(egui::Button::new(rich_text)).clicked() && has_audio {
                            let mut state = self.audio_state.lock().unwrap();
                            state.toggle_play();
                        }

                        ui.add_space(horizontal_gap);

                        let stop_button_color = if has_audio
                            && (state_copy.is_playing || state_copy.current_position > 0.0)
                        {
                            Color32::from_rgb(255, 100, 100)
                        } else {
                            Color32::from_gray(150)
                        };

                        let rich_text =
                            egui::RichText::new(regular::STOP_CIRCLE.to_string())
                                .size(24.0)
                                .color(stop_button_color);

                        if ui.add(egui::Button::new(rich_text)).clicked()
                            && has_audio
                            && (state_copy.is_playing || state_copy.current_position > 0.0)
                        {
                            let mut state = self.audio_state.lock().unwrap();
                            state.stop();
                        }

                        ui.add_space(horizontal_gap);

                        render_volume_controls(ui, volume_slider_width, slider_height);
                    });
                } else {
                    ui.horizontal(|ui| {
                        // Audio file name display (if any)
                        if let Some(audio) = &state_copy.current_audio {
                            ui.label(
                                RichText::new(&audio.name)
                                    .color(ui.visuals().strong_text_color())
                                    .size(16.0),
                            );

                            ui.add_space(horizontal_gap);

                            let type_color = match audio.file_type.as_str() {
                                "OPUS Audio" => Color32::from_rgb(100, 200, 100),
                                "IDSP Audio" => Color32::from_rgb(100, 150, 255),
                                _ => Color32::from_rgb(200, 150, 100),
                            };

                            ui.label(RichText::new(&audio.file_type).color(type_color).size(14.0));
                        } else {
                            ui.label(
                                RichText::new("No audio file loaded")
                                    .color(ui.visuals().weak_text_color()),
                            );
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            render_volume_controls(ui, volume_slider_width, slider_height);
                        });
                    });

                    ui.add_space(vertical_gap);

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(state_copy.format_position())
                                .monospace()
                                .size(14.0),
                        );

                        ui.add_space(horizontal_gap);

                        let mut progress = state_copy.progress();
                        let slider_width = ui.available_width() * 0.6;
                        let slider_response = ui.add_sized(
                            [slider_width, slider_height],
                            Slider::new(&mut progress, 0.0..=1.0)
                                .show_value(false)
                                .text(""),
                        );

                        if slider_response.drag_stopped() && has_audio {
                            let mut state: std::sync::MutexGuard<'_, AudioState> =
                                self.audio_state.lock().unwrap();
                            let new_position = progress * state.total_duration;
                            state.set_position(new_position);
                        }

                        ui.add_space(horizontal_gap);

                        ui.label(
                            RichText::new(state_copy.format_duration())
                                .monospace()
                                .size(14.0),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let (play_icon, _icon_name, play_color) = if state_copy.is_playing {
                                (
                                    regular::PAUSE_CIRCLE,
                                    "PAUSE_CIRCLE",
                                    Color32::from_rgb(255, 200, 100),
                                )
                            } else {
                                (
                                    regular::PLAY_CIRCLE,
                                    "PLAY_CIRCLE",
                                    Color32::from_rgb(100, 255, 150),
                                )
                            };

                            let play_button_color = if has_audio {
                                play_color
                            } else {
                                Color32::from_gray(150)
                            };

                            let rich_text = egui::RichText::new(play_icon.to_string())
                                .size(24.0)
                                .color(play_button_color);

                            if ui.add(egui::Button::new(rich_text)).clicked() && has_audio {
                                let mut state = self.audio_state.lock().unwrap();
                                state.toggle_play();
                            }

                            ui.add_space(horizontal_gap);

                            let stop_button_color = if has_audio
                                && (state_copy.is_playing || state_copy.current_position > 0.0)
                            {
                                Color32::from_rgb(255, 100, 100)
                            } else {
                                Color32::from_gray(150)
                            };

                            let rich_text =
                                egui::RichText::new(regular::STOP_CIRCLE.to_string())
                                    .size(24.0)
                                    .color(stop_button_color);

                            if ui.add(egui::Button::new(rich_text)).clicked()
                                && has_audio
                                && (state_copy.is_playing || state_copy.current_position > 0.0)
                            {
                                let mut state = self.audio_state.lock().unwrap();
                                state.stop();
                            }
                        });
                    });
                }

                ui.add_space(vertical_gap);
            });
    }
}
