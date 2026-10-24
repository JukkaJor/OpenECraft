//! Data of `crate::properties` that the project file holds (see `docs/architecture.md`).


use super::functions::FunctionDef;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Property values of an object, keyed by property key (with index).
pub type Values = BTreeMap<String, String>;

/// User defined properties, function definitions and placement schemes of a
/// project; also the format of a property library file shared between projects.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Library {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<PropDef>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub functions: Vec<FunctionDef>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub schemes: Vec<super::props::Scheme>,
}

/// Definition of a property.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PropDef {
    pub key: String,
    /// English name of a built-in property, the name of a user defined one.
    pub name: String,
    pub class: ObjectClass,
    pub group: Group,
    #[serde(rename = "type")]
    pub ty: DataType,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub unit: String,
    /// Values of a selection list.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<String>,
    /// Number of indices (`key[1]` … `key[n]`); 0 = a single value.
    #[serde(skip_serializing_if = "crate::model::is_zero")]
    pub indices: u16,
    /// Value while none is set.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub default: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(skip)]
    pub builtin: bool,
    /// Computed or taken from the model, cannot be changed.
    #[serde(skip)]
    pub read_only: bool,
    /// Taken from a field of the model, which is changed instead of a stored value.
    #[serde(skip)]
    pub field: bool,
    /// Property of the same object read while no value is set.
    #[serde(skip)]
    pub fallback: String,
}

/// Group of properties in the property editor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Group {
    #[default]
    General,
    Customer,
    Structure,
    Commercial,
    Electrical,
    Control,
    Contacts,
    Protection,
    Motor,
    Drive,
    Sensor,
    Signal,
    PowerSupply,
    Plc,
    Mechanical,
    Environment,
    Safety,
    Communication,
    Fluid,
    Process,
    Maintenance,
    Documentation,
    Supplementary,
    /// User defined properties without a group of their own.
    User,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataType {
    #[default]
    Text,
    MultiLine,
    Integer,
    Real,
    Bool,
    /// One of the definition's choices.
    Choice,
    /// `YYYY-MM-DD`.
    Date,
    /// File name or URL.
    Link,
}

/// The kind of object a property belongs to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ObjectClass {
    Project,
    Page,
    #[default]
    Device,
    /// A single function (placed symbol) of a device.
    Function,
    /// Functions that are terminals.
    Terminal,
    /// Functions that are PLC connection points.
    PlcPoint,
    Cable,
    /// Part data (read only; shown for devices through their assigned parts).
    Part,
}
