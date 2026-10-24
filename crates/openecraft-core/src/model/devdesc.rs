//! Data of `crate::devdesc` that the project file holds (see `docs/architecture.md`).


use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BusKind {
    #[default]
    Profinet,
    EtherCat,
    EthernetIp,
    ProfibusDp,
    ModbusTcp,
    CanOpen,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceDescription {
    /// Unique key: the part number, the GSDML module ID or the ESI type.
    pub id: String,
    pub name: String,
    pub manufacturer: String,
    pub order_number: String,
    pub kind: ModuleKind,
    /// Number of input / output channels.
    pub inputs: u32,
    pub outputs: u32,
    /// Size in the process image.
    pub input_bytes: u32,
    pub output_bytes: u32,
    /// Heads: number of slots for modules.
    pub slots: u32,
    pub ports: Vec<DescPort>,
    /// Where it comes from: empty for built-in, otherwise the file or "part".
    pub source: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DescPort {
    pub name: String,
    pub bus: BusKind,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModuleKind {
    Cpu,
    /// Head of a remote I/O station (bus coupler, interface module).
    Coupler,
    DigitalInput,
    DigitalOutput,
    DigitalMixed,
    AnalogInput,
    AnalogOutput,
    Power,
    Communication,
    #[default]
    Other,
}
