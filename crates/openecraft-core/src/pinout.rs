//! Connector pin-outs: the mating face of a plug (after the `Pin layout` of its part,
//! [`openecraft_parts::connector`]) with the pins of the schematic on it – potential,
//! targets, wire colour and cable core of both sides – drawn through [`Draw`] for the
//! plug window and the SVG export; flat cables and their cores on IDC connectors.

use crate::geom::{p, P};
use crate::issue::{Issue, Severity};
use crate::model::*;
use crate::netlist::Netlist;
use crate::plugs::Pin;
use crate::render::{Draw, Ink, VAnchor};
use crate::symbols::{Anchor, PinKind};
use openecraft_i18n::{tr, tr_fmt};
use openecraft_parts::connector::{self, Face, Layout, Outline, PinLayout};
use openecraft_parts::{FnKind, Part};

/// Side of a plug: the pins (male) or the sockets (female). The face of the socket
/// side is the mirror image of the pin side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Pin,
    Socket,
}

impl Side {
    pub fn title(self) -> &'static str {
        match self {
            Side::Pin => tr("Pin side (male)"),
            Side::Socket => tr("Socket side (female)"),
        }
    }
}

/// What one side of a pin is wired to.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct End {
    /// Connection points at the other end of the connection.
    pub targets: String,
    /// Colour code of the wire or cable core at the pin (`BN`, `GNYE` …).
    pub color: String,
    /// Cable and core of the wire at the pin (`-W1:3`).
    pub core: String,
}

/// A pin of the plug's face.
#[derive(Clone, Debug, PartialEq)]
pub struct FacePin {
    pub label: String,
    /// Position on the face; None: the face has no place for it.
    pub slot: Option<usize>,
    /// Placed symbols; empty for spare pins.
    pub pins: Vec<Pin>,
    pub potential: String,
    pub function_text: String,
    pub socket: End,
    pub pin: End,
}

impl FacePin {
    pub fn used(&self) -> bool {
        !self.pins.is_empty()
    }

    pub fn end(&self, side: Side) -> &End {
        match side {
            Side::Pin => &self.pin,
            Side::Socket => &self.socket,
        }
    }
}

/// The pin-out of a plug.
#[derive(Clone, Debug, PartialEq)]
pub struct Pinout {
    pub plug: String,
    /// Connector parts of the plug: number, description, gender.
    pub parts: Vec<(String, String, String)>,
    /// Pin layout of the part (`None`: none given, one or two rows are shown).
    pub layout: Option<PinLayout>,
    pub face: Face,
    pub coding: String,
    pub pins: Vec<FacePin>,
}

fn part<'a>(prj: &'a Project, number: &str) -> Option<&'a Part> {
    prj.parts.iter().chain(crate::parts::catalog()).find(|p| p.number == number)
}

fn plug_pins(part: &Part) -> Vec<String> {
    part.functions.iter().filter(|f| f.kind == FnKind::PlugPin).map(|f| f.pins.trim().to_string()).collect()
}

/// Connector parts of a plug, the part with a pin layout and the most pins first.
fn connector_parts<'a>(prj: &'a Project, plug: &str) -> Vec<&'a Part> {
    let mut v: Vec<&Part> = prj.devices.get(plug).map(|d| d.parts.iter().filter_map(|r| part(prj, &r.number)).filter(|p| !plug_pins(p).is_empty()).collect()).unwrap_or_default();
    v.sort_by_key(|p| (PinLayout::of_part(p).is_none(), std::cmp::Reverse(plug_pins(p).len())));
    v
}

/// The wire touching connection point `conn` of a placed symbol.
fn wire_at(prj: &Project, page: usize, sym: usize, conn: usize) -> Option<&Wire> {
    let s = &prj.pages[page].symbols[sym];
    let q = prj.symbol(&s.sym)?.pin_world(conn, s.pos, s.rot, s.mirror);
    prj.pages[page].wires.iter().find(|w| (w.a - q).len() < 0.01 || (w.b - q).len() < 0.01)
}

/// Connection points of the pins on one side: (symbol, connection point index).
fn side_points(pins: &[Pin], side: Side) -> Vec<(&Pin, usize)> {
    pins.iter()
        .filter_map(|p| match (p.kind, side) {
            (PinKind::Pair, Side::Socket) | (PinKind::Female, Side::Socket) | (PinKind::Male, Side::Pin) => Some((p, 0)),
            (PinKind::Pair, Side::Pin) => Some((p, 1)),
            _ => None,
        })
        .collect()
}

/// Ribbon cable: a flat cable with a `Pitch` (grey, core 1 marked red).
pub fn is_ribbon(prj: &Project, cable: &Cable) -> bool {
    is_flat(prj, cable) && part(prj, &cable.part).and_then(|p| p.tech_value("Pitch")).is_some()
}

/// Flat cable: `Cable form: Flat` in the technical data of the cable's part.
pub fn is_flat(prj: &Project, cable: &Cable) -> bool {
    part(prj, &cable.part).and_then(|p| p.tech_value("Cable form")).is_some_and(|f| f.eq_ignore_ascii_case("flat"))
}

/// IDC connector: a part of the plug has `Termination: IDC`.
pub fn is_idc(prj: &Project, plug: &str) -> bool {
    connector_parts(prj, plug).iter().any(|p| p.tech_value("Termination").is_some_and(|t| t.eq_ignore_ascii_case("IDC")))
}

fn end(prj: &Project, nl: &Netlist, open: &Netlist, pins: &[Pin], side: Side, targets: String) -> End {
    let mut e = End { targets, ..Default::default() };
    let Some(w) = side_points(pins, side).into_iter().find_map(|(p, conn)| wire_at(prj, p.page, p.sym, conn)) else { return e };
    if let Some(c) = prj.cable(&w.cable).filter(|_| !w.core.is_empty()) {
        e.core = format!("{}:{}", crate::structure::relative(&prj.pages[pins[0].page], &c.tag), w.core);
        if crate::potentials::color_rgb(&w.core).is_some() || !crate::potentials::color_parts(&w.core).is_empty() {
            e.color = crate::potentials::normalize_color(&w.core);
        } else if is_ribbon(prj, c) {
            e.color = if w.core == "1" { "RD".into() } else { "GY".into() };
        }
    }
    if e.color.is_empty() {
        e.color = crate::potentials::props_of_wire(prj, nl, open, w.id).color;
    }
    e
}

/// Name of the potential at a pin; None for connections without a named potential.
fn named_potential(nl: &Netlist, pins: &[Pin]) -> Option<String> {
    let n = *nl.pin_net.get(&(pins.first()?.id, 0))?;
    nl.nets[n].named.then(|| nl.nets[n].name.clone())
}

/// Pin-out of a plug: the pins of its part in face order (unused ones as spares),
/// then pins of the schematic the part does not have. `nl`: potentials, `open`:
/// plug sides apart.
pub fn pinout(prj: &Project, plug: &str, nl: &Netlist, open: &Netlist) -> Pinout {
    let parts = connector_parts(prj, plug);
    let rows: Vec<crate::plugs::Row> = crate::plugs::rows(prj, plug, nl, open).into_iter().filter(|r| !r.pins.is_empty()).collect();
    let mut labels: Vec<String> = parts.first().map(|p| plug_pins(p)).unwrap_or_default();
    // Pins the part does not have get no place on its face.
    let own = if labels.is_empty() { usize::MAX } else { labels.len() };
    for r in &rows {
        if !labels.contains(&r.label) {
            labels.push(r.label.clone());
        }
    }
    let layout = parts.first().and_then(|p| PinLayout::of_part(p));
    let pl = layout.clone().unwrap_or(PinLayout { layout: Layout::fallback(labels.len()), pe: false });
    let pitch = parts.first().and_then(|p| connector::pitch_of(p)).unwrap_or(0.0);
    let mut slots = connector::assign(&labels[..own.min(labels.len())], pl.slots());
    slots.resize(labels.len(), None);
    let pins = labels
        .iter()
        .zip(slots)
        .map(|(label, slot)| match rows.iter().find(|r| &r.label == label) {
            Some(r) => FacePin {
                label: label.clone(),
                slot,
                pins: r.pins.clone(),
                potential: named_potential(nl, &r.pins).unwrap_or_default(),
                function_text: r.function_text.clone(),
                socket: end(prj, nl, open, &r.pins, Side::Socket, r.socket.clone()),
                pin: end(prj, nl, open, &r.pins, Side::Pin, r.pin.clone()),
            },
            None => FacePin { label: label.clone(), slot, pins: Vec::new(), potential: String::new(), function_text: String::new(), socket: End::default(), pin: End::default() },
        })
        .collect();
    Pinout {
        plug: plug.to_string(),
        parts: parts.iter().map(|p| (p.number.clone(), p.description.clone(), p.tech_value("Gender").unwrap_or_default().to_string())).collect(),
        layout,
        face: connector::face(&pl, pitch),
        coding: parts.iter().find_map(|p| p.tech_value("Coding")).unwrap_or_default().to_string(),
        pins,
    }
}

/// Makes the wires at one side of the plug's pins cores of a flat cable: pin n takes
/// core n (or the core at the pin's place in the plug order), so that on an IDC
/// connector every core lands on its pin. Returns the number of wires.
pub fn connect_flat_cable(prj: &mut Project, cable: &str, plug: &str, side: Side) -> Result<usize, String> {
    let c = prj.cable(cable).ok_or_else(|| tr_fmt!("No cable {}", cable))?;
    let cores = c.core_list();
    let order: Vec<String> = connector_parts(prj, plug).first().map(|p| plug_pins(p)).unwrap_or_else(|| crate::plugs::designations(prj, plug));
    let mut set = Vec::new();
    for pin in crate::plugs::pins(prj, plug) {
        let core = if cores.contains(&pin.label) { Some(pin.label.clone()) } else { order.iter().position(|l| *l == pin.label).and_then(|i| cores.get(i).cloned()) };
        let Some(core) = core else { continue };
        for (p, conn) in side_points(std::slice::from_ref(&pin), side) {
            if let Some(w) = wire_at(prj, p.page, p.sym, conn) {
                set.push((p.page, w.id, core.clone()));
            }
        }
    }
    if set.is_empty() {
        return Err(tr_fmt!("No wires at the pins of {}", plug));
    }
    for (page, id, core) in &set {
        if let Some(w) = prj.pages[*page].wires.iter_mut().find(|w| w.id == *id) {
            w.cable = cable.to_string();
            w.core = core.clone();
        }
    }
    Ok(set.len())
}

/// Pins not on the plug's part, and flat cables on IDC connectors whose cores do not
/// land on the pins of the same number or whose core count differs from the pins.
pub fn problems(prj: &Project) -> Vec<Issue> {
    let mut v = Vec::new();
    for plug in crate::plugs::plug_tags(prj) {
        let parts = connector_parts(prj, &plug);
        let pins = crate::plugs::pins(prj, &plug);
        if let Some(part) = parts.first() {
            let own = plug_pins(part);
            let mut seen: Vec<&str> = Vec::new();
            for p in pins.iter().filter(|p| !own.contains(&p.label)) {
                if !seen.contains(&p.label.as_str()) {
                    seen.push(&p.label);
                    v.push(crate::issue!(Severity::Warning, Some((p.page, p.id)), "{}: pin {} is not a pin of the part {}", plug, p.label, part.number));
                }
            }
        }
        if !is_idc(prj, &plug) {
            continue;
        }
        let capacity = parts.first().map(|p| plug_pins(p).len()).unwrap_or(0);
        let mut cables: Vec<String> = Vec::new();
        for pin in &pins {
            for (p, conn) in side_points(std::slice::from_ref(pin), Side::Socket).into_iter().chain(side_points(std::slice::from_ref(pin), Side::Pin)) {
                let Some(w) = wire_at(prj, p.page, p.sym, conn) else { continue };
                let Some(c) = prj.cable(&w.cable).filter(|c| is_flat(prj, c)) else { continue };
                if w.core != pin.label {
                    v.push(crate::issue!(Severity::Warning, Some((p.page, p.id)), "{}: core {} of flat cable {} lands on pin {}", plug, w.core, c.tag, pin.label));
                }
                if !cables.contains(&c.tag) {
                    cables.push(c.tag.clone());
                    let n = c.core_list().len();
                    if capacity > 0 && n != capacity {
                        v.push(crate::issue!(Severity::Warning, Some((p.page, p.id)), "Flat cable {} has {} cores, the IDC connector {} has {} pins", c.tag, n, plug, capacity));
                    }
                }
            }
        }
    }
    v
}

/// Size of a face drawn at `scale`, without its caption.
pub fn face_size(po: &Pinout, scale: f32) -> (f32, f32) {
    let (w, h) = po.face.outline.size();
    (w * scale, h * scale)
}

/// Scale that draws the face `w` wide at most (and not larger than 10:1).
pub fn fit_scale(po: &Pinout, w: f32, h: f32) -> f32 {
    let (fw, fh) = po.face.outline.size();
    (w / fw.max(1.0)).min(h / fh.max(1.0)).min(10.0)
}

/// Centres of the pins on the face of one side drawn around `centre`: (index into
/// [`Pinout::pins`], centre, radius).
pub fn pin_points(po: &Pinout, side: Side, centre: P, scale: f32) -> Vec<(usize, P, f32)> {
    let mirror = if side == Side::Socket { -1.0 } else { 1.0 };
    po.pins
        .iter()
        .enumerate()
        .filter_map(|(i, fp)| {
            let [x, y] = *po.face.slots.get(fp.slot?)?;
            Some((i, p(centre.x + mirror * x * scale, centre.y + y * scale), po.face.pin_r * scale))
        })
        .collect()
}

fn semicircle(c: P, r: f32) -> Vec<P> {
    (0..=12).map(|i| {
        let a = std::f32::consts::PI * i as f32 / 12.0;
        p(c.x + r * a.cos(), c.y + r * a.sin())
    }).collect()
}

/// Draws the mating face of one side around `centre`: outline with coding key, the
/// pins filled with the colour of their wire (spares empty) and their designations;
/// `highlight`: index of a pin drawn emphasised.
pub fn draw_face(d: &mut dyn Draw, po: &Pinout, side: Side, centre: P, scale: f32, highlight: Option<usize>) {
    let at = |x: f32, y: f32| p(centre.x + x * scale, centre.y + y * scale);
    let line = 0.35;
    match po.face.outline {
        Outline::Circle { r } => {
            d.circle(centre, r * scale, 0.0, Ink::Plate, true);
            d.circle(centre, r * scale, line * 1.5, Ink::Line, false);
            d.circle(centre, r * scale * 0.88, line * 0.6, Ink::Frame, false);
        }
        Outline::Rect { w, h } => {
            d.rect(at(-w / 2.0, -h / 2.0), at(w / 2.0, h / 2.0), 0.0, Ink::Plate, true);
            d.rect(at(-w / 2.0, -h / 2.0), at(w / 2.0, h / 2.0), line * 1.5, Ink::Line, false);
        }
        Outline::DSub { top, bottom, h } => {
            let pts = [at(-top / 2.0, -h / 2.0), at(top / 2.0, -h / 2.0), at(bottom / 2.0, h / 2.0), at(-bottom / 2.0, h / 2.0)];
            d.poly(&pts, true, 0.0, Ink::Plate, true);
            d.poly(&pts, true, line * 1.5, Ink::Line, false);
        }
    }
    if let Some([kx, ky]) = po.face.key {
        // Coding key (circular) or polarisation notch (box header): a bar into the face.
        let pitch = po.face.pin_r / 0.28;
        let (hw, depth) = (pitch * 0.35, pitch * 0.45 * ky.signum());
        d.rect(at(kx - hw, ky), at(kx + hw, ky - depth), 0.0, Ink::Line, true);
    }
    for (i, c, r) in pin_points(po, side, centre, scale) {
        let fp = &po.pins[i];
        let end = fp.end(side);
        let colors = crate::potentials::color_parts(&end.color);
        let text_ink = match colors.first() {
            Some([r, g, b]) if (*r as u32 * 3 + *g as u32 * 6 + *b as u32) / 10 < 110 => Ink::Color([255, 255, 255]),
            Some(_) => Ink::Color([70, 70, 70]),
            None => Ink::Text,
        };
        if fp.used() {
            match colors.as_slice() {
                [] => d.circle(c, r, 0.0, Ink::Fill, true),
                [a] => d.circle(c, r, 0.0, Ink::Color(*a), true),
                [a, b, ..] => {
                    d.circle(c, r, 0.0, Ink::Color(*a), true);
                    d.poly(&semicircle(c, r), true, 0.0, Ink::Color(*b), true);
                }
            }
            d.circle(c, r, line, Ink::Line, false);
        } else {
            d.circle(c, r, 0.0, Ink::Paper, true);
            d.circle(c, r, line * 0.6, Ink::Frame, false);
        }
        if highlight == Some(i) {
            d.circle(c, r * 1.3, line * 2.0, Ink::Highlight, false);
        }
        let size = (r * 0.85).min(3.5) / (fp.label.chars().count() as f32).max(1.0).sqrt();
        d.text(c, &fp.label, size, Anchor::Center, VAnchor::Middle, if fp.used() { text_ink } else { Ink::FrameText });
    }
}

/// Caption under a face: side and the part of that gender (hermaphroditic parts fit both).
fn caption(po: &Pinout, side: Side) -> String {
    let gender = match side {
        Side::Pin => "male",
        Side::Socket => "female",
    };
    match po.parts.iter().find(|p| p.2 == gender || p.2 == "hermaphroditic") {
        Some(part) => format!("{}  {}", side.title(), part.0),
        None => side.title().to_string(),
    }
}

/// Columns of the pin table: pin, potential, pin side, socket side, cable core,
/// function text (text, width in mm).
fn table_rows(po: &Pinout) -> Vec<[String; 6]> {
    po.pins
        .iter()
        .map(|fp| {
            let side = |e: &End| [e.targets.as_str(), e.color.as_str()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("  ");
            let core = [fp.pin.core.as_str(), fp.socket.core.as_str()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", ");
            let ft = if fp.used() { fp.function_text.clone() } else { tr("spare").to_string() };
            [fp.label.clone(), fp.potential.clone(), side(&fp.pin), side(&fp.socket), core, ft]
        })
        .collect()
}

const ROW: f32 = 5.0;
const FACE: f32 = 56.0;

/// Lines under the plug's tag: one per part, then pin layout and coding.
fn header_lines(po: &Pinout) -> Vec<String> {
    let mut v: Vec<String> = po.parts.iter().map(|p| format!("{}   {}", p.0, p.1)).collect();
    let mut facts = Vec::new();
    if let Some(l) = &po.layout {
        facts.push(tr_fmt!("Pin layout: {}", l.text()));
    }
    if !po.coding.is_empty() {
        facts.push(tr_fmt!("Coding: {}", po.coding));
    }
    if !facts.is_empty() {
        v.push(facts.join("   "));
    }
    v
}

/// Height of a plug's block in the drawing of [`draw_block`].
pub fn block_height(po: &Pinout) -> f32 {
    10.0 + 4.5 * header_lines(po).len() as f32 + 2.0 + FACE + 12.0 + ROW * (po.pins.len() as f32 + 1.0) + 8.0
}

/// Draws a plug's pin-out at `origin` (`width` mm wide): title, the faces of the
/// pin and the socket side with a legend and the pin table. Returns the height.
pub fn draw_block(d: &mut dyn Draw, po: &Pinout, origin: P, width: f32) -> f32 {
    let x0 = origin.x;
    let mut y = origin.y;
    d.text(p(x0, y + 4.0), &po.plug, 5.0, Anchor::Left, VAnchor::Middle, Ink::Text);
    y += 10.0;
    for line in header_lines(po) {
        let max = (width / (2.4 * 0.6)) as usize;
        let shown: String = if line.chars().count() > max { line.chars().take(max - 1).collect::<String>() + "…" } else { line };
        d.text(p(x0, y), &shown, 2.4, Anchor::Left, VAnchor::Middle, Ink::FrameText);
        y += 4.5;
    }
    y += 2.0;
    let scale = fit_scale(po, FACE, FACE);
    for (k, side) in [Side::Pin, Side::Socket].into_iter().enumerate() {
        let cx = x0 + FACE / 2.0 + 4.0 + k as f32 * (FACE + 24.0);
        draw_face(d, po, side, p(cx, y + FACE / 2.0), scale, None);
        let (_, fh) = face_size(po, scale);
        d.text(p(cx, y + (FACE + fh) / 2.0 + 5.0), &caption(po, side), 2.6, Anchor::Center, VAnchor::Middle, Ink::Text);
    }
    // Legend beside the faces.
    let lx = x0 + 2.0 * FACE + 34.0;
    d.text(p(lx, y + 4.0), tr("Mating faces"), 2.6, Anchor::Left, VAnchor::Middle, Ink::Text);
    let r = 1.6;
    d.circle(p(lx + r, y + 11.0), r, 0.0, Ink::Color([150, 75, 0]), true);
    d.circle(p(lx + r, y + 11.0), r, 0.35, Ink::Line, false);
    d.text(p(lx + 2.0 * r + 2.0, y + 11.0), tr("used: colour of the wire or core"), 2.2, Anchor::Left, VAnchor::Middle, Ink::FrameText);
    d.circle(p(lx + r, y + 17.0), r, 0.0, Ink::Paper, true);
    d.circle(p(lx + r, y + 17.0), r, 0.2, Ink::Frame, false);
    d.text(p(lx + 2.0 * r + 2.0, y + 17.0), tr("spare"), 2.2, Anchor::Left, VAnchor::Middle, Ink::FrameText);
    d.rect(p(lx + 0.6, y + 21.8), p(lx + 2.6, y + 24.2), 0.0, Ink::Line, true);
    d.text(p(lx + 2.0 * r + 2.0, y + 23.0), tr("coding key"), 2.2, Anchor::Left, VAnchor::Middle, Ink::FrameText);
    y += FACE + 12.0;
    let cols = [0.0, 0.08, 0.2, 0.44, 0.68, 0.8];
    let heads = [tr("Pin"), tr("Potential"), tr("Pin side"), tr("Socket side"), tr("Cable core"), tr("Function text")];
    for (c, h) in cols.iter().zip(heads) {
        d.text(p(x0 + c * width, y + ROW / 2.0), h, 2.4, Anchor::Left, VAnchor::Middle, Ink::Text);
    }
    y += ROW;
    d.line(p(x0, y), p(x0 + width, y), 0.3, Ink::Frame, false);
    for (fp, row) in po.pins.iter().zip(table_rows(po)) {
        let ink = if fp.used() { Ink::Text } else { Ink::FrameText };
        for (k, (c, text)) in cols.iter().zip(row.iter()).enumerate() {
            let max = (((cols.get(k + 1).copied().unwrap_or(1.0) - c) * width - 1.5) / (2.2 * 0.6)) as usize;
            let shown: String = if text.chars().count() > max && max > 1 { text.chars().take(max - 1).collect::<String>() + "…" } else { text.clone() };
            d.text(p(x0 + c * width, y + ROW / 2.0), &shown, 2.2, Anchor::Left, VAnchor::Middle, ink);
        }
        y += ROW;
    }
    y - origin.y + 8.0
}

/// SVG drawing of the pin-outs of the plugs (all plugs when `plugs` is empty).
pub fn svg(prj: &Project, nl: &Netlist, open: &Netlist, plugs: &[String]) -> String {
    let tags: Vec<String> = if plugs.is_empty() { crate::plugs::plug_tags(prj) } else { plugs.to_vec() };
    let pinouts: Vec<Pinout> = tags.iter().map(|t| pinout(prj, t, nl, open)).collect();
    let (w, margin) = (210.0, 10.0);
    let h = pinouts.iter().map(block_height).sum::<f32>() + 2.0 * margin;
    crate::svg::drawing_svg(w, h.max(40.0), |d| {
        let mut y = margin;
        for po in &pinouts {
            y += draw_block(d, po, p(margin, y), w - 2.0 * margin);
        }
        if pinouts.is_empty() {
            d.text(p(margin, margin + 5.0), tr("No plugs"), 3.0, Anchor::Left, VAnchor::Middle, Ink::Text);
        }
    })
}

#[cfg(test)]
mod tests;
