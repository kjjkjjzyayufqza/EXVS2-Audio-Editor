use std::path::PathBuf;
use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Audio player state
#[derive(Clone, Debug, Deserialize, Serialize)]
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
        Self {
            current_audio: None,
            is_playing: false,
            current_position: 0.0,
            total_duration: 0.0,
            volume: 0.75, // Default volume at 75%
            is_muted: false,
            previous_volume: 0.75,
        }
    }
}

impl AudioState {
    /// Create a new audio state
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Play or pause the audio
    pub fn toggle_play(&mut self) {
        self.is_playing = !self.is_playing;
    }
    
    /// Stop the audio playback
    pub fn stop(&mut self) {
        self.is_playing = false;
        self.current_position = 0.0;
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
    }
    
    /// Set a new audio file for playback
    pub fn set_audio(&mut self, audio: AudioFile) {
        self.stop();
        self.current_audio = Some(audio);
    }
    
    /// Clear the current audio
    pub fn clear_audio(&mut self) {
        self.stop();
        self.current_audio = None;
    }
    
    /// Set the current position in seconds
    pub fn set_position(&mut self, position: f32) {
        self.current_position = position.clamp(0.0, self.total_duration);
    }
    
    /// Set the volume (0.0 - 1.0)
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if self.volume > 0.0 {
            self.is_muted = false;
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
