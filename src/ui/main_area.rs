use egui::{
    Align, Button, Color32, Context, Frame, Grid, Rect, RichText, Rounding, ScrollArea, Stroke, 
    TextWrapMode, Ui, Vec2,
};
use nus3audio::Nus3audioFile;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Structure to hold audio file information
#[derive(Clone)]
pub struct AudioFileInfo {
    pub name: String,
    pub id: String,
    pub size: usize,
    pub filename: String,
    pub file_type: String,
}

/// Enum to represent the column to search in
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchColumn {
    All,
    Name,
    Id,
    Size,
    Filename,
    Type,
}

impl SearchColumn {
    fn display_name(&self) -> &'static str {
        match self {
            SearchColumn::All => "All Columns",
            SearchColumn::Name => "Name",
            SearchColumn::Id => "ID",
            SearchColumn::Size => "Size",
            SearchColumn::Filename => "Filename",
            SearchColumn::Type => "Type",
        }
    }
    
    fn all_columns() -> Vec<SearchColumn> {
        vec![
            SearchColumn::All,
            SearchColumn::Name,
            SearchColumn::Id, 
            SearchColumn::Size,
            SearchColumn::Filename,
            SearchColumn::Type,
        ]
    }
}

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
                            // For size, convert to various formats for more flexible searching
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

    /// Export audio data to a WAV file
    fn export_to_wav(&self, audio_file_index: usize) -> Result<(), String> {
        // Check if audio files are loaded
        let audio_files = match &self.audio_files {
            Some(files) => files,
            None => return Err("No audio files loaded".to_string()),
        };

        // Check if index is valid
        if audio_file_index >= audio_files.len() {
            return Err("Invalid audio file index".to_string());
        }

        // Get the selected audio file info
        let audio_file_info = &audio_files[audio_file_index];
        
        // Get the original file path
        let original_file_path = match &self.selected_file {
            Some(path) => path,
            None => return Err("No file selected".to_string()),
        };

        // Load the NUS3AUDIO file
        let nus3_file = match Nus3audioFile::open(original_file_path) {
            Ok(file) => file,
            Err(e) => return Err(format!("Failed to open NUS3AUDIO file: {}", e)),
        };

        // Find the audio file by name
        let audio_file = nus3_file.files.iter().find(|f| f.name == audio_file_info.name);
        let audio_file = match audio_file {
            Some(file) => file,
            None => return Err("Audio file not found in NUS3AUDIO file".to_string()),
        };

        // Create output file path (same directory as original file with .wav extension)
        let original_path = Path::new(original_file_path);
        let parent_dir = match original_path.parent() {
            Some(dir) => dir,
            None => return Err("Failed to get parent directory".to_string()),
        };
        
        let output_filename = format!("{}.wav", audio_file_info.name);
        let output_path = parent_dir.join(output_filename);
        
        // Write audio data to WAV file
        match fs::write(&output_path, &audio_file.data) {
            Ok(_) => {
                println!("Successfully exported WAV file to: {:?}", output_path);
                Ok(())
            },
            Err(e) => Err(format!("Failed to write WAV file: {}", e)),
        }
    }

    /// Static method to render table UI with a callback for export button click
    fn render_table(
        ui: &mut Ui,
        audio_files: &[AudioFileInfo],
        selected_rows: &mut HashSet<usize>,
        striped: bool,
        clickable: bool,
        show_grid_lines: bool,
        available_height: f32,
        available_width: f32,
        on_export_clicked: &mut dyn FnMut(usize),
    ) {
        // Set row height and text style
        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);

        ui.set_height(text_height * 2.0); // Set height to twice the text height

        // Define column width with minimum sizes
        let col_width_name = available_width / 5.0; // Adjusted for better fit
        let col_width_id = available_width / 8.0; // Increased for long IDs
        let col_width_size = available_width / 8.0;
        let col_width_filename = available_width / 5.0;
        let col_width_type = available_width / 8.0;
        let col_action = available_width
            - col_width_name
            - col_width_id
            - col_width_size
            - col_width_filename
            - col_width_type;

        // Header text size
        let heading_size = 17.0;

        // Create header
        let header_bg_color = if ui.visuals().dark_mode {
            Color32::from_rgb(50, 50, 60)
        } else {
            Color32::from_rgb(220, 220, 230)
        };

        let header_rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(
            Rect::from_min_size(header_rect.min, Vec2::new(header_rect.width(), 35.0)),
            0.0,
            header_bg_color,
        );

        Grid::new("table_header")
            .num_columns(6)
            .spacing([5.0, 0.0])
            .show(ui, |ui| {
                // Header
                ui.add_sized(
                    [col_width_name, 35.0],
                    egui::Label::new(RichText::new("Name").size(heading_size).strong()),
                )
                .on_hover_text("Audio file name");

                ui.add_sized(
                    [col_width_id, 35.0],
                    egui::Label::new(RichText::new("ID").size(heading_size).strong()),
                )
                .on_hover_text("Audio file ID");

                ui.add_sized(
                    [col_width_size, 35.0],
                    egui::Label::new(RichText::new("Size").size(heading_size).strong()),
                )
                .on_hover_text("File size in bytes");

                ui.add_sized(
                    [col_width_filename, 35.0],
                    egui::Label::new(RichText::new("Filename").size(heading_size).strong()),
                )
                .on_hover_text("Audio filename");

                ui.add_sized(
                    [col_width_type, 35.0],
                    egui::Label::new(RichText::new("Type").size(heading_size).strong()),
                )
                .on_hover_text("Audio file type");

                ui.add_sized(
                    [col_action, 35.0],
                    egui::Label::new(RichText::new("Action").size(heading_size).strong()),
                )
                .on_hover_text("Action");
                ui.end_row();
            });

        // Create table content
        let row_height = text_height * 2.0;
        let text_size = 16.0;
        // let row_height = ui.spacing().interact_size.y; // if you are adding buttons instead of labels.
        ui.set_min_height(available_height - 180.0); // Adjusted for header and spacing

        ScrollArea::vertical().show_rows(ui, row_height, audio_files.len(), |ui, row_range| {
            Grid::new("table_content")
                .num_columns(6)
                .spacing([5.0, 2.0])
                .show(ui, |ui| {
                    for row_index in row_range {
                        let file = &audio_files[row_index];
                        let is_selected = selected_rows.contains(&row_index);

                        // Striped background
                        if striped && row_index % 2 == 1 {
                            let row_rect = ui.available_rect_before_wrap();
                            ui.painter().rect_filled(
                                Rect::from_min_size(
                                    row_rect.min,
                                    Vec2::new(row_rect.width(), row_height),
                                ),
                                0.0,
                                ui.visuals().faint_bg_color,
                            );
                        }

                        // Highlight selected row
                        if is_selected {
                            let row_rect = ui.available_rect_before_wrap();
                            ui.painter().rect_filled(
                                Rect::from_min_size(
                                    row_rect.min,
                                    Vec2::new(row_rect.width(), row_height),
                                ),
                                0.0,
                                ui.visuals().selection.bg_fill,
                            );
                        }

                        // Create a responsive area that includes the entire row
                        let row_rect = ui.available_rect_before_wrap();
                        let sense = if clickable {
                            egui::Sense::click()
                        } else {
                            egui::Sense::hover()
                        };
                        let row_response = ui.interact(
                            Rect::from_min_size(
                                row_rect.min,
                                Vec2::new(row_rect.width(), row_height),
                            ),
                            ui.id().with(row_index),
                            sense,
                        );

                        // Handle row click events
                        if row_response.clicked() && clickable {
                            if selected_rows.contains(&row_index) {
                                selected_rows.remove(&row_index);
                            } else {
                                selected_rows.insert(row_index);
                            }
                        }

                        // Column 1: Name - with text clipping
                        ui.scope(|ui| {
                            ui.style_mut().wrap_mode = Some(TextWrapMode::Truncate);
                            let text = RichText::new(&file.name).size(text_size);
                            ui.add_sized([col_width_name, row_height], egui::Label::new(text))
                                .on_hover_text(&file.name);
                        });

                        // Column 2: ID - with text clipping and ellipsis
                        ui.scope(|ui| {
                            ui.style_mut().wrap_mode = Some(TextWrapMode::Truncate);
                            let text = RichText::new(if file.id.len() > 20 {
                                format!("{}...", &file.id[0..17])
                            } else {
                                file.id.clone()
                            })
                            .size(text_size);
                            ui.add_sized([col_width_id, row_height], egui::Label::new(text))
                                .on_hover_text(&file.id);
                        });

                        // Column 3: Size
                        let size_text = if file.size < 1024 {
                            format!("{} B", file.size)
                        } else if file.size < 1024 * 1024 {
                            format!("{:.1} KB", file.size as f32 / 1024.0)
                        } else {
                            format!("{:.1} MB", file.size as f32 / (1024.0 * 1024.0))
                        };

                        ui.add_sized(
                            [col_width_size, row_height],
                            egui::Label::new(RichText::new(size_text).size(text_size)),
                        );

                        // Column 4: Filename - with text clipping
                        ui.scope(|ui| {
                            ui.style_mut().wrap_mode = Some(TextWrapMode::Truncate);
                            let text = RichText::new(&file.filename).size(text_size);
                            ui.add_sized([col_width_filename, row_height], egui::Label::new(text))
                                .on_hover_text(&file.filename);
                        });

                        // Column 5: Type
                        ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);

                        // Set different colors based on file type
                        let type_text = match file.file_type.as_str() {
                            "OPUS Audio" => RichText::new(&file.file_type)
                                .size(text_size)
                                .color(Color32::from_rgb(100, 200, 100)), // Green
                            "IDSP Audio" => RichText::new(&file.file_type)
                                .size(text_size)
                                .color(Color32::from_rgb(100, 150, 255)), // Blue
                            _ => RichText::new(&file.file_type)
                                .size(text_size)
                                .color(Color32::from_rgb(200, 150, 100)), // Yellow/Brown
                        };

                        ui.add_sized([col_width_type, row_height], egui::Label::new(type_text));
                        
                        // Column 6: Action - Add "Output to WAV" button
                        if ui.add_sized(
                            [col_action, row_height],
                            Button::new(RichText::new("Output to WAV").size(text_size))
                        ).clicked() {
                            // Call the callback to handle the export
                            on_export_clicked(row_index);
                        }

                        ui.end_row();

                        // Add grid lines
                        if show_grid_lines && row_index < audio_files.len() - 1 {
                            let line_start = row_rect.min + Vec2::new(0.0, row_height);
                            let line_end = line_start + Vec2::new(row_rect.width(), 0.0);
                            ui.painter().line_segment(
                                [line_start, line_end],
                                Stroke::new(
                                    0.5,
                                    ui.visuals().widgets.noninteractive.bg_stroke.color,
                                ),
                            );
                        }
                    }
                });
        });
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
                    Frame::group(ui.style())
                        .stroke(Stroke::new(1.0, ui.visuals().widgets.active.bg_fill))
                        .rounding(Rounding::same(5.0))
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
                    ui.add_space(10.0);

                    // Get filtered audio files
                    let filtered_audio_files = self.filtered_audio_files();
                    let files_count = filtered_audio_files.len();
                    let striped = self.striped;
                    let clickable = self.clickable;
                    let show_grid_lines = self.show_grid_lines;

                    // Add table configuration interface - use group frame to make it more beautiful
                    Frame::group(ui.style())
                        .stroke(Stroke::new(1.0, ui.visuals().widgets.active.bg_fill))
                        .rounding(Rounding::same(5.0))
                        .show(ui, |ui| {
                            ui.heading("Table Settings");
                            ui.add_space(5.0);

                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.striped, "Striped Background");
                                ui.checkbox(&mut self.resizable, "Resizable Columns");
                                ui.checkbox(&mut self.clickable, "Clickable Rows");
                                ui.checkbox(&mut self.show_grid_lines, "Show Grid Lines");
                            });

                            if self.clickable {
                                ui.label(format!("Selected {} items", self.selected_rows.len()));

                                if !self.selected_rows.is_empty()
                                    && ui.button("Clear Selection").clicked()
                                {
                                    self.selected_rows.clear();
                                }
                            }
                        });

                    ui.add_space(10.0);

                    // Add a nice border to the table
                    Frame::group(ui.style())
                        .stroke(Stroke::new(1.0, ui.visuals().faint_bg_color))
                        .rounding(Rounding::same(4.0))
                        // Remove non-existent padding method
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
                                    
                                    // Variable to store which file was exported
                                    let mut export_index = None;
                                    
                                    // Use static method to render table with filtered files
                                    Self::render_table(
                                        ui,
                                        &filtered_audio_files,
                                        &mut self.selected_rows,
                                        striped,
                                        clickable,
                                        show_grid_lines,
                                        available_height - 100.0, // Adjusted for header and spacing
                                        available_width - 16.0, // Adjusted for padding
                                        &mut |index| {
                                            export_index = Some(index);
                                        },
                                    );
                                    
                                    // Process export if needed
                                    if let Some(idx) = export_index {
                                        if idx < filtered_audio_files.len() {
                                            let audio_info = &filtered_audio_files[idx];
                                            
                                            // Load the NUS3AUDIO file again to get the data
                                            if let Some(file_path) = &self.selected_file {
                                                match Nus3audioFile::open(file_path) {
                                                    Ok(nus3_file) => {
                                                        if let Some(audio_file) = nus3_file.files.iter().find(|f| f.name == audio_info.name) {
                                                            // Create output directory (same as input file)
                                                            let path = Path::new(file_path);
                                                            if let Some(dir) = path.parent() {
                                                                let wav_path = dir.join(format!("{}.wav", audio_info.name));
                                                                
                                                                // Write the data
                                                                match fs::write(&wav_path, &audio_file.data) {
                                                                    Ok(_) => {
                                                                        ui.add_space(8.0);
                                                                        ui.colored_label(Color32::GREEN, 
                                                                            format!("Successfully exported to: {}", wav_path.display()));
                                                                    },
                                                                    Err(e) => {
                                                                        ui.add_space(8.0);
                                                                        ui.colored_label(Color32::RED, 
                                                                            format!("Failed to write WAV file: {}", e));
                                                                    }
                                                                }
                                                            } else {
                                                                ui.add_space(8.0);
                                                                ui.colored_label(Color32::RED, "Failed to get parent directory");
                                                            }
                                                        } else {
                                                            ui.add_space(8.0);
                                                            ui.colored_label(Color32::RED, "Audio file not found in NUS3AUDIO file");
                                                        }
                                                    },
                                                    Err(e) => {
                                                        ui.add_space(8.0);
                                                        ui.colored_label(Color32::RED, 
                                                            format!("Failed to open NUS3AUDIO file: {}", e));
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
                                    
                                    ui.add_space(8.0);
                                });
                                ui.add_space(8.0);
                            });
                            ui.add_space(8.0);
                        });
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
}
