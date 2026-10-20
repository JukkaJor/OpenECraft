//! Parts management window and part selection dialog. The part widgets are shared
//! with the parts manager (`openecraft_ui::part_editor`).

use crate::model::Project;
use crate::parts::{self, Part, PartRef, PartsDb};
use crate::io::{Loaded, Purpose};
use egui::{vec2, Color32, RichText};
use openecraft_ui::client::{Reply, ServerClient};
use openecraft_ui::part_editor::{self as pe, EditorState, ModelKind, PartTab};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Database,
    Project,
}

/// Requests the application has to carry out (file dialogs, messages).
pub enum PartsAction {
    Import,
    /// Pick a 2D / 3D model file for the selected part.
    ImportModel(ModelKind),
    Export { name: String, filter: (&'static str, &'static str), content: String },
    Message(String),
}

struct SelectRequest {
    tag: String,
}

pub struct PartsUi {
    pub open: bool,
    source: Source,
    filter: String,
    selected: Option<String>,
    editor: EditorState,
    select: Option<SelectRequest>,
    sel_filter: String,
    sel_matching: bool,
    sel_part: Option<String>,
    /// Connection to the global parts server.
    pub server: ServerClient,
    show_server: bool,
    /// Part (source, number) that receives the next imported model.
    model_target: Option<(Source, String)>,
}

impl Default for PartsUi {
    fn default() -> Self {
        PartsUi {
            open: false,
            source: Source::Database,
            filter: String::new(),
            selected: None,
            editor: EditorState::default(),
            select: None,
            sel_filter: String::new(),
            sel_matching: true,
            sel_part: None,
            server: ServerClient::default(),
            show_server: false,
            model_target: None,
        }
    }
}

impl PartsUi {
    /// Puts an imported 2D / 3D model into the part it was requested for.
    pub fn apply_model(&mut self, loaded: &Loaded, db: &mut PartsDb, prj: &mut Project) -> Result<String, String> {
        let Purpose::Model(kind) = loaded.purpose else { return Err("not a model file".into()) };
        let (source, number) = self.model_target.take().ok_or("no part selected for the model")?;
        let list = match source {
            Source::Database => &mut db.parts,
            Source::Project => &mut prj.parts,
        };
        let part = list.iter_mut().find(|p| p.number == number).ok_or(format!("part {number} not found"))?;
        pe::apply_model(part, kind, &loaded.name, &loaded.bytes)
    }

    /// Applies finished parts server requests (call every frame). Downloaded parts
    /// are merged into the database.
    pub fn poll_server(&mut self, db: &mut PartsDb) -> Option<String> {
        let mut message = None;
        for reply in self.server.poll() {
            if let Reply::Parts(Ok(list)) = reply {
                let n = list.parts.len();
                let (added, updated) = db.merge(list.parts);
                self.server.status = format!("Downloaded {n} parts (revision {}): {added} new, {updated} updated", list.revision);
            }
            message = Some(self.server.status.clone());
        }
        message
    }

    /// Server URL, token and transfer buttons.
    fn server_panel(&mut self, ui: &mut egui::Ui, db: &PartsDb, selected: Option<Part>) {
        let ctx = ui.ctx().clone();
        egui::Grid::new("server").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label("Server URL");
            ui.add(egui::TextEdit::singleline(&mut self.server.url).desired_width(320.0));
            ui.end_row();
            ui.label("Access token");
            ui.add(egui::TextEdit::singleline(&mut self.server.token).password(true).desired_width(320.0).hint_text("only needed for uploads"));
            ui.end_row();
        });
        let busy = self.server.busy();
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(!busy, egui::Button::new("Test connection")).clicked() {
                self.server.check(&ctx);
            }
            if ui
                .add_enabled(!busy, egui::Button::new("⬇ Download into database"))
                .on_hover_text("Adds new parts and updates existing ones in the local parts database")
                .clicked()
            {
                self.server.download(&ctx);
            }
            let upload_one = ui.add_enabled(!busy && selected.is_some(), egui::Button::new("⬆ Upload selected part"));
            if upload_one.clicked() {
                if let Some(p) = &selected {
                    self.server.upload(&ctx, std::slice::from_ref(p));
                }
            }
            if ui
                .add_enabled(!busy && !db.parts.is_empty(), egui::Button::new(format!("⬆ Upload database ({})", db.parts.len())))
                .on_hover_text("Adds / replaces all parts of the local database on the server")
                .clicked()
            {
                self.server.upload(&ctx, &db.parts);
            }
            if busy {
                ui.spinner();
            }
        });
        if !self.server.status.is_empty() {
            let ok = !self.server.failed();
            let color = if ok { ui.visuals().text_color() } else { Color32::from_rgb(220, 60, 60) };
            ui.label(RichText::new(&self.server.status).color(color));
        }
    }

    /// Opens the management window with a part selected.
    pub fn show_part(&mut self, number: &str, in_project: bool) {
        self.open = true;
        self.source = if in_project { Source::Project } else { Source::Database };
        self.selected = Some(number.to_string());
    }

    /// Opens the part selection dialog for a device.
    pub fn select_for(&mut self, tag: &str) {
        self.select = Some(SelectRequest { tag: tag.to_string() });
        self.sel_part = None;
    }

    pub fn manager(&mut self, ctx: &egui::Context, db: &mut PartsDb, prj: &mut Project) -> Vec<PartsAction> {
        let mut actions = Vec::new();
        if !self.open {
            return actions;
        }
        let mut open = true;
        egui::Window::new("Parts management").open(&mut open).default_size([1000.0, 620.0]).show(ctx, |ui| {
            // ---------------------------------------------------------- toolbar
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.source, Source::Database, "🗄 Parts database");
                ui.selectable_value(&mut self.source, Source::Project, format!("📁 Project parts ({})", prj.parts.len()));
                ui.separator();
                let list: &mut Vec<Part> = match self.source {
                    Source::Database => &mut db.parts,
                    Source::Project => &mut prj.parts,
                };
                if ui.button("➕ New").clicked() {
                    let number = parts::unique_number(list, "NEW.PART");
                    list.push(Part { number: number.clone(), manufacturer: "NEW".into(), ..Default::default() });
                    self.selected = Some(number);
                    self.editor.tab = PartTab::General;
                }
                let sel_idx = self.selected.as_ref().and_then(|n| list.iter().position(|p| &p.number == n));
                if ui.add_enabled(sel_idx.is_some(), egui::Button::new("⎘ Copy")).clicked() {
                    let mut p = list[sel_idx.unwrap()].clone();
                    p.number = parts::unique_number(list, &format!("{}_COPY", p.number));
                    self.selected = Some(p.number.clone());
                    list.push(p);
                }
                if ui.add_enabled(sel_idx.is_some(), egui::Button::new("🗑 Delete")).clicked() {
                    list.remove(sel_idx.unwrap());
                    self.selected = None;
                }
                ui.separator();
                ui.menu_button("Extras", |ui| {
                    if ui.button("Import parts (CSV / JSON)…").clicked() {
                        actions.push(PartsAction::Import);
                    }
                    let src = match self.source {
                        Source::Database => db.parts.clone(),
                        Source::Project => prj.parts.clone(),
                    };
                    let base = if self.source == Source::Database { "parts_database" } else { "project_parts" };
                    if ui.button("Export as CSV…").clicked() {
                        actions.push(PartsAction::Export { name: format!("{base}.csv"), filter: ("CSV", "csv"), content: parts::to_csv(&src) });
                    }
                    if ui.button("Export as JSON…").clicked() {
                        let json = serde_json::to_string_pretty(&PartsDb { parts: src }).unwrap_or_default();
                        actions.push(PartsAction::Export { name: format!("{base}.json"), filter: ("JSON", "json"), content: json });
                    }
                    ui.separator();
                    if ui.button("Synchronize project parts from database").on_hover_text("Updates the project parts with the current database data").clicked() {
                        let n = parts::sync_project_from_db(prj, db);
                        actions.push(PartsAction::Message(format!("{n} project parts updated")));
                    }
                    if ui.button("Transfer project parts to database").on_hover_text("Adds parts that only exist in the project to the database").clicked() {
                        let n = parts::transfer_project_to_db(prj, db);
                        actions.push(PartsAction::Message(format!("{n} parts added to the database")));
                    }
                    if ui.button("Remove unused project parts").clicked() {
                        let n = parts::purge_unused(prj);
                        actions.push(PartsAction::Message(format!("{n} unused project parts removed")));
                    }
                    ui.separator();
                    if ui
                        .button("Refresh example catalog parts (GEN.*)")
                        .on_hover_text("Replaces the GEN.* parts in the database with the current example catalog (including models)")
                        .clicked()
                    {
                        let catalog = parts::default_catalog();
                        let n = catalog.len();
                        for p in catalog {
                            db.upsert(p);
                        }
                        db.sort();
                        actions.push(PartsAction::Message(format!("{n} example catalog parts refreshed")));
                    }
                    if ui.button("Add missing parts of the example catalog").clicked() {
                        let mut n = 0;
                        for p in parts::default_catalog() {
                            if db.get(&p.number).is_none() {
                                db.parts.push(p);
                                n += 1;
                            }
                        }
                        actions.push(PartsAction::Message(format!("{n} catalog parts added")));
                    }
                });
                ui.separator();
                ui.toggle_value(&mut self.show_server, "🌐 Parts server")
                    .on_hover_text("Exchange parts with a global parts database (openecraft-server)");
            });
            ui.separator();
            if self.show_server {
                let selected = self.selected.as_ref().and_then(|n| match self.source {
                    Source::Database => db.get(n).cloned(),
                    Source::Project => prj.parts.iter().find(|p| &p.number == n).cloned(),
                });
                self.server_panel(ui, db, selected);
                ui.separator();
            }

            // ---------------------------------------------------------- body
            let numbers: Vec<String> = db.parts.iter().chain(prj.parts.iter()).map(|p| p.number.clone()).collect();
            let list: &mut Vec<Part> = match self.source {
                Source::Database => &mut db.parts,
                Source::Project => &mut prj.parts,
            };
            let h = ui.available_height();
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(vec2(300.0, h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Filter (number, type, manufacturer, text …)"));
                    ui.label(RichText::new(format!("{} parts", list.len())).weak());
                    egui::ScrollArea::vertical().id_salt("ptree").auto_shrink([false, false]).show(ui, |ui| {
                        let refs: Vec<&Part> = list.iter().collect();
                        pe::part_tree(ui, "mgr", &refs, &self.filter, &mut self.selected);
                    });
                });
                ui.separator();
                ui.vertical(|ui| {
                    let idx = self.selected.as_ref().and_then(|n| list.iter().position(|p| &p.number == n));
                    match idx {
                        Some(i) => {
                            let duplicate = list.iter().filter(|p| p.number == list[i].number).count() > 1;
                            let before = list[i].number.clone();
                            if let Some(kind) = pe::part_editor(ui, &mut list[i], &mut self.editor, &numbers, duplicate) {
                                self.model_target = Some((self.source, list[i].number.clone()));
                                actions.push(PartsAction::ImportModel(kind));
                            }
                            if list[i].number != before {
                                self.selected = Some(list[i].number.clone());
                            }
                        }
                        None => {
                            ui.weak("Select a part in the tree or create a new one.");
                            ui.add_space(8.0);
                            match self.source {
                                Source::Database => ui.label("The parts database is shared by all projects. Parts are copied into a project when they are assigned to a device."),
                                Source::Project => ui.label("Project parts are the copies stored in this project; they are used for the bill of materials and the project check."),
                            };
                        }
                    }
                });
            });
        });
        self.open = open;
        actions
    }

    /// Part selection dialog for a device. Adds the chosen part to the device.
    pub fn selector(&mut self, ctx: &egui::Context, db: &PartsDb, prj: &mut Project) {
        let Some(req) = &self.select else { return };
        let tag = req.tag.clone();
        let mut open = true;
        let mut chosen: Option<String> = None;
        // Database parts plus parts that exist only in the project.
        let mut all: Vec<Part> = db.parts.clone();
        for p in &prj.parts {
            if db.get(&p.number).is_none() {
                all.push(p.clone());
            }
        }
        let placed = parts::placed_functions(prj, &tag);
        let main = parts::main_kind(prj, &tag);
        egui::Window::new(format!("Part selection – {tag}")).open(&mut open).default_size([760.0, 480.0]).collapsible(false).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.sel_filter).hint_text("Filter…").desired_width(220.0));
                ui.checkbox(&mut self.sel_matching, "Device selection")
                    .on_hover_text("Only parts whose function template provides the device's main function");
            });
            ui.separator();
            let shown: Vec<&Part> = all
                .iter()
                .filter(|p| !self.sel_matching || placed.is_empty() || parts::fits_device(p, &placed, main))
                .collect();
            let h = ui.available_height() - 36.0;
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(vec2(300.0, h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    egui::ScrollArea::vertical().id_salt("seltree").auto_shrink([false, false]).show(ui, |ui| {
                        if let Some(n) = pe::part_tree(ui, "sel", &shown, &self.sel_filter, &mut self.sel_part) {
                            chosen = Some(n);
                        }
                    });
                });
                ui.separator();
                ui.vertical(|ui| {
                    match self.sel_part.as_ref().and_then(|n| all.iter().find(|p| &p.number == n)) {
                        Some(p) => pe::part_summary(ui, p),
                        None => {
                            ui.weak("Select a part. Double click adds it to the device.");
                        }
                    }
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.add_enabled(self.sel_part.is_some(), egui::Button::new("✔ Add to device")).clicked() {
                    chosen = self.sel_part.clone();
                }
                if ui.button("Cancel").clicked() {
                    self.select = None;
                }
            });
        });
        if let Some(number) = chosen {
            if db.get(&number).is_some() {
                parts::ensure_in_project(prj, db, &number);
            }
            let dev = prj.devices.entry(tag).or_default();
            match dev.parts.iter_mut().find(|r| r.number == number) {
                Some(r) => r.count += 1,
                None => dev.parts.push(PartRef { number, count: 1 }),
            }
            self.select = None;
        }
        if !open {
            self.select = None;
        }
    }
}

#[cfg(test)]
mod tests;
