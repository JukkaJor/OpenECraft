//! Device / cross-reference index of a project.

use crate::geom::P;
use crate::model::{column_of, natural_cmp, PageKind, Project};
use crate::structure;
use crate::symbols::{self, Func};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Copy, Debug)]
pub struct Loc {
    pub page: usize,
    pub sym_id: u64,
    pub pos: P,
}

impl Loc {
    /// Complete cross reference, e.g. `/3.4` or `=A1+CAB1/3.4`.
    pub fn reference(&self, prj: &Project) -> String {
        let page = &prj.pages[self.page];
        format!("{}/{}.{}", structure::page_structure(page), page.name, column_of(self.pos))
    }

    /// Cross reference as shown on page `from`: the structure is left out if both
    /// pages have the same.
    pub fn reference_from(&self, prj: &Project, from: usize) -> String {
        let (page, here) = (&prj.pages[self.page], &prj.pages[from]);
        if structure::page_structure(page) == structure::page_structure(here) {
            format!("/{}.{}", page.name, column_of(self.pos))
        } else {
            self.reference(prj)
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DeviceEntry {
    pub mains: Vec<Loc>,
    pub auxs: Vec<Loc>,
    /// Terminals, PLC points (no main / aux relation).
    pub others: Vec<Loc>,
    /// Single-line / overview representations.
    pub overviews: Vec<Loc>,
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
                    Func::Overview => idx.devices.entry(s.tag.clone()).or_default().overviews.push(loc),
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
