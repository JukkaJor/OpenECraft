//! Panel layout workspace: enclosure navigator, 2D mounting plate editor, 3D view
//! and property editor.

use openecraft_i18n::{tr, tr_fmt};
use crate::editor::{EguiDraw, Palette};
use openecraft_core::geom::{p, P};
use openecraft_core::model::{natural_cmp, Project};
use openecraft_core::panel::{self, Align, Anchor, Enclosure, Face, Fastening, ItemKind, PanelItem, SitePlace, PLATE_MARGIN, PLATE_THICKNESS, PLATE_Z};
use openecraft_core::parts::{self, PartsDb};
use openecraft_core::parts::vec3::Orbit;
use openecraft_core::render::{self, Draw, Ink};
use openecraft_core::analysis::Analysis;
use openecraft_core::routing3d;
use egui::{vec2, Color32, Key, PointerButton, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};
use std::collections::{BTreeSet, HashMap, HashSet};
use view3d::default_cam;

mod routing_ui;
mod view3d;

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
    /// Routing path over the mounting plate: click the corners.
    RoutingPath,
    /// Routing reservation over the mounting plate: click two corners.
    Reservation,
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

/// File of an NC export: drawing (DXF, DWG) or drilling list (CSV).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NcFormat {
    Dxf,
    Dwg,
    Csv,
}

struct MoveDrag {
    start: P,
    originals: Vec<(u64, f32, f32)>,
}

/// Climate settings and result of an enclosure (IEC/TR 60890).
fn climate_section(ui: &mut egui::Ui, prj: &mut Project, ei: usize) {
    use openecraft_core::climate::{self, Cooling, Installation, Material};
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
            ui.label(tr("Altitude"));
            ui.add(egui::DragValue::new(&mut c.altitude).speed(10.0).range(0.0..=5000.0).suffix(" m")).on_hover_text(tr("Thinner air needs more air flow"));
            ui.end_row();
            ui.label(tr("Relative humidity"));
            ui.add(egui::DragValue::new(&mut c.humidity).speed(1.0).range(0.0..=100.0).suffix(" %")).on_hover_text(tr("At the highest ambient temperature; the heater keeps the dew point. 0 = not considered"));
            ui.end_row();
            ui.label("");
            ui.checkbox(&mut c.sealed, tr("Keep the degree of protection")).on_hover_text(tr("No filter fans: an air/air heat exchanger or a cooling unit"));
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
            if let Some(d) = result.dew_point {
                ui.label(tr("Dew point"));
                ui.label(format!("{d:.1} °C"));
                ui.end_row();
            }
        });
        if !result.without_data.is_empty() {
            ui.weak(tr_fmt!("Without power loss data: {}", result.without_data.join(", ")));
        }
    });
}

/// Collision class chooser: any class, the built-in classes and the project's own.
fn class_combo(ui: &mut egui::Ui, salt: &str, value: &mut String, custom: &[String]) {
    egui::ComboBox::from_id_salt(salt).selected_text(panel::class_title(value)).width(100.0).show_ui(ui, |ui| {
        ui.selectable_value(value, "*".to_string(), panel::class_title("*"));
        for (k, _) in panel::CLASSES {
            ui.selectable_value(value, k.to_string(), panel::class_title(k));
        }
        for k in custom {
            ui.selectable_value(value, k.clone(), k.as_str());
        }
    });
}

/// Classes named on items or in rules that are not built in.
fn custom_classes(prj: &Project) -> Vec<String> {
    let mut v: Vec<String> = prj.layout.enclosures.iter().flat_map(|e| e.items.iter().map(|i| i.class.trim().to_ascii_lowercase())).collect();
    v.extend(prj.layout.checks.classes.iter().flat_map(|r| [r.a.clone(), r.b.clone()]));
    v.retain(|k| !k.trim().is_empty() && k.trim() != "*" && !panel::CLASSES.iter().any(|c| c.0 == k));
    v.sort();
    v.dedup();
    v
}

/// Rules and results of the layout checks of an enclosure: clearances, service
/// space, heights above the floor, collision classes, thermal and manufacturing.
fn checks_section(ui: &mut egui::Ui, prj: &mut Project, ei: usize) {
    let findings = panel::validate(prj, &prj.layout.enclosures[ei]);
    let temps = {
        let e = &prj.layout.enclosures[ei];
        panel::thermal::device_temperatures(prj, e, &panel::thermal::heat_map(prj, e))
    };
    let custom = custom_classes(prj);
    egui::CollapsingHeader::new(tr("Layout checks")).id_salt("checks").show(ui, |ui| {
        let e = &mut prj.layout.enclosures[ei];
        ui.horizontal(|ui| {
            let mut known = e.elevation.is_some();
            let tip = tr("Bottom of the enclosure (without plinth) above the floor; heights are checked when it is known or the enclosure stands on a plinth");
            if ui.checkbox(&mut known, tr("Height above floor")).on_hover_text(tip).changed() {
                e.elevation = known.then_some(0.0);
            }
            if let Some(h) = &mut e.elevation {
                ui.add(egui::DragValue::new(h).speed(5.0).range(0.0..=5000.0).suffix(" mm"));
            }
        });
        let r = &mut prj.layout.checks;
        let row = |ui: &mut egui::Ui, label: &str, v: &mut f32, max: f32, suffix: &str, tip: &str| {
            ui.label(label);
            let resp = ui.add(egui::DragValue::new(v).speed(1.0).range(0.0..=max).suffix(suffix));
            if !tip.is_empty() {
                resp.on_hover_text(tip);
            }
            ui.end_row();
        };
        let range = |ui: &mut egui::Ui, label: &str, a: &mut f32, b: &mut f32, tip: &str| {
            ui.label(label);
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(a).speed(5.0).range(0.0..=5000.0));
                ui.label("…");
                ui.add(egui::DragValue::new(b).speed(5.0).range(0.0..=5000.0).suffix(" mm"));
            })
            .response
            .on_hover_text(tip);
            ui.end_row();
        };
        ui.label(RichText::new(tr("Clearances")).strong());
        egui::Grid::new("pchecks_clear").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
            row(ui, tr("Gap between devices"), &mut r.device_gap, 200.0, " mm", tr("Mounting clearance side by side; parts add theirs (Clearance left / right)"));
            row(ui, tr("Distance to cable ducts"), &mut r.duct_gap, 200.0, " mm", tr("Wiring space between devices and ducts"));
            row(ui, tr("Distance to the plate edge"), &mut r.edge_distance, 200.0, " mm", "");
            row(ui, tr("Service space in front"), &mut r.service_space, 500.0, " mm", tr("Free space in front of every device on the plate up to the door; parts add theirs (Service space)"));
        });
        ui.label(RichText::new(tr("Heights above floor (IEC 60204-1)")).strong());
        egui::Grid::new("pchecks_height").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
            range(ui, tr("Devices"), &mut r.operating_min, &mut r.operating_max, tr("All devices between these heights (11.2.1); 0 … 0 switches the check off"));
            range(ui, tr("Actuators on the door"), &mut r.actuator_min, &mut r.actuator_max, tr("Recommended height of manual actuators (10.1.2)"));
            row(ui, tr("Terminals at least"), &mut r.terminal_min, 2000.0, " mm", "");
        });
        ui.label(RichText::new(tr("Thermal")).strong());
        egui::Grid::new("pchecks_thermal").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
            row(ui, tr("Air at devices at most"), &mut r.device_temperature, 150.0, " °C", tr("For parts without temperature data (Max temperature, Operating temperature); 0: not checked"));
            ui.label(tr("Stratification (factor c)"));
            ui.add(egui::DragValue::new(&mut r.stratification).speed(0.01).range(1.0..=2.0)).on_hover_text(tr("Temperature rise at the roof against the middle (IEC/TR 60890)"));
            ui.end_row();
        });
        ui.label(RichText::new(tr("Manufacturing")).strong());
        egui::Grid::new("pchecks_mfg").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
            row(ui, tr("Web between holes"), &mut r.hole_web, 50.0, " mm", "");
            row(ui, tr("Hole to sheet edge"), &mut r.hole_edge, 50.0, " mm", "");
            row(ui, tr("Web at cutouts"), &mut r.cutout_web, 100.0, " mm", "");
            row(ui, tr("Shortest profile"), &mut r.min_profile, 500.0, " mm", tr("Rails, ducts and busbars shorter than this are hard to cut"));
            row(ui, tr("Stock length"), &mut r.stock_length, 10000.0, " mm", tr("For parts without Stock length; longer profiles have to be joined"));
        });
        ui.label(RichText::new(tr("Collision classes")).strong()).on_hover_text(tr("Distances between classes of items, or classes that may overlap"));
        let mut remove = None;
        egui::Grid::new("pchecks_classes").num_columns(5).spacing([6.0, 4.0]).show(ui, |ui| {
            for (i, c) in r.classes.iter_mut().enumerate() {
                class_combo(ui, &format!("cls_a{i}"), &mut c.a, &custom);
                class_combo(ui, &format!("cls_b{i}"), &mut c.b, &custom);
                ui.add_enabled(!c.ignore, egui::DragValue::new(&mut c.distance).speed(1.0).range(0.0..=1000.0).suffix(" mm"));
                ui.checkbox(&mut c.ignore, tr("may overlap"));
                if ui.small_button("🗑").on_hover_text(tr("Delete")).clicked() {
                    remove = Some(i);
                }
                ui.end_row();
            }
        });
        if let Some(i) = remove {
            r.classes.remove(i);
        }
        if ui.button(tr("➕ Add rule")).clicked() {
            r.classes.push(panel::ClassRule::distance("power", "signal", 50.0));
        }
        if !temps.is_empty() {
            egui::CollapsingHeader::new(tr("Air temperature at the devices")).id_salt("ptemps").show(ui, |ui| {
                egui::Grid::new("ptemps_grid").striped(true).num_columns(3).show(ui, |ui| {
                    for t in &temps {
                        ui.label(RichText::new(&t.tag).monospace());
                        let text = format!("{:.0} °C", t.temperature);
                        if t.limit.is_some_and(|l| t.temperature > l) {
                            ui.colored_label(Color32::from_rgb(220, 60, 60), text);
                        } else {
                            ui.label(text);
                        }
                        ui.label(t.limit.map(|l| tr_fmt!("max. {} °C", format!("{l:.0}"))).unwrap_or_default());
                        ui.end_row();
                    }
                });
            });
        }
        if findings.is_empty() {
            ui.weak(tr("No problems found"));
        }
        for f in &findings {
            let color = if f.severity == openecraft_core::issue::Severity::Info { ui.visuals().weak_text_color() } else { Color32::from_rgb(220, 120, 0) };
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(f.kind.title()).strong().color(color));
                ui.label(&f.message);
            });
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

/// Choice of the part for a rail, duct or busbar system: generic parts, then those of
/// the parts store and the project. Returns the part picked.
fn profile_part_combo(ui: &mut egui::Ui, salt: &str, choices: &[panel::ProfileChoice], current: &str) -> Option<String> {
    let mut picked = None;
    egui::ComboBox::from_id_salt(salt)
        .selected_text(current.trim_start_matches("GEN."))
        .width(150.0)
        .show_ui(ui, |ui| {
            for (generic, heading) in [(true, tr("Generic")), (false, tr("Parts store"))] {
                let list: Vec<&panel::ProfileChoice> = choices.iter().filter(|c| c.generic == generic).collect();
                if list.is_empty() {
                    continue;
                }
                ui.label(RichText::new(heading).weak());
                for c in list {
                    if ui.selectable_label(c.number == current, &c.number).on_hover_text(&c.description).clicked() {
                        picked = Some(c.number.clone());
                    }
                }
            }
        })
        .response
        .on_hover_text(tr("Part of the profile: generic types or parts from the parts store"));
    picked.filter(|p| p != current)
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
    /// Items marked as mounted in the mounting assistant (drawn green in 3D).
    pub done_items: HashSet<u64>,
    /// The door is (being) opened in the 3D view.
    pub door_open: bool,
    /// Current opening angle of the door (radians), animated towards `door_open`.
    door_angle: f32,
    /// Cable ducts coloured by their fill level.
    pub show_fill: bool,
    /// Estimated air temperature over the mounting plate.
    pub show_heat: bool,
    /// The 3D view shows the enclosures bayed to the current one as well.
    pub show_lineup: bool,
    /// The 3D view shows all enclosures at their places in the room.
    pub show_site: bool,
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
    /// NC export of an enclosure face.
    pub export: Option<(usize, Face, NcFormat)>,
    pattern: Option<PatternDialog>,
    /// Name of the 3D macro being saved from the selection.
    save_macro: Option<String>,
    /// Gap between devices lined up on a rail.
    rail_gap: f32,
    /// Parts chosen for new rails, ducts and busbars (kind, on the door).
    profile_parts: HashMap<(ItemKind, bool), String>,
    /// Diameter of new cable passages.
    passage_diameter: f32,
    /// Corners of the routing path being drawn (plate coordinates).
    path_points: Vec<P>,
    /// New routing paths: height above the plate, width and kind.
    path_height: f32,
    path_width: f32,
    path_kind: routing3d::PathKind,
    /// What new reservations keep the space for.
    reserve_mode: routing3d::ReserveMode,
    /// Selected routed wire, cable or tube (its key).
    pub selected_route: Option<String>,
    /// Selected routing path or reservation (its id).
    selected_object: Option<u64>,
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
            done_items: HashSet::new(),
            door_open: true,
            door_angle: DOOR_OPEN_ANGLE,
            show_fill: true,
            show_heat: false,
            show_lineup: true,
            show_site: false,
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
            profile_parts: HashMap::new(),
            passage_diameter: panel::PASSAGE_DIAMETER,
            path_points: Vec::new(),
            path_height: 50.0,
            path_width: 50.0,
            path_kind: routing3d::PathKind::Any,
            reserve_mode: routing3d::ReserveMode::Keepout,
            selected_route: None,
            selected_object: None,
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
/// Routing paths and reservations.
const PATH_COLOR: Color32 = Color32::from_rgb(220, 180, 40);
const RESERVE_COLOR: Color32 = Color32::from_rgb(220, 70, 60);

impl PanelUi {
    /// True while a gesture is in progress (no undo step yet).
    pub fn busy(&self) -> bool {
        self.start.is_some() || self.drag.is_some() || !self.path_points.is_empty()
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
        // Routing paths and reservations are drawn over the mounting plate.
        if matches!(t, Tool::RoutingPath | Tool::Reservation) {
            return self.face == Face::Plate;
        }
        if self.face.is_wall() {
            return !matches!(t, Tool::Busbar);
        }
        !matches!(
            (self.face, self.door_inside, t),
            (Face::Door, _, Tool::Rail | Tool::Busbar) | (Face::Door, false, Tool::Duct) | (Face::Door, true, Tool::Cutout)
        )
    }

    /// Item kind and part of the profile tool on the face shown: the part chosen for it,
    /// otherwise the default (the door gets the smaller door duct).
    fn profile_here(&self) -> Option<(ItemKind, String)> {
        self.profile_here_for(&self.tool)
    }

    fn profile_here_for(&self, tool: &Tool) -> Option<(ItemKind, String)> {
        let (kind, default) = match tool.profile()? {
            (ItemKind::Duct, _) if self.face == Face::Door => (ItemKind::Duct, panel::DOOR_DUCT_PART),
            x => x,
        };
        let chosen = self.profile_parts.get(&(kind, self.face == Face::Door));
        Some((kind, chosen.cloned().unwrap_or_else(|| default.to_string())))
    }

    /// Part for the profiles of this kind placed from now on (on the face shown).
    fn choose_profile_part(&mut self, kind: ItemKind, part: &str) {
        self.profile_parts.insert((kind, self.face == Face::Door), part.to_string());
    }

    fn set_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.start = None;
        self.path_points.clear();
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
        let shown = self.shown_3d(prj, w.enclosure);
        let pose = shown.iter().find(|s| s.0 == w.enclosure).map(|s| s.1).unwrap_or_default();
        let cam = self.cam.get_or_insert_with(|| default_cam(prj, &shown));
        cam.target = pose.apply([PLATE_MARGIN + c.x, e.height - PLATE_MARGIN - c.y, PLATE_Z]);
    }

    /// Selects a panel item and turns the 3D view to it (mounting assistant).
    pub fn focus_item(&mut self, prj: &Project, enclosure: usize, id: u64) {
        let Some(e) = prj.layout.enclosures.get(enclosure) else { return };
        let Some(it) = e.items.iter().find(|i| i.id == id) else { return };
        if self.enclosure != enclosure {
            self.cam = None;
        }
        self.enclosure = enclosure;
        self.face = it.face;
        self.select_item(id);
        if it.face == Face::Door {
            self.door_open = true;
        }
        let shown = self.shown_3d(prj, enclosure);
        let pose = shown.iter().find(|s| s.0 == enclosure).map(|s| s.1).unwrap_or_default();
        let cam = self.cam.get_or_insert_with(|| default_cam(prj, &shown));
        cam.target = pose.apply([PLATE_MARGIN + it.x + it.w / 2.0, e.height - PLATE_MARGIN - it.y - it.h / 2.0, PLATE_Z]);
    }

    /// Selects a placed device (e.g. coming from the schematic); a valve or its coil
    /// selects the valve terminal it is mounted in.
    pub fn show_device(&mut self, prj: &Project, tag: &str) -> bool {
        match panel::locate(prj, tag) {
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
        // Line-ups from left to right; bayed enclosures are indented.
        for (k, i) in panel::lineups(&prj.layout).into_iter().flat_map(|lu| lu.into_iter().enumerate()) {
            let e = &prj.layout.enclosures[i];
            ui.horizontal(|ui| {
                if k > 0 {
                    ui.add_space(14.0);
                }
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
            panel::unbay(prj, i);
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

    #[allow(clippy::too_many_lines)]
    pub fn properties(&mut self, ui: &mut egui::Ui, prj: &mut Project, an: &Analysis, db: &PartsDb) {
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
            let air = prj.layout.enclosures[ei].item(id).filter(|it| it.kind == ItemKind::Device && it.face == Face::Plate).and_then(|_| {
                let e = &prj.layout.enclosures[ei];
                panel::thermal::device_temperatures(prj, e, &panel::thermal::heat_map(prj, e)).into_iter().find(|t| t.id == id)
            });
            let auto_class = prj.layout.enclosures[ei].item(id).map(|it| panel::item_class(prj, &panel::device_kinds(prj), &PanelItem { class: String::new(), ..it.clone() })).unwrap_or_default();
            let custom = custom_classes(prj);
            let choices = match prj.layout.enclosures[ei].item(id).map(|it| it.kind) {
                Some(k) if k.is_profile() => panel::profile_choices(k, db.parts.iter().chain(&prj.parts)),
                _ => Vec::new(),
            };
            let mut new_part = None;
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
                if it.kind.is_profile() {
                    new_part = profile_part_combo(ui, "itempart", &choices, &it.part);
                } else {
                    ui.label(RichText::new(if it.part.is_empty() { "–" } else { &it.part }).monospace());
                }
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
                ui.label(tr("Collision class"));
                ui.horizontal(|ui| {
                    let auto = tr_fmt!("Automatic ({})", panel::class_title(&auto_class));
                    let shown = if it.class.is_empty() { auto.clone() } else { panel::class_title(&it.class) };
                    egui::ComboBox::from_id_salt("itemclass").selected_text(shown).show_ui(ui, |ui| {
                        ui.selectable_value(&mut it.class, String::new(), auto);
                        for (k, _) in panel::CLASSES {
                            ui.selectable_value(&mut it.class, k.to_string(), panel::class_title(k));
                        }
                        for k in &custom {
                            ui.selectable_value(&mut it.class, k.clone(), k.as_str());
                        }
                    });
                    ui.add(egui::TextEdit::singleline(&mut it.class).desired_width(70.0).hint_text(tr("own class")));
                })
                .response
                .on_hover_text(tr("The checks keep the distances between classes and let classes overlap as set in Layout checks"));
                ui.end_row();
                if let Some(t) = &air {
                    ui.label(tr("Air temperature"));
                    let text = match t.limit {
                        Some(l) => tr_fmt!("{} °C (max. {} °C)", format!("{:.0}", t.temperature), format!("{l:.0}")),
                        None => format!("{:.0} °C", t.temperature),
                    };
                    if t.limit.is_some_and(|l| t.temperature > l) {
                        ui.colored_label(Color32::from_rgb(220, 60, 60), text);
                    } else {
                        ui.label(text);
                    }
                    ui.end_row();
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
            if let Some(part) = new_part.filter(|_| !self.selection.is_empty()) {
                parts::ensure_in_project(prj, db, &part);
                panel::change_profile(prj, ei, id, &part);
            }
            if !self.selection.is_empty() {
                self.selection_tools(ui, prj, ei, &sel);
                self.passage_tools(ui, prj, ei, id);
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
        self.lineup_section(ui, prj, ei);
        self.routing_section(ui, prj, an, ei);
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
        checks_section(ui, prj, ei);
        let e = &prj.layout.enclosures[ei];
        let devices = e.items.iter().filter(|i| i.kind == ItemKind::Device).count();
        let wires: Vec<_> = an.wiring.iter().filter(|w| w.in_enclosure(ei)).collect();
        let total: f32 = wires.iter().map(|w| w.length).sum();
        ui.separator();
        ui.label(tr_fmt!("{} devices placed, {} wires routed ({} m)", devices, wires.len(), format!("{:.1}", total / 1000.0)));
        let problems = panel::problem_items(prj, e).len();
        let holes = openecraft_core::production::holes(e, &prj.parts).len();
        let cutouts = e.items.iter().filter(|i| i.kind == ItemKind::Cutout).count();
        ui.label(tr_fmt!("Mounting plate: {} holes, {} cutouts", holes, cutouts));
        ui.horizontal(|ui| {
            if ui.button(tr("⬇ NC data (DXF)")).on_hover_text(tr("Plate outline, holes per diameter, cutouts")).clicked() {
                self.export = Some((ei, Face::Plate, NcFormat::Dxf));
            }
            if ui.button(tr("⬇ NC data (DWG)")).on_hover_text(tr("Plate outline, holes per diameter, cutouts")).clicked() {
                self.export = Some((ei, Face::Plate, NcFormat::Dwg));
            }
            if ui.button(tr("⬇ Drilling list (CSV)")).clicked() {
                self.export = Some((ei, Face::Plate, NcFormat::Csv));
            }
        });
        let door_devices = e.items.iter().filter(|i| i.face == Face::Door && i.kind == ItemKind::Device).count();
        ui.label(tr_fmt!("Door: {} devices, {} cutouts", door_devices, openecraft_core::production::openings(e, &prj.parts, Face::Door).len()));
        ui.horizontal(|ui| {
            if ui.button(tr("⬇ Door NC data (DXF)")).clicked() {
                self.export = Some((ei, Face::Door, NcFormat::Dxf));
            }
            if ui.button(tr("⬇ Door NC data (DWG)")).clicked() {
                self.export = Some((ei, Face::Door, NcFormat::Dwg));
            }
            if ui.button(tr("⬇ Door cutouts (CSV)")).clicked() {
                self.export = Some((ei, Face::Door, NcFormat::Csv));
            }
        });
        if self.face.is_wall() {
            let face = self.face;
            let n = openecraft_core::production::holes_on(e, &prj.parts, face).len();
            ui.label(tr_fmt!("{}: {} holes, {} cutouts", face.title(), n, openecraft_core::production::openings(e, &prj.parts, face).len()));
            ui.horizontal(|ui| {
                if ui.button(tr("⬇ NC data of this face (DXF)")).clicked() {
                    self.export = Some((ei, face, NcFormat::Dxf));
                }
                if ui.button(tr("⬇ NC data of this face (DWG)")).clicked() {
                    self.export = Some((ei, face, NcFormat::Dwg));
                }
                if ui.button(tr("⬇ Drilling list of this face (CSV)")).clicked() {
                    self.export = Some((ei, face, NcFormat::Csv));
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

    /// Line-up of an enclosure: the neighbours it is bayed to and the cable passages
    /// the wires take into them.
    fn lineup_section(&mut self, ui: &mut egui::Ui, prj: &mut Project, ei: usize) {
        let (left, right) = panel::neighbours(&prj.layout, ei);
        let name = |prj: &Project, i: Option<usize>| i.map(|i| prj.layout.enclosures[i].name.clone()).unwrap_or_else(|| tr("(none)").to_string());
        let mut bay: Option<Option<usize>> = None;
        let mut site: Option<Option<SitePlace>> = None;
        let mut add = false;
        egui::CollapsingHeader::new(tr("Line-up and cable passages")).id_salt("lineup").default_open(left.is_some() || right.is_some()).show(ui, |ui| {
            egui::Grid::new("plineup").num_columns(2).spacing([10.0, 5.0]).show(ui, |ui| {
                ui.label(tr("Bayed to the right of"));
                egui::ComboBox::from_id_salt("bayedto")
                    .selected_text(name(prj, left))
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(left.is_none(), tr("(none)")).clicked() {
                            bay = Some(None);
                        }
                        for (i, o) in prj.layout.enclosures.iter().enumerate().filter(|(i, _)| *i != ei) {
                            if ui.selectable_label(left == Some(i), &o.name).clicked() {
                                bay = Some(Some(i));
                            }
                        }
                    })
                    .response
                    .on_hover_text(tr("The enclosure stands with its left side against the right side of the chosen one; wires run between them through cable passages"));
                ui.end_row();
                ui.label(tr("Right neighbour"));
                ui.label(name(prj, right));
                ui.end_row();
                ui.label(tr("Place in the room"));
                let first = panel::lineup_of(&prj.layout, ei).first().copied().unwrap_or(ei);
                let current = prj.layout.enclosures[first].site;
                let mut place = current.unwrap_or_else(|| {
                    let pose = panel::site_poses(prj)[first];
                    SitePlace { x: pose.at[0], z: pose.at[2], angle: pose.angle.to_degrees() }
                });
                ui.horizontal(|ui| {
                    let mut by_hand = current.is_some();
                    let mut changed = ui.checkbox(&mut by_hand, tr("by hand")).on_hover_text(tr("Otherwise from the plant layout, or next to the other line-ups")).changed();
                    ui.add_enabled_ui(by_hand, |ui| {
                        changed |= ui.add(egui::DragValue::new(&mut place.x).speed(10.0).prefix("x ").suffix(" mm")).changed();
                        changed |= ui.add(egui::DragValue::new(&mut place.z).speed(10.0).prefix("z ").suffix(" mm")).changed();
                        changed |= ui
                            .add(egui::DragValue::new(&mut place.angle).speed(1.0).range(-360.0..=360.0).suffix(" °"))
                            .on_hover_text(tr("Turn about the vertical, counter-clockwise seen from above (90 °: the fronts face to the right)"))
                            .changed();
                    });
                    if changed {
                        site = Some(by_hand.then_some(place));
                    }
                })
                .response
                .on_hover_text(tr("Rear left corner of the line-up on the floor, as in the 3D view with all enclosures"));
                ui.end_row();
            });
            let pos = panel::positions(&prj.layout);
            for (hop, other) in [(left, left), (right.map(|_| ei), right)] {
                let (Some(hop), Some(other)) = (hop, other) else { continue };
                let list = panel::passages(&prj.layout, &pos, hop);
                let cutouts = list.iter().filter(|x| x.cutouts != (None, None)).count();
                let other = &prj.layout.enclosures[other].name;
                if list.is_empty() {
                    ui.colored_label(Color32::from_rgb(220, 120, 0), tr_fmt!("No cable passage to {}: wires to it are not routed", other));
                } else if cutouts == 0 {
                    ui.label(tr_fmt!("Open to {}: wires cross beside the mounting plates", other));
                } else {
                    ui.label(tr_fmt!("{} cable passages to {}", cutouts, other));
                }
            }
            if let Some(r) = right {
                ui.horizontal(|ui| {
                    add = ui.button(tr_fmt!("➕ Cable passage to {}", prj.layout.enclosures[r].name)).on_hover_text(tr("Round cutouts facing each other in both side walls")).clicked();
                    ui.add(egui::DragValue::new(&mut self.passage_diameter).speed(1.0).range(10.0..=300.0).suffix(" mm")).on_hover_text(tr("Diameter"));
                });
            }
        });
        match bay {
            Some(Some(l)) => {
                panel::bay_right_of(prj, ei, l);
            }
            Some(None) => panel::unbay(prj, ei),
            None => {}
        }
        if let Some(place) = site {
            panel::set_site(prj, ei, place);
        }
        if add {
            let (y, z) = panel::next_passage_place(&prj.layout, ei);
            match panel::add_passage(prj, ei, y, z, self.passage_diameter).first() {
                Some(&id) if !prj.layout.enclosures[ei].build.right_open => {
                    self.show_face(Face::Right, false);
                    self.select_item(id);
                }
                Some(_) => {}
                None => self.status = tr("Both sides are open: the wires need no cutout").to_string(),
            }
        }
    }

    /// A cutout in a side wall towards a bayed neighbour: whether it is a cable
    /// passage, otherwise the facing cutout to add.
    fn passage_tools(&mut self, ui: &mut egui::Ui, prj: &mut Project, ei: usize, id: u64) {
        let Some(face) = prj.layout.enclosures[ei].item(id).filter(|i| i.kind == ItemKind::Cutout).map(|i| i.face) else { return };
        let (left, right) = panel::neighbours(&prj.layout, ei);
        let (hop, other) = match face {
            Face::Right => (right.map(|_| ei), right),
            Face::Left => (left, left),
            _ => return,
        };
        let (Some(hop), Some(other)) = (hop, other) else { return };
        let pos = panel::positions(&prj.layout);
        let used = panel::passages(&prj.layout, &pos, hop).iter().any(|x| x.cutouts.0 == Some(id) || x.cutouts.1 == Some(id));
        let name = prj.layout.enclosures[other].name.clone();
        ui.separator();
        if used {
            ui.label(tr_fmt!("Cable passage to {}", name));
        } else if ui
            .button(tr_fmt!("Add the facing cutout in {}", name))
            .on_hover_text(tr("A cutout of the same size at the same place in the neighbour's side wall: together they are a cable passage"))
            .clicked()
            && panel::add_facing_cutout(prj, ei, id).is_none()
        {
            self.status = tr_fmt!("The side of {} is open", name);
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
            if !w.legs.is_empty() {
                ui.label(tr("Enclosures"));
                ui.label(w.enclosure_names(&prj.layout));
                ui.end_row();
            }
            if let Some(ri) = an.routing.items.iter().position(|i| i.wire == Some(wi)) {
                let it = &an.routing.items[ri];
                ui.label(tr("Bundles"));
                ui.label(tr_fmt!("{} (class {})", an.routing.bundles_of(ri).len(), it.class.title()));
                ui.end_row();
                ui.label(tr("Cut length"));
                ui.label(format!("{:.0} mm", it.cut));
                ui.end_row();
            }
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
                format!("{:.0}", w.plate_length()),
                format!("{:.0}", prj.layout.wire_reserve)
            ))
            .weak(),
        );
        ui.separator();
        ui.label(RichText::new(tr("Connected devices")).strong());
        let mut select = None;
        for (ei, id) in [(w.enclosure, w.items.0), (w.last_enclosure(), w.items.1)] {
            if let Some(it) = prj.layout.enclosures.get(ei).and_then(|e| e.item(id)) {
                ui.horizontal(|ui| {
                    if ui.link(RichText::new(&it.tag).monospace()).on_hover_text(tr("Select device")).clicked() {
                        select = Some((ei, id));
                    }
                    if ui.small_button(tr("Show in schematic")).clicked() {
                        self.goto_schematic = Some(it.tag.clone());
                    }
                });
            }
        }
        if let Some((ei, id)) = select {
            self.enclosure = ei;
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
                if panel::lineup_of(&prj.layout, ei).len() > 1
                    && !self.show_site
                    && ui.checkbox(&mut self.show_lineup, tr("Line-up")).on_hover_text(tr("Show the enclosures bayed to this one as well")).changed()
                {
                    self.cam = None;
                }
                if prj.layout.enclosures.len() > 1
                    && ui.checkbox(&mut self.show_site, tr("All enclosures")).on_hover_text(tr("Show all enclosures side by side at their places in the room")).changed()
                {
                    self.cam = None;
                }
            }
            ui.checkbox(&mut self.show_wires, tr("Wires"));
            ui.checkbox(&mut self.show_fill, tr("Duct fill")).on_hover_text(tr("Colour the cable ducts by their fill level"));
            ui.checkbox(&mut self.show_heat, tr("Heat map")).on_hover_text(tr("Estimated air temperature in front of the mounting plate (power loss of the devices, climate)"));
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
                (Tool::RoutingPath, tr("Routing path"), tr("Path for cables, tubes or wires over the plate: click the corners, double-click or Enter to finish")),
                (Tool::Reservation, tr("Reservation"), tr("Space kept free of routes, or for one class only: click two corners")),
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
            if let Some((kind, part)) = self.profile_here() {
                let choices = panel::profile_choices(kind, db.parts.iter().chain(&prj.parts));
                if let Some(picked) = profile_part_combo(ui, "profilepart", &choices, &part) {
                    self.choose_profile_part(kind, &picked);
                }
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

    #[allow(clippy::too_many_lines)]
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
                if self.start.is_some() || !self.path_points.is_empty() {
                    self.start = None;
                    self.path_points.clear();
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
                            let (kind, part) = self.profile_here().unwrap_or((ItemKind::Duct, panel::DUCT_PART.into()));
                            parts::ensure_in_project(prj, db, &part);
                            if kind == ItemKind::Busbar {
                                parts::ensure_in_project(prj, db, openecraft_core::production::BUSBAR_HOLDER);
                            }
                            let item = PanelItem { face: self.face, ..panel::new_profile(prj, kind, &part, a, sn) };
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
            Tool::RoutingPath => {
                if let (true, Some(sn)) = (resp.clicked_by(PointerButton::Primary), snapped) {
                    if self.path_points.last().is_none_or(|l| !l.same(sn)) {
                        self.path_points.push(sn);
                    }
                }
                let enter = !ui.ctx().egui_wants_keyboard_input() && ui.input(|i| i.key_pressed(Key::Enter));
                if (resp.double_clicked() || enter) && self.path_points.len() >= 2 {
                    let id = self.add_routing_path(prj, ei);
                    self.selected_object = Some(id);
                }
            }
            Tool::Reservation => {
                if let (true, Some(sn)) = (resp.clicked_by(PointerButton::Primary), snapped) {
                    match self.start.take() {
                        None => self.start = Some(sn),
                        Some(a) if (a.x - sn.x).abs() > 0.1 && (a.y - sn.y).abs() > 0.1 => {
                            let id = self.add_reservation(prj, ei, a, sn);
                            self.selected_object = Some(id);
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
        let problems = panel::problem_items(prj, e);
        let mut d = EguiDraw { painter: &painter, origin, zoom: self.zoom, pal: &pal, force: None, mirror, textures: None };
        render::draw_enclosure(&mut d, e, self.face, self.door_inside, p(0.0, 0.0), 1.0, &self.selection, &problems, prj);
        let to_screen = |q: P| origin + vec2(mirror.map(|m| m - (q.x + fm)).unwrap_or(q.x + fm), q.y + fm) * self.zoom;
        if self.show_heat && self.face == Face::Plate {
            let map = panel::thermal::heat_map(prj, e);
            let (pw, ph) = e.plate_size();
            let span = (map.max - map.min).max(1.0);
            for r in 0..map.rows {
                for c in 0..map.cols {
                    let (x, y) = (c as f32 * map.cell, r as f32 * map.cell);
                    let cell = Rect::from_two_pos(to_screen(p(x, y)), to_screen(p((x + map.cell).min(pw), (y + map.cell).min(ph))));
                    painter.rect_filled(cell, 0.0, heat_color((map.temps[r * map.cols + c] - map.min) / span).gamma_multiply(0.35));
                }
            }
            let hot = to_screen(map.hottest);
            painter.circle_stroke(hot, 6.0, Stroke::new(2.0, heat_color(1.0)));
            let label = tr_fmt!("{} °C (max.)", format!("{:.0}", map.max));
            painter.text(hot + vec2(9.0, 0.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(12.0), heat_color(1.0));
            let range = tr_fmt!("Air temperature {} … {} °C", format!("{:.0}", map.min), format!("{:.0}", map.max));
            painter.text(rect.left_bottom() + vec2(8.0, -8.0), egui::Align2::LEFT_BOTTOM, range, egui::FontId::proportional(12.0), pal.line);
        }
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
        // Routing paths, reservations and the routed cables and tubes over the plate.
        if self.face == Face::Plate {
            self.draw_routing_2d(&painter, prj, an, ei, &to_screen);
        }
        // Wires on the face shown: the routes on the plate, or on the door from the door
        // devices to the hinge.
        let face = self.face;
        let selected_wire = self.wire_index(an).filter(|i| !an.wiring[*i].paths_in(ei, face).is_empty());
        // The wiring of the door is behind it: seen from the front only the selected wire.
        let show_wires = self.show_wires && !(face == Face::Door && !self.door_inside);
        if show_wires || selected_wire.is_some() {
            // Other wires are dimmed while one is selected.
            let normal = if selected_wire.is_some() { WIRE_COLOR.gamma_multiply(0.25) } else { WIRE_COLOR.gamma_multiply(0.7) };
            for (i, w) in an.wiring.iter().enumerate().filter(|(_, w)| w.in_enclosure(ei)) {
                if Some(i) != selected_wire && show_wires {
                    for path in w.paths_in(ei, face) {
                        let pts: Vec<Pos2> = path.iter().map(|q| to_screen(*q)).collect();
                        painter.add(Shape::line(pts, Stroke::new(1.0, normal)));
                    }
                }
            }
        }
        if let Some(i) = selected_wire {
            let w = &an.wiring[i];
            for path in w.paths_in(ei, face) {
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
                        openecraft_core::parts::Exit::Up => vec2(0.0, -1.0),
                        openecraft_core::parts::Exit::Down => vec2(0.0, 1.0),
                        openecraft_core::parts::Exit::Left => vec2(-1.0, 0.0),
                        openecraft_core::parts::Exit::Right => vec2(1.0, 0.0),
                        openecraft_core::parts::Exit::Front => vec2(0.0, 0.0),
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
        let mut g = EguiDraw { painter: &painter, origin, zoom: self.zoom, pal: &pal, force: Some(Ink::Ghost), mirror, textures: None };
        let m = p(fm, fm);
        match (&self.tool, snapped, plate) {
            (Tool::RoutingPath, Some(sn), _) => {
                let mut pts: Vec<Pos2> = self.path_points.iter().map(|q| to_screen(*q)).collect();
                pts.push(to_screen(sn));
                painter.add(Shape::line(pts, Stroke::new((self.path_width * self.zoom).max(2.0), PATH_COLOR.gamma_multiply(0.4))));
                if !self.path_points.is_empty() {
                    let len: f32 = self.path_points.windows(2).map(|s| s[0].dist(s[1])).sum::<f32>() + self.path_points.last().map(|l| l.dist(sn)).unwrap_or(0.0);
                    self.status = tr_fmt!("Routing path {} mm – double-click or Enter to finish", format!("{len:.0}"));
                }
            }
            (Tool::Reservation, Some(sn), _) => {
                if let Some(a) = self.start {
                    let r = Rect::from_two_pos(to_screen(a), to_screen(sn));
                    painter.rect_filled(r, 0.0, RESERVE_COLOR.gamma_multiply(0.25));
                }
            }
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
                    let (kind, part) = self.profile_here().unwrap_or((ItemKind::Duct, panel::DUCT_PART.into()));
                    let pd = prj.parts.iter().chain(&db.parts).chain(parts::catalog()).find(|x| x.number == part);
                    let (pw, _) = panel::part_profile(kind, pd);
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
                g.text(ghost.center() + m, tag, (w.min(h) * 0.22).clamp(3.0, 9.0), openecraft_core::symbols::Anchor::Center, render::VAnchor::Middle, Ink::Ghost);
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
}

/// Items behind the door (cable ducts) are hidden in its front view: they cannot be
/// picked there.
fn hidden_behind_door(inside: bool, it: &PanelItem) -> bool {
    it.face == Face::Door && !inside && it.kind == ItemKind::Duct
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

/// Blue (cool) over yellow to red (hot) for 0 … 1.
fn heat_color(f: f32) -> Color32 {
    let f = f.clamp(0.0, 1.0);
    let lerp = |a: u8, b: u8, t: f32| (a as f32 + (b as f32 - a as f32) * t) as u8;
    if f < 0.5 {
        let t = f * 2.0;
        Color32::from_rgb(lerp(40, 240, t), lerp(110, 210, t), lerp(230, 40, t))
    } else {
        let t = (f - 0.5) * 2.0;
        Color32::from_rgb(lerp(240, 220, t), lerp(210, 40, t), lerp(40, 30, t))
    }
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


#[cfg(test)]
mod tests;
