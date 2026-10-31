//! Connection points of the devices in the panel layout: the connection points of a
//! device's parts (their own or generated ones) placed into the device box the same
//! way as its 3D model, and matched to the connection points of the schematic.

use super::{ItemKind, PanelItem};
use crate::model::{Project, SymbolInst};
use crate::parts::vec3::{add, norm, sub, V3};
use crate::parts::{Exit, Part};
use crate::symbols::Func;

/// A connection point of a placed device.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedPoint {
    /// Connection point designation (`A1`, `13`); terminals: their side (`1`, `2`).
    pub pin: String,
    /// Terminal number on terminal strips, otherwise empty.
    pub terminal: String,
    /// Position in the coordinates of the item's face (plate or door), z above it.
    pub pos: V3,
    pub exit: Exit,
    /// From the part's own data; false if generated from its function templates.
    pub exact: bool,
}

/// One part of a device and where it sits in depth (front mounted accessories are
/// stacked in front of the main part, like in [`super::device_size`]).
struct Layer<'a> {
    part: &'a Part,
    size: [f32; 3],
    z: f32,
}

fn same(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.5
}

/// Direction after the quarter turn of a rotated device (the part's top points to the
/// right, see [`super::place_mesh`]).
fn turned(exit: Exit) -> Exit {
    match exit {
        Exit::Up => Exit::Right,
        Exit::Right => Exit::Down,
        Exit::Down => Exit::Left,
        Exit::Left => Exit::Up,
        Exit::Front => Exit::Front,
    }
}

/// The direction a wire leaves a connection point in, in the coordinates of its face
/// (y down, z away from it).
pub fn exit_vector(exit: Exit) -> V3 {
    match exit {
        Exit::Up => [0.0, -1.0, 0.0],
        Exit::Down => [0.0, 1.0, 0.0],
        Exit::Left => [-1.0, 0.0, 0.0],
        Exit::Right => [1.0, 0.0, 0.0],
        Exit::Front => [0.0, 0.0, 1.0],
    }
}

/// The nozzle of the model of a fluid device for its connection point `pin`: its end
/// and the direction a pipe leaves it in, in the coordinates of the item's face.
pub fn nozzle(prj: &Project, it: &PanelItem, pin: &str) -> Option<(V3, V3)> {
    let part = super::part_by_number(prj, &it.part)?;
    let n = crate::parts::nozzles(part).into_iter().find(|n| n.pin == pin)?;
    let mesh = part.model_3d.as_deref()?;
    let (a, b) = (super::place_point(mesh, it, n.at), super::place_point(mesh, it, add(n.at, n.dir)));
    Some((a, norm(sub(b, a))))
}

/// The connection points of a device item, in the coordinates of its face. Terminal
/// strips get the points of their terminal part once per terminal, in strip order.
/// Accessories that are wider than the main part (side mounted) are not placed.
pub fn device_points(prj: &Project, it: &PanelItem) -> Vec<PlacedPoint> {
    if it.kind != ItemKind::Device {
        return Vec::new();
    }
    let Some(data) = prj.devices.get(&it.tag) else { return Vec::new() };
    let parts: Vec<(&Part, u32)> =
        data.parts.iter().filter_map(|r| prj.parts.iter().find(|p| p.number == r.number).map(|p| (p, r.count.max(1)))).collect();
    let Some((main, [mw, mh, md])) = parts.iter().find_map(|(p, _)| p.size().map(|s| (*p, s))) else { return Vec::new() };
    let mut layers = vec![Layer { part: main, size: [mw, mh, md], z: 0.0 }];
    let mut depth = md;
    for (p, count) in &parts {
        if let (true, Some(size)) = (p.number != main.number, p.size()) {
            if size[0] <= mw + 1.0 {
                layers.push(Layer { part: p, size, z: depth });
                depth += size[2] * *count as f32;
            }
        }
    }
    let strip = super::is_strip_part(main);
    if strip {
        depth = super::strip_depth(&parts);
    }
    // Rotated by a quarter turn: the part's height lies along x.
    let rotated = !same(it.h, mh) && same(it.w, mh);
    let along = if rotated { it.h } else { it.w };
    let sz = if depth > 0.0 { it.d / depth } else { 0.0 };
    // Places along the device: (start, width, terminal number, layers). Terminal
    // strips have one per terminal with its own part, in the widths of the strip
    // layout; other devices one for the whole device.
    let mut cells: Vec<(f32, f32, String, Vec<Layer>)> = Vec::new();
    let layout = if strip { crate::strips::strip_layout(prj, &it.tag) } else { Vec::new() };
    let total: f32 = layout.iter().map(|e| e.width).sum();
    if layout.is_empty() || total <= 0.0 {
        cells.push((0.0, along, String::new(), layers));
    } else {
        let mut at = 0.0;
        for e in &layout {
            let width = e.width * along / total;
            if let crate::strips::ElementKind::Terminal(label) = &e.kind {
                let part = prj.parts.iter().chain(crate::parts::catalog()).find(|p| p.number == e.part).unwrap_or(main);
                let size = part.size().unwrap_or([mw, mh, md]);
                cells.push((at, width, label.clone(), vec![Layer { part, size, z: 0.0 }]));
            }
            at += width;
        }
    }
    let mut out = Vec::new();
    for (start, cell, terminal, layers) in cells {
        let [cw, ch, _] = layers[0].size;
        for layer in &layers {
            let (points, exact) = layer.part.connection_points_or_default();
            for c in points {
                // Accessories are centred on the main part's front.
                let fx = c.pos[0] + (cw - layer.size[0]) / 2.0;
                let fy = c.pos[1] + (ch - layer.size[1]) / 2.0;
                let z = it.z + (layer.z + c.pos[2]) * sz;
                let (pos, exit) = if rotated {
                    ([it.x + (ch - fy) * (it.w / ch), it.y + start + fx * (cell / cw), z], turned(c.exit))
                } else {
                    ([it.x + start + fx * (cell / cw), it.y + fy * (it.h / ch), z], c.exit)
                };
                out.push(PlacedPoint { pin: c.pin, terminal: terminal.clone(), pos, exit, exact });
            }
        }
    }
    out
}

/// The placed point of connection point `pin` (index) of schematic symbol `s`:
/// terminals by terminal number and side (the studs of a busbar, such as the negative
/// busbar of a DC distributor, by their number), PLC points by their address, other
/// functions by designation (as in `netlist::pin_designation`).
pub fn point_of<'a>(prj: &Project, points: &'a [PlacedPoint], s: &SymbolInst, pin: usize) -> Option<&'a PlacedPoint> {
    let def = prj.symbol(&s.sym)?;
    if def.func == Func::Terminal {
        let side = (pin + 1).to_string();
        let stud = || points.iter().find(|p| p.terminal.is_empty() && p.pin == s.label);
        return points.iter().find(|p| p.terminal == s.label && p.pin == side).or_else(stud);
    }
    let name = match def.func {
        Func::Plc | Func::Plug(_) => s.label.as_str(),
        _ => s.pins.get(pin).map(String::as_str).or_else(|| def.pins.get(pin).map(|d| d.label))?,
    };
    points.iter().find(|p| p.terminal.is_empty() && p.pin == name)
}

#[cfg(test)]
mod tests;
