use egui::{Context, Frame, Rounding, Stroke, Ui};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use nus3audio::Nus3audioFile;

use super::audio_controls::AudioControls;
use super::audio_state::{AudioFile, AudioState};
use crate::ui::main_area::AudioFileInfo;

/// Main audio player component
pub struct AudioPlayer {
    /// Audio player state
    audio_state: Arc<Mutex<AudioState>>,
    /// Audio controls component
    audio_controls: AudioControls,
    /// Last update time for playback simulation
    last_update: Instant,
}

impl AudioPlayer {
    /// Create a new audio player
    pub fn new() -> Self {
        let audio_state = Arc::new(Mutex::new(AudioState::new()));
        let audio_controls = AudioControls::new(Arc::clone(&audio_state));
        
        Self {
            audio_state,
            audio_controls,
            last_update: Instant::now(),
        }
    }
    
    /// Show the audio player at the bottom of the screen
    pub fn show(&mut self, ctx: &Context) {
        // Update playback position
        self.update_playback_position();
        
        // Display audio player in a bottom panel
        egui::TopBottomPanel::bottom("audio_player_panel")
            .min_height(100.0)
            .resizable(false)
            .show(ctx, |ui| {
                self.render(ui);
            });
    }
    
    /// Render the audio player UI
    pub fn render(&mut self, ui: &mut Ui) {
        // Use a frame to make it look nicer
        Frame::group(ui.style())
            .rounding(Rounding::same(4.0))
            .stroke(Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color))
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.add_space(8.0);
                        
                        ui.heading("Audio Player");
                        ui.add_space(5.0);
                        
                        // Render audio controls
                        self.audio_controls.render(ui);
                        
                        ui.add_space(8.0);
                    });
                    ui.add_space(8.0);
                });
            });
    }
    
    /// Load audio from file info
    pub fn load_audio(&mut self, file_info: &AudioFileInfo, file_path: &str) -> Result<(), String> {
        // Try to open the NUS3AUDIO file
        let nus3_file = match Nus3audioFile::open(file_path) {
            Ok(file) => file,
            Err(e) => return Err(format!("Failed to open NUS3AUDIO file: {}", e)),
        };
        
        // Find the audio file with matching name
        let audio_file = nus3_file.files.iter()
            .find(|f| f.name == file_info.name)
            .ok_or_else(|| format!("Audio file '{}' not found in NUS3AUDIO file", file_info.name))?;
        
        // Create an audio file struct
        let audio = AudioFile {
            file_path: file_path.to_string(),
            data: audio_file.data.clone(),
            name: file_info.name.clone(),
            file_type: file_info.file_type.clone(),
            id: file_info.id.clone(),
            #[cfg(target_arch = "wasm32")]
            temp_url: None,
        };
        
        // Set the audio in the state
        let mut state = self.audio_state.lock().unwrap();
        state.set_audio(audio);
        
        // Set default duration - in real implementation, this would be determined from audio metadata
        // For now we'll just estimate based on file size (very rough approximation)
        let estimated_duration = estimate_duration_from_size(file_info.size);
        state.total_duration = estimated_duration;
        
        Ok(())
    }
    
    /// Update the playback position based on elapsed time
    fn update_playback_position(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f32();
        self.last_update = now;
        
        // Only update if playing
        let mut state = self.audio_state.lock().unwrap();
        if state.is_playing {
            let new_position = state.current_position + elapsed;
            
            // Check if we've reached the end
            if new_position >= state.total_duration {
                state.current_position = state.total_duration;
                state.is_playing = false;
            } else {
                state.current_position = new_position;
            }
        }
    }
    
    /// Get a reference to the audio state
    pub fn get_audio_state(&self) -> Arc<Mutex<AudioState>> {
        Arc::clone(&self.audio_state)
    }
}

/// Estimate audio duration from file size (rough approximation)
/// Most audio files in game archives are compressed, so this is just a rough guess
fn estimate_duration_from_size(size_bytes: usize) -> f32 {
    // Very rough estimate: Assuming ~16KB per second for compressed audio
    // This would vary greatly by format and compression
    let bytes_per_second = 16000.0;
    let estimated_seconds = size_bytes as f32 / bytes_per_second;
    
    // Clamp to reasonable values (at least 1 second, at most 10 minutes)
    estimated_seconds.max(1.0).min(600.0)
}
