use egui::{Color32, Context, ScrollArea, Ui, Window};

use crate::nus3bank::structures::{Nus3bankFile, PropLayout, PropSection};

use super::prop_pending;

pub struct PropEditModal {
    pub open: bool,
    file_path: Option<String>,
    prop: Option<PropSection>,
    error: Option<String>,
    dirty: bool,
    debug_mode: bool,
    
    // Debug preset values
    debug_presets: Vec<DebugPreset>,
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
        let debug_presets = vec![
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
            debug_mode: false,
            debug_presets,
            selected_preset: None,
        }
    }

    pub fn open_for_file(&mut self, file_path: &str) {
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
        let mut open = self.open;
        let was_open = open;
        let available_rect = ctx.available_rect();
        let min_width = available_rect.width() * 0.7;
        let min_height = available_rect.height() * 0.7;

        Window::new("Edit PROP Section")
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
            ui.heading("PROP Section Editor");
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
            if ui.button("Reload from File").clicked() {
                self.reload_from_file();
            }
            ui.checkbox(&mut self.debug_mode, "Debug Mode");
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        if self.prop.is_none() {
            ui.colored_label(Color32::YELLOW, "No PROP section in this file.");
            ui.add_space(8.0);
            if ui.button("Create New PROP Section").clicked() {
                self.create_default_prop();
            }
            return;
        }

        // Debug preset selector
        if self.debug_mode {
            self.render_debug_presets(ui);
        }

        // Main editor
        self.render_prop_editor(ui);
    }

    fn render_debug_presets(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.heading("Debug Presets");
            ui.add_space(6.0);

            let preset_list_height = ui.available_height() * 0.3;
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(preset_list_height)
                .show(ui, |ui| {
                    for (idx, preset) in self.debug_presets.iter().enumerate() {
                        let selected = self.selected_preset == Some(idx);
                        if ui.selectable_label(selected, &preset.name).clicked() {
                            self.selected_preset = Some(idx);
                        }
                    }
                });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Apply Selected Preset").clicked() {
                    self.apply_preset();
                }
                if ui.button("Save Current as Preset").clicked() {
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
                        ui.heading("Basic Fields");
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label("Project:");
                            let resp = ui.text_edit_singleline(&mut prop.project);
                            if resp.changed() {
                                self.dirty = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Timestamp:");
                            let resp = ui.text_edit_singleline(&mut prop.timestamp);
                            if resp.changed() {
                                self.dirty = true;
                            }
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.heading("Layout");
                        ui.horizontal(|ui| {
                            ui.label("Layout Type:");
                            let layout_minimal = prop.layout == PropLayout::Minimal;
                            if ui.radio(layout_minimal, "Minimal").clicked() {
                                prop.layout = PropLayout::Minimal;
                                self.dirty = true;
                            }
                            if ui.radio(!layout_minimal, "Extended").clicked() {
                                prop.layout = PropLayout::Extended;
                                self.dirty = true;
                            }
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.heading("Advanced Fields");
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label("unk1 (i32):");
                            let resp = ui.add(egui::DragValue::new(&mut prop.unk1));
                            if resp.changed() {
                                self.dirty = true;
                            }
                            if ui.button("?").clicked() {
                                // Info tooltip
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("reserved_u16 (u16):");
                            let mut temp_i32 = prop.reserved_u16 as i32;
                            let resp = ui.add(egui::DragValue::new(&mut temp_i32).range(0..=65535));
                            if resp.changed() {
                                prop.reserved_u16 = temp_i32 as u16;
                                self.dirty = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("unk2 (u16):");
                            let mut temp_i32 = prop.unk2 as i32;
                            let resp = ui.add(egui::DragValue::new(&mut temp_i32).range(0..=65535));
                            if resp.changed() {
                                prop.unk2 = temp_i32 as u16;
                                self.dirty = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("unk3 (u16):");
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
                ui.colored_label(Color32::YELLOW, "Unsaved changes");
            }
        });
    }

    fn apply_preset(&mut self) {
        let Some(idx) = self.selected_preset else {
            return;
        };
        if idx >= self.debug_presets.len() {
            return;
        }

        let preset = &self.debug_presets[idx];
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

    fn save_as_preset(&mut self) {
        let Some(prop) = self.prop.as_ref() else {
            return;
        };

        let preset_name = format!(
            "Custom: {} ({})",
            prop.project,
            match prop.layout {
                PropLayout::Minimal => "Minimal",
                PropLayout::Extended => "Extended",
            }
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

        self.debug_presets.push(new_preset);
        self.selected_preset = Some(self.debug_presets.len() - 1);
    }

    fn create_default_prop(&mut self) {
        self.prop = Some(PropSection {
            project: "DefaultProject".to_string(),
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
        if !self.dirty {
            return;
        }
        let Some(path) = self.file_path.as_deref() else {
            self.error = Some("No file selected for PROP edit".to_string());
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

        let file = Nus3bankFile::open(file_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;
        Ok(file.prop)
    }

    fn reload_from_file(&mut self) {
        let Some(path) = self.file_path.as_deref() else {
            self.error = Some("No file selected for PROP edit".to_string());
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
            Err(e) => self.error = Some(format!("Failed to open NUS3BANK file: {}", e)),
        }
    }
}

pub fn apply_prop_to_file(file: &mut Nus3bankFile, prop: Option<PropSection>) {
    file.prop = prop;
}
