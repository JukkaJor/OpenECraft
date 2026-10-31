//! The space of a line-up (or a room, see `super::room`) as a grid of cubic cells, and
//! the obstacle-aware shortest way through it (A* with turn costs). Cells are free
//! inside the enclosures (in front of the mounting plate, behind the door), blocked by
//! the walls, by devices, rails, busbars and accessories (with a clearance) and by
//! keep-out reservations. Cable ducts and routing paths are channels: the way through
//! them is cheaper than through free air; a cable too thick for a duct keeps out of it.
//! A route keeps half its thickness from what blocks the cells; a turn where its bend
//! (rounded with the bend radius) would not keep clear of them costs more. Walls between
//! bayed enclosures are crossed at cable passages only.

use super::{Class, PathKind, ReserveMode, RouteKind, RoutingRules};
use crate::model::Project;
use crate::panel::{self, Enclosure, ItemKind, PLATE_Z};
use crate::parts::vec3::{add, dist, sub, V3};
use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Cell blocked by a wall, the outside or a keep-out reservation.
const WALL: u8 = 2;
/// Cell blocked by an item (see `owner`): its clearance around it.
const ITEM: u8 = 1;
/// Cell blocked by the item itself (see `owner`).
const BODY: u8 = 3;
/// Cost factor of a channel that is already full.
const FULL: f32 = 10.0;
/// Upper limit of cells; a larger line-up gets coarser cells.
const MAX_CELLS: usize = 1_500_000;
/// Cost factor of a horizontal step in the open space of a room (see
/// [`Space::mark_open`]).
const OPEN: f32 = 10.0;
/// Cells of room measured around the blocked ones (see [`Space::measure`]).
const ROOM: u8 = 8;
/// How far a bend of radius r rounding a right-angled corner lies inside it: (√2 − 1) r.
pub const CORNER_CUT: f32 = std::f32::consts::SQRT_2 - 1.0;
/// Extra cost of a turn without room for its bend, in turns.
const TIGHT: f32 = 4.0;

/// A routing path in the grid and what runs through it.
#[derive(Clone, Debug, PartialEq)]
pub struct PathCells {
    pub enclosure: usize,
    pub id: u64,
    pub kind: PathKind,
    /// Allowed classes as bits; 0: all.
    pub classes: u8,
    /// Usable cross-section in mm² (without the reserve).
    pub capacity: f32,
    pub used: f32,
    pub items: usize,
}

/// Where wires may cross from one enclosure into the next: the boundary `x` and the
/// opening's `y` and `z` ranges.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Crossing {
    x: f32,
    y: [f32; 2],
    z: [f32; 2],
}

/// What is routed.
#[derive(Clone, Copy, Debug)]
pub struct Request<'a> {
    pub kind: RouteKind,
    pub class: Class,
    /// Items the ends are at: their cells are passable, those of the items themselves
    /// only near the start and the end `at` (a route leaves a device, not runs through it).
    pub ignore: &'a [u64],
    pub at: [V3; 2],
    /// Cross-section in mm² (capacity of the paths).
    pub area: f32,
    /// Extra cost of a turn (mm), from the bend radius.
    pub turn: f32,
    /// Half the outer diameter: the room the route keeps from what blocks the cells.
    pub half: f32,
}

/// A way through the grid.
#[derive(Clone, Debug, PartialEq)]
pub struct Way {
    /// Corner points in layout coordinates, from the start to the end point.
    pub points: Vec<V3>,
    pub cells: Vec<usize>,
    /// Length outside cable ducts and routing paths.
    pub free: f32,
}

pub struct Space {
    pub origin: V3,
    pub cell: f32,
    pub n: [usize; 3],
    block: Vec<u8>,
    owner: Vec<u64>,
    /// Inner size (mm) of the cable duct a cell lies in, 0 for none.
    duct: Vec<f32>,
    /// Cells from each cell to the nearest blocked one (at most [`ROOM`]); empty until
    /// [`Space::measure`]: room everywhere.
    room: Vec<u8>,
    /// Routing path of a cell: index + 1 into `paths`, 0 for none.
    path: Vec<u16>,
    /// Classes a reservation keeps the cell for (bits); 0: any.
    reserved: Vec<u8>,
    /// Classes routed through the cell.
    occ: Vec<u8>,
    /// Classes routed within the segregation distance.
    zone: Vec<u8>,
    pub paths: Vec<PathCells>,
    crossings: Vec<Crossing>,
    /// Enclosure of each cell column along x (index into the layout), `usize::MAX`
    /// outside.
    column: Vec<usize>,
    /// Cells in the open space of a room, away from its surfaces; empty in a line-up.
    open: Vec<bool>,
}

thread_local! {
    /// The arrays of the searches on this thread, kept from one search to the next
    /// until [`release_search`].
    static SEARCH: RefCell<Search> = RefCell::default();
}

/// Frees the arrays the searches on this thread kept.
pub fn release_search() {
    SEARCH.with_borrow_mut(|s| *s = Search::default());
}

/// The arrays of the search over all states (cell × direction) and, for the costs to
/// the goal, over all cells; they grow to the largest space searched. An entry belongs
/// to the current search only when its stamp is the search's.
#[derive(Default)]
struct Search {
    best: Vec<f32>,
    prev: Vec<u32>,
    stamp: Vec<u32>,
    /// Cost from the cell to the goal without turns, final where `settled`.
    to_goal: Vec<f32>,
    reached: Vec<u32>,
    settled: Vec<u32>,
    run: u32,
}

impl Search {
    /// Starts a search over `cells` cells.
    fn start(&mut self, cells: usize) -> u32 {
        if self.reached.len() < cells {
            *self = Search {
                best: vec![f32::INFINITY; cells * 7],
                prev: vec![u32::MAX; cells * 7],
                stamp: vec![0; cells * 7],
                to_goal: vec![f32::INFINITY; cells],
                reached: vec![0; cells],
                settled: vec![0; cells],
                run: 0,
            };
        }
        self.run = self.run.wrapping_add(1);
        if self.run == 0 {
            for v in [&mut self.stamp, &mut self.reached, &mut self.settled] {
                v.fill(0);
            }
            self.run = 1;
        }
        self.run
    }
}

impl Space {
    /// A grid over the box from `min` to `max` with every cell blocked, with cells of
    /// `cell` mm (coarser for large boxes); its columns belong to no enclosure.
    pub(super) fn empty(min: V3, max: V3, cell: f32) -> Space {
        let size = [0, 1, 2].map(|i| (max[i] - min[i]).max(1.0));
        let mut cell = cell.max(5.0);
        while size.iter().map(|s| (s / cell).ceil() as usize).product::<usize>() > MAX_CELLS {
            cell *= 1.25;
        }
        let n = size.map(|s| (s / cell).ceil().max(1.0) as usize);
        let total = n[0] * n[1] * n[2];
        Space {
            origin: min,
            cell,
            n,
            block: vec![WALL; total],
            owner: vec![0; total],
            duct: vec![0.0; total],
            room: Vec::new(),
            path: vec![0; total],
            reserved: vec![0; total],
            occ: vec![0; total],
            zone: vec![0; total],
            paths: Vec::new(),
            crossings: Vec::new(),
            column: vec![usize::MAX; n[0]],
            open: Vec::new(),
        }
    }

    /// Marks the free cells farther than `distance` from the walls, the ceiling (the
    /// top of the grid) and the items standing or hanging in the room as its open
    /// space: running through it horizontally costs more, so pipes and cables keep to
    /// the walls and the ceiling and drop straight to a device standing in the room.
    pub(super) fn mark_open(&mut self, distance: f32) {
        let k = (distance / self.cell).ceil() as u32;
        let total = self.block.len();
        let mut dist = vec![u32::MAX; total];
        let mut queue = std::collections::VecDeque::new();
        for (idx, d) in dist.iter_mut().enumerate() {
            if self.block[idx] != 0 || self.coords(idx)[1] == 0 {
                *d = 0;
                queue.push_back(idx);
            }
        }
        while let Some(idx) = queue.pop_front() {
            let d = dist[idx];
            if d >= k {
                continue;
            }
            let c = self.coords(idx);
            for axis in 0..3 {
                for up in [false, true] {
                    let mut nc = c;
                    if up {
                        if c[axis] + 1 >= self.n[axis] {
                            continue;
                        }
                        nc[axis] += 1;
                    } else {
                        if c[axis] == 0 {
                            continue;
                        }
                        nc[axis] -= 1;
                    }
                    let ni = self.index(nc);
                    if dist[ni] == u32::MAX {
                        dist[ni] = d + 1;
                        queue.push_back(ni);
                    }
                }
            }
        }
        self.open = dist.into_iter().map(|d| d == u32::MAX).collect();
    }

    /// The grid of a line-up (`pos`: [`panel::positions`]).
    pub fn build(prj: &Project, lineup: &[usize], pos: &[V3], rules: &RoutingRules) -> Space {
        let encs = &prj.layout.enclosures;
        let (mut min, mut max) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        for &k in lineup {
            let e = &encs[k];
            let far = [pos[k][0] + e.width, pos[k][1] + e.height, pos[k][2] + e.depth];
            for i in 0..3 {
                min[i] = min[i].min(pos[k][i]);
                max[i] = max[i].max(far[i]);
            }
        }
        if lineup.is_empty() {
            (min, max) = ([0.0; 3], [1.0; 3]);
        }
        let mut s = Space::empty(min, max, rules.grid);
        let (n, total, cell) = (s.n, s.block.len(), s.cell);
        for i in 0..n[0] {
            let x = s.origin[0] + (i as f32 + 0.5) * cell;
            s.column[i] = lineup.iter().copied().find(|&k| x >= pos[k][0] && x < pos[k][0] + encs[k].width).unwrap_or(usize::MAX);
        }
        // Inside the enclosures: in front of the mounting plate, behind the door.
        for idx in 0..total {
            let c = s.centre(idx);
            let k = s.column[s.coords(idx)[0]];
            if k == usize::MAX {
                continue;
            }
            let e = &encs[k];
            let l = [c[0] - pos[k][0], c[1] - pos[k][1], c[2] - pos[k][2]];
            let t = e.wall();
            if l[0] > t && l[0] < e.width - t && l[1] > t && l[1] < e.height - t && l[2] > PLATE_Z && l[2] < e.depth - t {
                s.block[idx] = 0;
            }
        }
        for &k in lineup {
            s.add_enclosure(&encs[k], k, pos[k], rules);
        }
        for w in lineup.windows(2) {
            for x in panel::passages(&prj.layout, pos, w[0]) {
                let (y, z) = ([x.at[1] - x.half[0], x.at[1] + x.half[0]], [x.at[2] - x.half[1], x.at[2] + x.half[1]]);
                s.crossings.push(Crossing { x: x.at[0], y, z });
            }
        }
        s.measure();
        s
    }

    /// Adds what is in enclosure `k` (at `o`, enclosure coordinates + `o` = grid
    /// coordinates) to the free space: its items, reservations and routing paths.
    /// Cable ducts are channels on the mounting plate, in a room on every face.
    pub(super) fn add_enclosure(&mut self, e: &Enclosure, k: usize, o: V3, rules: &RoutingRules) {
        let shift = |(a, b): (V3, V3), m: f32| ([a[0] + o[0] - m, a[1] + o[1] - m, a[2] + o[2] - m], [b[0] + o[0] + m, b[1] + o[1] + m, b[2] + o[2] + m]);
        for it in &e.items {
            let bx = shift(panel::item_box(e, it), 0.0);
            match it.kind {
                ItemKind::Device | ItemKind::Rail | ItemKind::Busbar | ItemKind::Accessory => {
                    let around = shift(panel::item_box(e, it), rules.clearance);
                    for idx in self.cells_in(around) {
                        if self.block[idx] == 0 {
                            self.block[idx] = ITEM;
                            self.owner[idx] = it.id;
                        }
                    }
                    for idx in self.cells_in(bx) {
                        if self.block[idx] == ITEM && self.owner[idx] == it.id {
                            self.block[idx] = BODY;
                        }
                    }
                }
                ItemKind::Duct if it.face == panel::Face::Plate || e.is_room() => {
                    // Its inside: across and as deep as it is, less its walls.
                    let size = (it.w.min(it.h).min(it.d) - 4.0).max(1.0);
                    self.cells_in(bx).into_iter().for_each(|idx| self.duct[idx] = size);
                }
                _ => {}
            }
        }
        for r in &e.reservations {
            let bx = shift((r.min, r.max), 0.0);
            for idx in self.cells_in(bx) {
                match r.mode {
                    ReserveMode::Keepout => self.block[idx] = WALL,
                    ReserveMode::For(class) => self.reserved[idx] |= class.bit(),
                }
            }
        }
        for p in &e.paths {
            let pts: Vec<V3> = p.points.iter().map(|q| [q[0] + o[0], q[1] + o[1], q[2] + o[2]]).collect();
            if pts.len() < 2 {
                continue;
            }
            self.paths.push(PathCells {
                enclosure: k,
                id: p.id,
                kind: p.kind,
                classes: p.classes.iter().fold(0, |m, c| m | c.bit()),
                capacity: p.capacity(),
                used: 0.0,
                items: 0,
            });
            let mark = self.paths.len() as u16;
            // At least the cells around the centre line.
            let r = (p.width / 2.0).max(self.cell * 0.75);
            for idx in self.cells_near(&pts, r) {
                if self.path[idx] == 0 {
                    self.path[idx] = mark;
                }
            }
        }
    }

    /// Frees the cells whose centre passes the test, and lets their columns belong to
    /// enclosure `k`.
    pub(super) fn free_where(&mut self, k: usize, inside: impl Fn(V3) -> bool) {
        for idx in 0..self.block.len() {
            if inside(self.centre(idx)) {
                self.block[idx] = 0;
                let x = self.coords(idx)[0];
                self.column[x] = k;
            }
        }
    }

    /// Measures the room around the blocked cells (once the space is complete): a cell
    /// k cells from the nearest blocked one along every axis has `k` (the cube around it
    /// of k - 1 cells is free).
    pub(super) fn measure(&mut self) {
        let total = self.block.len();
        let mut covered: Vec<bool> = self.block.iter().map(|&b| b != 0).collect();
        let mut room: Vec<u8> = covered.iter().map(|&c| if c { 0 } else { ROOM }).collect();
        let strides = [1, self.n[0], self.n[0] * self.n[1]];
        for k in 1..ROOM {
            for (&stride, &n) in strides.iter().zip(&self.n) {
                let mut grown = covered.clone();
                for idx in (0..total).filter(|&i| covered[i]) {
                    let c = (idx / stride) % n;
                    if c > 0 {
                        grown[idx - stride] = true;
                    }
                    if c + 1 < n {
                        grown[idx + stride] = true;
                    }
                }
                covered = grown;
            }
            for idx in 0..total {
                if covered[idx] && room[idx] == ROOM {
                    room[idx] = k;
                }
            }
        }
        self.room = room;
    }

    /// Room (mm) from the centre of a cell to the nearest blocked cell.
    fn room_mm(&self, idx: usize) -> f32 {
        self.room.get(idx).map_or(f32::INFINITY, |&k| if k >= ROOM { f32::INFINITY } else { (k as f32 - 0.5) * self.cell })
    }

    /// The cell lies near an end of the request: a route leaves its device there.
    fn near_end(&self, idx: usize, req: &Request) -> bool {
        req.at.iter().any(|q| dist(self.centre(idx), *q) <= 3.0 * self.cell + req.half)
    }

    /// The bend radius each corner of a way can have (`radius` at most, less where it
    /// would come nearer than half the thickness to a blocked cell); 0 at the ends.
    pub fn corner_radii(&self, points: &[V3], half: f32, radius: f32) -> Vec<f32> {
        let n = points.len();
        (0..n)
            .map(|i| {
                if i == 0 || i + 1 == n {
                    return 0.0;
                }
                let room = self.cell_of(points[i]).map_or(0.0, |idx| self.room_mm(idx)) - half;
                radius.min((room / CORNER_CUT).max(0.0))
            })
            .collect()
    }

    /// Blocks the cells whose centre lies in a box (walls, cabinets, door swings).
    pub(super) fn block_box(&mut self, bx: (V3, V3)) {
        for idx in self.cells_in(bx) {
            self.block[idx] = WALL;
        }
    }

    /// The cell a point lies in is blocked by a wall or an item (or outside the grid).
    #[cfg(test)]
    pub(super) fn blocked(&self, q: V3) -> bool {
        self.cell_of(q).is_none_or(|i| self.block[i] != 0)
    }

    pub fn coords(&self, idx: usize) -> [usize; 3] {
        [idx % self.n[0], (idx / self.n[0]) % self.n[1], idx / (self.n[0] * self.n[1])]
    }

    fn index(&self, c: [usize; 3]) -> usize {
        c[0] + self.n[0] * (c[1] + self.n[1] * c[2])
    }

    pub fn centre(&self, idx: usize) -> V3 {
        let c = self.coords(idx);
        [0, 1, 2].map(|i| self.origin[i] + (c[i] as f32 + 0.5) * self.cell)
    }

    /// The cell a point lies in.
    pub fn cell_of(&self, q: V3) -> Option<usize> {
        let mut c = [0usize; 3];
        for i in 0..3 {
            let v = ((q[i] - self.origin[i]) / self.cell).floor();
            if v < 0.0 || v as usize >= self.n[i] {
                return None;
            }
            c[i] = v as usize;
        }
        Some(self.index(c))
    }

    /// Cells whose centre lies in a box.
    fn cells_in(&self, (a, b): (V3, V3)) -> Vec<usize> {
        let range = |i: usize| {
            let lo = ((a[i] - self.origin[i]) / self.cell - 0.5).ceil().max(0.0) as usize;
            let hi = ((b[i] - self.origin[i]) / self.cell - 0.5).floor();
            (lo, if hi < 0.0 { None } else { Some((hi as usize).min(self.n[i] - 1)) })
        };
        let ([x0, y0, z0], [x1, y1, z1]) = ([0, 1, 2].map(|i| range(i).0), [0, 1, 2].map(|i| range(i).1));
        let (Some(x1), Some(y1), Some(z1)) = (x1, y1, z1) else { return Vec::new() };
        let mut v = Vec::new();
        for z in z0..=z1 {
            for y in y0..=y1 {
                for x in x0..=x1 {
                    v.push(self.index([x, y, z]));
                }
            }
        }
        v
    }

    /// Cells whose centre lies within `r` of a polyline.
    fn cells_near(&self, pts: &[V3], r: f32) -> Vec<usize> {
        let mut v = Vec::new();
        for s in pts.windows(2) {
            let lo = [0, 1, 2].map(|i| s[0][i].min(s[1][i]) - r);
            let hi = [0, 1, 2].map(|i| s[0][i].max(s[1][i]) + r);
            for idx in self.cells_in((lo, hi)) {
                if dist_to_segment(self.centre(idx), s[0], s[1]) <= r + 1e-3 {
                    v.push(idx);
                }
            }
        }
        v.sort_unstable();
        v.dedup();
        v
    }

    /// Cells along a polyline, in order, without repeats.
    pub fn cells_along(&self, pts: &[V3]) -> Vec<usize> {
        let mut v: Vec<usize> = Vec::new();
        for s in pts.windows(2) {
            let len = dist(s[0], s[1]);
            let steps = (len / (self.cell * 0.5)).ceil().max(1.0) as usize;
            for k in 0..=steps {
                let t = k as f32 / steps as f32;
                let q = [0, 1, 2].map(|i| s[0][i] + (s[1][i] - s[0][i]) * t);
                if let Some(idx) = self.cell_of(q) {
                    if v.last() != Some(&idx) {
                        v.push(idx);
                    }
                }
            }
        }
        v
    }

    fn passable(&self, idx: usize, req: &Request) -> bool {
        let free = match self.block[idx] {
            0 => true,
            ITEM => req.ignore.contains(&self.owner[idx]),
            BODY => req.ignore.contains(&self.owner[idx]) && req.at.iter().any(|q| dist(self.centre(idx), *q) <= 2.0 * self.cell),
            _ => false,
        };
        // As thick as it is: room around it, and only through ducts it fits into.
        let fits = req.kind == RouteKind::Wire || ((self.duct[idx] == 0.0 || self.duct[idx] >= 2.0 * req.half) && (self.room_mm(idx) >= req.half || self.near_end(idx, req)));
        free && fits && (self.reserved[idx] == 0 || self.reserved[idx] & req.class.bit() != 0)
    }

    /// A route may turn at the cell: its bend keeps half its thickness from the blocked
    /// cells (near its ends it may turn anywhere).
    fn turn_room(&self, idx: usize, req: &Request) -> bool {
        req.kind == RouteKind::Wire || self.room_mm(idx) >= req.half + CORNER_CUT * req.turn || self.near_end(idx, req)
    }

    /// The path a cell belongs to, if the request may use it as a channel.
    fn path_for(&self, idx: usize, req: &Request) -> Option<&PathCells> {
        let p = self.paths.get((self.path[idx] as usize).checked_sub(1)?)?;
        (p.kind.accepts(req.kind) && (p.classes == 0 || p.classes & req.class.bit() != 0)).then_some(p)
    }

    /// The cell is a channel for the request: a cable duct (not for tubes) or one of
    /// its routing paths.
    fn channel(&self, idx: usize, req: &Request) -> bool {
        (self.duct[idx] > 0.0 && req.kind != RouteKind::Tube) || self.path_for(idx, req).is_some()
    }

    /// Cost factor of a step into the cell (`axis` 1: up or down).
    fn factor(&self, idx: usize, axis: usize, req: &Request, rules: &RoutingRules) -> f32 {
        let channel = self.channel(idx, req);
        let mut f = if channel { 1.0 } else { rules.free_factor.max(1.0) };
        if !channel && axis != 1 && self.open.get(idx) == Some(&true) {
            f *= OPEN;
        }
        if self.path_for(idx, req).is_some_and(|p| p.capacity > 0.0 && p.used + req.area > p.capacity) {
            f *= FULL;
        }
        if self.zone[idx] & req.class.apart_bits() != 0 {
            f *= rules.segregation_factor.max(1.0);
        }
        f
    }

    /// Crossing between the cells `a` and `b` (neighbours along x) is allowed: in the
    /// same enclosure, or through a cable passage.
    fn crossing_ok(&self, a: usize, b: usize) -> bool {
        let (ca, cb) = (self.coords(a), self.coords(b));
        if self.column[ca[0]] == self.column[cb[0]] {
            return true;
        }
        let (pa, pb) = (self.centre(a), self.centre(b));
        let tol = self.cell * 0.5;
        self.crossings.iter().any(|c| {
            c.x >= pa[0].min(pb[0]) - 1e-3 && c.x <= pa[0].max(pb[0]) + 1e-3 && pa[1] >= c.y[0] - tol && pa[1] <= c.y[1] + tol && pa[2] >= c.z[0] - tol && pa[2] <= c.z[1] + tol
        })
    }

    /// The nearest passable cell to a point (searching a few cells around it).
    fn nearest_passable(&self, q: V3, req: &Request) -> Option<usize> {
        let r = self.cell * 4.0;
        let lo = [q[0] - r, q[1] - r, q[2] - r];
        let hi = [q[0] + r, q[1] + r, q[2] + r];
        if let Some(idx) = self.cell_of(q).filter(|&i| self.passable(i, req)) {
            return Some(idx);
        }
        self.cells_in((lo, hi)).into_iter().filter(|&i| self.passable(i, req)).min_by(|&a, &b| dist(self.centre(a), q).total_cmp(&dist(self.centre(b), q)).then(a.cmp(&b)))
    }

    /// The cheapest way from `a` to `b`, or `None` if there is none.
    pub fn route(&self, a: V3, b: V3, req: &Request, rules: &RoutingRules) -> Option<Way> {
        let start = self.nearest_passable(a, req)?;
        let goal = self.nearest_passable(b, req)?;
        // Cables and tubes turn only after half their bend radius straight on (but at
        // the start), where there is a way like that.
        let straight = if req.kind == RouteKind::Wire { 0 } else { (req.turn * 0.5 / self.cell).ceil() as usize };
        SEARCH.with_borrow_mut(|search| {
            self.search(search, a, b, start, goal, req, rules, straight).or_else(|| (straight > 0).then(|| self.search(search, a, b, start, goal, req, rules, 0)).flatten())
        })
    }

    /// The A* search of [`Space::route`] with the arrays of `search`; it turns only
    /// after `straight` cells straight on since the last turn or from the start.
    #[allow(clippy::too_many_arguments)]
    fn search(&self, search: &mut Search, a: V3, b: V3, start: usize, goal: usize, req: &Request, rules: &RoutingRules, straight: usize) -> Option<Way> {
        // States: cell × direction it was entered in (6: none).
        let run = search.start(self.block.len());
        let reach = self.costs_to_goal(search, run, start, goal, req, rules);
        let Search { best, prev, stamp, to_goal, settled, .. } = search;
        let mut heap = BinaryHeap::new();
        let gc = self.coords(goal);
        // Lower bound of the rest: the cost of the cells to the goal (at least `reach`
        // where it is not known) and the turns, one fewer than the axes it still lies
        // apart on when heading towards it along one of them.
        let h = |state: usize| {
            let (idx, dir) = (state / 7, state % 7);
            let c = self.coords(idx);
            let apart = (0..3).filter(|&i| c[i] != gc[i]).count();
            let towards = dir < 6 && {
                let axis = dir / 2;
                if dir % 2 == 1 { c[axis] < gc[axis] } else { c[axis] > gc[axis] }
            };
            let turns = if apart == 0 { 0 } else if dir == 6 || towards { apart - 1 } else { apart };
            let cells = if settled[idx] == run { to_goal[idx] } else { reach.max((0..3).map(|i| c[i].abs_diff(gc[i])).sum::<usize>() as f32 * self.cell) };
            cells + turns as f32 * req.turn
        };
        let key = |g: f32, state: usize| Reverse(((g + h(state)).to_bits(), state));
        let first = start * 7 + 6;
        (best[first], prev[first], stamp[first]) = (0.0, u32::MAX, run);
        heap.push(key(0.0, first));
        let mut found = None;
        while let Some(Reverse(popped)) = heap.pop() {
            let state = popped.1;
            let (idx, dir) = (state / 7, state % 7);
            let g = best[state];
            // A state reached again more cheaply was expanded with its better cost.
            if key(g, state).0 != popped {
                continue;
            }
            if idx == goal {
                found = Some(state);
                break;
            }
            let c = self.coords(idx);
            for nd in 0..6 {
                let (axis, up) = (nd / 2, nd % 2 == 1);
                let mut nc = c;
                if up {
                    if c[axis] + 1 >= self.n[axis] {
                        continue;
                    }
                    nc[axis] += 1;
                } else {
                    if c[axis] == 0 {
                        continue;
                    }
                    nc[axis] -= 1;
                }
                let ni = self.index(nc);
                if !self.passable(ni, req) || (axis == 0 && !self.crossing_ok(idx, ni)) {
                    continue;
                }
                let turning = dir != 6 && dir != nd;
                if turning && straight > 0 && !straight_on(prev, state, straight) {
                    continue;
                }
                let turn = match (turning, turning && self.turn_room(idx, req)) {
                    (false, _) => 0.0,
                    (true, true) => req.turn,
                    (true, false) => req.turn * (1.0 + TIGHT),
                };
                let ng = g + self.cell * self.factor(ni, axis, req, rules) + turn;
                let ns = ni * 7 + nd;
                if stamp[ns] != run || ng < best[ns] {
                    (best[ns], prev[ns], stamp[ns]) = (ng, state as u32, run);
                    heap.push(key(ng, ns));
                }
            }
        }
        let mut state = found?;
        let mut cells: Vec<usize> = vec![state / 7];
        while prev[state] != u32::MAX {
            state = prev[state] as usize;
            cells.push(state / 7);
        }
        cells.reverse();
        let mut points = vec![a];
        points.extend(cells.iter().map(|&i| self.centre(i)));
        points.push(b);
        let mut points = simplify(points);
        if self.spread_bends(&mut points, req, rules) {
            let inner: Vec<V3> = std::iter::once(self.centre(start)).chain(points[1..points.len() - 1].iter().copied()).chain(std::iter::once(self.centre(goal))).collect();
            cells = self.cells_along(&inner);
        }
        let free = cells.iter().filter(|&&i| !self.channel(i, req)).count() as f32 * self.cell;
        Some(Way { points, cells, free })
    }

    /// Moves the bends of a way apart where its stretches between bends are shorter
    /// than half the turn (the bend radius): a stretch moves to the start or the end
    /// of the way (the corners between move with it) or changes places with the next
    /// one, where that costs at most a turn more, until no stretch inside the way is
    /// too short or no move helps. Whether the way changed.
    fn spread_bends(&self, points: &mut Vec<V3>, req: &Request, rules: &RoutingRules) -> bool {
        let min = req.turn * 0.5;
        // How much the stretches between bends lack (the first and the last count not),
        // and how many lack anything.
        let lack = |p: &[V3]| -> (usize, f32) {
            if p.len() < 6 {
                return (0, 0.0);
            }
            p[2..p.len() - 2].windows(2).map(|s| (min - dist(s[0], s[1])).max(0.0)).fold((0, 0.0), |(n, sum), l| (n + usize::from(l > 0.0), sum + l))
        };
        let heading = |a: V3, b: V3| [0, 1, 2].map(|i| (b[i] - a[i]).signum() as i8);
        let turns = |p: &[V3]| p.windows(3).filter(|w| heading(w[0], w[1]) != heading(w[1], w[2])).count();
        // Cost of the way between the cell centres (`None` where it is blocked).
        let cost = |p: &[V3]| -> Option<f32> {
            let inner = &p[1..p.len() - 1];
            if inner.iter().any(|&q| self.cell_of(q).is_none()) {
                return None;
            }
            let cells = self.cells_along(inner);
            let free = cells.iter().all(|&i| self.passable(i, req)) && cells.windows(2).all(|w| self.coords(w[0])[0] == self.coords(w[1])[0] || self.crossing_ok(w[0], w[1]));
            let axis = |a: usize, b: usize| (0..3).find(|&k| self.coords(a)[k] != self.coords(b)[k]).unwrap_or(0);
            free.then(|| cells.windows(2).map(|w| self.factor(w[1], axis(w[0], w[1]), req, rules)).sum::<f32>() * self.cell + turns(p) as f32 * req.turn)
        };
        let mut changed = false;
        for _ in 0..16 {
            let now = lack(points);
            if now.0 == 0 {
                break;
            }
            let Some(base) = cost(points) else { break };
            let n = points.len();
            let mut candidates: Vec<Vec<V3>> = Vec::new();
            // Stretch k (between the cell centres k and k + 1) to the end or to the start,
            // or changed with the next one.
            for k in 1..n.saturating_sub(2) {
                let step = sub(points[k + 1], points[k]);
                if k + 2 < n - 1 {
                    let mut alt = points[..=k].to_vec();
                    alt.extend(points[k + 2..n - 1].iter().map(|q| sub(*q, step)));
                    alt.extend_from_slice(&points[n - 2..]);
                    candidates.push(alt);
                }
                if k > 1 {
                    let mut alt = points[..2].to_vec();
                    alt.extend(points[1..=k].iter().map(|q| add(*q, step)));
                    alt.extend_from_slice(&points[k + 2..]);
                    candidates.push(alt);
                }
                if k + 2 < n - 1 {
                    let mut alt = points.clone();
                    alt[k + 1] = add(points[k], sub(points[k + 2], points[k + 1]));
                    candidates.push(alt);
                }
            }
            let better = candidates.into_iter().map(simplify).filter(|alt| {
                let l = lack(alt);
                (l.0, l.1) < (now.0, now.1) && cost(alt).is_some_and(|c| c <= base + req.turn)
            });
            let Some(best) = better.min_by(|a, b| lack(a).1.total_cmp(&lack(b).1)) else { break };
            *points = best;
            changed = true;
        }
        changed
    }

    /// The costs from the cells to `goal` without turns (Dijkstra backwards), until
    /// the one of `start` is known. Gives the highest known cost: the other cells cost
    /// at least as much.
    fn costs_to_goal(&self, search: &mut Search, run: u32, start: usize, goal: usize, req: &Request, rules: &RoutingRules) -> f32 {
        let Search { to_goal, reached, settled, .. } = search;
        let mut heap = BinaryHeap::new();
        (to_goal[goal], reached[goal]) = (0.0, run);
        heap.push(Reverse((0.0f32.to_bits(), goal)));
        let mut reach = 0.0;
        while let Some(Reverse((d, idx))) = heap.pop() {
            if settled[idx] == run {
                continue;
            }
            settled[idx] = run;
            reach = f32::from_bits(d);
            if idx == start {
                break;
            }
            // Entering this cell from a neighbour costs its factor.
            let c = self.coords(idx);
            for nd in 0..6 {
                let (axis, up) = (nd / 2, nd % 2 == 1);
                let mut nc = c;
                if up {
                    if c[axis] + 1 >= self.n[axis] {
                        continue;
                    }
                    nc[axis] += 1;
                } else {
                    if c[axis] == 0 {
                        continue;
                    }
                    nc[axis] -= 1;
                }
                let ni = self.index(nc);
                if settled[ni] == run || !self.passable(ni, req) || (axis == 0 && !self.crossing_ok(ni, idx)) {
                    continue;
                }
                let nd = reach + self.cell * self.factor(idx, axis, req, rules);
                if reached[ni] != run || nd < to_goal[ni] {
                    (to_goal[ni], reached[ni]) = (nd, run);
                    heap.push(Reverse((nd.to_bits(), ni)));
                }
            }
        }
        reach
    }

    /// For a route of cross-section `area`, the routing paths are full or not as they
    /// were with the cross-sections `before` in them (`used` of [`Space::paths`]).
    pub fn paths_full_as(&self, before: &[f32], area: f32) -> bool {
        let full = |used: f32, p: &PathCells| p.capacity > 0.0 && used + area > p.capacity;
        before.len() == self.paths.len() && self.paths.iter().zip(before).all(|(p, &used)| full(used, p) == full(p.used, p))
    }

    /// A way that was routed before (frozen) is still possible: its cells are passable
    /// and it does not cross a wall outside the passages.
    pub fn still_free(&self, points: &[V3], req: &Request) -> bool {
        let cells = self.cells_along(points);
        // The ends may lie in the walls (cable glands) or in their devices.
        let inner = if cells.len() > 2 { &cells[1..cells.len() - 1] } else { &[][..] };
        inner.iter().all(|&i| self.passable(i, req)) && cells.windows(2).all(|w| self.coords(w[0])[0] == self.coords(w[1])[0] || self.crossing_ok(w[0], w[1]))
    }

    /// Registers what runs through the cells: its class (segregation) and its
    /// cross-section in the routing paths it uses.
    pub fn occupy(&mut self, cells: &[usize], class: Class, area: f32) {
        let mut used: Vec<u16> = Vec::new();
        for &i in cells {
            self.occ[i] |= class.bit();
            if self.path[i] != 0 && !used.contains(&self.path[i]) {
                used.push(self.path[i]);
            }
        }
        for p in used {
            let pc = &mut self.paths[p as usize - 1];
            pc.used += area;
            pc.items += 1;
        }
    }

    /// Recomputes the segregation zones: every cell within `distance` of a cell that
    /// classes run through.
    pub fn update_zones(&mut self, distance: f32) {
        let k = (distance / self.cell).ceil().max(0.0) as usize;
        let mut zone = self.occ.clone();
        for axis in 0..3 {
            let src = zone.clone();
            for (idx, &bits) in src.iter().enumerate() {
                if bits == 0 {
                    continue;
                }
                let c = self.coords(idx);
                let lo = c[axis].saturating_sub(k);
                let hi = (c[axis] + k).min(self.n[axis] - 1);
                for v in lo..=hi {
                    let mut nc = c;
                    nc[axis] = v;
                    let ni = self.index(nc);
                    zone[ni] |= bits;
                }
            }
        }
        self.zone = zone;
    }

    /// Classes routed through the cells (for checks).
    pub fn classes_at(&self, cells: &[usize]) -> u8 {
        cells.iter().fold(0, |m, &i| m | self.occ[i])
    }
}

fn dist_to_segment(q: V3, a: V3, b: V3) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let l2 = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
    let t = if l2 <= f32::EPSILON { 0.0 } else { (((q[0] - a[0]) * ab[0] + (q[1] - a[1]) * ab[1] + (q[2] - a[2]) * ab[2]) / l2).clamp(0.0, 1.0) };
    dist(q, [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t])
}

/// The way to `state` (in `prev`) came at least `n` steps straight on in its
/// direction, or straight from the start.
fn straight_on(prev: &[u32], mut state: usize, n: usize) -> bool {
    let dir = state % 7;
    // The state is `n` steps from the last turn when its `n - 1` predecessors came the
    // same way.
    for _ in 1..n {
        let p = prev[state];
        if p == u32::MAX || p as usize % 7 == 6 {
            return true;
        }
        if p as usize % 7 != dir {
            return false;
        }
        state = p as usize;
    }
    true
}

/// Removes repeated points and points in the middle of straight stretches.
pub fn simplify(points: Vec<V3>) -> Vec<V3> {
    let mut v: Vec<V3> = Vec::new();
    for q in points {
        if v.last().is_some_and(|l| dist(*l, q) < 1e-3) {
            continue;
        }
        if v.len() >= 2 {
            let (a, b) = (v[v.len() - 2], v[v.len() - 1]);
            let (d1, d2) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [q[0] - b[0], q[1] - b[1], q[2] - b[2]]);
            let cross = [d1[1] * d2[2] - d1[2] * d2[1], d1[2] * d2[0] - d1[0] * d2[2], d1[0] * d2[1] - d1[1] * d2[0]];
            let dot = d1[0] * d2[0] + d1[1] * d2[1] + d1[2] * d2[2];
            if cross.iter().all(|c| c.abs() < 1e-3) && dot > 0.0 {
                v.pop();
            }
        }
        v.push(q);
    }
    v
}

/// Length of a polyline.
pub fn length(points: &[V3]) -> f32 {
    points.windows(2).map(|s| dist(s[0], s[1])).sum()
}

/// Corners of a polyline (direction changes) and the length of the shortest stretch
/// between two of them.
pub fn bends(points: &[V3]) -> (usize, f32) {
    let n = points.len().saturating_sub(2);
    let inner = points.windows(2).skip(1).take(points.len().saturating_sub(3)).map(|s| dist(s[0], s[1])).fold(f32::INFINITY, f32::min);
    (n, inner)
}

#[cfg(test)]
mod tests;
