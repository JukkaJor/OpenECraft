//! Main application window: menus, toolbar, navigators, symbol library, property
//! editor, reports and dialogs.

use openecraft_i18n::{tr, tr_fmt};
use crate::editor::{symbol_icon, Editor, Tool};
use crate::session::Session;
use openecraft_core::index::Index;
use crate::io::{self, FileIo, Loaded, Purpose, PROJECT_EXT};
use openecraft_core::parts::{self, PartsDb};
use crate::panel_ui::{NcFormat, PanelUi};
use crate::wiring_ui::WiringUi;
use crate::parts_ui::{PartsAction, PartsUi};
use openecraft_core::model::*;
use openecraft_core::netlist::{pin_designation, PinRef};
use openecraft_core::ops;
use openecraft_core::analysis::Analysis;
use openecraft_core::issue::Severity;
use openecraft_core::structure::{self, Aspect, Dt};
use openecraft_core::symbols::{self, Category, Func};
use openecraft_core::{potentials, properties, sample, translation, wirenum};
use egui::{Color32, Key, KeyboardShortcut, Modifiers, RichText};
use openecraft_api::params::{text, Args};
use serde_json::Map;

mod arrange_ui;
mod cables_ui;
mod check_ui;
mod files;
mod find_ui;
mod menus;
mod navigator;
mod numbering_ui;
mod properties_panel;
mod toolbar;
mod windows;
mod xref_ui;
mod compare_ui;
mod engineering;
mod field_ui;
mod forms_ui;
mod harness_ui;
mod integration_ui;
mod languages_ui;
mod config_ui;
mod drawing_ui;
mod electrical_ui;
mod plc_ui;
mod plugins_ui;
mod plugs_ui;
mod tables_ui;
mod pages_ui;
mod potentials_ui;
mod projects_ui;
mod properties_ui;
mod structure_ui;
mod process_ui;
mod singleline_ui;
mod remote_ui;
mod requests_ui;
mod review_ui;
mod revisions_ui;
mod report_blocks_ui;
mod symbol_editor;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Workspace {
    Schematic,
    Panel,
    /// Wiring assistant for the workshop.
    Wiring,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LeftTab {
    Pages,
    Devices,
    Connections,
    Cables,
    Layers,
    Check,
    Review,
    /// Functions of the project data not placed yet.
    Unplaced,
}

pub struct App {
    /// The open project with its history, analysis and saved state.
    session: Session,
    editor: Editor,
    io: FileIo,
    left_tab: LeftTab,
    lib_filter: String,
    device_filter: String,
    connection_filter: String,
    /// Rename device dialog: old tag and the tag entered.
    rename_device: Option<(String, String)>,
    /// Active work area (name); empty = the whole project.
    work_area: String,
    structure_ui: structure_ui::StructureUi,
    properties_ui: properties_ui::PropertiesUi,
    projects_ui: projects_ui::ProjectsUi,
    pages_ui: pages_ui::PagesUi,
    show_reports: bool,
    report_kind: ReportId,
    show_project: bool,
    parts_db: PartsDb,
    parts_ui: PartsUi,
    workspace: Workspace,
    panel_ui: PanelUi,
    wiring_ui: WiringUi,
    show_help: bool,
    show_cables: bool,
    show_numbering: bool,
    numbering_ui: numbering_ui::NumberingUi,
    check_ui: check_ui::CheckUi,
    arrange_ui: arrange_ui::ArrangeUi,
    show_xref: bool,
    find_ui: find_ui::FindUi,
    /// Pages tab grouped by structure.
    page_tree: bool,
    /// Macros, terminal strips, PLC overview, revisions.
    eng: engineering::EngineeringUi,
    macro_lib: openecraft_core::macros::MacroLibrary,
    symed: symbol_editor::SymbolEditor,
    forms_ui: forms_ui::FormsUi,
    report_blocks_ui: report_blocks_ui::ReportBlocksUi,
    harness_ui: harness_ui::HarnessUi,
    field_ui: field_ui::FieldUi,
    plugs_ui: plugs_ui::PlugsUi,
    plc_ui: plc_ui::PlcUi,
    integration_ui: integration_ui::IntegrationUi,
    process_ui: process_ui::ProcessUi,
    electrical_ui: electrical_ui::ElectricalUi,
    singleline_ui: singleline_ui::SingleLineUi,
    languages_ui: languages_ui::LanguagesUi,
    tables_ui: tables_ui::TablesUi,
    config_ui: config_ui::ConfigUi,
    /// Wire numbering: also renumber / remove manually set numbers.
    numbering_overwrite: bool,
    page_props: Option<usize>,
    message: Option<(String, f64)>,
    title: String,
    /// Project server: shared projects and locks.
    remote: crate::remote::Remote,
    /// Comments and markup.
    review_ui: review_ui::ReviewUi,
    revisions_ui: revisions_ui::RevisionsUi,
    compare_ui: compare_ui::CompareUi,
    requests_ui: requests_ui::RequestsUi,
    /// Built-in and external plugins (`openecraft_api`).
    plugins: openecraft_api::Plugins,
    plugins_ui: plugins_ui::PluginsUi,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> App {
        let mut prj: Option<Project> = None;
        let mut file_name = None;
        let mut parts_db: Option<PartsDb> = None;
        let mut server_url: Option<String> = None;
        let mut remote = crate::remote::Remote::default();
        let mut macro_lib = None;
        let mut disabled_plugins: Vec<String> = Vec::new();
        let mut language: Option<String> = None;
        let mut projects_ui = projects_ui::ProjectsUi::default();
        if let Some(storage) = cc.storage {
            language = eframe::get_value(storage, openecraft_ui::language::STORAGE_KEY);
            // Packed; stores written before have the plain project.
            prj = storage.get_string("project_packed").and_then(|s| openecraft_core::archive::unpack(&s).ok()).or_else(|| eframe::get_value(storage, "project"));
            file_name = eframe::get_value(storage, "file_name").flatten();
            parts_db = eframe::get_value(storage, "parts_db");
            server_url = eframe::get_value(storage, "parts_server_url");
            macro_lib = eframe::get_value(storage, "macros");
            if let Some(db) = eframe::get_value(storage, "project_database") {
                projects_ui.db = db;
            }
            disabled_plugins = eframe::get_value(storage, plugins_ui::STORAGE_KEY).unwrap_or_default();
            if let Some(url) = eframe::get_value(storage, "project_server_url") {
                remote.url = url;
            }
            if let Some(user) = eframe::get_value::<String>(storage, "project_server_user").filter(|u| !u.is_empty()) {
                remote.user = user;
            }
        }
        openecraft_i18n::init(language.as_deref());
        let parts_db = parts_db.unwrap_or_default();
        let mut parts_ui = PartsUi::default();
        if let Some(url) = server_url {
            parts_ui.server.url = url;
        }
        let prj = prj.unwrap_or_else(sample::demo_project);
        let mut plugins = openecraft_api::Plugins::new(Vec::<String>::new());
        plugins.load_env();
        for id in &disabled_plugins {
            plugins.set_enabled(id, false);
        }
        App {
            session: Session::new(prj, file_name),
            editor: Editor::default(),
            io: FileIo::default(),
            left_tab: LeftTab::Pages,
            lib_filter: String::new(),
            device_filter: String::new(),
            connection_filter: String::new(),
            rename_device: None,
            work_area: String::new(),
            structure_ui: Default::default(),
            properties_ui: Default::default(),
            projects_ui,
            pages_ui: Default::default(),
            show_reports: false,
            report_kind: ReportKind::DeviceList.into(),
            show_project: false,
            parts_db,
            parts_ui,
            workspace: Workspace::Schematic,
            panel_ui: PanelUi::default(),
            wiring_ui: WiringUi::default(),
            show_help: false,
            show_cables: false,
            show_numbering: false,
            numbering_ui: Default::default(),
            check_ui: Default::default(),
            arrange_ui: Default::default(),
            show_xref: false,
            find_ui: Default::default(),
            page_tree: false,
            eng: Default::default(),
            macro_lib: macro_lib.unwrap_or_default(),
            symed: Default::default(),
            forms_ui: Default::default(),
            report_blocks_ui: Default::default(),
            harness_ui: Default::default(),
            field_ui: Default::default(),
            plugs_ui: Default::default(),
            plc_ui: Default::default(),
            integration_ui: Default::default(),
            process_ui: Default::default(),
            electrical_ui: Default::default(),
            singleline_ui: Default::default(),
            languages_ui: Default::default(),
            tables_ui: Default::default(),
            config_ui: Default::default(),
            numbering_overwrite: false,
            page_props: None,
            message: None,
            title: String::new(),
            remote,
            review_ui: Default::default(),
            revisions_ui: Default::default(),
            compare_ui: Default::default(),
            requests_ui: Default::default(),
            plugins,
            plugins_ui: Default::default(),
        }
    }

    /// Opens the wiring assistant instead of the schematic.
    pub fn start_wiring(mut self, wiring: bool) -> Self {
        if wiring {
            self.workspace = Workspace::Wiring;
        }
        self
    }

    /// The reports of the registry (built-in and of plugins): id and title.
    fn report_list(&self) -> Vec<(ReportId, String)> {
        self.plugins.reports().map(|r| (ReportId::new(&r.name), text(&r.title).to_string())).collect()
    }

    /// Title of a report: from the registry, otherwise of the built-in kind or the name.
    pub(super) fn report_title(&self, id: ReportId) -> String {
        self.plugins.report(id.as_str()).map(|r| text(&r.title).to_string()).unwrap_or_else(|| id.title())
    }

    fn notify(&mut self, ctx: &egui::Context, msg: impl Into<String>) {
        self.message = Some((msg.into(), ctx.input(|i| i.time)));
    }

    fn replace_project(&mut self, prj: Project, file_name: Option<String>) {
        self.remote.close();
        self.session.replace(prj, file_name);
        self.editor.selection.clear();
        self.editor.cancel();
        self.editor.fit_pending = true;
    }

    fn goto(&mut self, page: usize, id: u64) {
        if page >= self.session.prj.pages.len() {
            return;
        }
        self.session.page = page;
        self.editor.cancel();
        self.editor.selection.clear();
        if id != 0 {
            self.editor.selection.insert(id);
            if let Some(b) = ops::item_bounds(&self.session.prj.symbol_table(), &self.session.prj.pages[page], id) {
                if self.editor.zoom < 3.0 {
                    self.editor.zoom = 4.0;
                }
                self.editor.center_on = Some(b.center());
            }
        }
    }

    // ------------------------------------------------------------------ files

    // ------------------------------------------------------------------ pages

    fn add_page(&mut self, kind: PageKind) {
        let at = (self.session.page + 1).min(self.session.prj.pages.len());
        self.session.page = ops::add_page(&mut self.session.prj, kind, at);
        self.editor.selection.clear();
    }

    // ------------------------------------------------------------------ UI parts

}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_loaded(&ctx);
        self.poll_remote(&ctx);
        let actions = self.parts_ui.poll_server(&ctx, &mut self.parts_db);
        self.parts_actions(&ctx, actions);
        self.shortcuts(&ctx);
        self.session.clamp_page();

        // The device index follows every committed change; while an edit may be in
        // progress (mouse down, text field focused, drag) it is rebuilt every frame.
        let editing = ctx.input(|i| i.pointer.any_down()) || ctx.memory(|m| m.focused().is_some()) || self.editor.busy() || self.panel_ui.busy() || self.symed.busy() || self.forms_ui.busy();
        self.session.refresh(&self.plugins, editing);
        let idx = std::mem::take(&mut self.session.index);

        egui::Panel::top("menu").show(ui, |ui| self.menu_bar(ui));
        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.monospace(&self.editor.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let name = match &self.remote.shared {
                        Some(sh) => tr_fmt!("{} (project server, revision {})", sh.name, sh.revision),
                        None => self.session.file_name.as_deref().map(io::short_name).unwrap_or("unsaved").to_string(),
                    };
                    ui.weak(format!("{}{}", name, if self.session.dirty() { " *" } else { "" }));
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
                    let page = self.session.page;
                    self.editor.read_only = self.page_locked_by().is_some();
                    self.editor.review_selected = self.review_ui.selected;
                    let out = self.editor.ui(ui, &mut self.session.prj, page, &idx, &self.session.analysis);
                    let _ = out.open_properties;
                    if let Some(url) = out.open_url {
                        ctx.open_url(egui::OpenUrl::new_tab(url));
                    }
                    if let Some(msg) = out.message {
                        self.notify(&ctx, msg);
                    }
                    if let Some(input) = out.review {
                        self.review_input(input);
                    } else if let Some(at) = out.clicked {
                        self.review_pick(at);
                    }
                });
            }
            Workspace::Wiring => {
                self.panel_ui.selection.clear();
                self.wiring_ui.sync(&self.session.prj, &self.session.analysis, &mut self.panel_ui);
                egui::Panel::left("wiring_list").resizable(true).default_size(380.0).show(ui, |ui| {
                    self.wiring_ui.checklist(ui, &mut self.session.prj, &self.session.analysis, &mut self.panel_ui);
                });
                egui::CentralPanel::default().show(ui, |ui| {
                    self.wiring_ui.workbench(ui, &mut self.session.prj, &self.session.analysis, &mut self.panel_ui);
                });
                self.editor.status = self.panel_ui.status.clone();
                if std::mem::take(&mut self.wiring_ui.export) {
                    let content = openecraft_core::wiring::csv(&self.session.prj, &self.session.analysis.wiring, self.wiring_ui.order);
                    self.export(&ctx, "wiring.csv".into(), ("CSV", "csv"), content);
                }
            }
            Workspace::Panel => {
                self.panel_ui.done_wires.clear();
                egui::Panel::left("panel_nav").resizable(true).default_size(250.0).show(ui, |ui| {
                    self.panel_ui.navigator(ui, &mut self.session.prj, &self.parts_db);
                });
                egui::Panel::right("panel_side").resizable(true).default_size(300.0).show(ui, |ui| {
                    egui::ScrollArea::vertical().id_salt("panel_props").auto_shrink([false, false]).show(ui, |ui| {
                        self.panel_ui.properties(ui, &mut self.session.prj, &self.session.analysis, &self.parts_db);
                    });
                });
                egui::CentralPanel::default().show(ui, |ui| {
                    self.panel_ui.canvas(ui, &mut self.session.prj, &self.session.analysis, &self.parts_db);
                });
                self.panel_ui.dialogs(&ctx, &mut self.session.prj, &self.parts_db);
                self.editor.status = self.panel_ui.status.clone();
                if let Some(tag) = self.panel_ui.goto_schematic.take() {
                    self.workspace = Workspace::Schematic;
                    let target = idx.devices.get(&tag).and_then(|d| d.mains.first().or(d.others.first()).or(d.auxs.first()).copied());
                    if let Some(loc) = target {
                        self.goto(loc.page, loc.sym_id);
                    }
                }
                if let Some((ei, face, format)) = self.panel_ui.export.take() {
                    if let Some(e) = self.session.prj.layout.enclosures.get(ei) {
                        let base: String = e.name.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
                        let what = face.key();
                        match format {
                            NcFormat::Dxf => {
                                let content = openecraft_core::production::nc_dxf(&self.session.prj, ei, face);
                                self.export(&ctx, format!("{what}{base}.dxf"), (tr("DXF drawing"), "dxf"), content);
                            }
                            NcFormat::Dwg => {
                                let content = openecraft_core::dwg::nc_dwg(&self.session.prj, ei, face);
                                self.export_dwg(&ctx, format!("{what}{base}.dwg"), content);
                            }
                            NcFormat::Csv => {
                                let content = openecraft_core::production::nc_csv(&self.session.prj, ei, face);
                                self.export(&ctx, format!("{what}{base}_drilling.csv"), ("CSV", "csv"), content);
                            }
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
        self.remote_window(&ctx);

        // Commit finished edits as one undo step.
        let can_commit = !ctx.input(|i| i.pointer.any_down()) && ctx.memory(|m| m.focused().is_none()) && !self.editor.busy() && !self.panel_ui.busy() && !self.symed.busy() && !self.forms_ui.busy();
        self.session.commit(can_commit);
        self.check_locks(&ctx);

        let title = format!(
            "OpenECraft – {}{}",
            self.session.prj.info.name,
            if self.session.dirty() { " *" } else { "" }
        );
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        self.session.index = idx;
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // Packed: large projects would not fit the browser's local storage as text.
        storage.set_string("project_packed", openecraft_core::archive::pack(&self.session.prj));
        storage.set_string("project", String::new());
        eframe::set_value(storage, "file_name", &self.session.file_name);
        eframe::set_value(storage, "parts_db", &self.parts_db);
        eframe::set_value(storage, "parts_server_url", &self.parts_ui.server.url);
        eframe::set_value(storage, "macros", &self.macro_lib);
        eframe::set_value(storage, "project_database", &self.projects_ui.db);
        eframe::set_value(storage, "project_server_url", &self.remote.url);
        eframe::set_value(storage, "project_server_user", &self.remote.user);
        eframe::set_value(storage, plugins_ui::STORAGE_KEY, &self.plugins.disabled());
        eframe::set_value(storage, openecraft_ui::language::STORAGE_KEY, &openecraft_i18n::lang().code());
    }
}
