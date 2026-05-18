//! UI strings resolved with rust-i18n `t!` at compile time (`locales/*.yml`).
//! Global locale is set each frame via [`crate::sync_rust_i18n_locale`].

use std::fmt::Display;

#[inline]
pub fn prop_custom_preset_name(project: &str, layout_label: &str) -> String {
    rust_i18n::t!(
        "prop_custom_preset_name",
        project = project,
        layout = layout_label
    )
    .to_string()
}

#[inline]
pub fn window_title() -> String {
    rust_i18n::t!("window_title").to_string()
}

#[inline]
pub fn ok() -> String {
    rust_i18n::t!("ok").to_string()
}

#[inline]
pub fn cancel() -> String {
    rust_i18n::t!("cancel").to_string()
}

#[inline]
pub fn confirm() -> String {
    rust_i18n::t!("confirm").to_string()
}

#[inline]
pub fn confirm_title_default() -> String {
    rust_i18n::t!("confirm_title_default").to_string()
}

#[inline]
pub fn file_menu() -> String {
    rust_i18n::t!("file_menu").to_string()
}

#[inline]
pub fn save_changes() -> String {
    rust_i18n::t!("save_changes").to_string()
}

#[inline]
pub fn save_nus3bank() -> String {
    rust_i18n::t!("save_nus3bank").to_string()
}

#[inline]
pub fn save_nus3audio() -> String {
    rust_i18n::t!("save_nus3audio").to_string()
}

#[inline]
pub fn save_file_generic() -> String {
    rust_i18n::t!("save_file_generic").to_string()
}

#[inline]
pub fn settings_menu() -> String {
    rust_i18n::t!("settings_menu").to_string()
}

#[inline]
pub fn reset_layout() -> String {
    rust_i18n::t!("reset_layout").to_string()
}

#[inline]
pub fn language() -> String {
    rust_i18n::t!("language").to_string()
}

#[inline]
pub fn language_english() -> String {
    rust_i18n::t!("language_english").to_string()
}

#[inline]
pub fn language_chinese() -> String {
    rust_i18n::t!("language_chinese").to_string()
}

#[inline]
pub fn help_menu() -> String {
    rust_i18n::t!("help_menu").to_string()
}

#[inline]
pub fn about() -> String {
    rust_i18n::t!("about").to_string()
}

#[inline]
pub fn save_failed_title() -> String {
    rust_i18n::t!("save_failed_title").to_string()
}

#[inline]
pub fn no_file_selected_save() -> String {
    rust_i18n::t!("no_file_selected_save").to_string()
}

#[inline]
pub fn no_changes_title() -> String {
    rust_i18n::t!("no_changes_title").to_string()
}

#[inline]
pub fn no_pending_changes() -> String {
    rust_i18n::t!("no_pending_changes").to_string()
}

#[inline]
pub fn save_successful_title() -> String {
    rust_i18n::t!("save_successful_title").to_string()
}

#[inline]
pub fn save_success_body(count: usize, path: &str) -> String {
    rust_i18n::t!("save_success_body", count = count, path = path).to_string()
}

#[inline]
pub fn save_failed_msg(err: &str) -> String {
    rust_i18n::t!("save_failed_msg", err = err).to_string()
}

#[inline]
pub fn no_file_selected_save_as() -> String {
    rust_i18n::t!("no_file_selected_save_as").to_string()
}

#[inline]
pub fn layout_reset_title() -> String {
    rust_i18n::t!("layout_reset_title").to_string()
}

#[inline]
pub fn layout_reset_msg() -> String {
    rust_i18n::t!("layout_reset_msg").to_string()
}

#[inline]
pub fn about_title() -> String {
    rust_i18n::t!("about_title").to_string()
}

#[inline]
pub fn about_body(version: &str) -> String {
    rust_i18n::t!("about_body", version = version).to_string()
}

#[inline]
pub fn source_link_label() -> String {
    rust_i18n::t!("source_link_label").to_string()
}

#[inline]
pub fn update_available_title() -> String {
    rust_i18n::t!("update_available_title").to_string()
}

#[inline]
pub fn update_available_body(current: &str, latest: &str) -> String {
    rust_i18n::t!("update_available_body", current = current, latest = latest).to_string()
}

#[inline]
pub fn download_latest() -> String {
    rust_i18n::t!("download_latest").to_string()
}

#[inline]
pub fn save_success_export_title() -> String {
    rust_i18n::t!("save_success_export_title").to_string()
}

#[inline]
pub fn save_success_export_body(path: &str) -> String {
    rust_i18n::t!("save_success_export_body", path = path).to_string()
}

#[inline]
pub fn save_failed_export(err: &str) -> String {
    rust_i18n::t!("save_failed_export", err = err).to_string()
}

#[inline]
pub fn files_heading() -> String {
    rust_i18n::t!("files_heading").to_string()
}

#[inline]
pub fn add_files_tooltip() -> String {
    rust_i18n::t!("add_files_tooltip").to_string()
}

#[inline]
pub fn select_audio_files_title() -> String {
    rust_i18n::t!("select_audio_files_title").to_string()
}

#[inline]
pub fn select_audio_file_to_add() -> String {
    rust_i18n::t!("select_audio_file_to_add").to_string()
}

#[inline]
pub fn all_files_filter() -> String {
    rust_i18n::t!("all_files_filter").to_string()
}

#[inline]
pub fn audio_files_filter() -> String {
    rust_i18n::t!("audio_files_filter").to_string()
}

#[inline]
pub fn clear_all_files_title() -> String {
    rust_i18n::t!("clear_all_files_title").to_string()
}

#[inline]
pub fn clear_all_files_confirm(n: usize) -> String {
    rust_i18n::t!("clear_all_files_confirm", n = n).to_string()
}

#[inline]
pub fn search_files_hint() -> String {
    rust_i18n::t!("search_files_hint").to_string()
}

#[inline]
pub fn clear_search_tooltip() -> String {
    rust_i18n::t!("clear_search_tooltip").to_string()
}

#[inline]
pub fn no_files_added() -> String {
    rust_i18n::t!("no_files_added").to_string()
}

#[inline]
pub fn no_matching_files() -> String {
    rust_i18n::t!("no_matching_files").to_string()
}

#[inline]
pub fn remove_from_list_tooltip() -> String {
    rust_i18n::t!("remove_from_list_tooltip").to_string()
}

#[inline]
pub fn loading_audio() -> String {
    rust_i18n::t!("loading_audio").to_string()
}

#[inline]
pub fn no_file_selected_heading() -> String {
    rust_i18n::t!("no_file_selected_heading").to_string()
}

#[inline]
pub fn no_file_selected_hint() -> String {
    rust_i18n::t!("no_file_selected_hint").to_string()
}

#[inline]
pub fn audio_editor_heading() -> String {
    rust_i18n::t!("audio_editor_heading").to_string()
}

#[inline]
pub fn currently_editing() -> String {
    rust_i18n::t!("currently_editing").to_string()
}

#[inline]
pub fn audio_files_found(n: usize) -> String {
    rust_i18n::t!("audio_files_found", n = n).to_string()
}

#[inline]
pub fn refresh() -> String {
    rust_i18n::t!("refresh").to_string()
}

#[inline]
pub fn export_to() -> String {
    rust_i18n::t!("export_to").to_string()
}

#[inline]
pub fn output_folder_not_set() -> String {
    rust_i18n::t!("output_folder_not_set").to_string()
}

#[inline]
pub fn output_folder_hover() -> String {
    rust_i18n::t!("output_folder_hover").to_string()
}

#[inline]
pub fn browse() -> String {
    rust_i18n::t!("browse").to_string()
}

#[inline]
pub fn select_output_directory() -> String {
    rust_i18n::t!("select_output_directory").to_string()
}

#[inline]
pub fn clear_output_path_tooltip() -> String {
    rust_i18n::t!("clear_output_path_tooltip").to_string()
}

#[inline]
pub fn simple_view() -> String {
    rust_i18n::t!("simple_view").to_string()
}

#[inline]
pub fn advanced_search() -> String {
    rust_i18n::t!("advanced_search").to_string()
}

#[inline]
pub fn search_in() -> String {
    rust_i18n::t!("search_in").to_string()
}

#[inline]
pub fn tip_size_search() -> String {
    rust_i18n::t!("tip_size_search").to_string()
}

#[inline]
pub fn search_audio_hint() -> String {
    rust_i18n::t!("search_audio_hint").to_string()
}

#[inline]
pub fn clear_search_main_tooltip() -> String {
    rust_i18n::t!("clear_search_main_tooltip").to_string()
}

#[inline]
pub fn actions_colon() -> String {
    rust_i18n::t!("actions_colon").to_string()
}

#[inline]
pub fn add_audio_btn() -> String {
    rust_i18n::t!("add_audio_btn").to_string()
}

#[inline]
pub fn add_audio_tooltip() -> String {
    rust_i18n::t!("add_audio_tooltip").to_string()
}

#[inline]
pub fn export_all_btn() -> String {
    rust_i18n::t!("export_all_btn").to_string()
}

#[inline]
pub fn export_all_tooltip() -> String {
    rust_i18n::t!("export_all_tooltip").to_string()
}

#[inline]
pub fn edit_colon() -> String {
    rust_i18n::t!("edit_colon").to_string()
}

#[inline]
pub fn edit_grp_tooltip() -> String {
    rust_i18n::t!("edit_grp_tooltip").to_string()
}

#[inline]
pub fn edit_dton_tooltip() -> String {
    rust_i18n::t!("edit_dton_tooltip").to_string()
}

#[inline]
pub fn edit_prop_tooltip() -> String {
    rust_i18n::t!("edit_prop_tooltip").to_string()
}

#[inline]
pub fn batch_colon() -> String {
    rust_i18n::t!("batch_colon").to_string()
}

#[inline]
pub fn replace_btn() -> String {
    rust_i18n::t!("replace_btn").to_string()
}

#[inline]
pub fn replace_selected_tooltip() -> String {
    rust_i18n::t!("replace_selected_tooltip").to_string()
}

#[inline]
pub fn clear_btn() -> String {
    rust_i18n::t!("clear_btn").to_string()
}

#[inline]
pub fn clear_wav_tooltip() -> String {
    rust_i18n::t!("clear_wav_tooltip").to_string()
}

#[inline]
pub fn remove_btn() -> String {
    rust_i18n::t!("remove_btn").to_string()
}

#[inline]
pub fn remove_selected_tooltip() -> String {
    rust_i18n::t!("remove_selected_tooltip").to_string()
}

#[inline]
pub fn more_menu() -> String {
    rust_i18n::t!("more_menu").to_string()
}

#[inline]
pub fn debug_convert_all() -> String {
    rust_i18n::t!("debug_convert_all").to_string()
}

#[inline]
pub fn debug_convert_tooltip() -> String {
    rust_i18n::t!("debug_convert_tooltip").to_string()
}

#[inline]
pub fn selected_count(n: usize) -> String {
    rust_i18n::t!("selected_count", n = n).to_string()
}

#[inline]
pub fn found_count(found: usize, total: usize) -> String {
    rust_i18n::t!("found_count", found = found, total = total).to_string()
}

#[inline]
pub fn select_all_filtered_tooltip() -> String {
    rust_i18n::t!("select_all_filtered_tooltip").to_string()
}

#[inline]
pub fn col_name() -> String {
    rust_i18n::t!("col_name").to_string()
}

#[inline]
pub fn col_id() -> String {
    rust_i18n::t!("col_id").to_string()
}

#[inline]
pub fn col_size() -> String {
    rust_i18n::t!("col_size").to_string()
}

#[inline]
pub fn col_filename() -> String {
    rust_i18n::t!("col_filename").to_string()
}

#[inline]
pub fn col_type() -> String {
    rust_i18n::t!("col_type").to_string()
}

#[inline]
pub fn col_action() -> String {
    rust_i18n::t!("col_action").to_string()
}

#[inline]
pub fn play_tooltip() -> String {
    rust_i18n::t!("play_tooltip").to_string()
}

#[inline]
pub fn export_tooltip() -> String {
    rust_i18n::t!("export_tooltip").to_string()
}

#[inline]
pub fn replace_tooltip() -> String {
    rust_i18n::t!("replace_tooltip").to_string()
}

#[inline]
pub fn remove_tooltip() -> String {
    rust_i18n::t!("remove_tooltip").to_string()
}

#[inline]
pub fn more_label() -> String {
    rust_i18n::t!("more_label").to_string()
}

#[inline]
pub fn no_track_selected() -> String {
    rust_i18n::t!("no_track_selected").to_string()
}

#[inline]
pub fn shuffle_tooltip() -> String {
    rust_i18n::t!("shuffle_tooltip").to_string()
}

#[inline]
pub fn previous_track_tooltip() -> String {
    rust_i18n::t!("previous_track_tooltip").to_string()
}

#[inline]
pub fn pause_tooltip() -> String {
    rust_i18n::t!("pause_tooltip").to_string()
}

#[inline]
pub fn play_tooltip_player() -> String {
    rust_i18n::t!("play_tooltip_player").to_string()
}

#[inline]
pub fn next_track_tooltip() -> String {
    rust_i18n::t!("next_track_tooltip").to_string()
}

#[inline]
pub fn loop_off_tooltip() -> String {
    rust_i18n::t!("loop_off_tooltip").to_string()
}

#[inline]
pub fn loop_all_tooltip() -> String {
    rust_i18n::t!("loop_all_tooltip").to_string()
}

#[inline]
pub fn loop_one_tooltip() -> String {
    rust_i18n::t!("loop_one_tooltip").to_string()
}

#[inline]
pub fn stop_playback_tooltip() -> String {
    rust_i18n::t!("stop_playback_tooltip").to_string()
}

#[inline]
pub fn loop_settings_title(name: &str) -> String {
    rust_i18n::t!("loop_settings_title", name = name).to_string()
}

#[inline]
pub fn audio_information() -> String {
    rust_i18n::t!("audio_information").to_string()
}

#[inline]
pub fn name_label() -> String {
    rust_i18n::t!("name_label").to_string()
}

#[inline]
pub fn loop_settings_heading() -> String {
    rust_i18n::t!("loop_settings_heading").to_string()
}

#[inline]
pub fn enable_loop() -> String {
    rust_i18n::t!("enable_loop").to_string()
}

#[inline]
pub fn use_custom_loop() -> String {
    rust_i18n::t!("use_custom_loop").to_string()
}

#[inline]
pub fn loop_start_sec() -> String {
    rust_i18n::t!("loop_start_sec").to_string()
}

#[inline]
pub fn loop_end_sec() -> String {
    rust_i18n::t!("loop_end_sec").to_string()
}

#[inline]
pub fn loop_duration_sec(d: f32) -> String {
    rust_i18n::t!("loop_duration_sec", d = d : {:.2}).to_string()
}

#[inline]
pub fn loop_full_track() -> String {
    rust_i18n::t!("loop_full_track").to_string()
}

#[inline]
pub fn loop_disabled() -> String {
    rust_i18n::t!("loop_disabled").to_string()
}

#[inline]
pub fn gain_heading() -> String {
    rust_i18n::t!("gain_heading").to_string()
}

#[inline]
pub fn gain_db_label() -> String {
    rust_i18n::t!("gain_db_label").to_string()
}

#[inline]
pub fn reset_gain() -> String {
    rust_i18n::t!("reset_gain").to_string()
}

#[inline]
pub fn linear_factor(f: f32) -> String {
    rust_i18n::t!("linear_factor", f = f : {:.3}).to_string()
}

#[inline]
pub fn add_new_audio_title() -> String {
    rust_i18n::t!("add_new_audio_title").to_string()
}

#[inline]
pub fn file_information() -> String {
    rust_i18n::t!("file_information").to_string()
}

#[inline]
pub fn selected_file_label() -> String {
    rust_i18n::t!("selected_file_label").to_string()
}

#[inline]
pub fn duration_label() -> String {
    rust_i18n::t!("duration_label").to_string()
}

#[inline]
pub fn seconds_fmt(s: f32) -> String {
    rust_i18n::t!("seconds_fmt", s = s : {:.2}).to_string()
}

#[inline]
pub fn audio_metadata() -> String {
    rust_i18n::t!("audio_metadata").to_string()
}

#[inline]
pub fn id_label() -> String {
    rust_i18n::t!("id_label").to_string()
}

#[inline]
pub fn error_label() -> String {
    rust_i18n::t!("error_label").to_string()
}

#[inline]
pub fn name_exists_error() -> String {
    rust_i18n::t!("name_exists_error").to_string()
}

#[inline]
pub fn id_exists_error() -> String {
    rust_i18n::t!("id_exists_error").to_string()
}

#[inline]
pub fn no_audio_loaded() -> String {
    rust_i18n::t!("no_audio_loaded").to_string()
}

#[inline]
pub fn failed_read_audio(err: impl Display) -> String {
    rust_i18n::t!("failed_read_audio", err = err).to_string()
}

#[inline]
pub fn edit_grp_list_title() -> String {
    rust_i18n::t!("edit_grp_list_title").to_string()
}

#[inline]
pub fn grp_names_editor() -> String {
    rust_i18n::t!("grp_names_editor").to_string()
}

#[inline]
pub fn file_label_fmt(path: &str) -> String {
    rust_i18n::t!("file_label_fmt", path = path).to_string()
}

#[inline]
pub fn no_file_selected_short() -> String {
    rust_i18n::t!("no_file_selected_short").to_string()
}

#[inline]
pub fn search_label() -> String {
    rust_i18n::t!("search_label").to_string()
}

#[inline]
pub fn total_label(n: usize) -> String {
    rust_i18n::t!("total_label", n = n).to_string()
}

#[inline]
pub fn visible_label(n: usize) -> String {
    rust_i18n::t!("visible_label", n = n).to_string()
}

#[inline]
pub fn find_label() -> String {
    rust_i18n::t!("find_label").to_string()
}

#[inline]
pub fn replace_label() -> String {
    rust_i18n::t!("replace_label").to_string()
}

#[inline]
pub fn replace_in_visible() -> String {
    rust_i18n::t!("replace_in_visible").to_string()
}

#[inline]
pub fn add_row() -> String {
    rust_i18n::t!("add_row").to_string()
}

#[inline]
pub fn replace_with_template() -> String {
    rust_i18n::t!("replace_with_template").to_string()
}

#[inline]
pub fn reload_from_file() -> String {
    rust_i18n::t!("reload_from_file").to_string()
}

#[inline]
pub fn nus3bank_open_failed(err: impl Display) -> String {
    rust_i18n::t!("nus3bank_open_failed", err = err).to_string()
}

#[inline]
pub fn grp_find_text_empty() -> String {
    rust_i18n::t!("grp_find_text_empty").to_string()
}

#[inline]
pub fn grp_no_file_for_edit() -> String {
    rust_i18n::t!("grp_no_file_for_edit").to_string()
}

#[inline]
pub fn grp_template_empty() -> String {
    rust_i18n::t!("grp_template_empty").to_string()
}

#[inline]
pub fn dton_no_file_for_edit() -> String {
    rust_i18n::t!("dton_no_file_for_edit").to_string()
}

#[inline]
pub fn prop_no_file_for_edit() -> String {
    rust_i18n::t!("prop_no_file_for_edit").to_string()
}

#[inline]
pub fn dton_len_label(len: usize) -> String {
    rust_i18n::t!("dton_len_label", len = len).to_string()
}

#[inline]
pub fn dton_original_len(n: usize) -> String {
    rust_i18n::t!("dton_original_len", n = n).to_string()
}

#[inline]
pub fn data_length_mismatch(got: usize, expected: usize) -> String {
    rust_i18n::t!("data_length_mismatch", got = got, expected = expected).to_string()
}

#[inline]
pub fn parse_float_token_failed(i: usize, tok: &str) -> String {
    rust_i18n::t!("parse_float_token_failed", i = i, tok = tok).to_string()
}

#[inline]
pub fn prop_preset_1() -> String {
    rust_i18n::t!("prop_preset_1").to_string()
}

#[inline]
pub fn prop_preset_2() -> String {
    rust_i18n::t!("prop_preset_2").to_string()
}

#[inline]
pub fn prop_preset_3() -> String {
    rust_i18n::t!("prop_preset_3").to_string()
}

#[inline]
pub fn default_project_name() -> String {
    rust_i18n::t!("default_project_name").to_string()
}

#[inline]
pub fn clear_cell() -> String {
    rust_i18n::t!("clear_cell").to_string()
}

#[inline]
pub fn remove_row() -> String {
    rust_i18n::t!("remove_row").to_string()
}

#[inline]
pub fn edit_dton_title() -> String {
    rust_i18n::t!("edit_dton_title").to_string()
}

#[inline]
pub fn dton_editor_heading() -> String {
    rust_i18n::t!("dton_editor_heading").to_string()
}

#[inline]
pub fn keep_original_length() -> String {
    rust_i18n::t!("keep_original_length").to_string()
}

#[inline]
pub fn enable_advanced_fields() -> String {
    rust_i18n::t!("enable_advanced_fields").to_string()
}

#[inline]
pub fn tones_heading() -> String {
    rust_i18n::t!("tones_heading").to_string()
}

#[inline]
pub fn details_heading() -> String {
    rust_i18n::t!("details_heading").to_string()
}

#[inline]
pub fn select_tone_left() -> String {
    rust_i18n::t!("select_tone_left").to_string()
}

#[inline]
pub fn index_out_of_range() -> String {
    rust_i18n::t!("index_out_of_range").to_string()
}

#[inline]
pub fn data_length_label() -> String {
    rust_i18n::t!("data_length_label").to_string()
}

#[inline]
pub fn data_floats_label() -> String {
    rust_i18n::t!("data_floats_label").to_string()
}

#[inline]
pub fn dton_field_hash() -> String {
    rust_i18n::t!("dton_field_hash").to_string()
}

#[inline]
pub fn dton_field_unk1() -> String {
    rust_i18n::t!("dton_field_unk1").to_string()
}

#[inline]
pub fn prop_field_unk1() -> String {
    rust_i18n::t!("prop_field_unk1").to_string()
}

#[inline]
pub fn prop_field_reserved_u16() -> String {
    rust_i18n::t!("prop_field_reserved_u16").to_string()
}

#[inline]
pub fn prop_field_unk2() -> String {
    rust_i18n::t!("prop_field_unk2").to_string()
}

#[inline]
pub fn prop_field_unk3() -> String {
    rust_i18n::t!("prop_field_unk3").to_string()
}

#[inline]
pub fn duplicate_row() -> String {
    rust_i18n::t!("duplicate_row").to_string()
}

#[inline]
pub fn delete_row() -> String {
    rust_i18n::t!("delete_row").to_string()
}

#[inline]
pub fn edit_prop_title() -> String {
    rust_i18n::t!("edit_prop_title").to_string()
}

#[inline]
pub fn prop_section_editor() -> String {
    rust_i18n::t!("prop_section_editor").to_string()
}

#[inline]
pub fn no_prop_section() -> String {
    rust_i18n::t!("no_prop_section").to_string()
}

#[inline]
pub fn create_new_prop() -> String {
    rust_i18n::t!("create_new_prop").to_string()
}

#[inline]
pub fn presets_heading() -> String {
    rust_i18n::t!("presets_heading").to_string()
}

#[inline]
pub fn apply_selected_preset() -> String {
    rust_i18n::t!("apply_selected_preset").to_string()
}

#[inline]
pub fn save_current_as_preset() -> String {
    rust_i18n::t!("save_current_as_preset").to_string()
}

#[inline]
pub fn basic_fields() -> String {
    rust_i18n::t!("basic_fields").to_string()
}

#[inline]
pub fn project_label() -> String {
    rust_i18n::t!("project_label").to_string()
}

#[inline]
pub fn timestamp_label() -> String {
    rust_i18n::t!("timestamp_label").to_string()
}

#[inline]
pub fn layout_heading() -> String {
    rust_i18n::t!("layout_heading").to_string()
}

#[inline]
pub fn layout_type_label() -> String {
    rust_i18n::t!("layout_type_label").to_string()
}

#[inline]
pub fn layout_minimal() -> String {
    rust_i18n::t!("layout_minimal").to_string()
}

#[inline]
pub fn layout_extended() -> String {
    rust_i18n::t!("layout_extended").to_string()
}

#[inline]
pub fn advanced_fields() -> String {
    rust_i18n::t!("advanced_fields").to_string()
}

#[inline]
pub fn unsaved_changes() -> String {
    rust_i18n::t!("unsaved_changes").to_string()
}

#[inline]
pub fn confirm_replace_empty_title() -> String {
    rust_i18n::t!("confirm_replace_empty_title").to_string()
}

#[inline]
pub fn confirm_replace_empty_body(n: usize) -> String {
    rust_i18n::t!("confirm_replace_empty_body", n = n).to_string()
}

#[inline]
pub fn confirm_remove_selected_title() -> String {
    rust_i18n::t!("confirm_remove_selected_title").to_string()
}

#[inline]
pub fn confirm_remove_selected_body(n: usize) -> String {
    rust_i18n::t!("confirm_remove_selected_body", n = n).to_string()
}

#[inline]
pub fn confirm_debug_convert_body() -> String {
    rust_i18n::t!("confirm_debug_convert_body").to_string()
}

#[inline]
pub fn debug_nus3bank_only() -> String {
    rust_i18n::t!("debug_nus3bank_only").to_string()
}

#[inline]
pub fn configure_new_audio_toast() -> String {
    rust_i18n::t!("configure_new_audio_toast").to_string()
}

#[inline]
pub fn grp_nus3bank_only() -> String {
    rust_i18n::t!("grp_nus3bank_only").to_string()
}

#[inline]
pub fn dton_nus3bank_only() -> String {
    rust_i18n::t!("dton_nus3bank_only").to_string()
}

#[inline]
pub fn prop_nus3bank_only() -> String {
    rust_i18n::t!("prop_nus3bank_only").to_string()
}

#[inline]
pub fn no_file_selected() -> String {
    rust_i18n::t!("no_file_selected").to_string()
}

#[inline]
pub fn confirm_export_all_title() -> String {
    rust_i18n::t!("confirm_export_all_title").to_string()
}

#[inline]
pub fn confirm_export_all_body(n: usize) -> String {
    rust_i18n::t!("confirm_export_all_body", n = n).to_string()
}

#[inline]
pub fn exported_to(path: impl Display) -> String {
    rust_i18n::t!("exported_to", path = path).to_string()
}

#[inline]
pub fn export_failed(err: impl Display) -> String {
    rust_i18n::t!("export_failed", err = err).to_string()
}

#[inline]
pub fn no_output_dir() -> String {
    rust_i18n::t!("no_output_dir").to_string()
}

#[inline]
pub fn now_playing(name: impl Display) -> String {
    rust_i18n::t!("now_playing", name = name).to_string()
}

#[inline]
pub fn failed_load_audio(name: impl Display, err: impl Display) -> String {
    rust_i18n::t!("failed_load_audio", name = name, err = err).to_string()
}

#[inline]
pub fn audio_player_not_initialized() -> String {
    rust_i18n::t!("audio_player_not_initialized").to_string()
}

#[inline]
pub fn no_file_for_playback() -> String {
    rust_i18n::t!("no_file_for_playback").to_string()
}

#[inline]
pub fn invalid_audio_index(idx: usize, max: usize) -> String {
    rust_i18n::t!("invalid_audio_index", idx = idx, max = max).to_string()
}

#[inline]
pub fn replace_failed(err: impl Display) -> String {
    rust_i18n::t!("replace_failed", err = err).to_string()
}

#[inline]
pub fn configure_loop_for(name: impl Display) -> String {
    rust_i18n::t!("configure_loop_for", name = name).to_string()
}

#[inline]
pub fn confirm_delete_audio_body(name: impl Display) -> String {
    rust_i18n::t!("confirm_delete_audio_body", name = name).to_string()
}

#[inline]
pub fn exported_count_to(count: usize, dir: impl Display) -> String {
    rust_i18n::t!("exported_count_to", count = count, dir = dir).to_string()
}

#[inline]
pub fn failed_replace_key(key: impl Display, err: impl Display) -> String {
    rust_i18n::t!("failed_replace_key", key = key, err = err).to_string()
}

#[inline]
pub fn replaced_empty_wav(n: usize) -> String {
    rust_i18n::t!("replaced_empty_wav", n = n).to_string()
}

#[inline]
pub fn no_matching_replace() -> String {
    rust_i18n::t!("no_matching_replace").to_string()
}

#[inline]
pub fn debug_convert_bank_only() -> String {
    rust_i18n::t!("debug_convert_bank_only").to_string()
}

#[inline]
pub fn failed_open_bank(err: impl Display) -> String {
    rust_i18n::t!("failed_open_bank", err = err).to_string()
}

#[inline]
pub fn convert_failed_for(name: impl Display, err: impl Display) -> String {
    rust_i18n::t!("convert_failed_for", name = name, err = err).to_string()
}

#[inline]
pub fn debug_convert_done(c: usize, s: usize, f: usize) -> String {
    rust_i18n::t!("debug_convert_done", c = c, s = s, f = f).to_string()
}

#[inline]
pub fn failed_mark_deletion(err: impl Display) -> String {
    rust_i18n::t!("failed_mark_deletion", err = err).to_string()
}

#[inline]
pub fn marked_for_deletion_count(n: usize) -> String {
    rust_i18n::t!("marked_for_deletion_count", n = n).to_string()
}

#[inline]
pub fn no_matching_in_list() -> String {
    rust_i18n::t!("no_matching_in_list").to_string()
}

#[inline]
pub fn no_audio_list() -> String {
    rust_i18n::t!("no_audio_list").to_string()
}

#[inline]
pub fn marked_deleted_one(name: impl Display) -> String {
    rust_i18n::t!("marked_deleted_one", name = name).to_string()
}

#[inline]
pub fn no_audio_path() -> String {
    rust_i18n::t!("no_audio_path").to_string()
}

#[inline]
pub fn added_wav(name: impl Display) -> String {
    rust_i18n::t!("added_wav", name = name).to_string()
}

#[inline]
pub fn register_wav_failed(err: impl Display) -> String {
    rust_i18n::t!("register_wav_failed", err = err).to_string()
}

#[inline]
pub fn added_original(name: impl Display) -> String {
    rust_i18n::t!("added_original", name = name).to_string()
}

#[inline]
pub fn failed_add_audio(err: impl Display) -> String {
    rust_i18n::t!("failed_add_audio", err = err).to_string()
}

#[inline]
pub fn no_audio_data() -> String {
    rust_i18n::t!("no_audio_data").to_string()
}

#[inline]
pub fn name_and_id_required() -> String {
    rust_i18n::t!("name_and_id_required").to_string()
}

#[inline]
pub fn id_must_be_valid_number() -> String {
    rust_i18n::t!("id_must_be_valid_number").to_string()
}

#[inline]
pub fn failed_process_new_audio(err: impl Display) -> String {
    rust_i18n::t!("failed_process_new_audio", err = err).to_string()
}

#[inline]
pub fn add_audio_failed(err: impl Display) -> String {
    rust_i18n::t!("add_audio_failed", err = err).to_string()
}

#[inline]
pub fn no_replacement_path() -> String {
    rust_i18n::t!("no_replacement_path").to_string()
}

#[inline]
pub fn failed_process_replacement_key(key: impl Display, err: impl Display) -> String {
    rust_i18n::t!("failed_process_replacement_key", key = key, err = err).to_string()
}

#[inline]
pub fn loop_word_start() -> String {
    rust_i18n::t!("loop_word_start").to_string()
}

#[inline]
pub fn loop_word_end() -> String {
    rust_i18n::t!("loop_word_end").to_string()
}

#[inline]
pub fn loop_parenthetical_range(start: &str, end: &str) -> String {
    rust_i18n::t!("loop_parenthetical_range", start = start, end = end).to_string()
}

#[inline]
pub fn loop_parenthetical_full() -> String {
    rust_i18n::t!("loop_parenthetical_full").to_string()
}

#[inline]
pub fn replaced_in_memory_count(n: usize, loop_msg: &str) -> String {
    rust_i18n::t!("replaced_in_memory_count", n = n, loop_msg = loop_msg).to_string()
}

#[inline]
pub fn replaced_in_memory_one(name: impl Display, loop_msg: &str) -> String {
    rust_i18n::t!("replaced_in_memory_one", name = name, loop_msg = loop_msg).to_string()
}

#[inline]
pub fn failed_process_replacement(err: impl Display) -> String {
    rust_i18n::t!("failed_process_replacement", err = err).to_string()
}

#[inline]
pub fn prepare_playback_audio_failed() -> String {
    rust_i18n::t!("prepare_playback_audio_failed").to_string()
}
