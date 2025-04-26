use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use super::audio_backend::{AudioBackend, PlatformAudioBackend};

/// Audio player state
#[derive(Deserialize, Serialize)]
pub struct AudioState {
    /// Current audio file being played (if any)
    #[serde(skip)]
    pub current_audio: Option<AudioFile>,
    
    /// Is the audio currently playing
    #[serde(skip)]
    pub is_playing: bool,
    
    /// Current playback position in seconds
    #[serde(skip)]
    pub current_position: f32,
    
    /// Total duration in seconds
    #[serde(skip)]
    pub total_duration: f32,
    
    /// Current volume (0.0 - 1.0)
    pub volume: f32,
    
    /// Is the audio muted
    pub is_muted: bool,
    
    /// Previous volume before mute
    #[serde(skip)]
    pub previous_volume: f32,
    
    /// Audio backend for playback
    #[serde(skip)]
    audio_backend: Option<Box<dyn AudioBackend>>,
}

// Manual Debug implementation since dyn AudioBackend doesn't implement Debug
impl std::fmt::Debug for AudioState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioState")
            .field("current_audio", &self.current_audio)
            .field("is_playing", &self.is_playing)
            .field("current_position", &self.current_position)
            .field("total_duration", &self.total_duration)
            .field("volume", &self.volume)
            .field("is_muted", &self.is_muted)
            .field("previous_volume", &self.previous_volume)
            .field("audio_backend", &format!("<audio backend>"))
            .finish()
    }
}

// Implement Clone manually since we can't derive it with dyn AudioBackend
impl Clone for AudioState {
    fn clone(&self) -> Self {
        // Create a new instance without the audio_backend
        Self {
            current_audio: self.current_audio.clone(),
            is_playing: self.is_playing,
            current_position: self.current_position,
            total_duration: self.total_duration,
            volume: self.volume,
            is_muted: self.is_muted,
            previous_volume: self.previous_volume,
            audio_backend: None, // Don't clone the audio backend
        }
    }
}

/// Audio file information
#[derive(Clone, Debug)]
pub struct AudioFile {
    /// Original file path
    pub file_path: String,
    
    /// Audio file raw data
    pub data: Vec<u8>,
    
    /// Audio file name
    pub name: String,
    
    /// Audio file type
    pub file_type: String,
    
    /// Audio file ID
    pub id: String,
    
    /// Temporary file path for web playback
    /// 
    /// Web Audio API requires a URL to play audio, so we need to create a temporary
    /// file that can be accessed via URL for playback
    #[cfg(target_arch = "wasm32")]
    pub temp_url: Option<String>,
}

impl Default for AudioState {
    fn default() -> Self {
        let mut state = Self {
            current_audio: None,
            is_playing: false,
            current_position: 0.0,
            total_duration: 0.0,
            volume: 0.75, // Default volume at 75%
            is_muted: false,
            previous_volume: 0.75,
            audio_backend: None,
        };
        
        // Try to initialize the audio backend
        match state.init_audio_backend() {
            Ok(_) => log::info!("Audio backend initialized successfully"),
            Err(e) => log::error!("Failed to initialize audio backend: {}", e),
        }
        
        state
    }
}

impl AudioState {
    /// Create a new audio state
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Initialize the audio backend
    fn init_audio_backend(&mut self) -> Result<(), String> {
        // Create a new platform-specific audio backend
        let mut backend = Box::new(PlatformAudioBackend::new());
        
        // Initialize the backend
        backend.init()?;
        
        // Store the backend
        self.audio_backend = Some(backend);
        
        Ok(())
    }
    
    /// Play or pause the audio
    pub fn toggle_play(&mut self) {
        // Toggle playing state
        self.is_playing = !self.is_playing;
        
        if let Some(backend) = &mut self.audio_backend {
            if self.is_playing {
                // If starting playback and we have audio data
                if let Some(audio) = &self.current_audio {
                    let data_arc = Arc::new(audio.data.clone());
                    
                    // If we're resuming from a position other than the beginning,
                    // we need to set the position after starting playback
                    let position = self.current_position;
                    
                    if let Err(e) = backend.play_audio(data_arc) {
                        log::error!("Failed to play audio: {}", e);
                        self.is_playing = false;
                        return;
                    }
                    
                    // Get actual duration from backend
                    self.total_duration = backend.get_duration();
                    
                    // If we're resuming from a non-zero position, seek to that position
                    if position > 0.0 {
                        if let Err(e) = backend.set_position(position) {
                            log::error!("Failed to seek to position {}: {}", position, e);
                            // Continue playback even if seeking fails
                        }
                    }
                }
            } else if let Err(e) = backend.pause() {
                log::error!("Failed to pause audio: {}", e);
            }
        }
    }
    
    /// Stop the audio playback
    pub fn stop(&mut self) {
        self.is_playing = false;
        self.current_position = 0.0;
        
        if let Some(backend) = &mut self.audio_backend {
            if let Err(e) = backend.stop() {
                log::error!("Failed to stop audio: {}", e);
            }
        }
    }
    
    /// Toggle mute state
    pub fn toggle_mute(&mut self) {
        if self.is_muted {
            // Restore previous volume
            self.volume = self.previous_volume;
            self.is_muted = false;
        } else {
            // Store current volume and mute
            self.previous_volume = self.volume;
            self.volume = 0.0;
            self.is_muted = true;
        }
        
        // Update backend volume
        if let Some(backend) = &mut self.audio_backend {
            if let Err(e) = backend.set_volume(self.volume) {
                log::error!("Failed to set audio volume: {}", e);
            }
        }
    }
    
    /// Set a new audio file for playback
    pub fn set_audio(&mut self, audio: AudioFile) {
        // Stop any current playback
        self.stop();
        
        // Set new audio file
        self.current_audio = Some(audio);
        
        // Play the new audio right away if needed
        if self.is_playing {
            self.toggle_play();
        }
    }
    
    /// Clear the current audio
    pub fn clear_audio(&mut self) {
        self.stop();
        self.current_audio = None;
    }
    
    /// Set the current position in seconds
    pub fn set_position(&mut self, position: f32) {
        self.current_position = position.clamp(0.0, self.total_duration);
        
        // Only apply position change to the backend if we're playing
        // This avoids unnecessary reloading when paused
        if self.is_playing {
            if let Some(backend) = &mut self.audio_backend {
                if let Err(e) = backend.set_position(self.current_position) {
                    log::error!("Failed to set audio position: {}", e);
                }
            }
        }
        
        // If not playing, the position will be applied when play is resumed
        // via toggle_play which will use the current_position value
    }
    
    /// Set the volume (0.0 - 1.0)
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if self.volume > 0.0 {
            self.is_muted = false;
        }
        
        // Update backend volume
        if let Some(backend) = &mut self.audio_backend {
            if let Err(e) = backend.set_volume(self.volume) {
                log::error!("Failed to set audio volume: {}", e);
            }
        }
    }
    
    /// Update playback state from backend
    pub fn update_from_backend(&mut self) {
        if let Some(backend) = &mut self.audio_backend {
            // Update position
            if self.is_playing {
                self.current_position = backend.get_position();
            }
            
            // Check if we're actually playing
            self.is_playing = backend.is_playing();
            
            // Check if we've reached the end
            if self.current_position >= self.total_duration {
                self.is_playing = false;
                self.current_position = self.total_duration;
            }
        }
    }
    
    /// Get formatted current position (MM:SS)
    pub fn format_position(&self) -> String {
        let minutes = (self.current_position / 60.0).floor() as u32;
        let seconds = (self.current_position % 60.0).floor() as u32;
        format!("{:02}:{:02}", minutes, seconds)
    }
    
    /// Get formatted total duration (MM:SS)
    pub fn format_duration(&self) -> String {
        let minutes = (self.total_duration / 60.0).floor() as u32;
        let seconds = (self.total_duration % 60.0).floor() as u32;
        format!("{:02}:{:02}", minutes, seconds)
    }
    
    /// Get playback progress as a ratio (0.0 - 1.0)
    pub fn progress(&self) -> f32 {
        if self.total_duration > 0.0 {
            self.current_position / self.total_duration
        } else {
            0.0
        }
    }
}
