//! Mechanical drives on the floor of a room: a pump or compressor without its own
//! motor names the motor that turns it ([`DRIVEN_BY`]). In line with the machine the
//! motor turns it through a shaft coupling (or a gear unit between them), beside it
//! with the shafts side by side through V-belts (or a gear case); [`DRIVE`] chooses.
//! A motor whose shaft points away from its machine is turned end for end, a motor
//! standing higher than the floor stands on a riser block. Coordinates are those of
//! the floor (plan x, plan y, height above the floor).

use super::{part_by_number, place_point, place_triangles, Enclosure, Face, ItemKind, PanelItem};
use crate::model::Project;
use crate::parts::vec3::{add, cross, dot, norm, sub, Tri, V3};
use crate::parts::Mesh3d;

/// Device property of a pump or compressor: the tag of the motor that drives it.
pub const DRIVEN_BY: &str = "device.driven_by";
/// Device property of a pump or compressor: how its motor drives it ("Coupling",
/// "Belt" or "Gear"; none: a coupling in line, belts side by side).
pub const DRIVE: &str = "device.drive";

const STEEL: [u8; 3] = [172, 178, 186];
const HUB: [u8; 3] = [64, 68, 76];
const SPIDER: [u8; 3] = [226, 190, 52];
const PULLEY: [u8; 3] = [112, 116, 124];
const BELT: [u8; 3] = [38, 40, 44];
const CASE: [u8; 3] = [86, 110, 92];
const ROUND: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriveKind {
    Coupling,
    Belt,
    Gear,
}

/// A machine and the motor that drives it, with what connects them.
#[derive(Clone, Debug, PartialEq)]
pub struct Drive {
    pub machine: u64,
    pub motor: u64,
    pub kind: DriveKind,
    /// The motor is turned end for end (its shaft towards the machine).
    pub turned: bool,
    /// Shafts, coupling, pulleys and belts or gear case (enclosure coordinates) with
    /// their colours.
    pub triangles: Vec<(Tri, [u8; 3])>,
}

/// A shaft end and the direction the shaft points in.
#[derive(Clone, Copy, Debug)]
struct End {
    at: V3,
    dir: V3,
}

/// How a machine and its motor stand: `inline` or side by side, the motor `turned`,
/// the shaft ends (the motor's after turning it) along `axis` (0 x, 1 y).
struct Arrangement {
    inline: bool,
    turned: bool,
    machine: End,
    motor: End,
    axis: usize,
}

fn value<'a>(prj: &'a Project, tag: &str, key: &str) -> &'a str {
    prj.devices.get(tag).and_then(|d| d.values.get(key)).map_or("", |v| v.trim())
}

fn on_floor(it: &PanelItem) -> bool {
    it.kind == ItemKind::Device && it.face == Face::Bottom && !it.tag.is_empty()
}

/// The machines on the floor of `e` with the motors on that floor that drive them.
fn pairs<'a>(prj: &'a Project, e: &'a Enclosure) -> impl Iterator<Item = (&'a PanelItem, &'a PanelItem)> + 'a {
    e.items.iter().filter(|i| on_floor(i)).filter_map(move |machine| {
        let tag = value(prj, &machine.tag, DRIVEN_BY);
        let motor = e.items.iter().find(|i| on_floor(i) && i.tag == tag && i.id != machine.id)?;
        Some((machine, motor))
    })
}

/// The drives on the floor of `e`.
pub fn drives(prj: &Project, e: &Enclosure) -> Vec<Drive> {
    let floor = PanelItem { face: Face::Bottom, ..Default::default() };
    pairs(prj, e)
        .filter_map(|(machine, motor)| {
            let (a, kind, mesh) = drive_mesh(prj, machine, motor)?;
            let tris: Vec<Tri> = mesh.triangles.iter().map(|t| t.map(|i| mesh.vertices[i as usize])).collect();
            let colors = (0..tris.len()).map(|i| mesh.color(i).unwrap_or(STEEL));
            let triangles = place_triangles(e, &floor, tris).into_iter().zip(colors).collect();
            Some(Drive { machine: machine.id, motor: motor.id, kind, turned: a.turned, triangles })
        })
        .collect()
}

/// A motor on a floor turned end for end by its drive: the axis (0 x, 1 y) and the
/// middle to mirror its model about.
pub(super) fn turned_motor(prj: &Project, it: &PanelItem) -> Option<(usize, f32)> {
    if !on_floor(it) {
        return None;
    }
    let e = prj.layout.enclosures.iter().find(|e| e.items.iter().any(|i| i.id == it.id && i.tag == it.tag))?;
    let (machine, motor) = pairs(prj, e).find(|(_, m)| m.id == it.id)?;
    let a = arrange(prj, machine, motor)?;
    a.turned.then(|| (a.axis, if a.axis == 0 { it.x + it.w / 2.0 } else { it.y + it.h / 2.0 }))
}

fn centre(it: &PanelItem) -> V3 {
    [it.x + it.w / 2.0, it.y + it.h / 2.0, it.z + it.d / 2.0]
}

/// The shaft end of a placed motor or machine: from its part's model, otherwise in the
/// middle of the end of its longer side towards `toward`, half way up.
fn shaft_end(prj: &Project, it: &PanelItem, toward: V3) -> End {
    let part = part_by_number(prj, &it.part);
    if let Some((s, mesh)) = part.and_then(|p| Some((crate::parts::shaft(p)?, p.model_3d.as_deref()?))) {
        let a = place_point(mesh, it, s.at);
        let b = place_point(mesh, it, add(s.at, s.dir));
        return End { at: a, dir: norm(sub(b, a)) };
    }
    let k = if it.w >= it.h { 0 } else { 1 };
    let c = centre(it);
    let sign = if toward[k] >= c[k] { 1.0 } else { -1.0 };
    let mut at = c;
    at[k] += sign * [it.w, it.h][k] / 2.0;
    let mut dir = [0.0; 3];
    dir[k] = sign;
    End { at, dir }
}

fn arrange(prj: &Project, machine: &PanelItem, motor: &PanelItem) -> Option<Arrangement> {
    let m = shaft_end(prj, machine, centre(motor));
    let mut d = shaft_end(prj, motor, centre(machine));
    let axis = if m.dir[0].abs() >= m.dir[1].abs() { 0 } else { 1 };
    if m.dir[2].abs() > 0.1 || dot(m.dir, d.dir).abs() < 0.9 {
        return None;
    }
    let lateral = (d.at[1 - axis] - m.at[1 - axis]).abs();
    let inline = lateral < 0.25 * motor.w.min(motor.h);
    // In line the motor's shaft points back at the machine, beside it the same way.
    let turned = (dot(d.dir, m.dir) > 0.0) == inline;
    if turned {
        d.at[axis] = 2.0 * centre(motor)[axis] - d.at[axis];
        d.dir = d.dir.map(|v| -v);
    }
    if inline && dot(sub(d.at, m.at), m.dir) <= 0.0 {
        return None;
    }
    Some(Arrangement { inline, turned, machine: m, motor: d, axis })
}

/// The arrangement, the kind of drive and its parts in floor coordinates.
fn drive_mesh(prj: &Project, machine: &PanelItem, motor: &PanelItem) -> Option<(Arrangement, DriveKind, Mesh3d)> {
    let a = arrange(prj, machine, motor)?;
    let gap = dot(sub(a.motor.at, a.machine.at), a.machine.dir);
    let kind = match (a.inline, value(prj, &machine.tag, DRIVE)) {
        (true, "Gear") if gap >= 160.0 => DriveKind::Gear,
        (true, _) => DriveKind::Coupling,
        (false, "Gear") => DriveKind::Gear,
        (false, _) => DriveKind::Belt,
    };
    // The motor's body and shaft as its model draws them.
    let rm = (motor.w.min(motor.h) / 2.0).min(0.42 * motor.d) * 0.92;
    let rs = (0.15 * rm).max(3.0);
    let mut m = Mesh3d::default();
    match kind {
        DriveKind::Coupling => coupling(&mut m, &a, gap, rm, rs),
        DriveKind::Gear if a.inline => gear_unit(&mut m, &a, gap, rm, rs),
        DriveKind::Belt => belts(&mut m, &a, rm, rs),
        DriveKind::Gear => gear_case(&mut m, &a, rm, rs),
    }
    if motor.z > 1.0 {
        let inset = 0.08 * motor.w.min(motor.h);
        m.add_painted_box([motor.x + inset, motor.y + inset, 0.0], [motor.x + motor.w - inset, motor.y + motor.h - inset, motor.z], HUB);
    }
    Some((a, kind, m))
}

/// A cylinder along `axis` from `from` to `to` through the point `at`.
#[allow(clippy::too_many_arguments)]
fn rod(m: &mut Mesh3d, at: V3, axis: usize, from: f32, to: f32, r: f32, n: usize, c: [u8; 3]) {
    let mut base = at;
    base[axis] = from.min(to);
    if (to - from).abs() > 0.0 && r > 0.0 {
        m.add_cylinder(base, axis, r, (to - from).abs(), n, c);
    }
}

/// A box between two corners.
fn block(m: &mut Mesh3d, a: V3, b: V3, c: [u8; 3]) {
    m.add_painted_box([0, 1, 2].map(|k| a[k].min(b[k])), [0, 1, 2].map(|k| a[k].max(b[k])), c);
}

/// Shaft, hub, the elastic spider and the motor's hub and shaft between the two ends.
fn coupling(m: &mut Mesh3d, a: &Arrangement, gap: f32, rm: f32, rs: f32) {
    let (p, q, k) = (a.machine.at, a.motor.at, a.axis);
    let sgn = a.machine.dir[k].signum();
    let s = |t: f32| p[k] + sgn * t;
    let rh = (0.42 * rm).min(p[2].min(q[2]) - 2.0).max(1.5 * rs);
    let ls = (0.08 * gap).min(0.3 * rh);
    let lh = ((gap - ls) / 2.0 * 0.7).min(1.1 * rh);
    let s1 = (gap - ls) / 2.0 - lh;
    let mid = [0, 1, 2].map(|i| (p[i] + q[i]) / 2.0);
    rod(m, p, k, s(0.0), s(s1 + 0.5 * lh), rs, ROUND, STEEL);
    rod(m, p, k, s(s1), s(s1 + lh), rh, ROUND, HUB);
    rod(m, mid, k, s(s1 + lh), s(s1 + lh + ls), 0.9 * rh, ROUND, SPIDER);
    rod(m, q, k, s(s1 + lh + ls), s(gap - s1), rh, ROUND, HUB);
    rod(m, q, k, s(gap - s1 - 0.5 * lh), s(gap), rs, ROUND, STEEL);
}

/// A gear unit standing on the floor between them, coupled to both shafts.
fn gear_unit(m: &mut Mesh3d, a: &Arrangement, gap: f32, rm: f32, rs: f32) {
    let (p, q, k) = (a.machine.at, a.motor.at, a.axis);
    let l = 1 - k;
    let sgn = a.machine.dir[k].signum();
    let s = |t: f32| p[k] + sgn * t;
    let rh = (0.42 * rm).max(1.5 * rs);
    for (end, t0, t1) in [(p, 0.0, 0.3), (q, 0.7, 1.0)] {
        rod(m, end, k, s(t0 * gap), s(t1 * gap), rs, ROUND, STEEL);
        rod(m, end, k, s((t0 + 0.08) * gap), s((t0 + 0.22) * gap), rh, ROUND, HUB);
    }
    let (lc, half) = ((p[l] + q[l]) / 2.0, 1.3 * rh + (p[l] - q[l]).abs() / 2.0);
    let mut lo = [0.0; 3];
    let mut hi = [0.0; 3];
    (lo[k], hi[k]) = (s(0.3 * gap), s(0.7 * gap));
    (lo[l], hi[l]) = (lc - half, lc + half);
    hi[2] = p[2].max(q[2]) + 1.3 * rh;
    block(m, lo, hi, CASE);
}

/// The shafts out past both ends, a pulley on each and two V-belts around them.
fn belts(m: &mut Mesh3d, a: &Arrangement, rm: f32, rs: f32) {
    let (p, q, k) = (a.machine.at, a.motor.at, a.axis);
    let sgn = a.machine.dir[k].signum();
    let front = if sgn > 0.0 { p[k].max(q[k]) } else { p[k].min(q[k]) };
    let (bw, between, rim) = (13.0, 4.0, 4.0);
    let p0 = front + sgn * 12.0;
    let p1 = p0 + sgn * (2.0 * bw + between + 2.0 * rim);
    let (cm, cd) = (plane(p, k), plane(q, k));
    let rd = (0.75 * rm).min(q[2] - 10.0).max(15.0);
    let rp = (2.6 * rd).min(p[2] - 10.0).min(0.45 * dist2(cm, cd)).max(rd);
    for (end, r, shaft) in [(p, rp, 1.3 * rs), (q, rd, rs)] {
        rod(m, end, k, end[k], p1 + sgn * 6.0, shaft, ROUND, STEEL);
        rod(m, end, k, p0, p1, r, 2 * ROUND, PULLEY);
        rod(m, end, k, p0 - sgn * 4.0, p1 + sgn * 4.0, 0.3 * r, ROUND, HUB);
    }
    for j in 0..2 {
        let a0 = p0 + sgn * (rim + j as f32 * (bw + between));
        ribbon(m, k, (cm, rp), (cd, rd), (-5.0, 3.0), (a0, a0 + sgn * bw), BELT);
    }
}

/// The shafts into a closed gear case around both of them.
fn gear_case(m: &mut Mesh3d, a: &Arrangement, rm: f32, rs: f32) {
    let (p, q, k) = (a.machine.at, a.motor.at, a.axis);
    let sgn = a.machine.dir[k].signum();
    let front = if sgn > 0.0 { p[k].max(q[k]) } else { p[k].min(q[k]) };
    let p0 = front + sgn * 10.0;
    let (cm, cd) = (plane(p, k), plane(q, k));
    let rd = (rm + 25.0).min(q[2] - 5.0);
    let rp = (0.35 * dist2(cm, cd)).max(rd).min(p[2] - 5.0);
    for (end, shaft) in [(p, 1.3 * rs), (q, rs)] {
        rod(m, end, k, end[k], p0 + sgn * 20.0, shaft, ROUND, STEEL);
    }
    prism(m, k, (cm, rp), (cd, rd), (p0, p0 + sgn * 70.0), CASE);
}

/// A point in the plane across `axis`: its other horizontal coordinate and its height.
fn plane(v: V3, axis: usize) -> [f32; 2] {
    [v[1 - axis], v[2]]
}

/// The point of that plane at `along` on `axis`.
fn space(q: [f32; 2], axis: usize, along: f32) -> V3 {
    let mut v = [0.0; 3];
    v[axis] = along;
    v[1 - axis] = q[0];
    v[2] = q[1];
    v
}

fn dist2(a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// The outline around two circles (a belt laid around two pulleys), counter clockwise:
/// the arc around the first one away from the second, the arc around the second one.
/// Outlines of circles grown by the same amount have their points at the same angles.
fn outline((c1, r1): ([f32; 2], f32), (c2, r2): ([f32; 2], f32), n: usize) -> Vec<[f32; 2]> {
    let d = dist2(c1, c2).max(1e-3);
    let th = (c2[1] - c1[1]).atan2(c2[0] - c1[0]);
    let g = ((r1 - r2) / d).clamp(-1.0, 1.0).acos();
    let arc = |c: [f32; 2], r: f32, from: f32, to: f32| (0..=n).map(move |i| from + (to - from) * i as f32 / n as f32).map(move |t| [c[0] + r * t.cos(), c[1] + r * t.sin()]);
    arc(c1, r1, th + g, th - g + std::f32::consts::TAU).chain(arc(c2, r2, th - g, th + g)).collect()
}

/// A triangle or quad facing `out`.
fn face(m: &mut Mesh3d, pts: &[V3], out: V3, c: [u8; 3]) {
    let n = cross(sub(pts[1], pts[0]), sub(pts[2], pts[0]));
    let first = m.vertices.len() as u32;
    let from = m.triangles.len();
    if dot(n, out) >= 0.0 {
        m.vertices.extend_from_slice(pts);
    } else {
        m.vertices.extend(pts.iter().rev());
    }
    for i in 1..pts.len() as u32 - 1 {
        m.triangles.push([first, first + i, first + i + 1]);
    }
    m.paint(from, c);
}

/// A band laid around two circles between `along.0` and `along.1` on `axis`, from
/// `grow.0` inside to `grow.1` outside their radii.
fn ribbon(m: &mut Mesh3d, axis: usize, one: ([f32; 2], f32), two: ([f32; 2], f32), grow: (f32, f32), along: (f32, f32), c: [u8; 3]) {
    let inner = outline((one.0, one.1 + grow.0), (two.0, two.1 + grow.0), 12);
    let outer = outline((one.0, one.1 + grow.1), (two.0, two.1 + grow.1), 12);
    let mid = [(one.0[0] + two.0[0]) / 2.0, (one.0[1] + two.0[1]) / 2.0];
    let side = space([0.0, 0.0], axis, along.0 - along.1);
    let n = outer.len();
    for i in 0..n {
        let j = (i + 1) % n;
        let radial = |q: [f32; 2]| space([q[0] - mid[0], q[1] - mid[1]], axis, 0.0);
        let rim = [(outer[i][0] + outer[j][0]) / 2.0, (outer[i][1] + outer[j][1]) / 2.0];
        let at = |q: [f32; 2], s: f32| space(q, axis, s);
        face(m, &[at(outer[i], along.0), at(outer[j], along.0), at(outer[j], along.1), at(outer[i], along.1)], radial(rim), c);
        face(m, &[at(inner[i], along.0), at(inner[j], along.0), at(inner[j], along.1), at(inner[i], along.1)], radial(rim).map(|v| -v), c);
        face(m, &[at(outer[i], along.0), at(outer[j], along.0), at(inner[j], along.0), at(inner[i], along.0)], side, c);
        face(m, &[at(outer[i], along.1), at(outer[j], along.1), at(inner[j], along.1), at(inner[i], along.1)], side.map(|v| -v), c);
    }
}

/// A closed body with the outline around two circles between `along.0` and `along.1`.
fn prism(m: &mut Mesh3d, axis: usize, one: ([f32; 2], f32), two: ([f32; 2], f32), along: (f32, f32), c: [u8; 3]) {
    let ring = outline(one, two, 12);
    let n = ring.len();
    let mid = [ring.iter().map(|q| q[0]).sum::<f32>() / n as f32, ring.iter().map(|q| q[1]).sum::<f32>() / n as f32];
    let side = space([0.0, 0.0], axis, along.0 - along.1);
    let at = |q: [f32; 2], s: f32| space(q, axis, s);
    for i in 0..n {
        let j = (i + 1) % n;
        let rim = [(ring[i][0] + ring[j][0]) / 2.0 - mid[0], (ring[i][1] + ring[j][1]) / 2.0 - mid[1]];
        face(m, &[at(ring[i], along.0), at(ring[j], along.0), at(ring[j], along.1), at(ring[i], along.1)], space(rim, axis, 0.0), c);
        face(m, &[at(mid, along.0), at(ring[i], along.0), at(ring[j], along.0)], side, c);
        face(m, &[at(mid, along.1), at(ring[i], along.1), at(ring[j], along.1)], side.map(|v| -v), c);
    }
}

#[cfg(test)]
mod tests;
