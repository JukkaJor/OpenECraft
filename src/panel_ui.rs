//! Panel layout workspace: enclosure navigator, 2D mounting plate editor, 3D view
//! and property editor.

use crate::editor::{EguiDraw, Palette};
use crate::geom::{p, P};
use crate::model::{natural_cmp, Project};
use crate::panel::{self, Enclosure, ItemKind, PanelItem, PLATE_MARGIN, PLATE_THICKNESS, PLATE_Z};
use crate::parts::{self, PartsDb};
use crate::render::{self, Draw, Ink};
use crate::reports::Analysis;
use egui::{vec2, Color32, Key, PointerButton, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq)]
pub enum Tool {
    Select,
    Rail,
    Duct,
    PlaceDevice(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum View {
    Front,
    ThreeD,
}

struct Camera {
    yaw: f32,
    pitch: f32,
    dist: f32,
    target: [f32; 3],
}

struct MoveDrag {
    start: P,
    originals: Vec<(u64, f32, f32)>,
}

struct NewEnclosure {
    name: String,
    part: String,
    width: f32,
    height: f32,
    depth: f32,
}

pub struct PanelUi {
    pub enclosure: usize,
    pub tool: Tool,
    pub view: View,
    pub selection: BTreeSet<u64>,
    pub show_wires: bool,
    offset: Vec2,
    zoom: f32,
    fit_pending: bool,
    start: Option<P>,
    drag: Option<MoveDrag>,
    cam: Option<Camera>,
    device_filter: String,
    new_enclosure: Option<NewEnclosure>,
    pub status: String,
    /// Requests for the application.
    pub goto_schematic: Option<String>,
    pub add_page: Option<u64>,
}

impl Default for PanelUi {
    fn default() -> Self {
        PanelUi {
            enclosure: 0,
            tool: Tool::Select,
            view: View::Front,
            selection: BTreeSet::new(),
            show_wires: true,
            offset: vec2(20.0, 20.0),
            zoom: 0.5,
            fit_pending: true,
            start: None,
            drag: None,
            cam: None,
            device_filter: String::new(),
            new_enclosure: None,
            status: String::new(),
            goto_schematic: None,
            add_page: None,
        }
    }
}

/// Drawing layer of enclosure walls that are seen from the outside.
const OUTER_WALLS: u8 = 6;

const WIRE_COLOR: Color32 = Color32::from_rgb(40, 110, 230);

impl PanelUi {
    /// True while a gesture is in progress (no undo step yet).
    pub fn busy(&self) -> bool {
        self.start.is_some() || self.drag.is_some()
    }

    fn set_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.start = None;
    }

    fn enclosure_index(&mut self, prj: &Project) -> Option<usize> {
        if prj.layout.enclosures.is_empty() {
            return None;
        }
        self.enclosure = self.enclosure.min(prj.layout.enclosures.len() - 1);
        Some(self.enclosure)
    }

    /// Selects a placed device (e.g. coming from the schematic).
    pub fn show_device(&mut self, prj: &Project, tag: &str) -> bool {
        match prj.layout.find_device(tag) {
            Some((ei, item)) => {
                self.enclosure = ei;
                self.selection = BTreeSet::from([item.id]);
                true
            }
            None => false,
        }
    }

    // ------------------------------------------------------------------ navigator

    pub fn navigator(&mut self, ui: &mut egui::Ui, prj: &mut Project, db: &PartsDb) {
        ui.horizontal(|ui| {
            ui.heading("Enclosures");
            if ui.button("➕").on_hover_text("New enclosure").clicked() {
                let part = if db.get(panel::ENCLOSURE_PART).is_some() { panel::ENCLOSURE_PART } else { "" };
                let mut ne = NewEnclosure {
                    name: format!("+CAB{}", prj.layout.enclosures.len() + 1),
                    part: part.into(),
                    width: 600.0,
                    height: 800.0,
                    depth: 250.0,
                };
                apply_enclosure_part(&mut ne, db);
                self.new_enclosure = Some(ne);
            }
        });
        let mut remove = None;
        for (i, e) in prj.layout.enclosures.iter().enumerate() {
            ui.horizontal(|ui| {
                let label = format!("{}  {:.0}×{:.0}×{:.0}", e.name, e.width, e.height, e.depth);
                if ui.selectable_label(i == self.enclosure, label).clicked() {
                    self.enclosure = i;
                    self.selection.clear();
                    self.fit_pending = true;
                }
                if ui.small_button("🗑").on_hover_text("Delete enclosure").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            prj.layout.enclosures.remove(i);
            self.selection.clear();
        }
        if prj.layout.enclosures.is_empty() {
            ui.weak("No enclosure yet – create one with ➕.");
            return;
        }
        ui.separator();
        ui.heading("Devices to place");
        ui.add(egui::TextEdit::singleline(&mut self.device_filter).hint_text("Filter…"));
        let todo = panel::devices_to_place(prj);
        let f = self.device_filter.to_lowercase();
        egui::ScrollArea::vertical().id_salt("todo").max_height(ui.available_height() * 0.5).auto_shrink([false, true]).show(ui, |ui| {
            for d in todo.iter().filter(|d| f.is_empty() || d.tag.to_lowercase().contains(&f)) {
                let active = self.tool == Tool::PlaceDevice(d.tag.clone());
                let text = format!("{}   {:.0}×{:.0}×{:.0}", d.tag, d.w, d.h, d.d);
                if ui.selectable_label(active, text).on_hover_text(&d.part).clicked() {
                    self.set_tool(Tool::PlaceDevice(d.tag.clone()));
                    if self.view == View::ThreeD {
                        self.view = View::Front;
                    }
                }
            }
            if todo.is_empty() {
                ui.weak("All devices with mounting data are placed.");
            }
        });
        ui.separator();
        ui.heading("Placed");
        let Some(ei) = self.enclosure_index(prj) else { return };
        let mut placed: Vec<&PanelItem> = prj.layout.enclosures[ei].items.iter().filter(|i| i.kind == ItemKind::Device).collect();
        placed.sort_by(|a, b| natural_cmp(&a.tag, &b.tag));
        egui::ScrollArea::vertical().id_salt("placed").auto_shrink([false, false]).show(ui, |ui| {
            for it in placed {
                if ui.selectable_label(self.selection.contains(&it.id), &it.tag).clicked() {
                    self.selection = BTreeSet::from([it.id]);
                }
            }
        });
    }

    // ------------------------------------------------------------------ properties

    pub fn properties(&mut self, ui: &mut egui::Ui, prj: &mut Project, an: &Analysis) {
        ui.heading("Properties");
        let Some(ei) = self.enclosure_index(prj) else { return };
        let sel: Vec<u64> = self.selection.iter().copied().collect();
        if sel.len() > 1 {
            ui.label(format!("{} items selected", sel.len()));
            ui.horizontal(|ui| {
                if ui.button("⟳ Rotate").clicked() {
                    rotate(&mut prj.layout.enclosures[ei], &self.selection);
                }
                if ui.button("🗑 Delete").clicked() {
                    prj.layout.enclosures[ei].items.retain(|i| !self.selection.contains(&i.id));
                    self.selection.clear();
                }
            });
            return;
        }
        if let Some(id) = sel.first().copied() {
            let e = &mut prj.layout.enclosures[ei];
            let Some(idx) = e.items.iter().position(|i| i.id == id) else {
                self.selection.clear();
                return;
            };
            let mut it = e.items[idx].clone();
            ui.label(RichText::new(it.kind.title()).strong());
            egui::Grid::new("pitem").num_columns(2).spacing([10.0, 5.0]).show(ui, |ui| {
                if it.kind == ItemKind::Device {
                    ui.label("Device");
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&it.tag).monospace().strong());
                        if ui.small_button("Show in schematic").clicked() {
                            self.goto_schematic = Some(it.tag.clone());
                        }
                    });
                    ui.end_row();
                }
                ui.label("Part");
                ui.label(RichText::new(if it.part.is_empty() { "–" } else { &it.part }).monospace());
                ui.end_row();
                ui.label("X / Y");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut it.x).speed(1.0).suffix(" mm"));
                    ui.add(egui::DragValue::new(&mut it.y).speed(1.0).suffix(" mm"));
                });
                ui.end_row();
                if it.kind == ItemKind::Device {
                    ui.label("Width × height");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut it.w).speed(0.5).range(1.0..=5000.0));
                        ui.add(egui::DragValue::new(&mut it.h).speed(0.5).range(1.0..=5000.0));
                    });
                    ui.end_row();
                    ui.label("Depth / mounted at");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut it.d).speed(0.5).range(1.0..=2000.0));
                        ui.add(egui::DragValue::new(&mut it.z).speed(0.5).range(0.0..=2000.0).prefix("z "));
                    });
                    ui.end_row();
                } else {
                    ui.label("Length");
                    let mut len = it.length();
                    if ui.add(egui::DragValue::new(&mut len).speed(5.0).range(10.0..=5000.0).suffix(" mm")).changed() {
                        if it.horizontal() {
                            it.w = len;
                        } else {
                            it.h = len;
                        }
                    }
                    ui.end_row();
                    ui.label("Orientation");
                    ui.label(if it.horizontal() { "horizontal" } else { "vertical" });
                    ui.end_row();
                }
            });
            ui.horizontal(|ui| {
                if ui.button("⟳ Rotate").on_hover_text("R").clicked() {
                    it.rotate();
                }
                if ui.button("🗑 Delete").on_hover_text("Del").clicked() {
                    self.selection.clear();
                }
            });
            if self.selection.is_empty() {
                e.items.remove(idx);
            } else if it != e.items[idx] {
                e.items[idx] = it;
            }
            // Wires at this device
            if let Some(it) = prj.layout.enclosures[ei].items.iter().find(|i| i.id == id) {
                let wires: Vec<_> = an.wiring.iter().filter(|w| w.items.0 == it.id || w.items.1 == it.id).collect();
                if !wires.is_empty() {
                    ui.separator();
                    ui.label(RichText::new("Wires").strong());
                    egui::Grid::new("pwires").striped(true).num_columns(2).show(ui, |ui| {
                        for w in wires {
                            ui.label(format!("{} – {}", w.from, w.to));
                            ui.label(format!("{:.0} mm", w.length));
                            ui.end_row();
                        }
                    });
                }
            }
            return;
        }
        // Enclosure properties
        let e = &mut prj.layout.enclosures[ei];
        ui.label(RichText::new("Enclosure").strong());
        egui::Grid::new("penc").num_columns(2).spacing([10.0, 5.0]).show(ui, |ui| {
            ui.label("Location");
            ui.text_edit_singleline(&mut e.name);
            ui.end_row();
            ui.label("Part");
            ui.label(RichText::new(if e.part.is_empty() { "–" } else { &e.part }).monospace());
            ui.end_row();
            ui.label("Width × height");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut e.width).speed(5.0).range(100.0..=5000.0));
                ui.add(egui::DragValue::new(&mut e.height).speed(5.0).range(100.0..=5000.0));
            });
            ui.end_row();
            ui.label("Depth");
            ui.add(egui::DragValue::new(&mut e.depth).speed(5.0).range(60.0..=2000.0).suffix(" mm"));
            ui.end_row();
            let (pw, ph) = e.plate_size();
            ui.label("Mounting plate");
            ui.label(format!("{pw:.0} × {ph:.0} mm"));
            ui.end_row();
        });
        ui.horizontal(|ui| {
            ui.label("Wire reserve per connection");
            ui.add(egui::DragValue::new(&mut prj.layout.wire_reserve).speed(5.0).range(0.0..=2000.0).suffix(" mm"));
        });
        let e = &prj.layout.enclosures[ei];
        let devices = e.items.iter().filter(|i| i.kind == ItemKind::Device).count();
        let wires: Vec<_> = an.wiring.iter().filter(|w| w.enclosure == ei).collect();
        let total: f32 = wires.iter().map(|w| w.length).sum();
        ui.separator();
        ui.label(format!("{devices} devices placed, {} wires routed ({:.1} m)", wires.len(), total / 1000.0));
        let problems = panel::collisions(e).len() + panel::outside(e).len();
        if problems > 0 {
            ui.colored_label(Color32::from_rgb(220, 60, 60), format!("{problems} placement problems (red) – see Check"));
        }
        if ui.button("➕ Insert mounting layout page").clicked() {
            self.add_page = Some(e.id);
        }
    }

    // ------------------------------------------------------------------ dialogs

    pub fn dialogs(&mut self, ctx: &egui::Context, prj: &mut Project, db: &PartsDb) {
        let Some(ne) = &mut self.new_enclosure else { return };
        let mut open = true;
        let mut create = false;
        let enclosures: Vec<&parts::Part> = db
            .parts
            .iter()
            .chain(prj.parts.iter())
            .filter(|p| p.subgroup == "Enclosure" && p.width > 0.0)
            .collect();
        egui::Window::new("New enclosure").open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
            egui::Grid::new("newenc").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Location");
                ui.text_edit_singleline(&mut ne.name);
                ui.end_row();
                ui.label("Part");
                let before = ne.part.clone();
                egui::ComboBox::from_id_salt("encpart")
                    .selected_text(if ne.part.is_empty() { "(custom size)".to_string() } else { ne.part.clone() })
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut ne.part, String::new(), "(custom size)");
                        for p in &enclosures {
                            ui.selectable_value(&mut ne.part, p.number.clone(), format!("{} – {}", p.number, p.description));
                        }
                    });
                if ne.part != before {
                    apply_enclosure_part(ne, db);
                }
                ui.end_row();
                let custom = ne.part.is_empty();
                ui.label("Width × height × depth");
                ui.add_enabled_ui(custom, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut ne.width).speed(5.0).range(100.0..=5000.0));
                        ui.add(egui::DragValue::new(&mut ne.height).speed(5.0).range(100.0..=5000.0));
                        ui.add(egui::DragValue::new(&mut ne.depth).speed(5.0).range(60.0..=2000.0));
                    });
                });
                ui.end_row();
            });
            if ui.button("✔ Create").clicked() {
                create = true;
            }
        });
        if create {
            let ne = self.new_enclosure.take().unwrap();
            if !ne.part.is_empty() {
                parts::ensure_in_project(prj, db, &ne.part);
            }
            let mut e = panel::new_enclosure(prj, &ne.part, &ne.name);
            if ne.part.is_empty() {
                e.width = ne.width;
                e.height = ne.height;
                e.depth = ne.depth;
            }
            prj.layout.enclosures.push(e);
            self.enclosure = prj.layout.enclosures.len() - 1;
            self.selection.clear();
            self.fit_pending = true;
        } else if !open {
            self.new_enclosure = None;
        }
    }

    // ------------------------------------------------------------------ canvas

    pub fn canvas(&mut self, ui: &mut egui::Ui, prj: &mut Project, an: &Analysis, db: &PartsDb) {
        let Some(ei) = self.enclosure_index(prj) else {
            ui.centered_and_justified(|ui| {
                ui.label("Create an enclosure (➕ in the navigator on the left) to start the panel layout.");
            });
            return;
        };
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.view, View::Front, "Front view (2D)");
            ui.selectable_value(&mut self.view, View::ThreeD, "3D");
            ui.separator();
            if self.view == View::Front {
                let tool = self.tool.clone();
                for (t, label, tip) in [
                    (Tool::Select, "⬉ Select", "Select / move (Esc)"),
                    (Tool::Rail, "Mounting rail", "Click start and end point"),
                    (Tool::Duct, "Cable duct", "Click start and end point"),
                ] {
                    if ui.selectable_label(tool == t, label).on_hover_text(tip).clicked() {
                        self.set_tool(t);
                    }
                }
                if let Tool::PlaceDevice(tag) = &tool {
                    ui.label(RichText::new(format!("Placing {tag}")).strong());
                }
                ui.separator();
            }
            ui.checkbox(&mut self.show_wires, "Wires");
            if ui.button("⛶ Fit").clicked() {
                self.fit_pending = true;
                self.cam = None;
            }
        });
        ui.separator();
        match self.view {
            View::Front => self.front_view(ui, prj, ei, an, db),
            View::ThreeD => self.view_3d(ui, prj, ei, an),
        }
    }

    fn s2w(&self, rect: Rect, s: Pos2) -> P {
        p((s.x - rect.min.x - self.offset.x) / self.zoom, (s.y - rect.min.y - self.offset.y) / self.zoom)
    }

    fn front_view(&mut self, ui: &mut egui::Ui, prj: &mut Project, ei: usize, an: &Analysis, db: &PartsDb) {
        let pal = Palette::new(ui.visuals().dark_mode);
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = resp.rect;
        let (ew, eh) = (prj.layout.enclosures[ei].width, prj.layout.enclosures[ei].height);
        if self.fit_pending && rect.width() > 100.0 && rect.height() > 100.0 {
            self.zoom = ((rect.width() - 40.0) / ew).min((rect.height() - 40.0) / eh).max(0.05);
            self.offset = vec2((rect.width() - ew * self.zoom) / 2.0, (rect.height() - eh * self.zoom) / 2.0);
            self.fit_pending = false;
        }
        // Zoom / pan
        if resp.hovered() {
            let (wheel, zoom_delta, ptr) = ui.input(|i| (wheel_delta(i), i.zoom_delta(), i.pointer.hover_pos()));
            if let Some(ptr) = ptr {
                let factor = zoom_delta * (wheel * 0.0025).exp();
                if (factor - 1.0).abs() > 1e-4 {
                    let before = self.s2w(rect, ptr);
                    self.zoom = (self.zoom * factor).clamp(0.05, 20.0);
                    self.offset = ptr - rect.min - vec2(before.x, before.y) * self.zoom;
                }
            }
        }
        if resp.dragged_by(PointerButton::Middle) || resp.dragged_by(PointerButton::Secondary) {
            self.offset += resp.drag_delta();
        }
        let ptr = ui.input(|i| i.pointer.hover_pos()).filter(|q| rect.contains(*q));
        // Plate coordinates of the pointer
        let plate = ptr.map(|s| self.s2w(rect, s) - p(PLATE_MARGIN, PLATE_MARGIN));
        let snapped = plate.map(|q| p(panel::snap(q.x), panel::snap(q.y)));

        // Keyboard
        if !ui.ctx().egui_wants_keyboard_input() {
            let (esc, del, r, f) = ui.input(|i| {
                (i.key_pressed(Key::Escape), i.key_pressed(Key::Delete), i.key_pressed(Key::R) && i.modifiers.is_none(), i.key_pressed(Key::F) && i.modifiers.is_none())
            });
            let e = &mut prj.layout.enclosures[ei];
            if esc {
                if self.start.is_some() {
                    self.start = None;
                } else if self.tool != Tool::Select {
                    self.set_tool(Tool::Select);
                } else {
                    self.selection.clear();
                }
            }
            if del && !self.selection.is_empty() {
                e.items.retain(|i| !self.selection.contains(&i.id));
                self.selection.clear();
            }
            if r {
                rotate(e, &self.selection);
            }
            if f {
                self.fit_pending = true;
            }
        }
        if resp.clicked_by(PointerButton::Secondary) {
            if self.start.is_some() {
                self.start = None;
            } else {
                self.set_tool(Tool::Select);
            }
        }

        // Mouse
        match self.tool.clone() {
            Tool::Select => self.select_and_move(ui, &resp, prj, ei, rect),
            Tool::Rail | Tool::Duct => {
                if let (true, Some(sn)) = (resp.clicked_by(PointerButton::Primary), snapped) {
                    match self.start.take() {
                        None => self.start = Some(sn),
                        Some(a) if !a.same(sn) => {
                            let (kind, part) = if self.tool == Tool::Rail { (ItemKind::Rail, panel::RAIL_PART) } else { (ItemKind::Duct, panel::DUCT_PART) };
                            if db.get(part).is_some() {
                                parts::ensure_in_project(prj, db, part);
                            }
                            let item = panel::new_profile(prj, kind, part, a, sn);
                            prj.layout.enclosures[ei].items.push(item);
                        }
                        Some(_) => {}
                    }
                }
            }
            Tool::PlaceDevice(tag) => {
                if let (true, Some(q)) = (resp.clicked_by(PointerButton::Primary), plate) {
                    let size = panel::device_size(prj, &tag);
                    let (w, h) = size.map(|s| (s.w, s.h)).unwrap_or((panel::DEFAULT_DEVICE.0, panel::DEFAULT_DEVICE.1));
                    if let Some(id) = panel::place_device(prj, ei, &tag, p(q.x - w / 2.0, q.y - h / 2.0)) {
                        self.selection = BTreeSet::from([id]);
                    }
                    // Continue with the next unplaced device.
                    let next = panel::devices_to_place(prj).into_iter().next();
                    self.set_tool(next.map(|d| Tool::PlaceDevice(d.tag)).unwrap_or(Tool::Select));
                }
            }
        }

        // Draw
        let e = &prj.layout.enclosures[ei];
        painter.rect_filled(rect, 0.0, pal.canvas);
        let origin = rect.min + self.offset;
        let problems = panel::problem_items(e);
        let mut d = EguiDraw { painter: &painter, origin, zoom: self.zoom, pal: &pal, force: None };
        render::draw_enclosure(&mut d, e, p(0.0, 0.0), 1.0, &self.selection, &problems);
        let to_screen = |q: P| origin + vec2(q.x + PLATE_MARGIN, q.y + PLATE_MARGIN) * self.zoom;
        if self.show_wires {
            for w in an.wiring.iter().filter(|w| w.enclosure == ei) {
                let pts: Vec<Pos2> = w.route.points.iter().map(|q| to_screen(*q)).collect();
                painter.add(Shape::line(pts, Stroke::new(1.0, WIRE_COLOR.gamma_multiply(0.7))));
            }
        }
        // Previews
        let mut g = EguiDraw { painter: &painter, origin, zoom: self.zoom, pal: &pal, force: Some(Ink::Ghost) };
        let m = p(PLATE_MARGIN, PLATE_MARGIN);
        match (&self.tool, snapped, plate) {
            (Tool::Rail | Tool::Duct, Some(sn), _) => {
                if let Some(a) = self.start {
                    let kind = if self.tool == Tool::Rail { ItemKind::Rail } else { ItemKind::Duct };
                    let part = if kind == ItemKind::Rail { panel::RAIL_PART } else { panel::DUCT_PART };
                    let (pw, _) = panel::profile(prj, kind, part);
                    let horizontal = (sn.x - a.x).abs() >= (sn.y - a.y).abs();
                    let (r0, r1) = if horizontal {
                        (p(a.x.min(sn.x), a.y - pw / 2.0), p(a.x.max(sn.x), a.y + pw / 2.0))
                    } else {
                        (p(a.x - pw / 2.0, a.y.min(sn.y)), p(a.x + pw / 2.0, a.y.max(sn.y)))
                    };
                    g.rect(r0 + m, r1 + m, 0.5, Ink::Ghost, false);
                    let len = if horizontal { (sn.x - a.x).abs() } else { (sn.y - a.y).abs() };
                    self.status = format!("Length {len:.0} mm");
                }
                let c = to_screen(sn);
                painter.line_segment([c - vec2(8.0, 0.0), c + vec2(8.0, 0.0)], Stroke::new(1.0, pal.ghost));
                painter.line_segment([c - vec2(0.0, 8.0), c + vec2(0.0, 8.0)], Stroke::new(1.0, pal.ghost));
            }
            (Tool::PlaceDevice(tag), _, Some(q)) => {
                let (w, h) = panel::device_size(prj, tag).map(|s| (s.w, s.h)).unwrap_or((panel::DEFAULT_DEVICE.0, panel::DEFAULT_DEVICE.1));
                let mut ghost = PanelItem { kind: ItemKind::Device, x: panel::snap(q.x - w / 2.0), y: panel::snap(q.y - h / 2.0), w, h, ..Default::default() };
                panel::snap_to_rail(e, &mut ghost);
                g.rect(p(ghost.x, ghost.y) + m, p(ghost.x + w, ghost.y + h) + m, 0.6, Ink::Ghost, false);
                g.text(ghost.center() + m, tag, (w.min(h) * 0.22).clamp(3.0, 9.0), crate::symbols::Anchor::Center, render::VAnchor::Middle, Ink::Ghost);
            }
            _ => {}
        }
        if let Some(q) = plate {
            if self.start.is_none() {
                self.status = format!("X {:.0}  Y {:.0} mm on the mounting plate", q.x, q.y);
            }
        }
    }

    fn select_and_move(&mut self, ui: &egui::Ui, resp: &egui::Response, prj: &mut Project, ei: usize, rect: Rect) {
        let shift = ui.input(|i| i.modifiers.shift);
        let tol = 6.0 / self.zoom;
        let hit = |e: &Enclosure, q: P| -> Option<u64> {
            // Devices first, then ducts, then rails.
            let order = [ItemKind::Device, ItemKind::Duct, ItemKind::Rail];
            order.iter().find_map(|k| e.items.iter().rev().find(|i| i.kind == *k && i.rect().expand(tol).contains(q)).map(|i| i.id))
        };
        let (offset, zoom) = (self.offset, self.zoom);
        let to_plate = move |s: Pos2| {
            p((s.x - rect.min.x - offset.x) / zoom - PLATE_MARGIN, (s.y - rect.min.y - offset.y) / zoom - PLATE_MARGIN)
        };
        if resp.drag_started_by(PointerButton::Primary) {
            let origin = ui.input(|i| i.pointer.press_origin()).unwrap_or(rect.center());
            let q = to_plate(origin);
            let e = &prj.layout.enclosures[ei];
            if let Some(id) = hit(e, q) {
                if !self.selection.contains(&id) {
                    if !shift {
                        self.selection.clear();
                    }
                    self.selection.insert(id);
                }
                // Devices mounted on a moved rail move with it.
                let mut ids = self.selection.clone();
                for rail in e.items.iter().filter(|i| i.kind == ItemKind::Rail && self.selection.contains(&i.id)) {
                    for dev in e.items.iter().filter(|i| i.kind == ItemKind::Device && i.z > 0.0) {
                        if rail.rect().contains(dev.center()) {
                            ids.insert(dev.id);
                        }
                    }
                }
                let originals = e.items.iter().filter(|i| ids.contains(&i.id)).map(|i| (i.id, i.x, i.y)).collect();
                self.drag = Some(MoveDrag { start: q, originals });
            }
        }
        if resp.dragged_by(PointerButton::Primary) {
            if let (Some(drag), Some(ptr)) = (&self.drag, resp.interact_pointer_pos()) {
                let q = to_plate(ptr);
                let dx = panel::snap(q.x - drag.start.x);
                let dy = panel::snap(q.y - drag.start.y);
                let e = &mut prj.layout.enclosures[ei];
                for (id, x0, y0) in &drag.originals {
                    if let Some(it) = e.items.iter_mut().find(|i| i.id == *id) {
                        it.x = x0 + dx;
                        it.y = y0 + dy;
                    }
                }
                // Devices that are moved on their own snap onto rails.
                let moved: Vec<u64> = drag.originals.iter().map(|o| o.0).collect();
                let moving_rails = e.items.iter().any(|i| i.kind == ItemKind::Rail && moved.contains(&i.id));
                if !moving_rails {
                    for id in moved {
                        if let Some(idx) = e.items.iter().position(|i| i.id == id) {
                            let mut it = e.items[idx].clone();
                            panel::snap_to_rail(e, &mut it);
                            e.items[idx] = it;
                        }
                    }
                }
                self.status = format!("ΔX {dx:.0}  ΔY {dy:.0} mm");
            }
        }
        if resp.drag_stopped() {
            self.drag = None;
        }
        if resp.clicked_by(PointerButton::Primary) {
            if let Some(ptr) = resp.interact_pointer_pos() {
                let e = &prj.layout.enclosures[ei];
                match hit(e, to_plate(ptr)) {
                    Some(id) if shift => {
                        if !self.selection.remove(&id) {
                            self.selection.insert(id);
                        }
                    }
                    Some(id) => self.selection = BTreeSet::from([id]),
                    None if !shift => self.selection.clear(),
                    None => {}
                }
            }
        }
    }

    // ------------------------------------------------------------------ 3D

    fn view_3d(&mut self, ui: &mut egui::Ui, prj: &mut Project, ei: usize, an: &Analysis) {
        let e = &prj.layout.enclosures[ei];
        let dark = ui.visuals().dark_mode;
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = resp.rect;
        let cam = self.cam.get_or_insert_with(|| Camera {
            yaw: 0.55,
            pitch: 0.3,
            dist: e.width.max(e.height) * 1.9,
            target: [e.width / 2.0, e.height / 2.0, e.depth / 2.0],
        });
        // Orbit / pan / zoom
        if resp.dragged_by(PointerButton::Primary) {
            let dl = resp.drag_delta();
            cam.yaw -= dl.x * 0.008;
            cam.pitch = (cam.pitch + dl.y * 0.008).clamp(-1.4, 1.4);
        }
        if resp.dragged_by(PointerButton::Middle) || resp.dragged_by(PointerButton::Secondary) {
            let dl = resp.drag_delta();
            let (_, right, up, _) = cam_basis(cam);
            let s = cam.dist * 0.0015;
            for k in 0..3 {
                cam.target[k] += -right[k] * dl.x * s + up[k] * dl.y * s;
            }
        }
        if resp.hovered() {
            let (wheel, zoom_delta) = ui.input(|i| (wheel_delta(i), i.zoom_delta()));
            let factor = zoom_delta * (wheel * 0.0025).exp();
            cam.dist = (cam.dist / factor).clamp(100.0, 20000.0);
        }
        let (eye, right, up, fwd) = cam_basis(cam);
        let f = rect.height() * 1.3;
        let center = rect.center();
        let project = |q: [f32; 3]| -> Option<(Pos2, f32)> {
            let v = sub3(q, eye);
            let z = dot(v, fwd);
            if z < 10.0 {
                return None;
            }
            Some((center + vec2(dot(v, right) / z * f, -dot(v, up) / z * f), z))
        };

        painter.rect_filled(rect, 0.0, if dark { Color32::from_rgb(24, 26, 30) } else { Color32::from_rgb(214, 218, 224) });

        // Collect primitives
        enum Prim {
            Face { pts: Vec<Pos2>, color: Color32, item: Option<u64> },
            Line { a: Pos2, b: Pos2, color: Color32, width: f32 },
        }
        // (layer, depth, primitive): walls and plate are always behind the equipment
        // when looking through the open door, so they are drawn in earlier layers.
        let mut prims: Vec<(u8, f32, Prim)> = Vec::new();
        let light = norm([0.35, 0.75, 0.55]);
        let problems = panel::problem_items(e);
        let add_box = |prims: &mut Vec<(u8, f32, Prim)>, layer: u8, min: [f32; 3], max: [f32; 3], base: Color32, item: Option<u64>| {
            let c = |x: usize, y: usize, z: usize| [if x == 0 { min[0] } else { max[0] }, if y == 0 { min[1] } else { max[1] }, if z == 0 { min[2] } else { max[2] }];
            let faces: [([usize; 4], [f32; 3]); 6] = [
                ([0b000, 0b010, 0b011, 0b001], [-1.0, 0.0, 0.0]),
                ([0b100, 0b101, 0b111, 0b110], [1.0, 0.0, 0.0]),
                ([0b000, 0b001, 0b101, 0b100], [0.0, -1.0, 0.0]),
                ([0b010, 0b110, 0b111, 0b011], [0.0, 1.0, 0.0]),
                ([0b000, 0b100, 0b110, 0b010], [0.0, 0.0, -1.0]),
                ([0b001, 0b011, 0b111, 0b101], [0.0, 0.0, 1.0]),
            ];
            for (corners, n) in faces {
                let pts3: Vec<[f32; 3]> = corners.iter().map(|&b| c(b >> 2 & 1, b >> 1 & 1, b & 1)).collect();
                let fc = [
                    pts3.iter().map(|q| q[0]).sum::<f32>() / 4.0,
                    pts3.iter().map(|q| q[1]).sum::<f32>() / 4.0,
                    pts3.iter().map(|q| q[2]).sum::<f32>() / 4.0,
                ];
                if dot(n, sub3(eye, fc)) <= 0.0 {
                    continue; // back face
                }
                let Some(pr) = pts3.iter().map(|q| project(*q)).collect::<Option<Vec<_>>>() else { continue };
                let depth = pr.iter().map(|x| x.1).sum::<f32>() / 4.0;
                let shade = 0.55 + 0.45 * dot(n, light).max(0.0);
                let mut color = Color32::from_rgb(
                    (base.r() as f32 * shade) as u8,
                    (base.g() as f32 * shade) as u8,
                    (base.b() as f32 * shade) as u8,
                );
                if layer == OUTER_WALLS {
                    color = color.gamma_multiply(0.6);
                }
                prims.push((layer, depth, Prim::Face { pts: pr.into_iter().map(|x| x.0).collect(), color, item }));
            }
        };
        // Enclosure: rear wall and inner side walls (door open)
        let (w, h, dpt) = (e.width, e.height, e.depth);
        let wall = Color32::from_rgb(200, 204, 210);
        let t = 2.0;
        // A wall seen from the outside covers the equipment (drawn last, translucent);
        // seen from the inside it is behind everything.
        let wall_layer = |outside: bool| if outside { OUTER_WALLS } else { 0 };
        add_box(&mut prims, wall_layer(eye[2] < 0.0), [0.0, 0.0, 0.0], [w, h, t], wall, None);
        add_box(&mut prims, wall_layer(eye[0] < 0.0), [0.0, 0.0, 0.0], [t, h, dpt], wall, None);
        add_box(&mut prims, wall_layer(eye[0] > w), [w - t, 0.0, 0.0], [w, h, dpt], wall, None);
        add_box(&mut prims, wall_layer(eye[1] < 0.0), [0.0, 0.0, 0.0], [w, t, dpt], wall, None);
        add_box(&mut prims, wall_layer(eye[1] > h), [0.0, h - t, 0.0], [w, h, dpt], wall, None);
        // Mounting plate
        let (pw, ph) = e.plate_size();
        let plate_min = [PLATE_MARGIN, h - PLATE_MARGIN - ph, PLATE_Z - PLATE_THICKNESS];
        add_box(&mut prims, 1, plate_min, [PLATE_MARGIN + pw, h - PLATE_MARGIN, PLATE_Z], Color32::from_rgb(222, 218, 205), None);
        let to3 = |x: f32, y: f32, z: f32| [PLATE_MARGIN + x, h - PLATE_MARGIN - y, PLATE_Z + z];
        for it in &e.items {
            let base = if self.selection.contains(&it.id) {
                Color32::from_rgb(255, 150, 40)
            } else if problems.contains(&it.id) {
                Color32::from_rgb(230, 70, 70)
            } else {
                match it.kind {
                    ItemKind::Rail => Color32::from_rgb(205, 208, 212),
                    ItemKind::Duct => Color32::from_rgb(150, 154, 160),
                    ItemKind::Device => Color32::from_rgb(170, 186, 210),
                }
            };
            let min = to3(it.x, it.y + it.h, it.z);
            let max = to3(it.x + it.w, it.y, it.z + it.d);
            let layer = match it.kind {
                ItemKind::Rail => 2,
                ItemKind::Duct => 3,
                ItemKind::Device => 5,
            };
            add_box(&mut prims, layer, min, max, base, Some(it.id));
        }
        // Wires inside the ducts
        if self.show_wires {
            for wire in an.wiring.iter().filter(|wire| wire.enclosure == ei) {
                let z = if wire.route.via_ducts { panel::DUCT_PROFILE.1 * 0.5 } else { 5.0 };
                for seg in wire.route.points.windows(2) {
                    let (a, b) = (to3(seg[0].x, seg[0].y, z), to3(seg[1].x, seg[1].y, z));
                    if let (Some(pa), Some(pb)) = (project(a), project(b)) {
                        prims.push((4, (pa.1 + pb.1) / 2.0, Prim::Line { a: pa.0, b: pb.0, color: WIRE_COLOR, width: 1.5 }));
                    }
                }
            }
        }
        // Painter's algorithm: far to near
        prims.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.total_cmp(&a.1)));
        let mut pick: Vec<(f32, Vec<Pos2>, u64)> = Vec::new();
        for (_, depth, prim) in &prims {
            match prim {
                Prim::Face { pts, color, item } => {
                    painter.add(Shape::convex_polygon(pts.clone(), *color, Stroke::new(0.6, color.gamma_multiply(0.6))));
                    if let Some(id) = item {
                        pick.push((*depth, pts.clone(), *id));
                    }
                }
                Prim::Line { a, b, color, width } => {
                    painter.line_segment([*a, *b], Stroke::new(*width, *color));
                }
            }
        }
        // Enclosure outline
        let edges = [
            ([0.0, 0.0, dpt], [w, 0.0, dpt]),
            ([w, 0.0, dpt], [w, h, dpt]),
            ([w, h, dpt], [0.0, h, dpt]),
            ([0.0, h, dpt], [0.0, 0.0, dpt]),
        ];
        for (a, b) in edges {
            if let (Some(pa), Some(pb)) = (project(a), project(b)) {
                painter.line_segment([pa.0, pb.0], Stroke::new(1.5, Color32::from_rgb(110, 115, 125)));
            }
        }
        // Device labels on the front faces
        for it in e.items.iter().filter(|i| i.kind == ItemKind::Device) {
            if let Some((q, z)) = project(to3(it.x + it.w / 2.0, it.y + it.h / 2.0, it.z + it.d)) {
                let size = (it.w.min(it.h) * 0.25 * f / z).clamp(0.0, 18.0);
                if size >= 7.0 && dot(sub3(eye, to3(it.x, it.y, it.z + it.d)), [0.0, 0.0, 1.0]) > 0.0 {
                    painter.text(q, egui::Align2::CENTER_CENTER, &it.tag, egui::FontId::proportional(size), Color32::from_rgb(20, 30, 50));
                }
            }
        }
        // Picking
        if resp.clicked_by(PointerButton::Primary) {
            if let Some(ptr) = resp.interact_pointer_pos() {
                // Faces are in drawing order: the last one under the pointer is visible.
                let hit = pick.iter().rev().find(|(_, pts, _)| point_in_convex(ptr, pts)).map(|x| x.2);
                match hit {
                    Some(id) => self.selection = BTreeSet::from([id]),
                    None => self.selection.clear(),
                }
            }
        }
        painter.text(
            rect.left_bottom() + vec2(10.0, -10.0),
            egui::Align2::LEFT_BOTTOM,
            "Drag: orbit · right/middle drag: pan · wheel: zoom · click: select",
            egui::FontId::proportional(12.0),
            if dark { Color32::GRAY } else { Color32::DARK_GRAY },
        );
        self.status = format!("{}  {:.0} × {:.0} × {:.0} mm", e.name, e.width, e.height, e.depth);
    }
}

fn apply_enclosure_part(ne: &mut NewEnclosure, db: &PartsDb) {
    if let Some(p) = db.get(&ne.part) {
        if p.width > 0.0 && p.height > 0.0 {
            ne.width = p.width;
            ne.height = p.height;
            if p.depth > 0.0 {
                ne.depth = p.depth;
            }
        }
    }
}

fn rotate(e: &mut Enclosure, sel: &BTreeSet<u64>) {
    for i in 0..e.items.len() {
        if sel.contains(&e.items[i].id) {
            e.items[i].rotate();
            let mut it = e.items[i].clone();
            panel::snap_to_rail(e, &mut it);
            e.items[i] = it;
        }
    }
}

/// Mouse wheel movement in points (same handling as the schematic canvas).
fn wheel_delta(i: &egui::InputState) -> f32 {
    let mut wheel = 0.0;
    for ev in &i.events {
        if let egui::Event::MouseWheel { unit, delta, modifiers, .. } = ev {
            if modifiers.ctrl || modifiers.command {
                continue;
            }
            wheel += match unit {
                egui::MouseWheelUnit::Point => delta.y,
                egui::MouseWheelUnit::Line => delta.y * 40.0,
                egui::MouseWheelUnit::Page => delta.y * 400.0,
            };
        }
    }
    wheel
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn norm(a: [f32; 3]) -> [f32; 3] {
    let l = dot(a, a).sqrt().max(1e-6);
    [a[0] / l, a[1] / l, a[2] / l]
}

/// Eye position and camera axes (right, up, forward).
fn cam_basis(cam: &Camera) -> ([f32; 3], [f32; 3], [f32; 3], [f32; 3]) {
    let eye = [
        cam.target[0] + cam.dist * cam.pitch.cos() * cam.yaw.sin(),
        cam.target[1] + cam.dist * cam.pitch.sin(),
        cam.target[2] + cam.dist * cam.pitch.cos() * cam.yaw.cos(),
    ];
    let fwd = norm(sub3(cam.target, eye));
    let right = norm(cross(fwd, [0.0, 1.0, 0.0]));
    let up = cross(right, fwd);
    (eye, right, up, fwd)
}

fn point_in_convex(q: Pos2, pts: &[Pos2]) -> bool {
    let mut sign = 0.0f32;
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        let c = (b.x - a.x) * (q.y - a.y) - (b.y - a.y) * (q.x - a.x);
        if c.abs() < 1e-6 {
            continue;
        }
        if sign == 0.0 {
            sign = c.signum();
        } else if c.signum() != sign {
            return false;
        }
    }
    true
}
