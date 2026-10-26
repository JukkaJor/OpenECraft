//! The docked panes (see `crate::dock`): every pane drawn by the app, its tab title,
//! the pane menu (float, close, help), the page tabs above the page and the panes
//! floating as windows.

use super::*;
use crate::dock::{self, Pane};

/// What the pane menu of a tab bar asked for.
enum PaneRequest {
    Float(Pane),
    Close(Pane),
    Help(Pane),
}

/// Draws the panes of the tile tree with the app.
struct Docked<'a> {
    app: &'a mut App,
    idx: &'a Index,
    request: Option<PaneRequest>,
    documents: usize,
}

impl egui_tiles::Behavior<Pane> for Docked<'_> {
    fn pane_ui(&mut self, ui: &mut egui::Ui, _tile: egui_tiles::TileId, pane: &mut Pane) -> egui_tiles::UiResponse {
        self.app.pane_ui(ui, *pane, self.idx);
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
        tiles.get_pane(&tile).is_some_and(|p| !p.is_document() || self.documents > 1)
    }

    fn top_bar_right_ui(&mut self, tiles: &egui_tiles::Tiles<Pane>, ui: &mut egui::Ui, _tile: egui_tiles::TileId, tabs: &egui_tiles::Tabs, _scroll: &mut f32) {
        let Some(pane) = tabs.active.and_then(|id| tiles.get_pane(&id)).copied() else { return };
        ui.add_space(4.0);
        let r = icons::button(ui, Icon::Dots, tr("Pane menu"));
        egui::Popup::menu(&r).show(|ui| {
            if icons::text_button(ui, Icon::Float, tr("Float")).on_hover_text(tr("Show the pane as a window")).clicked() {
                self.request = Some(PaneRequest::Float(pane));
                ui.close();
            }
            if (!pane.is_document() || self.documents > 1) && icons::text_button(ui, Icon::Close, tr("Close")).clicked() {
                self.request = Some(PaneRequest::Close(pane));
                ui.close();
            }
            if icons::text_button(ui, Icon::Help, tr("Help")).clicked() {
                self.request = Some(PaneRequest::Help(pane));
                ui.close();
            }
        });
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        egui_tiles::SimplificationOptions { all_panes_must_have_tabs: true, ..Default::default() }
    }
}

impl App {
    /// The panes of the workspace as the user arranged them.
    pub(super) fn docked(&mut self, ui: &mut egui::Ui, idx: &Index) {
        let mut tree = std::mem::replace(&mut self.dock, egui_tiles::Tree::empty(egui::Id::unique("dock_swap")));
        let documents = dock::panes(&tree).iter().filter(|p| p.is_document()).count();
        let mut behavior = Docked { app: self, idx, request: None, documents };
        tree.ui(&mut behavior, ui);
        let request = behavior.request.take();
        // egui_tiles shows the grab hand over tabs, which Windows draws as the move cross.
        ui.ctx().output_mut(|o| {
            if o.cursor_icon == egui::CursorIcon::Grab {
                o.cursor_icon = egui::CursorIcon::Default;
            }
        });
        if tree.root().is_none() || dock::panes(&tree).is_empty() {
            tree = dock::default_tree_for(self.workspace);
        }
        self.dock = tree;
        match request {
            Some(PaneRequest::Float(p)) => {
                dock::close(&mut self.dock, p);
                if dock::find(&self.dock, p).is_none() {
                    self.shell.placement.set(p, true);
                    self.show_pane(p);
                }
            }
            Some(PaneRequest::Close(p)) => dock::close(&mut self.dock, p),
            Some(PaneRequest::Help(p)) => self.help_for(p),
            None => {}
        }
    }

    /// Panes shown as windows, opening in the middle of the editor; Dock puts them into
    /// the workspace (and they open docked from then on).
    pub(super) fn floating_panes(&mut self, ctx: &egui::Context) {
        // A pane docked in the workspace switched to is shown there only.
        self.floating.retain(|p| dock::find(&self.dock, *p).is_none());
        let idx = std::mem::take(&mut self.session.index);
        let screen = ctx.content_rect();
        for (i, pane) in self.floating.clone().into_iter().enumerate() {
            let mut open = true;
            let mut dock_back = false;
            let title = self.pane_title(pane).text().to_string();
            let id = egui::Id::unique(("floating", pane));
            let [w, h] = match pane {
                Pane::Tool(t) => t.window_size(),
                _ => [640.0, 440.0],
            };
            let size = egui::vec2(w.min(screen.width() * 0.9), h.min(screen.height() * 0.8));
            let at = screen.center() + egui::Vec2::splat(24.0 * (i % 8) as f32);
            if self.raise == Some(pane) {
                self.raise = None;
                ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, id));
            }
            egui::Window::new(title).id(id).open(&mut open).default_size(size).default_pos(at).pivot(egui::Align2::CENTER_CENTER).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if icons::text_button(ui, Icon::Panes, tr("Dock")).on_hover_text(tr("Put the pane back into the workspace")).clicked() {
                        dock_back = true;
                    }
                });
                ui.separator();
                self.pane_ui(ui, pane, &idx);
            });
            if !open || dock_back {
                self.floating.retain(|p| *p != pane);
                if dock_back {
                    self.shell.placement.set(pane, false);
                    self.show_pane(pane);
                }
            }
        }
        self.session.index = idx;
    }

    /// The content of a pane.
    pub(super) fn pane_ui(&mut self, ui: &mut egui::Ui, pane: Pane, idx: &Index) {
        // Clicks make a pane the active one: keys go to the document worked in.
        if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect()) {
            self.active_pane = Some(pane);
            self.focus_pane = None;
            if pane.is_document() {
                self.active_doc = pane;
            }
        }
        if self.focus_pane == Some(pane) {
            ui.painter().rect_stroke(ui.max_rect().shrink(1.0), 2.0, ui.visuals().selection.stroke, egui::StrokeKind::Inside);
        }
        match pane {
            Pane::Page => self.page_pane(ui, idx),
            Pane::LayoutView => self.layout_view_pane(ui),
            Pane::WiringWorkbench => self.wiring_workbench_pane(ui),
            pane => {
                egui::Frame::new().inner_margin(4).show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    match pane {
                        Pane::Navigator(tab) => self.navigator(ui, idx, tab),
                        Pane::Library => self.insert_center_pane(ui),
                        Pane::Properties => {
                            egui::ScrollArea::vertical().id_salt("props").auto_shrink([false, false]).show(ui, |ui| self.properties(ui, idx));
                        }
                        Pane::LayoutSpaces => self.layout_spaces_pane(ui),
                        Pane::LayoutProperties => self.layout_properties_pane(ui),
                        Pane::WiringChecklist => self.wiring_checklist_pane(ui),
                        Pane::Tool(t) => self.tool_pane(ui, t),
                        Pane::Page | Pane::LayoutView | Pane::WiringWorkbench => {}
                    }
                });
            }
        }
    }

    fn tool_pane(&mut self, ui: &mut egui::Ui, t: ToolPane) {
        use ToolPane as T;
        match t {
            T::Start => self.start_page(ui),
            T::Help => self.help_pane(ui),
            T::Reports => self.reports_pane(ui),
            T::Tables => self.tables_pane(ui),
            T::Find => self.find_pane(ui),
            T::Cables => self.cables_pane(ui),
            T::Potentials => self.potentials_pane(ui),
            T::TerminalStrips => self.strips_pane(ui),
            T::Plugs => self.plugs_pane(ui),
            T::PlcOverview => self.plc_overview_pane(ui),
            T::PlcHardware => self.plc_hw_pane(ui),
            T::Macros => self.macros_pane(ui),
            T::Harnesses => self.harness_pane(ui),
            T::FieldWiring => self.field_pane(ui),
            T::Revisions => self.revisions_pane(ui),
            T::Compare => self.compare_pane(ui),
            T::ChangeRequests => self.requests_pane(ui),
            T::Preplanning => self.preplanning_pane(ui),
            T::SingleLine => self.singleline_pane(ui),
            T::Process => self.process_pane(ui),
            T::Electrical => self.electrical_pane(ui),
            T::Integration => self.integration_pane(ui),
            T::Languages => self.languages_pane(ui),
            T::Configurator => self.config_pane(ui),
            T::Projects => self.projects_pane(ui),
            T::Structure => self.structure_pane(ui),
            T::PropertyDatabase => self.property_db_pane(ui),
            T::SymbolEditor => self.symbol_editor_pane(ui),
            T::Forms => self.forms_pane(ui),
            T::ReportBlocks => self.report_blocks_pane(ui),
            T::Parts => self.parts_pane(ui),
            T::DataPortal => self.portal_pane(ui),
            T::ProjectServer => self.remote_pane(ui),
            T::Vault => self.vault_pane(ui),
            T::Plugins => self.plugins_pane(ui),
            T::PluginCommands => self.plugin_commands_pane(ui),
        }
    }

    /// Shows a navigator tab (opens it again if it was closed).
    pub(super) fn show_tab(&mut self, tab: LeftTab) {
        self.show_pane(Pane::Navigator(tab));
    }

    /// Title of a pane's tab; the messages and review tabs count what is open.
    pub(super) fn pane_title(&self, pane: Pane) -> egui::WidgetText {
        match pane {
            Pane::Page => match self.session.prj.pages.get(self.session.page) {
                Some(p) => tr_fmt!("Page {}", structure::page_full_name(p)).into(),
                None => tr("Page").into(),
            },
            Pane::Navigator(LeftTab::Check) => {
                let n = self.session.analysis.issues.iter().filter(|i| i.severity == Severity::Error && !i.accepted).count();
                if n > 0 {
                    RichText::new(tr_fmt!("Messages ({})", n)).color(crate::style::colors(self.dark).error).into()
                } else {
                    tr("Messages").into()
                }
            }
            Pane::Navigator(LeftTab::Review) => {
                let n = self.session.prj.review.live_comments().filter(|c| !c.status.is_done()).count();
                if n > 0 { tr_fmt!("Review ({})", n).into() } else { tr("Review").into() }
            }
            Pane::Tool(ToolPane::Reports) => tr("Reports").into(),
            Pane::Tool(ToolPane::Find) => tr("Find").into(),
            Pane::Tool(ToolPane::Tables) => tr("Edit in table").into(),
            pane => match crate::commands::for_pane(pane) {
                Some(c) => tr(crate::commands::def(c).title).trim_end_matches('…').to_string().into(),
                None => String::new().into(),
            },
        }
    }

    /// The page being edited, with the page tabs above it.
    fn page_pane(&mut self, ui: &mut egui::Ui, idx: &Index) {
        let ctx = ui.ctx().clone();
        self.page_tabs_bar(ui);
        let page = self.session.page;
        self.editor.read_only = self.page_locked_by().is_some() || self.remote.viewer();
        self.editor.review_selected = self.review_ui.selected;
        let out = self.editor.ui(ui, &mut self.session.prj, page, idx, &self.session.analysis);
        if out.open_properties {
            self.props_window = true;
        }
        if let Some(url) = out.open_url {
            ctx.open_url(egui::OpenUrl::new_tab(url));
        }
        if let Some(msg) = out.message {
            self.notify(&ctx, msg);
        }
        if let Some(id) = out.placed {
            self.placed(id);
        }
        if let Some(resp) = &out.response {
            resp.context_menu(|ui| self.page_context_menu(ui));
        }
        if let Some(input) = out.review {
            self.review_input(input);
        } else if let Some(at) = out.clicked {
            self.review_pick(at);
        }
    }

    /// A symbol was placed: a part chosen in the Insert center goes to its device.
    fn placed(&mut self, id: u64) {
        let Some(number) = self.pending_part.take() else { return };
        let page = &self.session.prj.pages[self.session.page];
        if let Some(tag) = page.symbols.iter().find(|s| s.id == id).map(|s| s.tag.clone()).filter(|t| !t.is_empty()) {
            parts::add_to_device(&mut self.session.prj, &self.parts_db, &tag, &number, 1);
        }
    }

    /// Tabs of the pages opened lately; a click shows the page, the cross closes the tab.
    fn page_tabs_bar(&mut self, ui: &mut egui::Ui) {
        let current = self.session.prj.pages.get(self.session.page).map(|p| p.id);
        let mut goto = None;
        let mut close = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for &id in &self.page_tabs {
                let Some((i, pg)) = self.session.prj.pages.iter().enumerate().find(|(_, p)| p.id == id) else { continue };
                let name = structure::page_full_name(pg);
                let label = if pg.description.is_empty() { name } else { format!("{name}  {}", pg.description) };
                let label: String = label.chars().take(28).collect();
                let r = ui.selectable_label(Some(id) == current, RichText::new(label).small()).on_hover_text(&pg.description);
                if r.clicked() {
                    goto = Some(i);
                }
                if r.middle_clicked() {
                    close = Some(id);
                }
                r.context_menu(|ui| {
                    if ui.button(tr("Page properties…")).clicked() {
                        self.page_props = Some(i);
                        ui.close();
                    }
                    if self.page_tabs.len() > 1 && ui.button(tr("Close page tab")).clicked() {
                        close = Some(id);
                        ui.close();
                    }
                });
                if self.page_tabs.len() > 1 && Some(id) == current && icons::button(ui, Icon::Close, tr("Close page tab")).clicked() {
                    close = Some(id);
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icons::button(ui, Icon::Plus, tr("New schematic page")).clicked() {
                    self.add_page(PageKind::Schematic);
                }
            });
        });
        if let Some(i) = goto {
            self.goto(i, 0);
        }
        if let Some(id) = close {
            self.close_page_tab(id);
        }
    }

    /// Right click on the page: the commands for the selection or the page.
    fn page_context_menu(&mut self, ui: &mut egui::Ui) {
        let has_sel = !self.editor.selection.is_empty();
        if has_sel {
            if self.selected_tag().is_some() {
                for c in [Cmd::Properties, Cmd::RenameDevice, Cmd::PartSelection] {
                    self.cmd_button(ui, c);
                }
                ui.separator();
            }
            for c in [Cmd::Cut, Cmd::Copy, Cmd::Delete, Cmd::Rotate, Cmd::Mirror] {
                self.cmd_button(ui, c);
            }
            ui.menu_button(tr("Align"), |ui| self.cmd_menu(ui, Cmd::Align));
            ui.menu_button(tr("Distribute"), |ui| self.cmd_menu(ui, Cmd::Distribute));
            for c in [Cmd::MoveBy, Cmd::MultipleCopy, Cmd::AutoConnectSelection, Cmd::SaveSelectionAsMacro] {
                self.cmd_button(ui, c);
            }
        } else {
            for c in [Cmd::Paste, Cmd::SelectAll, Cmd::InsertCenter, Cmd::WireTool, Cmd::TextTool] {
                self.cmd_button(ui, c);
            }
            ui.separator();
            for c in [Cmd::PageProperties, Cmd::NewSchematicPage, Cmd::FitPage, Cmd::JoinCollinear] {
                self.cmd_button(ui, c);
            }
        }
    }
}
