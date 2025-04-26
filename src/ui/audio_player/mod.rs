// Audio player module components
mod audio_player_component;
mod audio_controls;
mod audio_state;
mod audio_backend;

// Re-export the main components
pub use audio_player_component::AudioPlayer;
pub use audio_state::AudioState;
pub use audio_backend::{AudioBackend, PlatformAudioBackend, NativeAudioBackend, WebAudioBackend};
