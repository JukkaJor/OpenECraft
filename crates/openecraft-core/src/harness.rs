//! Wire harnesses: connections (point-to-point, identified by their two connection
//! point designations) assigned to a harness are made as one assembly. The harness data lists its wires with lengths and end treatment, its
//! connectors (device ends with their pin assignment) and the bundle diameter, and
//! is exported as a simplified KBL file (VDA 4964) for harness manufacturing.

use crate::model::{natural_cmp, Project};
use crate::netlist::{pin_designation, PinRef};
use crate::reports::Analysis;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::Write;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Harness {
    pub id: u64,
    /// Complete device tag, e.g. `=A1-WH1`.
    pub tag: String,
    pub description: String,
    /// Length in metres of wires that are not routed in the panel layout (field
    /// wiring); 0 = unknown.
    pub length: f32,
    /// Part number of the finished harness (bill of materials).
    pub part: String,
    /// Connections of the harness: both ends as connection point designations
    /// (`-S1:13`), in [`key`] order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub connections: Vec<[String; 2]>,
}

/// Order independent key of a connection.
pub fn key(a: &str, b: &str) -> [String; 2] {
    if natural_cmp(a, b) == std::cmp::Ordering::Greater { [b.into(), a.into()] } else { [a.into(), b.into()] }
}

/// Harness of the connection between two connection points (empty: none).
pub fn of(prj: &Project, a: &str, b: &str) -> String {
    let k = key(a, b);
    prj.harnesses.iter().find(|h| h.connections.contains(&k)).map(|h| h.tag.clone()).unwrap_or_default()
}

/// Assigns a connection to a harness (empty: to none).
pub fn assign(prj: &mut Project, a: &str, b: &str, tag: &str) {
    let k = key(a, b);
    for h in &mut prj.harnesses {
        h.connections.retain(|c| *c != k);
    }
    if let Some(h) = prj.harnesses.iter_mut().find(|h| h.tag == tag) {
        h.connections.push(k);
    }
}

/// Keeps the assignments when a device is renamed.
pub fn rename_device(prj: &mut Project, old: &str, new: &str) {
    for h in &mut prj.harnesses {
        for c in &mut h.connections {
            for end in c.iter_mut() {
                if end == old {
                    *end = new.to_string();
                } else if let Some(pin) = end.strip_prefix(old).and_then(|r| r.strip_prefix(':')) {
                    *end = format!("{new}:{pin}");
                }
            }
            *c = key(&c[0], &c[1]);
        }
    }
}

/// Assigned connections that do not exist in the schematic (any more): (harness, ends).
pub fn stale(prj: &Project, an: &Analysis) -> Vec<(String, [String; 2])> {
    let existing: Vec<[String; 2]> = an.connections.iter().map(|c| key(&pin_designation(prj, c.a), &pin_designation(prj, c.b))).collect();
    prj.harnesses
        .iter()
        .flat_map(|h| h.connections.iter().filter(|c| !existing.contains(c)).map(|c| (h.tag.clone(), c.clone())))
        .collect()
}

/// Connections (indices into `an.connections`) whose two ends both lie on end
/// points of the given wires of a page: selecting the wires between two devices
/// selects their connection.
pub fn connections_at(prj: &Project, an: &Analysis, page: usize, wires: &BTreeSet<u64>) -> Vec<usize> {
    let ends: Vec<crate::geom::P> = prj.pages[page].wires.iter().filter(|w| wires.contains(&w.id)).flat_map(|w| [w.a, w.b]).collect();
    let pos = |pin: PinRef| {
        let s = &prj.pages[pin.page].symbols[pin.sym];
        crate::symbols::get(&s.sym).map(|d| d.pin_world(pin.pin, s.pos, s.rot, s.mirror))
    };
    let touched = |pin: PinRef| pin.page == page && pos(pin).map(|q| ends.iter().any(|e| e.same(q))).unwrap_or(false);
    (0..an.connections.len()).filter(|&i| touched(an.connections[i].a) && touched(an.connections[i].b)).collect()
}

/// Connections of the net a wire belongs to (indices into `an.connections`).
pub fn connections_of_wire(an: &Analysis, wire: u64) -> Vec<usize> {
    let Some(net) = an.open.wire_net.get(&wire).map(|&n| &an.open.nets[n]) else { return Vec::new() };
    (0..an.connections.len()).filter(|&i| net.pins.contains(&an.connections[i].a) && net.pins.contains(&an.connections[i].b)).collect()
}

/// One wire of a harness.
#[derive(Clone, Debug, PartialEq)]
pub struct HarnessWire {
    pub number: String,
    /// Device tag and connection point of both ends.
    pub from: (String, String),
    pub to: (String, String),
    pub color: String,
    pub cross_section: String,
    /// Length in mm (routed in the panel layout, or the harness length); 0 = unknown.
    pub length: f32,
    pub routed: bool,
    pub target: Option<(usize, u64)>,
}

/// A device end of a harness with its pin assignment: (pin, wire number, other end).
#[derive(Clone, Debug, PartialEq)]
pub struct Connector {
    pub device: String,
    pub pins: Vec<(String, String, String)>,
}

fn end(prj: &Project, pin: PinRef) -> (String, String) {
    let tag = prj.pages[pin.page].symbols[pin.sym].tag.clone();
    let d = pin_designation(prj, pin);
    let pin_name = d.strip_prefix(tag.as_str()).unwrap_or(&d).trim_start_matches(':').to_string();
    (tag, pin_name)
}

fn joined(e: &(String, String)) -> String {
    if e.1.is_empty() { e.0.clone() } else { format!("{}:{}", e.0, e.1) }
}

/// Wires of a harness, in wire number order.
pub fn wires(prj: &Project, an: &Analysis, tag: &str) -> Vec<HarnessWire> {
    let default = prj.harnesses.iter().find(|h| h.tag == tag).map(|h| h.length * 1000.0).unwrap_or(0.0);
    let mut v: Vec<HarnessWire> = an
        .connections
        .iter()
        .filter(|c| c.harness == tag)
        .map(|c| {
            let (from, to) = (end(prj, c.a), end(prj, c.b));
            let (fa, fb) = (pin_designation(prj, c.a), pin_designation(prj, c.b));
            let routed = an.wiring.iter().find(|w| (w.from == fa && w.to == fb) || (w.from == fb && w.to == fa)).map(|w| w.length);
            HarnessWire {
                number: c.number.clone(),
                from,
                to,
                color: c.color.clone(),
                cross_section: c.cross_section.clone(),
                length: routed.unwrap_or(default),
                routed: routed.is_some(),
                target: Some((c.a.page, prj.pages[c.a.page].symbols[c.a.sym].id)),
            }
        })
        .collect();
    v.sort_by(|a, b| natural_cmp(&a.number, &b.number).then(natural_cmp(&joined(&a.from), &joined(&b.from))));
    v
}

/// Device ends of the wires with their pin assignment.
pub fn connectors(wires: &[HarnessWire]) -> Vec<Connector> {
    let mut v: Vec<Connector> = Vec::new();
    let mut add = |dev: &str, pin: &str, number: &str, other: &(String, String)| {
        let entry = (pin.to_string(), number.to_string(), joined(other));
        match v.iter_mut().find(|c| c.device == dev) {
            Some(c) => c.pins.push(entry),
            None => v.push(Connector { device: dev.to_string(), pins: vec![entry] }),
        }
    };
    for w in wires {
        add(&w.from.0, &w.from.1, &w.number, &w.to);
        add(&w.to.0, &w.to.1, &w.number, &w.from);
    }
    for c in &mut v {
        c.pins.sort_by(|a, b| natural_cmp(&a.0, &b.0));
    }
    v.sort_by(|a, b| natural_cmp(&a.device, &b.device));
    v
}

/// Outer diameter of a single core wire (PVC insulated, H05V-K / H07V-K like).
pub fn outer_diameter(cross_section: &str) -> f32 {
    let cs = crate::production::cross_section_mm2(cross_section).unwrap_or(1.0);
    1.0 + 1.6 * cs.sqrt()
}

/// Diameter of the bundle (wires packed round, 15 % gaps).
pub fn bundle_diameter(wires: &[HarnessWire]) -> f32 {
    let sum: f32 = wires.iter().map(|w| outer_diameter(&w.cross_section).powi(2)).sum();
    sum.sqrt() * 1.15
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Harness data as simplified KBL (VDA 4964): general wires (cross-section and
/// colour), and per harness its connector occurrences with contact points, wire
/// occurrences with lengths and the connections between contact points.
pub fn kbl(prj: &Project, an: &Analysis) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<!-- Simplified KBL (VDA 4964) harness data, exported by OpenECraft -->\n");
    out.push_str("<kbl:KBL_container xmlns:kbl=\"http://www.prostep.org/Car_electric_container/KBL2.3/KBLSchema\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" version_id=\"2.4\">\n");
    let all: Vec<(String, Vec<HarnessWire>)> = prj.harnesses.iter().map(|h| (h.tag.clone(), wires(prj, an, &h.tag))).collect();
    // General wires: one per cross-section and colour.
    let mut general: Vec<(String, String)> = Vec::new();
    for w in all.iter().flat_map(|(_, ws)| ws) {
        let key = (w.cross_section.clone(), w.color.clone());
        if !general.contains(&key) {
            general.push(key);
        }
    }
    let gw_id = |cs: &str, color: &str| general.iter().position(|g| g.0 == cs && g.1 == color).map(|i| format!("GW{}", i + 1)).unwrap_or_default();
    // Connector housings: the (first) part of the plugs the harnesses end at.
    let plugs = crate::plugs::plug_tags(prj);
    let housing_of = |dev: &str| -> Option<String> {
        plugs.iter().any(|p| p == dev).then(|| prj.devices.get(dev).and_then(|d| d.parts.first()).map(|r| r.number.clone())).flatten()
    };
    let mut housings: Vec<String> = Vec::new();
    for c in all.iter().flat_map(|(_, ws)| connectors(ws)) {
        if let Some(h) = housing_of(&c.device).filter(|h| !housings.contains(h)) {
            housings.push(h);
        }
    }
    let ch_id = |dev: &str| housing_of(dev).and_then(|h| housings.iter().position(|x| *x == h)).map(|i| format!("CH{}", i + 1));
    for (i, (cs, color)) in general.iter().enumerate() {
        let area = crate::production::cross_section_mm2(cs).map(|v| v.to_string()).unwrap_or_default();
        let _ = writeln!(
            out,
            "  <General_wire id=\"GW{}\"><Part_number>{} {}</Part_number><Cross_section_area><Value_component>{area}</Value_component><Unit_component>mm2</Unit_component></Cross_section_area><Cover_colour>{}</Cover_colour><Outside_diameter><Value_component>{:.2}</Value_component><Unit_component>mm</Unit_component></Outside_diameter></General_wire>",
            i + 1,
            xml(cs),
            xml(color),
            xml(color),
            outer_diameter(cs)
        );
    }
    for (i, h) in housings.iter().enumerate() {
        let _ = writeln!(out, "  <Connector_housing id=\"CH{}\"><Part_number>{}</Part_number></Connector_housing>", i + 1, xml(h));
    }
    for (hi, h) in prj.harnesses.iter().enumerate() {
        let ws = &all[hi].1;
        let _ = writeln!(out, "  <Harness id=\"H{}\"><Part_number>{}</Part_number><Description>{}</Description>", hi + 1, xml(if h.part.is_empty() { &h.tag } else { &h.part }), xml(&h.description));
        let _ = writeln!(out, "    <Id>{}</Id>", xml(&h.tag));
        let cons = connectors(ws);
        let cp_id = |dev: &str, pin: &str| -> String {
            let ci = cons.iter().position(|c| c.device == dev).unwrap_or(0);
            let pi = cons.get(ci).and_then(|c| c.pins.iter().position(|p| p.0 == pin)).unwrap_or(0);
            format!("H{}_C{}_P{}", hi + 1, ci + 1, pi + 1)
        };
        for (ci, c) in cons.iter().enumerate() {
            let _ = write!(out, "    <Connector_occurrence id=\"H{}_C{}\"><Id>{}</Id>", hi + 1, ci + 1, xml(&c.device));
            if let Some(id) = ch_id(&c.device) {
                let _ = write!(out, "<Part>{id}</Part>");
            }
            out.push_str("<Contact_points>");
            let mut seen: Vec<&str> = Vec::new();
            for (pi, pin) in c.pins.iter().enumerate() {
                if seen.contains(&pin.0.as_str()) {
                    continue;
                }
                seen.push(&pin.0);
                let _ = write!(out, "<Contact_point id=\"H{}_C{}_P{}\"><Id>{}</Id></Contact_point>", hi + 1, ci + 1, pi + 1, xml(&pin.0));
            }
            out.push_str("</Contact_points></Connector_occurrence>\n");
        }
        for (wi, w) in ws.iter().enumerate() {
            let id = format!("H{}_W{}", hi + 1, wi + 1);
            let _ = writeln!(
                out,
                "    <General_wire_occurrence xsi:type=\"kbl:Wire_occurrence\" id=\"{id}\"><Part>{}</Part><Wire_number>{}</Wire_number><Length_information><Length_type>{}</Length_type><Length_value><Value_component>{:.0}</Value_component><Unit_component>mm</Unit_component></Length_value></Length_information></General_wire_occurrence>",
                gw_id(&w.cross_section, &w.color),
                xml(&w.number),
                if w.routed { "DMA" } else { "Nominal" },
                w.length
            );
            let _ = writeln!(
                out,
                "    <Connection id=\"H{}_N{}\"><Wire>{id}</Wire><Extremities><Extremity><Position_on_wire>0</Position_on_wire><Contact_point>{}</Contact_point></Extremity><Extremity><Position_on_wire>1</Position_on_wire><Contact_point>{}</Contact_point></Extremity></Extremities></Connection>",
                hi + 1,
                wi + 1,
                cp_id(&w.from.0, &w.from.1),
                cp_id(&w.to.0, &w.to.1)
            );
        }
        out.push_str("  </Harness>\n");
    }
    out.push_str("</kbl:KBL_container>\n");
    out
}

#[cfg(test)]
mod tests;
