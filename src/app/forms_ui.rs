//! Forms window: title block editor (lines, rectangles, texts with placeholders)
//! and report forms (columns, order, rows per page).

use openecraft_i18n::{tr, tr_fmt};
use super::App;
use crate::editor::{EguiDraw, Palette};
use crate::forms::{self, Form, FormItem, FormLibrary, ReportForm};
use crate::geom::{p, P};
use crate::io::Purpose;
use crate::model::{ReportKind, FRAME_X0, TITLE_Y0};
use crate::render::Ink;
use crate::symbols::Anchor;
use egui::{vec2, Color32, PointerButton, RichText, Sense, Stroke};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    TitleBlocks,
    Reports,
}

#[derive(Default)]
pub struct FormsUi {
    pub open: bool,
    tab: Tab,
    /// Name of the edited title block.
    form: String,
    item: Option<usize>,
    drag: Option<P>,
    report: Option<ReportKind>,
    /// Edited report scheme; empty: the report's form.
    scheme: String,
}

impl FormsUi {
    pub fn busy(&self) -> bool {
        self.drag.is_some()
    }

    /// Opens the report form of a report.
    pub fn open_report(&mut self, kind: ReportKind) {
        self.open = true;
        self.tab = Tab::Reports;
        self.report = Some(kind);
    }
}

enum Action {
    Import,
    Export,
}

impl App {
    pub(super) fn forms_window(&mut self, ctx: &egui::Context) {
        if !self.forms_ui.open {
            return;
        }
        let mut open = true;
        let mut action = None;
        egui::Window::new(tr("Forms – title blocks and reports")).open(&mut open).default_size([1150.0, 560.0]).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.forms_ui.tab, Tab::TitleBlocks, RichText::new(tr("Title blocks")).strong());
                ui.selectable_value(&mut self.forms_ui.tab, Tab::Reports, RichText::new(tr("Report forms")).strong());
            });
            ui.separator();
            match self.forms_ui.tab {
                Tab::TitleBlocks => action = self.title_block_tab(ui),
                Tab::Reports => action = self.report_form_tab(ui),
            }
        });
        self.forms_ui.open = open;
        match action {
            Some(Action::Import) => self.io.open(ctx, Purpose::Forms),
            Some(Action::Export) => {
                let lib = FormLibrary { title_blocks: self.prj.forms.title_blocks.clone(), schemes: self.prj.forms.schemes.clone() };
                let json = serde_json::to_string_pretty(&lib).unwrap_or_default();
                self.export(ctx, "title_blocks.json".into(), (tr("OpenECraft title blocks"), "json"), json);
            }
            None => {}
        }
    }

    fn title_block_tab(&mut self, ui: &mut egui::Ui) -> Option<Action> {
        let mut action = None;
        let names = forms::names(&self.prj);
        let fu = &mut self.forms_ui;
        let prj = &mut self.prj;
        if !names.contains(&fu.form) {
            fu.form = if prj.forms.default_title_block.is_empty() { forms::STANDARD.into() } else { prj.forms.default_title_block.clone() };
            fu.item = None;
        }
        let custom = prj.forms.title_blocks.iter().position(|f| f.name == fu.form);
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Title block"));
            egui::ComboBox::from_id_salt("tbsel").selected_text(&fu.form).width(180.0).show_ui(ui, |ui| {
                for n in &names {
                    let tag = if prj.forms.title_blocks.iter().any(|f| &f.name == n) { "" } else { "  (built-in)" };
                    if ui.selectable_label(&fu.form == n, format!("{n}{tag}")).clicked() {
                        fu.form = n.clone();
                        fu.item = None;
                    }
                }
            });
            if ui.button(tr("Copy as new")).on_hover_text(tr("Editable copy of this title block")).clicked() {
                let mut f = forms::find(prj, &fu.form).into_owned();
                let mut name = format!("{} (copy)", f.name);
                while names.contains(&name) {
                    name.push('+');
                }
                f.name = name.clone();
                prj.forms.title_blocks.push(f);
                fu.form = name;
                fu.item = None;
            }
            if ui.button(tr("New empty")).clicked() {
                let mut name = "My title block".to_string();
                while names.contains(&name) {
                    name.push('+');
                }
                let line = FormItem::Line { a: p(0.0, 0.0), b: p(forms::AREA_W, 0.0), width: 0.5 };
                prj.forms.title_blocks.push(Form { name: name.clone(), items: vec![line] });
                fu.form = name;
                fu.item = None;
            }
            let is_default = prj.forms.default_title_block == fu.form || (prj.forms.default_title_block.is_empty() && fu.form == forms::STANDARD);
            if ui.add_enabled(!is_default, egui::Button::new(tr("Use for all pages"))).on_hover_text(tr("Project default; pages can choose their own in the page properties")).clicked() {
                prj.forms.default_title_block = if fu.form == forms::STANDARD { String::new() } else { fu.form.clone() };
            }
            if let Some(ci) = custom {
                if ui.button(tr("🗑 Delete")).on_hover_text(tr("Pages using it get the project default")).clicked() {
                    let name = prj.forms.title_blocks.remove(ci).name;
                    for pg in prj.pages.iter_mut().filter(|pg| pg.title_block == name) {
                        pg.title_block.clear();
                    }
                    if prj.forms.default_title_block == name {
                        prj.forms.default_title_block.clear();
                    }
                    fu.item = None;
                    return;
                }
            }
            ui.separator();
            if ui.button(tr("⬆ Import…")).clicked() {
                action = Some(Action::Import);
            }
            if ui.add_enabled(!prj.forms.title_blocks.is_empty(), egui::Button::new(tr("⬇ Export…"))).on_hover_text(tr("The project's own title blocks")).clicked() {
                action = Some(Action::Export);
            }
        });
        let used = (0..prj.pages.len()).filter(|&i| forms::title_block(prj, i).name == fu.form).count();
        ui.weak(tr_fmt!("Used on {} of {} pages", used, prj.pages.len()));
        let custom = prj.forms.title_blocks.iter().position(|f| f.name == fu.form);
        let page = self.page.min(prj.pages.len().saturating_sub(1));
        match custom {
            None => {
                ui.colored_label(Color32::from_rgb(220, 140, 0), tr("Built-in title blocks are read only – use “Copy as new” to make your own."));
                let mut form = forms::find(prj, &fu.form).into_owned();
                preview(ui, fu, prj, page, &mut form, false);
            }
            Some(ci) => {
                let mut form = prj.forms.title_blocks[ci].clone();
                let old_name = form.name.clone();
                ui.horizontal(|ui| {
                    ui.label(tr("Name"));
                    ui.text_edit_singleline(&mut form.name);
                });
                preview(ui, fu, prj, page, &mut form, true);
                ui.separator();
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(380.0);
                        ui.horizontal_wrapped(|ui| {
                            let mid = p(forms::AREA_W / 2.0, forms::AREA_H / 2.0);
                            let new = [
                                (tr("➕ Line"), FormItem::Line { a: mid - p(20.0, 0.0), b: mid + p(20.0, 0.0), width: 0.25 }),
                                (tr("➕ Rectangle"), FormItem::Rect { a: mid - p(20.0, 8.0), b: mid + p(20.0, 8.0), width: 0.25, fill: false }),
                                (tr("➕ Text"), FormItem::Text { pos: mid, text: "{project}".into(), size: 2.5, anchor: Anchor::Left, label: false, max_width: 0.0 }),
                                (tr("➕ Label"), FormItem::Text { pos: mid, text: "Label".into(), size: 1.6, anchor: Anchor::Left, label: true, max_width: 0.0 }),
                            ];
                            for (label, item) in new {
                                if ui.button(label).clicked() {
                                    form.items.push(item);
                                    fu.item = Some(form.items.len() - 1);
                                }
                            }
                        });
                        egui::ScrollArea::vertical().id_salt("tbitems").max_height(220.0).show(ui, |ui| {
                            for (i, it) in form.items.iter().enumerate() {
                                let summary = match it {
                                    FormItem::Text { text, .. } => format!("{}: {text}", it.kind_name()),
                                    FormItem::Line { a, b, .. } | FormItem::Rect { a, b, .. } => format!("{}: {:.1},{:.1} – {:.1},{:.1}", it.kind_name(), a.x, a.y, b.x, b.y),
                                };
                                if ui.selectable_label(fu.item == Some(i), summary).clicked() {
                                    fu.item = Some(i);
                                }
                            }
                        });
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.set_width(330.0);
                        item_properties(ui, fu, &mut form);
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label(RichText::new(tr("Placeholders")).strong());
                        ui.weak(tr("Click to add to the selected text."));
                        egui::ScrollArea::vertical().id_salt("tbph").max_height(240.0).show(ui, |ui| {
                            for (ph, meaning) in forms::PLACEHOLDERS {
                                ui.horizontal(|ui| {
                                    if ui.link(RichText::new(ph).monospace()).clicked() {
                                        if let Some(FormItem::Text { text, .. }) = fu.item.and_then(|i| form.items.get_mut(i)) {
                                            text.push_str(ph);
                                        }
                                    }
                                    ui.weak(tr(meaning));
                                });
                            }
                        });
                    });
                });
                // Renaming keeps the pages that use the title block.
                if form.name != old_name && !form.name.trim().is_empty() && !names.contains(&form.name) {
                    for pg in prj.pages.iter_mut().filter(|pg| pg.title_block == old_name) {
                        pg.title_block = form.name.clone();
                    }
                    if prj.forms.default_title_block == old_name {
                        prj.forms.default_title_block = form.name.clone();
                    }
                    fu.form = form.name.clone();
                } else {
                    form.name = old_name;
                }
                if prj.forms.title_blocks[ci] != form {
                    prj.forms.title_blocks[ci] = form;
                }
            }
        }
        action
    }

    fn report_form_tab(&mut self, ui: &mut egui::Ui) -> Option<Action> {
        let mut action = None;
        let fu = &mut self.forms_ui;
        let kind = *fu.report.get_or_insert(ReportKind::DeviceList);
        let schemes = forms::scheme_names(&self.prj, kind);
        if !schemes.contains(&fu.scheme) {
            fu.scheme.clear();
        }
        let (mut new_scheme, mut delete_scheme) = (false, false);
        ui.horizontal(|ui| {
            ui.label(tr("Report"));
            egui::ComboBox::from_id_salt("rfkind").selected_text(kind.title()).width(240.0).show_ui(ui, |ui| {
                for k in ReportKind::ALL {
                    ui.selectable_value(&mut fu.report, Some(k), k.title());
                }
            });
            ui.label(tr("Scheme"));
            let standard = tr("Standard form of the report");
            let current = if fu.scheme.is_empty() { standard.to_string() } else { fu.scheme.clone() };
            egui::ComboBox::from_id_salt("rfscheme").selected_text(current).width(220.0).show_ui(ui, |ui| {
                ui.selectable_value(&mut fu.scheme, String::new(), standard);
                for s in &schemes {
                    ui.selectable_value(&mut fu.scheme, s.clone(), s);
                }
            });
            new_scheme = ui.button(tr("➕ New scheme")).on_hover_text(tr("Named copy of this form, for report blocks")).clicked();
            delete_scheme = ui.add_enabled(!fu.scheme.is_empty(), egui::Button::new(tr("🗑 Delete"))).clicked();
            ui.separator();
            if ui.button(tr("⬆ Import…")).clicked() {
                action = Some(Action::Import);
            }
            if ui.button(tr("⬇ Export…")).on_hover_text(tr("The project's own title blocks and report schemes")).clicked() {
                action = Some(Action::Export);
            }
        });
        if fu.report != Some(kind) {
            fu.scheme.clear();
            return action;
        }
        let Some(table) = self.analysis.reports.get(&kind) else { return action };
        let headers: Vec<&str> = table.headers.clone();
        let stored = if fu.scheme.is_empty() {
            self.prj.forms.reports.get(&kind).cloned()
        } else {
            self.prj.forms.schemes.iter().find(|s| s.kind == kind && s.name == fu.scheme).map(|s| s.form.clone())
        };
        let stored = stored.unwrap_or_default();
        let mut form = stored.clone();
        if new_scheme {
            let name = (1..).map(|i| tr_fmt!("Scheme {}", i)).find(|n| !schemes.contains(n)).unwrap_or_default();
            self.prj.forms.schemes.push(forms::ReportScheme { name: name.clone(), kind, form: form.clone() });
            fu.scheme = name;
            return action;
        }
        if delete_scheme {
            let name = std::mem::take(&mut fu.scheme);
            self.prj.forms.schemes.retain(|s| !(s.kind == kind && s.name == name));
            return action;
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::Grid::new("rform").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                if !fu.scheme.is_empty() {
                    ui.label(tr("Scheme name"));
                    let mut name = fu.scheme.clone();
                    let resp = ui.add(egui::TextEdit::singleline(&mut name).desired_width(320.0));
                    if resp.lost_focus() && name != fu.scheme && forms::rename_scheme(&mut self.prj, kind, &fu.scheme, &name) {
                        fu.scheme = name.trim().to_string();
                    }
                    ui.end_row();
                }
                ui.label(tr("Page heading"));
                ui.add(egui::TextEdit::singleline(&mut form.title).hint_text(kind.title()).desired_width(320.0));
                ui.end_row();
                ui.label(tr("Rows per page"));
                let default_rows = crate::report_blocks::rows_per_page(kind, None);
                let mut rows = if form.rows == 0 { default_rows } else { form.rows };
                if ui.add(egui::DragValue::new(&mut rows).range(5..=80)).changed() {
                    form.rows = if rows == default_rows { 0 } else { rows };
                }
                ui.end_row();
                ui.label(tr("Text size"));
                ui.add(egui::DragValue::new(&mut form.text_size).speed(0.05).range(0.0..=5.0).suffix(" mm"))
                    .on_hover_text(tr("0 = fitted to the rows per page"));
                ui.end_row();
                ui.label(tr("Group column"));
                let first = tr_fmt!("{} (first column)", headers.first().copied().unwrap_or(""));
                let current = if form.group.is_empty() { first.clone() } else { form.group.clone() };
                egui::ComboBox::from_id_salt("rfgroup").selected_text(current).width(240.0).show_ui(ui, |ui| {
                    ui.selectable_value(&mut form.group, String::new(), first);
                    for h in headers.iter().skip(1) {
                        ui.selectable_value(&mut form.group, h.to_string(), *h);
                    }
                })
                .response
                .on_hover_text(tr("Report blocks start new pages for every value of this column"));
                ui.end_row();
                ui.label("");
                ui.checkbox(&mut form.blank_repeats, tr("Leave repeated values of the first column blank"));
                ui.end_row();
            });
            ui.add_space(4.0);
            ui.columns(2, |cols| {
                columns_editor(&mut cols[0], &mut form, &headers, &table.widths);
                filter_editor(&mut cols[1], &mut form, &headers);
                cols[1].add_space(8.0);
                sort_editor(&mut cols[1], &mut form, &headers);
            });
            ui.horizontal(|ui| {
                if ui.button(tr("Reset to standard")).clicked() {
                    form = ReportForm::default();
                }
                ui.weak(tr("Applies to report pages, the PDF and the print export."));
            });
        });
        if form != stored {
            if !fu.scheme.is_empty() {
                if let Some(s) = self.prj.forms.schemes.iter_mut().find(|s| s.kind == kind && s.name == fu.scheme) {
                    s.form = form;
                }
            } else if form == ReportForm::default() {
                self.prj.forms.reports.remove(&kind);
            } else {
                self.prj.forms.reports.insert(kind, form);
            }
        }
        action
    }
}

/// Shown columns in their order, then the hidden ones; with their widths.
fn columns_editor(ui: &mut egui::Ui, form: &mut ReportForm, headers: &[&str], widths: &[f32]) {
    ui.label(RichText::new(tr("Columns")).strong());
    let shown: Vec<usize> = forms::report_columns(Some(form), headers);
    let mut order: Vec<(usize, bool)> = shown.iter().map(|&c| (c, true)).collect();
    order.extend((0..headers.len()).filter(|c| !shown.contains(c)).map(|c| (c, false)));
    let mut changed = false;
    let n = order.len();
    let mut swap = None;
    for (pos, (c, on)) in order.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            if ui.add_enabled(pos > 0, egui::Button::new("⬆").small()).clicked() {
                swap = Some((pos, pos - 1));
            }
            if ui.add_enabled(pos + 1 < n, egui::Button::new("⬇").small()).clicked() {
                swap = Some((pos, pos + 1));
            }
            let h = headers[*c];
            let mut w = form.widths.get(h).copied().unwrap_or(widths[*c]);
            if ui.add(egui::DragValue::new(&mut w).speed(0.05).range(0.1..=10.0)).on_hover_text(tr("Relative column width")).changed() {
                if (w - widths[*c]).abs() < 0.001 {
                    form.widths.remove(h);
                } else {
                    form.widths.insert(h.to_string(), w);
                }
            }
            changed |= ui.checkbox(on, h).changed();
        });
    }
    if let Some((a, b)) = swap {
        order.swap(a, b);
        changed = true;
    }
    if changed {
        let cols: Vec<usize> = order.iter().filter(|(_, on)| *on).map(|(c, _)| *c).collect();
        let all_in_order = cols.len() == headers.len() && cols.iter().enumerate().all(|(i, c)| i == *c);
        form.columns = if all_in_order || cols.is_empty() { Vec::new() } else { cols.iter().map(|&c| headers[c].to_string()).collect() };
    }
}

fn column_combo(ui: &mut egui::Ui, id: (&str, usize), column: &mut String, headers: &[&str]) {
    egui::ComboBox::from_id_salt(id).selected_text(column.as_str()).width(160.0).show_ui(ui, |ui| {
        for h in headers {
            ui.selectable_value(column, h.to_string(), *h);
        }
    });
}

fn filter_editor(ui: &mut egui::Ui, form: &mut ReportForm, headers: &[&str]) {
    ui.label(RichText::new(tr("Filter")).strong()).on_hover_text(tr("Only rows that pass all filters are shown"));
    let mut remove = None;
    for (i, f) in form.filters.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            column_combo(ui, ("rffcol", i), &mut f.column, headers);
            egui::ComboBox::from_id_salt(("rffop", i)).selected_text(f.op.title()).width(120.0).show_ui(ui, |ui| {
                for op in forms::FilterOp::ALL {
                    ui.selectable_value(&mut f.op, op, op.title());
                }
            });
            if f.op.has_value() {
                ui.add(egui::TextEdit::singleline(&mut f.value).desired_width(120.0));
            }
            if ui.small_button("🗑").clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        form.filters.remove(i);
    }
    if ui.button(tr("➕ Add filter")).clicked() {
        form.filters.push(forms::RowFilter { column: headers.first().map(|h| h.to_string()).unwrap_or_default(), ..Default::default() });
    }
}

fn sort_editor(ui: &mut egui::Ui, form: &mut ReportForm, headers: &[&str]) {
    ui.label(RichText::new(tr("Sort order")).strong());
    let mut remove = None;
    for (i, k) in form.sort.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            column_combo(ui, ("rfscol", i), &mut k.column, headers);
            ui.checkbox(&mut k.descending, tr("Descending"));
            if ui.small_button("🗑").clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        form.sort.remove(i);
    }
    if ui.button(tr("➕ Add sort key")).clicked() {
        form.sort.push(forms::SortKey { column: headers.first().map(|h| h.to_string()).unwrap_or_default(), descending: false });
    }
}

/// Title block drawn with the values of the current page; with `edit` its items
/// can be picked and dragged.
fn preview(ui: &mut egui::Ui, fu: &mut FormsUi, prj: &crate::model::Project, page: usize, form: &mut Form, edit: bool) {
    let width = ui.available_width().clamp(400.0, 1400.0);
    let zoom = width / forms::AREA_W;
    let (resp, painter) = ui.allocate_painter(vec2(width, forms::AREA_H * zoom + 8.0), Sense::click_and_drag());
    let pal = Palette::new(ui.visuals().dark_mode);
    painter.rect_filled(resp.rect, 2.0, pal.paper);
    let origin = resp.rect.min + vec2(0.0, 4.0) - vec2(FRAME_X0, TITLE_Y0) * zoom;
    if edit {
        pick(fu, &resp, ui, form, origin, zoom);
    }
    let mut d = EguiDraw { painter: &painter, origin, zoom, pal: &pal, force: None, mirror: None };
    forms::draw(&mut d, form, prj, page);
    if let Some(item) = fu.item.and_then(|i| form.items.get(i)) {
        let mut s = EguiDraw { painter: &painter, origin, zoom, pal: &pal, force: Some(Ink::Selected), mirror: None };
        forms::draw(&mut s, &Form { name: String::new(), items: vec![item.clone()] }, prj, page);
        if let FormItem::Text { pos, .. } = item {
            let c = origin + vec2(FRAME_X0 + pos.x, TITLE_Y0 + pos.y) * zoom;
            painter.circle_stroke(c, 3.0, Stroke::new(1.0, pal.selected));
        }
    }
}

/// Properties of the selected title block element.
fn item_properties(ui: &mut egui::Ui, fu: &mut FormsUi, form: &mut Form) {
    let Some(i) = fu.item.filter(|i| *i < form.items.len()) else {
        ui.weak(tr("Select an element in the list or the drawing."));
        return;
    };
    let mut delete = false;
    let mut duplicate = false;
    let pt = |ui: &mut egui::Ui, label: &str, q: &mut P| {
        ui.label(label);
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut q.x).speed(0.5).range(0.0..=forms::AREA_W).prefix("x "));
            ui.add(egui::DragValue::new(&mut q.y).speed(0.5).range(0.0..=forms::AREA_H).prefix("y "));
        });
        ui.end_row();
    };
    ui.label(RichText::new(form.items[i].kind_name()).strong());
    egui::Grid::new("tbitem").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| match &mut form.items[i] {
        FormItem::Line { a, b, width } => {
            pt(ui, tr("Start"), a);
            pt(ui, tr("End"), b);
            ui.label(tr("Line width"));
            ui.add(egui::DragValue::new(width).speed(0.05).range(0.05..=2.0).suffix(" mm"));
            ui.end_row();
        }
        FormItem::Rect { a, b, width, fill } => {
            pt(ui, tr("Corner"), a);
            pt(ui, tr("Corner"), b);
            ui.label(tr("Line width"));
            ui.add(egui::DragValue::new(width).speed(0.05).range(0.0..=2.0).suffix(" mm"));
            ui.end_row();
            ui.label(tr("Filled"));
            ui.checkbox(fill, "");
            ui.end_row();
        }
        FormItem::Text { pos, text, size, anchor, label, max_width } => {
            ui.label(tr("Text"));
            ui.add(egui::TextEdit::multiline(text).desired_rows(2).desired_width(220.0));
            ui.end_row();
            pt(ui, tr("Position"), pos);
            ui.label(tr("Size"));
            ui.add(egui::DragValue::new(size).speed(0.1).range(0.8..=20.0).suffix(" mm"));
            ui.end_row();
            ui.label(tr("Alignment"));
            ui.horizontal(|ui| {
                ui.selectable_value(anchor, Anchor::Left, tr("Left"));
                ui.selectable_value(anchor, Anchor::Center, tr("Centre"));
                ui.selectable_value(anchor, Anchor::Right, tr("Right"));
            });
            ui.end_row();
            ui.label(tr("Style"));
            ui.horizontal(|ui| {
                ui.selectable_value(label, false, tr("Value"));
                ui.selectable_value(label, true, tr("Label (grey)"));
            });
            ui.end_row();
            ui.label(tr("Max. width"));
            ui.add(egui::DragValue::new(max_width).speed(0.5).range(0.0..=forms::AREA_W).suffix(" mm"))
                .on_hover_text(tr("Longer values are drawn smaller (0 = no limit)"));
            ui.end_row();
        }
    });
    ui.horizontal(|ui| {
        duplicate = ui.button(tr("Duplicate")).clicked();
        delete = ui.button(tr("🗑 Delete")).clicked();
    });
    if duplicate {
        let mut copy = form.items[i].clone();
        copy.translate(p(2.0, 2.0));
        form.items.push(copy);
        fu.item = Some(form.items.len() - 1);
    }
    if delete {
        form.items.remove(i);
        fu.item = None;
    }
}

/// Picking and dragging in the editable preview.
fn pick(fu: &mut FormsUi, resp: &egui::Response, ui: &egui::Ui, form: &mut Form, origin: egui::Pos2, zoom: f32) {
    let to_mm = |s: egui::Pos2| p((s.x - origin.x) / zoom - FRAME_X0, (s.y - origin.y) / zoom - TITLE_Y0);
    let tol = 6.0 / zoom;
    let nearest = |form: &Form, q: P| {
        form.items
            .iter()
            .enumerate()
            .map(|(i, it)| (i, it.distance(q)))
            .filter(|(_, d)| *d <= tol)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    };
    if resp.drag_started_by(PointerButton::Primary) {
        if let Some(q) = ui.input(|i| i.pointer.press_origin()).map(to_mm) {
            fu.item = nearest(form, q);
            fu.drag = fu.item.map(|_| crate::symlib::snap(q, 0.5));
        }
    }
    if let (Some(last), Some(i), Some(q)) = (fu.drag, fu.item, resp.interact_pointer_pos().map(to_mm)) {
        let q = crate::symlib::snap(q, 0.5);
        let d = q - last;
        if (d.x != 0.0 || d.y != 0.0) && i < form.items.len() {
            form.items[i].translate(d);
            fu.drag = Some(q);
        }
    }
    if resp.drag_stopped() {
        fu.drag = None;
    }
    if resp.clicked_by(PointerButton::Primary) {
        if let Some(q) = resp.interact_pointer_pos().map(to_mm) {
            fu.item = nearest(form, q);
        }
    }
}
