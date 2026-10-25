//! View › Workspaces: saving and restoring the arrangement of the editor.

use super::*;
use crate::workspaces::{self, Layout};

impl LeftTab {
    const ALL: [LeftTab; 10] = [
        LeftTab::Pages,
        LeftTab::Devices,
        LeftTab::Connections,
        LeftTab::Cables,
        LeftTab::Layers,
        LeftTab::Check,
        LeftTab::Review,
        LeftTab::Unplaced,
        LeftTab::Plc,
        LeftTab::Strips,
    ];

    fn key(self) -> &'static str {
        match self {
            LeftTab::Pages => "pages",
            LeftTab::Devices => "devices",
            LeftTab::Connections => "connections",
            LeftTab::Cables => "cables",
            LeftTab::Layers => "layers",
            LeftTab::Check => "check",
            LeftTab::Review => "review",
            LeftTab::Unplaced => "unplaced",
            LeftTab::Plc => "plc",
            LeftTab::Strips => "strips",
        }
    }
}

impl Workspace {
    fn key(self) -> &'static str {
        match self {
            Workspace::Schematic => "schematic",
            Workspace::Panel => "panel",
            Workspace::Wiring => "wiring",
        }
    }

    fn from_key(k: &str) -> Option<Workspace> {
        [Workspace::Schematic, Workspace::Panel, Workspace::Wiring].into_iter().find(|w| w.key() == k)
    }
}

impl App {
    /// The windows a workspace remembers, by id.
    fn window_flags(&mut self) -> Vec<(&'static str, &mut bool)> {
        vec![
            ("reports", &mut self.show_reports),
            ("project", &mut self.show_project),
            ("cables", &mut self.show_cables),
            ("wire_numbering", &mut self.show_numbering),
            ("cross_references", &mut self.show_xref),
            ("device_numbering", &mut self.numbering_ui.show_devices),
            ("page_numbering", &mut self.numbering_ui.show_pages),
            ("check_settings", &mut self.check_ui.show_settings),
            ("macros", &mut self.eng.show_macros),
            ("terminal_strips", &mut self.eng.show_strips),
            ("potentials", &mut self.eng.show_potentials),
            ("plc_overview", &mut self.eng.show_plc),
            ("revisions", &mut self.eng.show_revisions),
            ("marking", &mut self.eng.show_marking),
            ("find", &mut self.find_ui.open),
            ("forms", &mut self.forms_ui.open),
            ("report_blocks", &mut self.report_blocks_ui.open),
            ("harnesses", &mut self.harness_ui.open),
            ("field_wiring", &mut self.field_ui.open),
            ("plugs", &mut self.plugs_ui.open),
            ("plc_hardware", &mut self.plc_ui.open),
            ("integration", &mut self.integration_ui.open),
            ("process", &mut self.process_ui.open),
            ("preplanning", &mut self.preplanning_ui.open),
            ("electrical", &mut self.electrical_ui.open),
            ("single_line", &mut self.singleline_ui.open),
            ("languages", &mut self.languages_ui.open),
            ("tables", &mut self.tables_ui.open),
            ("configurator", &mut self.config_ui.open),
            ("compare", &mut self.compare_ui.open),
            ("change_requests", &mut self.requests_ui.open),
            ("structure", &mut self.structure_ui.open),
            ("property_database", &mut self.properties_ui.open),
            ("projects", &mut self.projects_ui.open),
            ("parts", &mut self.parts_ui.open),
            ("symbol_editor", &mut self.symed.open),
        ]
    }

    fn current_layout(&mut self, name: &str) -> Layout {
        let (workspace, left_tab) = (self.workspace.key().to_string(), self.left_tab.key().to_string());
        let windows = workspaces::open_windows(&self.window_flags());
        Layout { name: name.into(), workspace, left_tab, windows }
    }

    fn apply_layout(&mut self, layout: &Layout) {
        if let Some(w) = Workspace::from_key(&layout.workspace) {
            self.workspace = w;
        }
        if let Some(t) = LeftTab::ALL.into_iter().find(|t| t.key() == layout.left_tab) {
            self.left_tab = t;
        }
        workspaces::apply_windows(layout, &mut self.window_flags());
    }

    /// The Workspaces submenu of the View menu.
    pub(super) fn workspaces_menu(&mut self, ui: &mut egui::Ui) {
        let mut apply = None;
        let mut remove = None;
        for l in &self.layouts {
            ui.horizontal(|ui| {
                if ui.button(&l.name).on_hover_text(tr_fmt!("{} windows", l.windows.len())).clicked() {
                    apply = Some(l.clone());
                }
                if ui.small_button("✖").on_hover_text(tr("Delete this workspace")).clicked() {
                    remove = Some(l.name.clone());
                }
            });
        }
        if self.layouts.is_empty() {
            ui.weak(tr("No saved workspaces yet."));
        }
        ui.separator();
        if ui.button(tr("Save current…")).on_hover_text(tr("Saves the workspace, the navigator tab and the open windows")).clicked() {
            self.layout_name = Some(String::new());
            ui.close();
        }
        if let Some(l) = apply {
            self.apply_layout(&l);
            ui.close();
        }
        if let Some(n) = remove {
            workspaces::remove(&mut self.layouts, &n);
        }
    }

    /// The small window that asks for the name of the workspace to save.
    pub(super) fn save_layout_window(&mut self, ctx: &egui::Context) {
        let Some(mut name) = self.layout_name.take() else { return };
        let mut open = true;
        let mut save = false;
        egui::Window::new(tr("Save workspace")).open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
            ui.horizontal(|ui| {
                let r = ui.add(egui::TextEdit::singleline(&mut name).hint_text(tr("name")).desired_width(180.0));
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if name.is_empty() && !r.has_focus() {
                    r.request_focus();
                }
                save = (ui.add_enabled(!name.trim().is_empty(), egui::Button::new(tr("Save"))).clicked() || enter) && !name.trim().is_empty();
            });
        });
        if save {
            let layout = self.current_layout(&name);
            workspaces::save(&mut self.layouts, layout);
        } else if open {
            self.layout_name = Some(name);
        }
    }
}
