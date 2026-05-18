use egui::{Color32, Context, ScrollArea, Ui, Window};

use crate::i18n::{I18n, Locale};
use crate::nus3bank::structures::{DtonSection, Nus3bankFile, ToneDes};

use super::dton_pending;

pub struct DtonTonesModal {
    pub open: bool,
    file_path: Option<String>,
    tones: Vec<ToneDes>,
    original_data_lens: Vec<usize>,

    search_query: String,
    selected_index: Option<usize>,

    advanced_fields: bool,
    keep_original_length: bool,

    data_text: String,
    data_parse_error: Option<String>,
    error: Option<String>,
    dirty: bool,
    locale: Locale,
}

impl Default for DtonTonesModal {
    fn default() -> Self {
        Self::new()
    }
}

impl DtonTonesModal {
    pub fn new() -> Self {
        Self {
            open: false,
            file_path: None,
            tones: Vec::new(),
            original_data_lens: Vec::new(),
            search_query: String::new(),
            selected_index: None,
            advanced_fields: false,
            keep_original_length: true,
            data_text: String::new(),
            data_parse_error: None,
            error: None,
            dirty: false,
            locale: Locale::detect_system(),
        }
    }

    pub fn open_for_file(&mut self, file_path: &str, locale: Locale) {
        self.locale = locale;
        self.file_path = Some(file_path.to_string());
        self.error = None;
        self.data_parse_error = None;
        self.dirty = false;

        match self.load_tones_for_file(file_path) {
            Ok(tones) => {
                self.tones = tones;
                self.original_data_lens = self.tones.iter().map(|t| t.data.len()).collect();
                self.selected_index = if self.tones.is_empty() { None } else { Some(0) };
                self.sync_data_text_from_selected();
            }
            Err(e) => {
                self.tones.clear();
                self.original_data_lens.clear();
                self.selected_index = None;
                self.data_text.clear();
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
        let default_width = available_rect.width() * 0.7;
        let default_height = available_rect.height() * 0.7;

        Window::new(I18n::new(self.locale).edit_dton_title())
            .open(&mut open)
            .default_width(default_width)
            .default_height(default_height)
            .min_width(available_rect.width() * 0.5)
            .min_height(available_rect.height() * 0.5)
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                self.render(ui);
            });

        self.open = open;
        if was_open && !self.open {
            self.flush_pending();
        }
    }

    fn render(&mut self, ui: &mut Ui) {
        let t = I18n::new(self.locale);
        ui.vertical_centered(|ui| {
            ui.heading(t.dton_editor_heading());
        });

        let Some(path) = self.file_path.as_deref() else {
            ui.colored_label(Color32::RED, t.no_file_selected_short());
            return;
        };

        ui.label(t.file_label_fmt(path));
        if let Some(err) = self.error.as_deref() {
            ui.add_space(6.0);
            ui.colored_label(Color32::RED, err);
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label(t.search_label());
            ui.text_edit_singleline(&mut self.search_query);
            ui.add_space(12.0);
            ui.label(t.total_label(self.tones.len()));
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(t.reload_from_file()).clicked() {
                self.reload_from_file();
            }
            ui.checkbox(&mut self.keep_original_length, t.keep_original_length());
            ui.checkbox(&mut self.advanced_fields, t.enable_advanced_fields());
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        let available_height = ui.available_height();
        ui.columns(2, |cols| {
            self.render_left_list(&mut cols[0], available_height, &t);
            self.render_right_details(&mut cols[1], available_height, &t);
        });

        self.flush_pending();
    }

    fn render_left_list(&mut self, ui: &mut Ui, available_height: f32, t: &I18n) {
        ui.heading(t.tones_heading());
        ui.add_space(6.0);

        let indices = self.visible_indices();
        let row_height = 22.0;
        let total_rows = indices.len();

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(available_height - 100.0) // Reserve space for buttons and search
            .show_rows(ui, row_height, total_rows, |ui, row_range| {
                for i in row_range {
                    let idx = indices[i];
                    let name = self.tones.get(idx).map(|t| t.name.as_str()).unwrap_or("");
                    let len = self.tones.get(idx).map(|t| t.data.len()).unwrap_or(0);
                    let selected = self.selected_index == Some(idx);
                    let label = format!("{:3}  {:<24}  {}", idx, name, t.dton_len_label(len));
                    if ui.selectable_label(selected, label).clicked() {
                        self.selected_index = Some(idx);
                        self.sync_data_text_from_selected();
                    }
                }
            });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(t.add_audio_btn()).clicked() {
                self.add_new_tone();
            }
            if ui.button(t.duplicate_row()).clicked() {
                self.duplicate_selected();
            }
            if ui.button(t.delete_row()).clicked() {
                self.delete_selected();
            }
        });
    }

    fn render_right_details(&mut self, ui: &mut Ui, available_height: f32, t: &I18n) {
        ui.heading(t.details_heading());
        ui.add_space(6.0);

        let Some(idx) = self.selected_index else {
            ui.label(t.select_tone_left());
            return;
        };
        if idx >= self.tones.len() {
            ui.colored_label(Color32::RED, t.index_out_of_range());
            return;
        }

        let tone = &mut self.tones[idx];

        ui.horizontal(|ui| {
            ui.label(t.name_label());
            let resp = ui.text_edit_singleline(&mut tone.name);
            if resp.changed() {
                self.dirty = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label(t.data_length_label());
            ui.label(format!("{}", tone.data.len()));
            if self.keep_original_length {
                let orig = self
                    .original_data_lens
                    .get(idx)
                    .copied()
                    .unwrap_or(tone.data.len());
                ui.add_space(8.0);
                ui.label(t.dton_original_len(orig));
            }
        });

        if self.advanced_fields {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(t.dton_field_hash());
                let resp = ui.add(egui::DragValue::new(&mut tone.hash));
                if resp.changed() {
                    self.dirty = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(t.dton_field_unk1());
                let resp = ui.add(egui::DragValue::new(&mut tone.unk1));
                if resp.changed() {
                    self.dirty = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Raw descriptor bytes");
                ui.label(format!("{}", tone.raw_data.len()));
            });
            ui.horizontal(|ui| {
                ui.label("Descriptor words");
                ui.label(format!("{:?}", tone.descriptor_words));
            });
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        ui.label(t.data_floats_label());
        ui.add_space(4.0);

        ui.push_id(format!("dton_data_text_{}", idx), |ui| {
            let data_area_height = (available_height - 250.0).max(150.0);
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(data_area_height)
                .show(ui, |ui| {
                    let resp = ui.add(
                        egui::TextEdit::multiline(&mut self.data_text)
                            .desired_rows(12)
                            .desired_width(f32::INFINITY),
                    );
                    if resp.changed() {
                        self.try_apply_data_text(idx);
                    }
                });
        });

        if let Some(err) = self.data_parse_error.as_deref() {
            ui.add_space(6.0);
            ui.colored_label(Color32::RED, err);
        }
    }

    fn visible_indices(&self) -> Vec<usize> {
        let q = self.search_query.trim();
        if q.is_empty() {
            return (0..self.tones.len()).collect();
        }
        let needle = q.to_lowercase();
        self.tones
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                if t.name.to_lowercase().contains(&needle) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }

    fn sync_data_text_from_selected(&mut self) {
        self.data_parse_error = None;
        let Some(idx) = self.selected_index else {
            self.data_text.clear();
            return;
        };
        if idx >= self.tones.len() {
            self.data_text.clear();
            return;
        }
        let tone = &self.tones[idx];
        self.data_text = floats_to_text(&tone.data);
    }

    fn try_apply_data_text(&mut self, idx: usize) {
        let t = I18n::new(self.locale);
        if idx >= self.tones.len() {
            self.data_parse_error = Some(t.index_out_of_range().to_string());
            return;
        }

        match parse_f32_list(&self.data_text, &t) {
            Ok(values) => {
                if self.keep_original_length {
                    let expected = self
                        .original_data_lens
                        .get(idx)
                        .copied()
                        .unwrap_or(values.len());
                    if values.len() != expected {
                        self.data_parse_error =
                            Some(t.data_length_mismatch(values.len(), expected));
                        return;
                    }
                }

                self.tones[idx].data = values;
                self.data_parse_error = None;
                self.dirty = true;
            }
            Err(e) => {
                self.data_parse_error = Some(e);
            }
        }
    }

    fn add_new_tone(&mut self) {
        let base_len = if self.keep_original_length {
            self.original_data_lens
                .get(self.selected_index.unwrap_or(0))
                .copied()
                .unwrap_or(0)
        } else {
            0
        };

        let tone = ToneDes {
            hash: 0,
            unk1: 0,
            name: String::new(),
            data: vec![0.0; base_len],
            raw_data: vec![0; base_len * 4],
            descriptor_words: Vec::new(),
        };
        self.tones.push(tone);
        self.original_data_lens.push(base_len);
        self.selected_index = Some(self.tones.len().saturating_sub(1));
        self.sync_data_text_from_selected();
        self.dirty = true;
    }

    fn duplicate_selected(&mut self) {
        let Some(idx) = self.selected_index else {
            return;
        };
        if idx >= self.tones.len() {
            return;
        }
        let cloned = self.tones[idx].clone();
        let orig_len = self
            .original_data_lens
            .get(idx)
            .copied()
            .unwrap_or(cloned.data.len());
        self.tones.push(cloned);
        self.original_data_lens.push(orig_len);
        self.selected_index = Some(self.tones.len().saturating_sub(1));
        self.sync_data_text_from_selected();
        self.dirty = true;
    }

    fn delete_selected(&mut self) {
        let Some(idx) = self.selected_index else {
            return;
        };
        if idx >= self.tones.len() {
            return;
        }
        self.tones[idx].name.clear();
        self.tones[idx].data.clear();
        self.tones[idx].raw_data.clear();
        self.tones[idx].descriptor_words.clear();
        self.original_data_lens[idx] = 0;
        self.sync_data_text_from_selected();
        self.dirty = true;
    }

    fn flush_pending(&mut self) {
        let t = I18n::new(self.locale);
        if !self.dirty {
            return;
        }
        if self.data_parse_error.is_some() {
            return;
        }
        let Some(path) = self.file_path.as_deref() else {
            self.error = Some(t.dton_no_file_for_edit().to_string());
            return;
        };
        if let Err(e) = dton_pending::set(path, self.tones.clone()) {
            self.error = Some(e);
            return;
        }
        self.error = None;
        self.dirty = false;
    }

    fn load_tones_for_file(&self, file_path: &str) -> Result<Vec<ToneDes>, String> {
        let t = I18n::new(self.locale);
        if let Some(pending) = dton_pending::get(file_path) {
            return Ok(pending);
        }

        let file = Nus3bankFile::open(file_path).map_err(|e| t.nus3bank_open_failed(e))?;
        Ok(file.dton.map(|d| d.tones).unwrap_or_else(|| Vec::new()))
    }

    fn reload_from_file(&mut self) {
        let t = I18n::new(self.locale);
        let Some(path) = self.file_path.as_deref() else {
            self.error = Some(t.dton_no_file_for_edit().to_string());
            return;
        };
        self.error = None;
        let _ = dton_pending::clear(path);

        match Nus3bankFile::open(path) {
            Ok(file) => {
                self.tones = file.dton.map(|d| d.tones).unwrap_or_default();
                self.original_data_lens = self.tones.iter().map(|t| t.data.len()).collect();
                self.selected_index = if self.tones.is_empty() { None } else { Some(0) };
                self.sync_data_text_from_selected();
                self.dirty = false;
                self.data_parse_error = None;
            }
            Err(e) => self.error = Some(t.nus3bank_open_failed(e)),
        }
    }
}

pub fn apply_dton_tones_to_file(file: &mut Nus3bankFile, tones: Vec<ToneDes>) {
    file.dton = Some(DtonSection { tones });
}

fn parse_f32_list(text: &str, t: &I18n) -> Result<Vec<f32>, String> {
    let mut out = Vec::new();
    for (i, tok) in text
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .enumerate()
    {
        let v: f32 = tok
            .parse()
            .map_err(|_| t.parse_float_token_failed(i, tok))?;
        out.push(v);
    }
    Ok(out)
}

fn floats_to_text(values: &[f32]) -> String {
    // One value per line for stable editing.
    let mut s = String::new();
    for (i, v) in values.iter().enumerate() {
        if i != 0 {
            s.push('\n');
        }
        s.push_str(&format!("{v}"));
    }
    s
}
