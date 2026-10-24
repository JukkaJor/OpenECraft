//! Data of `crate::electrofluid` that the project file holds (see `docs/architecture.md`).

use serde::{Deserialize, Serialize};

/// Fluid devices mounted together: the valves of a valve terminal in slot order, the
/// components of a service unit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DeviceGroup {
    /// Device tag of the group, e.g. `-KH1`.
    pub tag: String,
    pub kind: GroupKind,
    pub description: String,
    /// Device tags of the members in slot order.
    pub members: Vec<String>,
    /// Places of the group (valve terminal); 0 = not limited.
    pub slots: u32,
    /// Part number of the base (sub-base, manifold block).
    pub part: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum GroupKind {
    #[default]
    ValveTerminal,
    ServiceUnit,
    Manifold,
    PowerUnit,
    Other,
}
