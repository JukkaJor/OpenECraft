//! Structure management window (IEC 81346): aspects of the pages, templates, the
//! defined and used identifiers with project-wide renaming, and the work areas.

use openecraft_i18n::{tr, tr_fmt};
use super::App;
use crate::io::Purpose;
use crate::structure::{self, Aspect, Dt, WorkArea, TEMPLATES};
use egui::{Color32, RichText};

pub struct StructureUi {
    pub open: bool,
    template: usize,
    filter: String,
    new_aspect: Aspect,
    new_id: String,
    new_description: String,
    /// Identifier being renamed and the new name.
    renaming: Option<(Aspect, String, String)>,
}

impl Default for StructureUi {
    fn default() -> Self {
        StructureUi {
            open: false,
            template: 0,
            filter: String::new(),
            new_aspect: Aspect::Function,
            new_id: String::new(),
            new_description: String::new(),
            renaming: None,
        }
    }
}

enum Action {
    Template,
    LoadTemplate,
    SaveTemplate,
    DefineUsed,
    RemoveUnused,
    Rename(Aspect, String, String),
}

impl App {
    pub(super) fn structure_window(&mut self, ctx: &egui::Context) {
        if !self.structure_ui.open {
            return;
        }
        let mut open = true;
        let mut action = None;
        let rows = structure::overview(&self.prj);
        let st = &mut self.structure_ui;
        let structures = &mut self.prj.structures;
        egui::Window::new(tr("Structure management")).open(&mut open).default_size([760.0, 560.0]).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Template"));
                egui::ComboBox::from_id_salt("structure_template").selected_text(tr(TEMPLATES[st.template].name)).show_ui(ui, |ui| {
                    for (i, t) in TEMPLATES.iter().enumerate() {
                        ui.selectable_value(&mut st.template, i, tr(t.name));
                    }
                });
                if ui.button(tr("Apply template")).on_hover_text(tr("Sets the aspects of the pages and adds the template's identifiers")).clicked() {
                    action = Some(Action::Template);
                }
                ui.separator();
                if ui.button(tr("Load template…")).clicked() {
                    action = Some(Action::LoadTemplate);
                }
                if ui.button(tr("Save as template…")).on_hover_text(tr("Aspects, identifiers and work areas as a file for other projects")).clicked() {
                    action = Some(Action::SaveTemplate);
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Aspects of the pages"));
                let mut aspects = structures.page_aspects();
                let mut changed = false;
                for a in Aspect::ALL {
                    let mut on = aspects.contains(&a);
                    if ui.checkbox(&mut on, format!("{} {}", a.title(), a.prefix())).changed() {
                        changed = true;
                        if on {
                            aspects.push(a);
                        } else {
                            aspects.retain(|x| *x != a);
                        }
                    }
                }
                if changed {
                    aspects.sort();
                    structures.aspects = aspects;
                }
            });
            ui.separator();

            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("Identifiers")).strong());
                ui.add(egui::TextEdit::singleline(&mut st.filter).hint_text(tr("Filter…")).desired_width(120.0));
                if ui.button(tr("Define all used")).on_hover_text(tr("Adds the identifiers used on pages and in device tags")).clicked() {
                    action = Some(Action::DefineUsed);
                }
                if ui.button(tr("Remove unused")).on_hover_text(tr("Removes the definitions no page or device uses")).clicked() {
                    action = Some(Action::RemoveUnused);
                }
            });
            let f = st.filter.to_lowercase();
            egui::ScrollArea::vertical().id_salt("structure_ids").max_height(260.0).auto_shrink([false, true]).show(ui, |ui| {
                egui::Grid::new("structure_grid").num_columns(5).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                    ui.strong(tr("Identifier"));
                    ui.strong(tr("Description"));
                    ui.strong(tr("Pages"));
                    ui.strong(tr("Devices"));
                    ui.label("");
                    ui.end_row();
                    for r in rows.iter().filter(|r| f.is_empty() || r.aspect.format(&r.id).to_lowercase().contains(&f) || r.description.to_lowercase().contains(&f)) {
                        let ident = r.aspect.format(&r.id);
                        match &mut st.renaming {
                            Some((a, old, new)) if *a == r.aspect && *old == r.id => {
                                ui.horizontal(|ui| {
                                    ui.label(r.aspect.prefix());
                                    ui.add(egui::TextEdit::singleline(new).desired_width(90.0));
                                });
                            }
                            _ => {
                                ui.label(RichText::new(&ident).monospace().strong()).on_hover_text(r.aspect.title());
                            }
                        }
                        match structures.entries.iter_mut().find(|e| e.aspect == r.aspect && e.id == r.id) {
                            Some(e) => {
                                ui.add(egui::TextEdit::singleline(&mut e.description).min_size(egui::vec2(220.0, 0.0)));
                            }
                            None => {
                                ui.colored_label(Color32::from_rgb(220, 140, 0), tr("not defined"));
                            }
                        }
                        ui.label(r.pages.to_string());
                        ui.label(r.devices.to_string());
                        ui.horizontal(|ui| {
                            let renaming = st.renaming.as_ref().is_some_and(|(a, old, _)| *a == r.aspect && *old == r.id);
                            if renaming {
                                if ui.button(tr("Rename")).on_hover_text(tr("Renames the identifier on all pages and in all device tags")).clicked() {
                                    if let Some((a, old, new)) = st.renaming.take() {
                                        action = Some(Action::Rename(a, old, new));
                                    }
                                }
                                if ui.small_button("✖").on_hover_text(tr("Cancel")).clicked() {
                                    st.renaming = None;
                                }
                            } else {
                                if ui.small_button("✏").on_hover_text(tr("Rename in the whole project")).clicked() {
                                    st.renaming = Some((r.aspect, r.id.clone(), r.id.clone()));
                                }
                                if !r.defined {
                                    if ui.small_button("➕").on_hover_text(tr("Define")).clicked() {
                                        structures.define(r.aspect, &r.id, "");
                                    }
                                } else if ui.small_button("🗑").on_hover_text(tr("Remove the definition (pages and devices keep the identifier)")).clicked() {
                                    structures.remove(r.aspect, &r.id);
                                }
                            }
                        });
                        ui.end_row();
                    }
                });
            });
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("structure_new_aspect").selected_text(st.new_aspect.title()).show_ui(ui, |ui| {
                    for a in Aspect::ALL {
                        ui.selectable_value(&mut st.new_aspect, a, format!("{} {}", a.prefix(), a.title()));
                    }
                });
                ui.add(egui::TextEdit::singleline(&mut st.new_id).hint_text(tr("Identifier")).desired_width(90.0));
                ui.add(egui::TextEdit::singleline(&mut st.new_description).hint_text(tr("Description")).desired_width(200.0));
                let id = structure::clean_aspect(&st.new_id);
                let free = !id.is_empty() && structures.entry(st.new_aspect, &id).is_none();
                if ui.add_enabled(free, egui::Button::new(tr("➕ Add"))).clicked() {
                    structures.define(st.new_aspect, &id, &st.new_description);
                    st.new_id.clear();
                    st.new_description.clear();
                }
            });
            if !structures.entries.is_empty() {
                ui.weak(tr("Identifiers in use that are not defined are reported by the check."));
            }
            ui.separator();

            ui.label(RichText::new(tr("Work areas")).strong());
            ui.weak(tr("A work area limits the pages and devices in the navigator to a part of the project. Filters: empty = any, * = any characters; A1 also covers A1.2."));
            let mut delete = None;
            egui::Grid::new("work_areas").num_columns(6).striped(true).show(ui, |ui| {
                ui.strong(tr("Name"));
                for a in Aspect::ALL {
                    ui.strong(format!("{} {}", a.title(), a.prefix()));
                }
                ui.label("");
                ui.end_row();
                for (i, w) in structures.work_areas.iter_mut().enumerate() {
                    ui.add(egui::TextEdit::singleline(&mut w.name).desired_width(120.0));
                    for a in Aspect::ALL {
                        ui.add(egui::TextEdit::singleline(w.filter.get_mut(a)).desired_width(80.0));
                    }
                    if ui.small_button("🗑").on_hover_text(tr("Delete")).clicked() {
                        delete = Some(i);
                    }
                    ui.end_row();
                }
            });
            if let Some(i) = delete {
                structures.work_areas.remove(i);
            }
            if ui.button(tr("➕ Work area")).clicked() {
                let n = (1..).map(|n| tr_fmt!("Work area {}", n)).find(|n| structures.work_area(n).is_none()).unwrap_or_default();
                structures.work_areas.push(WorkArea { name: n, filter: Dt::default() });
            }
        });
        self.structure_ui.open = open;
        match action {
            Some(Action::Template) => {
                let n = structure::apply_template(&mut self.prj.structures, &TEMPLATES[self.structure_ui.template]);
                self.notify(ctx, tr_fmt!("{} structure identifiers added", n));
            }
            Some(Action::LoadTemplate) => self.io.open(ctx, Purpose::StructureTemplate),
            Some(Action::SaveTemplate) => {
                let json = self.prj.structures.to_json();
                self.export(ctx, "structure.json".into(), (tr("OpenECraft structure template"), "json"), json);
            }
            Some(Action::DefineUsed) => {
                let n = structure::define_used(&mut self.prj);
                self.notify(ctx, tr_fmt!("{} structure identifiers added", n));
            }
            Some(Action::RemoveUnused) => {
                let n = structure::remove_unused(&mut self.prj);
                self.notify(ctx, tr_fmt!("{} unused structure identifiers removed", n));
            }
            Some(Action::Rename(a, old, new)) => match structure::rename(&mut self.prj, a, &old, &new) {
                Ok(n) => self.notify(ctx, tr_fmt!("{} renamed: {} pages and devices", a.format(&old), n)),
                Err(e) => self.notify(ctx, e),
            },
            None => {}
        }
    }
}
