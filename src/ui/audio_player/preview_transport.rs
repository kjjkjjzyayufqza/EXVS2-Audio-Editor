//! Independent preview transport for add/replace dialogs.
//!
//! Gain changes only update output level. Play/pause of a loaded buffer uses
//! pause/resume — never a second `from_file` / decode.

use super::audio_backend::{AudioBackend, PlatformAudioBackend};
use super::gain::effective_linear_level;

/// Dedicated preview player (independent of the main bottom player).
pub struct PreviewTransport {
    backend: Box<dyn AudioBackend>,
    path: String,
    is_playing: bool,
    position: f32,
    duration: f32,
    /// User volume slider (0.0–1.0), independent of import gain.
    volume: f32,
    /// Import gain preview in dB (applied live on top of `volume`).
    gain_db: f32,
    loaded: bool,
    last_scrub_seek: Option<std::time::Instant>,
    last_scrub_pos: f32,
}

impl PreviewTransport {
    pub fn new() -> Self {
        let mut backend = PlatformAudioBackend::new();
        if let Err(e) = backend.init() {
            log::error!("Failed to init preview backend: {e}");
        }
        Self::with_backend(Box::new(backend))
    }

    pub(crate) fn with_backend(backend: Box<dyn AudioBackend>) -> Self {
        Self {
            backend,
            path: String::new(),
            is_playing: false,
            position: 0.0,
            duration: 0.0,
            volume: 0.8,
            gain_db: 0.0,
            loaded: false,
            last_scrub_seek: None,
            last_scrub_pos: f32::NAN,
        }
    }

    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    #[must_use]
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    #[must_use]
    pub fn position(&self) -> f32 {
        self.position
    }

    #[must_use]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Use a duration from waveform peaks when the backend reported 0.
    pub fn set_duration_if_unknown(&mut self, duration: f32) {
        if self.duration <= 0.0 && duration > 0.0 {
            self.duration = duration;
        }
    }

    #[must_use]
    pub fn volume(&self) -> f32 {
        self.volume
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn gain_db(&self) -> f32 {
        self.gain_db
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Linear amplitude = user volume × 10^(gain_db/20), clamped to the output cap.
    #[must_use]
    pub fn effective_linear(&self) -> f32 {
        effective_linear_level(self.volume, self.gain_db)
    }

    fn apply_output_level(&mut self) {
        if let Err(e) = self.backend.set_volume(self.effective_linear()) {
            log::warn!("Preview output level: {e}");
        }
    }

    /// Decode once and pause at 0 so Play can resume without another decode.
    pub fn load(&mut self, path: &str) -> Result<(), String> {
        self.stop();
        self.path = path.to_owned();
        self.gain_db = 0.0;
        self.apply_output_level();
        self.backend.play_audio(path)?;
        self.duration = self.backend.get_duration().max(0.0);
        self.position = 0.0;
        self.loaded = true;
        let _ = self.backend.pause();
        self.is_playing = false;
        let _ = self.backend.set_position(0.0);
        self.apply_output_level();
        let decoded_duration = self.backend.get_duration();
        if decoded_duration > 0.0 {
            self.duration = decoded_duration;
        }
        if self.duration <= 0.0 {
            log::warn!("Preview duration is 0 for {path}; playhead may be unreliable");
        }
        Ok(())
    }

    pub fn toggle_play(&mut self) {
        if !self.loaded {
            return;
        }
        if self.is_playing {
            if let Err(e) = self.backend.pause() {
                log::warn!("Preview pause: {e}");
            }
            self.is_playing = false;
            self.position = self.backend.get_position();
        } else {
            self.start_playback();
        }
    }

    fn start_playback(&mut self) {
        if !self.loaded {
            return;
        }
        if self.position >= self.duration - 0.05 && self.duration > 0.0 {
            let _ = self.backend.set_position(0.0);
            self.position = 0.0;
        }
        let started = if self.backend.is_loaded() {
            self.backend.resume()
        } else if self.path.is_empty() {
            Err("No preview path".to_owned())
        } else {
            // Explicit stop unloaded the handle; replay from the same path.
            // Native backend keeps the decoded buffer and must not hit from_file again.
            self.backend.play_audio(&self.path)
        };
        if let Err(e) = started {
            log::error!("Preview play failed: {e}");
            return;
        }
        self.apply_output_level();
        if self.position > 0.0 {
            let _ = self.backend.set_position(self.position);
        }
        self.is_playing = true;
    }

    /// Start playback if idle so a live gain tweak is audible, without decoding.
    fn ensure_audible(&mut self) {
        if self.loaded && !self.is_playing && self.backend.is_loaded() {
            self.start_playback();
        }
    }

    /// Unload the playing handle (modal close). Path is cleared.
    pub fn stop(&mut self) {
        let _ = self.backend.stop();
        self.is_playing = false;
        self.position = 0.0;
        self.loaded = false;
        self.path.clear();
    }

    /// Pause and rewind without unloading, so the next play resumes the buffer.
    ///
    /// A finished (Stopped) clip cannot be seeked/resumed; replay from the cached path.
    pub fn stop_to_start(&mut self) {
        if !self.loaded {
            return;
        }
        if self.backend.is_loaded() {
            let _ = self.backend.pause();
            let _ = self.backend.set_position(0.0);
        } else if !self.path.is_empty() && self.backend.play_audio(&self.path).is_ok() {
            let _ = self.backend.pause();
            let _ = self.backend.set_position(0.0);
            self.apply_output_level();
        }
        self.is_playing = false;
        self.position = 0.0;
    }

    pub fn seek(&mut self, t: f32) {
        if !self.loaded {
            return;
        }
        let t = if self.duration > 0.0 {
            t.clamp(0.0, self.duration)
        } else {
            t.max(0.0)
        };
        self.position = t;
        self.last_scrub_pos = t;
        self.last_scrub_seek = Some(std::time::Instant::now());
        if let Err(e) = self.backend.set_position(t) {
            log::warn!("Preview seek: {e}");
        }
    }

    /// Drag scrub: update UI always; backend at most ~12 Hz and only if position moved.
    pub fn scrub(&mut self, t: f32) {
        if !self.loaded {
            return;
        }
        let t = if self.duration > 0.0 {
            t.clamp(0.0, self.duration)
        } else {
            t.max(0.0)
        };
        self.position = t;

        if self.last_scrub_pos.is_finite() && (t - self.last_scrub_pos).abs() < 0.04 {
            return;
        }
        const MIN_INTERVAL: std::time::Duration = std::time::Duration::from_millis(80);
        if let Some(prev) = self.last_scrub_seek
            && prev.elapsed() < MIN_INTERVAL
        {
            return;
        }
        self.last_scrub_pos = t;
        self.last_scrub_seek = Some(std::time::Instant::now());
        if let Err(e) = self.backend.set_position(t) {
            log::warn!("Preview scrub: {e}");
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.apply_output_level();
    }

    /// Live gain preview (dB). Updates output immediately. Does not decode or unload.
    pub fn set_gain_db(&mut self, gain_db: f32) {
        self.gain_db = gain_db.clamp(-24.0, 24.0);
        self.apply_output_level();
        self.ensure_audible();
    }

    pub fn tick(&mut self, loop_start: Option<f32>, loop_end: Option<f32>, use_custom: bool) {
        if !self.loaded {
            return;
        }
        if self.is_playing {
            self.position = self.backend.get_position();
            self.is_playing = self.backend.is_playing();

            if use_custom && self.duration > 0.0 {
                let start = loop_start.unwrap_or(0.0).clamp(0.0, self.duration);
                let end = loop_end
                    .unwrap_or(self.duration)
                    .clamp(0.0, self.duration)
                    .max(start + 0.05);
                if self.position >= end - 0.03 {
                    self.seek(start);
                    if !self.is_playing {
                        let _ = self.backend.resume();
                        self.is_playing = true;
                    }
                }
            } else if self.duration > 0.0 && self.position >= self.duration - 0.05 {
                self.is_playing = false;
                self.position = self.duration;
            }
        } else {
            self.position = self.backend.get_position();
        }
    }
}

impl Default for PreviewTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::PreviewTransport;
    use crate::ui::audio_player::audio_backend::recording::{CallLog, RecordingAudioBackend};
    use crate::ui::audio_player::gain::effective_linear_level;

    fn loaded_preview(path: &str) -> (PreviewTransport, CallLog) {
        let log = CallLog::new();
        let backend = RecordingAudioBackend::new(log.clone());
        let mut preview = PreviewTransport::with_backend(Box::new(backend));
        preview
            .load(path)
            .expect("recording backend load must succeed");
        (preview, log)
    }

    #[test]
    fn transport_gain_does_not_unload_or_reload() {
        let (mut preview, log) = loaded_preview("dummy_preview.wav");
        let path_before = preview.path().to_owned();
        assert!(
            preview.is_loaded(),
            "preview must be loaded before changing gain"
        );
        assert_eq!(
            log.play_audio_count(),
            1,
            "load must decode the source once"
        );

        preview.set_gain_db(6.0);

        assert!(
            preview.is_loaded(),
            "applying gain_db must not clear the loaded flag"
        );
        assert_eq!(
            preview.path(),
            path_before.as_str(),
            "applying gain_db must not change the preview path"
        );
        assert_eq!(
            log.play_audio_count(),
            1,
            "applying gain_db must not invoke a file decode/encode"
        );
        let expected = effective_linear_level(preview.volume(), 6.0);
        let last = log
            .last_volume()
            .expect("set_gain_db must push an output level");
        assert!(
            (last - expected).abs() < 1e-6,
            "gain must set output level to volume * 10^(dB/20); got {last}, expected {expected}"
        );
    }

    #[test]
    fn transport_preview_pause_then_resume_does_not_reload() {
        let (mut preview, log) = loaded_preview("dummy_preview.wav");
        assert_eq!(log.play_audio_count(), 1, "load plays once then pauses");

        preview.toggle_play();
        assert!(
            preview.is_playing(),
            "first play must resume the loaded buffer"
        );
        assert_eq!(
            log.play_audio_count(),
            1,
            "play of a loaded buffer must not call play_audio/from_file again"
        );
        assert!(
            log.resume_count() >= 1,
            "play of a loaded buffer must resume"
        );

        preview.toggle_play();
        assert!(!preview.is_playing(), "second toggle must pause");
        assert_eq!(log.play_audio_count(), 1, "pause must not reload from disk");

        preview.toggle_play();
        assert!(preview.is_playing(), "third toggle must resume");
        assert_eq!(
            log.play_audio_count(),
            1,
            "pause then resume must keep the loaded handle without a second from_file"
        );
        let paths = log.play_audio_paths();
        assert_eq!(
            paths.as_slice(),
            &["dummy_preview.wav".to_owned()],
            "only the original load path may be passed to play_audio"
        );
    }

    #[test]
    fn transport_preview_restart_after_end_uses_play_audio() {
        let (mut preview, log) = loaded_preview("dummy_preview.wav");
        preview.toggle_play();
        assert!(preview.is_playing(), "preview must start");
        let plays_before_end = log.play_audio_count();
        let resumes_before_end = log.resume_count();

        preview.seek(preview.duration());
        preview.tick(None, None, false);
        assert!(
            !preview.is_playing(),
            "preview must stop when the playhead reaches duration"
        );

        preview.toggle_play();
        assert!(
            preview.is_playing(),
            "Play after the clip ends must start a new instance"
        );
        assert_eq!(
            log.play_audio_count(),
            plays_before_end + 1,
            "finished preview clips must replay via play_audio (cached path), not resume"
        );
        assert_eq!(
            log.resume_count(),
            resumes_before_end,
            "restart-after-end must not resume a Stopped handle"
        );
    }

    #[test]
    fn transport_preview_stop_to_start_after_end_replays() {
        let (mut preview, log) = loaded_preview("dummy_preview.wav");
        preview.toggle_play();
        preview.seek(preview.duration());
        preview.tick(None, None, false);
        assert!(!preview.is_playing(), "clip must have finished");
        let plays_at_end = log.play_audio_count();

        preview.stop_to_start();
        assert!(
            !preview.is_playing(),
            "stop_to_start must leave playback paused"
        );
        assert_eq!(
            log.play_audio_count(),
            plays_at_end + 1,
            "stop_to_start after Stopped must start a cached instance (pause at 0)"
        );

        let resumes_before_play = log.resume_count();
        preview.toggle_play();
        assert!(
            preview.is_playing(),
            "Play after stop_to_start must resume the replayed buffer"
        );
        assert_eq!(
            log.play_audio_count(),
            plays_at_end + 1,
            "the following Play must resume, not decode again"
        );
        assert!(
            log.resume_count() > resumes_before_play,
            "Play after stop_to_start on a finished clip should resume the new instance"
        );
    }
}
