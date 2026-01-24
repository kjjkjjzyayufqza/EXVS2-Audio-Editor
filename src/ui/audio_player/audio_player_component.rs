use egui::{Context, Frame, Ui};
use nus3audio::Nus3audioFile;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::audio_controls::AudioControls;
use super::audio_state::{AudioFile, AudioState};
use crate::ui::main_area::{AudioFileInfo, Nus3audioFileUtils, ReplaceUtils};

/// Action returned by the audio player to the parent component
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioPlayerAction {
    /// No action needed
    None,
    /// Play the next track
    PlayNext,
    /// Play the previous track
    PlayPrevious,
}

/// Main audio player component
pub struct AudioPlayer {
    /// Audio player state
    audio_state: Arc<Mutex<AudioState>>,
    /// Audio controls component
    audio_controls: AudioControls,
    /// Last update time for playback simulation
    last_update: Instant,
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
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
    /// Returns an action if a track transition is requested
    pub fn show(&mut self, ctx: &Context) -> AudioPlayerAction {
        // Update playback position
        self.update_playback_position();

        // Handle track transitions (auto-play next, etc.)
        let action = self.check_for_transitions();

        let available_rect = ctx.available_rect();
        let panel_default_height = available_rect.height() * 0.20;
        let panel_min_height = available_rect.height() * 0.12;

        // Display audio player in a bottom panel with resizable height
        egui::TopBottomPanel::bottom("audio_player_panel")
            .resizable(true)
            .min_height(panel_min_height)
            .default_height(panel_default_height)
            .frame(egui::Frame::new().fill(ctx.style().visuals.panel_fill))
            .show(ctx, |ui| {
                self.render(ui);
            });
            
        action
    }

    /// Check if a track transition is needed
    fn check_for_transitions(&mut self) -> AudioPlayerAction {
        let mut state = self.audio_state.lock().unwrap();
        
        if state.should_play_next {
            state.should_play_next = false;
            AudioPlayerAction::PlayNext
        } else if state.should_play_previous {
            state.should_play_previous = false;
            AudioPlayerAction::PlayPrevious
        } else {
            AudioPlayerAction::None
        }
    }

    /// Render the audio player UI
    pub fn render(&mut self, ui: &mut Ui) {
        // Use a frame with margin for spacing
        Frame::new()
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                // Render audio controls
                self.audio_controls.render(ui);
            });
    }

    /// Load audio from file info
    pub fn load_audio(&mut self, file_info: &AudioFileInfo, file_path: &str) -> Result<(), String> {
        // Check if there's a replacement audio data in memory first (unified method for both file types)
        let replacement_audio_data = ReplaceUtils::get_replacement_data_unified(file_info);

        // Check if there is pending added audio data not saved yet
        let pending_added_data =
            Nus3audioFileUtils::get_pending_added_data(&file_info.name, &file_info.id);

        // Determine which audio data to use (replacement or original)
        let audio_data = if let Some(replacement_data) = replacement_audio_data {
            // We have replacement data, use it directly
            log::info!("Using replacement audio data for: {}", file_info.name);

            // The replacement data has already been processed to add loop points during replacement
            replacement_data
        } else if let Some(added_data) = pending_added_data {
            // Use pending added audio data
            log::info!("Using pending added audio data for: {}", file_info.name);
            added_data
        } else {
            // No replacement data, use the original file
            log::info!(
                "No replacement/added data found, using original file for: {}",
                file_info.name
            );

            // Check if this is a NUS3BANK or NUS3AUDIO file
            if file_info.is_nus3bank {
                // For NUS3BANK files, use vgmstream to convert to WAV
                log::info!(
                    "Processing NUS3BANK file for: {} (hex_id: {})",
                    file_info.name,
                    file_info.hex_id.as_ref().unwrap_or(&file_info.id)
                );
                match crate::ui::main_area::ExportUtils::convert_to_wav_in_memory(
                    file_info, file_path,
                ) {
                    Ok(wav_data) => {
                        log::info!(
                            "Successfully converted NUS3BANK audio to WAV format: {} ({} bytes)",
                            file_info.name,
                            wav_data.len()
                        );
                        wav_data
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to convert NUS3BANK audio to WAV format for track '{}' ({}): {}",
                            file_info.name,
                            file_info.hex_id.as_ref().unwrap_or(&file_info.id),
                            e
                        );
                        return Err(format!(
                            "Failed to convert NUS3BANK audio to WAV format: {}",
                            e
                        ));
                    }
                }
            } else {
                // For NUS3AUDIO files, use the nus3audio library
                log::info!("Processing NUS3AUDIO file for: {}", file_info.name);
                let nus3_file = match Nus3audioFile::open(file_path) {
                    Ok(file) => file,
                    Err(e) => return Err(format!("Failed to open NUS3AUDIO file: {}", e)),
                };

                // Find the audio file with matching name
                let audio_file = nus3_file
                    .files
                    .iter()
                    .find(|f| f.name == file_info.name)
                    .ok_or_else(|| {
                        format!(
                            "Audio file '{}' not found in NUS3AUDIO file",
                            file_info.name
                        )
                    })?;

                // Try to convert the audio data to WAV format using vgmstream
                match crate::ui::main_area::ExportUtils::convert_to_wav_in_memory(
                    file_info, file_path,
                ) {
                    Ok(wav_data) => {
                        log::info!(
                            "Successfully converted NUS3AUDIO audio to WAV format: {} ({} bytes)",
                            file_info.name,
                            wav_data.len()
                        );
                        wav_data
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to convert NUS3AUDIO audio to WAV format: {}. Using original format instead.",
                            e
                        );
                        audio_file.data.clone()
                    }
                }
            }
        };

        // Create an audio file struct (wrap data in Arc to avoid expensive clones)
        let data_len = audio_data.len();
        let audio = AudioFile {
            file_path: file_path.to_string(),
            data: Arc::new(audio_data),
            name: file_info.name.clone(),
            file_type: file_info.file_type.clone(),
            id: file_info.id.clone(),
            #[cfg(target_arch = "wasm32")]
            temp_url: None,
        };

        log::info!(
            "Loading audio: {} ({} bytes)",
            file_info.name,
            data_len
        );

        // Set the audio in the state (this will call toggle_play which gets the real duration from backend)
        let mut state = self.audio_state.lock().unwrap();
        state.set_audio(audio);

        // Reset loop settings to defaults
        state.set_loop_points(None, None, false);

        // Apply audio-specific loop settings if present
        let key = format!("{}:{}", file_info.name, file_info.id);
        if let Ok(settings_map) = crate::ui::main_area::ReplaceUtils::get_loop_settings() {
            if let Some(&(start, end, use_custom)) = settings_map.get(&key) {
                // Apply loop settings for this audio
                log::info!(
                    "Applied custom loop settings for {}: start={:?}, end={:?}, use_custom={}",
                    file_info.name,
                    start,
                    end,
                    use_custom
                );
                state.set_loop_points(start, end, use_custom);
            } else {
                log::info!("No custom loop settings found for: {}", file_info.name);
            }
        }

        // Check if backend could determine the real duration
        if state.total_duration <= 0.0 {
            log::error!(
                "Failed to get audio duration for '{}': backend returned 0 or negative duration",
                file_info.name
            );
            return Err(format!(
                "Failed to get audio duration for '{}': unable to decode audio metadata",
                file_info.name
            ));
        }

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
