use egui::{Context, Frame, Rounding, Stroke, Ui};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use nus3audio::Nus3audioFile;

use super::audio_controls::AudioControls;
use super::audio_state::{AudioFile, AudioState};
use crate::ui::main_area::{AudioFileInfo, ReplaceUtils};

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
            .min_height(120.0)  // Increased height for better UX
            .frame(egui::Frame::none().fill(ctx.style().visuals.panel_fill))
            .resizable(false)
            .show(ctx, |ui| {
                self.render(ui);
            });
    }
    
    /// Render the audio player UI
    pub fn render(&mut self, ui: &mut Ui) {
        // Use a frame to make it look nicer with gradient background
        Frame::group(ui.style())
            .rounding(Rounding::same(8))
            .stroke(Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color))
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.vertical(|ui| {
                        ui.add_space(4.0);
                        
                        // Improved heading with better styling
                        ui.horizontal(|ui| {
                            ui.add_space(4.0);
                            ui.heading(egui::RichText::new("Audio Player")
                                .size(20.0)
                                .color(ui.visuals().strong_text_color()));
                        });
                        
                        ui.add_space(8.0);
                        
                        // Render audio controls
                        self.audio_controls.render(ui);
                    });
                });
            });
    }
    
    /// Load audio from file info
    pub fn load_audio(&mut self, file_info: &AudioFileInfo, file_path: &str) -> Result<(), String> {
        // Check if there's a replacement audio data in memory first
        let replacement_audio_data = ReplaceUtils::get_replacement_data(&file_info.name, &file_info.id);
        
        // Determine which audio data to use (replacement or original)
        let audio_data = if let Some(replacement_data) = replacement_audio_data {
            // We have replacement data, use it directly
            log::info!("Using replacement audio data for: {}", file_info.name);
            
            // The replacement data has already been processed to add loop points during replacement
            replacement_data
        } else {
            // No replacement data, use the original file
            log::info!("No replacement data found, using original file for: {}", file_info.name);
            
            // Try to open the NUS3AUDIO file
            let nus3_file = match Nus3audioFile::open(file_path) {
                Ok(file) => file,
                Err(e) => return Err(format!("Failed to open NUS3AUDIO file: {}", e)),
            };
            
            // Find the audio file with matching name
            let audio_file = nus3_file.files.iter()
                .find(|f| f.name == file_info.name)
                .ok_or_else(|| format!("Audio file '{}' not found in NUS3AUDIO file", file_info.name))?;

            // Try to convert the audio data to WAV format using vgmstream
            match crate::ui::main_area::ExportUtils::convert_to_wav_in_memory(file_info, file_path) {
                Ok(wav_data) => {
                    log::info!("Successfully converted audio to WAV format: {} ({} bytes)", file_info.name, wav_data.len());
                    wav_data
                },
                Err(e) => {
                    log::warn!("Failed to convert audio to WAV format: {}. Using original format instead.", e);
                    audio_file.data.clone()
                }
            }
        };
        
        // Create an audio file struct
        let audio = AudioFile {
            file_path: file_path.to_string(),
            data: audio_data,
            name: file_info.name.clone(),
            file_type: file_info.file_type.clone(),
            id: file_info.id.clone(),
            #[cfg(target_arch = "wasm32")]
            temp_url: None,
        };
        
        log::info!("Loading audio: {} ({} bytes)", file_info.name, audio.data.len());
        
        // Set the audio in the state
        let mut state = self.audio_state.lock().unwrap();
        state.set_audio(audio);
        
        // Duration will be determined by the audio backend when playback starts
        // We still set an estimated duration for the UI until playback begins
        let estimated_duration = estimate_duration_from_size(file_info.size);
        state.total_duration = estimated_duration;
        
        Ok(())
    }
    
    /// Update the playback position and state from the audio backend
    fn update_playback_position(&mut self) {
        let now = Instant::now();
        self.last_update = now;
        
        // Update state from the audio backend
        let mut state = self.audio_state.lock().unwrap();
        state.update_from_backend();
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
