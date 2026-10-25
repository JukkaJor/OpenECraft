//! Docking panels of the schematic workspace: the page, each navigator tab, the symbol
//! library and the properties are panes of a tile tree that the user rearranges by
//! dragging tabs (side by side, stacked, as tabs of one another). The tree is saved by
//! the app and in workspaces. No UI code; `app/dock_ui.rs` draws the panes.

use egui_tiles::{Container, Linear, LinearDir, Tile, TileId, Tiles, Tree};
use serde::{Deserialize, Serialize};

/// Storage key of the panel arrangement.
pub const STORAGE_KEY: &str = "dock";

/// Tabs of the navigator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeftTab {
    Pages,
    Devices,
    Connections,
    Cables,
    Layers,
    Check,
    Review,
    /// Functions of the project data not placed yet.
    Unplaced,
    /// PLC cards with their channels.
    Plc,
    /// Terminal strips and plugs with their terminals and pins.
    Strips,
}

impl LeftTab {
    pub const ALL: [LeftTab; 10] = [
        LeftTab::Pages,
        LeftTab::Devices,
        LeftTab::Connections,
        LeftTab::Cables,
        LeftTab::Layers,
        LeftTab::Check,
        LeftTab::Review,
        LeftTab::Unplaced,
        LeftTab::Plc,
        LeftTab::Strips,
    ];

    /// Name in saved workspaces.
    pub fn key(self) -> &'static str {
        match self {
            LeftTab::Pages => "pages",
            LeftTab::Devices => "devices",
            LeftTab::Connections => "connections",
            LeftTab::Cables => "cables",
            LeftTab::Layers => "layers",
            LeftTab::Check => "check",
            LeftTab::Review => "review",
            LeftTab::Unplaced => "unplaced",
            LeftTab::Plc => "plc",
            LeftTab::Strips => "strips",
        }
    }

    pub fn from_key(key: &str) -> Option<LeftTab> {
        LeftTab::ALL.into_iter().find(|t| t.key() == key)
    }
}

/// What a pane shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pane {
    /// The page being edited; it cannot be closed.
    Page,
    Navigator(LeftTab),
    /// Symbol library.
    Library,
    Properties,
}

impl Pane {
    /// Every pane, in the order of the View › Panels menu.
    pub fn all() -> Vec<Pane> {
        let mut v = vec![Pane::Page];
        v.extend(LeftTab::ALL.map(Pane::Navigator));
        v.extend([Pane::Library, Pane::Properties]);
        v
    }
}

/// The arrangement of the editor before any change: the navigator tabs on the left,
/// the page in the middle, library above properties on the right.
pub fn default_tree() -> Tree<Pane> {
    let mut tiles = Tiles::default();
    let navs: Vec<TileId> = LeftTab::ALL.into_iter().map(|t| tiles.insert_pane(Pane::Navigator(t))).collect();
    let nav = tiles.insert_tab_tile(navs);
    let page = tiles.insert_pane(Pane::Page);
    let page = tiles.insert_tab_tile(vec![page]);
    let library = tiles.insert_pane(Pane::Library);
    let library = tiles.insert_tab_tile(vec![library]);
    let props = tiles.insert_pane(Pane::Properties);
    let props = tiles.insert_tab_tile(vec![props]);
    let mut side = Linear::new(LinearDir::Vertical, vec![library, props]);
    side.shares.set_share(library, 1.0);
    side.shares.set_share(props, 2.0);
    let side = tiles.insert_container(side);
    let mut root = Linear::new(LinearDir::Horizontal, vec![nav, page, side]);
    root.shares.set_share(nav, 230.0);
    root.shares.set_share(page, 910.0);
    root.shares.set_share(side, 300.0);
    let root = tiles.insert_container(root);
    Tree::new(egui::Id::unique("dock"), root, tiles)
}

/// The tile of a pane, if it is in the tree.
pub fn find(tree: &Tree<Pane>, pane: Pane) -> Option<TileId> {
    tree.tiles.find_pane(&pane)
}

/// Shows a pane: makes it the active tab where it is, or adds it as a tab beside the
/// navigator (or beside the page) if it was closed.
pub fn show(tree: &mut Tree<Pane>, pane: Pane) {
    if find(tree, pane).is_none() {
        let near = LeftTab::ALL.into_iter().find_map(|t| find(tree, Pane::Navigator(t))).or_else(|| find(tree, Pane::Page));
        let tabs = near.and_then(|t| tree.tiles.parent_of(t)).filter(|p| matches!(tree.tiles.get(*p), Some(Tile::Container(Container::Tabs(_)))));
        let id = tree.tiles.insert_pane(pane);
        match tabs {
            Some(parent) => {
                if let Some(Tile::Container(c)) = tree.tiles.get_mut(parent) {
                    c.add_child(id);
                }
            }
            None => {
                let tabs = tree.tiles.insert_tab_tile(vec![id]);
                let root = match tree.root() {
                    Some(old) => tree.tiles.insert_horizontal_tile(vec![tabs, old]),
                    None => tabs,
                };
                tree.root = Some(root);
            }
        }
    }
    tree.make_active(|_, tile| matches!(tile, Tile::Pane(p) if *p == pane));
}

/// Closes a pane (not the page).
pub fn close(tree: &mut Tree<Pane>, pane: Pane) {
    if pane == Pane::Page {
        return;
    }
    if let Some(id) = find(tree, pane) {
        tree.remove_recursively(id);
    }
}

/// The navigator tab shown first: the active one of the first navigator pane found.
pub fn active_tab(tree: &Tree<Pane>) -> Option<LeftTab> {
    let active = tree.active_tiles();
    LeftTab::ALL.into_iter().find(|t| find(tree, Pane::Navigator(*t)).is_some_and(|id| active.contains(&id)))
}

pub fn to_json(tree: &Tree<Pane>) -> String {
    serde_json::to_string(tree).unwrap_or_default()
}

/// A saved arrangement; `None` if it cannot be read or has no page (or more than one).
pub fn from_json(text: &str) -> Option<Tree<Pane>> {
    let tree: Tree<Pane> = serde_json::from_str(text).ok()?;
    let pages = tree.tiles.tiles().filter(|t| matches!(t, Tile::Pane(Pane::Page))).count();
    let root = tree.root()?;
    (pages == 1 && tree.tiles.get(root).is_some()).then_some(tree)
}

#[cfg(test)]
mod tests;
