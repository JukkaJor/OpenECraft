//! Rooms: enclosures of the kind [`EnclosureKind::Room`]. A room's floor plan is an
//! outline of walls at right angles (x to the right, z towards the front, mm; the
//! inside surfaces of the walls); every wall is a face of its own ([`Face::Wall`],
//! seen from inside the room) and the ceiling is `Face::Roof`. Cabinets (their
//! line-ups) stand in the room at a place in its floor plan; walls and ceiling carry
//! mounting rails, cable ducts and devices. A device on a wall sits on a rail unless
//! its part is made for direct wall mounting ([`crate::parts::Part::wall_mountable`]).
//!
//! Room coordinates (the enclosure coordinates of a room): x and z of the floor plan,
//! y downwards from the ceiling (the floor at `height`).

use super::{item_name, lineup_of, new_profile, part_by_number, place_device_on, site_poses, snap, CheckKind, Enclosure, EnclosureKind, Face, InnerWall, ItemKind, Layout, Opening, PanelItem, SitePlace};
use crate::geom::P;
use crate::issue::Severity;
use crate::model::Project;
use crate::parts::vec3::V3;
use openecraft_i18n::{tr, tr_fmt};

/// Thickness of the walls and the ceiling drawn for a room, mm (outside its outline).
pub const ROOM_WALL: f32 = 10.0;
/// Width and height of a new door, mm.
pub const DOOR_SIZE: (f32, f32) = (1000.0, 2100.0);
/// Width and height of a new window and its sill above the floor, mm.
pub const WINDOW_SIZE: (f32, f32) = (1200.0, 1200.0);
pub const WINDOW_SILL: f32 = 900.0;
/// Shortest wall of an outline, mm.
pub const MIN_WALL: f32 = 50.0;
/// Thickness of a new inner wall, mm.
pub const INNER_WALL: f32 = 100.0;

/// A wall of a room's outline: from corner `a` to corner `b`, `normal` pointing into
/// the room (plan coordinates x / z).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wall {
    pub a: [f32; 2],
    pub b: [f32; 2],
    pub normal: [f32; 2],
}

impl Wall {
    pub fn length(&self) -> f32 {
        (self.b[0] - self.a[0]).abs() + (self.b[1] - self.a[1]).abs()
    }

    /// Unit direction from `a` to `b`.
    pub fn dir(&self) -> [f32; 2] {
        let l = self.length().max(1e-6);
        [(self.b[0] - self.a[0]) / l, (self.b[1] - self.a[1]) / l]
    }

    /// Direction of the face's x axis: to the right, seen from inside the room.
    pub fn right(&self) -> [f32; 2] {
        [self.normal[1], -self.normal[0]]
    }

    /// The corner the face's x axis starts at (its left end seen from inside).
    pub fn start(&self) -> [f32; 2] {
        let r = self.right();
        if (self.b[0] - self.a[0]) * r[0] + (self.b[1] - self.a[1]) * r[1] >= 0.0 {
            self.a
        } else {
            self.b
        }
    }
}

/// The corners of a room's floor plan: its outline, or the rectangle of its width and
/// depth.
pub fn outline(e: &Enclosure) -> Vec<[f32; 2]> {
    if e.outline.len() >= 4 {
        return e.outline.clone();
    }
    vec![[0.0, 0.0], [e.width, 0.0], [e.width, e.depth], [0.0, e.depth]]
}

/// Smallest and largest x / z of an outline.
pub fn bounds(pts: &[[f32; 2]]) -> ([f32; 2], [f32; 2]) {
    let mut lo = [f32::INFINITY; 2];
    let mut hi = [f32::NEG_INFINITY; 2];
    for q in pts {
        for k in 0..2 {
            lo[k] = lo[k].min(q[k]);
            hi[k] = hi[k].max(q[k]);
        }
    }
    if pts.is_empty() {
        return ([0.0; 2], [0.0; 2]);
    }
    (lo, hi)
}

/// The point lies inside the outline (even-odd rule).
pub fn inside_outline(pts: &[[f32; 2]], q: [f32; 2]) -> bool {
    let mut inside = false;
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        if (a[1] > q[1]) != (b[1] > q[1]) {
            let x = a[0] + (q[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
            if q[0] < x {
                inside = !inside;
            }
        }
    }
    inside
}

fn walls_of(pts: &[[f32; 2]]) -> Vec<Wall> {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            let l = ((b[0] - a[0]).abs() + (b[1] - a[1]).abs()).max(1e-6);
            let d = [(b[0] - a[0]) / l, (b[1] - a[1]) / l];
            let left = [d[1], -d[0]];
            let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
            let normal = if inside_outline(pts, [mid[0] + left[0], mid[1] + left[1]]) { left } else { [-left[0], -left[1]] };
            Wall { a, b, normal }
        })
        .collect()
}

/// The walls of a room in the order of its outline (wall `n` is `Face::Wall(n)`).
pub fn walls(e: &Enclosure) -> Vec<Wall> {
    walls_of(&outline(e))
}

/// The faces of a room: its walls, both sides of its inner walls and the ceiling.
pub fn room_faces(e: &Enclosure) -> Vec<Face> {
    let inner = e.inner_walls.iter().flat_map(|w| [Face::Inner(w.id, 0), Face::Inner(w.id, 1)]);
    (0..outline(e).len() as u16).map(Face::Wall).chain(inner).chain([Face::Roof]).collect()
}

fn inner_index(e: &Enclosure, id: u64) -> Option<usize> {
    e.inner_walls.iter().position(|w| w.id == id)
}

/// Title of a face of a room: `Wall 2`, `Inner wall 1, side A`, `Ceiling`.
pub fn room_face_title(e: &Enclosure, face: Face) -> String {
    match face {
        Face::Wall(n) => tr_fmt!("Wall {}", n + 1),
        Face::Inner(id, s) => tr_fmt!("Inner wall {}, side {}", inner_index(e, id).map_or(0, |k| k + 1), if s == 0 { "A" } else { "B" }),
        Face::Roof => tr("Ceiling").to_string(),
        Face::Bottom => tr("Floor").to_string(),
        f => f.title().to_string(),
    }
}

/// Key of a room face on the command line: `wall1`, …, `inner1a`, `inner1b`, …,
/// `ceiling`.
pub fn room_face_key(e: &Enclosure, face: Face) -> String {
    match face {
        Face::Wall(n) => format!("wall{}", n + 1),
        Face::Inner(id, s) => format!("inner{}{}", inner_index(e, id).map_or(0, |k| k + 1), if s == 0 { "a" } else { "b" }),
        Face::Roof => "ceiling".into(),
        f => f.key().into(),
    }
}

/// A room face from its key (also `roof`).
pub fn room_face_from_key(e: &Enclosure, key: &str) -> Option<Face> {
    let k = key.trim().to_lowercase();
    if k == "ceiling" || k == "roof" {
        return Some(Face::Roof);
    }
    if let Some(rest) = k.strip_prefix("inner") {
        let (num, side) = rest.split_at(rest.len().saturating_sub(1));
        let s = match side {
            "a" => 0,
            "b" => 1,
            _ => return None,
        };
        let w = e.inner_walls.get(num.trim().parse::<usize>().ok()?.checked_sub(1)?)?;
        return Some(Face::Inner(w.id, s));
    }
    k.strip_prefix("wall").and_then(|n| n.trim().parse::<u16>().ok()).filter(|n| *n >= 1).map(|n| Face::Wall(n - 1))
}

/// Both sides of an inner wall (side 0 first) as walls facing into the room.
pub fn inner_sides(w: &InnerWall) -> [Wall; 2] {
    let l = ((w.b[0] - w.a[0]).abs() + (w.b[1] - w.a[1]).abs()).max(1e-6);
    let d = [(w.b[0] - w.a[0]) / l, (w.b[1] - w.a[1]) / l];
    let n = [d[1], -d[0]];
    let h = w.thickness / 2.0;
    [1.0, -1.0].map(|s| {
        let o = [n[0] * h * s, n[1] * h * s];
        Wall { a: [w.a[0] + o[0], w.a[1] + o[1]], b: [w.b[0] + o[0], w.b[1] + o[1]], normal: [n[0] * s, n[1] * s] }
    })
}

/// The wall surface of a face of a room: a wall of the outline or a side of an inner
/// wall.
pub fn side(e: &Enclosure, face: Face) -> Option<Wall> {
    match face {
        Face::Wall(n) => walls(e).get(n as usize).copied(),
        Face::Inner(id, s) => e.inner_walls.iter().find(|w| w.id == id).map(|w| inner_sides(w)[(s as usize).min(1)]),
        _ => None,
    }
}

/// Size of a face of a room: a wall's length and the room's height, the ceiling's
/// extent; nothing for faces the room does not have.
pub(super) fn face_size(e: &Enclosure, face: Face) -> (f32, f32) {
    match face {
        Face::Roof | Face::Bottom => {
            let (lo, hi) = bounds(&outline(e));
            (hi[0] - lo[0], hi[1] - lo[1])
        }
        f => side(e, f).map(|w| (w.length(), e.height)).unwrap_or((0.0, 0.0)),
    }
}

/// How far items may reach into the room from a face.
pub(super) fn face_room(e: &Enclosure, face: Face) -> f32 {
    let (lo, hi) = bounds(&outline(e));
    match face {
        Face::Roof | Face::Bottom => e.height,
        f => match side(e, f) {
            Some(w) if w.normal[0].abs() > 0.5 => hi[0] - lo[0],
            Some(_) => hi[1] - lo[1],
            None => 0.0,
        },
    }
}

/// Frame of a face of a room (see `super::face_frame`).
pub(super) fn frame(e: &Enclosure, face: Face) -> [V3; 4] {
    match face {
        Face::Wall(_) | Face::Inner(..) => match side(e, face) {
            Some(w) => {
                let (s, r) = (w.start(), w.right());
                [[s[0], 0.0, s[1]], [r[0], 0.0, r[1]], [0.0, 1.0, 0.0], [w.normal[0], 0.0, w.normal[1]]]
            }
            None => [[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        },
        _ => {
            // The ceiling seen from above like the roof of a cabinet, the front at the bottom.
            let (lo, _) = bounds(&outline(e));
            [[lo[0], 0.0, lo[1]], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]]
        }
    }
}

/// Box of a wall (outside the outline, from the floor to the top of the ceiling;
/// lengthened at outside corners so the corners are closed).
pub(super) fn wall_box(e: &Enclosure, face: Face) -> (V3, V3) {
    let pts = outline(e);
    let ws = walls_of(&pts);
    let t = e.wall();
    let n = ws.len();
    if let Face::Inner(id, _) = face {
        // From the floor to the ceiling, as thick as it is.
        let Some(w) = e.inner_walls.iter().find(|w| w.id == id) else { return ([0.0; 3], [0.0; 3]) };
        let [p, q] = inner_sides(w);
        let xs = [p.a[0], p.b[0], q.a[0], q.b[0]];
        let zs = [p.a[1], p.b[1], q.a[1], q.b[1]];
        let min = |v: [f32; 4]| v.into_iter().fold(f32::INFINITY, f32::min);
        let max = |v: [f32; 4]| v.into_iter().fold(f32::NEG_INFINITY, f32::max);
        return ([min(xs), 0.0, min(zs)], [max(xs), e.height, max(zs)]);
    }
    let Face::Wall(i) = face else { return ([0.0; 3], [0.0; 3]) };
    let i = i as usize;
    let Some(w) = ws.get(i) else { return ([0.0; 3], [0.0; 3]) };
    // A corner is convex when the room lies only on the inner side of both walls.
    let convex = |p: &Wall, q: &Wall, v: [f32; 2]| !inside_outline(&pts, [v[0] + 0.5 * (p.normal[0] - q.normal[0]), v[1] + 0.5 * (p.normal[1] - q.normal[1])]);
    let d = w.dir();
    let ext_a = if convex(&ws[(i + n - 1) % n], w, w.a) { t } else { 0.0 };
    let ext_b = if convex(w, &ws[(i + 1) % n], w.b) { t } else { 0.0 };
    let a = [w.a[0] - d[0] * ext_a, w.a[1] - d[1] * ext_a];
    let b = [w.b[0] + d[0] * ext_b, w.b[1] + d[1] * ext_b];
    let o = [-w.normal[0] * t, -w.normal[1] * t];
    let xs = [a[0], b[0], a[0] + o[0], b[0] + o[0]];
    let zs = [a[1], b[1], a[1] + o[1], b[1] + o[1]];
    let min = |v: [f32; 4]| v.into_iter().fold(f32::INFINITY, f32::min);
    let max = |v: [f32; 4]| v.into_iter().fold(f32::NEG_INFINITY, f32::max);
    ([min(xs), -t, min(zs)], [max(xs), e.height, max(zs)])
}

/// Boxes of the ceiling (room coordinates): the cells of the outline's grid that lie
/// inside it; their x and z also fill the floor plan.
pub fn floor_cells(e: &Enclosure) -> Vec<(V3, V3)> {
    let pts = outline(e);
    let grid = |k: usize| {
        let mut v: Vec<f32> = pts.iter().map(|q| q[k]).collect();
        v.sort_by(f32::total_cmp);
        v.dedup_by(|a, b| (*a - *b).abs() < 0.01);
        v
    };
    let (xs, zs) = (grid(0), grid(1));
    let t = e.wall();
    let mut out = Vec::new();
    for x in xs.windows(2) {
        for z in zs.windows(2) {
            if inside_outline(&pts, [(x[0] + x[1]) / 2.0, (z[0] + z[1]) / 2.0]) {
                out.push(([x[0], -t, z[0]], [x[1], 0.0, z[1]]));
            }
        }
    }
    out
}

/// Checks an outline: at least four corners, every wall along x or z and at least
/// [`MIN_WALL`] long, no walls crossing or touching each other.
pub fn check_outline(pts: &[[f32; 2]]) -> Result<(), String> {
    let n = pts.len();
    if n < 4 {
        return Err(tr("A room needs at least four walls").to_string());
    }
    let ws = walls_of(pts);
    for (i, w) in ws.iter().enumerate() {
        let (dx, dz) = ((w.b[0] - w.a[0]).abs(), (w.b[1] - w.a[1]).abs());
        if dx > 0.01 && dz > 0.01 {
            return Err(tr_fmt!("Wall {} is not at a right angle", i + 1));
        }
        if w.length() < MIN_WALL - 0.01 {
            return Err(tr_fmt!("Wall {} is shorter than {} mm", i + 1, MIN_WALL));
        }
    }
    // Neighbours meet at a right angle (a wall running back over the last one would
    // be parallel), the others keep apart.
    for i in 0..n {
        let (a, b) = (ws[i].dir(), ws[(i + 1) % n].dir());
        if (a[0] * b[0] + a[1] * b[1]).abs() > 0.5 {
            return Err(tr_fmt!("Walls {} and {} cross", i + 1, (i + 1) % n + 1));
        }
    }
    for i in 0..n {
        for j in i + 1..n {
            if j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            let (a, b) = (&ws[i], &ws[j]);
            let lo = |w: &Wall, k: usize| w.a[k].min(w.b[k]);
            let hi = |w: &Wall, k: usize| w.a[k].max(w.b[k]);
            if lo(a, 0) <= hi(b, 0) + 0.01 && lo(b, 0) <= hi(a, 0) + 0.01 && lo(a, 1) <= hi(b, 1) + 0.01 && lo(b, 1) <= hi(a, 1) + 0.01 {
                return Err(tr_fmt!("Walls {} and {} cross", i + 1, j + 1));
            }
        }
    }
    Ok(())
}

/// Cleans up drawn corners (repeated corners, corners in the middle of a straight
/// wall) and checks the outline ([`check_outline`]).
pub fn normalize_outline(pts: &[[f32; 2]]) -> Result<Vec<[f32; 2]>, String> {
    let mut v: Vec<[f32; 2]> = Vec::new();
    for &q in pts {
        if v.last().is_none_or(|l| (l[0] - q[0]).abs() + (l[1] - q[1]).abs() > 0.01) {
            v.push(q);
        }
    }
    while v.len() > 1 && (v[0][0] - v[v.len() - 1][0]).abs() + (v[0][1] - v[v.len() - 1][1]).abs() <= 0.01 {
        v.pop();
    }
    loop {
        let n = v.len();
        let straight = (0..n).find(|&i| {
            let (p, q, r) = (v[(i + n - 1) % n], v[i], v[(i + 1) % n]);
            let in_line = ((p[0] - q[0]).abs() < 0.01 && (q[0] - r[0]).abs() < 0.01) || ((p[1] - q[1]).abs() < 0.01 && (q[1] - r[1]).abs() < 0.01);
            // Only a corner between the two (not where the wall runs back).
            in_line && (p[0] - q[0]) * (r[0] - q[0]) + (p[1] - q[1]) * (r[1] - q[1]) < 0.0
        });
        match straight {
            Some(i) if n > 4 => {
                v.remove(i);
            }
            _ => break,
        }
    }
    check_outline(&v)?;
    Ok(v)
}

/// Gives a room a new outline; its width and depth follow the outline's extent.
pub fn set_outline(e: &mut Enclosure, pts: &[[f32; 2]]) -> Result<(), String> {
    if !e.is_room() {
        return Err(tr_fmt!("{} is not a room", e.name));
    }
    let v = normalize_outline(pts)?;
    let (lo, hi) = bounds(&v);
    e.width = hi[0] - lo[0];
    e.depth = hi[1] - lo[1];
    e.outline = v;
    Ok(())
}

/// Corners of an outline with wall `i` moved by `offset` mm outwards (a negative
/// offset moves it into the room); its neighbours get longer or shorter.
pub fn moved_wall(pts: &[[f32; 2]], i: usize, offset: f32) -> Result<Vec<[f32; 2]>, String> {
    let ws = walls_of(pts);
    let w = ws.get(i).ok_or_else(|| tr_fmt!("No wall {}", i + 1))?;
    let mut v = pts.to_vec();
    let n = v.len();
    for k in [i, (i + 1) % n] {
        v[k][0] -= w.normal[0] * offset;
        v[k][1] -= w.normal[1] * offset;
    }
    check_outline(&v)?;
    Ok(v)
}

/// Corners of an outline with wall `i` made `length` mm long: its end moves along it,
/// the next wall with it.
pub fn wall_length_set(pts: &[[f32; 2]], i: usize, length: f32) -> Result<Vec<[f32; 2]>, String> {
    let ws = walls_of(pts);
    let w = ws.get(i).ok_or_else(|| tr_fmt!("No wall {}", i + 1))?;
    let (d, delta) = (w.dir(), length - w.length());
    let mut v = pts.to_vec();
    let n = v.len();
    for k in [(i + 1) % n, (i + 2) % n] {
        v[k][0] += d[0] * delta;
        v[k][1] += d[1] * delta;
    }
    check_outline(&v)?;
    Ok(v)
}

/// A new room: a rectangle of `width` × `depth` and `height` mm (inside).
pub fn new_room(prj: &mut Project, name: &str, width: f32, depth: f32, height: f32) -> Enclosure {
    let id = prj.new_id().max(1);
    Enclosure { id, name: name.into(), kind: EnclosureKind::Room, width: width.max(500.0), depth: depth.max(500.0), height: height.max(500.0), ..Default::default() }
}

/// Indices of the rooms of a layout.
pub fn rooms(layout: &Layout) -> Vec<usize> {
    (0..layout.enclosures.len()).filter(|&i| layout.enclosures[i].is_room()).collect()
}

/// The room the line-up of an enclosure stands in.
pub fn room_of(layout: &Layout, ei: usize) -> Option<usize> {
    let first = *lineup_of(layout, ei).first()?;
    let id = layout.enclosures[first].room;
    if id == 0 || layout.enclosures[first].is_room() {
        return None;
    }
    layout.enclosures.iter().position(|r| r.id == id && r.is_room())
}

/// The cabinets standing in a room.
pub fn cabinets_in(layout: &Layout, room: usize) -> Vec<usize> {
    (0..layout.enclosures.len()).filter(|&i| i != room && room_of(layout, i) == Some(room)).collect()
}

/// Where the line-up of a cabinet stands in its room's floor plan (also when it was
/// placed automatically); `None` outside rooms.
pub fn room_place(prj: &Project, ei: usize) -> Option<SitePlace> {
    let room = room_of(&prj.layout, ei)?;
    let first = *lineup_of(&prj.layout, ei).first()?;
    let poses = site_poses(prj);
    let local = poses[room].local(poses[first].at);
    Some(SitePlace { x: local[0], z: local[2], angle: (poses[first].angle - poses[room].angle).to_degrees() })
}

/// The place of a cabinet's line-up in its room turned by `degrees` (counter-clockwise
/// seen from above) about the centre of its footprint, so that it stays where it
/// stood; `None` outside rooms.
pub fn turned_place(prj: &Project, ei: usize, degrees: f32) -> Option<SitePlace> {
    let room = room_of(&prj.layout, ei)?;
    let place = room_place(prj, ei)?;
    let corners: Vec<[f32; 2]> = lineup_of(&prj.layout, ei).into_iter().flat_map(|c| footprint(prj, room, c)).collect();
    let (lo, hi) = bounds(&corners);
    let centre = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
    // The pose turns (x, z) by [[c, s], [-s, c]]; every point turns about the centre.
    let (s, c) = degrees.to_radians().sin_cos();
    let turn = |q: [f32; 2]| {
        let (dx, dz) = (q[0] - centre[0], q[1] - centre[1]);
        [centre[0] + dx * c + dz * s, centre[1] - dx * s + dz * c]
    };
    let [mut x, mut z] = turn([place.x, place.z]);
    // A cabinet that stood inside the room (against a wall, say) is pushed back inside
    // where turning would make it cross a wall.
    let (rlo, rhi) = bounds(&outline(&prj.layout.enclosures[room]));
    let inside = |lo: [f32; 2], hi: [f32; 2]| lo[0] >= rlo[0] - 0.5 && lo[1] >= rlo[1] - 0.5 && hi[0] <= rhi[0] + 0.5 && hi[1] <= rhi[1] + 0.5;
    let turned: Vec<[f32; 2]> = corners.iter().map(|&q| turn(q)).collect();
    let (tlo, thi) = bounds(&turned);
    if inside(lo, hi) && thi[0] - tlo[0] <= rhi[0] - rlo[0] && thi[1] - tlo[1] <= rhi[1] - rlo[1] {
        let shift = |lo: f32, hi: f32, min: f32, max: f32| if lo < min { min - lo } else if hi > max { max - hi } else { 0.0 };
        x += shift(tlo[0], thi[0], rlo[0], rhi[0]);
        z += shift(tlo[1], thi[1], rlo[1], rhi[1]);
    }
    Some(SitePlace { x: x.round(), z: z.round(), angle: (place.angle + degrees).rem_euclid(360.0) })
}

/// Puts the line-up of a cabinet into a room at a place of its floor plan (`None`:
/// along the rear of the room after the other line-ups), or takes it out of its room
/// (`room` `None`).
pub fn set_room(prj: &mut Project, ei: usize, room: Option<usize>, place: Option<SitePlace>) -> Result<(), String> {
    let n = prj.layout.enclosures.len();
    if ei >= n || room.is_some_and(|r| r >= n) {
        return Err(tr_fmt!("No enclosure {}", ei.max(room.unwrap_or(0))));
    }
    if prj.layout.enclosures[ei].is_room() {
        return Err(tr_fmt!("{} is a room and cannot stand in another room", prj.layout.enclosures[ei].name));
    }
    let room_id = match room {
        Some(r) if !prj.layout.enclosures[r].is_room() => return Err(tr_fmt!("{} is not a room", prj.layout.enclosures[r].name)),
        Some(r) => {
            if prj.layout.enclosures[r].id == 0 {
                prj.layout.enclosures[r].id = prj.new_id().max(1);
            }
            prj.layout.enclosures[r].id
        }
        None => 0,
    };
    let lu = lineup_of(&prj.layout, ei);
    for &i in &lu {
        prj.layout.enclosures[i].room = 0;
        prj.layout.enclosures[i].site = None;
    }
    if let Some(&first) = lu.first() {
        prj.layout.enclosures[first].room = room_id;
        prj.layout.enclosures[first].site = place;
    }
    Ok(())
}

/// Floor plan corners of a cabinet standing in a room (rear left, rear right, front
/// right, front left).
pub fn footprint(prj: &Project, room: usize, ci: usize) -> [[f32; 2]; 4] {
    let poses = site_poses(prj);
    let c = &prj.layout.enclosures[ci];
    let (rp, pose) = (poses[room], poses[ci]);
    [[0.0, 0.0], [c.width, 0.0], [c.width, c.depth], [0.0, c.depth]].map(|[x, z]| {
        let l = rp.local(pose.apply([x, 0.0, z]));
        [l[0], l[2]]
    })
}

/// A device or accessory on a room wall is mounted properly: on a rail of the same
/// wall, or its part is made for wall mounting. Other items (and items of cabinets)
/// always are.
pub fn wall_mount_ok(prj: &Project, e: &Enclosure, it: &PanelItem) -> bool {
    if !e.is_room() || !matches!(it.kind, ItemKind::Device | ItemKind::Accessory) {
        return true;
    }
    let c = it.center();
    let on_rail = e.items.iter().any(|r| r.kind == ItemKind::Rail && r.face == it.face && r.rect().contains(c));
    on_rail || part_by_number(prj, &it.part).is_some_and(|p| p.wall_mountable())
}

/// Mounts a device (by its tag) or a part (by its number, without a device tag) on a
/// wall or the ceiling of a room with its top left corner at `at`; on a rail there it
/// snaps onto the rail. Refused for parts that are neither on a rail nor made for
/// wall mounting.
pub fn mount_on_wall(prj: &mut Project, room: usize, what: &str, face: Face, at: P) -> Result<u64, String> {
    let e = prj.layout.enclosures.get(room).ok_or_else(|| tr_fmt!("No enclosure {}", room))?;
    if !e.is_room() {
        return Err(tr_fmt!("{} is not a room", e.name));
    }
    if !room_faces(e).contains(&face) {
        return Err(tr_fmt!("Unknown face: {}", room_face_key(e, face)));
    }
    let id = if prj.devices.contains_key(what) {
        place_device_on(prj, room, what, at, face).ok_or_else(|| tr_fmt!("No enclosure {}", room))?
    } else {
        if part_by_number(prj, what).is_none() {
            return Err(tr_fmt!("Unknown device or part: {}", what));
        }
        let item = super::new_accessory(prj, what, face, at);
        let id = item.id;
        prj.layout.enclosures[room].items.push(PanelItem { z: 0.0, ..item });
        id
    };
    let e = &prj.layout.enclosures[room];
    let it = e.item(id).cloned().unwrap_or_default();
    if !wall_mount_ok(prj, e, &it) {
        prj.layout.enclosures[room].items.retain(|i| i.id != id);
        return Err(tr_fmt!("{} is not made for wall mounting: place a mounting rail on the wall first", what));
    }
    Ok(id)
}

/// Height of an opening's bottom above the floor (0 for doors).
pub fn opening_sill(e: &Enclosure, it: &PanelItem) -> f32 {
    e.height - it.y - it.h
}

/// Adds a door or window to a wall of a room: its left edge `x` mm from the left end
/// of the wall (seen from inside), centred when `None`; default sizes and sill when
/// not given. A door stands on the floor.
pub fn add_opening(prj: &mut Project, room: usize, opening: Opening, face: Face, x: Option<f32>, size: Option<(f32, f32)>, sill: Option<f32>) -> Result<u64, String> {
    let e = prj.layout.enclosures.get(room).ok_or_else(|| tr_fmt!("No enclosure {}", room))?;
    if !e.is_room() {
        return Err(tr_fmt!("{} is not a room", e.name));
    }
    if !matches!(face, Face::Wall(_) | Face::Inner(..)) {
        return Err(tr("Doors and windows go into walls").to_string());
    }
    let wall_len = side(e, face).map(|w| w.length()).ok_or_else(|| tr_fmt!("Unknown face: {}", room_face_key(e, face)))?;
    let (w, h, sill) = match opening {
        Opening::Door => {
            let (w, h) = size.unwrap_or(DOOR_SIZE);
            (w, h, 0.0)
        }
        Opening::Window => {
            let (w, h) = size.unwrap_or(WINDOW_SIZE);
            (w, h, sill.unwrap_or(WINDOW_SILL))
        }
        Opening::Cutout => return Err(tr("Doors and windows go into walls").to_string()),
    };
    let x = snap(x.unwrap_or((wall_len - w) / 2.0));
    // Through the wall: an outer wall or the whole inner wall.
    let t = match face {
        Face::Inner(id, _) => e.inner_walls.iter().find(|w| w.id == id).map_or(INNER_WALL, |w| w.thickness),
        _ => e.wall(),
    };
    let y = e.height - sill - h;
    let id = prj.new_id();
    let item = PanelItem { id, kind: ItemKind::Cutout, opening, face, x, y, w, h, z: -t, d: t, ..Default::default() };
    prj.layout.enclosures[room].items.push(item);
    Ok(id)
}

/// Floor plan area a door swings through: as deep into the room as the door is wide.
pub fn door_zone(e: &Enclosure, it: &PanelItem) -> Option<[[f32; 2]; 4]> {
    if it.opening != Opening::Door {
        return None;
    }
    let w = side(e, it.face)?;
    let (s, r, nm) = (w.start(), w.right(), w.normal);
    let at = |along: f32, into: f32| [s[0] + r[0] * along + nm[0] * into, s[1] + r[1] * along + nm[1] * into];
    Some([at(it.x, 0.0), at(it.x + it.w, 0.0), at(it.x + it.w, it.w), at(it.x, it.w)])
}

/// Corners of an inner wall's footprint in the floor plan.
fn inner_footprint(w: &InnerWall) -> [[f32; 2]; 4] {
    let [p, q] = inner_sides(w);
    [p.a, p.b, q.b, q.a]
}

/// Checks an inner wall: along x or z, at least [`MIN_WALL`] long, thick, inside the
/// room's outline (it may end at a wall).
fn check_inner(e: &Enclosure, w: &InnerWall) -> Result<(), String> {
    let (dx, dz) = ((w.b[0] - w.a[0]).abs(), (w.b[1] - w.a[1]).abs());
    if dx > 0.01 && dz > 0.01 {
        return Err(tr("Inner walls run at right angles").to_string());
    }
    if dx + dz < MIN_WALL - 0.01 || w.thickness < 10.0 {
        return Err(tr_fmt!("Inner walls are at least {} mm long and 10 mm thick", MIN_WALL));
    }
    let pts = outline(e);
    let n = ((dx + dz) / 50.0).ceil().max(2.0) as usize;
    let inside = (0..=n).all(|k| {
        // Ends pulled in a little: an inner wall may meet a wall.
        let f = (k as f32 / n as f32).clamp(0.001, 0.999);
        inside_outline(&pts, [w.a[0] + (w.b[0] - w.a[0]) * f, w.a[1] + (w.b[1] - w.a[1]) * f])
    });
    if !inside {
        return Err(tr("The inner wall leaves the room").to_string());
    }
    Ok(())
}

/// Adds an inner wall from `a` to `b` (its centre line) to a room.
pub fn add_inner_wall(prj: &mut Project, room: usize, a: [f32; 2], b: [f32; 2], thickness: f32) -> Result<u64, String> {
    let e = prj.layout.enclosures.get(room).ok_or_else(|| tr_fmt!("No enclosure {}", room))?;
    if !e.is_room() {
        return Err(tr_fmt!("{} is not a room", e.name));
    }
    let w = InnerWall { id: prj.new_id().max(1), a, b, thickness };
    check_inner(&prj.layout.enclosures[room], &w)?;
    let id = w.id;
    prj.layout.enclosures[room].inner_walls.push(w);
    Ok(id)
}

/// Removes an inner wall with everything mounted on it.
pub fn remove_inner_wall(e: &mut Enclosure, id: u64) {
    e.inner_walls.retain(|w| w.id != id);
    e.items.retain(|i| !matches!(i.face, Face::Inner(w, _) if w == id));
}

/// An inner wall moved by `offset` mm across itself (towards its side 0), or given a
/// new length and thickness; checked like a new one.
pub fn moved_inner_wall(e: &Enclosure, id: u64, offset: f32, length: Option<f32>, thickness: Option<f32>) -> Result<InnerWall, String> {
    let mut w = e.inner_walls.iter().find(|w| w.id == id).cloned().ok_or_else(|| tr("No such inner wall").to_string())?;
    let n = inner_sides(&w)[0].normal;
    for q in [&mut w.a, &mut w.b] {
        q[0] += n[0] * offset;
        q[1] += n[1] * offset;
    }
    if let Some(l) = length {
        let d = Wall { a: w.a, b: w.b, normal: n }.dir();
        w.b = [w.a[0] + d[0] * l, w.a[1] + d[1] * l];
    }
    if let Some(t) = thickness {
        w.thickness = t;
    }
    check_inner(e, &w)?;
    Ok(w)
}

/// Two convex polygons overlap (more than touching).
fn convex_overlap(a: &[[f32; 2]], b: &[[f32; 2]]) -> bool {
    for poly in [a, b] {
        for i in 0..poly.len() {
            let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
            let axis = [q[1] - p[1], p[0] - q[0]];
            let range = |pts: &[[f32; 2]]| {
                pts.iter().map(|v| v[0] * axis[0] + v[1] * axis[1]).fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), d| (lo.min(d), hi.max(d)))
            };
            let ((a0, a1), (b0, b1)) = (range(a), range(b));
            let eps = 0.5 * (axis[0].abs() + axis[1].abs());
            if a1 <= b0 + eps || b1 <= a0 + eps {
                return false;
            }
        }
    }
    true
}

/// Adds a mounting rail or cable duct from `a` to `b` on a wall or the ceiling of a
/// room (horizontal or vertical by the dominant direction).
pub fn add_wall_profile(prj: &mut Project, room: usize, kind: ItemKind, part: &str, face: Face, a: P, b: P) -> Result<u64, String> {
    let e = prj.layout.enclosures.get(room).ok_or_else(|| tr_fmt!("No enclosure {}", room))?;
    if !e.is_room() {
        return Err(tr_fmt!("{} is not a room", e.name));
    }
    if !room_faces(e).contains(&face) || !matches!(kind, ItemKind::Rail | ItemKind::Duct) {
        return Err(tr_fmt!("Unknown face: {}", room_face_key(e, face)));
    }
    let snapped = |q: P| crate::geom::p(snap(q.x), snap(q.y));
    let item = PanelItem { face, ..new_profile(prj, kind, part, snapped(a), snapped(b)) };
    let id = item.id;
    prj.layout.enclosures[room].items.push(item);
    Ok(id)
}

/// True if the cabinet `ci` stands inside the floor plan of room `ri` (it may stand
/// against a wall, but not across one).
pub fn stands_inside(prj: &Project, ri: usize, ci: usize) -> bool {
    let pts = outline(&prj.layout.enclosures[ri]);
    let fp = footprint(prj, ri, ci);
    let centre = [fp.iter().map(|q| q[0]).sum::<f32>() / 4.0, fp.iter().map(|q| q[1]).sum::<f32>() / 4.0];
    // Corners pulled in a little (cabinets may stand against a wall), and no corner
    // of the room inside the footprint.
    let pulled = fp.map(|q| [q[0] + (centre[0] - q[0]).signum() * 0.5, q[1] + (centre[1] - q[1]).signum() * 0.5]);
    pulled.iter().all(|&q| inside_outline(&pts, q)) && !pts.iter().any(|&v| inside_outline(&pulled, v))
}

/// Checks of a room: devices on its walls that are neither on a rail nor made for
/// wall mounting, items on the ceiling outside the floor plan, and cabinets standing
/// outside the room (or across a wall) or higher than its ceiling.
pub(super) fn check(prj: &Project, e: &Enclosure, push: &mut impl FnMut(CheckKind, Severity, Vec<u64>, String)) {
    if !e.is_room() {
        return;
    }
    for it in e.items.iter().filter(|i| !wall_mount_ok(prj, e, i)) {
        push(
            CheckKind::Mounting,
            Severity::Warning,
            vec![it.id],
            tr_fmt!("{}: {} is not made for wall mounting, mount it on a rail", e.name, item_name(e, it.id)),
        );
    }
    let pts = outline(e);
    for (k, w) in e.inner_walls.iter().enumerate() {
        if let Err(msg) = check_inner(e, w) {
            push(CheckKind::Outside, Severity::Warning, Vec::new(), tr_fmt!("{}: inner wall {}: {}", e.name, k + 1, msg));
        }
    }
    let (lo, _) = bounds(&pts);
    for it in e.items.iter().filter(|i| i.face == Face::Roof) {
        let c = it.center();
        if !inside_outline(&pts, [lo[0] + c.x, lo[1] + c.y]) {
            push(CheckKind::Outside, Severity::Warning, vec![it.id], tr_fmt!("{}: {} outside the mounting plate / enclosure", e.name, item_name(e, it.id)));
        }
    }
    let layout = &prj.layout;
    let Some(ri) = layout.enclosures.iter().position(|x| std::ptr::eq(x, e)) else { return };
    for ci in cabinets_in(layout, ri) {
        for (kind, ids, msg) in cabinet_problems(prj, ri, ci) {
            push(kind, Severity::Warning, ids, msg);
        }
    }
}

/// What is wrong with the place of cabinet `ci` in room `ri`: standing in an inner
/// wall or the swing of a door (with the door's id), outside the room or across a
/// wall, higher than the ceiling.
pub fn cabinet_problems(prj: &Project, ri: usize, ci: usize) -> Vec<(CheckKind, Vec<u64>, String)> {
    let (e, c) = (&prj.layout.enclosures[ri], &prj.layout.enclosures[ci]);
    let poses = site_poses(prj);
    let fp = footprint(prj, ri, ci);
    let mut out = Vec::new();
    if e.inner_walls.iter().any(|w| convex_overlap(&fp, &inner_footprint(w))) {
        out.push((CheckKind::Collision, Vec::new(), tr_fmt!("{}: {} stands in an inner wall", e.name, c.name)));
    }
    for door in &e.items {
        let Some(zone) = door_zone(e, door) else { continue };
        if convex_overlap(&fp, &zone) {
            let wall = match door.face {
                Face::Wall(n) => n + 1,
                _ => 0,
            };
            out.push((CheckKind::Service, vec![door.id], tr_fmt!("{}: {} stands in the swing of the door in wall {}", e.name, c.name, wall)));
        }
    }
    let top = poses[ci].at[1] - poses[ri].at[1] + c.height;
    if !stands_inside(prj, ri, ci) {
        out.push((CheckKind::Outside, Vec::new(), tr_fmt!("{}: {} stands outside the room", e.name, c.name)));
    } else if top > e.height + 0.5 {
        out.push((CheckKind::Outside, Vec::new(), tr_fmt!("{}: {} is higher than the ceiling", e.name, c.name)));
    }
    out
}

#[cfg(test)]
mod tests;
