use egui::{Color32, Frame, Stroke, Ui};

use super::{
    audio_file_info::AudioFileInfo, export_utils::ExportUtils, main_area_core::MainArea,
    replace_utils::ReplaceUtils, table_renderer::TableRenderer, add_audio_utils::AddAudioUtils,
    remove_utils::RemoveUtils, nus3audio_file_utils::Nus3audioFileUtils,
};

impl MainArea {
    /// Render the audio file table and handle export/play actions
    pub fn render_audio_table(
        &mut self,
        ui: &mut Ui,
        filtered_audio_files: Vec<AudioFileInfo>,
        files_count: usize,
        available_height: f32,
        available_width: f32,
    ) {
        // Use these variables to capture action information outside the immediate UI context
        // This way we can perform actions after all UI rendering is done to avoid multiple self borrowing
        struct ActionData {
            export_index: Option<usize>,
            play_index: Option<usize>,
            replace_index: Option<usize>,
            remove_index: Option<usize>,
            export_all: bool,
            add_audio: bool,
        }

        let mut action_data = ActionData {
            export_index: None,
            play_index: None,
            replace_index: None,
            remove_index: None,
            export_all: false,
            add_audio: false,
        };

        // First, render the UI
        Frame::group(ui.style())
            .stroke(Stroke::new(1.0, ui.visuals().faint_bg_color))
            .show(ui, |ui| {
                // Margins
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        // Table header
                        ui.heading("Audio File List");

                        ui.horizontal(|ui| {
                            // Capture Export All button click, don't act on it yet
                            if ui.button("Export All").clicked() {
                                action_data.export_all = true;
                            }

                            if ui.button("Add Audio").clicked() {
                                println!("Add Audio button clicked");
                                action_data.add_audio = true;
                            }
                        });

                        // File count display
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if !self.search_query.is_empty() {
                                ui.label(format!(
                                    "Found: {} / {} files",
                                    files_count,
                                    self.file_count.unwrap_or(0)
                                ));
                            } else {
                                ui.label(format!("Total: {} files", files_count));
                            }
                        });

                        ui.add_space(5.0);

                        // Empty results message
                        if !self.search_query.is_empty() && filtered_audio_files.is_empty() {
                            ui.add_space(8.0);
                            ui.label("No audio files match the search criteria.");
                        }

                        // The actual table rendering - capture actions but don't execute them yet
                        TableRenderer::render_table(
                            ui,
                            &filtered_audio_files,
                            &mut self.selected_rows,
                            self.striped,
                            self.clickable,
                            self.show_grid_lines,
                            available_height,
                            available_width,
                            &mut |index| {
                                action_data.export_index = Some(index);
                            },
                            &mut |index| {
                                action_data.play_index = Some(index);
                            },
                            &mut |index| {
                                action_data.replace_index = Some(index);
                            },
                            &mut |index| {
                                action_data.remove_index = Some(index);
                            },
                            &mut self.sort_column,
                            &mut self.sort_ascending,
                        );

                        ui.add_space(8.0);
                    });
                    ui.add_space(8.0);
                });
                ui.add_space(8.0);
            });

        // Collect toast messages to add - we'll add them all at once to avoid multiple self.add_toast calls
        let mut toasts_to_add = Vec::new();

        // Process all actions and collect toast messages

        // Handle "Add Audio" action if clicked
        if action_data.add_audio {
            let selected_file = self.selected_file.clone();
            
            if let Some(file_path) = &selected_file {
                // Use AddAudioUtils to open file dialog and show add audio modal
                match AddAudioUtils::add_with_file_dialog(&mut self.add_audio_modal, self.audio_files.clone()) {
                    Ok(_) => {
                        toasts_to_add.push((
                            "Please configure settings for the new audio file".to_string(),
                            Color32::GOLD,
                        ));
                    }
                    Err(e) => {
                        toasts_to_add.push((format!("Add audio failed: {}", e), Color32::RED));
                    }
                }
            } else {
                toasts_to_add.push(("No file selected".to_string(), Color32::RED));
            }
        }

        // Handle "Export All" action if clicked
        if action_data.export_all {
            let selected_file = self.selected_file.clone();
            let output_path = self.output_path.clone();

            if let Some(file_path) = &selected_file {
                if let Some(output_dir) = &output_path {
                    // Use ExportUtils to export all files
                    match ExportUtils::export_all_to_wav(file_path, output_dir) {
                        Ok(paths) => {
                            toasts_to_add.push((
                                format!(
                                    "Successfully exported {} files to: {}",
                                    paths.len(),
                                    output_dir
                                ),
                                Color32::GREEN,
                            ));
                        }
                        Err(e) => {
                            toasts_to_add.push((format!("Export failed: {}", e), Color32::RED));
                        }
                    }
                } else {
                    toasts_to_add.push((
                        "No output directory set. Please set an output directory.".to_string(),
                        Color32::GOLD,
                    ));
                }
            } else {
                toasts_to_add.push(("No file selected".to_string(), Color32::RED));
            }
        }

        // Handle "Export" action for a specific file if clicked
        if let Some(idx) = action_data.export_index {
            if idx < filtered_audio_files.len() {
                let audio_info = &filtered_audio_files[idx];
                let selected_file = self.selected_file.clone();
                let output_path = self.output_path.clone();

                if let Some(file_path) = &selected_file {
                    if let Some(output_dir) = &output_path {
                        match ExportUtils::export_to_wav_with_custom_dir(
                            audio_info, file_path, output_dir,
                        ) {
                            Ok(path) => {
                                toasts_to_add.push((
                                    format!("Successfully exported to: {}", path),
                                    Color32::GREEN,
                                ));
                            }
                            Err(e) => {
                                toasts_to_add.push((format!("Export failed: {}", e), Color32::RED));
                            }
                        }
                    } else {
                        toasts_to_add.push((
                            "No output directory set. Please set an output directory.".to_string(),
                            Color32::GOLD,
                        ));
                    }
                } else {
                    toasts_to_add.push(("No file selected".to_string(), Color32::RED));
                }
            }
        }

        // Handle "Play" action if clicked
        if let Some(idx) = action_data.play_index {
            if idx < filtered_audio_files.len() {
                let audio_info = &filtered_audio_files[idx];
                let audio_name = audio_info.name.clone();
                let file_path = self.selected_file.clone();

                if let Some(path) = &file_path {
                    if let Some(audio_player) = &mut self.audio_player {
                        match audio_player.load_audio(audio_info, path) {
                            Ok(()) => {
                                // Start playing
                                let state = audio_player.get_audio_state();
                                let mut state = state.lock().unwrap();
                                if !state.is_playing {
                                    state.toggle_play();
                                }

                                toasts_to_add
                                    .push((format!("Now playing: {}", audio_name), Color32::GREEN));
                            }
                            Err(e) => {
                                toasts_to_add
                                    .push((format!("Failed to load audio: {}", e), Color32::RED));
                            }
                        }
                    } else {
                        toasts_to_add
                            .push(("Audio player is not initialized".to_string(), Color32::RED));
                    }
                } else {
                    toasts_to_add.push(("No file selected".to_string(), Color32::RED));
                }
            }
        }

        // Handle "Replace" action if clicked
        if let Some(idx) = action_data.replace_index {
            if idx < filtered_audio_files.len() {
                let audio_info = &filtered_audio_files[idx];
                let selected_file = self.selected_file.clone();

                if let Some(_file_path) = &selected_file {
                    // Print detailed information about the replacement operation
                    println!(
                        "Starting replacement for audio: {} (ID: {})",
                        audio_info.name, audio_info.id
                    );

                    // Use ReplaceUtils to open file dialog and show loop settings modal
                    // This doesn't replace the audio in memory yet - just stores the file path
                    match ReplaceUtils::replace_with_file_dialog(
                        audio_info,
                        &mut self.loop_settings_modal,
                    ) {
                        Ok(_) => {
                            // Don't update the display information yet
                            // Wait until the loop settings are confirmed before making any changes
                            toasts_to_add.push((
                                format!("Please configure loop settings for: {}", audio_info.name),
                                Color32::GOLD,
                            ));
                        }
                        Err(e) => {
                            toasts_to_add.push((format!("Replace failed: {}", e), Color32::RED));
                        }
                    }
                } else {
                    toasts_to_add.push(("No file selected".to_string(), Color32::RED));
                }
            }
        }

        // Handle "Remove" action if clicked
        if let Some(idx) = action_data.remove_index {
            if idx < filtered_audio_files.len() {
                let audio_info = filtered_audio_files[idx].clone();
                let selected_file = self.selected_file.clone();

                if let Some(_file_path) = &selected_file {
                    // Show the confirm dialog, don't delete directly
                    println!(
                        "Confirming removal of audio: {} (ID: {})",
                        audio_info.name, audio_info.id
                    );
                    
                    // Save the audio info to be removed
                    self.pending_remove_audio = Some(audio_info.clone());
                    
                    // Open the confirm dialog
                    self.confirm_modal.open(
                        "Confirm",
                        &format!("Are you sure you want to delete the audio \"{}\"? This action cannot be undone.", audio_info.name)
                    );
                    
                    toasts_to_add.push((
                        format!("请确认是否删除音频: {}", audio_info.name),
                        Color32::GOLD,
                    ));
                } else {
                    toasts_to_add.push(("No file selected".to_string(), Color32::RED));
                }
            }
        }
        
        // Process the confirm dialog's confirmation action
        if self.confirm_modal.confirmed {
            // Reset the confirmed state
            self.confirm_modal.reset_state();
            
            // If there is an audio to be removed, perform the removal
            if let Some(audio_info) = &self.pending_remove_audio {
                if let Some(file_path) = &self.selected_file {
                    println!(
                        "Confirmed removal of audio: {} (ID: {})",
                        audio_info.name, audio_info.id
                    );
                    
                    // Register the removal in memory only
                    match Nus3audioFileUtils::register_remove(audio_info) {
                        Ok(_) => {
                            // Remove the audio from memory
                            if let Some(ref mut audio_files) = self.audio_files {
                                if let Some(original_idx) = audio_files.iter().position(|f| 
                                    f.name == audio_info.name && f.id == audio_info.id
                                ) {
                                    // Remove from the collection
                                    audio_files.remove(original_idx);
                                    
                                    // Update the file count
                                    self.file_count = Some(audio_files.len());
                                    
                                    toasts_to_add.push((
                                        format!("Successfully marked for deletion: {}", audio_info.name),
                                        Color32::GREEN,
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            toasts_to_add.push((format!("Failed to mark for deletion: {}", e), Color32::RED));
                        }
                    }
                    
                    // Clear the audio info to be removed
                    self.pending_remove_audio = None;
                }
            }
        } else if self.confirm_modal.cancelled {
            // Process the case of cancelling the deletion
            self.confirm_modal.reset_state();
            
            if let Some(audio_info) = &self.pending_remove_audio {
                toasts_to_add.push((
                    format!("Cancelled deletion of audio: {}", audio_info.name),
                    Color32::BLUE,
                ));
                
                // Clear the audio info to be removed
                self.pending_remove_audio = None;
            }
        }

        // Check if add audio modal was confirmed
        if self.add_audio_modal.confirmed {
            // Reset the confirmed flag
            self.add_audio_modal.confirmed = false;

            // Get the selected file
            if let Some(file_path) = &self.selected_file {
                // 1. 获取原始文件路径
                let original_file_path = match &self.add_audio_modal.settings.file_path {
                    Some(path) => path,
                    None => {
                        toasts_to_add.push(("No audio file path available".to_string(), Color32::RED));
                        return;
                    }
                };
                
                // 2. 处理新音频文件
                match AddAudioUtils::process_new_audio(&self.add_audio_modal, file_path) {
                    Ok(new_audio_info) => {
                        // 3. 尝试将音频转换为WAV格式
                        match AddAudioUtils::convert_to_wav(original_file_path) {
                            Ok(wav_data) => {
                                // 4. 使用转换后的WAV数据注册添加操作
                                match Nus3audioFileUtils::register_add_audio(&new_audio_info, wav_data) {
                                    Ok(_) => {
                                        // 5. 更新内存中的音频文件列表
                                        if let Some(ref mut audio_files) = self.audio_files {
                                            audio_files.push(new_audio_info.clone());
                                            self.file_count = Some(audio_files.len());
                                            toasts_to_add.push((
                                                format!("Successfully added new audio (converted to WAV): {}", new_audio_info.name),
                                                Color32::GREEN,
                                            ));
                                        }
                                    },
                                    Err(e) => {
                                        toasts_to_add.push((format!("Failed to register WAV audio: {}", e), Color32::RED));
                                    }
                                }
                            },
                            Err(e) => {
                                // 6. 如果WAV转换失败，回退到使用原始音频数据
                                println!("Warning: Failed to convert to WAV: {}", e);
                                println!("Falling back to original file data");
                                
                                if let Some(data) = &self.add_audio_modal.file_data {
                                    match Nus3audioFileUtils::register_add(&new_audio_info, data.clone()) {
                                        Ok(_) => {
                                            if let Some(ref mut audio_files) = self.audio_files {
                                                audio_files.push(new_audio_info.clone());
                                                self.file_count = Some(audio_files.len());
                                                toasts_to_add.push((
                                                    format!("Successfully added new audio (original format): {}", new_audio_info.name),
                                                    Color32::GREEN,
                                                ));
                                            }
                                        },
                                        Err(e) => {
                                            toasts_to_add.push((format!("Failed to add audio: {}", e), Color32::RED));
                                        }
                                    }
                                } else {
                                    toasts_to_add.push(("No audio data available".to_string(), Color32::RED));
                                }
                            }
                        }
                    },
                    Err(e) => {
                        toasts_to_add.push((format!("Failed to process new audio: {}", e), Color32::RED));
                    }
                }
            }
        }

        // Check if loop settings modal was confirmed
        if self.loop_settings_modal.confirmed {
            // Reset the confirmed flag
            self.loop_settings_modal.confirmed = false;

            if let Some(audio_info) = &self.loop_settings_modal.audio_info {
                if let Some(file_path) = &self.selected_file {
                    // Get loop settings from the modal
                    let loop_start = if self.loop_settings_modal.settings.use_custom_loop {
                        self.loop_settings_modal.settings.loop_start
                    } else {
                        None
                    };

                    let loop_end = if self.loop_settings_modal.settings.use_custom_loop {
                        self.loop_settings_modal.settings.loop_end
                    } else {
                        None
                    };

                    let use_custom_loop = self.loop_settings_modal.settings.use_custom_loop;

                    // Print debug information to help us understand the processing
                    println!(
                        "Processing replacement for audio: {} (ID: {})",
                        audio_info.name, audio_info.id
                    );

                    // Use the stored file path instead of asking the user to reselect the file
                    // Process the replacement with the confirmed loop settings
                    match ReplaceUtils::process_replacement_with_loop_settings(
                        audio_info,
                        None, // Pass None to use the stored file path
                        loop_start,
                        loop_end,
                        use_custom_loop,
                    ) {
                        Ok(new_audio_info) => {
                            // Update the audio file in memory
                            if let Some(ref mut audio_files) = self.audio_files {
                                if let Some(original_idx) = audio_files.iter().position(|f| {
                                    f.name == audio_info.name && f.id == audio_info.id
                                }) {
                                    // Replace with the new audio info
                                    audio_files[original_idx] = new_audio_info.clone();

                                    // Get the replacement audio data from our static HashMap
                                    if let Some(replacement_data) =
                                        ReplaceUtils::get_replacement_data(
                                            &audio_info.name,
                                            &audio_info.id,
                                        )
                                    {
                                        // Create an audio file struct for the audio player
                                        let audio = crate::ui::audio_player::AudioFile {
                                            file_path: file_path.to_string(),
                                            data: replacement_data,
                                            name: audio_info.name.clone(),
                                            file_type: audio_info.file_type.clone(),
                                            id: audio_info.id.clone(),
                                            #[cfg(target_arch = "wasm32")]
                                            temp_url: None,
                                        };

                                        // Update the audio player if it exists
                                        if let Some(audio_player) = &mut self.audio_player {
                                            let state = audio_player.get_audio_state();
                                            let mut state = state.lock().unwrap();
                                            state.set_audio(audio);

                                            // AudioPlayer.load_audio will automatically apply the loop settings for the specific audio file
                                            // Therefore we don't need to set the loop points here
                                        }
                                    }

                                    let loop_message = if use_custom_loop {
                                        let start_str = loop_start
                                            .map_or("start".to_string(), |s| format!("{:.2}s", s));
                                        let end_str = loop_end
                                            .map_or("end".to_string(), |e| format!("{:.2}s", e));
                                        format!(" (Loop: {} to {})", start_str, end_str)
                                    } else {
                                        " (Full track loop)".to_string()
                                    };

                                    toasts_to_add.push((
                                        format!(
                                            "Successfully replaced audio in memory: {}{}",
                                            audio_info.name, loop_message
                                        ),
                                        Color32::GREEN,
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            toasts_to_add.push((
                                format!("Failed to process replacement: {}", e),
                                Color32::RED,
                            ));

                            // Add more debug information when there is an error
                            println!("Replacement error details: {}", e);
                        }
                    }
                }
            }
        }

        // Add all collected toast messages at once
        for (message, color) in toasts_to_add {
            self.add_toast(message, color);
        }
    }
}

