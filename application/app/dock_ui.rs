//! The docked panes of the schematic workspace (see `crate::dock`) and the View ›
//! Panels menu.

use super::*;
use crate::dock::{self, Pane};

/// Draws the panes of the tile tree with the app.
struct Docked<'a> {
    app: &'a mut App,
    idx: &'a Index,
}

impl egui_tiles::Behavior<Pane> for Docked<'_> {
    fn pane_ui(&mut self, ui: &mut egui::Ui, _tile: egui_tiles::TileId, pane: &mut Pane) -> egui_tiles::UiResponse {
        match *pane {
            Pane::Page => self.app.page_pane(ui, self.idx),
            pane => {
                egui::Frame::new().inner_margin(4).show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    match pane {
                        Pane::Navigator(tab) => self.app.navigator(ui, self.idx, tab),
                        Pane::Library => self.app.library(ui),
                        _ => {
                            egui::ScrollArea::vertical().id_salt("props").auto_shrink([false, false]).show(ui, |ui| self.app.properties(ui, self.idx));
                        }
                    }
                });
            }
        }
        egui_tiles::UiResponse::None
    }

    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        self.app.pane_title(*pane)
    }

    /// A tab holding several panes (split inside a tab) is named after them.
    fn tab_title_for_tile(&mut self, tiles: &egui_tiles::Tiles<Pane>, tile: egui_tiles::TileId) -> egui::WidgetText {
        let mut panes = Vec::new();
        let mut open = vec![tile];
        while let Some(id) = open.pop() {
            match tiles.get(id) {
                Some(egui_tiles::Tile::Pane(p)) => panes.push(*p),
                Some(egui_tiles::Tile::Container(c)) => open.extend(c.children_vec().into_iter().rev()),
                None => {}
            }
        }
        match panes.as_slice() {
            [pane] => self.app.pane_title(*pane),
            _ => panes.iter().map(|p| self.app.pane_title(*p).text().to_string()).collect::<Vec<_>>().join(" + ").into(),
        }
    }

    fn is_tab_closable(&self, tiles: &egui_tiles::Tiles<Pane>, tile: egui_tiles::TileId) -> bool {
        tiles.get_pane(&tile).is_some_and(|p| *p != Pane::Page)
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        egui_tiles::SimplificationOptions { all_panes_must_have_tabs: true, ..Default::default() }
    }
}

impl App {
    /// The schematic workspace: the panes as the user arranged them.
    pub(super) fn docked(&mut self, ui: &mut egui::Ui, idx: &Index) {
        let mut tree = std::mem::replace(&mut self.dock, egui_tiles::Tree::empty(egui::Id::unique("dock")));
        tree.ui(&mut Docked { app: self, idx }, ui);
        // egui_tiles shows the grab hand over tabs, which Windows draws as the move cross.
        ui.ctx().output_mut(|o| {
            if o.cursor_icon == egui::CursorIcon::Grab {
                o.cursor_icon = egui::CursorIcon::Default;
            }
        });
        if dock::find(&tree, Pane::Page).is_none() {
            tree = dock::default_tree();
        }
        self.dock = tree;
    }

    /// Shows a navigator tab (opens it again if it was closed).
    pub(super) fn show_tab(&mut self, tab: LeftTab) {
        dock::show(&mut self.dock, Pane::Navigator(tab));
    }

    /// Title of a pane's tab; the check and review tabs count what is open.
    fn pane_title(&self, pane: Pane) -> egui::WidgetText {
        match pane {
            Pane::Page => match self.session.prj.pages.get(self.session.page) {
                Some(p) => tr_fmt!("Page {}", structure::page_full_name(p)).into(),
                None => tr("Page").into(),
            },
            Pane::Library => tr("Symbols").into(),
            Pane::Properties => tr("Properties").into(),
            Pane::Navigator(tab) => match tab {
                LeftTab::Pages => tr("Pages").into(),
                LeftTab::Devices => tr("Devices").into(),
                LeftTab::Connections => tr("Connections").into(),
                LeftTab::Cables => tr("Cables").into(),
                LeftTab::Layers => tr("Layers").into(),
                LeftTab::Check => {
                    let n = self.session.analysis.issues.iter().filter(|i| i.severity == Severity::Error && !i.accepted).count();
                    if n > 0 {
                        RichText::new(tr_fmt!("Check ({})", n)).color(Color32::from_rgb(220, 60, 60)).into()
                    } else {
                        tr("Check").into()
                    }
                }
                LeftTab::Review => {
                    let n = self.session.prj.review.live_comments().filter(|c| !c.status.is_done()).count();
                    if n > 0 { tr_fmt!("Review ({})", n).into() } else { tr("Review").into() }
                }
                LeftTab::Unplaced => tr("Unplaced").into(),
                LeftTab::Plc => tr("PLC").into(),
                LeftTab::Strips => tr("Strips / plugs").into(),
            },
        }
    }

    /// The page being edited.
    fn page_pane(&mut self, ui: &mut egui::Ui, idx: &Index) {
        let ctx = ui.ctx().clone();
        let page = self.session.page;
        self.editor.read_only = self.page_locked_by().is_some() || self.remote.viewer();
        self.editor.review_selected = self.review_ui.selected;
        let out = self.editor.ui(ui, &mut self.session.prj, page, idx, &self.session.analysis);
        let _ = out.open_properties;
        if let Some(url) = out.open_url {
            ctx.open_url(egui::OpenUrl::new_tab(url));
        }
        if let Some(msg) = out.message {
            self.notify(&ctx, msg);
        }
        if let Some(input) = out.review {
            self.review_input(input);
        } else if let Some(at) = out.clicked {
            self.review_pick(at);
        }
    }

    /// View › Panels: which panes are shown; drag a tab to dock it elsewhere.
    pub(super) fn panels_menu(&mut self, ui: &mut egui::Ui) {
        for pane in Pane::all().into_iter().filter(|p| *p != Pane::Page) {
            let mut shown = dock::find(&self.dock, pane).is_some();
            if ui.checkbox(&mut shown, self.pane_title(pane)).changed() {
                if shown {
                    dock::show(&mut self.dock, pane);
                } else {
                    dock::close(&mut self.dock, pane);
                }
            }
        }
        ui.separator();
        if ui.button(tr("Reset panel layout")).on_hover_text(tr("Navigator on the left, symbols and properties on the right")).clicked() {
            self.dock = dock::default_tree();
            ui.close();
        }
    }
}
