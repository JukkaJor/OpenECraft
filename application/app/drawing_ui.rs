//! Schematic editing beyond symbols and wires: drawing tools, layers, dimensions,
//! images, connection and cable definitions, property placement, and the
//! connection / cable / layer navigators.

use openecraft_i18n::{tr, tr_fmt};
use super::App;
use crate::editor::Tool;
use crate::graphics::{self, Layer};
use crate::io::Loaded;
use crate::model::*;
use crate::netlist::pin_designation;
use crate::{cabledef, conndef, ops, properties, props, routing, structure, wirenum};
use egui::RichText;

/// Selection of the layer of an item ("" is the default layer).
fn layer_combo(ui: &mut egui::Ui, salt: &str, layer: &mut String, names: &[String]) {
    let shown = if layer.is_empty() { tr("(default)").to_string() } else { layer.clone() };
    egui::ComboBox::from_id_salt(salt).selected_text(shown).show_ui(ui, |ui| {
        ui.selectable_value(layer, String::new(), tr("(default)"));
        for n in names.iter().filter(|n| !n.is_empty()) {
            ui.selectable_value(layer, n.clone(), n);
        }
    });
}

/// Own colour of an item (none: the layer's or the normal colour).
fn color_field(ui: &mut egui::Ui, color: &mut Option<[u8; 3]>) {
    ui.horizontal(|ui| {
        let mut own = color.is_some();
        if ui.checkbox(&mut own, tr("Own colour")).changed() {
            *color = own.then_some([200, 30, 30]);
        }
        if let Some(c) = color {
            ui.color_edit_button_srgb(c);
        }
    });
}

impl App {
    /// Drawing tools, definition tools and editing options of the toolbar.
    pub(super) fn drawing_toolbar(&mut self, ui: &mut egui::Ui) {
        let tool = self.editor.tool.clone();
        let draw_tools = [
            (Tool::Line, tr("Line")),
            (Tool::Rect, tr("Rectangle")),
            (Tool::Circle, tr("Circle")),
            (Tool::Ellipse, tr("Ellipse")),
            (Tool::Arc, tr("Arc")),
            (Tool::Polyline, tr("Polyline")),
            (Tool::Polygon, tr("Polygon")),
        ];
        let active = draw_tools.iter().find(|(t, _)| *t == tool).map(|(_, n)| *n);
        let label = match active {
            Some(n) => tr_fmt!("✏ Draw: {}", n),
            None => tr("✏ Draw").to_string(),
        };
        ui.menu_button(label, |ui| {
            for (t, name) in draw_tools {
                if ui.selectable_label(tool == t, name).clicked() {
                    self.editor.set_tool(t);
                    ui.close();
                }
            }
        })
        .response
        .on_hover_text(tr("Graphics: lines, rectangles, circles, ellipses, arcs, polylines and polygons"));
        let dims = [(DimKind::Aligned, tr("Aligned")), (DimKind::Horizontal, tr("Horizontal")), (DimKind::Vertical, tr("Vertical"))];
        ui.menu_button(tr("↔ Dimension"), |ui| {
            for (k, name) in dims {
                if ui.selectable_label(tool == Tool::Dimension(k), name).clicked() {
                    self.editor.set_tool(Tool::Dimension(k));
                    ui.close();
                }
            }
        })
        .response
        .on_hover_text(tr("Dimension lines"));
        if ui.selectable_label(tool == Tool::ConnDef, tr("Conn. def.")).on_hover_text(tr("Connection definition point: defines number, colour, cross-section and cable core of a connection")).clicked() {
            self.editor.set_tool(Tool::ConnDef);
        }
        if ui.selectable_label(tool == Tool::CableDef, tr("Cable def.")).on_hover_text(tr("Cable definition: draw a line across the wires that are cores of a cable")).clicked() {
            self.editor.set_tool(Tool::CableDef);
        }
        let names = graphics::layer_names(&self.prj);
        if !names.is_empty() || !self.editor.active_layer.is_empty() {
            ui.label(tr("Layer"));
            layer_combo(ui, "active_layer", &mut self.editor.active_layer, &names);
        }
        ui.toggle_value(&mut self.editor.autoconnect, tr("Auto-connect"))
            .on_hover_text(tr("Draw connections between facing connection points when symbols are placed or moved"));
        if ui.toggle_value(&mut self.editor.autoroute, tr("Route")).on_hover_text(tr("The wire tool routes around symbols and other wires (A)")).changed() {
            self.editor.cancel();
        }
    }

    /// Entries of the Insert menu for graphics, dimensions and definitions.
    pub(super) fn insert_menu_entries(&mut self, ui: &mut egui::Ui) {
        ui.menu_button(tr("Graphics"), |ui| {
            for (t, name) in [
                (Tool::Circle, tr("Circle")),
                (Tool::Ellipse, tr("Ellipse")),
                (Tool::Arc, tr("Arc")),
                (Tool::Polyline, tr("Polyline")),
                (Tool::Polygon, tr("Polygon")),
            ] {
                if ui.button(name).clicked() {
                    self.editor.set_tool(t);
                }
            }
        });
        ui.menu_button(tr("Dimension"), |ui| {
            for (k, name) in [(DimKind::Aligned, tr("Aligned")), (DimKind::Horizontal, tr("Horizontal")), (DimKind::Vertical, tr("Vertical"))] {
                if ui.button(name).clicked() {
                    self.editor.set_tool(Tool::Dimension(k));
                }
            }
        });
        if ui.button(tr("Image…")).on_hover_text(tr("PNG or JPEG bitmap, e.g. a logo or a photo")).clicked() {
            self.io.open(ui.ctx(), crate::io::Purpose::Image);
        }
        if ui.button(tr("Connection definition point")).clicked() {
            self.editor.set_tool(Tool::ConnDef);
        }
        if ui.button(tr("Cable definition")).clicked() {
            self.editor.set_tool(Tool::CableDef);
        }
    }

    /// Entries of the Edit menu for routing and definitions.
    pub(super) fn edit_menu_entries(&mut self, ui: &mut egui::Ui) {
        let has_sel = !self.editor.selection.is_empty();
        let ctx = ui.ctx().clone();
        if ui.add_enabled(has_sel, egui::Button::new(tr("Auto-connect selection"))).on_hover_text(tr("Connects free connection points of the selected symbols with facing ones")).clicked() {
            let n = routing::apply_autoconnect(&mut self.prj, self.page, &self.editor.selection);
            self.notify(&ctx, tr_fmt!("{} connections drawn automatically", n));
        }
        if ui.button(tr("Join collinear wires")).on_hover_text(tr("Joins wire pieces in a line where nothing else connects (current page)")).clicked() {
            let n = routing::merge_collinear(&mut self.prj.pages[self.page]);
            self.editor.selection.clear();
            self.notify(&ctx, tr_fmt!("{} wire pieces joined", n));
        }
        if ui.button(tr("Apply connection definitions")).on_hover_text(tr("All connections take the values of their connection definition points")).clicked() {
            let n = conndef::apply_all(&mut self.prj);
            self.notify(&ctx, tr_fmt!("{} wires changed", n));
        }
    }

    /// An image file was picked: it is inserted at the next click.
    pub(super) fn load_image(&mut self, ctx: &egui::Context, loaded: &Loaded) {
        let name = crate::io::short_name(&loaded.name).to_string();
        let Some(px) = graphics::image_pixels(&loaded.bytes) else {
            self.notify(ctx, tr_fmt!("{} is not a PNG or JPEG image", name));
            return;
        };
        let size = graphics::image_size(px, 60.0);
        let img = ImageItem {
            a: crate::geom::p(0.0, 0.0),
            b: size,
            file: crate::document::Document::new(&name, &loaded.bytes),
            px,
            layer: self.editor.active_layer.clone(),
            ..Default::default()
        };
        self.editor.set_tool(Tool::Insert);
        self.editor.insert = Some(ops::Clip { images: vec![img], ..ops::Clip::new() });
        self.notify(ctx, tr("Click into the page to place the image"));
    }

    /// Properties of texts, graphics, dimensions, images and definitions. Returns
    /// false if `id` is none of them.
    pub(super) fn graphic_properties(&mut self, ui: &mut egui::Ui, id: u64) -> bool {
        let pi = self.page;
        let names = graphics::layer_names(&self.prj);
        let cables: Vec<(String, Vec<String>)> = self.prj.cables.iter().map(|c| (c.tag.clone(), c.core_list())).collect();
        let pg = &mut self.prj.pages[pi];
        if let Some(t) = pg.texts.iter_mut().find(|t| t.id == id) {
            ui.label(tr("Text"));
            ui.add(egui::TextEdit::multiline(&mut t.text).desired_rows(3));
            ui.add(egui::Slider::new(&mut t.size, 1.5..=20.0).text(tr("Height (mm)")));
            color_field(ui, &mut t.color);
            ui.horizontal(|ui| {
                ui.label(tr("Layer"));
                layer_combo(ui, "tlayer", &mut t.layer, &names);
            });
            return true;
        }
        if let Some(s) = pg.shapes.iter_mut().find(|s| s.id == id) {
            ui.label(RichText::new(shape_title(s.kind)).strong());
            egui::Grid::new("shprops").num_columns(2).show(ui, |ui| {
                ui.label(tr("Line width"));
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut s.width).range(0.0..=5.0).speed(0.05).suffix(" mm"));
                    if s.width == 0.0 {
                        ui.weak(tr("normal"));
                    }
                });
                ui.end_row();
                ui.label(tr("Dashed"));
                ui.checkbox(&mut s.dashed, "");
                ui.end_row();
                if graphics::closed(s.kind) {
                    ui.label(tr("Filled"));
                    ui.checkbox(&mut s.fill, "");
                    ui.end_row();
                }
                if s.kind == ShapeKind::Arc {
                    ui.label(tr("Angle"));
                    ui.add(egui::DragValue::new(&mut s.sweep).range(-360.0..=360.0).suffix(" °"));
                    ui.end_row();
                }
                if s.kind == ShapeKind::Circle {
                    ui.label(tr("Radius"));
                    let mut r = s.a.dist(s.b);
                    if ui.add(egui::DragValue::new(&mut r).range(0.5..=500.0).speed(0.5).suffix(" mm")).changed() {
                        s.b = s.a + crate::geom::p(r, 0.0);
                    }
                    ui.end_row();
                }
                ui.label(tr("Layer"));
                layer_combo(ui, "slayer", &mut s.layer, &names);
                ui.end_row();
            });
            color_field(ui, &mut s.color);
            if matches!(s.kind, ShapeKind::Polyline | ShapeKind::Polygon) {
                ui.weak(tr_fmt!("{} corner points", s.pts.len()));
            }
            return true;
        }
        if let Some(d) = pg.dimensions.iter_mut().find(|d| d.id == id) {
            ui.label(RichText::new(tr("Dimension")).strong());
            egui::Grid::new("dimprops").num_columns(2).show(ui, |ui| {
                ui.label(tr("Direction"));
                egui::ComboBox::from_id_salt("dimkind").selected_text(dim_title(d.kind)).show_ui(ui, |ui| {
                    for k in [DimKind::Aligned, DimKind::Horizontal, DimKind::Vertical] {
                        ui.selectable_value(&mut d.kind, k, dim_title(k));
                    }
                });
                ui.end_row();
                ui.label(tr("Measured"));
                ui.label(format!("{:.2} mm", graphics::dim_length(d).abs()));
                ui.end_row();
                ui.label(tr("Scale"));
                ui.add(egui::DragValue::new(&mut d.scale).range(0.001..=10000.0).speed(0.1).prefix("× ")).on_hover_text(tr("Real length per millimetre on the page, e.g. 10 for a 1:10 drawing"));
                ui.end_row();
                ui.label(tr("Decimals"));
                ui.add(egui::DragValue::new(&mut d.decimals).range(0..=4));
                ui.end_row();
                ui.label(tr("Unit"));
                ui.add(egui::TextEdit::singleline(&mut d.unit).hint_text("mm").desired_width(60.0));
                ui.end_row();
                ui.label(tr("Text"));
                let measured = graphics::dim_text(&Dimension { text: String::new(), ..d.clone() });
                ui.add(egui::TextEdit::singleline(&mut d.text).hint_text(measured).desired_width(120.0))
                    .on_hover_text(tr("Shown instead of the measured value"));
                ui.end_row();
                ui.label(tr("Distance"));
                ui.add(egui::DragValue::new(&mut d.offset).speed(0.5).suffix(" mm"));
                ui.end_row();
                ui.label(tr("Layer"));
                layer_combo(ui, "dlayer", &mut d.layer, &names);
                ui.end_row();
            });
            return true;
        }
        if let Some(img) = pg.images.iter_mut().find(|i| i.id == id) {
            ui.label(RichText::new(tr("Image")).strong());
            ui.label(RichText::new(&img.file.name).monospace());
            ui.weak(tr_fmt!("{} × {} pixels, {}", img.px[0], img.px[1], crate::document::size_text(img.file.size())));
            let (mut w, mut h) = (img.b.x - img.a.x, img.b.y - img.a.y);
            egui::Grid::new("imgprops").num_columns(2).show(ui, |ui| {
                ui.label(tr("Width"));
                let cw = ui.add(egui::DragValue::new(&mut w).range(2.0..=1000.0).speed(0.5).suffix(" mm")).changed();
                ui.end_row();
                ui.label(tr("Height"));
                let ch = ui.add(egui::DragValue::new(&mut h).range(2.0..=1000.0).speed(0.5).suffix(" mm")).changed();
                ui.end_row();
                if cw {
                    h = graphics::image_size(img.px, w).y;
                } else if ch && img.px[1] > 0 {
                    w = h * img.px[0] as f32 / img.px[1] as f32;
                }
                if cw || ch {
                    img.b = img.a + crate::geom::p(w, h);
                }
                ui.label(tr("Layer"));
                layer_combo(ui, "ilayer", &mut img.layer, &names);
                ui.end_row();
            });
            ui.weak(tr("The aspect ratio is kept."));
            return true;
        }
        if pg.conn_defs.iter().any(|c| c.id == id) {
            self.conn_def_properties(ui, id, &cables);
            return true;
        }
        if pg.cable_defs.iter().any(|c| c.id == id) {
            self.cable_def_properties(ui, id, &cables);
            return true;
        }
        false
    }

    fn conn_def_properties(&mut self, ui: &mut egui::Ui, id: u64, cables: &[(String, Vec<String>)]) {
        let pi = self.page;
        ui.label(RichText::new(tr("Connection definition point")).strong());
        let pg = &self.prj.pages[pi];
        let Some(before) = pg.conn_defs.iter().find(|c| c.id == id).cloned() else { return };
        let on_wire = conndef::wire_at(pg, before.pos).map(|w| w.id);
        if let Some(net) = on_wire.and_then(|w| self.analysis.open.net_of_wire(w)) {
            let mut v: Vec<String> = net.pins.iter().map(|p| pin_designation(&self.prj, *p)).collect();
            v.sort_by(|a, b| natural_cmp(a, b));
            v.dedup();
            ui.label(tr("Connection:"));
            ui.label(RichText::new(v.join(" – ")).monospace());
        } else {
            ui.colored_label(egui::Color32::from_rgb(220, 140, 0), tr("Not on a wire"));
        }
        let auto = on_wire.map(|w| crate::potentials::props_of_wire(&self.prj, &self.analysis.netlist, &self.analysis.open, w)).unwrap_or_default();
        let mut cd = before.clone();
        egui::Grid::new("cdprops").num_columns(2).show(ui, |ui| {
            ui.label(tr("Wire number"));
            ui.add(egui::TextEdit::singleline(&mut cd.number).hint_text(tr("not numbered")).desired_width(90.0));
            ui.end_row();
            super::potentials_ui::wire_fields(ui, &mut cd.color, &mut cd.cross_section, &auto);
            ui.label(tr("Cable"));
            let shown = if cd.cable.is_empty() { "(none)".to_string() } else { cd.cable.clone() };
            egui::ComboBox::from_id_salt("cdcable").selected_text(shown).show_ui(ui, |ui| {
                if ui.selectable_label(cd.cable.is_empty(), "(none)").clicked() {
                    cd.cable.clear();
                    cd.core.clear();
                }
                for (tag, _) in cables {
                    if ui.selectable_label(cd.cable == *tag, tag).clicked() && cd.cable != *tag {
                        cd.cable = tag.clone();
                        cd.core.clear();
                    }
                }
            });
            ui.end_row();
            if let Some((_, cores)) = cables.iter().find(|(t, _)| *t == cd.cable) {
                ui.label(tr("Core"));
                egui::ComboBox::from_id_salt("cdcore").selected_text(cd.core.clone()).show_ui(ui, |ui| {
                    for c in cores {
                        ui.selectable_value(&mut cd.core, c.clone(), c);
                    }
                });
                ui.end_row();
            }
            ui.label(tr("Description"));
            ui.text_edit_singleline(&mut cd.description);
            ui.end_row();
        });
        let reapply = ui.button(tr("Apply to the connection")).on_hover_text(tr("The wires of the connection take these values again")).clicked();
        let changed = cd != before;
        if let Some(c) = self.prj.pages[pi].conn_defs.iter_mut().find(|c| c.id == id) {
            *c = cd;
        }
        if changed || reapply {
            conndef::apply(&mut self.prj, &self.analysis.open, pi, id);
        }
    }

    fn cable_def_properties(&mut self, ui: &mut egui::Ui, id: u64, cables: &[(String, Vec<String>)]) {
        let pi = self.page;
        ui.label(RichText::new(tr("Cable definition")).strong());
        let Some(mut def) = self.prj.pages[pi].cable_defs.iter().find(|c| c.id == id).cloned() else { return };
        let crossed = cabledef::crossed_wires(&self.prj.pages[pi], def.a, def.b);
        let mut apply = false;
        egui::Grid::new("cbdprops").num_columns(2).show(ui, |ui| {
            ui.label(tr("Cable"));
            egui::ComboBox::from_id_salt("cbdcable").selected_text(def.cable.clone()).show_ui(ui, |ui| {
                for (tag, _) in cables {
                    if ui.selectable_label(def.cable == *tag, tag).clicked() && def.cable != *tag {
                        def.cable = tag.clone();
                        apply = true;
                    }
                }
            });
            ui.end_row();
            ui.label(tr("Shield"));
            ui.checkbox(&mut def.shield, "");
            ui.end_row();
            ui.label(tr("Crossed wires"));
            ui.label(crossed.len().to_string());
            ui.end_row();
        });
        if let Some(c) = self.prj.cables.iter_mut().find(|c| c.tag == def.cable) {
            ui.separator();
            egui::Grid::new("cbdcable_data").num_columns(2).show(ui, |ui| {
                ui.label(tr("Type"));
                ui.text_edit_singleline(&mut c.cable_type);
                ui.end_row();
                ui.label(tr("Cores"));
                ui.text_edit_singleline(&mut c.cores).on_hover_text(tr("Core designations separated by blanks"));
                ui.end_row();
                ui.label(tr("Cross-section"));
                ui.text_edit_singleline(&mut c.cross_section);
                ui.end_row();
                ui.label(tr("Length"));
                ui.add(egui::DragValue::new(&mut c.length).range(0.0..=10000.0).speed(0.1).suffix(" m"));
                ui.end_row();
            });
            ui.horizontal_wrapped(|ui| {
                if ui.button(tr("Number cores")).on_hover_text(tr("Cores 1, 2, 3 … for the wires crossed, plus GNYE")).clicked() {
                    c.cores = cabledef::numbered_cores(crossed.len().max(1), true);
                    apply = true;
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("Assign crossed wires")).on_hover_text(tr("The crossed wires get free cores of the cable in order")).clicked() {
                apply = true;
            }
            if ui.button(tr("Draw another line for this cable")).clicked() {
                self.editor.cable_target = def.cable.clone();
                self.editor.set_tool(Tool::CableDef);
            }
        });
        if let Some(c) = self.prj.pages[pi].cable_defs.iter_mut().find(|c| c.id == id) {
            *c = def;
        }
        if apply {
            let n = cabledef::apply(&mut self.prj, pi, id);
            let ctx = ui.ctx().clone();
            self.notify(&ctx, tr_fmt!("{} wires are cores of the cable", n));
        }
    }

    /// Tag position, placed properties and placement scheme of a symbol.
    pub(super) fn placement_properties(&mut self, ui: &mut egui::Ui, pi: usize, si: usize) {
        let s = &self.prj.pages[pi].symbols[si];
        let values: Vec<String> = s.props.iter().map(|pp| props::text(&self.prj, pi, s, pp)).collect();
        let titles: Vec<String> = s.props.iter().map(|pp| properties::def(&self.prj, &pp.key).map(|d| d.title_of(&pp.key)).unwrap_or_else(|| pp.key.clone())).collect();
        let schemes: Vec<(String, String)> = props::all_schemes(&self.prj).map(|x| (x.name.clone(), x.title())).collect();
        let classes = properties::Object::Function(s.id).classes(&self.prj);
        let classes: Vec<properties::ObjectClass> =
            [properties::ObjectClass::Device, properties::ObjectClass::Part].into_iter().filter(|_| !s.tag.is_empty()).chain(classes).chain([properties::ObjectClass::Page, properties::ObjectClass::Project]).collect();
        let open = !s.props.is_empty() || s.tag_offset.is_some();
        egui::CollapsingHeader::new(tr("Property placement")).id_salt("placement").default_open(open).show(ui, |ui| {
            let added = super::properties_ui::key_picker(ui, &self.prj, &classes, false, tr("➕ Place property"));
            let mut save_scheme = None;
            let mut apply_scheme = None;
            let s = &mut self.prj.pages[pi].symbols[si];
            ui.horizontal(|ui| {
                if s.tag_offset.is_some() {
                    ui.weak(tr("Tag placed by hand"));
                    if ui.small_button(tr("Reset")).clicked() {
                        s.tag_offset = None;
                    }
                } else {
                    ui.weak(tr("Drag the tag of the selected symbol to place it."));
                }
            });
            let mut remove = None;
            egui::Grid::new("pprops").num_columns(4).show(ui, |ui| {
                for (i, pp) in s.props.iter_mut().enumerate() {
                    ui.label(&titles[i]).on_hover_text(&pp.key);
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut pp.size).range(1.0..=10.0).speed(0.1).suffix(" mm"));
                        super::properties_ui::anchor_combo(ui, ("ppanchor", i), &mut pp.anchor);
                        ui.checkbox(&mut pp.name, "").on_hover_text(tr("Shown as \"Name: value\""));
                    });
                    let v = values.get(i).cloned().unwrap_or_default();
                    if v.is_empty() {
                        ui.weak(tr("(empty)"));
                    } else {
                        ui.add(egui::Label::new(v).truncate());
                    }
                    if ui.small_button("🗑").clicked() {
                        remove = Some(i);
                    }
                    ui.end_row();
                }
            });
            if let Some(i) = remove {
                s.props.remove(i);
            }
            if let Some(k) = added {
                props::add(s, &k);
            }
            ui.horizontal(|ui| {
                ui.label(tr("Scheme"));
                let shown = schemes.iter().find(|x| x.0 == s.scheme).map(|x| x.1.clone()).unwrap_or_else(|| tr("(none)").into());
                egui::ComboBox::from_id_salt("ppscheme").selected_text(shown).show_ui(ui, |ui| {
                    for (name, title) in &schemes {
                        if ui.selectable_label(*name == s.scheme, title).clicked() {
                            apply_scheme = Some(name.clone());
                        }
                    }
                });
                let id = ui.scope_id().with("new_scheme");
                let mut name: String = ui.data_mut(|d| d.get_temp(id)).unwrap_or_default();
                ui.add(egui::TextEdit::singleline(&mut name).hint_text(tr("Scheme name")).desired_width(90.0));
                if ui.add_enabled(!s.props.is_empty() && !name.trim().is_empty(), egui::Button::new(tr("Save as scheme"))).on_hover_text(tr("Saves the placed properties as a placement scheme of the project")).clicked() {
                    save_scheme = Some(props::capture(s, &name));
                    name.clear();
                }
                ui.data_mut(|d| d.insert_temp(id, name));
            });
            if let Some(name) = apply_scheme {
                let ids = [self.prj.pages[pi].symbols[si].id];
                if let Err(e) = props::apply_to(&mut self.prj, &name, &ids) {
                    self.message = Some((e, ui.ctx().input(|i| i.time)));
                }
            }
            if let Some(scheme) = save_scheme {
                let name = scheme.name.clone();
                match props::define_scheme(&mut self.prj, scheme) {
                    Ok(()) => self.prj.pages[pi].symbols[si].scheme = name,
                    Err(e) => self.message = Some((e, ui.ctx().input(|i| i.time))),
                }
            }
        });
    }

    // ------------------------------------------------------------------ navigators

    pub(super) fn layers_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button(tr("➕ Layer")).clicked() {
                let mut n = 1;
                while graphics::layer(&self.prj, &tr_fmt!("Layer {}", n)).is_some() {
                    n += 1;
                }
                self.prj.layers.push(Layer::new(&tr_fmt!("Layer {}", n)));
            }
        });
        ui.add(egui::Label::new(RichText::new(tr("Layers of graphics, texts, dimensions and images")).weak()).wrap());
        // Layers only used by items get a definition so they can be switched.
        for name in graphics::layer_names(&self.prj) {
            if graphics::layer(&self.prj, &name).is_none() {
                self.prj.layers.push(Layer::new(&name));
            }
        }
        let counts: Vec<usize> = self.prj.layers.iter().map(|l| graphics::layer_count(&self.prj, &l.name)).collect();
        let default_count = graphics::layer_count(&self.prj, "");
        let mut rename: Option<(String, String)> = None;
        let mut remove: Option<String> = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::Grid::new("layers").num_columns(6).striped(true).show(ui, |ui| {
                ui.label("");
                ui.label(RichText::new(tr("Name")).strong());
                ui.label(RichText::new(tr("Visible")).strong());
                ui.label(RichText::new(tr("Locked")).strong());
                ui.label(RichText::new(tr("Print")).strong());
                ui.label("");
                ui.end_row();
                ui.radio_value(&mut self.editor.active_layer, String::new(), "").on_hover_text(tr("New items go onto this layer"));
                ui.label(tr("(default)"));
                ui.weak(default_count.to_string());
                ui.end_row();
                for (i, l) in self.prj.layers.iter_mut().enumerate() {
                    ui.radio_value(&mut self.editor.active_layer, l.name.clone(), "").on_hover_text(tr("New items go onto this layer"));
                    let mut name = l.name.clone();
                    if ui.add(egui::TextEdit::singleline(&mut name).desired_width(90.0)).changed() && !name.is_empty() {
                        rename = Some((l.name.clone(), name));
                    }
                    ui.checkbox(&mut l.visible, "");
                    ui.checkbox(&mut l.locked, "");
                    ui.checkbox(&mut l.print, "");
                    ui.horizontal(|ui| {
                        let mut c = l.color.is_some();
                        if ui.checkbox(&mut c, "").on_hover_text(tr("Colour of the items without their own")).changed() {
                            l.color = c.then_some([0, 120, 200]);
                        }
                        if let Some(col) = &mut l.color {
                            ui.color_edit_button_srgb(col);
                        }
                        ui.weak(counts[i].to_string()).on_hover_text(tr("Items on the layer"));
                        if ui.small_button("🗑").on_hover_text(tr("Delete the layer; its items move to the default layer")).clicked() {
                            remove = Some(l.name.clone());
                        }
                    });
                    ui.end_row();
                }
            });
        });
        if let Some((old, new)) = rename {
            if graphics::layer(&self.prj, &new).is_none() {
                graphics::rename_layer(&mut self.prj, &old, &new);
                if self.editor.active_layer == old {
                    self.editor.active_layer = new;
                }
            }
        }
        if let Some(name) = remove {
            graphics::remove_layer(&mut self.prj, &name);
            if self.editor.active_layer == name {
                self.editor.active_layer.clear();
            }
        }
        // Selected items on hidden or locked layers are deselected.
        let page = &self.prj.pages[self.page];
        let prj = &self.prj;
        self.editor.selection.retain(|id| graphics::item_layer(page, *id).is_none_or(|l| graphics::shown(prj, l, false) && !graphics::locked(prj, l)));
    }

    pub(super) fn connections_tab(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::TextEdit::singleline(&mut self.connection_filter).hint_text(tr("Filter connections…")));
        let f = self.connection_filter.to_lowercase();
        // Number, connected points, first wire and all wires (page, id).
        type Row = (String, String, (usize, u64), Vec<(usize, u64)>);
        let mut rows: Vec<Row> = Vec::new();
        for net in self.analysis.open.nets.iter().filter(|n| !n.wires.is_empty()) {
            let number = wirenum::number_of(&self.prj, net);
            let mut pins: Vec<String> = net.pins.iter().map(|p| pin_designation(&self.prj, *p)).collect();
            pins.sort_by(|a, b| natural_cmp(a, b));
            pins.dedup();
            let text = pins.join(" – ");
            if !f.is_empty() && !number.to_lowercase().contains(&f) && !text.to_lowercase().contains(&f) && !net.name.to_lowercase().contains(&f) {
                continue;
            }
            rows.push((number, text, net.wires[0], net.wires.clone()));
        }
        rows.sort_by(|a, b| natural_cmp(&a.0, &b.0).then_with(|| natural_cmp(&a.1, &b.1)));
        ui.weak(tr_fmt!("{} connections", rows.len()));
        let mut jump = None;
        let mut select: Option<Vec<(usize, u64)>> = None;
        let mut define: Option<(usize, u64)> = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, 18.0, rows.len(), |ui, range| {
            for (number, text, first, wires) in &rows[range] {
                let title = if number.is_empty() { format!("–  {text}") } else { format!("{number}  {text}") };
                let r = ui.add(egui::Label::new(RichText::new(title).monospace()).sense(egui::Sense::click()).truncate());
                if r.clicked() {
                    jump = Some(*first);
                }
                r.context_menu(|ui| {
                    if ui.button(tr("Show")).clicked() {
                        jump = Some(*first);
                    }
                    if ui.button(tr("Select wires on this page")).clicked() {
                        select = Some(wires.clone());
                    }
                    if ui.button(tr("Place connection definition point")).on_hover_text(tr("In the middle of the first wire")).clicked() {
                        define = Some(*first);
                    }
                });
            }
        });
        if let Some((pg, id)) = jump {
            self.goto(pg, id);
        }
        if let Some(wires) = select {
            self.editor.selection = wires.iter().filter(|w| w.0 == self.page).map(|w| w.1).collect();
        }
        if let Some((pg, wid)) = define {
            if let Some(w) = self.prj.pages[pg].wires.iter().find(|w| w.id == wid) {
                let mid = crate::geom::p((w.a.x + w.b.x) / 2.0, (w.a.y + w.b.y) / 2.0);
                let mut cd = conndef::adopt(&self.prj, &self.analysis.open, pg, mid);
                cd.id = self.prj.new_id();
                let id = cd.id;
                self.prj.pages[pg].conn_defs.push(cd);
                self.goto(pg, id);
            }
        }
    }

    pub(super) fn cables_tab(&mut self, ui: &mut egui::Ui) {
        let structure = structure::page_structure(&self.prj.pages[self.page]);
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("➕ New cable")).on_hover_text(tr("In the structure of the current page")).clicked() {
                let tag = ops::add_cable(&mut self.prj, &structure);
                self.editor.cable_target = tag;
            }
            if ui.button(tr("Cables…")).clicked() {
                self.show_cables = true;
            }
        });
        // Cores in use: cable -> (core, page, wire id)
        let mut cores: std::collections::HashMap<&str, Vec<(String, usize, u64)>> = Default::default();
        for (pi, pg) in self.prj.pages.iter().enumerate() {
            for w in pg.wires.iter().filter(|w| !w.cable.is_empty()) {
                cores.entry(w.cable.as_str()).or_default().push((w.core.clone(), pi, w.id));
            }
        }
        let defs: Vec<(String, usize, u64)> = self
            .prj
            .pages
            .iter()
            .enumerate()
            .flat_map(|(pi, pg)| pg.cable_defs.iter().map(move |c| (c.cable.clone(), pi, c.id)))
            .collect();
        let mut jump = None;
        let mut draw = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for c in &self.prj.cables {
                let used = cores.get(c.tag.as_str()).cloned().unwrap_or_default();
                let total = c.core_list().len();
                let head = format!("{}   {}/{}  {}", c.tag, used.len(), total, c.cable_type);
                let target = self.editor.cable_target == c.tag;
                let header = egui::CollapsingHeader::new(RichText::new(head).monospace().strong()).id_salt(("cable", &c.tag)).show(ui, |ui| {
                    if ui.selectable_label(target, tr("Draw cable definition")).on_hover_text(tr("Draw a line across the wires that are cores of this cable")).clicked() {
                        draw = Some(c.tag.clone());
                    }
                    let mut used = used.clone();
                    used.sort_by(|a, b| natural_cmp(&a.0, &b.0));
                    for (core, pi, wid) in &used {
                        let page = structure::page_full_name(&self.prj.pages[*pi]);
                        if ui.link(tr_fmt!("{}  ›  page {}", core, page)).clicked() {
                            jump = Some((*pi, *wid));
                        }
                    }
                    for free in c.core_list().iter().filter(|k| !used.iter().any(|u| &u.0 == *k)) {
                        ui.weak(tr_fmt!("{}  (free)", free));
                    }
                    for (_, pi, id) in defs.iter().filter(|d| d.0 == c.tag) {
                        let page = structure::page_full_name(&self.prj.pages[*pi]);
                        if ui.link(tr_fmt!("Cable definition on page {}", page)).clicked() {
                            jump = Some((*pi, *id));
                        }
                    }
                });
                header.header_response.context_menu(|ui| {
                    if ui.button(tr("Draw cable definition")).clicked() {
                        draw = Some(c.tag.clone());
                    }
                });
            }
            if self.prj.cables.is_empty() {
                ui.weak(tr("No cables yet. Draw a cable definition across wires or add a cable."));
            }
        });
        if let Some(tag) = draw {
            self.editor.cable_target = tag;
            self.editor.set_tool(Tool::CableDef);
        }
        if let Some((pg, id)) = jump {
            self.goto(pg, id);
        }
    }

    pub(super) fn device_action(&mut self, tag: &str, action: DeviceAction) {
        match action {
            DeviceAction::Select => {
                self.editor.selection = self.prj.pages[self.page].symbols.iter().filter(|s| s.tag == tag).map(|s| s.id).collect();
            }
            DeviceAction::Place => self.editor.place_tag = Some(tag.to_string()),
            DeviceAction::StopPlacing => self.editor.place_tag = None,
            DeviceAction::Rename => self.rename_device = Some((tag.to_string(), tag.to_string())),
        }
    }

    /// Dialog to rename a device (all its functions).
    pub(super) fn rename_device_window(&mut self, ctx: &egui::Context) {
        let Some((old, mut new)) = self.rename_device.take() else { return };
        let mut open = true;
        let mut done = false;
        egui::Window::new(tr("Rename device")).open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
            ui.label(tr_fmt!("New tag for {}", old));
            let r = ui.text_edit_singleline(&mut new);
            let exists = new != old && self.prj.all_tags().contains(&new);
            if exists {
                ui.colored_label(egui::Color32::from_rgb(220, 60, 60), tr("This tag is already used"));
            }
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (ui.add_enabled(!exists && !new.is_empty(), egui::Button::new(tr("Rename"))).clicked() || (enter && !exists && !new.is_empty())) && new != old {
                ops::rename_device(&mut self.prj, &old, &new);
                done = true;
            }
        });
        if open && !done {
            self.rename_device = Some((old, new));
        }
    }
}

/// What the context menu of a device in the navigator asks for.
#[derive(Clone, Copy)]
pub(super) enum DeviceAction {
    Select,
    Place,
    StopPlacing,
    Rename,
}

/// Context menu of a device in the device navigator.
pub(super) fn device_menu(ui: &mut egui::Ui, placing: bool) -> Option<DeviceAction> {
    let mut a = None;
    if ui.button(tr("Select on this page")).clicked() {
        a = Some(DeviceAction::Select);
    }
    if ui.button(tr("Place functions of this device")).on_hover_text(tr("Symbols placed next get this device tag (contacts are numbered on)")).clicked() {
        a = Some(DeviceAction::Place);
    }
    if placing && ui.button(tr("Stop placing for this device")).clicked() {
        a = Some(DeviceAction::StopPlacing);
    }
    if ui.button(tr("Rename…")).clicked() {
        a = Some(DeviceAction::Rename);
    }
    if a.is_some() {
        ui.close();
    }
    a
}

fn shape_title(kind: ShapeKind) -> &'static str {
    match kind {
        ShapeKind::Line => tr("Graphic line"),
        ShapeKind::Rect => tr("Graphic rectangle"),
        ShapeKind::Circle => tr("Circle"),
        ShapeKind::Ellipse => tr("Ellipse"),
        ShapeKind::Arc => tr("Arc"),
        ShapeKind::Polyline => tr("Polyline"),
        ShapeKind::Polygon => tr("Polygon"),
    }
}

fn dim_title(kind: DimKind) -> &'static str {
    match kind {
        DimKind::Aligned => tr("Aligned"),
        DimKind::Horizontal => tr("Horizontal"),
        DimKind::Vertical => tr("Vertical"),
    }
}
