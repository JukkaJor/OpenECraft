//! Backend independent page rendering. The same code draws to the egui canvas and
//! to SVG export, so what you see is what you print.

use openecraft_i18n::{tr, tr_fmt};
use crate::geom::{p, P, R};
use crate::index::{Index, Loc};
use crate::model::*;
use crate::netlist;
use crate::panel::{self, Enclosure, Face, ItemKind, PanelItem};
use crate::parts::{Model2d, Prim2d};
use crate::parts::{self, FnKind};
use crate::reports::Analysis;
use crate::{structure, wirenum};
use crate::symbols::{self, Anchor, ContactKind, FieldKind, Func, Prim, SymbolDef};
use std::collections::{BTreeSet, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    Line,
    Wire,
    Text,
    Frame,
    FrameText,
    Selected,
    Highlight,
    Ghost,
    Pin,
    CrossRef,
    Error,
    /// Light fill of mounting plates.
    Plate,
    /// Fill of devices / ducts in mounting layouts.
    Fill,
    /// Wire drawn in its own colour (RGB).
    Color([u8; 3]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VAnchor {
    Middle,
    Bottom,
}

pub trait Draw {
    fn line(&mut self, a: P, b: P, width: f32, ink: Ink, dashed: bool);
    fn poly(&mut self, pts: &[P], closed: bool, width: f32, ink: Ink, fill: bool);
    fn circle(&mut self, c: P, r: f32, width: f32, ink: Ink, fill: bool);
    fn text(&mut self, pos: P, text: &str, size: f32, h: Anchor, v: VAnchor, ink: Ink);
    fn rect(&mut self, a: P, b: P, width: f32, ink: Ink, fill: bool) {
        self.poly(&[a, p(b.x, a.y), b, p(a.x, b.y)], true, width, ink, fill);
    }
    /// Marks the area `min`..`max` as a link to another page (PDF export).
    fn link(&mut self, _min: P, _max: P, _page: usize) {}
}

/// Links the area of a text (estimated from its length) to a page.
fn link_text(d: &mut dyn Draw, pos: P, text: &str, size: f32, h: Anchor, page: usize) {
    let w = text.chars().count() as f32 * size * 0.62;
    let x0 = match h {
        Anchor::Left => pos.x,
        Anchor::Center => pos.x - w / 2.0,
        Anchor::Right => pos.x - w,
    };
    d.link(p(x0, pos.y - size * 0.7), p(x0 + w, pos.y + size * 0.7), page);
}

pub const SYMBOL_W: f32 = 0.25;
pub const WIRE_W: f32 = 0.35;
pub const TAG_SIZE: f32 = 2.5;
pub const SMALL_SIZE: f32 = 1.8;

#[derive(Default)]
pub struct Opts {
    pub selected: BTreeSet<u64>,
    pub highlight_wires: HashSet<u64>,
    pub show_pins: bool,
    /// Revision marks for changes since the last revision are drawn unless hidden.
    pub hide_revision_marks: bool,
}

fn is_black(c: &[u8; 3]) -> bool {
    crate::potentials::color_rgb("BK") == Some(*c)
}

/// Draws a complete page including frame and title block.
pub fn draw_page(d: &mut dyn Draw, prj: &Project, idx: &Index, an: &Analysis, page_i: usize, opts: &Opts) {
    let page = &prj.pages[page_i];
    draw_frame(d, prj, page_i);
    match page.kind {
        PageKind::Cover => draw_cover(d, prj),
        PageKind::Report(kind) => draw_report(d, prj, an, kind),
        PageKind::Mounting(id) => draw_mounting_page(d, prj, id),
        PageKind::Schematic => {}
    }
    for s in &page.shapes {
        let ink = if opts.selected.contains(&s.id) { Ink::Selected } else { Ink::Line };
        draw_shape(d, s, ink);
    }
    let colors = if prj.wire_colors && page.kind == PageKind::Schematic {
        crate::potentials::wire_colors(prj, &an.netlist, &an.open)
    } else {
        Default::default()
    };
    for w in &page.wires {
        let ink = if opts.selected.contains(&w.id) {
            Ink::Selected
        } else if opts.highlight_wires.contains(&w.id) {
            Ink::Highlight
        } else {
            Ink::Wire
        };
        let parts = colors.get(&w.id).map(|c| crate::potentials::color_parts(c)).unwrap_or_default();
        match parts.as_slice() {
            // Black is the normal wire ink (stays visible on a dark canvas).
            [c] if ink == Ink::Wire && is_black(c) => d.line(w.a, w.b, WIRE_W, ink, false),
            [c] if ink == Ink::Wire => d.line(w.a, w.b, WIRE_W, Ink::Color(*c), false),
            // Two-colour wires: the second colour dashed over the first.
            [a, b] if ink == Ink::Wire => {
                d.line(w.a, w.b, WIRE_W, Ink::Color(*a), false);
                d.line(w.a, w.b, WIRE_W, Ink::Color(*b), true);
            }
            _ => d.line(w.a, w.b, WIRE_W, ink, false),
        }
    }
    for j in netlist::junctions(page) {
        d.circle(j, 0.7, 0.0, Ink::Wire, true);
    }
    draw_wire_labels(d, prj, an, page_i);
    let structured = !page.function.is_empty() || !page.location.is_empty();
    for s in &page.symbols {
        let Some(def) = symbols::get(&s.sym) else { continue };
        let ink = if opts.selected.contains(&s.id) { Ink::Selected } else { Ink::Line };
        if structured {
            let shown = SymbolInst { tag: structure::relative(page, &s.tag), ..s.clone() };
            draw_symbol(d, def, &shown, ink);
        } else {
            draw_symbol(d, def, s, ink);
        }
        draw_annotations(d, prj, idx, page_i, def, s, ink);
        if opts.show_pins {
            for i in 0..def.pins.len() {
                d.circle(def.pin_world(i, s.pos, s.rot, s.mirror), 0.6, 0.15, Ink::Pin, false);
            }
        }
    }
    for t in &page.texts {
        let ink = if opts.selected.contains(&t.id) { Ink::Selected } else { Ink::Text };
        for (i, line) in t.text.lines().enumerate() {
            d.text(t.pos + p(0.0, i as f32 * t.size * 1.4), line, t.size, Anchor::Left, VAnchor::Middle, ink);
        }
    }
    if !opts.hide_revision_marks {
        draw_revision_marks(d, prj, an, page_i);
    }
}

/// Triangle with the upcoming revision index at every item changed since the last
/// revision (at the title block for changed page properties / new pages).
fn draw_revision_marks(d: &mut dyn Draw, prj: &Project, an: &Analysis, page_i: usize) {
    let Some(pc) = an.changes.iter().find(|c| c.page == Some(page_i)) else { return };
    let name = crate::revision::next_name(prj);
    let mut done: Vec<(i64, i64)> = Vec::new();
    for c in &pc.changes {
        let at = if c.what == "Page" { p(203.0, 262.0) } else { c.pos + p(3.5, -3.5) };
        // One mark per spot (e.g. a moved wire split into several).
        if done.contains(&at.key()) {
            continue;
        }
        done.push(at.key());
        let ink = Ink::Highlight;
        d.poly(&[at + p(0.0, -2.4), at + p(2.2, 1.4), at + p(-2.2, 1.4)], true, 0.2, ink, false);
        d.text(at + p(0.0, 0.3), &name, 1.5, Anchor::Center, VAnchor::Middle, ink);
    }
}

pub fn draw_shape(d: &mut dyn Draw, s: &Shape, ink: Ink) {
    match s.kind {
        ShapeKind::Line => d.line(s.a, s.b, SYMBOL_W, ink, s.dashed),
        ShapeKind::Rect => {
            let (a, b) = (s.a, s.b);
            let c = [a, p(b.x, a.y), b, p(a.x, b.y), a];
            for w in c.windows(2) {
                d.line(w[0], w[1], SYMBOL_W, ink, s.dashed);
            }
        }
    }
}

/// Draws only the symbol graphics (used for canvas, ghost and library previews).
pub fn draw_symbol(d: &mut dyn Draw, def: &SymbolDef, s: &SymbolInst, ink: Ink) {
    draw_prims(d, &def.prims, s, ink);
}

/// Symbol graphics placed like the instance `s` (also used by the symbol editor).
pub fn draw_prims(d: &mut dyn Draw, prims: &[Prim], s: &SymbolInst, ink: Ink) {
    let tf = |q: P| s.pos + q.transform(s.rot, s.mirror);
    let text_ink = if ink == Ink::Line { Ink::Text } else { ink };
    for prim in prims {
        match prim {
            Prim::Line { a, b, dashed } => d.line(tf(*a), tf(*b), SYMBOL_W, ink, *dashed),
            Prim::Poly { pts, closed, fill } => {
                let v: Vec<P> = pts.iter().map(|q| tf(*q)).collect();
                d.poly(&v, *closed, SYMBOL_W, ink, *fill);
            }
            Prim::Circle { c, r, fill } => d.circle(tf(*c), *r, SYMBOL_W, ink, *fill),
            Prim::Text { pos, text, size, anchor } => {
                d.text(tf(*pos), text, *size, rotated_anchor(*anchor, s), VAnchor::Middle, text_ink)
            }
            Prim::Field { kind, pos, size, anchor } => {
                let t = match kind {
                    FieldKind::Label => &s.label,
                    FieldKind::Tag => &s.tag,
                };
                if !t.is_empty() {
                    d.text(tf(*pos), t, *size, rotated_anchor(*anchor, s), VAnchor::Middle, text_ink);
                }
            }
        }
    }
}

/// Text stays horizontal when a symbol is rotated, so left/right anchors follow the
/// side the text ends up on.
fn rotated_anchor(a: Anchor, s: &SymbolInst) -> Anchor {
    if a == Anchor::Center || (s.rot.is_multiple_of(4) && !s.mirror) {
        return a;
    }
    let dir = if a == Anchor::Left { p(1.0, 0.0) } else { p(-1.0, 0.0) };
    let w = dir.transform(s.rot, s.mirror);
    if w.x > 0.5 {
        Anchor::Left
    } else if w.x < -0.5 {
        Anchor::Right
    } else {
        Anchor::Center
    }
}

fn find_symbol(prj: &Project, loc: &Loc) -> Option<&'static SymbolDef> {
    prj.pages[loc.page].symbols.iter().find(|s| s.id == loc.sym_id).and_then(|s| symbols::get(&s.sym))
}

fn find_inst<'a>(prj: &'a Project, loc: &Loc) -> Option<&'a SymbolInst> {
    prj.pages[loc.page].symbols.iter().find(|s| s.id == loc.sym_id)
}

/// Text beside a wire at fraction `t` of its length: left of vertical wires, above
/// horizontal ones (`below`: right of / below the wire).
fn wire_text(d: &mut dyn Draw, w: &Wire, t: f32, text: &str, below: bool, ink: Ink) {
    let q = p(w.a.x + (w.b.x - w.a.x) * t, w.a.y + (w.b.y - w.a.y) * t);
    if (w.b.x - w.a.x).abs() < (w.b.y - w.a.y).abs() {
        let (dx, anchor) = if below { (1.2, Anchor::Left) } else { (-0.8, Anchor::Right) };
        d.text(p(q.x + dx, q.y), text, SMALL_SIZE, anchor, VAnchor::Middle, ink);
    } else if below {
        d.text(p(q.x, q.y + 2.8), text, SMALL_SIZE, Anchor::Center, VAnchor::Bottom, ink);
    } else {
        d.text(p(q.x, q.y - 0.6), text, SMALL_SIZE, Anchor::Center, VAnchor::Bottom, ink);
    }
}

/// Wire numbers (once per connection and page, on its longest wire) and cable cores
/// (a stroke across the wire, the core and once per page the cable tag).
fn draw_wire_labels(d: &mut dyn Draw, prj: &Project, an: &Analysis, page_i: usize) {
    let page = &prj.pages[page_i];
    for net in &an.open.nets {
        let Some(w) = wirenum::label_wire(page, page_i, net) else { continue };
        let number = wirenum::number_of(prj, net);
        if !number.is_empty() {
            // Cable cores are labelled at 30 % of the wire.
            let t = if w.cable.is_empty() { 0.5 } else { 0.72 };
            wire_text(d, w, t, &number, false, Ink::Text);
        }
    }
    let mut cabled: Vec<&Wire> = page.wires.iter().filter(|w| !w.cable.is_empty()).collect();
    cabled.sort_by(|a, b| a.a.x.min(a.b.x).total_cmp(&b.a.x.min(b.b.x)).then(a.a.y.total_cmp(&b.a.y)));
    let mut tagged: Vec<&str> = Vec::new();
    for w in cabled {
        let t = 0.3;
        let q = p(w.a.x + (w.b.x - w.a.x) * t, w.a.y + (w.b.y - w.a.y) * t);
        d.line(q + p(-1.0, 0.8), q + p(1.0, -0.8), SYMBOL_W, Ink::Line, false);
        if !tagged.contains(&w.cable.as_str()) {
            tagged.push(&w.cable);
            wire_text(d, w, t, &structure::relative(page, &w.cable), false, Ink::CrossRef);
        }
        wire_text(d, w, t, &w.core, true, Ink::CrossRef);
    }
}

/// Device tag, function text, connection point designations and cross references.
pub fn draw_annotations(d: &mut dyn Draw, prj: &Project, idx: &Index, page_i: usize, def: &SymbolDef, s: &SymbolInst, ink: Ink) {
    let text_ink = if ink == Ink::Line { Ink::Text } else { ink };
    let bounds = def.world_bounds(s.pos, s.rot, s.mirror);
    let vertical = s.rot.is_multiple_of(2);

    // Connection point designations
    if def.show_pin_labels {
        let c = bounds.center();
        for (i, label) in s.pins.iter().enumerate().take(def.pins.len()) {
            if label.is_empty() {
                continue;
            }
            let q = def.pin_world(i, s.pos, s.rot, s.mirror);
            let dlt = q - c;
            if dlt.y.abs() >= dlt.x.abs() {
                let y = q.y - dlt.y.signum() * 1.1;
                d.text(p(q.x - 0.6, y), label, SMALL_SIZE, Anchor::Right, VAnchor::Middle, text_ink);
            } else {
                let x = q.x - dlt.x.signum() * 1.1;
                d.text(p(x, q.y - 0.5), label, SMALL_SIZE, Anchor::Center, VAnchor::Bottom, text_ink);
            }
        }
    }

    // Cross references
    let mut refs: Vec<(String, Option<usize>)> = Vec::new();
    let dev = idx.devices.get(&s.tag);
    match def.func {
        Func::Aux(_) => match dev.map(|d| d.mains.as_slice()) {
            Some([]) | None => refs.push(("?".into(), None)),
            Some(mains) => refs.extend(mains.iter().map(|m| (m.reference_from(prj, page_i), Some(m.page)))),
        },
        Func::Overview => {
            if let Some(dev) = dev {
                refs.extend(dev.mains.iter().map(|m| (m.reference_from(prj, page_i), Some(m.page))));
            }
        }
        Func::Main { mirror: false } => {
            if let Some(dev) = dev {
                refs.extend(dev.auxs.iter().map(|a| (a.reference_from(prj, page_i), Some(a.page))));
            }
        }
        Func::Interruption => {
            if let Some(locs) = idx.interruptions.get(&s.label) {
                let others: Vec<&Loc> = locs.iter().filter(|l| l.sym_id != s.id).collect();
                let q = s.pos + p(2.2, 1.6).transform(s.rot, s.mirror) + p(0.0, 2.6);
                let txt = if others.is_empty() { "?".to_string() } else { others.iter().map(|l| l.reference_from(prj, page_i)).collect::<Vec<_>>().join(" ") };
                let ink = if others.is_empty() { Ink::Error } else { Ink::CrossRef };
                d.text(q, &txt, SMALL_SIZE, Anchor::Left, VAnchor::Middle, ink);
                if let Some(l) = others.first() {
                    link_text(d, q, &txt, SMALL_SIZE, Anchor::Left, l.page);
                }
            }
        }
        _ => {}
    }

    // Tag block
    if def.show_tag {
        let mut lines: Vec<(String, f32, Ink, Option<usize>)> = Vec::new();
        if !s.tag.is_empty() {
            lines.push((structure::relative(&prj.pages[page_i], &s.tag), TAG_SIZE, text_ink, None));
        }
        if !s.function_text.is_empty() {
            for l in s.function_text.lines() {
                lines.push((l.to_string(), 2.0, text_ink, None));
            }
        }
        for (r, target) in refs {
            let ink = if r == "?" { Ink::Error } else { Ink::CrossRef };
            lines.push((r, 2.0, ink, target));
        }
        let total: f32 = lines.iter().map(|l| l.1 * 1.35).sum();
        let (x, mut y, anchor) = if vertical {
            (bounds.min.x - 1.2, bounds.center().y - total / 2.0, Anchor::Right)
        } else {
            (bounds.center().x, bounds.min.y - 1.0 - total, Anchor::Center)
        };
        for (t, size, ink, target) in &lines {
            let q = p(x, y + size * 0.675);
            d.text(q, t, *size, anchor, VAnchor::Middle, *ink);
            if let Some(page) = target {
                link_text(d, q, t, *size, anchor, *page);
            }
            y += size * 1.35;
        }
    }

    // Contact image (mirror) below coils
    if matches!(def.func, Func::Main { mirror: true }) {
        if let Some(dev) = dev {
            draw_contact_image(d, prj, page_i, &s.tag, dev.auxs.as_slice(), &bounds, vertical, text_ink);
        }
    }
}

/// Contact image row: kind, connection designations, cross reference and its page
/// (None = reserve contact).
type ContactRow = (ContactKind, Vec<String>, Option<(String, usize)>);

#[allow(clippy::too_many_arguments)]
fn draw_contact_image(d: &mut dyn Draw, prj: &Project, page_i: usize, tag: &str, auxs: &[Loc], bounds: &R, vertical: bool, ink: Ink) {
    // Rows: contact kind, connection designations, cross reference (None = reserve).
    let mut rows: Vec<ContactRow> = Vec::new();
    for loc in auxs {
        let (Some(def), Some(inst)) = (find_symbol(prj, loc), find_inst(prj, loc)) else { continue };
        let kind = def.contact_kind().unwrap_or(ContactKind::No);
        rows.push((kind, inst.pins.clone(), Some((loc.reference_from(prj, page_i), loc.page))));
    }
    // Unused contacts of the assigned parts (function templates) are shown as reserve.
    for f in parts::reserve_functions(prj, None, tag) {
        let kind = match f.kind {
            FnKind::No => ContactKind::No,
            FnKind::Nc => ContactKind::Nc,
            FnKind::Changeover => ContactKind::Changeover,
            FnKind::MainContacts3 | FnKind::MainContact => ContactKind::MainNo,
            _ => continue,
        };
        rows.push((kind, f.pin_list(), None));
    }
    if rows.is_empty() {
        return;
    }
    // Placed right of the outgoing wire so it does not overlap it.
    let (cx, mut y) = if vertical {
        (bounds.center().x + 7.5, bounds.max.y + 3.0)
    } else {
        (bounds.max.x + 8.0, bounds.center().y - (rows.len() as f32 - 1.0) * 1.5)
    };
    for (kind, pins, reference) in rows {
        let (text_ink, line_ink) = match (&reference, ink) {
            (None, Ink::Text) => (Ink::FrameText, Ink::Frame),
            (_, Ink::Text) => (Ink::Text, Ink::Line),
            _ => (ink, ink),
        };
        let (left, right) = if kind == ContactKind::MainNo && pins.len() > 3 {
            // 3-pole main contacts: "1 3 5" / "2 4 6"
            let half = pins.len() / 2;
            (pins[..half].join(" "), pins[half..].join(" "))
        } else if kind == ContactKind::Changeover {
            // Common pin is listed first for changeover contacts.
            (pins.first().cloned().unwrap_or_default(), pins.get(1..).map(|r| r.join(" ")).unwrap_or_default())
        } else {
            (pins.first().cloned().unwrap_or_default(), pins.get(1).cloned().unwrap_or_default())
        };
        mirror_glyph(d, p(cx, y), kind, line_ink);
        d.text(p(cx - 3.6, y), &left, SMALL_SIZE, Anchor::Right, VAnchor::Middle, text_ink);
        d.text(p(cx + 3.6, y), &right, SMALL_SIZE, Anchor::Left, VAnchor::Middle, text_ink);
        let rx = (cx + 4.2 + right.chars().count() as f32 * SMALL_SIZE * 0.6).max(cx + 7.5);
        match reference {
            Some((r, page)) => {
                d.text(p(rx, y), &r, SMALL_SIZE, Anchor::Left, VAnchor::Middle, Ink::CrossRef);
                link_text(d, p(rx, y), &r, SMALL_SIZE, Anchor::Left, page);
            }
            None => d.text(p(rx, y), "reserve", SMALL_SIZE * 0.9, Anchor::Left, VAnchor::Middle, Ink::FrameText),
        }
        y += 3.0;
    }
}

/// Small horizontal contact symbol used in the contact image.
fn mirror_glyph(d: &mut dyn Draw, c: P, kind: ContactKind, ink: Ink) {
    let w = 0.18;
    d.line(c + p(-3.0, 0.0), c + p(-1.2, 0.0), w, ink, false);
    d.line(c + p(1.2, 0.0), c + p(3.0, 0.0), w, ink, false);
    match kind {
        ContactKind::No | ContactKind::MainNo => d.line(c + p(1.2, 0.0), c + p(-1.5, -1.1), w, ink, false),
        ContactKind::Nc => {
            d.line(c + p(-1.2, 0.0), c + p(-1.2, 0.9), w, ink, false);
            d.line(c + p(1.2, 0.0), c + p(-1.6, 1.2), w, ink, false);
        }
        ContactKind::Changeover => {
            d.line(c + p(-1.2, 0.0), c + p(-1.2, 0.9), w, ink, false);
            d.line(c + p(1.2, 0.0), c + p(-1.6, 1.2), w, ink, false);
            d.line(c + p(-1.2, 0.0), c + p(-1.2, -0.9), w, ink, false);
        }
    }
}

pub fn draw_frame(d: &mut dyn Draw, prj: &Project, page_i: usize) {
    let page = &prj.pages[page_i];
    let fi = Ink::Frame;
    d.rect(p(FRAME_X0, FRAME_Y0), p(FRAME_X1, FRAME_Y1), 0.5, fi, false);
    if page.kind == PageKind::Schematic {
        // Column strip
        d.line(p(AREA_X0, AREA_Y0), p(FRAME_X1, AREA_Y0), 0.25, fi, false);
        d.line(p(AREA_X0, FRAME_Y0), p(AREA_X0, TITLE_Y0), 0.25, fi, false);
        let cw = column_width();
        for c in 0..COLUMNS {
            let x = AREA_X0 + c as f32 * cw;
            if c > 0 {
                d.line(p(x, FRAME_Y0), p(x, AREA_Y0), 0.25, fi, false);
                d.line(p(x, TITLE_Y0 - 2.0), p(x, TITLE_Y0), 0.25, fi, false);
            }
            d.text(p(x + cw / 2.0, (FRAME_Y0 + AREA_Y0) / 2.0), &(c + 1).to_string(), 2.5, Anchor::Center, VAnchor::Middle, Ink::FrameText);
        }
        let rh = row_height();
        for r in 0..ROWS {
            let y = AREA_Y0 + r as f32 * rh;
            if r > 0 {
                d.line(p(FRAME_X0, y), p(AREA_X0, y), 0.25, fi, false);
            }
            let letter = ((b'A' + r as u8) as char).to_string();
            d.text(p((FRAME_X0 + AREA_X0) / 2.0, y + rh / 2.0), &letter, 2.5, Anchor::Center, VAnchor::Middle, Ink::FrameText);
        }
    }
    draw_title_block(d, prj, page_i);
}

fn draw_title_block(d: &mut dyn Draw, prj: &Project, page_i: usize) {
    let form = crate::forms::title_block(prj, page_i);
    crate::forms::draw(d, &form, prj, page_i);
}

fn draw_cover(d: &mut dyn Draw, prj: &Project) {
    let info = &prj.info;
    let cx = PAGE_W / 2.0;
    d.text(p(cx, 80.0), &info.name, 10.0, Anchor::Center, VAnchor::Middle, Ink::Text);
    for (i, l) in info.description.lines().enumerate() {
        d.text(p(cx, 100.0 + i as f32 * 8.0), l, 5.0, Anchor::Center, VAnchor::Middle, Ink::Text);
    }
    let rows = [
        (tr("Company"), info.company.as_str()),
        (tr("Created by"), info.creator.as_str()),
        (tr("Date"), info.date.as_str()),
        (tr("Revision"), info.revision.as_str()),
    ];
    let pages = prj.pages.len().to_string();
    let rows = rows.iter().copied().chain(std::iter::once((tr("Number of pages"), pages.as_str())));
    for (i, (k, v)) in rows.enumerate() {
        let y = 160.0 + i as f32 * 9.0;
        d.text(p(cx - 5.0, y), k, 3.5, Anchor::Right, VAnchor::Middle, Ink::FrameText);
        d.text(p(cx + 5.0, y), v, 3.5, Anchor::Left, VAnchor::Middle, Ink::Text);
    }
    d.rect(p(110.0, 150.0), p(310.0, 150.0 + 5.0 * 9.0 + 1.0), 0.25, Ink::Frame, false);
}

pub const REPORT_ROWS_PER_PAGE: usize = 40;

fn draw_report(d: &mut dyn Draw, prj: &Project, an: &Analysis, kind: ReportKind) {
    let Some(table) = an.reports.get(&kind) else { return };
    let form = prj.forms.reports.get(&kind);
    let title = form.map(|f| f.title.as_str()).filter(|t| !t.is_empty()).unwrap_or(kind.title());
    d.text(p(22.0, 24.0), title, 6.0, Anchor::Left, VAnchor::Middle, Ink::Text);
    let cols = crate::forms::report_columns(form, &table.headers);
    let per_page = form.map(|f| f.rows).filter(|r| *r > 0).unwrap_or(REPORT_ROWS_PER_PAGE);
    let (x0, x1) = (22.0, 400.0);
    let total_w: f32 = cols.iter().map(|&c| table.widths[c]).sum();
    let mut xs = vec![x0];
    for &c in &cols {
        xs.push(xs.last().unwrap() + table.widths[c] / total_w * (x1 - x0));
    }
    // Rows get lower (and their text smaller) when more of them must fit.
    let row_h = (5.0f32).min((TITLE_Y0 - 12.0 - 39.0) / per_page as f32).max(2.0);
    let k = row_h / 5.0;
    let mut y = 34.0;
    for (i, &c) in cols.iter().enumerate() {
        d.text(p(xs[i] + 1.2, y + 2.5), table.headers[c], 2.4, Anchor::Left, VAnchor::Middle, Ink::Text);
    }
    y += 5.0;
    d.line(p(x0, y), p(x1, y), 0.5, Ink::Frame, false);
    for (row, target) in table.rows.iter().zip(&table.targets).take(per_page) {
        if let Some((page, _)) = target {
            d.link(p(x0, y), p(x1, y + row_h), *page);
        }
        for (i, &c) in cols.iter().enumerate() {
            let cell = row.get(c).map(String::as_str).unwrap_or("");
            let maxc = ((xs[i + 1] - xs[i]) / (2.2 * k * 0.6)) as usize;
            let txt: String = if cell.chars().count() > maxc && maxc > 1 {
                cell.chars().take(maxc - 1).collect::<String>() + "…"
            } else {
                cell.to_string()
            };
            d.text(p(xs[i] + 1.2, y + row_h / 2.0), &txt, 2.2 * k, Anchor::Left, VAnchor::Middle, Ink::Text);
        }
        y += row_h;
        d.line(p(x0, y), p(x1, y), 0.1, Ink::Frame, false);
    }
    for x in &xs {
        d.line(p(*x, 34.0), p(*x, y), 0.15, Ink::Frame, false);
    }
    d.rect(p(x0, 34.0), p(x1, y), 0.3, Ink::Frame, false);
    if table.rows.len() > per_page {
        let more = tr_fmt!("… {} more rows – see Reports window / CSV export", table.rows.len() - per_page);
        d.text(p(x0, y + 5.0), &more, 2.5, Anchor::Left, VAnchor::Middle, Ink::FrameText);
    }
    if table.rows.is_empty() {
        d.text(p(x0 + 2.0, y + 5.0), "(no entries)", 2.5, Anchor::Left, VAnchor::Middle, Ink::FrameText);
    }
}

// ---------------------------------------------------------------------- panel layout

/// Draws an enclosure in front view: its mounting plate or its door (seen from the
/// front). `origin` is the page position of the enclosure's top left corner, `k` the
/// scale (page mm per real mm). From the front, the cable ducts behind the door are
/// hidden (dashed); `inside` draws the door as seen from the inside (the caller
/// mirrors it): ducts as they are, door devices from the rear.
#[allow(clippy::too_many_arguments)]
pub fn draw_enclosure(
    d: &mut dyn Draw,
    e: &Enclosure,
    face: Face,
    inside: bool,
    origin: P,
    k: f32,
    selected: &BTreeSet<u64>,
    problems: &BTreeSet<u64>,
    prj: &Project,
) {
    let parts = &prj.parts[..];
    let m = face.origin();
    let tf = |x: f32, y: f32| origin + p((x + m) * k, (y + m) * k);
    let (vw, vh) = e.view_size(face);
    d.rect(origin, origin + p(vw * k, vh * k), 0.5, Ink::Line, false);
    match face {
        Face::Plate => {
            let (pw, ph) = e.plate_size();
            d.rect(tf(0.0, 0.0), tf(pw, ph), 0.0, Ink::Plate, true);
            d.rect(tf(0.0, 0.0), tf(pw, ph), 0.25, Ink::Frame, false);
        }
        Face::Door => {
            // Door frame, hinges on the left, handle on the right.
            let f = panel::DOOR_FRAME;
            let (a, b) = (tf(f, f), tf(e.width - f, e.height - f));
            for (s, t) in [(a, p(b.x, a.y)), (p(b.x, a.y), b), (b, p(a.x, b.y)), (p(a.x, b.y), a)] {
                d.line(s, t, 0.15, Ink::Frame, true);
            }
            for y in [0.15, 0.85] {
                let hy = e.height * y;
                d.rect(tf(-3.0, hy - 25.0), tf(3.0, hy + 25.0), 0.25, Ink::Frame, false);
            }
            let hy = e.height / 2.0;
            d.rect(tf(e.width - 32.0, hy - 50.0), tf(e.width - 14.0, hy + 50.0), 0.0, Ink::Fill, true);
            d.rect(tf(e.width - 32.0, hy - 50.0), tf(e.width - 14.0, hy + 50.0), 0.25, Ink::Line, false);
        }
        wall => {
            // The sheet, its mounting area inside the flanges (dashed) and, for walls
            // that are a separate component, the outline of that part.
            d.rect(origin, origin + p(vw * k, vh * k), 0.0, Ink::Plate, true);
            let a = panel::mounting_area(e, wall);
            let (s, t) = (tf(a.min.x, a.min.y), tf(a.max.x, a.max.y));
            for (u, v) in [(s, p(t.x, s.y)), (p(t.x, s.y), t), (t, p(s.x, t.y)), (p(s.x, t.y), s)] {
                d.line(u, v, 0.15, Ink::Frame, true);
            }
            let open = (wall == Face::Left && e.build.left_open) || (wall == Face::Right && e.build.right_open);
            if open {
                d.line(origin, origin + p(vw * k, vh * k), 0.15, Ink::Frame, false);
                d.line(origin + p(0.0, vh * k), origin + p(vw * k, 0.0), 0.15, Ink::Frame, false);
                d.text(origin + p(vw * k / 2.0, vh * k / 2.0), tr("bayed (open)"), 3.0 * k.max(0.2), Anchor::Center, VAnchor::Middle, Ink::Text);
            }
            if wall == Face::Bottom && e.build.gland_plate {
                let g = 3.0 * e.flange();
                if vw > 2.0 * g && vh > 2.0 * g {
                    d.rect(tf(g, g / 2.0), tf(vw - g, vh - g / 2.0), 0.2, Ink::Frame, false);
                }
            }
        }
    }
    // Cutouts first, then rails, ducts, busbars, devices on top.
    let mut items: Vec<&PanelItem> = e.items.iter().filter(|i| i.face == face).collect();
    items.sort_by_key(|i| i.kind.layer());
    for it in items {
        let ink = if selected.contains(&it.id) {
            Ink::Selected
        } else if problems.contains(&it.id) {
            Ink::Error
        } else {
            Ink::Line
        };
        let (a, b) = (tf(it.x, it.y), tf(it.x + it.w, it.y + it.h));
        let horizontal = it.horizontal();
        let len = it.length();
        match it.kind {
            ItemKind::Rail => {
                d.rect(a, b, 0.0, Ink::Plate, true);
                d.rect(a, b, 0.2, ink, false);
                // Mounting slots
                let mut t = 12.5;
                while t + 10.0 < len {
                    let (s0, s1) = if horizontal {
                        (tf(it.x + t, it.y + it.h / 2.0), tf(it.x + t + 10.0, it.y + it.h / 2.0))
                    } else {
                        (tf(it.x + it.w / 2.0, it.y + t), tf(it.x + it.w / 2.0, it.y + t + 10.0))
                    };
                    d.line(s0, s1, 0.15, Ink::Frame, false);
                    t += 25.0;
                }
            }
            ItemKind::Duct if face == Face::Door && !inside => {
                let ink = if ink == Ink::Line { Ink::Frame } else { ink };
                for (s, t) in [(a, p(b.x, a.y)), (p(b.x, a.y), b), (b, p(a.x, b.y)), (p(a.x, b.y), a)] {
                    d.line(s, t, 0.2, ink, true);
                }
            }
            ItemKind::Duct => {
                d.rect(a, b, 0.0, Ink::Fill, true);
                d.rect(a, b, 0.25, ink, false);
                // Comb of the duct
                let mut t = 10.0;
                while t < len {
                    let pairs = if horizontal {
                        [
                            (tf(it.x + t, it.y), tf(it.x + t, it.y + it.h * 0.25)),
                            (tf(it.x + t, it.y + it.h * 0.75), tf(it.x + t, it.y + it.h)),
                        ]
                    } else {
                        [
                            (tf(it.x, it.y + t), tf(it.x + it.w * 0.25, it.y + t)),
                            (tf(it.x + it.w * 0.75, it.y + t), tf(it.x + it.w, it.y + t)),
                        ]
                    };
                    for (s0, s1) in pairs {
                        d.line(s0, s1, 0.12, Ink::Frame, false);
                    }
                    t += 10.0;
                }
            }
            ItemKind::Busbar => {
                d.rect(a, b, 0.0, Ink::Plate, true);
                d.rect(a, b, 0.2, ink, false);
                // The bars, evenly spaced across the system.
                let n = crate::production::bar_count(parts, &it.part);
                let across = if horizontal { it.h } else { it.w };
                let pitch = across / n as f32;
                for k in 0..n {
                    let off = pitch * (k as f32 + 0.5);
                    let half = (pitch * 0.18).min(3.0);
                    let (r0, r1) = if horizontal {
                        (tf(it.x, it.y + off - half), tf(it.x + it.w, it.y + off + half))
                    } else {
                        (tf(it.x + off - half, it.y), tf(it.x + off + half, it.y + it.h))
                    };
                    d.rect(r0, r1, 0.0, Ink::Fill, true);
                    d.rect(r0, r1, 0.15, ink, false);
                }
            }
            ItemKind::Cutout => {
                let line_ink = if ink == Ink::Line { Ink::Frame } else { ink };
                if it.round {
                    let c = tf(it.x + it.w / 2.0, it.y + it.h / 2.0);
                    d.circle(c, it.w / 2.0 * k, 0.3, ink, false);
                    let r = it.w / 2.0 * k * std::f32::consts::FRAC_1_SQRT_2;
                    d.line(c + p(-r, -r), c + p(r, r), 0.12, line_ink, false);
                    d.line(c + p(-r, r), c + p(r, -r), 0.12, line_ink, false);
                } else {
                    d.rect(a, b, 0.3, ink, false);
                    d.line(a, b, 0.12, line_ink, false);
                    d.line(p(a.x, b.y), p(b.x, a.y), 0.12, line_ink, false);
                }
            }
            ItemKind::Accessory => {
                // Accessories: their outline with the part number.
                d.rect(a, b, 0.0, Ink::Fill, true);
                let text_ink = if ink == Ink::Line { Ink::Text } else { ink };
                let size = (it.w.min(it.h) * 0.18).clamp(2.0, 7.0) * k;
                d.text(tf(it.x + it.w / 2.0, it.y + it.h / 2.0), it.part.trim_start_matches("GEN."), size, Anchor::Center, VAnchor::Middle, text_ink);
                d.rect(a, b, 0.25, ink, false);
                d.line(a, p(a.x + (b.x - a.x).min(b.y - a.y) * 0.3, a.y), 0.15, ink, false);
            }
            ItemKind::Device if inside => {
                // Rear of a door device: its box and tag.
                d.rect(a, b, 0.0, Ink::Fill, true);
                let size = (it.w.min(it.h) * 0.22).clamp(3.0, 9.0) * k;
                let text_ink = if ink == Ink::Line { Ink::Text } else { ink };
                d.text(tf(it.x + it.w / 2.0, it.y + it.h / 2.0), &it.tag, size, Anchor::Center, VAnchor::Middle, text_ink);
                d.rect(a, b, 0.3, ink, false);
            }
            ItemKind::Device => {
                d.rect(a, b, 0.0, Ink::Fill, true);
                let model = parts.iter().find(|x| x.number == it.part).and_then(|x| x.model_2d.as_ref());
                let text_ink = if ink == Ink::Line { Ink::Text } else { ink };
                let size = (it.w.min(it.h) * 0.22).clamp(3.0, 9.0) * k;
                let c = tf(it.x + it.w / 2.0, it.y + it.h / 2.0);
                // Terminal strips: every element of the strip in its place.
                let cells = panel::strip_cells(prj, it);
                match model {
                    Some(_) if !cells.is_empty() => {
                        draw_strip_2d(d, prj, it, &cells, &tf, k, ink);
                        let size = size * 0.8;
                        let half = p(it.tag.chars().count() as f32 * size * 0.33 + size * 0.2, size * 0.6);
                        d.rect(c - half, c + half, 0.0, Ink::Plate, true);
                        d.text(c, &it.tag, size, Anchor::Center, VAnchor::Middle, text_ink);
                    }
                    Some(m) => {
                        draw_model_2d(d, m, it, &tf, k, ink);
                        // Tag on a small label so it stays readable on the drawing
                        let size = size * 0.8;
                        let half = p(it.tag.chars().count() as f32 * size * 0.33 + size * 0.2, size * 0.6);
                        d.rect(c - half, c + half, 0.0, Ink::Plate, true);
                        d.text(c, &it.tag, size, Anchor::Center, VAnchor::Middle, text_ink);
                    }
                    None => d.text(c, &it.tag, size, Anchor::Center, VAnchor::Middle, text_ink),
                }
                d.rect(a, b, 0.3, ink, false);
            }
        }
    }
    // Cutouts of door devices and accessories (dashed: under the item)
    {
        let under = |o: &crate::production::Opening| e.item(o.item).map(|i| matches!(i.kind, ItemKind::Device | ItemKind::Accessory)).unwrap_or(false);
        for o in crate::production::openings(e, parts, face).iter().filter(|o| under(o)) {
            let c = tf(o.center.x, o.center.y);
            if o.round {
                d.circle(c, o.w / 2.0 * k, 0.15, Ink::Frame, false);
            } else {
                let half = p(o.w / 2.0 * k, o.h / 2.0 * k);
                let (a, b) = (c - half, c + half);
                for (s, t) in [(a, p(b.x, a.y)), (p(b.x, a.y), b), (b, p(a.x, b.y)), (p(a.x, b.y), a)] {
                    d.line(s, t, 0.15, Ink::Frame, true);
                }
            }
        }
    }
    // Drilling pattern (holes hidden under other devices are left out); weld studs as
    // a filled dot.
    for h in crate::production::holes_on(e, parts, face) {
        let covered = e.items.iter().any(|i| i.kind == ItemKind::Device && i.face == face && i.id != h.item && i.rect().contains(h.pos));
        if covered {
            continue;
        }
        let c = tf(h.pos.x, h.pos.y);
        let stud = h.fastening == panel::Fastening::Stud;
        d.circle(c, (h.diameter / 2.0 * k).max(0.25), if stud { 0.0 } else { 0.1 }, Ink::Frame, stud);
    }
}

/// Draws a part's 2D front view into a placed device. Rotated devices (width and
/// height swapped) get the drawing turned by 90°, other size differences are scaled;
/// a strip of equal parts repeats it.
fn draw_model_2d(d: &mut dyn Draw, m: &Model2d, it: &PanelItem, tf: &dyn Fn(f32, f32) -> P, k: f32, ink: Ink) {
    if m.width <= 0.0 || m.height <= 0.0 {
        return;
    }
    let rotated = panel::item_rotated(it, m.height);
    let n = if rotated { 1 } else { panel::tiles(it.w, m.width) };
    let cell = (if rotated { it.h } else { it.w }) / n as f32;
    for tile in 0..n {
        draw_model_2d_in(d, m, it, tf, k, ink, tile as f32 * cell, cell);
    }
}

/// A part's 2D front view in the stretch `at .. at + len` along a device.
#[allow(clippy::too_many_arguments)]
fn draw_model_2d_in(d: &mut dyn Draw, m: &Model2d, it: &PanelItem, tf: &dyn Fn(f32, f32) -> P, k: f32, ink: Ink, at: f32, len: f32) {
    if m.width <= 0.0 || m.height <= 0.0 {
        return;
    }
    let rotated = panel::item_rotated(it, m.height);
    let (sx, sy) = if rotated { (it.w / m.height, len / m.width) } else { (len / m.width, it.h / m.height) };
    let s = (sx + sy) / 2.0;
    let map = |q: [f32; 2]| {
        if rotated {
            tf(it.x + (m.height - q[1]) * sx, it.y + at + q[0] * sy)
        } else {
            tf(it.x + at + q[0] * sx, it.y + q[1] * sy)
        }
    };
    draw_prims_2d(d, &m.prims, &map, s * k, 0.15, ink);
}

/// The elements of a terminal strip: each part's front view in its cell, a plain
/// outline where a part has none.
#[allow(clippy::too_many_arguments)]
fn draw_strip_2d(d: &mut dyn Draw, prj: &Project, it: &PanelItem, cells: &[panel::Cell], tf: &dyn Fn(f32, f32) -> P, k: f32, ink: Ink) {
    let rotated = prj.parts.iter().chain(parts::catalog()).find(|x| x.number == it.part).and_then(|x| x.size()).is_some_and(|s| panel::item_rotated(it, s[1]));
    for c in cells {
        match prj.parts.iter().chain(parts::catalog()).find(|x| x.number == c.part).and_then(|x| x.model_2d.as_ref()) {
            Some(m) => draw_model_2d_in(d, m, it, tf, k, ink, c.at, c.len),
            None => {
                let (a, b) = if rotated {
                    (tf(it.x, it.y + c.at), tf(it.x + it.w, it.y + c.at + c.len))
                } else {
                    (tf(it.x + c.at, it.y), tf(it.x + c.at + c.len, it.y + it.h))
                };
                d.rect(a, b, 0.15, ink, false);
            }
        }
    }
}

pub fn draw_prims_2d(d: &mut dyn Draw, prims: &[Prim2d], map: &dyn Fn([f32; 2]) -> P, s: f32, w: f32, ink: Ink) {
    for prim in prims {
        match prim {
            Prim2d::Line { a, b } => d.line(map(*a), map(*b), w, ink, false),
            Prim2d::Rect { min, max, fill } => {
                let corners = [*min, [max[0], min[1]], *max, [min[0], max[1]]].map(map);
                d.poly(&corners, true, if *fill { 0.0 } else { w }, ink, *fill);
            }
            Prim2d::Circle { c, r, fill } => d.circle(map(*c), r * s, if *fill { 0.0 } else { w }, ink, *fill),
            Prim2d::Polyline { pts, closed, fill } => {
                let v: Vec<P> = pts.iter().map(|q| map(*q)).collect();
                d.poly(&v, *closed, if *fill { 0.0 } else { w }, ink, *fill && *closed);
            }
            Prim2d::Text { pos, text, size } => {
                let ink = if ink == Ink::Line { Ink::Text } else { ink };
                d.text(map(*pos), text, size * s, Anchor::Left, VAnchor::Middle, ink)
            }
        }
    }
}

/// "Nice" drawing scale 1:N so that `real` fits into `space`.
pub fn drawing_scale(real: (f32, f32), space: (f32, f32)) -> f32 {
    let needed = (real.0 / space.0).max(real.1 / space.1);
    [1.0, 2.0, 2.5, 4.0, 5.0, 10.0, 15.0, 20.0, 25.0, 50.0, 100.0]
        .into_iter()
        .find(|n| *n >= needed)
        .unwrap_or(needed.ceil())
}

fn draw_mounting_page(d: &mut dyn Draw, prj: &Project, id: u64) {
    let Some(e) = prj.layout.enclosures.iter().find(|e| e.id == id) else {
        let msg = tr("Enclosure not found - choose another one in the page properties");
        d.text(p(30.0, 40.0), msg, 4.0, Anchor::Left, VAnchor::Middle, Ink::Error);
        return;
    };
    let (x0, y0, x1, y1) = (AREA_X0 + 25.0, AREA_Y0 + 12.0, FRAME_X1 - 15.0, TITLE_Y0 - 22.0);
    // With door devices the door is drawn to the right of the mounting plate.
    let door = e.items.iter().any(|i| i.face == Face::Door);
    let gap = if door { e.width * 0.25 } else { 0.0 };
    let total_w = if door { 2.0 * e.width + gap } else { e.width };
    let n = drawing_scale((total_w, e.height), (x1 - x0, y1 - y0));
    let k = 1.0 / n;
    let (w, h) = (e.width * k, e.height * k);
    let origin = p(x0 + ((x1 - x0) - total_w * k) / 2.0, y0 + ((y1 - y0) - h) / 2.0);
    draw_enclosure(d, e, Face::Plate, false, origin, k, &BTreeSet::new(), &BTreeSet::new(), prj);
    if door {
        let o = origin + p(w + gap * k, 0.0);
        draw_enclosure(d, e, Face::Door, false, o, k, &BTreeSet::new(), &BTreeSet::new(), prj);
        d.text(p(origin.x + w / 2.0, origin.y - 3.0), tr("Mounting plate"), 3.0, Anchor::Center, VAnchor::Bottom, Ink::Text);
        d.text(p(o.x + w / 2.0, o.y - 3.0), tr("Door (front view)"), 3.0, Anchor::Center, VAnchor::Bottom, Ink::Text);
    }
    // Dimensions
    let bottom = origin.y + h + 6.0;
    d.line(p(origin.x, bottom), p(origin.x + w, bottom), 0.18, Ink::Frame, false);
    for x in [origin.x, origin.x + w] {
        d.line(p(x, bottom - 2.0), p(x, bottom + 2.0), 0.18, Ink::Frame, false);
    }
    d.text(p(origin.x + w / 2.0, bottom - 1.5), &format!("{:.0}", e.width), 2.5, Anchor::Center, VAnchor::Bottom, Ink::Text);
    let left = origin.x - 6.0;
    d.line(p(left, origin.y), p(left, origin.y + h), 0.18, Ink::Frame, false);
    for y in [origin.y, origin.y + h] {
        d.line(p(left - 2.0, y), p(left + 2.0, y), 0.18, Ink::Frame, false);
    }
    d.text(p(left - 1.5, origin.y + h / 2.0), &format!("{:.0}", e.height), 2.5, Anchor::Right, VAnchor::Middle, Ink::Text);
    let caption = format!(
        "{}  {}  -  {:.0} x {:.0} x {:.0} mm  -  Scale 1:{}",
        e.name, e.part, e.width, e.height, e.depth, n
    );
    d.text(p(x0, TITLE_Y0 - 8.0), &caption, 3.0, Anchor::Left, VAnchor::Middle, Ink::Text);
}
