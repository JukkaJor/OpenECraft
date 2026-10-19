//! Device / cross-reference index of a project.

use crate::geom::P;
use crate::model::{column_of, natural_cmp, PageKind, Project};
use crate::symbols::{self, Func};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Copy, Debug)]
pub struct Loc {
    pub page: usize,
    pub sym_id: u64,
    pub pos: P,
}

impl Loc {
    pub fn reference(&self, prj: &Project) -> String {
        format!("/{}.{}", prj.pages[self.page].name, column_of(self.pos))
    }
}

#[derive(Clone, Debug, Default)]
pub struct DeviceEntry {
    pub mains: Vec<Loc>,
    pub auxs: Vec<Loc>,
    /// Terminals, PLC points (no main / aux relation).
    pub others: Vec<Loc>,
}

#[derive(Default)]
pub struct Index {
    pub devices: BTreeMap<String, DeviceEntry>,
    pub interruptions: HashMap<String, Vec<Loc>>,
}

impl Index {
    pub fn build(prj: &Project) -> Index {
        let mut idx = Index::default();
        for (pi, page) in prj.pages.iter().enumerate() {
            if page.kind != PageKind::Schematic {
                continue;
            }
            for s in &page.symbols {
                let Some(def) = symbols::get(&s.sym) else { continue };
                let loc = Loc { page: pi, sym_id: s.id, pos: s.pos };
                match def.func {
                    Func::Interruption => idx.interruptions.entry(s.label.clone()).or_default().push(loc),
                    Func::Potential => {}
                    _ if s.tag.is_empty() => {}
                    Func::Main { .. } => idx.devices.entry(s.tag.clone()).or_default().mains.push(loc),
                    Func::Aux(_) => idx.devices.entry(s.tag.clone()).or_default().auxs.push(loc),
                    Func::Terminal | Func::Plc => idx.devices.entry(s.tag.clone()).or_default().others.push(loc),
                }
            }
        }
        // Sort aux functions by page and position for a stable contact image.
        for dev in idx.devices.values_mut() {
            dev.auxs.sort_by(|a, b| {
                a.page.cmp(&b.page).then(a.pos.x.total_cmp(&b.pos.x)).then(a.pos.y.total_cmp(&b.pos.y))
            });
        }
        idx
    }

    /// Device tags in natural order.
    pub fn sorted_tags(&self) -> Vec<&String> {
        let mut v: Vec<&String> = self.devices.keys().collect();
        v.sort_by(|a, b| natural_cmp(a, b));
        v
    }
}
