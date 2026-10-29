//! 19-inch racks (IEC 60297): enclosures of the system [`RACK_SYSTEM`] with a pair of
//! 19-inch mounting rails at the front and the rear instead of a mounting plate. Their
//! rack units (1 U = 44.45 mm) are numbered from the bottom; a 19-inch part (as wide
//! as a 19-inch panel, technical data `Rack units`) placed on the mounting level
//! (`Face::Plate`) snaps onto whole units, its front panel on the front rails.
//!
//! Plate coordinates as on the mounting plate: x to the right and y downwards from
//! [`super::PLATE_MARGIN`] inside the top left corner of the front.

use super::{item_name, part_by_number, place_device_on, CheckKind, Enclosure, Face, Fastening, ItemKind, PanelItem, PLATE_MARGIN, PLATE_Z};
use crate::issue::Severity;
use crate::model::Project;
use crate::parts::vec3::V3;
pub use crate::parts::mechanics::{rack_rail_number, RACK_FRONT, RACK_OPENING, RACK_SYSTEM, RACK_UNIT, RACK_WIDTH};
use openecraft_i18n::tr_fmt;

/// Width of a mounting rail beside the opening, mm.
const RAIL_WIDTH: f32 = 15.0;
/// Depth of a mounting rail profile, mm.
const RAIL_DEPTH: f32 = 20.0;

/// The enclosure is a 19-inch rack.
pub fn is_rack(e: &Enclosure) -> bool {
    e.build.system == RACK_SYSTEM && !e.is_room()
}

/// Rack units of a rack (0 for other enclosures).
pub fn rack_units(e: &Enclosure) -> u32 {
    if is_rack(e) {
        crate::parts::mechanics::rack_units(e.height)
    } else {
        0
    }
}

/// Plate y of the bottom of unit 1: the units are centred in the height of the rack.
fn bottom(e: &Enclosure) -> f32 {
    let free = e.height - rack_units(e) as f32 * RACK_UNIT;
    e.height - free / 2.0 - PLATE_MARGIN
}

/// Plate x of the left edge of a 19-inch panel.
pub fn panel_left(e: &Enclosure) -> f32 {
    (e.width - RACK_WIDTH) / 2.0 - PLATE_MARGIN
}

/// Plate y of the top of `n` units starting at unit `first` (from the bottom, 1-based).
pub fn unit_top(e: &Enclosure, first: u32, n: u32) -> f32 {
    bottom(e) - (first + n - 1) as f32 * RACK_UNIT
}

/// Height above the mounting level (`PanelItem::z`) of a part `d` deep whose front
/// panel is on the front rails.
pub fn front_z(e: &Enclosure, d: f32) -> f32 {
    (e.depth - RACK_FRONT - PLATE_Z - d).max(0.0)
}

/// An item is a 19-inch part: a device or accessory as wide as a 19-inch panel.
pub fn is_rack_part(it: &PanelItem) -> bool {
    matches!(it.kind, ItemKind::Device | ItemKind::Accessory) && (it.w - RACK_WIDTH).abs() < 1.0
}

/// Rack units an item takes (from its height).
pub fn units_of(it: &PanelItem) -> u32 {
    ((it.h / RACK_UNIT).round() as u32).max(1)
}

/// The unit nearest to the bottom edge of an item (from the bottom, 1-based; 0 or
/// less below the rack units).
fn unit_at(e: &Enclosure, it: &PanelItem) -> i64 {
    ((bottom(e) - it.y - it.h) / RACK_UNIT).round() as i64 + 1
}

/// The units a 19-inch part of a rack takes: its lowest unit and how many.
pub fn slot(e: &Enclosure, it: &PanelItem) -> Option<(u32, u32)> {
    if !is_rack(e) || it.face != Face::Plate || !is_rack_part(it) {
        return None;
    }
    Some((unit_at(e, it).max(0) as u32, units_of(it)))
}

/// Puts a 19-inch part placed on the mounting level of a rack onto the nearest whole
/// units inside the rack, its panel centred and its front on the front rails, held by
/// the rails (no holes in a plate). False (nothing changed) for other items and
/// enclosures.
pub fn snap_into_rack(e: &Enclosure, it: &mut PanelItem) -> bool {
    if !is_rack(e) || it.face != Face::Plate || !is_rack_part(it) {
        return false;
    }
    let (n, total) = (units_of(it), rack_units(e));
    let first = unit_at(e, it).clamp(1, (total as i64 - n as i64 + 1).max(1)) as u32;
    it.h = n as f32 * RACK_UNIT;
    it.y = unit_top(e, first, n);
    it.x = panel_left(e);
    it.z = front_z(e, it.d);
    if it.fastening == Fastening::Auto {
        it.fastening = Fastening::Clip;
    }
    true
}

/// The 19-inch parts of a rack with their units (lowest unit, units, id), from the
/// top.
pub fn occupied(e: &Enclosure) -> Vec<(u32, u32, u64)> {
    let mut v: Vec<(u32, u32, u64)> = e.items.iter().filter_map(|it| slot(e, it).map(|(first, n)| (first, n, it.id))).collect();
    v.sort_by_key(|o| std::cmp::Reverse(o.0));
    v
}

/// The highest `n` free units of a rack (their lowest unit), leaving out item `skip`.
pub fn free_units(e: &Enclosure, n: u32, skip: u64) -> Option<u32> {
    let n = n.max(1);
    let taken: Vec<(u32, u32, u64)> = occupied(e).into_iter().filter(|o| o.2 != skip).collect();
    let highest = rack_units(e).checked_sub(n)? + 1;
    (1..=highest).rev().find(|&first| taken.iter().all(|&(f, m, _)| first + n <= f || f + m <= first))
}

/// Mounts a device (by its tag) or a 19-inch part (by its number, without a device
/// tag) in a rack at unit `unit` (its lowest unit, from the bottom); without a unit
/// in the highest free units. Refused for parts that are no 19-inch parts and for
/// units outside the rack or taken by another part.
pub fn mount_in_rack(prj: &mut Project, rack: usize, what: &str, unit: Option<u32>) -> Result<u64, String> {
    let e = prj.layout.enclosures.get(rack).ok_or_else(|| tr_fmt!("No enclosure {}", rack))?;
    if !is_rack(e) {
        return Err(tr_fmt!("{} is no 19-inch rack", e.name));
    }
    let id = if prj.devices.contains_key(what) {
        place_device_on(prj, rack, what, crate::geom::p(0.0, 0.0), Face::Plate).ok_or_else(|| tr_fmt!("No enclosure {}", rack))?
    } else {
        if part_by_number(prj, what).is_none() {
            return Err(tr_fmt!("Unknown device or part: {}", what));
        }
        let item = super::new_accessory(prj, what, Face::Plate, crate::geom::p(0.0, 0.0));
        let id = item.id;
        prj.layout.enclosures[rack].items.push(item);
        id
    };
    let e = &prj.layout.enclosures[rack];
    let it = e.item(id).cloned().unwrap_or_default();
    let n = units_of(&it);
    let problem = if !is_rack_part(&it) {
        Some(tr_fmt!("{} is no 19-inch part ({} mm wide)", what, format!("{RACK_WIDTH}")))
    } else {
        match unit.or_else(|| free_units(e, n, id)) {
            None => Some(tr_fmt!("{} has no {} free rack units", e.name, n)),
            Some(first) if first < 1 || first + n - 1 > rack_units(e) => Some(tr_fmt!("Units {} to {} are outside the rack ({} U)", first, first + n - 1, rack_units(e))),
            Some(first) => match occupied(e).into_iter().find(|&(f, m, other)| other != id && first < f + m && f < first + n) {
                Some((_, _, other)) => Some(tr_fmt!("Units {} to {} are taken by {}", first, first + n - 1, item_name(e, other))),
                None => {
                    let top = unit_top(e, first, n);
                    let e = &mut prj.layout.enclosures[rack];
                    if let Some(it) = e.items.iter_mut().find(|i| i.id == id) {
                        it.y = top;
                        it.fastening = Fastening::Auto;
                    }
                    let mut it = e.item(id).cloned().unwrap_or_default();
                    snap_into_rack(e, &mut it);
                    if let Some(old) = e.items.iter_mut().find(|i| i.id == id) {
                        *old = it;
                    }
                    None
                }
            },
        }
    };
    match problem {
        Some(msg) => {
            prj.layout.enclosures[rack].items.retain(|i| i.id != id);
            Err(msg)
        }
        None => Ok(id),
    }
}

/// The four 19-inch mounting rails of a rack (enclosure coordinates, y down): beside
/// the opening, at the front and towards the rear, as high as the rack units.
pub fn rail_boxes(e: &Enclosure) -> Vec<(V3, V3)> {
    let top = unit_top(e, 1, rack_units(e)) + PLATE_MARGIN;
    let bot = bottom(e) + PLATE_MARGIN;
    let (l, r) = ((e.width - RACK_OPENING) / 2.0, (e.width + RACK_OPENING) / 2.0);
    let front = e.depth - RACK_FRONT;
    let rear = (PLATE_Z + RACK_FRONT).min(front - RAIL_DEPTH);
    let mut v = Vec::new();
    for z in [front, rear] {
        for x in [l - RAIL_WIDTH, r] {
            v.push(([x, top, z - RAIL_DEPTH], [x + RAIL_WIDTH, bot, z]));
        }
    }
    v
}

/// Checks of a rack: parts on its mounting level that are no 19-inch parts (they need
/// a shelf or a mounting plate) and 19-inch parts outside its units.
pub(super) fn check(e: &Enclosure, push: &mut impl FnMut(CheckKind, Severity, Vec<u64>, String)) {
    if !is_rack(e) {
        return;
    }
    let total = rack_units(e);
    for it in e.items.iter().filter(|i| i.face == Face::Plate && matches!(i.kind, ItemKind::Device | ItemKind::Accessory)) {
        match slot(e, it) {
            Some((first, n)) if first < 1 || first + n - 1 > total => {
                push(CheckKind::Outside, Severity::Warning, vec![it.id], tr_fmt!("{}: {} is outside the rack units", e.name, item_name(e, it.id)));
            }
            Some(_) => {}
            None => push(CheckKind::Mounting, Severity::Info, vec![it.id], tr_fmt!("{}: {} is no 19-inch part, mount it on a shelf or a mounting plate", e.name, item_name(e, it.id))),
        }
    }
}

#[cfg(test)]
mod tests;
