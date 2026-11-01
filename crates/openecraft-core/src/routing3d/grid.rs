//! The space of a line-up (or a room, see `super::room`) as a grid of cubic cells, and
//! the obstacle-aware shortest way through it (A* with turn costs). Cells are free
//! inside the enclosures (in front of the mounting plate, behind the door), blocked by
//! the walls, by devices, rails, busbars and accessories (with a clearance) and by
//! keep-out reservations. Cable ducts and routing paths are channels: the way through
//! them is cheaper than through free air; a cable too thick for a duct keeps out of it,
//! one that fits enters and leaves the ducts of a cabinet through the slots of their
//! walls (never through their base or cover) and keeps off their walls outside them.
//! The room around every cell is measured from what blocks the space (its boxes and
//! walls themselves, not the cells they block): a route keeps half its thickness from
//! all of it but the items at its ends, and where it cannot (near its ends, or within
//! half a cell of it where there is no other way) it is squeezed through at a price. A
//! turn where its bend, rounded with the bend radius, would not keep clear costs more.
//! A route leaves each of its ends straight on, as far as half its bend radius, and
//! between two of its bends it runs straight at least as far; a lane change with an
//! S-bend is free of that, bends closer to each other cost more the closer they are.
//! Cables and tubes keep apart from the ones routed before them by their thicknesses:
//! none runs inside another in free air (in a duct or routing path they lie side by
//! side, but where they come up to their ends). Walls between bayed enclosures are
//! crossed at cable passages only.

use super::{Class, PathKind, ReserveMode, RouteKind, RoutingRules};
use crate::model::Project;
use crate::panel::{self, Enclosure, ItemKind, PLATE_Z};
use crate::parts::vec3::{add, dist, norm, scale, sub, V3};
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
/// Cost factor of a cell a route is squeezed through (see [`Space::level`]).
const SQUEEZE: f32 = 10.0;
/// Upper limit of cells; a larger line-up gets coarser cells.
const MAX_CELLS: usize = 1_500_000;
/// Cost factor of a horizontal step in the open space of a room (see
/// [`Space::mark_open`]).
const OPEN: f32 = 10.0;
/// How far (mm, at least [`ROOM_CELLS`] cells) the room around the cells is measured
/// (see [`Space::measure`]): farther from everything a cell has room for anything.
const ROOM_CAP: f32 = 300.0;
const ROOM_CELLS: f32 = 8.0;
/// How far a bend of radius r rounding a right-angled corner lies inside it: (√2 − 1) r.
pub const CORNER_CUT: f32 = std::f32::consts::SQRT_2 - 1.0;
/// Extra cost of a turn without room for its bend, in turns.
const TIGHT: f32 = 4.0;
/// The most extra cost of a bend closer to the one before it than half the bend
/// radius (as close as can be), in turns.
const CLOSE: f32 = 16.0;
/// Most cells whose costs to the goal a search keeps for searching again.
const KEEP_COSTS: usize = 400_000;
/// Cells looked at along the way out of an end for the cell the route leaves it in.
const LEAD_CELLS: usize = 4;

/// How a cell can be passed: not at all, squeezed (with less room than half the
/// thickness: near an end, or within half a cell of it), or freely.
const NO: u8 = 0;
const SQUEEZED: u8 = 1;
const FREE: u8 = 2;

/// The costs to the goal a search worked out (the cells known with their costs, the
/// highest), to search the same way again (the same channels full) without working
/// them out: to a goal nearby as well, less the cost from it to this one.
#[derive(Clone, Debug, PartialEq)]
pub struct GoalCosts {
    goal: usize,
    cells: Vec<(u32, f32)>,
    reach: f32,
}

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
    /// Not a routing path of the layout: the cable way of a 19-inch rack beside its
    /// mounting rails.
    pub implied: bool,
}

/// Where wires may cross from one enclosure into the next: the boundary `x` and the
/// opening's `y` and `z` ranges.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Crossing {
    x: f32,
    y: [f32; 2],
    z: [f32; 2],
}

/// What blocks the space, to measure the room around the cells: a box (an item with
/// its clearance, a keep-out, a wall, a cabinet standing in a room) with the item it is
/// (0 for none), or the walls, the ceiling and the floor of a room (its floor plan at
/// an origin, the room's height).
#[derive(Clone, Debug, PartialEq)]
enum Solid {
    Box { lo: V3, hi: V3, owner: u64 },
    Room { plan: Vec<[f32; 2]>, origin: V3, height: f32 },
}

/// A cable duct of a cabinet: its box, the axis it runs along, and the heights above
/// its base of the closed rim of its slotted walls and of their top (its cover lies on
/// them). A route outside it keeps off it, one enters and leaves it through its walls
/// between the rim and the top, never through its base or its cover.
#[derive(Clone, Debug, PartialEq)]
struct DuctBox {
    lo: V3,
    hi: V3,
    axis: usize,
    rim: f32,
    top: f32,
}

/// What is routed.
#[derive(Clone, Copy, Debug)]
pub struct Request<'a> {
    pub kind: RouteKind,
    pub class: Class,
    /// Items the ends are at: their cells are passable and the room they take does not
    /// count, those of the items themselves only near the start and the end `at` (a
    /// route leaves a device, not runs through it).
    pub ignore: &'a [u64],
    pub at: [V3; 2],
    /// The direction the route leaves each end in (a unit vector along an axis; zero
    /// where it may leave any way): it runs straight out of the end, as far as half its
    /// bend radius where it can, before it bends.
    pub leave: [V3; 2],
    /// How far the route runs straight already before each end point (out of the
    /// nozzle of a device), mm.
    pub out: [f32; 2],
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
    /// Squeezed through cells with less room than half the thickness away from its ends.
    pub squeezed: bool,
}

/// Where a run ends and what its cells cost, `None` where it is blocked.
type RunEnd = Option<(usize, f32)>;

/// Where a search begins and ends: the cells from the one the route leaves its start
/// in to the one the search begins at (straight out of the start), the direction it
/// heads there (6: none), the cells from the one the search ends at to the one the
/// route arrives at its end in (straight into the end) and the direction it heads
/// along them (6: none).
struct Ends {
    lead_in: Vec<usize>,
    dir: usize,
    lead_out: Vec<usize>,
    arrive: usize,
}

pub struct Space {
    pub origin: V3,
    pub cell: f32,
    pub n: [usize; 3],
    block: Vec<u8>,
    owner: Vec<u64>,
    /// Inner size (mm) of the cable duct a cell lies in, 0 for none.
    duct: Vec<f32>,
    /// The cable duct of a cabinet a cell lies in (index + 1 into `ducts`, 0 for none).
    duct_of: Vec<u16>,
    ducts: Vec<DuctBox>,
    /// Room (mm) from the centre of each cell outside the cable ducts of the cabinets
    /// to the nearest of them (index + 1, 0 for none within reach); empty until
    /// [`Space::measure`].
    duct_room: Vec<(f32, u16)>,
    /// What blocks the space, until [`Space::measure`].
    solids: Vec<Solid>,
    /// Room (mm) from the centre of each cell to the nearest solid, which item that is
    /// (index + 1 into `owners`, 0 for none) and the room to the nearest solid of
    /// another item; empty until [`Space::measure`]: room everywhere.
    room: Vec<f32>,
    room_owner: Vec<u32>,
    room_other: Vec<f32>,
    owners: Vec<u64>,
    /// How far the cables and tubes routed through the space reach into each cell: the
    /// largest half thickness less the distance from its middle line (see
    /// [`Space::take`]).
    taken: Vec<f32>,
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
    /// How far (in cells) each cell of a room lies from its surfaces, `open_cells` + 1
    /// at most (see [`Space::mark_open`]); empty in a line-up.
    open: Vec<u8>,
    open_cells: u32,
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
    /// The run out of each cell in each direction (cell × 6): where it ends
    /// (`u32::MAX`: blocked) and what it costs, where `run_stamp` is the search's.
    runs: Vec<(u32, f32)>,
    run_stamp: Vec<u32>,
    /// What a step into each cell costs for the request, up or down and along the
    /// other axes, and how it can be passed ([`NO`]: not at all), where `pass_stamp`
    /// is the search's.
    pass: Vec<(f32, f32, u8)>,
    pass_stamp: Vec<u32>,
    run: u32,
}

/// The memos of a search: what the steps into the cells cost and the runs out of them.
struct Memo<'a> {
    pass: &'a mut [(f32, f32, u8)],
    pass_stamp: &'a mut [u32],
    runs: &'a mut [(u32, f32)],
    run_stamp: &'a mut [u32],
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
                runs: vec![(u32::MAX, 0.0); cells * 6],
                run_stamp: vec![0; cells * 6],
                pass: vec![(-1.0, -1.0, NO); cells],
                pass_stamp: vec![0; cells],
                run: 0,
            };
        }
        self.run = self.run.wrapping_add(1);
        if self.run == 0 {
            for v in [&mut self.stamp, &mut self.reached, &mut self.settled, &mut self.run_stamp, &mut self.pass_stamp] {
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
            duct_of: vec![0; total],
            ducts: Vec::new(),
            duct_room: Vec::new(),
            solids: Vec::new(),
            room: Vec::new(),
            room_owner: Vec::new(),
            room_other: Vec::new(),
            owners: Vec::new(),
            taken: vec![f32::NEG_INFINITY; total],
            path: vec![0; total],
            reserved: vec![0; total],
            occ: vec![0; total],
            zone: vec![0; total],
            paths: Vec::new(),
            crossings: Vec::new(),
            column: vec![usize::MAX; n[0]],
            open: Vec::new(),
            open_cells: 0,
        }
    }

    /// Measures how far (in cells) the free cells lie from the walls, the ceiling (the
    /// top of the grid) and the items standing or hanging in the room, up to `distance`:
    /// running through the room horizontally costs the more the farther from them (see
    /// [`Space::open_factor`]), so pipes and cables keep to the walls and the ceiling
    /// and drop straight to a device standing in the room.
    pub(super) fn mark_open(&mut self, distance: f32) {
        let k = ((distance / self.cell).ceil() as u32).clamp(1, 200);
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
            for nd in 0..6 {
                let Some(ni) = self.step(idx, nd) else { continue };
                if dist[ni] == u32::MAX {
                    dist[ni] = d + 1;
                    queue.push_back(ni);
                }
            }
        }
        self.open = dist.into_iter().map(|d| d.min(k + 1) as u8).collect();
        self.open_cells = k;
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
            if panel::is_rack(&encs[k]) {
                s.add_rack_ways(&encs[k], k, pos[k]);
            }
        }
        let mut passages: Vec<panel::Passage> = Vec::new();
        for w in lineup.windows(2) {
            for x in panel::passages(&prj.layout, pos, w[0]) {
                let (y, z) = ([x.at[1] - x.half[0], x.at[1] + x.half[0]], [x.at[2] - x.half[1], x.at[2] + x.half[1]]);
                s.crossings.push(Crossing { x: x.at[0], y, z });
                passages.push(x);
            }
        }
        for &k in lineup {
            s.add_walls(&encs[k], pos[k], &passages);
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
                    for idx in self.cells_in(self.widened(around)) {
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
                    self.solids.push(Solid::Box { lo: around.0, hi: around.1, owner: it.id });
                }
                ItemKind::Duct if it.face == panel::Face::Plate || e.is_room() => {
                    // Its inside: across and as deep as it is, less its walls.
                    let size = (it.w.min(it.h).min(it.d) - 4.0).max(1.0);
                    let cells = self.cells_in(self.widened(bx));
                    cells.iter().for_each(|&idx| self.duct[idx] = size);
                    // In a cabinet its walls bound the routes outside it and the ways
                    // into it (a tray in a room is open).
                    if !e.is_room() {
                        let (rim, top, _) = panel::duct_walls(it);
                        self.ducts.push(DuctBox { lo: bx.0, hi: bx.1, axis: usize::from(it.w < it.h), rim, top });
                        let mark = self.ducts.len() as u16;
                        cells.iter().for_each(|&idx| self.duct_of[idx] = mark);
                    }
                }
                _ => {}
            }
        }
        for r in &e.reservations {
            let bx = shift((r.min, r.max), 0.0);
            match r.mode {
                ReserveMode::Keepout => self.block_box(bx),
                ReserveMode::For(class) => {
                    for idx in self.cells_in(bx) {
                        self.reserved[idx] |= class.bit();
                    }
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
                implied: false,
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

    /// The cable ways of a 19-inch rack `e` (at `o`): the space beside its mounting
    /// rails from its front to its rear is a channel the routes run along, leaving it
    /// for the devices in the rack; the rails themselves block the space.
    fn add_rack_ways(&mut self, e: &Enclosure, k: usize, o: V3) {
        let rails = panel::rack::rail_boxes(e);
        let (l, r) = (rails.iter().map(|b| b.0[0]).fold(f32::INFINITY, f32::min), rails.iter().map(|b| b.1[0]).fold(f32::NEG_INFINITY, f32::max));
        for (lo, hi) in rails {
            self.block_box((add(lo, o), add(hi, o)));
        }
        let t = e.wall();
        for (x0, x1) in [(t, l), (r, e.width - t)] {
            if x1 <= x0 {
                continue;
            }
            self.paths.push(PathCells { enclosure: k, id: 0, kind: PathKind::Any, classes: 0, capacity: 0.0, used: 0.0, items: 0, implied: true });
            let mark = self.paths.len() as u16;
            for idx in self.cells_in((add(o, [x0, t, PLATE_Z]), add(o, [x1, e.height - t, e.depth - t]))) {
                if self.path[idx] == 0 {
                    self.path[idx] = mark;
                }
            }
        }
    }

    /// The walls of enclosure `e` (at `o`): its sides, roof and bottom, the mounting
    /// plate with the rear wall and the door, the sides open at the cable `passages` to
    /// the bayed neighbours.
    fn add_walls(&mut self, e: &Enclosure, o: V3, passages: &[panel::Passage]) {
        let (w, h, d, t) = (e.width, e.height, e.depth, e.wall());
        let far = add(o, [w, h, d]);
        let sides = [((o, [o[0] + t, far[1], far[2]]), o[0]), (([far[0] - t, o[1], o[2]], far), far[0])];
        for (bx, x) in sides {
            let openings: Vec<([f32; 2], [f32; 2])> = passages
                .iter()
                .filter(|p| (p.at[0] - x).abs() < 1.0)
                .map(|p| ([p.at[1] - p.half[0], p.at[1] + p.half[0]], [p.at[2] - p.half[1], p.at[2] + p.half[1]]))
                .collect();
            for (lo, hi) in cut_openings(bx, &openings) {
                self.solids.push(Solid::Box { lo, hi, owner: 0 });
            }
        }
        for (lo, hi) in [(o, [far[0], o[1] + t, far[2]]), ([o[0], far[1] - t, o[2]], far), (o, [far[0], far[1], o[2] + PLATE_Z]), ([o[0], o[1], far[2] - t], far)] {
            self.solids.push(Solid::Box { lo, hi, owner: 0 });
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

    /// The walls of a room with the floor `plan` (at `origin`, `height` high, y down
    /// from the ceiling), its ceiling and its floor, to measure the room from.
    pub(super) fn add_room(&mut self, plan: Vec<[f32; 2]>, origin: V3, height: f32) {
        self.solids.push(Solid::Room { plan, origin, height });
    }

    /// How far the room around the cells is measured.
    fn cap(&self) -> f32 {
        ROOM_CAP.max(ROOM_CELLS * self.cell)
    }

    /// Measures the room around the cells (once the space is complete): from the
    /// centre of each cell that is not a wall to the nearest of the solids, up to
    /// [`Space::cap`], and to the nearest one of another item.
    pub(super) fn measure(&mut self) {
        let total = self.block.len();
        let cap = self.cap();
        self.room = vec![cap; total];
        self.room_owner = vec![0; total];
        self.room_other = vec![cap; total];
        let solids = std::mem::take(&mut self.solids);
        for solid in &solids {
            match solid {
                Solid::Box { lo, hi, owner } => {
                    let key = match *owner {
                        0 => 0,
                        id => match self.owners.iter().position(|&o| o == id) {
                            Some(i) => i as u32 + 1,
                            None => {
                                self.owners.push(id);
                                self.owners.len() as u32
                            }
                        },
                    };
                    for idx in self.cells_in((lo.map(|v| v - cap), hi.map(|v| v + cap))) {
                        if self.block[idx] != WALL {
                            let d = box_distance(self.centre(idx), *lo, *hi);
                            self.note_room(idx, d, key);
                        }
                    }
                }
                Solid::Room { plan, origin, height } => {
                    for idx in 0..total {
                        if self.block[idx] == WALL {
                            continue;
                        }
                        let c = sub(self.centre(idx), *origin);
                        let d = c[1].min(height - c[1]).min(plan_distance(plan, [c[0], c[2]]));
                        self.note_room(idx, d.max(0.0), 0);
                    }
                }
            }
        }
        self.solids = solids;
        self.duct_room = vec![(cap, 0); total];
        for k in 0..self.ducts.len() {
            let (lo, hi) = (self.ducts[k].lo, self.ducts[k].hi);
            for idx in self.cells_in((lo.map(|v| v - cap), hi.map(|v| v + cap))) {
                let d = box_distance(self.centre(idx), lo, hi);
                if self.block[idx] != WALL && self.duct_of[idx] == 0 && d < self.duct_room[idx].0 {
                    self.duct_room[idx] = (d, k as u16 + 1);
                }
            }
        }
    }

    /// A solid of the item `key` lies `d` mm from the centre of a cell.
    fn note_room(&mut self, idx: usize, d: f32, key: u32) {
        if d < self.room[idx] {
            if key != self.room_owner[idx] {
                self.room_other[idx] = self.room[idx];
            }
            self.room[idx] = d;
            self.room_owner[idx] = key;
        } else if key != self.room_owner[idx] && d < self.room_other[idx] {
            self.room_other[idx] = d;
        }
    }

    /// Room (mm) from the centre of a cell to the nearest solid but those of the items
    /// `ignore`.
    fn room_for(&self, idx: usize, ignore: &[u64]) -> f32 {
        let Some(&room) = self.room.get(idx) else { return f32::INFINITY };
        match self.room_owner[idx] {
            0 => room,
            k if ignore.contains(&self.owners[k as usize - 1]) => self.room_other[idx],
            _ => room,
        }
    }

    /// The cell lies near an end of the request: within a few cells of it, or as far as
    /// the route runs straight out of it (half its bend radius) at least.
    fn near_end(&self, idx: usize, req: &Request) -> bool {
        req.at.iter().any(|q| dist(self.centre(idx), *q) <= (3.0 * self.cell).max(req.turn * 0.5) + req.half)
    }

    /// The request keeps apart from the cables and tubes routed before it in the cell
    /// (wires do not; in a cable duct or routing path they lie side by side, as far as
    /// it holds them; where it comes up to its ends it need not).
    fn apart(&self, idx: usize, req: &Request) -> bool {
        req.kind == RouteKind::Wire || self.taken[idx] + req.half + 0.5 <= 0.0 || self.channel(idx, req) || self.near_end(idx, req)
    }

    /// A cable or tube of half thickness `half` routed along `points`: the routes after
    /// it keep apart from it, up to `reach` mm beyond its surface (half the thickness of
    /// the thickest of them).
    pub fn take(&mut self, points: &[V3], half: f32, reach: f32) {
        for idx in self.cells_near(points, half + reach) {
            let c = self.centre(idx);
            let d = points.windows(2).map(|s| dist_to_segment(c, s[0], s[1])).fold(f32::INFINITY, f32::min);
            self.taken[idx] = self.taken[idx].max(half - d);
        }
    }

    /// The cells of a way found before other routes took their room are still apart from
    /// them.
    pub fn still_apart(&self, cells: &[usize], req: &Request) -> bool {
        cells.iter().all(|&i| self.apart(i, req))
    }

    /// The bend radius each corner of a way can have (`radius` at most, halved while
    /// its bend would come nearer than half the thickness to what blocks the space, but
    /// the items `ignore`); 0 at the ends.
    pub fn corner_radii(&self, points: &[V3], half: f32, radius: f32, ignore: &[u64]) -> Vec<f32> {
        let n = points.len();
        (0..n)
            .map(|i| {
                if i == 0 || i + 1 == n {
                    return 0.0;
                }
                let (from, to) = (norm(sub(points[i], points[i - 1])), norm(sub(points[i + 1], points[i])));
                let mut r = radius;
                while r > 1.0 && !self.bend_room(points[i], from, to, r, half, ignore) {
                    r /= 2.0;
                }
                if r > 1.0 { r } else { 0.0 }
            })
            .collect()
    }

    /// A bend of radius `radius` rounding the corner at `corner` between the directions
    /// `from` and `to` keeps `half` from what blocks the space (but the items `ignore`):
    /// the middle of its arc, which lies inside the corner, is in a free cell (or the
    /// clearance of an item it may pass) with that much room (less how far the middle
    /// is from the cell's centre).
    fn bend_room(&self, corner: V3, from: V3, to: V3, radius: f32, half: f32, ignore: &[u64]) -> bool {
        let inside = scale(sub(to, from), CORNER_CUT * radius / std::f32::consts::SQRT_2);
        let middle = add(corner, inside);
        let free = |m: usize| self.block[m] == 0 || (self.block[m] == ITEM && ignore.contains(&self.owner[m]));
        self.cell_of(middle).is_some_and(|m| free(m) && self.room_for(m, ignore) - dist(middle, self.centre(m)) >= half)
    }

    /// Blocks the cells whose centre lies in a box (walls, cabinets, door swings,
    /// keep-outs), which the room is measured from as well.
    pub(super) fn block_box(&mut self, bx: (V3, V3)) {
        for idx in self.cells_in(self.widened(bx)) {
            self.block[idx] = WALL;
        }
        self.solids.push(Solid::Box { lo: bx.0, hi: bx.1, owner: 0 });
    }

    /// The box widened to at least a cell along every axis (about its middle), so that
    /// it covers the centre of a cell along each: a way between the centres of cells it
    /// does not cover never crosses it.
    fn widened(&self, (mut lo, mut hi): (V3, V3)) -> (V3, V3) {
        for k in 0..3 {
            let lack = self.cell - (hi[k] - lo[k]);
            if lack > 0.0 {
                lo[k] -= lack / 2.0;
                hi[k] += lack / 2.0;
            }
        }
        (lo, hi)
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

    /// The cell `cells` steps from `idx` in direction `dir`, if inside the grid.
    fn beyond(&self, idx: usize, dir: usize, cells: usize) -> Option<usize> {
        let (axis, up) = (dir / 2, dir % 2 == 1);
        let stride = [1, self.n[0], self.n[0] * self.n[1]][axis];
        let c = (idx / stride) % self.n[axis];
        if up {
            (c + cells < self.n[axis]).then(|| idx + cells * stride)
        } else {
            (c >= cells).then(|| idx - cells * stride)
        }
    }

    /// The cell `other` lies beside `idx`, across the axis of `dir` along one other axis,
    /// within `cells` steps: the direction towards it and how many steps.
    fn beside(&self, idx: usize, dir: usize, other: usize, cells: usize) -> Option<(usize, usize)> {
        let (c, o) = (self.coords(idx), self.coords(other));
        let mut found = None;
        for axis in (0..3).filter(|&k| k != dir / 2) {
            if c[axis] != o[axis] {
                if found.is_some() {
                    return None;
                }
                found = Some((axis * 2 + usize::from(o[axis] > c[axis]), c[axis].abs_diff(o[axis])));
            }
        }
        found.filter(|&(_, k)| c[dir / 2] == o[dir / 2] && k >= 1 && k <= cells)
    }

    /// The cell `other` lies ahead of `idx` in direction `dir`, within `cells` steps.
    fn on_line(&self, idx: usize, dir: usize, other: usize, cells: usize) -> bool {
        let (axis, up) = (dir / 2, dir % 2 == 1);
        let stride = [1, self.n[0], self.n[0] * self.n[1]][axis] as i64;
        let diff = other as i64 - idx as i64;
        let steps = diff / stride;
        let c = ((idx / stride as usize) % self.n[axis]) as i64;
        diff % stride == 0 && steps != 0 && (up == (steps > 0)) && steps.unsigned_abs() as usize <= cells && c + steps >= 0 && ((c + steps) as usize) < self.n[axis]
    }

    /// The neighbour of a cell in direction `dir` (axis × 2 + 1 for up), if any.
    fn step(&self, idx: usize, dir: usize) -> Option<usize> {
        let (axis, up) = (dir / 2, dir % 2 == 1);
        let mut c = self.coords(idx);
        if up {
            if c[axis] + 1 >= self.n[axis] {
                return None;
            }
            c[axis] += 1;
        } else {
            if c[axis] == 0 {
                return None;
            }
            c[axis] -= 1;
        }
        Some(self.index(c))
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

    /// How the request can pass the cell (see [`NO`], [`SQUEEZED`], [`FREE`]), keeping
    /// apart from the other routes where `apart`.
    fn level(&self, idx: usize, req: &Request, apart: bool) -> u8 {
        let free = match self.block[idx] {
            0 => true,
            ITEM => req.ignore.contains(&self.owner[idx]),
            BODY => req.ignore.contains(&self.owner[idx]) && req.at.iter().any(|q| dist(self.centre(idx), *q) <= 2.0 * self.cell),
            _ => false,
        };
        // A cable only through ducts it fits into; only classes a reservation is for.
        if !free || (req.kind != RouteKind::Wire && self.duct[idx] > 0.0 && self.duct[idx] < 2.0 * req.half) || (self.reserved[idx] != 0 && self.reserved[idx] & req.class.bit() == 0) || (apart && !self.apart(idx, req)) {
            return NO;
        }
        // As thick as it is: room around it, or squeezed through where there is a
        // little less, and near its ends whatever there is.
        let room = self.room_for(idx, req.ignore);
        if room >= req.half {
            FREE
        } else if room + self.cell * 0.5 >= req.half || self.near_end(idx, req) {
            SQUEEZED
        } else {
            NO
        }
    }

    fn passable(&self, idx: usize, req: &Request) -> bool {
        self.level(idx, req, true) != NO
    }

    /// The cell is passable for the request but for the other routes in it.
    fn open_for(&self, idx: usize, req: &Request) -> bool {
        self.level(idx, req, false) != NO
    }

    /// A route coming in direction `from` may turn at the cell into direction `to`: its
    /// bend, rounded with the bend radius, keeps half its thickness from what blocks
    /// the space (near an end it may turn anywhere).
    fn turn_room(&self, idx: usize, from: usize, to: usize, req: &Request) -> bool {
        req.kind == RouteKind::Wire || self.bend_room(self.centre(idx), unit(from), unit(to), req.turn, req.half, req.ignore) || self.near_end(idx, req)
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
        if !channel && axis != 1 {
            f *= self.open_factor(idx);
        }
        if self.path_for(idx, req).is_some_and(|p| p.capacity > 0.0 && p.used + req.area > p.capacity) {
            f *= FULL;
        }
        if self.zone[idx] & req.class.apart_bits() != 0 {
            f *= rules.segregation_factor.max(1.0);
        }
        f
    }

    /// Cost factor of a horizontal step into a cell of a room by how far it lies from
    /// the room's surfaces: none next to them, [`OPEN`] from the distance of
    /// [`Space::mark_open`] on, in between as far as it is.
    fn open_factor(&self, idx: usize) -> f32 {
        match self.open.get(idx) {
            Some(&d) if d > 1 => 1.0 + (OPEN - 1.0) * ((d - 1) as f32 / (self.open_cells.max(2) - 1) as f32).min(1.0),
            _ => 1.0,
        }
    }

    /// What the walls of the cabinets' cable ducts make of the step from cell `a` into
    /// its neighbour `b` in direction `dir`: `None` where it is barred, through the base
    /// or the cover of a duct, or through its walls below their rim or above their top
    /// (a cell either way); otherwise whether the route is squeezed against a duct
    /// it runs along outside, nearer than half its thickness, with a little less room
    /// (heading into or out of a duct through its wall it comes as near as it likes).
    fn duct_step(&self, a: usize, b: usize, dir: usize, req: &Request) -> Option<bool> {
        let (da, db) = (self.duct_of[a], self.duct_of[b]);
        if da != db && (da == 0 || db == 0) {
            if dir / 2 == 2 {
                return None;
            }
            let d = &self.ducts[da.max(db) as usize - 1];
            let z = self.centre(if da != 0 { a } else { b })[2] - d.lo[2];
            return (z - req.half >= d.rim - self.cell && z + req.half <= d.top + self.cell).then_some(false);
        }
        let Some(&(room, k)) = self.duct_room.get(b).filter(|_| db == 0) else { return Some(false) };
        if k == 0 || room >= req.half {
            return Some(false);
        }
        // Beside the duct (not over its cover) heading at its wall or away from it.
        let d = &self.ducts[k as usize - 1];
        let z = self.centre(b)[2];
        if dir / 2 != d.axis && dir / 2 != 2 && z >= d.lo[2] && z <= d.hi[2] {
            return Some(false);
        }
        (room + self.cell * 0.5 >= req.half || self.near_end(b, req)).then_some(true)
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

    /// The step from cell `a` into its neighbour `b` in direction `dir` is open for
    /// the request.
    fn step_ok(&self, a: usize, b: usize, dir: usize, req: &Request) -> bool {
        self.passable(b, req) && (dir / 2 != 0 || self.crossing_ok(a, b)) && self.duct_step(a, b, dir, req).is_some()
    }

    /// The cost of the step from cell `a` into its neighbour `b` in direction `dir`,
    /// `None` where it is not open; what the steps into the cells cost is remembered
    /// for the search.
    fn step_cost(&self, memo: &mut Memo, a: usize, b: usize, dir: usize, req: &Request, rules: &RoutingRules) -> Option<f32> {
        if memo.pass_stamp[b] != memo.run {
            memo.pass[b] = match self.level(b, req, true) {
                NO => (-1.0, -1.0, NO),
                level => (self.cell * self.factor(b, 1, req, rules), self.cell * self.factor(b, 0, req, rules), level),
            };
            memo.pass_stamp[b] = memo.run;
        }
        let (up, along, level) = memo.pass[b];
        if level == NO || (dir / 2 == 0 && !self.crossing_ok(a, b)) {
            return None;
        }
        let against_duct = self.duct_step(a, b, dir, req)?;
        let cost = if dir / 2 == 1 { up } else { along };
        Some(if level == SQUEEZED || against_duct { cost * SQUEEZE } else { cost })
    }

    /// The nearest passable cell to a point (searching a few cells around it).
    fn nearest_passable(&self, q: V3, req: &Request) -> Option<usize> {
        let r = self.cell * LEAD_CELLS as f32;
        let lo = [q[0] - r, q[1] - r, q[2] - r];
        let hi = [q[0] + r, q[1] + r, q[2] + r];
        if let Some(idx) = self.cell_of(q).filter(|&i| self.passable(i, req)) {
            return Some(idx);
        }
        self.cells_in((lo, hi)).into_iter().filter(|&i| self.passable(i, req)).min_by(|&a, &b| dist(self.centre(a), q).total_cmp(&dist(self.centre(b), q)).then(a.cmp(&b)))
    }

    /// The axis a direction runs along and which way (±1); `None` for one along no axis.
    fn axis_of(leave: V3) -> Option<(usize, f32)> {
        let axis = (0..3).max_by(|&i, &j| leave[i].abs().total_cmp(&leave[j].abs()))?;
        let along = leave[axis].abs();
        (along >= 1e-3 && along >= 0.9 * dist(leave, [0.0; 3])).then(|| (axis, leave[axis].signum()))
    }

    /// How far along its axis the way out of an end at `q` in the direction `leave` may
    /// go at most, so that it does not pass the way out of the `other` end (leaving it
    /// in `other_leave`): to the middle between the two where they face each other,
    /// before the other end where it would run into it; `None` for no limit.
    fn lead_limit(&self, q: V3, leave: V3, other: V3, other_leave: V3) -> Option<f32> {
        let (axis, sign) = Space::axis_of(leave)?;
        if Space::axis_of(other_leave).is_some_and(|(oa, os)| oa == axis && os == -sign) {
            return Some((q[axis] + other[axis]) / 2.0);
        }
        let lateral = (0..3).filter(|&i| i != axis).map(|i| (q[i] - other[i]).abs()).fold(0.0, f32::max);
        (lateral <= 2.0 * self.cell).then_some(other[axis])
    }

    /// The way out of an end at `q` in the direction `leave`: the cells from the first
    /// passable one along it (its centre ahead of `q`) straight on, until the route is
    /// `far` mm (with the `out` it runs straight already) out of the end, as far as
    /// `straight` cells at most and never beyond `limit` along the axis, and the
    /// direction. `None` where `leave` runs along no axis or none of the first cells
    /// along it is passable.
    #[allow(clippy::too_many_arguments)]
    fn lead(&self, q: V3, leave: V3, limit: Option<f32>, out: f32, far: f32, req: &Request, straight: usize) -> Option<(Vec<usize>, usize)> {
        let (axis, sign) = Space::axis_of(leave)?;
        let dir = axis * 2 + usize::from(sign > 0.0);
        let within = |idx: usize| limit.is_none_or(|l| (self.centre(idx)[axis] - l) * sign <= 0.0);
        let ahead = |idx: usize| (self.centre(idx)[axis] - q[axis]) * sign + out;
        // The cells straight out of `from`.
        let from = |from: V3| -> Option<Vec<usize>> {
            let mut first = None;
            for k in 0..LEAD_CELLS {
                let mut p = from;
                p[axis] += sign * (k as f32 + 0.5) * self.cell;
                let Some(idx) = self.cell_of(p) else { break };
                if !within(idx) {
                    break;
                }
                if self.passable(idx, req) {
                    first = Some(idx);
                    break;
                }
            }
            let mut cells = vec![first?];
            for _ in 0..straight {
                let last = cells[cells.len() - 1];
                if ahead(last) >= far {
                    break;
                }
                match self.step(last, dir) {
                    Some(next) if within(next) && self.step_ok(last, next, dir, req) => cells.push(next),
                    _ => break,
                }
            }
            Some(cells)
        };
        // Barred before it ran `far` straight out of the end, the way may leave it from
        // the cell beside (a cell aside at most), where it runs farther.
        let mut best = from(q);
        if best.as_ref().is_none_or(|c| ahead(c[c.len() - 1]) < far) {
            for other in (0..3).filter(|&k| k != axis) {
                for side in [-1.0, 1.0] {
                    let mut p = q;
                    p[other] += side * self.cell;
                    if let Some(cells) = from(p).filter(|c| best.as_ref().is_none_or(|b| c.len() > b.len())) {
                        best = Some(cells);
                    }
                }
            }
        }
        Some((best?, dir))
    }

    /// The cheapest way from `a` to `b`, or `None` if there is none.
    pub fn route(&self, a: V3, b: V3, req: &Request, rules: &RoutingRules) -> Option<Way> {
        self.route_with(a, b, req, rules, None).0
    }

    /// [`Space::route`] with the costs to the goal of a search of the same request before
    /// (where they still hold), and the costs it used.
    pub fn route_with(&self, a: V3, b: V3, req: &Request, rules: &RoutingRules, known: Option<&GoalCosts>) -> (Option<Way>, Option<GoalCosts>) {
        // Cables and tubes run straight out of their ends and on between their bends
        // for half their bend radius, where there is a way like that.
        let straight = if req.kind == RouteKind::Wire { 0 } else { (req.turn * 0.5 / self.cell).ceil() as usize };
        let far = req.turn * 0.5;
        let lead_in = self.lead(a, req.leave[0], self.lead_limit(a, req.leave[0], b, req.leave[1]), req.out[0], far, req, straight);
        let lead_out = self.lead(b, req.leave[1], self.lead_limit(b, req.leave[1], a, req.leave[0]), req.out[1], far, req, straight);
        // Where the route cannot leave an end straight on (or has no direction there),
        // it starts at the nearest passable cell.
        let mut ends = Ends { lead_in: Vec::new(), dir: 6, lead_out: Vec::new(), arrive: 6 };
        match lead_in {
            Some((cells, dir)) => (ends.lead_in, ends.dir) = (cells, dir),
            None => match self.nearest_passable(a, req) {
                Some(idx) => ends.lead_in = vec![idx],
                None => return (None, None),
            },
        }
        match lead_out {
            Some((mut cells, dir)) => {
                cells.reverse();
                if cells.len() > 1 {
                    ends.arrive = dir ^ 1;
                }
                ends.lead_out = cells;
            }
            None => match self.nearest_passable(b, req) {
                Some(idx) => ends.lead_out = vec![idx],
                None => return (None, None),
            },
        }
        let mut costs = known.cloned();
        SEARCH.with_borrow_mut(|search| {
            let mut way = self.search(search, a, b, &ends, req, rules, straight, &mut costs);
            if way.is_none() && straight > 0 {
                // No way like that: bends wherever they fit, from and to the nearest
                // passable cells.
                let (Some(start), Some(goal)) = (self.nearest_passable(a, req), self.nearest_passable(b, req)) else { return (None, costs) };
                ends = Ends { lead_in: vec![start], dir: 6, lead_out: vec![goal], arrive: 6 };
                way = self.search(search, a, b, &ends, req, rules, 0, &mut costs);
            }
            (way, costs)
        })
    }

    /// The A* search of [`Space::route`] with the arrays of `search`, from the start to
    /// the goal of `ends`: a turn is followed by `straight` cells straight on (one at
    /// least; the way out of the start and into the goal is given by `ends`, the goal
    /// reached heading along the way into the end or across it, never against it),
    /// unless the next bend comes sooner at a price. It takes the costs to the goal from
    /// `costs` where known, otherwise works them out and keeps them there.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn search(&self, search: &mut Search, a: V3, b: V3, ends: &Ends, req: &Request, rules: &RoutingRules, straight: usize, costs: &mut Option<GoalCosts>) -> Option<Way> {
        let (start, goal) = (ends.lead_in[ends.lead_in.len() - 1], ends.lead_out[0]);
        // States: cell × direction it was entered in (6: none).
        let run = search.start(self.block.len());
        // The costs known lead to their own goal: from this one they are less by the
        // cost from here to there (at least `reach` where it is not known).
        let (reach, offset) = match costs {
            Some(known) => {
                for &(idx, c) in &known.cells {
                    (search.to_goal[idx as usize], search.settled[idx as usize]) = (c, run);
                }
                let offset = if known.goal == goal { 0.0 } else if search.settled[goal] == run { search.to_goal[goal] } else { known.reach };
                (known.reach, offset)
            }
            None => {
                let (reach, cells) = self.costs_to_goal(search, run, start, goal, req, rules);
                // The start not reached even without turns: there is no way.
                let reached = search.settled[start] == run;
                if cells.len() <= KEEP_COSTS {
                    *costs = Some(GoalCosts { goal, cells: cells.into_iter().map(|i| (i as u32, search.to_goal[i])).collect(), reach });
                }
                if !reached {
                    return None;
                }
                (reach, 0.0)
            }
        };
        let Search { best, prev, stamp, to_goal, settled, runs, run_stamp, pass, pass_stamp, .. } = search;
        let mut memo = Memo { pass, pass_stamp, runs, run_stamp, run };
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
            let known = if settled[idx] == run { to_goal[idx] } else { reach } - offset;
            let cells = known.max((0..3).map(|i| c[i].abs_diff(gc[i])).sum::<usize>() as f32 * self.cell);
            cells + turns as f32 * req.turn
        };
        let key = |g: f32, state: usize| Reverse(((g + h(state)).to_bits(), state));
        // The way out of the start costs its cells.
        let first = start * 7 + ends.dir;
        let g0 = ends.lead_in.windows(2).map(|w| self.step_cost(&mut memo, w[0], w[1], ends.dir, req, rules).unwrap_or(0.0)).sum::<f32>();
        (best[first], prev[first], stamp[first]) = (g0, u32::MAX, run);
        heap.push(key(g0, first));
        let jump = straight.max(1);
        // Extra cost of a bend `k` cells after the one before it, fewer than a run.
        let close = |k: usize| req.turn * CLOSE * (straight - k) as f32 / straight as f32;
        let mut found = None;
        while let Some(Reverse(popped)) = heap.pop() {
            let state = popped.1;
            let (idx, dir) = (state / 7, state % 7);
            let g = best[state];
            // A state reached again more cheaply was expanded with its better cost.
            if key(g, state).0 != popped {
                continue;
            }
            if idx == goal && (ends.arrive == 6 || dir != ends.arrive ^ 1) {
                found = Some(state);
                break;
            }
            // A state reached more cheaply than before, from this one.
            macro_rules! relax {
                ($ns:expr, $ng:expr) => {{
                    let (ns, ng): (usize, f32) = ($ns, $ng);
                    if stamp[ns] != run || ng < best[ns] {
                        (best[ns], prev[ns], stamp[ns]) = (ng, state as u32, run);
                        heap.push(key(ng, ns));
                    }
                }};
            }
            // The state is known already at no more than this cost: no way through
            // here, which costs at least that, improves it.
            macro_rules! settled_under {
                ($ns:expr, $floor:expr) => {{
                    let (ns, floor): (usize, f32) = ($ns, $floor);
                    stamp[ns] == run && best[ns] <= floor
                }};
            }
            for nd in 0..6 {
                if nd == dir {
                    // Straight on, one cell.
                    if let Some((ni, cost)) = self.step(idx, nd).and_then(|ni| self.step_cost(&mut memo, idx, ni, nd, req, rules).map(|c| (ni, c))) {
                        relax!(ni * 7 + nd, g + cost);
                    }
                    continue;
                }
                if dir < 6 && nd == dir ^ 1 {
                    continue; // no way back
                }
                // A turn (none at a start without direction), then straight on for
                // `jump` cells (or to the goal, heading into its end), where all of them
                // are open.
                let turn = match (dir < 6, dir < 6 && self.turn_room(idx, dir, nd, req)) {
                    (false, _) => 0.0,
                    (true, true) => req.turn,
                    (true, false) => req.turn * (1.0 + TIGHT),
                };
                let run_cells = if dir < 6 { jump } else { 1 };
                let (mut at, mut ng) = (idx, g + turn);
                for k in 1..=run_cells {
                    let Some((ni, cost)) = self.step(at, nd).and_then(|ni| self.step_cost(&mut memo, at, ni, nd, req, rules).map(|c| (ni, c))) else {
                        at = usize::MAX;
                        break;
                    };
                    ng += cost;
                    at = ni;
                    if at == goal {
                        // Heading into the end the run ends here; across it, the bend
                        // into the end comes close to this one, at a price.
                        if ends.arrive == 6 || nd == ends.arrive {
                            break;
                        }
                        if k < straight {
                            relax!(at * 7 + nd, ng + close(k));
                        }
                        continue;
                    }
                    // The goal beside the run, fewer cells aside than a run, back in the
                    // direction it came (the way into the end running on): a lane change
                    // with an S-bend into it.
                    if let Some((sd, k2)) = (straight > 1 && dir < 6).then(|| self.beside(at, nd, goal, straight - 1)).flatten() {
                        if sd == dir && (ends.arrive == 6 || sd == ends.arrive) {
                            let back = if self.turn_room(at, nd, sd, req) { req.turn } else { req.turn * (1.0 + TIGHT) };
                            let (mut c, mut cost) = (at, ng + back);
                            for _ in 0..k2 {
                                match self.step(c, sd).and_then(|ni| self.step_cost(&mut memo, c, ni, sd, req, rules).map(|x| (ni, x))) {
                                    Some((ni, x)) => (c, cost) = (ni, cost + x),
                                    None => break,
                                }
                            }
                            if c == goal {
                                relax!(goal * 7 + sd, cost);
                            }
                        }
                    }
                    if dir < 6 && k < straight {
                        // Fewer cells aside than a run, then a turn and a run: back into
                        // the direction it came, a lane change with an S-bend; across
                        // both, two bends closer than their radius, at a price growing
                        // with how close.
                        let other = 3 - dir / 2 - nd / 2;
                        for (nd2, price) in [(dir, 0.0), (other * 2, close(k)), (other * 2 + 1, close(k))] {
                            // The run lands `jump` cells on (unless it ends at the goal
                            // before): known cheaper already, it is not looked at.
                            let floor = ng + req.turn + price + jump as f32 * self.cell;
                            if self.beyond(at, nd2, jump).is_some_and(|end| settled_under!(end * 7 + nd2, floor)) && !self.on_line(at, nd2, goal, jump) {
                                continue;
                            }
                            let back = if self.turn_room(at, nd, nd2, req) { req.turn } else { req.turn * (1.0 + TIGHT) };
                            let (run, through) = self.run_memo(&mut memo, at, nd2, jump, goal, ends.arrive, req, rules);
                            if let Some((fwd, fg)) = run {
                                relax!(fwd * 7 + nd2, ng + back + price + fg);
                            }
                            if let Some((k2, gg)) = through.filter(|(k2, _)| *k2 < straight) {
                                relax!(goal * 7 + nd2, ng + back + price + gg + close(k2));
                            }
                        }
                        // Or on from here any way, with the next bend wherever it comes,
                        // at the price of one as close as can be.
                        if k == 1 && straight > 1 {
                            relax!(at * 7 + nd, ng + close(1));
                        }
                    }
                }
                if at != usize::MAX {
                    relax!(at * 7 + nd, ng);
                }
            }
        }
        let mut state = found?;
        // The cells of the way: the runs between the states filled in (a step aside
        // first, then the run in the direction the state was entered in).
        let mut cells: Vec<usize> = vec![state / 7];
        while prev[state] != u32::MAX {
            let from = prev[state] as usize;
            let (ca, cb) = (self.coords(from / 7), self.coords(state / 7));
            let along = (state % 7) / 2;
            let aside = (0..3).find(|&k| k != along && ca[k] != cb[k]);
            let mut c = cb;
            for axis in [Some(along), aside].into_iter().flatten() {
                while c[axis] != ca[axis] {
                    c[axis] = if ca[axis] < cb[axis] { c[axis] - 1 } else { c[axis] + 1 };
                    cells.push(self.index(c));
                }
            }
            state = from;
        }
        cells.reverse();
        let mut all: Vec<usize> = ends.lead_in[..ends.lead_in.len() - 1].to_vec();
        all.extend(cells);
        all.extend_from_slice(&ends.lead_out[1..]);
        let mut points = vec![a];
        points.extend(all.iter().map(|&i| self.centre(i)));
        points.push(b);
        let points = simplify(points);
        let free = all.iter().filter(|&&i| !self.channel(i, req)).count() as f32 * self.cell;
        let squeezed = all.iter().any(|&i| !self.near_end(i, req) && self.room_for(i, req.ignore) < req.half);
        Some(Way { points, cells: all, free, squeezed })
    }

    /// A run of `cells` cells from `at` in direction `dir` (fewer up to the `goal` where
    /// it heads into its end, `arrive`), where all of them are open: the cell it ends
    /// at and what its cells cost (remembered for the search), and where it passes the
    /// goal across the way into its end, after how many cells and at what cost.
    #[allow(clippy::too_many_arguments)]
    fn run_memo(&self, memo: &mut Memo, at: usize, dir: usize, cells: usize, goal: usize, arrive: usize, req: &Request, rules: &RoutingRules) -> (RunEnd, RunEnd) {
        let key = at * 6 + dir;
        if memo.run_stamp[key] != memo.run {
            let (mut end, mut g) = (at, 0.0);
            let mut open = true;
            for _ in 0..cells {
                let Some((ni, cost)) = self.step(end, dir).and_then(|ni| self.step_cost(memo, end, ni, dir, req, rules).map(|c| (ni, c))) else {
                    open = false;
                    break;
                };
                g += cost;
                end = ni;
                if end == goal && (arrive == 6 || dir == arrive) {
                    break;
                }
            }
            memo.runs[key] = if open { (end as u32, g) } else { (u32::MAX, 0.0) };
            memo.run_stamp[key] = memo.run;
        }
        let (e, c) = memo.runs[key];
        // The goal on the way across the way into its end: ahead, within the run.
        let through = (arrive != 6 && dir != arrive && self.on_line(at, dir, goal, cells)).then(|| {
            let (mut end, mut g) = (at, 0.0);
            for k in 1..=cells {
                let (ni, cost) = self.step(end, dir).and_then(|ni| self.step_cost(memo, end, ni, dir, req, rules).map(|c| (ni, c)))?;
                g += cost;
                end = ni;
                if end == goal {
                    return Some((k, g));
                }
            }
            None
        });
        ((e != u32::MAX).then_some((e as usize, c)), through.flatten())
    }

    /// The costs from the cells to `goal` without turns and without the other routes
    /// (Dijkstra backwards: they only make the way longer), with the price of the cells
    /// a route is squeezed through, until the one of `start` is known. Gives the
    /// highest known cost (the other cells cost at least as much) and the cells known.
    fn costs_to_goal(&self, search: &mut Search, run: u32, start: usize, goal: usize, req: &Request, rules: &RoutingRules) -> (f32, Vec<usize>) {
        let Search { to_goal, reached, settled, .. } = search;
        let mut heap = BinaryHeap::new();
        (to_goal[goal], reached[goal]) = (0.0, run);
        heap.push(Reverse((0.0f32.to_bits(), goal)));
        let mut reach = 0.0;
        let mut known = Vec::new();
        while let Some(Reverse((d, idx))) = heap.pop() {
            if settled[idx] == run {
                continue;
            }
            settled[idx] = run;
            known.push(idx);
            reach = f32::from_bits(d);
            if idx == start {
                break;
            }
            // Entering this cell from a neighbour costs its factor.
            let squeeze = if self.level(idx, req, false) == SQUEEZED { SQUEEZE } else { 1.0 };
            for nd in 0..6 {
                let Some(ni) = self.step(idx, nd) else { continue };
                if settled[ni] == run || !self.open_for(ni, req) || (nd / 2 == 0 && !self.crossing_ok(ni, idx)) {
                    continue;
                }
                let nd = reach + self.cell * self.factor(idx, nd / 2, req, rules) * squeeze;
                if reached[ni] != run || nd < to_goal[ni] {
                    (to_goal[ni], reached[ni]) = (nd, run);
                    heap.push(Reverse((nd.to_bits(), ni)));
                }
            }
        }
        (reach, known)
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

/// A slab with rectangular `openings` ([y range], [z range]) cut out of it, as boxes.
fn cut_openings(slab: (V3, V3), openings: &[([f32; 2], [f32; 2])]) -> Vec<(V3, V3)> {
    let mut boxes = vec![slab];
    for &(y, z) in openings {
        let mut next = Vec::new();
        for (lo, hi) in boxes {
            if y[0] >= hi[1] || y[1] <= lo[1] || z[0] >= hi[2] || z[1] <= lo[2] {
                next.push((lo, hi));
                continue;
            }
            // Above and below the opening across the whole slab, beside it between.
            if y[0] > lo[1] {
                next.push((lo, [hi[0], y[0], hi[2]]));
            }
            if y[1] < hi[1] {
                next.push(([lo[0], y[1], lo[2]], hi));
            }
            let (ya, yb) = (y[0].max(lo[1]), y[1].min(hi[1]));
            if z[0] > lo[2] {
                next.push(([lo[0], ya, lo[2]], [hi[0], yb, z[0]]));
            }
            if z[1] < hi[2] {
                next.push(([lo[0], ya, z[1]], [hi[0], yb, hi[2]]));
            }
        }
        boxes = next;
    }
    boxes
}

/// Distance of a point from a box (0 inside).
fn box_distance(q: V3, lo: V3, hi: V3) -> f32 {
    let d = [0, 1, 2].map(|i| (lo[i] - q[i]).max(q[i] - hi[i]).max(0.0));
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// Distance of a point of a floor plan from its outline.
fn plan_distance(plan: &[[f32; 2]], q: [f32; 2]) -> f32 {
    let mut best = f32::INFINITY;
    for i in 0..plan.len() {
        let (a, b) = (plan[i], plan[(i + 1) % plan.len()]);
        let ab = [b[0] - a[0], b[1] - a[1]];
        let l2 = ab[0] * ab[0] + ab[1] * ab[1];
        let t = if l2 <= f32::EPSILON { 0.0 } else { (((q[0] - a[0]) * ab[0] + (q[1] - a[1]) * ab[1]) / l2).clamp(0.0, 1.0) };
        let p = [a[0] + ab[0] * t, a[1] + ab[1] * t];
        best = best.min((q[0] - p[0]).hypot(q[1] - p[1]));
    }
    best
}

/// Unit vector of a direction (axis × 2 + 1 for up).
fn unit(dir: usize) -> V3 {
    let mut v = [0.0; 3];
    v[dir / 2] = if dir % 2 == 1 { 1.0 } else { -1.0 };
    v
}

fn dist_to_segment(q: V3, a: V3, b: V3) -> f32 {
    let ab = sub(b, a);
    let l2 = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
    let t = if l2 <= f32::EPSILON { 0.0 } else { (((q[0] - a[0]) * ab[0] + (q[1] - a[1]) * ab[1] + (q[2] - a[2]) * ab[2]) / l2).clamp(0.0, 1.0) };
    dist(q, add(a, scale(ab, t)))
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

/// Stretches of a polyline between two bends shorter than `least`: between corners
/// of the grid (the short ways from the ends onto it and out of the nozzles are no
/// bends), not counting the lane changes with an S-bend (a stretch between two bends
/// that turn it back into the direction it came).
pub fn close_bends(points: &[V3], least: f32) -> usize {
    let sign = |v: f32| if v > 1e-3 { 1 } else if v < -1e-3 { -1 } else { 0 };
    let heading = |a: V3, b: V3| [0, 1, 2].map(|i| sign(b[i] - a[i]));
    let along_axis = |h: [i8; 3]| h.iter().filter(|v| **v != 0).count() == 1;
    let n = points.len();
    if n < 4 {
        return 0;
    }
    (1..n - 2)
        .filter(|&i| {
            // The stretch from corner i to corner i + 1.
            let (before, along, after) = (heading(points[i - 1], points[i]), heading(points[i], points[i + 1]), heading(points[i + 1], points[i + 2]));
            along_axis(before) && along_axis(along) && along_axis(after) && before != after && dist(points[i], points[i + 1]) < least
        })
        .count()
}

#[cfg(test)]
mod tests;
