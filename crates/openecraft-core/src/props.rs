//! Property placement: device properties shown as texts beside a symbol, and the
//! position of the tag block.

use openecraft_i18n::{tr, tr_const};
use crate::geom::P;
use crate::model::*;

impl PropKind {
    pub const ALL: [PropKind; 7] = [
        PropKind::Description,
        PropKind::PartNumber,
        PropKind::TypeNumber,
        PropKind::Manufacturer,
        PropKind::Label,
        PropKind::Pins,
        PropKind::Structure,
    ];

    pub fn title(self) -> &'static str {
        tr(match self {
            PropKind::Description => tr_const!("Device description"),
            PropKind::PartNumber => tr_const!("Part number"),
            PropKind::TypeNumber => tr_const!("Type number"),
            PropKind::Manufacturer => tr_const!("Manufacturer"),
            PropKind::Label => tr_const!("Designation"),
            PropKind::Pins => tr_const!("Connection points"),
            PropKind::Structure => tr_const!("Structure"),
        })
    }
}

/// Text of a property of the symbol's device.
pub fn value(prj: &Project, s: &SymbolInst, kind: PropKind) -> String {
    let dev = prj.devices.get(&s.tag);
    let parts: Vec<&Part> = dev
        .map(|d| d.parts.iter().filter_map(|r| prj.parts.iter().find(|p| p.number == r.number)).collect())
        .unwrap_or_default();
    let join = |v: Vec<&str>| {
        let mut v: Vec<&str> = v.into_iter().filter(|t| !t.is_empty()).collect();
        v.dedup();
        v.join(", ")
    };
    match kind {
        PropKind::Description => dev.map(|d| d.description.clone()).unwrap_or_default(),
        PropKind::PartNumber => join(dev.map(|d| d.parts.iter().map(|r| r.number.as_str()).collect()).unwrap_or_default()),
        PropKind::TypeNumber => join(parts.iter().map(|p| p.type_number.as_str()).collect()),
        PropKind::Manufacturer => join(parts.iter().map(|p| p.manufacturer.as_str()).collect()),
        PropKind::Label => s.label.clone(),
        PropKind::Pins => s.pins.iter().filter(|p| !p.is_empty()).cloned().collect::<Vec<_>>().join(" "),
        PropKind::Structure => crate::structure::Dt::parse(&s.tag).structure(),
    }
}

/// Adds a property text below the symbol (below the ones already placed).
pub fn add(s: &mut SymbolInst, kind: PropKind, below: P) {
    let y = s.props.iter().map(|p| p.offset.y + 2.8).fold(below.y, f32::max);
    s.props.push(PlacedProp { kind, offset: crate::geom::p(below.x, y), size: 2.0 });
}

#[cfg(test)]
mod tests;
