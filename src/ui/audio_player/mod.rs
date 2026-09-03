// Audio player module components
pub mod audio_backend;
mod audio_controls;
mod audio_player_component;
mod audio_state;
mod gain;
mod preview_transport;

// Re-export the main components
pub use audio_player_component::{AudioPlayer, AudioPlayerAction};
pub use audio_state::{AudioPlayerSettings, LoopMode};
pub use preview_transport::PreviewTransport;
