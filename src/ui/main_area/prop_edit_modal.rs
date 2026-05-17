use egui::{Color32, Context, ScrollArea, Ui, Window};

use crate::i18n::{I18n, Locale};
use crate::nus3bank::structures::{Nus3bankFile, PropLayout, PropSection};

use super::prop_pending;

pub struct PropEditModal {
    pub open: bool,
    file_path: Option<String>,
    prop: Option<PropSection>,
    error: Option<String>,
    dirty: bool,
    locale: Locale,

    // Preset values
    presets: Vec<DebugPreset>,
    selected_preset: Option<usize>,
}

#[derive(Clone, Debug)]
struct DebugPreset {
    name: String,
    project: String,
    timestamp: String,
    unk1: i32,
    reserved_u16: u16,
    unk2: u16,
    unk3: u16,
    layout: PropLayout,
}

impl Default for PropEditModal {
    fn default() -> Self {
        Self::new()
    }
}

impl PropEditModal {
    pub fn new() -> Self {
        let presets = vec![
            DebugPreset {
                name: "Preset 1: Test (Minimal)".to_string(),
                project: "Test".to_string(),
                timestamp: String::new(),
                unk1: 17,
                reserved_u16: 1,
                unk2: 3,
                unk3: 0,
                layout: PropLayout::Minimal,
            },
            DebugPreset {
                name: "Preset 2: DefaultProject (Extended)".to_string(),
                project: "DefaultProject".to_string(),
                timestamp: String::new(),
                unk1: 241,
                reserved_u16: 0,
                unk2: 3,
                unk3: 0,
                layout: PropLayout::Extended,
            },
            DebugPreset {
                name: "Preset 3: DefaultProject (Extended v2)".to_string(),
                project: "DefaultProject".to_string(),
                timestamp: String::new(),
                unk1: 241,
                reserved_u16: 1,
                unk2: 3,
                unk3: 0,
                layout: PropLayout::Extended,
            },
        ];

        Self {
            open: false,
            file_path: None,
            prop: None,
            error: None,
            dirty: false,
            locale: Locale::detect_system(),
            presets,
            selected_preset: None,
        }
    }

    pub fn open_for_file(&mut self, file_path: &str, locale: Locale) {
        self.locale = locale;
        self.file_path = Some(file_path.to_string());
        self.error = None;
        self.dirty = false;
        self.selected_preset = None;

        match self.load_prop_for_file(file_path) {
            Ok(prop) => {
                self.prop = prop;
            }
            Err(e) => {
                self.prop = None;
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

        Window::new(I18n::new(self.locale).edit_prop_title())
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
        let t = I18n::new(self.locale);
        ui.vertical_centered(|ui| {
            ui.heading(t.prop_section_editor());
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

        if ui.button(t.reload_from_file()).clicked() {
            self.reload_from_file();
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        if self.prop.is_none() {
            ui.colored_label(Color32::YELLOW, t.no_prop_section());
            ui.add_space(8.0);
            if ui.button(t.create_new_prop()).clicked() {
                self.create_default_prop();
            }
            return;
        }

        // Preset selector
        self.render_presets(ui, &t);

        // Main editor
        self.render_prop_editor(ui, &t);
    }

    fn render_presets(&mut self, ui: &mut Ui, t: &I18n) {
        ui.group(|ui| {
            ui.heading(t.presets_heading());
            ui.add_space(6.0);

            let preset_list_height = ui.available_height() * 0.3;
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(preset_list_height)
                .show(ui, |ui| {
                    for (idx, preset) in self.presets.iter().enumerate() {
                        let selected = self.selected_preset == Some(idx);
                        let label = preset_label(idx, preset, t);
                        if ui.selectable_label(selected, label).clicked() {
                            self.selected_preset = Some(idx);
                        }
                    }
                });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(t.apply_selected_preset()).clicked() {
                    self.apply_preset();
                }
                if ui.button(t.save_current_as_preset()).clicked() {
                    self.save_as_preset(t);
                }
            });
        });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);
    }

    fn render_prop_editor(&mut self, ui: &mut Ui, t: &I18n) {
        let Some(prop) = self.prop.as_mut() else {
            return;
        };

        ui.push_id("prop_editor_fields", |ui| {
            let editor_height = ui.available_height() * 0.6;
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(editor_height)
                .show(ui, |ui| {
                    ui.group(|ui| {
                        ui.heading(t.basic_fields());
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label(t.project_label());
                            let resp = ui.text_edit_singleline(&mut prop.project);
                            if resp.changed() {
                                self.dirty = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(t.timestamp_label());
                            let resp = ui.text_edit_singleline(&mut prop.timestamp);
                            if resp.changed() {
                                self.dirty = true;
                            }
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.heading(t.layout_heading());
                        ui.horizontal(|ui| {
                            ui.label(t.layout_type_label());
                            let layout_minimal = prop.layout == PropLayout::Minimal;
                            if ui.radio(layout_minimal, t.layout_minimal()).clicked() {
                                prop.layout = PropLayout::Minimal;
                                self.dirty = true;
                            }
                            if ui.radio(!layout_minimal, t.layout_extended()).clicked() {
                                prop.layout = PropLayout::Extended;
                                self.dirty = true;
                            }
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.heading(t.advanced_fields());
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label(t.prop_field_unk1());
                            let resp = ui.add(egui::DragValue::new(&mut prop.unk1));
                            if resp.changed() {
                                self.dirty = true;
                            }
                            if ui.button("?").clicked() {
                                // Info tooltip
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(t.prop_field_reserved_u16());
                            let mut temp_i32 = prop.reserved_u16 as i32;
                            let resp = ui.add(egui::DragValue::new(&mut temp_i32).range(0..=65535));
                            if resp.changed() {
                                prop.reserved_u16 = temp_i32 as u16;
                                self.dirty = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(t.prop_field_unk2());
                            let mut temp_i32 = prop.unk2 as i32;
                            let resp = ui.add(egui::DragValue::new(&mut temp_i32).range(0..=65535));
                            if resp.changed() {
                                prop.unk2 = temp_i32 as u16;
                                self.dirty = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(t.prop_field_unk3());
                            let mut temp_i32 = prop.unk3 as i32;
                            let resp = ui.add(egui::DragValue::new(&mut temp_i32).range(0..=65535));
                            if resp.changed() {
                                prop.unk3 = temp_i32 as u16;
                                self.dirty = true;
                            }
                        });
                    });
                });
        });

        ui.add_space(12.0);

        ui.horizontal(|ui| {
            if self.dirty {
                ui.colored_label(Color32::YELLOW, t.unsaved_changes());
            }
        });
    }

    fn apply_preset(&mut self) {
        let Some(idx) = self.selected_preset else {
            return;
        };
        if idx >= self.presets.len() {
            return;
        }

        let preset = &self.presets[idx];
        self.prop = Some(PropSection {
            project: preset.project.clone(),
            timestamp: preset.timestamp.clone(),
            unk1: preset.unk1,
            reserved_u16: preset.reserved_u16,
            unk2: preset.unk2,
            unk3: preset.unk3,
            layout: preset.layout,
        });
        self.dirty = true;
        self.error = None;
    }

    fn save_as_preset(&mut self, t: &I18n) {
        let Some(prop) = self.prop.as_ref() else {
            return;
        };

        let preset_name = t.prop_custom_preset_name(
            &prop.project,
            prop.layout == PropLayout::Minimal,
        );

        let new_preset = DebugPreset {
            name: preset_name,
            project: prop.project.clone(),
            timestamp: prop.timestamp.clone(),
            unk1: prop.unk1,
            reserved_u16: prop.reserved_u16,
            unk2: prop.unk2,
            unk3: prop.unk3,
            layout: prop.layout,
        };

        self.presets.push(new_preset);
        self.selected_preset = Some(self.presets.len() - 1);
    }

    fn create_default_prop(&mut self) {
        let t = I18n::new(self.locale);
        self.prop = Some(PropSection {
            project: t.default_project_name().to_string(),
            timestamp: String::new(),
            unk1: 241,
            reserved_u16: 0,
            unk2: 3,
            unk3: 0,
            layout: PropLayout::Extended,
        });
        self.dirty = true;
        self.error = None;
    }

    fn flush_pending(&mut self) {
        let t = I18n::new(self.locale);
        if !self.dirty {
            return;
        }
        let Some(path) = self.file_path.as_deref() else {
            self.error = Some(t.prop_no_file_for_edit().to_string());
            return;
        };
        let Some(prop) = self.prop.as_ref() else {
            return;
        };

        if let Err(e) = prop_pending::set(path, prop.clone()) {
            self.error = Some(e);
            return;
        }
        self.error = None;
        self.dirty = false;
    }

    fn load_prop_for_file(&self, file_path: &str) -> Result<Option<PropSection>, String> {
        let t = I18n::new(self.locale);
        if let Some(pending) = prop_pending::get(file_path) {
            return Ok(Some(pending));
        }

        let file = Nus3bankFile::open(file_path).map_err(|e| t.nus3bank_open_failed(e))?;
        Ok(file.prop)
    }

    fn reload_from_file(&mut self) {
        let t = I18n::new(self.locale);
        let Some(path) = self.file_path.as_deref() else {
            self.error = Some(t.prop_no_file_for_edit().to_string());
            return;
        };
        self.error = None;
        let _ = prop_pending::clear(path);

        match Nus3bankFile::open(path) {
            Ok(file) => {
                self.prop = file.prop;
                self.dirty = false;
                self.selected_preset = None;
            }
            Err(e) => self.error = Some(t.nus3bank_open_failed(e)),
        }
    }
}

fn preset_label<'a>(idx: usize, preset: &'a DebugPreset, t: &I18n) -> &'a str {
    match idx {
        0 => t.prop_preset_1(),
        1 => t.prop_preset_2(),
        2 => t.prop_preset_3(),
        _ => preset.name.as_str(),
    }
}

pub fn apply_prop_to_file(file: &mut Nus3bankFile, prop: Option<PropSection>) {
    file.prop = prop;
}
