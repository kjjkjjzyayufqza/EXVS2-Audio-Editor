use egui::{Color32, Context, ScrollArea, Ui, Window};

use crate::{localized, Locale};
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
        self.locale = crate::locale_from_ctx(ctx);
        let mut open = self.open;
        let was_open = open;
        let available_rect = ctx.content_rect();
        let min_width = available_rect.width() * 0.7;
        let min_height = available_rect.height() * 0.7;

        Window::new(localized::edit_prop_title())
            .id(egui::Id::new("prop_edit_modal"))
            .open(&mut open)
            .min_width(min_width)
            .min_height(min_height)
            .resizable(true)
            .movable(true)
            .collapsible(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(available_rect.center())
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
            ui.heading(localized::prop_section_editor());
        });

        let Some(path) = self.file_path.as_deref() else {
            ui.colored_label(Color32::RED, localized::no_file_selected_short());
            return;
        };

        ui.label(localized::file_label_fmt(path));
        if let Some(err) = self.error.as_deref() {
            ui.add_space(6.0);
            ui.colored_label(Color32::RED, err);
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        if ui.button(localized::reload_from_file()).clicked() {
            self.reload_from_file();
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        if self.prop.is_none() {
            ui.colored_label(Color32::YELLOW, localized::no_prop_section());
            ui.add_space(8.0);
            if ui.button(localized::create_new_prop()).clicked() {
                self.create_default_prop();
            }
            return;
        }

        // Preset selector
        self.render_presets(ui);

        // Main editor
        self.render_prop_editor(ui);
    }

    fn render_presets(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.heading(localized::presets_heading());
            ui.add_space(6.0);

            let preset_list_height = ui.available_height() * 0.3;
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(preset_list_height)
                .show(ui, |ui| {
                    for (idx, preset) in self.presets.iter().enumerate() {
                        let selected = self.selected_preset == Some(idx);
                        let label = preset_label(idx, preset);
                        if ui.selectable_label(selected, label).clicked() {
                            self.selected_preset = Some(idx);
                        }
                    }
                });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(localized::apply_selected_preset()).clicked() {
                    self.apply_preset();
                }
                if ui.button(localized::save_current_as_preset()).clicked() {
                    self.save_as_preset();
                }
            });
        });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);
    }

    fn render_prop_editor(&mut self, ui: &mut Ui) {
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
                        ui.heading(localized::basic_fields());
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label(localized::project_label());
                            let resp = ui.text_edit_singleline(&mut prop.project);
                            if resp.changed() {
                                self.dirty = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(localized::timestamp_label());
                            let resp = ui.text_edit_singleline(&mut prop.timestamp);
                            if resp.changed() {
                                self.dirty = true;
                            }
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.heading(localized::layout_heading());
                        ui.horizontal(|ui| {
                            ui.label(localized::layout_type_label());
                            let layout_minimal = prop.layout == PropLayout::Minimal;
                            if ui.radio(layout_minimal, localized::layout_minimal()).clicked() {
                                prop.layout = PropLayout::Minimal;
                                self.dirty = true;
                            }
                            if ui
                                .radio(prop.layout == PropLayout::Extended, localized::layout_extended())
                                .clicked()
                            {
                                prop.layout = PropLayout::Extended;
                                self.dirty = true;
                            }
                            if ui
                                .radio(prop.layout == PropLayout::Bitmask, "OB bitmask")
                                .clicked()
                            {
                                prop.layout = PropLayout::Bitmask;
                                prop.presence_mask |= 1;
                                if prop.version & 0xFFFF_0000 != 0x0003_0000 {
                                    prop.version = 0x0003_0000;
                                }
                                self.dirty = true;
                            }
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.heading(localized::advanced_fields());
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label(localized::prop_field_unk1());
                            let resp = ui.add(egui::DragValue::new(&mut prop.unk1));
                            if resp.changed() {
                                self.dirty = true;
                            }
                            if ui.button("?").clicked() {
                                // Info tooltip
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(localized::prop_field_reserved_u16());
                            let mut temp_i32 = prop.reserved_u16 as i32;
                            let resp = ui.add(egui::DragValue::new(&mut temp_i32).range(0..=65535));
                            if resp.changed() {
                                prop.reserved_u16 = temp_i32 as u16;
                                self.dirty = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(localized::prop_field_unk2());
                            let mut temp_i32 = prop.unk2 as i32;
                            let resp = ui.add(egui::DragValue::new(&mut temp_i32).range(0..=65535));
                            if resp.changed() {
                                prop.unk2 = temp_i32 as u16;
                                self.dirty = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(localized::prop_field_unk3());
                            let mut temp_i32 = prop.unk3 as i32;
                            let resp = ui.add(egui::DragValue::new(&mut temp_i32).range(0..=65535));
                            if resp.changed() {
                                prop.unk3 = temp_i32 as u16;
                                self.dirty = true;
                            }
                        });

                        if prop.layout == PropLayout::Bitmask {
                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(8.0);
                            ui.heading("OB runtime fields");

                            ui.horizontal(|ui| {
                                ui.label("Leading u32");
                                let resp = ui.add(egui::DragValue::new(&mut prop.leading_u32));
                                if resp.changed() {
                                    self.dirty = true;
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label("Presence mask");
                                let resp = ui.add(egui::DragValue::new(&mut prop.presence_mask));
                                if resp.changed() {
                                    prop.presence_mask |= 1;
                                    self.dirty = true;
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label("Version");
                                let resp = ui.add(egui::DragValue::new(&mut prop.version));
                                if resp.changed() {
                                    self.dirty = true;
                                }
                            });

                            ui.horizontal(|ui| {
                                let mut enabled = prop.bit5_u32.is_some();
                                if ui.checkbox(&mut enabled, "Bit 5 u32").changed() {
                                    prop.bit5_u32 = if enabled { Some(0) } else { None };
                                    self.dirty = true;
                                }
                                if let Some(value) = prop.bit5_u32.as_mut() {
                                    let resp = ui.add(egui::DragValue::new(value));
                                    if resp.changed() {
                                        self.dirty = true;
                                    }
                                }
                            });

                            ui.horizontal(|ui| {
                                let mut enabled = prop.bit6_u32.is_some();
                                if ui.checkbox(&mut enabled, "Bit 6 u32").changed() {
                                    prop.bit6_u32 = if enabled { Some(0) } else { None };
                                    self.dirty = true;
                                }
                                if let Some(value) = prop.bit6_u32.as_mut() {
                                    let resp = ui.add(egui::DragValue::new(value));
                                    if resp.changed() {
                                        self.dirty = true;
                                    }
                                }
                            });
                        }
                    });
                });
        });

        ui.add_space(12.0);

        ui.horizontal(|ui| {
            if self.dirty {
                ui.colored_label(Color32::YELLOW, localized::unsaved_changes());
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
            leading_u32: 0,
            presence_mask: 0,
            version: 0x0003_0000,
            bit5_u32: None,
            bit6_u32: None,
        });
        self.dirty = true;
        self.error = None;
    }

    fn save_as_preset(&mut self) {
        let Some(prop) = self.prop.as_ref() else {
            return;
        };

        let layout_label = if prop.layout == PropLayout::Minimal {
            localized::layout_minimal()
        } else {
            localized::layout_extended()
        };
        let preset_name = localized::prop_custom_preset_name(&prop.project, &layout_label);

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
        self.prop = Some(PropSection {
            project: localized::default_project_name().to_string(),
            timestamp: String::new(),
            unk1: 241,
            reserved_u16: 0,
            unk2: 3,
            unk3: 0,
            layout: PropLayout::Extended,
            leading_u32: 0,
            presence_mask: 0,
            version: 0x0003_0000,
            bit5_u32: None,
            bit6_u32: None,
        });
        self.dirty = true;
        self.error = None;
    }

    fn flush_pending(&mut self) {
        if !self.dirty {
            return;
        }
        let Some(path) = self.file_path.as_deref() else {
            self.error = Some(localized::prop_no_file_for_edit().to_string());
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
        if let Some(pending) = prop_pending::get(file_path) {
            return Ok(Some(pending));
        }

        let file = Nus3bankFile::open(file_path).map_err(|e| localized::nus3bank_open_failed(e))?;
        Ok(file.prop)
    }

    fn reload_from_file(&mut self) {
        let Some(path) = self.file_path.as_deref() else {
            self.error = Some(localized::prop_no_file_for_edit().to_string());
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
            Err(e) => self.error = Some(localized::nus3bank_open_failed(e)),
        }
    }
}

fn preset_label(idx: usize, preset: &DebugPreset) -> String {
    match idx {
        0 => localized::prop_preset_1(),
        1 => localized::prop_preset_2(),
        2 => localized::prop_preset_3(),
        _ => preset.name.clone(),
    }
}

pub fn apply_prop_to_file(file: &mut Nus3bankFile, prop: Option<PropSection>) {
    file.prop = prop;
}
