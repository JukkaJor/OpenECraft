//! Data of `crate::marking` that the project file holds (see `docs/architecture.md`).


use serde::{Deserialize, Serialize};

/// Marking settings of a project: wire marker text and the marker material per kind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub wire_text: WireText,
    pub device_material: String,
    /// Legend plates of door devices.
    pub door_material: String,
    pub terminal_material: String,
    pub wire_material: String,
    pub cable_material: String,
    /// Labeling scheme used for the export (name of a built-in or project scheme).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub scheme: String,
    /// Labeling schemes of the project; one with the name of a built-in scheme replaces it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub schemes: Vec<Scheme>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Device,
    Terminal,
    Wire,
    Cable,
}

/// A column of a labeling scheme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Column {
    Material,
    Category,
    Text,
    Count,
    Enclosure,
    Group,
    /// Running number of the row.
    Number,
}

/// Output layout for a marking program (EPLAN: labeling scheme): which markers, which
/// columns in which order, separator, header and whether every printed label gets its
/// own row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Scheme {
    pub name: String,
    pub columns: Vec<Column>,
    /// Field separator of the text export (`;`, `,` or a tab).
    pub separator: String,
    pub header: bool,
    /// UTF-8 byte order mark at the start of the text export.
    pub bom: bool,
    /// One row per printed label instead of a count column.
    pub one_row_per_label: bool,
    /// Markers exported; empty = all.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<Category>,
}

impl Default for Scheme {
    fn default() -> Self {
        Scheme {
            name: String::new(),
            columns: vec![Column::Material, Column::Category, Column::Text, Column::Count, Column::Enclosure, Column::Group],
            separator: ";".into(),
            header: true,
            bom: true,
            one_row_per_label: false,
            categories: Vec::new(),
        }
    }
}

/// What a wire marker shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireText {
    /// The wire number, the same on both ends.
    #[default]
    Number,
    /// At each end the connection point at the other end (destination marking).
    Destination,
}
