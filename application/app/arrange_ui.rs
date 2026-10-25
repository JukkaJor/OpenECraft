//! Editing aids: typed coordinates, grid size, align and distribute, move the
//! selection by a distance, multiple copy.

use openecraft_i18n::{tr, tr_fmt};
use super::App;
use openecraft_core::arrange::{self, Align};
use openecraft_core::geom::p;
use openecraft_core::ops;

/// Grid sizes offered (mm).
pub const GRIDS: [f32; 5] = [1.0, 2.0, 4.0, 8.0, 16.0];

pub struct ArrangeUi {
    coord: String,
    pub show_move: bool,
    pub show_copy: bool,
    dx: f32,
    dy: f32,
    count: usize,
}

impl Default for ArrangeUi {
    fn default() -> Self {
        ArrangeUi { coord: String::new(), show_move: false, show_copy: false, dx: 0.0, dy: 40.0, count: 3 }
    }
}

pub(super) fn align_title(a: Align) -> &'static str {
    match a {
        Align::Left => tr("Left edges"),
        Align::Center => tr("Horizontal centres"),
        Align::Right => tr("Right edges"),
        Align::Top => tr("Top edges"),
        Align::Middle => tr("Vertical centres"),
        Align::Bottom => tr("Bottom edges"),
    }
}

impl App {
    /// Field for a typed point (tool bar): Enter places it like a click.
    pub(super) fn coordinate_field(&mut self, ui: &mut egui::Ui) {
        let r = ui
            .add(egui::TextEdit::singleline(&mut self.arrange_ui.coord).desired_width(80.0).hint_text(tr("x y")))
            .on_hover_text(tr("Type a point and press Enter: x y in mm, or @dx dy relative to the last point; it acts like a click of the current tool"));
        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            match arrange::parse_point(&self.arrange_ui.coord, self.editor.last_point) {
                Some(q) => {
                    self.editor.typed = Some(q);
                    self.arrange_ui.coord.clear();
                }
                None => {
                    let ctx = ui.ctx().clone();
                    self.notify(&ctx, tr("Not a point: type x y or @dx dy"));
                }
            }
        }
    }

    pub(super) fn arrange_windows(&mut self, ctx: &egui::Context) {
        let page = self.session.page;
        if self.arrange_ui.show_move {
            let mut open = true;
            let mut apply = false;
            let st = &mut self.arrange_ui;
            egui::Window::new(tr("Move selection by")).open(&mut open).resizable(false).show(ctx, |ui| {
                egui::Grid::new("movesel").num_columns(2).show(ui, |ui| {
                    ui.label(tr("Horizontal (mm)"));
                    ui.add(egui::DragValue::new(&mut st.dx).speed(1.0));
                    ui.end_row();
                    ui.label(tr("Vertical (mm, downwards)"));
                    ui.add(egui::DragValue::new(&mut st.dy).speed(1.0));
                    ui.end_row();
                });
                apply = ui.button(tr("Move")).clicked();
            });
            if apply && !self.editor.selection.is_empty() {
                let symbols = self.session.prj.symbol_table();
                let pg = &mut self.session.prj.pages[page];
                let attached = ops::attached_wire_ends(&symbols, pg, &self.editor.selection);
                ops::translate(pg, &self.editor.selection, &attached, p(self.arrange_ui.dx, self.arrange_ui.dy));
            }
            self.arrange_ui.show_move = open && !apply;
        }
        if self.arrange_ui.show_copy {
            let mut open = true;
            let mut apply = false;
            let st = &mut self.arrange_ui;
            egui::Window::new(tr("Multiple copy")).open(&mut open).resizable(false).show(ctx, |ui| {
                ui.label(tr("Copies the selection several times; devices get new tags, terminals new numbers."));
                egui::Grid::new("multicopy").num_columns(2).show(ui, |ui| {
                    ui.label(tr("Copies"));
                    ui.add(egui::DragValue::new(&mut st.count).range(1..=100));
                    ui.end_row();
                    ui.label(tr("Horizontal (mm)"));
                    ui.add(egui::DragValue::new(&mut st.dx).speed(1.0));
                    ui.end_row();
                    ui.label(tr("Vertical (mm, downwards)"));
                    ui.add(egui::DragValue::new(&mut st.dy).speed(1.0));
                    ui.end_row();
                });
                apply = ui.button(tr("Copy")).clicked();
            });
            if apply && !self.editor.selection.is_empty() {
                let (count, d) = (self.arrange_ui.count, p(self.arrange_ui.dx, self.arrange_ui.dy));
                let ids = arrange::multiple_copy(&mut self.session.prj, page, &self.editor.selection, count, d);
                self.notify(ctx, tr_fmt!("{} items copied", ids.len()));
                self.editor.selection = ids;
            }
            self.arrange_ui.show_copy = open && !apply;
        }
    }
}
