//! Panel layout workspace: enclosure navigator, 2D mounting plate editor, 3D view
//! and property editor.

use openecraft_i18n::{tr, tr_fmt};
use crate::editor::{EguiDraw, Palette};
use crate::geom::{p, P};
use crate::model::{natural_cmp, Project};
use crate::panel::{self, Align, Anchor, Enclosure, Face, Fastening, ItemKind, PanelItem, Role, PLATE_MARGIN, PLATE_THICKNESS, PLATE_Z};
use crate::parts::{self, PartsDb};
use crate::parts::vec3::{cross, dot, norm, sub, Orbit};
use crate::render::{self, Draw, Ink};
use crate::reports::Analysis;
use egui::{vec2, Color32, Key, PointerButton, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};
use std::collections::{BTreeSet, HashSet};

#[derive(Clone, Debug, PartialEq)]
pub enum Tool {
    Select,
    Rail,
    Duct,
    Busbar,
    Cutout,
    PlaceDevice(String),
    /// Accessory of this part.
    Accessory(String),
    /// 3D macro of this name.
    Macro(String),
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

/// Climate settings and result of an enclosure (IEC/TR 60890).
fn climate_section(ui: &mut egui::Ui, prj: &mut Project, ei: usize) {
    use crate::climate::{self, Cooling, Installation, Material};
    let result = climate::calculate(prj, &prj.layout.enclosures[ei]);
    let c = &mut prj.layout.enclosures[ei].climate;
    egui::CollapsingHeader::new(tr("Climate (IEC 60890)")).id_salt("climate").show(ui, |ui| {
        egui::Grid::new("pclimate").num_columns(2).spacing([10.0, 5.0]).show(ui, |ui| {
            ui.label(tr("Installation"));
            egui::ComboBox::from_id_salt("installation").selected_text(c.installation.title()).show_ui(ui, |ui| {
                for i in Installation::ALL {
                    ui.selectable_value(&mut c.installation, i, i.title());
                }
            });
            ui.end_row();
            ui.label(tr("Material"));
            egui::ComboBox::from_id_salt("material").selected_text(c.material.title()).show_ui(ui, |ui| {
                for m in Material::ALL {
                    ui.selectable_value(&mut c.material, m, m.title());
                }
            });
            ui.end_row();
            ui.label(tr("Ambient temperature"));
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut c.ambient_min).speed(0.5).range(-50.0..=80.0).suffix(" °C"));
                ui.label("…");
                ui.add(egui::DragValue::new(&mut c.ambient_max).speed(0.5).range(-50.0..=80.0).suffix(" °C"));
            });
            ui.end_row();
            ui.label(tr("Inside temperature"));
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut c.inside_min).speed(0.5).range(-50.0..=80.0).suffix(" °C"));
                ui.label("…");
                ui.add(egui::DragValue::new(&mut c.inside_max).speed(0.5).range(-50.0..=80.0).suffix(" °C"));
            });
            ui.end_row();
            ui.label(tr("Additional power loss"));
            ui.add(egui::DragValue::new(&mut c.extra_loss).speed(1.0).range(0.0..=100000.0).suffix(" W"))
                .on_hover_text(tr("Power loss not covered by the parts (tech data Power loss: 7.5 W)"));
            ui.end_row();
            ui.label(tr("Power loss"));
            ui.label(format!("{:.0} W", result.loss));
            ui.end_row();
            ui.label(tr("Effective surface"));
            ui.label(format!("{:.2} m²", result.surface));
            ui.end_row();
            ui.label(tr("Inside without cooling"));
            let hot = result.cooling != Cooling::None;
            let text = format!("{:.1} °C", result.inside);
            if hot {
                ui.colored_label(Color32::from_rgb(220, 120, 0), text);
            } else {
                ui.label(text);
            }
            ui.end_row();
            ui.label(tr("Cooling"));
            ui.label(result.cooling.text());
            ui.end_row();
            ui.label(tr("Heating"));
            ui.label(if result.heating > 0.0 { format!("{:.0} W", result.heating) } else { "–".into() });
            ui.end_row();
        });
        if !result.without_data.is_empty() {
            ui.weak(tr_fmt!("Without power loss data: {}", result.without_data.join(", ")));
        }
    });
}

struct NewEnclosure {
    name: String,
    part: String,
    /// Enclosure system the part list is narrowed to.
    system: String,
    width: f32,
    height: f32,
    depth: f32,
}

/// Rectangular pattern of copies of the selection.
struct PatternDialog {
    nx: usize,
    ny: usize,
    px: f32,
    py: f32,
}

/// Enclosure accessories of the parts database, the project and the catalog.
fn accessory_parts(db: &PartsDb, prj: &Project) -> Vec<parts::Part> {
    let mut v: Vec<parts::Part> = Vec::new();
    for p in db.parts.iter().chain(prj.parts.iter()).chain(parts::catalog()) {
        if parts::mechanics::ACCESSORY_SUBGROUPS.contains(&p.subgroup.as_str()) && !v.iter().any(|x| x.number == p.number) {
            v.push(p.clone());
        }
    }
    v.sort_by(|a, b| a.subgroup.cmp(&b.subgroup).then(natural_cmp(&a.number, &b.number)));
    v
}

/// The item and the other members of its group (assembly).
fn with_group(e: &Enclosure, id: u64) -> Vec<u64> {
    match e.item(id).map(|i| i.group) {
        Some(g) if g != 0 => panel::group_members(e, g),
        _ => vec![id],
    }
}

/// Placement along one axis: kind and distances.
fn align_editor(ui: &mut egui::Ui, salt: &str, a: &mut Align) {
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt(salt).selected_text(a.title()).width(90.0).show_ui(ui, |ui| {
            for k in Align::KINDS {
                if ui.selectable_label(a.same_kind(k), k.title()).clicked() && !a.same_kind(k) {
                    *a = k;
                }
            }
        });
        match a {
            Align::Free => {}
            Align::Start(v) | Align::Center(v) | Align::End(v) => {
                ui.add(egui::DragValue::new(v).speed(1.0).suffix(" mm"));
            }
            Align::Stretch(s, t) => {
                ui.add(egui::DragValue::new(s).speed(1.0));
                ui.add(egui::DragValue::new(t).speed(1.0).suffix(" mm"));
            }
        }
    });
}

pub struct PanelUi {
    pub enclosure: usize,
    pub tool: Tool,
    pub view: View,
    /// Face shown in the front view (mounting plate or door).
    pub face: Face,
    /// The door is seen from the inside (mirrored, hinges on the right).
    pub door_inside: bool,
    pub selection: BTreeSet<u64>,
    /// Selected wire of the virtual wiring (from, to).
    pub selected_wire: Option<(String, String)>,
    pub show_wires: bool,
    /// Wires marked as wired in the wiring assistant (drawn green in 3D).
    pub done_wires: HashSet<(String, String)>,
    /// The door is (being) opened in the 3D view.
    pub door_open: bool,
    /// Current opening angle of the door (radians), animated towards `door_open`.
    door_angle: f32,
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
    pattern: Option<PatternDialog>,
    /// Name of the 3D macro being saved from the selection.
    save_macro: Option<String>,
    /// Gap between devices lined up on a rail.
    rail_gap: f32,
}

impl Default for PanelUi {
    fn default() -> Self {
        PanelUi {
            enclosure: 0,
            tool: Tool::Select,
            view: View::Front,
            face: Face::Plate,
            door_inside: false,
            selection: BTreeSet::new(),
            selected_wire: None,
            show_wires: true,
            done_wires: HashSet::new(),
            door_open: true,
            door_angle: DOOR_OPEN_ANGLE,
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
            pattern: None,
            save_macro: None,
            rail_gap: 0.0,
        }
    }
}

/// Drawing layer of enclosure walls that are seen from the outside.
const OUTER_WALLS: u8 = 6;

const WIRE_COLOR: Color32 = Color32::from_rgb(40, 110, 230);
const WIRE_DONE: Color32 = Color32::from_rgb(50, 170, 80);
/// Opening angle of the door in the 3D view.
const DOOR_OPEN_ANGLE: f32 = 1.85;
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

    /// Selects an item with the other members of its assembly.
    fn select_assembly(&mut self, e: &Enclosure, id: u64) {
        self.selection = with_group(e, id).into_iter().collect();
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

    fn hidden(&self, it: &PanelItem) -> bool {
        hidden_behind_door(self.door_inside, it)
    }

    /// Tools of the face shown: no rails or busbars on the door, cable ducts only on its
    /// inside, cutouts from the front; no busbars on the walls.
    fn tool_allowed(&self, t: &Tool) -> bool {
        if self.face.is_wall() {
            return !matches!(t, Tool::Busbar);
        }
        !matches!(
            (self.face, self.door_inside, t),
            (Face::Door, _, Tool::Rail | Tool::Busbar) | (Face::Door, false, Tool::Duct) | (Face::Door, true, Tool::Cutout)
        )
    }

    /// Item kind and default part of the profile tool on the face shown: the door gets
    /// the smaller door duct.
    fn profile_here(&self) -> Option<(ItemKind, &'static str)> {
        self.profile_here_for(&self.tool)
    }

    fn profile_here_for(&self, tool: &Tool) -> Option<(ItemKind, &'static str)> {
        match tool.profile()? {
            (ItemKind::Duct, _) if self.face == Face::Door => Some((ItemKind::Duct, panel::DOOR_DUCT_PART)),
            x => Some(x),
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

    pub fn reset_camera(&mut self) {
        self.cam = None;
    }

    /// Selects a wire and turns the 3D camera towards it (wiring assistant).
    pub fn focus_wire(&mut self, prj: &Project, w: &panel::RoutedWire) {
        let Some(e) = prj.layout.enclosures.get(w.enclosure) else { return };
        if self.enclosure != w.enclosure {
            self.cam = None;
        }
        self.enclosure = w.enclosure;
        self.select_wire(w.key());
        if w.door.0.is_some() || w.door.1.is_some() {
            self.door_open = true;
        }
        let pts = &w.route.points;
        if pts.is_empty() {
            return;
        }
        let n = pts.len() as f32;
        let c = p(pts.iter().map(|q| q.x).sum::<f32>() / n, pts.iter().map(|q| q.y).sum::<f32>() / n);
        let cam = self.cam.get_or_insert_with(|| default_cam(e));
        cam.target = [PLATE_MARGIN + c.x, e.height - PLATE_MARGIN - c.y, PLATE_Z];
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
                    system: String::new(),
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
                let text = match it.face {
                    Face::Plate => it.tag.clone(),
                    Face::Door => tr_fmt!("{}  (door)", it.tag),
                    f => format!("{}  ({})", it.tag, f.title()),
                };
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
            self.selection_tools(ui, prj, ei, &sel);
            return;
        }
        if let Some(id) = sel.first().copied() {
            let e = &mut prj.layout.enclosures[ei];
            let Some(idx) = e.items.iter().position(|i| i.id == id) else {
                self.selection.clear();
                return;
            };
            let mut it = e.items[idx].clone();
            let before_item = it.clone();
            let part = prj.parts.iter().chain(parts::catalog()).find(|p| p.number == it.part).cloned();
            let (face_w, face_h) = e.face_size(it.face);
            ui.label(RichText::new(it.kind.title()).strong());
            if it.kind == ItemKind::Accessory {
                if let Some(p) = &part {
                    ui.weak(&p.description);
                }
            }
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
                if matches!(it.kind, ItemKind::Device | ItemKind::Accessory) {
                    ui.label(tr("Width × height"));
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut it.w).speed(0.5).range(1.0..=5000.0));
                        ui.add(egui::DragValue::new(&mut it.h).speed(0.5).range(1.0..=5000.0));
                    });
                    ui.end_row();
                    ui.label(tr("Mounted on"));
                    let before = it.face;
                    let faces = part.as_ref().filter(|_| it.kind == ItemKind::Accessory).map(panel::allowed_faces).unwrap_or(Face::ALL.to_vec());
                    egui::ComboBox::from_id_salt("itemface").selected_text(it.face.title()).show_ui(ui, |ui| {
                        for f in faces {
                            ui.selectable_value(&mut it.face, f, f.title());
                        }
                    });
                    if it.face != before {
                        // Keep the position in front view; door devices sit directly on the door.
                        let shift = it.face.origin() - before.origin();
                        it.x -= shift;
                        it.y -= shift;
                        it.z = if it.kind == ItemKind::Accessory { before_item.z.min(0.0) } else { 0.0 };
                        it.anchor = Anchor::default();
                        self.face = it.face;
                        self.door_inside = false;
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
                    if it.kind == ItemKind::Rail {
                        ui.label(tr("Standoff height"));
                        ui.add(egui::DragValue::new(&mut it.z).speed(1.0).range(0.0..=200.0).suffix(" mm"))
                            .on_hover_text(tr("Rail raised on standoffs above the plate (devices on it move up)"));
                        ui.end_row();
                    }
                }
                if it.kind != ItemKind::Cutout {
                    ui.label(tr("Fastening"));
                    let resolved = Fastening::resolve(&PanelItem { fastening: Fastening::Auto, ..it.clone() }, part.as_ref());
                    let text = |f: Fastening| if f == Fastening::Auto { format!("{} ({})", f.title(), resolved.title()) } else { f.title().to_string() };
                    egui::ComboBox::from_id_salt("fastening").selected_text(text(it.fastening)).show_ui(ui, |ui| {
                        for f in Fastening::ALL {
                            ui.selectable_value(&mut it.fastening, f, text(f));
                        }
                    });
                    ui.end_row();
                }
                ui.label(tr("Horizontal position"));
                align_editor(ui, "anchorx", &mut it.anchor.x);
                ui.end_row();
                ui.label(tr("Vertical position"));
                align_editor(ui, "anchory", &mut it.anchor.y);
                ui.end_row();
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
            // A new anchor places the item; a moved anchored item keeps its anchor
            // with the new distances.
            if it.anchor != before_item.anchor {
                let a = it.anchor;
                a.apply(&mut it, (face_w, face_h));
            } else if !it.anchor.is_free() && (it.x, it.y, it.w, it.h) != (before_item.x, before_item.y, before_item.w, before_item.h) {
                it.anchor = it.anchor.capture(&it, e.face_size(it.face));
            }
            let group = it.group;
            if self.selection.is_empty() {
                e.items.remove(idx);
            } else if it != e.items[idx] {
                e.items[idx] = it;
            }
            if group != 0 {
                let name = e.group(group).map(|g| g.name.clone()).unwrap_or_default();
                ui.horizontal(|ui| {
                    ui.label(tr_fmt!("Assembly: {}", name));
                    if ui.small_button(tr("Select assembly")).clicked() {
                        self.selection = panel::group_members(e, group).into_iter().collect();
                    }
                    if ui.small_button(tr("Ungroup")).clicked() {
                        ungroup(e, &[id]);
                    }
                });
            }
            if !self.selection.is_empty() {
                self.selection_tools(ui, prj, ei, &sel);
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
        let size_before = (e.width, e.height, e.depth);
        ui.label(RichText::new(tr("Enclosure")).strong());
        egui::Grid::new("penc").num_columns(2).spacing([10.0, 5.0]).show(ui, |ui| {
            ui.label(tr("Location"));
            ui.text_edit_singleline(&mut e.name);
            ui.end_row();
            ui.label(tr("Enclosure system"));
            let current = e.system().map(|s| s.title()).unwrap_or(tr("(none)"));
            let mut choice: Option<String> = None;
            egui::ComboBox::from_id_salt("encsystem").selected_text(current).width(200.0).show_ui(ui, |ui| {
                if ui.selectable_label(e.system().is_none(), tr("(none)")).clicked() {
                    choice = Some(String::new());
                }
                for s in &panel::SYSTEMS {
                    if ui.selectable_label(e.build.system == s.id(), s.title()).clicked() {
                        choice = Some(s.id().to_string());
                    }
                }
            });
            if let Some(id) = choice {
                panel::apply_system(e, &id, None);
            }
            ui.end_row();
            if let Some(s) = e.system() {
                ui.label(tr("Standard size"));
                let now = (e.width, e.height, e.depth);
                let label = |(w, h, d): (f32, f32, f32)| format!("{w:.0} × {h:.0} × {d:.0}");
                let standard = s.data.sizes.contains(&now);
                let mut size = None;
                egui::ComboBox::from_id_salt("encsize").selected_text(if standard { label(now) } else { tr("(custom size)").to_string() }).show_ui(ui, |ui| {
                    for x in s.data.sizes {
                        if ui.selectable_label(*x == now, label(*x)).clicked() {
                            size = Some(*x);
                        }
                    }
                });
                if size.is_some() {
                    let id = s.id();
                    panel::apply_system(e, id, size);
                }
                ui.end_row();
            }
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
            ui.label(tr("Plinth"));
            ui.add(egui::DragValue::new(&mut e.build.plinth).speed(5.0).range(0.0..=300.0).suffix(" mm")).on_hover_text(tr("Height of the plinth, 0 without plinth"));
            ui.end_row();
            if e.system().is_some_and(|s| s.data.bayable) {
                ui.label(tr("Bayed"));
                ui.horizontal(|ui| {
                    ui.checkbox(&mut e.build.left_open, tr("left")).on_hover_text(tr("Bayed to the neighbouring enclosure: no side panel, a baying kit instead"));
                    ui.checkbox(&mut e.build.right_open, tr("right"));
                });
                ui.end_row();
            }
            ui.label(tr("Gland plate"));
            ui.checkbox(&mut e.build.gland_plate, tr("Bottom with removable gland plate"));
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
        if (e.width, e.height, e.depth) != size_before {
            panel::sync_part(e);
            panel::apply_anchors(e);
        }
        egui::CollapsingHeader::new(tr("Components (assembly)")).id_salt("components").show(ui, |ui| {
            egui::Grid::new("pcomp").striped(true).num_columns(3).show(ui, |ui| {
                for c in panel::components(e) {
                    ui.label(c.role.title());
                    ui.label(RichText::new(if c.part.is_empty() { tr("(included)") } else { &c.part }).monospace());
                    ui.label(if c.count > 1 { format!("{} ×", c.count) } else { String::new() });
                    ui.end_row();
                }
            });
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
        ui.checkbox(&mut prj.layout.hardware, tr("Fastening hardware in the bill of materials")).on_hover_text(tr("Screws, nuts, washers and rivet nuts of the fixing points"));
        climate_section(ui, prj, ei);
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
        if self.face.is_wall() {
            let face = self.face;
            let n = crate::production::holes_on(e, &prj.parts, face).len();
            ui.label(tr_fmt!("{}: {} holes, {} cutouts", face.title(), n, crate::production::openings(e, &prj.parts, face).len()));
            ui.horizontal(|ui| {
                if ui.button(tr("⬇ NC data of this face (DXF)")).clicked() {
                    self.export = Some((ei, face, true));
                }
                if ui.button(tr("⬇ Drilling list of this face (CSV)")).clicked() {
                    self.export = Some((ei, face, false));
                }
            });
        }
        if problems > 0 {
            ui.colored_label(Color32::from_rgb(220, 60, 60), tr_fmt!("{} placement problems (red) – see Check", problems));
        }
        if ui.button(tr("➕ Insert mounting layout page")).clicked() {
            self.add_page = Some(e.id);
        }
    }

    /// Tools for the selected items: assemblies, aligning, rails, patterns and macros.
    fn selection_tools(&mut self, ui: &mut egui::Ui, prj: &mut Project, ei: usize, sel: &[u64]) {
        let e = &mut prj.layout.enclosures[ei];
        ui.separator();
        if sel.len() > 1 {
            ui.horizontal(|ui| {
                if ui.button(tr("Group as assembly")).on_hover_text(tr("The items are selected and moved together")).clicked() {
                    let n = e.groups.len() + 1;
                    let id = e.items.iter().map(|i| i.id).chain(e.groups.iter().map(|g| g.id)).max().unwrap_or(0) + 1;
                    ungroup(e, sel);
                    e.groups.push(panel::ItemGroup { id, name: tr_fmt!("Assembly {}", n), source: String::new() });
                    for it in e.items.iter_mut().filter(|i| sel.contains(&i.id)) {
                        it.group = id;
                    }
                }
                if e.items.iter().any(|i| sel.contains(&i.id) && i.group != 0) && ui.button(tr("Ungroup")).clicked() {
                    ungroup(e, sel);
                }
            });
            ui.horizontal_wrapped(|ui| {
                for (to, icon) in panel::AlignTo::ALL.into_iter().zip(["⏮", "↔", "⏭", "⏫", "↕", "⏬"]) {
                    if ui.small_button(icon).on_hover_text(to.title()).clicked() {
                        panel::align(e, sel, to);
                    }
                }
                if ui.small_button("|↔|").on_hover_text(tr("Distribute horizontally")).clicked() {
                    panel::distribute(e, sel, true);
                }
                if ui.small_button("|↕|").on_hover_text(tr("Distribute vertically")).clicked() {
                    panel::distribute(e, sel, false);
                }
            });
            let rail = e.items.iter().find(|i| sel.contains(&i.id) && i.kind == ItemKind::Rail).map(|i| i.id);
            if let Some(rail) = rail {
                ui.horizontal(|ui| {
                    let line_up = ui.button(tr("Line up on the rail")).on_hover_text(tr("The selected devices side by side on the selected rail, from its left end")).clicked();
                    ui.label(tr("Gap"));
                    ui.add(egui::DragValue::new(&mut self.rail_gap).speed(0.5).range(0.0..=100.0).suffix(" mm"));
                    if line_up && !panel::arrange_on_rail(e, rail, sel, self.rail_gap) {
                        self.status = tr("The devices do not fit onto the rail").to_string();
                    }
                });
            }
        }
        ui.horizontal(|ui| {
            if ui.button(tr("Pattern…")).on_hover_text(tr("Copies in rows and columns")).clicked() {
                self.pattern = Some(PatternDialog { nx: 2, ny: 1, px: 50.0, py: 50.0 });
            }
            if ui.button(tr("Save as 3D macro…")).clicked() {
                self.save_macro = Some(String::new());
            }
        });
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
            ui.label(w.routing());
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
        self.pattern_dialog(ctx, prj);
        self.save_macro_dialog(ctx, prj);
        let Some(ne) = &mut self.new_enclosure else { return };
        let mut open = true;
        let mut create = false;
        let mut enclosures: Vec<&parts::Part> = Vec::new();
        for p in db.parts.iter().chain(prj.parts.iter()).chain(parts::catalog()) {
            let system = panel::system_of_part(p).map(|s| s.id()).unwrap_or("");
            let wanted = ne.system.is_empty() || system == ne.system;
            if p.subgroup == "Enclosure" && p.width > 0.0 && wanted && !enclosures.iter().any(|x| x.number == p.number) {
                enclosures.push(p);
            }
        }
        egui::Window::new(tr("New enclosure")).open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
            egui::Grid::new("newenc").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label(tr("Location"));
                ui.text_edit_singleline(&mut ne.name);
                ui.end_row();
                ui.label(tr("Enclosure system"));
                let title = panel::SYSTEMS.iter().find(|s| s.id() == ne.system).map(|s| s.title()).unwrap_or(tr("(all)"));
                egui::ComboBox::from_id_salt("newencsys").selected_text(title).width(220.0).show_ui(ui, |ui| {
                    ui.selectable_value(&mut ne.system, String::new(), tr("(all)"));
                    for s in &panel::SYSTEMS {
                        ui.selectable_value(&mut ne.system, s.id().to_string(), s.title());
                    }
                });
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
                if !ne.system.is_empty() {
                    panel::apply_system(&mut e, &ne.system, None);
                }
            }
            prj.layout.enclosures.push(e);
            self.enclosure = prj.layout.enclosures.len() - 1;
            self.selection.clear();
            self.fit_pending = true;
        } else if !open {
            self.new_enclosure = None;
        }
    }

    fn pattern_dialog(&mut self, ctx: &egui::Context, prj: &mut Project) {
        let Some(pd) = &mut self.pattern else { return };
        let mut open = true;
        let mut apply = false;
        egui::Window::new(tr("Pattern")).open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
            egui::Grid::new("pattern").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label(tr("Columns × rows"));
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut pd.nx).range(1..=50));
                    ui.add(egui::DragValue::new(&mut pd.ny).range(1..=50));
                });
                ui.end_row();
                ui.label(tr("Pitch X / Y"));
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut pd.px).speed(1.0).suffix(" mm"));
                    ui.add(egui::DragValue::new(&mut pd.py).speed(1.0).suffix(" mm"));
                });
                ui.end_row();
            });
            ui.weak(tr("Devices are not copied: a device is placed once."));
            apply = ui.button(tr("✔ Copy")).clicked();
        });
        if apply {
            let ids: Vec<u64> = self.selection.iter().copied().collect();
            let new = panel::pattern_copy(prj, self.enclosure, &ids, pd.nx, pd.ny, pd.px, pd.py);
            self.status = tr_fmt!("{} copies placed", new.len());
            self.pattern = None;
        } else if !open {
            self.pattern = None;
        }
    }

    fn save_macro_dialog(&mut self, ctx: &egui::Context, prj: &mut Project) {
        let Some(name) = &mut self.save_macro else { return };
        let mut open = true;
        let mut save = false;
        egui::Window::new(tr("Save as 3D macro")).open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(tr("Name"));
                ui.text_edit_singleline(name);
            });
            ui.weak(tr("Items with an anchor keep it; the others are inserted relative to the click."));
            save = ui.add_enabled(!name.trim().is_empty(), egui::Button::new(tr("✔ Save"))).clicked();
        });
        if save {
            let ids: Vec<u64> = self.selection.iter().copied().collect();
            let name = name.trim().to_string();
            if let Some(m) = prj.layout.enclosures.get(self.enclosure).and_then(|e| panel::macro_from_items(e, &ids, &name)) {
                prj.layout.macros.retain(|x| x.name != name);
                prj.layout.macros.push(m);
                self.status = tr_fmt!("3D macro {} saved", name);
            }
            self.save_macro = None;
        } else if !open {
            self.save_macro = None;
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
                for (face, inside, label, tip) in [
                    (Face::Plate, false, Face::Plate.title(), ""),
                    (Face::Door, false, tr("Door (front)"), tr("The door seen from the front: door devices and cutouts")),
                    (Face::Door, true, tr("Door (inside)"), tr("The door seen from the inside (hinges on the right): cable ducts and the wiring behind the door")),
                ] {
                    let current = self.face == face && (face == Face::Plate || self.door_inside == inside);
                    let resp = ui.selectable_label(current, label);
                    let resp = if tip.is_empty() { resp } else { resp.on_hover_text(tip) };
                    if resp.clicked() && !current {
                        self.show_face(face, inside);
                    }
                }
                let wall = if self.face.is_wall() { self.face.title() } else { tr("Walls") };
                let mut picked = None;
                egui::ComboBox::from_id_salt("wallface")
                    .selected_text(RichText::new(wall).strong().color(if self.face.is_wall() { ui.visuals().strong_text_color() } else { ui.visuals().text_color() }))
                    .show_ui(ui, |ui| {
                        for f in Face::WALLS {
                            if ui.selectable_label(self.face == f, f.title()).clicked() {
                                picked = Some(f);
                            }
                        }
                    })
                    .response
                    .on_hover_text(tr("Side walls, rear wall, roof and bottom seen from the outside: fans, cable glands, cutouts"));
                if let Some(f) = picked {
                    self.show_face(f, false);
                }
                ui.separator();
            }
            if self.view == View::ThreeD {
                self.door_button(ui);
            }
            ui.checkbox(&mut self.show_wires, tr("Wires"));
            ui.checkbox(&mut self.show_fill, tr("Duct fill")).on_hover_text(tr("Colour the cable ducts by their fill level"));
            if ui.button(tr("⛶ Fit")).clicked() {
                self.fit_pending = true;
                self.cam = None;
            }
        });
        if self.view == View::Front {
            ui.horizontal_wrapped(|ui| self.tool_bar(ui, prj, db));
        }
        ui.separator();
        match self.view {
            View::Front => self.front_view(ui, prj, ei, an, db),
            View::ThreeD => self.view_3d(ui, prj, ei, an),
        }
    }

    /// Editing tools of the front view: profiles, cutouts, accessories, 3D macros.
    fn tool_bar(&mut self, ui: &mut egui::Ui, prj: &mut Project, db: &PartsDb) {
            let tool = self.tool.clone();
            for (t, label, tip) in [
                (Tool::Select, tr("⬉ Select"), tr("Select / move (Esc)")),
                (Tool::Rail, tr("Mounting rail"), tr("Click start and end point")),
                (Tool::Duct, tr("Cable duct"), tr("Click start and end point")),
                (Tool::Busbar, tr("Busbar"), tr("Copper busbar system: click start and end point")),
                (Tool::Cutout, tr("Cutout"), tr("Opening in the mounting plate / door: click two corners (round: in the properties)")),
            ] {
                if !self.tool_allowed(&t) {
                    continue;
                }
                if ui.selectable_label(tool == t, label).on_hover_text(tip).clicked() {
                    self.set_tool(t);
                }
            }
            if let Tool::PlaceDevice(tag) = &tool {
                ui.label(RichText::new(tr_fmt!("Placing {}", tag)).strong());
            }
            // Accessories for the face shown, 3D macros.
            let face = self.face;
            let accessories: Vec<parts::Part> = accessory_parts(db, prj).into_iter().filter(|p| panel::allowed_faces(p).contains(&face)).collect();
            let current = match &tool {
                Tool::Accessory(part) => accessories.iter().find(|p| &p.number == part).map(|p| p.subgroup.clone()).unwrap_or(part.clone()),
                _ => tr("Accessory").to_string(),
            };
            egui::ComboBox::from_id_salt("accessory")
                .selected_text(current)
                .width(130.0)
                .show_ui(ui, |ui| {
                    for p in &accessories {
                        let active = tool == Tool::Accessory(p.number.clone());
                        if ui.selectable_label(active, format!("{} – {}", p.subgroup, p.number)).on_hover_text(&p.description).clicked() {
                            self.set_tool(Tool::Accessory(p.number.clone()));
                        }
                    }
                })
                .response
                .on_hover_text(tr("Fans, filters, cable glands, lights, handles: choose one, then click where it goes"));
            let macros = panel::all_macros(prj);
            let current = match &tool {
                Tool::Macro(name) => macros.iter().find(|m| &m.name == name).map(|m| m.title().to_string()).unwrap_or(name.clone()),
                _ => tr("3D macro").to_string(),
            };
            let mut remove = None;
            egui::ComboBox::from_id_salt("macro")
                .selected_text(current)
                .width(130.0)
                .show_ui(ui, |ui| {
                    for m in &macros {
                        ui.horizontal(|ui| {
                            let active = tool == Tool::Macro(m.name.clone());
                            let text = if m.is_builtin() { m.title().to_string() } else { format!("★ {}", m.name) };
                            if ui.selectable_label(active, text).on_hover_text(&m.description).clicked() {
                                self.set_tool(Tool::Macro(m.name.clone()));
                            }
                            if !m.is_builtin() && ui.small_button("🗑").on_hover_text(tr("Delete macro")).clicked() {
                                remove = Some(m.name.clone());
                            }
                        });
                    }
                })
                .response
                .on_hover_text(tr("Insert a group of items (anchored items go to their place, the others where you click)"));
            if let Some(name) = remove {
                prj.layout.macros.retain(|m| m.name != name);
            }
    }

    /// Shows a face in the front view.
    fn show_face(&mut self, face: Face, inside: bool) {
        if self.face != face {
            self.selection.clear();
            self.selected_wire = None;
            self.fit_pending = true;
        }
        self.face = face;
        self.door_inside = inside;
        if !self.tool_allowed(&self.tool) {
            self.set_tool(Tool::Select);
        }
    }

    /// Mirror axis of the inside view of the door: face x ↔ `width - x`.
    fn mirror(&self, e: &Enclosure) -> Option<f32> {
        (self.face == Face::Door && self.door_inside).then_some(e.width)
    }

    fn s2w(&self, rect: Rect, s: Pos2) -> P {
        p((s.x - rect.min.x - self.offset.x) / self.zoom, (s.y - rect.min.y - self.offset.y) / self.zoom)
    }

    fn front_view(&mut self, ui: &mut egui::Ui, prj: &mut Project, ei: usize, an: &Analysis, db: &PartsDb) {
        let pal = Palette::new(ui.visuals().dark_mode);
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = resp.rect;
        let (ew, eh) = prj.layout.enclosures[ei].view_size(self.face);
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
        let mirror = self.mirror(&prj.layout.enclosures[ei]);
        let unmirror = move |q: P| p(mirror.map(|m| m - q.x).unwrap_or(q.x), q.y);
        let plate = ptr.map(|s| unmirror(self.s2w(rect, s)) - p(fm, fm));
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
            if !self.tool_allowed(&self.tool) {
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
                            let (kind, part) = self.profile_here().unwrap_or((ItemKind::Duct, panel::DUCT_PART));
                            if db.get(part).is_some() {
                                parts::ensure_in_project(prj, db, part);
                            }
                            if kind == ItemKind::Busbar && db.get(crate::production::BUSBAR_HOLDER).is_some() {
                                parts::ensure_in_project(prj, db, crate::production::BUSBAR_HOLDER);
                            }
                            let item = PanelItem { face: self.face, ..panel::new_profile(prj, kind, part, a, sn) };
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
                            let t = prj.layout.enclosures[ei].wall();
                            let (z, d) = match self.face {
                                Face::Plate => (-PLATE_THICKNESS, PLATE_THICKNESS),
                                Face::Door => (0.0, 2.0),
                                _ => (-t, t),
                            };
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
            Tool::Accessory(part) => {
                if let (true, Some(q)) = (resp.clicked_by(PointerButton::Primary), plate) {
                    if db.get(&part).is_some() {
                        parts::ensure_in_project(prj, db, &part);
                    }
                    let probe = panel::new_accessory(prj, &part, self.face, p(0.0, 0.0));
                    if let Some(id) = panel::place_accessory(prj, ei, &part, self.face, p(q.x - probe.w / 2.0, q.y - probe.h / 2.0)) {
                        self.select_item(id);
                    }
                }
            }
            Tool::Macro(name) => {
                if let (true, Some(sn)) = (resp.clicked_by(PointerButton::Primary), snapped) {
                    if let Some(m) = panel::all_macros(prj).into_iter().find(|m| m.name == name) {
                        for it in m.items.iter().filter(|i| !i.part.is_empty()) {
                            if db.get(&it.part).is_some() {
                                parts::ensure_in_project(prj, db, &it.part);
                            }
                        }
                        let ids = panel::insert_macro(prj, ei, &m, self.face, sn);
                        self.selection = ids.into_iter().collect();
                        self.status = tr_fmt!("3D macro {} inserted", m.title());
                    }
                }
            }
        }

        // Draw
        let e = &prj.layout.enclosures[ei];
        painter.rect_filled(rect, 0.0, pal.canvas);
        let origin = rect.min + self.offset;
        let problems = panel::problem_items(e, &prj.parts);
        let mut d = EguiDraw { painter: &painter, origin, zoom: self.zoom, pal: &pal, force: None, mirror };
        render::draw_enclosure(&mut d, e, self.face, self.door_inside, p(0.0, 0.0), 1.0, &self.selection, &problems, prj);
        let to_screen = |q: P| origin + vec2(mirror.map(|m| m - (q.x + fm)).unwrap_or(q.x + fm), q.y + fm) * self.zoom;
        // Cable ducts coloured by their fill level.
        if self.show_fill {
            let limit = prj.layout.duct_fill_limit;
            for f in an.duct_fill.iter().filter(|f| f.enclosure == ei) {
                let Some(d) = e.item(f.duct).filter(|d| d.face == self.face && !self.hidden(d)) else { continue };
                let r = Rect::from_two_pos(to_screen(p(d.x, d.y)), to_screen(p(d.x + d.w, d.y + d.h)));
                let color = fill_color(f.percent, limit);
                painter.rect_filled(r, 0.0, color.gamma_multiply(0.3));
                if r.width().min(r.height()) >= 12.0 {
                    painter.text(r.center(), egui::Align2::CENTER_CENTER, format!("{:.0} %", f.percent), egui::FontId::proportional(11.0), color);
                }
            }
        }
        // Wires on the face shown: the routes on the plate, or on the door from the door
        // devices to the hinge.
        let face = self.face;
        let selected_wire = self.wire_index(an).filter(|i| an.wiring[*i].enclosure == ei && !an.wiring[*i].paths_on(face).is_empty());
        // The wiring of the door is behind it: seen from the front only the selected wire.
        let show_wires = self.show_wires && !(face == Face::Door && !self.door_inside);
        if show_wires || selected_wire.is_some() {
            // Other wires are dimmed while one is selected.
            let normal = if selected_wire.is_some() { WIRE_COLOR.gamma_multiply(0.25) } else { WIRE_COLOR.gamma_multiply(0.7) };
            for (i, w) in an.wiring.iter().enumerate().filter(|(_, w)| w.enclosure == ei) {
                if Some(i) != selected_wire && show_wires {
                    for path in w.paths_on(face) {
                        let pts: Vec<Pos2> = path.iter().map(|q| to_screen(*q)).collect();
                        painter.add(Shape::line(pts, Stroke::new(1.0, normal)));
                    }
                }
            }
        }
        if let Some(i) = selected_wire {
            let w = &an.wiring[i];
            for path in w.paths_on(face) {
                let pts: Vec<Pos2> = path.iter().map(|q| to_screen(*q)).collect();
                painter.add(Shape::line(pts.clone(), Stroke::new(3.0, WIRE_HIGHLIGHT)));
                for end in [pts.first(), pts.last()].into_iter().flatten() {
                    painter.circle_filled(*end, 4.5, WIRE_HIGHLIGHT);
                }
            }
            // The connected devices on this face
            for id in [w.items.0, w.items.1] {
                if let Some(it) = e.item(id).filter(|it| it.face == face) {
                    let r = Rect::from_two_pos(to_screen(p(it.x, it.y)), to_screen(p(it.x + it.w, it.y + it.h)));
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
        let mut g = EguiDraw { painter: &painter, origin, zoom: self.zoom, pal: &pal, force: Some(Ink::Ghost), mirror };
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
                    let (kind, part) = self.profile_here().unwrap_or((ItemKind::Duct, panel::DUCT_PART));
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
            (Tool::Accessory(part), _, Some(q)) => {
                let pd = prj.parts.iter().chain(parts::catalog()).find(|x| &x.number == part);
                let [w, h, _] = pd.and_then(|x| x.size()).unwrap_or([panel::DEFAULT_DEVICE.0, panel::DEFAULT_DEVICE.1, 0.0]);
                let (x, y) = (panel::snap(q.x - w / 2.0), panel::snap(q.y - h / 2.0));
                g.rect(p(x, y) + m, p(x + w, y + h) + m, 0.6, Ink::Ghost, false);
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
        let inside = self.door_inside;
        let hidden = move |i: &PanelItem| hidden_behind_door(inside, i);
        let hit = |e: &Enclosure, q: P| -> Option<u64> {
            // Top layer first: devices, busbars, ducts, rails, cutouts.
            let mut items: Vec<&PanelItem> = e.items.iter().filter(|i| i.face == face && !hidden(i) && i.rect().expand(tol).contains(q)).collect();
            items.sort_by_key(|i| std::cmp::Reverse(i.kind.layer()));
            items.first().map(|i| i.id)
        };
        let (offset, zoom, fm) = (self.offset, self.zoom, face.origin());
        let mirror = self.mirror(&prj.layout.enclosures[ei]);
        let to_plate = move |s: Pos2| {
            let x = (s.x - rect.min.x - offset.x) / zoom;
            p(mirror.map(|m| m - x).unwrap_or(x) - fm, (s.y - rect.min.y - offset.y) / zoom - fm)
        };
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
                    self.selection.extend(with_group(e, id));
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
            if let Some(drag) = self.drag.take() {
                let moved: Vec<u64> = drag.originals.iter().map(|o| o.0).collect();
                panel::capture_anchors(&mut prj.layout.enclosures[ei], &moved);
            }
        }
        if resp.clicked_by(PointerButton::Primary) {
            if let Some(ptr) = resp.interact_pointer_pos() {
                let q = to_plate(ptr);
                let e = &prj.layout.enclosures[ei];
                let item = hit(e, q);
                let on_device = item.and_then(|id| e.item(id)).map(|i| i.kind == ItemKind::Device).unwrap_or(false);
                // Wires lie on top of ducts and rails, devices on top of wires.
                if self.show_wires && !(face == Face::Door && !inside) && !on_device && !shift {
                    let candidates = panel::wires_at(&an.wiring, ei, face, q, tol);
                    if !candidates.is_empty() {
                        self.pick_wire(an, &candidates);
                        return;
                    }
                }
                self.selected_wire = None;
                let alt = ui.input(|i| i.modifiers.alt);
                match item {
                    Some(id) if shift => {
                        if !self.selection.remove(&id) {
                            self.selection.insert(id);
                        }
                    }
                    // Alt: the single item of an assembly.
                    Some(id) if alt => self.select_item(id),
                    Some(id) => self.select_assembly(e, id),
                    None if !shift => self.selection.clear(),
                    None => {}
                }
            }
        }
    }

    // ------------------------------------------------------------------ 3D

    /// Opens or closes the door in the 3D view.
    pub fn door_button(&mut self, ui: &mut egui::Ui) {
        let text = if self.door_open { tr("Close door") } else { tr("Open door") };
        if ui.button(text).on_hover_text(tr("Swing the enclosure door open or shut (door devices and their wires move with it)")).clicked() {
            self.door_open = !self.door_open;
        }
    }

    /// 3D view of the current enclosure without the toolbar (wiring assistant).
    pub fn view_3d_current(&mut self, ui: &mut egui::Ui, prj: &mut Project, an: &Analysis) {
        if let Some(ei) = self.enclosure_index(prj) {
            self.view_3d(ui, prj, ei, an);
        }
    }

    fn view_3d(&mut self, ui: &mut egui::Ui, prj: &mut Project, ei: usize, an: &Analysis) {
        let e = &prj.layout.enclosures[ei];
        let dark = ui.visuals().dark_mode;
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = resp.rect;
        let target = if self.door_open { DOOR_OPEN_ANGLE } else { 0.0 };
        if (self.door_angle - target).abs() > 1e-3 {
            let step = ui.input(|i| i.stable_dt).min(0.1) * 2.5;
            self.door_angle += (target - self.door_angle).clamp(-step, step);
            ui.ctx().request_repaint();
        }
        let (sin, cos) = self.door_angle.sin_cos();
        // The door turns about its hinge on the left front edge (view coordinates).
        let swing_dir = |v: [f32; 3]| [v[0] * cos - v[2] * sin, v[1], v[0] * sin + v[2] * cos];
        let swing = |q: [f32; 3]| {
            let r = swing_dir([q[0], q[1], q[2] - e.depth]);
            [r[0], r[1], r[2] + e.depth]
        };
        let cam = self.cam.get_or_insert_with(|| default_cam(e));
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
        let add_box = |prims: &mut Vec<(u8, f32, Prim)>, layer: u8, min: [f32; 3], max: [f32; 3], base: Color32, item: Option<u64>, door: bool| {
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
                let mut pts3: Vec<[f32; 3]> = corners.iter().map(|&b| c(b >> 2 & 1, b >> 1 & 1, b & 1)).collect();
                let n = if door {
                    pts3.iter_mut().for_each(|q| *q = swing(*q));
                    swing_dir(n)
                } else {
                    n
                };
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
        // The enclosure's components: walls, side panels, frame, plinth, mounting plate,
        // door leaf.
        let (w, h, dpt) = (e.width, e.height, e.depth);
        let t = e.wall();
        // Enclosure coordinates (y down) to the 3D view (y up).
        let up = |q: [f32; 3]| [q[0], h - q[1], q[2]];
        let up_box = |(a, b): ([f32; 3], [f32; 3])| (up([a[0], b[1], a[2]]), up([b[0], a[1], b[2]]));
        // The eye in enclosure coordinates.
        let eye_e = [eye[0], h - eye[1], eye[2]];
        // A wall seen from the outside covers the equipment (drawn last, translucent);
        // seen from the inside it is behind everything.
        let outside_of = |face: Face, b: ([f32; 3], [f32; 3])| {
            let n = panel::outward(face);
            let c = [0, 1, 2].map(|k| if n[k] > 0.0 { b.1[k] } else if n[k] < 0.0 { b.0[k] } else { (b.0[k] + b.1[k]) / 2.0 });
            dot(n, sub(eye_e, c)) > 0.0
        };
        let wall_layer = |outside: bool| if outside { OUTER_WALLS } else { 0 };
        let leaf_out = sub(eye, swing([w / 2.0, h / 2.0, dpt + t]));
        for c in panel::components(e) {
            let [r, g, b] = crate::export3d::component_color(c.role);
            let color = Color32::from_rgb(r, g, b);
            for bx in c.boxes {
                let (min, max) = up_box(bx);
                match c.role {
                    // Door leaf on its hinge: seen from the outside it covers the equipment
                    // (translucent), seen from the inside it is behind the door devices.
                    Role::Door => {
                        let layer = if dot(swing_dir([0.0, 0.0, 1.0]), leaf_out) > 0.0 { OUTER_WALLS } else { 3 };
                        add_box(&mut prims, layer, min, max, Color32::from_rgb(190, 194, 200), None, true);
                    }
                    Role::Wall(face) => add_box(&mut prims, wall_layer(outside_of(face, bx)), min, max, color, None, false),
                    Role::GlandPlate => add_box(&mut prims, wall_layer(outside_of(Face::Bottom, bx)), min, max, color, None, false),
                    Role::MountingPlate | Role::Body => add_box(&mut prims, 1, min, max, color, None, false),
                    Role::Plinth | Role::BayingKit => add_box(&mut prims, 0, min, max, color, None, false),
                }
            }
        }
        let to3 = |x: f32, y: f32, z: f32| [PLATE_MARGIN + x, h - PLATE_MARGIN - y, PLATE_Z + z];
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
                    ItemKind::Accessory => Color32::from_rgb(95, 100, 110),
                }
            };
            // Devices with a 3D model in their part are drawn with it instead of a box.
            let on_door = it.face == Face::Door;
            if let Some(tris) = panel::device_mesh(prj, it) {
                for tri in panel::place_triangles(e, it, tris) {
                    let w = tri.map(up).map(|q| if on_door { swing(q) } else { q });
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
            } else if it.kind == ItemKind::Cutout && it.face.is_wall() {
                // Through the wall, a little proud of both of its surfaces.
                let n = panel::outward(it.face);
                let (a, b) = ([0, 1, 2].map(|k| b0[k] - n[k].abs() * 0.5), [0, 1, 2].map(|k| b1[k] + n[k].abs() * 0.5));
                (up([a[0], b[1], a[2]]), up([b[0], a[1], b[2]]))
            } else {
                (up([b0[0], b1[1], b0[2]]), up([b1[0], b0[1], b1[2]]))
            };
            let layer = match it.kind {
                ItemKind::Cutout if it.face.is_wall() => wall_layer(outside_of(it.face, (b0, b1))),
                ItemKind::Cutout => 1,
                ItemKind::Rail => 2,
                ItemKind::Duct | ItemKind::Busbar => 3,
                ItemKind::Device | ItemKind::Accessory => 5,
            };
            add_box(&mut prims, layer, min, max, base, Some(it.id), on_door);
        }
        // Wires inside the ducts; the selected one is drawn on top of everything.
        let mut wire_segs: Vec<(Pos2, Pos2, usize)> = Vec::new();
        for (wi, wire) in an.wiring.iter().enumerate().filter(|(_, wire)| wire.enclosure == ei) {
            let selected = Some(wi) == selected_wire;
            let done = self.done_wires.contains(&wire.key());
            if !self.show_wires && !selected && !done {
                continue;
            }
            let z = if wire.route.via_ducts { panel::DUCT_PROFILE.1 * 0.5 } else { 5.0 };
            let (layer, color, width) = if selected {
                (OUTER_WALLS + 1, WIRE_HIGHLIGHT, 3.5)
            } else if done {
                (4, if selected_wire.is_some() { WIRE_DONE.gamma_multiply(0.5) } else { WIRE_DONE }, 1.5)
            } else if selected_wire.is_some() {
                (4, WIRE_COLOR.gamma_multiply(0.3), 1.0)
            } else {
                (4, WIRE_COLOR, 1.5)
            };
            // Door ends run along the door and over the hinge loop; the part on the
            // door swings with it.
            let door_end = |at: [f32; 3], route: &Option<panel::Route>, plate: Option<&P>| -> Vec<[f32; 3]> {
                let (Some(q), Some(route)) = (plate, route) else { return Vec::new() };
                let (path, n) = panel::door_path(e, at, route, [PLATE_MARGIN + q.x, PLATE_MARGIN + q.y, PLATE_Z + z]);
                path.into_iter().enumerate().map(|(i, v)| if i < n { swing(up(v)) } else { up(v) }).collect()
            };
            let path: Vec<[f32; 3]> = match wire.door {
                (Some(a), Some(b)) if wire.on_door() => match &wire.door_routes.0 {
                    Some(r) => panel::door_jumper_path(e, a, b, r).into_iter().map(|v| swing(up(v))).collect(),
                    None => Vec::new(),
                },
                (a, b) => {
                    let mut v = a.map(|a| door_end(a, &wire.door_routes.0, wire.route.points.first())).unwrap_or_default();
                    v.extend(wire.route.points.iter().map(|q| to3(q.x, q.y, z)));
                    if let Some(b) = b {
                        v.extend(door_end(b, &wire.door_routes.1, wire.route.points.last()).into_iter().rev());
                    }
                    v
                }
            };
            for seg in path.windows(2) {
                let (a, b) = (seg[0], seg[1]);
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
        let _ = (PLATE_THICKNESS, t);
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
            // Door devices show their front on the outside of the door leaf.
            let (front, normal) = if it.face == Face::Door {
                (swing(up([(b0[0] + b1[0]) / 2.0, (b0[1] + b1[1]) / 2.0, b1[2] + 2.5])), swing_dir([0.0, 0.0, 1.0]))
            } else {
                // The side facing into the enclosure.
                let n = panel::face_frame(e, it.face)[3];
                let q = panel::face_point(e, it.face, [it.x + it.w / 2.0, it.y + it.h / 2.0, it.z + it.d]);
                (up(q), [n[0], -n[1], n[2]])
            };
            if let Some((q, z)) = project(front) {
                let size = (it.w.min(it.h) * 0.25 * f / z).clamp(0.0, 18.0);
                if size >= 7.0 && dot(sub(eye, front), normal) > 0.0 {
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

/// Items behind the door (cable ducts) are hidden in its front view: they cannot be
/// picked there.
fn hidden_behind_door(inside: bool, it: &PanelItem) -> bool {
    it.face == Face::Door && !inside && it.kind == ItemKind::Duct
}

fn default_cam(e: &Enclosure) -> Orbit {
    Orbit { yaw: 0.55, pitch: 0.3, dist: e.width.max(e.height) * 1.9, target: [e.width / 2.0, e.height / 2.0, e.depth / 2.0] }
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

/// Dissolves the groups of the items (all members leave them).
fn ungroup(e: &mut Enclosure, ids: &[u64]) {
    let groups: HashSet<u64> = e.items.iter().filter(|i| ids.contains(&i.id) && i.group != 0).map(|i| i.group).collect();
    for it in e.items.iter_mut().filter(|i| groups.contains(&i.group)) {
        it.group = 0;
    }
    e.groups.retain(|g| !groups.contains(&g.id));
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
