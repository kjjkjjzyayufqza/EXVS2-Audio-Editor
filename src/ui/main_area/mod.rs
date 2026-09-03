// Main area module components
mod add_audio_modal;
mod add_audio_utils;
mod audio_file_info;
mod confirm_modal;
mod dton_pending;
mod dton_tones_modal;
mod export_utils;
mod grp_list_modal;
mod grp_pending;
mod grp_template;
mod loop_settings_modal;
mod nus3audio_file_utils;
mod prop_edit_modal;
mod prop_pending;
mod replace_utils;
mod search_column;
mod table_renderer;

// New modular components
mod main_area_core;
mod main_area_filtering;
mod main_area_output;
mod main_area_rendering;
mod main_area_search;
mod main_area_table;
mod main_component;
mod sort_column;
mod toast_message;

// Re-export the main struct
pub use audio_file_info::AudioFileInfo;
pub use confirm_modal::ConfirmModal;
pub use export_utils::ExportUtils;
pub use main_component::MainArea;
pub use nus3audio_file_utils::Nus3audioFileUtils;
pub use replace_utils::ReplaceUtils;
