//! In-memory [`AudioBackend`] for tests. Does not open a DAC or decode files.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use super::AudioBackend;

/// Shared counters so tests can inspect a boxed backend.
#[derive(Clone, Debug)]
pub(crate) struct CallLog {
    play_audio: Arc<Mutex<Vec<String>>>,
    pause: Arc<AtomicU32>,
    resume: Arc<AtomicU32>,
    stop: Arc<AtomicU32>,
    set_volume: Arc<Mutex<Vec<f32>>>,
}

impl CallLog {
    pub(crate) fn new() -> Self {
        Self {
            play_audio: Arc::new(Mutex::new(Vec::new())),
            pause: Arc::new(AtomicU32::new(0)),
            resume: Arc::new(AtomicU32::new(0)),
            stop: Arc::new(AtomicU32::new(0)),
            set_volume: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub(crate) fn play_audio_count(&self) -> usize {
        self.play_audio.lock().expect("play_audio log").len()
    }

    pub(crate) fn play_audio_paths(&self) -> Vec<String> {
        self.play_audio.lock().expect("play_audio log").clone()
    }

    pub(crate) fn pause_count(&self) -> u32 {
        self.pause.load(Ordering::SeqCst)
    }

    pub(crate) fn resume_count(&self) -> u32 {
        self.resume.load(Ordering::SeqCst)
    }

    pub(crate) fn last_volume(&self) -> Option<f32> {
        self.set_volume
            .lock()
            .expect("set_volume log")
            .last()
            .copied()
    }
}

/// Fake device: play_audio marks a buffer loaded; pause/resume never decode.
///
/// Reaching the clip end (kira `Stopped`) is not resumable until `play_audio`.
#[derive(Debug)]
pub(crate) struct RecordingAudioBackend {
    log: CallLog,
    loaded: bool,
    playing: bool,
    /// True after the playhead hits the end; resume is a no-op until play_audio.
    finished: bool,
    position: f32,
    duration: f32,
    volume: f32,
}

impl RecordingAudioBackend {
    pub(crate) fn new(log: CallLog) -> Self {
        Self {
            log,
            loaded: false,
            playing: false,
            finished: false,
            position: 0.0,
            duration: 1.0,
            volume: 1.0,
        }
    }
}

impl AudioBackend for RecordingAudioBackend {
    fn init(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn play_audio(&mut self, file_path: &str) -> Result<(), String> {
        self.log
            .play_audio
            .lock()
            .expect("play_audio log")
            .push(file_path.to_owned());
        self.loaded = true;
        self.playing = true;
        self.finished = false;
        self.position = 0.0;
        self.duration = 1.0;
        Ok(())
    }

    fn pause(&mut self) -> Result<(), String> {
        self.log.pause.fetch_add(1, Ordering::SeqCst);
        if !self.loaded {
            return Err("No audio playing".to_owned());
        }
        self.playing = false;
        Ok(())
    }

    fn resume(&mut self) -> Result<(), String> {
        self.log.resume.fetch_add(1, Ordering::SeqCst);
        if !self.loaded || self.finished {
            return Err("No audio loaded".to_owned());
        }
        self.playing = true;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        self.log.stop.fetch_add(1, Ordering::SeqCst);
        if !self.loaded {
            return Err("No audio playing".to_owned());
        }
        self.loaded = false;
        self.playing = false;
        self.finished = true;
        self.position = 0.0;
        Ok(())
    }

    fn set_position(&mut self, position_secs: f32) -> Result<(), String> {
        if !self.loaded {
            return Err("No audio loaded".to_owned());
        }
        self.position = if self.duration > 0.0 {
            position_secs.clamp(0.0, self.duration)
        } else {
            position_secs.max(0.0)
        };
        // Hitting the end is kira Stopped: seek-to-start does not revive the handle.
        if self.duration > 0.0 && self.position >= self.duration - 0.05 {
            self.finished = true;
            self.playing = false;
        }
        Ok(())
    }

    fn set_volume(&mut self, volume: f32) -> Result<(), String> {
        self.volume = volume;
        self.log
            .set_volume
            .lock()
            .expect("set_volume log")
            .push(volume);
        Ok(())
    }

    fn is_playing(&self) -> bool {
        self.playing && !self.finished
    }

    fn get_position(&self) -> f32 {
        self.position
    }

    fn get_duration(&self) -> f32 {
        self.duration
    }

    fn is_available(&self) -> bool {
        true
    }

    fn is_loaded(&self) -> bool {
        self.loaded && !self.finished
    }
}
