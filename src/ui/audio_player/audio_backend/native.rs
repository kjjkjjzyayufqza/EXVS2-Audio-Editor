use std::fs::File;
use std::io::Read;
use std::time::{Duration, Instant};

use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, Easing, StartTime, Tween,
    sound::PlaybackState,
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
};

use super::super::gain::OUTPUT_LEVEL_CAP;
use crate::ui::audio_player::audio_backend::trait_def::AudioBackend;

/// Native audio backend implementation using kira (static sound for instant seek)
pub struct NativeAudioBackend {
    manager: Option<AudioManager<DefaultBackend>>,
    sound_handle: Option<StaticSoundHandle>,
    current_position: f32,
    playback_start_time: Option<Instant>,
    playback_start_position: f32,
    duration: f32,
    audio_loaded: bool,
    is_playing: bool,
    initialized: bool,
    volume: f32,
    cached_path: Option<String>,
    cached_data: Option<StaticSoundData>,
}

impl NativeAudioBackend {
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
            volume: 1.0,
            cached_path: None,
            cached_data: None,
        }
    }

    fn instant_tween() -> Tween {
        Tween {
            start_time: StartTime::Immediate,
            duration: Duration::ZERO,
            easing: Easing::Linear,
        }
    }

    fn decode_or_reuse(&mut self, file_path: &str) -> Result<StaticSoundData, String> {
        if self.cached_path.as_deref() == Some(file_path)
            && let Some(data) = &self.cached_data
        {
            return Ok(data.clone());
        }
        let t_load = Instant::now();
        let sound_data = StaticSoundData::from_file(file_path)
            .map_err(|e| format!("Failed to load audio file: {e}"))?;
        println!(
            "[PERF] kira StaticSoundData::from_file: {}ms",
            t_load.elapsed().as_millis()
        );
        self.cached_path = Some(file_path.to_owned());
        self.cached_data = Some(sound_data.clone());
        Ok(sound_data)
    }

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

    /// Convert a linear amplitude scale to dB for kira.
    /// Values > 1.0 are allowed (e.g. replace-preview gain boosts).
    fn volume_to_decibels(volume: f32) -> f32 {
        if volume <= 0.0 {
            -80.0
        } else {
            20.0 * volume.max(1e-6).log10()
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

        if let Some(mut handle) = self.sound_handle.take() {
            handle.stop(Self::instant_tween());
        }

        let sound_data = self.decode_or_reuse(file_path)?;

        // Prefer kira's decoded duration. Header estimation often fails (extensible WAV,
        // late data chunk, non-WAV) and then get_position() clamps to 0 forever.
        let kira_duration = sound_data.duration().as_secs_f64() as f32;
        let header_duration = self.estimate_wav_duration_from_file(file_path);
        self.duration = if kira_duration.is_finite() && kira_duration > 0.0 {
            kira_duration
        } else if header_duration > 0.0 {
            header_duration
        } else {
            0.0
        };
        println!(
            "[PERF] duration: kira={kira_duration:.3}s header={header_duration:.3}s -> using={:.3}s",
            self.duration
        );

        let manager = self
            .manager
            .as_mut()
            .ok_or_else(|| "Audio manager not available".to_string())?;

        let t_play = Instant::now();
        let mut handle = manager
            .play(sound_data)
            .map_err(|e| format!("Failed to start audio playback: {e}"))?;
        println!(
            "[PERF] kira manager.play: {}ms",
            t_play.elapsed().as_millis()
        );

        self.current_position = 0.0;
        self.playback_start_time = Some(Instant::now());
        self.playback_start_position = 0.0;
        self.audio_loaded = true;
        self.is_playing = true;

        let volume_db = Self::volume_to_decibels(self.volume);
        let _ = handle.set_volume(volume_db, Self::instant_tween());

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

            handle.pause(Self::instant_tween());
            self.is_playing = false;
            Ok(())
        } else {
            Err("No audio playing".to_string())
        }
    }

    fn resume(&mut self) -> Result<(), String> {
        if !self.is_loaded() {
            return Err("No audio loaded".to_string());
        }
        if let Some(handle) = &mut self.sound_handle {
            let volume_db = Self::volume_to_decibels(self.volume);
            let _ = handle.set_volume(volume_db, Self::instant_tween());
            handle.resume(Self::instant_tween());

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
            self.audio_loaded = false;

            handle.stop(Self::instant_tween());
            Ok(())
        } else {
            self.audio_loaded = false;
            Err("No audio playing".to_string())
        }
    }

    fn set_position(&mut self, position_secs: f32) -> Result<(), String> {
        if !self.audio_loaded {
            return Err("No audio loaded".to_string());
        }

        let clamped_position = if self.duration > 0.0 {
            position_secs.clamp(0.0, self.duration)
        } else {
            position_secs.max(0.0)
        };
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
        // Allow >1.0 so preview gain (dB boost) can be heard; hard-cap for safety.
        self.volume = volume.clamp(0.0, OUTPUT_LEVEL_CAP);
        if let Some(handle) = &mut self.sound_handle {
            let volume_db = Self::volume_to_decibels(self.volume);
            let _ = handle.set_volume(volume_db, Self::instant_tween());
        }
        Ok(())
    }

    fn is_playing(&self) -> bool {
        if !self.is_loaded() {
            return false;
        }

        if self.is_playing && self.duration > 0.0 {
            return self.get_position() < self.duration;
        }

        self.is_playing
    }

    fn get_position(&self) -> f32 {
        if !self.is_playing {
            return self.current_position.max(0.0);
        }

        if let Some(start_time) = self.playback_start_time {
            let elapsed = start_time.elapsed().as_secs_f32();
            let position = self.playback_start_position + elapsed;
            // Only clamp when we have a known positive duration
            if self.duration > 0.0 {
                position.clamp(0.0, self.duration)
            } else {
                position.max(0.0)
            }
        } else {
            self.current_position.max(0.0)
        }
    }

    fn get_duration(&self) -> f32 {
        self.duration
    }

    fn is_available(&self) -> bool {
        self.initialized
    }

    fn is_loaded(&self) -> bool {
        let Some(handle) = self.sound_handle.as_ref() else {
            return false;
        };
        // kira resume() is a no-op once the clip has Stopped (natural end or stop()).
        !matches!(
            handle.state(),
            PlaybackState::Stopped | PlaybackState::Stopping
        )
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
            .field(
                "sound_handle",
                &self.sound_handle.as_ref().map(|_| "<sound handle>"),
            )
            .finish()
    }
}
