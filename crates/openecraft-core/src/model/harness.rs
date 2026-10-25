//! Data of `crate::harness` that the project file holds (see `docs/architecture.md`).


use openecraft_parts::vec3::V3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Harness {
    pub id: u64,
    /// Complete device tag, e.g. `=A1-WH1`.
    pub tag: String,
    pub description: String,
    /// Length in metres of wires that are not routed in the panel layout (field
    /// wiring); 0 = unknown.
    pub length: f32,
    /// Part number of the finished harness (bill of materials).
    pub part: String,
    /// Connections of the harness: both ends as connection point designations
    /// (`-S1:13`), in [`key`] order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub connections: Vec<[String; 2]>,
    /// Stored 3D topology: nodes and the segments between them. Empty: the topology
    /// is derived from the panel layout or generated ([`crate::harness::topology`]).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<Node>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<Segment>,
    /// Wire protections (tape, tubes …) on segments.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub protections: Vec<Protection>,
    /// Fixings (clips, cable ties, grommets …) on segments.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fixings: Vec<Fixing>,
    /// End treatment of the connectors and pins: terminals, seals, cavity plugs.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ends: Vec<End>,
    /// Further parts of the harness (accessories, co-pack parts, assemblies …).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<ExtraPart>,
}

/// A node of the harness topology: connector position, branch point or a point
/// where the bundle changes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Node {
    pub id: String,
    /// Position, mm (layout coordinates: x right, y down, z from the rear wall).
    pub pos: V3,
    /// Device tags of the connectors at this node.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub connectors: Vec<String>,
    /// Bend radius of the bundle at the node, mm; 0 = not given.
    pub bend_radius: f32,
}

/// A segment of the harness topology: the bundle between two nodes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Segment {
    pub id: String,
    /// Node ids of both ends.
    pub start: String,
    pub end: String,
    /// Points of the centre curve between the two nodes, mm.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub points: Vec<V3>,
    /// Length on the formboard, mm; 0 = the length of the centre curve.
    pub length: f32,
}

/// Kind of a wire protection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtectionKind {
    #[default]
    Tape,
    Tube,
    CorrugatedTube,
    Sleeve,
    HeatShrink,
}

/// A wire protection on (a part of) a segment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Protection {
    pub id: String,
    pub kind: ProtectionKind,
    pub part: String,
    /// Segment id; empty: every segment of the harness.
    pub segment: String,
    /// Start and end along the segment, 0 … 1.
    pub start: f32,
    pub end: f32,
}

/// Kind of a fixing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixingKind {
    #[default]
    Clip,
    CableTie,
    Grommet,
    Bracket,
}

/// A fixing on a segment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Fixing {
    pub id: String,
    pub kind: FixingKind,
    pub part: String,
    pub segment: String,
    /// Location along the segment, 0 … 1.
    pub location: f32,
}

/// End treatment of a connector (`pin` empty) or of one of its pins.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct End {
    /// Device tag of the connector.
    pub device: String,
    /// Connection point; empty: all pins of the device.
    pub pin: String,
    /// Part numbers of the terminal (crimp contact, ferrule, ring terminal) and of the
    /// single wire seal.
    pub terminal: String,
    pub seal: String,
    /// Part number of the plugs closing the unused cavities (device only).
    pub cavity_plug: String,
    /// Stripping length, mm; 0 = from the cross-section.
    pub strip: f32,
    /// KBL connector usage: "", "no end", "ring terminal", "splice" or "dangler".
    pub usage: String,
}

/// Kind of a further harness part.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartKind {
    #[default]
    Accessory,
    CoPack,
    Assembly,
    Component,
}

/// A further part of the harness.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExtraPart {
    pub kind: PartKind,
    pub part: String,
    pub count: u32,
    pub description: String,
    /// Node the part sits at; empty: none.
    pub node: String,
}

impl Default for ExtraPart {
    fn default() -> Self {
        ExtraPart { kind: PartKind::default(), part: String::new(), count: 1, description: String::new(), node: String::new() }
    }
}
