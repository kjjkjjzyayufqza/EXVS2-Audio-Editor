// Main area module components
mod audio_file_info;
mod search_column;
mod table_renderer;
mod export_utils;
mod main_component;

// Re-export the main struct
pub use main_component::MainArea;
pub use audio_file_info::AudioFileInfo;
pub use search_column::SearchColumn;
