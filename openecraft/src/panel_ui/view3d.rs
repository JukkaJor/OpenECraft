//! 3D view of the panel layout: the current enclosure or its whole line-up with walls,
//! door, mounting plate, rails, ducts, busbars, devices and the routed wiring; orbit,
//! pan, zoom and picking.

use openecraft_i18n::{tr, tr_fmt};
use super::routing_ui::RouteRef;
use super::{wheel_delta, PanelUi, DOOR_OPEN_ANGLE, OUTER_WALLS, WIRE_COLOR, WIRE_DONE, WIRE_HIGHLIGHT};
use openecraft_core::geom::{p, P};
use openecraft_core::model::Project;
use openecraft_core::panel::{self, Face, ItemKind, Opening, Role, SitePose, PLATE_MARGIN, PLATE_Z};
use openecraft_core::parts::vec3::{self, cross, dot, norm, sub, Orbit};
use openecraft_core::analysis::Analysis;
use egui::{vec2, Color32, PointerButton, Pos2, Sense, Stroke};

/// Layer of the equipment (ducts, busbars, devices), painted in the order of its boxes.
const EQUIPMENT: u8 = 5;

impl PanelUi {
    /// Opens or closes the door in the 3D view.
    pub fn door_button(&mut self, ui: &mut egui::Ui) {
        let text = if self.door_open { tr("Close door") } else { tr("Open door") };
        if ui.button(text).on_hover_text(tr("Swing the enclosure door open or shut (door devices and their wires move with it)")).clicked() {
            self.door_open = !self.door_open;
        }
    }

    /// 3D view of the current enclosure without the toolbar (wiring assistant).
    pub fn view_3d_current(&mut self, ui: &mut egui::Ui, prj: &mut Project, an: &Analysis) {
        if let Some(ei) = self.enclosure_index(prj) {
            self.view_3d(ui, prj, ei, an);
        }
    }

    /// Enclosures shown in the 3D view with their poses in view coordinates (x right,
    /// y up, z towards the door): all of them at their places in the room, a room with
    /// the cabinets standing in it, or the current one or its whole line-up from the
    /// left with the first one's bottom at y 0.
    pub(super) fn shown_3d(&self, prj: &Project, ei: usize) -> Vec<(usize, SitePose)> {
        if self.show_site {
            return panel::site_poses(prj).into_iter().enumerate().collect();
        }
        if prj.layout.enclosures[ei].is_room() {
            let poses = panel::site_poses(prj);
            let room = poses[ei];
            let local = |p: SitePose| SitePose { at: room.local(p.at), angle: p.angle - room.angle };
            return std::iter::once(ei).chain(panel::cabinets_in(&prj.layout, ei)).map(|k| (k, local(poses[k]))).collect();
        }
        let lu = if self.show_lineup { panel::lineup_of(&prj.layout, ei) } else { vec![ei] };
        let pos = panel::positions(&prj.layout);
        let first = lu.first().copied().unwrap_or(ei);
        let base = pos[first];
        let bottom = base[1] + prj.layout.enclosures[first].height;
        lu.into_iter()
            .map(|k| {
                let o = pos[k];
                (k, SitePose { at: [o[0] - base[0], bottom - o[1] - prj.layout.enclosures[k].height, o[2] - base[2]], angle: 0.0 })
            })
            .collect()
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn view_3d(&mut self, ui: &mut egui::Ui, prj: &mut Project, ei: usize, an: &Analysis) {
        let shown = self.shown_3d(prj, ei);
        let pos = panel::positions(&prj.layout);
        let dark = ui.visuals().dark_mode;
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = resp.rect;
        let target = if self.door_open { DOOR_OPEN_ANGLE } else { 0.0 };
        if (self.door_angle - target).abs() > 1e-3 {
            let step = ui.input(|i| i.stable_dt).min(0.1) * 2.5;
            self.door_angle += (target - self.door_angle).clamp(-step, step);
            ui.ctx().request_repaint();
        }
        let (sin, cos) = self.door_angle.sin_cos();
        // The doors turn about their hinges on the left front edge (view coordinates).
        let swing_dir = |v: [f32; 3]| [v[0] * cos - v[2] * sin, v[1], v[0] * sin + v[2] * cos];
        let cam = self.cam.get_or_insert_with(|| default_cam(prj, &shown));
        // Orbit / pan / zoom
        if resp.dragged_by(PointerButton::Primary) {
            let dl = resp.drag_delta();
            cam.yaw -= dl.x * 0.008;
            cam.pitch = (cam.pitch + dl.y * 0.008).clamp(-1.4, 1.4);
        }
        if resp.dragged_by(PointerButton::Middle) || resp.dragged_by(PointerButton::Secondary) {
            let dl = resp.drag_delta();
            let (_, right, up, _) = cam.basis();
            let s = cam.dist * 0.0015;
            for k in 0..3 {
                cam.target[k] += -right[k] * dl.x * s + up[k] * dl.y * s;
            }
        }
        if resp.hovered() {
            let (wheel, zoom_delta) = ui.input(|i| (wheel_delta(i), i.zoom_delta()));
            let factor = zoom_delta * (wheel * 0.0025).exp();
            cam.dist = (cam.dist / factor).clamp(100.0, 200000.0);
        }
        let (eye_view, right, vup, fwd) = cam.basis();
        let f = rect.height() * 1.3;
        let center = rect.center();
        let project_view = |q: [f32; 3]| -> Option<(Pos2, f32)> {
            let v = sub(q, eye_view);
            let z = dot(v, fwd);
            if z < 10.0 {
                return None;
            }
            Some((center + vec2(dot(v, right) / z * f, -dot(v, vup) / z * f), z))
        };

        painter.rect_filled(rect, 0.0, if dark { Color32::from_rgb(24, 26, 30) } else { Color32::from_rgb(214, 218, 224) });

        // Collect primitives
        enum Prim {
            /// `item`: enclosure and item id for picking.
            Face { pts: Vec<Pos2>, color: Color32, item: Option<(usize, u64)>, outline: bool },
            Line { a: Pos2, b: Pos2, color: Color32, width: f32 },
        }
        type Drawn = (u8, Option<(usize, u8)>, f32, Prim);
        // (layer, object, depth, primitive): walls and plate are always behind the equipment
        // when looking through the open door, so they are drawn in earlier layers. The
        // equipment (ducts, busbars, devices) is painted object by object in the order of
        // their boxes (`paint_order`); the object is (index, part): 0 wires it may hide,
        // 1 its faces, 2 the wires in a duct, 3 wires nothing hides (after the last object).
        let mut prims: Vec<Drawn> = Vec::new();
        // Boxes of the equipment in view coordinates, and which of them are ducts.
        let mut objects: Vec<([f32; 3], [f32; 3])> = Vec::new();
        let mut ducts: Vec<usize> = Vec::new();
        // Whether a box is the object's own (not enlarged by a turn of the door or the
        // enclosure).
        let mut exact: Vec<bool> = Vec::new();
        let bounds = |pts: &[[f32; 3]]| {
            let lo = [0, 1, 2].map(|k| pts.iter().map(|q| q[k]).fold(f32::INFINITY, f32::min));
            let hi = [0, 1, 2].map(|k| pts.iter().map(|q| q[k]).fold(f32::NEG_INFINITY, f32::max));
            (lo, hi)
        };
        // Wire segments among the equipment (view coordinates), placed once its order is known.
        let mut equipment_wires: Vec<([f32; 3], [f32; 3], Color32, f32)> = Vec::new();
        let light_view = norm([0.35, 0.75, 0.55]);
        let shown_wire = |i: usize| shown.iter().any(|(k, _)| an.wiring[i].in_enclosure(*k));
        let selected_wire = self.wire_index(an).filter(|i| shown_wire(*i));
        let wire_ends = selected_wire.map(|i| an.wiring[i].items);
        let mut wire_segs: Vec<(Pos2, Pos2, usize)> = Vec::new();
        let mut outlines: Vec<(Pos2, Pos2)> = Vec::new();
        let mut labels: Vec<(Pos2, f32, &str)> = Vec::new();
        for &(k, pose) in &shown {
            let e = &prj.layout.enclosures[k];
            // Everything of this enclosure in its own view coordinates; the eye and the
            // light too.
            let eye = pose.local(eye_view);
            let light = pose.unturn(light_view);
            let project = |q: [f32; 3]| project_view(pose.apply(q));
            let swing = |q: [f32; 3]| {
                let r = swing_dir([q[0], q[1], q[2] - e.depth]);
                [r[0], r[1], r[2] + e.depth]
            };
            let problems = panel::problem_items(prj, e);
            let add_box = |prims: &mut Vec<Drawn>, layer: u8, obj: Option<usize>, min: [f32; 3], max: [f32; 3], base: Color32, item: Option<u64>, door: bool| {
                let c = |x: usize, y: usize, z: usize| [if x == 0 { min[0] } else { max[0] }, if y == 0 { min[1] } else { max[1] }, if z == 0 { min[2] } else { max[2] }];
                let faces: [([usize; 4], [f32; 3]); 6] = [
                    ([0b000, 0b010, 0b011, 0b001], [-1.0, 0.0, 0.0]),
                    ([0b100, 0b101, 0b111, 0b110], [1.0, 0.0, 0.0]),
                    ([0b000, 0b001, 0b101, 0b100], [0.0, -1.0, 0.0]),
                    ([0b010, 0b110, 0b111, 0b011], [0.0, 1.0, 0.0]),
                    ([0b000, 0b100, 0b110, 0b010], [0.0, 0.0, -1.0]),
                    ([0b001, 0b011, 0b111, 0b101], [0.0, 0.0, 1.0]),
                ];
                for (corners, n) in faces {
                    let mut pts3: Vec<[f32; 3]> = corners.iter().map(|&b| c(b >> 2 & 1, b >> 1 & 1, b & 1)).collect();
                    let n = if door {
                        pts3.iter_mut().for_each(|q| *q = swing(*q));
                        swing_dir(n)
                    } else {
                        n
                    };
                    let fc = [
                        pts3.iter().map(|q| q[0]).sum::<f32>() / 4.0,
                        pts3.iter().map(|q| q[1]).sum::<f32>() / 4.0,
                        pts3.iter().map(|q| q[2]).sum::<f32>() / 4.0,
                    ];
                    if dot(n, sub(eye, fc)) <= 0.0 {
                        continue; // back face
                    }
                    let Some(pr) = pts3.iter().map(|q| project(*q)).collect::<Option<Vec<_>>>() else { continue };
                    let depth = pr.iter().map(|x| x.1).sum::<f32>() / 4.0;
                    let shade = 0.55 + 0.45 * dot(n, light).max(0.0);
                    let mut color = Color32::from_rgb(
                        (base.r() as f32 * shade) as u8,
                        (base.g() as f32 * shade) as u8,
                        (base.b() as f32 * shade) as u8,
                    );
                    if layer == OUTER_WALLS {
                        color = color.gamma_multiply(0.6);
                    }
                    prims.push((layer, obj.map(|o| (o, 1)), depth, Prim::Face { pts: pr.into_iter().map(|x| x.0).collect(), color, item: item.map(|id| (k, id)), outline: true }));
                }
            };
            // The enclosure's components: walls, side panels, frame, plinth, mounting
            // plate, door leaf.
            let (w, h, dpt) = (e.width, e.height, e.depth);
            let t = e.wall();
            // Enclosure coordinates (y down) to the 3D view (y up).
            let up = |q: [f32; 3]| [q[0], h - q[1], q[2]];
            let up_box = |(a, b): ([f32; 3], [f32; 3])| (up([a[0], b[1], a[2]]), up([b[0], a[1], b[2]]));
            // The eye in enclosure coordinates.
            let eye_e = [eye[0], h - eye[1], eye[2]];
            // A wall seen from the outside covers the equipment (drawn last, translucent);
            // seen from the inside it is behind everything.
            let outside_of = |face: Face, b: ([f32; 3], [f32; 3])| {
                let n = panel::outward(e, face);
                let c = [0, 1, 2].map(|k| if n[k] > 0.0 { b.1[k] } else if n[k] < 0.0 { b.0[k] } else { (b.0[k] + b.1[k]) / 2.0 });
                dot(n, sub(eye_e, c)) > 0.0
            };
            let wall_layer = |outside: bool| if outside { OUTER_WALLS } else { 0 };
            let leaf_out = sub(eye, swing([w / 2.0, h / 2.0, dpt + t]));
            for c in panel::components(e) {
                let [r, g, b] = openecraft_core::export3d::component_color(c.role);
                let color = Color32::from_rgb(r, g, b);
                for bx in c.boxes {
                    let (min, max) = up_box(bx);
                    match c.role {
                        // Door leaf on its hinge: seen from the outside it covers the
                        // equipment (translucent), seen from the inside it is behind the
                        // door devices.
                        Role::Door => {
                            let layer = if dot(swing_dir([0.0, 0.0, 1.0]), leaf_out) > 0.0 { OUTER_WALLS } else { 3 };
                            add_box(&mut prims, layer, None, min, max, Color32::from_rgb(190, 194, 200), None, true);
                        }
                        Role::Wall(face) | Role::RoomWall(face) => add_box(&mut prims, wall_layer(outside_of(face, bx)), None, min, max, color, None, false),
                        Role::GlandPlate => add_box(&mut prims, wall_layer(outside_of(Face::Bottom, bx)), None, min, max, color, None, false),
                        Role::MountingPlate | Role::Body => add_box(&mut prims, 1, None, min, max, color, None, false),
                        Role::Plinth | Role::BayingKit => add_box(&mut prims, 0, None, min, max, color, None, false),
                    }
                }
            }
            let to3 = |x: f32, y: f32, z: f32| [PLATE_MARGIN + x, h - PLATE_MARGIN - y, PLATE_Z + z];
            for it in &e.items {
                let base = if self.selection.contains(&it.id) && k == ei {
                    Color32::from_rgb(255, 150, 40)
                } else if wire_ends.map(|(a, b)| a == it.id || b == it.id).unwrap_or(false) {
                    WIRE_HIGHLIGHT
                } else if problems.contains(&it.id) {
                    Color32::from_rgb(230, 70, 70)
                } else if self.done_items.contains(&it.id) {
                    Color32::from_rgb(110, 185, 125)
                } else {
                    match it.kind {
                        ItemKind::Rail => Color32::from_rgb(205, 208, 212),
                        ItemKind::Duct => Color32::from_rgb(150, 154, 160),
                        ItemKind::Busbar => Color32::from_rgb(200, 120, 60),
                        ItemKind::Cutout => match it.opening {
                            Opening::Door => Color32::from_rgb(150, 110, 70),
                            Opening::Window => Color32::from_rgb(160, 205, 235),
                            Opening::Cutout => Color32::from_rgb(40, 40, 44),
                        },
                        ItemKind::Device => Color32::from_rgb(170, 186, 210),
                        ItemKind::Accessory => Color32::from_rgb(95, 100, 110),
                    }
                };
                let on_door = it.face == Face::Door;
                let (b0, b1) = panel::item_box(e, it);
                let (min, max) = if it.kind == ItemKind::Cutout && it.face == Face::Plate {
                    // Dark patch just in front of the plate.
                    (to3(it.x, it.y + it.h, 0.0), to3(it.x + it.w, it.y, 0.3))
                } else if it.kind == ItemKind::Cutout && it.face.is_wall() {
                    // Through the wall, a little proud of both of its surfaces.
                    let n = panel::outward(e, it.face);
                    let (a, b) = ([0, 1, 2].map(|k| b0[k] - n[k].abs() * 0.5), [0, 1, 2].map(|k| b1[k] + n[k].abs() * 0.5));
                    (up([a[0], b[1], a[2]]), up([b[0], a[1], b[2]]))
                } else {
                    (up([b0[0], b1[1], b0[2]]), up([b1[0], b0[1], b1[2]]))
                };
                let obj = (!matches!(it.kind, ItemKind::Cutout | ItemKind::Rail)).then(|| {
                    let corners: Vec<[f32; 3]> = (0..8)
                        .map(|c| {
                            let q = [if c & 1 == 0 { min[0] } else { max[0] }, if c & 2 == 0 { min[1] } else { max[1] }, if c & 4 == 0 { min[2] } else { max[2] }];
                            pose.apply(if on_door { swing(q) } else { q })
                        })
                        .collect();
                    objects.push(bounds(&corners));
                    exact.push(!on_door && (pose.angle.sin() * pose.angle.cos()).abs() < 1e-4);
                    if it.kind == ItemKind::Duct {
                        ducts.push(objects.len() - 1);
                    }
                    objects.len() - 1
                });
                // Devices with a 3D model in their part are drawn with it instead of a box.
                if let Some(tris) = panel::device_mesh(prj, it) {
                    for tri in panel::place_triangles(e, it, tris) {
                        let w = tri.map(up).map(|q| if on_door { swing(q) } else { q });
                        let n = norm(cross(sub(w[1], w[0]), sub(w[2], w[0])));
                        if dot(n, sub(eye, w[0])) <= 0.0 {
                            continue; // back face
                        }
                        let Some(pr) = w.iter().map(|q| project(*q)).collect::<Option<Vec<_>>>() else { continue };
                        let depth = pr.iter().map(|x| x.1).sum::<f32>() / 3.0;
                        let shade = 0.55 + 0.45 * dot(n, light).max(0.0);
                        let color = Color32::from_rgb(
                            (base.r() as f32 * shade) as u8,
                            (base.g() as f32 * shade) as u8,
                            (base.b() as f32 * shade) as u8,
                        );
                        let pts = pr.into_iter().map(|x| x.0).collect();
                        prims.push((EQUIPMENT, obj.map(|o| (o, 1)), depth, Prim::Face { pts, color, item: Some((k, it.id)), outline: false }));
                    }
                    continue;
                }
                let layer = match it.kind {
                    ItemKind::Cutout if it.face.is_wall() => wall_layer(outside_of(it.face, (b0, b1))),
                    ItemKind::Cutout => 1,
                    ItemKind::Rail => 2,
                    ItemKind::Duct | ItemKind::Busbar | ItemKind::Device | ItemKind::Accessory => EQUIPMENT,
                };
                add_box(&mut prims, layer, obj, min, max, base, Some(it.id), on_door);
            }
            // Wires inside the ducts; the selected one is drawn on top of everything.
            // Wires routed in 3D are drawn with the routing below.
            let routed_3d = |w: &panel::RoutedWire| prj.layout.routing.wires_3d && w.door == (None, None);
            for (wi, wire) in an.wiring.iter().enumerate().filter(|(_, wire)| wire.in_enclosure(k) && !routed_3d(wire)) {
                let selected = Some(wi) == selected_wire;
                let done = self.done_wires.contains(&wire.key());
                if !self.show_wires && !selected && !done {
                    continue;
                }
                let (layer, color, width) = if selected {
                    (OUTER_WALLS + 1, WIRE_HIGHLIGHT, 3.5)
                } else if done {
                    (4, if selected_wire.is_some() { WIRE_DONE.gamma_multiply(0.5) } else { WIRE_DONE }, 1.5)
                } else if selected_wire.is_some() {
                    (4, WIRE_COLOR.gamma_multiply(0.3), 1.0)
                } else {
                    (4, WIRE_COLOR, 1.5)
                };
                // Door ends run along the door and over the hinge loop; the part on the
                // door swings with it.
                let door_end = |at: [f32; 3], route: &Option<panel::Route>, plate: Option<&P>, z: f32| -> Vec<[f32; 3]> {
                    let (Some(q), Some(route)) = (plate, route) else { return Vec::new() };
                    let (path, n) = panel::door_path(e, at, route, [PLATE_MARGIN + q.x, PLATE_MARGIN + q.y, PLATE_Z + z]);
                    path.into_iter().enumerate().map(|(i, v)| if i < n { swing(up(v)) } else { up(v) }).collect()
                };
                // Cable passages (line-up coordinates) in this enclosure's view.
                let passage = |q: [f32; 3]| up([q[0] - pos[k][0], q[1] - pos[k][1], q[2] - pos[k][2]]);
                let paths: Vec<Vec<[f32; 3]>> = match wire.door {
                    (Some(a), Some(b)) if wire.on_door() => match &wire.door_routes.0 {
                        Some(r) => vec![panel::door_jumper_path(e, a, b, r).into_iter().map(|v| swing(up(v))).collect()],
                        None => Vec::new(),
                    },
                    (a, b) => wire
                        .stretch_passages()
                        .into_iter()
                        .filter(|s| s.0 == k)
                        .map(|(_, route, entry, exit)| {
                            let z = if route.via_ducts { panel::DUCT_PROFILE.1 * 0.5 } else { 5.0 };
                            let mut v = match (entry, a) {
                                (None, Some(a)) => door_end(a, &wire.door_routes.0, route.points.first(), z),
                                _ => Vec::new(),
                            };
                            v.extend(entry.map(passage));
                            v.extend(route.points.iter().map(|q| to3(q.x, q.y, z)));
                            v.extend(exit.map(passage));
                            if let (None, Some(b)) = (exit, b) {
                                v.extend(door_end(b, &wire.door_routes.1, route.points.last(), z).into_iter().rev());
                            }
                            v
                        })
                        .collect(),
                };
                for seg in paths.iter().flat_map(|p| p.windows(2)) {
                    let (a, b) = (seg[0], seg[1]);
                    if let (Some(pa), Some(pb)) = (project(a), project(b)) {
                        if layer == 4 {
                            equipment_wires.push((pose.apply(a), pose.apply(b), color, width));
                        } else {
                            prims.push((layer, None, (pa.1 + pb.1) / 2.0, Prim::Line { a: pa.0, b: pb.0, color, width }));
                        }
                        wire_segs.push((pa.0, pb.0, wi));
                    }
                }
            }
            // Enclosure outline (the front of a cabinet; rooms show their walls)
            let edges = if e.is_room() { [([0.0; 3], [0.0; 3]); 4] } else { [
                ([0.0, 0.0, dpt], [w, 0.0, dpt]),
                ([w, 0.0, dpt], [w, h, dpt]),
                ([w, h, dpt], [0.0, h, dpt]),
                ([0.0, h, dpt], [0.0, 0.0, dpt]),
            ] };
            for (a, b) in edges.into_iter().filter(|(a, b)| a != b) {
                if let (Some(pa), Some(pb)) = (project(a), project(b)) {
                    outlines.push((pa.0, pb.0));
                }
            }
            // Device labels on the front faces
            for it in e.items.iter().filter(|i| i.kind == ItemKind::Device) {
                let (b0, b1) = panel::item_box(e, it);
                // Door devices show their front on the outside of the door leaf.
                let (front, normal) = if it.face == Face::Door {
                    (swing(up([(b0[0] + b1[0]) / 2.0, (b0[1] + b1[1]) / 2.0, b1[2] + 2.5])), swing_dir([0.0, 0.0, 1.0]))
                } else {
                    // The side facing into the enclosure.
                    let n = panel::face_frame(e, it.face)[3];
                    let q = panel::face_point(e, it.face, [it.x + it.w / 2.0, it.y + it.h / 2.0, it.z + it.d]);
                    (up(q), [n[0], -n[1], n[2]])
                };
                if let Some((q, z)) = project(front) {
                    let size = (it.w.min(it.h) * 0.25 * f / z).clamp(0.0, 18.0);
                    if size >= 7.0 && dot(sub(eye, front), normal) > 0.0 {
                        labels.push((q, size, &it.tag));
                    }
                }
            }
        }
        // 3D routing across the enclosures shown: routing paths, reservations, cables and
        // tubes (and the wires routed in 3D), from layout to view coordinates through the
        // enclosure a point lies in (the nearest one along the line-up).
        let to_view = |q: [f32; 3]| {
            let gap = |k: usize| (pos[k][0] - q[0]).max(q[0] - pos[k][0] - prj.layout.enclosures[k].width).max(0.0);
            let (k, pose) = shown.iter().min_by(|a, b| gap(a.0).total_cmp(&gap(b.0))).copied().unwrap_or((ei, SitePose::default()));
            pose.apply([q[0] - pos[k][0], prj.layout.enclosures[k].height - (q[1] - pos[k][1]), q[2] - pos[k][2]])
        };
        let keys: Vec<usize> = shown.iter().map(|s| s.0).collect();
        let mut route_segs: Vec<(Pos2, Pos2, String)> = Vec::new();
        for line in self.routing_lines_3d(prj, an, &keys, &to_view, selected_wire) {
            for seg in line.points.windows(2) {
                if let (Some(pa), Some(pb)) = (project_view(seg[0]), project_view(seg[1])) {
                    if line.on_top {
                        prims.push((OUTER_WALLS + 1, None, (pa.1 + pb.1) / 2.0, Prim::Line { a: pa.0, b: pb.0, color: line.color, width: line.width }));
                    } else {
                        equipment_wires.push((seg[0], seg[1], line.color, line.width));
                    }
                    match &line.target {
                        RouteRef::Wire(wi) => wire_segs.push((pa.0, pb.0, *wi)),
                        RouteRef::Item(key) => route_segs.push((pa.0, pb.0, key.clone())),
                        RouteRef::None => {}
                    }
                }
            }
        }
        let in_view = |q: [f32; 3]| keys.iter().any(|&k| q[0] >= pos[k][0] && q[0] <= pos[k][0] + prj.layout.enclosures[k].width);
        let branch_points: Vec<Pos2> = an.routing.branches.iter().filter(|b| in_view(b.at)).filter_map(|b| project_view(to_view(b.at)).map(|x| x.0)).collect();
        // Painter's algorithm: far to near; the equipment object by object.
        let order = vec3::paint_order(&objects, eye_view);
        let mut rank = vec![0; objects.len()];
        for (r, &i) in order.iter().enumerate() {
            rank[i] = r;
        }
        // Wires, cut where they enter and leave the ducts and the devices: over the ducts that
        // cover them on the screen (the one they run in and its neighbours), under the
        // devices; the parts inside a device are not seen.
        let mut is_duct = vec![false; objects.len()];
        ducts.iter().for_each(|&d| is_duct[d] = true);
        let cut: Vec<usize> = (0..objects.len()).filter(|&i| is_duct[i] || exact[i]).collect();
        let cut_boxes: Vec<([f32; 3], [f32; 3])> =
            cut.iter().map(|&i| if is_duct[i] { (objects[i].0.map(|v| v - 0.5), objects[i].1.map(|v| v + 0.5)) } else { objects[i] }).collect();
        // Screen rectangles of the objects; `None` when a corner is behind the eye.
        let rects: Vec<Option<egui::Rect>> = objects
            .iter()
            .map(|(lo, hi)| {
                let corners = (0..8).map(|c| project_view([if c & 1 == 0 { lo[0] } else { hi[0] }, if c & 2 == 0 { lo[1] } else { hi[1] }, if c & 4 == 0 { lo[2] } else { hi[2] }]));
                corners.map(|p| p.map(|x| x.0)).collect::<Option<Vec<Pos2>>>().map(|pts| egui::Rect::from_points(&pts))
            })
            .collect();
        for (a, b, color, width) in equipment_wires {
            let at = |t: f32| [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * t);
            for (t0, t1, inside) in vec3::split_by_boxes(a, b, &cut_boxes) {
                let inside = inside.map(|c| cut[c]);
                if inside.is_some_and(|i| !is_duct[i]) {
                    continue;
                }
                let (p0, p1) = (at(t0), at(t1));
                let (Some(pa), Some(pb)) = (project_view(p0), project_view(p1)) else { continue };
                let piece = bounds(&[p0, p1]);
                let on_screen = egui::Rect::from_two_pos(pa.0, pb.0).expand(width);
                let hiders = (0..objects.len())
                    .filter(|&i| rects[i].is_none_or(|r| r.intersects(on_screen)) && vec3::may_hide(&objects[i], &piece, eye_view))
                    .map(|i| (i, is_duct[i]))
                    .chain(inside.map(|d| (d, true)));
                let obj = match vec3::wire_place(hiders, &rank) {
                    Some((i, true)) => Some((i, 2)),
                    Some((i, false)) => Some((i, 0)),
                    None => order.last().map(|&i| (i, 3)),
                };
                prims.push((EQUIPMENT, obj, (pa.1 + pb.1) / 2.0, Prim::Line { a: pa.0, b: pb.0, color, width }));
            }
        }
        let key = |o: &Option<(usize, u8)>| o.map(|(i, part)| (rank[i], part));
        prims.sort_by(|a, b| a.0.cmp(&b.0).then(key(&a.1).cmp(&key(&b.1))).then(b.2.total_cmp(&a.2)));
        let mut pick: Vec<(f32, Vec<Pos2>, (usize, u64))> = Vec::new();
        for (_, _, depth, prim) in &prims {
            match prim {
                Prim::Face { pts, color, item, outline } => {
                    // Mesh triangles get a hairline in their own colour to close the seams.
                    let stroke = if *outline { Stroke::new(0.6, color.gamma_multiply(0.6)) } else { Stroke::new(0.5, *color) };
                    painter.add(openecraft_ui::faces::face(pts.clone(), *color, stroke));
                    if let Some(item) = item {
                        pick.push((*depth, pts.clone(), *item));
                    }
                }
                Prim::Line { a, b, color, width } => {
                    painter.line_segment([*a, *b], Stroke::new(*width, *color));
                }
            }
        }
        for (a, b) in outlines {
            painter.line_segment([a, b], Stroke::new(1.5, Color32::from_rgb(110, 115, 125)));
        }
        for (q, size, tag) in labels {
            painter.text(q, egui::Align2::CENTER_CENTER, tag, egui::FontId::proportional(size), Color32::from_rgb(20, 30, 50));
        }
        if self.show_wires {
            for q in branch_points {
                painter.circle(q, 3.5, Color32::WHITE, Stroke::new(1.5, Color32::from_rgb(60, 60, 60)));
            }
        }
        // Picking: items of a neighbour in the line-up make it the current enclosure.
        if resp.clicked_by(PointerButton::Primary) {
            if let Some(ptr) = resp.interact_pointer_pos() {
                // Faces are in drawing order: the last one under the pointer is visible.
                let hit = pick.iter().rev().find(|(_, pts, _)| point_in_convex(ptr, pts)).map(|x| x.2);
                let on_device = hit.and_then(|(k, id)| prj.layout.enclosures[k].item(id)).map(|i| i.kind == ItemKind::Device).unwrap_or(false);
                let mut candidates: Vec<(usize, f32)> = Vec::new();
                for (a, b, wi) in &wire_segs {
                    let d = openecraft_core::geom::dist_to_segment(p(ptr.x, ptr.y), p(a.x, a.y), p(b.x, b.y));
                    if d <= 6.0 {
                        match candidates.iter_mut().find(|c| c.0 == *wi) {
                            Some(c) => c.1 = c.1.min(d),
                            None => candidates.push((*wi, d)),
                        }
                    }
                }
                candidates.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
                let candidates: Vec<usize> = candidates.into_iter().map(|c| c.0).collect();
                match hit {
                    Some((k, id)) if on_device => {
                        self.enclosure = k;
                        self.select_item(id);
                    }
                    _ if !candidates.is_empty() => {
                        self.pick_wire(an, &candidates);
                        return;
                    }
                    // Cables and tubes of the 3D routing.
                    _ if route_segs.iter().any(|(a, b, _)| openecraft_core::geom::dist_to_segment(p(ptr.x, ptr.y), p(a.x, a.y), p(b.x, b.y)) <= 6.0) => {
                        let near = route_segs.iter().min_by(|x, y| {
                            let d = |s: &(Pos2, Pos2, String)| openecraft_core::geom::dist_to_segment(p(ptr.x, ptr.y), p(s.0.x, s.0.y), p(s.1.x, s.1.y));
                            d(x).total_cmp(&d(y))
                        });
                        if let Some((_, _, key)) = near {
                            self.selected_route = Some(key.clone());
                            self.selected_wire = None;
                            self.selection.clear();
                            self.status = an.routing.item(key).map(|i| format!("{} {}  {:.0} mm", i.kind.title(), i.label, i.length)).unwrap_or_default();
                        }
                        return;
                    }
                    Some((k, id)) => {
                        self.enclosure = k;
                        self.select_item(id);
                    }
                    None => {
                        self.selection.clear();
                        self.selected_wire = None;
                    }
                }
            }
        }
        painter.text(
            rect.left_bottom() + vec2(10.0, -10.0),
            egui::Align2::LEFT_BOTTOM,
            tr("Drag: orbit · right/middle drag: pan · wheel: zoom · click: select device or wire"),
            egui::FontId::proportional(12.0),
            if dark { Color32::GRAY } else { Color32::DARK_GRAY },
        );
        if self.selected_wire.is_none() {
            let e = &prj.layout.enclosures[self.enclosure.min(prj.layout.enclosures.len() - 1)];
            // A room as in the list: width × depth × height; an enclosure width × height × depth.
            self.status = if e.is_room() {
                tr_fmt!("{}  room {} × {} × {}", e.name, format!("{:.0}", e.width), format!("{:.0}", e.depth), format!("{:.0}", e.height))
            } else {
                format!("{}  {:.0} × {:.0} × {:.0} mm", e.name, e.width, e.height, e.depth)
            };
        }
    }
}

/// Camera looking at the enclosures shown (see [`PanelUi::shown_3d`]).
pub(super) fn default_cam(prj: &Project, shown: &[(usize, SitePose)]) -> Orbit {
    let (mut min, mut max) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
    for &(k, pose) in shown {
        let e = &prj.layout.enclosures[k];
        for c in 0..8 {
            let q = pose.apply([if c & 1 == 0 { 0.0 } else { e.width }, if c & 2 == 0 { 0.0 } else { e.height }, if c & 4 == 0 { 0.0 } else { e.depth }]);
            for i in 0..3 {
                min[i] = min[i].min(q[i]);
                max[i] = max[i].max(q[i]);
            }
        }
    }
    if shown.is_empty() {
        (min, max) = ([0.0; 3], [600.0, 800.0, 250.0]);
    }
    let target = [0, 1, 2].map(|i| (min[i] + max[i]) / 2.0);
    let size = (max[0] - min[0]).max(max[1] - min[1]).max(max[2] - min[2]);
    Orbit { yaw: 0.55, pitch: 0.3, dist: size * 1.9, target }
}

fn point_in_convex(q: Pos2, pts: &[Pos2]) -> bool {
    let mut sign = 0.0f32;
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        let c = (b.x - a.x) * (q.y - a.y) - (b.y - a.y) * (q.x - a.x);
        if c.abs() < 1e-6 {
            continue;
        }
        if sign == 0.0 {
            sign = c.signum();
        } else if c.signum() != sign {
            return false;
        }
    }
    true
}
