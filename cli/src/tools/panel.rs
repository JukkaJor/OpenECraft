//! Panel layout tools: enclosure systems and their components, accessories on the
//! plate, the door and the walls, fastening, parametric placement (anchors, patterns,
//! aligning, rails), assemblies and 3D macros.

use super::common::enclosure_index;
use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::geom::p;
use openecraft_core::model::Project;
use openecraft_core::panel::{self, Align, AlignTo, Anchor, Enclosure, Face, Fastening, ItemGroup, PanelItem};
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

const ENCLOSURE: &str = tr_const!("Enclosure (location, e.g. +CAB1)");
const IDS: &str = tr_const!("Item ids");
const FACE: &str = tr_const!("plate, door, left, right, rear, roof or bottom");
const ANCHOR: &str = tr_const!("Anchor along the axis: free, start:<mm>, center:<mm>, end:<mm> or stretch:<mm>,<mm>");

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "enclosure_systems",
        help: tr_const!("Lists the enclosure systems with their standard sizes, wall thickness, plinth and whether they can be bayed"),
        params: &[],
        needs_project: false,
        run: enclosure_systems,
    },
    Tool {
        name: "list_enclosures",
        help: tr_const!("Lists the enclosures with their system, components (assembly), items per face and assemblies"),
        params: &[optional("enclosure", Kind::Str, ENCLOSURE)],
        needs_project: true,
        run: list_enclosures,
    },
    Tool {
        name: "add_enclosure",
        help: tr_const!("Adds an enclosure: of a system in one of its sizes (or a custom size), or from an enclosure part"),
        params: &[
            param("name", Kind::Str, ENCLOSURE),
            optional("system", Kind::Str, tr_const!("compact, baying, terminal_box or plastic")),
            optional("part", Kind::Str, tr_const!("Enclosure part number")),
            optional("width", Kind::Num, tr_const!("Width in mm")),
            optional("height", Kind::Num, tr_const!("Height in mm")),
            optional("depth", Kind::Num, tr_const!("Depth in mm")),
        ],
        needs_project: true,
        run: add_enclosure,
    },
    Tool {
        name: "set_enclosure",
        help: tr_const!("Changes the system, size and components of an enclosure (plinth, bayed sides, gland plate); anchored items follow"),
        params: &[
            param("enclosure", Kind::Str, ENCLOSURE),
            optional("system", Kind::Str, tr_const!("compact, baying, terminal_box or plastic; empty for none")),
            optional("width", Kind::Num, tr_const!("Width in mm")),
            optional("height", Kind::Num, tr_const!("Height in mm")),
            optional("depth", Kind::Num, tr_const!("Depth in mm")),
            optional("plinth", Kind::Num, tr_const!("Plinth height in mm, 0 without plinth")),
            optional("left_open", Kind::Bool, tr_const!("Left side bayed (no side panel)")),
            optional("right_open", Kind::Bool, tr_const!("Right side bayed (no side panel)")),
            optional("gland_plate", Kind::Bool, tr_const!("Bottom with removable gland plate")),
        ],
        needs_project: true,
        run: set_enclosure,
    },
    Tool {
        name: "bay_enclosure",
        help: tr_const!("Bays an enclosure side by side to the right of another one (line-up); without right_of it leaves its line-up"),
        params: &[
            param("enclosure", Kind::Str, ENCLOSURE),
            optional("right_of", Kind::Str, tr_const!("Enclosure it stands right of; empty to take it out of its line-up")),
        ],
        needs_project: true,
        run: bay_enclosure,
    },
    Tool {
        name: "add_cable_passage",
        help: tr_const!("Adds a cable passage to the right neighbour: round cutouts facing each other in both side walls; wires between the enclosures run through it"),
        params: &[
            param("enclosure", Kind::Str, ENCLOSURE),
            optional("y", Kind::Num, tr_const!("Centre from the top of the enclosure in mm")),
            optional("z", Kind::Num, tr_const!("Centre from the rear wall in mm")),
            optional("diameter", Kind::Num, tr_const!("Diameter in mm")),
        ],
        needs_project: true,
        run: add_cable_passage,
    },
    Tool {
        name: "place_accessory",
        help: tr_const!("Places an enclosure accessory (fan, filter, cable gland, light, handle, PE bar) on a face"),
        params: &[
            param("enclosure", Kind::Str, ENCLOSURE),
            param("part", Kind::Str, tr_const!("Accessory part number, e.g. GEN.GLAND-M25")),
            param("face", Kind::Str, FACE),
            optional("x", Kind::Num, tr_const!("Left edge on the face in mm")),
            optional("y", Kind::Num, tr_const!("Top edge on the face in mm")),
            optional("anchor_x", Kind::Str, ANCHOR),
            optional("anchor_y", Kind::Str, ANCHOR),
            optional("fastening", Kind::Str, tr_const!("auto, tapped, through, rivet_nut, stud, clip or adhesive")),
        ],
        needs_project: true,
        run: place_accessory,
    },
    Tool {
        name: "set_panel_item",
        help: tr_const!("Changes position, face, fastening or anchors of a panel item"),
        params: &[
            param("enclosure", Kind::Str, ENCLOSURE),
            param("id", Kind::Int, tr_const!("Item id")),
            optional("x", Kind::Num, tr_const!("Left edge on the face in mm")),
            optional("y", Kind::Num, tr_const!("Top edge on the face in mm")),
            optional("z", Kind::Num, tr_const!("Distance from the face in mm (rails: standoff height)")),
            optional("face", Kind::Str, FACE),
            optional("anchor_x", Kind::Str, ANCHOR),
            optional("anchor_y", Kind::Str, ANCHOR),
            optional("fastening", Kind::Str, tr_const!("auto, tapped, through, rivet_nut, stud, clip or adhesive")),
        ],
        needs_project: true,
        run: set_panel_item,
    },
    Tool {
        name: "arrange_panel_items",
        help: tr_const!("Aligns items, spaces them evenly, lines devices up on a rail or copies items in a pattern"),
        params: &[
            param("enclosure", Kind::Str, ENCLOSURE),
            param("ids", Kind::Ids, IDS),
            param("action", Kind::Str, tr_const!("left, center_x, right, top, center_y, bottom, distribute_x, distribute_y, rail or pattern")),
            optional("rail", Kind::Int, tr_const!("Rail id (action rail)")),
            optional("gap", Kind::Num, tr_const!("Gap between the devices on the rail in mm")),
            optional("columns", Kind::Int, tr_const!("Columns of the pattern")),
            optional("rows", Kind::Int, tr_const!("Rows of the pattern")),
            optional("pitch_x", Kind::Num, tr_const!("Horizontal pitch of the pattern in mm")),
            optional("pitch_y", Kind::Num, tr_const!("Vertical pitch of the pattern in mm")),
        ],
        needs_project: true,
        run: arrange_panel_items,
    },
    Tool {
        name: "group_panel_items",
        help: tr_const!("Groups items into an assembly that is selected and moved together; without a name the items leave their assemblies"),
        params: &[param("enclosure", Kind::Str, ENCLOSURE), param("ids", Kind::Ids, IDS), optional("name", Kind::Str, tr_const!("Name of the assembly"))],
        needs_project: true,
        run: group_panel_items,
    },
    Tool {
        name: "list_panel_macros",
        help: tr_const!("Lists the 3D macros: the project's own and the built-in ones"),
        params: &[],
        needs_project: true,
        run: list_panel_macros,
    },
    Tool {
        name: "save_panel_macro",
        help: tr_const!("Saves items of an enclosure as a 3D macro of the project"),
        params: &[param("enclosure", Kind::Str, ENCLOSURE), param("ids", Kind::Ids, IDS), param("name", Kind::Str, tr_const!("Macro name"))],
        needs_project: true,
        run: save_panel_macro,
    },
    Tool {
        name: "insert_panel_macro",
        help: tr_const!("Inserts a 3D macro as an assembly: anchored items at their place, the others relative to x / y on the face"),
        params: &[
            param("enclosure", Kind::Str, ENCLOSURE),
            param("macro", Kind::Str, tr_const!("Macro name")),
            optional("face", Kind::Str, FACE),
            optional("x", Kind::Num, tr_const!("Left edge on the face in mm")),
            optional("y", Kind::Num, tr_const!("Top edge on the face in mm")),
        ],
        needs_project: true,
        run: insert_panel_macro,
    },
];

fn face_arg(a: &Args, name: &str) -> Result<Option<Face>, String> {
    a.str(name).map(|f| Face::from_key(f).ok_or_else(|| tr_fmt!("Unknown face: {}", f))).transpose()
}

fn align_arg(a: &Args, name: &str) -> Result<Option<Align>, String> {
    a.str(name).map(|v| Align::parse(v).ok_or_else(|| tr_fmt!("Unknown anchor: {}", v))).transpose()
}

fn fastening_arg(a: &Args) -> Result<Option<Fastening>, String> {
    a.str("fastening").map(|v| Fastening::from_key(v).ok_or_else(|| tr_fmt!("Unknown fastening: {}", v))).transpose()
}

fn bool_arg(a: &Args, name: &str) -> Option<bool> {
    a.value(name).and_then(Value::as_bool)
}

fn item_json(prj: &Project, it: &PanelItem) -> Value {
    let part = prj.parts.iter().chain(openecraft_core::parts::catalog()).find(|p| p.number == it.part);
    json!({
        "id": it.id,
        "kind": format!("{:?}", it.kind).to_lowercase(),
        "tag": it.tag,
        "part": it.part,
        "face": it.face.key(),
        "x": it.x, "y": it.y, "w": it.w, "h": it.h, "z": it.z, "d": it.d,
        "fastening": Fastening::resolve(it, part).key(),
        "anchor": [it.anchor.x.text(), it.anchor.y.text()],
        "group": it.group,
    })
}

/// Line-up of an enclosure: neighbours and the cable passages to the right one.
fn lineup_json(prj: &Project, ei: usize) -> Value {
    let (left, right) = panel::neighbours(&prj.layout, ei);
    let name = |i: Option<usize>| i.map(|i| prj.layout.enclosures[i].name.clone());
    let pos = panel::positions(&prj.layout);
    let passages: Vec<Value> = panel::passages(&prj.layout, &pos, ei)
        .into_iter()
        .map(|x| json!({ "y": x.at[1] - pos[ei][1], "z": x.at[2] - pos[ei][2], "cutouts": [x.cutouts.0, x.cutouts.1] }))
        .collect();
    json!({ "right_of": name(left), "left_of": name(right), "position": pos[ei], "passages_right": passages })
}

fn enclosure_json(prj: &Project, e: &Enclosure) -> Value {
    let components: Vec<Value> = panel::components(e).into_iter().map(|c| json!({ "role": c.role.title(), "part": c.part, "count": c.count })).collect();
    let faces: Vec<Value> = Face::ALL
        .into_iter()
        .filter_map(|f| {
            let items: Vec<Value> = e.items.iter().filter(|i| i.face == f).map(|i| item_json(prj, i)).collect();
            (!items.is_empty()).then(|| json!({ "face": f.key(), "items": items }))
        })
        .collect();
    let ei = prj.layout.enclosures.iter().position(|x| std::ptr::eq(x, e)).unwrap_or(0);
    json!({
        "name": e.name,
        "lineup": lineup_json(prj, ei),
        "system": e.build.system,
        "part": e.part,
        "size": [e.width, e.height, e.depth],
        "plinth": e.build.plinth,
        "bayed": [e.build.left_open, e.build.right_open],
        "gland_plate": e.build.gland_plate,
        "components": components,
        "faces": faces,
        "assemblies": e.groups.iter().map(|g| json!({ "id": g.id, "name": g.name, "items": panel::group_members(e, g.id) })).collect::<Vec<_>>(),
        "problems": panel::problem_items(prj, e).into_iter().collect::<Vec<_>>(),
    })
}

fn enclosure_systems(_: &mut Session, _: &Args) -> Result<Value, String> {
    Ok(Value::Array(
        panel::SYSTEMS
            .iter()
            .map(|s| {
                json!({
                    "system": s.id(),
                    "title": s.title(),
                    "sizes": s.data.sizes.iter().map(|x| [x.0, x.1, x.2]).collect::<Vec<_>>(),
                    "wall_mm": s.data.wall,
                    "flange_mm": s.data.flange,
                    "plinth_mm": s.data.plinth,
                    "bayable": s.data.bayable,
                    "protection": s.data.ip,
                })
            })
            .collect(),
    ))
}

fn list_enclosures(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let list = match a.str("enclosure") {
        Some(name) => vec![&prj.layout.enclosures[enclosure_index(prj, name)?]],
        None => prj.layout.enclosures.iter().collect(),
    };
    Ok(Value::Array(list.into_iter().map(|e| enclosure_json(prj, e)).collect()))
}

fn add_enclosure(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let name = a.req("name").to_string();
    if prj.layout.enclosures.iter().any(|e| e.name == name) {
        return Err(tr_fmt!("Enclosure {} exists already", name));
    }
    let mut e = panel::new_enclosure(prj, a.str("part").unwrap_or(""), &name);
    if let Some(system) = a.str("system") {
        if panel::SYSTEMS.iter().all(|x| x.id() != system) {
            return Err(tr_fmt!("Unknown enclosure system: {}", system));
        }
        let size = e.system().filter(|x| x.id() == system).map(|_| (e.width, e.height, e.depth));
        let size = size.or_else(|| panel::SYSTEMS.iter().find(|x| x.id() == system).and_then(|x| x.data.sizes.first().copied()));
        panel::apply_system(&mut e, system, size);
    }
    resize(&mut e, a);
    prj.layout.enclosures.push(e);
    Ok(enclosure_json(prj, prj.layout.enclosures.last().unwrap()))
}

fn resize(e: &mut Enclosure, a: &Args) {
    for (name, field) in [("width", &mut e.width), ("height", &mut e.height), ("depth", &mut e.depth)] {
        if let Some(v) = a.num(name) {
            *field = v.max(50.0);
        }
    }
    panel::sync_part(e);
    panel::apply_anchors(e);
}

fn set_enclosure(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let ei = enclosure_index(prj, a.req("enclosure"))?;
    let e = &mut prj.layout.enclosures[ei];
    if let Some(system) = a.str("system") {
        if !system.is_empty() && panel::SYSTEMS.iter().all(|x| x.id() != system) {
            return Err(tr_fmt!("Unknown enclosure system: {}", system));
        }
        panel::apply_system(e, system, None);
    }
    if let Some(v) = a.num("plinth") {
        e.build.plinth = v.max(0.0);
    }
    for (name, field) in [("left_open", &mut e.build.left_open), ("right_open", &mut e.build.right_open), ("gland_plate", &mut e.build.gland_plate)] {
        if let Some(v) = bool_arg(a, name) {
            *field = v;
        }
    }
    resize(e, a);
    Ok(enclosure_json(prj, &prj.layout.enclosures[ei]))
}

fn bay_enclosure(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let ei = enclosure_index(prj, a.req("enclosure"))?;
    match a.str("right_of").filter(|n| !n.trim().is_empty()) {
        Some(left) => {
            let li = enclosure_index(prj, left.trim())?;
            if !panel::bay_right_of(prj, ei, li) {
                return Err(tr_fmt!("{} cannot be bayed to itself", prj.layout.enclosures[ei].name));
            }
        }
        None => panel::unbay(prj, ei),
    }
    let lineup: Vec<&str> = panel::lineup_of(&prj.layout, ei).into_iter().map(|i| prj.layout.enclosures[i].name.as_str()).collect();
    Ok(json!({ "lineup": lineup }))
}

fn add_cable_passage(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let ei = enclosure_index(prj, a.req("enclosure"))?;
    let Some(right) = panel::neighbours(&prj.layout, ei).1 else {
        return Err(tr_fmt!("{} has no enclosure bayed to its right side", prj.layout.enclosures[ei].name));
    };
    let (y, z) = panel::next_passage_place(&prj.layout, ei);
    let (y, z) = (a.num("y").unwrap_or(y), a.num("z").unwrap_or(z));
    let ids = panel::add_passage(prj, ei, y, z, a.num("diameter").unwrap_or(panel::PASSAGE_DIAMETER).max(1.0));
    let (left_name, right_name) = (prj.layout.enclosures[ei].name.clone(), prj.layout.enclosures[right].name.clone());
    Ok(json!({ "cutouts": ids, "from": left_name, "to": right_name, "lineup": lineup_json(prj, ei) }))
}

fn place_accessory(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let ei = enclosure_index(prj, a.req("enclosure"))?;
    let face = face_arg(a, "face")?.unwrap_or_default();
    let part = a.req("part").to_string();
    let known = prj.parts.iter().chain(openecraft_core::parts::catalog()).find(|p| p.number == part).cloned();
    let pd = known.ok_or_else(|| tr_fmt!("Unknown part: {}", part))?;
    if !panel::allowed_faces(&pd).contains(&face) {
        return Err(tr_fmt!("{} cannot be mounted on the {}", part, face.title()));
    }
    let mut it = panel::new_accessory(prj, &part, face, p(a.num("x").unwrap_or(0.0), a.num("y").unwrap_or(0.0)));
    it.fastening = fastening_arg(a)?.unwrap_or_default();
    it.anchor = Anchor { x: align_arg(a, "anchor_x")?.unwrap_or_default(), y: align_arg(a, "anchor_y")?.unwrap_or_default() };
    let e = &mut prj.layout.enclosures[ei];
    panel::apply_one_anchor(e, &mut it);
    let id = it.id;
    e.items.push(it);
    Ok(item_json(prj, prj.layout.enclosures[ei].item(id).unwrap()))
}

fn set_panel_item(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let ei = enclosure_index(prj, a.req("enclosure"))?;
    let id = a.int("id").unwrap_or(0) as u64;
    let (face, fastening, ax, ay) = (face_arg(a, "face")?, fastening_arg(a)?, align_arg(a, "anchor_x")?, align_arg(a, "anchor_y")?);
    let e = &mut prj.layout.enclosures[ei];
    let idx = e.items.iter().position(|i| i.id == id).ok_or_else(|| tr_fmt!("No item {}", id))?;
    let mut it = e.items[idx].clone();
    let moved = a.num("x").is_some() || a.num("y").is_some();
    for (name, field) in [("x", &mut it.x), ("y", &mut it.y), ("z", &mut it.z)] {
        if let Some(v) = a.num(name) {
            *field = v;
        }
    }
    if let Some(f) = face {
        it.face = f;
    }
    if let Some(f) = fastening {
        it.fastening = f;
    }
    if moved && ax.is_none() && ay.is_none() {
        it.anchor = it.anchor.capture(&it, e.face_size(it.face));
    }
    it.anchor = Anchor { x: ax.unwrap_or(it.anchor.x), y: ay.unwrap_or(it.anchor.y) };
    panel::apply_one_anchor(e, &mut it);
    panel::snap_to_rail(e, &mut it);
    e.items[idx] = it;
    Ok(item_json(prj, &prj.layout.enclosures[ei].items[idx]))
}

fn arrange_panel_items(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let ei = enclosure_index(prj, a.req("enclosure"))?;
    let ids = a.ids("ids");
    let action = a.req("action").to_string();
    let mut created = Vec::new();
    let e = &mut prj.layout.enclosures[ei];
    match action.as_str() {
        "distribute_x" => panel::distribute(e, &ids, true),
        "distribute_y" => panel::distribute(e, &ids, false),
        "rail" => {
            let rail = a.int("rail").unwrap_or(0) as u64;
            if e.item(rail).is_none() {
                return Err(tr_fmt!("No item {}", rail));
            }
            if !panel::arrange_on_rail(e, rail, &ids, a.num("gap").unwrap_or(0.0)) {
                return Err(tr_fmt!("The devices do not fit onto rail {}", rail));
            }
        }
        "pattern" => {
            let (nx, ny) = (a.int("columns").unwrap_or(1).max(1) as usize, a.int("rows").unwrap_or(1).max(1) as usize);
            created = panel::pattern_copy(prj, ei, &ids, nx, ny, a.num("pitch_x").unwrap_or(0.0), a.num("pitch_y").unwrap_or(0.0));
        }
        other => {
            let to = AlignTo::ALL.into_iter().find(|t| t.key() == other).ok_or_else(|| tr_fmt!("Unknown action: {}", other))?;
            panel::align(e, &ids, to);
        }
    }
    let e = &prj.layout.enclosures[ei];
    let items: Vec<Value> = e.items.iter().filter(|i| ids.contains(&i.id) || created.contains(&i.id)).map(|i| item_json(prj, i)).collect();
    Ok(json!({ "items": items, "created": created }))
}

fn group_panel_items(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let ei = enclosure_index(prj, a.req("enclosure"))?;
    let ids = a.ids("ids");
    let group = match a.str("name").filter(|n| !n.trim().is_empty()) {
        Some(name) => {
            let id = prj.new_id();
            prj.layout.enclosures[ei].groups.push(ItemGroup { id, name: name.trim().into(), source: String::new() });
            id
        }
        None => 0,
    };
    let e = &mut prj.layout.enclosures[ei];
    for it in e.items.iter_mut().filter(|i| ids.contains(&i.id)) {
        it.group = group;
    }
    let used: Vec<u64> = e.items.iter().map(|i| i.group).collect();
    e.groups.retain(|g| used.contains(&g.id));
    Ok(json!({ "group": group, "items": panel::group_members(e, group) }))
}

fn list_panel_macros(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    Ok(Value::Array(
        panel::all_macros(prj)
            .into_iter()
            .map(|m| {
                let faces: Vec<&str> = Face::ALL.into_iter().filter(|f| m.items.iter().any(|i| i.face == *f)).map(|f| f.key()).collect();
                json!({ "name": m.name, "title": m.title(), "built_in": m.is_builtin(), "description": m.description, "face": m.face.key(), "faces": faces, "items": m.items.len() })
            })
            .collect(),
    ))
}

fn save_panel_macro(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let ei = enclosure_index(prj, a.req("enclosure"))?;
    let name = a.req("name").trim().to_string();
    let m = panel::macro_from_items(&prj.layout.enclosures[ei], &a.ids("ids"), &name).ok_or_else(|| tr_fmt!("No items to save in {}", name))?;
    let n = m.items.len();
    prj.layout.macros.retain(|x| x.name != name);
    prj.layout.macros.push(m);
    Ok(json!({ "name": name, "items": n }))
}

fn insert_panel_macro(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let ei = enclosure_index(prj, a.req("enclosure"))?;
    let name = a.req("macro");
    let m = panel::all_macros(prj).into_iter().find(|m| m.name == name || m.title() == name).ok_or_else(|| tr_fmt!("No macro named {}", name))?;
    let face = face_arg(a, "face")?.unwrap_or(m.face);
    let ids = panel::insert_macro(prj, ei, &m, face, p(a.num("x").unwrap_or(0.0), a.num("y").unwrap_or(0.0)));
    let e = &prj.layout.enclosures[ei];
    Ok(json!({ "items": ids.iter().filter_map(|id| e.item(*id)).map(|i| item_json(prj, i)).collect::<Vec<_>>() }))
}

#[cfg(test)]
mod tests;
