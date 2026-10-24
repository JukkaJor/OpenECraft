//! 3D routing of the panel layout: wires, cables and tubes through the line-ups of
//! enclosures, in cable ducts, along routing paths or freely through the air around
//! the devices, with segregation of classes (power, control, signal, fluid), routing
//! reservations, bend radii, cable bundles and their branches, shield handling and a
//! recalculation that keeps routes frozen for production as long as they are valid.
//!
//! Wires are routed through the cable ducts by [`crate::panel::virtual_wiring`]; with
//! [`RoutingRules::wires_3d`] they are routed in 3D like cables and tubes. All routes
//! are polylines in layout coordinates ([`crate::panel::positions`]: enclosure
//! coordinates of the whole layout, x right, y down, z from the rear walls).

use crate::model::{Cable, Project};
use crate::netlist::{Connection, Netlist};
use crate::panel::{self, Face, ItemKind, PanelItem, RoutedWire, DUCT_PROFILE, PLATE_MARGIN, PLATE_Z};
use crate::parts::vec3::{add, manhattan, sub, V3};
use openecraft_i18n::{tr, tr_fmt};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

pub mod bundles;
pub mod grid;
pub mod shield;
pub use bundles::{Branch, Bundle};
pub use shield::{is_shielded, ShieldEnd, ShieldInfo, Termination};

// ---------------------------------------------------------------------- rules

/// Segregation class of what is routed: classes that must keep apart avoid each
/// other's neighbourhood ([`RoutingRules::segregation`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Class {
    /// Mains and motor circuits (phase, neutral, PE, more than 50 V AC / 120 V DC).
    Power,
    /// Control circuits (24 V DC …).
    #[default]
    Control,
    /// Signals, analogue values, bus and network cables, shielded cables.
    Signal,
    /// Hoses and pipes.
    Fluid,
}

impl Class {
    pub const ALL: [Class; 4] = [Class::Power, Class::Control, Class::Signal, Class::Fluid];

    pub fn title(self) -> &'static str {
        match self {
            Class::Power => tr("Power"),
            Class::Control => tr("Control"),
            Class::Signal => tr("Signal"),
            Class::Fluid => tr("Fluid"),
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Class::Power => "power",
            Class::Control => "control",
            Class::Signal => "signal",
            Class::Fluid => "fluid",
        }
    }

    pub fn from_key(key: &str) -> Option<Class> {
        Class::ALL.into_iter().find(|c| c.key() == key.trim().to_ascii_lowercase())
    }

    pub fn bit(self) -> u8 {
        1 << self as u8
    }

    /// The two classes must be kept apart: power from signals and from fluids.
    pub fn keeps_apart(self, other: Class) -> bool {
        matches!((self, other), (Class::Power, Class::Signal) | (Class::Signal, Class::Power) | (Class::Power, Class::Fluid) | (Class::Fluid, Class::Power))
    }

    /// Bits of the classes this one keeps apart from.
    pub fn apart_bits(self) -> u8 {
        Class::ALL.into_iter().filter(|c| self.keeps_apart(*c)).fold(0, |m, c| m | c.bit())
    }
}

/// What is routed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteKind {
    #[default]
    Wire,
    Cable,
    Tube,
}

impl RouteKind {
    pub fn title(self) -> &'static str {
        match self {
            RouteKind::Wire => tr("Wire"),
            RouteKind::Cable => tr("Cable"),
            RouteKind::Tube => tr("Tube"),
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            RouteKind::Wire => "wire",
            RouteKind::Cable => "cable",
            RouteKind::Tube => "tube",
        }
    }
}

/// What a routing path is for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PathKind {
    #[default]
    Any,
    Wires,
    Cables,
    Tubes,
}

impl PathKind {
    pub const ALL: [PathKind; 4] = [PathKind::Any, PathKind::Wires, PathKind::Cables, PathKind::Tubes];

    pub fn title(self) -> &'static str {
        match self {
            PathKind::Any => tr("Wires, cables and tubes"),
            PathKind::Wires => tr("Wires"),
            PathKind::Cables => tr("Cables"),
            PathKind::Tubes => tr("Tubes"),
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            PathKind::Any => "any",
            PathKind::Wires => "wires",
            PathKind::Cables => "cables",
            PathKind::Tubes => "tubes",
        }
    }

    pub fn from_key(key: &str) -> Option<PathKind> {
        PathKind::ALL.into_iter().find(|k| k.key() == key.trim().to_ascii_lowercase())
    }

    pub fn accepts(self, kind: RouteKind) -> bool {
        matches!((self, kind), (PathKind::Any, _) | (PathKind::Wires, RouteKind::Wire) | (PathKind::Cables, RouteKind::Cable) | (PathKind::Tubes, RouteKind::Tube))
    }
}

/// A routing path drawn in an enclosure: a channel through free space (cable tray,
/// cable clamp rail, tube guide) that routes prefer.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RoutingPath {
    pub id: u64,
    pub name: String,
    pub kind: PathKind,
    /// Corner points in enclosure coordinates (x right, y down from the top left
    /// front corner, z from the rear wall).
    pub points: Vec<V3>,
    /// Width of the path: diameter of the space its contents may take, mm.
    pub width: f32,
    /// Classes allowed on the path; empty: all.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<Class>,
    /// Share of the capacity kept free for later additions, percent.
    pub reserve: f32,
}

impl RoutingPath {
    /// Usable cross-section in mm² (without the reserve).
    pub fn capacity(&self) -> f32 {
        std::f32::consts::PI / 4.0 * self.width * self.width * (1.0 - self.reserve.clamp(0.0, 100.0) / 100.0)
    }

    pub fn length(&self) -> f32 {
        grid::length(&self.points)
    }
}

/// What a reservation keeps the space for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReserveMode {
    /// Nothing is routed through it (room for later devices, the swing of the door).
    #[default]
    Keepout,
    /// Only this class is routed through it.
    For(Class),
}

impl ReserveMode {
    pub fn title(self) -> String {
        match self {
            ReserveMode::Keepout => tr("Keep free").to_string(),
            ReserveMode::For(c) => tr_fmt!("Only {}", c.title()),
        }
    }
}

/// A routing reservation: a box of an enclosure (enclosure coordinates).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Reservation {
    pub id: u64,
    pub name: String,
    pub min: V3,
    pub max: V3,
    pub mode: ReserveMode,
}

/// Ends at which the shield of a cable is connected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShieldEnds {
    #[default]
    Both,
    First,
    Second,
    None,
}

impl ShieldEnds {
    pub const ALL: [ShieldEnds; 4] = [ShieldEnds::Both, ShieldEnds::First, ShieldEnds::Second, ShieldEnds::None];

    pub fn is_both(&self) -> bool {
        *self == ShieldEnds::Both
    }

    pub fn title(self) -> &'static str {
        match self {
            ShieldEnds::Both => tr("Both ends"),
            ShieldEnds::First => tr("First end only"),
            ShieldEnds::Second => tr("Second end only"),
            ShieldEnds::None => tr("Not connected"),
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            ShieldEnds::Both => "both",
            ShieldEnds::First => "first",
            ShieldEnds::Second => "second",
            ShieldEnds::None => "none",
        }
    }

    pub fn from_key(key: &str) -> Option<ShieldEnds> {
        ShieldEnds::ALL.into_iter().find(|k| k.key() == key.trim().to_ascii_lowercase())
    }

    /// The shield is connected at end 0 or 1.
    pub fn connects(self, end: usize) -> bool {
        matches!((self, end), (ShieldEnds::Both, _) | (ShieldEnds::First, 0) | (ShieldEnds::Second, 1))
    }
}

/// Rules of the 3D routing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RoutingRules {
    /// Edge length of the routing grid, mm.
    pub grid: f32,
    /// Distance routes keep from devices, rails and busbars, mm.
    pub clearance: f32,
    /// A way through free air costs this many times the way in a duct or path.
    pub free_factor: f32,
    /// Distance between classes that keep apart (power – signal, power – fluid), mm.
    pub segregation: f32,
    /// A way closer than the segregation distance costs this many times more.
    pub segregation_factor: f32,
    /// Wires are routed in 3D as well (otherwise through the cable ducts on the plate).
    pub wires_3d: bool,
    /// Smallest bend radius as a multiple of the outer diameter: wires, cables, tubes.
    pub bend_wire: f32,
    pub bend_cable: f32,
    pub bend_tube: f32,
    /// Cut lengths are rounded up to these steps (mm): wires, cables, tubes.
    pub cut_wire: f32,
    pub cut_cable: f32,
    pub cut_tube: f32,
    /// Length added to each cable end in the enclosure (stripping, connection), mm.
    pub cable_reserve: f32,
    /// Length of a shield pigtail where there is no shield clamp, mm.
    pub pigtail: f32,
}

impl Default for RoutingRules {
    fn default() -> Self {
        RoutingRules {
            grid: 25.0,
            clearance: 5.0,
            free_factor: 2.0,
            segregation: 100.0,
            segregation_factor: 4.0,
            wires_3d: false,
            bend_wire: 4.0,
            bend_cable: 7.5,
            bend_tube: 5.0,
            cut_wire: 10.0,
            cut_cable: 100.0,
            cut_tube: 50.0,
            cable_reserve: 300.0,
            pigtail: 50.0,
        }
    }
}

impl RoutingRules {
    pub fn is_default(&self) -> bool {
        *self == RoutingRules::default()
    }

    fn bend(&self, kind: RouteKind) -> f32 {
        match kind {
            RouteKind::Wire => self.bend_wire,
            RouteKind::Cable => self.bend_cable,
            RouteKind::Tube => self.bend_tube,
        }
    }

    fn cut(&self, kind: RouteKind) -> f32 {
        match kind {
            RouteKind::Wire => self.cut_wire,
            RouteKind::Cable => self.cut_cable,
            RouteKind::Tube => self.cut_tube,
        }
    }
}

/// A route frozen for production: the recalculation keeps it while it is still
/// possible and its ends have not moved.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FrozenRoute {
    pub key: String,
    pub kind: RouteKind,
    /// Id of the enclosure the points are relative to.
    pub enclosure: u64,
    /// Corner points relative to that enclosure.
    pub points: Vec<V3>,
    /// Length when it was frozen, mm.
    pub length: f32,
}

// ---------------------------------------------------------------------- results

/// An end of a route.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct End {
    /// Device tag, or the cable gland / "field".
    pub tag: String,
    /// Enclosure (index) the end is in; `None` in the field.
    pub enclosure: Option<usize>,
    /// The end point in layout coordinates.
    pub point: Option<V3>,
    /// Item the route ends at (device, cable gland).
    pub item: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Routed,
    /// Kept from the frozen route.
    Frozen,
    /// The frozen route was no longer possible and was routed again.
    Rerouted,
    /// Not routed, with the reason.
    Unrouted(String),
}

/// A routed wire, cable or tube.
#[derive(Clone, Debug, PartialEq)]
pub struct RoutedItem {
    pub kind: RouteKind,
    /// Stable key across recalculations: wires `from|to`, cables their tag, tubes
    /// `hose:from|to`.
    pub key: String,
    pub label: String,
    pub class: Class,
    pub ends: [End; 2],
    /// Enclosures the route runs through, in order.
    pub enclosures: Vec<usize>,
    pub points: Vec<V3>,
    /// Length in the panel layout including the reserves, mm.
    pub length: f32,
    /// Cut length (rounded up), mm.
    pub cut: f32,
    /// Outer diameter, mm.
    pub diameter: f32,
    /// Length outside ducts and routing paths, mm.
    pub free: f32,
    pub bends: usize,
    pub shielded: bool,
    pub status: Status,
    /// Index into the routed wires (wires only).
    pub wire: Option<usize>,
}

impl RoutedItem {
    pub fn routed(&self) -> bool {
        !matches!(self.status, Status::Unrouted(_))
    }

    pub fn status_text(&self) -> String {
        match &self.status {
            Status::Routed if self.kind == RouteKind::Wire && self.free == 0.0 => tr("in the cable ducts").to_string(),
            Status::Routed => tr("routed").to_string(),
            Status::Frozen => tr("frozen").to_string(),
            Status::Rerouted => tr("rerouted (frozen route no longer possible)").to_string(),
            Status::Unrouted(reason) => tr_fmt!("not routed: {}", reason),
        }
    }
}

/// Use of a routing path.
#[derive(Clone, Debug, PartialEq)]
pub struct PathUse {
    pub enclosure: usize,
    pub path: u64,
    pub items: usize,
    /// Cross-section of the contents, mm².
    pub area: f32,
    /// Usable cross-section, mm².
    pub capacity: f32,
}

/// Length of a frozen route against its current length.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    pub key: String,
    pub label: String,
    pub before: f32,
    /// `None`: the item no longer exists.
    pub after: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
    Info,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    pub level: Level,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Routing {
    pub items: Vec<RoutedItem>,
    pub bundles: Vec<Bundle>,
    pub branches: Vec<Branch>,
    pub shields: Vec<ShieldInfo>,
    pub paths: Vec<PathUse>,
    pub changes: Vec<Change>,
    pub problems: Vec<Problem>,
}

impl Routing {
    pub fn item(&self, key: &str) -> Option<&RoutedItem> {
        self.items.iter().find(|i| i.key == key)
    }

    /// Bundles an item runs in.
    pub fn bundles_of(&self, item: usize) -> Vec<usize> {
        (0..self.bundles.len()).filter(|&b| self.bundles[b].items.contains(&item)).collect()
    }
}

// ---------------------------------------------------------------------- analysis

/// Key of a routed wire.
pub fn wire_key(w: &RoutedWire) -> String {
    format!("{}|{}", w.from, w.to)
}

/// Class of a connection from its potential (kind and voltage).
pub fn class_of_net(prj: &Project, net: &str) -> Class {
    use crate::potentials::PotentialKind;
    let Some(p) = prj.potential(net) else { return Class::Control };
    let volts: f32 = p.voltage.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect::<String>().parse().unwrap_or(0.0);
    let ac = p.voltage.to_ascii_uppercase().contains("AC");
    match p.kind {
        PotentialKind::L | PotentialKind::N | PotentialKind::Pe => Class::Power,
        PotentialKind::Signal => Class::Signal,
        _ if (ac && volts > 50.0) || volts > 120.0 => Class::Power,
        _ => Class::Control,
    }
}

/// Class of a cable: signal for shielded and bus cables, otherwise the class of its
/// cores (the highest).
pub fn class_of_cable(prj: &Project, c: &Cable, conns: &[&Connection]) -> Class {
    if is_shielded(c) {
        return Class::Signal;
    }
    conns.iter().map(|x| class_of_net(prj, &x.net)).min().unwrap_or(Class::Control)
}

/// Outer diameter of a cable: as set, otherwise from its cores and cross-section.
pub fn cable_diameter(c: &Cable) -> f32 {
    if c.diameter > 0.0 {
        return c.diameter;
    }
    let n = c.core_list().len().max(2) as f32;
    let core = crate::harness::outer_diameter(&c.cross_section);
    // Cores laid up round, plus the sheath (and the shield).
    core * (1.0 + 0.5 * n).sqrt() * 1.2 + 2.0 + if is_shielded(c) { 1.0 } else { 0.0 }
}

/// Outer diameter of a tube from its nominal size ("8 mm", "DN25").
pub fn tube_diameter(size: &str) -> f32 {
    let digits: String = size.chars().skip_while(|c| !c.is_ascii_digit()).take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',').collect();
    digits.replace(',', ".").parse::<f32>().ok().filter(|v| *v > 0.0).unwrap_or(8.0)
}

fn part_subgroup<'a>(prj: &'a Project, number: &str) -> Option<&'a str> {
    prj.parts.iter().chain(crate::parts::catalog()).find(|p| p.number == number).map(|p| p.subgroup.as_str())
}

/// Items of a subgroup (accessories) in an enclosure.
pub fn accessories<'a>(prj: &'a Project, ei: usize, subgroup: &str) -> Vec<&'a PanelItem> {
    prj.layout.enclosures[ei].items.iter().filter(|i| i.kind == ItemKind::Accessory && part_subgroup(prj, &i.part) == Some(subgroup)).collect()
}

/// Where a device is reached: the middle of its front (layout coordinates); devices of
/// a placed valve terminal at the terminal ([`panel::locate`]).
fn device_end(prj: &Project, pos: &[V3], tag: &str) -> End {
    match panel::locate(prj, tag) {
        Some((ei, it)) => {
            let e = &prj.layout.enclosures[ei];
            let q = panel::face_point(e, it.face, [it.x + it.w / 2.0, it.y + it.h / 2.0, it.z + it.d]);
            End { tag: tag.into(), enclosure: Some(ei), point: Some(add(q, pos[ei])), item: Some(it.id) }
        }
        None => End { tag: tag.into(), ..Default::default() },
    }
}

/// Where a cable or tube from the field enters the enclosure: the nearest cable
/// gland, otherwise the middle of the bottom.
fn entry_end(prj: &Project, pos: &[V3], ei: usize, near: V3) -> End {
    let e = &prj.layout.enclosures[ei];
    let glands = accessories(prj, ei, "Cable gland");
    let at = |it: &PanelItem| add(panel::face_point(e, it.face, [it.x + it.w / 2.0, it.y + it.h / 2.0, 5.0]), pos[ei]);
    match glands.into_iter().min_by(|a, b| manhattan(at(a), near).total_cmp(&manhattan(at(b), near)).then(a.id.cmp(&b.id))) {
        Some(g) => End { tag: g.part.clone(), enclosure: Some(ei), point: Some(at(g)), item: Some(g.id) },
        None => {
            let q = [e.width / 2.0, e.height - e.wall() - 5.0, (PLATE_Z + e.depth) / 2.0];
            End { tag: tr("Bottom").to_string(), enclosure: Some(ei), point: Some(add(q, pos[ei])), item: None }
        }
    }
}

/// What is to be routed.
struct Job {
    kind: RouteKind,
    key: String,
    label: String,
    class: Class,
    ends: [End; 2],
    diameter: f32,
    shielded: bool,
    wire: Option<usize>,
    /// Route already known (wires through the cable ducts).
    fixed: Option<Vec<V3>>,
    /// Length to add (wires: from the connection points to the plate).
    extra: f32,
    /// Smallest bend radius in mm; 0: from the rules.
    bend: f32,
    /// Length set in the schematic (hoses), mm; 0: not set.
    set_length: f32,
}

/// The two end tags of a cable: the device most of its cores start at, and the one
/// most of them end at.
fn cable_ends(prj: &Project, conns: &[&Connection]) -> Option<(String, String)> {
    let tag = |p: crate::netlist::PinRef| prj.pages[p.page].symbols[p.sym].tag.clone();
    let most = |tags: Vec<String>| {
        let mut count: BTreeMap<String, usize> = BTreeMap::new();
        for t in tags {
            *count.entry(t).or_default() += 1;
        }
        count.into_iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0))).map(|x| x.0)
    };
    let a = most(conns.iter().map(|c| tag(c.a)).collect())?;
    let b = most(conns.iter().map(|c| tag(c.b)).filter(|t| *t != a).collect())?;
    Some((a, b))
}

/// The two end tags of a cable: from its cores, bus cables between the stations of
/// their bus connection.
pub fn cable_end_tags(prj: &Project, c: &Cable, cores: &[&Connection]) -> Option<(String, String)> {
    let bus = || {
        let link = crate::plc_hw::cable_links(prj, &c.tag).into_iter().next()?;
        let tag = |s: &str| prj.plc.station(s).map(|s| s.tag.clone()).filter(|t| !t.is_empty());
        Some((tag(&link.a.station)?, tag(&link.b.station)?))
    };
    cable_ends(prj, cores).or_else(bus)
}

/// The ends of a cable or tube between two devices: placed devices, or the cable
/// entry of the enclosure the other end is in. `None` if neither end is placed.
fn ends_between(prj: &Project, pos: &[V3], a: &str, b: &str) -> Option<[End; 2]> {
    let (ea, eb) = (device_end(prj, pos, a), device_end(prj, pos, b));
    match (ea.point, eb.point) {
        (None, None) => None,
        // Both in one device group: part of its base.
        _ if ea.item.is_some() && ea.item == eb.item => None,
        (Some(p), None) => Some([ea.clone(), End { tag: b.into(), ..entry_end(prj, pos, ea.enclosure?, p) }]),
        (None, Some(p)) => Some([End { tag: a.into(), ..entry_end(prj, pos, eb.enclosure?, p) }, eb]),
        _ => Some([ea, eb]),
    }
}

/// The 3D route of a wire routed through the cable ducts: its stretches on the
/// plates at duct depth, joined through the cable passages.
fn wire_points(pos: &[V3], w: &RoutedWire) -> Vec<V3> {
    let mut v = Vec::new();
    for (ei, route, entry, exit) in w.stretch_passages() {
        let z = if route.via_ducts { PLATE_Z + DUCT_PROFILE.1 / 2.0 } else { PLATE_Z + 5.0 };
        v.extend(entry);
        v.extend(route.points.iter().map(|q| add([PLATE_MARGIN + q.x, PLATE_MARGIN + q.y, z], pos[ei])));
        v.extend(exit);
    }
    grid::simplify(v)
}

/// Routes everything of the panel layout in 3D: cables (as a whole, field cables to
/// their cable gland), tubes, and the wires (`wiring`; with
/// [`RoutingRules::wires_3d`] their routes are replaced by the 3D routes). Classes
/// are routed one after the other (power first), so that later ones keep their
/// distance; frozen routes are kept while they are still possible.
pub fn analyse(prj: &Project, wiring: &mut [RoutedWire], conns: &[Connection], open: &Netlist) -> Routing {
    let rules = &prj.layout.routing;
    let pos = panel::positions(&prj.layout);
    let lineups = panel::lineups(&prj.layout);
    let lineup_of = |ei: usize| lineups.iter().position(|lu| lu.contains(&ei));
    let mut jobs: Vec<Job> = Vec::new();
    // Cables
    for c in &prj.cables {
        let cores: Vec<&Connection> = conns.iter().filter(|x| x.cable == c.tag).collect();
        let Some((a, b)) = cable_end_tags(prj, c, &cores) else { continue };
        let Some(ends) = ends_between(prj, &pos, &a, &b) else { continue };
        let shielded = is_shielded(c);
        jobs.push(Job {
            kind: RouteKind::Cable,
            key: c.tag.clone(),
            label: c.tag.clone(),
            class: class_of_cable(prj, c, &cores),
            ends,
            diameter: cable_diameter(c),
            shielded,
            wire: None,
            fixed: None,
            extra: 2.0 * rules.cable_reserve,
            bend: 0.0,
            set_length: 0.0,
        });
    }
    // Hoses, tubes and pipes with their configuration: bend radius, and the insertion
    // depth at both ends for the cut length.
    for h in crate::fluid::hoses(prj, open) {
        let tag = |p: crate::netlist::PinRef| prj.pages[p.page].symbols[p.sym].tag.clone();
        let Some(ends) = ends_between(prj, &pos, &tag(h.a), &tag(h.b)) else { continue };
        let label = if h.number.is_empty() { format!("{} – {}", h.from, h.to) } else { h.number.clone() };
        jobs.push(Job {
            kind: RouteKind::Tube,
            key: format!("hose:{}|{}", h.from, h.to),
            label,
            class: Class::Fluid,
            ends,
            diameter: tube_diameter(&h.size),
            shielded: false,
            wire: None,
            fixed: None,
            extra: 2.0 * prj.fluid.allowance,
            bend: h.config.bend_radius,
            set_length: h.length * 1000.0,
        });
    }
    // Wires: cable cores run in their cable.
    let net_of: HashMap<(String, String), &Connection> = conns
        .iter()
        .map(|c| ((crate::netlist::pin_designation(prj, c.a), crate::netlist::pin_designation(prj, c.b)), c))
        .collect();
    for (i, w) in wiring.iter().enumerate() {
        let conn = net_of.get(&(w.from.clone(), w.to.clone())).or_else(|| net_of.get(&(w.to.clone(), w.from.clone())));
        if conn.is_some_and(|c| !c.cable.is_empty()) {
            continue;
        }
        let class = class_of_net(prj, &w.net);
        let end = |tag: &str, ei: usize, item: u64, q: Option<&crate::geom::P>| End {
            tag: tag.into(),
            enclosure: Some(ei),
            point: q.map(|q| add([PLATE_MARGIN + q.x, PLATE_MARGIN + q.y, PLATE_Z + 5.0], pos[ei])),
            item: Some(item),
        };
        let last = w.stretches().last().map(|s| s.1).unwrap_or(&w.route);
        let ends = [end(&w.from, w.enclosure, w.items.0, w.route.points.first()), end(&w.to, w.last_enclosure(), w.items.1, last.points.last())];
        let free = rules.wires_3d && w.door == (None, None) && !w.route.points.is_empty();
        let fixed = (!free).then(|| wire_points(&pos, w));
        jobs.push(Job {
            kind: RouteKind::Wire,
            key: wire_key(w),
            label: format!("{} – {}", w.from, w.to),
            class,
            ends,
            diameter: crate::harness::outer_diameter(&w.cross_section),
            shielded: false,
            wire: Some(i),
            fixed,
            extra: w.length - w.plate_length(),
            bend: 0.0,
            set_length: 0.0,
        });
    }
    jobs.sort_by(|a, b| a.class.cmp(&b.class).then(a.kind.cmp(&b.kind)).then(a.key.cmp(&b.key)));

    let mut spaces: HashMap<usize, grid::Space> = HashMap::new();
    let mut zones_of: Option<Class> = None;
    let frozen: HashMap<&str, &FrozenRoute> = prj.layout.frozen.iter().map(|f| (f.key.as_str(), f)).collect();
    let mut items: Vec<RoutedItem> = Vec::new();
    let mut problems: Vec<Problem> = Vec::new();
    for job in jobs {
        let start = job.ends.iter().find_map(|e| e.enclosure);
        let lu = start.and_then(lineup_of);
        let mut item = RoutedItem {
            kind: job.kind,
            key: job.key.clone(),
            label: job.label.clone(),
            class: job.class,
            ends: job.ends.clone(),
            enclosures: Vec::new(),
            points: Vec::new(),
            length: 0.0,
            cut: 0.0,
            diameter: job.diameter,
            free: 0.0,
            bends: 0,
            shielded: job.shielded,
            status: Status::Routed,
            wire: job.wire,
        };
        // Wires through the cable ducts are routed already (also those on the door).
        let known = job.fixed.is_some();
        let (Some(lu), Some(a), Some(b)) = (lu, job.ends[0].point.or(known.then_some([0.0; 3])), job.ends[1].point.or(known.then_some([0.0; 3]))) else {
            item.status = Status::Unrouted(tr("not in the panel layout").to_string());
            items.push(item);
            continue;
        };
        if job.ends.iter().filter_map(|e| e.enclosure).any(|ei| lineup_of(ei) != Some(lu)) {
            item.status = Status::Unrouted(tr("enclosures not bayed").to_string());
            items.push(item);
            continue;
        }
        let space = spaces.entry(lu).or_insert_with(|| grid::Space::build(prj, &lineups[lu], &pos, rules));
        // Classes that come later keep apart from those routed before them.
        if zones_of != Some(job.class) {
            space.update_zones(rules.segregation);
            zones_of = Some(job.class);
        }
        let ignore: Vec<u64> = job.ends.iter().filter_map(|e| e.item).collect();
        let area = std::f32::consts::PI / 4.0 * job.diameter * job.diameter;
        let radius = if job.bend > 0.0 { job.bend } else { rules.bend(job.kind) * job.diameter };
        let req = grid::Request { kind: job.kind, class: job.class, ignore: &ignore, area, turn: (space.cell * 0.5).max(radius) };
        let thawed = frozen.get(job.key.as_str()).and_then(|f| {
            let o = prj.layout.enclosures.iter().position(|e| e.id == f.enclosure).map(|k| pos[k])?;
            Some(f.points.iter().map(|q| add(*q, o)).collect::<Vec<V3>>())
        });
        let near = |q: Option<&V3>, e: V3| q.is_some_and(|q| manhattan(*q, e) <= 2.0 * space.cell);
        let way = match (&job.fixed, thawed) {
            (Some(points), _) => Some((points.clone(), space.cells_along(points), 0.0)),
            (None, Some(points)) if near(points.first(), a) && near(points.last(), b) && space.still_free(&points, &req) => {
                item.status = Status::Frozen;
                let cells = space.cells_along(&points);
                Some((points, cells, 0.0))
            }
            (None, thawed) => {
                if thawed.is_some() {
                    item.status = Status::Rerouted;
                    problems.push(Problem { level: Level::Warning, message: tr_fmt!("Frozen route of {} is no longer possible and was routed again", job.label) });
                }
                space.route(a, b, &req, rules).map(|w| (w.points, w.cells, w.free))
            }
        };
        let Some((points, cells, free)) = way else {
            item.status = Status::Unrouted(tr("no way through the enclosure").to_string());
            problems.push(Problem { level: Level::Warning, message: tr_fmt!("{} {} cannot be routed: no way around the obstacles", job.kind.title(), job.label) });
            items.push(item);
            continue;
        };
        space.occupy(&cells, job.class, area);
        let (bends, shortest) = grid::bends(&points);
        // Corners are rounded with the bend radius where the stretches allow it.
        let rounding = if bends > 0 { bends as f32 * radius.min(shortest / 2.0).max(0.0) * (2.0 - std::f32::consts::FRAC_PI_2) } else { 0.0 };
        item.length = match (job.fixed.is_some(), job.wire) {
            (true, Some(wi)) => wiring[wi].length,
            _ => grid::length(&points) - rounding + job.extra,
        };
        // Stretches between bends need room for the bend radius (half of it on each);
        // the legs to the exact end points are left out.
        let shortest = if points.len() > 4 { grid::bends(&points[1..points.len() - 1]).1 } else { f32::INFINITY };
        if bends > 0 && shortest < radius * 0.5 && job.kind != RouteKind::Wire {
            problems.push(Problem { level: Level::Warning, message: tr_fmt!("{} {}: bends closer than the bend radius ({} mm)", job.kind.title(), job.label, format!("{radius:.0}")) });
        }
        // A hose with its length set in the schematic must reach along its route.
        if job.set_length > 0.0 && job.set_length + 0.5 < item.length {
            let (set, routed) = (format!("{:.2}", job.set_length / 1000.0), format!("{:.2}", item.length / 1000.0));
            problems.push(Problem { level: Level::Warning, message: tr_fmt!("Hose {}: length {} m shorter than its 3D route of {} m", job.label, set, routed) });
        }
        let step = rules.cut(job.kind).max(1.0);
        item.cut = (item.length / step).ceil() * step;
        item.free = if job.fixed.is_some() { 0.0 } else { free };
        item.bends = bends;
        let mut encs: Vec<usize> = Vec::new();
        for q in &points {
            if let Some(k) = lineups[lu].iter().copied().find(|&k| q[0] >= pos[k][0] - 0.5 && q[0] <= pos[k][0] + prj.layout.enclosures[k].width + 0.5) {
                if encs.last() != Some(&k) {
                    encs.push(k);
                }
            }
        }
        item.enclosures = encs;
        item.points = points;
        if let (Some(wi), true) = (job.wire, job.fixed.is_none()) {
            apply_to_wire(prj, &pos, &mut wiring[wi], &item.points, job.extra);
            item.length = wiring[wi].length;
            item.cut = (item.length / step).ceil() * step;
        }
        items.push(item);
    }
    let (bundles, branches) = bundles::find(&items, rules.grid.max(5.0));
    // Classes that keep apart in one bundle.
    for b in &bundles {
        let classes: Vec<Class> = b.items.iter().map(|&i| items[i].class).collect();
        if classes.iter().any(|a| classes.iter().any(|c| a.keeps_apart(*c))) {
            // Shielded cables are reported with their shield.
            let labels: Vec<&str> = b.items.iter().filter(|&&i| items[i].kind != RouteKind::Wire && !items[i].shielded).map(|&i| items[i].label.as_str()).collect();
            if !labels.is_empty() {
                problems.push(Problem { level: Level::Warning, message: tr_fmt!("Segregation: {} run together with power conductors over {} mm", labels.join(", "), format!("{:.0}", b.length)) });
            }
        }
    }
    let mut paths = Vec::new();
    for space in spaces.values() {
        for p in &space.paths {
            paths.push(PathUse { enclosure: p.enclosure, path: p.id, items: p.items, area: p.used, capacity: p.capacity });
            if p.capacity > 0.0 && p.used > p.capacity {
                let e = &prj.layout.enclosures[p.enclosure];
                let name = e.paths.iter().find(|x| x.id == p.id).map(|x| x.name.clone()).unwrap_or_default();
                problems.push(Problem { level: Level::Warning, message: tr_fmt!("{}: routing path {} is full ({} % of its usable cross-section)", e.name, name, format!("{:.0}", p.used / p.capacity * 100.0)) });
            }
        }
    }
    paths.sort_by(|a, b| a.enclosure.cmp(&b.enclosure).then(a.path.cmp(&b.path)));
    let (shields, shield_problems) = shield::analyse(prj, &items, &bundles);
    problems.extend(shield_problems);
    for it in items.iter().filter(|i| i.kind != RouteKind::Wire && i.ends.iter().any(|e| e.enclosure.is_some() && e.item.is_none())) {
        problems.push(Problem { level: Level::Info, message: tr_fmt!("{} {} enters through the bottom: no cable gland in the enclosure", it.kind.title(), it.label) });
    }
    let mut changes = Vec::new();
    for f in &prj.layout.frozen {
        let after = items.iter().find(|i| i.key == f.key && i.routed()).map(|i| i.length);
        if after.is_none_or(|a| (a - f.length).abs() > 1.0) {
            let label = items.iter().find(|i| i.key == f.key).map(|i| i.label.clone()).unwrap_or_else(|| f.key.clone());
            changes.push(Change { key: f.key.clone(), label, before: f.length, after });
        }
    }
    Routing { items, bundles, branches, shields, paths, changes, problems }
}

/// Replaces the route of a wire by its 3D route: the stretches per enclosure as
/// seen on the plates, the cable ducts it runs through, the cable passages, and its
/// length (`extra`: from the connection points to the plate and the reserve).
fn apply_to_wire(prj: &Project, pos: &[V3], w: &mut RoutedWire, points: &[V3], extra: f32) {
    let encs = &prj.layout.enclosures;
    let enclosure_at = |q: V3| (0..encs.len()).find(|&k| q[0] >= pos[k][0] - 0.5 && q[0] <= pos[k][0] + encs[k].width + 0.5);
    // Split at the boundaries between enclosures.
    let mut stretches: Vec<(usize, Vec<V3>, Option<V3>)> = Vec::new();
    for s in points.windows(2) {
        let mid = [(s[0][0] + s[1][0]) / 2.0, (s[0][1] + s[1][1]) / 2.0, (s[0][2] + s[1][2]) / 2.0];
        let Some(k) = enclosure_at(mid) else { continue };
        match stretches.last_mut() {
            Some(last) if last.0 == k => last.1.push(s[1]),
            _ => {
                let entry = (!stretches.is_empty()).then_some(s[0]);
                stretches.push((k, vec![s[0], s[1]], entry));
            }
        }
    }
    let to_route = |k: usize, pts: &[V3]| {
        let e = &encs[k];
        let (pw, ph) = e.plate_size();
        let mut plate: Vec<crate::geom::P> = Vec::new();
        for q in pts {
            let l = sub(*q, pos[k]);
            let pq = crate::geom::p((l[0] - PLATE_MARGIN).clamp(0.0, pw), (l[1] - PLATE_MARGIN).clamp(0.0, ph));
            if plate.last().is_none_or(|x| !x.same(pq)) {
                plate.push(pq);
            }
        }
        let mut runs: Vec<panel::DuctRun> = Vec::new();
        for s in pts.windows(2) {
            let (a, b) = (sub(s[0], pos[k]), sub(s[1], pos[k]));
            for d in e.items.iter().filter(|i| i.kind == ItemKind::Duct && i.face == Face::Plate) {
                let (lo, hi) = panel::item_box(e, d);
                let inside = |q: V3| (0..3).all(|i| q[i] >= lo[i] - 1.0 && q[i] <= hi[i] + 1.0);
                if !(inside(a) && inside(b)) {
                    continue;
                }
                let axis = |q: V3| if d.horizontal() { q[0] - PLATE_MARGIN } else { q[1] - PLATE_MARGIN };
                let (s0, s1) = (axis(a), axis(b));
                if (s1 - s0).abs() > 0.01 {
                    runs.push(panel::DuctRun { duct: d.id, from: s0.min(s1), to: s0.max(s1) });
                }
            }
        }
        panel::Route { points: plate, plate_length: grid::length(pts), via_ducts: !runs.is_empty(), runs }
    };
    let mut it = stretches.into_iter();
    let Some((k0, p0, _)) = it.next() else { return };
    w.enclosure = k0;
    w.route = to_route(k0, &p0);
    w.legs = it.map(|(k, pts, entry)| panel::Leg { enclosure: k, route: to_route(k, &pts), passage: entry.unwrap_or(pts[0]), passage_length: 0.0 }).collect();
    w.length = w.plate_length() + extra;
}

/// Freezes the routes of the given keys (all routed items without keys) for
/// production. Returns how many.
pub fn freeze(prj: &mut Project, routing: &Routing, keys: &[String]) -> usize {
    let pos = panel::positions(&prj.layout);
    let mut n = 0;
    for it in routing.items.iter().filter(|i| i.routed() && (keys.is_empty() || keys.contains(&i.key))) {
        let Some(&k) = it.enclosures.first() else { continue };
        let e = &prj.layout.enclosures[k];
        let f = FrozenRoute { key: it.key.clone(), kind: it.kind, enclosure: e.id, points: it.points.iter().map(|q| sub(*q, pos[k])).collect(), length: it.length };
        prj.layout.frozen.retain(|x| x.key != it.key);
        prj.layout.frozen.push(f);
        n += 1;
    }
    prj.layout.frozen.sort_by(|a, b| a.key.cmp(&b.key));
    n
}

/// Releases frozen routes (all without keys). Returns how many.
pub fn release(prj: &mut Project, keys: &[String]) -> usize {
    let before = prj.layout.frozen.len();
    prj.layout.frozen.retain(|f| !keys.is_empty() && !keys.contains(&f.key));
    before - prj.layout.frozen.len()
}

#[cfg(test)]
mod tests;
