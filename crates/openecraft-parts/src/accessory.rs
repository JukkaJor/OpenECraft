//! Accessories: parts that only work together with the parts they fit (auxiliary and
//! signal contacts, shunt trips, contact blocks). A main part lists its accessories
//! ([`Part::accessories`]), an accessory the parts it fits ([`Part::fits`]); either
//! side makes a pair, so one accessory can serve many main parts and the data of a
//! manufacturer can add an accessory without changing its main parts.

use crate::{natural_cmp, Part};
use serde::{Deserialize, Serialize};

/// Where an accessory sits on the part it is attached to; also read in lower case.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccessoryMount {
    /// In front if it is not wider than the main part (auxiliary contact blocks),
    /// otherwise not drawn.
    #[default]
    #[serde(alias = "auto")]
    Auto,
    /// Snapped onto the front of the main part.
    #[serde(alias = "front")]
    Front,
    /// Next to the main part on the rail, on its left.
    #[serde(alias = "left")]
    Left,
    /// Next to the main part on the rail, on its right.
    #[serde(alias = "right")]
    Right,
    /// Not mounted on the main part (keys, markers, covers in the box).
    #[serde(alias = "loose")]
    Loose,
}

impl AccessoryMount {
    pub const ALL: [AccessoryMount; 5] = [AccessoryMount::Auto, AccessoryMount::Front, AccessoryMount::Left, AccessoryMount::Right, AccessoryMount::Loose];

    pub fn title(self) -> &'static str {
        match self {
            AccessoryMount::Auto => "Automatic",
            AccessoryMount::Front => "On the front",
            AccessoryMount::Left => "On the left side",
            AccessoryMount::Right => "On the right side",
            AccessoryMount::Loose => "Not mounted",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            AccessoryMount::Auto => "auto",
            AccessoryMount::Front => "front",
            AccessoryMount::Left => "left",
            AccessoryMount::Right => "right",
            AccessoryMount::Loose => "loose",
        }
    }

    pub fn from_key(key: &str) -> Option<AccessoryMount> {
        let key = key.trim();
        if key.is_empty() {
            return Some(AccessoryMount::Auto);
        }
        AccessoryMount::ALL.into_iter().find(|m| m.key().eq_ignore_ascii_case(key) || format!("{m:?}").eq_ignore_ascii_case(key))
    }

    pub fn is_auto(&self) -> bool {
        *self == AccessoryMount::Auto
    }

    /// Mounted beside the main part, taking room on the rail.
    pub fn is_side(self) -> bool {
        matches!(self, AccessoryMount::Left | AccessoryMount::Right)
    }
}

/// True if `accessory` fits `main`: the main part lists it or it lists the main part.
pub fn fits(accessory: &Part, main: &Part) -> bool {
    accessory.number != main.number && (main.accessories.iter().any(|a| a == &accessory.number) || accessory.fits.iter().any(|m| m == &main.number))
}

/// The accessories of `main` among `parts`: those it lists, in its order, then those
/// that list it, by number.
pub fn accessories_of<'a>(parts: impl IntoIterator<Item = &'a Part>, main: &Part) -> Vec<&'a Part> {
    let mut listed: Vec<&Part> = Vec::new();
    let mut others: Vec<&Part> = Vec::new();
    for p in parts {
        if listed.iter().chain(&others).any(|q| q.number == p.number) || !fits(p, main) {
            continue;
        }
        if main.accessories.contains(&p.number) {
            listed.push(p);
        } else {
            others.push(p);
        }
    }
    listed.sort_by_key(|p| main.accessories.iter().position(|a| a == &p.number));
    others.sort_by(|a, b| natural_cmp(&a.number, &b.number));
    listed.extend(others);
    listed
}

/// The parts among `parts` that `accessory` fits, by number.
pub fn mains_of<'a>(parts: impl IntoIterator<Item = &'a Part>, accessory: &Part) -> Vec<&'a Part> {
    let mut v: Vec<&Part> = Vec::new();
    for p in parts {
        if fits(accessory, p) && !v.iter().any(|q| q.number == p.number) {
            v.push(p);
        }
    }
    v.sort_by(|a, b| natural_cmp(&a.number, &b.number));
    v
}

/// Part numbers of the accessories of `main` and of the parts `accessory` fits,
/// including numbers not found among `parts` (missing parts of a catalog).
pub fn related_numbers(parts: &[Part], part: &Part) -> (Vec<String>, Vec<String>) {
    let mut accessories: Vec<String> = part.accessories.clone();
    accessories.extend(parts.iter().filter(|p| p.fits.contains(&part.number)).map(|p| p.number.clone()));
    let mut mains: Vec<String> = part.fits.clone();
    mains.extend(parts.iter().filter(|p| p.accessories.contains(&part.number)).map(|p| p.number.clone()));
    for list in [&mut accessories, &mut mains] {
        list.retain(|n| n != &part.number);
        list.sort_by(|a, b| natural_cmp(a, b));
        list.dedup();
    }
    (accessories, mains)
}

#[cfg(test)]
mod tests;
