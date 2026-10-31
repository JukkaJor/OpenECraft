//! How the parts of a device are mounted together: the main part, accessories
//! snapped onto its front (auxiliary contact blocks, stacked in depth) and accessories
//! beside it on the rail (auxiliary and signal contacts of breakers). The footprint,
//! the connection points, the 3D model and the 2D front view of a device item all
//! follow it.

use super::{Face, ItemKind, PanelItem};
use crate::model::Project;
use crate::parts::{AccessoryMount, Part};

/// One part of an assembly and its box in the device, unscaled (mm): `at` along the
/// rail from the left end, `z` above the plate.
#[derive(Clone, Copy, Debug)]
pub struct Piece<'a> {
    pub part: &'a Part,
    pub size: [f32; 3],
    pub at: f32,
    pub z: f32,
}

#[derive(Clone, Debug)]
pub struct Assembly<'a> {
    pub main: Piece<'a>,
    /// Accessories on the front of the main part, centred on it, back to front.
    pub front: Vec<Piece<'a>>,
    /// Accessories beside the main part, left to right.
    pub sides: Vec<Piece<'a>>,
    /// Length along the rail and depth of the whole device.
    pub w: f32,
    pub d: f32,
}

impl<'a> Assembly<'a> {
    /// Only the main part: nothing mounted on it.
    pub fn is_plain(&self) -> bool {
        self.front.is_empty() && self.sides.is_empty()
    }

    /// All pieces: the main part first, then the front and the side accessories.
    pub fn pieces(&self) -> impl Iterator<Item = &Piece<'a>> {
        std::iter::once(&self.main).chain(&self.front).chain(&self.sides)
    }

    /// The box of a piece placed into the device item `it` (scaled to the item, turned
    /// with it), as an item of its own.
    pub fn piece_item(&self, it: &PanelItem, piece: &Piece) -> PanelItem {
        let rotated = super::item_rotated(it, self.main.size[1]);
        let (along, across) = if rotated { (it.h, it.w) } else { (it.w, it.h) };
        let sa = along / self.w.max(1e-3);
        let sc = across / self.main.size[1].max(1e-3);
        let sz = it.d / self.d.max(1e-3);
        // Centred across the rail.
        let (a0, len) = (piece.at * sa, piece.size[0] * sa);
        let cross = piece.size[1] * sc;
        let c0 = (across - cross) / 2.0;
        let mut sub = it.clone();
        if rotated {
            (sub.x, sub.w, sub.y, sub.h) = (it.x + c0, cross, it.y + a0, len);
        } else {
            (sub.x, sub.w, sub.y, sub.h) = (it.x + a0, len, it.y + c0, cross);
        }
        (sub.z, sub.d) = (it.z + piece.z * sz, piece.size[2] * sz);
        sub.part = piece.part.number.clone();
        sub
    }
}

/// Where the parts of a device sit: the main part is the first part with dimensions
/// that is no accessory (otherwise the first part with dimensions). Accessories
/// mounted in front ([`AccessoryMount::Front`], or [`AccessoryMount::Auto`] when they
/// are not wider than the main part) are stacked in front of it as often as the device
/// uses them, accessories mounted left or right stand beside it. Terminal strips and
/// devices without parts with dimensions have none.
pub fn assembly<'a>(prj: &'a Project, tag: &str) -> Option<Assembly<'a>> {
    let data = prj.devices.get(tag)?;
    let parts: Vec<(&Part, [f32; 3], u32)> =
        data.parts.iter().filter_map(|r| prj.parts.iter().find(|x| x.number == r.number).and_then(|p| p.size().map(|s| (p, s, r.count.max(1))))).collect();
    let (main, size, _) = *parts.iter().find(|(p, _, _)| !p.accessory).or(parts.first())?;
    if super::is_strip_part(main) {
        return None;
    }
    let mut a = Assembly { main: Piece { part: main, size, at: 0.0, z: 0.0 }, front: Vec::new(), sides: Vec::new(), w: size[0], d: size[2] };
    let (mut left, mut right) = (Vec::new(), Vec::new());
    for (p, s, count) in parts.iter().filter(|(p, _, _)| p.number != main.number) {
        let mount = match p.accessory_mount {
            AccessoryMount::Auto if s[0] <= size[0] + 1.0 => AccessoryMount::Front,
            AccessoryMount::Auto => AccessoryMount::Loose,
            m => m,
        };
        for _ in 0..*count {
            match mount {
                AccessoryMount::Front => {
                    a.front.push(Piece { part: p, size: *s, at: 0.0, z: a.d });
                    a.d += s[2];
                }
                AccessoryMount::Left => left.push((*p, *s)),
                AccessoryMount::Right => right.push((*p, *s)),
                AccessoryMount::Auto | AccessoryMount::Loose => {}
            }
        }
    }
    // Left accessories in their order outwards from the main part.
    let mut at = 0.0;
    for (p, s) in left.iter().rev() {
        a.sides.push(Piece { part: p, size: *s, at, z: 0.0 });
        at += s[0];
    }
    a.main.at = at;
    for f in &mut a.front {
        f.at = at + (size[0] - f.size[0]) / 2.0;
    }
    at += size[0];
    for (p, s) in right {
        a.sides.push(Piece { part: p, size: s, at, z: 0.0 });
        at += s[0];
    }
    a.w = at;
    let deepest = a.sides.iter().map(|s| s.size[2]).fold(a.d, f32::max);
    a.d = deepest;
    Some(a)
}

/// The assembly of a placed device item, if parts are mounted on its main part (the
/// item drawn piece by piece). Door and floor devices are drawn as one.
pub fn mounted<'a>(prj: &'a Project, it: &PanelItem) -> Option<Assembly<'a>> {
    if it.kind != ItemKind::Device || matches!(it.face, Face::Door | Face::Bottom) {
        return None;
    }
    assembly(prj, &it.tag).filter(|a| !a.is_plain() && a.main.part.number == it.part)
}

#[cfg(test)]
mod tests;
