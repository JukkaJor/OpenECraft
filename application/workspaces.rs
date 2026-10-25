//! Workspaces (View › Workspaces): named arrangements of the editor – the workspace
//! (schematic, panel layout, wiring), the docked panels and the open windows – saved by
//! the app and restored with one click.

use serde::{Deserialize, Serialize};

/// Storage key of the saved workspaces.
pub const STORAGE_KEY: &str = "workspaces";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub name: String,
    /// `schematic`, `panel` or `wiring`.
    pub workspace: String,
    /// Key of the navigator tab.
    pub left_tab: String,
    /// Ids of the open windows.
    pub windows: Vec<String>,
    /// Arrangement of the docked panes (JSON, see `crate::dock`); empty in workspaces
    /// saved before panels could be docked.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub dock: String,
}

/// Adds a layout or replaces the one with the same name; names are trimmed, empty
/// names are refused.
pub fn save(list: &mut Vec<Layout>, mut layout: Layout) -> bool {
    layout.name = layout.name.trim().to_string();
    if layout.name.is_empty() {
        return false;
    }
    match list.iter_mut().find(|l| l.name == layout.name) {
        Some(l) => *l = layout,
        None => list.push(layout),
    }
    true
}

pub fn remove(list: &mut Vec<Layout>, name: &str) {
    list.retain(|l| l.name != name);
}

/// Opens the windows of a layout and closes the others.
pub fn apply_windows(layout: &Layout, flags: &mut [(&'static str, &mut bool)]) {
    for (id, open) in flags.iter_mut() {
        **open = layout.windows.iter().any(|w| w == id);
    }
}

/// The ids of the open windows.
pub fn open_windows(flags: &[(&'static str, &mut bool)]) -> Vec<String> {
    flags.iter().filter(|(_, open)| **open).map(|(id, _)| id.to_string()).collect()
}

#[cfg(test)]
mod tests;
