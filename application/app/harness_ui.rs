//! Harness window: harnesses with their wires, connectors (pin assignment), lengths
//! and bundle diameter; the 3D topology with protections, fixings and end treatment;
//! KBL, harness interface, wire processing list and 3D exports.

use openecraft_i18n::{tr, tr_fmt};
use super::App;
use openecraft_core::harness::topology::{self, Source, Topology};
use openecraft_core::harness::{self, Fixing, FixingKind, Harness, Protection, ProtectionKind};
use openecraft_core::model::harness::End;
use openecraft_core::model::ReportKind;
use openecraft_core::structure;
use egui::{Color32, RichText};

#[derive(Default)]
pub struct HarnessUi {
    pub open: bool,
    /// Tag of the harness whose connectors are shown.
    pub(super) sel: Option<String>,
    /// Projection of the topology preview: 0 front, 1 top, 2 side.
    pub(super) view: usize,
}

/// What the topology section asks for.
#[derive(Default)]
struct TopologyAction {
    store: bool,
    clear: bool,
}

impl App {
    #[allow(clippy::too_many_lines)]
    pub(super) fn harness_window(&mut self, ctx: &egui::Context) {
        if !self.harness_ui.open {
            return;
        }
        let mut open = true;
        let mut renamed: Vec<(String, String)> = Vec::new();
        let mut remove = None;
        let mut unassign: Option<(String, [String; 2])> = None;
        let mut report = false;
        let mut export: Option<(&'static str, Option<String>)> = None;
        let mut nailboard: Option<String> = None;
        let mut action = TopologyAction::default();
        let structure = structure::page_structure(&self.session.prj.pages[self.session.page]);
        egui::Window::new(tr("Harnesses")).open(&mut open).default_size([900.0, 560.0]).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button(tr("➕ New harness")).on_hover_text(tr("In the structure of the current page")).clicked() {
                    let tag = self.session.prj.next_tag(&structure, "WH");
                    let id = self.session.prj.new_id();
                    self.harness_ui.sel = Some(tag.clone());
                    self.session.prj.harnesses.push(Harness { id, tag, ..Default::default() });
                }
                report = ui.button(tr("📊 Harness data")).on_hover_text(tr("Report with all harness wires")).clicked();
                let any = !self.session.prj.harnesses.is_empty();
                if ui.add_enabled(any, egui::Button::new(tr("⬇ All as KBL (ZIP)…"))).on_hover_text(tr("One KBL file (VDA 4964) per harness")).clicked() {
                    export = Some(("kbl_zip", None));
                }
                if ui.add_enabled(any, egui::Button::new(tr("⬇ Wire processing list (CSV)…"))).on_hover_text(tr("All harness wires in processing order for cutting, stripping and crimping machines")).clicked() {
                    export = Some(("harness_processing", None));
                }
                ui.weak(tr("Assign connections: select the wires between two devices in the schematic › Assign to harness."));
            });
            ui.separator();
            let an = &self.session.analysis;
            // Wires, connectors and bundle diameter per harness.
            let stats: Vec<(usize, usize, f32)> = self
                .session.prj
                .harnesses
                .iter()
                .map(|h| {
                    let ws = harness::wires(&self.session.prj, an, &h.tag);
                    (ws.len(), harness::connectors(&ws).len(), harness::bundle_diameter(&ws))
                })
                .collect();
            egui::Grid::new("harnesses").striped(true).num_columns(8).spacing([8.0, 4.0]).show(ui, |ui| {
                for h in [tr("Harness"), tr("Description"), tr("Field length"), tr("Part"), tr("Wires"), tr("Connectors"), tr("Bundle Ø"), ""] {
                    ui.label(RichText::new(h).strong());
                }
                ui.end_row();
                for (i, h) in self.session.prj.harnesses.iter_mut().enumerate() {
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
            if self.session.prj.harnesses.is_empty() {
                ui.weak(tr("No harnesses defined."));
            }
            // Connectors of the selected harness
            if let Some(tag) = self.harness_ui.sel.clone().filter(|t| self.session.prj.harnesses.iter().any(|h| &h.tag == t)) {
                ui.separator();
                let ws = harness::wires(&self.session.prj, an, &tag);
                let total: f32 = ws.iter().map(|w| w.length).sum();
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(tr_fmt!("{}: {} wires, {} m wire in total", tag, ws.len(), format!("{:.2}", total / 1000.0))).strong());
                    if ui.add_enabled(!ws.is_empty(), egui::Button::new(tr("📐 Nailboard (SVG)…"))).on_hover_text(tr("Formboard drawing of the harness to scale with branch lengths and the pin tables of the connectors")).clicked() {
                        nailboard = Some(tag.clone());
                    }
                    for (name, label, hover) in [
                        ("kbl", tr("⬇ KBL (XML)…"), tr("KBL (VDA 4964) for harness manufacturing: parts, connectors, wires, connections and the 3D topology")),
                        ("harness_json", tr("⬇ Harness interface (JSON)…"), tr("Complete harness data for machines and tools")),
                        ("harness_gltf", tr("⬇ 3D (glTF)…"), tr("The bundle with protections, fixings and connectors in 3D")),
                    ] {
                        if ui.button(label).on_hover_text(hover).clicked() {
                            export = Some((name, Some(tag.clone())));
                        }
                    }
                });
                if ws.iter().any(|w| w.length <= 0.0) {
                    ui.colored_label(Color32::from_rgb(220, 140, 0), tr("Some wires have no length: place their devices in the panel layout or set the field length."));
                }
                let stale = harness::stale(&self.session.prj, an);
                ui.collapsing(tr_fmt!("Connections ({})", self.session.prj.harnesses.iter().find(|h| h.tag == tag).map(|h| h.connections.len()).unwrap_or(0)), |ui| {
                    egui::ScrollArea::vertical().id_salt("hlist").max_height(160.0).show(ui, |ui| {
                        for k in self.session.prj.harnesses.iter().find(|h| h.tag == tag).map(|h| h.connections.clone()).unwrap_or_default() {
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
                let hi = self.session.prj.harnesses.iter().position(|h| h.tag == tag).unwrap_or(0);
                let t = topology::build(&self.session.prj, an, &self.session.prj.harnesses[hi], &ws);
                let problems = topology::problems(&self.session.prj, an, &self.session.prj.harnesses[hi]).len();
                let devices: Vec<String> = harness::connectors(&ws).into_iter().map(|c| c.device).collect();
                let h = &mut self.session.prj.harnesses[hi];
                ui.collapsing(tr("3D topology"), |ui| topology_section(ui, &mut self.harness_ui.view, h, &t, problems, &mut action));
                ui.collapsing(tr_fmt!("Protections and fixings ({})", h.protections.len() + h.fixings.len()), |ui| protections_section(ui, h, &t));
                ui.collapsing(tr("End treatment of the connectors"), |ui| ends_section(ui, h, &devices));
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
            self.session.prj.harnesses.remove(i);
        }
        if let Some((tag, k)) = unassign {
            if let Some(h) = self.session.prj.harnesses.iter_mut().find(|h| h.tag == tag) {
                h.connections.retain(|c| *c != k);
            }
        }
        if let Some(tag) = self.harness_ui.sel.clone() {
            if action.store {
                topology::store(&mut self.session.prj, &self.session.analysis, &tag);
            }
            if action.clear {
                topology::clear(&mut self.session.prj, &tag);
            }
        }
        if report {
            self.report_kind = ReportKind::HarnessData.into();
            self.show_reports = true;
        }
        if let Some(nb) = nailboard.and_then(|tag| openecraft_core::nailboard::layout(&self.session.prj, &self.session.analysis, &tag)) {
            let name = format!("nailboard_{}.svg", nb.tag.trim_start_matches(['-', '=', '+']).replace(['/', '\\', ' '], "_"));
            self.export(ctx, name, ("SVG", "svg"), openecraft_core::nailboard::svg(&nb));
        }
        if let Some((name, tag)) = export {
            if let Some(e) = self.plugins.exporter(name).cloned() {
                let mut args = serde_json::Map::new();
                if let Some(tag) = tag {
                    args.insert("harness".into(), tag.into());
                }
                self.run_exporter_with(ctx, &e, None, args);
            }
        }
        self.harness_ui.open = open;
    }
}

fn source_text(s: Source) -> &'static str {
    match s {
        Source::Stored => tr("Stored and edited in the harness"),
        Source::Layout => tr("Derived from the routes of the panel layout"),
        Source::Generated => tr("Generated around a common node (no connector in the panel layout)"),
    }
}

fn topology_section(ui: &mut egui::Ui, view: &mut usize, h: &mut Harness, t: &Topology, problems: usize, action: &mut TopologyAction) {
    ui.horizontal_wrapped(|ui| {
        ui.label(source_text(t.source));
        ui.label(tr_fmt!("{} nodes, {} segments, {} m bundle", t.nodes.len(), t.segments.len(), format!("{:.2}", t.length() / 1000.0)));
        if t.source == Source::Stored {
            action.clear = ui.button(tr("⟳ Derive again")).on_hover_text(tr("Drops the stored topology")).clicked();
        } else {
            action.store = ui.button(tr("✏ Store for editing")).on_hover_text(tr("Stores the derived topology in the harness, so nodes and segments can be edited")).clicked();
        }
        if problems > 0 {
            ui.colored_label(Color32::from_rgb(220, 140, 0), tr_fmt!("{} problems: see the check results", problems));
        }
    });
    ui.horizontal(|ui| {
        for (i, name) in [tr("Front"), tr("Top"), tr("Side")].into_iter().enumerate() {
            ui.selectable_value(view, i, name);
        }
    });
    preview(ui, *view, t);
    let editable = t.source == Source::Stored;
    egui::ScrollArea::vertical().id_salt("htopo").max_height(200.0).show(ui, |ui| {
        egui::Grid::new("hnodes").striped(true).num_columns(6).show(ui, |ui| {
            for head in [tr("Node"), "x", "y", "z", tr("Bend radius"), tr("Connectors")] {
                ui.label(RichText::new(head).strong());
            }
            ui.end_row();
            for (i, n) in t.nodes.iter().enumerate() {
                ui.monospace(&n.id);
                match h.nodes.get_mut(i).filter(|s| editable && s.id == n.id) {
                    Some(s) => {
                        for k in 0..3 {
                            ui.add(egui::DragValue::new(&mut s.pos[k]).speed(1.0).suffix(" mm"));
                        }
                        ui.add(egui::DragValue::new(&mut s.bend_radius).speed(1.0).range(0.0..=10_000.0).suffix(" mm"));
                    }
                    None => {
                        for k in 0..3 {
                            ui.label(format!("{:.0}", n.pos[k]));
                        }
                        ui.label(if n.bend_radius > 0.0 { format!("{:.0} mm", n.bend_radius) } else { "–".into() });
                    }
                }
                ui.label(n.connectors.join(", "));
                ui.end_row();
            }
        });
        ui.separator();
        egui::Grid::new("hsegments").striped(true).num_columns(6).show(ui, |ui| {
            for head in [tr("Segment"), tr("Nodes"), tr("Formboard length"), tr("Centre curve"), tr("Bundle Ø"), tr("Wires")] {
                ui.label(RichText::new(head).strong());
            }
            ui.end_row();
            for (i, s) in t.segments.iter().enumerate() {
                ui.monospace(&s.id);
                ui.label(format!("{} – {}", t.nodes[s.start].id, t.nodes[s.end].id));
                match h.segments.get_mut(i).filter(|x| editable && x.id == s.id) {
                    Some(x) => {
                        ui.add(egui::DragValue::new(&mut x.length).speed(1.0).range(0.0..=1e6).suffix(" mm")).on_hover_text(tr("0: the length of the centre curve"));
                    }
                    None => {
                        ui.label(format!("{:.0} mm", s.length));
                    }
                }
                ui.label(format!("{:.0} mm", s.physical));
                ui.label(format!("{:.1} mm", s.diameter));
                ui.label(s.wires.len().to_string());
                ui.end_row();
            }
        });
    });
}

/// The topology projected onto a plane, scaled to fit.
fn preview(ui: &mut egui::Ui, view: usize, t: &Topology) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width().min(560.0), 220.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);
    let project = |q: [f32; 3]| -> (f32, f32) {
        match view {
            1 => (q[0], -q[2]),
            2 => (q[2], q[1]),
            _ => (q[0], q[1]),
        }
    };
    let all: Vec<(f32, f32)> = t.segments.iter().flat_map(|s| s.points.iter().map(|q| project(*q))).chain(t.nodes.iter().map(|n| project(n.pos))).collect();
    if all.is_empty() {
        return;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for (x, y) in &all {
        x0 = x0.min(*x);
        y0 = y0.min(*y);
        x1 = x1.max(*x);
        y1 = y1.max(*y);
    }
    let inner = rect.shrink(16.0);
    let scale = (inner.width() / (x1 - x0).max(1.0)).min(inner.height() / (y1 - y0).max(1.0));
    let at = |q: [f32; 3]| {
        let (x, y) = project(q);
        egui::pos2(inner.center().x + (x - (x0 + x1) / 2.0) * scale, inner.center().y + (y - (y0 + y1) / 2.0) * scale)
    };
    let ink = ui.visuals().text_color();
    for s in &t.segments {
        let pts: Vec<egui::Pos2> = s.points.iter().map(|q| at(*q)).collect();
        painter.add(egui::Shape::line(pts, egui::Stroke::new((s.diameter * scale).clamp(1.5, 8.0), ink)));
    }
    for n in &t.nodes {
        let p = at(n.pos);
        if n.connectors.is_empty() {
            painter.circle_filled(p, 3.0, ink);
        } else {
            painter.rect_filled(egui::Rect::from_center_size(p, egui::vec2(8.0, 8.0)), 1.0, Color32::from_rgb(200, 150, 40));
            painter.text(p + egui::vec2(6.0, -6.0), egui::Align2::LEFT_BOTTOM, n.connectors.join(", "), egui::FontId::monospace(10.0), ink);
        }
    }
}

fn segment_combo(ui: &mut egui::Ui, salt: (&str, usize), value: &mut String, t: &Topology, all: bool) {
    let shown = if value.is_empty() { tr("All segments").to_string() } else { value.clone() };
    egui::ComboBox::from_id_salt(salt).selected_text(shown).show_ui(ui, |ui| {
        if all {
            ui.selectable_value(value, String::new(), tr("All segments"));
        }
        for s in &t.segments {
            ui.selectable_value(value, s.id.clone(), &s.id);
        }
    });
}

fn next_id(prefix: &str, taken: &[&str]) -> String {
    (1..).map(|i| format!("{prefix}{i}")).find(|id| !taken.contains(&id.as_str())).unwrap_or_default()
}

fn protections_section(ui: &mut egui::Ui, h: &mut Harness, t: &Topology) {
    if t.source != Source::Stored && (!h.protections.is_empty() || !h.fixings.is_empty()) {
        ui.weak(tr("Segment ids of a derived topology may change: store the topology before placing parts on single segments."));
    }
    let mut drop_p = None;
    egui::Grid::new("hprot").striped(true).num_columns(6).show(ui, |ui| {
        for (i, p) in h.protections.iter_mut().enumerate() {
            ui.monospace(&p.id);
            egui::ComboBox::from_id_salt(("hpk", i)).selected_text(p.kind.title()).show_ui(ui, |ui| {
                for k in ProtectionKind::ALL {
                    ui.selectable_value(&mut p.kind, k, k.title());
                }
            });
            ui.add_sized([110.0, 20.0], egui::TextEdit::singleline(&mut p.part).hint_text(tr("part number")));
            segment_combo(ui, ("hps", i), &mut p.segment, t, true);
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut p.start).speed(0.01).range(0.0..=1.0));
                ui.label("…");
                ui.add(egui::DragValue::new(&mut p.end).speed(0.01).range(0.0..=1.0));
            });
            if ui.small_button("🗑").clicked() {
                drop_p = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = drop_p {
        h.protections.remove(i);
    }
    let mut drop_f = None;
    egui::Grid::new("hfix").striped(true).num_columns(6).show(ui, |ui| {
        for (i, f) in h.fixings.iter_mut().enumerate() {
            ui.monospace(&f.id);
            egui::ComboBox::from_id_salt(("hfk", i)).selected_text(f.kind.title()).show_ui(ui, |ui| {
                for k in FixingKind::ALL {
                    ui.selectable_value(&mut f.kind, k, k.title());
                }
            });
            ui.add_sized([110.0, 20.0], egui::TextEdit::singleline(&mut f.part).hint_text(tr("part number")));
            segment_combo(ui, ("hfs", i), &mut f.segment, t, false);
            ui.add(egui::DragValue::new(&mut f.location).speed(0.01).range(0.0..=1.0)).on_hover_text(tr("Location along the segment, 0 … 1"));
            if ui.small_button("🗑").clicked() {
                drop_f = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = drop_f {
        h.fixings.remove(i);
    }
    ui.horizontal(|ui| {
        if ui.button(tr("➕ Protection")).clicked() {
            let id = next_id("P", &h.protections.iter().map(|p| p.id.as_str()).collect::<Vec<_>>());
            h.protections.push(Protection { id, end: 1.0, ..Default::default() });
        }
        if ui.add_enabled(!t.segments.is_empty(), egui::Button::new(tr("➕ Fixing"))).clicked() {
            let id = next_id("F", &h.fixings.iter().map(|f| f.id.as_str()).collect::<Vec<_>>());
            h.fixings.push(Fixing { id, segment: t.segments[0].id.clone(), location: 0.5, ..Default::default() });
        }
    });
}

fn ends_section(ui: &mut egui::Ui, h: &mut Harness, devices: &[String]) {
    egui::Grid::new("hends").striped(true).num_columns(5).show(ui, |ui| {
        for head in [tr("Connector"), tr("Terminal"), tr("Seal"), tr("Cavity plug"), tr("KBL usage")] {
            ui.label(RichText::new(head).strong());
        }
        ui.end_row();
        for d in devices {
            let i = match h.ends.iter().position(|e| e.device == *d && e.pin.is_empty()) {
                Some(i) => i,
                None => {
                    h.ends.push(End { device: d.clone(), ..Default::default() });
                    h.ends.len() - 1
                }
            };
            let e = &mut h.ends[i];
            ui.monospace(d);
            ui.add_sized([120.0, 20.0], egui::TextEdit::singleline(&mut e.terminal).hint_text(tr("from the cross-section")));
            ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(&mut e.seal));
            ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(&mut e.cavity_plug));
            egui::ComboBox::from_id_salt(("husage", d)).selected_text(if e.usage.is_empty() { "–" } else { e.usage.as_str() }).show_ui(ui, |ui| {
                ui.selectable_value(&mut e.usage, String::new(), "–");
                for u in harness::USAGES {
                    ui.selectable_value(&mut e.usage, u.to_string(), u);
                }
            });
            ui.end_row();
        }
    });
    // Entries without any setting are not kept.
    h.ends.retain(|e| !e.pin.is_empty() || !(e.terminal.is_empty() && e.seal.is_empty() && e.cavity_plug.is_empty() && e.usage.is_empty() && e.strip == 0.0));
}
