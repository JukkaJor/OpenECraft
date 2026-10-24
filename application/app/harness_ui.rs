//! Harness window: harnesses with their wires, connectors (pin assignment), lengths
//! and bundle diameter; KBL export.

use openecraft_i18n::{tr, tr_fmt};
use super::App;
use crate::harness::{self, Harness};
use crate::model::ReportKind;
use crate::structure;
use egui::{Color32, RichText};

#[derive(Default)]
pub struct HarnessUi {
    pub open: bool,
    /// Tag of the harness whose connectors are shown.
    sel: Option<String>,
}

impl App {
    pub(super) fn harness_window(&mut self, ctx: &egui::Context) {
        if !self.harness_ui.open {
            return;
        }
        let mut open = true;
        let mut renamed: Vec<(String, String)> = Vec::new();
        let mut remove = None;
        let mut unassign: Option<(String, [String; 2])> = None;
        let (mut kbl, mut report) = (false, false);
        let structure = structure::page_structure(&self.prj.pages[self.page]);
        egui::Window::new(tr("Harnesses")).open(&mut open).default_size([900.0, 460.0]).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button(tr("➕ New harness")).on_hover_text(tr("In the structure of the current page")).clicked() {
                    let tag = self.prj.next_tag(&structure, "WH");
                    let id = self.prj.new_id();
                    self.harness_ui.sel = Some(tag.clone());
                    self.prj.harnesses.push(Harness { id, tag, ..Default::default() });
                }
                report = ui.button(tr("📊 Harness data")).on_hover_text(tr("Report with all harness wires")).clicked();
                kbl = ui.add_enabled(!self.prj.harnesses.is_empty(), egui::Button::new(tr("⬇ Export KBL (XML)…"))).on_hover_text(tr("Simplified KBL (VDA 4964) for harness manufacturing")).clicked();
                ui.weak(tr("Assign connections: select the wires between two devices in the schematic › Assign to harness."));
            });
            ui.separator();
            let an = &self.analysis;
            // Wires, connectors and bundle diameter per harness.
            let stats: Vec<(usize, usize, f32)> = self
                .prj
                .harnesses
                .iter()
                .map(|h| {
                    let ws = harness::wires(&self.prj, an, &h.tag);
                    (ws.len(), harness::connectors(&ws).len(), harness::bundle_diameter(&ws))
                })
                .collect();
            egui::Grid::new("harnesses").striped(true).num_columns(8).spacing([8.0, 4.0]).show(ui, |ui| {
                for h in [tr("Harness"), tr("Description"), tr("Field length"), tr("Part"), tr("Wires"), tr("Connectors"), tr("Bundle Ø"), ""] {
                    ui.label(RichText::new(h).strong());
                }
                ui.end_row();
                for (i, h) in self.prj.harnesses.iter_mut().enumerate() {
                    let old = h.tag.clone();
                    let sel = self.harness_ui.sel.as_deref() == Some(old.as_str());
                    ui.horizontal(|ui| {
                        if ui.selectable_label(sel, "▶").on_hover_text(tr("Show connectors")).clicked() {
                            self.harness_ui.sel = Some(old.clone());
                        }
                        ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(&mut h.tag).font(egui::TextStyle::Monospace));
                    });
                    if h.tag != old {
                        renamed.push((old.clone(), h.tag.clone()));
                    }
                    ui.add_sized([180.0, 20.0], egui::TextEdit::singleline(&mut h.description));
                    ui.add(egui::DragValue::new(&mut h.length).speed(0.1).range(0.0..=1000.0).suffix(" m"))
                        .on_hover_text(tr("Length of wires that are not routed in the panel layout"));
                    ui.add_sized([110.0, 20.0], egui::TextEdit::singleline(&mut h.part).hint_text(tr("part number")));
                    let (n, c, dia) = stats[i];
                    ui.label(n.to_string());
                    ui.label(c.to_string());
                    ui.label(format!("{dia:.1} mm"));
                    if ui.small_button("🗑").on_hover_text(tr("Delete harness (its wires are removed from it)")).clicked() {
                        remove = Some(i);
                    }
                    ui.end_row();
                }
            });
            if self.prj.harnesses.is_empty() {
                ui.weak(tr("No harnesses defined."));
            }
            // Connectors of the selected harness
            if let Some(tag) = self.harness_ui.sel.clone().filter(|t| self.prj.harnesses.iter().any(|h| &h.tag == t)) {
                ui.separator();
                let ws = harness::wires(&self.prj, an, &tag);
                let total: f32 = ws.iter().map(|w| w.length).sum();
                ui.label(RichText::new(tr_fmt!("{}: {} wires, {} m wire in total", tag, ws.len(), format!("{:.2}", total / 1000.0))).strong());
                if ws.iter().any(|w| w.length <= 0.0) {
                    ui.colored_label(Color32::from_rgb(220, 140, 0), tr("Some wires have no length: place their devices in the panel layout or set the field length."));
                }
                let stale = harness::stale(&self.prj, an);
                ui.collapsing(tr_fmt!("Connections ({})", self.prj.harnesses.iter().find(|h| h.tag == tag).map(|h| h.connections.len()).unwrap_or(0)), |ui| {
                    egui::ScrollArea::vertical().id_salt("hlist").max_height(160.0).show(ui, |ui| {
                        for k in self.prj.harnesses.iter().find(|h| h.tag == tag).map(|h| h.connections.clone()).unwrap_or_default() {
                            ui.horizontal(|ui| {
                                ui.monospace(format!("{} – {}", k[0], k[1]));
                                if stale.iter().any(|(t, c)| *t == tag && *c == k) {
                                    ui.colored_label(Color32::from_rgb(220, 140, 0), tr("not in the schematic"));
                                }
                                if ui.small_button("🗑").on_hover_text(tr("Remove from the harness")).clicked() {
                                    unassign = Some((tag.clone(), k.clone()));
                                }
                            });
                        }
                    });
                });
                egui::ScrollArea::vertical().id_salt("hconn").max_height(220.0).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for c in harness::connectors(&ws) {
                            ui.group(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(&c.device).monospace().strong());
                                    egui::Grid::new(("hpins", &c.device)).striped(true).show(ui, |ui| {
                                        for (pin, number, other) in &c.pins {
                                            ui.monospace(if pin.is_empty() { "–" } else { pin });
                                            ui.monospace(number);
                                            ui.label(format!("– {other}"));
                                            ui.end_row();
                                        }
                                    });
                                });
                            });
                        }
                    });
                });
            }
        });
        for (old, new) in renamed {
            if self.harness_ui.sel.as_deref() == Some(old.as_str()) {
                self.harness_ui.sel = Some(new);
            }
        }
        if let Some(i) = remove {
            self.prj.harnesses.remove(i);
        }
        if let Some((tag, k)) = unassign {
            if let Some(h) = self.prj.harnesses.iter_mut().find(|h| h.tag == tag) {
                h.connections.retain(|c| *c != k);
            }
        }
        if report {
            self.report_kind = ReportKind::HarnessData;
            self.show_reports = true;
        }
        if kbl {
            let xml = harness::kbl(&self.prj, &self.analysis);
            self.export(ctx, self.suggested_name("kbl"), (tr("KBL harness data (XML)"), "kbl"), xml);
        }
        self.harness_ui.open = open;
    }
}
