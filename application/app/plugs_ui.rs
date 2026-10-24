//! Plugs window: pins of a plug in plug order with both sides, the pin count of
//! the plug parts and unused pins.

use openecraft_i18n::{tr, tr_fmt};
use super::App;
use openecraft_core::model::ReportKind;
use openecraft_core::plugs;
use openecraft_core::symbols::PinKind;
use egui::RichText;

#[derive(Default)]
pub struct PlugsUi {
    pub open: bool,
    plug: String,
}

fn kind_text(kinds: &[PinKind]) -> String {
    let mut v: Vec<&str> = Vec::new();
    for k in kinds {
        let t = match k {
            PinKind::Pair => tr("pin and socket"),
            PinKind::Male => tr("male"),
            PinKind::Female => tr("female"),
        };
        if !v.contains(&t) {
            v.push(t);
        }
    }
    v.join(", ")
}

impl App {
    pub(super) fn plugs_window(&mut self, ctx: &egui::Context) {
        if !self.plugs_ui.open {
            return;
        }
        let tags = plugs::plug_tags(&self.session.prj);
        if !tags.contains(&self.plugs_ui.plug) {
            self.plugs_ui.plug = tags.first().cloned().unwrap_or_default();
        }
        let plug = self.plugs_ui.plug.clone();
        let rows = plugs::rows(&self.session.prj, &plug, &self.session.analysis.netlist, &self.session.analysis.open);
        let capacity = plugs::capacity(&self.session.prj, &plug);
        let parts: Vec<String> = self.session.prj.devices.get(&plug).map(|d| d.parts.iter().map(|r| r.number.clone()).collect()).unwrap_or_default();
        let used = rows.iter().filter(|r| !r.pins.is_empty()).count();
        let mut open = true;
        let (mut sort, mut moved, mut jump, mut diagram) = (false, None, None, false);
        egui::Window::new(tr("Plugs")).open(&mut open).default_size([900.0, 400.0]).show(ctx, |ui| {
            if tags.is_empty() {
                ui.weak(tr("No plugs: place plug pins (symbol group Plugs) in a schematic first."));
                return;
            }
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Plug"));
                egui::ComboBox::from_id_salt("plug").selected_text(&plug).show_ui(ui, |ui| {
                    for t in &tags {
                        ui.selectable_value(&mut self.plugs_ui.plug, t.clone(), t);
                    }
                });
                ui.separator();
                sort = ui.button(tr("Sort by designation")).clicked();
                diagram = ui.button(tr("📊 Plug diagram")).clicked();
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(match capacity {
                    Some(n) => tr_fmt!("{} of {} pins used", used, n),
                    None => tr_fmt!("{} pins used", used),
                });
                ui.separator();
                if parts.is_empty() {
                    ui.weak(tr("No parts: assign the housings in the properties of a pin (parts with the function Plug pin give the pin count)."));
                } else {
                    ui.label(tr_fmt!("Parts: {}", parts.join(", ")));
                }
            });
            ui.separator();
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                egui::Grid::new("plugpins").striped(true).num_columns(8).spacing([10.0, 4.0]).show(ui, |ui| {
                    for h in ["", tr("Pin"), tr("Symbols"), tr("Socket side"), tr("Pin side"), tr("Potential"), tr("Function text"), tr("Location")] {
                        ui.label(RichText::new(h).strong());
                    }
                    ui.end_row();
                    let placed = rows.iter().filter(|r| !r.pins.is_empty()).count();
                    for (i, r) in rows.iter().enumerate() {
                        if r.pins.is_empty() {
                            ui.label("");
                            ui.weak(&r.label);
                            ui.label("");
                            ui.weak(tr("spare"));
                            ui.end_row();
                            continue;
                        }
                        ui.horizontal(|ui| {
                            if ui.add_enabled(i > 0, egui::Button::new("⏶").small()).clicked() {
                                moved = Some((i, true));
                            }
                            if ui.add_enabled(i + 1 < placed, egui::Button::new("⏷").small()).clicked() {
                                moved = Some((i, false));
                            }
                        });
                        ui.label(RichText::new(&r.label).monospace());
                        ui.label(kind_text(&r.pins.iter().map(|p| p.kind).collect::<Vec<_>>()));
                        ui.label(&r.socket);
                        ui.label(&r.pin);
                        ui.label(&r.potential);
                        ui.label(&r.function_text);
                        if ui.link(&r.location).clicked() {
                            jump = r.pins.first().map(|p| (p.page, p.id));
                        }
                        ui.end_row();
                    }
                });
            });
        });
        self.plugs_ui.open = open;
        if sort {
            plugs::sort(&mut self.session.prj, &plug);
        }
        if let Some((i, up)) = moved {
            plugs::move_pin(&mut self.session.prj, &plug, i, up);
        }
        if diagram {
            self.report_kind = ReportKind::PlugDiagram.into();
            self.show_reports = true;
        }
        if let Some((page, id)) = jump {
            self.goto(page, id);
        }
    }
}
