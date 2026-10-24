//! Cables window (cable list with type, cores, part and usage) and the wire
//! numbering window.

use openecraft_i18n::{tr, tr_fmt};
use super::App;
use crate::model::*;
use crate::{ops, parts, structure, wirenum};
use egui::RichText;

impl App {
    pub(super) fn cables_window(&mut self, ctx: &egui::Context) {
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
        let mut bus_links: std::collections::HashMap<String, usize> = Default::default();
        for l in self.prj.plc.connections.iter().filter(|l| !l.cable.is_empty()) {
            *bus_links.entry(l.cable.clone()).or_default() += 1;
        }
        let structure = structure::page_structure(&self.prj.pages[self.page]);
        egui::Window::new(tr("Cables")).open(&mut open).default_size([1000.0, 360.0]).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button(tr("➕ New cable")).on_hover_text(tr("In the structure of the current page")).clicked() {
                    ops::add_cable(&mut self.prj, &structure);
                }
                if ui.button(tr("📊 Cable overview")).clicked() {
                    overview = true;
                }
                ui.weak(tr("Assign wires: select them in the schematic › Assign to cable."));
            });
            ui.separator();
            let cable_parts: Vec<&parts::Part> = self.parts_db.parts.iter().filter(|p| p.group == "Cables").collect();
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                egui::Grid::new("cables").striped(true).num_columns(10).spacing([8.0, 4.0]).show(ui, |ui| {
                    for h in [tr("Cable"), tr("Type"), tr("Bus"), tr("Cores"), tr("Cross-section"), tr("Length"), tr("Part"), tr("Description"), tr("Used"), ""] {
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
                        egui::ComboBox::from_id_salt(("cablebus", i)).width(90.0).selected_text(c.bus.map(|b| b.title()).unwrap_or("–")).show_ui(ui, |ui| {
                            ui.selectable_value(&mut c.bus, None, "–").on_hover_text(tr("Ordinary cable"));
                            for b in crate::plc_hw::BusKind::ALL {
                                ui.selectable_value(&mut c.bus, Some(b), b.title());
                            }
                        });
                        ui.add_sized([140.0, 20.0], egui::TextEdit::singleline(&mut c.cores)).on_hover_text(tr("Core designations separated by blanks"));
                        ui.add_sized([70.0, 20.0], egui::TextEdit::singleline(&mut c.cross_section));
                        ui.add(egui::DragValue::new(&mut c.length).speed(0.5).range(0.0..=100000.0).suffix(" m"));
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut c.part).desired_width(120.0));
                            ui.menu_button("…", |ui| {
                                for p in &cable_parts {
                                    if ui.button(format!("{}  {}", p.number, p.description)).clicked() {
                                        ops::apply_cable_part(c, p);
                                        picked.push(p.number.clone());
                                    }
                                }
                                if cable_parts.is_empty() {
                                    ui.weak(tr("No parts of the product group Cables in the parts database."));
                                }
                            });
                        });
                        ui.add_sized([160.0, 20.0], egui::TextEdit::singleline(&mut c.description));
                        let links = bus_links.get(&old).copied().unwrap_or(0);
                        if links > 0 {
                            ui.label(tr_fmt!("{} bus connections", links));
                        } else {
                            let n = used.get(&old).map(|s| s.len()).unwrap_or(0);
                            ui.label(format!("{n}/{}", c.core_list().len()));
                        }
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
            ops::rename_cable(&mut self.prj, &old, &new);
        }
        if let Some(i) = remove {
            ops::remove_cable(&mut self.prj, i);
        }
        for n in picked {
            parts::ensure_in_project(&mut self.prj, &self.parts_db, &n);
        }
        if overview {
            self.report_kind = ReportKind::CableOverview.into();
            self.show_reports = true;
        }
        self.show_cables = open;
    }

    pub(super) fn numbering_window(&mut self, ctx: &egui::Context) {
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
}
