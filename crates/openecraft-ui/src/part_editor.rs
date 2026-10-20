//! Part widgets: product group tree, tabbed part editor, read only summary and
//! previews of the 2D front view and the 3D model.

use crate::io::{short_name, FileKind};
use egui::{vec2, Color32, Pos2, RichText, Sense, Shape, Stroke};
use openecraft_parts::vec3::{cross, dot, norm, sub, Orbit};
use openecraft_parts::{parse_mesh, parse_svg, natural_cmp, FnKind, FunctionTemplate, Mesh3d, Model2d, Part, Prim2d, TopGroup, PRODUCT_GROUPS};
use std::collections::BTreeMap;

/// Graphics file of a part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelKind {
    /// 2D front view (SVG).
    Front2d,
    /// 3D model (STL, OBJ).
    Mesh3d,
}

impl FileKind for ModelKind {
    fn filter(self) -> (&'static str, &'static [&'static str]) {
        match self {
            ModelKind::Front2d => ("2D front view (SVG)", &["svg"]),
            ModelKind::Mesh3d => ("3D model (STL, OBJ)", &["stl", "obj"]),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartTab {
    General,
    Commercial,
    Technical,
    Mounting,
    Functions,
    Accessories,
    Graphics,
}

/// Editor settings kept between frames.
pub struct EditorState {
    pub tab: PartTab,
    /// Factor of the "Scale" button for imported 3D models.
    pub model_scale: f32,
}

impl Default for EditorState {
    fn default() -> Self {
        EditorState { tab: PartTab::General, model_scale: 25.4 }
    }
}

/// Product group tree. Returns the part number that was double clicked.
pub fn part_tree(ui: &mut egui::Ui, id: &str, parts: &[&Part], filter: &str, selected: &mut Option<String>) -> Option<String> {
    let mut tree: BTreeMap<TopGroup, BTreeMap<&str, BTreeMap<&str, Vec<&Part>>>> = BTreeMap::new();
    for p in parts.iter().filter(|p| p.matches_text(filter)) {
        let group = if p.group.is_empty() { "(no product group)" } else { p.group.as_str() };
        tree.entry(p.top_group).or_default().entry(group).or_default().entry(p.subgroup.as_str()).or_default().push(p);
    }
    let open = if filter.is_empty() { None } else { Some(true) };
    let mut activated = None;
    let mut leaf = |ui: &mut egui::Ui, list: &mut Vec<&Part>| {
        list.sort_by(|a, b| natural_cmp(&a.number, &b.number));
        for p in list.iter() {
            let is_sel = selected.as_deref() == Some(p.number.as_str());
            let r = ui.selectable_label(is_sel, &p.number).on_hover_text(&p.description);
            if r.clicked() {
                *selected = Some(p.number.clone());
            }
            if r.double_clicked() {
                activated = Some(p.number.clone());
            }
        }
    };
    for (top, groups) in tree.iter_mut() {
        egui::CollapsingHeader::new(RichText::new(top.title()).strong())
            .id_salt((id, top.title()))
            .default_open(true)
            .open(open)
            .show(ui, |ui| {
                for (group, subs) in groups.iter_mut() {
                    egui::CollapsingHeader::new(*group).id_salt((id, top.title(), *group)).open(open).show(ui, |ui| {
                        for (sub, list) in subs.iter_mut() {
                            if sub.is_empty() {
                                leaf(ui, list);
                            } else {
                                egui::CollapsingHeader::new(*sub)
                                    .id_salt((id, top.title(), *group, *sub))
                                    .open(open)
                                    .show(ui, |ui| leaf(ui, list));
                            }
                        }
                    });
                }
            });
    }
    if tree.is_empty() {
        ui.weak("No parts found.");
    }
    activated
}

fn text_row(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.label(label);
    ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY));
    ui.end_row();
}

fn num_row(ui: &mut egui::Ui, label: &str, value: &mut f32, suffix: &str) {
    ui.label(label);
    ui.add(egui::DragValue::new(value).speed(0.1).range(0.0..=1.0e7).suffix(suffix));
    ui.end_row();
}

/// Tabbed editor for one part. `all_numbers` are offered as accessories, `duplicate`
/// marks the part number as already used. Returns a requested model import.
pub fn part_editor(ui: &mut egui::Ui, part: &mut Part, state: &mut EditorState, all_numbers: &[String], duplicate: bool) -> Option<ModelKind> {
    let mut request = None;
    ui.horizontal_wrapped(|ui| {
        for (t, name) in [
            (PartTab::General, "General"),
            (PartTab::Commercial, "Prices / other"),
            (PartTab::Technical, "Technical data"),
            (PartTab::Mounting, "Mounting data"),
            (PartTab::Functions, "Function template"),
            (PartTab::Accessories, "Accessories"),
            (PartTab::Graphics, "Graphics"),
        ] {
            ui.selectable_value(&mut state.tab, t, name);
        }
    });
    ui.separator();
    egui::ScrollArea::vertical().id_salt("part_editor").auto_shrink([false, false]).show(ui, |ui| match state.tab {
        PartTab::General => {
            egui::Grid::new("pg").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Part number");
                ui.vertical(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut part.number).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
                    if duplicate {
                        ui.colored_label(Color32::from_rgb(220, 60, 60), "Part number already exists");
                    }
                });
                ui.end_row();
                text_row(ui, "Type number", &mut part.type_number);
                text_row(ui, "Order number", &mut part.order_number);
                text_row(ui, "Manufacturer", &mut part.manufacturer);
                text_row(ui, "Supplier", &mut part.supplier);
                ui.label("Description");
                ui.add(egui::TextEdit::multiline(&mut part.description).desired_rows(2).desired_width(f32::INFINITY));
                ui.end_row();
                ui.label("Product top group");
                egui::ComboBox::from_id_salt("top").selected_text(part.top_group.title()).show_ui(ui, |ui| {
                    for t in TopGroup::ALL {
                        ui.selectable_value(&mut part.top_group, t, t.title());
                    }
                });
                ui.end_row();
                ui.label("Product group");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut part.group).desired_width(200.0));
                    egui::ComboBox::from_id_salt("grp").selected_text("…").width(24.0).show_ui(ui, |ui| {
                        for (g, _) in PRODUCT_GROUPS {
                            if ui.selectable_label(part.group == *g, *g).clicked() {
                                part.group = g.to_string();
                            }
                        }
                    });
                });
                ui.end_row();
                ui.label("Product subgroup");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut part.subgroup).desired_width(200.0));
                    let subs = PRODUCT_GROUPS.iter().find(|(g, _)| *g == part.group).map(|(_, s)| *s).unwrap_or(&[]);
                    if !subs.is_empty() {
                        egui::ComboBox::from_id_salt("sub").selected_text("…").width(24.0).show_ui(ui, |ui| {
                            for s in subs {
                                if ui.selectable_label(part.subgroup == *s, *s).clicked() {
                                    part.subgroup = s.to_string();
                                }
                            }
                        });
                    }
                });
                ui.end_row();
                ui.label("Identifier");
                ui.add(egui::TextEdit::singleline(&mut part.identifier).desired_width(50.0))
                    .on_hover_text("Identifying letter of devices using this part (K, Q, S …)");
                ui.end_row();
                ui.label("Note");
                ui.add(egui::TextEdit::multiline(&mut part.note).desired_rows(2).desired_width(f32::INFINITY));
                ui.end_row();
            });
        }
        PartTab::Commercial => {
            egui::Grid::new("pc").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Price");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut part.price).speed(0.1).range(0.0..=1.0e7).fixed_decimals(2));
                    ui.add(egui::TextEdit::singleline(&mut part.currency).desired_width(40.0));
                });
                ui.end_row();
                ui.label("Price unit");
                ui.add(egui::DragValue::new(&mut part.price_unit).range(1..=100000)).on_hover_text("The price refers to this many units");
                ui.end_row();
                ui.label("Packaging quantity");
                ui.add(egui::DragValue::new(&mut part.pack_quantity).range(1..=100000));
                ui.end_row();
                ui.label("Quantity unit");
                ui.add(egui::TextEdit::singleline(&mut part.quantity_unit).desired_width(60.0));
                ui.end_row();
                text_row(ui, "Certificates", &mut part.certificates);
                text_row(ui, "Datasheet (URL)", &mut part.datasheet);
            });
            if part.datasheet.starts_with("http") {
                ui.hyperlink_to("Open datasheet", &part.datasheet);
            }
        }
        PartTab::Technical => {
            egui::Grid::new("pt").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                text_row(ui, "Voltage", &mut part.voltage);
                text_row(ui, "Current", &mut part.current);
                text_row(ui, "Power", &mut part.power);
                ui.label("Technical data");
                ui.add(egui::TextEdit::multiline(&mut part.tech_data).desired_rows(5).desired_width(f32::INFINITY));
                ui.end_row();
            });
        }
        PartTab::Mounting => {
            egui::Grid::new("pm").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                num_row(ui, "Width", &mut part.width, " mm");
                num_row(ui, "Height", &mut part.height, " mm");
                num_row(ui, "Depth", &mut part.depth, " mm");
                num_row(ui, "Weight", &mut part.weight, " kg");
                text_row(ui, "Mounting", &mut part.mounting);
            });
        }
        PartTab::Functions => {
            ui.weak("Functions the part provides. Used for device selection, the contact image (reserve contacts), the project check and to assign connection designations. Designations are listed in symbol connection order, separated by blanks.");
            ui.add_space(4.0);
            let mut remove = None;
            egui::Grid::new("pf").num_columns(4).striped(true).spacing([8.0, 4.0]).show(ui, |ui| {
                ui.label(RichText::new("Function").strong());
                ui.label(RichText::new("Connection designations").strong());
                ui.label(RichText::new("Description").strong());
                ui.label("");
                ui.end_row();
                for (i, f) in part.functions.iter_mut().enumerate() {
                    egui::ComboBox::from_id_salt(("fk", i)).selected_text(f.kind.title()).width(170.0).show_ui(ui, |ui| {
                        for k in FnKind::ALL {
                            ui.selectable_value(&mut f.kind, k, k.title());
                        }
                    });
                    ui.add(egui::TextEdit::singleline(&mut f.pins).desired_width(140.0).font(egui::TextStyle::Monospace));
                    ui.add(egui::TextEdit::singleline(&mut f.description).desired_width(160.0));
                    if ui.small_button("🗑").clicked() {
                        remove = Some(i);
                    }
                    ui.end_row();
                }
            });
            if let Some(i) = remove {
                part.functions.remove(i);
            }
            if ui.button("➕ Add function").clicked() {
                part.functions.push(FunctionTemplate::new(FnKind::No, ""));
            }
        }
        PartTab::Accessories => {
            let mut remove = None;
            for (i, a) in part.accessories.iter().enumerate() {
                ui.horizontal(|ui| {
                    let known = all_numbers.contains(a);
                    ui.label(RichText::new(a).monospace().color(if known { ui.visuals().text_color() } else { Color32::from_rgb(220, 60, 60) }));
                    if ui.small_button("🗑").clicked() {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                part.accessories.remove(i);
            }
            let mut add = None;
            egui::ComboBox::from_id_salt("acc").selected_text("➕ Add accessory").width(240.0).show_ui(ui, |ui| {
                for n in all_numbers {
                    if *n != part.number && !part.accessories.contains(n) && ui.selectable_label(false, n).clicked() {
                        add = Some(n.clone());
                    }
                }
            });
            if let Some(n) = add {
                part.accessories.push(n);
            }
        }
        PartTab::Graphics => request = graphics_tab(ui, part, &mut state.model_scale),
    });
    request
}

/// 2D front view and 3D model of a part.
fn graphics_tab(ui: &mut egui::Ui, part: &mut Part, model_scale: &mut f32) -> Option<ModelKind> {
    let mut request = None;
    ui.weak("The 2D front view is used in the panel front view and on mounting layout pages, the 3D model in the 3D panel view. Both are fitted to the mounting data (width × height × depth).");
    ui.add_space(4.0);
    ui.label(RichText::new("2D front view").strong());
    ui.horizontal(|ui| {
        model_2d_preview(ui, part.model_2d.as_ref(), vec2(180.0, 140.0));
        ui.vertical(|ui| {
            match &part.model_2d {
                Some(m) => ui.label(format!("{:.1} × {:.1} mm, {} elements", m.width, m.height, m.prims.len())),
                None => ui.weak("none – a plain rectangle is drawn"),
            };
            if ui.button("Import SVG…").clicked() {
                request = Some(ModelKind::Front2d);
            }
            if part.model_2d.is_some() && ui.button("🗑 Remove").clicked() {
                part.model_2d = None;
            }
        });
    });
    ui.separator();
    ui.label(RichText::new("3D model").strong());
    ui.horizontal(|ui| {
        match &part.model_3d {
            Some(m) => mesh_preview(ui, m, vec2(180.0, 160.0)),
            None => {
                ui.allocate_space(vec2(180.0, 40.0));
            }
        }
        ui.vertical(|ui| {
            match &part.model_3d {
                Some(m) => {
                    let s = m.size();
                    ui.label(format!("{:.1} × {:.1} × {:.1} mm, {} triangles", s[0], s[1], s[2], m.triangles.len()));
                }
                None => {
                    ui.weak("none – a box of the mounting dimensions is drawn");
                }
            }
            if ui.button("Import STL / OBJ…").clicked() {
                request = Some(ModelKind::Mesh3d);
            }
            if part.model_3d.is_some() && ui.button("🗑 Remove").clicked() {
                part.model_3d = None;
            }
        });
    });
    if let Some(m) = &mut part.model_3d {
        ui.horizontal_wrapped(|ui| {
            ui.label("Rotate 90°:");
            for (axis, label) in [(0, "X"), (1, "Y"), (2, "Z")] {
                if ui.button(label).on_hover_text("Turn the model so that its front faces the viewer (+Z) and its top is +Y").clicked() {
                    m.rotate90(axis);
                }
            }
            ui.separator();
            ui.label("Scale ×");
            ui.add(egui::DragValue::new(model_scale).speed(0.01).range(0.001..=10000.0));
            if ui.button("Apply").on_hover_text("e.g. 25.4 for models in inch, 1000 for models in metres").clicked() {
                m.scale(*model_scale);
            }
        });
        let s = m.size();
        let differs = [(s[0], part.width), (s[1], part.height), (s[2], part.depth)].iter().any(|(a, b)| (a - b).abs() > 0.5);
        if differs {
            ui.colored_label(
                Color32::from_rgb(220, 140, 0),
                format!(
                    "The model ({:.1} × {:.1} × {:.1}) differs from the mounting data ({:.1} × {:.1} × {:.1}); it is stretched to the mounting data.",
                    s[0], s[1], s[2], part.width, part.height, part.depth
                ),
            );
            if ui.button("Use model size as mounting data").clicked() {
                part.width = (s[0] * 10.0).round() / 10.0;
                part.height = (s[1] * 10.0).round() / 10.0;
                part.depth = (s[2] * 10.0).round() / 10.0;
            }
        }
    }
    request
}

/// Parses a model file and assigns it to the part. Returns a status message.
pub fn apply_model(part: &mut Part, kind: ModelKind, name: &str, bytes: &[u8]) -> Result<String, String> {
    let file = short_name(name);
    let number = &part.number;
    match kind {
        ModelKind::Front2d => {
            let m = parse_svg(&String::from_utf8_lossy(bytes))?;
            let msg = format!("{file}: 2D front view with {} elements assigned to {number}", m.prims.len());
            part.model_2d = Some(m);
            Ok(msg)
        }
        ModelKind::Mesh3d => {
            let m = parse_mesh(file, bytes)?;
            let s = m.size();
            let msg = format!("{file}: 3D model with {} triangles ({:.1} × {:.1} × {:.1} mm) assigned to {number}", m.triangles.len(), s[0], s[1], s[2]);
            // Parts without mounting data take the model size.
            if part.width <= 0.0 || part.height <= 0.0 {
                part.width = s[0];
                part.height = s[1];
                part.depth = s[2];
            }
            part.model_3d = Some(m);
            Ok(msg)
        }
    }
}

/// Scaled drawing of a 2D front view.
pub fn model_2d_preview(ui: &mut egui::Ui, model: Option<&Model2d>, size: egui::Vec2) {
    let (resp, painter) = ui.allocate_painter(size, Sense::hover());
    let visuals = ui.visuals();
    painter.rect_filled(resp.rect, 4.0, visuals.extreme_bg_color);
    let Some(m) = model.filter(|m| m.width > 0.0 && m.height > 0.0) else {
        painter.text(resp.rect.center(), egui::Align2::CENTER_CENTER, "no 2D model", egui::FontId::proportional(12.0), visuals.weak_text_color());
        return;
    };
    let zoom = ((size.x - 12.0) / m.width).min((size.y - 12.0) / m.height);
    let origin = resp.rect.center() - vec2(m.width, m.height) * zoom / 2.0;
    let map = |q: [f32; 2]| origin + vec2(q[0], q[1]) * zoom;
    let ink = visuals.text_color();
    let line = Stroke::new(1.0, ink);
    painter.rect_filled(egui::Rect::from_two_pos(map([0.0, 0.0]), map([m.width, m.height])), 0.0, visuals.widgets.inactive.bg_fill);
    for prim in &m.prims {
        match prim {
            Prim2d::Line { a, b } => {
                painter.line_segment([map(*a), map(*b)], line);
            }
            Prim2d::Rect { min, max, fill } => {
                let r = egui::Rect::from_two_pos(map(*min), map(*max));
                if *fill {
                    painter.rect_filled(r, 0.0, ink);
                } else {
                    painter.rect_stroke(r, 0.0, line, egui::StrokeKind::Middle);
                }
            }
            Prim2d::Circle { c, r, fill } => {
                let r = (r * zoom).max(1.0);
                if *fill {
                    painter.circle_filled(map(*c), r, ink);
                } else {
                    painter.circle_stroke(map(*c), r, line);
                }
            }
            Prim2d::Polyline { pts, closed, fill } => {
                let v: Vec<Pos2> = pts.iter().map(|q| map(*q)).collect();
                painter.add(match (*closed, *fill) {
                    (true, true) => Shape::convex_polygon(v, ink, Stroke::NONE),
                    (true, false) => Shape::closed_line(v, line),
                    _ => Shape::line(v, line),
                });
            }
            Prim2d::Text { pos, text, size } => {
                let px = size * zoom * 1.25;
                if px >= 3.0 {
                    painter.text(map(*pos), egui::Align2::LEFT_CENTER, text, egui::FontId::proportional(px), ink);
                }
            }
        }
    }
}

/// Small shaded view of a mesh; drag to rotate.
pub fn mesh_preview(ui: &mut egui::Ui, mesh: &Mesh3d, size: egui::Vec2) {
    let (resp, painter) = ui.allocate_painter(size, Sense::drag());
    let id = resp.id.with("angles");
    let (mut yaw, mut pitch): (f32, f32) = ui.data(|d| d.get_temp(id)).unwrap_or((0.6, 0.35));
    if resp.dragged() {
        yaw -= resp.drag_delta().x * 0.01;
        pitch = (pitch + resp.drag_delta().y * 0.01).clamp(-1.5, 1.5);
        ui.data_mut(|d| d.insert_temp(id, (yaw, pitch)));
    }
    let dark = ui.visuals().dark_mode;
    painter.rect_filled(resp.rect, 4.0, if dark { Color32::from_rgb(30, 32, 36) } else { Color32::from_rgb(232, 235, 240) });
    let s = mesh.size();
    let (min, _) = mesh.bounds();
    let c = [min[0] + s[0] / 2.0, min[1] + s[1] / 2.0, min[2] + s[2] / 2.0];
    let (eye, right, up, fwd) = Orbit { yaw, pitch, dist: 1.0, target: c }.basis();
    let view = sub(eye, c); // direction towards the viewer
    let radius = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt().max(1e-3) / 2.0;
    let scale = size.x.min(size.y) * 0.45 / radius;
    let center = resp.rect.center();
    let light = norm([0.35, 0.75, 0.55]);
    let mut tris: Vec<(f32, [Pos2; 3], Color32)> = Vec::new();
    for t in &mesh.triangles {
        let v = t.map(|i| mesh.vertices[i as usize]);
        let n = norm(cross(sub(v[1], v[0]), sub(v[2], v[0])));
        if dot(n, view) <= 0.0 {
            continue;
        }
        let pts = v.map(|q| {
            let d = sub(q, c);
            center + vec2(dot(d, right), -dot(d, up)) * scale
        });
        let depth = v.iter().map(|q| dot(sub(*q, c), fwd)).sum::<f32>();
        let shade = 0.5 + 0.5 * dot(n, light).max(0.0);
        let g = (190.0 * shade) as u8;
        tris.push((depth, pts, Color32::from_rgb(g, g.saturating_add(6), g.saturating_add(14))));
    }
    tris.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (_, pts, color) in tris {
        painter.add(Shape::convex_polygon(pts.to_vec(), color, Stroke::new(0.5, color)));
    }
    if resp.hovered() {
        painter.text(resp.rect.left_bottom() + vec2(4.0, -4.0), egui::Align2::LEFT_BOTTOM, "drag to rotate", egui::FontId::proportional(10.0), Color32::GRAY);
    }
}

/// Read only summary of a part.
pub fn part_summary(ui: &mut egui::Ui, p: &Part) {
    ui.label(RichText::new(&p.number).monospace().strong());
    ui.label(&p.description);
    egui::Grid::new("sum").num_columns(2).show(ui, |ui| {
        for (k, v) in [
            ("Manufacturer", p.manufacturer.clone()),
            ("Type number", p.type_number.clone()),
            ("Order number", p.order_number.clone()),
            ("Product group", format!("{} / {}", p.group, p.subgroup)),
            ("Voltage / current", format!("{} {}", p.voltage, p.current)),
            ("Price", format!("{:.2} {} / {} {}", p.price, p.currency, p.price_unit, p.quantity_unit)),
            ("Dimensions", format!("{} × {} × {} mm", p.width, p.height, p.depth)),
            ("Functions", p.template_summary()),
            ("Accessories", p.accessories.join(", ")),
        ] {
            if !v.trim().is_empty() && v.trim() != "/" {
                ui.label(RichText::new(k).weak());
                ui.label(v);
                ui.end_row();
            }
        }
    });
}

#[cfg(test)]
mod tests;
