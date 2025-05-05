// Main area module components
mod audio_file_info;
mod search_column;
mod table_renderer;
mod export_utils;
mod replace_utils;
mod loop_settings_modal;

// New modular components
mod sort_column;
mod toast_message;
mod main_area_core;
mod main_area_filtering;
mod main_area_rendering;
mod main_area_search;
mod main_area_output;
mod main_area_table;
mod main_component;

// Re-export the main struct
pub use main_component::MainArea;
pub use audio_file_info::AudioFileInfo;
pub use replace_utils::ReplaceUtils;
pub use export_utils::ExportUtils;
pub use loop_settings_modal::{LoopSettingsModal, LoopSettings};
