//! Parts management in the application: re-exports the shared part model of the
//! `openecraft-parts` library and adds everything that needs a project (project
//! parts, device functions, function template usage).

use crate::model::Project;
use crate::symbols::{self, Func};
pub use openecraft_parts::*;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// The built-in catalog, built once (reports look up wiring material in it).
pub fn catalog() -> &'static [Part] {
    static CATALOG: OnceLock<Vec<Part>> = OnceLock::new();
    CATALOG.get_or_init(default_catalog)
}

/// Function kind represented by a library symbol.
pub fn kind_of_symbol(key: &str) -> Option<FnKind> {
    Some(match key {
        "coil" | "coil_on" | "coil_off" | "nfpa_coil" | "nfpa_tr_coil" => FnKind::Coil,
        "valve" | "nfpa_sol" => FnKind::Valve,
        "no" | "nfpa_no" => FnKind::No,
        "nc" | "ol_nc" | "nfpa_nc" | "nfpa_ol_nc" => FnKind::Nc,
        "co" => FnKind::Changeover,
        "no_on" | "nfpa_notc" => FnKind::NoOnDelay,
        "nc_on" | "nfpa_ncto" => FnKind::NcOnDelay,
        "no_off" => FnKind::NoOffDelay,
        "nc_off" => FnKind::NcOffDelay,
        "main3" | "nfpa_main3" => FnKind::MainContacts3,
        "main4" => FnKind::MainContacts4,
        "main1" => FnKind::MainContact,
        "pb_no" | "nfpa_pb_no" => FnKind::PushbuttonNo,
        "pb_nc" | "estop" | "nfpa_pb_nc" => FnKind::PushbuttonNc,
        "sel" | "key_no" | "nfpa_ss" => FnKind::SwitchNo,
        "sel_nc" => FnKind::SwitchNc,
        "disc3" | "nfpa_disc3" => FnKind::Disconnector3,
        "prox" | "limit" | "pressure_no" | "level_no" | "nfpa_ls_no" => FnKind::Sensor,
        "limit_nc" | "temp_nc" | "nfpa_ls_nc" => FnKind::SensorNc,
        "transmitter" => FnKind::Transmitter,
        "ptc" => FnKind::Thermistor,
        "fuse" | "nfpa_fuse" => FnKind::Fuse,
        "fuse3" => FnKind::Fuse3,
        "cb1" => FnKind::Breaker1,
        "cb2" => FnKind::Breaker2,
        "cb3" | "nfpa_cb3" => FnKind::Breaker3,
        "cb4" => FnKind::Breaker4,
        "rcd4" => FnKind::ResidualCurrent,
        "spd" => FnKind::SurgeProtection,
        "mps3" => FnKind::MotorProtection,
        "ol3" | "nfpa_ol" => FnKind::Overload,
        "motor3" | "nfpa_motor" => FnKind::Motor3,
        "motor1" => FnKind::Motor1,
        "motor_dc" => FnKind::MotorDc,
        "brake" => FnKind::Brake,
        "vfd" => FnKind::FrequencyConverter,
        "softstarter" => FnKind::SoftStarter,
        "heater" => FnKind::Heater,
        "socket" => FnKind::SocketOutlet,
        "lamp" | "horn" | "nfpa_lamp" => FnKind::Signal,
        "ammeter" | "voltmeter" => FnKind::Meter,
        "psu" => FnKind::PowerSupply,
        "trafo" => FnKind::Transformer,
        "ct" => FnKind::CurrentTransformer,
        "battery" => FnKind::Battery,
        "term" | "term_pe" | "term_n" | "term_disc" | "term_fuse" => FnKind::Terminal,
        "plug_pair" | "plug_m" | "plug_f" => FnKind::PlugPin,
        "plc_in" => FnKind::PlcInput,
        "plc_out" => FnKind::PlcOutput,
        "plc_ai" => FnKind::PlcAnalogInput,
        "plc_ao" => FnKind::PlcAnalogOutput,
        "fl_valve42" | "fl_valve52" => FnKind::DirectionalValve,
        "fl_cylinder" | "fl_cylinder_sa" => FnKind::Cylinder,
        "fl_pump" | "pid_pump" => FnKind::Pump,
        "fl_filter" | "fl_gauge" | "fl_tank" | "fl_source" | "fl_silencer" | "fl_throttle" | "pid_vessel" | "pid_valve"
        | "pid_control_valve" | "pid_check_valve" | "pid_heat_exchanger" => FnKind::FluidComponent,
        "res" | "cap" | "diode" | "led" | "inductor" | "varistor" | "potentiometer" | "cap_pol" | "zener" | "photodiode" | "npn" | "pnp" | "nmos" | "thyristor" | "triac" | "ntc" | "ldr"
        | "optocoupler" | "bridge_rect" => FnKind::Generic,
        _ => return symbols::iec_kind(key),
    })
}

// ---------------------------------------------------------------------- project

/// Copies a part from the database (or the built-in catalog) into the project parts
/// (if not there yet).
pub fn ensure_in_project(prj: &mut Project, db: &PartsDb, number: &str) {
    if prj.parts.iter().any(|p| p.number == number) {
        return;
    }
    if let Some(p) = db.get(number).or_else(|| catalog().iter().find(|p| p.number == number)) {
        prj.parts.push(p.clone());
        // Accessories come along so the BOM can resolve them later.
        for acc in p.accessories.clone() {
            if !prj.parts.iter().any(|x| x.number == acc) {
                if let Some(a) = db.get(&acc) {
                    prj.parts.push(a.clone());
                }
            }
        }
    }
}

/// Adds `count` units of a part to a device; the part is copied from the database
/// into the project parts.
pub fn add_to_device(prj: &mut Project, db: &PartsDb, tag: &str, number: &str, count: u32) {
    ensure_in_project(prj, db, number);
    let dev = prj.devices.entry(tag.to_string()).or_default();
    match dev.parts.iter_mut().find(|r| r.number == number) {
        Some(r) => r.count += count,
        None => dev.parts.push(PartRef { number: number.into(), count }),
    }
}

/// Resolves a part: project parts first, then the database.
pub fn lookup<'a>(prj: &'a Project, db: Option<&'a PartsDb>, number: &str) -> Option<&'a Part> {
    prj.parts.iter().find(|p| p.number == number).or_else(|| db.and_then(|d| d.get(number)))
}

/// Updates all project parts from the database. Returns the number of updated parts.
pub fn sync_project_from_db(prj: &mut Project, db: &PartsDb) -> usize {
    let mut n = 0;
    // Make sure every referenced part is present.
    let refs: Vec<String> =
        prj.devices.values().flat_map(|d| d.parts.iter().map(|r| r.number.clone())).collect();
    for r in refs {
        ensure_in_project(prj, db, &r);
    }
    for p in prj.parts.iter_mut() {
        if let Some(src) = db.get(&p.number) {
            if p != src {
                *p = src.clone();
                n += 1;
            }
        }
    }
    n
}

/// Adds project parts that are unknown in the database. Returns the number added.
pub fn transfer_project_to_db(prj: &Project, db: &mut PartsDb) -> usize {
    let mut n = 0;
    for p in &prj.parts {
        if db.get(&p.number).is_none() {
            db.parts.push(p.clone());
            n += 1;
        }
    }
    n
}

/// Removes project parts that no device uses any more.
pub fn purge_unused(prj: &mut Project) -> usize {
    let mut used: Vec<String> = prj.devices.values().flat_map(|d| d.parts.iter().map(|r| r.number.clone())).collect();
    let acc: Vec<String> = prj
        .parts
        .iter()
        .filter(|p| used.contains(&p.number))
        .flat_map(|p| p.accessories.clone())
        .collect();
    used.extend(acc);
    let before = prj.parts.len();
    prj.parts.retain(|p| used.contains(&p.number));
    before - prj.parts.len()
}

// ---------------------------------------------------------------------- devices

/// Functions placed for a device: (kind, connection designations, symbol id).
pub fn placed_functions(prj: &Project, tag: &str) -> Vec<(FnKind, Vec<String>, u64)> {
    let mut v = Vec::new();
    for pg in &prj.pages {
        for s in pg.symbols.iter().filter(|s| s.tag == tag) {
            if let Some(k) = kind_of_symbol(&s.sym) {
                v.push((k, s.pins.clone(), s.id));
            }
        }
    }
    v
}

/// Functions available from the parts of a device (templates × count).
pub fn available_functions(prj: &Project, db: Option<&PartsDb>, tag: &str) -> Vec<FunctionTemplate> {
    let Some(dev) = prj.devices.get(tag) else { return Vec::new() };
    let mut v = Vec::new();
    for r in &dev.parts {
        if let Some(p) = lookup(prj, db, &r.number) {
            for _ in 0..r.count.max(1) {
                v.extend(p.functions.iter().cloned());
            }
        }
    }
    v
}

/// Template functions not used by any placed function (shown as reserve contacts).
pub fn reserve_functions(prj: &Project, db: Option<&PartsDb>, tag: &str) -> Vec<FunctionTemplate> {
    let mut avail = available_functions(prj, db, tag);
    let placed = placed_functions(prj, tag);
    // First match by kind and designations, then by kind only.
    let mut rest = Vec::new();
    for (k, pins, _) in &placed {
        if let Some(i) = avail.iter().position(|f| f.kind == *k && &f.pin_list() == pins) {
            avail.remove(i);
        } else {
            rest.push(*k);
        }
    }
    for k in rest {
        if let Some(i) = avail.iter().position(|f| f.kind == k) {
            avail.remove(i);
        }
    }
    avail
}

/// Per function kind: (placed, available) for devices that have parts assigned.
pub fn usage(prj: &Project, db: Option<&PartsDb>, tag: &str) -> BTreeMap<FnKind, (usize, usize)> {
    usage_with(prj, db, tag, &placed_functions(prj, tag))
}

/// The kinds of the placed functions of every device (one pass over the project).
pub fn placed_kinds(prj: &Project) -> std::collections::HashMap<&str, Vec<FnKind>> {
    let mut m: std::collections::HashMap<&str, Vec<FnKind>> = std::collections::HashMap::new();
    for s in prj.pages.iter().flat_map(|pg| pg.symbols.iter()) {
        if let Some(k) = kind_of_symbol(&s.sym) {
            m.entry(s.tag.as_str()).or_default().push(k);
        }
    }
    m
}

/// [`usage`] with the kinds of the placed functions given (see [`placed_kinds`]).
pub fn usage_of(prj: &Project, db: Option<&PartsDb>, tag: &str, placed: &[FnKind]) -> BTreeMap<FnKind, (usize, usize)> {
    let mut m: BTreeMap<FnKind, (usize, usize)> = BTreeMap::new();
    for k in placed {
        m.entry(*k).or_default().0 += 1;
    }
    for f in available_functions(prj, db, tag) {
        m.entry(f.kind).or_default().1 += 1;
    }
    m
}

fn usage_with(prj: &Project, db: Option<&PartsDb>, tag: &str, placed: &[(FnKind, Vec<String>, u64)]) -> BTreeMap<FnKind, (usize, usize)> {
    let mut m: BTreeMap<FnKind, (usize, usize)> = BTreeMap::new();
    for (k, _, _) in placed {
        m.entry(*k).or_default().0 += 1;
    }
    for f in available_functions(prj, db, tag) {
        m.entry(f.kind).or_default().1 += 1;
    }
    m
}

/// Assigns connection designations from the function templates to the placed
/// functions of a device (in page / position order). Returns how many changed.
pub fn apply_designations(prj: &mut Project, db: Option<&PartsDb>, tag: &str) -> usize {
    let mut avail = available_functions(prj, db, tag);
    let mut placed: Vec<(usize, usize)> = Vec::new();
    for (pi, pg) in prj.pages.iter().enumerate() {
        for (si, s) in pg.symbols.iter().enumerate() {
            if s.tag == tag {
                placed.push((pi, si));
            }
        }
    }
    placed.sort_by(|a, b| {
        let (sa, sb) = (&prj.pages[a.0].symbols[a.1], &prj.pages[b.0].symbols[b.1]);
        a.0.cmp(&b.0).then(sa.pos.x.total_cmp(&sb.pos.x)).then(sa.pos.y.total_cmp(&sb.pos.y))
    });
    let mut changed = 0;
    let symbols = prj.symbol_table();
    for (pi, si) in placed {
        let s = &mut prj.pages[pi].symbols[si];
        let Some(kind) = kind_of_symbol(&s.sym) else { continue };
        let n_pins = symbols.get(&s.sym).map(|d| d.pins.len()).unwrap_or(0);
        if let Some(i) = avail.iter().position(|f| f.kind == kind && f.pin_list().len() == n_pins) {
            let pins = avail.remove(i).pin_list();
            if s.pins != pins {
                s.pins = pins;
                changed += 1;
            }
        }
    }
    changed
}

/// Device selection: does the part fit a device with these placed functions?
/// The part must provide the main function (or, without one, any placed function).
pub fn fits_device(part: &Part, placed: &[(FnKind, Vec<String>, u64)], main: Option<FnKind>) -> bool {
    if part.functions.is_empty() {
        return false;
    }
    match main {
        Some(m) => part.functions.iter().any(|f| f.kind == m),
        None => placed.iter().any(|(k, _, _)| part.functions.iter().any(|f| f.kind == *k)),
    }
}

/// Main function kind of a device, if one is placed.
pub fn main_kind(prj: &Project, tag: &str) -> Option<FnKind> {
    prj.pages
        .iter()
        .flat_map(|pg| pg.symbols.iter())
        .filter(|s| s.tag == tag)
        .find(|s| prj.symbol(&s.sym).map(|d| matches!(d.func, Func::Main { .. })).unwrap_or(false))
        .and_then(|s| kind_of_symbol(&s.sym))
}

#[cfg(test)]
mod tests;
