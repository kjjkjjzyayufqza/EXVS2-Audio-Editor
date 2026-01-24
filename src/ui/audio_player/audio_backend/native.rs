use std::fs::File;
use std::io::Read;
use std::time::Instant;

use kira::{
    AudioManager,
    AudioManagerSettings,
    DefaultBackend,
    Tween,
    sound::FromFileError,
    sound::streaming::{StreamingSoundData, StreamingSoundHandle},
};

use crate::ui::audio_player::audio_backend::trait_def::AudioBackend;

/// Native audio backend implementation using kira
pub struct NativeAudioBackend {
    /// Audio manager for playback
    manager: Option<AudioManager<DefaultBackend>>,
    /// Handle to the currently playing sound
    sound_handle: Option<StreamingSoundHandle<FromFileError>>,
    /// Current position in seconds
    current_position: f32,
    /// Start time of playback for position tracking
    playback_start_time: Option<Instant>,
    /// Position when playback started
    playback_start_position: f32,
    /// Audio duration in seconds
    duration: f32,
    /// Whether audio is currently loaded
    audio_loaded: bool,
    /// Is currently playing
    is_playing: bool,
    /// Whether backend initialization succeeded
    initialized: bool,
    /// Current volume level (0.0 - 1.0)
    volume: f32,
}

impl NativeAudioBackend {
    /// Create a new native audio backend
    pub fn new() -> Self {
        Self {
            manager: None,
            sound_handle: None,
            current_position: 0.0,
            playback_start_time: None,
            playback_start_position: 0.0,
            duration: 0.0,
            audio_loaded: false,
            is_playing: false,
            initialized: false,
            volume: 1.0, // Default volume is 100%
        }
    }

    /// Estimate the duration of audio from the WAV header
    fn estimate_wav_duration_from_file(&self, file_path: &str) -> f32 {
        let mut file = match File::open(file_path) {
            Ok(file) => file,
            Err(_) => return 0.0,
        };

        let mut header = vec![0u8; 512];
        let read_len = match file.read(&mut header) {
            Ok(len) => len,
            Err(_) => return 0.0,
        };
        header.truncate(read_len);

        if header.len() < 44 {
            return 0.0;
        }

        if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
            return 0.0;
        }

        let channels = u16::from_le_bytes([header[22], header[23]]);
        let sample_rate = u32::from_le_bytes([header[24], header[25], header[26], header[27]]);
        let bits_per_sample = u16::from_le_bytes([header[34], header[35]]);

        let mut data_size = 0u32;
        let mut i = 36;
        while i + 8 <= header.len() {
            if &header[i..i + 4] == b"data" {
                data_size = u32::from_le_bytes([
                    header[i + 4],
                    header[i + 5],
                    header[i + 6],
                    header[i + 7],
                ]);
                break;
            }
            i += 1;
        }

        if data_size == 0 || sample_rate == 0 || channels == 0 || bits_per_sample == 0 {
            return 0.0;
        }

        let bytes_per_sample = (bits_per_sample / 8) as u32;
        let bytes_per_second = sample_rate * channels as u32 * bytes_per_sample;

        if bytes_per_second > 0 {
            data_size as f32 / bytes_per_second as f32
        } else {
            0.0
        }
    }

    fn volume_to_decibels(volume: f32) -> f32 {
        let clamped = volume.clamp(0.0, 1.0);
        if clamped <= 0.0 {
            -80.0
        } else {
            20.0 * clamped.log10()
        }
    }
}

impl AudioBackend for NativeAudioBackend {
    fn init(&mut self) -> Result<(), String> {
        match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()) {
            Ok(manager) => {
                self.manager = Some(manager);
                self.initialized = true;
                Ok(())
            }
            Err(e) => {
                self.initialized = false;
                Err(format!("Failed to initialize audio manager: {}", e))
            }
        }
    }

    fn play_audio(&mut self, file_path: &str) -> Result<(), String> {
        if !self.initialized {
            return Err("Audio backend not initialized".to_string());
        }

        let manager = self.manager.as_mut().ok_or_else(|| "Audio manager not available".to_string())?;

        if let Some(mut handle) = self.sound_handle.take() {
            handle.stop(Tween::default());
        }

        let sound_data = StreamingSoundData::from_file(file_path)
            .map_err(|e| format!("Failed to load audio file: {}", e))?;

        let mut handle = manager.play(sound_data)
            .map_err(|e| format!("Failed to start audio playback: {}", e))?;

        self.duration = self.estimate_wav_duration_from_file(file_path);
        self.current_position = 0.0;
        self.playback_start_time = Some(Instant::now());
        self.playback_start_position = 0.0;
        self.audio_loaded = true;
        self.is_playing = true;

        let volume_db = Self::volume_to_decibels(self.volume);
        let _ = handle.set_volume(volume_db, Tween::default());

        self.sound_handle = Some(handle);
        Ok(())
    }

    fn pause(&mut self) -> Result<(), String> {
        if let Some(handle) = &mut self.sound_handle {
            if self.is_playing {
                if let Some(start_time) = self.playback_start_time {
                    let elapsed = start_time.elapsed().as_secs_f32();
                    self.current_position = self.playback_start_position + elapsed;
                }
            }

            handle.pause(Tween::default());
            self.is_playing = false;
            Ok(())
        } else {
            Err("No audio playing".to_string())
        }
    }

    fn resume(&mut self) -> Result<(), String> {
        if let Some(handle) = &mut self.sound_handle {
            let volume_db = Self::volume_to_decibels(self.volume);
            let _ = handle.set_volume(volume_db, Tween::default());
            handle.resume(Tween::default());

            self.playback_start_time = Some(Instant::now());
            self.playback_start_position = self.current_position;
            self.is_playing = true;
            Ok(())
        } else {
            Err("No audio loaded".to_string())
        }
    }

    fn stop(&mut self) -> Result<(), String> {
        if let Some(mut handle) = self.sound_handle.take() {
            self.current_position = 0.0;
            self.playback_start_position = 0.0;
            self.playback_start_time = None;
            self.is_playing = false;

            handle.stop(Tween::default());
            Ok(())
        } else {
            Err("No audio playing".to_string())
        }
    }

    fn set_position(&mut self, position_secs: f32) -> Result<(), String> {
        if !self.audio_loaded {
            return Err("No audio loaded".to_string());
        }

        let clamped_position = position_secs.clamp(0.0, self.duration);
        self.current_position = clamped_position;
        self.playback_start_position = clamped_position;

        if let Some(handle) = &mut self.sound_handle {
            handle.seek_to(clamped_position as f64);
            if self.is_playing {
                self.playback_start_time = Some(Instant::now());
            }
            Ok(())
        } else {
            Err("No audio handle available".to_string())
        }
    }

    fn set_volume(&mut self, volume: f32) -> Result<(), String> {
        self.volume = volume;
        if let Some(handle) = &mut self.sound_handle {
            let volume_db = Self::volume_to_decibels(self.volume);
            let _ = handle.set_volume(volume_db, Tween::default());
        }
        Ok(())
    }

    fn is_playing(&self) -> bool {
        if !self.audio_loaded {
            return false;
        }

        if self.is_playing && self.duration > 0.0 {
            return self.get_position() < self.duration;
        }

        self.is_playing
    }

    fn get_position(&self) -> f32 {
        if !self.is_playing {
            return self.current_position;
        }

        if let Some(start_time) = self.playback_start_time {
            let elapsed = start_time.elapsed().as_secs_f32();
            let position = self.playback_start_position + elapsed;
            position.min(self.duration)
        } else {
            self.current_position
        }
    }

    fn get_duration(&self) -> f32 {
        self.duration
    }

    fn is_available(&self) -> bool {
        self.initialized
    }
}

impl Default for NativeAudioBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for NativeAudioBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeAudioBackend")
            .field("duration", &self.duration)
            .field("audio_loaded", &self.audio_loaded)
            .field("initialized", &self.initialized)
            .field("volume", &self.volume)
            .field("manager", &self.manager.as_ref().map(|_| "<audio manager>"))
            .field("sound_handle", &self.sound_handle.as_ref().map(|_| "<sound handle>"))
            .finish()
    }
}
