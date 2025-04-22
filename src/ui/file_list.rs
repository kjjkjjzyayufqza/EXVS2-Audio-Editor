use egui::{Ui, ScrollArea};
use std::path::PathBuf;

/// File item structure
#[derive(Debug, Clone)]
pub struct FileItem {
    pub path: String,
    pub name: String,
    pub is_selected: bool,
}

/// File list component
#[derive(Default)]
pub struct FileList {
    pub files: Vec<FileItem>,
    pub selected_file: Option<String>,
}

impl FileList {
    /// Create a new file list
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a file to the list
    pub fn add_file(&mut self, path: String) {
        let path_obj = PathBuf::from(&path);
        let name = path_obj
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown file")
            .to_string();
            
        self.files.push(FileItem {
            path: path.clone(),
            name,
            is_selected: false,
        });
        
        // Automatically select the newly added file
        self.selected_file = Some(path);
        self.update_selection();
    }
    
    /// Update selection state
    fn update_selection(&mut self) {
        for file in &mut self.files {
            if let Some(selected) = &self.selected_file {
                file.is_selected = &file.path == selected;
            } else {
                file.is_selected = false;
            }
        }
    }
    
    /// Display the file list
    pub fn show(&mut self, ui: &mut Ui) -> bool {
        let mut file_selected = false;
        let mut selected_path = None;
        
        ui.heading("File List");
        ui.add_space(8.0);
        
        if self.files.is_empty() {
            ui.label("No files selected. Click the 'Add File' button below.");
        } else {
            ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    // Copy paths to avoid borrowing conflicts
                    let file_paths: Vec<(bool, String, String)> = self.files
                        .iter()
                        .map(|f| (f.is_selected, f.path.clone(), f.name.clone()))
                        .collect();
                    
                    for (is_selected, path, name) in file_paths {
                        let response = ui.selectable_label(is_selected, &name);
                        
                        if response.clicked() {
                            selected_path = Some(path.clone());
                            file_selected = true;
                        }
                    }
                });
        }
        
        // Update selection outside ScrollArea to avoid borrowing conflicts
        if let Some(path) = selected_path {
            self.selected_file = Some(path);
            self.update_selection();
        }
        
        ui.add_space(8.0);
        if ui.button("Add File").clicked() {
            // Open file dialog
            if let Some(path) = rfd::FileDialog::new().pick_file() {
                let path_str = path.to_string_lossy().to_string();
                println!("Selected file: {}", path_str);
                self.add_file(path_str);
                file_selected = true;
            }
        }
        
        file_selected
    }
}
