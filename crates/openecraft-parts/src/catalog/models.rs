//! Generated 3D models of devices by what they are: pumps, motors, vessels, heat pumps,
//! compressors, valves and pipe fittings, air treatment, wall-mounted electronics,
//! 19-inch devices and batteries, PV modules, boxes and NH fuse switch disconnectors,
//! painted in their colours, with x to the right, y up and z towards the front; each
//! model fills the part's box. Floor-standing ones are drawn standing on the floor
//! (x width, y depth from the front, z height) and turned upright at the end.

use crate::{Mesh3d, Model2d, Part, Prim2d};

pub(super) const BODY: [u8; 3] = [214, 217, 222];
pub(super) const DARK: [u8; 3] = [64, 68, 76];
pub(super) const BLACK: [u8; 3] = [38, 40, 44];
pub(super) const DISPLAY: [u8; 3] = [30, 46, 66];
pub(super) const STEEL: [u8; 3] = [172, 178, 186];
pub(super) const WHITE: [u8; 3] = [234, 236, 238];
const MOTOR: [u8; 3] = [66, 106, 158];
const PUMP: [u8; 3] = [40, 88, 168];
const RED: [u8; 3] = [192, 50, 42];
const BRASS: [u8; 3] = [188, 150, 74];
const LED: [u8; 3] = [72, 196, 92];
const BOWL: [u8; 3] = [184, 210, 228];
const CELL: [u8; 3] = [28, 42, 94];
const VICTRON: [u8; 3] = [26, 94, 172];
/// Segments of round bodies.
const ROUND: usize = 16;

/// The end of the shaft of a motor or of a machine without its own motor and the
/// direction it points in, in the coordinates of the part's model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shaft {
    pub at: [f32; 3],
    pub dir: [f32; 3],
}

/// What a part is modelled as.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    /// `bare`: without its own motor, the shaft end out of its bearing bracket.
    Pump { bare: bool },
    WallPump,
    Motor,
    Vessel([u8; 3]),
    HeatPump,
    /// `motor`: with its own motor on the receiver.
    Compressor { motor: bool },
    /// A floor-standing unit in a cabinet housing (refrigeration dryer).
    Unit,
    Valve { actuator: bool },
    Inline { dial: bool },
    Coupling,
    /// A pipe through the wall: wall plate and pipe stub.
    Entry,
    Manifold(usize),
    AirUnit { gauge: bool, long: bool },
    WallBox { color: [u8; 3], display: bool },
    Rack(RackKind),
    Pv,
    Housing,
    FuseSwitch,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum RackKind {
    Battery,
    Ports,
    Sockets,
    Vents,
}

fn kind_of(p: &Part) -> Option<Kind> {
    let floor = p.floor_standing();
    let has = |text: &str| p.description.to_lowercase().contains(text);
    let rack = (p.width - crate::mechanics::RACK_WIDTH).abs() < 1.0;
    let bare = has("without motor") || has("bare shaft");
    Some(match p.subgroup.as_str() {
        "Pump" | "Compressor" if !floor => Kind::WallPump,
        "Pump" => Kind::Pump { bare },
        "Compressor" => Kind::Compressor { motor: !bare },
        "Three-phase motor" | "Motor" if floor => Kind::Motor,
        "Vessel" | "Air receiver" | "Buffer tank" if floor => Kind::Vessel(if p.subgroup == "Air receiver" {
            [150, 156, 164]
        } else if has("pressure") || has("expansion") {
            [56, 98, 176]
        } else if has("buffer") || has("insulat") {
            [198, 202, 208]
        } else {
            WHITE
        }),
        "Heat pump" if floor => Kind::HeatPump,
        "Air dryer" if floor => Kind::Unit,
        "Air dryer" => Kind::AirUnit { gauge: false, long: true },
        "Service unit" | "Filter regulator" | "Pressure regulator" => Kind::AirUnit { gauge: true, long: false },
        "Lubricator" | "Filter" | "Intake filter" => Kind::AirUnit { gauge: false, long: false },
        "Shut-off valve" | "Ball valve" | "Motor valve" => Kind::Valve { actuator: has("motor") || has("actuat") },
        "Check valve" => Kind::Inline { dial: false },
        "Flow meter" | "Water meter" => Kind::Inline { dial: true },
        "Quick coupling" => Kind::Coupling,
        "Pipe connection" if p.functions.len() > 2 => Kind::Manifold(p.functions.len() - 1),
        "Pipe connection" => Kind::Entry,
        "PV module" => Kind::Pv,
        "House connection box" | "Junction box" | "Generator junction box" => Kind::Housing,
        "Fuse switch disconnector" => Kind::FuseSwitch,
        "Battery" if rack => Kind::Rack(RackKind::Battery),
        "19-inch device" if has("switch") || has("patch") => Kind::Rack(RackKind::Ports),
        "19-inch device" if has("socket") || has("power distribution") => Kind::Rack(RackKind::Sockets),
        "19-inch device" => Kind::Rack(RackKind::Vents),
        _ if p.manufacturer.starts_with("Victron") && !floor => Kind::WallBox { color: VICTRON, display: matches!(p.subgroup.as_str(), "Inverter / charger" | "Inverter" | "System monitoring" | "Battery monitor") },
        "Inverter / charger" | "Inverter" | "Solar charger" | "Charger" => Kind::WallBox { color: BODY, display: true },
        _ => return None,
    })
}

/// The 3D model and front view of a part that is one of the kinds above, or of a
/// device of the bundled catalogs drawn after its photos ([`brands`]).
pub(super) fn model(p: &Part) -> Option<(Model2d, Mesh3d)> {
    build(p).map(|(m2, m3, _)| (m2, m3))
}

/// The shaft of the generated model of a motor or of a pump or compressor without its
/// own motor.
pub fn shaft(p: &Part) -> Option<Shaft> {
    build(p)?.2
}

#[allow(clippy::too_many_lines)]
fn build(p: &Part) -> Option<(Model2d, Mesh3d, Option<Shaft>)> {
    let (w, h, d) = (p.width, p.height, p.depth);
    if let Some(brand) = brands::brand_of(p) {
        let mut f = brands::Frac::new(w, h, d, p.connection_points_or_default().0);
        brands::draw(&mut f, brand);
        fit(&mut f.m, [w, h, d], &mut []);
        let mut prims = vec![Prim2d::Rect { min: [0.0, 0.0], max: [w, h], fill: false }];
        prims.append(&mut f.prims);
        return Some((Model2d { width: w, height: h, prims }, f.m, None));
    }
    let kind = kind_of(p)?;
    let mut m = Mesh3d::default();
    // Front view: the outline, from above for floor-standing parts.
    let floor = matches!(kind, Kind::Pump { .. } | Kind::Motor | Kind::Vessel(_) | Kind::HeatPump | Kind::Compressor { .. } | Kind::Unit);
    let mut prims = Vec::new();
    let circle = |prims: &mut Vec<Prim2d>, c: [f32; 2], r: f32| prims.push(Prim2d::Circle { c, r, fill: false });
    // The shaft end and a point one millimetre further out.
    let mut shaft: Option<[[f32; 3]; 2]> = None;
    match kind {
        Kind::Pump { .. } if h >= w.max(d) => {
            // Vertical multistage pump: base, pump body with the pipe stubs, motor on top.
            let (cx, cy) = (w / 2.0, d / 2.0);
            let r = 0.36 * w.min(d);
            m.add_painted_box([0.0, 0.0, 0.0], [w, d, 0.06 * h], DARK);
            m.add_cylinder([cx, cy, 0.06 * h], 2, r, 0.5 * h, ROUND, STEEL);
            m.add_cylinder([0.0, cy, 0.16 * h], 0, 0.3 * r, w, ROUND, STEEL);
            for x in [0.0, 0.94 * w] {
                m.add_cylinder([x, cy, 0.16 * h], 0, 0.5 * r, 0.06 * w, ROUND, DARK);
            }
            m.add_cylinder([cx, cy, 0.56 * h], 2, 0.5 * r, 0.02 * h, ROUND, DARK);
            m.add_cylinder([cx, cy, 0.58 * h], 2, 1.15 * r, 0.36 * h, ROUND, MOTOR);
            m.add_frustum([cx, cy, 0.94 * h], 2, 1.15 * r, 0.8 * r, 0.06 * h, ROUND, DARK);
            m.add_painted_box([cx - 0.3 * r, cy + 0.9 * r, 0.7 * h], [cx + 0.3 * r, cy + 1.35 * r, 0.82 * h], DARK);
            circle(&mut prims, [cx, cy], 1.15 * r);
        }
        Kind::Pump { bare: true } => {
            // Bare-shaft pump on its base frame lying along its longer side: the shaft
            // end out of the bearing bracket at the front (left), the volute casing,
            // the suction nozzle at the back and the discharge nozzle on top.
            let along_y = d >= w;
            let (len, across) = if along_y { (d, w) } else { (w, d) };
            let r = (across / 2.0).min(0.4 * h) * 0.85;
            let zc = 0.08 * h + r;
            let at = |s: f32, c: f32, z: f32| if along_y { [c, s, z] } else { [s, c, z] };
            let axis = if along_y { 1 } else { 0 };
            let span = |a: [f32; 3], b: [f32; 3]| ([0, 1, 2].map(|k| a[k].min(b[k])), [0, 1, 2].map(|k| a[k].max(b[k])));
            m.add_painted_box([0.0, 0.0, 0.0], [w, d, 0.08 * h], DARK);
            m.add_cylinder(at(0.0, across / 2.0, zc), axis, 0.15 * r, 0.1 * len, ROUND, STEEL);
            m.add_cylinder(at(0.1 * len, across / 2.0, zc), axis, 0.5 * r, 0.32 * len, ROUND, PUMP);
            let (a, b) = span(at(0.18 * len, 0.3 * across, 0.08 * h), at(0.36 * len, 0.7 * across, zc));
            m.add_painted_box(a, b, PUMP);
            m.add_cylinder(at(0.42 * len, across / 2.0, zc), axis, (1.15 * r).min(across / 2.0), 0.3 * len, ROUND + 8, PUMP);
            m.add_cylinder(at(0.72 * len, across / 2.0, zc), axis, 0.4 * r, 0.28 * len, ROUND, STEEL);
            m.add_cylinder(at(0.57 * len, across / 2.0, zc), 2, 0.3 * r, h - zc, ROUND, STEEL);
            shaft = Some([at(0.0, across / 2.0, zc), at(-1.0, across / 2.0, zc)]);
        }
        Kind::Pump { bare: false } => {
            // Horizontal pump on a base frame: motor, pump casing, stubs.
            let r = (d / 2.0).min(0.4 * h) * 0.85;
            let zc = 0.08 * h + r;
            m.add_painted_box([0.0, 0.0, 0.0], [w, d, 0.08 * h], DARK);
            m.add_cylinder([0.04 * w, d / 2.0, zc], 0, r, 0.55 * w, ROUND, MOTOR);
            m.add_cylinder([0.6 * w, d / 2.0, zc], 0, 0.3 * r, 0.04 * w, ROUND, STEEL);
            m.add_cylinder([0.64 * w, d / 2.0, zc], 0, (1.15 * r).min(d / 2.0), 0.28 * w, ROUND, PUMP);
            m.add_cylinder([0.92 * w, d / 2.0, zc], 0, 0.35 * r, 0.08 * w, ROUND, STEEL);
            m.add_cylinder([0.78 * w, d / 2.0, zc], 2, 0.3 * r, h - zc, ROUND, STEEL);
            prims.push(Prim2d::Rect { min: [0.04 * w, d / 2.0 - r], max: [0.59 * w, d / 2.0 + r], fill: false });
        }
        Kind::WallPump => {
            // Circulating pump in a vertical pipe: pipe, pump housing, motor and its
            // control box out of the wall.
            let (cx, rp) = (w / 2.0, 0.1 * w.min(h));
            m.add_painted_box([0.1 * w, 0.45 * h, 0.0], [0.9 * w, 0.55 * h, 0.12 * d], DARK);
            m.add_cylinder([cx, 0.0, 0.32 * d], 1, rp, h, ROUND, STEEL);
            m.add_cylinder([cx, 0.33 * h, 0.32 * d], 1, 0.3 * w, 0.34 * h, ROUND, PUMP);
            let rm = (0.32 * w.min(h)).min(0.5 * w);
            m.add_cylinder([cx, 0.5 * h, 0.32 * d], 2, rm, 0.5 * d, ROUND, PUMP);
            m.add_cylinder([cx, 0.5 * h, 0.82 * d], 2, 0.85 * rm, 0.18 * d, ROUND, BLACK);
            m.add_painted_box([cx - 0.3 * rm, 0.42 * h, 0.96 * d], [cx + 0.3 * rm, 0.48 * h, d], DISPLAY);
            circle(&mut prims, [cx, 0.5 * h], rm);
        }
        Kind::Motor => {
            // Foot-mounted motor lying along its longer side: feet, body, fan cover,
            // terminal box, shaft.
            let along_y = d >= w;
            let (len, across) = if along_y { (d, w) } else { (w, d) };
            let r = (across / 2.0).min(0.42 * h) * 0.92;
            let zc = 0.06 * h + r;
            let at = |s: f32, c: f32, z: f32| if along_y { [c, s, z] } else { [s, c, z] };
            let axis = if along_y { 1 } else { 0 };
            for s in [0.2 * len, 0.7 * len] {
                let (a, b) = (at(s, 0.08 * across, 0.0), at(s + 0.12 * len, 0.92 * across, 0.06 * h + 0.3 * r));
                m.add_painted_box([a[0].min(b[0]), a[1].min(b[1]), 0.0], [a[0].max(b[0]), a[1].max(b[1]), b[2]], DARK);
            }
            m.add_cylinder(at(0.0, across / 2.0, zc), axis, 0.15 * r, 0.12 * len, ROUND, STEEL);
            m.add_cylinder(at(0.12 * len, across / 2.0, zc), axis, r, 0.7 * len, ROUND, MOTOR);
            m.add_frustum(at(0.82 * len, across / 2.0, zc), axis, r, 0.85 * r, 0.18 * len, ROUND, DARK);
            let (a, b) = (at(0.35 * len, across / 2.0 - 0.35 * r, zc), at(0.6 * len, across / 2.0 + 0.35 * r, h));
            m.add_painted_box([a[0].min(b[0]), a[1].min(b[1]), zc + 0.7 * r], [a[0].max(b[0]), a[1].max(b[1]), h], MOTOR);
            shaft = Some([at(0.0, across / 2.0, zc), at(-1.0, across / 2.0, zc)]);
        }
        Kind::Vessel(color) => {
            // Vertical vessel on legs: shell, dished head, nozzles.
            let (cx, cy) = (w / 2.0, d / 2.0);
            let r = 0.48 * w.min(d);
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let (x, y) = (cx + sx * 0.6 * r, cy + sy * 0.6 * r);
                m.add_painted_box([x - 0.06 * r, y - 0.06 * r, 0.0], [x + 0.06 * r, y + 0.06 * r, 0.07 * h], DARK);
            }
            m.add_frustum([cx, cy, 0.06 * h], 2, 0.8 * r, r, 0.04 * h, ROUND + 8, color);
            m.add_cylinder([cx, cy, 0.1 * h], 2, r, 0.76 * h, ROUND + 8, color);
            m.add_frustum([cx, cy, 0.86 * h], 2, r, 0.72 * r, 0.08 * h, ROUND + 8, color);
            m.add_frustum([cx, cy, 0.94 * h], 2, 0.72 * r, 0.3 * r, 0.04 * h, ROUND + 8, color);
            m.add_cylinder([cx, cy, 0.98 * h], 2, 0.12 * r, 0.02 * h, ROUND, STEEL);
            for z in [0.18 * h, 0.7 * h] {
                m.add_cylinder([cx + 0.9 * r, cy, z], 0, 0.08 * r, w - cx - 0.9 * r, ROUND, STEEL);
            }
            circle(&mut prims, [cx, cy], r);
        }
        Kind::HeatPump => {
            // Outdoor unit standing on its plinth: housing, fan behind its grille in the
            // front (y 0), control panel.
            m.add_painted_box([0.02 * w, 0.06 * d, 0.0], [0.98 * w, 0.94 * d, 0.04 * h], DARK);
            m.add_painted_box([0.0, 0.03 * d, 0.04 * h], [w, d, h], WHITE);
            let r = (0.42 * w).min(0.2 * h);
            m.add_cylinder([w / 2.0, 0.01 * d, 0.68 * h], 1, r, 0.02 * d, ROUND + 8, DARK);
            m.add_cylinder([w / 2.0, 0.005 * d, 0.68 * h], 1, 0.2 * r, 0.02 * d, ROUND, BLACK);
            // The guard grille across the fan and the air outlet louvres at the top.
            for k in 0..7 {
                let z = 0.68 * h - r + (k as f32 + 0.5) * 2.0 * r / 7.0;
                m.add_painted_box([w / 2.0 - 1.05 * r, 0.0, z - 0.004 * h], [w / 2.0 + 1.05 * r, 0.01 * d, z + 0.004 * h], [150, 154, 160]);
            }
            for k in 0..5 {
                let z = 0.9 * h + k as f32 * 0.018 * h;
                m.add_painted_box([0.12 * w, 0.0, z], [0.88 * w, 0.03 * d, z + 0.008 * h], [180, 184, 190]);
            }
            m.add_painted_box([0.3 * w, 0.0, 0.3 * h], [0.7 * w, 0.03 * d, 0.36 * h], DISPLAY);
            for x in [0.25 * w, 0.75 * w] {
                m.add_cylinder([x, 0.97 * d, 0.12 * h], 1, 0.05 * w, 0.03 * d, ROUND, STEEL);
            }
        }
        Kind::Compressor { motor } => {
            // Piston compressor on its horizontal receiver: feet, tank, motor, belt
            // guard and compressor block on top; without a motor the crankshaft comes
            // out of the block at the front.
            let r = (d / 2.0).min(0.3 * h) * 0.95;
            let zc = 0.07 * h + r;
            for x in [0.12 * w, 0.82 * w] {
                m.add_painted_box([x, 0.1 * d, 0.0], [x + 0.06 * w, 0.9 * d, 0.07 * h + 0.2 * r], DARK);
            }
            m.add_frustum([0.0, d / 2.0, zc], 0, 0.6 * r, r, 0.05 * w, ROUND + 8, RED);
            m.add_cylinder([0.05 * w, d / 2.0, zc], 0, r, 0.9 * w, ROUND + 8, RED);
            m.add_frustum([0.95 * w, d / 2.0, zc], 0, r, 0.6 * r, 0.05 * w, ROUND + 8, RED);
            let top = zc + r;
            if motor {
                let rm = (0.16 * (h - top)).max(0.1 * d).min(0.3 * d);
                m.add_cylinder([0.12 * w, d / 2.0, top + rm], 0, rm, 0.3 * w, ROUND, MOTOR);
                m.add_painted_box([0.43 * w, 0.15 * d, top], [0.5 * w, 0.85 * d, 0.9 * h], RED);
            } else {
                let zk = top + 0.3 * (0.75 * h - top);
                m.add_cylinder([0.65 * w, 0.0, zk], 1, 0.035 * d, 0.25 * d, ROUND, STEEL);
                shaft = Some([[0.65 * w, 0.0, zk], [0.65 * w, -1.0, zk]]);
            }
            m.add_painted_box([0.52 * w, 0.25 * d, top], [0.78 * w, 0.75 * d, 0.75 * h], STEEL);
            m.add_cylinder([0.65 * w, d / 2.0, 0.75 * h], 2, 0.1 * d, 0.25 * h, ROUND, DARK);
            m.add_cylinder([0.86 * w, 0.0, top + 0.1 * h], 1, 0.06 * h, 0.1 * d, ROUND, WHITE);
        }
        Kind::Unit => {
            // Cabinet housing: plinth, body, air grille at the front, control panel.
            m.add_painted_box([0.03 * w, 0.03 * d, 0.0], [0.97 * w, 0.97 * d, 0.05 * h], DARK);
            m.add_painted_box([0.0, 0.02 * d, 0.05 * h], [w, d, h], BODY);
            for k in 0..6 {
                let z = 0.15 * h + k as f32 * 0.06 * h;
                m.add_painted_box([0.15 * w, 0.0, z], [0.85 * w, 0.02 * d, z + 0.03 * h], DARK);
            }
            m.add_painted_box([0.3 * w, 0.0, 0.8 * h], [0.7 * w, 0.02 * d, 0.88 * h], DISPLAY);
            for x in [0.3 * w, 0.7 * w] {
                m.add_cylinder([x, 0.98 * d, 0.9 * h], 1, 0.05 * w, 0.02 * d, ROUND, STEEL);
            }
        }
        Kind::Valve { actuator } => {
            // Pipe along the wall on its brackets, valve body, stem with the lever or the
            // actuator.
            let rb = (0.24 * h).min(0.3 * d);
            let (yc, zc) = (0.35 * h, d - rb);
            pipe_along_x(&mut m, w, yc, zc, 0.65 * rb, BRASS);
            m.add_cylinder([0.3 * w, yc, zc], 0, rb, 0.4 * w, ROUND, BRASS);
            for x in [0.02 * w, 0.86 * w] {
                m.add_frustum([x, yc, zc], 0, 0.85 * rb, 0.85 * rb, 0.12 * w, 6, STEEL);
            }
            if actuator {
                m.add_cylinder([w / 2.0, yc + rb, zc], 1, 0.2 * rb, 0.1 * h, ROUND, STEEL);
                m.add_painted_box([0.22 * w, yc + rb + 0.1 * h, zc - 1.2 * rb], [0.78 * w, h, 0.98 * d], BODY);
                m.add_painted_box([0.4 * w, 0.82 * h, 0.98 * d], [0.6 * w, 0.9 * h, d], LED);
            } else {
                m.add_cylinder([w / 2.0, yc + rb, zc], 1, 0.2 * rb, h - 0.12 * h - yc - rb, ROUND, STEEL);
                m.add_painted_box([0.45 * w, 0.88 * h, zc - 0.25 * rb], [w, h, zc + 0.25 * rb], RED);
            }
            prims.push(Prim2d::Line { a: [0.0, h - yc], b: [w, h - yc] });
            circle(&mut prims, [w / 2.0, h - yc], rb);
        }
        Kind::Inline { dial } => {
            // Fitting in a pipe along the wall: brackets, pipe, body, register on top.
            let rb = (0.32 * h).min(0.42 * d);
            let (yc, zc) = (if dial { 0.32 * h } else { 0.5 * h }, d - rb);
            pipe_along_x(&mut m, w, yc, zc, 0.6 * rb, BRASS);
            m.add_cylinder([0.25 * w, yc, zc], 0, rb, 0.5 * w, ROUND, if dial { BRASS } else { STEEL });
            if dial {
                m.add_cylinder([w / 2.0, yc + 0.8 * rb, zc], 1, 0.8 * rb, h - yc - 0.8 * rb - 0.04 * h, ROUND, BLACK);
                m.add_cylinder([w / 2.0, h - 0.04 * h, zc], 1, 0.7 * rb, 0.04 * h, ROUND, BOWL);
            } else {
                m.add_frustum([0.75 * w, yc, zc], 0, rb, 0.6 * rb, 0.1 * w, ROUND, STEEL);
            }
            circle(&mut prims, [w / 2.0, h - yc], rb);
        }
        Kind::Coupling => {
            // Quick coupling hanging from its outlet: nipple, sleeve with its collar.
            let (cx, zc) = (w / 2.0, d / 2.0);
            let r = 0.42 * w.min(d);
            m.add_painted_box([0.2 * w, 0.85 * h, 0.0], [0.8 * w, h, zc], DARK);
            m.add_cylinder([cx, 0.0, zc], 1, 0.35 * r, 0.25 * h, ROUND, BRASS);
            m.add_cylinder([cx, 0.25 * h, zc], 1, r, 0.45 * h, ROUND, STEEL);
            m.add_cylinder([cx, 0.45 * h, zc], 1, 1.1 * r.min(0.5 * w), 0.08 * h, ROUND, DARK);
            m.add_frustum([cx, 0.7 * h, zc], 1, 0.6 * r, 0.6 * r, 0.3 * h, 6, BRASS);
            circle(&mut prims, [cx, h / 2.0], r);
        }
        Kind::Entry => {
            // Pipe out of the wall: wall plate, pipe stub, flange.
            let (cx, cy) = (w / 2.0, h / 2.0);
            let r = 0.48 * w.min(h);
            m.add_cylinder([cx, cy, 0.0], 2, r, 0.1 * d, ROUND + 8, DARK);
            m.add_cylinder([cx, cy, 0.1 * d], 2, 0.3 * r, 0.9 * d, ROUND, STEEL);
            m.add_cylinder([cx, cy, 0.55 * d], 2, 0.55 * r, 0.06 * d, ROUND, STEEL);
            circle(&mut prims, [cx, cy], r);
            circle(&mut prims, [cx, cy], 0.3 * r);
        }
        Kind::Manifold(n) => {
            // Header pipe on brackets with its outlets downwards, each with a ball valve.
            let rp = (0.22 * h).min(0.3 * d);
            let (yc, zc) = (0.7 * h, d - rp);
            pipe_along_x(&mut m, w, yc, zc, rp, BRASS);
            let n = n.max(1);
            for k in 0..n {
                let x = w * (k as f32 + 0.5) / n as f32;
                m.add_cylinder([x, 0.0, zc], 1, 0.4 * rp, yc, ROUND, BRASS);
                m.add_cylinder([x, 0.3 * h, zc], 1, 0.65 * rp, 0.12 * h, ROUND, BRASS);
                m.add_painted_box([x, 0.36 * h - 0.02 * h, zc - 0.15 * rp], [x + (0.45 * w / n as f32), 0.36 * h + 0.02 * h, zc + 0.15 * rp], RED);
                prims.push(Prim2d::Line { a: [x, h - yc], b: [x, h] });
            }
            prims.push(Prim2d::Line { a: [0.0, h - yc], b: [w, h - yc] });
        }
        Kind::AirUnit { gauge, long } => {
            // Air treatment unit on its wall bracket: head with the ports, bowl or
            // membrane cartridge, drain, regulator knob and gauge.
            let r = 0.4 * w.min(d);
            let (cx, zc) = (w / 2.0, 0.55 * d);
            let (head0, head1) = if long { (0.82 * h, 0.94 * h) } else { (0.6 * h, 0.82 * h) };
            m.add_painted_box([0.25 * w, head0, 0.0], [0.75 * w, head1, 0.12 * d], DARK);
            m.add_painted_box([0.05 * w, head0, 0.12 * d], [0.95 * w, head1, 0.96 * d], DARK);
            m.add_cylinder([0.0, (head0 + head1) / 2.0, zc], 0, 0.3 * r, w, ROUND, STEEL);
            m.add_cylinder([cx, 0.0, zc], 1, 0.25 * r, 0.06 * h, ROUND, BLACK);
            m.add_cylinder([cx, 0.06 * h, zc], 1, r, head0 - 0.06 * h, ROUND, if long { STEEL } else { BOWL });
            let top = if gauge { BLACK } else { BOWL };
            m.add_frustum([cx, head1, zc], 1, 0.7 * r, 0.4 * r, h - head1, ROUND, top);
            if gauge {
                m.add_cylinder([cx, (head0 + head1) / 2.0, 0.96 * d], 2, 0.6 * r, 0.04 * d, ROUND, WHITE);
            }
            circle(&mut prims, [cx, h - (0.06 * h + head0) / 2.0], r);
            prims.push(Prim2d::Rect { min: [0.05 * w, h - head1], max: [0.95 * w, h - head0], fill: false });
        }
        Kind::WallBox { color, display } => {
            // Wall-mounted electronics: housing, front cover, display, cable glands
            // underneath.
            m.add_painted_box([0.0, 0.06 * h, 0.0], [w, h, 0.9 * d], color);
            m.add_painted_box([0.04 * w, 0.1 * h, 0.9 * d], [0.96 * w, 0.96 * h, 0.97 * d], color);
            if display {
                m.add_painted_box([0.3 * w, 0.68 * h, 0.97 * d], [0.7 * w, 0.82 * h, d], DISPLAY);
            }
            m.add_painted_box([0.42 * w, 0.58 * h, 0.97 * d], [0.58 * w, 0.62 * h, d], LED);
            let n = ((w / 60.0) as usize).clamp(2, 6);
            for k in 0..n {
                let x = w * (k as f32 + 0.5) / n as f32;
                m.add_cylinder([x, 0.0, 0.45 * d], 1, (0.03 * w).clamp(3.0, 12.0).min(0.2 * d), 0.06 * h, ROUND, BLACK);
            }
            if display {
                prims.push(Prim2d::Rect { min: [0.3 * w, 0.18 * h], max: [0.7 * w, 0.32 * h], fill: false });
            }
        }
        Kind::Rack(kind) => {
            // 19-inch device: body behind the front panel with its mounting ears,
            // and what its front shows.
            let face = match kind {
                RackKind::Battery => [226, 226, 222],
                RackKind::Ports => [52, 56, 62],
                _ => DARK,
            };
            m.add_painted_box([0.05 * w, 0.02 * h, 0.0], [0.95 * w, 0.98 * h, 0.95 * d], DARK);
            m.add_painted_box([0.0, 0.0, 0.95 * d], [w, h, 0.98 * d], face);
            let front = |m: &mut Mesh3d, x0: f32, y0: f32, x1: f32, y1: f32, c: [u8; 3]| m.add_painted_box([x0 * w, y0 * h, 0.98 * d], [x1 * w, y1 * h, d], c);
            match kind {
                RackKind::Battery => {
                    for x in [0.06, 0.9] {
                        front(&mut m, x, 0.2, x + 0.04, 0.8, BLACK);
                    }
                    for k in 0..6 {
                        let x = 0.2 + k as f32 * 0.03;
                        front(&mut m, x, 0.6, x + 0.015, 0.68, LED);
                    }
                    front(&mut m, 0.7, 0.3, 0.74, 0.45, RED);
                    front(&mut m, 0.76, 0.3, 0.8, 0.45, BLACK);
                    front(&mut m, 0.5, 0.3, 0.6, 0.5, BLACK);
                }
                RackKind::Ports => {
                    for row in [0.2, 0.55] {
                        for k in 0..12 {
                            let x = 0.12 + k as f32 * 0.055;
                            front(&mut m, x, row, x + 0.04, row + 0.28, BLACK);
                        }
                    }
                    front(&mut m, 0.82, 0.35, 0.86, 0.55, LED);
                }
                RackKind::Sockets => {
                    for k in 0..8 {
                        let x = 0.1 + k as f32 * 0.1;
                        front(&mut m, x, 0.15, x + 0.07, 0.85, BLACK);
                    }
                }
                RackKind::Vents => {
                    let n = ((h / 15.0) as usize).clamp(1, 8);
                    for k in 0..n {
                        let y = (k as f32 + 0.3) / n as f32;
                        front(&mut m, 0.15, y, 0.6, y + 0.4 / n as f32, BLACK);
                    }
                    front(&mut m, 0.75, 0.4, 0.8, 0.6, LED);
                }
            }
        }
        Kind::Pv => {
            // Glass-glass module: frame and the cells behind the glass in a grid.
            m.add_painted_box([0.0, 0.0, 0.0], [w, h, 0.6 * d], [196, 200, 206]);
            m.add_painted_box([0.015 * w, 0.01 * h, 0.6 * d], [0.985 * w, 0.99 * h, 0.9 * d], CELL);
            for k in 1..6 {
                let x = w * k as f32 / 6.0;
                m.add_painted_box([x - 1.0, 0.01 * h, 0.9 * d], [x + 1.0, 0.99 * h, d], [150, 160, 180]);
            }
            for k in 1..10 {
                let y = h * k as f32 / 10.0;
                m.add_painted_box([0.015 * w, y - 1.0, 0.9 * d], [0.985 * w, y + 1.0, d], [150, 160, 180]);
            }
        }
        Kind::Housing => {
            // Insulated box: base, lid with its sealing screw.
            m.add_painted_box([0.0, 0.0, 0.0], [w, h, 0.65 * d], [196, 198, 192]);
            m.add_painted_box([0.02 * w, 0.02 * h, 0.65 * d], [0.98 * w, 0.98 * h, 0.96 * d], [210, 212, 206]);
            m.add_cylinder([w / 2.0, 0.85 * h, 0.96 * d], 2, 0.03 * w.min(h), 0.04 * d, ROUND, DARK);
            m.add_painted_box([0.3 * w, 0.4 * h, 0.96 * d], [0.7 * w, 0.5 * h, d], WHITE);
            for x in [0.2 * w, 0.5 * w, 0.8 * w] {
                m.add_cylinder([x, 0.0, 0.3 * d], 1, 0.05 * w, 0.02 * h, ROUND, BLACK);
            }
        }
        Kind::FuseSwitch => {
            // NH fuse switch disconnector: base, the cover with the fuse windows and
            // its grip, connection bolts top and bottom.
            m.add_painted_box([0.0, 0.0, 0.0], [w, h, 0.45 * d], BLACK);
            m.add_painted_box([0.04 * w, 0.12 * h, 0.45 * d], [0.96 * w, 0.88 * h, 0.9 * d], BODY);
            m.add_painted_box([0.2 * w, 0.78 * h, 0.9 * d], [0.8 * w, 0.86 * h, d], DARK);
            for k in 0..3 {
                let x = w * (k as f32 + 0.5) / 3.0;
                m.add_painted_box([x - 0.08 * w, 0.3 * h, 0.9 * d], [x + 0.08 * w, 0.6 * h, 0.92 * d], DISPLAY);
                for y in [0.02 * h, 0.94 * h] {
                    m.add_cylinder([x, y + 0.02 * h, 0.45 * d], 2, (0.1 * w).min(0.03 * h), 0.08 * d, 6, STEEL);
                }
                prims.push(Prim2d::Rect { min: [x - 0.08 * w, 0.4 * h], max: [x + 0.08 * w, 0.7 * h], fill: false });
            }
            prims.push(Prim2d::Rect { min: [0.04 * w, 0.12 * h], max: [0.96 * w, 0.88 * h], fill: false });
        }
    }
    let mut ends = shaft.unwrap_or_default();
    if floor {
        // Upright: the height along y, the front (y 0 of the floor) at z = depth; the
        // front view shows the outline only.
        for v in m.vertices.iter_mut().chain(&mut ends) {
            *v = [v[0], v[2], d - v[1]];
        }
        prims.clear();
    }
    fit(&mut m, [w, h, d], &mut ends);
    prims.insert(0, Prim2d::Rect { min: [0.0, 0.0], max: [w, h], fill: false });
    let shaft = shaft.map(|_| {
        let dir = crate::vec3::norm(crate::vec3::sub(ends[1], ends[0]));
        Shaft { at: ends[0], dir }
    });
    Some((Model2d { width: w, height: h, prims }, m, shaft))
}

/// Stretches the mesh to fill the box from the origin to `size` exactly, and `points`
/// with it.
fn fit(m: &mut Mesh3d, size: [f32; 3], points: &mut [[f32; 3]]) {
    let (lo, hi) = m.bounds();
    for v in m.vertices.iter_mut().chain(points) {
        for k in 0..3 {
            let span = hi[k] - lo[k];
            v[k] = if span > 0.0 { (v[k] - lo[k]) / span * size[k] } else { 0.0 };
        }
    }
}

/// A pipe along the wall over the whole width at height `yc` and `zc` out of the wall,
/// held by two brackets.
fn pipe_along_x(m: &mut Mesh3d, w: f32, yc: f32, zc: f32, r: f32, c: [u8; 3]) {
    for x in [0.1 * w, 0.82 * w] {
        m.add_painted_box([x, yc - 0.6 * r, 0.0], [x + 0.08 * w, yc + 0.6 * r, zc], DARK);
    }
    m.add_cylinder([0.0, yc, zc], 0, r, w, ROUND, c);
}

mod brands;

#[cfg(test)]
mod tests;
