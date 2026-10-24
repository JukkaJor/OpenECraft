//! Data of `crate::field` that the project file holds (see `docs/architecture.md`).


use serde::{Deserialize, Serialize};

/// Field wiring data of the project.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Field {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<Node>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<Segment>,
    /// Enclosures, field devices and machine parts at the nodes.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub places: Vec<Place>,
    /// Outlines of machines, buildings and areas.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub outlines: Vec<Outline>,
    /// Route definitions of single cables.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub routes: Vec<RouteDef>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bonds: Vec<Bond>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub accessories: Vec<Accessory>,
    #[serde(skip_serializing_if = "FieldRules::is_default")]
    pub rules: FieldRules,
}

/// An enclosure (location `+CAB1`), a device (`-M1`) or a machine part (`+M1`) at a
/// node of the network.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Place {
    pub tag: String,
    pub node: u64,
    /// Length from the node to the device or into the enclosure, metres.
    pub drop: f32,
}

/// A stretch of the route network between two nodes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Segment {
    pub id: u64,
    pub name: String,
    pub a: u64,
    pub b: u64,
    pub kind: SegmentKind,
    /// Inner width and height in mm; 0 = no limit.
    pub width: f32,
    pub height: f32,
    /// Allowed classes ([`Class::bit`]); 0 = all.
    pub classes: u8,
    /// Has a divider between the classes that must be kept apart.
    pub divider: bool,
    /// Share of the cross-section kept free, percent.
    pub reserve: f32,
    /// Part number of the carrier (bill of materials per metre).
    pub part: String,
    /// Length in metres; 0 = from the node positions.
    pub length: f32,
    /// Bend radius of the carrier in mm (drag chains); 0 = none.
    pub bend_radius: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SegmentKind {
    #[default]
    Tray,
    Ladder,
    Trunking,
    Conduit,
    /// Drag chain (energy chain) of a moving machine axis.
    DragChain,
    /// Laid in the ground.
    Buried,
    /// Free run without carrier (clipped to the machine frame).
    Free,
}

/// Outline of a machine, building or area of the plant layout.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Outline {
    pub id: u64,
    pub name: String,
    /// Corner points in metres.
    pub points: Vec<[f32; 2]>,
    /// Conductive frame that must be connected to the equipotential bonding.
    pub conductive: bool,
}

/// A bonding conductor between two bonding points (names of [`points`]).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Bond {
    pub id: u64,
    pub a: String,
    pub b: String,
    pub kind: BondKind,
    /// Cross-section in mm², e.g. "6" or "16".
    pub cross_section: String,
    /// Length in metres.
    pub length: f32,
    /// Part number of the conductor (bill of materials per metre).
    pub part: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BondKind {
    /// Protective bonding to the main earthing terminal (IEC 60364-5-54).
    #[default]
    Main,
    /// Supplementary protective bonding between conductive parts.
    Supplementary,
    /// Functional bonding (EMC, no protective function).
    Functional,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FieldRules {
    /// Length added at each cable end, metres.
    pub reserve: f32,
    /// Length inside an enclosure from the cable entry to the terminals, metres.
    pub cabinet: f32,
    /// Cut lengths rounded up to this step, metres.
    pub cut: f32,
    /// Maximum fill of a carrier, percent of its cross-section.
    pub fill: f32,
    /// Bend radius of cables in drag chains as a multiple of the diameter.
    pub bend_factor: f32,
    /// Name of the main earthing terminal.
    pub main_earth: String,
    /// Metal carriers that meet at a node are connected to each other.
    pub continuity: bool,
}

/// An accessory of a cable at one end (device tag of the end) or at both ends.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Accessory {
    pub id: u64,
    pub cable: String,
    /// Device tag of the end; empty = at both ends.
    pub at: String,
    pub kind: AccessoryKind,
    pub part: String,
    /// Quantity per end.
    pub count: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccessoryKind {
    #[default]
    Gland,
    /// Cable gland with shield contact.
    EmcGland,
    Connector,
    StrainRelief,
    ShieldClamp,
    ProtectiveHose,
    EndSleeve,
    Marker,
    Other,
}

/// Route definition of a cable: nodes it must pass in this order and segments it must
/// not use.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RouteDef {
    pub cable: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub via: Vec<u64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub avoid: Vec<u64>,
}

/// A point of the route network: junction, branch, wall or floor penetration, the
/// point where an enclosure or a device is reached. Coordinates in metres, z is the
/// height above the floor.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Node {
    pub id: u64,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
