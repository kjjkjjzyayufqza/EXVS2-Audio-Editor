use egui::{Ui, ScrollArea};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// File item structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileItem {
    pub path: String,
    pub name: String,
    pub is_selected: bool,
}

/// File list component
#[derive(Default, Deserialize, Serialize)]
pub struct FileList {
    pub files: Vec<FileItem>,
    pub selected_file: Option<String>,
    #[serde(skip)]
    pub search_query: String,
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
    
    /// Remove a file from the list
    pub fn remove_file(&mut self, path: &str) {
        // Find index of file to remove
        let index = self.files.iter().position(|f| f.path == path);
        
        if let Some(idx) = index {
            // Remove file from list
            self.files.remove(idx);
            
            // Update selection if the removed file was selected
            if let Some(selected) = &self.selected_file {
                if selected == path {
                    // Select another file if available
                    self.selected_file = self.files.first().map(|f| f.path.clone());
                }
            }
            
            self.update_selection();
        }
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
    
    /// Get filtered files based on search query
    fn filtered_files(&self) -> Vec<&FileItem> {
        if self.search_query.is_empty() {
            // If no search query, return all files
            return self.files.iter().collect();
        }
        
        let query = self.search_query.to_lowercase();
        self.files
            .iter()
            .filter(|file| {
                file.name.to_lowercase().contains(&query) || 
                file.path.to_lowercase().contains(&query)
            })
            .collect()
    }
    
    /// Display the file list
    pub fn show(&mut self, ui: &mut Ui) -> bool {
        let mut file_selected = false;
        let mut selected_path = None;
        
        ui.heading("File List");
        ui.add_space(8.0);
        
        // Add search box
        ui.horizontal(|ui| {
            ui.label("Search:");
            if ui.text_edit_singleline(&mut self.search_query).changed() {
                // When search query changes, no need to do anything special
                // as filtered_files() will be called below
            }
            if !self.search_query.is_empty() && ui.button("✖").clicked() {
                self.search_query.clear();
            }
        });
        ui.add_space(4.0);
        
        if self.files.is_empty() {
            ui.label("No files selected. Click the 'Add File' button below.");
        } else {
            // Get filtered files
            let filtered = self.filtered_files();
            
            if filtered.is_empty() {
                ui.label("No matching files found.");
            } else {
                ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        // Copy filtered files to avoid borrowing conflicts
                        let file_paths: Vec<(bool, String, String)> = filtered
                            .iter()
                            .map(|f| (f.is_selected, f.path.clone(), f.name.clone()))
                            .collect();
                        
                        for (is_selected, path, name) in file_paths {
                            ui.horizontal(|ui| {
                                let response = ui.selectable_label(is_selected, &name);
                                
                                if response.clicked() {
                                    selected_path = Some(path.clone());
                                    file_selected = true;
                                }
                                
                                // Add remove button
                                if ui.small_button("❌").clicked() {
                                    // Remove this file
                                    // We can't directly modify the files list here due to borrowing rules
                                    // So we'll store the path and remove it after the loop
                                    selected_path = Some(path.clone());
                                    // Set this flag to false as we're removing, not selecting
                                    file_selected = false;
                                }
                            });
                        }
                    });
            }
        }
        
        // Update selection or remove file outside ScrollArea to avoid borrowing conflicts
        if let Some(path) = selected_path {
            if file_selected {
                // Regular selection
                self.selected_file = Some(path);
                self.update_selection();
            } else {
                // Remove file
                self.remove_file(&path);
            }
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
