//! Main application window: menus, toolbar, navigators, symbol library, property
//! editor, reports and dialogs.

use crate::editor::{symbol_icon, Editor, Tool};
use crate::index::Index;
use crate::io::{self, FileIo, Purpose, PROJECT_EXT};
use crate::parts::{self, PartsDb};
use crate::parts_ui::{PartsAction, PartsUi};
use crate::model::*;
use crate::netlist::{pin_designation, PinRef};
use crate::ops;
use crate::reports::{Analysis, Severity};
use crate::symbols::{self, Category, Func};
use crate::{history::History, sample, svg};
use egui::{Color32, Key, KeyboardShortcut, Modifiers, RichText};

#[derive(Clone, Copy, PartialEq, Eq)]
enum LeftTab {
    Pages,
    Devices,
    Check,
}

pub struct App {
    prj: Project,
    page: usize,
    editor: Editor,
    history: History,
    analysis: Analysis,
    analysis_rev: u64,
    saved_rev: u64,
    file_name: Option<String>,
    io: FileIo,
    left_tab: LeftTab,
    lib_filter: String,
    device_filter: String,
    show_reports: bool,
    report_kind: ReportKind,
    show_project: bool,
    parts_db: PartsDb,
    parts_ui: PartsUi,
    show_help: bool,
    page_props: Option<usize>,
    message: Option<(String, f64)>,
    title: String,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> App {
        let mut prj: Option<Project> = None;
        let mut file_name = None;
        let mut parts_db: Option<PartsDb> = None;
        if let Some(storage) = cc.storage {
            prj = eframe::get_value(storage, "project");
            file_name = eframe::get_value(storage, "file_name").flatten();
            parts_db = eframe::get_value(storage, "parts_db");
        }
        let parts_db = parts_db.unwrap_or_default();
        let mut prj = prj.unwrap_or_else(sample::demo_project);
        prj.normalize();
        let history = History::new(&prj);
        let rev = history.rev;
        App {
            prj,
            page: 0,
            editor: Editor::default(),
            history,
            analysis: Analysis::default(),
            analysis_rev: 0,
            saved_rev: rev,
            file_name,
            io: FileIo::default(),
            left_tab: LeftTab::Pages,
            lib_filter: String::new(),
            device_filter: String::new(),
            show_reports: false,
            report_kind: ReportKind::DeviceList,
            show_project: false,
            parts_db,
            parts_ui: PartsUi::default(),
            show_help: false,
            page_props: None,
            message: None,
            title: String::new(),
        }
        .start_on_first_schematic()
    }

    fn start_on_first_schematic(mut self) -> Self {
        self.page = self.prj.pages.iter().position(|p| p.kind == PageKind::Schematic).unwrap_or(0);
        self
    }

    fn dirty(&self) -> bool {
        self.history.rev != self.saved_rev
    }

    fn notify(&mut self, ctx: &egui::Context, msg: impl Into<String>) {
        self.message = Some((msg.into(), ctx.input(|i| i.time)));
    }

    fn replace_project(&mut self, prj: Project, file_name: Option<String>) {
        self.prj = prj;
        self.prj.normalize();
        self.file_name = file_name;
        self.page = self.prj.pages.iter().position(|p| p.kind == PageKind::Schematic).unwrap_or(0);
        self.editor.selection.clear();
        self.editor.cancel();
        self.editor.fit_pending = true;
        // The replacement becomes one undo step; mark it as saved.
        self.history.track(&self.prj, true);
        self.saved_rev = self.history.rev;
    }

    fn goto(&mut self, page: usize, id: u64) {
        if page >= self.prj.pages.len() {
            return;
        }
        self.page = page;
        self.editor.cancel();
        self.editor.selection.clear();
        if id != 0 {
            self.editor.selection.insert(id);
            if let Some(b) = ops::item_bounds(&self.prj.pages[page], id) {
                if self.editor.zoom < 3.0 {
                    self.editor.zoom = 4.0;
                }
                self.editor.center_on = Some(b.center());
            }
        }
    }

    // ------------------------------------------------------------------ files

    fn project_json(&self) -> String {
        serde_json::to_string_pretty(&self.prj).unwrap_or_default()
    }

    fn suggested_name(&self, ext: &str) -> String {
        let base: String = self
            .prj
            .info
            .name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        format!("{}.{ext}", if base.is_empty() { "project".into() } else { base })
    }

    fn save(&mut self, ctx: &egui::Context, save_as: bool) {
        let json = self.project_json();
        let result = match (&self.file_name, save_as || !FileIo::can_save_in_place()) {
            (Some(path), false) => self.io.save_to(path, &json).map(|_| Some(path.clone())),
            _ => {
                let name = self.file_name.as_deref().map(io::short_name).map(String::from);
                let name = name.unwrap_or_else(|| self.suggested_name(PROJECT_EXT));
                self.io.save_as(&name, ("OpenECraft project", PROJECT_EXT), &json)
            }
        };
        match result {
            Ok(Some(path)) => {
                self.notify(ctx, format!("Saved {}", io::short_name(&path)));
                self.file_name = Some(path);
                self.saved_rev = self.history.rev;
            }
            Ok(None) => {}
            Err(e) => self.notify(ctx, format!("Saving failed: {e}")),
        }
    }

    fn export(&mut self, ctx: &egui::Context, name: String, filter: (&str, &str), content: String) {
        match self.io.save_as(&name, filter, &content) {
            Ok(Some(path)) => self.notify(ctx, format!("Exported {}", io::short_name(&path))),
            Ok(None) => {}
            Err(e) => self.notify(ctx, format!("Export failed: {e}")),
        }
    }

    fn poll_loaded(&mut self, ctx: &egui::Context) {
        if let Some(loaded) = self.io.poll() {
            if loaded.purpose == Purpose::Parts {
                match parts::import(&loaded.content) {
                    Ok(list) => {
                        let n = list.len();
                        for p in list {
                            self.parts_db.upsert(p);
                        }
                        self.parts_db.sort();
                        self.notify(ctx, format!("Imported {n} parts into the database"));
                    }
                    Err(e) => self.notify(ctx, format!("Parts import failed: {e}")),
                }
                return;
            }
            match serde_json::from_str::<Project>(&loaded.content) {
                Ok(prj) => {
                    self.replace_project(prj, Some(loaded.name.clone()));
                    self.notify(ctx, format!("Opened {}", io::short_name(&loaded.name)));
                }
                Err(e) => self.notify(ctx, format!("Not a valid project file: {e}")),
            }
        }
    }

    // ------------------------------------------------------------------ pages

    fn next_page_name(&self) -> String {
        let max = self.prj.pages.iter().filter_map(|p| p.name.parse::<u32>().ok()).max().unwrap_or(0);
        (max + 1).to_string()
    }

    fn add_page(&mut self, kind: PageKind) {
        let id = self.prj.new_id();
        let desc = match kind {
            PageKind::Schematic => "Schematic".to_string(),
            PageKind::Cover => "Cover sheet".to_string(),
            PageKind::Report(r) => r.title().to_string(),
        };
        let page = Page::new(id, &self.next_page_name(), &desc, kind);
        let at = (self.page + 1).min(self.prj.pages.len());
        self.prj.pages.insert(at, page);
        self.page = at;
        self.editor.selection.clear();
    }

    // ------------------------------------------------------------------ UI parts

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New project").clicked() {
                    self.replace_project(Project::default(), None);
                }
                if ui.button("Open…        Ctrl+O").clicked() {
                    self.io.open(&ctx, Purpose::Project);
                }
                if ui.button("Open demo project").clicked() {
                    self.replace_project(sample::demo_project(), None);
                }
                ui.separator();
                let save_label = if FileIo::can_save_in_place() { "Save          Ctrl+S" } else { "Download project   Ctrl+S" };
                if ui.button(save_label).clicked() {
                    self.save(&ctx, false);
                }
                if FileIo::can_save_in_place() && ui.button("Save as…").clicked() {
                    self.save(&ctx, true);
                }
                ui.separator();
                if ui.button("Export page as SVG…").clicked() {
                    let svg = svg::page_svg(&self.prj, self.page);
                    let name = format!("{}_page_{}.svg", self.suggested_name("x").trim_end_matches(".x"), self.prj.pages[self.page].name);
                    self.export(&ctx, name, ("SVG image", "svg"), svg);
                }
                if ui.button("Export all pages for printing (HTML/PDF)…").clicked() {
                    let html = svg::print_document(&self.prj);
                    self.export(&ctx, self.suggested_name("html"), ("HTML document", "html"), html);
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.add_enabled(self.history.can_undo(), egui::Button::new("Undo   Ctrl+Z")).clicked() {
                    self.history.undo(&mut self.prj);
                }
                if ui.add_enabled(self.history.can_redo(), egui::Button::new("Redo   Ctrl+Y")).clicked() {
                    self.history.redo(&mut self.prj);
                }
                ui.separator();
                let has_sel = !self.editor.selection.is_empty();
                if ui.add_enabled(has_sel, egui::Button::new("Rotate   R")).clicked() {
                    ops::rotate(&mut self.prj.pages[self.page], &self.editor.selection);
                }
                if ui.add_enabled(has_sel, egui::Button::new("Mirror   M")).clicked() {
                    ops::mirror(&mut self.prj.pages[self.page], &self.editor.selection);
                }
                if ui.add_enabled(has_sel, egui::Button::new("Delete   Del")).clicked() {
                    ops::delete(&mut self.prj.pages[self.page], &self.editor.selection);
                    self.editor.selection.clear();
                }
                if ui.button("Select all   Ctrl+A").clicked() {
                    self.editor.selection = ops::all_ids(&self.prj.pages[self.page]).into_iter().collect();
                }
                ui.label(RichText::new("Copy / paste: Ctrl+C, Ctrl+X, Ctrl+V").weak());
            });
            ui.menu_button("Insert", |ui| {
                if ui.button("Wire   W").clicked() {
                    self.editor.set_tool(Tool::Wire);
                }
                if ui.button("Text   T").clicked() {
                    self.editor.set_tool(Tool::Text);
                }
                if ui.button("Graphic line").clicked() {
                    self.editor.set_tool(Tool::Line);
                }
                if ui.button("Graphic rectangle").clicked() {
                    self.editor.set_tool(Tool::Rect);
                }
                ui.separator();
                for cat in Category::ALL {
                    ui.menu_button(cat.title(), |ui| {
                        for def in symbols::library().iter().filter(|d| d.category == cat) {
                            if ui.button(def.name).clicked() {
                                self.editor.set_tool(Tool::Place(def.key));
                            }
                        }
                    });
                }
            });
            ui.menu_button("Page", |ui| {
                if ui.button("New schematic page").clicked() {
                    self.add_page(PageKind::Schematic);
                }
                if ui.button("New cover sheet").clicked() {
                    self.add_page(PageKind::Cover);
                }
                ui.menu_button("New report page", |ui| {
                    for k in ReportKind::ALL {
                        if ui.button(k.title()).clicked() {
                            self.add_page(PageKind::Report(k));
                        }
                    }
                });
                ui.separator();
                if ui.button("Page properties…").clicked() {
                    self.page_props = Some(self.page);
                }
                if ui.add_enabled(self.page > 0, egui::Button::new("Move page up")).clicked() {
                    self.prj.pages.swap(self.page, self.page - 1);
                    self.page -= 1;
                }
                if ui.add_enabled(self.page + 1 < self.prj.pages.len(), egui::Button::new("Move page down")).clicked() {
                    self.prj.pages.swap(self.page, self.page + 1);
                    self.page += 1;
                }
                if ui.add_enabled(self.prj.pages.len() > 1, egui::Button::new("Delete page")).clicked() {
                    self.prj.pages.remove(self.page);
                    self.page = self.page.min(self.prj.pages.len() - 1);
                    self.editor.selection.clear();
                }
            });
            ui.menu_button("Project", |ui| {
                if ui.button("Project properties…").clicked() {
                    self.show_project = true;
                }
                if ui.button("Parts management…").clicked() {
                    self.parts_ui.open = true;
                }
                if ui.button("Check project").clicked() {
                    self.left_tab = LeftTab::Check;
                }
            });
            ui.menu_button("Reports", |ui| {
                for k in ReportKind::ALL {
                    if ui.button(k.title()).clicked() {
                        self.report_kind = k;
                        self.show_reports = true;
                    }
                }
            });
            ui.menu_button("View", |ui| {
                if ui.button("Fit page   F").clicked() {
                    self.editor.fit_pending = true;
                }
                ui.checkbox(&mut self.editor.show_grid, "Grid");
                ui.checkbox(&mut self.editor.show_pins, "Connection points");
                ui.separator();
                let mut pref = ui.ctx().options(|o| o.theme_preference);
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut pref, egui::ThemePreference::System, "System");
                    ui.selectable_value(&mut pref, egui::ThemePreference::Light, "Light");
                    ui.selectable_value(&mut pref, egui::ThemePreference::Dark, "Dark");
                });
                if pref != ui.ctx().options(|o| o.theme_preference) {
                    ui.ctx().set_theme(pref);
                }
            });
            ui.menu_button("Help", |ui| {
                if ui.button("Shortcuts & usage").clicked() {
                    self.show_help = true;
                }
            });
        });
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            let tool = self.editor.tool.clone();
            let mut tbtn = |ui: &mut egui::Ui, t: Tool, label: &str, tip: &str| {
                if ui.selectable_label(tool == t, label).on_hover_text(tip).clicked() {
                    self.editor.set_tool(t);
                }
            };
            tbtn(ui, Tool::Select, "⬉ Select", "Select / move (Esc)");
            tbtn(ui, Tool::Wire, "╱ Wire", "Draw connections (W)");
            tbtn(ui, Tool::Text, "T Text", "Place text (T)");
            tbtn(ui, Tool::Line, "─ Line", "Graphic line");
            tbtn(ui, Tool::Rect, "▭ Rect", "Graphic rectangle");
            if let Tool::Place(key) = tool {
                if let Some(def) = symbols::get(key) {
                    ui.label(RichText::new(format!("Placing: {}", def.name)).strong());
                }
            }
            ui.separator();
            if ui.add_enabled(self.history.can_undo(), egui::Button::new("↶")).on_hover_text("Undo (Ctrl+Z)").clicked() {
                self.history.undo(&mut self.prj);
            }
            if ui.add_enabled(self.history.can_redo(), egui::Button::new("↷")).on_hover_text("Redo (Ctrl+Y)").clicked() {
                self.history.redo(&mut self.prj);
            }
            let has_sel = !self.editor.selection.is_empty();
            if ui.add_enabled(has_sel, egui::Button::new("⟳")).on_hover_text("Rotate (R)").clicked() {
                ops::rotate(&mut self.prj.pages[self.page], &self.editor.selection);
            }
            if ui.add_enabled(has_sel, egui::Button::new("🗑")).on_hover_text("Delete (Del)").clicked() {
                ops::delete(&mut self.prj.pages[self.page], &self.editor.selection);
                self.editor.selection.clear();
            }
            ui.separator();
            if ui.button("⛶ Fit").on_hover_text("Fit page (F)").clicked() {
                self.editor.fit_pending = true;
            }
            ui.label(format!("{:.0}%", self.editor.zoom / 3.78 * 100.0)).on_hover_text("100% ≈ real size on a 96 dpi screen");
            ui.separator();
            if ui.add_enabled(self.page > 0, egui::Button::new("◀")).on_hover_text("Previous page (Page Up)").clicked() {
                self.goto(self.page - 1, 0);
            }
            let pg = &self.prj.pages[self.page];
            ui.label(RichText::new(format!("Page {}  {}", pg.name, pg.description)).strong());
            if ui.add_enabled(self.page + 1 < self.prj.pages.len(), egui::Button::new("▶")).on_hover_text("Next page (Page Down)").clicked() {
                self.goto(self.page + 1, 0);
            }
            ui.separator();
            if ui.button("📊 Reports").clicked() {
                self.show_reports = true;
            }
        });
    }

    fn navigator(&mut self, ui: &mut egui::Ui, idx: &Index) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.left_tab, LeftTab::Pages, "Pages");
            ui.selectable_value(&mut self.left_tab, LeftTab::Devices, "Devices");
            let n_err = self.analysis.issues.iter().filter(|i| i.severity == Severity::Error).count();
            let label = if n_err > 0 { RichText::new(format!("Check ({n_err})")).color(Color32::from_rgb(220, 60, 60)) } else { RichText::new("Check") };
            ui.selectable_value(&mut self.left_tab, LeftTab::Check, label);
        });
        ui.separator();
        match self.left_tab {
            LeftTab::Pages => {
                ui.horizontal(|ui| {
                    if ui.button("+ Page").on_hover_text("New schematic page after the current one").clicked() {
                        self.add_page(PageKind::Schematic);
                    }
                    if ui.button("⚙").on_hover_text("Page properties").clicked() {
                        self.page_props = Some(self.page);
                    }
                    if ui.add_enabled(self.page > 0, egui::Button::new("⏶")).clicked() {
                        self.prj.pages.swap(self.page, self.page - 1);
                        self.page -= 1;
                    }
                    if ui.add_enabled(self.page + 1 < self.prj.pages.len(), egui::Button::new("⏷")).clicked() {
                        self.prj.pages.swap(self.page, self.page + 1);
                        self.page += 1;
                    }
                });
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    let mut goto = None;
                    for (i, pg) in self.prj.pages.iter().enumerate() {
                        let icon = match pg.kind {
                            PageKind::Schematic => "🗋",
                            PageKind::Cover => "🏷",
                            PageKind::Report(_) => "📊",
                        };
                        let r = ui.selectable_label(i == self.page, format!("{icon} {}   {}", pg.name, pg.description));
                        if r.clicked() {
                            goto = Some(i);
                        }
                        if r.double_clicked() {
                            self.page_props = Some(i);
                        }
                    }
                    if let Some(i) = goto {
                        self.goto(i, 0);
                    }
                });
            }
            LeftTab::Devices => {
                ui.add(egui::TextEdit::singleline(&mut self.device_filter).hint_text("Filter devices…"));
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    let mut jump = None;
                    let f = self.device_filter.to_lowercase();
                    for tag in idx.sorted_tags() {
                        if !f.is_empty() && !tag.to_lowercase().contains(&f) {
                            continue;
                        }
                        let dev = &idx.devices[tag];
                        egui::CollapsingHeader::new(RichText::new(tag).monospace().strong()).id_salt(tag).show(ui, |ui| {
                            for (kind, locs) in [("Main", &dev.mains), ("Aux", &dev.auxs), ("", &dev.others)] {
                                for loc in locs {
                                    let inst = self.prj.pages[loc.page].symbols.iter().find(|s| s.id == loc.sym_id);
                                    let Some(inst) = inst else { continue };
                                    let name = symbols::get(&inst.sym).map(|d| d.name).unwrap_or("?");
                                    let extra = if inst.label.is_empty() { inst.pins.join("-") } else { inst.label.clone() };
                                    let text = format!("{} {kind} {name} {extra}", loc.reference(&self.prj));
                                    if ui.link(text.trim()).clicked() {
                                        jump = Some((loc.page, loc.sym_id));
                                    }
                                }
                            }
                        });
                    }
                    if idx.devices.is_empty() {
                        ui.weak("No devices yet. Pick a symbol from the library on the right and click into the page.");
                    }
                    if let Some((pg, id)) = jump {
                        self.goto(pg, id);
                    }
                });
            }
            LeftTab::Check => {
                let issues = self.analysis.issues.clone();
                let count = |s| issues.iter().filter(|i| i.severity == s).count();
                ui.label(format!(
                    "{} errors, {} warnings, {} notes",
                    count(Severity::Error),
                    count(Severity::Warning),
                    count(Severity::Info)
                ));
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    let mut jump = None;
                    for issue in &issues {
                        let (icon, color) = match issue.severity {
                            Severity::Error => ("⛔", Color32::from_rgb(220, 60, 60)),
                            Severity::Warning => ("⚠", Color32::from_rgb(220, 160, 0)),
                            Severity::Info => ("ℹ", ui.visuals().weak_text_color()),
                        };
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(icon).color(color));
                            let r = ui.add(egui::Label::new(&issue.message).sense(egui::Sense::click()));
                            if r.on_hover_text("Click to show").clicked() {
                                jump = issue.target;
                            }
                        });
                    }
                    if issues.is_empty() {
                        ui.label("✔ No problems found.");
                    }
                    if let Some((pg, id)) = jump {
                        self.goto(pg, id);
                    }
                });
            }
        }
    }

    fn library(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Symbols");
            ui.add(egui::TextEdit::singleline(&mut self.lib_filter).hint_text("Search…").desired_width(f32::INFINITY));
        });
        let f = self.lib_filter.to_lowercase();
        egui::ScrollArea::vertical().id_salt("lib").max_height(ui.available_height() * 0.45).auto_shrink([false, true]).show(ui, |ui| {
            for cat in Category::ALL {
                let defs: Vec<_> = symbols::library()
                    .iter()
                    .filter(|d| d.category == cat && (f.is_empty() || d.name.to_lowercase().contains(&f)))
                    .collect();
                if defs.is_empty() {
                    continue;
                }
                egui::CollapsingHeader::new(cat.title()).default_open(f.is_empty() || !defs.is_empty()).show(ui, |ui| {
                    for def in defs {
                        let active = self.editor.tool == Tool::Place(def.key);
                        ui.horizontal(|ui| {
                            let icon = symbol_icon(ui, def.key, egui::vec2(40.0, 30.0));
                            let lbl = ui.selectable_label(active, format!("{}  ({})", def.name, if def.letter.is_empty() { "–" } else { def.letter }));
                            if icon.clicked() || lbl.clicked() {
                                self.editor.set_tool(Tool::Place(def.key));
                                self.editor.place_rot = 0;
                                self.editor.place_mirror = false;
                            }
                        });
                    }
                });
            }
        });
    }

    fn properties(&mut self, ui: &mut egui::Ui, idx: &Index) {
        ui.heading("Properties");
        let sel: Vec<u64> = self.editor.selection.iter().copied().collect();
        let pi = self.page;
        if sel.is_empty() {
            let pg = &mut self.prj.pages[pi];
            ui.label(RichText::new(format!("Page {} – {}", pg.name, pg.kind.label())).strong());
            ui.label("Description");
            ui.text_edit_singleline(&mut pg.description);
            ui.add_space(6.0);
            ui.weak(self.editor.tool.hint());
            return;
        }
        if sel.len() > 1 {
            ui.label(format!("{} objects selected", sel.len()));
            ui.horizontal(|ui| {
                if ui.button("⟳ Rotate").clicked() {
                    ops::rotate(&mut self.prj.pages[pi], &self.editor.selection);
                }
                if ui.button("🗑 Delete").clicked() {
                    ops::delete(&mut self.prj.pages[pi], &self.editor.selection);
                    self.editor.selection.clear();
                }
            });
            let wires: Vec<u64> = self.prj.pages[pi].wires.iter().filter(|w| self.editor.selection.contains(&w.id)).map(|w| w.id).collect();
            if !wires.is_empty() {
                ui.separator();
                ui.label(format!("{} wires", wires.len()));
                let first = self.prj.pages[pi].wires.iter().find(|w| w.id == wires[0]).cloned().unwrap_or_default();
                let (mut color, mut cs) = (first.color.clone(), first.cross_section.clone());
                egui::Grid::new("wprops").num_columns(2).show(ui, |ui| {
                    ui.label("Colour");
                    ui.text_edit_singleline(&mut color);
                    ui.end_row();
                    ui.label("Cross-section");
                    ui.text_edit_singleline(&mut cs);
                    ui.end_row();
                });
                if color != first.color || cs != first.cross_section {
                    for w in self.prj.pages[pi].wires.iter_mut().filter(|w| wires.contains(&w.id)) {
                        w.color = color.clone();
                        w.cross_section = cs.clone();
                    }
                }
            }
            return;
        }
        let id = sel[0];
        let page = &self.prj.pages[pi];
        if let Some(si) = page.symbols.iter().position(|s| s.id == id) {
            self.symbol_properties(ui, idx, pi, si);
        } else if let Some(wi) = page.wires.iter().position(|w| w.id == id) {
            let net = self.analysis.netlist.net_of_wire(id);
            if let Some(net) = net {
                ui.label(RichText::new(format!("Potential: {}", net.name)).strong());
                ui.label("Connected to:");
                let mut v: Vec<String> = net.pins.iter().map(|p| pin_designation(&self.prj, *p)).collect();
                v.sort_by(|a, b| natural_cmp(a, b));
                v.dedup();
                ui.label(RichText::new(v.join(", ")).monospace());
                ui.weak("All wires of this potential are highlighted.");
            }
            ui.separator();
            let w = &mut self.prj.pages[pi].wires[wi];
            egui::Grid::new("wprops1").num_columns(2).show(ui, |ui| {
                ui.label("Colour");
                ui.add(egui::TextEdit::singleline(&mut w.color).hint_text("e.g. BU, BK, GNYE"));
                ui.end_row();
                ui.label("Cross-section");
                ui.add(egui::TextEdit::singleline(&mut w.cross_section).hint_text("e.g. 1.5 mm²"));
                ui.end_row();
                ui.label("From");
                ui.label(format!("{:.0} / {:.0}", w.a.x, w.a.y));
                ui.end_row();
                ui.label("To");
                ui.label(format!("{:.0} / {:.0}", w.b.x, w.b.y));
                ui.end_row();
            });
        } else if let Some(t) = self.prj.pages[pi].texts.iter_mut().find(|t| t.id == id) {
            ui.label("Text");
            ui.add(egui::TextEdit::multiline(&mut t.text).desired_rows(3));
            ui.add(egui::Slider::new(&mut t.size, 1.5..=20.0).text("Height (mm)"));
        } else if let Some(s) = self.prj.pages[pi].shapes.iter_mut().find(|s| s.id == id) {
            ui.label(match s.kind {
                ShapeKind::Line => "Graphic line",
                ShapeKind::Rect => "Graphic rectangle",
            });
            ui.checkbox(&mut s.dashed, "Dashed");
        }
    }

    fn symbol_properties(&mut self, ui: &mut egui::Ui, idx: &Index, pi: usize, si: usize) {
        let s = self.prj.pages[pi].symbols[si].clone();
        let Some(def) = symbols::get(&s.sym) else {
            ui.label(format!("Unknown symbol {}", s.sym));
            return;
        };
        ui.label(RichText::new(def.name).strong());
        let mut edited = s.clone();
        egui::Grid::new("sprops").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
            if def.func.has_tag() {
                ui.label(if def.func == Func::Terminal { "Terminal strip" } else if def.func == Func::Plc { "PLC card" } else { "Device tag" });
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut edited.tag).desired_width(90.0).font(egui::TextStyle::Monospace));
                    // Assign to an existing device (e.g. contact → coil)
                    egui::ComboBox::from_id_salt("assign").selected_text("…").width(30.0).show_ui(ui, |ui| {
                        for tag in idx.sorted_tags() {
                            let dev = &idx.devices[tag];
                            let fits = match def.func {
                                Func::Aux(_) => !dev.mains.is_empty(),
                                Func::Terminal | Func::Plc => !dev.others.is_empty(),
                                _ => true,
                            };
                            if fits && ui.selectable_label(*tag == edited.tag, tag.as_str()).clicked() {
                                edited.tag = tag.clone();
                            }
                        }
                    })
                    .response
                    .on_hover_text("Assign to existing device");
                });
                ui.end_row();
            }
            let label_name = match def.func {
                Func::Terminal => Some("Terminal number"),
                Func::Plc => Some("Address"),
                Func::Potential => Some("Potential name"),
                Func::Interruption => Some("Name"),
                _ => None,
            };
            if let Some(name) = label_name {
                ui.label(name);
                ui.add(egui::TextEdit::singleline(&mut edited.label).desired_width(120.0));
                ui.end_row();
            }
            if def.func.has_tag() {
                ui.label("Function text");
                ui.add(egui::TextEdit::multiline(&mut edited.function_text).desired_rows(2).desired_width(160.0));
                ui.end_row();
            }
            if def.show_pin_labels {
                ui.label("Connection points");
                ui.horizontal_wrapped(|ui| {
                    for pin in edited.pins.iter_mut() {
                        ui.add(egui::TextEdit::singleline(pin).desired_width(32.0));
                    }
                });
                ui.end_row();
            }
            ui.label("Orientation");
            ui.horizontal(|ui| {
                if ui.button("⟲").clicked() {
                    edited.rot = (edited.rot + 3) % 4;
                }
                ui.label(format!("{}°", edited.rot as u32 * 90));
                if ui.button("⟳").clicked() {
                    edited.rot = (edited.rot + 1) % 4;
                }
                ui.checkbox(&mut edited.mirror, "Mirror");
            });
            ui.end_row();
            ui.label("Position");
            ui.label(format!("{:.0} / {:.0} mm  (column {})", s.pos.x, s.pos.y, column_of(s.pos)));
            ui.end_row();
        });
        if let Func::Aux(kind) = def.func {
            if def.auto_number && ui.button("Renumber contact").on_hover_text("Next free contact numbers on this device").clicked() {
                edited.pins = ops::next_contact_pins(&self.prj, &edited.tag, kind, Some(s.id));
            }
        }
        if edited != s {
            self.prj.pages[pi].symbols[si] = edited.clone();
        }

        // Device data
        if def.func.has_tag() && !edited.tag.is_empty() {
            ui.separator();
            ui.label(RichText::new(format!("Device {}", edited.tag)).strong());
            let mut data = self.prj.devices.get(&edited.tag).cloned().unwrap_or_default();
            let before = data.clone();
            ui.horizontal(|ui| {
                ui.label("Description");
                ui.text_edit_singleline(&mut data.description);
            });
            // Part assignment (several parts per device, e.g. contactor + auxiliary block)
            ui.label(RichText::new("Parts").strong());
            let mut remove = None;
            let mut show_part = None;
            egui::Grid::new("devparts").num_columns(2).striped(true).show(ui, |ui| {
                for (i, r) in data.parts.iter_mut().enumerate() {
                    let known = self.prj.parts.iter().find(|p| p.number == r.number);
                    let link = ui.link(RichText::new(&r.number).monospace());
                    let link = match known {
                        Some(p) => link.on_hover_text(format!("{}\n{}", p.description, p.template_summary())),
                        None => link.on_hover_text("Not in the project parts – synchronize in parts management"),
                    };
                    if link.clicked() {
                        show_part = Some(r.number.clone());
                    }
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut r.count).range(1..=999).prefix("× "));
                        if ui.small_button("🗑").on_hover_text("Remove part").clicked() {
                            remove = Some(i);
                        }
                    });
                    ui.end_row();
                }
            });
            if let Some(i) = remove {
                data.parts.remove(i);
            }
            if let Some(n) = show_part {
                self.parts_ui.show_part(&n, true);
            }
            ui.horizontal(|ui| {
                if ui.button("➕ Add part…").on_hover_text("Part selection from the parts database").clicked() {
                    self.parts_ui.select_for(&edited.tag);
                }
                if !data.parts.is_empty()
                    && ui
                        .button("Apply designations")
                        .on_hover_text("Takes the connection point designations of all functions of this device from the function templates of its parts")
                        .clicked()
                {
                    parts::apply_designations(&mut self.prj, Some(&self.parts_db), &edited.tag);
                }
            });
            if data != before {
                self.prj.devices.insert(edited.tag.clone(), data.clone());
            }
            // Functions placed vs. provided by the parts' function templates
            let usage = parts::usage(&self.prj, None, &edited.tag);
            if usage.values().any(|(_, a)| *a > 0) {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    for (kind, (placed, avail)) in &usage {
                        let txt = format!("{} {placed}/{avail}", kind.title());
                        let color = if placed > avail && *kind != parts::FnKind::Terminal {
                            Color32::from_rgb(220, 60, 60)
                        } else if placed < avail {
                            ui.visuals().weak_text_color()
                        } else {
                            ui.visuals().text_color()
                        };
                        ui.label(RichText::new(txt).small().color(color)).on_hover_text("placed / provided by the parts");
                    }
                });
            }
            // Rename the complete device
            let rename_id = ui.scope_id().with("rename");
            let mut new_tag: String = ui.data_mut(|d| d.get_temp(rename_id)).unwrap_or_else(|| edited.tag.clone());
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut new_tag).desired_width(90.0));
                if ui.button("Rename device").on_hover_text("Changes the tag of all functions of this device").clicked() {
                    ops::rename_device(&mut self.prj, &edited.tag, &new_tag);
                }
            });
            ui.data_mut(|d| d.insert_temp(rename_id, new_tag));

            // Cross references
            if let Some(dev) = idx.devices.get(&edited.tag) {
                ui.separator();
                ui.label(RichText::new("Cross references").strong());
                let mut jump = None;
                for (kind, locs) in [("Main", &dev.mains), ("Aux", &dev.auxs), ("", &dev.others)] {
                    for loc in locs {
                        let inst = self.prj.pages[loc.page].symbols.iter().find(|x| x.id == loc.sym_id);
                        let Some(inst) = inst else { continue };
                        let dname = symbols::get(&inst.sym).map(|d| d.name).unwrap_or("?");
                        let extra = if inst.label.is_empty() { inst.pins.join("-") } else { inst.label.clone() };
                        let me = if loc.sym_id == s.id { " ◀" } else { "" };
                        if ui.link(format!("{} {kind} {dname} {extra}{me}", loc.reference(&self.prj))).clicked() {
                            jump = Some((loc.page, loc.sym_id));
                        }
                    }
                }
                if let Some((pg, id)) = jump {
                    self.goto(pg, id);
                }
            }
        }

        // Potentials at the connection points
        if !def.pins.is_empty() {
            ui.separator();
            ui.label(RichText::new("Connections").strong());
            egui::Grid::new("pinnets").num_columns(2).striped(true).show(ui, |ui| {
                for i in 0..def.pins.len() {
                    let pin = PinRef { page: pi, sym: si, pin: i };
                    ui.label(RichText::new(pin_designation(&self.prj, pin)).monospace());
                    let net = self.analysis.netlist.pin_net.get(&(s.id, i)).map(|&n| self.analysis.netlist.nets[n].name.clone());
                    let unconnected = self.analysis.netlist.unconnected.iter().any(|&(_, id, p)| id == s.id && p == i);
                    if unconnected {
                        ui.label(RichText::new("open").color(Color32::from_rgb(220, 120, 0)));
                    } else {
                        ui.label(net.unwrap_or_default());
                    }
                    ui.end_row();
                }
            });
        }
    }

    fn windows(&mut self, ctx: &egui::Context) {
        // Reports
        if self.show_reports {
            let mut open = true;
            let mut jump = None;
            let mut export = None;
            let mut add_page = None;
            egui::Window::new("Reports").open(&mut open).default_size([900.0, 520.0]).show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for k in ReportKind::ALL {
                        ui.selectable_value(&mut self.report_kind, k, k.title());
                    }
                });
                ui.separator();
                let kind = self.report_kind;
                ui.horizontal(|ui| {
                    if ui.button("⬇ Export CSV").clicked() {
                        export = Some(kind);
                    }
                    if ui.button("➕ Insert as report page").clicked() {
                        add_page = Some(kind);
                    }
                });
                let Some(table) = self.analysis.reports.get(&kind) else { return };
                egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                    egui::Grid::new("report").striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
                        for h in &table.headers {
                            ui.label(RichText::new(*h).strong());
                        }
                        ui.end_row();
                        for (row, target) in table.rows.iter().zip(&table.targets) {
                            for (i, cell) in row.iter().enumerate() {
                                if i == 0 && target.is_some() {
                                    if ui.link(cell).clicked() {
                                        jump = *target;
                                    }
                                } else {
                                    ui.label(cell);
                                }
                            }
                            ui.end_row();
                        }
                    });
                    if table.rows.is_empty() {
                        ui.weak("(no entries)");
                    }
                });
            });
            self.show_reports = open;
            if let Some((pg, id)) = jump {
                self.goto(pg, id);
            }
            if let Some(kind) = export {
                if let Some(t) = self.analysis.reports.get(&kind) {
                    let csv = t.to_csv();
                    let name = format!("{}.csv", kind.title().replace(' ', "_").to_lowercase());
                    self.export(ctx, name, ("CSV", "csv"), csv);
                }
            }
            if let Some(kind) = add_page {
                self.add_page(PageKind::Report(kind));
            }
        }

        // Project properties
        if self.show_project {
            let mut open = true;
            egui::Window::new("Project properties").open(&mut open).resizable(false).show(ctx, |ui| {
                let info = &mut self.prj.info;
                egui::Grid::new("prj").num_columns(2).show(ui, |ui| {
                    ui.label("Project name");
                    ui.text_edit_singleline(&mut info.name);
                    ui.end_row();
                    ui.label("Description");
                    ui.text_edit_multiline(&mut info.description);
                    ui.end_row();
                    ui.label("Company");
                    ui.text_edit_singleline(&mut info.company);
                    ui.end_row();
                    ui.label("Created by");
                    ui.text_edit_singleline(&mut info.creator);
                    ui.end_row();
                    ui.label("Date");
                    ui.text_edit_singleline(&mut info.date);
                    ui.end_row();
                    ui.label("Revision");
                    ui.text_edit_singleline(&mut info.revision);
                    ui.end_row();
                });
            });
            self.show_project = open;
        }

        // Parts management and part selection
        for action in self.parts_ui.manager(ctx, &mut self.parts_db, &mut self.prj) {
            match action {
                PartsAction::Import => self.io.open(ctx, Purpose::Parts),
                PartsAction::Export { name, filter, content } => self.export(ctx, name, filter, content),
                PartsAction::Message(m) => self.notify(ctx, m),
            }
        }
        self.parts_ui.selector(ctx, &self.parts_db, &mut self.prj);

        // Page properties
        if let Some(pi) = self.page_props {
            let mut open = true;
            if pi < self.prj.pages.len() {
                egui::Window::new("Page properties").open(&mut open).resizable(false).show(ctx, |ui| {
                    let pg = &mut self.prj.pages[pi];
                    egui::Grid::new("pgprops").num_columns(2).show(ui, |ui| {
                        ui.label("Page name / number");
                        ui.text_edit_singleline(&mut pg.name);
                        ui.end_row();
                        ui.label("Description");
                        ui.text_edit_singleline(&mut pg.description);
                        ui.end_row();
                        ui.label("Page type");
                        egui::ComboBox::from_id_salt("pgkind").selected_text(pg.kind.label()).show_ui(ui, |ui| {
                            ui.selectable_value(&mut pg.kind, PageKind::Schematic, PageKind::Schematic.label());
                            ui.selectable_value(&mut pg.kind, PageKind::Cover, PageKind::Cover.label());
                            for k in ReportKind::ALL {
                                ui.selectable_value(&mut pg.kind, PageKind::Report(k), PageKind::Report(k).label());
                            }
                        });
                        ui.end_row();
                    });
                    if pg.kind != PageKind::Schematic && !pg.is_empty() {
                        ui.colored_label(Color32::from_rgb(220, 140, 0), "This page contains schematic content which is not evaluated on non-schematic pages.");
                    }
                });
            } else {
                open = false;
            }
            if !open {
                self.page_props = None;
            }
        }

        if self.show_help {
            let mut open = true;
            egui::Window::new("OpenECraft – usage").open(&mut open).default_width(520.0).show(ctx, |ui| {
                ui.label(RichText::new("Workflow").strong());
                ui.label("1. Pick a symbol in the library (right) and click into the page to place it. Device tags (-K1, -S2 …) are assigned automatically.");
                ui.label("2. Select a coil, then place contacts: they are assigned to that coil and numbered (13/14, 21/22 …). Cross references and the contact image update live.");
                ui.label("3. Draw wires with the wire tool (W). Wires end automatically on connection points. T-junctions get a connection dot.");
                ui.label("4. Potentials (L1, +24V …) and interruption points name the nets across pages. Select a wire to highlight its potential.");
                ui.label("5. Assign parts in the property panel; generate reports (device list, BOM, terminal diagram, connection list, PLC list) and insert them as report pages.");
                ui.separator();
                ui.label(RichText::new("Shortcuts").strong());
                egui::Grid::new("keys").striped(true).show(ui, |ui| {
                    for (k, v) in [
                        ("Esc", "Cancel / select tool / clear selection"),
                        ("W / T", "Wire tool / text tool"),
                        ("R / M", "Rotate / mirror (selection or symbol being placed)"),
                        ("Space", "Toggle corner direction while drawing wires"),
                        ("Del", "Delete selection"),
                        ("Arrows", "Move selection by one grid step"),
                        ("Ctrl+C / X / V", "Copy / cut / paste (devices get new tags)"),
                        ("Ctrl+A", "Select all"),
                        ("Ctrl+Z / Ctrl+Y", "Undo / redo"),
                        ("Ctrl+S / Ctrl+O", "Save / open"),
                        ("F / Home", "Fit page"),
                        ("Page Up / Down", "Previous / next page"),
                        ("Wheel / right or middle drag", "Zoom / pan"),
                        ("Shift + click / drag", "Add to selection"),
                    ] {
                        ui.label(RichText::new(k).monospace());
                        ui.label(v);
                        ui.end_row();
                    }
                });
            });
            self.show_help = open;
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let sc = |m, k| KeyboardShortcut::new(m, k);
        let (undo, redo, redo2, save, open, pgup, pgdn, w, t) = ctx.input_mut(|i| {
            (
                i.consume_shortcut(&sc(Modifiers::COMMAND, Key::Z)),
                i.consume_shortcut(&sc(Modifiers::COMMAND, Key::Y)),
                i.consume_shortcut(&sc(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)),
                i.consume_shortcut(&sc(Modifiers::COMMAND, Key::S)),
                i.consume_shortcut(&sc(Modifiers::COMMAND, Key::O)),
                i.key_pressed(Key::PageUp),
                i.key_pressed(Key::PageDown),
                i.modifiers.is_none() && i.key_pressed(Key::W),
                i.modifiers.is_none() && i.key_pressed(Key::T),
            )
        });
        if undo {
            self.editor.cancel();
            self.history.undo(&mut self.prj);
        }
        if redo || redo2 {
            self.editor.cancel();
            self.history.redo(&mut self.prj);
        }
        if save {
            self.save(ctx, false);
        }
        if open {
            self.io.open(ctx, Purpose::Project);
        }
        if pgup && self.page > 0 {
            self.goto(self.page - 1, 0);
        }
        if pgdn && self.page + 1 < self.prj.pages.len() {
            self.goto(self.page + 1, 0);
        }
        if w {
            self.editor.set_tool(Tool::Wire);
        }
        if t {
            self.editor.set_tool(Tool::Text);
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_loaded(&ctx);
        self.shortcuts(&ctx);
        self.page = self.page.min(self.prj.pages.len().saturating_sub(1));

        if self.analysis_rev != self.history.rev {
            self.analysis = Analysis::build(&self.prj);
            self.analysis_rev = self.history.rev;
        }
        let idx = Index::build(&self.prj);

        egui::Panel::top("menu").show(ui, |ui| self.menu_bar(ui));
        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.monospace(&self.editor.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let name = self.file_name.as_deref().map(io::short_name).unwrap_or("unsaved");
                    ui.weak(format!("{}{}", name, if self.dirty() { " *" } else { "" }));
                    if let Some((msg, t)) = &self.message {
                        if ctx.input(|i| i.time) - t < 6.0 {
                            ui.label(RichText::new(msg).color(Color32::from_rgb(60, 160, 90)));
                        }
                    }
                });
            });
        });
        egui::Panel::left("nav").resizable(true).default_size(230.0).show(ui, |ui| self.navigator(ui, &idx));
        egui::Panel::right("side").resizable(true).default_size(300.0).show(ui, |ui| {
            self.library(ui);
            ui.separator();
            egui::ScrollArea::vertical().id_salt("props").auto_shrink([false, false]).show(ui, |ui| self.properties(ui, &idx));
        });
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
            let page = self.page;
            let out = self.editor.ui(ui, &mut self.prj, page, &idx, &self.analysis);
            let _ = out.open_properties;
        });
        self.windows(&ctx);

        // Commit finished edits as one undo step.
        let can_commit = !ctx.input(|i| i.pointer.any_down()) && ctx.memory(|m| m.focused().is_none()) && !self.editor.busy();
        self.history.track(&self.prj, can_commit);

        let title = format!(
            "OpenECraft – {}{}",
            self.prj.info.name,
            if self.dirty() { " *" } else { "" }
        );
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "project", &self.prj);
        eframe::set_value(storage, "file_name", &self.file_name);
        eframe::set_value(storage, "parts_db", &self.parts_db);
    }
}
