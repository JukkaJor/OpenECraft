//! Panel layout workspace: enclosure navigator, 2D mounting plate editor, 3D view
//! and property editor.

use openecraft_i18n::{tr, tr_fmt};
use crate::editor::{EguiDraw, Palette};
use crate::geom::{p, P};
use crate::model::{natural_cmp, Project};
use crate::panel::{self, Enclosure, Face, ItemKind, PanelItem, PLATE_MARGIN, PLATE_THICKNESS, PLATE_Z};
use crate::parts::{self, PartsDb};
use crate::parts::vec3::{cross, dot, norm, sub, Orbit};
use crate::render::{self, Draw, Ink};
use crate::reports::Analysis;
use egui::{vec2, Color32, Key, PointerButton, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq)]
pub enum Tool {
    Select,
    Rail,
    Duct,
    Busbar,
    Cutout,
    PlaceDevice(String),
}

impl Tool {
    /// Item kind and default part of the profile tools.
    fn profile(&self) -> Option<(ItemKind, &'static str)> {
        match self {
            Tool::Rail => Some((ItemKind::Rail, panel::RAIL_PART)),
            Tool::Duct => Some((ItemKind::Duct, panel::DUCT_PART)),
            Tool::Busbar => Some((ItemKind::Busbar, panel::BUSBAR_PART)),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum View {
    Front,
    ThreeD,
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
    /// Face shown in the front view (mounting plate or door).
    pub face: Face,
    pub selection: BTreeSet<u64>,
    /// Selected wire of the virtual wiring (from, to).
    pub selected_wire: Option<(String, String)>,
    pub show_wires: bool,
    /// Cable ducts coloured by their fill level.
    pub show_fill: bool,
    offset: Vec2,
    zoom: f32,
    fit_pending: bool,
    start: Option<P>,
    drag: Option<MoveDrag>,
    cam: Option<Orbit>,
    device_filter: String,
    new_enclosure: Option<NewEnclosure>,
    pub status: String,
    /// Requests for the application.
    pub goto_schematic: Option<String>,
    pub add_page: Option<u64>,
    /// NC export of an enclosure face (DXF = true, CSV = false).
    pub export: Option<(usize, Face, bool)>,
}

impl Default for PanelUi {
    fn default() -> Self {
        PanelUi {
            enclosure: 0,
            tool: Tool::Select,
            view: View::Front,
            face: Face::Plate,
            selection: BTreeSet::new(),
            selected_wire: None,
            show_wires: true,
            show_fill: true,
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
            export: None,
        }
    }
}

/// Drawing layer of enclosure walls that are seen from the outside.
const OUTER_WALLS: u8 = 6;

const WIRE_COLOR: Color32 = Color32::from_rgb(40, 110, 230);
/// Selected wire and the devices it connects (same colour as potential highlighting).
const WIRE_HIGHLIGHT: Color32 = Color32::from_rgb(230, 40, 200);

impl PanelUi {
    /// True while a gesture is in progress (no undo step yet).
    pub fn busy(&self) -> bool {
        self.start.is_some() || self.drag.is_some()
    }

    fn select_item(&mut self, id: u64) {
        self.selection = [id].into_iter().collect();
        self.selected_wire = None;
    }

    fn select_wire(&mut self, key: (String, String)) {
        self.selected_wire = Some(key);
        self.selection.clear();
    }

    /// Index of the selected wire in the current analysis.
    fn wire_index(&self, an: &Analysis) -> Option<usize> {
        let key = self.selected_wire.as_ref()?;
        an.wiring.iter().position(|w| &w.key() == key)
    }

    /// Selects the next wire among `candidates` (cycling on repeated clicks).
    fn pick_wire(&mut self, an: &Analysis, candidates: &[usize]) {
        let current = self.wire_index(an);
        if let Some(i) = panel::next_candidate(candidates, current) {
            let w = &an.wiring[i];
            self.select_wire(w.key());
            self.status = if candidates.len() > 1 {
                let pos = candidates.iter().position(|c| *c == i).unwrap_or(0) + 1;
                tr_fmt!("Wire {} – {}  ({} of {} here, click again for the next)", w.from, w.to, pos, candidates.len())
            } else {
                tr_fmt!("Wire {} – {}  {} mm", w.from, w.to, format!("{:.0}", w.length))
            };
        }
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
                self.face = item.face;
                self.select_item(item.id);
                true
            }
            None => false,
        }
    }

    // ------------------------------------------------------------------ navigator

    pub fn navigator(&mut self, ui: &mut egui::Ui, prj: &mut Project, db: &PartsDb) {
        ui.horizontal(|ui| {
            ui.heading(tr("Enclosures"));
            if ui.button("➕").on_hover_text(tr("New enclosure")).clicked() {
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
                if ui.small_button("🗑").on_hover_text(tr("Delete enclosure")).clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            prj.layout.enclosures.remove(i);
            self.selection.clear();
        }
        if prj.layout.enclosures.is_empty() {
            ui.weak(tr("No enclosure yet – create one with ➕."));
            return;
        }
        ui.separator();
        ui.heading(tr("Devices to place"));
        ui.add(egui::TextEdit::singleline(&mut self.device_filter).hint_text(tr("Filter…")));
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
                ui.weak(tr("All devices with mounting data are placed."));
            }
        });
        ui.separator();
        ui.heading(tr("Placed"));
        let Some(ei) = self.enclosure_index(prj) else { return };
        let mut placed: Vec<&PanelItem> = prj.layout.enclosures[ei].items.iter().filter(|i| i.kind == ItemKind::Device).collect();
        placed.sort_by(|a, b| natural_cmp(&a.tag, &b.tag));
        egui::ScrollArea::vertical().id_salt("placed").auto_shrink([false, false]).show(ui, |ui| {
            for it in placed {
                let text = if it.face == Face::Door { tr_fmt!("{}  (door)", it.tag) } else { it.tag.clone() };
                if ui.selectable_label(self.selection.contains(&it.id), text).clicked() {
                    self.face = it.face;
                    self.select_item(it.id);
                }
            }
        });
    }

    // ------------------------------------------------------------------ properties

    pub fn properties(&mut self, ui: &mut egui::Ui, prj: &mut Project, an: &Analysis) {
        ui.heading(tr("Properties"));
        let Some(ei) = self.enclosure_index(prj) else { return };
        if let Some(wi) = self.wire_index(an) {
            self.wire_properties(ui, prj, an, wi);
            return;
        }
        let sel: Vec<u64> = self.selection.iter().copied().collect();
        if sel.len() > 1 {
            ui.label(tr_fmt!("{} items selected", sel.len()));
            ui.horizontal(|ui| {
                if ui.button(tr("⟳ Rotate")).clicked() {
                    rotate(&mut prj.layout.enclosures[ei], &self.selection);
                }
                if ui.button(tr("🗑 Delete")).clicked() {
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
                    ui.label(tr("Device"));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&it.tag).monospace().strong());
                        if ui.small_button(tr("Show in schematic")).clicked() {
                            self.goto_schematic = Some(it.tag.clone());
                        }
                    });
                    ui.end_row();
                }
                ui.label(tr("Part"));
                ui.label(RichText::new(if it.part.is_empty() { "–" } else { &it.part }).monospace());
                ui.end_row();
                ui.label("X / Y");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut it.x).speed(1.0).suffix(" mm"));
                    ui.add(egui::DragValue::new(&mut it.y).speed(1.0).suffix(" mm"));
                });
                ui.end_row();
                if it.kind == ItemKind::Device {
                    ui.label(tr("Width × height"));
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut it.w).speed(0.5).range(1.0..=5000.0));
                        ui.add(egui::DragValue::new(&mut it.h).speed(0.5).range(1.0..=5000.0));
                    });
                    ui.end_row();
                    ui.label(tr("Mounted on"));
                    let before = it.face;
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut it.face, Face::Plate, tr("Mounting plate"));
                        ui.selectable_value(&mut it.face, Face::Door, tr("Door"));
                    });
                    if it.face != before {
                        // Keep the position in front view; door devices sit directly on the door.
                        let shift = it.face.origin() - before.origin();
                        it.x -= shift;
                        it.y -= shift;
                        it.z = 0.0;
                        self.face = it.face;
                    }
                    ui.end_row();
                    ui.label(tr("Depth / mounted at"));
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut it.d).speed(0.5).range(1.0..=2000.0));
                        ui.add(egui::DragValue::new(&mut it.z).speed(0.5).range(0.0..=2000.0).prefix("z "));
                    });
                    ui.end_row();
                } else if it.kind == ItemKind::Cutout {
                    ui.label(tr("Shape"));
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut it.round, false, tr("Rectangular"));
                        ui.selectable_value(&mut it.round, true, tr("Round"));
                    });
                    ui.end_row();
                    if it.round {
                        ui.label(tr("Diameter"));
                        if ui.add(egui::DragValue::new(&mut it.w).speed(0.5).range(1.0..=2000.0).suffix(" mm")).changed() {
                            it.h = it.w;
                        }
                    } else {
                        ui.label(tr("Width × height"));
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut it.w).speed(0.5).range(1.0..=5000.0));
                            ui.add(egui::DragValue::new(&mut it.h).speed(0.5).range(1.0..=5000.0));
                        });
                    }
                    ui.end_row();
                    if it.round {
                        it.h = it.w;
                    }
                } else {
                    ui.label(tr("Length"));
                    let mut len = it.length();
                    if ui.add(egui::DragValue::new(&mut len).speed(5.0).range(10.0..=5000.0).suffix(" mm")).changed() {
                        if it.horizontal() {
                            it.w = len;
                        } else {
                            it.h = len;
                        }
                    }
                    ui.end_row();
                    ui.label(tr("Orientation"));
                    ui.label(if it.horizontal() { tr("horizontal") } else { tr("vertical") });
                    ui.end_row();
                }
                if it.kind == ItemKind::Duct {
                    let limit = prj.layout.duct_fill_limit;
                    match an.duct_fill.iter().find(|f| f.enclosure == ei && f.duct == it.id) {
                        Some(f) => {
                            ui.label(tr("Fill level"));
                            ui.label(RichText::new(tr_fmt!("{} % (limit {} %)", format!("{:.0}", f.percent), format!("{limit:.0}"))).color(fill_color(f.percent, limit)));
                            ui.end_row();
                            ui.label(tr("Wires"));
                            ui.label(f.total_wires.to_string());
                            ui.end_row();
                            ui.label(tr("Fullest stretch"));
                            ui.label(tr_fmt!("{} wires, {} – {} mm", f.wires, format!("{:.0}", f.from), format!("{:.0}", f.to)));
                            ui.end_row();
                        }
                        None => {
                            ui.label(tr("Fill level"));
                            ui.label(tr("no wires"));
                            ui.end_row();
                        }
                    }
                }
            });
            ui.horizontal(|ui| {
                if ui.button(tr("⟳ Rotate")).on_hover_text("R").clicked() {
                    it.rotate();
                }
                if ui.button(tr("🗑 Delete")).on_hover_text(tr("Del")).clicked() {
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
                    ui.label(RichText::new(tr("Wires")).strong());
                    let mut clicked = None;
                    egui::Grid::new("pwires").striped(true).num_columns(2).show(ui, |ui| {
                        for w in wires {
                            if ui.selectable_label(false, format!("{} – {}", w.from, w.to)).on_hover_text(tr("Show this wire")).clicked() {
                                clicked = Some(w.key());
                            }
                            ui.label(format!("{:.0} mm", w.length));
                            ui.end_row();
                        }
                    });
                    if let Some(key) = clicked {
                        self.select_wire(key);
                    }
                }
            }
            return;
        }
        // Enclosure properties
        let e = &mut prj.layout.enclosures[ei];
        ui.label(RichText::new(tr("Enclosure")).strong());
        egui::Grid::new("penc").num_columns(2).spacing([10.0, 5.0]).show(ui, |ui| {
            ui.label(tr("Location"));
            ui.text_edit_singleline(&mut e.name);
            ui.end_row();
            ui.label(tr("Part"));
            ui.label(RichText::new(if e.part.is_empty() { "–" } else { &e.part }).monospace());
            ui.end_row();
            ui.label(tr("Width × height"));
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut e.width).speed(5.0).range(100.0..=5000.0));
                ui.add(egui::DragValue::new(&mut e.height).speed(5.0).range(100.0..=5000.0));
            });
            ui.end_row();
            ui.label(tr("Depth"));
            ui.add(egui::DragValue::new(&mut e.depth).speed(5.0).range(60.0..=2000.0).suffix(" mm"));
            ui.end_row();
            ui.label(tr("Door clearance"));
            ui.add(egui::DragValue::new(&mut e.door_clearance).speed(1.0).range(0.0..=500.0).suffix(" mm"))
                .on_hover_text(tr("Space behind the door that devices must keep free (door-mounted devices, handles)"));
            ui.end_row();
            let (pw, ph) = e.plate_size();
            ui.label(tr("Mounting plate"));
            ui.label(format!("{pw:.0} × {ph:.0} mm"));
            ui.end_row();
        });
        ui.horizontal(|ui| {
            ui.label(tr("Wire reserve per connection"));
            ui.add(egui::DragValue::new(&mut prj.layout.wire_reserve).speed(5.0).range(0.0..=2000.0).suffix(" mm"));
        });
        ui.horizontal(|ui| {
            ui.label(tr("Duct fill limit"));
            ui.add(egui::DragValue::new(&mut prj.layout.duct_fill_limit).speed(1.0).range(5.0..=100.0).suffix(" %"))
                .on_hover_text(tr("The check warns about cable ducts filled above this level"));
        });
        ui.checkbox(&mut prj.layout.ferrules, tr("Wire ends with ferrules")).on_hover_text(tr("Production wire list and bill of materials"));
        let e = &prj.layout.enclosures[ei];
        let devices = e.items.iter().filter(|i| i.kind == ItemKind::Device).count();
        let wires: Vec<_> = an.wiring.iter().filter(|w| w.enclosure == ei).collect();
        let total: f32 = wires.iter().map(|w| w.length).sum();
        ui.separator();
        ui.label(tr_fmt!("{} devices placed, {} wires routed ({} m)", devices, wires.len(), format!("{:.1}", total / 1000.0)));
        let problems = panel::collisions(e, &prj.parts).len() + panel::outside(e).len() + crate::production::clearance_violations(e, &prj.parts).len();
        let holes = crate::production::holes(e, &prj.parts).len();
        let cutouts = e.items.iter().filter(|i| i.kind == ItemKind::Cutout).count();
        ui.label(tr_fmt!("Mounting plate: {} holes, {} cutouts", holes, cutouts));
        ui.horizontal(|ui| {
            if ui.button(tr("⬇ NC data (DXF)")).on_hover_text(tr("Plate outline, holes per diameter, cutouts")).clicked() {
                self.export = Some((ei, Face::Plate, true));
            }
            if ui.button(tr("⬇ Drilling list (CSV)")).clicked() {
                self.export = Some((ei, Face::Plate, false));
            }
        });
        let door_devices = e.items.iter().filter(|i| i.face == Face::Door && i.kind == ItemKind::Device).count();
        ui.label(tr_fmt!("Door: {} devices, {} cutouts", door_devices, crate::production::openings(e, &prj.parts, Face::Door).len()));
        ui.horizontal(|ui| {
            if ui.button(tr("⬇ Door NC data (DXF)")).clicked() {
                self.export = Some((ei, Face::Door, true));
            }
            if ui.button(tr("⬇ Door cutouts (CSV)")).clicked() {
                self.export = Some((ei, Face::Door, false));
            }
        });
        if problems > 0 {
            ui.colored_label(Color32::from_rgb(220, 60, 60), tr_fmt!("{} placement problems (red) – see Check", problems));
        }
        if ui.button(tr("➕ Insert mounting layout page")).clicked() {
            self.add_page = Some(e.id);
        }
    }

    fn wire_properties(&mut self, ui: &mut egui::Ui, prj: &Project, an: &Analysis, wi: usize) {
        let w = &an.wiring[wi];
        ui.label(RichText::new(tr("Wire")).strong());
        egui::Grid::new("pwire").num_columns(2).spacing([10.0, 5.0]).show(ui, |ui| {
            ui.label(tr("From"));
            ui.label(RichText::new(&w.from).monospace().color(WIRE_HIGHLIGHT));
            ui.end_row();
            ui.label(tr("To"));
            ui.label(RichText::new(&w.to).monospace().color(WIRE_HIGHLIGHT));
            ui.end_row();
            ui.label(tr("Potential"));
            ui.label(&w.net);
            ui.end_row();
            ui.label(tr("Length"));
            ui.label(format!("{:.0} mm", w.length));
            ui.end_row();
            ui.label(tr("Routing"));
            ui.label(if w.route.via_ducts { tr("via cable ducts") } else { tr("direct (no ducts)") });
            ui.end_row();
            ui.label(tr("Wire ends"));
            ui.label(format!("{} / {}", end_kind(w.ends.0), end_kind(w.ends.1)))
                .on_hover_text(tr("Where the wire starts and ends: a connection point from the part data, one generated from the function template, or the device centre if no connection point matches"));
            ui.end_row();
            if !w.color.is_empty() || !w.cross_section.is_empty() {
                ui.label(tr("Colour / cross-section"));
                ui.label(format!("{} {}", w.color, w.cross_section));
                ui.end_row();
            }
        });
        ui.label(
            RichText::new(tr_fmt!(
                "{} mm on the plate + {} mm reserve; the rest is the way from the connection points to the plate.",
                format!("{:.0}", w.route.plate_length),
                format!("{:.0}", prj.layout.wire_reserve)
            ))
            .weak(),
        );
        ui.separator();
        ui.label(RichText::new(tr("Connected devices")).strong());
        let e = &prj.layout.enclosures[w.enclosure];
        let mut select = None;
        for id in [w.items.0, w.items.1] {
            if let Some(it) = e.item(id) {
                ui.horizontal(|ui| {
                    if ui.link(RichText::new(&it.tag).monospace()).on_hover_text(tr("Select device")).clicked() {
                        select = Some(id);
                    }
                    if ui.small_button(tr("Show in schematic")).clicked() {
                        self.goto_schematic = Some(it.tag.clone());
                    }
                });
            }
        }
        if let Some(id) = select {
            self.select_item(id);
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
        egui::Window::new(tr("New enclosure")).open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
            egui::Grid::new("newenc").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label(tr("Location"));
                ui.text_edit_singleline(&mut ne.name);
                ui.end_row();
                ui.label(tr("Part"));
                let before = ne.part.clone();
                egui::ComboBox::from_id_salt("encpart")
                    .selected_text(if ne.part.is_empty() { "(custom size)".to_string() } else { ne.part.clone() })
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut ne.part, String::new(), tr("(custom size)"));
                        for p in &enclosures {
                            ui.selectable_value(&mut ne.part, p.number.clone(), format!("{} – {}", p.number, p.description));
                        }
                    });
                if ne.part != before {
                    apply_enclosure_part(ne, db);
                }
                ui.end_row();
                let custom = ne.part.is_empty();
                ui.label(tr("Width × height × depth"));
                ui.add_enabled_ui(custom, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut ne.width).speed(5.0).range(100.0..=5000.0));
                        ui.add(egui::DragValue::new(&mut ne.height).speed(5.0).range(100.0..=5000.0));
                        ui.add(egui::DragValue::new(&mut ne.depth).speed(5.0).range(60.0..=2000.0));
                    });
                });
                ui.end_row();
            });
            if ui.button(tr("✔ Create")).clicked() {
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
                ui.label(tr("Create an enclosure (➕ in the navigator on the left) to start the panel layout."));
            });
            return;
        };
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.view, View::Front, tr("Front view (2D)"));
            ui.selectable_value(&mut self.view, View::ThreeD, "3D");
            ui.separator();
            if self.view == View::Front {
                ui.separator();
                for face in [Face::Plate, Face::Door] {
                    if ui.selectable_label(self.face == face, face.title()).clicked() && self.face != face {
                        self.face = face;
                        self.selection.clear();
                        self.selected_wire = None;
                        if self.tool.profile().is_some() {
                            self.set_tool(Tool::Select);
                        }
                    }
                }
                ui.separator();
                let tool = self.tool.clone();
                for (t, label, tip) in [
                    (Tool::Select, tr("⬉ Select"), tr("Select / move (Esc)")),
                    (Tool::Rail, tr("Mounting rail"), tr("Click start and end point")),
                    (Tool::Duct, tr("Cable duct"), tr("Click start and end point")),
                    (Tool::Busbar, tr("Busbar"), tr("Copper busbar system: click start and end point")),
                    (Tool::Cutout, tr("Cutout"), tr("Opening in the mounting plate / door: click two corners (round: in the properties)")),
                ] {
                    if self.face == Face::Door && t.profile().is_some() {
                        continue;
                    }
                    if ui.selectable_label(tool == t, label).on_hover_text(tip).clicked() {
                        self.set_tool(t);
                    }
                }
                if let Tool::PlaceDevice(tag) = &tool {
                    ui.label(RichText::new(tr_fmt!("Placing {}", tag)).strong());
                }
                ui.separator();
            }
            ui.checkbox(&mut self.show_wires, tr("Wires"));
            ui.checkbox(&mut self.show_fill, tr("Duct fill")).on_hover_text(tr("Colour the cable ducts by their fill level"));
            if ui.button(tr("⛶ Fit")).clicked() {
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
        let fm = self.face.origin();
        let plate = ptr.map(|s| self.s2w(rect, s) - p(fm, fm));
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
                    self.selected_wire = None;
                }
            }
            if self.face == Face::Door && self.tool.profile().is_some() {
                self.set_tool(Tool::Select);
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
            Tool::Select => self.select_and_move(ui, &resp, prj, ei, rect, an),
            Tool::Rail | Tool::Duct | Tool::Busbar => {
                if let (true, Some(sn)) = (resp.clicked_by(PointerButton::Primary), snapped) {
                    match self.start.take() {
                        None => self.start = Some(sn),
                        Some(a) if !a.same(sn) => {
                            let (kind, part) = self.tool.profile().unwrap_or((ItemKind::Duct, panel::DUCT_PART));
                            if db.get(part).is_some() {
                                parts::ensure_in_project(prj, db, part);
                            }
                            if kind == ItemKind::Busbar && db.get(crate::production::BUSBAR_HOLDER).is_some() {
                                parts::ensure_in_project(prj, db, crate::production::BUSBAR_HOLDER);
                            }
                            let item = panel::new_profile(prj, kind, part, a, sn);
                            prj.layout.enclosures[ei].items.push(item);
                        }
                        Some(_) => {}
                    }
                }
            }
            Tool::Cutout => {
                if let (true, Some(sn)) = (resp.clicked_by(PointerButton::Primary), snapped) {
                    match self.start.take() {
                        None => self.start = Some(sn),
                        Some(a) if (a.x - sn.x).abs() > 0.1 && (a.y - sn.y).abs() > 0.1 => {
                            let id = prj.new_id();
                            let (x, y) = (a.x.min(sn.x), a.y.min(sn.y));
                            let (w, h) = ((a.x - sn.x).abs(), (a.y - sn.y).abs());
                            let (z, d) = if self.face == Face::Door { (0.0, 2.0) } else { (-PLATE_THICKNESS, PLATE_THICKNESS) };
                            prj.layout.enclosures[ei].items.push(PanelItem { id, kind: ItemKind::Cutout, x, y, w, h, z, d, face: self.face, ..Default::default() });
                            self.select_item(id);
                        }
                        Some(_) => {}
                    }
                }
            }
            Tool::PlaceDevice(tag) => {
                if let (true, Some(q)) = (resp.clicked_by(PointerButton::Primary), plate) {
                    let size = panel::device_size(prj, &tag);
                    let (w, h) = size.map(|s| (s.w, s.h)).unwrap_or((panel::DEFAULT_DEVICE.0, panel::DEFAULT_DEVICE.1));
                    if let Some(id) = panel::place_device_on(prj, ei, &tag, p(q.x - w / 2.0, q.y - h / 2.0), self.face) {
                        self.select_item(id);
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
        let problems = panel::problem_items(e, &prj.parts);
        let mut d = EguiDraw { painter: &painter, origin, zoom: self.zoom, pal: &pal, force: None };
        render::draw_enclosure(&mut d, e, self.face, p(0.0, 0.0), 1.0, &self.selection, &problems, prj);
        let to_screen = |q: P| origin + vec2(q.x + fm, q.y + fm) * self.zoom;
        // Cable ducts coloured by their fill level.
        if self.show_fill && self.face == Face::Plate {
            let limit = prj.layout.duct_fill_limit;
            for f in an.duct_fill.iter().filter(|f| f.enclosure == ei) {
                let Some(d) = e.item(f.duct) else { continue };
                let r = Rect::from_min_max(to_screen(p(d.x, d.y)), to_screen(p(d.x + d.w, d.y + d.h)));
                let color = fill_color(f.percent, limit);
                painter.rect_filled(r, 0.0, color.gamma_multiply(0.3));
                if r.width().min(r.height()) >= 12.0 {
                    painter.text(r.center(), egui::Align2::CENTER_CENTER, format!("{:.0} %", f.percent), egui::FontId::proportional(11.0), color);
                }
            }
        }
        // Wires run on the mounting plate.
        let selected_wire = self.wire_index(an).filter(|i| an.wiring[*i].enclosure == ei && self.face == Face::Plate);
        if (self.show_wires && self.face == Face::Plate) || selected_wire.is_some() {
            // Other wires are dimmed while one is selected.
            let normal = if selected_wire.is_some() { WIRE_COLOR.gamma_multiply(0.25) } else { WIRE_COLOR.gamma_multiply(0.7) };
            for (i, w) in an.wiring.iter().enumerate().filter(|(_, w)| w.enclosure == ei) {
                if Some(i) != selected_wire && self.show_wires {
                    let pts: Vec<Pos2> = w.route.points.iter().map(|q| to_screen(*q)).collect();
                    painter.add(Shape::line(pts, Stroke::new(1.0, normal)));
                }
            }
        }
        if let Some(i) = selected_wire {
            let w = &an.wiring[i];
            let pts: Vec<Pos2> = w.route.points.iter().map(|q| to_screen(*q)).collect();
            painter.add(Shape::line(pts.clone(), Stroke::new(3.0, WIRE_HIGHLIGHT)));
            for end in [pts.first(), pts.last()].into_iter().flatten() {
                painter.circle_filled(*end, 4.5, WIRE_HIGHLIGHT);
            }
            // The two connected devices
            for id in [w.items.0, w.items.1] {
                if let Some(it) = e.item(id) {
                    let r = Rect::from_min_max(to_screen(p(it.x, it.y)), to_screen(p(it.x + it.w, it.y + it.h)));
                    painter.rect_filled(r, 0.0, WIRE_HIGHLIGHT.gamma_multiply(0.18));
                    painter.rect_stroke(r.expand(2.0), 2.0, Stroke::new(2.5, WIRE_HIGHLIGHT), egui::StrokeKind::Outside);
                }
            }
        }
        // Connection points of the selected device.
        if let [id] = self.selection.iter().copied().collect::<Vec<_>>()[..] {
            if let Some(it) = e.item(id).filter(|it| it.kind == ItemKind::Device && it.face == self.face) {
                let ink = ui.visuals().text_color();
                for pt in panel::device_points(prj, it) {
                    let c = to_screen(p(pt.pos[0], pt.pos[1]));
                    let dir = match pt.exit {
                        crate::parts::Exit::Up => vec2(0.0, -1.0),
                        crate::parts::Exit::Down => vec2(0.0, 1.0),
                        crate::parts::Exit::Left => vec2(-1.0, 0.0),
                        crate::parts::Exit::Right => vec2(1.0, 0.0),
                        crate::parts::Exit::Front => vec2(0.0, 0.0),
                    };
                    painter.line_segment([c, c + dir * 9.0], Stroke::new(1.5, WIRE_COLOR));
                    if pt.exact {
                        painter.circle_filled(c, 3.0, WIRE_COLOR);
                    } else {
                        painter.circle_stroke(c, 3.0, Stroke::new(1.5, WIRE_COLOR));
                    }
                    if self.zoom >= 2.0 {
                        let name = if pt.terminal.is_empty() { pt.pin.clone() } else { format!("{}:{}", pt.terminal, pt.pin) };
                        painter.text(c - dir * 4.0, egui::Align2::CENTER_CENTER, name, egui::FontId::monospace(10.0), ink);
                    }
                }
            }
        }
        // Previews
        let mut g = EguiDraw { painter: &painter, origin, zoom: self.zoom, pal: &pal, force: Some(Ink::Ghost) };
        let m = p(fm, fm);
        match (&self.tool, snapped, plate) {
            (Tool::Cutout, Some(sn), _) => {
                if let Some(a) = self.start {
                    g.rect(a + m, sn + m, 0.5, Ink::Ghost, false);
                    self.status = tr_fmt!("Cutout {} × {} mm", format!("{:.0}", (sn.x - a.x).abs()), format!("{:.0}", (sn.y - a.y).abs()));
                }
                let c = to_screen(sn);
                painter.line_segment([c - vec2(8.0, 0.0), c + vec2(8.0, 0.0)], Stroke::new(1.0, pal.ghost));
                painter.line_segment([c - vec2(0.0, 8.0), c + vec2(0.0, 8.0)], Stroke::new(1.0, pal.ghost));
            }
            (Tool::Rail | Tool::Duct | Tool::Busbar, Some(sn), _) => {
                if let Some(a) = self.start {
                    let (kind, part) = self.tool.profile().unwrap_or((ItemKind::Duct, panel::DUCT_PART));
                    let (pw, _) = panel::profile(prj, kind, part);
                    let horizontal = (sn.x - a.x).abs() >= (sn.y - a.y).abs();
                    let (r0, r1) = if horizontal {
                        (p(a.x.min(sn.x), a.y - pw / 2.0), p(a.x.max(sn.x), a.y + pw / 2.0))
                    } else {
                        (p(a.x - pw / 2.0, a.y.min(sn.y)), p(a.x + pw / 2.0, a.y.max(sn.y)))
                    };
                    g.rect(r0 + m, r1 + m, 0.5, Ink::Ghost, false);
                    let len = if horizontal { (sn.x - a.x).abs() } else { (sn.y - a.y).abs() };
                    self.status = tr_fmt!("Length {} mm", format!("{len:.0}"));
                }
                let c = to_screen(sn);
                painter.line_segment([c - vec2(8.0, 0.0), c + vec2(8.0, 0.0)], Stroke::new(1.0, pal.ghost));
                painter.line_segment([c - vec2(0.0, 8.0), c + vec2(0.0, 8.0)], Stroke::new(1.0, pal.ghost));
            }
            (Tool::PlaceDevice(tag), _, Some(q)) => {
                let (w, h) = panel::device_size(prj, tag).map(|s| (s.w, s.h)).unwrap_or((panel::DEFAULT_DEVICE.0, panel::DEFAULT_DEVICE.1));
                let mut ghost = PanelItem { kind: ItemKind::Device, x: panel::snap(q.x - w / 2.0), y: panel::snap(q.y - h / 2.0), w, h, face: self.face, ..Default::default() };
                panel::snap_to_rail(e, &mut ghost);
                g.rect(p(ghost.x, ghost.y) + m, p(ghost.x + w, ghost.y + h) + m, 0.6, Ink::Ghost, false);
                g.text(ghost.center() + m, tag, (w.min(h) * 0.22).clamp(3.0, 9.0), crate::symbols::Anchor::Center, render::VAnchor::Middle, Ink::Ghost);
            }
            _ => {}
        }
        if let Some(q) = plate {
            if self.start.is_none() && !resp.clicked() {
                self.status = tr_fmt!("X {}  Y {} mm on the {}", format!("{:.0}", q.x), format!("{:.0}", q.y), self.face.title().to_lowercase());
            }
        }
    }

    fn select_and_move(&mut self, ui: &egui::Ui, resp: &egui::Response, prj: &mut Project, ei: usize, rect: Rect, an: &Analysis) {
        let shift = ui.input(|i| i.modifiers.shift);
        let tol = 6.0 / self.zoom;
        let face = self.face;
        let hit = |e: &Enclosure, q: P| -> Option<u64> {
            // Top layer first: devices, busbars, ducts, rails, cutouts.
            let mut items: Vec<&PanelItem> = e.items.iter().filter(|i| i.face == face && i.rect().expand(tol).contains(q)).collect();
            items.sort_by_key(|i| std::cmp::Reverse(i.kind.layer()));
            items.first().map(|i| i.id)
        };
        let (offset, zoom, fm) = (self.offset, self.zoom, face.origin());
        let to_plate = move |s: Pos2| p((s.x - rect.min.x - offset.x) / zoom - fm, (s.y - rect.min.y - offset.y) / zoom - fm);
        if resp.drag_started_by(PointerButton::Primary) {
            let origin = ui.input(|i| i.pointer.press_origin()).unwrap_or(rect.center());
            let q = to_plate(origin);
            let e = &prj.layout.enclosures[ei];
            if let Some(id) = hit(e, q) {
                self.selected_wire = None;
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
                let q = to_plate(ptr);
                let e = &prj.layout.enclosures[ei];
                let item = hit(e, q);
                let on_device = item.and_then(|id| e.item(id)).map(|i| i.kind == ItemKind::Device).unwrap_or(false);
                // Wires lie on top of ducts and rails, devices on top of wires.
                if self.show_wires && face == Face::Plate && !on_device && !shift {
                    let candidates = panel::wires_at(&an.wiring, ei, q, tol);
                    if !candidates.is_empty() {
                        self.pick_wire(an, &candidates);
                        return;
                    }
                }
                self.selected_wire = None;
                match item {
                    Some(id) if shift => {
                        if !self.selection.remove(&id) {
                            self.selection.insert(id);
                        }
                    }
                    Some(id) => self.select_item(id),
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
        let cam = self.cam.get_or_insert_with(|| Orbit {
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
            let (_, right, up, _) = cam.basis();
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
        let (eye, right, up, fwd) = cam.basis();
        let f = rect.height() * 1.3;
        let center = rect.center();
        let project = |q: [f32; 3]| -> Option<(Pos2, f32)> {
            let v = sub(q, eye);
            let z = dot(v, fwd);
            if z < 10.0 {
                return None;
            }
            Some((center + vec2(dot(v, right) / z * f, -dot(v, up) / z * f), z))
        };

        painter.rect_filled(rect, 0.0, if dark { Color32::from_rgb(24, 26, 30) } else { Color32::from_rgb(214, 218, 224) });

        // Collect primitives
        enum Prim {
            Face { pts: Vec<Pos2>, color: Color32, item: Option<u64>, outline: bool },
            Line { a: Pos2, b: Pos2, color: Color32, width: f32 },
        }
        // (layer, depth, primitive): walls and plate are always behind the equipment
        // when looking through the open door, so they are drawn in earlier layers.
        let mut prims: Vec<(u8, f32, Prim)> = Vec::new();
        let light = norm([0.35, 0.75, 0.55]);
        let problems = panel::problem_items(e, &prj.parts);
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
                if dot(n, sub(eye, fc)) <= 0.0 {
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
                prims.push((layer, depth, Prim::Face { pts: pr.into_iter().map(|x| x.0).collect(), color, item, outline: true }));
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
        // Enclosure coordinates (y down) to the 3D view (y up).
        let up = |q: [f32; 3]| [q[0], h - q[1], q[2]];
        let selected_wire = self.wire_index(an).filter(|i| an.wiring[*i].enclosure == ei);
        let wire_ends = selected_wire.map(|i| an.wiring[i].items);
        for it in &e.items {
            let base = if self.selection.contains(&it.id) {
                Color32::from_rgb(255, 150, 40)
            } else if wire_ends.map(|(a, b)| a == it.id || b == it.id).unwrap_or(false) {
                WIRE_HIGHLIGHT
            } else if problems.contains(&it.id) {
                Color32::from_rgb(230, 70, 70)
            } else {
                match it.kind {
                    ItemKind::Rail => Color32::from_rgb(205, 208, 212),
                    ItemKind::Duct => Color32::from_rgb(150, 154, 160),
                    ItemKind::Busbar => Color32::from_rgb(200, 120, 60),
                    ItemKind::Cutout => Color32::from_rgb(40, 40, 44),
                    ItemKind::Device => Color32::from_rgb(170, 186, 210),
                }
            };
            // Devices with a 3D model in their part are drawn with it instead of a box.
            if let Some(tris) = panel::device_mesh(prj, it) {
                for tri in tris {
                    let w = tri.map(|v| up(panel::to_enclosure(e, it, v)));
                    let n = norm(cross(sub(w[1], w[0]), sub(w[2], w[0])));
                    if dot(n, sub(eye, w[0])) <= 0.0 {
                        continue; // back face
                    }
                    let Some(pr) = w.iter().map(|q| project(*q)).collect::<Option<Vec<_>>>() else { continue };
                    let depth = pr.iter().map(|x| x.1).sum::<f32>() / 3.0;
                    let shade = 0.55 + 0.45 * dot(n, light).max(0.0);
                    let color = Color32::from_rgb(
                        (base.r() as f32 * shade) as u8,
                        (base.g() as f32 * shade) as u8,
                        (base.b() as f32 * shade) as u8,
                    );
                    let pts = pr.into_iter().map(|x| x.0).collect();
                    prims.push((5, depth, Prim::Face { pts, color, item: Some(it.id), outline: false }));
                }
                continue;
            }
            let (b0, b1) = panel::item_box(e, it);
            let (min, max) = if it.kind == ItemKind::Cutout && it.face == Face::Plate {
                // Dark patch just in front of the plate.
                (to3(it.x, it.y + it.h, 0.0), to3(it.x + it.w, it.y, 0.3))
            } else {
                (up([b0[0], b1[1], b0[2]]), up([b1[0], b0[1], b1[2]]))
            };
            let layer = match it.kind {
                ItemKind::Cutout => 1,
                ItemKind::Rail => 2,
                ItemKind::Duct | ItemKind::Busbar => 3,
                ItemKind::Device => 5,
            };
            add_box(&mut prims, layer, min, max, base, Some(it.id));
        }
        // Wires inside the ducts; the selected one is drawn on top of everything.
        let mut wire_segs: Vec<(Pos2, Pos2, usize)> = Vec::new();
        for (wi, wire) in an.wiring.iter().enumerate().filter(|(_, wire)| wire.enclosure == ei) {
            let selected = Some(wi) == selected_wire;
            if !self.show_wires && !selected {
                continue;
            }
            let z = if wire.route.via_ducts { panel::DUCT_PROFILE.1 * 0.5 } else { 5.0 };
            let (layer, color, width) = if selected {
                (OUTER_WALLS + 1, WIRE_HIGHLIGHT, 3.5)
            } else if selected_wire.is_some() {
                (4, WIRE_COLOR.gamma_multiply(0.3), 1.0)
            } else {
                (4, WIRE_COLOR, 1.5)
            };
            for seg in wire.route.points.windows(2) {
                let (a, b) = (to3(seg[0].x, seg[0].y, z), to3(seg[1].x, seg[1].y, z));
                if let (Some(pa), Some(pb)) = (project(a), project(b)) {
                    prims.push((layer, (pa.1 + pb.1) / 2.0, Prim::Line { a: pa.0, b: pb.0, color, width }));
                    wire_segs.push((pa.0, pb.0, wi));
                }
            }
        }
        // Painter's algorithm: far to near
        prims.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.total_cmp(&a.1)));
        let mut pick: Vec<(f32, Vec<Pos2>, u64)> = Vec::new();
        for (_, depth, prim) in &prims {
            match prim {
                Prim::Face { pts, color, item, outline } => {
                    // Mesh triangles get a hairline in their own colour to close the seams.
                    let stroke = if *outline { Stroke::new(0.6, color.gamma_multiply(0.6)) } else { Stroke::new(0.5, *color) };
                    painter.add(Shape::convex_polygon(pts.clone(), *color, stroke));
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
            let (b0, b1) = panel::item_box(e, it);
            let front = up([(b0[0] + b1[0]) / 2.0, (b0[1] + b1[1]) / 2.0, b1[2]]);
            if let Some((q, z)) = project(front) {
                let size = (it.w.min(it.h) * 0.25 * f / z).clamp(0.0, 18.0);
                if size >= 7.0 && dot(sub(eye, front), [0.0, 0.0, 1.0]) > 0.0 {
                    painter.text(q, egui::Align2::CENTER_CENTER, &it.tag, egui::FontId::proportional(size), Color32::from_rgb(20, 30, 50));
                }
            }
        }
        // Picking
        if resp.clicked_by(PointerButton::Primary) {
            if let Some(ptr) = resp.interact_pointer_pos() {
                // Faces are in drawing order: the last one under the pointer is visible.
                let hit = pick.iter().rev().find(|(_, pts, _)| point_in_convex(ptr, pts)).map(|x| x.2);
                let on_device = hit.and_then(|id| e.item(id)).map(|i| i.kind == ItemKind::Device).unwrap_or(false);
                let mut candidates: Vec<(usize, f32)> = Vec::new();
                for (a, b, wi) in &wire_segs {
                    let d = crate::geom::dist_to_segment(p(ptr.x, ptr.y), p(a.x, a.y), p(b.x, b.y));
                    if d <= 6.0 {
                        match candidates.iter_mut().find(|c| c.0 == *wi) {
                            Some(c) => c.1 = c.1.min(d),
                            None => candidates.push((*wi, d)),
                        }
                    }
                }
                candidates.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
                let candidates: Vec<usize> = candidates.into_iter().map(|c| c.0).collect();
                match hit {
                    Some(id) if on_device => self.select_item(id),
                    _ if !candidates.is_empty() => {
                        self.pick_wire(an, &candidates);
                        return;
                    }
                    Some(id) => self.select_item(id),
                    None => {
                        self.selection.clear();
                        self.selected_wire = None;
                    }
                }
            }
        }
        painter.text(
            rect.left_bottom() + vec2(10.0, -10.0),
            egui::Align2::LEFT_BOTTOM,
            tr("Drag: orbit · right/middle drag: pan · wheel: zoom · click: select device or wire"),
            egui::FontId::proportional(12.0),
            if dark { Color32::GRAY } else { Color32::DARK_GRAY },
        );
        if self.selected_wire.is_none() {
            self.status = format!("{}  {:.0} × {:.0} × {:.0} mm", e.name, e.width, e.height, e.depth);
        }
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

/// Green below 80 % of the limit, orange up to the limit, red above.
fn fill_color(percent: f32, limit: f32) -> Color32 {
    if percent > limit {
        Color32::from_rgb(220, 60, 60)
    } else if percent > 0.8 * limit {
        Color32::from_rgb(230, 150, 30)
    } else {
        Color32::from_rgb(60, 170, 90)
    }
}

fn end_kind(kind: panel::EndKind) -> &'static str {
    match kind {
        panel::EndKind::Exact => tr("connection point"),
        panel::EndKind::Generated => tr("connection point (generated)"),
        panel::EndKind::Centre => tr("device centre"),
    }
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

#[cfg(test)]
mod tests;
