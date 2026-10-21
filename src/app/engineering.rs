//! Engineering windows: macros, terminal strips, PLC overview and revisions.

use super::App;
use crate::editor::{EguiDraw, Palette, Tool};
use crate::io::Purpose;
use crate::macros::{self, Macro, MacroKind};
use crate::model::*;
use crate::ops::Clip;
use crate::render::{self, Draw, Ink};
use crate::{plc, revision, strips, symbols};
use egui::{vec2, Color32, RichText};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct EngineeringUi {
    pub show_macros: bool,
    macro_filter: String,
    /// Name of the selected macro.
    macro_sel: Option<String>,
    /// Placeholder values for the next insertion.
    macro_values: BTreeMap<String, String>,
    /// Macro waiting for its name (save dialog).
    saving: Option<Macro>,
    pub show_strips: bool,
    strip: String,
    pub show_plc: bool,
    card: String,
    pub show_revisions: bool,
    rev_name: String,
    rev_date: String,
    rev_author: String,
    rev_description: String,
    rev_release: bool,
}

enum MacroAction {
    Insert(usize),
    Delete(usize),
    Export,
    Import,
}

/// Scaled drawing of a macro's items.
fn clip_preview(ui: &mut egui::Ui, clip: &Clip, size: egui::Vec2) {
    let (resp, painter) = ui.allocate_painter(size, egui::Sense::hover());
    let pal = Palette::new(ui.visuals().dark_mode);
    painter.rect_filled(resp.rect, 4.0, pal.paper);
    let b = clip.bounds().expand(4.0);
    if !b.is_valid() {
        return;
    }
    let zoom = ((size.x - 8.0) / b.width()).min((size.y - 8.0) / b.height()).min(4.0);
    let origin = resp.rect.center() - vec2(b.center().x, b.center().y) * zoom;
    let mut d = EguiDraw { painter: &painter, origin, zoom, pal: &pal, force: None };
    for w in &clip.wires {
        d.line(w.a, w.b, render::WIRE_W, Ink::Wire, false);
    }
    for s in &clip.shapes {
        render::draw_shape(&mut d, s, Ink::Line);
    }
    for s in &clip.symbols {
        if let Some(def) = symbols::get(&s.sym) {
            render::draw_symbol(&mut d, def, s, Ink::Line);
        }
    }
    for t in &clip.texts {
        d.text(t.pos, t.text.lines().next().unwrap_or(""), t.size, symbols::Anchor::Left, render::VAnchor::Middle, Ink::Text);
    }
}

impl App {
    pub(super) fn engineering_windows(&mut self, ctx: &egui::Context) {
        self.save_macro_dialog(ctx);
        self.macros_window(ctx);
        self.strips_window(ctx);
        self.plc_window(ctx);
        self.revisions_window(ctx);
    }

    /// Opens the save dialog for the selection (window macro) or the page (page macro).
    pub(super) fn save_as_macro(&mut self, whole_page: bool) {
        let page = &self.prj.pages[self.page];
        let m = if whole_page {
            macros::from_page(page, &page.description)
        } else {
            macros::from_selection(page, &self.editor.selection, "")
        };
        self.eng.saving = Some(m);
    }

    fn save_macro_dialog(&mut self, ctx: &egui::Context) {
        let Some(m) = &mut self.eng.saving else { return };
        let (mut save, mut cancel) = (false, false);
        egui::Window::new("Save as macro").collapsible(false).resizable(false).show(ctx, |ui| {
            ui.label(format!("{} macro: {}", if m.kind == MacroKind::Page { "Page" } else { "Window" }, m.summary()));
            egui::Grid::new("savemacro").num_columns(2).show(ui, |ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut m.name);
                ui.end_row();
                ui.label("Category");
                ui.add(egui::TextEdit::singleline(&mut m.category).hint_text("e.g. Motors"));
                ui.end_row();
                ui.label("Description");
                ui.text_edit_multiline(&mut m.description);
                ui.end_row();
            });
            ui.weak("Placeholders like {MOTOR} in tags, texts and function texts are filled in when inserting.");
            ui.horizontal(|ui| {
                save = ui.add_enabled(!m.name.trim().is_empty(), egui::Button::new("💾 Save")).clicked();
                cancel = ui.button("Cancel").clicked();
            });
        });
        if save {
            let m = self.eng.saving.take().unwrap();
            let replaced = self.macro_lib.macros.iter().any(|x| x.name == m.name);
            self.eng.macro_sel = Some(m.name.clone());
            self.notify(ctx, format!("Macro {} {}", m.name, if replaced { "replaced" } else { "saved" }));
            self.macro_lib.upsert(m);
            self.eng.show_macros = true;
        }
        if cancel {
            self.eng.saving = None;
        }
    }

    fn macros_window(&mut self, ctx: &egui::Context) {
        if !self.eng.show_macros {
            return;
        }
        let mut open = true;
        let mut action = None;
        let can_save_selection = !self.editor.selection.is_empty() && self.prj.pages[self.page].kind == PageKind::Schematic;
        let mut save = None;
        egui::Window::new("Macros").open(&mut open).default_size([820.0, 480.0]).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.eng.macro_filter).hint_text("Filter…").desired_width(160.0));
                if ui.add_enabled(can_save_selection, egui::Button::new("💾 Save selection as macro…")).clicked() {
                    save = Some(false);
                }
                if ui.button("Save page as macro…").clicked() {
                    save = Some(true);
                }
                ui.menu_button("Library", |ui| {
                    if ui.button("Export macros (JSON)…").clicked() {
                        action = Some(MacroAction::Export);
                    }
                    if ui.button("Import macros…").clicked() {
                        action = Some(MacroAction::Import);
                    }
                });
            });
            ui.separator();
            let h = ui.available_height();
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(vec2(240.0, h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    egui::ScrollArea::vertical().id_salt("macrolist").auto_shrink([false, false]).show(ui, |ui| {
                        let f = self.eng.macro_filter.to_lowercase();
                        let mut categories: Vec<&str> = self.macro_lib.macros.iter().map(|m| m.category.as_str()).collect();
                        categories.dedup();
                        for cat in categories {
                            let title = if cat.is_empty() { "(no category)" } else { cat };
                            egui::CollapsingHeader::new(RichText::new(title).strong()).id_salt(("mcat", cat)).default_open(true).show(ui, |ui| {
                                for m in self.macro_lib.macros.iter().filter(|m| m.category == cat) {
                                    if !f.is_empty() && !m.name.to_lowercase().contains(&f) && !m.description.to_lowercase().contains(&f) {
                                        continue;
                                    }
                                    let icon = if m.kind == MacroKind::Page { "🗋" } else { "☐" };
                                    let sel = self.eng.macro_sel.as_deref() == Some(m.name.as_str());
                                    if ui.selectable_label(sel, format!("{icon} {}", m.name)).on_hover_text(&m.description).clicked() {
                                        self.eng.macro_sel = Some(m.name.clone());
                                        self.eng.macro_values.clear();
                                    }
                                }
                            });
                        }
                        if self.macro_lib.macros.is_empty() {
                            ui.weak("No macros yet. Select items in a schematic and save them as a macro.");
                        }
                    });
                });
                ui.separator();
                ui.vertical(|ui| {
                    let Some(i) = self.eng.macro_sel.as_ref().and_then(|n| self.macro_lib.macros.iter().position(|m| &m.name == n)) else {
                        ui.weak("Select a macro.");
                        return;
                    };
                    let values = &mut self.eng.macro_values;
                    let m = &mut self.macro_lib.macros[i];
                    egui::ScrollArea::vertical().id_salt("macrodetail").auto_shrink([false, false]).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.heading(&m.name);
                            ui.weak(if m.kind == MacroKind::Page { "page macro" } else { "window macro" });
                        });
                        egui::Grid::new("macroprops").num_columns(2).show(ui, |ui| {
                            ui.label("Category");
                            ui.text_edit_singleline(&mut m.category);
                            ui.end_row();
                            ui.label("Description");
                            ui.text_edit_multiline(&mut m.description);
                            ui.end_row();
                            if m.kind == MacroKind::Page {
                                ui.label("Page description");
                                ui.text_edit_singleline(&mut m.page_description);
                                ui.end_row();
                            }
                        });
                        ui.weak(m.summary());
                        clip_preview(ui, &m.clip, vec2(420.0, 200.0));
                        let placeholders = m.placeholders();
                        if !placeholders.is_empty() {
                            ui.separator();
                            ui.label(RichText::new("Value sets").strong());
                            let mut remove = None;
                            egui::Grid::new("valuesets").striped(true).num_columns(placeholders.len() + 2).show(ui, |ui| {
                                ui.label(RichText::new("Name").strong());
                                for ph in &placeholders {
                                    ui.label(RichText::new(format!("{{{ph}}}")).strong());
                                }
                                ui.label("");
                                ui.end_row();
                                for (vi, vs) in m.value_sets.iter_mut().enumerate() {
                                    ui.add_sized([110.0, 20.0], egui::TextEdit::singleline(&mut vs.name));
                                    for ph in &placeholders {
                                        ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(vs.values.entry(ph.clone()).or_default()));
                                    }
                                    if ui.small_button("🗑").clicked() {
                                        remove = Some(vi);
                                    }
                                    ui.end_row();
                                }
                            });
                            if let Some(vi) = remove {
                                m.value_sets.remove(vi);
                            }
                            if ui.button("➕ Value set").clicked() {
                                let name = format!("Set {}", m.value_sets.len() + 1);
                                m.value_sets.push(macros::ValueSet { name, values: values.clone() });
                            }
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Insert with").strong());
                                egui::ComboBox::from_id_salt("useset").selected_text("value set…").show_ui(ui, |ui| {
                                    for vs in &m.value_sets {
                                        if ui.button(&vs.name).clicked() {
                                            *values = vs.values.clone();
                                        }
                                    }
                                });
                            });
                            egui::Grid::new("insertvalues").num_columns(2).show(ui, |ui| {
                                for ph in &placeholders {
                                    ui.label(format!("{{{ph}}}"));
                                    ui.text_edit_singleline(values.entry(ph.clone()).or_default());
                                    ui.end_row();
                                }
                            });
                        }
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            let label = if m.kind == MacroKind::Page { "➕ Insert as new page" } else { "➕ Insert (click into the page)" };
                            if ui.button(label).clicked() {
                                action = Some(MacroAction::Insert(i));
                            }
                            if ui.button("🗑 Delete macro").clicked() {
                                action = Some(MacroAction::Delete(i));
                            }
                        });
                    });
                });
            });
        });
        self.eng.show_macros = open;
        if let Some(whole_page) = save {
            self.save_as_macro(whole_page);
        }
        match action {
            Some(MacroAction::Insert(i)) => {
                let m = self.macro_lib.macros[i].clone();
                if m.kind == MacroKind::Page {
                    let at = macros::insert_page(&mut self.prj, self.page, &m, &self.eng.macro_values);
                    self.goto(at, 0);
                    self.notify(ctx, format!("Page macro {} inserted as page {}", m.name, self.prj.pages[at].name));
                } else {
                    self.editor.insert = Some(m.resolve(&self.eng.macro_values));
                    self.editor.set_tool(Tool::Insert);
                    self.workspace = super::Workspace::Schematic;
                }
            }
            Some(MacroAction::Delete(i)) => {
                self.macro_lib.macros.remove(i);
                self.eng.macro_sel = None;
            }
            Some(MacroAction::Export) => {
                let json = serde_json::to_string_pretty(&self.macro_lib).unwrap_or_default();
                self.export(ctx, "macros.json".into(), ("JSON", "json"), json);
            }
            Some(MacroAction::Import) => self.io.open(ctx, Purpose::Macros),
            None => {}
        }
    }

    fn strips_window(&mut self, ctx: &egui::Context) {
        if !self.eng.show_strips {
            return;
        }
        let tags = strips::strip_tags(&self.prj);
        if !tags.contains(&self.eng.strip) {
            self.eng.strip = tags.first().cloned().unwrap_or_default();
        }
        let strip = self.eng.strip.clone();
        let rows = strips::rows(&self.prj, &strip, &self.analysis.netlist, &self.analysis.open);
        let mut data = self.prj.strips.get(&strip).cloned().unwrap_or_default();
        let before = data.clone();
        let mut open = true;
        let (mut renumber, mut sort) = (false, false);
        let mut moved = None;
        let mut relabel: Vec<(usize, usize, String, String)> = Vec::new();
        let mut jump = None;
        egui::Window::new("Terminal strips").open(&mut open).default_size([900.0, 420.0]).show(ctx, |ui| {
            if tags.is_empty() {
                ui.weak("No terminal strips: place terminals in a schematic first.");
                return;
            }
            ui.horizontal_wrapped(|ui| {
                ui.label("Strip");
                egui::ComboBox::from_id_salt("strip").selected_text(&strip).show_ui(ui, |ui| {
                    for t in &tags {
                        ui.selectable_value(&mut self.eng.strip, t.clone(), t);
                    }
                });
                ui.separator();
                ui.label("Spare terminals");
                ui.add(egui::DragValue::new(&mut data.spares).range(0..=500));
                ui.separator();
                renumber = ui.button("Renumber 1 … n").on_hover_text("In strip order; PE and other non-numeric terminals keep their designation").clicked();
                sort = ui.button("Sort by designation").clicked();
            });
            ui.weak("Terminals with the same jumper number are bridged (same potential). Spare terminals are counted in the bill of materials.");
            ui.separator();
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                egui::Grid::new("strip").striped(true).num_columns(8).spacing([10.0, 4.0]).show(ui, |ui| {
                    for h in ["", "Terminal", "Jumper", "External (top)", "Internal (bottom)", "Potential", "Function text", "Location"] {
                        ui.label(RichText::new(h).strong());
                    }
                    ui.end_row();
                    let placed = rows.iter().filter(|r| r.terminal.is_some()).count();
                    for (i, r) in rows.iter().enumerate() {
                        let Some(t) = &r.terminal else {
                            ui.label("");
                            ui.weak(&r.label);
                            ui.label("");
                            ui.weak("spare");
                            ui.end_row();
                            continue;
                        };
                        ui.horizontal(|ui| {
                            if ui.add_enabled(i > 0, egui::Button::new("⏶").small()).clicked() {
                                moved = Some((i, true));
                            }
                            if ui.add_enabled(i + 1 < placed, egui::Button::new("⏷").small()).clicked() {
                                moved = Some((i, false));
                            }
                        });
                        let mut label = r.label.clone();
                        ui.add_sized([50.0, 20.0], egui::TextEdit::singleline(&mut label));
                        if label != r.label {
                            relabel.push((t.page, t.sym, r.label.clone(), label));
                        }
                        let mut j = data.jumpers.get(&r.label).copied().unwrap_or(0);
                        if ui.add(egui::DragValue::new(&mut j).range(0..=99)).on_hover_text("0 = no jumper").changed() {
                            if j == 0 {
                                data.jumpers.remove(&r.label);
                            } else {
                                data.jumpers.insert(r.label.clone(), j);
                            }
                        }
                        ui.label(&r.external);
                        ui.label(&r.internal);
                        ui.label(&r.potential);
                        ui.label(&r.function_text);
                        if ui.link(&r.location).clicked() {
                            jump = Some((t.page, t.id));
                        }
                        ui.end_row();
                    }
                });
            });
        });
        self.eng.show_strips = open;
        if data != before {
            self.prj.strips.insert(strip.clone(), data);
        }
        for (page, sym, old, new) in relabel {
            self.prj.pages[page].symbols[sym].label = new.clone();
            if let Some(d) = self.prj.strips.get_mut(&strip) {
                if let Some(j) = d.jumpers.remove(&old) {
                    d.jumpers.insert(new.clone(), j);
                }
                for o in d.order.iter_mut().filter(|o| **o == old) {
                    *o = new.clone();
                }
            }
        }
        if let Some((i, up)) = moved {
            strips::move_terminal(&mut self.prj, &strip, i, up);
        }
        if sort {
            if let Some(d) = self.prj.strips.get_mut(&strip) {
                d.order.clear();
            }
        }
        if renumber {
            let n = strips::renumber(&mut self.prj, &strip);
            self.notify(ctx, format!("{strip}: {n} terminals renumbered"));
        }
        if let Some((pg, id)) = jump {
            self.goto(pg, id);
        }
    }

    fn plc_window(&mut self, ctx: &egui::Context) {
        if !self.eng.show_plc {
            return;
        }
        let cards = plc::cards(&self.prj);
        if !self.eng.card.is_empty() && !cards.contains(&self.eng.card) {
            self.eng.card.clear();
        }
        let card = self.eng.card.clone();
        let points: Vec<plc::Point> =
            plc::points(&self.prj, &self.analysis.netlist).into_iter().filter(|p| card.is_empty() || p.card == card).collect();
        let mut data = self.prj.plc_cards.get(&card).cloned().unwrap_or_default();
        let before = data.clone();
        let mut open = true;
        let (mut readdress, mut export, mut import) = (false, false, false);
        let mut edits: Vec<(usize, usize, Option<String>, Option<String>)> = Vec::new();
        let mut jump = None;
        egui::Window::new("PLC overview").open(&mut open).default_size([900.0, 420.0]).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Card");
                egui::ComboBox::from_id_salt("plccard").selected_text(if card.is_empty() { "all cards" } else { &card }).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.eng.card, String::new(), "all cards");
                    for c in &cards {
                        ui.selectable_value(&mut self.eng.card, c.clone(), c);
                    }
                });
                ui.separator();
                export = ui.button("⬇ Export symbol table (CSV)…").on_hover_text("Name; Data Type; Logical Address; Comment – for PLC programming tools").clicked();
                import = ui.button("⬆ Import symbol table…").on_hover_text("Takes the comments as function texts (matched by address)").clicked();
            });
            if !card.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Start byte");
                    ui.add(egui::DragValue::new(&mut data.start_byte).range(0..=65535));
                    ui.label("Rack");
                    ui.add(egui::TextEdit::singleline(&mut data.rack).desired_width(40.0));
                    ui.label("Slot");
                    ui.add(egui::TextEdit::singleline(&mut data.slot).desired_width(40.0));
                    ui.label("Description");
                    ui.add(egui::TextEdit::singleline(&mut data.description).desired_width(180.0));
                    readdress = ui.button("Readdress card").on_hover_text("Consecutive addresses from the start byte, in schematic order").clicked();
                });
            }
            ui.separator();
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                egui::Grid::new("plcpoints").striped(true).num_columns(6).spacing([10.0, 4.0]).show(ui, |ui| {
                    for h in ["Card", "Address", "Type", "Function text", "Connected to", "Location"] {
                        ui.label(RichText::new(h).strong());
                    }
                    ui.end_row();
                    for p in &points {
                        ui.label(&p.card);
                        let mut addr = p.address.clone();
                        ui.add_sized([60.0, 20.0], egui::TextEdit::singleline(&mut addr));
                        ui.label(if p.input { "DI" } else { "DO" });
                        let mut text = p.function_text.clone();
                        ui.add_sized([200.0, 20.0], egui::TextEdit::singleline(&mut text));
                        if addr != p.address || text != p.function_text {
                            edits.push((p.page, p.sym, (addr != p.address).then_some(addr), (text != p.function_text).then_some(text)));
                        }
                        ui.label(&p.connected);
                        if ui.link(&p.location).clicked() {
                            jump = Some((p.page, p.id));
                        }
                        ui.end_row();
                    }
                });
                if points.is_empty() {
                    ui.weak("No PLC inputs / outputs placed.");
                }
            });
        });
        self.eng.show_plc = open;
        if data != before {
            self.prj.plc_cards.insert(card.clone(), data);
        }
        for (page, sym, addr, text) in edits {
            let s = &mut self.prj.pages[page].symbols[sym];
            if let Some(a) = addr {
                s.label = a;
            }
            if let Some(t) = text {
                s.function_text = t;
            }
        }
        if readdress {
            let n = plc::readdress(&mut self.prj, &card);
            self.notify(ctx, format!("{card}: {n} addresses changed"));
        }
        if export {
            let csv = plc::symbol_table(&self.prj, &self.analysis.netlist);
            self.export(ctx, "plc_symbols.csv".into(), ("CSV", "csv"), csv);
        }
        if import {
            self.io.open(ctx, Purpose::PlcTable);
        }
        if let Some((pg, id)) = jump {
            self.goto(pg, id);
        }
    }

    fn revisions_window(&mut self, ctx: &egui::Context) {
        if !self.eng.show_revisions {
            return;
        }
        let mut open = true;
        let mut create = false;
        let mut lock = None;
        let mut jump = None;
        let mut overview = false;
        if self.eng.rev_name.is_empty() {
            self.eng.rev_name = revision::next_name(&self.prj);
            self.eng.rev_date = self.prj.info.date.clone();
            self.eng.rev_author = self.prj.info.creator.clone();
        }
        let locked = self.prj.pages.iter().filter(|p| p.locked).count();
        egui::Window::new("Revisions").open(&mut open).default_size([640.0, 480.0]).show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.label(RichText::new("Revisions").strong());
                egui::Grid::new("revs").striped(true).num_columns(4).show(ui, |ui| {
                    for h in ["Index", "Date", "Author", "Description"] {
                        ui.label(RichText::new(h).strong());
                    }
                    ui.end_row();
                    for r in &self.prj.revisions {
                        ui.label(&r.name);
                        ui.label(&r.date);
                        ui.label(&r.author);
                        ui.label(&r.description);
                        ui.end_row();
                    }
                });
                if self.prj.revisions.is_empty() {
                    ui.weak("No revision yet. Changes are tracked from the first revision on.");
                }
                ui.separator();
                ui.label(RichText::new("New revision").strong());
                egui::Grid::new("newrev").num_columns(2).show(ui, |ui| {
                    ui.label("Index");
                    ui.add(egui::TextEdit::singleline(&mut self.eng.rev_name).desired_width(60.0));
                    ui.end_row();
                    ui.label("Date");
                    ui.add(egui::TextEdit::singleline(&mut self.eng.rev_date).hint_text("YYYY-MM-DD").desired_width(100.0));
                    ui.end_row();
                    ui.label("Author");
                    ui.text_edit_singleline(&mut self.eng.rev_author);
                    ui.end_row();
                    ui.label("Description");
                    ui.text_edit_multiline(&mut self.eng.rev_description);
                    ui.end_row();
                });
                ui.checkbox(&mut self.eng.rev_release, "Release: lock all pages");
                create = ui.add_enabled(!self.eng.rev_name.trim().is_empty(), egui::Button::new("Create revision")).clicked();
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("{locked} of {} pages locked", self.prj.pages.len()));
                    if ui.button("Lock all").clicked() {
                        lock = Some(true);
                    }
                    if ui.button("Unlock all").clicked() {
                        lock = Some(false);
                    }
                    let pg = &mut self.prj.pages[self.page];
                    ui.checkbox(&mut pg.locked, format!("Page {} locked", pg.name));
                });
                ui.separator();
                ui.horizontal(|ui| {
                    let since = self.prj.revisions.last().map(|r| r.name.clone()).unwrap_or_default();
                    ui.label(RichText::new(if since.is_empty() { "Changes".to_string() } else { format!("Changes since revision {since}") }).strong());
                    ui.checkbox(&mut self.editor.show_revision_marks, "Revision marks");
                    overview = ui.button("📊 Revision overview").clicked();
                });
                for pc in &self.analysis.changes {
                    let title = match pc.page {
                        Some(_) => format!("Page {} ({})", pc.name, pc.changes.len()),
                        None => format!("Page {} (deleted)", pc.name),
                    };
                    egui::CollapsingHeader::new(title).id_salt(("chg", pc.page_id)).show(ui, |ui| {
                        for c in &pc.changes {
                            let text = format!("{} {} {}", c.what, c.kind.title(), c.label);
                            match pc.page {
                                Some(pi) if c.kind != revision::ChangeKind::Removed => {
                                    if ui.link(text.trim()).clicked() {
                                        jump = Some((pi, if c.what == "Page" { 0 } else { c.id }));
                                    }
                                }
                                _ => {
                                    ui.label(RichText::new(text.trim()).color(Color32::from_rgb(200, 80, 80)));
                                }
                            }
                        }
                    });
                }
                if self.analysis.changes.is_empty() && !self.prj.revisions.is_empty() {
                    ui.weak("No changes since the last revision.");
                }
            });
        });
        self.eng.show_revisions = open;
        if create {
            let e = &self.eng;
            revision::create(&mut self.prj, e.rev_name.trim(), &e.rev_date, &e.rev_author, &e.rev_description, e.rev_release);
            self.notify(ctx, format!("Revision {} created", self.eng.rev_name.trim()));
            self.eng.rev_name.clear();
            self.eng.rev_description.clear();
        }
        if let Some(l) = lock {
            for pg in &mut self.prj.pages {
                pg.locked = l;
            }
        }
        if let Some((pg, id)) = jump {
            self.goto(pg, id);
        }
        if overview {
            self.report_kind = ReportKind::RevisionOverview;
            self.show_reports = true;
        }
    }
}
