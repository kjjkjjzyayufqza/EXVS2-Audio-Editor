use std::io::Cursor;
use std::sync::Arc;

/// Audio playback backend trait
/// Defines the interface for platform-specific audio playback implementations
pub trait AudioBackend: std::fmt::Debug {
    /// Initialize the audio backend
    fn init(&mut self) -> Result<(), String>;
    
    /// Play audio from raw data
    fn play_audio(&mut self, data: Arc<Vec<u8>>) -> Result<(), String>;
    
    /// Pause audio playback
    fn pause(&mut self) -> Result<(), String>;
    
    /// Resume audio playback
    fn resume(&mut self) -> Result<(), String>;
    
    /// Stop audio playback
    fn stop(&mut self) -> Result<(), String>;
    
    /// Set the playback position in seconds
    fn set_position(&mut self, position_secs: f32) -> Result<(), String>;
    
    /// Set the volume (0.0 - 1.0)
    fn set_volume(&mut self, volume: f32) -> Result<(), String>;
    
    /// Check if audio is currently playing
    fn is_playing(&self) -> bool;
    
    /// Get the current playback position in seconds
    fn get_position(&self) -> f32;
    
    /// Get the duration of the current audio in seconds
    fn get_duration(&self) -> f32;
    
    /// Check if the backend is available (properly initialized)
    fn is_available(&self) -> bool;
}

// Native platform implementation (rodio-based)
#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;
    use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    
    /// Native audio backend implementation using rodio
    pub struct NativeAudioBackend {
        /// Audio output stream
        _stream: Option<OutputStream>,
        /// Audio output stream handle
        stream_handle: Option<OutputStreamHandle>,
        /// Audio sink for playback control
        sink: Option<Arc<Mutex<Sink>>>,
        /// Raw audio data
        audio_data: Option<Arc<Vec<u8>>>,
        /// Cached decoded audio data for faster seeking
        decoded_audio: Option<Vec<u8>>,
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
    }
    
    impl NativeAudioBackend {
        /// Create a new native audio backend
        pub fn new() -> Self {
            Self {
                _stream: None,
                stream_handle: None,
                sink: None,
                audio_data: None,
                decoded_audio: None,
                current_position: 0.0,
                playback_start_time: None,
                playback_start_position: 0.0,
                duration: 0.0,
                audio_loaded: false,
                is_playing: false,
                initialized: false,
            }
        }
        
        /// Estimate the duration of audio from the WAV header
        fn estimate_wav_duration(&self, data: &[u8]) -> f32 {
            if data.len() < 44 {
                return 0.0; // Not enough data for a WAV header
            }
            
            // Validate WAV header
            if &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
                return 0.0; // Not a valid WAV file
            }
            
            // Extract format information
            let channels = u16::from_le_bytes([data[22], data[23]]);
            let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
            let bits_per_sample = u16::from_le_bytes([data[34], data[35]]);
            
            // Find data chunk
            let mut data_size = 0;
            let mut i = 36;
            while i < data.len() - 8 {
                if &data[i..i+4] == b"data" {
                    data_size = u32::from_le_bytes([data[i+4], data[i+5], data[i+6], data[i+7]]);
                    break;
                }
                i += 1;
            }
            
            if data_size == 0 || sample_rate == 0 || channels == 0 || bits_per_sample == 0 {
                return 0.0;
            }
            
            // Calculate duration
            let bytes_per_sample = (bits_per_sample / 8) as u32;
            let bytes_per_second = sample_rate * channels as u32 * bytes_per_sample;
            
            if bytes_per_second > 0 {
                data_size as f32 / bytes_per_second as f32
            } else {
                0.0
            }
        }
    }
    
    impl AudioBackend for NativeAudioBackend {
        fn init(&mut self) -> Result<(), String> {
            match OutputStream::try_default() {
                Ok((stream, stream_handle)) => {
                    self._stream = Some(stream);
                    self.stream_handle = Some(stream_handle);
                    self.initialized = true;
                    Ok(())
                }
                Err(e) => {
                    self.initialized = false;
                    Err(format!("Failed to initialize audio output stream: {}", e))
                }
            }
        }
        
        fn play_audio(&mut self, data: Arc<Vec<u8>>) -> Result<(), String> {
            if !self.initialized {
                return Err("Audio backend not initialized".to_string());
            }
            
            // Stop any currently playing audio
            if let Some(sink) = &self.sink {
                let _ = sink.lock().unwrap().stop();
            }
            
            // Reset position tracking
            self.current_position = 0.0;
            self.playback_start_time = Some(Instant::now());
            self.playback_start_position = 0.0;
            
            // Save audio data
            self.audio_data = Some(Arc::clone(&data));
            
            // Cache the raw data for faster seeking later
            self.decoded_audio = Some((*data).clone());
            
            // Try to decode the audio
            let cursor = Cursor::new((*data).clone());
            let source = match Decoder::new(cursor) {
                Ok(source) => source,
                Err(e) => return Err(format!("Failed to decode audio data: {}", e)),
            };
            
            // Extract duration from the source
            self.duration = source.total_duration()
                .unwrap_or(Duration::from_secs(0))
                .as_secs_f32();
            
            // If we couldn't get the duration from the decoder, try to estimate it from the WAV header
            if self.duration == 0.0 {
                self.duration = self.estimate_wav_duration(&data);
            }
            
            // Create a new sink
            let stream_handle = self.stream_handle.as_ref()
                .ok_or_else(|| "Audio stream handle not available".to_string())?;
                
            let sink = match Sink::try_new(stream_handle) {
                Ok(sink) => sink,
                Err(e) => return Err(format!("Failed to create audio sink: {}", e)),
            };
            
            // Add the source to the sink
            sink.append(source);
            
            // Save the sink
            self.sink = Some(Arc::new(Mutex::new(sink)));
            self.audio_loaded = true;
            self.is_playing = true;
            
            Ok(())
        }
        
        fn pause(&mut self) -> Result<(), String> {
            if let Some(sink) = &self.sink {
                // Update position before pausing
                if self.is_playing {
                    if let Some(start_time) = self.playback_start_time {
                        let elapsed = start_time.elapsed().as_secs_f32();
                        self.current_position = self.playback_start_position + elapsed;
                    }
                }
                
                sink.lock().unwrap().pause();
                self.is_playing = false;
                Ok(())
            } else {
                Err("No audio playing".to_string())
            }
        }
        
        fn resume(&mut self) -> Result<(), String> {
            if let Some(sink) = &self.sink {
                sink.lock().unwrap().play();
                
                // Update time tracking for proper position calculation
                self.playback_start_time = Some(Instant::now());
                self.playback_start_position = self.current_position;
                self.is_playing = true;
                
                Ok(())
            } else {
                Err("No audio loaded".to_string())
            }
        }
        
        fn stop(&mut self) -> Result<(), String> {
            if let Some(sink) = &self.sink {
                // Reset position tracking
                self.current_position = 0.0;
                self.playback_start_position = 0.0;
                self.playback_start_time = None;
                self.is_playing = false;
                
                sink.lock().unwrap().stop();
                self.sink = None;
                
                // Don't clear audio_loaded or audio_data so we can play it again
                Ok(())
            } else {
                Err("No audio playing".to_string())
            }
        }
        
        fn set_position(&mut self, position_secs: f32) -> Result<(), String> {
            if !self.audio_loaded || self.audio_data.is_none() {
                return Err("No audio loaded".to_string());
            }
            
            let was_playing = self.is_playing;
            let clamped_position = position_secs.clamp(0.0, self.duration);
            
            // Store the requested position
            self.current_position = clamped_position;
            self.playback_start_position = clamped_position;
            
            // If not playing, just update the position without reloading audio
            if !was_playing {
                return Ok(());
            }
            
            // For playback, we need to reload the audio at the new position
            if let Some(data) = &self.decoded_audio {
                // Use cached data if available to save decoding time
                let data_arc = Arc::new(data.clone());
                
                // Stop current playback
                if let Some(sink) = &self.sink {
                    let _ = sink.lock().unwrap().stop();
                }
                
                // Create a new sink
                let stream_handle = self.stream_handle.as_ref()
                    .ok_or_else(|| "Audio stream handle not available".to_string())?;
                    
                let sink = match Sink::try_new(stream_handle) {
                    Ok(sink) => sink,
                    Err(e) => return Err(format!("Failed to create audio sink: {}", e)),
                };
                
                // Try to decode the audio using cached data
                let cursor = Cursor::new(data_arc.to_vec());
                let source = match Decoder::new(cursor) {
                    Ok(source) => source,
                    Err(e) => return Err(format!("Failed to decode audio data: {}", e)),
                };
                
                // Skip to the desired position
                let skip_duration = Duration::from_secs_f32(clamped_position);
                let skipped_source = source.skip_duration(skip_duration);
                
                // Add the source to the sink
                sink.append(skipped_source);
                
                // Save the sink
                self.sink = Some(Arc::new(Mutex::new(sink)));
                
                // Update time tracking
                self.playback_start_time = Some(Instant::now());
                self.is_playing = true;
            } else {
                // Fallback if no cached data is available
                let data = self.audio_data.as_ref().unwrap().clone();
                
                // Stop current playback
                if let Some(sink) = &self.sink {
                    let _ = sink.lock().unwrap().stop();
                }
                
                // Create a new sink
                let stream_handle = self.stream_handle.as_ref()
                    .ok_or_else(|| "Audio stream handle not available".to_string())?;
                    
                let sink = match Sink::try_new(stream_handle) {
                    Ok(sink) => sink,
                    Err(e) => return Err(format!("Failed to create audio sink: {}", e)),
                };
                
                // Try to decode the audio
                let cursor = Cursor::new((*data).clone());
                let source = match Decoder::new(cursor) {
                    Ok(source) => source,
                    Err(e) => return Err(format!("Failed to decode audio data: {}", e)),
                };
                
                // Skip to the desired position
                let skip_duration = Duration::from_secs_f32(clamped_position);
                let skipped_source = source.skip_duration(skip_duration);
                
                // Add the source to the sink
                sink.append(skipped_source);
                
                // Save the sink
                self.sink = Some(Arc::new(Mutex::new(sink)));
                
                // Update time tracking
                self.playback_start_time = Some(Instant::now());
                self.is_playing = true;
                
                // Cache the data for future seeking operations
                self.decoded_audio = Some((*data).clone());
            }
            
            Ok(())
        }
        
        fn set_volume(&mut self, volume: f32) -> Result<(), String> {
            if let Some(sink) = &self.sink {
                sink.lock().unwrap().set_volume(volume);
                Ok(())
            } else {
                Err("No audio playing".to_string())
            }
        }
        
        fn is_playing(&self) -> bool {
            if let Some(sink) = &self.sink {
                !sink.lock().unwrap().is_paused()
            } else {
                false
            }
        }
        
        fn get_position(&self) -> f32 {
            if !self.is_playing {
                // When not playing, return the stored position
                return self.current_position;
            }
            
            // When playing, calculate the current position from the elapsed time
            if let Some(start_time) = self.playback_start_time {
                let elapsed = start_time.elapsed().as_secs_f32();
                let position = self.playback_start_position + elapsed;
                
                // Make sure we don't exceed the duration
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
                .field("audio_data", &self.audio_data.as_ref().map(|_| "<audio data>"))
                .field("_stream", &"<stream>")
                .field("stream_handle", &"<stream handle>")
                .field("sink", &"<sink>")
                .finish()
        }
    }
}

// Web platform implementation
#[cfg(target_arch = "wasm32")]
mod web {
    use super::*;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::{AudioContext, AudioBufferSourceNode, AudioBuffer};
    use std::sync::Mutex;
    use js_sys::{ArrayBuffer, Uint8Array, Float32Array};
    
    /// Web audio backend implementation using Web Audio API
    pub struct WebAudioBackend {
        /// Audio context
        audio_context: Option<AudioContext>,
        /// Audio buffer
        audio_buffer: Option<AudioBuffer>,
        /// Current audio source node
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
            // In a real implementation, we would use a GainNode, but for now we just save the value
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
    
    impl Default for WebAudioBackend {
        fn default() -> Self {
            Self::new()
        }
    }
}

// Export the appropriate audio backend based on the platform
#[cfg(not(target_arch = "wasm32"))]
pub use native::NativeAudioBackend as PlatformAudioBackend;

#[cfg(target_arch = "wasm32")]
pub use web::WebAudioBackend as PlatformAudioBackend;
