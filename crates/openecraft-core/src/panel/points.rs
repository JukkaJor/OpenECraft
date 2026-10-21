//! Connection points of the devices in the panel layout: the connection points of a
//! device's parts (their own or generated ones) placed into the device box the same
//! way as its 3D model, and matched to the connection points of the schematic.

use super::{ItemKind, PanelItem};
use crate::model::{Project, SymbolInst};
use crate::parts::vec3::V3;
use crate::parts::{Exit, FnKind, Part};
use crate::symbols::{self, Func};

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
    let strip = !main.functions.is_empty() && main.functions.iter().all(|f| f.kind == FnKind::Terminal);
    let terminals: Vec<String> = if strip { crate::strips::terminals(prj, &it.tag).into_iter().map(|t| t.label).collect() } else { Vec::new() };
    // Rotated by a quarter turn: the part's height lies along x.
    let rotated = !same(it.h, mh) && same(it.w, mh);
    let n = terminals.len().max(1);
    let cell = (if rotated { it.h } else { it.w }) / n as f32;
    let sz = if depth > 0.0 { it.d / depth } else { 0.0 };
    let mut out = Vec::new();
    for k in 0..n {
        let terminal = terminals.get(k).cloned().unwrap_or_default();
        for layer in &layers {
            let (points, exact) = layer.part.connection_points_or_default();
            for c in points {
                // Accessories are centred on the main part's front.
                let fx = c.pos[0] + (mw - layer.size[0]) / 2.0;
                let fy = c.pos[1] + (mh - layer.size[1]) / 2.0;
                let z = it.z + (layer.z + c.pos[2]) * sz;
                let (pos, exit) = if rotated {
                    ([it.x + (mh - fy) * (it.w / mh), it.y + k as f32 * cell + fx * (cell / mw), z], turned(c.exit))
                } else {
                    ([it.x + k as f32 * cell + fx * (cell / mw), it.y + fy * (it.h / mh), z], c.exit)
                };
                out.push(PlacedPoint { pin: c.pin, terminal: terminal.clone(), pos, exit, exact });
            }
        }
    }
    out
}

/// The placed point of connection point `pin` (index) of schematic symbol `s`:
/// terminals by terminal number and side, other functions by designation.
pub fn point_of<'a>(points: &'a [PlacedPoint], s: &SymbolInst, pin: usize) -> Option<&'a PlacedPoint> {
    let def = symbols::get(&s.sym)?;
    if def.func == Func::Terminal {
        let side = (pin + 1).to_string();
        return points.iter().find(|p| p.terminal == s.label && p.pin == side);
    }
    let name = s.pins.get(pin).map(String::as_str).or_else(|| def.pins.get(pin).map(|d| d.label))?;
    points.iter().find(|p| p.terminal.is_empty() && p.pin == name)
}

#[cfg(test)]
mod tests;
