use crate::localized;
use egui::{Color32, Ui, RichText};
use egui_phosphor::regular;

use super::{
    audio_file_info::AudioFileInfo, export_utils::ExportUtils, main_area_core::MainArea,
    replace_utils::ReplaceUtils, table_renderer::TableRenderer, add_audio_utils::AddAudioUtils, nus3audio_file_utils::Nus3audioFileUtils,
};
use crate::ui::audio_player::{AudioPlayerAction, LoopMode};

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
        let selected_count = self.selected_items.len();

        // Use these variables to capture action information outside the immediate UI context
        // This way we can perform actions after all UI rendering is done to avoid multiple self borrowing
        struct ActionData {
            export_index: Option<usize>,
            play_index: Option<usize>,
            replace_index: Option<usize>,
            remove_index: Option<usize>,
            export_all_confirm: bool,
            add_audio: bool,
            edit_grp_list: bool,
            edit_dton_tones: bool,
            edit_prop: bool,
            replace_new: bool,
            replace_empty: bool,
            remove_selected: bool,
            debug_convert_all_wav: bool,
        }

        let mut action_data = ActionData {
            export_index: None,
            play_index: None,
            replace_index: None,
            remove_index: None,
            export_all_confirm: false,
            add_audio: false,
            edit_grp_list: false,
            edit_dton_tones: false,
            edit_prop: false,
            replace_new: false,
            replace_empty: false,
            remove_selected: false,
            debug_convert_all_wav: false,
        };

        // First, render the UI - Actions Bar
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            
            // Primary Actions Group
            ui.label(RichText::new(localized::actions_colon()).weak().size(11.0));
            
            if ui.button(RichText::new(format!("{} {}", regular::PLUS, localized::add_audio_btn()))).on_hover_text(localized::add_audio_tooltip()).clicked() {
                action_data.add_audio = true;
            }
            
            if ui.button(RichText::new(format!("{} {}", regular::EXPORT, localized::export_all_btn()))).on_hover_text(localized::export_all_tooltip()).clicked() {
                action_data.export_all_confirm = true;
            }

            ui.separator();

            // Edit Group
            ui.label(RichText::new(localized::edit_colon()).weak().size(11.0));
            if ui.button("GRP").on_hover_text(localized::edit_grp_tooltip()).clicked() {
                action_data.edit_grp_list = true;
            }
            if ui.button("DTON").on_hover_text(localized::edit_dton_tooltip()).clicked() {
                action_data.edit_dton_tones = true;
            }
            if ui.button("PROP").on_hover_text(localized::edit_prop_tooltip()).clicked() {
                action_data.edit_prop = true;
            }

            ui.separator();

            // Batch Operations Group
            ui.label(RichText::new(localized::batch_colon()).weak().size(11.0));
            let batch_enabled = selected_count > 0;
            
            ui.add_enabled_ui(batch_enabled, |ui| {
                if ui.button(RichText::new(format!("{} {}", regular::FILE_ARROW_UP, localized::replace_btn()))).on_hover_text(localized::replace_selected_tooltip()).clicked() {
                    action_data.replace_new = true;
                }
                if ui.button(RichText::new(format!("{} {}", regular::ERASER, localized::clear_btn()))).on_hover_text(localized::clear_wav_tooltip()).clicked() {
                    action_data.replace_empty = true;
                }
                if ui.button(RichText::new(format!("{} {}", regular::TRASH, localized::remove_btn()))).on_hover_text(localized::remove_selected_tooltip()).clicked() {
                    action_data.remove_selected = true;
                }
            });
            
            ui.separator();

            // More Actions
            ui.menu_button(localized::more_menu(), |ui| {
                if ui.button(localized::debug_convert_all()).on_hover_text(localized::debug_convert_tooltip()).clicked() {
                    action_data.debug_convert_all_wav = true;
                    ui.close();
                }
            });

            // Right-aligned Info
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if selected_count > 0 {
                    ui.label(
                        RichText::new(localized::selected_count(selected_count))
                            .color(Color32::from_rgb(100, 150, 255))
                            .strong()
                    );
                }
                
                if !self.search_query.is_empty() {
                    ui.label(RichText::new(localized::found_count(files_count, self.file_count.unwrap_or(0))).weak());
                }
            });
        });

        ui.add_space(8.0);

        // Keep player queue aligned with the live table (search + sort order).
        if let Some(player) = self.audio_player.as_ref() {
            let state = player.get_audio_state();
            if let Ok(mut state) = state.lock() {
                state.sync_playlist_preserving_current(filtered_audio_files.clone());
            }
        }

        let now_playing_key = self.audio_player.as_ref().and_then(|player| {
            let state = player.get_audio_state();
            let state = state.lock().unwrap();
            state
                .current_audio
                .as_ref()
                .map(|audio| format!("{}:{}", audio.name, audio.id))
        });

        // The actual table rendering - capture actions but don't execute them yet
        TableRenderer::render_table(
            ui,
            &filtered_audio_files,
            &mut self.selected_rows,
            &mut self.selected_items,
            now_playing_key.as_deref(),
            self.striped,
            self.show_grid_lines,
            available_height - 40.0, // Account for actions bar
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

        // Map captured actions to class members for processing
        if action_data.replace_new {
            if let Some(ref audio_files) = self.audio_files {
                let mut representative: Option<AudioFileInfo> = None;
                for key in self.selected_items.iter() {
                    if let Some((name, id)) = key.split_once(':') {
                        if let Some(info) = audio_files.iter().find(|f| f.name == name && f.id == id) {
                            representative = Some(info.clone());
                            break;
                        }
                    }
                }

                if let Some(rep) = representative {
                    self.pause_main_player_for_preview();
                    match ReplaceUtils::replace_with_file_dialog(&rep, &mut self.loop_settings_modal) {
                        Ok(_) => {
                            self.pending_replace_new = true;
                        }
                        Err(e) => {
                            self.add_toast(localized::replace_failed(&e), Color32::RED);
                        }
                    }
                }
            }
        }
        
        if action_data.replace_empty {
            self.pending_replace_empty = true;
            self.confirm_modal.open(
                &localized::confirm_replace_empty_title(),
                &localized::confirm_replace_empty_body(selected_count),
            );
        }

        if action_data.remove_selected {
            self.pending_remove_selected = true;
            self.confirm_modal.open(
                &localized::confirm_remove_selected_title(),
                &localized::confirm_remove_selected_body(selected_count),
            );
        }

        if action_data.debug_convert_all_wav {
            if let Some(path) = self.selected_file.as_deref() {
                if path.to_lowercase().ends_with(".nus3bank") {
                    self.pending_debug_convert_all_wav = true;
                    self.confirm_modal.open(
                        &localized::debug_convert_all(),
                        &localized::confirm_debug_convert_body(),
                    );
                } else {
                    self.add_toast(
                        localized::debug_nus3bank_only().to_string(),
                        Color32::GOLD,
                    );
                }
            }
        }

        // Collect toast messages to add - we'll add them all at once to avoid multiple self.add_toast calls
        let mut toasts_to_add = Vec::new();

        // Process all actions and collect toast messages

        // Persistent row selection in the table uses checkboxes only (`selected_items`).

        // Handle "Add Audio" action if clicked
        if action_data.add_audio {
            let selected_file = self.selected_file.clone();
            
            if let Some(_file_path) = &selected_file {
                // Use AddAudioUtils to open file dialog and show add audio modal
                match AddAudioUtils::add_with_file_dialog(
                    &mut self.add_audio_modal,
                    self.audio_files.clone(),
                    crate::locale_from_ctx(ui.ctx()),
                ) {
                    Ok(_) => {
                        toasts_to_add.push((
                            localized::configure_new_audio_toast().to_string(),
                            Color32::GOLD,
                        ));
                    }
                    Err(e) => {
                        toasts_to_add.push((localized::add_audio_failed(&e), Color32::RED));
                    }
                }
            }
        }

        // Handle "Edit GRP List" action if clicked
        if action_data.edit_grp_list {
            if let Some(file_path) = self.selected_file.clone() {
                if file_path.to_lowercase().ends_with(".nus3bank") {
                    self.grp_list_modal.open_for_file(&file_path, crate::locale_from_ctx(ui.ctx()));
                } else {
                    toasts_to_add.push((
                        localized::grp_nus3bank_only().to_string(),
                        Color32::GOLD,
                    ));
                }
            } else {
                toasts_to_add.push((localized::no_file_selected().to_string(), Color32::GOLD));
            }
        }

        // Handle "Edit DTON Tones" action if clicked
        if action_data.edit_dton_tones {
            if let Some(file_path) = self.selected_file.clone() {
                if file_path.to_lowercase().ends_with(".nus3bank") {
                    self.dton_tones_modal.open_for_file(&file_path, crate::locale_from_ctx(ui.ctx()));
                } else {
                    toasts_to_add.push((
                        localized::dton_nus3bank_only().to_string(),
                        Color32::GOLD,
                    ));
                }
            } else {
                toasts_to_add.push((localized::no_file_selected().to_string(), Color32::GOLD));
            }
        }

        // Handle "Edit PROP" action if clicked
        if action_data.edit_prop {
            if let Some(file_path) = self.selected_file.clone() {
                if file_path.to_lowercase().ends_with(".nus3bank") {
                    self.prop_edit_modal.open_for_file(&file_path, crate::locale_from_ctx(ui.ctx()));
                } else {
                    toasts_to_add.push((
                        localized::prop_nus3bank_only().to_string(),
                        Color32::GOLD,
                    ));
                }
            } else {
                toasts_to_add.push((localized::no_file_selected().to_string(), Color32::GOLD));
            }
        }

        // Handle "Export All" confirm dialog if clicked
        if action_data.export_all_confirm {
            let file_count = if let Some(ref audio_files) = self.audio_files {
                audio_files.len()
            } else {
                0
            };
            
            // Set the pending export all flag
            self.pending_export_all = true;
            
            self.confirm_modal.open(
                &localized::confirm_export_all_title(),
                &localized::confirm_export_all_body(file_count),
            );
        }

        // Handle "Export" action for a specific file if clicked
        if let Some(idx) = action_data.export_index {
            if idx < filtered_audio_files.len() {
                let audio_info = &filtered_audio_files[idx];
                let selected_file = self.selected_file.clone();
                let output_path = self.output_path.clone();

                if let Some(file_path) = &selected_file {
                    if let Some(output_dir) = &output_path {
                        match ExportUtils::export_to_wav_with_custom_dir_unified(
                            audio_info, file_path, output_dir,
                        ) {
                            Ok(path) => {
                                toasts_to_add.push((
                                    localized::exported_to(path),
                                    Color32::GREEN,
                                ));
                            }
                            Err(e) => {
                                toasts_to_add.push((localized::export_failed(&e), Color32::RED));
                            }
                        }
                    } else {
                        toasts_to_add.push((
                            localized::no_output_dir().to_string(),
                            Color32::GOLD,
                        ));
                    }
                }
            }
        }

        // Handle "Play" action if clicked
        if let Some(idx) = action_data.play_index {
            if idx < filtered_audio_files.len() {
                let audio_info = &filtered_audio_files[idx];
                let audio_name = audio_info.name.clone();
                let file_path = self.selected_file.clone();

                log::info!("Play button clicked for audio: {} (id: {}, is_nus3bank: {})", 
                          audio_name, audio_info.id, audio_info.is_nus3bank);

                if let Some(path) = &file_path {
                    if let Some(audio_player) = &mut self.audio_player {
                        log::info!("Loading audio from file: {}", path);
                        match audio_player.load_audio(audio_info, path) {
                            Ok(()) => {
                                // Update playlist in audio state
                                let state = audio_player.get_audio_state();
                                {
                                    let mut state = state.lock().unwrap();
                                    state.update_playlist(filtered_audio_files.clone(), &audio_info.name, &audio_info.id);
                                    
                                    // Start playing
                                    if !state.is_playing {
                                        state.toggle_play();
                                    }
                                }

                                toasts_to_add
                                    .push((localized::now_playing(&audio_name), Color32::GREEN));
                                log::info!("Successfully started playing: {}", audio_name);
                            }
                            Err(e) => {
                                let error_msg = localized::failed_load_audio(&audio_name, &e);
                                log::error!("{}", error_msg);
                                toasts_to_add
                                    .push((error_msg, Color32::RED));
                            }
                        }
                    } else {
                        let error_msg = localized::audio_player_not_initialized().to_string();
                        log::error!("{}", error_msg);
                        toasts_to_add
                            .push((error_msg, Color32::RED));
                    }
                } else {
                    let error_msg = localized::no_file_for_playback().to_string();
                    log::error!("{}", error_msg);
                    toasts_to_add
                        .push((error_msg, Color32::RED));
                }
            } else {
                let error_msg = localized::invalid_audio_index(idx, filtered_audio_files.len());
                log::error!("{}", error_msg);
                toasts_to_add
                    .push((error_msg, Color32::RED));
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

                    // Ensure batch replace flag is cleared when doing single replace
                    self.pending_replace_new = false;

                    // Pause main player so replace preview transport can own audio output
                    self.pause_main_player_for_preview();

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
                                localized::configure_loop_for(&audio_info.name),
                                Color32::GOLD,
                            ));
                        }
                        Err(e) => {
                            toasts_to_add.push((localized::replace_failed(&e), Color32::RED));
                        }
                    }
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
                        &localized::confirm_title_default(),
                        &localized::confirm_delete_audio_body(&audio_info.name),
                    );
                }
            }
        }
        
        // Process the confirm dialog's confirmation action
        if self.confirm_modal.confirmed {
            // Reset the confirmed state
            self.confirm_modal.reset_state();
            
            // If there is a pending export all action, perform the export
            if self.pending_export_all {
                self.pending_export_all = false;
                
                let selected_file = self.selected_file.clone();
                let output_path = self.output_path.clone();

                if let Some(file_path) = &selected_file {
                    if let Some(output_dir) = &output_path {
                        // Use ExportUtils to export all files
                        match ExportUtils::export_all_to_wav_unified(file_path, output_dir) {
                            Ok(paths) => {
                                toasts_to_add.push((
                                    localized::exported_count_to(paths.len(), output_dir),
                                    Color32::GREEN,
                                ));
                            }
                            Err(e) => {
                                toasts_to_add.push((localized::export_failed(&e), Color32::RED));
                            }
                        }
                    } else {
                        toasts_to_add.push((
                            localized::no_output_dir().to_string(),
                            Color32::GOLD,
                        ));
                    }
                }
            }
            // If there is a pending replace with empty wav action, perform it
            else if self.pending_replace_empty {
                self.pending_replace_empty = false;

                if let Some(file_path) = &self.selected_file {
                    // Replace for each selected item that exists in current full list
                    let mut replaced = 0usize;
                    if let Some(ref mut audio_files) = self.audio_files {
                        // Build index by key for quick lookup
                        use std::collections::HashMap;
                        let mut index_by_key: HashMap<String, usize> = HashMap::new();
                        for (i, f) in audio_files.iter().enumerate() {
                            index_by_key.insert(format!("{}:{}", f.name, f.id), i);
                        }

                        for key in self.selected_items.clone().into_iter() {
                            if let Some(&idx) = index_by_key.get(&key) {
                                let audio_info = audio_files[idx].clone();
                                match ReplaceUtils::replace_with_empty_wav_in_memory(&audio_info, file_path) {
                                    Ok(new_info) => {
                                        audio_files[idx] = new_info;
                                        replaced += 1;
                                    }
                                    Err(e) => {
                                        toasts_to_add.push((localized::failed_replace_key(&key, &e), Color32::RED));
                                    }
                                }
                            }
                        }

                        // Update file count and notify
                        self.file_count = Some(audio_files.len());
                        if replaced > 0 {
                            // Clear all selected items after successful batch replacement
                            self.selected_items.clear();
                            
                            toasts_to_add.push((
                                localized::replaced_empty_wav(replaced),
                                Color32::GREEN,
                            ));
                        } else {
                            toasts_to_add.push((localized::no_matching_replace().to_string(), Color32::GOLD));
                        }
                    }
                }
            }
            // Debug: Convert all tracks to WAV (in memory)
            else if self.pending_debug_convert_all_wav {
                self.pending_debug_convert_all_wav = false;

                let selected_file_path = match self.selected_file.as_deref() {
                    Some(p) => p,
                    None => {
                        toasts_to_add.push((localized::no_file_selected().to_string(), Color32::GOLD));
                        return;
                    }
                };

                if !selected_file_path.to_lowercase().ends_with(".nus3bank") {
                    toasts_to_add.push((
                        localized::debug_convert_bank_only().to_string(),
                        Color32::GOLD,
                    ));
                    return;
                }

                // Load current bank to read original payloads.
                let bank = match crate::nus3bank::structures::Nus3bankFile::open(selected_file_path) {
                    Ok(f) => f,
                    Err(e) => {
                        toasts_to_add.push((localized::failed_open_bank(&e), Color32::RED));
                        return;
                    }
                };

                use std::collections::HashMap;
                let mut payload_by_hex: HashMap<String, Vec<u8>> = HashMap::new();
                for (i, tone) in bank.tone.tones.iter().enumerate() {
                    if tone.removed {
                        continue;
                    }
                    let hex_id = format!("0x{:x}", i as u32);
                    payload_by_hex.insert(hex_id, tone.payload.clone());
                }

                let mut converted = 0usize;
                let mut skipped = 0usize;
                let mut failed = 0usize;

                if let Some(ref mut audio_files) = self.audio_files {
                    for info in audio_files.iter_mut() {
                        if !info.is_nus3bank {
                            continue;
                        }
                        let hex_id = match info.hex_id.as_deref() {
                            Some(h) => h,
                            None => {
                                failed += 1;
                                continue;
                            }
                        };

                        let source = super::replace_utils::ReplaceUtils::get_replacement_data_unified(info)
                            .or_else(|| payload_by_hex.get(hex_id).cloned());

                        let Some(source_bytes) = source else {
                            failed += 1;
                            continue;
                        };

                        if super::replace_utils::ReplaceUtils::is_standard_pcm16_wav(&source_bytes) {
                            skipped += 1;
                            continue;
                        }

                        match super::replace_utils::ReplaceUtils::convert_audio_bytes_to_pcm_wav(&source_bytes) {
                            Ok(wav_bytes) => {
                                // Stage replacement for export/save.
                                let _ = crate::nus3bank::replace::Nus3bankReplacer::replace_track_in_memory(
                                    selected_file_path,
                                    hex_id,
                                    wav_bytes.clone(),
                                );
                                // Update playback replacement cache.
                                let key = format!("{}:{}", hex_id, info.name);
                                let _ = super::replace_utils::ReplaceUtils::store_audio_data_for_playback(
                                    key,
                                    wav_bytes.clone(),
                                );

                                info.size = wav_bytes.len();
                                info.file_type = "WAV".to_string();
                                converted += 1;
                            }
                            Err(e) => {
                                failed += 1;
                                toasts_to_add.push((localized::convert_failed_for(&info.name, &e), Color32::RED));
                            }
                        }
                    }
                }

                toasts_to_add.push((
                    localized::debug_convert_done(converted, skipped, failed),
                    if failed == 0 { Color32::GREEN } else { Color32::GOLD },
                ));
            }
            // If there is a pending remove-selected action, perform it
            else if self.pending_remove_selected {
                self.pending_remove_selected = false;

                let selected_file_path = match self.selected_file.as_deref() {
                    Some(p) => p,
                    None => {
                        toasts_to_add.push((localized::no_file_selected().to_string(), Color32::GOLD));
                        return;
                    }
                };

                if let Some(ref mut audio_files) = self.audio_files {
                    use std::collections::HashSet;

                    // Work on a stable snapshot of selected keys.
                    let selected_keys: Vec<String> = self.selected_items.iter().cloned().collect();
                    let mut removed_keys: HashSet<String> = HashSet::new();
                    let mut removed_count = 0usize;

                    for key in selected_keys {
                        let Some((name, id)) = key.split_once(':') else {
                            continue;
                        };

                        let Some(info) = audio_files.iter().find(|f| f.name == name && f.id == id).cloned() else {
                            continue;
                        };

                        match Nus3audioFileUtils::register_remove(&info, Some(selected_file_path)) {
                            Ok(_) => {
                                // Remove from the in-memory list
                                if let Some(pos) = audio_files.iter().position(|f| f.name == info.name && f.id == info.id) {
                                    audio_files.remove(pos);
                                    removed_count += 1;
                                    removed_keys.insert(format!("{}:{}", info.name, info.id));
                                }
                            }
                            Err(e) => {
                                toasts_to_add.push((localized::failed_mark_deletion(&e), Color32::RED));
                            }
                        }
                    }

                    // Update selection and file count
                    for k in removed_keys {
                        self.selected_items.remove(&k);
                    }
                    self.file_count = Some(audio_files.len());

                    if removed_count > 0 {
                        toasts_to_add.push((
                            localized::marked_for_deletion_count(removed_count),
                            Color32::GREEN,
                        ));
                    } else {
                        toasts_to_add.push((localized::no_matching_in_list().to_string(), Color32::GOLD));
                    }
                } else {
                    toasts_to_add.push((localized::no_audio_list().to_string(), Color32::GOLD));
                }
            }
            // If there is an audio to be removed, perform the removal
            else if let Some(audio_info) = &self.pending_remove_audio {
                if let Some(_file_path) = &self.selected_file {
                    println!(
                        "Confirmed removal of audio: {} (ID: {})",
                        audio_info.name, audio_info.id
                    );
                    
                    // Register the removal in memory only
                    match Nus3audioFileUtils::register_remove(audio_info, self.selected_file.as_deref()) {
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
                                    
                                    // Remove from persistent selection if present
                                    let key = format!("{}:{}", audio_info.name, audio_info.id);
                                    self.selected_items.remove(&key);
                                    
                                    toasts_to_add.push((
                                        localized::marked_deleted_one(&audio_info.name),
                                        Color32::GREEN,
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            toasts_to_add.push((localized::failed_mark_deletion(&e), Color32::RED));
                        }
                    }
                    
                    // Clear the audio info to be removed
                    self.pending_remove_audio = None;
                }
            }
        } else if self.confirm_modal.cancelled {
            // Process the case of cancelling the action
            self.confirm_modal.reset_state();
            
            if self.pending_export_all {
                self.pending_export_all = false;
            } else if self.pending_replace_empty {
                self.pending_replace_empty = false;
            } else if self.pending_debug_convert_all_wav {
                self.pending_debug_convert_all_wav = false;
            } else if self.pending_remove_selected {
                self.pending_remove_selected = false;
            } else if let Some(_audio_info) = &self.pending_remove_audio {
                // Clear the audio info to be removed
                self.pending_remove_audio = None;
            }
        }

        // Check if add audio modal was confirmed
        if self.add_audio_modal.confirmed {
            // Reset the confirmed flag
            self.add_audio_modal.confirmed = false;

            // Get the selected file
            if let Some(_file_path) = &self.selected_file {
                // 1. 获取原始文件路径
                let original_file_path = match &self.add_audio_modal.settings.file_path {
                    Some(path) => path,
                    None => {
                        toasts_to_add.push((localized::no_audio_path().to_string(), Color32::RED));
                        return;
                    }
                };
                
                // 2. 确定文件类型 - 检查当前选择的文件是否为NUS3BANK
                let selected_file_path = self.selected_file.as_ref().unwrap();
                let is_nus3bank = selected_file_path.to_lowercase().ends_with(".nus3bank");
                
                // 处理新音频文件
                match AddAudioUtils::process_new_audio(&self.add_audio_modal, is_nus3bank, crate::locale_from_ctx(ui.ctx()))
                {
                    Ok(new_audio_info) => {
                        // 3. 尝试将音频转换为WAV格式
                        match AddAudioUtils::convert_to_wav(original_file_path) {
                            Ok(wav_data) => {
                                // 4. 使用转换后的WAV数据注册添加操作
                                let register_result = if new_audio_info.is_nus3bank {
                                    let selected_file_path = self.selected_file.as_ref().unwrap();
                                    Nus3audioFileUtils::register_add_nus3bank(selected_file_path, &new_audio_info, wav_data)
                                } else {
                                    Nus3audioFileUtils::register_add_audio(&new_audio_info, wav_data)
                                };
                                
                                match register_result {
                                    Ok(_) => {
                                        // 5. 更新内存中的音频文件列表
                                        if let Some(ref mut audio_files) = self.audio_files {
                                            audio_files.push(new_audio_info.clone());
                                            self.file_count = Some(audio_files.len());
                                            toasts_to_add.push((
                                                localized::added_wav(&new_audio_info.name),
                                                Color32::GREEN,
                                            ));
                                        }
                                    },
                                    Err(e) => {
                                        toasts_to_add.push((localized::register_wav_failed(&e), Color32::RED));
                                    }
                                }
                            },
                            Err(e) => {
                                // 6. 如果WAV转换失败，回退到使用原始音频数据
                                println!("Warning: Failed to convert to WAV: {}", e);
                                println!("Falling back to original file data");
                                
                                if let Some(data) = &self.add_audio_modal.file_data {
                                    let fallback_result = if new_audio_info.is_nus3bank {
                                        let selected_file_path = self.selected_file.as_ref().unwrap();
                                        Nus3audioFileUtils::register_add_nus3bank(selected_file_path, &new_audio_info, data.clone())
                                    } else {
                                        Nus3audioFileUtils::register_add_audio(&new_audio_info, data.clone())
                                    };
                                    
                                    match fallback_result {
                                        Ok(_) => {
                                            if let Some(ref mut audio_files) = self.audio_files {
                                                audio_files.push(new_audio_info.clone());
                                                self.file_count = Some(audio_files.len());
                                                toasts_to_add.push((
                                                    localized::added_original(&new_audio_info.name),
                                                    Color32::GREEN,
                                                ));
                                            }
                                        },
                                        Err(e) => {
                                            toasts_to_add.push((localized::failed_add_audio(&e), Color32::RED));
                                        }
                                    }
                                } else {
                                    toasts_to_add.push((localized::no_audio_data().to_string(), Color32::RED));
                                }
                            }
                        }
                    },
                    Err(e) => {
                        toasts_to_add.push((localized::failed_process_new_audio(&e), Color32::RED));
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
                    // Get loop settings from the modal (always materialize A/B when custom is on)
                    let use_custom_loop = self.loop_settings_modal.settings.use_custom_loop;
                    let enable_loop = self.loop_settings_modal.settings.enable_loop;
                    let duration_hint = self
                        .loop_settings_modal
                        .settings
                        .estimated_duration
                        .max(0.0);
                    let loop_start = if use_custom_loop {
                        Some(
                            self.loop_settings_modal
                                .settings
                                .loop_start
                                .unwrap_or(0.0),
                        )
                    } else {
                        None
                    };
                    let loop_end = if use_custom_loop {
                        Some(
                            self.loop_settings_modal
                                .settings
                                .loop_end
                                .unwrap_or(duration_hint)
                                .max(loop_start.unwrap_or(0.0)),
                        )
                    } else {
                        None
                    };

                    if self.pending_replace_new {
                        // Batch replace for all selected items using the chosen file and loop settings
                        self.pending_replace_new = false;

                        // Retrieve the file path chosen during the dialog (from representative)
                        let rep_path_opt = ReplaceUtils::get_replacement_path(&audio_info.name, &audio_info.id);
                        if rep_path_opt.is_none() {
                            toasts_to_add.push((localized::no_replacement_path().to_string(), Color32::RED));
                            return;
                        }
                        let rep_path = rep_path_opt.unwrap();
                        let rep_path_ref = rep_path.as_path();

                        if let Some(ref mut audio_files) = self.audio_files {
                            use std::collections::HashMap;
                            let mut index_by_key: HashMap<String, usize> = HashMap::new();
                            for (i, f) in audio_files.iter().enumerate() {
                                index_by_key.insert(format!("{}:{}", f.name, f.id), i);
                            }

                            let mut replaced_count: usize = 0;
                            for key in self.selected_items.clone().into_iter() {
                                if let Some(&idx) = index_by_key.get(&key) {
                                    let target_info = audio_files[idx].clone();
                                    match ReplaceUtils::process_replacement_with_loop_settings(
                                        &target_info,
                                        Some(rep_path_ref),
                                        loop_start,
                                        loop_end,
                                        use_custom_loop,
                                        enable_loop,
                                        self.loop_settings_modal.settings.gain_db,
                                    ) {
                                        Ok(new_audio_info) => {
                                            audio_files[idx] = new_audio_info;
                                            replaced_count += 1;
                                        }
                                        Err(e) => {
                                            toasts_to_add.push((localized::failed_process_replacement_key(&key, &e), Color32::RED));
                                        }
                                    }
                                }
                            }

                            self.file_count = Some(audio_files.len());

                            let loop_message = if use_custom_loop {
                                let start_str = loop_start.map_or(localized::loop_word_start().to_string(), |s| format!("{:.2}s", s));
                                let end_str = loop_end.map_or(localized::loop_word_end().to_string(), |e| format!("{:.2}s", e));
                                localized::loop_parenthetical_range(&start_str, &end_str)
                            } else {
                                localized::loop_parenthetical_full().to_string()
                            };

                            if replaced_count > 0 {
                                // Play the representative track via full load (uses in-memory replacement)
                                if let Some(audio_player) = &mut self.audio_player {
                                    if let Err(e) = audio_player.load_audio(audio_info, file_path)
                                    {
                                        log::error!(
                                            "Failed to play replacement after batch: {}",
                                            e
                                        );
                                        toasts_to_add.push((
                                            localized::prepare_playback_audio_failed().to_string(),
                                            Color32::RED,
                                        ));
                                    }
                                }

                                // Clear all selected items after successful batch replacement
                                self.selected_items.clear();

                                toasts_to_add.push((
                                    localized::replaced_in_memory_count(replaced_count, &loop_message),
                                    Color32::GREEN,
                                ));
                            } else {
                                toasts_to_add.push((localized::no_matching_replace().to_string(), Color32::GOLD));
                            }
                        }
                    } else {
                        // Single item flow (existing behavior)
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
                            enable_loop,
                            self.loop_settings_modal.settings.gain_db,
                        ) {
                            Ok(new_audio_info) => {
                                // Update the audio file in memory
                                if let Some(ref mut audio_files) = self.audio_files {
                                    if let Some(original_idx) = audio_files.iter().position(|f| {
                                        f.name == audio_info.name && f.id == audio_info.id
                                    }) {
                                        // Replace with the new audio info
                                        audio_files[original_idx] = new_audio_info.clone();

                                        // Full load path: reads REPLACED_AUDIO_DATA, writes temp, waveform, loop
                                        if let Some(audio_player) = &mut self.audio_player {
                                            match audio_player
                                                .load_audio(&new_audio_info, file_path)
                                            {
                                                Ok(()) => {
                                                    println!(
                                                        "Playback after replace OK: {} ({} bytes)",
                                                        new_audio_info.name, new_audio_info.size
                                                    );
                                                }
                                                Err(e) => {
                                                    log::error!(
                                                        "Failed to play replacement audio: {}",
                                                        e
                                                    );
                                                    toasts_to_add.push((
                                                        localized::prepare_playback_audio_failed()
                                                            .to_string(),
                                                        Color32::RED,
                                                    ));
                                                }
                                            }
                                        }

                                        let loop_message = if use_custom_loop {
                                            let start_str = loop_start
                                                .map_or(localized::loop_word_start().to_string(), |s| format!("{:.2}s", s));
                                            let end_str = loop_end
                                                .map_or(localized::loop_word_end().to_string(), |e| format!("{:.2}s", e));
                                            localized::loop_parenthetical_range(&start_str, &end_str)
                                        } else {
                                            localized::loop_parenthetical_full().to_string()
                                        };

                                        toasts_to_add.push((
                                            localized::replaced_in_memory_one(&audio_info.name, &loop_message),
                                            Color32::GREEN,
                                        ));
                                    }
                                }
                            }
                            Err(e) => {
                                toasts_to_add.push((
                                    localized::failed_process_replacement(&e),
                                    Color32::RED,
                                ));

                                // Add more debug information when there is an error
                                println!("Replacement error details: {}", e);
                            }
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

    /// Handle actions from the audio player (next/previous track)
    pub fn handle_audio_player_action(&mut self, action: AudioPlayerAction) {
        match action {
            AudioPlayerAction::None => {}
            AudioPlayerAction::PlayNext => {
                self.play_next_track();
            }
            AudioPlayerAction::PlayPrevious => {
                self.play_previous_track();
            }
            AudioPlayerAction::PlaylistEnded => {
                self.add_toast(localized::playlist_ended(), Color32::from_rgb(220, 180, 80));
            }
        }
    }

    fn play_next_track(&mut self) {
        // Always re-sync to current table order (search + sort) before advancing.
        let visible = self.filtered_audio_files();
        let decision: Result<(usize, AudioFileInfo), String> = {
            let Some(player) = self.audio_player.as_ref() else {
                return;
            };
            let state = player.get_audio_state();
            let mut state = state.lock().unwrap();
            state.sync_playlist_preserving_current(visible);

            if state.playlist.is_empty() {
                Err(localized::no_tracks_in_queue())
            } else {
                let next_index = if state.shuffle {
                    state.take_shuffle_next_index()
                } else {
                    match state.current_track_index {
                        Some(current_index) => {
                            let next = current_index + 1;
                            if next < state.playlist.len() {
                                Some(next)
                            } else {
                                match state.loop_mode {
                                    // Manual next wraps on All; Single does not trap the user.
                                    LoopMode::All => Some(0),
                                    LoopMode::None | LoopMode::Single => None,
                                }
                            }
                        }
                        // Current track filtered out of the list — jump to first visible
                        None => Some(0),
                    }
                };

                match next_index {
                    Some(idx) if idx < state.playlist.len() => {
                        Ok((idx, state.playlist[idx].clone()))
                    }
                    _ => Err(localized::playlist_ended()),
                }
            }
        };

        match decision {
            Ok((next_index, next_track)) => {
                self.load_queue_track(next_index, &next_track);
            }
            Err(msg) => {
                self.add_toast(msg, Color32::from_rgb(220, 180, 80));
            }
        }
    }

    fn play_previous_track(&mut self) {
        let visible = self.filtered_audio_files();
        let decision: Result<(usize, AudioFileInfo), String> = {
            let Some(player) = self.audio_player.as_ref() else {
                return;
            };
            let state = player.get_audio_state();
            let mut state = state.lock().unwrap();
            state.sync_playlist_preserving_current(visible);

            if state.playlist.is_empty() {
                Err(localized::no_tracks_in_queue())
            } else {
                let prev_index = if state.shuffle {
                    // Shuffle: previous draws another bag entry (no immediate repeat)
                    state.take_shuffle_next_index()
                } else {
                    match state.current_track_index {
                        Some(0) => match state.loop_mode {
                            LoopMode::All => Some(state.playlist.len() - 1),
                            LoopMode::None | LoopMode::Single => Some(0),
                        },
                        Some(current_index) => Some(current_index - 1),
                        None => Some(0),
                    }
                };

                match prev_index {
                    Some(idx) if idx < state.playlist.len() => {
                        Ok((idx, state.playlist[idx].clone()))
                    }
                    _ => Err(localized::no_tracks_in_queue()),
                }
            }
        };

        match decision {
            Ok((prev_index, prev_track)) => {
                self.load_queue_track(prev_index, &prev_track);
            }
            Err(msg) => {
                self.add_toast(msg, Color32::from_rgb(220, 180, 80));
            }
        }
    }

    fn load_queue_track(&mut self, index: usize, track: &AudioFileInfo) {
        let file_path = self.selected_file.clone();
        if let Some(path) = file_path {
            if let Some(player) = &mut self.audio_player {
                if let Ok(()) = player.load_audio(track, &path) {
                    let state = player.get_audio_state();
                    if let Ok(mut state) = state.lock() {
                        state.current_track_index = Some(index);
                    }
                    self.add_toast(localized::now_playing(&track.name), Color32::GREEN);
                }
            }
        }
    }
}


