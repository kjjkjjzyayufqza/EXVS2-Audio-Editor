use nus3audio::Nus3audioFile;
use super::{
    main_area_core::MainArea,
    audio_file_info::AudioFileInfo,
    search_column::SearchColumn,
    sort_column::SortColumn,
    replace_utils::ReplaceUtils
};
use crate::nus3bank::Nus3bankFile;

impl MainArea {
    /// Get filtered audio files based on search query and column, then sort them
    pub fn filtered_audio_files(&self) -> Vec<AudioFileInfo> {
        if let Some(audio_files) = &self.audio_files {
            // First, filter the files based on search criteria
            let mut filtered_files = if self.search_query.is_empty() {
                // If no search query, use all audio files
                audio_files.clone()
            } else {
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
            };
            
            // Then sort the filtered files based on sort column and direction
            if self.sort_column != SortColumn::None {
                filtered_files.sort_by(|a, b| {
                    let ordering = match self.sort_column {
                        SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                        SortColumn::Id => {
                            // Try to parse IDs as numbers for numeric sorting
                            let parse_a = a.id.parse::<usize>();
                            let parse_b = b.id.parse::<usize>();
                            
                            match (parse_a, parse_b) {
                                // If both can be parsed as numbers, sort numerically
                                (Ok(num_a), Ok(num_b)) => num_a.cmp(&num_b),
                                // If one can be parsed but the other can't, prioritize the numeric one
                                (Ok(_), Err(_)) => std::cmp::Ordering::Less,
                                (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
                                // If neither can be parsed as numbers, fall back to string comparison
                                (Err(_), Err(_)) => a.id.to_lowercase().cmp(&b.id.to_lowercase()),
                            }
                        },
                        SortColumn::Size => a.size.cmp(&b.size),
                        SortColumn::Filename => a.filename.to_lowercase().cmp(&b.filename.to_lowercase()),
                        SortColumn::Type => a.file_type.to_lowercase().cmp(&b.file_type.to_lowercase()),
                        SortColumn::None => std::cmp::Ordering::Equal,
                    };
                    
                    if self.sort_ascending {
                        ordering
                    } else {
                        ordering.reverse()
                    }
                });
            }
            
            filtered_files
        } else {
            Vec::new()
        }
    }
    
    /// Helper function to match size values in different formats
    pub fn size_matches(&self, size: usize, query: &str) -> bool {
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
        // Clear any previously replaced audio data in memory
        ReplaceUtils::clear_replacements();
        
        self.selected_file = file_path;
        self.file_count = None;
        self.audio_files = None;
        self.error_message = None;

        // If file is selected, determine type and load accordingly
        if let Some(file_name) = &self.selected_file {
            let file_name = file_name.clone(); // Clone to avoid borrowing issues
            if file_name.to_lowercase().ends_with(".nus3audio") {
                self.load_nus3audio_file(&file_name);
            } else if file_name.to_lowercase().ends_with(".nus3bank") {
                self.load_nus3bank_file(&file_name);
            }
        }
    }
    
    /// Load NUS3AUDIO file (existing implementation)
    fn load_nus3audio_file(&mut self, file_name: &str) {
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
                                    b"OPUS" => "OPUS",
                                    b"IDSP" => "IDSP",
                                    b"RIFF" => "WAV",
                                    b"BNSF" => "BNSF",
                                    _ => "Unknown",
                                }
                            } else {
                                "Unknown"
                            };

                            audio_files.push(AudioFileInfo::from_nus3audio(
                                audio_file.name.clone(),
                                audio_file.id.to_string(),
                                audio_file.data.len(),
                                audio_file.filename(),
                                file_type.to_string(),
                            ));
                        }

                        self.audio_files = Some(audio_files);
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Error loading NUS3AUDIO file: {}", e));
                    }
                }
    }
    
    /// Load NUS3BANK file (new implementation)
    fn load_nus3bank_file(&mut self, file_name: &str) {
        match Nus3bankFile::open(file_name) {
            Ok(nus3bank_file) => {
                self.file_count = Some(nus3bank_file.tracks.len());
                let mut audio_files = Vec::new();

                for track in nus3bank_file.tracks.iter() {
                    audio_files.push(AudioFileInfo::from_nus3bank_track(
                        track.name.clone(),
                        track.numeric_id,
                        track.hex_id.clone(),
                        track.size as usize,
                        track.filename(),
                    ));
                }

                self.audio_files = Some(audio_files);
            }
            Err(e) => {
                self.error_message = Some(format!("Error loading NUS3BANK file: {}", e));
            }
        }
    }
}
