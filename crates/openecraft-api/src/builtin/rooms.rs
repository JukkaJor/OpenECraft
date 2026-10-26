//! Room commands: rooms in which cabinets stand, mounting rails and cable ducts on
//! their walls and ceiling, and devices mounted there – on a rail, or directly on the
//! wall when their part is made for it ([`openecraft_core::panel::rooms`]).

use crate::builtin::common::enclosure_index;
use crate::params::{optional, param, Args, Kind};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::geom::p;
use openecraft_core::model::Project;
use openecraft_core::panel::{self, Face, ItemKind, Opening, SitePlace};
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub struct Rooms;

impl Plugin for Rooms {
    fn info(&self) -> Info {
        info("rooms", tr_const!("Rooms"), tr_const!("Rooms with cabinets, rails and cable ducts on walls and ceiling, wall-mounted devices"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

pub static COMMANDS: &[Spec] = &[
    Spec {
        name: "list_rooms",
        help: tr_const!("Lists the rooms with their size, place, the cabinets standing in them and the items on their walls and ceiling"),
        params: &[optional("room", Kind::Str, ROOM)],
        run: list_rooms,
    },
    Spec {
        name: "add_room",
        help: tr_const!("Adds a room in which cabinets stand; its walls and ceiling carry rails, cable ducts and wall-mounted devices"),
        params: &[
            param("name", Kind::Str, ROOM),
            optional("width", Kind::Num, tr_const!("Width in mm")),
            optional("depth", Kind::Num, tr_const!("Depth in mm")),
            optional("height", Kind::Num, tr_const!("Height in mm")),
            optional("x", Kind::Num, tr_const!("Rear left corner in the plant, x in mm")),
            optional("z", Kind::Num, tr_const!("Rear left corner in the plant, z (towards the front) in mm")),
            optional("angle", Kind::Num, tr_const!("Turn about the vertical in degrees, counter-clockwise seen from above")),
        ],
        run: add_room,
    },
    Spec {
        name: "set_room",
        help: tr_const!("Changes the size or the place of a room; the cabinets in it move with it. Width or depth make it rectangular again"),
        params: &[
            param("room", Kind::Str, ROOM),
            optional("width", Kind::Num, tr_const!("Width in mm")),
            optional("depth", Kind::Num, tr_const!("Depth in mm")),
            optional("height", Kind::Num, tr_const!("Height in mm")),
            optional("x", Kind::Num, tr_const!("Rear left corner in the plant, x in mm")),
            optional("z", Kind::Num, tr_const!("Rear left corner in the plant, z (towards the front) in mm")),
            optional("angle", Kind::Num, tr_const!("Turn about the vertical in degrees, counter-clockwise seen from above")),
        ],
        run: set_room,
    },
    Spec {
        name: "set_room_outline",
        help: tr_const!("Gives a room an irregular floor plan: its corners in order, walls at right angles (L, U or T shaped rooms); wall n runs from corner n to the next one"),
        params: &[
            param("room", Kind::Str, ROOM),
            param("corners", Kind::Str, tr_const!("Corners as x,z pairs in mm separated by semicolons, e.g. 0,0; 6000,0; 6000,6000; 3000,6000; 3000,3000; 0,3000")),
        ],
        run: set_room_outline,
    },
    Spec {
        name: "set_room_wall",
        help: tr_const!("Moves a wall of a room outwards (negative: inwards) or gives it a length; the neighbouring walls follow"),
        params: &[
            param("room", Kind::Str, ROOM),
            param("wall", Kind::Int, tr_const!("Wall number (1 = from the first corner to the second)")),
            optional("offset", Kind::Num, tr_const!("Distance to move the wall outwards in mm")),
            optional("length", Kind::Num, tr_const!("New length of the wall in mm; its end moves")),
        ],
        run: set_room_wall,
    },
    Spec {
        name: "add_inner_wall",
        help: tr_const!("Adds a wall inside a room from x1 / z1 to x2 / z2 (its centre line, along x or z); both sides (inner1a, inner1b) carry rails, ducts, devices, doors and windows"),
        params: &[
            param("room", Kind::Str, ROOM),
            param("x1", Kind::Num, tr_const!("x in the floor plan in mm")),
            param("z1", Kind::Num, tr_const!("z in the floor plan (towards the front) in mm")),
            param("x2", Kind::Num, tr_const!("x in the floor plan in mm")),
            param("z2", Kind::Num, tr_const!("z in the floor plan (towards the front) in mm")),
            optional("thickness", Kind::Num, tr_const!("Thickness in mm")),
        ],
        run: add_inner_wall,
    },
    Spec {
        name: "set_inner_wall",
        help: tr_const!("Moves an inner wall across itself (towards side A), changes its length or thickness, or removes it with everything on it"),
        params: &[
            param("room", Kind::Str, ROOM),
            param("wall", Kind::Str, tr_const!("inner1, inner2, … (as listed by list_rooms)")),
            optional("offset", Kind::Num, tr_const!("Distance to move it towards side A in mm")),
            optional("length", Kind::Num, tr_const!("New length in mm; its end moves")),
            optional("thickness", Kind::Num, tr_const!("Thickness in mm")),
            optional("remove", Kind::Bool, tr_const!("Remove the inner wall")),
        ],
        run: set_inner_wall,
    },
    Spec {
        name: "place_in_room",
        help: tr_const!("Places the line-up of a cabinet in a room at a place of its floor plan (rear left corner of the line-up); without x and z at the rear of the room, without room out of its room"),
        params: &[
            param("enclosure", Kind::Str, tr_const!("Enclosure (location, e.g. +CAB1)")),
            optional("room", Kind::Str, tr_const!("Room; empty to take the cabinet out of its room")),
            optional("x", Kind::Num, tr_const!("x in the floor plan in mm")),
            optional("z", Kind::Num, tr_const!("z in the floor plan (towards the front) in mm")),
            optional("angle", Kind::Num, tr_const!("Turn about the vertical in degrees, counter-clockwise seen from above")),
        ],
        run: place_in_room,
    },
    Spec {
        name: "add_wall_profile",
        help: tr_const!("Adds a mounting rail or cable duct on a wall or the ceiling of a room, from x1 / y1 to x2 / y2 (walls seen from inside the room, the ceiling from above)"),
        params: &[
            param("room", Kind::Str, ROOM),
            param("kind", Kind::Str, tr_const!("rail or duct")),
            param("face", Kind::Str, FACE),
            param("x1", Kind::Num, tr_const!("Start, from the left edge of the face in mm")),
            param("y1", Kind::Num, tr_const!("Start, from the top edge of the face in mm")),
            param("x2", Kind::Num, tr_const!("End, from the left edge of the face in mm")),
            param("y2", Kind::Num, tr_const!("End, from the top edge of the face in mm")),
            optional("part", Kind::Str, tr_const!("Part number of the rail or duct")),
        ],
        run: add_wall_profile,
    },
    Spec {
        name: "add_wall_opening",
        help: tr_const!("Adds a door or a window to a wall of a room; a door stands on the floor and swings into the room, the check keeps its swing free of cabinets"),
        params: &[
            param("room", Kind::Str, ROOM),
            param("kind", Kind::Str, tr_const!("door or window")),
            param("wall", Kind::Str, tr_const!("wall1, wall2, …, inner1a, inner1b, … (as listed by list_rooms)")),
            optional("x", Kind::Num, tr_const!("Left edge from the left end of the wall (seen from inside) in mm; centred without")),
            optional("width", Kind::Num, tr_const!("Width in mm")),
            optional("height", Kind::Num, tr_const!("Height in mm")),
            optional("sill", Kind::Num, tr_const!("Windows: height of the sill above the floor in mm")),
        ],
        run: add_wall_opening,
    },
    Spec {
        name: "mount_on_wall",
        help: tr_const!("Mounts a device or a part on a wall or the ceiling of a room: on a rail there, or directly on the wall when its part is made for wall mounting"),
        params: &[
            param("room", Kind::Str, ROOM),
            param("device", Kind::Str, tr_const!("Device tag, or a part number for a part without device tag")),
            param("face", Kind::Str, FACE),
            param("x", Kind::Num, tr_const!("Left edge on the face in mm")),
            param("y", Kind::Num, tr_const!("Top edge on the face in mm")),
        ],
        run: mount_on_wall,
    },
];

const ROOM: &str = tr_const!("Room (location, e.g. =ROOM1)");

const FACE: &str = tr_const!("wall1, wall2, …, inner1a, inner1b, … (as listed by list_rooms) or ceiling");

fn room_index(prj: &Project, name: &str) -> Result<usize, String> {
    let i = enclosure_index(prj, name)?;
    if !prj.layout.enclosures[i].is_room() {
        return Err(tr_fmt!("{} is not a room", name));
    }
    Ok(i)
}

fn face_arg(prj: &Project, ri: usize, a: &Args, name: &str) -> Result<Face, String> {
    let key = a.req(name);
    panel::room_face_from_key(&prj.layout.enclosures[ri], key).ok_or_else(|| tr_fmt!("Unknown face: {}", key))
}

/// An inner wall by its key `inner1`, `inner2`, … (a side letter is ignored).
fn inner_arg(prj: &Project, ri: usize, a: &Args) -> Result<u64, String> {
    let key = a.req("wall");
    let e = &prj.layout.enclosures[ri];
    let k = key.trim().to_lowercase();
    let k = if k.ends_with(['a', 'b']) && k.len() > 5 { k[..k.len() - 1].to_string() } else { k };
    match panel::room_face_from_key(e, &format!("{k}a")) {
        Some(Face::Inner(id, _)) => Ok(id),
        _ => Err(tr_fmt!("Unknown face: {}", key)),
    }
}

fn place_arg(a: &Args, old: Option<SitePlace>) -> Option<SitePlace> {
    if a.num("x").is_none() && a.num("z").is_none() && a.num("angle").is_none() {
        return old;
    }
    let old = old.unwrap_or_default();
    Some(SitePlace { x: a.num("x").unwrap_or(old.x), z: a.num("z").unwrap_or(old.z), angle: a.num("angle").unwrap_or(old.angle) })
}

fn room_json(prj: &Project, ri: usize) -> Value {
    let e = &prj.layout.enclosures[ri];
    let poses = panel::site_poses(prj);
    let cabinets: Vec<Value> = panel::cabinets_in(&prj.layout, ri)
        .into_iter()
        .filter_map(|ci| {
            let place = panel::room_place(prj, ci)?;
            Some(json!({ "enclosure": prj.layout.enclosures[ci].name, "x": place.x, "z": place.z, "angle": place.angle }))
        })
        .collect();
    let walls: Vec<Value> = panel::walls(e)
        .iter()
        .enumerate()
        .map(|(i, w)| json!({ "wall": panel::room_face_key(e, panel::Face::Wall(i as u16)), "from": w.a, "to": w.b, "length": w.length() }))
        .collect();
    let inner: Vec<Value> = e
        .inner_walls
        .iter()
        .enumerate()
        .map(|(k, w)| json!({ "wall": format!("inner{}", k + 1), "from": w.a, "to": w.b, "thickness": w.thickness, "sides": [panel::room_face_key(e, Face::Inner(w.id, 0)), panel::room_face_key(e, Face::Inner(w.id, 1))] }))
        .collect();
    let faces: Vec<Value> = panel::room_faces(e)
        .into_iter()
        .filter_map(|f| {
            let items: Vec<Value> = e
                .items
                .iter()
                .filter(|i| i.face == f)
                .map(|i| {
                    let kind = if i.opening.is_cutout() { format!("{:?}", i.kind).to_lowercase() } else { i.opening.key().to_string() };
                    json!({ "id": i.id, "kind": kind, "tag": i.tag, "part": i.part, "x": i.x, "y": i.y, "w": i.w, "h": i.h, "z": i.z, "d": i.d, "wall_mounted": panel::wall_mount_ok(prj, e, i) })
                })
                .collect();
            (!items.is_empty()).then(|| json!({ "face": panel::room_face_key(e, f), "items": items }))
        })
        .collect();
    json!({
        "name": e.name,
        "size": { "width": e.width, "depth": e.depth, "height": e.height },
        "outline": panel::outline(e),
        "walls": walls,
        "inner_walls": inner,
        "at": poses[ri].at,
        "angle": poses[ri].angle.to_degrees(),
        "cabinets": cabinets,
        "faces": faces,
        "problems": panel::validate(prj, e).into_iter().map(|f| f.message).collect::<Vec<_>>(),
    })
}

fn list_rooms(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let list = match a.str("room") {
        Some(name) => vec![room_index(prj, name)?],
        None => panel::rooms(&prj.layout),
    };
    Ok(Value::Array(list.into_iter().map(|ri| room_json(prj, ri)).collect()))
}

fn resize(prj: &mut Project, ri: usize, a: &Args) {
    let e = &mut prj.layout.enclosures[ri];
    if a.num("width").is_some() || a.num("depth").is_some() {
        e.outline.clear();
    }
    for (name, field) in [("width", &mut e.width), ("depth", &mut e.depth), ("height", &mut e.height)] {
        if let Some(v) = a.num(name) {
            *field = v.max(500.0);
        }
    }
    e.site = place_arg(a, e.site);
    panel::apply_anchors(e);
}

fn add_room(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let name = a.req("name").to_string();
    if prj.layout.enclosures.iter().any(|e| e.name == name) {
        return Err(tr_fmt!("Enclosure {} exists already", name));
    }
    let room = panel::new_room(prj, &name, 4000.0, 3000.0, 2800.0);
    prj.layout.enclosures.push(room);
    let ri = prj.layout.enclosures.len() - 1;
    resize(prj, ri, a);
    Ok(room_json(prj, ri))
}

fn set_room(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let ri = room_index(prj, a.req("room"))?;
    resize(prj, ri, a);
    Ok(room_json(prj, ri))
}

fn set_room_outline(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let ri = room_index(prj, a.req("room"))?;
    let corners = parse_corners(a.req("corners"))?;
    panel::set_outline(&mut prj.layout.enclosures[ri], &corners)?;
    Ok(room_json(prj, ri))
}

/// `0,0; 6000,0; …` → corners.
fn parse_corners(text: &str) -> Result<Vec<[f32; 2]>, String> {
    text.split(';')
        .filter(|c| !c.trim().is_empty())
        .map(|c| {
            let (x, z) = c.split_once(',').ok_or_else(|| tr_fmt!("Not a corner: {}", c.trim()))?;
            match (x.trim().parse::<f32>(), z.trim().parse::<f32>()) {
                (Ok(x), Ok(z)) => Ok([x, z]),
                _ => Err(tr_fmt!("Not a corner: {}", c.trim())),
            }
        })
        .collect()
}

fn set_room_wall(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let ri = room_index(prj, a.req("room"))?;
    let wall = (a.int("wall").unwrap_or(0) - 1).max(0) as usize;
    let e = &mut prj.layout.enclosures[ri];
    let mut corners = panel::outline(e);
    if let Some(offset) = a.num("offset") {
        corners = panel::moved_wall(&corners, wall, offset)?;
    }
    if let Some(length) = a.num("length") {
        corners = panel::wall_length_set(&corners, wall, length)?;
    }
    panel::set_outline(e, &corners)?;
    Ok(room_json(prj, ri))
}

fn place_in_room(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let ei = enclosure_index(prj, a.req("enclosure"))?;
    let room = a.str("room").map(str::trim).filter(|r| !r.is_empty()).map(|r| room_index(prj, r)).transpose()?;
    panel::set_room(prj, ei, room, place_arg(a, None))?;
    match room {
        Some(ri) => Ok(room_json(prj, ri)),
        None => Ok(json!({ "enclosure": prj.layout.enclosures[ei].name, "room": Value::Null })),
    }
}

fn add_wall_profile(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let ri = room_index(prj, a.req("room"))?;
    let (kind, default) = match a.req("kind").trim().to_lowercase().as_str() {
        "rail" => (ItemKind::Rail, panel::RAIL_PART),
        "duct" => (ItemKind::Duct, panel::DUCT_PART),
        k => return Err(tr_fmt!("Unknown kind: {}", k)),
    };
    let face = face_arg(prj, ri, a, "face")?;
    let num = |n: &str| a.num(n).unwrap_or_default();
    let part = a.str("part").unwrap_or(default).to_string();
    let id = panel::add_wall_profile(prj, ri, kind, &part, face, p(num("x1"), num("y1")), p(num("x2"), num("y2")))?;
    Ok(json!({ "id": id, "room": room_json(prj, ri) }))
}

fn add_inner_wall(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let ri = room_index(prj, a.req("room"))?;
    let num = |n: &str| a.num(n).unwrap_or_default();
    let id = panel::add_inner_wall(prj, ri, [num("x1"), num("z1")], [num("x2"), num("z2")], a.num("thickness").unwrap_or(panel::INNER_WALL))?;
    Ok(json!({ "id": id, "room": room_json(prj, ri) }))
}

fn set_inner_wall(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let ri = room_index(prj, a.req("room"))?;
    let id = inner_arg(prj, ri, a)?;
    let e = &mut prj.layout.enclosures[ri];
    if a.value("remove").and_then(Value::as_bool).unwrap_or(false) {
        panel::remove_inner_wall(e, id);
    } else {
        let w = panel::moved_inner_wall(e, id, a.num("offset").unwrap_or(0.0), a.num("length"), a.num("thickness"))?;
        if let Some(old) = e.inner_walls.iter_mut().find(|x| x.id == id) {
            *old = w;
        }
    }
    Ok(room_json(prj, ri))
}

fn add_wall_opening(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let ri = room_index(prj, a.req("room"))?;
    let kind = a.req("kind");
    let opening = Opening::from_key(kind).filter(|o| !o.is_cutout()).ok_or_else(|| tr_fmt!("Unknown kind: {}", kind))?;
    let face = face_arg(prj, ri, a, "wall")?;
    let default = if opening == Opening::Door { panel::DOOR_SIZE } else { panel::WINDOW_SIZE };
    let size = (a.num("width").is_some() || a.num("height").is_some()).then(|| (a.num("width").unwrap_or(default.0).max(100.0), a.num("height").unwrap_or(default.1).max(100.0)));
    let id = panel::add_opening(prj, ri, opening, face, a.num("x"), size, a.num("sill"))?;
    Ok(json!({ "id": id, "room": room_json(prj, ri) }))
}

fn mount_on_wall(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let ri = room_index(prj, a.req("room"))?;
    let face = face_arg(prj, ri, a, "face")?;
    let at = p(a.num("x").unwrap_or_default(), a.num("y").unwrap_or_default());
    let id = panel::mount_on_wall(prj, ri, a.req("device"), face, at)?;
    let it = prj.layout.enclosures[ri].item(id).cloned().unwrap_or_default();
    Ok(json!({ "id": id, "x": it.x, "y": it.y, "z": it.z, "on_rail": it.z > 0.0 }))
}

#[cfg(test)]
mod tests;
