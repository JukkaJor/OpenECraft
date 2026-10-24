//! Data of `crate::routing3d` that the project file holds (see `docs/architecture.md`).


use serde::{Deserialize, Serialize};
use openecraft_parts::vec3::V3;

/// Ends at which the shield of a cable is connected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShieldEnds {
    #[default]
    Both,
    First,
    Second,
    None,
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

/// What a reservation keeps the space for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReserveMode {
    /// Nothing is routed through it (room for later devices, the swing of the door).
    #[default]
    Keepout,
    /// Only this class is routed through it.
    For(Class),
}

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

/// What a routing path is for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PathKind {
    #[default]
    Any,
    Wires,
    Cables,
    Tubes,
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

/// What is routed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteKind {
    #[default]
    Wire,
    Cable,
    Tube,
}
