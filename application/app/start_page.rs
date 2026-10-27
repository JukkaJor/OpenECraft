//! The start page (first start, File › Home, Help › Start page): new and recent
//! projects, templates, the demo, the example projects, the tutorial chapters and
//! what is new.

use super::*;
use crate::commands;

impl App {
    pub(super) fn start_page(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().id_salt("start_page").auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(12.0);
            ui.heading(tr("Welcome to OpenECraft"));
            ui.label(tr("An open electrical CAE editor in the spirit of EPLAN Electric P8."));
            ui.add_space(12.0);
            ui.label(RichText::new(tr("Start")).strong());
            ui.horizontal_wrapped(|ui| {
                for c in [Cmd::NewProject, Cmd::Open, Cmd::OpenDemo, Cmd::NewFromTemplateFile] {
                    let d = commands::def(c);
                    let r = super::backstage::tile(ui, d.icon, tr(d.title));
                    let r = if d.tip.is_empty() { r } else { r.on_hover_text(tr(d.tip)) };
                    if r.clicked() {
                        self.run(&ui.ctx().clone(), c);
                    }
                }
            });
            ui.add_space(12.0);
            self.recent_list(ui);
            ui.add_space(12.0);
            self.example_list(ui);
            ui.add_space(12.0);
            ui.label(RichText::new(tr("Getting started")).strong());
            ui.horizontal_wrapped(|ui| {
                for (n, icon, title) in [
                    (1, Icon::FileNew, tr("A new project")),
                    (3, Icon::InsertCenter, tr("Placing symbols")),
                    (4, Icon::Wire, tr("Wires and potentials")),
                    (5, Icon::Parts, tr("Parts")),
                    (8, Icon::Check, tr("The project check")),
                    (9, Icon::Report, tr("Reports")),
                    (10, Icon::Enclosure, tr("Panel layout")),
                    (11, Icon::Upload, tr("Printing and export")),
                ] {
                    if super::backstage::tile(ui, icon, title).clicked() {
                        self.help_chapter(n);
                    }
                }
            });
            ui.add_space(12.0);
            ui.label(RichText::new(tr("What is new")).strong());
            for line in [
                tr("The ribbon with the File tab replaces the menus; Ctrl+Shift+P finds any command."),
                tr("Every tool is a pane: drag its tab beside the page, into the navigators or out as a window."),
                tr("Workspaces for the schematic, the panel layout, the wiring and mounting assistants, the review and the pre-planning."),
                tr("The Insert center finds symbols, macros, devices and unplaced functions; favorites and recently used ones come first."),
            ] {
                ui.label(format!("•  {line}"));
            }
            ui.add_space(12.0);
            if ui.button(tr("Close the start page")).clicked() {
                crate::dock::close(&mut self.dock, Pane::Tool(ToolPane::Start));
            }
        });
    }

    /// The example projects, simplest first: a click opens one (asking before unsaved
    /// changes are dropped).
    pub(super) fn example_list(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new(tr("Example projects")).strong());
        ui.weak(tr("Each example builds on the one before and shows more of the features."));
        let mut chosen = None;
        for e in &sample::EXAMPLES {
            ui.horizontal(|ui| {
                icons::show(ui, Icon::Book, 16.0);
                if ui.link(tr(e.title)).clicked() {
                    chosen = Some(e.key);
                }
            });
            ui.indent(e.key, |ui| {
                ui.weak(tr(e.description));
            });
        }
        if let Some(key) = chosen {
            self.pending_example = Some(key);
            self.backstage = None;
            self.run(&ui.ctx().clone(), Cmd::OpenExample);
        }
    }
}
