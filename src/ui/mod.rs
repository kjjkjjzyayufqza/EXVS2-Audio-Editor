// UI component modules
mod top_panel;
mod file_list;
pub mod main_area;  // Make this public

pub use top_panel::TopPanel;
pub use file_list::FileList;
pub use main_area::MainArea;
pub use main_area::AudioFileInfo;
pub use main_area::SearchColumn;
