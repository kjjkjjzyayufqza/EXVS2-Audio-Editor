use egui::{
    widgets::Slider, Color32, Frame, Label, RichText, Rounding, Stroke, Ui, Vec2
};
use super::audio_state::AudioState;
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
            state.clone()
        };

        // Check if there's an audio file loaded
        let has_audio = state_copy.current_audio.is_some();
        
        // Frame around the controls
        Frame::group(ui.style())
            .rounding(Rounding::same(6.0))
            .stroke(Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color))
            .show(ui, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    
                    // Audio file name display (if any)
                    if let Some(audio) = &state_copy.current_audio {
                        ui.label(RichText::new(&audio.name)
                            .color(ui.visuals().strong_text_color())
                            .size(16.0));
                        
                        ui.add_space(10.0);
                        
                        // Audio type label with color
                        let type_color = match audio.file_type.as_str() {
                            "OPUS Audio" => Color32::from_rgb(100, 200, 100), // Green
                            "IDSP Audio" => Color32::from_rgb(100, 150, 255), // Blue
                            _ => Color32::from_rgb(200, 150, 100), // Yellow/Brown
                        };
                        
                        ui.label(RichText::new(&audio.file_type)
                            .color(type_color)
                            .size(14.0));
                    } else {
                        ui.label(RichText::new("No audio file loaded")
                            .color(ui.visuals().weak_text_color()));
                    }
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Volume control with icon
                        let volume_icon = if state_copy.is_muted || state_copy.volume <= 0.0 {
                            "🔇" // Muted
                        } else if state_copy.volume < 0.33 {
                            "🔈" // Low volume
                        } else if state_copy.volume < 0.66 {
                            "🔉" // Medium volume
                        } else {
                            "🔊" // High volume
                        };
                        
                        // Volume slider
                        let mut volume = state_copy.volume;
                        if ui.add(Slider::new(&mut volume, 0.0..=1.0)
                            .text(volume_icon)
                            .show_value(false)
                            .fixed_decimals(1))
                            .changed() 
                        {
                            let mut state = self.audio_state.lock().unwrap();
                            state.set_volume(volume);
                        }
                        
                        // Mute button
                        if ui.button(RichText::new(volume_icon)).clicked() {
                            let mut state = self.audio_state.lock().unwrap();
                            state.toggle_mute();
                        }
                    });
                });
                
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    
                    // Current position
                    ui.label(RichText::new(state_copy.format_position())
                        .monospace()
                        .size(14.0));

                    ui.add_space(5.0);

                    // Progress slider
                    let mut progress = state_copy.progress();
                    
                    let resp = ui.add(
                        Slider::new(&mut progress, 0.0..=1.0)
                            .show_value(false)
                            .text(if has_audio { "▓" } else { "░" })
                    );

                    if resp.changed() && has_audio {
                        let mut state = self.audio_state.lock().unwrap();
                        let new_position = progress * state.total_duration;
                        state.set_position(new_position);
                    }

                    ui.add_space(5.0);

                    // Total duration
                    ui.label(RichText::new(state_copy.format_duration())
                        .monospace()
                        .size(14.0));
                        
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Play/pause button
                        let (play_text, play_color) = if state_copy.is_playing {
                            ("⏸", Color32::from_rgb(255, 200, 100))  // Pause
                        } else {
                            ("▶", Color32::from_rgb(100, 255, 150))  // Play
                        };
                        
                        // Play/pause button with different colors based on state
                        let play_button_color = if has_audio {
                            play_color
                        } else {
                            Color32::from_gray(150) // Grayed out
                        };
                        
                        if ui.button(RichText::new(play_text).size(18.0).color(play_button_color)).clicked() && has_audio {
                            let mut state = self.audio_state.lock().unwrap();
                            state.toggle_play();
                        }
                        
                        // Stop button with different colors based on state
                        let stop_button_color = if has_audio && (state_copy.is_playing || state_copy.current_position > 0.0) {
                            Color32::from_rgb(255, 100, 100)
                        } else {
                            Color32::from_gray(150) // Grayed out
                        };
                        
                        if ui.button(RichText::new("■").size(18.0).color(stop_button_color)).clicked() &&
                           has_audio && (state_copy.is_playing || state_copy.current_position > 0.0) {
                            let mut state = self.audio_state.lock().unwrap();
                            state.stop();
                        }
                    });
                });
                
                ui.add_space(8.0);
            });
    }
}
