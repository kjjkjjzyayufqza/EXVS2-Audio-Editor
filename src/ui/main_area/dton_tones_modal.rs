use egui::{Color32, Context, ScrollArea, Ui, Window};

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
        }
    }

    pub fn open_for_file(&mut self, file_path: &str) {
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
        let mut open = self.open;
        let was_open = open;
        let available_rect = ctx.available_rect();
        let min_width = available_rect.width() * 0.8;
        let min_height = available_rect.height() * 0.7;

        Window::new("Edit DTON Tones")
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
            self.flush_pending();
        }
    }

    fn render(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("DTON Tones Editor");
        });

        let Some(path) = self.file_path.as_deref() else {
            ui.colored_label(Color32::RED, "No file selected.");
            return;
        };

        ui.label(format!("File: {}", path));
        if let Some(err) = self.error.as_deref() {
            ui.add_space(6.0);
            ui.colored_label(Color32::RED, err);
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search_query);
            ui.add_space(12.0);
            ui.label(format!("Total: {}", self.tones.len()));
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Reload from File").clicked() {
                self.reload_from_file();
            }
            ui.checkbox(&mut self.keep_original_length, "Keep original data length");
            ui.checkbox(&mut self.advanced_fields, "Enable advanced fields");
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        ui.columns(2, |cols| {
            self.render_left_list(&mut cols[0]);
            self.render_right_details(&mut cols[1]);
        });

        self.flush_pending();
    }

    fn render_left_list(&mut self, ui: &mut Ui) {
        ui.heading("Tones");
        ui.add_space(6.0);

        let indices = self.visible_indices();
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for idx in indices {
                    let name = self.tones.get(idx).map(|t| t.name.as_str()).unwrap_or("");
                    let len = self.tones.get(idx).map(|t| t.data.len()).unwrap_or(0);
                    let selected = self.selected_index == Some(idx);
                    let label = format!("{:3}  {:<24}  len={}", idx, name, len);
                    if ui.selectable_label(selected, label).clicked() {
                        self.selected_index = Some(idx);
                        self.sync_data_text_from_selected();
                    }
                }
            });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Add").clicked() {
                self.add_new_tone();
            }
            if ui.button("Duplicate").clicked() {
                self.duplicate_selected();
            }
            if ui.button("Delete").clicked() {
                self.delete_selected();
            }
        });
    }

    fn render_right_details(&mut self, ui: &mut Ui) {
        ui.heading("Details");
        ui.add_space(6.0);

        let Some(idx) = self.selected_index else {
            ui.label("Select a tone on the left.");
            return;
        };
        if idx >= self.tones.len() {
            ui.colored_label(Color32::RED, "Selected index out of range.");
            return;
        }

        let tone = &mut self.tones[idx];

        ui.horizontal(|ui| {
            ui.label("Name:");
            let resp = ui.text_edit_singleline(&mut tone.name);
            if resp.changed() {
                self.dirty = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Data length:");
            ui.label(format!("{}", tone.data.len()));
            if self.keep_original_length {
                let orig = self.original_data_lens.get(idx).copied().unwrap_or(tone.data.len());
                ui.add_space(8.0);
                ui.label(format!("(original: {})", orig));
            }
        });

        if self.advanced_fields {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("hash (i32):");
                let resp = ui.add(egui::DragValue::new(&mut tone.hash));
                if resp.changed() {
                    self.dirty = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("unk1 (i32):");
                let resp = ui.add(egui::DragValue::new(&mut tone.unk1));
                if resp.changed() {
                    self.dirty = true;
                }
            });
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        ui.label("Data (floats, separated by spaces/commas/newlines):");
        ui.add_space(4.0);
        
        ui.push_id(format!("dton_data_text_{}", idx), |ui| {
            let data_area_height = ui.available_height() * 0.4;
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
        if idx >= self.tones.len() {
            self.data_parse_error = Some("Selected index out of range.".to_string());
            return;
        }

        match parse_f32_list(&self.data_text) {
            Ok(values) => {
                if self.keep_original_length {
                    let expected = self.original_data_lens.get(idx).copied().unwrap_or(values.len());
                    if values.len() != expected {
                        self.data_parse_error = Some(format!(
                            "Data length mismatch: got {}, expected {}",
                            values.len(),
                            expected
                        ));
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
        let orig_len = self.original_data_lens.get(idx).copied().unwrap_or(cloned.data.len());
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
        self.tones.remove(idx);
        self.original_data_lens.remove(idx);

        if self.tones.is_empty() {
            self.selected_index = None;
            self.data_text.clear();
        } else {
            let next = idx.min(self.tones.len().saturating_sub(1));
            self.selected_index = Some(next);
            self.sync_data_text_from_selected();
        }
        self.dirty = true;
    }

    fn flush_pending(&mut self) {
        if !self.dirty {
            return;
        }
        if self.data_parse_error.is_some() {
            return;
        }
        let Some(path) = self.file_path.as_deref() else {
            self.error = Some("No file selected for DTON edit".to_string());
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
        if let Some(pending) = dton_pending::get(file_path) {
            return Ok(pending);
        }

        let file = Nus3bankFile::open(file_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;
        Ok(file
            .dton
            .map(|d| d.tones)
            .unwrap_or_else(|| Vec::new()))
    }

    fn reload_from_file(&mut self) {
        let Some(path) = self.file_path.as_deref() else {
            self.error = Some("No file selected for DTON edit".to_string());
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
            Err(e) => self.error = Some(format!("Failed to open NUS3BANK file: {}", e)),
        }
    }
}

pub fn apply_dton_tones_to_file(file: &mut Nus3bankFile, tones: Vec<ToneDes>) {
    if tones.is_empty() {
        file.dton = None;
        return;
    }
    file.dton = Some(DtonSection { tones });
}

fn parse_f32_list(text: &str) -> Result<Vec<f32>, String> {
    let mut out = Vec::new();
    for (i, tok) in text
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .enumerate()
    {
        let v: f32 = tok
            .parse()
            .map_err(|_| format!("Failed to parse float at token {}: '{}'", i, tok))?;
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

