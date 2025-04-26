use std::sync::Arc;
use crate::ui::audio_player::audio_backend::trait_def::AudioBackend;

#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen::prelude::*,
    wasm_bindgen::JsCast,
    web_sys::{AudioContext, AudioBufferSourceNode, AudioBuffer},
    std::sync::Mutex,
    js_sys::{ArrayBuffer, Uint8Array, Float32Array},
};

/// Web audio backend implementation using Web Audio API
pub struct WebAudioBackend {
    /// Audio context
    #[cfg(target_arch = "wasm32")]
    audio_context: Option<AudioContext>,
    /// Audio buffer
    #[cfg(target_arch = "wasm32")]
    audio_buffer: Option<AudioBuffer>,
    /// Current audio source node
    #[cfg(target_arch = "wasm32")]
    audio_source: Option<AudioBufferSourceNode>,
    /// Audio data
    audio_data: Option<Arc<Vec<u8>>>,
    /// Start time (when playback began)
    start_time: f64,
    /// Playback offset in seconds
    offset: f64,
    /// Audio duration in seconds
    duration: f32,
    /// Current playback state
    is_playing: bool,
    /// Current volume
    volume: f32,
    /// Whether backend initialization succeeded
    initialized: bool,
}

#[cfg(target_arch = "wasm32")]
impl WebAudioBackend {
    /// Create a new web audio backend
    pub fn new() -> Self {
        Self {
            audio_context: None,
            audio_buffer: None,
            audio_source: None,
            audio_data: None,
            start_time: 0.0,
            offset: 0.0,
            duration: 0.0,
            is_playing: false,
            volume: 1.0,
            initialized: false,
        }
    }
    
    /// Create an audio buffer from raw WAV data
    fn create_audio_buffer_from_wav(&self, data: &[u8]) -> Result<AudioBuffer, String> {
        // This is a simplified WAV parser
        // It assumes a standard WAV format with PCM encoding
        
        if data.len() < 44 {
            return Err("Invalid WAV data: too short".to_string());
        }
        
        // Validate WAV header
        if &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
            return Err("Invalid WAV header".to_string());
        }
        
        // Extract format information
        let channels = u16::from_le_bytes([data[22], data[23]]) as usize;
        let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
        let bits_per_sample = u16::from_le_bytes([data[34], data[35]]) as usize;
        
        // Find data chunk
        let mut data_offset = 0;
        let mut data_size = 0;
        let mut i = 36;
        while i < data.len() - 8 {
            if &data[i..i+4] == b"data" {
                data_size = u32::from_le_bytes([data[i+4], data[i+5], data[i+6], data[i+7]]) as usize;
                data_offset = i + 8;
                break;
            }
            i += 1;
        }
        
        if data_offset == 0 || data_size == 0 {
            return Err("Could not find WAV data chunk".to_string());
        }
        
        // Get audio context
        let context = self.audio_context.as_ref()
            .ok_or_else(|| "Audio context not available".to_string())?;
        
        // Create audio buffer
        let audio_buffer = context.create_buffer(
            channels as u32, 
            (data_size / (channels * bits_per_sample / 8)) as u32, 
            sample_rate as f32
        ).map_err(|_| "Failed to create audio buffer".to_string())?;
        
        // Extract samples from the WAV data
        let bytes_per_sample = bits_per_sample / 8;
        
        for channel in 0..channels {
            let channel_data = audio_buffer.get_channel_data(channel as u32)
                .map_err(|_| "Failed to get channel data".to_string())?;
            
            let mut sample_index = 0;
            let mut data_index = data_offset;
            
            while data_index < data_offset + data_size && sample_index < channel_data.length() as usize {
                if data_index + bytes_per_sample > data.len() {
                    break;
                }
                
                let sample_value = match bits_per_sample {
                    8 => (data[data_index] as i8 as f32) / 128.0,
                    16 => {
                        let sample = i16::from_le_bytes([data[data_index], data[data_index + 1]]);
                        (sample as f32) / 32768.0
                    },
                    24 => {
                        let sample_bytes = [data[data_index], data[data_index + 1], data[data_index + 2], 0];
                        let sample = i32::from_le_bytes(sample_bytes) >> 8;
                        (sample as f32) / 8388608.0
                    },
                    32 => {
                        let sample = i32::from_le_bytes([
                            data[data_index], 
                            data[data_index + 1], 
                            data[data_index + 2],
                            data[data_index + 3]
                        ]);
                        (sample as f32) / 2147483648.0
                    },
                    _ => return Err(format!("Unsupported bits per sample: {}", bits_per_sample)),
                };
                
                // Set sample in the channel data
                channel_data.set_index(sample_index as u32, sample_value);
                
                sample_index += 1;
                data_index += bytes_per_sample * channels;
            }
        }
        
        Ok(audio_buffer)
    }
}

#[cfg(target_arch = "wasm32")]
impl AudioBackend for WebAudioBackend {
    fn init(&mut self) -> Result<(), String> {
        match AudioContext::new() {
            Ok(context) => {
                self.audio_context = Some(context);
                self.initialized = true;
                Ok(())
            }
            Err(_) => {
                self.initialized = false;
                Err("Failed to initialize AudioContext".to_string())
            }
        }
    }
    
    fn play_audio(&mut self, data: Arc<Vec<u8>>) -> Result<(), String> {
        if !self.initialized {
            return Err("Audio backend not initialized".to_string());
        }
        
        // Stop any currently playing audio
        self.stop()?;
        
        // Save audio data
        self.audio_data = Some(Arc::clone(&data));
        
        // Get audio context
        let context = self.audio_context.as_ref()
            .ok_or_else(|| "Audio context not available".to_string())?;
        
        // Create audio buffer
        let audio_buffer = self.create_audio_buffer_from_wav(&data)?;
        
        // Save duration
        self.duration = audio_buffer.duration() as f32;
        
        // Create source node
        let audio_source = context.create_buffer_source()
            .map_err(|_| "Failed to create audio source".to_string())?;
        
        // Set buffer
        audio_source.set_buffer(Some(&audio_buffer));
        
        // Connect to destination
        audio_source.connect_with_audio_node(&context.destination())
            .map_err(|_| "Failed to connect audio source".to_string())?;
        
        // Start playback
        audio_source.start_with_when(0.0)
            .map_err(|_| "Failed to start audio playback".to_string())?;
        
        // Save state
        self.audio_buffer = Some(audio_buffer);
        self.audio_source = Some(audio_source);
        self.start_time = context.current_time();
        self.offset = 0.0;
        self.is_playing = true;
        
        Ok(())
    }
    
    fn pause(&mut self) -> Result<(), String> {
        if !self.is_playing || self.audio_source.is_none() {
            return Err("No audio playing".to_string());
        }
        
        // Get context
        let context = self.audio_context.as_ref()
            .ok_or_else(|| "Audio context not available".to_string())?;
            
        // Calculate current position
        self.offset += context.current_time() - self.start_time;
        
        // Stop the current source
        if let Some(source) = &self.audio_source {
            let _ = source.stop();
        }
        
        self.audio_source = None;
        self.is_playing = false;
        
        Ok(())
    }
    
    fn resume(&mut self) -> Result<(), String> {
        if self.is_playing || self.audio_buffer.is_none() {
            return Err("No paused audio to resume".to_string());
        }
        
        // Get context
        let context = self.audio_context.as_ref()
            .ok_or_else(|| "Audio context not available".to_string())?;
            
        // Get buffer
        let buffer = self.audio_buffer.as_ref()
            .ok_or_else(|| "No audio buffer available".to_string())?;
            
        // Create a new source
        let source = context.create_buffer_source()
            .map_err(|_| "Failed to create audio source".to_string())?;
            
        // Set buffer
        source.set_buffer(Some(buffer));
        
        // Connect to destination
        source.connect_with_audio_node(&context.destination())
            .map_err(|_| "Failed to connect audio source".to_string())?;
        
        // TODO: In a real implementation, this would apply the volume using a GainNode
        // For now we just save the volume value in the WebAudioBackend struct
            
        // Start playback from the current offset
        source.start_with_when_and_grain_offset(0.0, self.offset)
            .map_err(|_| "Failed to start audio playback".to_string())?;
            
        // Save state
        self.audio_source = Some(source);
        self.start_time = context.current_time();
        self.is_playing = true;
        
        Ok(())
    }
    
    fn stop(&mut self) -> Result<(), String> {
        if self.audio_source.is_none() {
            return Ok(());
        }
        
        // Stop the current source
        if let Some(source) = &self.audio_source {
            let _ = source.stop();
        }
        
        // Reset state
        self.audio_source = None;
        self.is_playing = false;
        self.offset = 0.0;
        
        Ok(())
    }
    
    fn set_position(&mut self, position_secs: f32) -> Result<(), String> {
        if self.audio_buffer.is_none() {
            return Err("No audio loaded".to_string());
        }
        
        let was_playing = self.is_playing;
        
        // Stop current playback
        self.stop()?;
        
        // Set new offset
        self.offset = position_secs as f64;
        
        // If we were playing, resume at the new position
        if was_playing {
            self.resume()?;
        }
        
        Ok(())
    }
    
    fn set_volume(&mut self, volume: f32) -> Result<(), String> {
        // Web Audio API doesn't have a simple volume property on source nodes
        // In a real implementation, we would use a GainNode to control volume
        // For now we just save the value for future use
        self.volume = volume;
        Ok(())
    }
    
    fn is_playing(&self) -> bool {
        self.is_playing
    }
    
    fn get_position(&self) -> f32 {
        if !self.is_playing {
            return self.offset as f32;
        }
        
        if let Some(context) = &self.audio_context {
            (self.offset + context.current_time() - self.start_time) as f32
        } else {
            self.offset as f32
        }
    }
    
    fn get_duration(&self) -> f32 {
        self.duration
    }
    
    fn is_available(&self) -> bool {
        self.initialized
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl WebAudioBackend {
    /// Create a new web audio backend stub for non-web platforms
    pub fn new() -> Self {
        Self {
            audio_data: None,
            start_time: 0.0,
            offset: 0.0,
            duration: 0.0,
            is_playing: false,
            volume: 1.0,
            initialized: false,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl AudioBackend for WebAudioBackend {
    fn init(&mut self) -> Result<(), String> {
        Err("Web audio backend is not available on this platform".to_string())
    }
    
    fn play_audio(&mut self, _data: Arc<Vec<u8>>) -> Result<(), String> {
        Err("Web audio backend is not available on this platform".to_string())
    }
    
    fn pause(&mut self) -> Result<(), String> {
        Err("Web audio backend is not available on this platform".to_string())
    }
    
    fn resume(&mut self) -> Result<(), String> {
        Err("Web audio backend is not available on this platform".to_string())
    }
    
    fn stop(&mut self) -> Result<(), String> {
        Err("Web audio backend is not available on this platform".to_string())
    }
    
    fn set_position(&mut self, _position_secs: f32) -> Result<(), String> {
        Err("Web audio backend is not available on this platform".to_string())
    }
    
    fn set_volume(&mut self, _volume: f32) -> Result<(), String> {
        Err("Web audio backend is not available on this platform".to_string())
    }
    
    fn is_playing(&self) -> bool {
        false
    }
    
    fn get_position(&self) -> f32 {
        0.0
    }
    
    fn get_duration(&self) -> f32 {
        0.0
    }
    
    fn is_available(&self) -> bool {
        false
    }
}

impl Default for WebAudioBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for WebAudioBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebAudioBackend")
            .field("duration", &self.duration)
            .field("initialized", &self.initialized)
            .field("volume", &self.volume)
            .field("audio_data", &self.audio_data.as_ref().map(|_| "<audio data>"))
            .finish()
    }
}
