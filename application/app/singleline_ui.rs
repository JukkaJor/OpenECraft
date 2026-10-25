//! Single-line and multi-line engineering: the devices of the single-line diagram and
//! the schematic with their status, and the transformation between both.

use openecraft_i18n::{tr, tr_fmt};
use super::App;
use openecraft_core::singleline::{self, Status};
use egui::{Color32, RichText};

#[derive(Default)]
pub struct SingleLineUi {
    pub open: bool,
    /// Result of the last action.
    message: String,
}

fn status_color(s: Status) -> Color32 {
    match s {
        Status::Planned => Color32::GRAY,
        Status::Engineered => Color32::from_rgb(50, 170, 80),
        Status::NotInSingleLine => crate::style::warning(),
        Status::Mismatch => crate::style::error(),
    }
}

impl App {
    pub(super) fn singleline_pane(&mut self, ui: &mut egui::Ui) {
        let mut jump = None;
        ui.scope(|ui| {
            ui.weak(tr("Devices are planned with single-line symbols and engineered in the multi-line schematic; both share the device tag."));
            let page = self.session.page;
            let can_edit = !self.session.prj.pages[page].locked;
            ui.horizontal_wrapped(|ui| {
                let multi = ui
                    .add_enabled(can_edit, egui::Button::new(tr("Generate schematic from this page")))
                    .on_hover_text(tr("Multi-line power symbols and conductors for the single-line symbols of the current page, on a new page"));
                if multi.clicked() {
                    self.singleline_ui.message = match singleline::to_multiline(&mut self.session.prj, page) {
                        Ok(g) => {
                            self.session.page = g.page;
                            tr_fmt!("{} symbols and {} wires generated on page {}", g.symbols, g.wires, self.session.prj.pages[g.page].name)
                        }
                        Err(e) => e,
                    };
                }
                let single = ui.button(tr("Generate single-line diagram")).on_hover_text(tr("Single-line symbols for the power devices of the schematic that have none yet, on a new overview page"));
                if single.clicked() {
                    self.singleline_ui.message = match singleline::to_singleline(&mut self.session.prj, page) {
                        Ok(g) => {
                            self.session.page = g.page;
                            tr_fmt!("{} symbols and {} wires generated on page {}", g.symbols, g.wires, self.session.prj.pages[g.page].name)
                        }
                        Err(e) => e,
                    };
                }
                if ui.button(tr("Synchronize function texts")).on_hover_text(tr("Copies function texts between both representations where one is empty")).clicked() {
                    let n = singleline::synchronize(&mut self.session.prj);
                    self.singleline_ui.message = tr_fmt!("{} function texts copied", n);
                }
            });
            if !self.singleline_ui.message.is_empty() {
                ui.weak(&self.singleline_ui.message);
            }
            ui.separator();
            let idx = openecraft_core::index::Index::build(&self.session.prj);
            let devices = singleline::devices(&self.session.prj, &idx);
            if devices.is_empty() {
                ui.weak(tr("No single-line symbols and no power devices in the schematic."));
                return;
            }
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                egui::Grid::new("singleline").striped(true).num_columns(4).spacing([12.0, 4.0]).show(ui, |ui| {
                    for h in [tr("Device"), tr("Single-line"), tr("Schematic"), tr("Status")] {
                        ui.label(RichText::new(h).strong());
                    }
                    ui.end_row();
                    for d in &devices {
                        ui.label(&d.tag);
                        for side in [&d.single, &d.multi] {
                            match side {
                                Some((key, at)) => {
                                    let name = self.session.prj.symbol(key).map(|s| tr(s.name)).unwrap_or_default();
                                    let text = format!("{} ({})", name, self.session.prj.pages[at.0].name);
                                    if ui.link(text).clicked() {
                                        jump = Some(*at);
                                    }
                                }
                                None => {
                                    ui.weak("–");
                                }
                            }
                        }
                        ui.colored_label(status_color(d.status), d.status.title());
                        ui.end_row();
                    }
                });
            });
        });
        if let Some((page, id)) = jump {
            self.goto(page, id);
        }
    }
}
