//! Data of `crate::plc_hw` that the project file holds (see `docs/architecture.md`).

use super::devdesc::{BusKind, DeviceDescription};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Hardware {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stations: Vec<Station>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub networks: Vec<BusSystem>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub connections: Vec<BusConnection>,
    /// Imported device descriptions (GSDML, ESI, AutomationML).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub descriptions: Vec<DeviceDescription>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BusSystem {
    pub name: String,
    pub kind: BusKind,
    /// First address of the subnet, e.g. 192.168.0.0.
    pub subnet: String,
}

/// A PLC, remote I/O station or other device on a bus. Its modules are the PLC cards
/// whose `station` is the station's name.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Station {
    pub name: String,
    pub kind: StationKind,
    /// Device tag of the CPU / bus coupler / drive.
    pub tag: String,
    /// Device description of the head (CPU, coupler).
    pub device: String,
    pub scheme: Scheme,
    /// Bus system the station is connected to.
    pub network: String,
    /// IP address or bus address.
    pub address: String,
    pub device_number: u32,
    /// Remote I/O: name of the controlling station.
    pub controller: String,
    pub comment: String,
}

/// Address format of a PLC type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Scheme {
    /// `I0.0` / `Q0.0` (STEP 7, TIA Portal).
    #[default]
    Siemens,
    /// `%IX0.0` / `%QX0.0` (IEC 61131-3: CODESYS, TwinCAT).
    Iec,
    /// `Local:1:I.Data.0` (Studio 5000, per slot).
    Rockwell,
}

/// A bit address in any scheme. Rockwell addresses count from the module (`slot`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Address {
    pub input: bool,
    pub byte: u32,
    pub bit: u32,
    pub slot: Option<u32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StationKind {
    /// PLC with CPU: owns an address space.
    #[default]
    Controller,
    /// Remote I/O: its addresses are in the address space of its controller.
    IoDevice,
    Drive,
    Hmi,
    Other,
}

/// Network cable between two ports.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BusConnection {
    pub network: String,
    pub a: PortRef,
    pub b: PortRef,
    /// Cable tag (cables of the project).
    pub cable: String,
}

/// One end of a bus connection: a port of a station.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct PortRef {
    pub station: String,
    pub port: String,
}
