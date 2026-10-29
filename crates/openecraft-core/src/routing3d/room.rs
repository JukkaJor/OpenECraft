//! Routing through a room ([`crate::panel::rooms`]): the room's free space between its
//! walls, its inner walls, the cabinets standing in it and the swing of its doors, with
//! the cable ducts and pipe runs on its walls, ceiling and floor as channels; and the
//! way from a device in a cabinet into the room through a cable gland of the cabinet.
//! A route between ends in different line-ups of one room runs in legs: through the
//! cabinet to its gland, through the room, through the other cabinet.
//!
//! Room coordinates are those of the room's items (x and z of the floor plan, y down
//! from the ceiling); in the grid they are shifted to the room's place in the layout
//! ([`crate::panel::positions`]) like the coordinates of an enclosure.

use super::grid::Space;
use super::{accessories, End, RoutingRules};
use crate::model::Project;
use crate::panel::{self, Face, SitePose};
use crate::parts::vec3::{add, manhattan, sub, V3};
use openecraft_i18n::tr;

/// Faces of a cabinet through which a route leaves it into the room.
const ROOM_SIDES: [Face; 4] = [Face::Roof, Face::Left, Face::Right, Face::Rear];
/// How far outside its cable gland a route through the room starts, mm.
const OUTSIDE: f32 = 60.0;
/// Smallest edge of the cells of a room's grid, mm: rooms are large, their routes
/// run along walls and pipe runs.
pub const ROOM_GRID: f32 = 50.0;

/// A point of cabinet `ci` (enclosure coordinates) in room `ri` (room coordinates).
pub fn cabinet_in_room(prj: &Project, poses: &[SitePose], ri: usize, ci: usize, q: V3) -> V3 {
    let (r, c) = (&prj.layout.enclosures[ri], &prj.layout.enclosures[ci]);
    let l = poses[ri].local(poses[ci].apply([q[0], c.height - q[1], q[2]]));
    [l[0], r.height - l[1], l[2]]
}

/// The box cabinet `ci` takes in room `ri` (room coordinates), from the floor (with its
/// plinth) to its top.
pub fn cabinet_box(prj: &Project, poses: &[SitePose], ri: usize, ci: usize) -> (V3, V3) {
    let (r, c) = (&prj.layout.enclosures[ri], &prj.layout.enclosures[ci]);
    let (lo, hi) = panel::bounds(&panel::footprint(prj, ri, ci));
    let top = poses[ci].at[1] - poses[ri].at[1] + c.height;
    ([lo[0], r.height - top, lo[1]], [hi[0], r.height, hi[1]])
}

/// The grid of room `ri` (`pos`: [`panel::positions`], `poses`: [`panel::site_poses`]).
pub fn room_space(prj: &Project, ri: usize, pos: &[V3], poses: &[SitePose], rules: &RoutingRules) -> Space {
    let e = &prj.layout.enclosures[ri];
    let o = pos[ri];
    let pts = panel::outline(e);
    let (lo, hi) = panel::bounds(&pts);
    let mut s = Space::empty([o[0] + lo[0], o[1], o[2] + lo[1]], [o[0] + hi[0], o[1] + e.height, o[2] + hi[1]], rules.grid.max(ROOM_GRID));
    s.free_where(ri, |c| {
        let l = sub(c, o);
        l[1] > 0.0 && l[1] < e.height && panel::inside_outline(&pts, [l[0], l[2]])
    });
    let shift = |(a, b): (V3, V3)| (add(a, o), add(b, o));
    let plan_box = |fp: &[[f32; 2]], top: f32| {
        let (lo, hi) = panel::bounds(fp);
        shift(([lo[0], e.height - top, lo[1]], [hi[0], e.height, hi[1]]))
    };
    for w in &e.inner_walls {
        s.block_box(plan_box(&panel::inner_footprint(w), e.height));
    }
    for door in &e.items {
        if let Some(zone) = panel::door_zone(e, door) {
            s.block_box(plan_box(&zone, door.h));
        }
    }
    for ci in panel::cabinets_in(&prj.layout, ri) {
        s.block_box(shift(cabinet_box(prj, poses, ri, ci)));
    }
    s.add_enclosure(e, ri, o, rules);
    s
}

/// Where a route leaves the cabinet `ci` standing in room `ri` towards `near` (room
/// coordinates): the nearest cable gland on the roof, a side or the rear of the
/// cabinet, otherwise of its line-up, otherwise the middle of its roof (`true`). Gives
/// the end in the cabinet and where the way through the room starts (layout
/// coordinates).
pub(super) fn room_entry(prj: &Project, pos: &[V3], poses: &[SitePose], ri: usize, ci: usize, near: V3) -> (End, V3, bool) {
    let has_gland = |k: usize| accessories(prj, k, "Cable gland").iter().any(|g| ROOM_SIDES.contains(&g.face));
    let lineup = panel::lineup_of(&prj.layout, ci);
    let cabinets = if has_gland(ci) { vec![ci] } else { lineup };
    let mut best: Option<(f32, End, V3)> = None;
    for k in cabinets {
        let e = &prj.layout.enclosures[k];
        for g in accessories(prj, k, "Cable gland").into_iter().filter(|g| ROOM_SIDES.contains(&g.face)) {
            let c = [g.x + g.w / 2.0, g.y + g.h / 2.0];
            let outside = cabinet_in_room(prj, poses, ri, k, panel::face_point(e, g.face, [c[0], c[1], -OUTSIDE]));
            let d = manhattan(outside, near);
            if best.as_ref().is_none_or(|b| d < b.0) {
                let end = End { tag: g.part.clone(), enclosure: Some(k), point: Some(add(panel::face_point(e, g.face, [c[0], c[1], 5.0]), pos[k])), item: Some(g.id) };
                best = Some((d, end, add(outside, pos[ri])));
            }
        }
    }
    if let Some((_, end, at)) = best {
        return (end, at, false);
    }
    let e = &prj.layout.enclosures[ci];
    let c = [e.width / 2.0, e.depth / 2.0];
    let inside = add(panel::face_point(e, Face::Roof, [c[0], c[1], 5.0]), pos[ci]);
    let outside = cabinet_in_room(prj, poses, ri, ci, panel::face_point(e, Face::Roof, [c[0], c[1], -OUTSIDE]));
    (End { tag: tr("Roof").to_string(), enclosure: Some(ci), point: Some(inside), item: None }, add(outside, pos[ri]), true)
}

/// A stretch of a route to be routed in one line-up or room: from `a` to `b` (layout
/// coordinates), passing the items `ignore` (the cable glands it goes through).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct LegPlan {
    pub lineup: usize,
    pub a: V3,
    pub b: V3,
    pub ignore: Vec<u64>,
}

/// The legs of a route between two placed ends in different line-ups of room `ri` (or
/// in the room itself): from a device in a cabinet to its entry, through the room, from
/// the other entry to the other device. True if an entry is the middle of a roof.
pub(super) fn plan_legs(prj: &Project, pos: &[V3], poses: &[SitePose], lineup_of: &dyn Fn(usize) -> Option<usize>, ri: usize, ends: &[End; 2]) -> Option<(Vec<LegPlan>, bool)> {
    // The ends in room coordinates.
    let in_room = |end: &End| -> Option<V3> {
        let (k, q) = (end.enclosure?, end.point?);
        Some(if k == ri { sub(q, pos[ri]) } else { cabinet_in_room(prj, poses, ri, k, sub(q, pos[k])) })
    };
    let at = [in_room(&ends[0])?, in_room(&ends[1])?];
    let mut roof = false;
    let mut through = [ends[0].point?, ends[1].point?];
    let mut legs: [Option<LegPlan>; 2] = [None, None];
    let mut glands: Vec<u64> = Vec::new();
    for i in 0..2 {
        let k = ends[i].enclosure?;
        if k == ri {
            continue;
        }
        let (entry, start, by_roof) = room_entry(prj, pos, poses, ri, k, at[1 - i]);
        roof |= by_roof;
        glands.extend(entry.item);
        let inside = entry.point?;
        let (a, b) = if i == 0 { (through[0], inside) } else { (inside, through[1]) };
        legs[i] = Some(LegPlan { lineup: lineup_of(entry.enclosure?)?, a, b, ignore: Vec::new() });
        through[i] = start;
    }
    let room = LegPlan { lineup: lineup_of(ri)?, a: through[0], b: through[1], ignore: Vec::new() };
    let [first, last] = legs;
    let mut v: Vec<LegPlan> = first.into_iter().chain([room]).chain(last).collect();
    for leg in &mut v {
        leg.ignore = glands.clone();
    }
    Some((v, roof))
}

#[cfg(test)]
mod tests;
