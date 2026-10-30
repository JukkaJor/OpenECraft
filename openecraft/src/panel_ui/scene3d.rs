//! What the 3D view of the panel layout shows, in view coordinates and independent of
//! the camera: faces with their lit colours, wires and routing lines, the boxes of the
//! equipment for the painting order, labels and outlines. It is built again only when
//! the analysis, the layout or what the view highlights changes; each frame projects it
//! ([`super::view3d`]) or hands it to the graphics card ([`super::gpu3d`]).

use super::routing_ui::RouteRef;
use super::{PanelUi, OUTER_WALLS, WIRE_HIGHLIGHT};
use openecraft_core::analysis::Analysis;
use openecraft_core::model::Project;
use openecraft_core::panel::{self, Face, ItemKind, Opening, Role, SitePose, PLATE_MARGIN, PLATE_Z};
use openecraft_core::parts::vec3::{self, cross, dot, norm, sub, V3};
use egui::Color32;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Layer of the equipment (ducts, busbars, devices), painted in the order of its boxes.
pub(super) const EQUIPMENT: u8 = 5;
/// Layer of what is drawn over everything (the selected wire or route).
pub(super) const ON_TOP: u8 = OUTER_WALLS + 1;

/// A flat face of the view: a triangle of a model or a side of a box.
pub(super) struct SceneFace {
    /// Corners; a triangle repeats its last one.
    pub pts: [V3; 4],
    /// Number of corners: 3 or 4.
    pub n: u8,
    /// Outward normal: a face seen from behind is left out.
    pub normal: V3,
    pub color: Color32,
    /// Painting layer (walls and plate behind the equipment, …).
    pub layer: u8,
    /// Index in [`Scene3d::sides`] for a face whose layer depends on the side it is seen from.
    pub side: Option<usize>,
    /// Index in [`Scene3d::objects`] of the equipment it belongs to.
    pub obj: Option<usize>,
    /// Enclosure and item, for picking.
    pub item: Option<(usize, u64)>,
    /// Outlined (sides of boxes, not triangles of models).
    pub outline: bool,
    /// Lies on another face (a cutout on the plate): drawn in front of it.
    pub decal: bool,
}

/// A wall, door or wall cutout that is painted over the equipment (translucent) when
/// seen from the outside and behind it when seen from the inside.
pub(super) struct SceneSide {
    pub normal: V3,
    pub at: V3,
    /// Layer seen from the inside.
    pub inside: u8,
    /// Left out when seen from the outside and the walls in front are hidden.
    pub hideable: bool,
    /// A cutout lying on its wall.
    pub decal: bool,
}

impl SceneSide {
    pub fn outside(&self, eye: V3) -> bool {
        dot(self.normal, sub(eye, self.at)) > 0.0
    }

    /// Layer of its faces seen from `eye`; `None` when they are left out.
    pub fn layer(&self, eye: V3, hide_front: bool) -> Option<u8> {
        match self.outside(eye) {
            true if hide_front && self.hideable => None,
            true => Some(OUTER_WALLS),
            false => Some(self.inside),
        }
    }
}

/// A line: a wire, cable, tube or routing path segment.
#[derive(Clone)]
pub(super) struct SceneLine {
    pub a: V3,
    pub b: V3,
    pub color: Color32,
    /// Second colour of a two-colour wire, along the middle.
    pub stripe: Option<Color32>,
    /// Least width (pixels).
    pub least: f32,
    /// Outer diameter (mm), drawn in perspective; 0 for none.
    pub diameter: f32,
}

/// A piece of a line among the equipment, cut where it enters and leaves the ducts.
pub(super) struct SceneWire {
    pub line: SceneLine,
    /// Duct (index in [`Scene3d::objects`]) it runs in.
    pub inside: Option<usize>,
}

/// Name of a device on its front face.
pub(super) struct SceneLabel {
    pub at: V3,
    /// Direction its front faces.
    pub normal: V3,
    /// Smaller side of the front (mm).
    pub size: f32,
    pub tag: String,
}

/// The content of the 3D view; see the module documentation.
#[derive(Default)]
pub(super) struct Scene3d {
    /// Tells this scene from every other one (the graphics card keeps its upload).
    pub id: u64,
    /// What it was built from.
    pub(super) analysis: u64,
    pub(super) layout: panel::Layout,
    pub(super) state: u64,
    pub faces: Vec<SceneFace>,
    pub sides: Vec<SceneSide>,
    /// Lines on top of everything.
    pub lines: Vec<SceneLine>,
    /// Lines among the equipment.
    pub wires: Vec<SceneWire>,
    /// Boxes (min, max) of the equipment, and which of them are ducts.
    pub objects: Vec<(V3, V3)>,
    pub ducts: Vec<bool>,
    /// Front edges of the enclosures.
    pub outlines: Vec<(V3, V3)>,
    pub labels: Vec<SceneLabel>,
    /// Branch points of the bundles.
    pub branches: Vec<V3>,
    /// Segments of the wires and routes for picking.
    pub picks: Vec<(V3, V3, RouteRef)>,
    /// Box around everything.
    pub bounds: (V3, V3),
}

static NEXT_SCENE: AtomicU64 = AtomicU64::new(1);

/// Hash of a set whatever the order of its elements.
fn unordered<T: Hash>(items: impl Iterator<Item = T>) -> u64 {
    items
        .map(|x| {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            x.hash(&mut h);
            h.finish()
        })
        .fold(0u64, u64::wrapping_add)
}

fn bounds(pts: &[V3]) -> (V3, V3) {
    let lo = [0, 1, 2].map(|k| pts.iter().map(|q| q[k]).fold(f32::INFINITY, f32::min));
    let hi = [0, 1, 2].map(|k| pts.iter().map(|q| q[k]).fold(f32::NEG_INFINITY, f32::max));
    (lo, hi)
}

impl PanelUi {
    /// The scene of the 3D view of enclosure `ei` showing `shown`: the one built before
    /// while nothing it shows changed (always a new one during a gesture, whose layout
    /// checks are made live).
    pub(super) fn scene_3d(&mut self, prj: &Project, an: &Analysis, ei: usize, shown: &[(usize, SitePose)]) -> Arc<Scene3d> {
        let state = self.scene_state(an, ei, shown);
        if let Some(s) = self.scene.as_ref().filter(|s| !self.busy() && s.analysis == an.id && s.state == state && s.layout == prj.layout) {
            return s.clone();
        }
        let s = Arc::new(self.build_scene(prj, an, ei, shown, state));
        self.scene = Some(s.clone());
        s
    }

    /// Hash of the state of the view the scene depends on.
    fn scene_state(&self, an: &Analysis, ei: usize, shown: &[(usize, SitePose)]) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        ei.hash(&mut h);
        for (k, pose) in shown {
            k.hash(&mut h);
            pose.at.map(f32::to_bits).hash(&mut h);
            pose.angle.to_bits().hash(&mut h);
        }
        self.selection.hash(&mut h);
        self.wire_index(an).hash(&mut h);
        (self.show_wires, self.hide_ceiling, self.door_angle.to_bits()).hash(&mut h);
        (&self.selected_route, self.selected_object).hash(&mut h);
        unordered(self.done_items.iter()).hash(&mut h);
        unordered(self.done_wires.iter()).hash(&mut h);
        h.finish()
    }

    #[allow(clippy::too_many_lines)]
    fn build_scene(&self, prj: &Project, an: &Analysis, ei: usize, shown: &[(usize, SitePose)], state: u64) -> Scene3d {
        let mut s = Scene3d { id: NEXT_SCENE.fetch_add(1, Ordering::Relaxed), analysis: an.id, layout: prj.layout.clone(), state, ..Default::default() };
        let pos = panel::positions(&prj.layout);
        let (sin, cos) = self.door_angle.sin_cos();
        // The doors turn about their hinges on the left front edge (view coordinates).
        let swing_dir = |v: V3| [v[0] * cos - v[2] * sin, v[1], v[0] * sin + v[2] * cos];
        let light = norm([0.35, 0.75, 0.55]);
        let lit = |base: Color32, n: V3| {
            let shade = 0.55 + 0.45 * dot(n, light).max(0.0);
            Color32::from_rgb((base.r() as f32 * shade) as u8, (base.g() as f32 * shade) as u8, (base.b() as f32 * shade) as u8)
        };
        // Wire segments among the equipment, cut at the boxes once all of them are known.
        let mut equipment_wires: Vec<SceneLine> = Vec::new();
        // Whether a box is the object's own (not enlarged by a turn of the door or the
        // enclosure).
        let mut exact: Vec<bool> = Vec::new();
        let shown_wire = |i: usize| shown.iter().any(|(k, _)| an.wiring[i].in_enclosure(*k));
        let selected_wire = self.wire_index(an).filter(|i| shown_wire(*i));
        let wire_ends = selected_wire.map(|i| an.wiring[i].items);
        for &(k, pose) in shown {
            let e = &prj.layout.enclosures[k];
            let swing = |q: V3| {
                let r = swing_dir([q[0], q[1], q[2] - e.depth]);
                [r[0], r[1], r[2] + e.depth]
            };
            let problems = panel::problems_of(&self.findings(prj, an, k));
            #[allow(clippy::too_many_arguments)]
            let add_box = |s: &mut Scene3d, layer: u8, side: Option<usize>, obj: Option<usize>, min: V3, max: V3, base: Color32, item: Option<u64>, door: bool, decal: bool| {
                let c = |x: usize, y: usize, z: usize| [if x == 0 { min[0] } else { max[0] }, if y == 0 { min[1] } else { max[1] }, if z == 0 { min[2] } else { max[2] }];
                let faces: [([usize; 4], V3); 6] = [
                    ([0b000, 0b010, 0b011, 0b001], [-1.0, 0.0, 0.0]),
                    ([0b100, 0b101, 0b111, 0b110], [1.0, 0.0, 0.0]),
                    ([0b000, 0b001, 0b101, 0b100], [0.0, -1.0, 0.0]),
                    ([0b010, 0b110, 0b111, 0b011], [0.0, 1.0, 0.0]),
                    ([0b000, 0b100, 0b110, 0b010], [0.0, 0.0, -1.0]),
                    ([0b001, 0b011, 0b111, 0b101], [0.0, 0.0, 1.0]),
                ];
                for (corners, n) in faces {
                    let mut pts = corners.map(|b| c(b >> 2 & 1, b >> 1 & 1, b & 1));
                    let n = if door {
                        pts = pts.map(swing);
                        swing_dir(n)
                    } else {
                        n
                    };
                    let normal = pose.turn(n);
                    s.faces.push(SceneFace { pts: pts.map(|q| pose.apply(q)), n: 4, normal, color: lit(base, normal), layer, side, obj, item: item.map(|id| (k, id)), outline: true, decal });
                }
            };
            // The enclosure's components: walls, side panels, frame, plinth, mounting
            // plate, door leaf.
            let (w, h, dpt) = (e.width, e.height, e.depth);
            let t = e.wall();
            // Enclosure coordinates (y down) to the 3D view (y up).
            let up = |q: V3| [q[0], h - q[1], q[2]];
            let up_box = |(a, b): (V3, V3)| (up([a[0], b[1], a[2]]), up([b[0], a[1], b[2]]));
            // A wall seen from the outside covers the equipment (drawn last, translucent);
            // seen from the inside it is behind everything.
            let wall_side = |s: &mut Scene3d, face: Face, b: (V3, V3), hideable: bool, decal: bool| {
                let n = panel::outward(e, face);
                let c = [0, 1, 2].map(|k| if n[k] > 0.0 { b.1[k] } else if n[k] < 0.0 { b.0[k] } else { (b.0[k] + b.1[k]) / 2.0 });
                s.sides.push(SceneSide { normal: pose.turn([n[0], -n[1], n[2]]), at: pose.apply(up(c)), inside: 0, hideable, decal });
                Some(s.sides.len() - 1)
            };
            // Door leaf on its hinge: seen from the outside it covers the equipment
            // (translucent), seen from the inside it is behind the door devices.
            let mut door_side = None;
            for c in panel::components(e) {
                let [r, g, b] = openecraft_core::export3d::component_color(c.role);
                let color = Color32::from_rgb(r, g, b);
                for bx in c.boxes {
                    let (min, max) = up_box(bx);
                    match c.role {
                        Role::Door => {
                            let side = *door_side.get_or_insert_with(|| {
                                s.sides.push(SceneSide { normal: pose.turn(swing_dir([0.0, 0.0, 1.0])), at: pose.apply(swing([w / 2.0, h / 2.0, dpt + t])), inside: 3, hideable: false, decal: false });
                                s.sides.len() - 1
                            });
                            add_box(&mut s, 3, Some(side), None, min, max, Color32::from_rgb(190, 194, 200), None, true, false);
                        }
                        // Left out on request: the ceiling (the walls in front go per frame).
                        Role::RoomWall(Face::Roof) if self.hide_ceiling => {}
                        Role::Wall(face) | Role::RoomWall(face) => {
                            let side = wall_side(&mut s, face, bx, true, false);
                            add_box(&mut s, 0, side, None, min, max, color, None, false, false);
                        }
                        Role::GlandPlate => {
                            let side = wall_side(&mut s, Face::Bottom, bx, false, false);
                            add_box(&mut s, 0, side, None, min, max, color, None, false, false);
                        }
                        Role::MountingPlate | Role::Body | Role::RackRails => add_box(&mut s, 1, None, None, min, max, color, None, false, false),
                        Role::Plinth | Role::BayingKit => add_box(&mut s, 0, None, None, min, max, color, None, false, false),
                    }
                }
            }
            let to3 = |x: f32, y: f32, z: f32| [PLATE_MARGIN + x, h - PLATE_MARGIN - y, PLATE_Z + z];
            for it in &e.items {
                // Selected, at the end of the selected wire, with a problem or mounted:
                // the whole item in that colour, its model's own colours otherwise.
                let highlight = if self.selection.contains(&it.id) && k == ei {
                    Some(Color32::from_rgb(255, 150, 40))
                } else if wire_ends.map(|(a, b)| a == it.id || b == it.id).unwrap_or(false) {
                    Some(WIRE_HIGHLIGHT)
                } else if problems.contains(&it.id) {
                    Some(Color32::from_rgb(230, 70, 70))
                } else if self.done_items.contains(&it.id) {
                    Some(Color32::from_rgb(110, 185, 125))
                } else {
                    None
                };
                let base = match highlight {
                    Some(c) => c,
                    None => match it.kind {
                        ItemKind::Rail => Color32::from_rgb(205, 208, 212),
                        ItemKind::Duct => Color32::from_rgb(150, 154, 160),
                        ItemKind::Busbar => Color32::from_rgb(200, 120, 60),
                        ItemKind::Cutout => match it.opening {
                            Opening::Door => Color32::from_rgb(150, 110, 70),
                            Opening::Window => Color32::from_rgb(160, 205, 235),
                            Opening::Cutout | Opening::Penetration | Opening::Entry => Color32::from_rgb(40, 40, 44),
                        },
                        ItemKind::Device => Color32::from_rgb(170, 186, 210),
                        ItemKind::Accessory => Color32::from_rgb(95, 100, 110),
                    },
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
                    let corners: Vec<V3> = (0..8)
                        .map(|c| {
                            let q = [if c & 1 == 0 { min[0] } else { max[0] }, if c & 2 == 0 { min[1] } else { max[1] }, if c & 4 == 0 { min[2] } else { max[2] }];
                            pose.apply(if on_door { swing(q) } else { q })
                        })
                        .collect();
                    s.objects.push(bounds(&corners));
                    s.ducts.push(it.kind == ItemKind::Duct);
                    exact.push(!on_door && (pose.angle.sin() * pose.angle.cos()).abs() < 1e-4);
                    s.objects.len() - 1
                });
                // Devices with a 3D model in their part are drawn with it instead of a box.
                if let Some(painted) = panel::device_mesh_painted(prj, it) {
                    let (tris, colors): (Vec<_>, Vec<_>) = painted.into_iter().unzip();
                    for (tri, own) in panel::place_triangles(e, it, tris).into_iter().zip(colors) {
                        let base = match (highlight, own) {
                            (None, Some([r, g, b])) => Color32::from_rgb(r, g, b),
                            _ => base,
                        };
                        let w = tri.map(up).map(|q| if on_door { swing(q) } else { q });
                        let n = norm(cross(sub(w[1], w[0]), sub(w[2], w[0])));
                        if n == [0.0; 3] {
                            continue; // no area
                        }
                        let normal = pose.turn(n);
                        let w = w.map(|q| pose.apply(q));
                        s.faces.push(SceneFace { pts: [w[0], w[1], w[2], w[2]], n: 3, normal, color: lit(base, normal), layer: EQUIPMENT, side: None, obj, item: Some((k, it.id)), outline: false, decal: false });
                    }
                    continue;
                }
                let (layer, side) = match it.kind {
                    ItemKind::Cutout if it.face.is_wall() => (0, wall_side(&mut s, it.face, (b0, b1), false, true)),
                    ItemKind::Cutout => (1, None),
                    ItemKind::Rail => (2, None),
                    ItemKind::Duct | ItemKind::Busbar | ItemKind::Device | ItemKind::Accessory => (EQUIPMENT, None),
                };
                let decal = it.kind == ItemKind::Cutout && !it.face.is_wall();
                add_box(&mut s, layer, side, obj, min, max, base, Some(it.id), on_door, decal);
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
                let (color, stripe, width) = super::wire_look(&wire.color, selected, done, selected_wire.is_some());
                let diameter = openecraft_core::harness::outer_diameter(&wire.cross_section);
                // Door ends run along the door and over the hinge loop; the part on the
                // door swings with it.
                let door_end = |at: V3, route: &Option<panel::Route>, plate: Option<&openecraft_core::geom::P>, z: f32| -> Vec<V3> {
                    let (Some(q), Some(route)) = (plate, route) else { return Vec::new() };
                    let (path, n) = panel::door_path(e, at, route, [PLATE_MARGIN + q.x, PLATE_MARGIN + q.y, PLATE_Z + z]);
                    path.into_iter().enumerate().map(|(i, v)| if i < n { swing(up(v)) } else { up(v) }).collect()
                };
                // Cable passages (line-up coordinates) in this enclosure's view.
                let passage = |q: V3| up([q[0] - pos[k][0], q[1] - pos[k][1], q[2] - pos[k][2]]);
                let paths: Vec<Vec<V3>> = match wire.door {
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
                    let (a, b) = (pose.apply(seg[0]), pose.apply(seg[1]));
                    if selected {
                        s.lines.push(SceneLine { a, b, color, stripe: None, least: width, diameter });
                    } else {
                        equipment_wires.push(SceneLine { a, b, color, stripe, least: width, diameter });
                    }
                    s.picks.push((a, b, RouteRef::Wire(wi)));
                }
            }
            // Enclosure outline (the front of a cabinet; rooms show their walls)
            if !e.is_room() {
                let corners = [[0.0, 0.0, dpt], [w, 0.0, dpt], [w, h, dpt], [0.0, h, dpt]];
                s.outlines.extend((0..4).map(|i| (pose.apply(corners[i]), pose.apply(corners[(i + 1) % 4]))));
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
                s.labels.push(SceneLabel { at: pose.apply(front), normal: pose.turn(normal), size: it.w.min(it.h), tag: it.tag.clone() });
            }
        }
        // 3D routing across the enclosures shown: routing paths, reservations, cables and
        // tubes (and the wires routed in 3D), from layout to view coordinates through the
        // enclosure a point lies in (the nearest one along the line-up), points of a room
        // through the room.
        let to_view = |room: Option<usize>, q: V3| {
            let gap = |k: usize| (pos[k][0] - q[0]).max(q[0] - pos[k][0] - prj.layout.enclosures[k].width).max(0.0);
            let in_room = room.filter(|r| prj.layout.enclosures[*r].is_room()).and_then(|r| shown.iter().find(|s| s.0 == r));
            let (k, pose) = in_room.or_else(|| shown.iter().min_by(|a, b| gap(a.0).total_cmp(&gap(b.0)))).copied().unwrap_or((ei, SitePose::default()));
            pose.apply([q[0] - pos[k][0], prj.layout.enclosures[k].height - (q[1] - pos[k][1]), q[2] - pos[k][2]])
        };
        let keys: Vec<usize> = shown.iter().map(|s| s.0).collect();
        for line in self.routing_lines_3d(prj, an, &keys, &to_view, selected_wire) {
            for seg in line.points.windows(2) {
                let l = SceneLine { a: seg[0], b: seg[1], color: line.color, stripe: line.stripe, least: line.width, diameter: line.diameter };
                if line.on_top {
                    s.lines.push(SceneLine { stripe: None, ..l });
                } else {
                    equipment_wires.push(l);
                }
                if !matches!(line.target, RouteRef::None) {
                    s.picks.push((seg[0], seg[1], line.target.clone()));
                }
            }
        }
        let in_view = |q: V3| keys.iter().any(|&k| q[0] >= pos[k][0] && q[0] <= pos[k][0] + prj.layout.enclosures[k].width);
        if self.show_wires {
            s.branches = an.routing.branches.iter().filter(|b| in_view(b.at)).map(|b| to_view(None, b.at)).collect();
        }
        // Wires, cut where they enter and leave the ducts and the devices: the parts
        // inside a device are not seen.
        let cut: Vec<usize> = (0..s.objects.len()).filter(|&i| s.ducts[i] || exact[i]).collect();
        let cut_boxes: Vec<(V3, V3)> = cut.iter().map(|&i| if s.ducts[i] { (s.objects[i].0.map(|v| v - 0.5), s.objects[i].1.map(|v| v + 0.5)) } else { s.objects[i] }).collect();
        for l in equipment_wires {
            let at = |t: f32| [0, 1, 2].map(|k| l.a[k] + (l.b[k] - l.a[k]) * t);
            for (t0, t1, inside) in vec3::split_by_boxes(l.a, l.b, &cut_boxes) {
                let inside = inside.map(|c| cut[c]);
                if inside.is_some_and(|i| !s.ducts[i]) {
                    continue;
                }
                s.wires.push(SceneWire { line: SceneLine { a: at(t0), b: at(t1), ..l.clone() }, inside });
            }
        }
        let pts: Vec<V3> = s.faces.iter().flat_map(|f| f.pts).chain(s.wires.iter().flat_map(|w| [w.line.a, w.line.b])).chain(s.lines.iter().flat_map(|l| [l.a, l.b])).collect();
        s.bounds = if pts.is_empty() { ([0.0; 3], [0.0; 3]) } else { bounds(&pts) };
        s
    }
}
