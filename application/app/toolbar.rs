//! Tool bar and the symbol library beside the canvas.

use super::*;

impl App {
    pub(super) fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.workspace, Workspace::Schematic, RichText::new(tr("🗋 Schematic")).strong());
            ui.selectable_value(&mut self.workspace, Workspace::Panel, RichText::new(tr("🗄 Panel layout")).strong());
            ui.selectable_value(&mut self.workspace, Workspace::Wiring, RichText::new(tr("✔ Wiring")).strong())
                .on_hover_text(tr("Wiring assistant: checklist of the wires with 3D view and progress (tablet: open the web app with #wiring)"));
            ui.separator();
            if self.workspace == Workspace::Wiring {
                if ui.add_enabled(self.session.history.can_undo(), egui::Button::new("↩")).on_hover_text(tr("Undo (Ctrl+Z)")).clicked() {
                    self.session.undo();
                }
                if ui.add_enabled(self.session.history.can_redo(), egui::Button::new("↪")).on_hover_text(tr("Redo (Ctrl+Y)")).clicked() {
                    self.session.redo();
                }
                return;
            }
            if self.workspace == Workspace::Panel {
                if ui.add_enabled(self.session.history.can_undo(), egui::Button::new("↩")).on_hover_text(tr("Undo (Ctrl+Z)")).clicked() {
                    self.session.undo();
                }
                if ui.add_enabled(self.session.history.can_redo(), egui::Button::new("↪")).on_hover_text(tr("Redo (Ctrl+Y)")).clicked() {
                    self.session.redo();
                }
                ui.separator();
                if ui.button(tr("📊 Reports")).clicked() {
                    self.report_kind = ReportKind::PanelWiring.into();
                    self.show_reports = true;
                }
                return;
            }
            let tool = self.editor.tool.clone();
            let mut tbtn = |ui: &mut egui::Ui, t: Tool, label: &str, tip: &str| {
                if ui.selectable_label(tool == t, label).on_hover_text(tip).clicked() {
                    self.editor.set_tool(t);
                }
            };
            tbtn(ui, Tool::Select, tr("⬉ Select"), tr("Select / move (Esc)"));
            tbtn(ui, Tool::Wire, tr("✏ Wire"), tr("Draw connections (W)"));
            tbtn(ui, Tool::Text, tr("T Text"), tr("Place text (T)"));
            self.coordinate_field(ui);
            self.drawing_toolbar(ui);
            self.review_tools(ui);
            if let Tool::Place(key) = tool {
                if let Some(def) = self.session.prj.symbol(key) {
                    ui.label(RichText::new(tr_fmt!("Placing: {}", tr(def.name))).strong());
                }
            }
            ui.separator();
            if ui.add_enabled(self.session.history.can_undo(), egui::Button::new("↩")).on_hover_text(tr("Undo (Ctrl+Z)")).clicked() {
                self.session.undo();
            }
            if ui.add_enabled(self.session.history.can_redo(), egui::Button::new("↪")).on_hover_text(tr("Redo (Ctrl+Y)")).clicked() {
                self.session.redo();
            }
            let has_sel = !self.editor.selection.is_empty();
            if ui.add_enabled(has_sel, egui::Button::new("⟳")).on_hover_text(tr("Rotate (R)")).clicked() {
                ops::rotate(&self.session.prj.symbol_table(), &mut self.session.prj.pages[self.session.page], &self.editor.selection);
            }
            if ui.add_enabled(has_sel, egui::Button::new("🗑")).on_hover_text(tr("Delete (Del)")).clicked() {
                ops::delete(&mut self.session.prj.pages[self.session.page], &self.editor.selection);
                self.editor.selection.clear();
            }
            ui.separator();
            if ui.button(tr("⛶ Fit")).on_hover_text(tr("Fit page (F)")).clicked() {
                self.editor.fit_pending = true;
            }
            ui.label(format!("{:.0}%", self.editor.zoom / 3.78 * 100.0)).on_hover_text(tr("100% ≈ real size on a 96 dpi screen"));
            ui.separator();
            if ui.add_enabled(self.session.page > 0, egui::Button::new("◀")).on_hover_text(tr("Previous page (Page Up)")).clicked() {
                self.goto(self.session.page - 1, 0);
            }
            let pg = &self.session.prj.pages[self.session.page];
            ui.label(RichText::new(tr_fmt!("Page {}  {}", structure::page_full_name(pg), pg.description)).strong());
            if ui.add_enabled(self.session.page + 1 < self.session.prj.pages.len(), egui::Button::new("▶")).on_hover_text(tr("Next page (Page Down)")).clicked() {
                self.goto(self.session.page + 1, 0);
            }
            ui.separator();
            if ui.button(tr("📊 Reports")).clicked() {
                self.show_reports = true;
            }
        });
    }

    pub(super) fn library(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(tr("Symbols"));
            ui.add(egui::TextEdit::singleline(&mut self.lib_filter).hint_text(tr("Search…")).desired_width(f32::INFINITY));
        });
        let f = self.lib_filter.to_lowercase();
        let user = self.user_symbols();
        let mut libs: Vec<&str> = user.iter().map(|d| d.library).collect();
        libs.dedup();
        egui::ScrollArea::vertical().id_salt("lib").max_height(ui.available_height() * 0.45).auto_shrink([false, true]).show(ui, |ui| {
            let groups = libs
                .iter()
                .map(|lib| (format!("{}: {lib}", Category::User.title()), user.iter().map(|d| &**d).filter(|d| d.library == *lib).collect::<Vec<_>>()))
                .chain(Category::ALL.into_iter().map(|cat| (cat.title().to_string(), symbols::library().iter().filter(|d| d.category == cat).collect())));
            for (title, defs) in groups {
                let defs: Vec<&symbols::SymbolDef> = defs.into_iter().filter(|d| f.is_empty() || (d.name.to_lowercase().contains(&f) || tr(d.name).to_lowercase().contains(&f))).collect();
                if defs.is_empty() {
                    continue;
                }
                // The extra libraries and the long ones start collapsed.
                let main = !matches!(defs[0].category, Category::Nfpa | Category::Fluid | Category::Process | Category::SingleLine | Category::BlackBox | Category::Installation | Category::Gb | Category::Gost | Category::Jis) && defs.len() <= 24;
                egui::CollapsingHeader::new(title).default_open(!f.is_empty() || main).show(ui, |ui| {
                    for def in defs {
                        let active = self.editor.tool == Tool::Place(def.key);
                        ui.horizontal(|ui| {
                            let icon = symbol_icon(ui, &self.session.prj, def.key, egui::vec2(40.0, 30.0));
                            let lbl = ui.selectable_label(active, format!("{}  ({})", tr(def.name), if def.letter.is_empty() { "–" } else { def.letter }));
                            if icon.clicked() || lbl.clicked() {
                                self.editor.set_tool(Tool::Place(def.key));
                                self.editor.place_rot = 0;
                                self.editor.place_mirror = false;
                            }
                        });
                    }
                });
            }
        });
    }

    /// Registered symbols of the project symbol library, sorted by library and name.
    pub(super) fn user_symbols(&self) -> Vec<symbols::SymbolRef> {
        let mut v: Vec<symbols::SymbolRef> = self.session.prj.symbols.iter().filter_map(|u| self.session.prj.symbol(&u.key)).collect();
        v.sort_by(|a, b| a.library.cmp(b.library).then(natural_cmp(a.name, b.name)));
        v
    }
}
