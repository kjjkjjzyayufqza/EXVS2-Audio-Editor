use egui::{Color32, Context, ScrollArea, Ui, Window};

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
        }
    }

    pub fn open_for_file(&mut self, file_path: &str) {
        self.file_path = Some(file_path.to_string());
        self.error = None;

        match self.load_names_for_file(file_path) {
            Ok(names) => self.names = names,
            Err(e) => {
                self.names.clear();
                self.error = Some(e);
            }
        }

        self.open = true;
    }

    pub fn show(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        Window::new("Edit GRP List")
            .min_width(760.0)
            .min_height(520.0)
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                self.render(ui);
            });
    }

    fn render(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("GRP Names Editor");
        });

        if let Some(path) = self.file_path.as_deref() {
            ui.label(format!("File: {}", path));
        } else {
            ui.colored_label(Color32::RED, "No file selected.");
            return;
        }

        if let Some(err) = self.error.as_deref() {
            ui.add_space(6.0);
            ui.colored_label(Color32::RED, err);
        }

        ui.add_space(8.0);
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search_query);

            ui.add_space(12.0);
            ui.label(format!("Total: {}", self.names.len()));
            let visible = self.visible_indices();
            ui.add_space(12.0);
            ui.label(format!("Visible: {}", visible.len()));
        });

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Find:");
            ui.text_edit_singleline(&mut self.find_text);
            ui.label("Replace:");
            ui.text_edit_singleline(&mut self.replace_text);

            if ui.button("Replace in Visible").clicked() {
                self.replace_in_visible();
            }
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Add Row").clicked() {
                self.names.push(String::new());
            }
            if ui.button("Replace with Template").clicked() {
                self.replace_with_template();
            }
            if ui.button("Reload from File").clicked() {
                self.reload_from_file();
            }
            if ui.button("Apply (Pending)").clicked() {
                if let Err(e) = self.apply_pending() {
                    self.error = Some(e);
                } else {
                    self.error = None;
                }
            }
            if ui.button("Close").clicked() {
                self.open = false;
            }
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        let mut remove_index: Option<usize> = None;
        let visible = self.visible_indices();

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for &i in &visible {
                    ui.horizontal(|ui| {
                        ui.label(format!("{:4}", i));
                        ui.add_sized([480.0, 22.0], egui::TextEdit::singleline(&mut self.names[i]));

                        if ui.button("Clear").clicked() {
                            self.names[i].clear();
                        }
                        if ui.button("Remove").clicked() {
                            remove_index = Some(i);
                        }
                    });
                }
            });

        if let Some(idx) = remove_index {
            if idx < self.names.len() {
                self.names.remove(idx);
            }
        }
    }

    fn visible_indices(&self) -> Vec<usize> {
        let q = self.search_query.trim();
        if q.is_empty() {
            return (0..self.names.len()).collect();
        }

        let needle = q.to_lowercase();
        self.names
            .iter()
            .enumerate()
            .filter_map(|(i, s)| {
                if s.to_lowercase().contains(&needle) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }

    fn replace_in_visible(&mut self) {
        let find = self.find_text.clone();
        if find.is_empty() {
            self.error = Some("Find text is empty".to_string());
            return;
        }

        let replace = self.replace_text.clone();
        let visible = self.visible_indices();
        for i in visible {
            let updated = self.names[i].replace(&find, &replace);
            self.names[i] = updated;
        }
        self.error = None;
    }

    fn apply_pending(&mut self) -> Result<(), String> {
        let path = self
            .file_path
            .as_deref()
            .ok_or_else(|| "No file selected for GRP edit".to_string())?;
        grp_pending::set(path, self.names.clone())
    }

    fn load_names_for_file(&self, file_path: &str) -> Result<Vec<String>, String> {
        if let Some(pending) = grp_pending::get(file_path) {
            return Ok(pending);
        }

        let file = Nus3bankFile::open(file_path)
            .map_err(|e| format!("Failed to open NUS3BANK file: {}", e))?;
        Ok(file.grp.map(|g| g.names).unwrap_or_default())
    }

    fn reload_from_file(&mut self) {
        let Some(path) = self.file_path.as_deref() else {
            self.error = Some("No file selected for GRP edit".to_string());
            return;
        };
        self.error = None;
        match Nus3bankFile::open(path) {
            Ok(file) => self.names = file.grp.map(|g| g.names).unwrap_or_default(),
            Err(e) => self.error = Some(format!("Failed to open NUS3BANK file: {}", e)),
        }
    }

    fn replace_with_template(&mut self) {
        let template = grp_template::grp_template_names();
        if template.is_empty() {
            self.error = Some("Template is empty. Please paste the full list into grp_template.rs".to_string());
            return;
        }

        self.names = template;
        self.error = None;
    }
}

pub fn apply_grp_names_to_file(file: &mut Nus3bankFile, names: Vec<String>) {
    if names.is_empty() {
        file.grp = None;
        return;
    }
    file.grp = Some(GrpSection { names });
}

