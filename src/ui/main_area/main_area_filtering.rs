use nus3audio::Nus3audioFile;
use std::sync::mpsc;
use std::time::Instant;
use super::{
    main_area_core::{FileLoadResult, MainArea},
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

    /// Poll for background file load completion. Call once per frame.
    pub fn poll_file_load(&mut self, ctx: &egui::Context) {
        let result = match &self.file_load_receiver {
            Some(rx) => rx.try_recv().ok(),
            None => None,
        };
        if let Some(result) = result {
            self.file_load_receiver = None;
            match result {
                FileLoadResult::Nus3audio { file_name, file_count, audio_files, track_ids } => {
                    super::export_utils::ExportUtils::prime_indexing_cache(&file_name, &track_ids);
                    self.file_count = Some(file_count);
                    self.audio_files = Some(audio_files);
                }
                FileLoadResult::Nus3bank { file_count, audio_files } => {
                    self.file_count = Some(file_count);
                    self.audio_files = Some(audio_files);
                }
                FileLoadResult::Error(msg) => {
                    self.error_message = Some(msg);
                }
            }
        } else if self.file_load_receiver.is_some() {
            ctx.request_repaint();
        }
    }

    /// Update the selected file and load NUS3AUDIO info if applicable.
    /// Skips reloading if the same file is already loaded to avoid redundant I/O.
    pub fn update_selected_file(&mut self, file_path: Option<String>) {
        if file_path == self.selected_file && self.audio_files.is_some() {
            return;
        }

        ReplaceUtils::clear_replacements();

        self.selected_file = file_path;
        self.file_count = None;
        self.audio_files = None;
        self.error_message = None;

        if let Some(file_name) = &self.selected_file {
            self.spawn_file_load(file_name.clone());
        }
    }

    /// Force a full reload of the currently selected file, bypassing the same-file skip.
    pub fn force_reload_selected_file(&mut self) {
        ReplaceUtils::clear_replacements();
        self.file_count = None;
        self.audio_files = None;
        self.error_message = None;

        if let Some(file_name) = self.selected_file.clone() {
            self.spawn_file_load(file_name);
        }
    }

    fn spawn_file_load(&mut self, file_name: String) {
        let (tx, rx) = mpsc::channel();
        self.file_load_receiver = Some(rx);

        std::thread::spawn(move || {
            let lower = file_name.to_lowercase();
            let result = if lower.ends_with(".nus3audio") {
                Self::load_nus3audio_bg(&file_name)
            } else if lower.ends_with(".nus3bank") {
                Self::load_nus3bank_bg(&file_name)
            } else {
                FileLoadResult::Error(format!("Unsupported file format: {}", file_name))
            };
            let _ = tx.send(result);
        });
    }

    fn load_nus3audio_bg(file_name: &str) -> FileLoadResult {
        let t_open = Instant::now();
        match Nus3audioFile::open(file_name) {
            Ok(nus3_file) => {
                let open_ms = t_open.elapsed().as_millis();
                println!("[PERF] NUS3AUDIO open: {}ms ({} tracks)", open_ms, nus3_file.files.len());

                let track_ids: Vec<u32> = nus3_file.files.iter().map(|f| f.id).collect();
                let file_count = nus3_file.files.len();

                let t_process = Instant::now();
                let mut audio_files = Vec::new();

                for audio_file in nus3_file.files.iter() {
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

                println!("[PERF] NUS3AUDIO process tracks: {}ms", t_process.elapsed().as_millis());
                println!("[PERF] NUS3AUDIO load total: {}ms", t_open.elapsed().as_millis());

                FileLoadResult::Nus3audio {
                    file_name: file_name.to_string(),
                    file_count,
                    audio_files,
                    track_ids,
                }
            }
            Err(e) => {
                println!("[PERF] NUS3AUDIO open FAILED: {}ms", t_open.elapsed().as_millis());
                FileLoadResult::Error(format!("Error loading NUS3AUDIO file: {}", e))
            }
        }
    }

    fn load_nus3bank_bg(file_name: &str) -> FileLoadResult {
        let t_open = Instant::now();
        match Nus3bankFile::open(file_name) {
            Ok(nus3bank_file) => {
                let open_ms = t_open.elapsed().as_millis();
                println!("[PERF] NUS3BANK open: {}ms ({} tracks)", open_ms, nus3bank_file.tracks.len());

                let t_process = Instant::now();
                let file_count = nus3bank_file.tracks.len();
                let mut audio_files = Vec::new();

                for track in nus3bank_file.tracks.iter() {
                    let mut info = AudioFileInfo::from_nus3bank_track(
                        track.name.clone(),
                        track.index as u32,
                        track.hex_id.clone(),
                        track.size as usize,
                        track.filename(),
                    );
                    let data = track.audio_data.as_deref().unwrap_or_default();
                    info.file_type = if data.starts_with(b"BNSF") {
                        "BNSF"
                    } else if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WAVE") {
                        "WAV"
                    } else {
                        "Unknown"
                    }.to_string();
                    audio_files.push(info);
                }

                println!("[PERF] NUS3BANK process tracks: {}ms", t_process.elapsed().as_millis());
                println!("[PERF] NUS3BANK load total: {}ms", t_open.elapsed().as_millis());

                FileLoadResult::Nus3bank {
                    file_count,
                    audio_files,
                }
            }
            Err(e) => {
                println!("[PERF] NUS3BANK open FAILED: {}ms", t_open.elapsed().as_millis());
                FileLoadResult::Error(format!("Error loading NUS3BANK file: {}", e))
            }
        }
    }
}
