use std::io::Cursor;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use crate::ui::audio_player::audio_backend::trait_def::AudioBackend;

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
    /// Current volume level (0.0 - 1.0)
    volume: f32,
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
            volume: 1.0, // Default volume is 100%
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
            sink.lock().unwrap().stop();
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
        
        // Apply current volume
        if let Some(sink) = &self.sink {
            sink.lock().unwrap().set_volume(self.volume);
        }
        
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
            // Apply current volume setting before resuming playback
            sink.lock().unwrap().set_volume(self.volume);
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
                sink.lock().unwrap().stop();
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
            
            // Apply current volume
            sink.set_volume(self.volume);
            
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
                sink.lock().unwrap().stop();
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
            
            // Apply current volume
            sink.set_volume(self.volume);
            
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
        // Save the volume value for future use
        self.volume = volume;
        
        // Apply to the current sink if available
        if let Some(sink) = &self.sink {
            sink.lock().unwrap().set_volume(volume);
            Ok(())
        } else {
            // Even if there's no active sink, we consider this successful
            // as we've saved the volume for future playback
            Ok(())
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
            .field("volume", &self.volume)
            .field("audio_data", &self.audio_data.as_ref().map(|_| "<audio data>"))
            .field("_stream", &"<stream>")
            .field("stream_handle", &"<stream handle>")
            .field("sink", &"<sink>")
            .finish()
    }
}
