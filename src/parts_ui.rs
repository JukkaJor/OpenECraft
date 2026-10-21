//! Parts management window and part selection dialog. The part widgets are shared
//! with the parts manager (`openecraft_ui::part_editor`).

use openecraft_i18n::{tr, tr_fmt};
use crate::model::Project;
use crate::parts::{self, Part, PartFilter, PartsDb};
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
    filter: PartFilter,
    selected: Option<String>,
    editor: EditorState,
    select: Option<SelectRequest>,
    sel_filter: PartFilter,
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
            filter: PartFilter::default(),
            selected: None,
            editor: EditorState::default(),
            select: None,
            sel_filter: PartFilter::default(),
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
        let part = list.iter_mut().find(|p| p.number == number).ok_or(tr_fmt!("part {} not found", number))?;
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
                self.server.status = tr_fmt!("Downloaded {} parts (revision {}): {} new, {} updated", n, list.revision, added, updated);
            }
            message = Some(self.server.status.clone());
        }
        message
    }

    /// Server URL, token and transfer buttons.
    fn server_panel(&mut self, ui: &mut egui::Ui, db: &PartsDb, selected: Option<Part>) {
        let ctx = ui.ctx().clone();
        egui::Grid::new("server").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label(tr("Server URL"));
            ui.add(egui::TextEdit::singleline(&mut self.server.url).desired_width(320.0));
            ui.end_row();
            ui.label(tr("Access token"));
            ui.add(egui::TextEdit::singleline(&mut self.server.token).password(true).desired_width(320.0).hint_text(tr("only needed for uploads")));
            ui.end_row();
        });
        let busy = self.server.busy();
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(!busy, egui::Button::new(tr("Test connection"))).clicked() {
                self.server.check(&ctx);
            }
            if ui
                .add_enabled(!busy, egui::Button::new(tr("⬇ Download into database")))
                .on_hover_text(tr("Adds new parts and updates existing ones in the local parts database"))
                .clicked()
            {
                self.server.download(&ctx);
            }
            let upload_one = ui.add_enabled(!busy && selected.is_some(), egui::Button::new(tr("⬆ Upload selected part")));
            if upload_one.clicked() {
                if let Some(p) = &selected {
                    self.server.upload(&ctx, std::slice::from_ref(p));
                }
            }
            if ui
                .add_enabled(!busy && !db.parts.is_empty(), egui::Button::new(tr_fmt!("⬆ Upload database ({})", db.parts.len())))
                .on_hover_text(tr("Adds / replaces all parts of the local database on the server"))
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
        egui::Window::new(tr("Parts management")).open(&mut open).default_size([1000.0, 620.0]).show(ctx, |ui| {
            // ---------------------------------------------------------- toolbar
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.source, Source::Database, tr("🗄 Parts database"));
                ui.selectable_value(&mut self.source, Source::Project, tr_fmt!("📁 Project parts ({})", prj.parts.len()));
                ui.separator();
                let list: &mut Vec<Part> = match self.source {
                    Source::Database => &mut db.parts,
                    Source::Project => &mut prj.parts,
                };
                if ui.button(tr("➕ New")).clicked() {
                    let number = parts::unique_number(list, "NEW.PART");
                    list.push(Part { number: number.clone(), manufacturer: "NEW".into(), ..Default::default() });
                    self.selected = Some(number);
                    self.editor.tab = PartTab::General;
                }
                let sel_idx = self.selected.as_ref().and_then(|n| list.iter().position(|p| &p.number == n));
                if ui.add_enabled(sel_idx.is_some(), egui::Button::new(tr("⎘ Copy"))).clicked() {
                    let mut p = list[sel_idx.unwrap()].clone();
                    p.number = parts::unique_number(list, &format!("{}_COPY", p.number));
                    self.selected = Some(p.number.clone());
                    list.push(p);
                }
                if ui.add_enabled(sel_idx.is_some(), egui::Button::new(tr("🗑 Delete"))).clicked() {
                    list.remove(sel_idx.unwrap());
                    self.selected = None;
                }
                ui.separator();
                ui.menu_button(tr("Extras"), |ui| {
                    if ui.button(tr("Import parts (CSV / JSON)…")).clicked() {
                        actions.push(PartsAction::Import);
                    }
                    let src = match self.source {
                        Source::Database => db.parts.clone(),
                        Source::Project => prj.parts.clone(),
                    };
                    let base = if self.source == Source::Database { "parts_database" } else { "project_parts" };
                    if ui.button(tr("Export as CSV…")).clicked() {
                        actions.push(PartsAction::Export { name: format!("{base}.csv"), filter: ("CSV", "csv"), content: parts::to_csv(&src) });
                    }
                    if ui.button(tr("Export as JSON…")).clicked() {
                        let json = serde_json::to_string_pretty(&PartsDb { parts: src }).unwrap_or_default();
                        actions.push(PartsAction::Export { name: format!("{base}.json"), filter: ("JSON", "json"), content: json });
                    }
                    ui.separator();
                    if ui.button(tr("Synchronize project parts from database")).on_hover_text(tr("Updates the project parts with the current database data")).clicked() {
                        let n = parts::sync_project_from_db(prj, db);
                        actions.push(PartsAction::Message(tr_fmt!("{} project parts updated", n)));
                    }
                    if ui.button(tr("Transfer project parts to database")).on_hover_text(tr("Adds parts that only exist in the project to the database")).clicked() {
                        let n = parts::transfer_project_to_db(prj, db);
                        actions.push(PartsAction::Message(tr_fmt!("{} parts added to the database", n)));
                    }
                    if ui.button(tr("Remove unused project parts")).clicked() {
                        let n = parts::purge_unused(prj);
                        actions.push(PartsAction::Message(tr_fmt!("{} unused project parts removed", n)));
                    }
                    ui.separator();
                    if ui
                        .button(tr("Refresh example catalog parts (GEN.*)"))
                        .on_hover_text(tr("Replaces the GEN.* parts in the database with the current example catalog (including models)"))
                        .clicked()
                    {
                        let catalog = parts::default_catalog();
                        let n = catalog.len();
                        for p in catalog {
                            db.upsert(p);
                        }
                        db.sort();
                        actions.push(PartsAction::Message(tr_fmt!("{} example catalog parts refreshed", n)));
                    }
                    if ui.button(tr("Add missing parts of the example catalog")).clicked() {
                        let mut n = 0;
                        for p in parts::default_catalog() {
                            if db.get(&p.number).is_none() {
                                db.parts.push(p);
                                n += 1;
                            }
                        }
                        actions.push(PartsAction::Message(tr_fmt!("{} catalog parts added", n)));
                    }
                });
                ui.separator();
                ui.toggle_value(&mut self.show_server, tr("🌐 Parts server"))
                    .on_hover_text(tr("Exchange parts with a global parts database (openecraft-server)"));
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
                    let refs: Vec<&Part> = list.iter().collect();
                    pe::part_filter(ui, "mgr", &refs, &mut self.filter);
                    egui::ScrollArea::vertical().id_salt("ptree").auto_shrink([false, false]).show(ui, |ui| {
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
                            ui.weak(tr("Select a part in the tree or create a new one."));
                            ui.add_space(8.0);
                            match self.source {
                                Source::Database => ui.label(tr("The parts database is shared by all projects. Parts are copied into a project when they are assigned to a device.")),
                                Source::Project => ui.label(tr("Project parts are the copies stored in this project; they are used for the bill of materials and the project check.")),
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
        egui::Window::new(tr_fmt!("Part selection – {}", tag)).open(&mut open).default_size([760.0, 480.0]).collapsible(false).show(ctx, |ui| {
            ui.checkbox(&mut self.sel_matching, tr("Device selection"))
                .on_hover_text(tr("Only parts whose function template provides the device's main function"));
            ui.separator();
            let shown: Vec<&Part> = all
                .iter()
                .filter(|p| !self.sel_matching || placed.is_empty() || parts::fits_device(p, &placed, main))
                .collect();
            let h = ui.available_height() - 36.0;
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(vec2(300.0, h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    pe::part_filter(ui, "sel", &shown, &mut self.sel_filter);
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
                            ui.weak(tr("Select a part. Double click adds it to the device."));
                        }
                    }
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.add_enabled(self.sel_part.is_some(), egui::Button::new(tr("✔ Add to device"))).clicked() {
                    chosen = self.sel_part.clone();
                }
                if ui.button(tr("Cancel")).clicked() {
                    self.select = None;
                }
            });
        });
        if let Some(number) = chosen {
            parts::add_to_device(prj, db, &tag, &number, 1);
            self.select = None;
        }
        if !open {
            self.select = None;
        }
    }
}

#[cfg(test)]
mod tests;
