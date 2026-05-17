use egui::Context;
use std::fmt::Display;

/// Stored each frame in egui temp data (`TemplateApp::update`).
#[inline]
pub fn locale_ctx_id() -> egui::Id {
    egui::Id::new("exvs2_app_locale")
}

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Debug)]
pub enum Locale {
    #[serde(rename = "en")]
    En,
    #[serde(rename = "zh")]
    Zh,
}

impl Default for Locale {
    fn default() -> Self {
        Self::detect_system()
    }
}

impl Locale {
    pub fn detect_system() -> Self {
        if let Some(loc) = sys_locale::get_locale() {
            let l = loc.to_lowercase();
            if l.starts_with("zh") {
                return Self::Zh;
            }
        }
        Self::En
    }
}

#[derive(Clone, Copy)]
pub struct I18n {
    pub locale: Locale,
}

impl I18n {
    pub fn new(locale: Locale) -> Self {
        Self { locale }
    }

    pub fn from_ctx(ctx: &Context) -> Self {
        Self {
            locale: ctx
                .data(|d| d.get_temp::<Locale>(locale_ctx_id()))
                .unwrap_or_else(Locale::detect_system),
        }
    }

    // ——— General ———
    pub fn window_title(&self) -> &'static str {
        match self.locale {
            Locale::En => "EXVS2 Audio Editor",
            Locale::Zh => "EXVS2 音频编辑器",
        }
    }

    pub fn ok(&self) -> &'static str {
        match self.locale {
            Locale::En => "OK",
            Locale::Zh => "确定",
        }
    }

    pub fn cancel(&self) -> &'static str {
        match self.locale {
            Locale::En => "Cancel",
            Locale::Zh => "取消",
        }
    }

    pub fn confirm(&self) -> &'static str {
        match self.locale {
            Locale::En => "Confirm",
            Locale::Zh => "确认",
        }
    }

    pub fn confirm_title_default(&self) -> &'static str {
        match self.locale {
            Locale::En => "Confirm",
            Locale::Zh => "确认",
        }
    }

    // ——— Top menu ———
    pub fn file_menu(&self) -> &'static str {
        match self.locale {
            Locale::En => "File",
            Locale::Zh => "文件",
        }
    }

    pub fn save_changes(&self) -> &'static str {
        match self.locale {
            Locale::En => "Save Changes",
            Locale::Zh => "保存更改",
        }
    }

    pub fn save_nus3bank(&self) -> &'static str {
        match self.locale {
            Locale::En => "Save .nus3bank",
            Locale::Zh => "保存 .nus3bank",
        }
    }

    pub fn save_nus3audio(&self) -> &'static str {
        match self.locale {
            Locale::En => "Save .nus3audio",
            Locale::Zh => "保存 .nus3audio",
        }
    }

    pub fn save_file_generic(&self) -> &'static str {
        match self.locale {
            Locale::En => "Save file",
            Locale::Zh => "保存文件",
        }
    }

    pub fn settings_menu(&self) -> &'static str {
        match self.locale {
            Locale::En => "Settings",
            Locale::Zh => "设置",
        }
    }

    pub fn reset_layout(&self) -> &'static str {
        match self.locale {
            Locale::En => "Reset Layout",
            Locale::Zh => "重置布局",
        }
    }

    pub fn language(&self) -> &'static str {
        match self.locale {
            Locale::En => "Language",
            Locale::Zh => "语言",
        }
    }

    pub fn language_english(&self) -> &'static str {
        "English"
    }

    pub fn language_chinese(&self) -> &'static str {
        match self.locale {
            Locale::En => "简体中文",
            Locale::Zh => "简体中文",
        }
    }

    pub fn help_menu(&self) -> &'static str {
        match self.locale {
            Locale::En => "Help",
            Locale::Zh => "帮助",
        }
    }

    pub fn about(&self) -> &'static str {
        match self.locale {
            Locale::En => "About",
            Locale::Zh => "关于",
        }
    }

    // ——— Top panel dialogs ———
    pub fn save_failed_title(&self) -> &'static str {
        match self.locale {
            Locale::En => "Save Failed",
            Locale::Zh => "保存失败",
        }
    }

    pub fn no_file_selected_save(&self) -> &'static str {
        match self.locale {
            Locale::En => "No file selected to save changes to",
            Locale::Zh => "未选择要保存更改的文件",
        }
    }

    pub fn no_changes_title(&self) -> &'static str {
        match self.locale {
            Locale::En => "No Changes",
            Locale::Zh => "无更改",
        }
    }

    pub fn no_pending_changes(&self) -> &'static str {
        match self.locale {
            Locale::En => "There are no pending changes to save",
            Locale::Zh => "没有待保存的更改",
        }
    }

    pub fn save_successful_title(&self) -> &'static str {
        match self.locale {
            Locale::En => "Save Successful",
            Locale::Zh => "保存成功",
        }
    }

    pub fn save_success_body(&self, count: usize, path: &str) -> String {
        match self.locale {
            Locale::En => format!(
                "Successfully saved {} changes to:\n{}",
                count, path
            ),
            Locale::Zh => format!("已成功保存 {} 项更改到：\n{}", count, path),
        }
    }

    pub fn save_failed_msg(&self, err: &str) -> String {
        match self.locale {
            Locale::En => format!("Failed to save changes: {}", err),
            Locale::Zh => format!("保存更改失败：{}", err),
        }
    }

    pub fn no_file_selected_save_as(&self) -> &'static str {
        match self.locale {
            Locale::En => "No file selected to save",
            Locale::Zh => "未选择要保存的文件",
        }
    }

    pub fn layout_reset_title(&self) -> &'static str {
        match self.locale {
            Locale::En => "Layout Reset",
            Locale::Zh => "布局已重置",
        }
    }

    pub fn layout_reset_msg(&self) -> &'static str {
        match self.locale {
            Locale::En => "The layout has been reset to defaults.",
            Locale::Zh => "布局已恢复为默认。",
        }
    }

    pub fn about_title(&self) -> &'static str {
        match self.locale {
            Locale::En => "About EXVS2 Audio Editor",
            Locale::Zh => "关于 EXVS2 音频编辑器",
        }
    }

    pub fn about_body(&self, version: &str) -> String {
        match self.locale {
            Locale::En => format!(
                "EXVS2 Audio Editor\n\nVersion: {}\n\nA tool for editing audio files in EXVS2 game.",
                version
            ),
            Locale::Zh => format!(
                "EXVS2 音频编辑器\n\n版本：{}\n\n用于编辑 EXVS2 游戏中的音频文件。",
                version
            ),
        }
    }

    pub fn source_link_label(&self) -> &'static str {
        match self.locale {
            Locale::En => "Source: https://github.com/kjjkjjzyayufqza/EXVS2-Audio-Editor",
            Locale::Zh => "源码：https://github.com/kjjkjjzyayufqza/EXVS2-Audio-Editor",
        }
    }

    pub fn update_available_title(&self) -> &'static str {
        match self.locale {
            Locale::En => "Update Available",
            Locale::Zh => "有可用更新",
        }
    }

    pub fn update_available_body(&self, current: &str, latest: &str) -> String {
        match self.locale {
            Locale::En => format!(
                "A new version of EXVS2 Audio Editor is available!\n\nCurrent version: {}\nLatest version: {}\n\nClick the link below to download:",
                current, latest
            ),
            Locale::Zh => format!(
                "EXVS2 音频编辑器有新版本！\n\n当前版本：{}\n最新版本：{}\n\n点击下方链接下载：",
                current, latest
            ),
        }
    }

    pub fn download_latest(&self) -> &'static str {
        match self.locale {
            Locale::En => "Download latest version",
            Locale::Zh => "下载最新版本",
        }
    }

    pub fn save_success_export_title(&self) -> &'static str {
        match self.locale {
            Locale::En => "Save success",
            Locale::Zh => "保存成功",
        }
    }

    pub fn save_success_export_body(&self, path: &str) -> String {
        match self.locale {
            Locale::En => format!("Audio file has been successfully saved to:\n{}", path),
            Locale::Zh => format!("音频文件已成功保存到：\n{}", path),
        }
    }

    pub fn save_failed_export(&self, err: &str) -> String {
        match self.locale {
            Locale::En => format!("Failed to save file: {}", err),
            Locale::Zh => format!("保存文件失败：{}", err),
        }
    }

    // ——— File list ———
    pub fn files_heading(&self) -> &'static str {
        match self.locale {
            Locale::En => "Files",
            Locale::Zh => "文件",
        }
    }

    pub fn add_files_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Add Files",
            Locale::Zh => "添加文件",
        }
    }

    pub fn select_audio_files_title(&self) -> &'static str {
        match self.locale {
            Locale::En => "Select Audio Files",
            Locale::Zh => "选择音频文件",
        }
    }

    pub fn select_audio_file_to_add(&self) -> &'static str {
        match self.locale {
            Locale::En => "Select Audio File to Add",
            Locale::Zh => "选择要添加的音频文件",
        }
    }

    pub fn all_files_filter(&self) -> &'static str {
        match self.locale {
            Locale::En => "All Files",
            Locale::Zh => "所有文件",
        }
    }

    pub fn audio_files_filter(&self) -> &'static str {
        match self.locale {
            Locale::En => "Audio Files",
            Locale::Zh => "音频文件",
        }
    }

    pub fn clear_all_files_title(&self) -> &'static str {
        match self.locale {
            Locale::En => "Clear All Files",
            Locale::Zh => "清空所有文件",
        }
    }

    pub fn clear_all_files_confirm(&self, n: usize) -> String {
        match self.locale {
            Locale::En => format!(
                "Are you sure you want to remove all {} file(s) from the list?",
                n
            ),
            Locale::Zh => format!("确定要从列表中移除全部 {} 个文件吗？", n),
        }
    }

    pub fn search_files_hint(&self) -> &'static str {
        match self.locale {
            Locale::En => "Search files...",
            Locale::Zh => "搜索文件…",
        }
    }

    pub fn clear_search_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Clear Search",
            Locale::Zh => "清除搜索",
        }
    }

    pub fn no_files_added(&self) -> &'static str {
        match self.locale {
            Locale::En => "No files added",
            Locale::Zh => "未添加文件",
        }
    }

    pub fn no_matching_files(&self) -> &'static str {
        match self.locale {
            Locale::En => "No matching files",
            Locale::Zh => "没有匹配的文件",
        }
    }

    pub fn remove_from_list_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Remove from list",
            Locale::Zh => "从列表移除",
        }
    }

    // ——— Main area / header / toolbar ———
    pub fn loading_audio(&self) -> &'static str {
        match self.locale {
            Locale::En => "Loading audio file info...",
            Locale::Zh => "正在加载音频信息…",
        }
    }

    pub fn no_file_selected_heading(&self) -> &'static str {
        match self.locale {
            Locale::En => "No file selected",
            Locale::Zh => "未选择文件",
        }
    }

    pub fn no_file_selected_hint(&self) -> &'static str {
        match self.locale {
            Locale::En => "Please select a file from the list on the left to start editing",
            Locale::Zh => "请从左侧列表选择文件以开始编辑",
        }
    }

    pub fn audio_editor_heading(&self) -> &'static str {
        match self.locale {
            Locale::En => "Audio Editor",
            Locale::Zh => "音频编辑",
        }
    }

    pub fn currently_editing(&self) -> &'static str {
        match self.locale {
            Locale::En => "Currently editing:",
            Locale::Zh => "当前编辑：",
        }
    }

    pub fn audio_files_found(&self, n: usize) -> String {
        match self.locale {
            Locale::En => format!("{} audio files found", n),
            Locale::Zh => format!("共 {} 条音频", n),
        }
    }

    pub fn refresh(&self) -> &'static str {
        match self.locale {
            Locale::En => "Refresh",
            Locale::Zh => "刷新",
        }
    }

    pub fn export_to(&self) -> &'static str {
        match self.locale {
            Locale::En => "Export to:",
            Locale::Zh => "导出到：",
        }
    }

    pub fn output_folder_not_set(&self) -> &'static str {
        match self.locale {
            Locale::En => "Output folder not set",
            Locale::Zh => "未设置输出文件夹",
        }
    }

    pub fn output_folder_hover(&self) -> &'static str {
        match self.locale {
            Locale::En => "Please select a folder where exported files will be saved",
            Locale::Zh => "请选择用于保存导出文件的文件夹",
        }
    }

    pub fn browse(&self) -> &'static str {
        match self.locale {
            Locale::En => "Browse",
            Locale::Zh => "浏览",
        }
    }

    pub fn select_output_directory(&self) -> &'static str {
        match self.locale {
            Locale::En => "Select Output Directory",
            Locale::Zh => "选择输出目录",
        }
    }

    pub fn clear_output_path_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Clear output path",
            Locale::Zh => "清除输出路径",
        }
    }

    pub fn simple_view(&self) -> &'static str {
        match self.locale {
            Locale::En => "Simple View",
            Locale::Zh => "简单视图",
        }
    }

    pub fn advanced_search(&self) -> &'static str {
        match self.locale {
            Locale::En => "Advanced Search",
            Locale::Zh => "高级搜索",
        }
    }

    pub fn search_in(&self) -> &'static str {
        match self.locale {
            Locale::En => "Search in:",
            Locale::Zh => "搜索范围：",
        }
    }

    pub fn tip_size_search(&self) -> &'static str {
        match self.locale {
            Locale::En => "Tip: Use 'KB' or 'MB' for size search",
            Locale::Zh => "提示：大小搜索可使用 KB 或 MB",
        }
    }

    pub fn search_audio_hint(&self) -> &'static str {
        match self.locale {
            Locale::En => "Search audio files...",
            Locale::Zh => "搜索音频…",
        }
    }

    pub fn clear_search_main_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Clear search",
            Locale::Zh => "清除搜索",
        }
    }

    // ——— Table toolbar ———
    pub fn actions_colon(&self) -> &'static str {
        match self.locale {
            Locale::En => "Actions:",
            Locale::Zh => "操作：",
        }
    }

    pub fn add_audio_btn(&self) -> &'static str {
        match self.locale {
            Locale::En => "Add",
            Locale::Zh => "添加",
        }
    }

    pub fn add_audio_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Add new audio file",
            Locale::Zh => "添加新音频",
        }
    }

    pub fn export_all_btn(&self) -> &'static str {
        match self.locale {
            Locale::En => "Export All",
            Locale::Zh => "全部导出",
        }
    }

    pub fn export_all_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Export all files to WAV",
            Locale::Zh => "将所有条目导出为 WAV",
        }
    }

    pub fn edit_colon(&self) -> &'static str {
        match self.locale {
            Locale::En => "Edit:",
            Locale::Zh => "编辑：",
        }
    }

    pub fn edit_grp_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Edit GRP List",
            Locale::Zh => "编辑 GRP 列表",
        }
    }

    pub fn edit_dton_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Edit DTON Tones",
            Locale::Zh => "编辑 DTON Tones",
        }
    }

    pub fn edit_prop_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Edit PROP",
            Locale::Zh => "编辑 PROP",
        }
    }

    pub fn batch_colon(&self) -> &'static str {
        match self.locale {
            Locale::En => "Batch:",
            Locale::Zh => "批量：",
        }
    }

    pub fn replace_btn(&self) -> &'static str {
        match self.locale {
            Locale::En => "Replace",
            Locale::Zh => "替换",
        }
    }

    pub fn replace_selected_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Replace selected with new audio",
            Locale::Zh => "用新音频替换所选项",
        }
    }

    pub fn clear_btn(&self) -> &'static str {
        match self.locale {
            Locale::En => "Clear",
            Locale::Zh => "清空",
        }
    }

    pub fn clear_wav_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Replace selected with empty WAV",
            Locale::Zh => "将所选替换为空 WAV",
        }
    }

    pub fn remove_btn(&self) -> &'static str {
        match self.locale {
            Locale::En => "Remove",
            Locale::Zh => "移除",
        }
    }

    pub fn remove_selected_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Remove selected items",
            Locale::Zh => "移除所选条目",
        }
    }

    pub fn more_menu(&self) -> &'static str {
        match self.locale {
            Locale::En => "More",
            Locale::Zh => "更多",
        }
    }

    pub fn debug_convert_all(&self) -> &'static str {
        match self.locale {
            Locale::En => "Debug: Convert All to WAV",
            Locale::Zh => "调试：全部转为 WAV",
        }
    }

    pub fn debug_convert_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Convert all tracks to PCM16 WAV in memory (NUS3BANK only)",
            Locale::Zh => "在内存中将全部轨道转为 PCM16 WAV（仅 .nus3bank）",
        }
    }

    pub fn selected_count(&self, n: usize) -> String {
        match self.locale {
            Locale::En => format!("{} selected", n),
            Locale::Zh => format!("已选 {} 项", n),
        }
    }

    pub fn found_count(&self, found: usize, total: usize) -> String {
        match self.locale {
            Locale::En => format!("Found {} / {}", found, total),
            Locale::Zh => format!("找到 {} / {}", found, total),
        }
    }

    // ——— Table headers & actions ———
    pub fn select_all_filtered_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Select/Deselect all (filtered)",
            Locale::Zh => "全选/取消全选（当前筛选）",
        }
    }

    pub fn col_name(&self) -> &'static str {
        match self.locale {
            Locale::En => "Name",
            Locale::Zh => "名称",
        }
    }

    pub fn col_id(&self) -> &'static str {
        "ID"
    }

    pub fn col_size(&self) -> &'static str {
        match self.locale {
            Locale::En => "Size",
            Locale::Zh => "大小",
        }
    }

    pub fn col_filename(&self) -> &'static str {
        match self.locale {
            Locale::En => "Filename",
            Locale::Zh => "文件名",
        }
    }

    pub fn col_type(&self) -> &'static str {
        match self.locale {
            Locale::En => "Type",
            Locale::Zh => "类型",
        }
    }

    pub fn col_action(&self) -> &'static str {
        match self.locale {
            Locale::En => "Action",
            Locale::Zh => "操作",
        }
    }

    pub fn play_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Play",
            Locale::Zh => "播放",
        }
    }

    pub fn export_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Export",
            Locale::Zh => "导出",
        }
    }

    pub fn replace_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Replace",
            Locale::Zh => "替换",
        }
    }

    pub fn remove_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Remove",
            Locale::Zh => "移除",
        }
    }

    pub fn more_label(&self) -> &'static str {
        match self.locale {
            Locale::En => "More",
            Locale::Zh => "更多",
        }
    }

    // ——— Audio player ———
    pub fn no_track_selected(&self) -> &'static str {
        match self.locale {
            Locale::En => "No track selected",
            Locale::Zh => "未选择曲目",
        }
    }

    pub fn shuffle_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Shuffle",
            Locale::Zh => "随机",
        }
    }

    pub fn previous_track_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Previous Track",
            Locale::Zh => "上一曲",
        }
    }

    pub fn pause_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Pause",
            Locale::Zh => "暂停",
        }
    }

    pub fn play_tooltip_player(&self) -> &'static str {
        match self.locale {
            Locale::En => "Play",
            Locale::Zh => "播放",
        }
    }

    pub fn next_track_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Next Track",
            Locale::Zh => "下一曲",
        }
    }

    pub fn loop_off_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Loop: Off",
            Locale::Zh => "循环：关",
        }
    }

    pub fn loop_all_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Loop: All",
            Locale::Zh => "循环：全部",
        }
    }

    pub fn loop_one_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Loop: One",
            Locale::Zh => "循环：单曲",
        }
    }

    pub fn stop_playback_tooltip(&self) -> &'static str {
        match self.locale {
            Locale::En => "Stop Playback",
            Locale::Zh => "停止播放",
        }
    }

    // ——— Loop settings modal ———
    pub fn loop_settings_title(&self, name: &str) -> String {
        match self.locale {
            Locale::En => format!("Loop Settings - {}", name),
            Locale::Zh => format!("循环设置 - {}", name),
        }
    }

    pub fn audio_information(&self) -> &'static str {
        match self.locale {
            Locale::En => "Audio Information",
            Locale::Zh => "音频信息",
        }
    }

    pub fn name_label(&self) -> &'static str {
        match self.locale {
            Locale::En => "Name:",
            Locale::Zh => "名称：",
        }
    }

    pub fn loop_settings_heading(&self) -> &'static str {
        match self.locale {
            Locale::En => "Loop Settings",
            Locale::Zh => "循环设置",
        }
    }

    pub fn enable_loop(&self) -> &'static str {
        match self.locale {
            Locale::En => "Enable loop functionality",
            Locale::Zh => "启用循环",
        }
    }

    pub fn use_custom_loop(&self) -> &'static str {
        match self.locale {
            Locale::En => "Use custom loop points",
            Locale::Zh => "使用自定义循环点",
        }
    }

    pub fn loop_start_sec(&self) -> &'static str {
        match self.locale {
            Locale::En => "Loop Start (seconds):",
            Locale::Zh => "循环起点（秒）：",
        }
    }

    pub fn loop_end_sec(&self) -> &'static str {
        match self.locale {
            Locale::En => "Loop End (seconds):",
            Locale::Zh => "循环终点（秒）：",
        }
    }

    pub fn loop_duration_sec(&self, d: f32) -> String {
        match self.locale {
            Locale::En => format!("Loop Duration: {:.2} seconds", d),
            Locale::Zh => format!("循环长度：{:.2} 秒", d),
        }
    }

    pub fn loop_full_track(&self) -> &'static str {
        match self.locale {
            Locale::En => "Audio will loop from beginning to end",
            Locale::Zh => "音频将从头到尾循环播放",
        }
    }

    pub fn loop_disabled(&self) -> &'static str {
        match self.locale {
            Locale::En => "Loop functionality is disabled",
            Locale::Zh => "循环已关闭",
        }
    }

    pub fn gain_heading(&self) -> &'static str {
        match self.locale {
            Locale::En => "Gain",
            Locale::Zh => "增益",
        }
    }

    pub fn gain_db_label(&self) -> &'static str {
        match self.locale {
            Locale::En => "Gain (dB):",
            Locale::Zh => "增益（dB）：",
        }
    }

    pub fn reset_gain(&self) -> &'static str {
        match self.locale {
            Locale::En => "Reset",
            Locale::Zh => "重置",
        }
    }

    pub fn linear_factor(&self, f: f32) -> String {
        match self.locale {
            Locale::En => format!("Linear factor: {:.3}", f),
            Locale::Zh => format!("线性系数：{:.3}", f),
        }
    }

    // ——— Add audio modal ———
    pub fn add_new_audio_title(&self) -> &'static str {
        match self.locale {
            Locale::En => "Add New Audio File",
            Locale::Zh => "添加新音频",
        }
    }

    pub fn file_information(&self) -> &'static str {
        match self.locale {
            Locale::En => "File Information",
            Locale::Zh => "文件信息",
        }
    }

    pub fn selected_file_label(&self) -> &'static str {
        match self.locale {
            Locale::En => "Selected File:",
            Locale::Zh => "所选文件：",
        }
    }

    pub fn duration_label(&self) -> &'static str {
        match self.locale {
            Locale::En => "Duration:",
            Locale::Zh => "时长：",
        }
    }

    pub fn seconds_fmt(&self, s: f32) -> String {
        match self.locale {
            Locale::En => format!("{:.2} seconds", s),
            Locale::Zh => format!("{:.2} 秒", s),
        }
    }

    pub fn audio_metadata(&self) -> &'static str {
        match self.locale {
            Locale::En => "Audio Metadata",
            Locale::Zh => "音频元数据",
        }
    }

    pub fn id_label(&self) -> &'static str {
        match self.locale {
            Locale::En => "ID:",
            Locale::Zh => "ID：",
        }
    }

    pub fn error_label(&self) -> &'static str {
        match self.locale {
            Locale::En => "Error:",
            Locale::Zh => "错误：",
        }
    }

    pub fn name_exists_error(&self) -> &'static str {
        match self.locale {
            Locale::En => "Error: Name already exists!",
            Locale::Zh => "错误：名称已存在！",
        }
    }

    pub fn id_exists_error(&self) -> &'static str {
        match self.locale {
            Locale::En => "Error: ID already exists!",
            Locale::Zh => "错误：ID 已存在！",
        }
    }

    pub fn no_audio_loaded(&self) -> &'static str {
        match self.locale {
            Locale::En => "No audio file loaded. Please select a valid audio file.",
            Locale::Zh => "未加载音频文件，请选择有效的音频文件。",
        }
    }

    pub fn failed_read_audio(&self, err: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Failed to read audio file: {}", err),
            Locale::Zh => format!("读取音频文件失败：{}", err),
        }
    }

    // ——— GRP modal ———
    pub fn edit_grp_list_title(&self) -> &'static str {
        match self.locale {
            Locale::En => "Edit GRP List",
            Locale::Zh => "编辑 GRP 列表",
        }
    }

    pub fn grp_names_editor(&self) -> &'static str {
        match self.locale {
            Locale::En => "GRP Names Editor",
            Locale::Zh => "GRP 名称编辑",
        }
    }

    pub fn file_label_fmt(&self, path: &str) -> String {
        match self.locale {
            Locale::En => format!("File: {}", path),
            Locale::Zh => format!("文件：{}", path),
        }
    }

    pub fn no_file_selected_short(&self) -> &'static str {
        match self.locale {
            Locale::En => "No file selected.",
            Locale::Zh => "未选择文件。",
        }
    }

    pub fn search_label(&self) -> &'static str {
        match self.locale {
            Locale::En => "Search:",
            Locale::Zh => "搜索：",
        }
    }

    pub fn total_label(&self, n: usize) -> String {
        match self.locale {
            Locale::En => format!("Total: {}", n),
            Locale::Zh => format!("总计：{}", n),
        }
    }

    pub fn visible_label(&self, n: usize) -> String {
        match self.locale {
            Locale::En => format!("Visible: {}", n),
            Locale::Zh => format!("可见：{}", n),
        }
    }

    pub fn find_label(&self) -> &'static str {
        match self.locale {
            Locale::En => "Find:",
            Locale::Zh => "查找：",
        }
    }

    pub fn replace_label(&self) -> &'static str {
        match self.locale {
            Locale::En => "Replace:",
            Locale::Zh => "替换：",
        }
    }

    pub fn replace_in_visible(&self) -> &'static str {
        match self.locale {
            Locale::En => "Replace in Visible",
            Locale::Zh => "替换可见项",
        }
    }

    pub fn add_row(&self) -> &'static str {
        match self.locale {
            Locale::En => "Add Row",
            Locale::Zh => "添加行",
        }
    }

    pub fn replace_with_template(&self) -> &'static str {
        match self.locale {
            Locale::En => "Replace with Template",
            Locale::Zh => "用模板替换",
        }
    }

    pub fn reload_from_file(&self) -> &'static str {
        match self.locale {
            Locale::En => "Reload from File",
            Locale::Zh => "从文件重新加载",
        }
    }

    pub fn nus3bank_open_failed(&self, err: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Failed to open NUS3BANK file: {}", err),
            Locale::Zh => format!("打开 NUS3BANK 文件失败：{}", err),
        }
    }

    pub fn grp_find_text_empty(&self) -> &'static str {
        match self.locale {
            Locale::En => "Find text is empty",
            Locale::Zh => "查找内容为空",
        }
    }

    pub fn grp_no_file_for_edit(&self) -> &'static str {
        match self.locale {
            Locale::En => "No file selected for GRP edit",
            Locale::Zh => "未选择用于 GRP 编辑的文件",
        }
    }

    pub fn grp_template_empty(&self) -> &'static str {
        match self.locale {
            Locale::En => {
                "Template is empty. Paste the full list into grp_template.rs"
            }
            Locale::Zh => {
                "模板为空。请将完整列表粘贴到 grp_template.rs"
            }
        }
    }

    pub fn dton_no_file_for_edit(&self) -> &'static str {
        match self.locale {
            Locale::En => "No file selected for DTON edit",
            Locale::Zh => "未选择用于 DTON 编辑的文件",
        }
    }

    pub fn prop_no_file_for_edit(&self) -> &'static str {
        match self.locale {
            Locale::En => "No file selected for PROP edit",
            Locale::Zh => "未选择用于 PROP 编辑的文件",
        }
    }

    pub fn dton_len_label(&self, len: usize) -> String {
        match self.locale {
            Locale::En => format!("len={}", len),
            Locale::Zh => format!("长度={}", len),
        }
    }

    pub fn dton_original_len(&self, n: usize) -> String {
        match self.locale {
            Locale::En => format!("(original: {})", n),
            Locale::Zh => format!("（原始：{}）", n),
        }
    }

    pub fn data_length_mismatch(&self, got: usize, expected: usize) -> String {
        match self.locale {
            Locale::En => format!(
                "Data length mismatch: got {}, expected {}",
                got, expected
            ),
            Locale::Zh => format!("数据长度不匹配：得到 {}，期望 {}", got, expected),
        }
    }

    pub fn parse_float_token_failed(&self, i: usize, tok: &str) -> String {
        match self.locale {
            Locale::En => format!("Failed to parse float at token {}: '{}'", i, tok),
            Locale::Zh => format!("无法解析第 {} 个数值标记「{}」为浮点数", i, tok),
        }
    }

    pub fn prop_preset_1(&self) -> &'static str {
        match self.locale {
            Locale::En => "Preset 1: Test (Minimal)",
            Locale::Zh => "预设 1：测试（最小）",
        }
    }

    pub fn prop_preset_2(&self) -> &'static str {
        match self.locale {
            Locale::En => "Preset 2: DefaultProject (Extended)",
            Locale::Zh => "预设 2：DefaultProject（扩展）",
        }
    }

    pub fn prop_preset_3(&self) -> &'static str {
        match self.locale {
            Locale::En => "Preset 3: DefaultProject (Extended v2)",
            Locale::Zh => "预设 3：DefaultProject（扩展 v2）",
        }
    }

    pub fn prop_custom_preset_name(&self, project: &str, minimal: bool) -> String {
        let layout = if minimal {
            self.layout_minimal()
        } else {
            self.layout_extended()
        };
        match self.locale {
            Locale::En => format!("Custom: {} ({})", project, layout),
            Locale::Zh => format!("自定义：{}（{}）", project, layout),
        }
    }

    pub fn default_project_name(&self) -> &'static str {
        "DefaultProject"
    }

    pub fn clear_cell(&self) -> &'static str {
        match self.locale {
            Locale::En => "Clear",
            Locale::Zh => "清除",
        }
    }

    pub fn remove_row(&self) -> &'static str {
        match self.locale {
            Locale::En => "Remove",
            Locale::Zh => "删除",
        }
    }

    // ——— DTON modal ———
    pub fn edit_dton_title(&self) -> &'static str {
        match self.locale {
            Locale::En => "Edit DTON Tones",
            Locale::Zh => "编辑 DTON Tones",
        }
    }

    pub fn dton_editor_heading(&self) -> &'static str {
        match self.locale {
            Locale::En => "DTON Tones Editor",
            Locale::Zh => "DTON Tones 编辑",
        }
    }

    pub fn keep_original_length(&self) -> &'static str {
        match self.locale {
            Locale::En => "Keep original data length",
            Locale::Zh => "保持原始数据长度",
        }
    }

    pub fn enable_advanced_fields(&self) -> &'static str {
        match self.locale {
            Locale::En => "Enable advanced fields",
            Locale::Zh => "启用高级字段",
        }
    }

    pub fn tones_heading(&self) -> &'static str {
        match self.locale {
            Locale::En => "Tones",
            Locale::Zh => "Tones",
        }
    }

    pub fn details_heading(&self) -> &'static str {
        match self.locale {
            Locale::En => "Details",
            Locale::Zh => "详情",
        }
    }

    pub fn select_tone_left(&self) -> &'static str {
        match self.locale {
            Locale::En => "Select a tone on the left.",
            Locale::Zh => "请在左侧选择一项。",
        }
    }

    pub fn index_out_of_range(&self) -> &'static str {
        match self.locale {
            Locale::En => "Selected index out of range.",
            Locale::Zh => "所选索引越界。",
        }
    }

    pub fn data_length_label(&self) -> &'static str {
        match self.locale {
            Locale::En => "Data length:",
            Locale::Zh => "数据长度：",
        }
    }

    pub fn data_floats_label(&self) -> &'static str {
        match self.locale {
            Locale::En => "Data (floats, separated by spaces/commas/newlines):",
            Locale::Zh => "数据（浮点数，空格/逗号/换行分隔）：",
        }
    }

    pub fn dton_field_hash(&self) -> &'static str {
        match self.locale {
            Locale::En => "hash (i32):",
            Locale::Zh => "hash（i32）：",
        }
    }

    pub fn dton_field_unk1(&self) -> &'static str {
        match self.locale {
            Locale::En => "unk1 (i32):",
            Locale::Zh => "unk1（i32）：",
        }
    }

    pub fn prop_field_unk1(&self) -> &'static str {
        match self.locale {
            Locale::En => "unk1 (i32):",
            Locale::Zh => "unk1（i32）：",
        }
    }

    pub fn prop_field_reserved_u16(&self) -> &'static str {
        match self.locale {
            Locale::En => "reserved_u16 (u16):",
            Locale::Zh => "reserved_u16（u16）：",
        }
    }

    pub fn prop_field_unk2(&self) -> &'static str {
        match self.locale {
            Locale::En => "unk2 (u16):",
            Locale::Zh => "unk2（u16）：",
        }
    }

    pub fn prop_field_unk3(&self) -> &'static str {
        match self.locale {
            Locale::En => "unk3 (u16):",
            Locale::Zh => "unk3（u16）：",
        }
    }

    pub fn duplicate_row(&self) -> &'static str {
        match self.locale {
            Locale::En => "Duplicate",
            Locale::Zh => "复制",
        }
    }

    pub fn delete_row(&self) -> &'static str {
        match self.locale {
            Locale::En => "Delete",
            Locale::Zh => "删除",
        }
    }

    // ——— PROP modal ———
    pub fn edit_prop_title(&self) -> &'static str {
        match self.locale {
            Locale::En => "Edit PROP Section",
            Locale::Zh => "编辑 PROP 区段",
        }
    }

    pub fn prop_section_editor(&self) -> &'static str {
        match self.locale {
            Locale::En => "PROP Section Editor",
            Locale::Zh => "PROP 区段编辑",
        }
    }

    pub fn no_prop_section(&self) -> &'static str {
        match self.locale {
            Locale::En => "No PROP section in this file.",
            Locale::Zh => "此文件中无 PROP 区段。",
        }
    }

    pub fn create_new_prop(&self) -> &'static str {
        match self.locale {
            Locale::En => "Create New PROP Section",
            Locale::Zh => "创建新 PROP 区段",
        }
    }

    pub fn presets_heading(&self) -> &'static str {
        match self.locale {
            Locale::En => "Presets",
            Locale::Zh => "预设",
        }
    }

    pub fn apply_selected_preset(&self) -> &'static str {
        match self.locale {
            Locale::En => "Apply Selected Preset",
            Locale::Zh => "应用所选预设",
        }
    }

    pub fn save_current_as_preset(&self) -> &'static str {
        match self.locale {
            Locale::En => "Save Current as Preset",
            Locale::Zh => "将当前保存为预设",
        }
    }

    pub fn basic_fields(&self) -> &'static str {
        match self.locale {
            Locale::En => "Basic Fields",
            Locale::Zh => "基本字段",
        }
    }

    pub fn project_label(&self) -> &'static str {
        match self.locale {
            Locale::En => "Project:",
            Locale::Zh => "工程：",
        }
    }

    pub fn timestamp_label(&self) -> &'static str {
        match self.locale {
            Locale::En => "Timestamp:",
            Locale::Zh => "时间戳：",
        }
    }

    pub fn layout_heading(&self) -> &'static str {
        match self.locale {
            Locale::En => "Layout",
            Locale::Zh => "布局",
        }
    }

    pub fn layout_type_label(&self) -> &'static str {
        match self.locale {
            Locale::En => "Layout Type:",
            Locale::Zh => "布局类型：",
        }
    }

    pub fn layout_minimal(&self) -> &'static str {
        match self.locale {
            Locale::En => "Minimal",
            Locale::Zh => "最小",
        }
    }

    pub fn layout_extended(&self) -> &'static str {
        match self.locale {
            Locale::En => "Extended",
            Locale::Zh => "扩展",
        }
    }

    pub fn advanced_fields(&self) -> &'static str {
        match self.locale {
            Locale::En => "Advanced Fields",
            Locale::Zh => "高级字段",
        }
    }

    pub fn unsaved_changes(&self) -> &'static str {
        match self.locale {
            Locale::En => "Unsaved changes",
            Locale::Zh => "未保存的更改",
        }
    }

    // ——— Confirm / table messages ———
    pub fn confirm_replace_empty_title(&self) -> &'static str {
        match self.locale {
            Locale::En => "Confirm Replace with Empty WAV",
            Locale::Zh => "确认替换为空 WAV",
        }
    }

    pub fn confirm_replace_empty_body(&self, n: usize) -> String {
        match self.locale {
            Locale::En => format!(
                "This will replace the audio data of {} selected file(s) with empty WAV. Names and IDs will be preserved. Continue?",
                n
            ),
            Locale::Zh => format!(
                "将把 {} 个所选文件的音频数据替换为空 WAV，名称与 ID 将保留。是否继续？",
                n
            ),
        }
    }

    pub fn confirm_remove_selected_title(&self) -> &'static str {
        match self.locale {
            Locale::En => "Confirm Remove Selected",
            Locale::Zh => "确认移除所选",
        }
    }

    pub fn confirm_remove_selected_body(&self, n: usize) -> String {
        match self.locale {
            Locale::En => format!(
                "This will mark {} selected item(s) for deletion (in memory only). Continue?",
                n
            ),
            Locale::Zh => format!(
                "将把 {} 个所选条目标记为删除（仅内存）。是否继续？",
                n
            ),
        }
    }

    pub fn confirm_debug_convert_body(&self) -> &'static str {
        match self.locale {
            Locale::En => "This will normalize all tracks in the currently opened .nus3bank to standard PCM16 WAV in memory (skips tracks that are already PCM16 WAV). This may take some time. Continue?",
            Locale::Zh => "将把当前打开的 .nus3bank 中全部轨道在内存中规范为标准 PCM16 WAV（已是 PCM16 的会跳过）。可能较耗时。是否继续？",
        }
    }

    pub fn debug_nus3bank_only(&self) -> &'static str {
        match self.locale {
            Locale::En => "This debug action is only available for .nus3bank files",
            Locale::Zh => "此调试操作仅适用于 .nus3bank 文件",
        }
    }

    pub fn configure_new_audio_toast(&self) -> &'static str {
        match self.locale {
            Locale::En => "Please configure settings for the new audio file",
            Locale::Zh => "请为新音频完成设置",
        }
    }

    pub fn grp_nus3bank_only(&self) -> &'static str {
        match self.locale {
            Locale::En => "GRP editing is only available for .nus3bank files",
            Locale::Zh => "GRP 编辑仅支持 .nus3bank",
        }
    }

    pub fn dton_nus3bank_only(&self) -> &'static str {
        match self.locale {
            Locale::En => "DTON editing is only available for .nus3bank files",
            Locale::Zh => "DTON 编辑仅支持 .nus3bank",
        }
    }

    pub fn prop_nus3bank_only(&self) -> &'static str {
        match self.locale {
            Locale::En => "PROP editing is only available for .nus3bank files",
            Locale::Zh => "PROP 编辑仅支持 .nus3bank",
        }
    }

    pub fn no_file_selected(&self) -> &'static str {
        match self.locale {
            Locale::En => "No file selected",
            Locale::Zh => "未选择文件",
        }
    }

    pub fn confirm_export_all_title(&self) -> &'static str {
        match self.locale {
            Locale::En => "Confirm Export All",
            Locale::Zh => "确认全部导出",
        }
    }

    pub fn confirm_export_all_body(&self, n: usize) -> String {
        match self.locale {
            Locale::En => format!(
                "Are you sure you want to export all {} audio files? This may take some time.",
                n
            ),
            Locale::Zh => format!(
                "确定要导出全部 {} 条音频吗？可能需要一些时间。",
                n
            ),
        }
    }

    pub fn exported_to(&self, path: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Successfully exported to: {}", path),
            Locale::Zh => format!("已成功导出到：{}", path),
        }
    }

    pub fn export_failed(&self, err: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Export failed: {}", err),
            Locale::Zh => format!("导出失败：{}", err),
        }
    }

    pub fn no_output_dir(&self) -> &'static str {
        match self.locale {
            Locale::En => "No output directory set. Please set an output directory.",
            Locale::Zh => "未设置输出目录，请先设置。",
        }
    }

    pub fn now_playing(&self, name: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Now playing: {}", name),
            Locale::Zh => format!("正在播放：{}", name),
        }
    }

    pub fn failed_load_audio(&self, name: impl Display, err: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Failed to load audio '{}': {}", name, err),
            Locale::Zh => format!("加载音频「{}」失败：{}", name, err),
        }
    }

    pub fn audio_player_not_initialized(&self) -> &'static str {
        match self.locale {
            Locale::En => "Audio player is not initialized",
            Locale::Zh => "音频播放器未初始化",
        }
    }

    pub fn no_file_for_playback(&self) -> &'static str {
        match self.locale {
            Locale::En => "No file selected for playback",
            Locale::Zh => "未选择用于播放的文件",
        }
    }

    pub fn invalid_audio_index(&self, idx: usize, max: usize) -> String {
        match self.locale {
            Locale::En => format!("Invalid audio index: {} (max: {})", idx, max),
            Locale::Zh => format!("无效的音频索引：{}（最大：{}）", idx, max),
        }
    }

    pub fn replace_failed(&self, err: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Replace failed: {}", err),
            Locale::Zh => format!("替换失败：{}", err),
        }
    }

    pub fn configure_loop_for(&self, name: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Please configure loop settings for: {}", name),
            Locale::Zh => format!("请为「{}」配置循环设置", name),
        }
    }

    pub fn confirm_delete_audio_body(&self, name: impl Display) -> String {
        match self.locale {
            Locale::En => format!(
                "Are you sure you want to delete the audio \"{}\"? This action cannot be undone.",
                name
            ),
            Locale::Zh => format!(
                "确定要删除音频「{}」吗？此操作无法撤销。",
                name
            ),
        }
    }

    pub fn exported_count_to(&self, count: usize, dir: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Successfully exported {} files to: {}", count, dir),
            Locale::Zh => format!("已成功导出 {} 个文件到：{}", count, dir),
        }
    }

    pub fn failed_replace_key(&self, key: impl Display, err: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Failed to replace {}: {}", key, err),
            Locale::Zh => format!("替换 {} 失败：{}", key, err),
        }
    }

    pub fn replaced_empty_wav(&self, n: usize) -> String {
        match self.locale {
            Locale::En => format!(
                "Replaced {} item(s) with empty WAV (names/ids preserved)",
                n
            ),
            Locale::Zh => format!("已将 {} 项替换为空 WAV（名称/ID 保留）", n),
        }
    }

    pub fn no_matching_replace(&self) -> &'static str {
        match self.locale {
            Locale::En => "No matching selected items to replace",
            Locale::Zh => "没有可替换的所选匹配项",
        }
    }

    pub fn debug_convert_bank_only(&self) -> &'static str {
        match self.locale {
            Locale::En => "Debug convert is only available for .nus3bank files",
            Locale::Zh => "调试转换仅适用于 .nus3bank",
        }
    }

    pub fn failed_open_bank(&self, err: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Failed to open .nus3bank: {}", err),
            Locale::Zh => format!("打开 .nus3bank 失败：{}", err),
        }
    }

    pub fn convert_failed_for(&self, name: impl Display, err: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Convert failed for {}: {}", name, err),
            Locale::Zh => format!("转换 {} 失败：{}", name, err),
        }
    }

    pub fn debug_convert_done(&self, c: usize, s: usize, f: usize) -> String {
        match self.locale {
            Locale::En => format!(
                "Debug convert done: converted={}, skipped={}, failed={}",
                c, s, f
            ),
            Locale::Zh => format!(
                "调试转换完成：已转换={}，跳过={}，失败={}",
                c, s, f
            ),
        }
    }

    pub fn failed_mark_deletion(&self, err: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Failed to mark for deletion: {}", err),
            Locale::Zh => format!("标记删除失败：{}", err),
        }
    }

    pub fn marked_for_deletion_count(&self, n: usize) -> String {
        match self.locale {
            Locale::En => format!("Successfully marked {} item(s) for deletion", n),
            Locale::Zh => format!("已成功标记 {} 项为删除", n),
        }
    }

    pub fn no_matching_in_list(&self) -> &'static str {
        match self.locale {
            Locale::En => "No matching selected items found in list",
            Locale::Zh => "列表中找不到所选匹配项",
        }
    }

    pub fn no_audio_list(&self) -> &'static str {
        match self.locale {
            Locale::En => "No audio list loaded",
            Locale::Zh => "未加载音频列表",
        }
    }

    pub fn marked_deleted_one(&self, name: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Successfully marked for deletion: {}", name),
            Locale::Zh => format!("已成功标记删除：{}", name),
        }
    }

    pub fn no_audio_path(&self) -> &'static str {
        match self.locale {
            Locale::En => "No audio file path available",
            Locale::Zh => "无可用音频路径",
        }
    }

    pub fn added_wav(&self, name: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Successfully added new audio (converted to WAV): {}", name),
            Locale::Zh => format!("已成功添加新音频（已转 WAV）：{}", name),
        }
    }

    pub fn register_wav_failed(&self, err: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Failed to register WAV audio: {}", err),
            Locale::Zh => format!("注册 WAV 音频失败：{}", err),
        }
    }

    pub fn added_original(&self, name: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Successfully added new audio (original format): {}", name),
            Locale::Zh => format!("已成功添加新音频（原始格式）：{}", name),
        }
    }

    pub fn failed_add_audio(&self, err: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Failed to add audio: {}", err),
            Locale::Zh => format!("添加音频失败：{}", err),
        }
    }

    pub fn no_audio_data(&self) -> &'static str {
        match self.locale {
            Locale::En => "No audio data available",
            Locale::Zh => "无可用音频数据",
        }
    }

    pub fn name_and_id_required(&self) -> &'static str {
        match self.locale {
            Locale::En => "Name and ID cannot be empty",
            Locale::Zh => "名称与 ID 不能为空",
        }
    }

    pub fn id_must_be_valid_number(&self) -> &'static str {
        match self.locale {
            Locale::En => "ID must be a valid number",
            Locale::Zh => "ID 必须是有效数字",
        }
    }

    pub fn failed_process_new_audio(&self, err: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Failed to process new audio: {}", err),
            Locale::Zh => format!("处理新音频失败：{}", err),
        }
    }

    pub fn add_audio_failed(&self, err: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Add audio failed: {}", err),
            Locale::Zh => format!("添加音频失败：{}", err),
        }
    }

    pub fn no_replacement_path(&self) -> &'static str {
        match self.locale {
            Locale::En => "No replacement file path found",
            Locale::Zh => "未找到替换文件路径",
        }
    }

    pub fn failed_process_replacement_key(&self, key: impl Display, err: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Failed to process replacement for {}: {}", key, err),
            Locale::Zh => format!("处理 {} 的替换失败：{}", key, err),
        }
    }

    pub fn loop_word_start(&self) -> &'static str {
        match self.locale {
            Locale::En => "start",
            Locale::Zh => "起点",
        }
    }

    pub fn loop_word_end(&self) -> &'static str {
        match self.locale {
            Locale::En => "end",
            Locale::Zh => "终点",
        }
    }

    pub fn loop_parenthetical_range(&self, start: &str, end: &str) -> String {
        match self.locale {
            Locale::En => format!(" (Loop: {} to {})", start, end),
            Locale::Zh => format!("（循环：{} → {}）", start, end),
        }
    }

    pub fn loop_parenthetical_full(&self) -> &'static str {
        match self.locale {
            Locale::En => " (Full track loop)",
            Locale::Zh => "（整曲循环）",
        }
    }

    pub fn replaced_in_memory_count(&self, n: usize, loop_msg: &str) -> String {
        match self.locale {
            Locale::En => format!(
                "Successfully replaced {} item(s) in memory{}",
                n, loop_msg
            ),
            Locale::Zh => format!(
                "已在内存中替换 {} 项{}",
                n, loop_msg
            ),
        }
    }

    pub fn replaced_in_memory_one(&self, name: impl Display, loop_msg: &str) -> String {
        match self.locale {
            Locale::En => format!(
                "Successfully replaced audio in memory: {}{}",
                name, loop_msg
            ),
            Locale::Zh => format!(
                "已在内存中替换音频：{}{}",
                name, loop_msg
            ),
        }
    }

    pub fn failed_process_replacement(&self, err: impl Display) -> String {
        match self.locale {
            Locale::En => format!("Failed to process replacement: {}", err),
            Locale::Zh => format!("处理替换失败：{}", err),
        }
    }

    pub fn prepare_playback_audio_failed(&self) -> &'static str {
        match self.locale {
            Locale::En => "Failed to prepare playback audio",
            Locale::Zh => "准备回放音频失败",
        }
    }
}
