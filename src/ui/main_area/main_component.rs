use egui::{
    Color32, Context, Frame, Rect, Rounding, Stroke, Ui, Vec2,
};
use nus3audio::Nus3audioFile;
use std::collections::HashSet;

use super::audio_file_info::AudioFileInfo;
use super::export_utils::ExportUtils;
use super::search_column::SearchColumn;
use super::table_renderer::TableRenderer;
use crate::ui::audio_player::AudioPlayer;

/// Main editing area component
pub struct MainArea {
    pub selected_file: Option<String>,
    pub file_count: Option<usize>,
    pub audio_files: Option<Vec<AudioFileInfo>>,
    pub error_message: Option<String>,
    // Table configuration
    pub striped: bool,
    pub resizable: bool,
    pub clickable: bool,
    // Set of selected row indices
    pub selected_rows: HashSet<usize>,
    // Whether to display table grid lines
    pub show_grid_lines: bool,
    // Search functionality
    pub search_query: String,
    pub search_column: SearchColumn,
    pub show_advanced_search: bool,
    // Audio player
    pub audio_player: Option<AudioPlayer>,
}

impl MainArea {
    /// Create a new main area
    pub fn new() -> Self {
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
            // Create new audio player
            audio_player: Some(AudioPlayer::new()),
        }
    }
    
    /// Get filtered audio files based on search query and column
    fn filtered_audio_files(&self) -> Vec<AudioFileInfo> {
        if let Some(audio_files) = &self.audio_files {
            if self.search_query.is_empty() {
                // If no search query, return all audio files
                return audio_files.clone();
            }
            
            // Filter audio files based on search query and selected column
            let query = self.search_query.to_lowercase();
            audio_files
                .iter()
                .filter(|file| {
                    match self.search_column {
                        SearchColumn::All => {
                            file.name.to_lowercase().contains(&query) ||
                            file.id.to_lowercase().contains(&query) ||
                            self.size_matches(file.size, &query) ||
                            file.filename.to_lowercase().contains(&query) ||
                            file.file_type.to_lowercase().contains(&query)
                        },
                        SearchColumn::Name => file.name.to_lowercase().contains(&query),
                        SearchColumn::Id => file.id.to_lowercase().contains(&query),
                        SearchColumn::Size => self.size_matches(file.size, &query),
                        SearchColumn::Filename => file.filename.to_lowercase().contains(&query),
                        SearchColumn::Type => file.file_type.to_lowercase().contains(&query),
                    }
                })
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Helper function to match size values in different formats
    fn size_matches(&self, size: usize, query: &str) -> bool {
        // Convert size to different formats for more flexible searching
        let size_bytes = format!("{} B", size).to_lowercase();
        let size_kb = format!("{:.1} KB", size as f32 / 1024.0).to_lowercase();
        let size_mb = format!("{:.1} MB", size as f32 / (1024.0 * 1024.0)).to_lowercase();
        
        // Also check raw size value as string
        let size_raw = size.to_string();
        
        size_bytes.contains(query) ||
        size_kb.contains(query) || 
        size_mb.contains(query) ||
        size_raw.contains(query)
    }

    /// Update the selected file and load NUS3AUDIO info if applicable
    pub fn update_selected_file(&mut self, file_path: Option<String>) {
        self.selected_file = file_path;
        self.file_count = None;
        self.audio_files = None;
        self.error_message = None;

        // If file is selected and it's a NUS3AUDIO file, load its info
        if let Some(file_name) = &self.selected_file {
            if file_name.to_lowercase().ends_with(".nus3audio")
                || file_name.to_lowercase().ends_with(".nus3bank")
            {
                match Nus3audioFile::open(file_name) {
                    Ok(nus3_file) => {
                        // Store file count
                        self.file_count = Some(nus3_file.files.len());

                        // Extract structured file info
                        let mut audio_files = Vec::new();

                        for audio_file in nus3_file.files.iter() {
                            // Attempt to detect file type based on header
                            let file_type = if audio_file.data.len() >= 4 {
                                match &audio_file.data[..4] {
                                    b"OPUS" => "OPUS Audio",
                                    b"IDSP" => "IDSP Audio",
                                    _ => "Audio",
                                }
                            } else {
                                "Audio"
                            };

                            audio_files.push(AudioFileInfo {
                                name: audio_file.name.clone(),
                                id: audio_file.id.to_string(),
                                size: audio_file.data.len(),
                                filename: audio_file.filename(),
                                file_type: file_type.to_string(),
                            });
                        }

                        self.audio_files = Some(audio_files);
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Error loading NUS3AUDIO file: {}", e));
                    }
                }
            }
        }
    }

    /// Display the main editing area
    pub fn show(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render(ui);
        });
    }

    /// Render the main area content
    pub fn render(&mut self, ui: &mut Ui) {
        let available_height = ui.available_height();
        let available_width = ui.available_width();

        ui.vertical_centered(|ui| {
            ui.add_space(10.0); // Reduced space to allow more content

            ui.heading("Audio Editor");

            if let Some(selected) = &self.selected_file {
                // Display filename with ellipsis if too long
                let display_name = if selected.len() > 60 {
                    format!(
                        "{}...{}",
                        &selected[0..30],
                        &selected[selected.len() - 27..]
                    )
                } else {
                    selected.clone()
                };

                ui.label(format!("Currently editing: {}", display_name))
                    .on_hover_text(selected);

                ui.add_space(10.0); // Reduced space
                ui.heading("NUS3AUDIO Info Display");

                // Display file info if available
                if let Some(audio_files) = &self.audio_files {
                    ui.add_space(10.0);

                    // Display file count
                    if let Some(count) = self.file_count {
                        ui.label(format!("Number of audio files: {}", count));
                        ui.add_space(5.0);
                    }

                    // Add search box before the table
                    self.render_search_box(ui);
                    ui.add_space(10.0);

                    // Get filtered audio files
                    let filtered_audio_files = self.filtered_audio_files();
                    let files_count = filtered_audio_files.len();
                    let striped = self.striped;
                    let clickable = self.clickable;
                    let show_grid_lines = self.show_grid_lines;

                    // Render the table with audio files
                    self.render_audio_table(
                        ui, 
                        filtered_audio_files, 
                        files_count, 
                        available_height, 
                        available_width
                    );
                } else if let Some(error) = &self.error_message {
                    ui.add_space(10.0);
                    ui.colored_label(Color32::RED, error);
                } else {
                    let rect = Rect::from_min_size(
                        ui.cursor().min,
                        Vec2::new(ui.available_width(), 200.0),
                    );
                    ui.painter()
                        .rect_filled(rect, 4.0, Color32::from_rgb(80, 80, 80));
                    ui.add_space(200.0); // Add space to account for the rect

                    if selected.to_lowercase().ends_with(".nus3audio")
                        || selected.to_lowercase().ends_with(".nus3bank")
                    {
                        ui.label("Loading NUS3AUDIO file info...");
                    } else {
                        ui.label("Selected file is not a NUS3AUDIO file.");
                    }
                }
            } else {
                ui.label("Please select a file from the list on the left to edit");
            }
        });
    }
    
    /// Render search box
    fn render_search_box(&mut self, ui: &mut Ui) {
        Frame::group(ui.style())
            .stroke(Stroke::new(1.0, ui.visuals().widgets.active.bg_fill))
            .rounding(Rounding::same(5))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Search");
                    
                    // Toggle advanced search
                    if ui.button(if self.show_advanced_search { "▲ Basic" } else { "▼ Advanced" }).clicked() {
                        self.show_advanced_search = !self.show_advanced_search;
                    }
                });
                ui.add_space(5.0);
                
                // Basic search - always visible
                ui.horizontal(|ui| {
                    ui.label("Query:");
                    if ui.text_edit_singleline(&mut self.search_query).changed() {
                        // Search query changed - will be applied automatically
                    }
                    if !self.search_query.is_empty() && ui.button("✖").clicked() {
                        self.search_query.clear();
                    }
                });
                
                // Advanced search options
                if self.show_advanced_search {
                    ui.add_space(5.0);
                    
                    // Column selection
                    ui.horizontal(|ui| {
                        ui.label("Search in:");
                        egui::ComboBox::from_id_source("search_column")
                            .selected_text(self.search_column.display_name())
                            .show_ui(ui, |ui| {
                                for column in SearchColumn::all_columns() {
                                    ui.selectable_value(
                                        &mut self.search_column,
                                        column,
                                        column.display_name()
                                    );
                                }
                            });
                    });
                    
                    // Search tips
                    ui.add_space(5.0);
                    ui.small("Tip: For size column, you can search by 'KB', 'MB', etc.");
                }
            });
    }
    
    /// Render the audio file table
    fn render_audio_table(&mut self, ui: &mut Ui, filtered_audio_files: Vec<AudioFileInfo>, files_count: usize, available_height: f32, available_width: f32) {
        // Add a nice border to the table
        Frame::group(ui.style())
            .stroke(Stroke::new(1.0, ui.visuals().faint_bg_color))
            .rounding(Rounding::same(4))
            .show(ui, |ui| {
                // Manually add margins
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        // Table title and information
                        ui.horizontal(|ui| {
                            ui.heading("Audio File List");
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if !self.search_query.is_empty() {
                                        ui.label(format!("Found: {} / {} files", files_count, self.file_count.unwrap_or(0)));
                                    } else {
                                        ui.label(format!("Total: {} files", files_count));
                                    }
                                },
                            );
                        });

                        ui.add_space(5.0);
                        // Show message if no files match the search query
                        if !self.search_query.is_empty() && filtered_audio_files.is_empty() {
                            ui.add_space(8.0); // Adjusted for better spacing
                            ui.label("No audio files match the search criteria.");
                        }
                        
                        // Variables to store action indices
                        let mut export_index = None;
                        let mut play_index = None;
                        
                        // Use table renderer to render the table
                        TableRenderer::render_table(
                            ui,
                            &filtered_audio_files,
                            &mut self.selected_rows,
                            self.striped,
                            self.clickable,
                            self.show_grid_lines,
                            available_height - 100.0, // Adjusted for header and spacing
                            available_width - 16.0,  // Adjusted for padding
                            &mut |index| {
                                export_index = Some(index);
                            },
                            &mut |index| {
                                play_index = Some(index);
                            },
                        );
                        
                        // Process export if needed
                        if let Some(idx) = export_index {
                            self.handle_export(ui, &filtered_audio_files, idx);
                        }
                        
                        // Process play if needed
                        if let Some(idx) = play_index {
                            self.handle_play_audio(ui, &filtered_audio_files, idx);
                        }
                        
                        ui.add_space(8.0);
                    });
                    ui.add_space(8.0);
                });
                ui.add_space(8.0);
            });
    }
    
    /// Handle exporting an audio file
    fn handle_export(&self, ui: &mut Ui, filtered_audio_files: &[AudioFileInfo], index: usize) {
        if index < filtered_audio_files.len() {
            let audio_info = &filtered_audio_files[index];
            
            // Get the selected file path
            if let Some(file_path) = &self.selected_file {
                // Use the ExportUtils to export the file
                match ExportUtils::export_to_wav(audio_info, file_path) {
                    Ok(path) => {
                        ui.add_space(8.0);
                        ui.colored_label(Color32::GREEN, 
                            format!("Successfully exported to: {}", path));
                    },
                    Err(e) => {
                        ui.add_space(8.0);
                        ui.colored_label(Color32::RED, e);
                    }
                }
            } else {
                ui.add_space(8.0);
                ui.colored_label(Color32::RED, "No file selected");
            }
        } else {
            ui.add_space(8.0);
            ui.colored_label(Color32::RED, "Invalid audio file index");
        }
    }
    
    /// Handle playing an audio file
    pub fn handle_play_audio(&mut self, ui: &mut Ui, filtered_audio_files: &[AudioFileInfo], index: usize) {
        if index < filtered_audio_files.len() {
            let audio_info = &filtered_audio_files[index];
            
            // Get the selected file path
            if let Some(file_path) = &self.selected_file {
                // Try to load and play the audio
                if let Some(audio_player) = &mut self.audio_player {
                    match audio_player.load_audio(audio_info, &file_path) {
                        Ok(()) => {
                            ui.add_space(8.0);
                            ui.colored_label(Color32::GREEN, 
                                format!("Now playing: {}", audio_info.name));
                            
                            // Start playing
                            let state = audio_player.get_audio_state();
                            let mut state = state.lock().unwrap();
                            if !state.is_playing {
                                state.toggle_play();
                            }
                        },
                        Err(e) => {
                            ui.add_space(8.0);
                            ui.colored_label(Color32::RED, format!("Failed to load audio: {}", e));
                        }
                    }
                } else {
                    ui.add_space(8.0);
                    ui.colored_label(Color32::RED, "Audio player is not initialized");
                }
            } else {
                ui.add_space(8.0);
                ui.colored_label(Color32::RED, "No file selected");
            }
        } else {
            ui.add_space(8.0);
            ui.colored_label(Color32::RED, "Invalid audio file index");
        }
    }
}
