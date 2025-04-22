use egui::{Context, Ui};
use nus3audio::Nus3audioFile;

/// Structure to hold audio file information
#[derive(Clone)]
pub struct AudioFileInfo {
    pub name: String,
    pub id: String,
    pub size: usize,
    pub filename: String,
    pub file_type: String,
}

/// Main editing area component
pub struct MainArea {
    pub selected_file: Option<String>,
    pub file_count: Option<usize>,
    pub audio_files: Option<Vec<AudioFileInfo>>,
    pub error_message: Option<String>,
}

impl MainArea {
    /// Create a new main area
    pub fn new() -> Self {
        Self {
            selected_file: None,
            file_count: None,
            audio_files: None,
            error_message: None,
        }
    }
    
    /// Update the selected file and load NUS3AUDIO info if applicable
    pub fn update_selected_file(&mut self, file_path: Option<String>) {
        self.selected_file = file_path;
        self.file_count = None;
        self.audio_files = None;
        self.error_message = None;
        
        // If file is selected and it's a NUS3AUDIO file, load its info
        if let Some(file_name) = &self.selected_file {
            if file_name.to_lowercase().ends_with(".nus3audio") || 
               file_name.to_lowercase().ends_with(".nus3bank") {
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
                                    _ => "Unknown Format",
                                }
                            } else {
                                "Unknown Format"
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
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            
            ui.heading("Audio Editor");
            
            if let Some(selected) = &self.selected_file {
                ui.label(format!("Currently editing: {}", selected));
                
                ui.add_space(20.0);
                ui.heading("NUS3AUDIO Info Display");
                
                // Display file info if available
                if let Some(audio_files) = &self.audio_files {
                    ui.add_space(10.0);
                    
                    // Display file count
                    if let Some(count) = self.file_count {
                        ui.label(format!("Number of audio files: {}", count));
                        ui.add_space(5.0);
                    }
                    
                    // Create a scrollable area for the table
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .auto_shrink([false; 2])  // 防止自动收缩，让表格充满整个宽度
                        .show(ui, |ui| {
                            // 使表格使用所有可用宽度
                            ui.set_min_width(ui.available_width());
                            // Table header
                            ui.push_id("audio_files_table", |ui| {
                                egui::Grid::new("audio_files_grid")
                                    .num_columns(5)
                                    .striped(true)
                                    .spacing([10.0, 8.0])  // 增加行间距以增加行高
                                    .min_col_width(100.0)  // 设置最小列宽度
                                    .show(ui, |ui| {
                                        // Table header with improved styling
                                        let heading_size = 16.0; // 设置表头字体大小
                                        ui.strong(egui::RichText::new("Name").size(heading_size)).on_hover_text("Audio file name");
                                        ui.strong(egui::RichText::new("ID").size(heading_size)).on_hover_text("Audio file ID");
                                        ui.strong(egui::RichText::new("Size (bytes)").size(heading_size)).on_hover_text("File size in bytes");
                                        ui.strong(egui::RichText::new("Filename").size(heading_size)).on_hover_text("Audio filename");
                                        ui.strong(egui::RichText::new("Type").size(heading_size)).on_hover_text("Audio file type");
                                        ui.end_row();
                                        
                                        // Table rows with improved styling
                                        for file in audio_files {
                                            // 为所有列设置一致的行高
                                            let row_height = 30.0; // 设置更高的行高
                                            
                                            // 为所有列应用相同的高度
                                            ui.add_sized([0.0, row_height], egui::Label::new(&file.name));
                                            ui.add_sized([0.0, row_height], egui::Label::new(&file.id));
                                            ui.add_sized([0.0, row_height], egui::Label::new(format!("{}", file.size)));
                                            ui.add_sized([0.0, row_height], egui::Label::new(&file.filename));
                                            ui.add_sized([0.0, row_height], egui::Label::new(&file.file_type));
                                            ui.end_row();
                                        }
                                    });
                            });
                        });
                } else if let Some(error) = &self.error_message {
                    ui.add_space(10.0);
                    ui.colored_label(egui::Color32::RED, error);
                } else {
                    let rect = egui::Rect::from_min_size(
                        ui.cursor().min,
                        egui::vec2(ui.available_width(), 200.0),
                    );
                    ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(80, 80, 80));
                    ui.add_space(200.0); // Add space to account for the rect
                    
                    if selected.to_lowercase().ends_with(".nus3audio") || 
                       selected.to_lowercase().ends_with(".nus3bank") {
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
