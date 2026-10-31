//! 3D routing in the panel layout: routing paths and reservations drawn over the
//! mounting plate, the routed cables and tubes, the routing rules and the freezing of
//! routes for production.

use super::{PanelUi, PATH_COLOR, RESERVE_COLOR, WIRE_HIGHLIGHT};
use openecraft_core::geom::{p, P};
use openecraft_core::model::Project;
use openecraft_core::panel::{PLATE_MARGIN, PLATE_Z};
use openecraft_core::analysis::Analysis;
use openecraft_core::routing3d::{self, Class, PathKind, ReserveMode, Reservation, RouteKind, RoutingPath, ShieldEnds};
use egui::{Color32, Pos2, Rect, RichText, Shape, Stroke};
use openecraft_i18n::{tr, tr_fmt};
use openecraft_ui::icons::{self, Icon};

/// Colour of a class in the drawings.
pub(super) fn class_color(c: Class) -> Color32 {
    match c {
        Class::Power => Color32::from_rgb(225, 80, 40),
        Class::Control => Color32::from_rgb(40, 110, 230),
        Class::Signal => Color32::from_rgb(150, 70, 210),
        Class::Fluid => Color32::from_rgb(30, 170, 200),
    }
}

/// What a line of the 3D routing belongs to (for picking).
#[derive(Clone)]
pub(super) enum RouteRef {
    Item(String),
    Wire(usize),
    None,
}

/// A line of the 3D routing in view coordinates.
pub(super) struct RouteLine {
    pub points: Vec<[f32; 3]>,
    pub color: Color32,
    /// Second colour of a two-colour wire, drawn as a stripe.
    pub stripe: Option<Color32>,
    /// Width in pixels; the least one where the line has a `diameter`.
    pub width: f32,
    /// Outer diameter (mm) of a wire, cable or tube, drawn in perspective; 0 for none.
    pub diameter: f32,
    pub on_top: bool,
    pub target: RouteRef,
}

fn mode_choice(ui: &mut egui::Ui, salt: impl egui::AsIdSalt, mode: &mut ReserveMode) {
    egui::ComboBox::from_id_salt(salt).selected_text(mode.title()).show_ui(ui, |ui| {
        ui.selectable_value(mode, ReserveMode::Keepout, ReserveMode::Keepout.title());
        for c in Class::ALL {
            ui.selectable_value(mode, ReserveMode::For(c), ReserveMode::For(c).title());
        }
    });
}

fn kind_choice(ui: &mut egui::Ui, salt: &str, kind: &mut PathKind) {
    egui::ComboBox::from_id_salt(salt).selected_text(kind.title()).show_ui(ui, |ui| {
        for k in PathKind::ALL {
            ui.selectable_value(kind, k, k.title());
        }
    });
}

/// Colour of the pipe runs of rooms.
pub(super) const PIPE_COLOR: Color32 = Color32::from_rgb(40, 150, 200);

impl PanelUi {
    /// The pipe runs on the face of a room shown, with their names.
    pub(super) fn draw_pipe_runs(&self, painter: &egui::Painter, prj: &Project, ei: usize, to_screen: &dyn Fn(P) -> Pos2) {
        let e = &prj.layout.enclosures[ei];
        for path in &e.paths {
            let Some((face, pts, _)) = openecraft_core::panel::pipe_run_on_face(e, path) else { continue };
            if face != self.face {
                continue;
            }
            let pts: Vec<Pos2> = pts.into_iter().map(to_screen).collect();
            let selected = self.selected_object == Some(path.id);
            let width = (path.width * self.zoom).max(2.0);
            painter.add(Shape::line(pts.clone(), Stroke::new(width, PIPE_COLOR.gamma_multiply(if selected { 0.45 } else { 0.25 }))));
            painter.add(Shape::line(pts.clone(), Stroke::new(if selected { 2.0 } else { 1.0 }, PIPE_COLOR)));
            if let Some(first) = pts.first() {
                painter.text(*first, egui::Align2::LEFT_BOTTOM, &path.name, egui::FontId::proportional(11.0), PIPE_COLOR);
            }
        }
    }

    /// Makes a routing path of the corners drawn. Returns its id.
    pub(super) fn add_routing_path(&mut self, prj: &mut Project, ei: usize) -> u64 {
        let id = prj.new_id();
        let e = &mut prj.layout.enclosures[ei];
        let z = PLATE_Z + self.path_height;
        let points = self.path_points.drain(..).map(|q| [PLATE_MARGIN + q.x, PLATE_MARGIN + q.y, z]).collect();
        let name = format!("P{}", e.paths.len() + 1);
        e.paths.push(RoutingPath { id, name, kind: self.path_kind, points, width: self.path_width, classes: Vec::new(), reserve: 0.0, face: None });
        id
    }

    /// Makes a reservation over the plate between two corners, through the whole
    /// depth. Returns its id.
    pub(super) fn add_reservation(&mut self, prj: &mut Project, ei: usize, a: P, b: P) -> u64 {
        let id = prj.new_id();
        let e = &mut prj.layout.enclosures[ei];
        let (lo, hi) = (p(a.x.min(b.x), a.y.min(b.y)), p(a.x.max(b.x), a.y.max(b.y)));
        let name = format!("R{}", e.reservations.len() + 1);
        e.reservations.push(Reservation {
            id,
            name,
            min: [PLATE_MARGIN + lo.x, PLATE_MARGIN + lo.y, PLATE_Z],
            max: [PLATE_MARGIN + hi.x, PLATE_MARGIN + hi.y, e.depth],
            mode: self.reserve_mode,
        });
        id
    }

    /// Routing paths, reservations and the routed cables and tubes seen on the plate.
    pub(super) fn draw_routing_2d(&self, painter: &egui::Painter, prj: &Project, an: &Analysis, ei: usize, to_screen: &dyn Fn(P) -> Pos2) {
        let e = &prj.layout.enclosures[ei];
        let plate = |q: [f32; 3]| to_screen(p(q[0] - PLATE_MARGIN, q[1] - PLATE_MARGIN));
        for r in &e.reservations {
            let rect = Rect::from_two_pos(plate(r.min), plate(r.max));
            let color = match r.mode {
                ReserveMode::Keepout => RESERVE_COLOR,
                ReserveMode::For(c) => class_color(c),
            };
            let selected = self.selected_object == Some(r.id);
            painter.rect_filled(rect, 0.0, color.gamma_multiply(if selected { 0.3 } else { 0.15 }));
            painter.rect_stroke(rect, 0.0, Stroke::new(if selected { 2.5 } else { 1.0 }, color), egui::StrokeKind::Inside);
            painter.text(rect.left_top() + egui::vec2(4.0, 4.0), egui::Align2::LEFT_TOP, &r.name, egui::FontId::proportional(11.0), color);
        }
        for path in &e.paths {
            let pts: Vec<Pos2> = path.points.iter().map(|q| plate(*q)).collect();
            let selected = self.selected_object == Some(path.id);
            let width = (path.width * self.zoom).max(2.0);
            painter.add(Shape::line(pts.clone(), Stroke::new(width, PATH_COLOR.gamma_multiply(if selected { 0.45 } else { 0.25 }))));
            painter.add(Shape::line(pts.clone(), Stroke::new(if selected { 2.0 } else { 1.0 }, PATH_COLOR)));
            if let Some(first) = pts.first() {
                painter.text(*first, egui::Align2::LEFT_BOTTOM, &path.name, egui::FontId::proportional(11.0), PATH_COLOR);
            }
        }
        if !self.show_wires && self.selected_route.is_none() {
            return;
        }
        let pos = openecraft_core::panel::positions(&prj.layout);
        for it in an.routing.items.iter().filter(|i| i.kind != RouteKind::Wire && i.enclosures.contains(&ei)) {
            let selected = self.selected_route.as_deref() == Some(it.key.as_str());
            if !self.show_wires && !selected {
                continue;
            }
            let (color, width) = if selected { (WIRE_HIGHLIGHT, 3.0) } else { (class_color(it.class), (it.diameter * self.zoom * 0.5).clamp(1.5, 6.0)) };
            // The legs in this line-up, not those through a room or another line-up.
            for points in it.stretches_in_lineup(&prj.layout, ei) {
                let pts: Vec<Pos2> = points.iter().map(|q| plate([q[0] - pos[ei][0], q[1] - pos[ei][1], q[2]])).collect();
                painter.add(Shape::line(pts, Stroke::new(width, color.gamma_multiply(0.8))));
            }
        }
    }

    /// Lines of the 3D routing in view coordinates (`to_view`: layout to view
    /// coordinates, through a room for points of the room): routing paths and reservations of the enclosures shown, the routed
    /// cables and tubes, and the wires when they are routed in 3D.
    pub(super) fn routing_lines_3d(&self, prj: &Project, an: &Analysis, shown: &[usize], to_view: &dyn Fn(Option<usize>, [f32; 3]) -> [f32; 3], selected_wire: Option<usize>) -> Vec<RouteLine> {
        let pos = openecraft_core::panel::positions(&prj.layout);
        let mut out = Vec::new();
        for &k in shown {
            let e = &prj.layout.enclosures[k];
            let g = |q: [f32; 3]| to_view(Some(k), [q[0] + pos[k][0], q[1] + pos[k][1], q[2] + pos[k][2]]);
            for path in &e.paths {
                let selected = self.selected_object == Some(path.id);
                let color = if path.face.is_some() { PIPE_COLOR } else { PATH_COLOR };
                out.push(RouteLine { points: path.points.iter().map(|q| g(*q)).collect(), color, stripe: None, width: if selected { 3.0 } else { 1.5 }, diameter: 0.0, on_top: selected, target: RouteRef::None });
            }
            for r in &e.reservations {
                let color = match r.mode {
                    ReserveMode::Keepout => RESERVE_COLOR,
                    ReserveMode::For(c) => class_color(c),
                };
                let (a, b) = (r.min, r.max);
                let c = |x: usize, y: usize, z: usize| g([[a[0], b[0]][x], [a[1], b[1]][y], [a[2], b[2]][z]]);
                // The twelve edges of the box.
                for (s, t) in [
                    ((0, 0, 0), (1, 0, 0)),
                    ((0, 1, 0), (1, 1, 0)),
                    ((0, 0, 1), (1, 0, 1)),
                    ((0, 1, 1), (1, 1, 1)),
                    ((0, 0, 0), (0, 1, 0)),
                    ((1, 0, 0), (1, 1, 0)),
                    ((0, 0, 1), (0, 1, 1)),
                    ((1, 0, 1), (1, 1, 1)),
                    ((0, 0, 0), (0, 0, 1)),
                    ((1, 0, 0), (1, 0, 1)),
                    ((0, 1, 0), (0, 1, 1)),
                    ((1, 1, 0), (1, 1, 1)),
                ] {
                    out.push(RouteLine { points: vec![c(s.0, s.1, s.2), c(t.0, t.1, t.2)], color, stripe: None, width: 1.0, diameter: 0.0, on_top: false, target: RouteRef::None });
                }
            }
        }
        let wires_3d = prj.layout.routing.wires_3d;
        for it in an.routing.items.iter().filter(|i| i.routed() && i.enclosures.iter().any(|k| shown.contains(k))) {
            // Leg by leg; legs through an enclosure not shown are left out.
            let stretches = it.stretches();
            let last = stretches.len().saturating_sub(1);
            for (s, (leg, points)) in stretches.into_iter().enumerate() {
                if leg.is_some_and(|k| !shown.contains(&k)) {
                    continue;
                }
                // The cores part where the cable comes up to the device at the ends of the
                // first and the last stretch (off its face, so the device does not hide them).
                let mut points = points;
                let mut cores: Vec<[[f32; 3]; 2]> = Vec::new();
                if s == 0 && !it.ends[0].cores.is_empty() && points.len() > 2 {
                    cores.extend(it.ends[0].cores.iter().map(|q| [points[1], *q]));
                    points = &points[1..];
                }
                if s == last && !it.ends[1].cores.is_empty() && points.len() > 2 {
                    let n = points.len();
                    cores.extend(it.ends[1].cores.iter().map(|q| [points[n - 2], *q]));
                    points = &points[..n - 1];
                }
                let cores: Vec<[[f32; 3]; 2]> = cores.into_iter().map(|c| c.map(|q| to_view(leg, q))).collect();
                let points: Vec<[f32; 3]> = points.iter().map(|q| to_view(leg, *q)).collect();
                match (it.kind, it.wire) {
                    (RouteKind::Wire, Some(wi)) if wires_3d => {
                        let selected = selected_wire == Some(wi);
                        let done = self.done_wires.contains(&an.wiring[wi].key());
                        if !self.show_wires && !selected && !done {
                            continue;
                        }
                        let wire = &an.wiring[wi];
                        let (color, stripe, width) = super::wire_look(&wire.color, selected, done, selected_wire.is_some());
                        let diameter = openecraft_core::harness::outer_diameter(&wire.cross_section);
                        out.push(RouteLine { points, color, stripe, width, diameter, on_top: selected, target: RouteRef::Wire(wi) });
                    }
                    (RouteKind::Wire, _) => {}
                    _ => {
                        let selected = self.selected_route.as_deref() == Some(it.key.as_str());
                        if !self.show_wires && !selected {
                            continue;
                        }
                        // In the colour of its class, as thick as it is.
                        let (color, width) = if selected { (WIRE_HIGHLIGHT, 4.0) } else { (class_color(it.class), 2.0) };
                        out.push(RouteLine { points, color, stripe: None, width, diameter: it.diameter, on_top: selected, target: RouteRef::Item(it.key.clone()) });
                        for core in cores {
                            out.push(RouteLine { points: core.to_vec(), color, stripe: None, width: width * 0.5, diameter: it.diameter * 0.5, on_top: selected, target: RouteRef::Item(it.key.clone()) });
                        }
                    }
                }
            }
        }
        out
    }

    /// Properties of the 3D routing of an enclosure: what is routed, the selected
    /// route, routing paths, reservations, rules and freezing for production.
    pub(super) fn routing_section(&mut self, ui: &mut egui::Ui, prj: &mut Project, an: &Analysis, ei: usize) {
        let r = &an.routing;
        let here: Vec<usize> = (0..r.items.len()).filter(|&i| r.items[i].enclosures.contains(&ei) || r.items[i].ends.iter().any(|e| e.enclosure == Some(ei))).collect();
        let open = self.selected_route.is_some() || self.selected_object.is_some();
        egui::CollapsingHeader::new(tr("3D routing")).id_salt("routing3d").default_open(open).show(ui, |ui| {
            let count = |k: RouteKind| here.iter().filter(|&&i| r.items[i].kind == k && r.items[i].routed()).count();
            ui.label(tr_fmt!("{} wires, {} cables and {} tubes routed", count(RouteKind::Wire), count(RouteKind::Cable), count(RouteKind::Tube)));
            let bundles = r.bundles.iter().filter(|b| b.items.iter().any(|i| here.contains(i))).count();
            ui.label(tr_fmt!("{} bundles, {} branch points", bundles, r.branches.len()));
            for &i in here.iter().filter(|&&i| !r.items[i].routed() && r.items[i].kind != RouteKind::Wire) {
                ui.colored_label(Color32::from_rgb(220, 120, 0), format!("{} {}: {}", r.items[i].kind.title(), r.items[i].label, r.items[i].status_text()));
            }
            ui.separator();
            // Cables and tubes of the enclosure.
            ui.label(RichText::new(tr("Cables and tubes")).strong());
            let mut pick = None;
            egui::Grid::new("rroutes").striped(true).num_columns(4).show(ui, |ui| {
                for &i in here.iter().filter(|&&i| r.items[i].kind != RouteKind::Wire) {
                    let it = &r.items[i];
                    let selected = self.selected_route.as_deref() == Some(it.key.as_str());
                    if ui.selectable_label(selected, RichText::new(&it.label).color(class_color(it.class))).clicked() {
                        pick = Some(if selected { None } else { Some(it.key.clone()) });
                    }
                    ui.label(it.class.title());
                    ui.label(if it.routed() { format!("{:.0} mm", it.length) } else { "–".into() });
                    ui.label(it.status_text());
                    ui.end_row();
                }
            });
            if let Some(key) = pick {
                self.selected_route = key;
                self.selected_wire = None;
            }
            ui.separator();
            self.paths_and_reservations(ui, prj, an, ei);
            ui.separator();
            self.production(ui, prj, an);
            ui.separator();
            rules(ui, prj);
        });
    }

    /// Properties of the selected cable or tube (as those of a selected wire): its route,
    /// shield and freezing, the devices at its ends.
    pub(super) fn route_properties(&mut self, ui: &mut egui::Ui, prj: &mut Project, an: &Analysis) {
        let r = &an.routing;
        let Some((idx, it)) = self.selected_route.as_ref().and_then(|k| r.items.iter().enumerate().find(|(_, i)| &i.key == k)) else { return };
        ui.label(RichText::new(format!("{} {}", it.kind.title(), it.label)).strong().color(WIRE_HIGHLIGHT));
        egui::Grid::new("rroute").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
            ui.label(tr("From / to"));
            ui.label(format!("{} – {}", it.ends[0].tag, it.ends[1].tag));
            ui.end_row();
            ui.label(tr("Class"));
            ui.label(it.class.title());
            ui.end_row();
            ui.label(tr("Length"));
            ui.label(tr_fmt!("{} mm (cut {} mm)", format!("{:.0}", it.length), format!("{:.0}", it.cut)));
            ui.end_row();
            ui.label(tr("Free air"));
            ui.label(format!("{:.0} mm", it.free));
            ui.end_row();
            ui.label(tr("Diameter / bends"));
            ui.label(format!("{:.1} mm / {}", it.diameter, it.bends));
            ui.end_row();
            ui.label(tr("Bundles"));
            ui.label(r.bundles_of(idx).len().to_string());
            ui.end_row();
            ui.label(tr("Status"));
            ui.label(it.status_text());
            ui.end_row();
        });
        if let Some(s) = r.shields.iter().find(|s| s.cable == it.key) {
            ui.label(RichText::new(tr("Shield")).strong());
            for e in &s.ends {
                ui.label(format!("{}: {}", e.tag, e.termination.text()));
            }
        }
        if it.kind == RouteKind::Cable {
            if let Some(c) = prj.cables.iter_mut().find(|c| c.tag == it.key) {
                ui.horizontal(|ui| {
                    let mut shielded = routing3d::is_shielded(c);
                    if ui.checkbox(&mut shielded, tr("Shielded")).changed() {
                        c.shielded = Some(shielded);
                    }
                    egui::ComboBox::from_id_salt("shieldends").selected_text(c.shield_ends.title()).show_ui(ui, |ui| {
                        for e in ShieldEnds::ALL {
                            ui.selectable_value(&mut c.shield_ends, e, e.title());
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.label(tr("Outer diameter"));
                    ui.add(egui::DragValue::new(&mut c.diameter).speed(0.1).range(0.0..=100.0).suffix(" mm")).on_hover_text(tr("0 = estimated from cores and cross-section"));
                });
            }
        }
        let frozen = prj.layout.frozen.iter().any(|f| f.key == it.key);
        ui.horizontal(|ui| {
            if frozen {
                if ui.button(tr("Release route")).clicked() {
                    routing3d::release(prj, std::slice::from_ref(&it.key));
                }
            } else if it.routed() && ui.button(tr("Freeze route")).on_hover_text(tr("Keep this route when recalculating, as long as it is still possible")).clicked() {
                routing3d::freeze(prj, r, std::slice::from_ref(&it.key));
            }
            if ui.small_button(tr("Deselect")).clicked() {
                self.selected_route = None;
            }
        });
        ui.separator();
        ui.label(RichText::new(tr("Connected devices")).strong());
        let mut select = None;
        for end in &it.ends {
            let (Some(ei), Some(id)) = (end.enclosure, end.item) else { continue };
            let Some(item) = prj.layout.enclosures.get(ei).and_then(|e| e.item(id)).filter(|i| !i.tag.is_empty()) else { continue };
            ui.horizontal(|ui| {
                if ui.link(RichText::new(&item.tag).monospace()).on_hover_text(tr("Select device")).clicked() {
                    select = Some((ei, id));
                }
                if ui.small_button(tr("Show in schematic")).clicked() {
                    self.goto_schematic = Some(item.tag.clone());
                }
            });
        }
        if let Some((ei, id)) = select {
            self.switch_enclosure(ei);
            self.select_item(id);
        }
    }

    fn paths_and_reservations(&mut self, ui: &mut egui::Ui, prj: &mut Project, an: &Analysis, ei: usize) {
        ui.label(RichText::new(tr("Routing paths")).strong());
        ui.horizontal(|ui| {
            ui.label(tr("New"));
            ui.add(egui::DragValue::new(&mut self.path_height).speed(1.0).range(0.0..=1000.0).prefix("z ").suffix(" mm")).on_hover_text(tr("Height above the mounting plate"));
            ui.add(egui::DragValue::new(&mut self.path_width).speed(1.0).range(5.0..=500.0).suffix(" mm")).on_hover_text(tr("Width"));
            kind_choice(ui, "newpathkind", &mut self.path_kind);
        });
        let e = &mut prj.layout.enclosures[ei];
        let mut remove = None;
        let mut select = None;
        for path in &mut e.paths {
            let selected = self.selected_object == Some(path.id);
            ui.horizontal(|ui| {
                if ui.selectable_label(selected, RichText::new(&path.name).color(PATH_COLOR)).clicked() {
                    select = Some(if selected { None } else { Some(path.id) });
                }
                ui.label(path.kind.title());
                if let Some(u) = an.routing.paths.iter().find(|u| u.enclosure == ei && u.path == path.id) {
                    let full = if u.capacity > 0.0 { u.area / u.capacity * 100.0 } else { 0.0 };
                    let text = tr_fmt!("{} items, {} %", u.items, format!("{full:.0}"));
                    if full > 100.0 {
                        ui.colored_label(Color32::from_rgb(220, 60, 60), text);
                    } else {
                        ui.label(text);
                    }
                }
                if icons::button(ui, Icon::Delete, tr("Delete")).clicked() {
                    remove = Some(path.id);
                }
            });
            if selected {
                egui::Grid::new(("rpath", path.id)).num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                    ui.label(tr("Name"));
                    ui.text_edit_singleline(&mut path.name);
                    ui.end_row();
                    ui.label(tr("For"));
                    kind_choice(ui, "pathkind", &mut path.kind);
                    ui.end_row();
                    ui.label(tr("Width"));
                    ui.add(egui::DragValue::new(&mut path.width).speed(1.0).range(5.0..=500.0).suffix(" mm"));
                    ui.end_row();
                    ui.label(tr("Height above the plate"));
                    let mut z = path.points.first().map(|q| q[2] - PLATE_Z).unwrap_or(0.0);
                    if ui.add(egui::DragValue::new(&mut z).speed(1.0).range(0.0..=2000.0).suffix(" mm")).changed() {
                        path.points.iter_mut().for_each(|q| q[2] = PLATE_Z + z);
                    }
                    ui.end_row();
                    ui.label(tr("Reserve"));
                    ui.add(egui::DragValue::new(&mut path.reserve).speed(1.0).range(0.0..=90.0).suffix(" %")).on_hover_text(tr("Share of the capacity kept free for later additions"));
                    ui.end_row();
                    ui.label(tr("Classes"));
                    ui.horizontal_wrapped(|ui| {
                        for c in Class::ALL {
                            let mut on = path.classes.is_empty() || path.classes.contains(&c);
                            if ui.checkbox(&mut on, c.title()).changed() {
                                let mut all: Vec<Class> = if path.classes.is_empty() { Class::ALL.to_vec() } else { path.classes.clone() };
                                all.retain(|x| *x != c);
                                if on {
                                    all.push(c);
                                    all.sort();
                                }
                                path.classes = if all.len() == Class::ALL.len() { Vec::new() } else { all };
                            }
                        }
                    });
                    ui.end_row();
                    ui.label(tr("Length"));
                    ui.label(format!("{:.0} mm", path.length()));
                    ui.end_row();
                });
            }
        }
        ui.label(RichText::new(tr("Reservations")).strong());
        ui.horizontal(|ui| {
            ui.label(tr("New"));
            mode_choice(ui, "newreserve", &mut self.reserve_mode);
        });
        for res in &mut e.reservations {
            let selected = self.selected_object == Some(res.id);
            ui.horizontal(|ui| {
                if ui.selectable_label(selected, &res.name).clicked() {
                    select = Some(if selected { None } else { Some(res.id) });
                }
                mode_choice(ui, ("reservemode", res.id), &mut res.mode);
                ui.label(format!("{:.0} × {:.0} mm", res.max[0] - res.min[0], res.max[1] - res.min[1]));
                if icons::button(ui, Icon::Delete, tr("Delete")).clicked() {
                    remove = Some(res.id);
                }
            });
            if selected {
                ui.horizontal(|ui| {
                    ui.label(tr("Name"));
                    ui.text_edit_singleline(&mut res.name);
                });
                ui.horizontal(|ui| {
                    ui.label(tr("Depth from / to"));
                    ui.add(egui::DragValue::new(&mut res.min[2]).speed(1.0).range(0.0..=2000.0));
                    ui.add(egui::DragValue::new(&mut res.max[2]).speed(1.0).range(0.0..=2000.0).suffix(" mm"));
                });
            }
        }
        if let Some(id) = remove {
            e.paths.retain(|x| x.id != id);
            e.reservations.retain(|x| x.id != id);
            self.selected_object = None;
        }
        if let Some(s) = select {
            self.selected_object = s;
        }
    }

    /// Freezing routes for production and what changed against the frozen ones.
    fn production(&mut self, ui: &mut egui::Ui, prj: &mut Project, an: &Analysis) {
        ui.label(RichText::new(tr("Production")).strong());
        ui.label(tr_fmt!("{} routes frozen", prj.layout.frozen.len()));
        ui.horizontal(|ui| {
            let freeze = ui.add_enabled(!an.routes_pending, egui::Button::new(tr("Freeze all routes")));
            if freeze.on_hover_text(tr("The recalculation keeps frozen routes as long as they are still possible")).on_disabled_hover_text(tr("Cables and pipes are routed again in the background; until then the 3D view shows their earlier routes.")).clicked() {
                let n = routing3d::freeze(prj, &an.routing, &[]);
                self.status = tr_fmt!("{} routes frozen", n);
            }
            if !prj.layout.frozen.is_empty() && ui.button(tr("Release all")).clicked() {
                routing3d::release(prj, &[]);
            }
        });
        if !an.routing.changes.is_empty() {
            ui.colored_label(Color32::from_rgb(220, 120, 0), tr("Changed against the frozen routes:"));
            egui::Grid::new("rchanges").striped(true).num_columns(2).show(ui, |ui| {
                for c in &an.routing.changes {
                    ui.label(&c.label);
                    ui.label(match c.after {
                        Some(a) => tr_fmt!("{} mm, now {} mm", format!("{:.0}", c.before), format!("{a:.0}")),
                        None => tr_fmt!("{} mm, removed", format!("{:.0}", c.before)),
                    });
                    ui.end_row();
                }
            });
        }
    }
}

/// The routing rules of the project.
fn rules(ui: &mut egui::Ui, prj: &mut Project) {
    let r = &mut prj.layout.routing;
    egui::CollapsingHeader::new(tr("Routing rules")).id_salt("rrules").show(ui, |ui| {
        ui.checkbox(&mut r.wires_3d, tr("Route wires in 3D")).on_hover_text(tr("Wires take the obstacle-aware 3D routes too, instead of only the cable ducts on the plate"));
        egui::Grid::new("rrulesgrid").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
            let row = |ui: &mut egui::Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, suffix: &str, tip: &str| {
                ui.label(label);
                let resp = ui.add(egui::DragValue::new(v).speed(0.5).range(range).suffix(suffix));
                if !tip.is_empty() {
                    resp.on_hover_text(tip);
                }
                ui.end_row();
            };
            row(ui, tr("Grid"), &mut r.grid, 5.0..=200.0, " mm", tr("Edge length of the routing grid: smaller is finer and slower"));
            row(ui, tr("Clearance"), &mut r.clearance, 0.0..=100.0, " mm", tr("Distance from devices, rails and busbars"));
            row(ui, tr("Free air factor"), &mut r.free_factor, 1.0..=20.0, "", tr("A way through free air costs this many times the way in a duct or routing path"));
            row(ui, tr("Segregation"), &mut r.segregation, 0.0..=1000.0, " mm", tr("Distance of signals and fluids from power"));
            row(ui, tr("Segregation factor"), &mut r.segregation_factor, 1.0..=50.0, "", tr("Cost of running closer than the segregation distance"));
            row(ui, tr("Bend radius wires"), &mut r.bend_wire, 0.0..=30.0, " × d", "");
            row(ui, tr("Bend radius cables"), &mut r.bend_cable, 0.0..=30.0, " × d", "");
            row(ui, tr("Bend radius tubes"), &mut r.bend_tube, 0.0..=30.0, " × d", "");
            row(ui, tr("Cut length step wires"), &mut r.cut_wire, 1.0..=1000.0, " mm", "");
            row(ui, tr("Cut length step cables"), &mut r.cut_cable, 1.0..=1000.0, " mm", "");
            row(ui, tr("Cut length step tubes"), &mut r.cut_tube, 1.0..=1000.0, " mm", "");
            row(ui, tr("Cable end reserve"), &mut r.cable_reserve, 0.0..=5000.0, " mm", tr("Added to each cable end in the enclosure"));
            row(ui, tr("Shield pigtail"), &mut r.pigtail, 0.0..=500.0, " mm", tr("Length of a shield pigtail where there is no shield clamp"));
        });
        if ui.small_button(tr("Default rules")).clicked() {
            *r = Default::default();
        }
    });
}
