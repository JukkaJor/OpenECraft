//! The status bar: position of the cursor (and typed coordinates), grid, layer, zoom,
//! the hint of the tool, the work area, the language, the lock of the page, the state
//! of the servers and the messages of the session. Segments open what they show.

use super::*;
use crate::commands::Cmd;

impl App {
    #[allow(clippy::too_many_lines)]
    pub(super) fn status_bar(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let on_page = self.active_doc == Pane::Page;
        // The hint fills the space between the left and the right segments.
        let hint = if !on_page {
            self.panel_ui.status.clone()
        } else if self.editor.status.is_empty() {
            tr(self.editor.tool.hint()).to_string()
        } else {
            self.editor.status.clone()
        };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if on_page {
                let pos = match self.editor.hover {
                    Some(p) => format!("x {:>7.1}  y {:>7.1} mm", p.x, p.y),
                    None => "x –  y –".to_string(),
                };
                ui.monospace(pos).on_hover_text(tr("Position of the cursor"));
                self.coordinate_field(ui);
                ui.separator();
                let r = ui.add(egui::Button::new(tr_fmt!("Grid {} mm", self.editor.grid)).frame(false)).on_hover_text(tr("Grid size"));
                egui::Popup::menu(&r).show(|ui| {
                    self.cmd_button(ui, Cmd::Grid);
                    self.cmd_menu(ui, Cmd::GridSize);
                });
                let layer = if self.editor.active_layer.is_empty() { tr("(default)").to_string() } else { self.editor.active_layer.clone() };
                let r = ui.add(egui::Button::new(tr_fmt!("Layer: {}", layer)).frame(false)).on_hover_text(tr("Layer"));
                egui::Popup::menu(&r).show(|ui| self.cmd_menu(ui, Cmd::ActiveLayer));
                let r = ui.add(egui::Button::new(format!("{:.0} %", self.editor.zoom / 3.78 * 100.0)).frame(false)).on_hover_text(tr("100% ≈ real size on a 96 dpi screen"));
                if r.clicked() {
                    self.editor.fit_pending = true;
                }
                ui.separator();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Messages of the session: the newest for a few seconds, all on click.
                let r = icons::button(ui, Icon::List, tr("Messages of this session"));
                egui::Popup::menu(&r).show(|ui| {
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        for m in self.messages.iter().rev() {
                            ui.label(m);
                        }
                        if self.messages.is_empty() {
                            ui.weak(tr("(no entries)"));
                        }
                    });
                });
                if let Some((msg, t)) = &self.message {
                    if ctx.input(|i| i.time) - t < 6.0 {
                        ui.label(RichText::new(msg).color(crate::style::colors(self.dark).ok));
                    }
                }
                ui.separator();
                // Servers: project server, parts server.
                if let Some(sh) = &self.remote.shared {
                    let r = ui.label(RichText::new(tr_fmt!("{} (project server, revision {})", sh.name, sh.revision)).small());
                    if r.on_hover_text(tr("Project server")).clicked() {
                        self.show_pane(Pane::Tool(ToolPane::ProjectServer));
                    }
                }
                // A shared project is saved on the project server (shown before), not in a file.
                let file = match self.session.file_name.as_deref() {
                    Some(f) => io::short_name(f).to_string(),
                    None if self.remote.shared.is_some() => String::new(),
                    None => tr("unsaved").to_string(),
                };
                ui.weak(format!("{}{}", file, if self.session.dirty() { " *" } else { "" }));
                ui.separator();
                let lang = openecraft_i18n::lang();
                let r = ui.add(egui::Button::new(lang.native_name()).frame(false)).on_hover_text(tr("Language"));
                egui::Popup::menu(&r).show(|ui| self.cmd_menu(ui, Cmd::Language));
                if on_page {
                    let pg = &self.session.prj.pages[self.session.page];
                    if pg.locked {
                        icons::show(ui, Icon::Lock, 14.0).on_hover_text(tr("🔒 Released page – read only"));
                    } else if let Some(user) = self.page_locked_by() {
                        icons::show(ui, Icon::Lock, 14.0).on_hover_text(tr_fmt!("🔒 Locked by {} – read only", user));
                    }
                    let n = self.session.analysis.issues.iter().filter(|i| i.severity == Severity::Error && !i.accepted).count();
                    let w = self.session.analysis.issues.iter().filter(|i| i.severity == Severity::Warning && !i.accepted).count();
                    let c = crate::style::colors(self.dark);
                    let r = ui.add(egui::Button::new(RichText::new(tr_fmt!("{} errors, {} warnings", n, w)).color(if n > 0 { c.error } else if w > 0 { c.warning } else { ui.visuals().weak_text_color() })).frame(false));
                    if r.on_hover_text(tr("Results of the project check")).clicked() {
                        self.show_tab(LeftTab::Check);
                    }
                    ui.separator();
                    // Work area: the navigators show its pages and devices.
                    let areas: Vec<String> = self.session.prj.structures.work_areas.iter().map(|w| w.name.clone()).collect();
                    if !areas.is_empty() || !self.work_area.is_empty() {
                        let shown = if self.work_area.is_empty() { tr("(whole project)").to_string() } else { self.work_area.clone() };
                        egui::ComboBox::from_id_salt("status_work_area").selected_text(shown).show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.work_area, String::new(), tr("(whole project)"));
                            for a in areas {
                                ui.selectable_value(&mut self.work_area, a.clone(), a);
                            }
                        });
                        ui.label(tr("Work area"));
                    }
                }
                // Long hints are cut off (the whole text shows on hover).
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add(egui::Label::new(RichText::new(hint).small()).truncate());
                });
            });
        });
    }
}
