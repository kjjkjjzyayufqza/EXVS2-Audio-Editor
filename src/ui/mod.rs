// UI component modules
pub mod audio_player; // Audio player module
mod file_list;
pub mod main_area; // Make this public
pub mod shortcuts;
mod top_panel;
pub mod update_log; // Version update notice + history
pub mod waveform; // Sound-wave visualization

pub use file_list::FileList;
pub use main_area::MainArea;
pub use top_panel::TopPanel;
