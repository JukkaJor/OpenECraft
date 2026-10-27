//! Field wiring commands: plant layout with the route network (nodes, segments, places,
//! outlines), route definitions of cables, the cable planning through the network,
//! equipotential bonding and cable accessories.

use crate::builtin::common::{int_in, num_in, NO_MAX};
use crate::params::{optional, param, Args, Kind};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::analysis::Analysis;
use openecraft_core::field::{self, Accessory, AccessoryKind, Bond, BondKind, Field, Node, Outline, Place, Segment, SegmentKind};
use openecraft_core::model::Project;
use openecraft_core::routing3d::Class;
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub struct FieldWiring;

impl Plugin for FieldWiring {
    fn info(&self) -> Info {
        info("field", tr_const!("Field wiring"), tr_const!("Field plan: nodes, segments, outlines, routes, bonds and accessories"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

pub static COMMANDS: &[Spec] = &[
    Spec {
        name: "field_add_node",
        help: tr_const!("Adds a node of the field route network (junction, penetration, where an enclosure or device is reached)"),
        params: &[
            param("x", Kind::Num, tr_const!("Position right in metres")),
            param("y", Kind::Num, tr_const!("Position down in metres")),
            optional("z", Kind::Num, tr_const!("Height above the floor in metres")),
            optional("name", Kind::Str, tr_const!("Name")),
        ],
        run: add_node,
    },
    Spec {
        name: "field_add_segment",
        help: tr_const!("Adds a segment of the field route network between two nodes: cable tray, ladder, trunking, conduit, drag chain, buried or free run"),
        params: &[
            param("a", Kind::Str, NODE),
            param("b", Kind::Str, NODE),
            optional("kind", Kind::Str, tr_const!("tray (default), ladder, trunking, conduit, drag_chain, buried or free")),
            optional("width", Kind::Num, tr_const!("Inner width in mm (0 = no limit)")),
            optional("height", Kind::Num, tr_const!("Inner height in mm (0 = no limit)")),
            optional("classes", Kind::Str, tr_const!("Allowed classes, comma separated: power, control, signal, fluid (default all)")),
            optional("divider", Kind::Bool, tr_const!("Has a divider between power and signal cables")),
            optional("reserve", Kind::Num, tr_const!("Share of the capacity kept free in percent")),
            optional("part", Kind::Str, tr_const!("Part number of the carrier (per metre)")),
            optional("length", Kind::Num, tr_const!("Length in metres (default from the node positions)")),
            optional("bend_radius", Kind::Num, tr_const!("Bend radius of a drag chain in mm")),
            optional("name", Kind::Str, tr_const!("Name")),
        ],
        run: add_segment,
    },
    Spec {
        name: "field_place",
        help: tr_const!("Places an enclosure (+CAB1), a device (-M1) or a machine location (+M1) at a node of the field route network"),
        params: &[
            param("tag", Kind::Str, tr_const!("Location or device tag")),
            param("node", Kind::Str, NODE),
            optional("drop", Kind::Num, tr_const!("Length from the node to the device in metres")),
        ],
        run: place,
    },
    Spec {
        name: "field_add_outline",
        help: tr_const!("Adds the outline of a machine, building or area to the plant layout"),
        params: &[
            param("name", Kind::Str, tr_const!("Name, e.g. the location +M1")),
            param("points", Kind::Points, tr_const!("Corner points in metres")),
            optional("conductive", Kind::Bool, tr_const!("Conductive frame that needs equipotential bonding")),
        ],
        run: add_outline,
    },
    Spec {
        name: "field_route",
        help: tr_const!("Sets the field route definition of a cable: nodes it must pass and segments it must not use"),
        params: &[
            param("cable", Kind::Str, tr_const!("Cable tag")),
            optional("via", Kind::Str, tr_const!("Nodes to pass in this order, comma separated")),
            optional("avoid", Kind::Str, tr_const!("Segments not to use, comma separated")),
        ],
        run: route,
    },
    Spec {
        name: "field_add_bond",
        help: tr_const!("Adds an equipotential bonding conductor between two bonding points (main earthing terminal, enclosures, conductive outlines, metal carriers)"),
        params: &[
            param("a", Kind::Str, tr_const!("Bonding point")),
            param("b", Kind::Str, tr_const!("Bonding point")),
            optional("kind", Kind::Str, tr_const!("main (default), supplementary or functional")),
            optional("cross_section", Kind::Str, tr_const!("Cross-section in mm²")),
            optional("length", Kind::Num, tr_const!("Length in metres")),
            optional("part", Kind::Str, tr_const!("Part number of the conductor (per metre)")),
        ],
        run: add_bond,
    },
    Spec {
        name: "field_add_accessory",
        help: tr_const!("Adds an accessory to a cable end: gland, EMC gland, connector, strain relief, shield clamp, protective hose, end sleeve, marker"),
        params: &[
            param("cable", Kind::Str, tr_const!("Cable tag")),
            param("kind", Kind::Str, tr_const!("gland, emc_gland, connector, strain_relief, shield_clamp, protective_hose, end_sleeve, marker or other")),
            optional("part", Kind::Str, tr_const!("Part number")),
            optional("at", Kind::Str, tr_const!("Device tag of the end (both ends if missing)")),
            optional("count", Kind::Int, tr_const!("Quantity per end (default 1)")),
        ],
        run: add_accessory,
    },
    Spec {
        name: "field_remove",
        help: tr_const!("Removes a node (with its segments and places), segment, place, outline, bonding conductor or accessory"),
        params: &[
            param("what", Kind::Str, tr_const!("node, segment, place, outline, bond or accessory")),
            param("id", Kind::Str, tr_const!("Name or id; the tag for places")),
        ],
        run: remove,
    },
    Spec {
        name: "field_rules",
        help: tr_const!("Shows the rules of the field cable planning and changes those given"),
        params: &[
            optional("reserve", Kind::Num, tr_const!("Length added at each cable end in metres")),
            optional("cabinet", Kind::Num, tr_const!("Length inside an enclosure per end in metres")),
            optional("cut", Kind::Num, tr_const!("Cut lengths rounded up to this step in metres")),
            optional("fill", Kind::Num, tr_const!("Maximum fill of a carrier in percent")),
            optional("bend_factor", Kind::Num, tr_const!("Bend radius in drag chains as a multiple of the cable diameter")),
            optional("main_earth", Kind::Str, tr_const!("Name of the main earthing terminal")),
            optional("continuity", Kind::Bool, tr_const!("Metal carriers meeting at a node are connected")),
        ],
        run: rules,
    },
    Spec {
        name: "field_plan",
        help: tr_const!("Plans the field cables through the route network: routes, lengths, segment fill, bonding points and problems; optionally takes the cut lengths over"),
        params: &[optional("apply", Kind::Bool, tr_const!("Take the cut lengths over as cable lengths"))],
        run: plan,
    },
];

const NODE: &str = tr_const!("Node name or id");

/// Node by name, `N<id>` or id.
fn node_id(f: &Field, key: &str) -> Result<u64, String> {
    let key = key.trim();
    f.nodes
        .iter()
        .find(|n| n.name == key || format!("N{}", n.id) == key || n.id.to_string() == key)
        .map(|n| n.id)
        .ok_or_else(|| tr_fmt!("Unknown node: {}", key))
}

/// Segment by name, `S<id>` or id.
fn segment_id(f: &Field, key: &str) -> Result<u64, String> {
    let key = key.trim();
    f.segments
        .iter()
        .find(|s| f.segment_name(s.id) == key || s.id.to_string() == key)
        .map(|s| s.id)
        .ok_or_else(|| tr_fmt!("Unknown segment: {}", key))
}

fn list(a: &Args, name: &str) -> Vec<String> {
    a.str(name).map(|v| v.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()).unwrap_or_default()
}

fn add_node(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let id = prj.new_id();
    let name = a.str("name").map(String::from).unwrap_or_default();
    if !name.is_empty() && prj.field.nodes.iter().any(|n| n.name == name) {
        return Err(tr_fmt!("Node {} exists already", name));
    }
    let n = Node { id, name, x: a.num("x").unwrap_or(0.0), y: a.num("y").unwrap_or(0.0), z: a.num("z").unwrap_or(0.0) };
    let out = serde_json::to_value(&n).unwrap_or(Value::Null);
    prj.field.nodes.push(n);
    Ok(out)
}

fn add_segment(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let f = &prj.field;
    let (na, nb) = (node_id(f, a.req("a"))?, node_id(f, a.req("b"))?);
    if na == nb {
        return Err(tr_fmt!("A segment needs two different nodes"));
    }
    let kind = match a.str("kind") {
        Some(k) => SegmentKind::from_key(k).ok_or_else(|| tr_fmt!("Unknown segment kind: {}", k))?,
        None => SegmentKind::Tray,
    };
    let mut classes = 0;
    for c in list(a, "classes") {
        classes |= Class::from_key(&c).ok_or_else(|| tr_fmt!("Unknown class: {}", c))?.bit();
    }
    let id = prj.new_id();
    let seg = Segment {
        id,
        name: a.str("name").map(String::from).unwrap_or_default(),
        a: na,
        b: nb,
        kind,
        width: num_in(a, "width", 0.0, NO_MAX)?.unwrap_or(0.0),
        height: num_in(a, "height", 0.0, NO_MAX)?.unwrap_or(0.0),
        classes,
        divider: a.flag("divider"),
        reserve: num_in(a, "reserve", 0.0, 90.0)?.unwrap_or(0.0),
        part: a.str("part").map(String::from).unwrap_or_default(),
        length: num_in(a, "length", 0.0, NO_MAX)?.unwrap_or(0.0),
        bend_radius: num_in(a, "bend_radius", 0.0, NO_MAX)?.unwrap_or(0.0),
    };
    let mut out = serde_json::to_value(&seg).unwrap_or(Value::Null);
    out["length_m"] = json!(prj.field.segment_length(&seg));
    prj.field.segments.push(seg);
    Ok(out)
}

fn place(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let node = node_id(&prj.field, a.req("node"))?;
    let tag = a.req("tag").trim().to_string();
    prj.field.places.retain(|p| p.tag != tag);
    let p = Place { tag, node, drop: num_in(a, "drop", 0.0, NO_MAX)?.unwrap_or(0.0) };
    let out = serde_json::to_value(&p).unwrap_or(Value::Null);
    prj.field.places.push(p);
    Ok(out)
}

fn add_outline(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let points = a.points("points");
    if points.len() < 3 {
        return Err(tr_fmt!("An outline needs {} points at least", 3));
    }
    let id = prj.new_id();
    let o = Outline { id, name: a.req("name").into(), points, conductive: a.flag("conductive") };
    let out = serde_json::to_value(&o).unwrap_or(Value::Null);
    prj.field.outlines.push(o);
    Ok(out)
}

fn route(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let cable = a.req("cable").trim().to_string();
    if prj.cable(&cable).is_none() {
        return Err(tr_fmt!("Unknown cable: {}", cable));
    }
    let via = list(a, "via").iter().map(|n| node_id(&prj.field, n)).collect::<Result<Vec<_>, _>>()?;
    let avoid = list(a, "avoid").iter().map(|n| segment_id(&prj.field, n)).collect::<Result<Vec<_>, _>>()?;
    let def = prj.field.route_mut(&cable);
    def.via = via;
    def.avoid = avoid;
    let out = serde_json::to_value(&*def).unwrap_or(Value::Null);
    prj.field.routes.retain(|r| !r.is_empty());
    Ok(out)
}

fn add_bond(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let kind = match a.str("kind") {
        Some(k) => BondKind::from_key(k).ok_or_else(|| tr_fmt!("Unknown bonding kind: {}", k))?,
        None => BondKind::Main,
    };
    let known = field::bonding::names(prj);
    for end in [a.req("a"), a.req("b")] {
        if !known.iter().any(|(n, _)| n == end.trim()) {
            return Err(tr_fmt!("Unknown bonding point: {}", end));
        }
    }
    let id = prj.new_id();
    let b = Bond {
        id,
        a: a.req("a").trim().into(),
        b: a.req("b").trim().into(),
        kind,
        cross_section: a.str("cross_section").map(String::from).unwrap_or_default(),
        length: num_in(a, "length", 0.0, NO_MAX)?.unwrap_or(0.0),
        part: a.str("part").map(String::from).unwrap_or_default(),
    };
    let out = serde_json::to_value(&b).unwrap_or(Value::Null);
    prj.field.bonds.push(b);
    Ok(out)
}

fn add_accessory(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let cable = a.req("cable").trim().to_string();
    if prj.cable(&cable).is_none() {
        return Err(tr_fmt!("Unknown cable: {}", cable));
    }
    let kind = AccessoryKind::from_key(a.req("kind")).ok_or_else(|| tr_fmt!("Unknown accessory kind: {}", a.req("kind")))?;
    let id = prj.new_id();
    let acc = Accessory {
        id,
        cable,
        at: a.str("at").map(|t| t.trim().to_string()).unwrap_or_default(),
        kind,
        part: a.str("part").map(String::from).unwrap_or_default(),
        count: int_in(a, "count", 1, u32::MAX.into())?.unwrap_or(1) as u32,
    };
    let out = serde_json::to_value(&acc).unwrap_or(Value::Null);
    prj.field.accessories.push(acc);
    Ok(out)
}

fn remove(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let f = &mut prj.field;
    let key = a.req("id").trim();
    let id: u64 = key.parse().unwrap_or(0);
    let what = a.req("what").trim().to_ascii_lowercase();
    let done = match what.as_str() {
        "node" => {
            let n = node_id(f, key)?;
            f.remove_node(n)
        }
        "segment" => {
            let s = segment_id(f, key)?;
            f.remove_segment(s)
        }
        "place" => {
            let n = f.places.len();
            f.places.retain(|p| p.tag != key);
            f.places.len() != n
        }
        "outline" => {
            let n = f.outlines.len();
            f.outlines.retain(|o| o.id != id && o.name != key);
            f.outlines.len() != n
        }
        "bond" => {
            let n = f.bonds.len();
            f.bonds.retain(|b| b.id != id);
            f.bonds.len() != n
        }
        "accessory" => {
            let n = f.accessories.len();
            f.accessories.retain(|x| x.id != id);
            f.accessories.len() != n
        }
        _ => return Err(tr_fmt!("Unknown field object: {}", what)),
    };
    if !done {
        return Err(tr_fmt!("Nothing to remove: {} {}", what, key));
    }
    Ok(json!({ "removed": key }))
}

fn rules(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let r = &mut prj.field.rules;
    for (name, v) in [("reserve", &mut r.reserve), ("cabinet", &mut r.cabinet), ("cut", &mut r.cut), ("fill", &mut r.fill), ("bend_factor", &mut r.bend_factor)] {
        if let Some(x) = num_in(a, name, if name == "fill" { 1.0 } else { 0.0 }, if name == "fill" { 100.0 } else { NO_MAX })? {
            *v = x;
        }
    }
    if let Some(m) = a.str("main_earth").map(str::trim).filter(|m| !m.is_empty()) {
        r.main_earth = m.into();
    }
    if let Some(c) = a.value("continuity").and_then(Value::as_bool) {
        r.continuity = c;
    }
    Ok(serde_json::to_value(&*r).unwrap_or(Value::Null))
}

fn plan(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let an = Analysis::build(prj);
    let p = an.field;
    let applied = if a.flag("apply") { field::apply_lengths(prj, &p) } else { 0 };
    let f = &prj.field;
    let r1 = |v: f32| (v * 10.0).round() / 10.0;
    let cables: Vec<Value> = p
        .cables
        .iter()
        .map(|c| {
            json!({
                "cable": c.cable,
                "class": c.class.key(),
                "from": c.ends[0],
                "to": c.ends[1],
                "nodes": c.nodes.iter().map(|&n| f.node_name(n)).collect::<Vec<_>>(),
                "segments": c.segments.iter().map(|&s| f.segment_name(s)).collect::<Vec<_>>(),
                "network_m": r1(c.network),
                "length_m": r1(c.length),
                "cut_m": c.cut,
                "diameter_mm": r1(c.diameter),
                "shielded": c.shielded,
                "routed": c.routed(),
                "status": c.status_text(),
            })
        })
        .collect();
    let segments: Vec<Value> = p
        .segments
        .iter()
        .filter_map(|u| {
            let s = f.segment(u.segment)?;
            Some(json!({
                "segment": f.segment_name(s.id),
                "kind": s.kind.key(),
                "length_m": r1(f.segment_length(s)),
                "cables": u.cables,
                "area_mm2": u.area.round(),
                "fill_percent": u.fill(s).map(|x| x.round()),
                "classes": Class::ALL.iter().filter(|c| u.classes & c.bit() != 0).map(|c| c.key()).collect::<Vec<_>>(),
            }))
        })
        .collect();
    let bonding: Vec<Value> = field::bonding::points(prj).into_iter().map(|b| json!({ "point": b.name, "kind": b.kind.title(), "bonded": b.bonded })).collect();
    let material: Vec<Value> = field::material(prj).into_iter().map(|(part, qty, used)| json!({ "part": part, "quantity": qty, "used_by": used })).collect();
    let problems: Vec<String> = p.issues.iter().map(|i| i.message()).collect();
    Ok(json!({ "cables": cables, "segments": segments, "islands": p.islands, "bonding": bonding, "material": material, "problems": problems, "applied": applied }))
}

#[cfg(test)]
mod tests;
