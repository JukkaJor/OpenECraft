//! Harness commands: the harnesses with their figures, the harness interface data, the
//! 3D topology (stored or derived) and its editing (nodes, segments, protections,
//! fixings), the end treatment of the connectors and the further parts.

use crate::params::{optional, param, Args, Kind, Param};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::analysis::Analysis;
use openecraft_core::harness::topology::{self, Problem};
use openecraft_core::harness::{self, End, ExtraPart, Fixing, FixingKind, Harness, PartKind, Protection, ProtectionKind};
use openecraft_core::model::harness::{Node, Segment};
use openecraft_core::model::Project;
use openecraft_i18n::{tr, tr_const, tr_fmt};
use serde_json::{json, Value};

pub struct Harnesses;

impl Plugin for Harnesses {
    fn info(&self) -> Info {
        info("harnesses", tr_const!("Harnesses"), tr_const!("Harness data for machines and tools, 3D topology, protections, fixings and end treatment"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

const HARNESS: Param = param("harness", Kind::Str, tr_const!("Harness tag, e.g. -WH1"));
const REMOVE: Param = optional("remove", Kind::Bool, tr_const!("Remove it"));

pub static COMMANDS: &[Spec] = &[
    Spec { name: "list_harnesses", help: tr_const!("Lists the harnesses with wires, connectors, wire length, bundle diameter and where their 3D topology comes from"), params: &[], run: list_harnesses },
    Spec {
        name: "harness_data",
        help: tr_const!("Harness interface for machines and tools: connectors with end treatment, wires with lengths and cut lengths, 3D topology, protections, fixings and parts (JSON format openecraft.harness)"),
        params: &[optional("harness", Kind::Str, tr_const!("Harness tag, e.g. -WH1 (default: all harnesses)"))],
        run: harness_data,
    },
    Spec { name: "harness_topology", help: tr_const!("3D topology of a harness: nodes, segments with centre curve, length and bundle diameter, the way of each wire and the problems"), params: &[HARNESS], run: harness_topology },
    Spec { name: "store_harness_topology", help: tr_const!("Stores the derived 3D topology of a harness for editing (replaces a stored one)"), params: &[HARNESS], run: store_topology },
    Spec { name: "clear_harness_topology", help: tr_const!("Drops the stored 3D topology of a harness; it is derived from the panel layout again"), params: &[HARNESS], run: clear_topology },
    Spec {
        name: "set_harness_node",
        help: tr_const!("Adds or changes a node of the stored topology (the derived one is stored first), or removes it"),
        params: &[
            HARNESS,
            param("id", Kind::Str, tr_const!("Node id, e.g. N1")),
            optional("x", Kind::Num, tr_const!("Position x, mm")),
            optional("y", Kind::Num, tr_const!("Position y, mm")),
            optional("z", Kind::Num, tr_const!("Position z, mm")),
            optional("connectors", Kind::Str, tr_const!("Device tags of the connectors at the node, separated by commas")),
            optional("bend_radius", Kind::Num, tr_const!("Bend radius of the bundle at the node, mm")),
            REMOVE,
        ],
        run: set_node,
    },
    Spec {
        name: "set_harness_segment",
        help: tr_const!("Adds or changes a segment of the stored topology (the derived one is stored first), or removes it"),
        params: &[
            HARNESS,
            param("id", Kind::Str, tr_const!("Segment id, e.g. S1")),
            optional("start", Kind::Str, tr_const!("Node id at the start")),
            optional("end", Kind::Str, tr_const!("Node id at the end")),
            optional("points", Kind::Str, tr_const!("Points of the centre curve between the nodes: x,y,z separated by semicolons, mm")),
            optional("length", Kind::Num, tr_const!("Formboard length, mm (0: length of the centre curve)")),
            REMOVE,
        ],
        run: set_segment,
    },
    Spec {
        name: "set_harness_protection",
        help: tr_const!("Adds or changes a wire protection on a segment (or on all segments), or removes it"),
        params: &[
            HARNESS,
            param("id", Kind::Str, tr_const!("Id of the protection, e.g. P1")),
            optional("kind", Kind::Str, tr_const!("tape, tube, corrugated_tube, sleeve or heat_shrink")),
            optional("part", Kind::Str, tr_const!("Part number")),
            optional("segment", Kind::Str, tr_const!("Segment id (empty: all segments)")),
            optional("start", Kind::Num, tr_const!("Start along the segment, 0 … 1")),
            optional("end", Kind::Num, tr_const!("End along the segment, 0 … 1")),
            REMOVE,
        ],
        run: set_protection,
    },
    Spec {
        name: "set_harness_fixing",
        help: tr_const!("Adds or changes a fixing on a segment, or removes it"),
        params: &[
            HARNESS,
            param("id", Kind::Str, tr_const!("Id of the fixing, e.g. F1")),
            optional("kind", Kind::Str, tr_const!("clip, cable_tie, grommet or bracket")),
            optional("part", Kind::Str, tr_const!("Part number")),
            optional("segment", Kind::Str, tr_const!("Segment id")),
            optional("location", Kind::Num, tr_const!("Location along the segment, 0 … 1")),
            REMOVE,
        ],
        run: set_fixing,
    },
    Spec {
        name: "set_harness_end",
        help: tr_const!("Sets the end treatment of a connector or of one of its pins: terminal, seal, cavity plug, stripping length, KBL usage"),
        params: &[
            HARNESS,
            param("device", Kind::Str, tr_const!("Device tag of the connector")),
            optional("pin", Kind::Str, tr_const!("Connection point (empty: all pins of the device)")),
            optional("terminal", Kind::Str, tr_const!("Part number of the terminal (crimp contact, ferrule, ring terminal)")),
            optional("seal", Kind::Str, tr_const!("Part number of the single wire seal")),
            optional("cavity_plug", Kind::Str, tr_const!("Part number of the plugs of unused cavities")),
            optional("strip", Kind::Num, tr_const!("Stripping length, mm (0: from the cross-section)")),
            optional("usage", Kind::Str, tr_const!("KBL usage: no end, ring terminal, splice or dangler (empty: none)")),
            REMOVE,
        ],
        run: set_end,
    },
    Spec {
        name: "set_harness_part",
        help: tr_const!("Adds or changes a further part of a harness (accessory, co-pack part, assembly, component), or removes it"),
        params: &[
            HARNESS,
            param("part", Kind::Str, tr_const!("Part number")),
            optional("kind", Kind::Str, tr_const!("accessory, co_pack, assembly or component")),
            optional("count", Kind::Int, tr_const!("Quantity")),
            optional("description", Kind::Str, tr_const!("Description")),
            optional("node", Kind::Str, tr_const!("Node id the part sits at")),
            REMOVE,
        ],
        run: set_part,
    },
];

fn harness_mut<'a>(prj: &'a mut Project, tag: &str) -> Result<&'a mut Harness, String> {
    prj.harnesses.iter_mut().find(|h| h.tag == tag.trim()).ok_or_else(|| tr_fmt!("No harness {}", tag.trim()))
}

fn round(v: f32) -> f64 {
    (f64::from(v) * 10.0).round() / 10.0
}

fn list_harnesses(prj: &mut Project, _: &Args) -> Result<Value, String> {
    let an = Analysis::checked(prj);
    let list: Vec<Value> = prj
        .harnesses
        .iter()
        .map(|h| {
            let ws = harness::wires(prj, &an, &h.tag);
            let t = topology::build(prj, &an, h, &ws);
            json!({
                "tag": h.tag, "description": h.description, "part": h.part, "wires": ws.len(), "connectors": harness::connectors(&ws).len(),
                "wire_length_m": round(ws.iter().map(|w| w.length).sum::<f32>() / 100.0) / 10.0,
                "bundle_diameter_mm": round(if ws.is_empty() { 0.0 } else { harness::bundle_diameter(&ws) }),
                "topology": t.source.key(), "nodes": t.nodes.len(), "segments": t.segments.len(),
                "protections": h.protections.len(), "fixings": h.fixings.len(),
            })
        })
        .collect();
    Ok(json!({ "harnesses": list }))
}

fn harness_data(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let an = Analysis::checked(prj);
    openecraft_core::harness_export::document(prj, &an, a.str("harness"))
}

fn problem_json(p: &Problem) -> Value {
    match p {
        Problem::DuplicateId(id) => json!({ "problem": "duplicate_id", "id": id }),
        Problem::UnknownNode(s, n) => json!({ "problem": "unknown_node", "segment": s, "node": n }),
        Problem::UnknownSegment(id, s) => json!({ "problem": "unknown_segment", "id": id, "segment": s }),
        Problem::OutOfRange(id) => json!({ "problem": "out_of_range", "id": id }),
        Problem::ConnectorWithoutNode(d) => json!({ "problem": "connector_without_node", "connector": d }),
        Problem::NotRouted(n, a, b) => json!({ "problem": "not_routed", "wire": n, "from": a, "to": b }),
        Problem::BendRadius(n, r, min) => json!({ "problem": "bend_radius", "node": n, "radius_mm": round(*r), "smallest_mm": round(*min) }),
    }
}

fn harness_topology(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let tag = a.req("harness").trim().to_string();
    let an = Analysis::checked(prj);
    let h = prj.harnesses.iter().find(|h| h.tag == tag).ok_or_else(|| tr_fmt!("No harness {}", tag))?;
    let ws = harness::wires(prj, &an, &tag);
    let t = topology::build(prj, &an, h, &ws);
    let point = |q: [f32; 3]| json!([round(q[0]), round(q[1]), round(q[2])]);
    let nodes: Vec<Value> = t.nodes.iter().map(|n| json!({ "id": n.id, "position": point(n.pos), "connectors": n.connectors, "bend_radius_mm": round(n.bend_radius) })).collect();
    let segments: Vec<Value> = t
        .segments
        .iter()
        .map(|s| {
            json!({
                "id": s.id, "start": t.nodes[s.start].id, "end": t.nodes[s.end].id, "length_mm": round(s.length), "physical_length_mm": round(s.physical),
                "diameter_mm": round(s.diameter), "points": s.points.iter().map(|q| point(*q)).collect::<Vec<_>>(), "wires": s.wires.iter().map(|&w| ws[w].number.clone()).collect::<Vec<_>>(),
            })
        })
        .collect();
    let routes: Vec<Value> = ws
        .iter()
        .zip(&t.routes)
        .enumerate()
        .map(|(i, (w, r))| {
            json!({
                "wire": w.number, "from": harness::joined(&w.from), "to": harness::joined(&w.to),
                "segments": r.as_ref().map(|r| r.iter().map(|&s| t.segments[s].id.clone()).collect::<Vec<_>>()),
                "length_mm": t.wire_length(i).map(round),
            })
        })
        .collect();
    let problems: Vec<Value> = topology::problems(prj, &an, h).iter().map(problem_json).collect();
    Ok(json!({ "harness": tag, "source": t.source.key(), "length_mm": round(t.length()), "nodes": nodes, "segments": segments, "routes": routes, "problems": problems }))
}

fn store_topology(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let tag = a.req("harness").trim().to_string();
    let an = Analysis::checked(prj);
    let (nodes, segments) = topology::store(prj, &an, &tag).ok_or_else(|| tr_fmt!("No harness {}", tag))?;
    Ok(json!({ "harness": tag, "nodes": nodes, "segments": segments }))
}

fn clear_topology(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let tag = a.req("harness").trim().to_string();
    harness_mut(prj, &tag)?;
    Ok(json!({ "harness": tag, "cleared": topology::clear(prj, &tag) }))
}

/// The harness with a stored topology (the derived one stored first).
fn stored<'a>(prj: &'a mut Project, tag: &str) -> Result<&'a mut Harness, String> {
    if harness_mut(prj, tag)?.nodes.is_empty() {
        let an = Analysis::checked(prj);
        topology::store(prj, &an, tag.trim());
    }
    harness_mut(prj, tag)
}

fn id_of(a: &Args) -> Result<String, String> {
    let id = a.req("id").trim().to_string();
    if id.is_empty() {
        return Err(tr_fmt!("Parameter {} has the wrong type", "id"));
    }
    Ok(id)
}

fn fraction(a: &Args, name: &str) -> Result<Option<f32>, String> {
    match a.num(name) {
        Some(v) if !(0.0..=1.0).contains(&v) => Err(tr_fmt!("{} must be between 0 and 1", name)),
        v => Ok(v),
    }
}

fn set_node(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let id = id_of(a)?;
    let h = stored(prj, a.req("harness"))?;
    if a.flag("remove") {
        let before = h.nodes.len();
        h.nodes.retain(|n| n.id != id);
        if h.nodes.len() == before {
            return Err(tr_fmt!("No node {}", id));
        }
        h.segments.retain(|s| s.start != id && s.end != id);
        return harness_topology(prj, a);
    }
    let n = match h.nodes.iter().position(|n| n.id == id) {
        Some(i) => &mut h.nodes[i],
        None => {
            h.nodes.push(Node { id: id.clone(), ..Default::default() });
            h.nodes.last_mut().unwrap()
        }
    };
    for (k, name) in ["x", "y", "z"].iter().enumerate() {
        if let Some(v) = a.num(name) {
            n.pos[k] = v;
        }
    }
    if let Some(v) = a.num("bend_radius") {
        n.bend_radius = v.max(0.0);
    }
    if let Some(list) = a.str("connectors") {
        let connectors: Vec<String> = list.split(',').map(|c| c.trim().to_string()).filter(|c| !c.is_empty()).collect();
        for other in h.nodes.iter_mut().filter(|n| n.id != id) {
            other.connectors.retain(|c| !connectors.contains(c));
        }
        h.nodes.iter_mut().find(|n| n.id == id).unwrap().connectors = connectors;
    }
    harness_topology(prj, a)
}

fn parse_points(s: &str) -> Result<Vec<[f32; 3]>, String> {
    s.split(';')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| {
            let v: Vec<f32> = p.split(',').map(|c| c.trim().parse::<f32>()).collect::<Result<_, _>>().map_err(|_| tr_fmt!("Not a point: {}", p))?;
            match v.as_slice() {
                [x, y, z] => Ok([*x, *y, *z]),
                [x, y] => Ok([*x, *y, 0.0]),
                _ => Err(tr_fmt!("Not a point: {}", p)),
            }
        })
        .collect()
}

fn set_segment(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let id = id_of(a)?;
    let h = stored(prj, a.req("harness"))?;
    if a.flag("remove") {
        let before = h.segments.len();
        h.segments.retain(|s| s.id != id);
        if h.segments.len() == before {
            return Err(tr_fmt!("No segment {}", id));
        }
        return harness_topology(prj, a);
    }
    let points = a.str("points").map(parse_points).transpose()?;
    let known = |n: &str| h.nodes.iter().any(|x| x.id == n);
    for end in ["start", "end"] {
        if let Some(n) = a.str(end).map(str::trim).filter(|n| !known(n)) {
            return Err(tr_fmt!("No node {}", n));
        }
    }
    let i = match h.segments.iter().position(|s| s.id == id) {
        Some(i) => i,
        None => {
            if a.str("start").is_none() || a.str("end").is_none() {
                return Err(tr("A new segment needs its start and end node").into());
            }
            h.segments.push(Segment { id: id.clone(), ..Default::default() });
            h.segments.len() - 1
        }
    };
    let s = &mut h.segments[i];
    if let Some(n) = a.str("start") {
        s.start = n.trim().into();
    }
    if let Some(n) = a.str("end") {
        s.end = n.trim().into();
    }
    if let Some(p) = points {
        s.points = p;
    }
    if let Some(l) = a.num("length") {
        s.length = l.max(0.0);
    }
    harness_topology(prj, a)
}

fn set_protection(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let id = id_of(a)?;
    let kind = a.str("kind").map(|k| ProtectionKind::from_key(k).ok_or_else(|| tr_fmt!("Unknown protection: {}", k))).transpose()?;
    let (start, end) = (fraction(a, "start")?, fraction(a, "end")?);
    let h = harness_mut(prj, a.req("harness"))?;
    if a.flag("remove") {
        let before = h.protections.len();
        h.protections.retain(|p| p.id != id);
        if h.protections.len() == before {
            return Err(tr_fmt!("No protection {}", id));
        }
        return Ok(json!({ "removed": id }));
    }
    let p = match h.protections.iter().position(|p| p.id == id) {
        Some(i) => &mut h.protections[i],
        None => {
            h.protections.push(Protection { id: id.clone(), end: 1.0, ..Default::default() });
            h.protections.last_mut().unwrap()
        }
    };
    if let Some(k) = kind {
        p.kind = k;
    }
    if let Some(v) = a.str("part") {
        p.part = v.trim().into();
    }
    if let Some(v) = a.str("segment") {
        p.segment = v.trim().into();
    }
    if let Some(v) = start {
        p.start = v;
    }
    if let Some(v) = end {
        p.end = v;
    }
    Ok(json!({ "id": p.id, "kind": p.kind.key(), "part": p.part, "segment": p.segment, "start": p.start, "end": p.end }))
}

fn set_fixing(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let id = id_of(a)?;
    let kind = a.str("kind").map(|k| FixingKind::from_key(k).ok_or_else(|| tr_fmt!("Unknown fixing: {}", k))).transpose()?;
    let location = fraction(a, "location")?;
    let h = harness_mut(prj, a.req("harness"))?;
    if a.flag("remove") {
        let before = h.fixings.len();
        h.fixings.retain(|f| f.id != id);
        if h.fixings.len() == before {
            return Err(tr_fmt!("No fixing {}", id));
        }
        return Ok(json!({ "removed": id }));
    }
    let f = match h.fixings.iter().position(|f| f.id == id) {
        Some(i) => &mut h.fixings[i],
        None => {
            h.fixings.push(Fixing { id: id.clone(), location: 0.5, ..Default::default() });
            h.fixings.last_mut().unwrap()
        }
    };
    if let Some(k) = kind {
        f.kind = k;
    }
    if let Some(v) = a.str("part") {
        f.part = v.trim().into();
    }
    if let Some(v) = a.str("segment") {
        f.segment = v.trim().into();
    }
    if let Some(v) = location {
        f.location = v;
    }
    Ok(json!({ "id": f.id, "kind": f.kind.key(), "part": f.part, "segment": f.segment, "location": f.location }))
}

fn set_end(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let device = a.req("device").trim().to_string();
    let pin = a.str("pin").unwrap_or_default().trim().to_string();
    let usage = a.str("usage").map(|u| u.trim().to_string());
    if let Some(u) = usage.as_deref().filter(|u| !u.is_empty() && !harness::USAGES.contains(u)) {
        return Err(tr_fmt!("Unknown usage: {}", u));
    }
    let h = harness_mut(prj, a.req("harness"))?;
    if a.flag("remove") {
        let before = h.ends.len();
        h.ends.retain(|e| !(e.device == device && e.pin == pin));
        return Ok(json!({ "removed": before - h.ends.len() }));
    }
    let e = match h.ends.iter().position(|e| e.device == device && e.pin == pin) {
        Some(i) => &mut h.ends[i],
        None => {
            h.ends.push(End { device: device.clone(), pin: pin.clone(), ..Default::default() });
            h.ends.last_mut().unwrap()
        }
    };
    if let Some(v) = a.str("terminal") {
        e.terminal = v.trim().into();
    }
    if let Some(v) = a.str("seal") {
        e.seal = v.trim().into();
    }
    if let Some(v) = a.str("cavity_plug") {
        e.cavity_plug = v.trim().into();
    }
    if let Some(v) = a.num("strip") {
        e.strip = v.max(0.0);
    }
    if let Some(u) = usage {
        e.usage = u;
    }
    Ok(json!({ "device": e.device, "pin": e.pin, "terminal": e.terminal, "seal": e.seal, "cavity_plug": e.cavity_plug, "strip_mm": e.strip, "usage": e.usage }))
}

fn set_part(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let part = a.req("part").trim().to_string();
    let kind = match a.str("kind") {
        Some(k) => PartKind::from_key(k).ok_or_else(|| tr_fmt!("Unknown part kind: {}", k))?,
        None => PartKind::Accessory,
    };
    let h = harness_mut(prj, a.req("harness"))?;
    if a.flag("remove") {
        let before = h.parts.len();
        h.parts.retain(|p| !(p.part == part && p.kind == kind));
        return Ok(json!({ "removed": before - h.parts.len() }));
    }
    let p = match h.parts.iter().position(|p| p.part == part && p.kind == kind) {
        Some(i) => &mut h.parts[i],
        None => {
            h.parts.push(ExtraPart { kind, part: part.clone(), ..Default::default() });
            h.parts.last_mut().unwrap()
        }
    };
    if let Some(n) = a.int("count") {
        p.count = n.max(1) as u32;
    }
    if let Some(v) = a.str("description") {
        p.description = v.trim().into();
    }
    if let Some(v) = a.str("node") {
        p.node = v.trim().into();
    }
    Ok(json!({ "kind": p.kind.key(), "part": p.part, "count": p.count, "description": p.description, "node": p.node }))
}

#[cfg(test)]
mod tests;
