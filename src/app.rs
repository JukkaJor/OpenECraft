//! Main application window: menus, toolbar, navigators, symbol library, property
//! editor, reports and dialogs.

use openecraft_i18n::{tr, tr_fmt};
use crate::editor::{symbol_icon, Editor, Tool};
use crate::index::Index;
use crate::io::{self, FileIo, Purpose, PROJECT_EXT};
use crate::parts::{self, PartsDb};
use crate::panel_ui::PanelUi;
use crate::parts_ui::{PartsAction, PartsUi};
use crate::model::*;
use crate::netlist::{pin_designation, PinRef};
use crate::ops;
use crate::reports::{Analysis, Severity};
use crate::structure::{self, Dt};
use crate::symbols::{self, Category, Func};
use crate::{history::History, sample, svg, wirenum};
use egui::{Color32, Key, KeyboardShortcut, Modifiers, RichText};

mod engineering;
mod forms_ui;
mod harness_ui;
mod symbol_editor;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Workspace {
    Schematic,
    Panel,
}

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
    analysis_lang: openecraft_i18n::Lang,
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
    workspace: Workspace,
    panel_ui: PanelUi,
    show_help: bool,
    show_cables: bool,
    show_numbering: bool,
    /// Macros, terminal strips, PLC overview, revisions.
    eng: engineering::EngineeringUi,
    macro_lib: crate::macros::MacroLibrary,
    symed: symbol_editor::SymbolEditor,
    forms_ui: forms_ui::FormsUi,
    harness_ui: harness_ui::HarnessUi,
    /// Wire numbering: also renumber / remove manually set numbers.
    numbering_overwrite: bool,
    page_props: Option<usize>,
    message: Option<(String, f64)>,
    title: String,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> App {
        let mut prj: Option<Project> = None;
        let mut file_name = None;
        let mut parts_db: Option<PartsDb> = None;
        let mut server_url: Option<String> = None;
        let mut macro_lib = None;
        let mut language: Option<String> = None;
        if let Some(storage) = cc.storage {
            language = eframe::get_value(storage, openecraft_ui::language::STORAGE_KEY);
            prj = eframe::get_value(storage, "project");
            file_name = eframe::get_value(storage, "file_name").flatten();
            parts_db = eframe::get_value(storage, "parts_db");
            server_url = eframe::get_value(storage, "parts_server_url");
            macro_lib = eframe::get_value(storage, "macros");
        }
        openecraft_i18n::init(language.as_deref());
        let parts_db = parts_db.unwrap_or_default();
        let mut parts_ui = PartsUi::default();
        if let Some(url) = server_url {
            parts_ui.server.url = url;
        }
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
            analysis_lang: openecraft_i18n::lang(),
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
            parts_ui,
            workspace: Workspace::Schematic,
            panel_ui: PanelUi::default(),
            show_help: false,
            show_cables: false,
            show_numbering: false,
            eng: Default::default(),
            macro_lib: macro_lib.unwrap_or_default(),
            symed: Default::default(),
            forms_ui: Default::default(),
            harness_ui: Default::default(),
            numbering_overwrite: false,
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
                self.notify(ctx, tr_fmt!("Saved {}", io::short_name(&path)));
                self.file_name = Some(path);
                self.saved_rev = self.history.rev;
            }
            Ok(None) => {}
            Err(e) => self.notify(ctx, tr_fmt!("Saving failed: {}", e)),
        }
    }

    fn export(&mut self, ctx: &egui::Context, name: String, filter: (&str, &str), content: String) {
        match self.io.save_as(&name, filter, &content) {
            Ok(Some(path)) => self.notify(ctx, tr_fmt!("Exported {}", io::short_name(&path))),
            Ok(None) => {}
            Err(e) => self.notify(ctx, tr_fmt!("Export failed: {}", e)),
        }
    }

    fn poll_loaded(&mut self, ctx: &egui::Context) {
        if let Some(loaded) = self.io.poll() {
            if matches!(loaded.purpose, Purpose::Model(_)) {
                let msg = match self.parts_ui.apply_model(&loaded, &mut self.parts_db, &mut self.prj) {
                    Ok(m) => m,
                    Err(e) => tr_fmt!("Model import failed: {}", e),
                };
                self.notify(ctx, msg);
                return;
            }
            if loaded.purpose == Purpose::Macros {
                let text = loaded.text();
                let lib = serde_json::from_str::<crate::macros::MacroLibrary>(&text)
                    .or_else(|_| serde_json::from_str::<crate::macros::Macro>(&text).map(|m| crate::macros::MacroLibrary { macros: vec![m] }));
                match lib {
                    Ok(lib) => {
                        let n = self.macro_lib.merge(lib);
                        self.notify(ctx, tr_fmt!("Imported {} macros", n));
                    }
                    Err(e) => self.notify(ctx, tr_fmt!("Not a macro file: {}", e)),
                }
                return;
            }
            if loaded.purpose == Purpose::Symbols {
                let text = loaded.text();
                let lib = serde_json::from_str::<crate::symlib::SymbolLibrary>(&text)
                    .or_else(|_| serde_json::from_str::<crate::symlib::UserSymbol>(&text).map(|u| crate::symlib::SymbolLibrary { symbols: vec![u] }));
                match lib {
                    Ok(lib) => {
                        let (added, replaced) = crate::symlib::merge(&mut self.prj.symbols, lib);
                        self.symed.open = true;
                        self.notify(ctx, tr_fmt!("{} symbols added, {} replaced", added, replaced));
                    }
                    Err(e) => self.notify(ctx, tr_fmt!("Not a symbol library: {}", e)),
                }
                return;
            }
            if loaded.purpose == Purpose::Forms {
                match serde_json::from_str::<crate::forms::FormLibrary>(&loaded.text()) {
                    Ok(lib) => {
                        let n = lib.title_blocks.len();
                        for f in lib.title_blocks {
                            match self.prj.forms.title_blocks.iter_mut().find(|x| x.name == f.name) {
                                Some(x) => *x = f,
                                None => self.prj.forms.title_blocks.push(f),
                            }
                        }
                        self.notify(ctx, tr_fmt!("Imported {} title blocks", n));
                    }
                    Err(e) => self.notify(ctx, tr_fmt!("Not a title block file: {}", e)),
                }
                return;
            }
            if loaded.purpose == Purpose::PlcTable {
                match crate::plc::import_symbol_table(&mut self.prj, &loaded.text()) {
                    Ok(n) => self.notify(ctx, tr_fmt!("{} PLC function texts updated", n)),
                    Err(e) => self.notify(ctx, tr_fmt!("Symbol table import failed: {}", e)),
                }
                return;
            }
            if loaded.purpose == Purpose::Parts {
                match parts::import(&loaded.text()) {
                    Ok(list) => {
                        let n = list.len();
                        for p in list {
                            self.parts_db.upsert(p);
                        }
                        self.parts_db.sort();
                        self.notify(ctx, tr_fmt!("Imported {} parts into the database", n));
                    }
                    Err(e) => self.notify(ctx, tr_fmt!("Parts import failed: {}", e)),
                }
                return;
            }
            match serde_json::from_str::<Project>(&loaded.text()) {
                Ok(prj) => {
                    self.replace_project(prj, Some(loaded.name.clone()));
                    self.notify(ctx, tr_fmt!("Opened {}", io::short_name(&loaded.name)));
                }
                Err(e) => self.notify(ctx, tr_fmt!("Not a valid project file: {}", e)),
            }
        }
    }

    // ------------------------------------------------------------------ pages

    fn add_page(&mut self, kind: PageKind) {
        let at = (self.page + 1).min(self.prj.pages.len());
        self.page = ops::add_page(&mut self.prj, kind, at);
        self.editor.selection.clear();
    }

    // ------------------------------------------------------------------ UI parts

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(tr("File"), |ui| {
                if ui.button(tr("New project")).clicked() {
                    self.replace_project(Project::default(), None);
                }
                if ui.button(tr("Open…        Ctrl+O")).clicked() {
                    self.io.open(&ctx, Purpose::Project);
                }
                if ui.button(tr("Open demo project")).clicked() {
                    self.replace_project(sample::demo_project(), None);
                }
                ui.separator();
                let save_label = if FileIo::can_save_in_place() { tr("Save          Ctrl+S") } else { tr("Download project   Ctrl+S") };
                if ui.button(save_label).clicked() {
                    self.save(&ctx, false);
                }
                if FileIo::can_save_in_place() && ui.button(tr("Save as…")).clicked() {
                    self.save(&ctx, true);
                }
                ui.separator();
                if ui.button(tr("Export page as SVG…")).clicked() {
                    let svg = svg::page_svg(&self.prj, self.page);
                    let name = format!("{}_page_{}.svg", self.suggested_name("x").trim_end_matches(".x"), self.prj.pages[self.page].name);
                    self.export(&ctx, name, (tr("SVG image"), "svg"), svg);
                }
                if ui.button(tr("Export all pages for printing (HTML/PDF)…")).clicked() {
                    let html = svg::print_document(&self.prj);
                    self.export(&ctx, self.suggested_name("html"), (tr("HTML document"), "html"), html);
                }
                if ui.button(tr("Export as PDF (bookmarks, linked cross references)…")).clicked() {
                    let pdf = crate::pdf::document(&self.prj);
                    self.export(&ctx, self.suggested_name("pdf"), (tr("PDF document"), "pdf"), pdf);
                }
                if ui.button(tr("Export page as DXF…")).clicked() {
                    let dxf = crate::dxf::pages_dxf(&self.prj, &[self.page]);
                    let name = format!("{}_page_{}.dxf", self.suggested_name("x").trim_end_matches(".x"), self.prj.pages[self.page].name);
                    self.export(&ctx, name, (tr("DXF drawing"), "dxf"), dxf);
                }
                if ui.button(tr("Export all pages as DXF…")).clicked() {
                    let all: Vec<usize> = (0..self.prj.pages.len()).collect();
                    let dxf = crate::dxf::pages_dxf(&self.prj, &all);
                    self.export(&ctx, self.suggested_name("dxf"), (tr("DXF drawing"), "dxf"), dxf);
                }
                if ui.button(tr("Export production wire list (CSV)…")).on_hover_text(tr("Wire numbers, ends, colour, cross-section, routed length, cable cores")).clicked() {
                    let csv = crate::reports::production_wire_list(&self.prj, &self.analysis).to_csv();
                    self.export(&ctx, "wire_list.csv".into(), ("CSV", "csv"), csv);
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.separator();
                    if ui.button(tr("Quit")).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            });
            ui.menu_button(tr("Edit"), |ui| {
                if ui.add_enabled(self.history.can_undo(), egui::Button::new(tr("Undo   Ctrl+Z"))).clicked() {
                    self.history.undo(&mut self.prj);
                }
                if ui.add_enabled(self.history.can_redo(), egui::Button::new(tr("Redo   Ctrl+Y"))).clicked() {
                    self.history.redo(&mut self.prj);
                }
                ui.separator();
                let has_sel = !self.editor.selection.is_empty();
                if ui.add_enabled(has_sel, egui::Button::new(tr("Rotate   R"))).clicked() {
                    ops::rotate(&mut self.prj.pages[self.page], &self.editor.selection);
                }
                if ui.add_enabled(has_sel, egui::Button::new(tr("Mirror   M"))).clicked() {
                    ops::mirror(&mut self.prj.pages[self.page], &self.editor.selection);
                }
                if ui.add_enabled(has_sel, egui::Button::new(tr("Delete   Del"))).clicked() {
                    ops::delete(&mut self.prj.pages[self.page], &self.editor.selection);
                    self.editor.selection.clear();
                }
                if ui.button(tr("Select all   Ctrl+A")).clicked() {
                    self.editor.selection = ops::all_ids(&self.prj.pages[self.page]).into_iter().collect();
                }
                ui.label(RichText::new(tr("Copy / paste: Ctrl+C, Ctrl+X, Ctrl+V")).weak());
                ui.separator();
                if ui.add_enabled(has_sel, egui::Button::new(tr("Save selection as macro…"))).clicked() {
                    self.save_as_macro(false);
                }
            });
            ui.menu_button(tr("Insert"), |ui| {
                if ui.button(tr("Macro…")).clicked() {
                    self.eng.show_macros = true;
                }
                ui.separator();
                if ui.button(tr("Wire   W")).clicked() {
                    self.editor.set_tool(Tool::Wire);
                }
                if ui.button(tr("Text   T")).clicked() {
                    self.editor.set_tool(Tool::Text);
                }
                if ui.button(tr("Graphic line")).clicked() {
                    self.editor.set_tool(Tool::Line);
                }
                if ui.button(tr("Graphic rectangle")).clicked() {
                    self.editor.set_tool(Tool::Rect);
                }
                ui.separator();
                for cat in Category::ALL {
                    ui.menu_button(cat.title(), |ui| {
                        for def in symbols::library().iter().filter(|d| d.category == cat) {
                            if ui.button(tr(def.name)).clicked() {
                                self.editor.set_tool(Tool::Place(def.key));
                            }
                        }
                    });
                }
                let user = self.user_symbols();
                if !user.is_empty() {
                    ui.menu_button(Category::User.title(), |ui| {
                        for def in user {
                            if ui.button(format!("{}  ({})", tr(def.name), def.library)).clicked() {
                                self.editor.set_tool(Tool::Place(def.key));
                            }
                        }
                    });
                }
            });
            ui.menu_button(tr("Page"), |ui| {
                if ui.button(tr("New schematic page")).clicked() {
                    self.add_page(PageKind::Schematic);
                }
                if ui.button(tr("New cover sheet")).clicked() {
                    self.add_page(PageKind::Cover);
                }
                ui.menu_button(tr("New report page"), |ui| {
                    for k in ReportKind::ALL {
                        if ui.button(k.title()).clicked() {
                            self.add_page(PageKind::Report(k));
                        }
                    }
                });
                ui.menu_button(tr("New mounting layout page"), |ui| {
                    let encs: Vec<(u64, String)> = self.prj.layout.enclosures.iter().map(|e| (e.id, e.name.clone())).collect();
                    for (id, name) in encs {
                        if ui.button(name).clicked() {
                            self.add_page(PageKind::Mounting(id));
                        }
                    }
                    if self.prj.layout.enclosures.is_empty() {
                        ui.weak(tr("Create an enclosure in the panel layout first."));
                    }
                });
                ui.separator();
                if ui.button(tr("Page properties…")).clicked() {
                    self.page_props = Some(self.page);
                }
                if ui.button(tr("Save page as macro…")).clicked() {
                    self.save_as_macro(true);
                }
                if ui.add_enabled(self.page > 0, egui::Button::new(tr("Move page up"))).clicked() {
                    self.prj.pages.swap(self.page, self.page - 1);
                    self.page -= 1;
                }
                if ui.add_enabled(self.page + 1 < self.prj.pages.len(), egui::Button::new(tr("Move page down"))).clicked() {
                    self.prj.pages.swap(self.page, self.page + 1);
                    self.page += 1;
                }
                if ui.add_enabled(self.prj.pages.len() > 1, egui::Button::new(tr("Delete page"))).clicked() {
                    self.prj.pages.remove(self.page);
                    self.page = self.page.min(self.prj.pages.len() - 1);
                    self.editor.selection.clear();
                }
            });
            ui.menu_button(tr("Project"), |ui| {
                if ui.button(tr("Project properties…")).clicked() {
                    self.show_project = true;
                }
                if ui.button(tr("Parts management…")).clicked() {
                    self.parts_ui.open = true;
                }
                if ui.button(tr("Cables…")).clicked() {
                    self.show_cables = true;
                }
                if ui.button(tr("Wire numbering…")).clicked() {
                    self.show_numbering = true;
                }
                if ui.button(tr("Harnesses…")).clicked() {
                    self.harness_ui.open = true;
                }
                if ui.button(tr("Terminal strips…")).clicked() {
                    self.eng.show_strips = true;
                }
                if ui.button(tr("PLC overview…")).clicked() {
                    self.eng.show_plc = true;
                }
                if ui.button(tr("Revisions…")).clicked() {
                    self.eng.show_revisions = true;
                }
                if ui.button(tr("Macros…")).clicked() {
                    self.eng.show_macros = true;
                }
                ui.separator();
                if ui.button(tr("Symbol editor…")).clicked() {
                    self.symed.open = true;
                }
                if ui.button(tr("Title blocks and report forms…")).clicked() {
                    self.forms_ui.open = true;
                }
                if ui.button(tr("Check project")).clicked() {
                    self.left_tab = LeftTab::Check;
                }
            });
            ui.menu_button(tr("Reports"), |ui| {
                for k in ReportKind::ALL {
                    if ui.button(k.title()).clicked() {
                        self.report_kind = k;
                        self.show_reports = true;
                    }
                }
            });
            ui.menu_button(tr("View"), |ui| {
                if ui.button(tr("Fit page   F")).clicked() {
                    self.editor.fit_pending = true;
                }
                ui.checkbox(&mut self.editor.show_grid, tr("Grid"));
                ui.checkbox(&mut self.editor.show_pins, tr("Connection points"));
                ui.checkbox(&mut self.editor.show_revision_marks, tr("Revision marks"));
                ui.separator();
                openecraft_ui::language::menu(ui);
                let mut pref = ui.ctx().options(|o| o.theme_preference);
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut pref, egui::ThemePreference::System, tr("System"));
                    ui.selectable_value(&mut pref, egui::ThemePreference::Light, tr("Light"));
                    ui.selectable_value(&mut pref, egui::ThemePreference::Dark, tr("Dark"));
                });
                if pref != ui.ctx().options(|o| o.theme_preference) {
                    ui.ctx().set_theme(pref);
                }
            });
            ui.menu_button(tr("Help"), |ui| {
                if ui.button(tr("Shortcuts & usage")).clicked() {
                    self.show_help = true;
                }
            });
        });
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.workspace, Workspace::Schematic, RichText::new(tr("🗋 Schematic")).strong());
            ui.selectable_value(&mut self.workspace, Workspace::Panel, RichText::new(tr("🗄 Panel layout")).strong());
            ui.separator();
            if self.workspace == Workspace::Panel {
                if ui.add_enabled(self.history.can_undo(), egui::Button::new("↩")).on_hover_text(tr("Undo (Ctrl+Z)")).clicked() {
                    self.history.undo(&mut self.prj);
                }
                if ui.add_enabled(self.history.can_redo(), egui::Button::new("↪")).on_hover_text(tr("Redo (Ctrl+Y)")).clicked() {
                    self.history.redo(&mut self.prj);
                }
                ui.separator();
                if ui.button(tr("📊 Reports")).clicked() {
                    self.report_kind = ReportKind::PanelWiring;
                    self.show_reports = true;
                }
                return;
            }
            let tool = self.editor.tool.clone();
            let mut tbtn = |ui: &mut egui::Ui, t: Tool, label: &str, tip: &str| {
                if ui.selectable_label(tool == t, label).on_hover_text(tip).clicked() {
                    self.editor.set_tool(t);
                }
            };
            tbtn(ui, Tool::Select, tr("⬉ Select"), tr("Select / move (Esc)"));
            tbtn(ui, Tool::Wire, tr("✏ Wire"), tr("Draw connections (W)"));
            tbtn(ui, Tool::Text, tr("T Text"), tr("Place text (T)"));
            tbtn(ui, Tool::Line, tr("— Line"), tr("Graphic line"));
            tbtn(ui, Tool::Rect, tr("☐ Rect"), tr("Graphic rectangle"));
            if let Tool::Place(key) = tool {
                if let Some(def) = symbols::get(key) {
                    ui.label(RichText::new(tr_fmt!("Placing: {}", tr(def.name))).strong());
                }
            }
            ui.separator();
            if ui.add_enabled(self.history.can_undo(), egui::Button::new("↩")).on_hover_text(tr("Undo (Ctrl+Z)")).clicked() {
                self.history.undo(&mut self.prj);
            }
            if ui.add_enabled(self.history.can_redo(), egui::Button::new("↪")).on_hover_text(tr("Redo (Ctrl+Y)")).clicked() {
                self.history.redo(&mut self.prj);
            }
            let has_sel = !self.editor.selection.is_empty();
            if ui.add_enabled(has_sel, egui::Button::new("⟳")).on_hover_text(tr("Rotate (R)")).clicked() {
                ops::rotate(&mut self.prj.pages[self.page], &self.editor.selection);
            }
            if ui.add_enabled(has_sel, egui::Button::new("🗑")).on_hover_text(tr("Delete (Del)")).clicked() {
                ops::delete(&mut self.prj.pages[self.page], &self.editor.selection);
                self.editor.selection.clear();
            }
            ui.separator();
            if ui.button(tr("⛶ Fit")).on_hover_text(tr("Fit page (F)")).clicked() {
                self.editor.fit_pending = true;
            }
            ui.label(format!("{:.0}%", self.editor.zoom / 3.78 * 100.0)).on_hover_text(tr("100% ≈ real size on a 96 dpi screen"));
            ui.separator();
            if ui.add_enabled(self.page > 0, egui::Button::new("◀")).on_hover_text(tr("Previous page (Page Up)")).clicked() {
                self.goto(self.page - 1, 0);
            }
            let pg = &self.prj.pages[self.page];
            ui.label(RichText::new(tr_fmt!("Page {}  {}", structure::page_full_name(pg), pg.description)).strong());
            if ui.add_enabled(self.page + 1 < self.prj.pages.len(), egui::Button::new("▶")).on_hover_text(tr("Next page (Page Down)")).clicked() {
                self.goto(self.page + 1, 0);
            }
            ui.separator();
            if ui.button(tr("📊 Reports")).clicked() {
                self.show_reports = true;
            }
        });
    }

    fn navigator(&mut self, ui: &mut egui::Ui, idx: &Index) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.left_tab, LeftTab::Pages, tr("Pages"));
            ui.selectable_value(&mut self.left_tab, LeftTab::Devices, tr("Devices"));
            let n_err = self.analysis.issues.iter().filter(|i| i.severity == Severity::Error).count();
            let label = if n_err > 0 { RichText::new(tr_fmt!("Check ({})", n_err)).color(Color32::from_rgb(220, 60, 60)) } else { RichText::new(tr("Check")) };
            ui.selectable_value(&mut self.left_tab, LeftTab::Check, label);
        });
        ui.separator();
        match self.left_tab {
            LeftTab::Pages => {
                ui.horizontal(|ui| {
                    if ui.button(tr("+ Page")).on_hover_text(tr("New schematic page after the current one")).clicked() {
                        self.add_page(PageKind::Schematic);
                    }
                    if ui.button("⚙").on_hover_text(tr("Page properties")).clicked() {
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
                            PageKind::Mounting(_) => "🗄",
                        };
                        let r = ui.selectable_label(i == self.page, format!("{icon} {}   {}", structure::page_full_name(pg), pg.description));
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
                ui.add(egui::TextEdit::singleline(&mut self.device_filter).hint_text(tr("Filter devices…")));
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    let mut jump = None;
                    let f = self.device_filter.to_lowercase();
                    // Devices grouped by structure (=function+location).
                    let mut groups: Vec<(String, Vec<&String>)> = Vec::new();
                    for tag in idx.sorted_tags().into_iter().filter(|t| f.is_empty() || t.to_lowercase().contains(&f)) {
                        let st = Dt::parse(tag).structure();
                        match groups.iter_mut().find(|g| g.0 == st) {
                            Some(g) => g.1.push(tag),
                            None => groups.push((st, vec![tag])),
                        }
                    }
                    groups.sort_by(|a, b| natural_cmp(&a.0, &b.0));
                    let structured = groups.iter().any(|g| !g.0.is_empty());
                    let prj = &self.prj;
                    let mut device = |ui: &mut egui::Ui, tag: &String| {
                        let dev = &idx.devices[tag];
                        let title = if structured { Dt::parse(tag).name } else { tag.clone() };
                        egui::CollapsingHeader::new(RichText::new(title).monospace().strong()).id_salt(tag).show(ui, |ui| {
                            for (kind, locs) in [(tr("Main"), &dev.mains), (tr("Aux"), &dev.auxs), ("", &dev.others)] {
                                for loc in locs {
                                    let inst = prj.pages[loc.page].symbols.iter().find(|s| s.id == loc.sym_id);
                                    let Some(inst) = inst else { continue };
                                    let name = symbols::get(&inst.sym).map(|d| tr(d.name)).unwrap_or("?");
                                    let extra = if inst.label.is_empty() { inst.pins.join("-") } else { inst.label.clone() };
                                    let text = format!("{} {kind} {name} {extra}", loc.reference(prj));
                                    if ui.link(text.trim()).clicked() {
                                        jump = Some((loc.page, loc.sym_id));
                                    }
                                }
                            }
                        });
                    };
                    for (st, tags) in &groups {
                        if structured {
                            let title = if st.is_empty() { "(no structure)" } else { st.as_str() };
                            egui::CollapsingHeader::new(RichText::new(title).strong()).id_salt(("structure", st)).default_open(true).show(ui, |ui| {
                                for tag in tags {
                                    device(ui, tag);
                                }
                            });
                        } else {
                            for tag in tags {
                                device(ui, tag);
                            }
                        }
                    }
                    if idx.devices.is_empty() {
                        ui.weak(tr("No devices yet. Pick a symbol from the library on the right and click into the page."));
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
                            if r.on_hover_text(tr("Click to show")).clicked() {
                                jump = issue.target;
                            }
                        });
                    }
                    if issues.is_empty() {
                        ui.label(tr("✔ No problems found."));
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
            ui.heading(tr("Symbols"));
            ui.add(egui::TextEdit::singleline(&mut self.lib_filter).hint_text(tr("Search…")).desired_width(f32::INFINITY));
        });
        let f = self.lib_filter.to_lowercase();
        let user = self.user_symbols();
        let mut libs: Vec<&str> = user.iter().map(|d| d.library).collect();
        libs.dedup();
        egui::ScrollArea::vertical().id_salt("lib").max_height(ui.available_height() * 0.45).auto_shrink([false, true]).show(ui, |ui| {
            let groups = libs
                .iter()
                .map(|lib| (format!("{}: {lib}", Category::User.title()), user.iter().copied().filter(|d| d.library == *lib).collect::<Vec<_>>()))
                .chain(Category::ALL.into_iter().map(|cat| (cat.title().to_string(), symbols::library().iter().filter(|d| d.category == cat).collect())));
            for (title, defs) in groups {
                let defs: Vec<&symbols::SymbolDef> = defs.into_iter().filter(|d| f.is_empty() || (d.name.to_lowercase().contains(&f) || tr(d.name).to_lowercase().contains(&f))).collect();
                if defs.is_empty() {
                    continue;
                }
                // The extra libraries start collapsed.
                let main = !matches!(defs[0].category, Category::Nfpa | Category::Fluid | Category::SingleLine | Category::BlackBox);
                egui::CollapsingHeader::new(title).default_open(!f.is_empty() || main).show(ui, |ui| {
                    for def in defs {
                        let active = self.editor.tool == Tool::Place(def.key);
                        ui.horizontal(|ui| {
                            let icon = symbol_icon(ui, def.key, egui::vec2(40.0, 30.0));
                            let lbl = ui.selectable_label(active, format!("{}  ({})", tr(def.name), if def.letter.is_empty() { "–" } else { def.letter }));
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

    /// Registered symbols of the project symbol library, sorted by library and name.
    fn user_symbols(&self) -> Vec<&'static symbols::SymbolDef> {
        let mut v: Vec<&'static symbols::SymbolDef> = self.prj.symbols.iter().filter_map(|u| symbols::get(&u.key)).collect();
        v.sort_by(|a, b| a.library.cmp(b.library).then(natural_cmp(a.name, b.name)));
        v
    }

    fn properties(&mut self, ui: &mut egui::Ui, idx: &Index) {
        ui.heading(tr("Properties"));
        let sel: Vec<u64> = self.editor.selection.iter().copied().collect();
        let pi = self.page;
        if self.prj.pages[pi].locked {
            ui.label(RichText::new(tr("🔒 Released page – read only")).color(Color32::from_rgb(220, 140, 0)));
            ui.disable();
        }
        if sel.is_empty() {
            let pg = &mut self.prj.pages[pi];
            ui.label(RichText::new(tr_fmt!("Page {} – {}", structure::page_full_name(pg), pg.kind.label())).strong());
            ui.label(tr("Description"));
            ui.text_edit_singleline(&mut pg.description);
            if ui.link(tr("Structure (function / location)…")).on_hover_text(tr("Page properties")).clicked() {
                self.page_props = Some(pi);
            }
            ui.add_space(6.0);
            ui.weak(self.editor.tool.hint());
            return;
        }
        if sel.len() > 1 {
            ui.label(tr_fmt!("{} objects selected", sel.len()));
            ui.horizontal(|ui| {
                if ui.button(tr("⟳ Rotate")).clicked() {
                    ops::rotate(&mut self.prj.pages[pi], &self.editor.selection);
                }
                if ui.button(tr("🗑 Delete")).clicked() {
                    ops::delete(&mut self.prj.pages[pi], &self.editor.selection);
                    self.editor.selection.clear();
                }
            });
            let wires: Vec<u64> = self.prj.pages[pi].wires.iter().filter(|w| self.editor.selection.contains(&w.id)).map(|w| w.id).collect();
            if !wires.is_empty() {
                ui.separator();
                ui.label(tr_fmt!("{} wires", wires.len()));
                let first = self.prj.pages[pi].wires.iter().find(|w| w.id == wires[0]).cloned().unwrap_or_default();
                let (mut color, mut cs) = (first.color.clone(), first.cross_section.clone());
                egui::Grid::new("wprops").num_columns(2).show(ui, |ui| {
                    ui.label(tr("Colour"));
                    ui.text_edit_singleline(&mut color);
                    ui.end_row();
                    ui.label(tr("Cross-section"));
                    ui.text_edit_singleline(&mut cs);
                    ui.end_row();
                });
                if color != first.color || cs != first.cross_section {
                    for w in self.prj.pages[pi].wires.iter_mut().filter(|w| wires.contains(&w.id)) {
                        w.color = color.clone();
                        w.cross_section = cs.clone();
                    }
                }
                let cables: Vec<String> = self.prj.cables.iter().map(|c| c.tag.clone()).collect();
                let mut assign = None;
                ui.horizontal(|ui| {
                    ui.menu_button(tr("Assign to cable"), |ui| {
                        for c in &cables {
                            if ui.button(c).clicked() {
                                assign = Some(c.clone());
                            }
                        }
                        if cables.is_empty() {
                            ui.weak(tr("No cables yet – Project › Cables…"));
                        }
                    })
                    .response
                    .on_hover_text(tr("The wires get the free cores of the cable in order (left to right)"));
                    if ui.button(tr("Remove from cable")).clicked() {
                        for w in self.prj.pages[pi].wires.iter_mut().filter(|w| wires.contains(&w.id)) {
                            w.cable.clear();
                            w.core.clear();
                        }
                    }
                });
                let harnesses: Vec<String> = self.prj.harnesses.iter().map(|h| h.tag.clone()).collect();
                let mut to_harness = None;
                ui.horizontal(|ui| {
                    ui.menu_button(tr("Assign to harness"), |ui| {
                        for h in &harnesses {
                            if ui.button(h).clicked() {
                                to_harness = Some(h.clone());
                            }
                        }
                        if harnesses.is_empty() {
                            ui.weak(tr("No harnesses yet – Project › Harnesses…"));
                        }
                    })
                    .response
                    .on_hover_text(tr("Connections whose two ends are both on selected wires"));
                    if ui.button(tr("Remove from harness")).clicked() {
                        to_harness = Some(String::new());
                    }
                });
                if let Some(tag) = to_harness {
                    let sel: std::collections::BTreeSet<u64> = wires.iter().copied().collect();
                    let conns = crate::harness::connections_at(&self.prj, &self.analysis, pi, &sel);
                    let ends: Vec<(String, String)> = conns
                        .iter()
                        .map(|&i| {
                            let c = &self.analysis.connections[i];
                            (pin_designation(&self.prj, c.a), pin_designation(&self.prj, c.b))
                        })
                        .collect();
                    for (a, b) in &ends {
                        crate::harness::assign(&mut self.prj, a, b, &tag);
                    }
                    let ctx = ui.ctx().clone();
                    self.notify(&ctx, if tag.is_empty() { tr_fmt!("{} connections removed from their harness", ends.len()) } else { tr_fmt!("{} connections assigned to {}", ends.len(), tag) });
                }
                if let Some(c) = assign {
                    let sel: std::collections::BTreeSet<u64> = wires.iter().copied().collect();
                    let n = ops::assign_cores(&mut self.prj, pi, &sel, &c);
                    let ctx = ui.ctx().clone();
                    self.notify(&ctx, tr_fmt!("{} of {} wires assigned to cores of {}", n, wires.len(), c));
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
                ui.label(RichText::new(tr_fmt!("Potential: {}", net.name)).strong());
                ui.label(tr("Connected to:"));
                let mut v: Vec<String> = net.pins.iter().map(|p| pin_designation(&self.prj, *p)).collect();
                v.sort_by(|a, b| natural_cmp(a, b));
                v.dedup();
                ui.label(RichText::new(v.join(", ")).monospace());
                ui.weak(tr("All wires of this potential are highlighted."));
            }
            ui.separator();
            let open_net = self.analysis.open.wire_net.get(&id).map(|&n| self.analysis.open.nets[n].clone());
            let number_now = open_net.as_ref().map(|n| wirenum::number_of(&self.prj, n)).unwrap_or_default();
            let mut number = number_now.clone();
            let here = Page { function: self.prj.pages[pi].function.clone(), location: self.prj.pages[pi].location.clone(), ..Default::default() };
            let cables: Vec<(String, Vec<String>)> = self.prj.cables.iter().map(|c| (c.tag.clone(), c.core_list())).collect();
            // Cores of the wire's cable already used by other connections
            let own: Vec<u64> = open_net.as_ref().map(|n| n.wires.iter().map(|w| w.1).collect()).unwrap_or_default();
            let cable_now = self.prj.pages[pi].wires[wi].cable.clone();
            let used: Vec<String> = self
                .prj
                .pages
                .iter()
                .flat_map(|pg| pg.wires.iter())
                .filter(|x| x.cable == cable_now && !cable_now.is_empty() && !own.contains(&x.id))
                .map(|x| x.core.clone())
                .collect();
            let harnesses: Vec<String> = self.prj.harnesses.iter().map(|h| h.tag.clone()).collect();
            // Connections of this wire's potential section, each with its own harness.
            let conns: Vec<(String, String, String)> = crate::harness::connections_of_wire(&self.analysis, id)
                .into_iter()
                .map(|i| {
                    let c = &self.analysis.connections[i];
                    (pin_designation(&self.prj, c.a), pin_designation(&self.prj, c.b), c.harness.clone())
                })
                .collect();
            let mut harness_change = None;
            let w = &mut self.prj.pages[pi].wires[wi];
            egui::Grid::new("wprops1").num_columns(2).show(ui, |ui| {
                ui.label(tr("Wire number"));
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut number).hint_text(tr("not numbered")).desired_width(90.0))
                        .on_hover_text(tr("Applies to all wires of this connection; automatic numbering keeps manual numbers"));
                    if w.number_fixed {
                        ui.weak(tr("manual"));
                    }
                });
                ui.end_row();
                ui.label(tr("Cable"));
                let shown = if w.cable.is_empty() { "(none)".to_string() } else { structure::relative(&here, &w.cable) };
                egui::ComboBox::from_id_salt("wcable").selected_text(shown).show_ui(ui, |ui| {
                    if ui.selectable_label(w.cable.is_empty(), "(none)").clicked() {
                        w.cable.clear();
                        w.core.clear();
                    }
                    for (tag, _) in &cables {
                        if ui.selectable_label(w.cable == *tag, tag).clicked() && w.cable != *tag {
                            w.cable = tag.clone();
                            w.core.clear();
                        }
                    }
                });
                ui.end_row();
                if let Some((_, cores)) = cables.iter().find(|(t, _)| *t == w.cable) {
                    ui.label(tr("Core"));
                    egui::ComboBox::from_id_salt("wcore").selected_text(w.core.clone()).show_ui(ui, |ui| {
                        for c in cores {
                            let text = if used.contains(c) { tr_fmt!("{}  (used)", c) } else { c.clone() };
                            ui.selectable_value(&mut w.core, c.clone(), text);
                        }
                    });
                    ui.end_row();
                }
                ui.label(tr("Colour"));
                ui.add(egui::TextEdit::singleline(&mut w.color).hint_text(tr("e.g. BU, BK, GNYE")));
                ui.end_row();
                ui.label(tr("Cross-section"));
                ui.add(egui::TextEdit::singleline(&mut w.cross_section).hint_text(tr("e.g. 1.5 mm²")));
                ui.end_row();
                ui.label(tr("From"));
                ui.label(format!("{:.0} / {:.0}", w.a.x, w.a.y));
                ui.end_row();
                ui.label(tr("To"));
                ui.label(format!("{:.0} / {:.0}", w.b.x, w.b.y));
                ui.end_row();
            });
            if number != number_now {
                if let Some(net) = &open_net {
                    wirenum::set(&mut self.prj, net, number.trim(), true);
                }
            }
            if !conns.is_empty() {
                ui.separator();
                ui.label(RichText::new(tr("Connections / harness")).strong()).on_hover_text(tr("Each point-to-point connection can belong to a harness"));
                egui::Grid::new("wharness").num_columns(2).show(ui, |ui| {
                    for (i, (a, b, h)) in conns.iter().enumerate() {
                        ui.label(RichText::new(format!("{a} – {b}")).monospace());
                        let shown = if h.is_empty() { "(none)".to_string() } else { structure::relative(&here, h) };
                        egui::ComboBox::from_id_salt(("wharness", i)).selected_text(shown).show_ui(ui, |ui| {
                            if ui.selectable_label(h.is_empty(), "(none)").clicked() {
                                harness_change = Some((a.clone(), b.clone(), String::new()));
                            }
                            for t in &harnesses {
                                if ui.selectable_label(t == h, t).clicked() {
                                    harness_change = Some((a.clone(), b.clone(), t.clone()));
                                }
                            }
                        });
                        ui.end_row();
                    }
                });
            }
            if let Some((a, b, tag)) = harness_change {
                crate::harness::assign(&mut self.prj, &a, &b, &tag);
            }
        } else if let Some(t) = self.prj.pages[pi].texts.iter_mut().find(|t| t.id == id) {
            ui.label(tr("Text"));
            ui.add(egui::TextEdit::multiline(&mut t.text).desired_rows(3));
            ui.add(egui::Slider::new(&mut t.size, 1.5..=20.0).text(tr("Height (mm)")));
        } else if let Some(s) = self.prj.pages[pi].shapes.iter_mut().find(|s| s.id == id) {
            ui.label(match s.kind {
                ShapeKind::Line => tr("Graphic line"),
                ShapeKind::Rect => tr("Graphic rectangle"),
            });
            ui.checkbox(&mut s.dashed, tr("Dashed"));
        }
    }

    fn symbol_properties(&mut self, ui: &mut egui::Ui, idx: &Index, pi: usize, si: usize) {
        let s = self.prj.pages[pi].symbols[si].clone();
        let Some(def) = symbols::get(&s.sym) else {
            ui.label(tr_fmt!("Unknown symbol {}", s.sym));
            return;
        };
        ui.label(RichText::new(tr(def.name)).strong());
        let mut edited = s.clone();
        let here = Page { function: self.prj.pages[pi].function.clone(), location: self.prj.pages[pi].location.clone(), ..Default::default() };
        egui::Grid::new("sprops").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
            if def.func.has_tag() {
                ui.label(if def.func == Func::Terminal { tr("Terminal strip") } else if def.func == Func::Plc { tr("PLC card") } else { tr("Device tag") });
                ui.horizontal(|ui| {
                    // Shown relative to the page structure, stored complete.
                    let mut shown = structure::relative(&here, &edited.tag);
                    let before = shown.clone();
                    ui.add(egui::TextEdit::singleline(&mut shown).desired_width(90.0).font(egui::TextStyle::Monospace))
                        .on_hover_text(tr_fmt!("Complete tag: {}\nAspects missing here are taken from the page (=function +location).", edited.tag));
                    if shown != before {
                        edited.tag = if shown.trim().is_empty() { String::new() } else { structure::complete(&here, &shown) };
                    }
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
                    .on_hover_text(tr("Assign to existing device"));
                });
                ui.end_row();
            }
            let label_name = match def.func {
                Func::Terminal => Some(tr("Terminal number")),
                Func::Plc => Some(tr("Address")),
                Func::Potential => Some(tr("Potential name")),
                Func::Interruption => Some(tr("Name")),
                _ => None,
            };
            if let Some(name) = label_name {
                ui.label(name);
                ui.add(egui::TextEdit::singleline(&mut edited.label).desired_width(120.0));
                ui.end_row();
            }
            if def.func.has_tag() {
                ui.label(tr("Function text"));
                ui.add(egui::TextEdit::multiline(&mut edited.function_text).desired_rows(2).desired_width(160.0));
                ui.end_row();
            }
            if def.show_pin_labels {
                ui.label(tr("Connection points"));
                ui.horizontal_wrapped(|ui| {
                    for pin in edited.pins.iter_mut() {
                        ui.add(egui::TextEdit::singleline(pin).desired_width(32.0));
                    }
                });
                ui.end_row();
            }
            ui.label(tr("Orientation"));
            ui.horizontal(|ui| {
                if ui.button("⟲").clicked() {
                    edited.rot = (edited.rot + 3) % 4;
                }
                ui.label(format!("{}°", edited.rot as u32 * 90));
                if ui.button("⟳").clicked() {
                    edited.rot = (edited.rot + 1) % 4;
                }
                ui.checkbox(&mut edited.mirror, tr("Mirror"));
            });
            ui.end_row();
            ui.label(tr("Position"));
            ui.label(tr_fmt!("{} / {} mm  (column {})", format!("{:.0}", s.pos.x), format!("{:.0}", s.pos.y), column_of(s.pos)));
            ui.end_row();
        });
        if let Func::Aux(kind) = def.func {
            if def.auto_number && ui.button(tr("Renumber contact")).on_hover_text(tr("Next free contact numbers on this device")).clicked() {
                edited.pins = ops::next_contact_pins(&self.prj, &edited.tag, kind, Some(s.id));
            }
        }
        if edited != s {
            self.prj.pages[pi].symbols[si] = edited.clone();
        }

        // Device data
        if def.func.has_tag() && !edited.tag.is_empty() {
            ui.separator();
            ui.label(RichText::new(tr_fmt!("Device {}", edited.tag)).strong());
            let mut data = self.prj.devices.get(&edited.tag).cloned().unwrap_or_default();
            let before = data.clone();
            ui.horizontal(|ui| {
                ui.label(tr("Description"));
                ui.text_edit_singleline(&mut data.description);
            });
            // Part assignment (several parts per device, e.g. contactor + auxiliary block)
            ui.label(RichText::new(tr("Parts")).strong());
            let mut remove = None;
            let mut show_part = None;
            egui::Grid::new("devparts").num_columns(2).striped(true).show(ui, |ui| {
                for (i, r) in data.parts.iter_mut().enumerate() {
                    let known = self.prj.parts.iter().find(|p| p.number == r.number);
                    let link = ui.link(RichText::new(&r.number).monospace());
                    let link = match known {
                        Some(p) => link.on_hover_text(format!("{}\n{}", p.description, p.template_summary())),
                        None => link.on_hover_text(tr("Not in the project parts – synchronize in parts management")),
                    };
                    if link.clicked() {
                        show_part = Some(r.number.clone());
                    }
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut r.count).range(1..=999).prefix("× "));
                        if ui.small_button("🗑").on_hover_text(tr("Remove part")).clicked() {
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
                if ui.button(tr("➕ Add part…")).on_hover_text(tr("Part selection from the parts database")).clicked() {
                    self.parts_ui.select_for(&edited.tag);
                }
                if !data.parts.is_empty()
                    && ui
                        .button(tr("Apply designations"))
                        .on_hover_text(tr("Takes the connection point designations of all functions of this device from the function templates of its parts"))
                        .clicked()
                {
                    parts::apply_designations(&mut self.prj, Some(&self.parts_db), &edited.tag);
                }
            });
            if data != before {
                self.prj.devices.insert(edited.tag.clone(), data.clone());
            }
            if let Some((ei, _)) = self.prj.layout.find_device(&edited.tag) {
                let enc = self.prj.layout.enclosures[ei].name.clone();
                if ui.button(tr_fmt!("🗄 Show in panel layout ({})", enc)).clicked() && self.panel_ui.show_device(&self.prj, &edited.tag) {
                    self.workspace = Workspace::Panel;
                }
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
                        ui.label(RichText::new(txt).small().color(color)).on_hover_text(tr("placed / provided by the parts"));
                    }
                });
            }
            // Rename the complete device
            let rename_id = ui.scope_id().with("rename");
            let mut new_tag: String = ui.data_mut(|d| d.get_temp(rename_id)).unwrap_or_else(|| edited.tag.clone());
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut new_tag).desired_width(90.0));
                if ui.button(tr("Rename device")).on_hover_text(tr("Changes the tag of all functions of this device")).clicked() {
                    ops::rename_device(&mut self.prj, &edited.tag, &new_tag);
                }
            });
            ui.data_mut(|d| d.insert_temp(rename_id, new_tag));

            // Cross references
            if let Some(dev) = idx.devices.get(&edited.tag) {
                ui.separator();
                ui.label(RichText::new(tr("Cross references")).strong());
                let mut jump = None;
                for (kind, locs) in [(tr("Main"), &dev.mains), (tr("Aux"), &dev.auxs), ("", &dev.others)] {
                    for loc in locs {
                        let inst = self.prj.pages[loc.page].symbols.iter().find(|x| x.id == loc.sym_id);
                        let Some(inst) = inst else { continue };
                        let dname = symbols::get(&inst.sym).map(|d| tr(d.name)).unwrap_or("?");
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
            ui.label(RichText::new(tr("Connections")).strong());
            egui::Grid::new("pinnets").num_columns(2).striped(true).show(ui, |ui| {
                for i in 0..def.pins.len() {
                    let pin = PinRef { page: pi, sym: si, pin: i };
                    ui.label(RichText::new(pin_designation(&self.prj, pin)).monospace());
                    let net = self.analysis.netlist.pin_net.get(&(s.id, i)).map(|&n| self.analysis.netlist.nets[n].name.clone());
                    let unconnected = self.analysis.netlist.unconnected.iter().any(|&(_, id, p)| id == s.id && p == i);
                    if unconnected {
                        ui.label(RichText::new(tr("open")).color(Color32::from_rgb(220, 120, 0)));
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
            egui::Window::new(tr("Reports")).open(&mut open).default_size([900.0, 520.0]).show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for k in ReportKind::ALL {
                        ui.selectable_value(&mut self.report_kind, k, k.title());
                    }
                });
                ui.separator();
                let kind = self.report_kind;
                ui.horizontal(|ui| {
                    if ui.button(tr("⬇ Export CSV")).clicked() {
                        export = Some(kind);
                    }
                    if ui.button(tr("➕ Insert as report page")).clicked() {
                        add_page = Some(kind);
                    }
                    if ui.button(tr("Form…")).on_hover_text(tr("Columns, order and rows of the report pages")).clicked() {
                        self.forms_ui.open_report(kind);
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
                        ui.weak(tr("(no entries)"));
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
            let prj_tb_names = crate::forms::names(&self.prj);
            egui::Window::new(tr("Project properties")).open(&mut open).resizable(false).show(ctx, |ui| {
                let info = &mut self.prj.info;
                egui::Grid::new("prj").num_columns(2).show(ui, |ui| {
                    ui.label(tr("Project name"));
                    ui.text_edit_singleline(&mut info.name);
                    ui.end_row();
                    ui.label(tr("Description"));
                    ui.text_edit_multiline(&mut info.description);
                    ui.end_row();
                    ui.label(tr("Company"));
                    ui.text_edit_singleline(&mut info.company);
                    ui.end_row();
                    ui.label(tr("Created by"));
                    ui.text_edit_singleline(&mut info.creator);
                    ui.end_row();
                    ui.label(tr("Date"));
                    ui.text_edit_singleline(&mut info.date);
                    ui.end_row();
                    ui.label(tr("Revision"));
                    ui.text_edit_singleline(&mut info.revision);
                    ui.end_row();
                    ui.label(tr("Title block"));
                    let forms = &mut self.prj.forms;
                    let current = if forms.default_title_block.is_empty() { crate::forms::STANDARD.to_string() } else { forms.default_title_block.clone() };
                    egui::ComboBox::from_id_salt("prjtb").selected_text(&current).show_ui(ui, |ui| {
                        for n in prj_tb_names.iter().cloned() {
                            if ui.selectable_label(n == current, &n).clicked() {
                                forms.default_title_block = if n == crate::forms::STANDARD { String::new() } else { n };
                            }
                        }
                    });
                    ui.end_row();
                });
                ui.separator();
                ui.label(RichText::new(tr("Project properties")).strong()).on_hover_text(tr("Shown in title blocks with {prop:Name}"));
                let props = &mut self.prj.info.properties;
                let mut remove = None;
                let mut rename = None;
                egui::Grid::new("prjprops").num_columns(3).show(ui, |ui| {
                    for (k, v) in props.iter_mut() {
                        let mut key = k.clone();
                        if ui.add(egui::TextEdit::singleline(&mut key).desired_width(120.0)).changed() {
                            rename = Some((k.clone(), key));
                        }
                        ui.add(egui::TextEdit::singleline(v).desired_width(200.0));
                        if ui.small_button("🗑").clicked() {
                            remove = Some(k.clone());
                        }
                        ui.end_row();
                    }
                });
                if let Some((old, new)) = rename {
                    if !new.is_empty() && !props.contains_key(&new) {
                        if let Some(v) = props.remove(&old) {
                            props.insert(new, v);
                        }
                    }
                }
                if let Some(k) = remove {
                    props.remove(&k);
                }
                if ui.button(tr("➕ Property")).clicked() {
                    let mut n = props.len() + 1;
                    while props.contains_key(&format!("Property{n}")) {
                        n += 1;
                    }
                    props.insert(format!("Property{n}"), String::new());
                }
            });
            self.show_project = open;
        }

        // Parts management and part selection
        for action in self.parts_ui.manager(ctx, &mut self.parts_db, &mut self.prj) {
            match action {
                PartsAction::Import => self.io.open(ctx, Purpose::Parts),
                PartsAction::ImportModel(kind) => self.io.open(ctx, Purpose::Model(kind)),
                PartsAction::Export { name, filter, content } => self.export(ctx, name, filter, content),
                PartsAction::Message(m) => self.notify(ctx, m),
            }
        }
        self.parts_ui.selector(ctx, &self.parts_db, &mut self.prj);

        self.cables_window(ctx);
        self.numbering_window(ctx);
        self.engineering_windows(ctx);
        self.symbol_editor_window(ctx);
        self.forms_window(ctx);
        self.harness_window(ctx);

        // Page properties
        if let Some(pi) = self.page_props {
            let mut open = true;
            let mut apply_structure = None;
            if pi < self.prj.pages.len() {
                let tb_names = crate::forms::names(&self.prj);
                egui::Window::new(tr("Page properties")).open(&mut open).resizable(false).show(ctx, |ui| {
                    let pg = &mut self.prj.pages[pi];
                    egui::Grid::new("pgprops").num_columns(2).show(ui, |ui| {
                        ui.label(tr("Page name / number"));
                        ui.text_edit_singleline(&mut pg.name);
                        ui.end_row();
                        ui.label(tr("Description"));
                        ui.text_edit_singleline(&mut pg.description);
                        ui.end_row();
                        ui.label(tr("Page type"));
                        egui::ComboBox::from_id_salt("pgkind").selected_text(pg.kind.label()).show_ui(ui, |ui| {
                            ui.selectable_value(&mut pg.kind, PageKind::Schematic, PageKind::Schematic.label());
                            ui.selectable_value(&mut pg.kind, PageKind::Cover, PageKind::Cover.label());
                            for k in ReportKind::ALL {
                                ui.selectable_value(&mut pg.kind, PageKind::Report(k), PageKind::Report(k).label());
                            }
                            for e in &self.prj.layout.enclosures {
                                ui.selectable_value(&mut pg.kind, PageKind::Mounting(e.id), tr_fmt!("Mounting layout {}", e.name));
                            }
                        });
                        ui.end_row();
                    });
                    ui.horizontal(|ui| {
                        ui.label(tr("Title block"));
                        let shown = if pg.title_block.is_empty() { "(project default)".to_string() } else { pg.title_block.clone() };
                        egui::ComboBox::from_id_salt("pgtb").selected_text(shown).show_ui(ui, |ui| {
                            ui.selectable_value(&mut pg.title_block, String::new(), tr("(project default)"));
                            for n in &tb_names {
                                ui.selectable_value(&mut pg.title_block, n.clone(), n);
                            }
                        });
                    });
                    if pg.kind != PageKind::Schematic && !pg.is_empty() {
                        ui.colored_label(Color32::from_rgb(220, 140, 0), tr("This page contains schematic content which is not evaluated on non-schematic pages."));
                    }
                    ui.separator();
                    ui.label(RichText::new(tr("Structure (IEC 81346)")).strong());
                    let key = ui.scope_id().with(("page_structure", pi));
                    let (mut function, mut location, mut move_devices): (String, String, bool) =
                        ui.data_mut(|d| d.get_temp(key)).unwrap_or_else(|| (pg.function.clone(), pg.location.clone(), true));
                    egui::Grid::new("pgstruct").num_columns(2).show(ui, |ui| {
                        ui.label(tr("Function  ="));
                        ui.add(egui::TextEdit::singleline(&mut function).hint_text(tr("e.g. A1, PUMP.2")));
                        ui.end_row();
                        ui.label(tr("Location  +"));
                        ui.add(egui::TextEdit::singleline(&mut location).hint_text(tr("e.g. CAB1, FIELD")));
                        ui.end_row();
                    });
                    ui.checkbox(&mut move_devices, tr("Move the devices of this page into the new structure"))
                        .on_hover_text(tr("Renames the devices (all their functions, also on other pages)"));
                    let changed = structure::clean_aspect(&function) != pg.function || structure::clean_aspect(&location) != pg.location;
                    if ui.add_enabled(changed, egui::Button::new(tr("Apply structure"))).clicked() {
                        apply_structure = Some((function.clone(), location.clone(), move_devices));
                    }
                    ui.weak(tr("Devices placed on the page get its structure; their tags are shown without it."));
                    ui.data_mut(|d| d.insert_temp(key, (function, location, move_devices)));
                });
            } else {
                open = false;
            }
            if let Some((function, location, move_devices)) = apply_structure {
                ops::set_page_structure(&mut self.prj, pi, &function, &location, move_devices);
            }
            if !open {
                self.page_props = None;
            }
        }

        if self.show_help {
            let mut open = true;
            egui::Window::new(tr("OpenECraft – usage")).open(&mut open).default_width(520.0).show(ctx, |ui| {
                ui.label(RichText::new(tr("Workflow")).strong());
                ui.label(tr("1. Pick a symbol in the library (right) and click into the page to place it. Device tags (-K1, -S2 …) are assigned automatically."));
                ui.label(tr("2. Select a coil, then place contacts: they are assigned to that coil and numbered (13/14, 21/22 …). Cross references and the contact image update live."));
                ui.label(tr("3. Draw wires with the wire tool (W). Wires end automatically on connection points. T-junctions get a connection dot."));
                ui.label(tr("4. Potentials (L1, +24V …) and interruption points name the nets across pages. Select a wire to highlight its potential."));
                ui.label(tr("5. Assign parts in the property panel; generate reports (device list, BOM, terminal diagram, connection list, PLC list) and insert them as report pages."));
                ui.label(tr("6. Structure: give pages a function (=A1) and location (+CAB1) in the page properties. Devices placed there get it (=A1+CAB1-K1); on the page only the differing part is shown."));
                ui.label(tr("7. Cables: define them in Project › Cables…, select wires and use Assign to cable. Wire numbers: Project › Wire numbering…, or type a number in the wire properties."));
                ui.label(tr("8. Macros: select items › Edit › Save selection as macro; insert them from Insert › Macro…. Placeholders {NAME} are filled from value sets."));
                ui.label(tr("9. Project › Terminal strips / PLC overview / Revisions: strip order, jumpers and spares; PLC addresses and symbol tables; revisions with change marks and released (locked) pages."));
                ui.label(tr("10. Project › Symbol editor: draw your own symbols (or copy built-in ones) with connection points and function; they are saved in the project and can be exchanged as library files."));
                ui.label(tr("11. Project › Title blocks and report forms: own title blocks with placeholders ({project}, {page}, {prop:Name} …) and the columns of report pages."));
                ui.label(tr("12. Panel layout: busbars and cutouts; the enclosure properties export the drilling pattern (NC data as DXF / CSV). The production wire list contains stripping lengths and ferrules."));
                ui.label(tr("13. Harnesses: Project › Harnesses…; select the wires between two devices › Assign to harness, or per connection in the wire properties. Harness data report with lengths, connectors and bundle diameter, KBL export."));
                ui.label(tr("14. Door: switch the panel front view to Door and place command / signal devices there; their door cutouts are generated. Collisions are checked with the parts' 3D models."));
                ui.separator();
                ui.label(RichText::new(tr("Shortcuts")).strong());
                egui::Grid::new("keys").striped(true).show(ui, |ui| {
                    for (k, v) in [
                        (tr("Esc"), tr("Cancel / select tool / clear selection")),
                        ("W / T", tr("Wire tool / text tool")),
                        ("R / M", tr("Rotate / mirror (selection or symbol being placed)")),
                        (tr("Space"), tr("Toggle corner direction while drawing wires")),
                        (tr("Del"), tr("Delete selection")),
                        (tr("Arrows"), tr("Move selection by one grid step")),
                        (tr("Ctrl+C / X / V"), tr("Copy / cut / paste (devices get new tags)")),
                        (tr("Ctrl+A"), tr("Select all")),
                        (tr("Ctrl+Z / Ctrl+Y"), tr("Undo / redo")),
                        (tr("Ctrl+S / Ctrl+O"), tr("Save / open")),
                        (tr("F / Home"), tr("Fit page")),
                        (tr("Page Up / Down"), tr("Previous / next page")),
                        (tr("Wheel / right or middle drag"), tr("Zoom / pan")),
                        (tr("Shift + click / drag"), tr("Add to selection")),
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

    fn cables_window(&mut self, ctx: &egui::Context) {
        if !self.show_cables {
            return;
        }
        let mut open = true;
        let mut renamed: Vec<(String, String)> = Vec::new();
        let mut remove = None;
        let mut picked: Vec<String> = Vec::new();
        let mut overview = false;
        let mut used: std::collections::HashMap<String, std::collections::BTreeSet<String>> = Default::default();
        for w in self.prj.pages.iter().flat_map(|pg| pg.wires.iter()).filter(|w| !w.cable.is_empty()) {
            used.entry(w.cable.clone()).or_default().insert(w.core.clone());
        }
        let structure = structure::page_structure(&self.prj.pages[self.page]);
        egui::Window::new(tr("Cables")).open(&mut open).default_size([1000.0, 360.0]).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button(tr("➕ New cable")).on_hover_text(tr("In the structure of the current page")).clicked() {
                    let tag = self.prj.next_tag(&structure, "W");
                    let id = self.prj.new_id();
                    self.prj.cables.push(Cable { id, tag, cores: "BN BK GY GNYE".into(), cross_section: "1.5 mm²".into(), ..Default::default() });
                }
                if ui.button(tr("📊 Cable overview")).clicked() {
                    overview = true;
                }
                ui.weak(tr("Assign wires: select them in the schematic › Assign to cable."));
            });
            ui.separator();
            let cable_parts: Vec<&parts::Part> = self.parts_db.parts.iter().filter(|p| p.group == "Cables").collect();
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                egui::Grid::new("cables").striped(true).num_columns(9).spacing([8.0, 4.0]).show(ui, |ui| {
                    for h in [tr("Cable"), tr("Type"), tr("Cores"), tr("Cross-section"), tr("Length"), tr("Part"), tr("Description"), tr("Used"), ""] {
                        ui.label(RichText::new(h).strong());
                    }
                    ui.end_row();
                    for (i, c) in self.prj.cables.iter_mut().enumerate() {
                        let old = c.tag.clone();
                        ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(&mut c.tag).font(egui::TextStyle::Monospace));
                        if c.tag != old {
                            renamed.push((old.clone(), c.tag.clone()));
                        }
                        ui.add_sized([120.0, 20.0], egui::TextEdit::singleline(&mut c.cable_type));
                        ui.add_sized([140.0, 20.0], egui::TextEdit::singleline(&mut c.cores)).on_hover_text(tr("Core designations separated by blanks"));
                        ui.add_sized([70.0, 20.0], egui::TextEdit::singleline(&mut c.cross_section));
                        ui.add(egui::DragValue::new(&mut c.length).speed(0.5).range(0.0..=100000.0).suffix(" m"));
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut c.part).desired_width(120.0));
                            ui.menu_button("…", |ui| {
                                for p in &cable_parts {
                                    if ui.button(format!("{}  {}", p.number, p.description)).clicked() {
                                        c.part = p.number.clone();
                                        c.cable_type = p.type_number.clone();
                                        if let Some(cores) = p.tech_value("Cores") {
                                            c.cores = cores.to_string();
                                        }
                                        if let Some(cs) = p.tech_value("Cross-section") {
                                            c.cross_section = cs.to_string();
                                        }
                                        picked.push(p.number.clone());
                                    }
                                }
                                if cable_parts.is_empty() {
                                    ui.weak(tr("No parts of the product group Cables in the parts database."));
                                }
                            });
                        });
                        ui.add_sized([160.0, 20.0], egui::TextEdit::singleline(&mut c.description));
                        let n = used.get(&old).map(|s| s.len()).unwrap_or(0);
                        ui.label(format!("{n}/{}", c.core_list().len()));
                        if ui.small_button("🗑").on_hover_text(tr("Delete cable (its wires are removed from it)")).clicked() {
                            remove = Some(i);
                        }
                        ui.end_row();
                    }
                });
                if self.prj.cables.is_empty() {
                    ui.weak(tr("No cables defined."));
                }
            });
        });
        for (old, new) in renamed {
            for w in self.prj.pages.iter_mut().flat_map(|pg| pg.wires.iter_mut()).filter(|w| w.cable == old) {
                w.cable = new.clone();
            }
        }
        if let Some(i) = remove {
            let tag = self.prj.cables.remove(i).tag;
            for w in self.prj.pages.iter_mut().flat_map(|pg| pg.wires.iter_mut()).filter(|w| w.cable == tag) {
                w.cable.clear();
                w.core.clear();
            }
        }
        for n in picked {
            parts::ensure_in_project(&mut self.prj, &self.parts_db, &n);
        }
        if overview {
            self.report_kind = ReportKind::CableOverview;
            self.show_reports = true;
        }
        self.show_cables = open;
    }

    fn numbering_window(&mut self, ctx: &egui::Context) {
        if !self.show_numbering {
            return;
        }
        let mut open = true;
        let (mut number, mut remove) = (false, false);
        let nets: Vec<&crate::netlist::Net> = self.analysis.open.nets.iter().filter(|n| !n.wires.is_empty() && !n.pins.is_empty()).collect();
        let numbered = nets.iter().filter(|n| !wirenum::number_of(&self.prj, n).is_empty()).count();
        let total = nets.len();
        egui::Window::new(tr("Wire numbering")).open(&mut open).resizable(false).show(ctx, |ui| {
            let s = &mut self.prj.wire_numbering;
            ui.label(tr("Every connection gets one number for all its wires; the two sides of a terminal are separate connections."));
            ui.add_space(4.0);
            for scheme in NumberScheme::ALL {
                ui.radio_value(&mut s.scheme, scheme, scheme.title());
            }
            egui::Grid::new("numbering").num_columns(2).show(ui, |ui| {
                ui.label(tr("Prefix"));
                ui.add(egui::TextEdit::singleline(&mut s.prefix).desired_width(60.0).hint_text(tr("e.g. W")));
                ui.end_row();
                ui.label(tr("Start at"));
                ui.add(egui::DragValue::new(&mut s.start).range(0..=1_000_000));
                ui.end_row();
            });
            ui.checkbox(&mut self.numbering_overwrite, tr("Include manually set numbers"));
            ui.label(tr_fmt!("{} of {} connections numbered", numbered, total));
            ui.horizontal(|ui| {
                number = ui.button(tr("Number wires")).clicked();
                remove = ui.button(tr("Remove numbers")).clicked();
            });
        });
        if number {
            let n = wirenum::renumber(&mut self.prj, &self.analysis.open, &self.analysis.netlist, self.numbering_overwrite);
            self.notify(ctx, tr_fmt!("{} connections numbered", n));
        }
        if remove {
            let n = wirenum::clear(&mut self.prj, self.numbering_overwrite);
            self.notify(ctx, tr_fmt!("Numbers removed from {} wires", n));
        }
        self.show_numbering = open;
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
        if self.workspace != Workspace::Schematic {
            return;
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
        if let Some(msg) = self.parts_ui.poll_server(&mut self.parts_db) {
            self.notify(&ctx, msg);
        }
        self.shortcuts(&ctx);
        self.page = self.page.min(self.prj.pages.len().saturating_sub(1));

        // User symbols are rebuilt when they change (not while one is dragged in the
        // symbol editor).
        if !self.symed.busy() {
            crate::symlib::register(&self.prj.symbols);
        }
        if self.analysis_rev != self.history.rev || self.analysis_lang != openecraft_i18n::lang() {
            self.analysis = Analysis::build(&self.prj);
            self.analysis_rev = self.history.rev;
            self.analysis_lang = openecraft_i18n::lang();
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
        match self.workspace {
            Workspace::Schematic => {
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
            }
            Workspace::Panel => {
                egui::Panel::left("panel_nav").resizable(true).default_size(250.0).show(ui, |ui| {
                    self.panel_ui.navigator(ui, &mut self.prj, &self.parts_db);
                });
                egui::Panel::right("panel_side").resizable(true).default_size(300.0).show(ui, |ui| {
                    egui::ScrollArea::vertical().id_salt("panel_props").auto_shrink([false, false]).show(ui, |ui| {
                        self.panel_ui.properties(ui, &mut self.prj, &self.analysis);
                    });
                });
                egui::CentralPanel::default().show(ui, |ui| {
                    self.panel_ui.canvas(ui, &mut self.prj, &self.analysis, &self.parts_db);
                });
                self.panel_ui.dialogs(&ctx, &mut self.prj, &self.parts_db);
                self.editor.status = self.panel_ui.status.clone();
                if let Some(tag) = self.panel_ui.goto_schematic.take() {
                    self.workspace = Workspace::Schematic;
                    let target = idx.devices.get(&tag).and_then(|d| d.mains.first().or(d.others.first()).or(d.auxs.first()).copied());
                    if let Some(loc) = target {
                        self.goto(loc.page, loc.sym_id);
                    }
                }
                if let Some((ei, face, dxf)) = self.panel_ui.export.take() {
                    if let Some(e) = self.prj.layout.enclosures.get(ei) {
                        let base: String = e.name.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
                        let what = if face == crate::panel::Face::Door { "door" } else { "plate" };
                        if dxf {
                            let content = crate::production::nc_dxf(&self.prj, ei, face);
                            self.export(&ctx, format!("{what}{base}.dxf"), (tr("DXF drawing"), "dxf"), content);
                        } else {
                            let content = crate::production::nc_csv(&self.prj, ei, face);
                            self.export(&ctx, format!("{what}{base}_drilling.csv"), ("CSV", "csv"), content);
                        }
                    }
                }
                if let Some(id) = self.panel_ui.add_page.take() {
                    self.add_page(PageKind::Mounting(id));
                    self.workspace = Workspace::Schematic;
                }
            }
        }
        self.windows(&ctx);

        // Commit finished edits as one undo step.
        let can_commit = !ctx.input(|i| i.pointer.any_down()) && ctx.memory(|m| m.focused().is_none()) && !self.editor.busy() && !self.panel_ui.busy() && !self.symed.busy() && !self.forms_ui.busy();
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
        eframe::set_value(storage, "parts_server_url", &self.parts_ui.server.url);
        eframe::set_value(storage, "macros", &self.macro_lib);
        eframe::set_value(storage, openecraft_ui::language::STORAGE_KEY, &openecraft_i18n::lang().code());
    }
}
