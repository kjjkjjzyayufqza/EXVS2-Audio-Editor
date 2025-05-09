use std::collections::HashSet;
use egui::Color32;

use crate::ui::audio_player::AudioPlayer;
use super::{
    sort_column::SortColumn, 
    search_column::SearchColumn, 
    audio_file_info::AudioFileInfo,
    toast_message::ToastMessage
};

/// Main editing area component
#[derive(serde::Deserialize, serde::Serialize)]
pub struct MainArea {
    #[serde(skip)]
    pub selected_file: Option<String>,
    #[serde(skip)]
    pub file_count: Option<usize>,
    #[serde(skip)]
    pub audio_files: Option<Vec<AudioFileInfo>>,
    #[serde(skip)]
    pub error_message: Option<String>,
    // Table configuration
    pub striped: bool,
    pub resizable: bool,
    pub clickable: bool,
    #[serde(skip)]
    pub selected_rows: HashSet<usize>,
    // Whether to display table grid lines
    pub show_grid_lines: bool,
    // Search functionality
    #[serde(skip)]
    pub search_query: String,
    pub search_column: SearchColumn,
    pub show_advanced_search: bool,
    // Sorting functionality
    pub sort_column: SortColumn,
    pub sort_ascending: bool,
    // Audio player
    #[serde(skip)]
    pub audio_player: Option<AudioPlayer>,
    // Output path configuration
    pub output_path: Option<String>,
    // Toast notifications
    #[serde(skip)]
    pub(crate) toast_messages: Vec<ToastMessage>,
}

impl MainArea {
    /// Create a new main area
    pub fn new() -> Self {
        println!("Creating new MainArea instance");
        
        Self {
            selected_file: None,
            file_count: None,
            audio_files: None,
            error_message: None,
            // Set default table style
            striped: true,
            resizable: true,
            clickable: true,
            selected_rows: HashSet::new(),
            show_grid_lines: false,
            // Initialize search query as empty
            search_query: String::new(),
            search_column: SearchColumn::All,
            show_advanced_search: false,
            // Initialize with no sorting
            sort_column: SortColumn::None,
            sort_ascending: true,
            // Create new audio player
            audio_player: Some(AudioPlayer::new()),
            // Initialize output path as None
            output_path: None,
            // Initialize toast messages
            toast_messages: Vec::new(),
        }
    }
    
    /// Add a toast notification
    pub fn add_toast(&mut self, message: String, color: Color32) {
        let toast = ToastMessage::new(message, color, 3); // Display for 3 seconds
        self.toast_messages.push(toast);
    }
    
    /// Ensure that the audio player is initialized
    /// This is called after deserialization to make sure audio player is recreated
    pub fn ensure_audio_player_initialized(&mut self) {
        println!("Ensuring audio player is initialized");
        if self.audio_player.is_none() {
            println!("Audio player was None, creating new instance");
            self.audio_player = Some(AudioPlayer::new());
        } else {
            println!("Audio player was already initialized");
        }
    }
}
