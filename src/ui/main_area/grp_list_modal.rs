use egui::{Color32, Context, ScrollArea, Ui, Window};

use crate::i18n::{I18n, Locale};
use crate::nus3bank::structures::{GrpSection, Nus3bankFile};

use super::{grp_pending, grp_template};

pub struct GrpListModal {
    pub open: bool,
    file_path: Option<String>,
    names: Vec<String>,
    search_query: String,
    find_text: String,
    replace_text: String,
    error: Option<String>,
    dirty: bool,
    locale: Locale,

    // Cache for performance
    visible_indices_cache: Vec<usize>,
    last_search_query: String,
    scroll_offset: f32,
}

impl Default for GrpListModal {
    fn default() -> Self {
        Self::new()
    }
}

impl GrpListModal {
    pub fn new() -> Self {
        Self {
            open: false,
            file_path: None,
            names: Vec::new(),
            search_query: String::new(),
            find_text: String::new(),
            replace_text: String::new(),
            error: None,
            dirty: false,
            locale: Locale::detect_system(),
            visible_indices_cache: Vec::new(),
            last_search_query: String::new(),
            scroll_offset: 0.0,
        }
    }

    pub fn open_for_file(&mut self, file_path: &str, locale: Locale) {
        self.locale = locale;
        self.file_path = Some(file_path.to_string());
        self.error = None;
        self.dirty = false;
        self.search_query.clear();
        self.last_search_query.clear();
        self.visible_indices_cache.clear();
        self.scroll_offset = 0.0;

        match self.load_names_for_file(file_path) {
            Ok(names) => {
                self.names = names;
                self.update_visible_indices_cache();
            }
            Err(e) => {
                self.names.clear();
                self.error = Some(e);
            }
        }

        self.open = true;
    }

    pub fn show(&mut self, ctx: &Context) {
        self.locale = I18n::from_ctx(ctx).locale;
        let mut open = self.open;
        let was_open = open;
        let available_rect = ctx.available_rect();
        let min_width = available_rect.width() * 0.7;
        let min_height = available_rect.height() * 0.7;

        Window::new(I18n::new(self.locale).edit_grp_list_title())
            .open(&mut open)
            .min_width(min_width)
            .min_height(min_height)
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                self.render(ui);
            });

        self.open = open;

        if was_open && !self.open {
            // If the user closed the window via the X button, persist any pending changes.
            self.flush_pending();
        }
    }

    fn render(&mut self, ui: &mut Ui) {
        let t = I18n::new(self.locale);
        ui.vertical_centered(|ui| {
            ui.heading(t.grp_names_editor());
        });

        if let Some(path) = self.file_path.as_deref() {
            ui.label(t.file_label_fmt(path));
        } else {
            ui.colored_label(Color32::RED, t.no_file_selected_short());
            return;
        }

        if let Some(err) = self.error.as_deref() {
            ui.add_space(6.0);
            ui.colored_label(Color32::RED, err);
        }

        ui.add_space(8.0);
        ui.separator();

        let search_changed = ui
            .horizontal(|ui| {
                ui.label(t.search_label());
                let resp = ui.text_edit_singleline(&mut self.search_query);

                ui.add_space(12.0);
                ui.label(t.total_label(self.names.len()));
                ui.add_space(12.0);
                ui.label(t.visible_label(self.visible_indices_cache.len()));

                resp.changed()
            })
            .inner;

        if search_changed {
            self.update_visible_indices_cache();
        }

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label(t.find_label());
            ui.text_edit_singleline(&mut self.find_text);
            ui.label(t.replace_label());
            ui.text_edit_singleline(&mut self.replace_text);

            if ui.button(t.replace_in_visible()).clicked() {
                self.replace_in_visible();
                self.dirty = true;
            }
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(t.add_row()).clicked() {
                self.names.push(String::new());
                self.update_visible_indices_cache();
                self.dirty = true;
            }
            if ui.button(t.replace_with_template()).clicked() {
                self.replace_with_template();
                self.update_visible_indices_cache();
                self.dirty = true;
            }
            if ui.button(t.reload_from_file()).clicked() {
                self.reload_from_file();
                self.update_visible_indices_cache();
                self.dirty = false;
            }
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        let mut needs_flush = false;

        self.render_virtual_list(ui, &mut needs_flush);

        if needs_flush || self.dirty {
            self.flush_pending();
        }
    }

    fn update_visible_indices_cache(&mut self) {
        let q = self.search_query.trim();
        if q.is_empty() {
            self.visible_indices_cache = (0..self.names.len()).collect();
        } else {
            let needle = q.to_lowercase();
            self.visible_indices_cache = self
                .names
                .iter()
                .enumerate()
                .filter_map(|(i, s)| {
                    if s.to_lowercase().contains(&needle) {
                        Some(i)
                    } else {
                        None
                    }
                })
                .collect();
        }
        self.last_search_query = self.search_query.clone();
    }

    fn render_virtual_list(&mut self, ui: &mut Ui, needs_flush: &mut bool) {
        let t = I18n::new(self.locale);
        const ROW_HEIGHT: f32 = 28.0;
        const OVERSCAN: usize = 5;

        let available_height = ui.available_height();
        let total_rows = self.visible_indices_cache.len();

        let visible_rows = (available_height / ROW_HEIGHT).ceil() as usize + 1;

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let scroll_offset = ui.clip_rect().min.y - ui.cursor().min.y;
                let start_row =
                    ((scroll_offset.abs() / ROW_HEIGHT).floor() as usize).saturating_sub(OVERSCAN);
                let end_row = (start_row + visible_rows + OVERSCAN * 2).min(total_rows);

                if start_row > 0 {
                    ui.add_space(start_row as f32 * ROW_HEIGHT);
                }

                for idx in start_row..end_row {
                    if idx >= self.visible_indices_cache.len() {
                        break;
                    }

                    let i = self.visible_indices_cache[idx];
                    if i >= self.names.len() {
                        continue;
                    }

                    ui.horizontal(|ui| {
                        ui.label(format!("{:6}", i));
                        let resp = ui.add_sized(
                            [480.0, 22.0],
                            egui::TextEdit::singleline(&mut self.names[i]),
                        );
                        if resp.changed() {
                            self.dirty = true;
                            *needs_flush = true;
                        }

                        if ui.button(t.clear_cell()).clicked() {
                            self.names[i].clear();
                            self.dirty = true;
                            *needs_flush = true;
                        }
                    });
                }

                let remaining = total_rows.saturating_sub(end_row);
                if remaining > 0 {
                    ui.add_space(remaining as f32 * ROW_HEIGHT);
                }
            });
    }

    fn replace_in_visible(&mut self) {
        let t = I18n::new(self.locale);
        let find = self.find_text.clone();
        if find.is_empty() {
            self.error = Some(t.grp_find_text_empty().to_string());
            return;
        }

        let replace = self.replace_text.clone();
        for &i in &self.visible_indices_cache {
            if i < self.names.len() {
                let updated = self.names[i].replace(&find, &replace);
                self.names[i] = updated;
            }
        }
        self.update_visible_indices_cache();
        self.error = None;
    }

    fn flush_pending(&mut self) {
        let t = I18n::new(self.locale);
        if !self.dirty {
            return;
        }

        let Some(path) = self.file_path.as_deref() else {
            self.error = Some(t.grp_no_file_for_edit().to_string());
            return;
        };

        if let Err(e) = grp_pending::set(path, self.names.clone()) {
            self.error = Some(e);
            return;
        }

        self.error = None;
        self.dirty = false;
    }

    fn load_names_for_file(&self, file_path: &str) -> Result<Vec<String>, String> {
        let t = I18n::new(self.locale);
        if let Some(pending) = grp_pending::get(file_path) {
            return Ok(pending);
        }

        let file = Nus3bankFile::open(file_path).map_err(|e| t.nus3bank_open_failed(e))?;
        Ok(file.grp.map(|g| g.names).unwrap_or_default())
    }

    fn reload_from_file(&mut self) {
        let t = I18n::new(self.locale);
        let Some(path) = self.file_path.as_deref() else {
            self.error = Some(t.grp_no_file_for_edit().to_string());
            return;
        };
        self.error = None;
        let _ = grp_pending::clear(path);
        match Nus3bankFile::open(path) {
            Ok(file) => {
                self.names = file.grp.map(|g| g.names).unwrap_or_default();
                self.update_visible_indices_cache();
            }
            Err(e) => self.error = Some(t.nus3bank_open_failed(e)),
        }
    }

    fn replace_with_template(&mut self) {
        let t = I18n::new(self.locale);
        let template = grp_template::grp_template_names();
        if template.is_empty() {
            self.error = Some(t.grp_template_empty().to_string());
            return;
        }

        self.names = template;
        self.update_visible_indices_cache();
        self.error = None;
    }
}

pub fn apply_grp_names_to_file(file: &mut Nus3bankFile, names: Vec<String>) {
    file.grp = Some(GrpSection { names });
}
