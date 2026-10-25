//! Data of `crate::forms` that the project file holds (see `docs/architecture.md`).


use crate::geom::P;
use super::ReportId;
use crate::symbols::Anchor;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Forms {
    /// Title blocks defined for the project.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub title_blocks: Vec<Form>,
    /// Title block of pages that do not choose one; empty: the standard one.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub default_title_block: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub reports: BTreeMap<ReportId, ReportForm>,
    /// Named report forms.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub schemes: Vec<ReportScheme>,
    /// Plot frames defined for the project.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub frames: Vec<super::frame::PlotFrame>,
    /// Plot frame of pages that do not choose one; empty: A3 landscape.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub default_frame: String,
}

/// A named report form of one report, e.g. "Terminals for the workshop"; report
/// blocks choose it by name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReportScheme {
    pub name: String,
    pub kind: ReportId,
    pub form: ReportForm,
}

/// Layout of a report page.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReportForm {
    /// Heading of the page; empty: the name of the report.
    pub title: String,
    /// Columns shown, in this order (header names); empty: all columns.
    pub columns: Vec<String>,
    /// Rows per page; 0: the default.
    pub rows: usize,
    /// Only rows that pass all filters are shown.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<RowFilter>,
    /// Sort order (first key first); empty: the order of the report.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sort: Vec<SortKey>,
    /// Column that groups the rows: report blocks start new pages for every value;
    /// empty: the first column.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub group: String,
    /// Values of the first shown column that repeat the row above are left blank.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub blank_repeats: bool,
    /// Relative column widths by header name (instead of the report's).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub widths: BTreeMap<String, f32>,
    /// Text size of the rows (mm); 0: fitted to the rows per page.
    pub text_size: f32,
    /// Drawn as a graphic (terminal and cable diagrams) instead of a table.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub graphic: bool,
    /// Drawn by the user's own form (form editor) instead of a table.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<FormLayout>,
}

/// A report form drawn by the user: graphics drawn once per page and a row drawing
/// repeated for every row of the page. Texts take the column names of the report as
/// placeholders (`{Terminal}`, `{Target}`; the English name or the one shown) besides
/// those of title blocks (`{project}`, `{page}` …). Coordinates in mm from the top
/// left corner of the drawing area of the plot frame.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FormLayout {
    /// Drawn once per page.
    pub header: Vec<FormItem>,
    /// Drawn for every row, relative to the row's origin.
    pub row: Vec<FormItem>,
    /// Origin of the first row.
    pub first_row: P,
    /// From one row to the next: downwards for lists, sideways to draw the rows
    /// side by side (e.g. terminals of a strip).
    pub step: P,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SortKey {
    pub column: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub descending: bool,
}

/// Keeps the rows whose cell in `column` passes (case-insensitive).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RowFilter {
    pub column: String,
    pub op: FilterOp,
    pub value: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterOp {
    #[default]
    Contains,
    NotContains,
    Equals,
    NotEquals,
    StartsWith,
    Empty,
    NotEmpty,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Form {
    pub name: String,
    pub items: Vec<FormItem>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FormItem {
    Line {
        a: P,
        b: P,
        width: f32,
    },
    Rect {
        a: P,
        b: P,
        width: f32,
        #[serde(default)]
        fill: bool,
    },
    /// Text with placeholders. `label` texts are drawn in the frame colour;
    /// `max_width` > 0 shrinks long values to fit.
    Text {
        pos: P,
        text: String,
        size: f32,
        anchor: Anchor,
        #[serde(default)]
        label: bool,
        #[serde(default)]
        max_width: f32,
    },
}
