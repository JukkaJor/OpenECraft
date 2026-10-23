//! Fluid and process: hose properties of fluid wires, the hose defaults and the PCT
//! loops of the pre-planning with their engineering status.

use openecraft_i18n::{tr, tr_fmt};
use super::App;
use crate::fluid::Medium;
use crate::model::ReportKind;
use crate::process::{self, PctLoop, Status};
use egui::{Color32, RichText};

#[derive(Default)]
pub struct ProcessUi {
    pub open: bool,
}

fn medium_combo(ui: &mut egui::Ui, id: &str, value: &mut Option<Medium>, default: Medium) {
    let shown = value.map(|m| m.title().to_string()).unwrap_or_else(|| tr_fmt!("{} (default)", default.title()));
    egui::ComboBox::from_id_salt(id).selected_text(shown).show_ui(ui, |ui| {
        ui.selectable_value(value, None, tr_fmt!("{} (default)", default.title()));
        for m in Medium::ALL {
            ui.selectable_value(value, Some(m), m.title());
        }
    });
}

fn status_color(s: Status) -> Color32 {
    match s {
        Status::Planned => Color32::GRAY,
        Status::InProgress => Color32::from_rgb(230, 150, 30),
        Status::Engineered => Color32::from_rgb(50, 170, 80),
    }
}

impl App {
    /// Properties of a wire of a fluid net (hose / pipe); they apply to the whole hose.
    pub(super) fn hose_properties(&mut self, ui: &mut egui::Ui, id: u64) {
        let Some(net) = self.analysis.open.net_of_wire(id).cloned() else { return };
        let (medium, size, color, length) = crate::fluid::props_of(&self.prj, &net);
        ui.label(RichText::new(tr_fmt!("Hose / pipe {}", net.name)).strong());
        let hoses: Vec<String> = crate::fluid::hoses(&self.prj, &self.analysis.open).into_iter().filter(|h| h.net == net.name).map(|h| format!("{} – {}", h.from, h.to)).collect();
        if !hoses.is_empty() {
            ui.label(RichText::new(hoses.join("\n")).monospace());
        }
        ui.separator();
        let number_now = crate::wirenum::number_of(&self.prj, &net);
        let mut number = number_now.clone();
        let first = crate::wirenum::wires_of(&self.prj, &net).first().map(|w| (w.medium, w.cross_section.clone(), w.color.clone(), w.length));
        let (mut m, mut s, mut c, mut l) = first.clone().unwrap_or((None, String::new(), String::new(), 0.0));
        let defaults = self.prj.fluid.clone();
        egui::Grid::new("hoseprops").num_columns(2).show(ui, |ui| {
            ui.label(tr("Hose number"));
            ui.add(egui::TextEdit::singleline(&mut number).hint_text(tr("not numbered")).desired_width(90.0));
            ui.end_row();
            ui.label(tr("Medium"));
            medium_combo(ui, "hmedium", &mut m, defaults.medium);
            ui.end_row();
            ui.label(tr("Nominal size"));
            ui.add(egui::TextEdit::singleline(&mut s).hint_text(&defaults.size).desired_width(90.0)).on_hover_text(tr("Outer diameter (e.g. 8 mm) or DN"));
            ui.end_row();
            ui.label(tr("Colour"));
            ui.add(egui::TextEdit::singleline(&mut c).hint_text(&defaults.color).desired_width(90.0));
            ui.end_row();
            ui.label(tr("Length"));
            ui.add(egui::DragValue::new(&mut l).speed(0.05).range(0.0..=1000.0).suffix(" m"));
            ui.end_row();
        });
        ui.weak(tr_fmt!("Resulting: {} · {} · {} · {} m", medium.title(), size, color, format!("{length:.2}")));
        if first != Some((m, s.clone(), c.clone(), l)) {
            for (pi, wid) in &net.wires {
                if let Some(w) = self.prj.pages[*pi].wires.iter_mut().find(|w| w.id == *wid) {
                    (w.medium, w.cross_section, w.color, w.length) = (m, s.trim().to_string(), c.trim().to_string(), l);
                }
            }
        }
        if number != number_now {
            crate::wirenum::set(&mut self.prj, &net, number.trim(), true);
        }
    }

    pub(super) fn process_window(&mut self, ctx: &egui::Context) {
        if !self.process_ui.open {
            return;
        }
        let idx = crate::index::Index::build(&self.prj);
        let loops = process::loops(&self.prj, &idx);
        let mut open = true;
        let (mut add, mut remove, mut define, mut jump, mut report) = (false, None, None, None, None);
        egui::Window::new(tr("Fluid and process")).open(&mut open).default_size([900.0, 460.0]).show(ctx, |ui| {
            ui.label(RichText::new(tr("Hoses and pipes")).strong());
            ui.weak(tr("Wires between the ports of fluid power and P&ID symbols are hoses / pipes: their own nets, numbered with the wire numbering, listed in the hose list."));
            let f = &mut self.prj.fluid;
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Default medium"));
                let mut m = Some(f.medium);
                egui::ComboBox::from_id_salt("fmedium").selected_text(f.medium.title()).show_ui(ui, |ui| {
                    for x in Medium::ALL {
                        ui.selectable_value(&mut m, Some(x), x.title());
                    }
                });
                f.medium = m.unwrap_or_default();
                ui.label(tr("Nominal size"));
                ui.add(egui::TextEdit::singleline(&mut f.size).desired_width(70.0));
                ui.label(tr("Colour"));
                ui.add(egui::TextEdit::singleline(&mut f.color).desired_width(50.0));
                ui.label(tr("Number prefix"));
                ui.add(egui::TextEdit::singleline(&mut f.prefix).desired_width(40.0));
                if ui.button(tr("📊 Hose list")).clicked() {
                    report = Some(ReportKind::HoseList);
                }
            });
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("PCT loops (pre-planning)")).strong());
                add = ui.button(tr("➕ Loop")).clicked();
                if ui.button(tr("📊 PCT loop overview")).clicked() {
                    report = Some(ReportKind::PctLoops);
                }
            });
            ui.weak(tr("Instruments in the P&ID carry function letters and loop number in their label (TIC 101); their device tag links them with the schematic."));
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                egui::Grid::new("pctloops").striped(true).num_columns(8).spacing([10.0, 4.0]).show(ui, |ui| {
                    for h in [tr("PCT loop"), tr("Description"), tr("Range"), tr("Further devices"), tr("Functions"), tr("Devices"), tr("Status"), ""] {
                        ui.label(RichText::new(h).strong());
                    }
                    ui.end_row();
                    for l in &loops {
                        match self.prj.pct_loops.iter_mut().position(|d| d.name == l.name) {
                            Some(i) => {
                                let d = &mut self.prj.pct_loops[i];
                                ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(60.0));
                                ui.add(egui::TextEdit::singleline(&mut d.description).desired_width(200.0));
                                ui.add(egui::TextEdit::singleline(&mut d.range).desired_width(100.0));
                                let mut devices = d.devices.join(" ");
                                if ui.add(egui::TextEdit::singleline(&mut devices).hint_text("-KF101").desired_width(120.0)).changed() {
                                    d.devices = devices.split_whitespace().map(String::from).collect();
                                }
                            }
                            None => {
                                ui.label(&l.name);
                                if ui.button(tr("Add to the pre-planning")).clicked() {
                                    define = Some(l.name.clone());
                                }
                                ui.label("");
                                ui.label("");
                            }
                        }
                        let funcs = l.functions.join(", ");
                        if l.target.is_some() && ui.link(&funcs).clicked() {
                            jump = l.target;
                        } else if l.target.is_none() {
                            ui.weak(tr("not in the P&ID"));
                        }
                        let devices = l.devices.iter().map(|d| if l.missing.contains(d) { format!("{d}?") } else { d.clone() }).collect::<Vec<_>>().join(", ");
                        ui.label(devices).on_hover_text(tr("? = not in the schematic yet"));
                        ui.colored_label(status_color(l.status), l.status.title());
                        if l.defined && ui.small_button("🗑").on_hover_text(tr("Delete loop")).clicked() {
                            remove = Some(l.name.clone());
                        }
                        ui.end_row();
                    }
                });
            });
        });
        self.process_ui.open = open;
        if add {
            let next = (1..).map(|n| (100 + n).to_string()).find(|n| !loops.iter().any(|l| &l.name == n)).unwrap_or_default();
            self.prj.pct_loops.push(PctLoop { name: next, ..Default::default() });
        }
        if let Some(name) = define {
            self.prj.pct_loops.push(PctLoop { name, ..Default::default() });
        }
        if let Some(name) = remove {
            self.prj.pct_loops.retain(|l| l.name != name);
        }
        if let Some(k) = report {
            self.report_kind = k;
            self.show_reports = true;
        }
        if let Some((page, id)) = jump {
            self.goto(page, id);
        }
    }
}
