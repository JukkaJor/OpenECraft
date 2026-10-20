//! Parts management in the application: re-exports the shared part model of the
//! `openecraft-parts` library and adds everything that needs a project (project
//! parts, device functions, function template usage).

use crate::model::Project;
use crate::symbols::{self, Func};
pub use openecraft_parts::*;
use std::collections::BTreeMap;

/// Function kind represented by a library symbol.
pub fn kind_of_symbol(key: &str) -> Option<FnKind> {
    Some(match key {
        "coil" | "coil_on" | "coil_off" => FnKind::Coil,
        "valve" => FnKind::Valve,
        "no" => FnKind::No,
        "nc" | "ol_nc" => FnKind::Nc,
        "co" => FnKind::Changeover,
        "main3" => FnKind::MainContacts3,
        "main1" => FnKind::MainContact,
        "pb_no" => FnKind::PushbuttonNo,
        "pb_nc" | "estop" => FnKind::PushbuttonNc,
        "sel" => FnKind::SwitchNo,
        "prox" | "limit" => FnKind::Sensor,
        "fuse" => FnKind::Fuse,
        "cb1" => FnKind::Breaker1,
        "cb3" => FnKind::Breaker3,
        "mps3" => FnKind::MotorProtection,
        "ol3" => FnKind::Overload,
        "motor3" => FnKind::Motor3,
        "motor1" => FnKind::Motor1,
        "lamp" | "horn" => FnKind::Signal,
        "psu" => FnKind::PowerSupply,
        "trafo" => FnKind::Transformer,
        "term" | "term_pe" => FnKind::Terminal,
        "plc_in" => FnKind::PlcInput,
        "plc_out" => FnKind::PlcOutput,
        "res" | "cap" | "diode" => FnKind::Generic,
        _ => return None,
    })
}

// ---------------------------------------------------------------------- project

/// Copies a part from the database into the project parts (if not there yet).
pub fn ensure_in_project(prj: &mut Project, db: &PartsDb, number: &str) {
    if prj.parts.iter().any(|p| p.number == number) {
        return;
    }
    if let Some(p) = db.get(number) {
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
    let mut m: BTreeMap<FnKind, (usize, usize)> = BTreeMap::new();
    for (k, _, _) in placed_functions(prj, tag) {
        m.entry(k).or_default().0 += 1;
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
    for (pi, si) in placed {
        let s = &mut prj.pages[pi].symbols[si];
        let Some(kind) = kind_of_symbol(&s.sym) else { continue };
        let n_pins = symbols::get(&s.sym).map(|d| d.pins.len()).unwrap_or(0);
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
        .find(|s| symbols::get(&s.sym).map(|d| matches!(d.func, Func::Main { .. })).unwrap_or(false))
        .and_then(|s| kind_of_symbol(&s.sym))
}

#[cfg(test)]
mod tests;
