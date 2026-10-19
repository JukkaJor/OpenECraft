//! Panel layout (in the spirit of EPLAN Pro Panel): enclosures with a mounting
//! plate, mounting rails, cable ducts and devices placed with their real dimensions
//! from the parts data, collision checks and virtual wiring through the duct network.
//!
//! Coordinates on the mounting plate are millimetres, origin at the top left corner
//! of the plate, x to the right, y downwards (like schematic pages). `z` is the height
//! above the plate surface.

use crate::geom::{p, P, R};
use crate::model::Project;
use crate::netlist::{connections, Netlist};
use crate::parts::{self, FnKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};

/// Distance between enclosure wall and mounting plate edge.
pub const PLATE_MARGIN: f32 = 25.0;
/// Distance of the plate surface from the rear wall.
pub const PLATE_Z: f32 = 20.0;
pub const PLATE_THICKNESS: f32 = 3.0;
/// Snap grid on the mounting plate.
pub const SNAP: f32 = 5.0;
/// Mounting rail profile (width × height) if the part has no data.
pub const RAIL_PROFILE: (f32, f32) = (35.0, 7.5);
/// Cable duct profile (width × depth) if the part has no data.
pub const DUCT_PROFILE: (f32, f32) = (40.0, 60.0);
/// Device size if the part has no mounting data.
pub const DEFAULT_DEVICE: (f32, f32, f32) = (40.0, 40.0, 40.0);

pub const RAIL_PART: &str = "GEN.RAIL-TS35-1M";
pub const DUCT_PART: &str = "GEN.DUCT-40X60";
pub const ENCLOSURE_PART: &str = "GEN.ENC-600X800";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemKind {
    #[default]
    Device,
    Rail,
    Duct,
}

impl ItemKind {
    pub fn title(self) -> &'static str {
        match self {
            ItemKind::Device => "Device",
            ItemKind::Rail => "Mounting rail",
            ItemKind::Duct => "Cable duct",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelItem {
    pub id: u64,
    pub kind: ItemKind,
    /// Device tag (devices only).
    pub tag: String,
    pub part: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Height of the item (extent away from the plate).
    pub d: f32,
    /// Base height above the plate (devices on a rail sit on the rail).
    pub z: f32,
}

impl PanelItem {
    pub fn rect(&self) -> R {
        R::from_points(p(self.x, self.y), p(self.x + self.w, self.y + self.h))
    }

    pub fn center(&self) -> P {
        p(self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    pub fn horizontal(&self) -> bool {
        self.w >= self.h
    }

    /// Rails and ducts: the running length.
    pub fn length(&self) -> f32 {
        self.w.max(self.h)
    }

    /// Centre line of a rail / duct.
    pub fn center_line(&self) -> (P, P) {
        if self.horizontal() {
            let y = self.y + self.h / 2.0;
            (p(self.x, y), p(self.x + self.w, y))
        } else {
            let x = self.x + self.w / 2.0;
            (p(x, self.y), p(x, self.y + self.h))
        }
    }

    /// Rotates by 90° around the centre.
    pub fn rotate(&mut self) {
        let c = self.center();
        std::mem::swap(&mut self.w, &mut self.h);
        self.x = c.x - self.w / 2.0;
        self.y = c.y - self.h / 2.0;
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Enclosure {
    pub id: u64,
    /// Location designation, e.g. "+CAB1".
    pub name: String,
    pub part: String,
    pub width: f32,
    pub height: f32,
    pub depth: f32,
    pub items: Vec<PanelItem>,
}

impl Default for Enclosure {
    fn default() -> Self {
        Enclosure {
            id: 0,
            name: "+CAB1".into(),
            part: String::new(),
            width: 600.0,
            height: 800.0,
            depth: 250.0,
            items: Vec::new(),
        }
    }
}

impl Enclosure {
    pub fn plate_size(&self) -> (f32, f32) {
        ((self.width - 2.0 * PLATE_MARGIN).max(10.0), (self.height - 2.0 * PLATE_MARGIN).max(10.0))
    }

    pub fn plate_rect(&self) -> R {
        let (w, h) = self.plate_size();
        R::from_points(p(0.0, 0.0), p(w, h))
    }

    pub fn item(&self, id: u64) -> Option<&PanelItem> {
        self.items.iter().find(|i| i.id == id)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub enclosures: Vec<Enclosure>,
    /// Additional wire length per connection (stripping, reserve loops) in mm.
    pub wire_reserve: f32,
}

impl Default for Layout {
    fn default() -> Self {
        Layout { enclosures: Vec::new(), wire_reserve: 100.0 }
    }
}

impl Layout {
    pub fn placed_tags(&self) -> HashSet<&str> {
        self.enclosures
            .iter()
            .flat_map(|e| e.items.iter())
            .filter(|i| i.kind == ItemKind::Device)
            .map(|i| i.tag.as_str())
            .collect()
    }

    /// Enclosure index and item of a placed device.
    pub fn find_device(&self, tag: &str) -> Option<(usize, &PanelItem)> {
        self.enclosures.iter().enumerate().find_map(|(ei, e)| {
            e.items.iter().find(|i| i.kind == ItemKind::Device && i.tag == tag).map(|i| (ei, i))
        })
    }
}

/// Snaps a value to the plate grid.
pub fn snap(v: f32) -> f32 {
    (v / SNAP).round() * SNAP
}

// ---------------------------------------------------------------------- creation

/// Creates an enclosure, taking the dimensions from a part if it has them.
pub fn new_enclosure(prj: &mut Project, part: &str, name: &str) -> Enclosure {
    let mut e = Enclosure { id: prj.new_id(), name: name.into(), part: part.into(), ..Default::default() };
    if let Some(pd) = prj.parts.iter().find(|x| x.number == part) {
        if pd.width > 0.0 && pd.height > 0.0 {
            e.width = pd.width;
            e.height = pd.height;
            e.depth = if pd.depth > 0.0 { pd.depth } else { e.depth };
        }
    }
    e
}

/// Rail or duct between two points (orientation by the dominant direction).
pub fn new_profile(prj: &mut Project, kind: ItemKind, part: &str, a: P, b: P) -> PanelItem {
    let (pw, pd) = profile(prj, kind, part);
    let horizontal = (b.x - a.x).abs() >= (b.y - a.y).abs();
    let (x, y, w, h) = if horizontal {
        (a.x.min(b.x), a.y - pw / 2.0, (b.x - a.x).abs(), pw)
    } else {
        (a.x - pw / 2.0, a.y.min(b.y), pw, (b.y - a.y).abs())
    };
    PanelItem { id: prj.new_id(), kind, part: part.into(), x, y, w: w.max(SNAP), h: h.max(SNAP), d: pd, ..Default::default() }
}

/// Profile width and height of rails / ducts (from the part or the defaults).
pub fn profile(prj: &Project, kind: ItemKind, part: &str) -> (f32, f32) {
    let def = if kind == ItemKind::Rail { RAIL_PROFILE } else { DUCT_PROFILE };
    match prj.parts.iter().find(|x| x.number == part) {
        Some(pd) if pd.height > 0.0 && pd.depth > 0.0 => (pd.height, pd.depth),
        _ => def,
    }
}

/// A device that can be placed: tag, size and whether the size came from part data.
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceSize {
    pub tag: String,
    pub part: String,
    pub w: f32,
    pub h: f32,
    pub d: f32,
    pub from_part: bool,
}

/// Footprint of a device from its parts: the first part with mounting data gives
/// width / height; further parts (e.g. auxiliary blocks) add their depth. Terminal
/// strips are as wide as all their terminals together.
pub fn device_size(prj: &Project, tag: &str) -> Option<DeviceSize> {
    let data = prj.devices.get(tag)?;
    let parts: Vec<_> = data
        .parts
        .iter()
        .filter_map(|r| prj.parts.iter().find(|x| x.number == r.number).map(|pd| (pd, r.count.max(1))))
        .collect();
    let (main, _) = parts.iter().find(|(pd, _)| pd.width > 0.0 && pd.height > 0.0)?;
    let terminals = parts::placed_functions(prj, tag).iter().filter(|f| f.0 == FnKind::Terminal).count();
    let mut w = main.width;
    if terminals > 0 && main.functions.iter().all(|f| f.kind == FnKind::Terminal) {
        w = main.width * terminals as f32;
    }
    let mut d = main.depth;
    for (pd, count) in &parts {
        if pd.number != main.number && pd.depth > 0.0 && pd.width > 0.0 && pd.width <= main.width + 1.0 {
            d += pd.depth * *count as f32;
        }
    }
    Some(DeviceSize { tag: tag.into(), part: main.number.clone(), w, h: main.height, d, from_part: true })
}

/// Devices with parts that are not placed in any enclosure yet. Devices whose parts
/// have no mounting data (e.g. motors in the field) are not listed.
pub fn devices_to_place(prj: &Project) -> Vec<DeviceSize> {
    let placed = prj.layout.placed_tags();
    let mut v: Vec<DeviceSize> = prj
        .devices
        .keys()
        .filter(|t| !placed.contains(t.as_str()))
        .filter_map(|t| device_size(prj, t))
        .collect();
    v.sort_by(|a, b| crate::model::natural_cmp(&a.tag, &b.tag));
    v
}

/// Places a device with its top left corner at `at` (snapped, rail aware).
pub fn place_device(prj: &mut Project, enc: usize, tag: &str, at: P) -> Option<u64> {
    let size = device_size(prj, tag).unwrap_or(DeviceSize {
        tag: tag.into(),
        part: String::new(),
        w: DEFAULT_DEVICE.0,
        h: DEFAULT_DEVICE.1,
        d: DEFAULT_DEVICE.2,
        from_part: false,
    });
    let id = prj.new_id();
    let mut item = PanelItem {
        id,
        kind: ItemKind::Device,
        tag: tag.into(),
        part: size.part,
        x: snap(at.x),
        y: snap(at.y),
        w: size.w,
        h: size.h,
        d: size.d,
        z: 0.0,
    };
    let e = prj.layout.enclosures.get_mut(enc)?;
    snap_to_rail(e, &mut item);
    e.items.push(item);
    Some(id)
}

/// Devices whose centre lies on a horizontal rail are centred on it and mounted on
/// top of it; otherwise they sit directly on the plate.
pub fn snap_to_rail(e: &Enclosure, item: &mut PanelItem) {
    if item.kind != ItemKind::Device {
        return;
    }
    let c = item.center();
    let rail = e.items.iter().find(|r| {
        r.kind == ItemKind::Rail
            && r.horizontal()
            && c.x >= r.x
            && c.x <= r.x + r.w
            && (c.y - r.center().y).abs() <= item.h / 2.0
    });
    match rail {
        Some(r) => {
            item.y = r.center().y - item.h / 2.0;
            item.z = r.z + r.d;
        }
        None => item.z = 0.0,
    }
}

// ---------------------------------------------------------------------- checks

/// Pairs of items that occupy the same space.
pub fn collisions(e: &Enclosure) -> Vec<(u64, u64)> {
    let mut v = Vec::new();
    for (i, a) in e.items.iter().enumerate() {
        for b in &e.items[i + 1..] {
            let ra = a.rect();
            let rb = b.rect();
            let eps = 0.01;
            let xy = ra.min.x < rb.max.x - eps && rb.min.x < ra.max.x - eps && ra.min.y < rb.max.y - eps && rb.min.y < ra.max.y - eps;
            let z = a.z < b.z + b.d - eps && b.z < a.z + a.d - eps;
            // Crossing rails / ducts of the same kind are joined, not colliding.
            let joint = a.kind == b.kind && a.kind != ItemKind::Device && a.horizontal() != b.horizontal();
            if xy && z && !joint {
                v.push((a.id, b.id));
            }
        }
    }
    v
}

/// Items that leave the mounting plate or are higher than the enclosure allows.
pub fn outside(e: &Enclosure) -> Vec<u64> {
    let plate = e.plate_rect().expand(0.01);
    let max_z = e.depth - PLATE_Z - PLATE_THICKNESS;
    e.items
        .iter()
        .filter(|i| !plate.contains_rect(&i.rect()) || i.z + i.d > max_z)
        .map(|i| i.id)
        .collect()
}

// ---------------------------------------------------------------------- routing

#[derive(Clone, Debug, PartialEq)]
pub struct Route {
    /// Path on the plate (plate coordinates).
    pub points: Vec<P>,
    /// Length along the plate (without the vertical parts and reserve).
    pub plate_length: f32,
    pub via_ducts: bool,
}

/// Manhattan length of a polyline.
fn manhattan(a: P, b: P) -> f32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

fn closest_on_segment(q: P, a: P, b: P) -> P {
    let ab = b - a;
    let l2 = ab.x * ab.x + ab.y * ab.y;
    if l2 <= f32::EPSILON {
        return a;
    }
    let t = (((q.x - a.x) * ab.x + (q.y - a.y) * ab.y) / l2).clamp(0.0, 1.0);
    a + ab * t
}

/// Routes a wire from `a` to `b` through the cable duct network (shortest path).
/// Without ducts the wire runs directly (orthogonally).
pub fn route(e: &Enclosure, a: P, b: P) -> Route {
    let ducts: Vec<&PanelItem> = e.items.iter().filter(|i| i.kind == ItemKind::Duct).collect();
    if ducts.is_empty() {
        let corner = p(b.x, a.y);
        return Route { points: vec![a, corner, b], plate_length: manhattan(a, b), via_ducts: false };
    }
    // Nodes: per duct, the points on its centre line that matter.
    let mut nodes: Vec<P> = vec![a, b];
    let mut on_duct: Vec<Vec<usize>> = vec![Vec::new(); ducts.len()];
    let mut edges: Vec<(usize, usize, f32)> = Vec::new();
    let add_node = |nodes: &mut Vec<P>, q: P| {
        nodes.push(q);
        nodes.len() - 1
    };
    for (di, d) in ducts.iter().enumerate() {
        let (s, t) = d.center_line();
        let ns = add_node(&mut nodes, s);
        let nt = add_node(&mut nodes, t);
        on_duct[di].extend([ns, nt]);
    }
    // Junctions between ducts (crossing, T or touching at the ends).
    for i in 0..ducts.len() {
        for j in i + 1..ducts.len() {
            let (ra, rb) = (ducts[i].rect().expand(1.0), ducts[j].rect().expand(1.0));
            let overlap = ra.min.x <= rb.max.x && rb.min.x <= ra.max.x && ra.min.y <= rb.max.y && rb.min.y <= ra.max.y;
            if !overlap {
                continue;
            }
            let (ai, aj) = (ducts[i].center_line(), ducts[j].center_line());
            // Closest pair of points between the two centre lines (good enough for
            // axis parallel ducts).
            let candidates = [aj.0, aj.1, closest_on_segment(ai.0, aj.0, aj.1), closest_on_segment(ai.1, aj.0, aj.1)];
            let mut best: Option<(P, P, f32)> = None;
            for c in candidates {
                let on_i = closest_on_segment(c, ai.0, ai.1);
                let on_j = closest_on_segment(on_i, aj.0, aj.1);
                let dist = manhattan(on_i, on_j);
                if best.map(|bst| dist < bst.2).unwrap_or(true) {
                    best = Some((on_i, on_j, dist));
                }
            }
            if let Some((pi, pj, dist)) = best {
                let ni = add_node(&mut nodes, pi);
                let nj = add_node(&mut nodes, pj);
                on_duct[i].push(ni);
                on_duct[j].push(nj);
                edges.push((ni, nj, dist));
            }
        }
    }
    // Entries: a wire leaves its device into the nearest duct (not across the
    // plate to some farther duct).
    for (end, q) in [(0usize, a), (1usize, b)] {
        let entries: Vec<(usize, P, f32)> = ducts
            .iter()
            .enumerate()
            .map(|(di, d)| {
                let (s, t) = d.center_line();
                let c = closest_on_segment(q, s, t);
                (di, c, manhattan(q, c))
            })
            .collect();
        let nearest = entries.iter().map(|e| e.2).fold(f32::INFINITY, f32::min);
        for (di, c, dist) in entries.into_iter().filter(|e| e.2 <= nearest + 1.0) {
            let n = add_node(&mut nodes, c);
            on_duct[di].push(n);
            edges.push((end, n, dist));
        }
    }
    // Along each duct: connect consecutive nodes.
    for (di, d) in ducts.iter().enumerate() {
        let horizontal = d.horizontal();
        let list = &mut on_duct[di];
        list.sort_by(|&x, &y| {
            let (px, py) = (nodes[x], nodes[y]);
            if horizontal { px.x.total_cmp(&py.x) } else { px.y.total_cmp(&py.y) }
        });
        for w in list.windows(2) {
            edges.push((w[0], w[1], nodes[w[0]].dist(nodes[w[1]])));
        }
    }
    // Dijkstra (small graphs, simple O(n²)).
    let n = nodes.len();
    let mut adj: Vec<Vec<(usize, f32)>> = vec![Vec::new(); n];
    for &(x, y, w) in &edges {
        adj[x].push((y, w));
        adj[y].push((x, w));
    }
    let mut dist = vec![f32::INFINITY; n];
    let mut prev = vec![usize::MAX; n];
    let mut done = vec![false; n];
    dist[0] = 0.0;
    for _ in 0..n {
        let Some(u) = (0..n).filter(|&i| !done[i] && dist[i].is_finite()).min_by(|&x, &y| dist[x].total_cmp(&dist[y])) else {
            break;
        };
        if u == 1 {
            break;
        }
        done[u] = true;
        for &(v, w) in &adj[u] {
            if dist[u] + w < dist[v] {
                dist[v] = dist[u] + w;
                prev[v] = u;
            }
        }
    }
    if !dist[1].is_finite() {
        let corner = p(b.x, a.y);
        return Route { points: vec![a, corner, b], plate_length: manhattan(a, b), via_ducts: false };
    }
    let mut path = vec![1];
    while *path.last().unwrap() != 0 {
        path.push(prev[*path.last().unwrap()]);
    }
    path.reverse();
    let mut points: Vec<P> = Vec::new();
    for idx in path {
        let q = nodes[idx];
        if let Some(&last) = points.last() {
            // Orthogonal legs for the way into / out of the duct.
            let last: P = last;
            if last.x != q.x && last.y != q.y {
                points.push(p(q.x, last.y));
            }
        }
        if points.last().map(|l: &P| !l.same(q)).unwrap_or(true) {
            points.push(q);
        }
    }
    Route { points, plate_length: dist[1], via_ducts: true }
}

/// A routed wire of the virtual wiring.
#[derive(Clone, Debug)]
pub struct RoutedWire {
    pub enclosure: usize,
    pub from: String,
    pub to: String,
    pub net: String,
    pub color: String,
    pub cross_section: String,
    pub route: Route,
    /// Total length including the way down to the plate and the reserve.
    pub length: f32,
    /// (from device item, to device item)
    pub items: (u64, u64),
}

impl RoutedWire {
    /// Stable identification of a wire across re-analysis (its two ends).
    pub fn key(&self) -> (String, String) {
        (self.from.clone(), self.to.clone())
    }
}

/// Distance from `q` to a route polyline.
pub fn route_distance(points: &[P], q: P) -> f32 {
    points.windows(2).map(|s| crate::geom::dist_to_segment(q, s[0], s[1])).fold(f32::INFINITY, f32::min)
}

/// Indices of the wires of an enclosure whose route passes within `tol` of `q`,
/// nearest first.
pub fn wires_at(wiring: &[RoutedWire], enclosure: usize, q: P, tol: f32) -> Vec<usize> {
    let mut v: Vec<(usize, f32)> = wiring
        .iter()
        .enumerate()
        .filter(|(_, w)| w.enclosure == enclosure)
        .map(|(i, w)| (i, route_distance(&w.route.points, q)))
        .filter(|(_, d)| *d <= tol)
        .collect();
    v.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    v.into_iter().map(|(i, _)| i).collect()
}

/// Clicking repeatedly on overlapping wires steps through them: the candidate after
/// the current one, or the first.
pub fn next_candidate(candidates: &[usize], current: Option<usize>) -> Option<usize> {
    match current.and_then(|c| candidates.iter().position(|x| *x == c)) {
        Some(pos) => Some(candidates[(pos + 1) % candidates.len()]),
        None => candidates.first().copied(),
    }
}

/// Unrouted connection (at least one device not placed or in different enclosures).
#[derive(Clone, Debug)]
pub struct OpenWire {
    pub from: String,
    pub to: String,
    pub net: String,
    pub reason: &'static str,
}

/// Virtual wiring: routes every point-to-point connection of the schematic between
/// placed devices through the cable ducts.
pub fn virtual_wiring(prj: &Project, nl: &Netlist) -> (Vec<RoutedWire>, Vec<OpenWire>) {
    let mut routed = Vec::new();
    let mut open = Vec::new();
    for c in connections(prj, nl) {
        let sa = &prj.pages[c.a.page].symbols[c.a.sym];
        let sb = &prj.pages[c.b.page].symbols[c.b.sym];
        let from = crate::netlist::pin_designation(prj, c.a);
        let to = crate::netlist::pin_designation(prj, c.b);
        if sa.tag == sb.tag {
            continue; // internal connection (e.g. bridged on the device)
        }
        let (Some((ea, ia)), Some((eb, ib))) = (prj.layout.find_device(&sa.tag), prj.layout.find_device(&sb.tag)) else {
            open.push(OpenWire { from, to, net: c.net.clone(), reason: "device not placed" });
            continue;
        };
        if ea != eb {
            open.push(OpenWire { from, to, net: c.net.clone(), reason: "different enclosures" });
            continue;
        }
        let e = &prj.layout.enclosures[ea];
        let r = route(e, ia.center(), ib.center());
        // Down from the device front to the plate and up again at the other end.
        let length = r.plate_length + ia.z + ia.d + ib.z + ib.d + prj.layout.wire_reserve;
        routed.push(RoutedWire {
            enclosure: ea,
            from,
            to,
            net: c.net,
            color: c.color,
            cross_section: c.cross_section,
            route: r,
            length,
            items: (ia.id, ib.id),
        });
    }
    (routed, open)
}

/// Material for the BOM: (part number, quantity, quantity unit) of rails, ducts and
/// enclosures. Rails / ducts are counted by cut length.
pub fn layout_material(prj: &Project) -> Vec<(String, f32, Vec<String>)> {
    let mut out: Vec<(String, f32, Vec<String>)> = Vec::new();
    let add = |out: &mut Vec<(String, f32, Vec<String>)>, part: &str, qty: f32, used: String| {
        if part.is_empty() {
            return;
        }
        match out.iter_mut().find(|x| x.0 == part) {
            Some(x) => {
                x.1 += qty;
                if !x.2.contains(&used) {
                    x.2.push(used);
                }
            }
            None => out.push((part.to_string(), qty, vec![used])),
        }
    };
    for e in &prj.layout.enclosures {
        add(&mut out, &e.part, 1.0, e.name.clone());
        let mut lengths: Vec<(String, f32)> = Vec::new();
        for i in e.items.iter().filter(|i| i.kind != ItemKind::Device) {
            match lengths.iter_mut().find(|x| x.0 == i.part) {
                Some(x) => x.1 += i.length(),
                None => lengths.push((i.part.clone(), i.length())),
            }
        }
        for (part, mm) in lengths {
            let pd = prj.parts.iter().find(|x| x.number == part);
            // Parts sold per metre are counted in metres, others in pieces of their length.
            let qty = match pd {
                Some(pd) if pd.quantity_unit == "m" => (mm / 1000.0 * 100.0).ceil() / 100.0,
                Some(pd) if pd.width > 0.0 => (mm / pd.width).ceil(),
                _ => (mm / 1000.0 * 100.0).ceil() / 100.0,
            };
            add(&mut out, &part, qty, e.name.clone());
        }
    }
    out
}

/// Item ids involved in any problem (for highlighting).
pub fn problem_items(e: &Enclosure) -> BTreeSet<u64> {
    let mut s: BTreeSet<u64> = collisions(e).into_iter().flat_map(|(a, b)| [a, b]).collect();
    s.extend(outside(e));
    s
}

#[cfg(test)]
mod tests;
