//! Editor of a drawn report form (Forms › Report forms › Own drawn form): the page
//! graphics and the row drawing on a preview of the report page with its first rows.

use super::forms_ui::{add_buttons, item_list, item_properties, pick, FormsUi};
use crate::editor::{EguiDraw, Palette};
use egui::{vec2, RichText, Sense, Stroke};
use openecraft_core::forms::{self, FormItem, FormLayout};
use openecraft_core::geom::{p, P};
use openecraft_core::model::report_blocks::PageTable;
use openecraft_core::model::Project;
use openecraft_core::render::Ink;
use openecraft_core::table::Table;
use openecraft_i18n::tr;

pub(super) fn layout_editor(ui: &mut egui::Ui, fu: &mut FormsUi, prj: &Project, page: usize, layout: &mut FormLayout, table: &Table, title: &str) {
    let area = openecraft_core::frame::of_page(prj, page).area();
    let size = p(area.width(), area.height());
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(tr("Drawn form")).strong());
        ui.label(tr("Edit"));
        let before = fu.layout_row;
        ui.selectable_value(&mut fu.layout_row, false, tr("Page graphics"));
        ui.selectable_value(&mut fu.layout_row, true, tr("Row drawing"));
        if fu.layout_row != before {
            fu.select(None);
        }
        ui.separator();
        ui.label(tr("First row"));
        ui.add(egui::DragValue::new(&mut layout.first_row.x).speed(0.5).prefix("x "));
        ui.add(egui::DragValue::new(&mut layout.first_row.y).speed(0.5).prefix("y "));
        ui.label(tr("Next row"));
        ui.add(egui::DragValue::new(&mut layout.step.x).speed(0.5).prefix("x ")).on_hover_text(tr("Offset from one row to the next: downwards for lists, sideways for rows side by side"));
        ui.add(egui::DragValue::new(&mut layout.step.y).speed(0.5).prefix("y "));
    });
    let rows = table.rows.len().min(page_rows(layout, size));
    let t = PageTable {
        title: title.to_string(),
        headers: table.header_texts().into_iter().map(String::from).collect(),
        keys: table.header_keys().into_iter().map(String::from).collect(),
        rows: table.rows.iter().take(rows).cloned().collect(),
        ..Default::default()
    };
    // The preview: the whole drawing area of the page.
    let width = ui.available_width().clamp(400.0, 1400.0);
    let zoom = width / size.x;
    let (resp, painter) = ui.allocate_painter(vec2(width, size.y * zoom), Sense::click_and_drag());
    let pal = Palette::new(ui.visuals().dark_mode);
    painter.rect_filled(resp.rect, 2.0, pal.paper);
    let origin = resp.rect.min;
    let offset = if fu.layout_row { layout.first_row } else { p(0.0, 0.0) };
    let items_origin = origin + vec2(offset.x, offset.y) * zoom;
    let items = if fu.layout_row { &mut layout.row } else { &mut layout.header };
    pick(fu, &resp, ui, items, items_origin, zoom);
    let mut d = EguiDraw { painter: &painter, origin, zoom, pal: &pal, force: None, mirror: None, textures: None };
    forms::draw_layout(&mut d, layout, &t, prj, page, p(0.0, 0.0));
    let items = if fu.layout_row { &layout.row } else { &layout.header };
    if let Some(item) = fu.selected().and_then(|i| items.get(i)) {
        let mut s = EguiDraw { painter: &painter, origin, zoom, pal: &pal, force: Some(Ink::Selected), mirror: None, textures: None };
        forms::draw_items(&mut s, std::slice::from_ref(item), offset, 1.0, &|text| text.to_string());
        if let FormItem::Text { pos, .. } = item {
            painter.circle_stroke(items_origin + vec2(pos.x, pos.y) * zoom, 3.0, Stroke::new(1.0, pal.selected));
        }
    }
    if fu.layout_row {
        // Where the row drawing of the first row starts.
        painter.circle_stroke(items_origin, 4.0, Stroke::new(1.0, pal.selected));
    }
    ui.separator();
    let items = if fu.layout_row { &mut layout.row } else { &mut layout.header };
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(380.0);
            let first = table.header_keys().first().map(|k| format!("{{{k}}}")).unwrap_or_default();
            let mid = if fu.layout_row { p(20.0, 3.0) } else { size * 0.5 };
            if let Some(i) = add_buttons(ui, items, mid, if fu.layout_row { &first } else { "{report}" }) {
                fu.select(Some(i));
            }
            if let Some(i) = item_list(ui, "layoutitems", items, fu.selected()) {
                fu.select(Some(i));
            }
        });
        ui.separator();
        ui.vertical(|ui| {
            ui.set_width(330.0);
            item_properties(ui, fu, items, size);
        });
        ui.separator();
        ui.vertical(|ui| {
            ui.label(RichText::new(tr("Placeholders")).strong());
            ui.weak(tr("Click to add to the selected text."));
            egui::ScrollArea::vertical().id_salt("layoutph").max_height(240.0).show(ui, |ui| {
                let columns = table.header_keys().into_iter().zip(table.header_texts()).map(|(k, t)| (format!("{{{k}}}"), t.to_string()));
                let page = std::iter::once(("{report}".to_string(), tr("Report name (page heading)").to_string()));
                let common = forms::PLACEHOLDERS.iter().map(|(ph, meaning)| (ph.to_string(), tr(meaning).to_string()));
                for (ph, meaning) in columns.chain(page).chain(common) {
                    ui.horizontal(|ui| {
                        if ui.link(RichText::new(&ph).monospace()).clicked() {
                            if let Some(FormItem::Text { text, .. }) = fu.selected().and_then(|i| items.get_mut(i)) {
                                text.push_str(&ph);
                            }
                        }
                        ui.weak(meaning);
                    });
                }
            });
        });
    });
}

/// Rows that fit the drawing area (at least one, at most 200).
fn page_rows(layout: &FormLayout, size: P) -> usize {
    let fits = |start: f32, step: f32, len: f32| if step > 0.01 { ((len - start) / step).floor().max(1.0) as usize } else { 200 };
    fits(layout.first_row.x, layout.step.x, size.x).min(fits(layout.first_row.y, layout.step.y, size.y)).clamp(1, 200)
}
