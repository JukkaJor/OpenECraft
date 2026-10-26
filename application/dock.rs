//! Docking panes of the editor: the page, the navigators, the Insert center, the
//! properties, the panel layout, the wiring assistant and every tool (reports, cables,
//! revisions, parts, …) are panes of a tile tree that the user rearranges by dragging
//! tabs (side by side, stacked, as tabs of one another). Each workspace has its own
//! tree; the trees are saved by the app and in workspaces. Tools open as floating
//! windows unless the user docked them (`Placement`). No UI code:
//! `app/dock_ui.rs` draws the panes.

use crate::workspaces::Workspace;
use egui_tiles::{Container, Linear, LinearDir, Tile, TileId, Tiles, Tree};
use serde::{Deserialize, Serialize};

/// Storage key of the panel arrangement.
pub const STORAGE_KEY: &str = "dock";

/// Tabs of the navigator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LeftTab {
    Pages,
    Devices,
    Connections,
    Cables,
    Layers,
    /// The messages of the project check.
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

/// The tools of the editor that are panes (they were windows before).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolPane {
    Start,
    Help,
    Reports,
    Tables,
    Find,
    Cables,
    Potentials,
    TerminalStrips,
    Plugs,
    PlcOverview,
    PlcHardware,
    Macros,
    Harnesses,
    FieldWiring,
    Revisions,
    Compare,
    ChangeRequests,
    Preplanning,
    SingleLine,
    Process,
    Electrical,
    Integration,
    Languages,
    Configurator,
    Projects,
    Structure,
    PropertyDatabase,
    SymbolEditor,
    Forms,
    ReportBlocks,
    Parts,
    DataPortal,
    ProjectServer,
    Vault,
    Plugins,
    PluginCommands,
}

impl ToolPane {
    pub const ALL: [ToolPane; 36] = [
        ToolPane::Start,
        ToolPane::Help,
        ToolPane::Reports,
        ToolPane::Tables,
        ToolPane::Find,
        ToolPane::Cables,
        ToolPane::Potentials,
        ToolPane::TerminalStrips,
        ToolPane::Plugs,
        ToolPane::PlcOverview,
        ToolPane::PlcHardware,
        ToolPane::Macros,
        ToolPane::Harnesses,
        ToolPane::FieldWiring,
        ToolPane::Revisions,
        ToolPane::Compare,
        ToolPane::ChangeRequests,
        ToolPane::Preplanning,
        ToolPane::SingleLine,
        ToolPane::Process,
        ToolPane::Electrical,
        ToolPane::Integration,
        ToolPane::Languages,
        ToolPane::Configurator,
        ToolPane::Projects,
        ToolPane::Structure,
        ToolPane::PropertyDatabase,
        ToolPane::SymbolEditor,
        ToolPane::Forms,
        ToolPane::ReportBlocks,
        ToolPane::Parts,
        ToolPane::DataPortal,
        ToolPane::ProjectServer,
        ToolPane::Vault,
        ToolPane::Plugins,
        ToolPane::PluginCommands,
    ];

    /// Narrow tools open beside the navigators and the properties; the others are wide
    /// and open as tabs beside the page.
    pub fn is_side(self) -> bool {
        matches!(self, ToolPane::Find | ToolPane::ProjectServer | ToolPane::Vault | ToolPane::Plugins)
    }

    /// Tools open as floating windows, so that it is clear what a button opened; the
    /// start page and the find results open docked.
    pub fn floats(self) -> bool {
        !matches!(self, ToolPane::Start | ToolPane::Find)
    }

    /// Size of the tool's window when it opens.
    pub fn window_size(self) -> [f32; 2] {
        use ToolPane as T;
        match self {
            T::Tables | T::PlcHardware | T::Harnesses | T::FieldWiring | T::Compare | T::Preplanning | T::SingleLine | T::Process | T::Electrical | T::Integration | T::Projects | T::SymbolEditor | T::Forms | T::Parts | T::DataPortal => [960.0, 640.0],
            T::Help | T::Potentials | T::Plugs | T::ProjectServer | T::Vault | T::Plugins => [480.0, 560.0],
            _ => [720.0, 500.0],
        }
    }
}

/// What a pane shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pane {
    /// The page being edited.
    Page,
    Navigator(LeftTab),
    /// The Insert center (symbols, macros, devices, unplaced functions).
    Library,
    Properties,
    Tool(ToolPane),
    /// Panel layout: enclosures, devices to place and placed devices.
    LayoutSpaces,
    /// Panel layout: the front view or the 3D view.
    LayoutView,
    /// Panel layout: the properties of the selection.
    LayoutProperties,
    /// Wiring and mounting assistant: the list of steps.
    WiringChecklist,
    /// Wiring and mounting assistant: the 3D view with the current step.
    WiringWorkbench,
}

/// Where a pane opens when it is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Area {
    Left,
    Center,
    Right,
    Bottom,
}

impl Pane {
    /// Every pane, in the order of the View › Panes menu.
    pub fn all() -> Vec<Pane> {
        let mut v = vec![Pane::Page];
        v.extend(LeftTab::ALL.map(Pane::Navigator));
        v.extend([Pane::Library, Pane::Properties]);
        v.extend([Pane::LayoutSpaces, Pane::LayoutView, Pane::LayoutProperties, Pane::WiringChecklist, Pane::WiringWorkbench]);
        v.extend(ToolPane::ALL.map(Pane::Tool));
        v
    }

    pub fn area(self) -> Area {
        match self {
            Pane::Navigator(LeftTab::Check) => Area::Bottom,
            Pane::Navigator(_) | Pane::LayoutSpaces | Pane::WiringChecklist => Area::Left,
            Pane::Library | Pane::Properties | Pane::LayoutProperties => Area::Right,
            Pane::Tool(t) if t.is_side() => Area::Right,
            _ => Area::Center,
        }
    }

    /// The page cannot be closed while it is the only document of the workspace.
    pub fn is_document(self) -> bool {
        self.area() == Area::Center
    }

    /// The pane opens as a window unless the user docked it.
    pub fn floats(self) -> bool {
        matches!(self, Pane::Tool(t) if t.floats())
    }
}

/// Where panes open that are not in the workspace: as a window or docked. Panes follow
/// `Pane::floats` until the user floats or docks one (pane menu › Float, Dock).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Placement {
    docked: Vec<Pane>,
    floated: Vec<Pane>,
}

impl Placement {
    pub fn floats(&self, pane: Pane) -> bool {
        if self.docked.contains(&pane) {
            false
        } else {
            self.floated.contains(&pane) || pane.floats()
        }
    }

    /// Remembers that the user floated (`true`) or docked a pane.
    pub fn set(&mut self, pane: Pane, floats: bool) {
        self.docked.retain(|p| *p != pane);
        self.floated.retain(|p| *p != pane);
        if floats != pane.floats() {
            if floats { &mut self.floated } else { &mut self.docked }.push(pane);
        }
    }
}

fn tabs(tiles: &mut Tiles<Pane>, panes: &[Pane]) -> TileId {
    let ids: Vec<TileId> = panes.iter().map(|p| tiles.insert_pane(*p)).collect();
    tiles.insert_tab_tile(ids)
}

fn linear(tiles: &mut Tiles<Pane>, dir: LinearDir, children: &[(TileId, f32)]) -> TileId {
    let mut l = Linear::new(dir, children.iter().map(|c| c.0).collect());
    for (id, share) in children {
        l.shares.set_share(*id, *share);
    }
    tiles.insert_container(l)
}

const NAVIGATORS: [Pane; 9] = [
    Pane::Navigator(LeftTab::Pages),
    Pane::Navigator(LeftTab::Devices),
    Pane::Navigator(LeftTab::Connections),
    Pane::Navigator(LeftTab::Cables),
    Pane::Navigator(LeftTab::Strips),
    Pane::Navigator(LeftTab::Plc),
    Pane::Navigator(LeftTab::Unplaced),
    Pane::Navigator(LeftTab::Layers),
    Pane::Navigator(LeftTab::Review),
];

/// The arrangement of a workspace before any change.
pub fn default_tree_for(workspace: Workspace) -> Tree<Pane> {
    let mut tiles = Tiles::default();
    let root = match workspace {
        Workspace::Schematic => {
            let nav = tabs(&mut tiles, &NAVIGATORS);
            let page = tabs(&mut tiles, &[Pane::Page]);
            let messages = tabs(&mut tiles, &[Pane::Navigator(LeftTab::Check)]);
            let center = linear(&mut tiles, LinearDir::Vertical, &[(page, 4.0), (messages, 1.0)]);
            let library = tabs(&mut tiles, &[Pane::Library]);
            let props = tabs(&mut tiles, &[Pane::Properties]);
            let side = linear(&mut tiles, LinearDir::Vertical, &[(library, 1.0), (props, 1.2)]);
            linear(&mut tiles, LinearDir::Horizontal, &[(nav, 230.0), (center, 910.0), (side, 300.0)])
        }
        Workspace::Panel => {
            let nav = tabs(&mut tiles, &[Pane::LayoutSpaces, Pane::Navigator(LeftTab::Devices)]);
            let view = tabs(&mut tiles, &[Pane::LayoutView, Pane::Page]);
            let props = tabs(&mut tiles, &[Pane::LayoutProperties]);
            linear(&mut tiles, LinearDir::Horizontal, &[(nav, 250.0), (view, 900.0), (props, 300.0)])
        }
        Workspace::Wiring | Workspace::Mounting => {
            let list = tabs(&mut tiles, &[Pane::WiringChecklist]);
            let bench = tabs(&mut tiles, &[Pane::WiringWorkbench]);
            linear(&mut tiles, LinearDir::Horizontal, &[(list, 380.0), (bench, 1000.0)])
        }
        Workspace::Review => {
            let nav = tabs(&mut tiles, &[Pane::Navigator(LeftTab::Review), Pane::Navigator(LeftTab::Pages), Pane::Navigator(LeftTab::Check)]);
            let page = tabs(&mut tiles, &[Pane::Page, Pane::Tool(ToolPane::Revisions), Pane::Tool(ToolPane::ChangeRequests)]);
            let props = tabs(&mut tiles, &[Pane::Properties]);
            linear(&mut tiles, LinearDir::Horizontal, &[(nav, 300.0), (page, 900.0), (props, 280.0)])
        }
        Workspace::Preplanning => {
            let nav = tabs(&mut tiles, &[Pane::Navigator(LeftTab::Pages), Pane::Navigator(LeftTab::Devices)]);
            let page = tabs(&mut tiles, &[Pane::Tool(ToolPane::Preplanning), Pane::Tool(ToolPane::SingleLine), Pane::Page]);
            let props = tabs(&mut tiles, &[Pane::Properties]);
            linear(&mut tiles, LinearDir::Horizontal, &[(nav, 230.0), (page, 950.0), (props, 280.0)])
        }
    };
    Tree::new(egui::Id::unique(("dock", workspace.key())), root, tiles)
}

/// The arrangement of the schematic workspace before any change.
#[cfg(test)]
pub fn default_tree() -> Tree<Pane> {
    default_tree_for(Workspace::Schematic)
}

/// The tile of a pane, if it is in the tree.
pub fn find(tree: &Tree<Pane>, pane: Pane) -> Option<TileId> {
    tree.tiles.find_pane(&pane)
}

/// The panes of the tree in their order.
pub fn panes(tree: &Tree<Pane>) -> Vec<Pane> {
    let mut out = Vec::new();
    let mut open: Vec<TileId> = tree.root().into_iter().collect();
    while let Some(id) = open.pop() {
        match tree.tiles.get(id) {
            Some(Tile::Pane(p)) => out.push(*p),
            Some(Tile::Container(c)) => open.extend(c.children_vec().into_iter().rev()),
            None => {}
        }
    }
    out
}

/// The panes that are shown (active tabs), in their order.
pub fn visible(tree: &Tree<Pane>) -> Vec<Pane> {
    let active = tree.active_tiles();
    panes(tree).into_iter().filter(|p| find(tree, *p).is_some_and(|id| active.contains(&id))).collect()
}

/// The tab container a pane of `area` joins: the parent of a pane of the same area.
fn container_for(tree: &Tree<Pane>, pane: Pane) -> Option<TileId> {
    let area = pane.area();
    let order: Vec<Pane> = panes(tree);
    // Prefer a pane of the same kind (navigator beside navigator, tool beside the page).
    let candidates = order
        .iter()
        .filter(|p| p.area() == area)
        .chain(order.iter().filter(|p| area == Area::Bottom && p.area() == Area::Center))
        .chain(order.iter().filter(|p| area == Area::Left && p.area() == Area::Center));
    for p in candidates {
        let Some(id) = find(tree, *p) else { continue };
        if let Some(parent) = tree.tiles.parent_of(id) {
            if matches!(tree.tiles.get(parent), Some(Tile::Container(Container::Tabs(_)))) {
                return Some(parent);
            }
        }
    }
    None
}

/// Shows a pane: makes it the active tab where it is, or adds it as a tab of the area it
/// belongs to (navigators on the left, tools beside the page, …) if it was closed.
pub fn show(tree: &mut Tree<Pane>, pane: Pane) {
    if find(tree, pane).is_none() {
        let parent = container_for(tree, pane);
        let id = tree.tiles.insert_pane(pane);
        match parent {
            Some(parent) => {
                if let Some(Tile::Container(c)) = tree.tiles.get_mut(parent) {
                    c.add_child(id);
                }
            }
            None => {
                let tabs = tree.tiles.insert_tab_tile(vec![id]);
                let root = match tree.root() {
                    Some(old) if pane.area() == Area::Right => tree.tiles.insert_horizontal_tile(vec![old, tabs]),
                    Some(old) if pane.area() == Area::Bottom => tree.tiles.insert_vertical_tile(vec![old, tabs]),
                    Some(old) => tree.tiles.insert_horizontal_tile(vec![tabs, old]),
                    None => tabs,
                };
                tree.root = Some(root);
            }
        }
    }
    tree.make_active(|_, tile| matches!(tile, Tile::Pane(p) if *p == pane));
}

/// Closes a pane; the last document of a tree stays.
pub fn close(tree: &mut Tree<Pane>, pane: Pane) {
    if !can_close(tree, pane) {
        return;
    }
    if let Some(id) = find(tree, pane) {
        tree.remove_recursively(id);
    }
}

/// A pane can be closed unless it is the only document left.
pub fn can_close(tree: &Tree<Pane>, pane: Pane) -> bool {
    !pane.is_document() || panes(tree).iter().filter(|p| p.is_document()).count() > 1
}

/// The navigator tab shown first: the active one of the navigators on the left.
pub fn active_tab(tree: &Tree<Pane>) -> Option<LeftTab> {
    let active = tree.active_tiles();
    LeftTab::ALL.into_iter().filter(|t| Pane::Navigator(*t).area() == Area::Left).find(|t| find(tree, Pane::Navigator(*t)).is_some_and(|id| active.contains(&id)))
}

pub fn to_json(tree: &Tree<Pane>) -> String {
    serde_json::to_string(tree).unwrap_or_default()
}

/// A saved arrangement; `None` if it cannot be read, is empty or has a pane twice.
pub fn from_json(text: &str) -> Option<Tree<Pane>> {
    let tree: Tree<Pane> = serde_json::from_str(text).ok()?;
    let root = tree.root()?;
    tree.tiles.get(root)?;
    let all: Vec<Pane> = tree.tiles.tiles().filter_map(|t| if let Tile::Pane(p) = t { Some(*p) } else { None }).collect();
    let unique = all.iter().all(|p| all.iter().filter(|q| *q == p).count() == 1);
    (unique && all.iter().any(|p| p.is_document())).then_some(tree)
}

#[cfg(test)]
mod tests;
