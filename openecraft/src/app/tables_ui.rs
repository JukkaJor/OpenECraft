//! Table editing window: devices, connections and terminals in a table with a filter,
//! cells edited in place, fill / find and replace for the filtered rows and the CSV
//! round trip through a spreadsheet.

use openecraft_ui::icons::{self, Icon};
use openecraft_i18n::{tr, tr_fmt};
use super::App;
use crate::io::Purpose;
use openecraft_core::tables::{self, TableKind};
use egui::RichText;
use std::collections::HashMap;

pub struct TablesUi {
    pub open: bool,
    kind: TableKind,
    filter: String,
    /// Cells being edited: (row key, column) → text.
    buffer: HashMap<(String, usize), String>,
    column: usize,
    find: String,
    replace: String,
}

impl Default for TablesUi {
    fn default() -> Self {
        TablesUi { open: false, kind: TableKind::Devices, filter: String::new(), buffer: HashMap::new(), column: 0, find: String::new(), replace: String::new() }
    }
}

enum Action {
    Set(String, usize, String),
    Replace(bool),
    Export,
    ExportXlsx,
    Import,
    Jump(usize, u64),
}

impl TablesUi {
    /// Shows a kind of rows (from a navigator), filtered.
    pub fn show(&mut self, kind: TableKind, filter: &str) {
        if self.kind != kind {
            self.buffer.clear();
        }
        self.kind = kind;
        self.filter = filter.to_string();
    }
}

impl App {
    pub(super) fn tables_pane(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
        let kind = self.tables_ui.kind;
        let all = tables::rows(&self.session.prj, &self.session.analysis.netlist, &self.session.analysis.open, kind);
        let rows: Vec<&tables::Row> = all.iter().filter(|r| tables::matches(r, &self.tables_ui.filter)).collect();
        let columns = kind.columns();
        let editable: Vec<usize> = (0..columns.len()).filter(|&c| columns[c].editable).collect();
        if !editable.contains(&self.tables_ui.column) {
            self.tables_ui.column = editable[0];
        }
        let mut actions: Vec<Action> = Vec::new();
        let ui_state = &mut self.tables_ui;
        ui.scope(|ui| {
            ui.horizontal(|ui| {
                for k in TableKind::ALL {
                    if ui.selectable_label(ui_state.kind == k, k.title()).clicked() && ui_state.kind != k {
                        ui_state.kind = k;
                        ui_state.buffer.clear();
                    }
                }
                ui.separator();
                ui.label(tr("Filter"));
                ui.add(egui::TextEdit::singleline(&mut ui_state.filter).desired_width(140.0));
                ui.label(tr_fmt!("{} of {} rows", rows.len(), all.len()));
                ui.separator();
                if ui.button(tr("Export CSV…")).clicked() {
                    actions.push(Action::Export);
                }
                if ui.button(tr("Export Excel…")).clicked() {
                    actions.push(Action::ExportXlsx);
                }
                if ui.button(tr("Import CSV / Excel…")).on_hover_text(tr("Changed cells of a CSV or Excel file exported here (rows by their key)")).clicked() {
                    actions.push(Action::Import);
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Column"));
                egui::ComboBox::from_id_salt("tblcol").selected_text(tr(columns[ui_state.column].name)).show_ui(ui, |ui| {
                    for &c in &editable {
                        ui.selectable_value(&mut ui_state.column, c, tr(columns[c].name));
                    }
                });
                ui.label(tr("Find"));
                ui.add(egui::TextEdit::singleline(&mut ui_state.find).desired_width(110.0));
                ui.label(tr("Replace with"));
                ui.add(egui::TextEdit::singleline(&mut ui_state.replace).desired_width(110.0));
                if ui.add_enabled(!ui_state.find.is_empty(), egui::Button::new(tr("Replace"))).on_hover_text(tr("In the filtered rows")).clicked() {
                    actions.push(Action::Replace(false));
                }
                if ui.button(tr("Fill")).on_hover_text(tr("Sets the column of all filtered rows to the replacement")).clicked() {
                    actions.push(Action::Replace(true));
                }
            });
            ui.separator();
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                egui::Grid::new(("tbl", kind.key())).striped(true).num_columns(columns.len() + 1).spacing([8.0, 3.0]).show(ui, |ui| {
                    ui.label("");
                    for c in columns {
                        ui.label(RichText::new(tr(c.name)).strong());
                    }
                    ui.end_row();
                    for r in &rows {
                        match r.target {
                            Some((page, id)) => {
                                if icons::button(ui, Icon::Next, tr("Show in the schematic")).clicked() {
                                    actions.push(Action::Jump(page, id));
                                }
                            }
                            None => {
                                ui.label("");
                            }
                        }
                        for (c, cell) in r.cells.iter().enumerate() {
                            if !columns[c].editable || r.locked {
                                ui.label(cell.replace('\n', " "));
                                continue;
                            }
                            let id = (r.key.clone(), c);
                            let mut text = ui_state.buffer.get(&id).cloned().unwrap_or_else(|| cell.replace('\n', "\\n"));
                            let width = if c == 0 { 90.0 } else { 130.0 };
                            let resp = ui.add(egui::TextEdit::singleline(&mut text).desired_width(width));
                            if resp.changed() {
                                ui_state.buffer.insert(id.clone(), text.clone());
                            }
                            if resp.lost_focus() {
                                if let Some(t) = ui_state.buffer.remove(&id) {
                                    if t != cell.replace('\n', "\\n") {
                                        actions.push(Action::Set(r.key.clone(), c, t));
                                    }
                                }
                            }
                        }
                        ui.end_row();
                    }
                });
            });
            ui.weak(tr("Rows on released pages cannot be changed. A line break in a text is written as \\n."));
        });
        let mut problems: Vec<String> = Vec::new();
        for a in actions {
            match a {
                Action::Set(key, c, value) => {
                    if let Err(e) = tables::set_cell(&mut self.session.prj, &self.session.analysis.open, kind, &key, c, &value) {
                        problems.push(e);
                    }
                }
                Action::Replace(fill) => {
                    let visible: Vec<tables::Row> = rows.iter().map(|r| (*r).clone()).filter(|r| !r.locked).collect();
                    let find = if fill { String::new() } else { self.tables_ui.find.clone() };
                    let (n, p) = tables::replace(&mut self.session.prj, &self.session.analysis.open, kind, &visible, self.tables_ui.column, &find, &self.tables_ui.replace);
                    problems.extend(p);
                    self.notify(ctx, tr_fmt!("{} cells changed", n));
                }
                Action::Export => {
                    let csv = tables::to_csv(kind, &all);
                    self.export(ctx, format!("{}.csv", kind.key()), ("CSV", "csv"), csv);
                }
                Action::ExportXlsx => {
                    let rows = openecraft_core::xlsx::rows_of_csv(&tables::to_csv(kind, &all));
                    let bytes = openecraft_core::xlsx::write(&[(kind.title().to_string(), rows)]);
                    self.export_bytes(ctx, format!("{}.xlsx", kind.key()), ("Excel", "xlsx"), &bytes);
                }
                Action::Import => self.io.open(ctx, Purpose::Table(kind)),
                Action::Jump(page, id) => self.goto(page, id),
            }
        }
        if let Some(p) = problems.first() {
            self.notify(ctx, if problems.len() > 1 { tr_fmt!("{} (and {} more)", p, problems.len() - 1) } else { p.clone() });
        }
    }

    /// Applies a CSV file to the table it was exported from.
    pub(super) fn import_table(&mut self, ctx: &egui::Context, kind: TableKind, text: &str) {
        match tables::apply_csv(&mut self.session.prj, &self.session.analysis.netlist, &self.session.analysis.open, kind, text) {
            Ok((n, problems)) if problems.is_empty() => self.notify(ctx, tr_fmt!("{} cells changed", n)),
            Ok((n, problems)) => self.notify(ctx, tr_fmt!("{} cells changed, {} problems: {}", n, problems.len(), problems.join("; "))),
            Err(e) => self.notify_error(ctx, tr_fmt!("Table import failed: {}", e)),
        }
    }
}
