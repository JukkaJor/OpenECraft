//! Accessories at the cable ends: cable glands (EMC glands with shield contact),
//! connectors, strain reliefs, shield clamps, protective hoses, end sleeves and
//! markers. They go into the bill of materials; glands are checked against the
//! cable diameter and shielded field cables need a shield contact where the shield is
//! connected in an enclosure.

use super::Plan;
use crate::model::Project;
use crate::reports::{Issue, Severity};
use openecraft_i18n::{tr, tr_fmt};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccessoryKind {
    #[default]
    Gland,
    /// Cable gland with shield contact.
    EmcGland,
    Connector,
    StrainRelief,
    ShieldClamp,
    ProtectiveHose,
    EndSleeve,
    Marker,
    Other,
}

impl AccessoryKind {
    pub const ALL: [AccessoryKind; 9] = [
        AccessoryKind::Gland,
        AccessoryKind::EmcGland,
        AccessoryKind::Connector,
        AccessoryKind::StrainRelief,
        AccessoryKind::ShieldClamp,
        AccessoryKind::ProtectiveHose,
        AccessoryKind::EndSleeve,
        AccessoryKind::Marker,
        AccessoryKind::Other,
    ];

    pub fn title(self) -> &'static str {
        match self {
            AccessoryKind::Gland => tr("Cable gland"),
            AccessoryKind::EmcGland => tr("EMC cable gland"),
            AccessoryKind::Connector => tr("Connector"),
            AccessoryKind::StrainRelief => tr("Strain relief"),
            AccessoryKind::ShieldClamp => tr("Shield clamp"),
            AccessoryKind::ProtectiveHose => tr("Protective hose"),
            AccessoryKind::EndSleeve => tr("End sleeve"),
            AccessoryKind::Marker => tr("Cable marker"),
            AccessoryKind::Other => tr("Other"),
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            AccessoryKind::Gland => "gland",
            AccessoryKind::EmcGland => "emc_gland",
            AccessoryKind::Connector => "connector",
            AccessoryKind::StrainRelief => "strain_relief",
            AccessoryKind::ShieldClamp => "shield_clamp",
            AccessoryKind::ProtectiveHose => "protective_hose",
            AccessoryKind::EndSleeve => "end_sleeve",
            AccessoryKind::Marker => "marker",
            AccessoryKind::Other => "other",
        }
    }

    pub fn from_key(key: &str) -> Option<AccessoryKind> {
        AccessoryKind::ALL.into_iter().find(|k| k.key() == key.trim().to_ascii_lowercase())
    }

    /// Makes contact with the cable shield.
    pub fn contacts_shield(self) -> bool {
        matches!(self, AccessoryKind::EmcGland | AccessoryKind::ShieldClamp | AccessoryKind::Connector)
    }

    /// Clamps the cable sheath (diameter checked against the clamping range).
    pub fn clamps(self) -> bool {
        matches!(self, AccessoryKind::Gland | AccessoryKind::EmcGland | AccessoryKind::StrainRelief)
    }
}

/// An accessory of a cable at one end (device tag of the end) or at both ends.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Accessory {
    pub id: u64,
    pub cable: String,
    /// Device tag of the end; empty = at both ends.
    pub at: String,
    pub kind: AccessoryKind,
    pub part: String,
    /// Quantity per end.
    pub count: u32,
}

impl Accessory {
    /// Number of ends it is used at.
    fn ends(&self) -> u32 {
        if self.at.is_empty() { 2 } else { 1 }
    }

    pub fn at_end(&self, tag: &str) -> bool {
        self.at.is_empty() || self.at == tag
    }
}

/// Accessories by part number with their quantity.
pub fn material(prj: &Project) -> Vec<(String, f32, Vec<String>)> {
    let mut m: BTreeMap<String, (f32, Vec<String>)> = BTreeMap::new();
    for a in prj.field.accessories.iter().filter(|a| !a.part.is_empty()) {
        let e = m.entry(a.part.clone()).or_default();
        e.0 += (a.count.max(1) * a.ends()) as f32;
        if !e.1.contains(&a.cable) {
            e.1.push(a.cable.clone());
        }
    }
    m.into_iter().map(|(k, (q, u))| (k, q, u)).collect()
}

/// Clamping range of a part in mm (technical data "Clamping range min" / "max").
pub fn clamping_range(prj: &Project, part: &str) -> Option<(f32, f32)> {
    let p = prj.parts.iter().chain(crate::parts::catalog()).find(|p| p.number == part)?;
    Some((crate::panel::tech_number(p, "Clamping range min")?, crate::panel::tech_number(p, "Clamping range max")?))
}

/// Accessories of unknown cables, glands that do not fit the cable, and shielded field
/// cables without a shield contact at an enclosure end where the shield is connected.
pub fn problems(prj: &Project, plan: &Plan) -> Vec<Issue> {
    let mut v = Vec::new();
    let warn = |message: String| Issue { severity: Severity::Warning, message, target: None };
    for a in &prj.field.accessories {
        let Some(c) = prj.cable(&a.cable) else {
            v.push(warn(tr_fmt!("Accessory {} refers to the unknown cable {}", a.kind.title(), a.cable)));
            continue;
        };
        if !a.kind.clamps() {
            continue;
        }
        let Some((min, max)) = clamping_range(prj, &a.part) else { continue };
        let d = crate::routing3d::cable_diameter(c);
        if d < min || d > max {
            v.push(Issue {
                severity: Severity::Error,
                message: tr_fmt!("Cable {} ({} mm) does not fit the clamping range of {} ({}–{} mm)", c.tag, format!("{d:.1}"), a.part, format!("{min}"), format!("{max}")),
                target: None,
            });
        }
    }
    for fc in plan.cables.iter().filter(|c| c.shielded && c.routed()) {
        let Some(c) = prj.cable(&fc.cable) else { continue };
        for k in 0..2 {
            if !fc.in_enclosure[k] || !c.shield_ends.connects(k) {
                continue;
            }
            let tag = &fc.ends[k];
            let contact = prj.field.accessories.iter().any(|a| a.cable == fc.cable && a.at_end(tag) && a.kind.contacts_shield());
            if !contact {
                v.push(warn(tr_fmt!("Shielded cable {} has no EMC gland, shield clamp or connector at {}", fc.cable, tag)));
            }
        }
    }
    v
}
