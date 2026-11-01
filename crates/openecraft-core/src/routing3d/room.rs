//! Routing through a room ([`crate::panel::rooms`]): the room's free space between its
//! walls, its inner walls, the cabinets standing in it and the swing of its doors, with
//! the cable ducts and pipe runs on its walls, ceiling and floor as channels; and the
//! way from a device in a cabinet into the room through a cable gland of the cabinet.
//! A route between ends in different line-ups of one room runs in legs: through the
//! cabinet to its gland, through the room, through the other cabinet; between rooms
//! also through the rooms between them and the wall penetrations joining them.
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
use std::collections::{HashMap, VecDeque};

/// Faces of a cabinet through which a route leaves it into the room.
const ROOM_SIDES: [Face; 4] = [Face::Roof, Face::Left, Face::Right, Face::Rear];
/// How far outside its cable gland a route through the room starts, mm.
const OUTSIDE: f32 = 60.0;
/// Smallest edge of the cells of a room's grid, mm: rooms are large, their routes
/// run along walls and pipe runs.
pub const ROOM_GRID: f32 = 50.0;
/// Routes in a room keep within this distance of its walls, its ceiling and what
/// stands in it, where they can.
pub const KEEP_TO_SURFACES: f32 = 300.0;

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
    s.add_enclosure(e, ri, o, rules, prj.layout.duct_fill_limit);
    s.mark_open(KEEP_TO_SURFACES);
    s.measure();
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
                let end = End { tag: g.part.clone(), enclosure: Some(k), point: Some(add(panel::face_point(e, g.face, [c[0], c[1], 5.0]), pos[k])), item: Some(g.id), ..Default::default() };
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
    (End { tag: tr("Roof").to_string(), enclosure: Some(ci), point: Some(inside), item: None, ..Default::default() }, add(outside, pos[ri]), true)
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

/// The rooms a route passes from room `a` to room `b`: the ways through their walls
/// in order, through as few walls as possible (empty within one room); `None` if no
/// chain of wall penetrations joins them.
pub(super) fn room_chain(links: &[panel::WallLink], a: usize, b: usize) -> Option<Vec<panel::WallLink>> {
    let both: Vec<panel::WallLink> = links.iter().flat_map(|l| [*l, l.reversed()]).collect();
    let mut came: HashMap<usize, Option<panel::WallLink>> = HashMap::from([(a, None)]);
    let mut queue = VecDeque::from([a]);
    while let Some(r) = queue.pop_front() {
        if r == b {
            break;
        }
        for l in both.iter().filter(|l| l.from == r) {
            if let std::collections::hash_map::Entry::Vacant(v) = came.entry(l.to) {
                v.insert(Some(*l));
                queue.push_back(l.to);
            }
        }
    }
    let mut chain = Vec::new();
    let mut r = b;
    while let Some(l) = *came.get(&r)? {
        chain.push(l);
        r = l.from;
    }
    chain.reverse();
    Some(chain)
}

/// Of the wall penetrations joining the rooms of `l`, the one with the shortest way
/// from `from` (room coordinates of `l.from`) through it to `to` (of `l.to`).
pub(super) fn nearest_link(links: &[panel::WallLink], l: panel::WallLink, from: V3, to: V3) -> panel::WallLink {
    let way = |k: &panel::WallLink| manhattan(from, k.a) + manhattan(k.b, to);
    links
        .iter()
        .flat_map(|k| [*k, k.reversed()])
        .filter(|k| k.from == l.from && k.to == l.to)
        .min_by(|a, b| way(a).total_cmp(&way(b)).then(a.item.cmp(&b.item)))
        .unwrap_or(l)
}

/// The legs of a route between two placed ends in rooms `rooms` (in a room itself or
/// in a cabinet standing there): from a device in a cabinet to its entry, through the
/// rooms and the wall penetrations between them (`links`: [`panel::wall_links`]), from
/// the other entry to the other device. Gives the legs, whether an entry is the middle
/// of a roof and the length of the ways through the walls.
#[allow(clippy::too_many_arguments)]
pub(super) fn plan_legs(prj: &Project, pos: &[V3], poses: &[SitePose], lineup_of: &dyn Fn(usize) -> Option<usize>, rooms: [usize; 2], links: &[panel::WallLink], ends: &[End; 2]) -> Option<(Vec<LegPlan>, bool, f32)> {
    let mut chain = if rooms[0] == rooms[1] { Vec::new() } else { room_chain(links, rooms[0], rooms[1])? };
    // An end in room coordinates of its room.
    let in_room = |ri: usize, end: &End| -> Option<V3> {
        let (k, q) = (end.enclosure?, end.point?);
        Some(if k == ri { sub(q, pos[ri]) } else { cabinet_in_room(prj, poses, ri, k, sub(q, pos[k])) })
    };
    let at = [in_room(rooms[0], &ends[0])?, in_room(rooms[1], &ends[1])?];
    // Through the nearest of the wall penetrations joining two rooms.
    for i in 0..chain.len() {
        let from = if i == 0 { at[0] } else { chain[i - 1].b };
        let to = chain.get(i + 1).map_or(at[1], |l| l.a);
        chain[i] = nearest_link(links, chain[i], from, to);
    }
    // What each end heads for in its room: the other end, or the wall penetration.
    let near = [chain.first().map_or(at[1], |l| l.a), chain.last().map_or(at[0], |l| l.b)];
    let mut roof = false;
    let mut through = [ends[0].point?, ends[1].point?];
    let mut legs: [Option<LegPlan>; 2] = [None, None];
    let mut glands: Vec<u64> = Vec::new();
    for i in 0..2 {
        let (k, ri) = (ends[i].enclosure?, rooms[i]);
        if k == ri {
            continue;
        }
        let (entry, start, by_roof) = room_entry(prj, pos, poses, ri, k, near[i]);
        roof |= by_roof;
        glands.extend(entry.item);
        let inside = entry.point?;
        let (a, b) = if i == 0 { (through[0], inside) } else { (inside, through[1]) };
        legs[i] = Some(LegPlan { lineup: lineup_of(entry.enclosure?)?, a, b, ignore: Vec::new() });
        through[i] = start;
    }
    // Through every room of the chain, from where the route comes in to where it leaves.
    let mut v: Vec<LegPlan> = legs[0].take().into_iter().collect();
    let mut from = (rooms[0], through[0]);
    for l in &chain {
        v.push(LegPlan { lineup: lineup_of(from.0)?, a: from.1, b: add(l.a, pos[l.from]), ignore: Vec::new() });
        from = (l.to, add(l.b, pos[l.to]));
    }
    v.push(LegPlan { lineup: lineup_of(from.0)?, a: from.1, b: through[1], ignore: Vec::new() });
    v.extend(legs[1].take());
    for leg in &mut v {
        leg.ignore = glands.clone();
    }
    Some((v, roof, chain.iter().map(|l| l.length).sum()))
}

#[cfg(test)]
mod tests;
