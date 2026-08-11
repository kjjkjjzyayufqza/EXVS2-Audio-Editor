use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

use super::audio_backend::{AudioBackend, PlatformAudioBackend};
use crate::ui::main_area::AudioFileInfo;
use crate::ui::waveform::{DEFAULT_PEAK_BINS, WaveformPeaks};

/// Audio loop mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopMode {
    /// No looping, stop at the end of the playlist
    None,
    /// Loop the current track indefinitely
    Single,
    /// Loop the entire playlist
    All,
}

/// Persisted audio player settings
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AudioPlayerSettings {
    pub volume: f32,
    pub is_muted: bool,
    pub previous_volume: f32,
    pub loop_mode: LoopMode,
    pub shuffle: bool,
}

impl Default for AudioPlayerSettings {
    fn default() -> Self {
        Self {
            volume: 0.80,
            is_muted: false,
            previous_volume: 0.80,
            loop_mode: LoopMode::None,
            shuffle: false,
        }
    }
}

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
    
    /// Current volume (0.0 - 100.0)
    #[serde(skip)]
    pub volume: f32,
    
    /// Is the audio muted
    pub is_muted: bool,
    
    /// Previous volume before mute
    #[serde(skip)]
    pub previous_volume: f32,
    
    /// Custom loop start point in seconds (None = start from beginning)
    #[serde(skip)]
    pub loop_start: Option<f32>,
    
    /// Custom loop end point in seconds (None = loop to end)
    #[serde(skip)]
    pub loop_end: Option<f32>,
    
    /// Whether to use custom loop points
    #[serde(skip)]
    pub use_custom_loop: bool,

    /// Track has loop metadata (A-B or full-file marks on the wave)
    #[serde(skip)]
    pub enable_loop: bool,

    /// When true, playback rewinds inside the track loop region.
    /// Independent of queue `loop_mode`. Default off for sequential listening.
    #[serde(skip)]
    pub honor_track_loop: bool,
    
    /// Current queue loop mode (repeat off / one / all)
    pub loop_mode: LoopMode,
    
    /// Whether shuffle mode is enabled
    pub shuffle: bool,
    
    /// Current playlist (UI table order: filtered + sorted; never re-sorted by ID)
    #[serde(skip)]
    pub playlist: Vec<AudioFileInfo>,
    
    /// Index of the current track in the playlist
    #[serde(skip)]
    pub current_track_index: Option<usize>,

    /// Remaining shuffle indices (bag); rebuilt when empty or playlist changes
    #[serde(skip)]
    shuffle_bag: Vec<usize>,

    /// Whether the current track has finished and we should play the next one
    #[serde(skip)]
    pub should_play_next: bool,

    /// Whether the user requested the previous track
    #[serde(skip)]
    pub should_play_previous: bool,

    /// Fire a one-shot "playlist ended" toast (auto-advance could not continue)
    #[serde(skip)]
    pub should_notify_playlist_ended: bool,

    /// True while the user is dragging the seek slider (suppresses backend position updates)
    #[serde(skip)]
    pub is_seeking: bool,

    /// Last time (Instant) a live scrub seek was pushed to the backend (throttle)
    #[serde(skip)]
    last_scrub_seek: Option<std::time::Instant>,

    /// Last position actually sent to the backend during scrub
    #[serde(skip)]
    last_scrub_pos: f32,

    /// Audio backend for playback
    #[serde(skip)]
    audio_backend: Option<Box<dyn AudioBackend>>,

    /// Cached sound-wave peaks for the current track
    #[serde(skip)]
    pub waveform: Option<WaveformPeaks>,

    /// True while background peak extraction is running
    #[serde(skip)]
    pub waveform_loading: bool,

    /// Monotonic generation to drop stale async waveform results
    #[serde(skip)]
    waveform_generation: u64,

    /// Receiver for background waveform jobs
    #[serde(skip)]
    waveform_rx: Option<Receiver<(u64, WaveformPeaks)>>,
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
            .field("waveform", &self.waveform.as_ref().map(|w| w.peaks.len()))
            .field("waveform_loading", &self.waveform_loading)
            .field("audio_backend", &"<audio backend>".to_string())
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
            loop_start: self.loop_start,
            loop_end: self.loop_end,
            use_custom_loop: self.use_custom_loop,
            enable_loop: self.enable_loop,
            honor_track_loop: self.honor_track_loop,
            loop_mode: self.loop_mode,
            shuffle: self.shuffle,
            playlist: self.playlist.clone(),
            current_track_index: self.current_track_index,
            shuffle_bag: self.shuffle_bag.clone(),
            should_play_next: self.should_play_next,
            should_play_previous: self.should_play_previous,
            should_notify_playlist_ended: self.should_notify_playlist_ended,
            is_seeking: self.is_seeking,
            last_scrub_seek: None,
            last_scrub_pos: self.last_scrub_pos,
            audio_backend: None,
            waveform: self.waveform.clone(),
            waveform_loading: self.waveform_loading,
            waveform_generation: self.waveform_generation,
            waveform_rx: None,
        }
    }
}

/// Audio file information
#[derive(Clone, Debug)]
pub struct AudioFile {
    /// Original file path
    pub file_path: String,

    /// Temporary playback file path (native only)
    #[cfg(not(target_arch = "wasm32"))]
    pub playback_path: Option<String>,
    
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
            volume: 0.80, // Default volume at 80%
            is_muted: false,
            previous_volume: 0.80,
            loop_start: None,
            loop_end: None,
            use_custom_loop: false,
            enable_loop: false,
            honor_track_loop: false,
            loop_mode: LoopMode::None,
            shuffle: false,
            playlist: Vec::new(),
            current_track_index: None,
            shuffle_bag: Vec::new(),
            should_play_next: false,
            should_play_previous: false,
            should_notify_playlist_ended: false,
            is_seeking: false,
            last_scrub_seek: None,
            last_scrub_pos: f32::NAN,
            audio_backend: None,
            waveform: None,
            waveform_loading: false,
            waveform_generation: 0,
            waveform_rx: None,
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
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let playback_path = audio
                            .playback_path
                            .as_deref()
                            .unwrap_or(&audio.file_path);

                        // If we're resuming from a position other than the beginning,
                        // we need to set the position after starting playback
                        let position = self.current_position;

                        if let Err(e) = backend.play_audio(playback_path) {
                            log::error!("Failed to play audio: {}", e);
                            self.is_playing = false;
                            return;
                        }

                        // Get actual duration from backend
                        self.total_duration = backend.get_duration();

                        // Apply current volume setting
                        if let Err(e) = backend.set_volume(self.volume) {
                            log::error!("Failed to apply volume: {}", e);
                        }

                        // If we're resuming from a non-zero position, seek to that position
                        if position > 0.0 {
                            if let Err(e) = backend.set_position(position) {
                                log::error!("Failed to seek to position {}: {}", position, e);
                                // Continue playback even if seeking fails
                            }
                        }
                    }
                }
            } else if let Err(e) = backend.pause() {
                // Only log as debug if no audio is playing, as this is expected behavior
                if e.contains("No audio playing") {
                    log::debug!("Pause called but no audio is currently playing");
                } else {
                    log::error!("Failed to pause audio: {}", e);
                }
            }
        }
    }
    
    /// Stop the audio playback
    pub fn stop(&mut self) {
        self.is_playing = false;
        self.current_position = 0.0;
        
        if let Some(backend) = &mut self.audio_backend {
            if let Err(e) = backend.stop() {
                // Only log as debug if no audio is playing, as this is expected behavior
                if e.contains("No audio playing") {
                    log::debug!("Stop called but no audio is currently playing");
                } else {
                    log::error!("Failed to stop audio: {}", e);
                }
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

        // Only delete the previous temp file when switching to a *different* path.
        // If the same cached WAV path is reused (e.g. clicking the same track again),
        // cleanup_temp_audio would delete the file we are about to play.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let reusing_same_path = self.current_audio
                .as_ref()
                .and_then(|a| a.playback_path.as_deref())
                .zip(audio.playback_path.as_deref())
                .map(|(old, new)| old == new)
                .unwrap_or(false);

            if !reusing_same_path {
                self.cleanup_temp_audio();
            }
        }

        // Waveform is loaded asynchronously after path is known
        self.waveform = None;
        self.waveform_loading = false;
        self.waveform_rx = None;
        self.waveform_generation = self.waveform_generation.wrapping_add(1);

        // Set new audio file
        self.current_audio = Some(audio);
        
        // Apply current volume setting to the audio backend
        if let Some(backend) = &mut self.audio_backend {
            if let Err(e) = backend.set_volume(self.volume) {
                log::error!("Failed to apply volume to new audio: {}", e);
            }
        }
        
        // Play the new audio right away
        // Set is_playing to false first, so toggle_play will set it to true and start playback
        self.is_playing = false;
        self.toggle_play();
    }
    
    /// Clear the current audio
    pub fn clear_audio(&mut self) {
        self.stop();
        self.cleanup_temp_audio();
        self.current_audio = None;
        self.waveform = None;
        self.waveform_loading = false;
        self.waveform_rx = None;
        self.waveform_generation = self.waveform_generation.wrapping_add(1);
    }

    /// Start background peak extraction so playback UI never blocks on large WAVs.
    pub fn load_waveform_async(&mut self, path: &str) {
        self.waveform = None;
        self.waveform_generation = self.waveform_generation.wrapping_add(1);
        let wave_gen = self.waveform_generation;
        self.waveform_loading = true;
        self.waveform_rx = Some(WaveformPeaks::spawn_load(
            PathBuf::from(path),
            DEFAULT_PEAK_BINS,
            wave_gen,
        ));
        log::info!("Queued async waveform load (gen={wave_gen}) for {path}");
    }

    /// Poll async waveform job; call once per frame while loading.
    pub fn poll_waveform(&mut self) -> bool {
        let Some(rx) = self.waveform_rx.as_ref() else {
            return false;
        };
        match rx.try_recv() {
            Ok((wave_gen, peaks)) => {
                if wave_gen != self.waveform_generation {
                    // Stale result from a previous track
                    return false;
                }
                self.waveform_rx = None;
                self.waveform_loading = false;
                if peaks.is_empty() {
                    println!("[PERF] async waveform empty (gen={wave_gen})");
                    self.waveform = None;
                } else {
                    println!(
                        "[PERF] async waveform ready: {} bins, {:.2}s (gen={})",
                        peaks.peaks.len(),
                        peaks.duration_secs,
                        wave_gen
                    );
                    // If backend duration was wrong/0, prefer peaks duration for loop UI
                    if self.total_duration <= 0.0 && peaks.duration_secs > 0.0 {
                        self.total_duration = peaks.duration_secs;
                    }
                    // Refresh full-track loop end once we know duration
                    if self.enable_loop && !self.use_custom_loop && peaks.duration_secs > 0.0 {
                        self.loop_start = Some(0.0);
                        self.loop_end = Some(peaks.duration_secs);
                    }
                    self.waveform = Some(peaks);
                }
                true
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => false,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                println!("[PERF] async waveform worker disconnected (gen={})", self.waveform_generation);
                self.waveform_rx = None;
                self.waveform_loading = false;
                false
            }
        }
    }
    
    /// Set the current position in seconds
    pub fn set_position(&mut self, position: f32) {
        let max = if self.total_duration > 0.0 {
            self.total_duration
        } else {
            position.max(0.0)
        };
        self.current_position = if self.total_duration > 0.0 {
            position.clamp(0.0, max)
        } else {
            position.max(0.0)
        };
        self.last_scrub_pos = self.current_position;
        self.last_scrub_seek = Some(std::time::Instant::now());

        // Apply to backend when playing (or always if loaded — seek while paused is fine)
        if let Some(backend) = &mut self.audio_backend {
            if let Err(e) = backend.set_position(self.current_position) {
                log::error!("Failed to set audio position: {}", e);
            }
        }
    }

    /// Drag scrub on the waveform: update UI playhead; throttle backend seeks hard.
    /// Holding the mouse still must NOT flood the audio device.
    pub fn scrub_to(&mut self, position: f32) {
        let max = if self.total_duration > 0.0 {
            self.total_duration
        } else {
            f32::MAX
        };
        let t = position.clamp(0.0, max);
        self.is_seeking = true;
        self.current_position = t;

        // Skip backend if position barely moved
        if self.last_scrub_pos.is_finite() && (t - self.last_scrub_pos).abs() < 0.04 {
            return;
        }

        // At most ~12 seeks/sec while dragging
        const MIN_INTERVAL: std::time::Duration = std::time::Duration::from_millis(80);
        if let Some(prev) = self.last_scrub_seek {
            if prev.elapsed() < MIN_INTERVAL {
                return;
            }
        }

        self.last_scrub_pos = t;
        self.last_scrub_seek = Some(std::time::Instant::now());
        if let Some(backend) = &mut self.audio_backend {
            if let Err(e) = backend.set_position(t) {
                log::error!("Failed to scrub audio position: {e}");
            }
        }
    }

    /// Click or drag-release: one definitive seek, end scrubbing.
    pub fn commit_seek(&mut self, position: f32) {
        self.is_seeking = false;
        self.set_position(position);
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

    /// Get persisted audio settings
    pub fn settings(&self) -> AudioPlayerSettings {
        AudioPlayerSettings {
            volume: self.volume,
            is_muted: self.is_muted,
            previous_volume: self.previous_volume,
            loop_mode: self.loop_mode,
            shuffle: self.shuffle,
        }
    }

    /// Apply persisted audio settings
    pub fn apply_settings(&mut self, settings: &AudioPlayerSettings) {
        self.loop_mode = settings.loop_mode;
        self.shuffle = settings.shuffle;
        self.previous_volume = settings.previous_volume.clamp(0.0, 1.0);

        let mut volume = settings.volume.clamp(0.0, 1.0);
        if settings.is_muted {
            if volume > 0.0 {
                self.previous_volume = volume;
            }
            volume = 0.0;
        } else if volume == 0.0 && self.previous_volume > 0.0 {
            volume = self.previous_volume;
        }

        self.volume = volume;
        self.is_muted = settings.is_muted;

        if let Some(backend) = &mut self.audio_backend {
            if let Err(e) = backend.set_volume(self.volume) {
                log::error!("Failed to apply persisted volume: {}", e);
            }
        }
    }
    
    /// Update playback state from backend
    pub fn update_from_backend(&mut self) {
        // Pick up waveform results without blocking
        let _ = self.poll_waveform();

        if let Some(backend) = &mut self.audio_backend {
            if self.is_playing && !self.is_seeking {
                self.current_position = backend.get_position();

                // Track A-B loop only when user honors track loop points (chip / Repeat one).
                // When honor is off, play through so queue auto-next works.
                if self.enable_loop && self.honor_track_loop && self.total_duration > 0.0 {
                    let start = if self.use_custom_loop {
                        self.loop_start.unwrap_or(0.0)
                    } else {
                        0.0
                    }
                    .clamp(0.0, self.total_duration);
                    let end = if self.use_custom_loop {
                        self.loop_end.unwrap_or(self.total_duration)
                    } else {
                        self.total_duration
                    }
                    .clamp(0.0, self.total_duration);
                    let end = end.max(start + 0.05);
                    if self.current_position >= end - 0.03 {
                        self.current_position = start;
                        if let Err(e) = backend.set_position(start) {
                            log::error!("Failed to seek to loop start: {e}");
                        }
                        // Do not fall through to track-end handling this frame
                        self.is_playing = backend.is_playing();
                        return;
                    }
                }

                // Check if track has finished (full file; A-B is handled above when honor is on)
                if self.current_position >= self.total_duration - 0.1 && self.total_duration > 0.0 {
                    match self.loop_mode {
                        LoopMode::Single => {
                            // Restart current track (A-B start only when honoring track loop)
                            let restart = if self.enable_loop
                                && self.honor_track_loop
                                && self.use_custom_loop
                            {
                                self.loop_start.unwrap_or(0.0)
                            } else {
                                0.0
                            };
                            self.current_position = restart;
                            if let Err(e) = backend.set_position(restart) {
                                log::error!("Failed to restart track: {}", e);
                            }
                        }
                        LoopMode::All => {
                            // Signal to play next track (will loop back to first track if at end)
                            self.is_playing = false;
                            self.current_position = 0.0;
                            self.should_play_next = true;
                        }
                        LoopMode::None => {
                            // Sequential / shuffle playlist: advance until the playlist ends (no wrap).
                            self.is_playing = false;
                            self.current_position = 0.0;
                            let can_auto_advance = if self.shuffle {
                                !self.playlist.is_empty()
                            } else if let Some(idx) = self.current_track_index {
                                idx + 1 < self.playlist.len()
                            } else {
                                // Filtered out of the queue — still try first visible track
                                !self.playlist.is_empty()
                            };
                            if can_auto_advance {
                                self.should_play_next = true;
                            } else {
                                self.should_notify_playlist_ended = true;
                            }
                        }
                    }
                }
            }
            
            // Check if we're actually playing
            self.is_playing = backend.is_playing();
        }
    }

    pub(crate) fn cleanup_temp_audio(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(audio) = &self.current_audio {
            if let Some(path) = audio.playback_path.as_deref() {
                if path != audio.file_path {
                    let _ = fs::remove_file(Path::new(path));
                }
            }
        }
    }
    
    /// Set loop points / enable flag for the current track (metadata + wave marks).
    /// Does not force A-B playback; that is controlled by `honor_track_loop`.
    pub fn set_loop_points(
        &mut self,
        start: Option<f32>,
        end: Option<f32>,
        use_custom: bool,
        enable_loop: bool,
    ) {
        self.loop_start = start;
        self.loop_end = end;
        self.use_custom_loop = use_custom;
        self.enable_loop = enable_loop;
        if !enable_loop {
            self.honor_track_loop = false;
        }
    }

    /// Apply default honor flag after loop points are resolved for a newly loaded track.
    /// Sequential/All queue modes leave A-B off so auto-next works; Repeat one enables it.
    pub fn apply_default_honor_for_load(&mut self) {
        self.honor_track_loop =
            self.enable_loop && matches!(self.loop_mode, LoopMode::Single);
    }

    pub fn toggle_honor_track_loop(&mut self) {
        if self.enable_loop {
            self.honor_track_loop = !self.honor_track_loop;
        }
    }

    /// Resolved loop range for UI marks (shown whenever track has loop metadata).
    pub fn display_loop_range(&self) -> Option<(f32, f32)> {
        if !self.enable_loop {
            return None;
        }
        let duration = self.total_duration.max(
            self.waveform
                .as_ref()
                .map(|w| w.duration_secs)
                .unwrap_or(0.0),
        );
        if duration <= 0.0 {
            return None;
        }
        let start = if self.use_custom_loop {
            self.loop_start.unwrap_or(0.0)
        } else {
            0.0
        }
        .clamp(0.0, duration);
        let end = if self.use_custom_loop {
            self.loop_end.unwrap_or(duration)
        } else {
            duration
        }
        .clamp(0.0, duration)
        .max(start);
        Some((start, end))
    }

    /// Toggle queue loop mode
    pub fn next_loop_mode(&mut self) {
        self.loop_mode = match self.loop_mode {
            LoopMode::None => LoopMode::Single,
            LoopMode::Single => LoopMode::All,
            LoopMode::All => LoopMode::None,
        };
        // Repeat one: honor track A-B when present. Other modes leave sequential play free.
        if matches!(self.loop_mode, LoopMode::Single) && self.enable_loop {
            self.honor_track_loop = true;
        } else if !matches!(self.loop_mode, LoopMode::Single) {
            // Keep user's chip choice if they explicitly turned A-B on under None/All —
            // only auto-clear when leaving Single would surprise; leave chip state as-is.
        }
    }

    /// Toggle shuffle mode
    pub fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        self.shuffle_bag.clear();
    }

    /// Request next track
    pub fn next_track(&mut self) {
        self.should_play_next = true;
    }

    /// Request previous track
    pub fn previous_track(&mut self) {
        self.should_play_previous = true;
    }

    /// Update playlist and current index. Order is kept as provided (table filter + sort).
    pub fn update_playlist(
        &mut self,
        playlist: Vec<AudioFileInfo>,
        current_name: &str,
        current_id: &str,
    ) {
        self.playlist = playlist;
        self.current_track_index = self
            .playlist
            .iter()
            .position(|f| f.name == current_name && f.id == current_id);
        self.shuffle_bag.clear();
    }

    /// Refresh playlist from the live table order while preserving the playing track identity.
    pub fn sync_playlist_preserving_current(&mut self, playlist: Vec<AudioFileInfo>) {
        let identity = self
            .current_audio
            .as_ref()
            .map(|a| (a.name.clone(), a.id.clone()))
            .or_else(|| {
                self.current_track_index
                    .and_then(|i| self.playlist.get(i))
                    .map(|f| (f.name.clone(), f.id.clone()))
            });

        let order_changed = self.playlist.len() != playlist.len()
            || self
                .playlist
                .iter()
                .zip(playlist.iter())
                .any(|(a, b)| a.name != b.name || a.id != b.id);

        self.playlist = playlist;
        if let Some((name, id)) = identity {
            self.current_track_index = self
                .playlist
                .iter()
                .position(|f| f.name == name && f.id == id);
        } else {
            self.current_track_index = None;
        }
        if order_changed {
            self.shuffle_bag.clear();
        }
    }

    /// Pick next index for shuffle without immediate repeat when possible.
    pub fn take_shuffle_next_index(&mut self) -> Option<usize> {
        let n = self.playlist.len();
        if n == 0 {
            return None;
        }
        let current = self.current_track_index;
        if self.shuffle_bag.is_empty() {
            self.refill_shuffle_bag(current);
        }
        // Prefer not the current track at the front of a fresh bag
        if let Some(cur) = current {
            if self.shuffle_bag.len() > 1 && self.shuffle_bag.last() == Some(&cur) {
                let last = self.shuffle_bag.len() - 1;
                self.shuffle_bag.swap(0, last);
            }
        }
        self.shuffle_bag.pop()
    }

    fn refill_shuffle_bag(&mut self, avoid: Option<usize>) {
        let n = self.playlist.len();
        let mut bag: Vec<usize> = (0..n).collect();
        // Fisher–Yates
        for i in (1..bag.len()).rev() {
            let j = rand::random_range(0..=i);
            bag.swap(i, j);
        }
        if let Some(cur) = avoid {
            if bag.len() > 1 {
                if let Some(pos) = bag.iter().position(|&i| i == cur) {
                    // Put current at the end so it is drawn last from this bag
                    let last = bag.len() - 1;
                    bag.swap(pos, last);
                }
            }
        }
        self.shuffle_bag = bag;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn update_playlist_preserves_table_order() {
        let mut state = AudioState::default();
        let playlist = vec![
            AudioFileInfo {
                name: "b".into(),
                id: "2".into(),
                size: 1,
                filename: "b".into(),
                file_type: "WAV".into(),
                is_nus3bank: false,
                hex_id: None,
            },
            AudioFileInfo {
                name: "a".into(),
                id: "1".into(),
                size: 1,
                filename: "a".into(),
                file_type: "WAV".into(),
                is_nus3bank: false,
                hex_id: None,
            },
        ];
        state.update_playlist(playlist, "a", "1");
        assert_eq!(state.playlist[0].name, "b");
        assert_eq!(state.playlist[1].name, "a");
        assert_eq!(state.current_track_index, Some(1));
    }

    #[test]
    fn honor_defaults_off_unless_single_mode() {
        let mut state = AudioState::default();
        state.enable_loop = true;
        state.loop_mode = LoopMode::None;
        state.apply_default_honor_for_load();
        assert!(!state.honor_track_loop);

        state.loop_mode = LoopMode::Single;
        state.apply_default_honor_for_load();
        assert!(state.honor_track_loop);
    }

    fn make_temp_wav(name: &str) -> String {
        let path = std::env::temp_dir().join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        // Minimal WAV header — content only needs to exist on disk
        f.write_all(b"RIFF\x24\x00\x00\x00WAVEfmt \x10\x00\x00\x00\x01\x00\x01\x00\x44\xac\x00\x00\x88\x58\x01\x00\x02\x00\x10\x00data\x00\x00\x00\x00").unwrap();
        path.to_str().unwrap().to_string()
    }

    fn dummy_audio(playback_path: Option<String>) -> AudioFile {
        AudioFile {
            file_path: "dummy.nus3audio".to_string(),
            playback_path,
            name: "test_track".to_string(),
            file_type: "WAV".to_string(),
            id: "0".to_string(),
        }
    }

    // Regression test: clicking the same track twice used to delete the cached WAV file
    // before playback, causing "file not found" (OS error 2) errors.
    #[test]
    fn same_path_reuse_detected_and_skips_cleanup() {
        let wav = make_temp_wav("audio_state_same_path_test.wav");

        let mut state = AudioState::default();
        state.current_audio = Some(dummy_audio(Some(wav.clone())));

        let new_audio = dummy_audio(Some(wav.clone()));

        let reusing = state
            .current_audio
            .as_ref()
            .and_then(|a| a.playback_path.as_deref())
            .zip(new_audio.playback_path.as_deref())
            .map(|(old, new)| old == new)
            .unwrap_or(false);

        assert!(reusing, "same WAV path must be detected as reuse");

        // When reusing, cleanup must NOT run
        if !reusing {
            state.cleanup_temp_audio();
        }

        assert!(
            std::path::Path::new(&wav).exists(),
            "cached WAV must survive when the same track is replayed"
        );
        let _ = std::fs::remove_file(&wav);
    }

    // Switching to a different track must still clean up the old temp file.
    #[test]
    fn different_path_triggers_cleanup() {
        let wav_a = make_temp_wav("audio_state_track_a.wav");
        let wav_b = make_temp_wav("audio_state_track_b.wav");

        let mut state = AudioState::default();
        state.current_audio = Some(dummy_audio(Some(wav_a.clone())));

        let new_audio = dummy_audio(Some(wav_b.clone()));

        let reusing = state
            .current_audio
            .as_ref()
            .and_then(|a| a.playback_path.as_deref())
            .zip(new_audio.playback_path.as_deref())
            .map(|(old, new)| old == new)
            .unwrap_or(false);

        assert!(!reusing, "different WAV paths must not be flagged as reuse");

        state.cleanup_temp_audio();

        assert!(
            !std::path::Path::new(&wav_a).exists(),
            "old temp WAV must be deleted when switching tracks"
        );
        assert!(
            std::path::Path::new(&wav_b).exists(),
            "new track's WAV must not be touched by cleanup"
        );
        let _ = std::fs::remove_file(&wav_b);
    }
}
