//! Backend independent page rendering. The same code draws to the egui canvas and
//! to SVG export, so what you see is what you print.

use crate::geom::{p, P, R};
use crate::index::{Index, Loc};
use crate::model::*;
use crate::netlist;
use crate::panel::{self, Enclosure, ItemKind, PanelItem};
use crate::parts::{self, FnKind};
use crate::reports::Analysis;
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
}

/// Draws a complete page including frame and title block.
pub fn draw_page(d: &mut dyn Draw, prj: &Project, idx: &Index, an: &Analysis, page_i: usize, opts: &Opts) {
    let page = &prj.pages[page_i];
    draw_frame(d, prj, page_i);
    match page.kind {
        PageKind::Cover => draw_cover(d, prj),
        PageKind::Report(kind) => draw_report(d, an, kind),
        PageKind::Mounting(id) => draw_mounting_page(d, prj, id),
        PageKind::Schematic => {}
    }
    for s in &page.shapes {
        let ink = if opts.selected.contains(&s.id) { Ink::Selected } else { Ink::Line };
        draw_shape(d, s, ink);
    }
    for w in &page.wires {
        let ink = if opts.selected.contains(&w.id) {
            Ink::Selected
        } else if opts.highlight_wires.contains(&w.id) {
            Ink::Highlight
        } else {
            Ink::Wire
        };
        d.line(w.a, w.b, WIRE_W, ink, false);
    }
    for j in netlist::junctions(page) {
        d.circle(j, 0.7, 0.0, Ink::Wire, true);
    }
    for s in &page.symbols {
        let Some(def) = symbols::get(&s.sym) else { continue };
        let ink = if opts.selected.contains(&s.id) { Ink::Selected } else { Ink::Line };
        draw_symbol(d, def, s, ink);
        draw_annotations(d, prj, idx, def, s, ink);
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
    let tf = |q: P| s.pos + q.transform(s.rot, s.mirror);
    let text_ink = if ink == Ink::Line { Ink::Text } else { ink };
    for prim in &def.prims {
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

/// Device tag, function text, connection point designations and cross references.
pub fn draw_annotations(d: &mut dyn Draw, prj: &Project, idx: &Index, def: &SymbolDef, s: &SymbolInst, ink: Ink) {
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
    let mut refs: Vec<String> = Vec::new();
    let dev = idx.devices.get(&s.tag);
    match def.func {
        Func::Aux(_) => match dev.map(|d| d.mains.as_slice()) {
            Some([]) | None => refs.push("?".into()),
            Some(mains) => refs.extend(mains.iter().map(|m| m.reference(prj))),
        },
        Func::Main { mirror: false } => {
            if let Some(dev) = dev {
                refs.extend(dev.auxs.iter().map(|a| a.reference(prj)));
            }
        }
        Func::Interruption => {
            if let Some(locs) = idx.interruptions.get(&s.label) {
                let others: Vec<String> =
                    locs.iter().filter(|l| l.sym_id != s.id).map(|l| l.reference(prj)).collect();
                let q = s.pos + p(2.2, 1.6).transform(s.rot, s.mirror);
                let txt = if others.is_empty() { "?".to_string() } else { others.join(" ") };
                let ink = if others.is_empty() { Ink::Error } else { Ink::CrossRef };
                d.text(q + p(0.0, 2.6), &txt, SMALL_SIZE, Anchor::Left, VAnchor::Middle, ink);
            }
        }
        _ => {}
    }

    // Tag block
    if def.show_tag {
        let mut lines: Vec<(String, f32, Ink)> = Vec::new();
        if !s.tag.is_empty() {
            lines.push((s.tag.clone(), TAG_SIZE, text_ink));
        }
        if !s.function_text.is_empty() {
            for l in s.function_text.lines() {
                lines.push((l.to_string(), 2.0, text_ink));
            }
        }
        for r in refs {
            let ink = if r == "?" { Ink::Error } else { Ink::CrossRef };
            lines.push((r, 2.0, ink));
        }
        let total: f32 = lines.iter().map(|l| l.1 * 1.35).sum();
        if vertical {
            let x = bounds.min.x - 1.2;
            let mut y = bounds.center().y - total / 2.0;
            for (t, size, ink) in &lines {
                d.text(p(x, y + size * 0.675), t, *size, Anchor::Right, VAnchor::Middle, *ink);
                y += size * 1.35;
            }
        } else {
            let x = bounds.center().x;
            let mut y = bounds.min.y - 1.0 - total;
            for (t, size, ink) in &lines {
                d.text(p(x, y + size * 0.675), t, *size, Anchor::Center, VAnchor::Middle, *ink);
                y += size * 1.35;
            }
        }
    }

    // Contact image (mirror) below coils
    if matches!(def.func, Func::Main { mirror: true }) {
        if let Some(dev) = dev {
            draw_contact_image(d, prj, &s.tag, dev.auxs.as_slice(), &bounds, vertical, text_ink);
        }
    }
}

fn draw_contact_image(d: &mut dyn Draw, prj: &Project, tag: &str, auxs: &[Loc], bounds: &R, vertical: bool, ink: Ink) {
    // Rows: contact kind, connection designations, cross reference (None = reserve).
    let mut rows: Vec<(ContactKind, Vec<String>, Option<String>)> = Vec::new();
    for loc in auxs {
        let (Some(def), Some(inst)) = (find_symbol(prj, loc), find_inst(prj, loc)) else { continue };
        let kind = def.contact_kind().unwrap_or(ContactKind::No);
        rows.push((kind, inst.pins.clone(), Some(loc.reference(prj))));
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
            Some(r) => d.text(p(rx, y), &r, SMALL_SIZE, Anchor::Left, VAnchor::Middle, Ink::CrossRef),
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
    let page = &prj.pages[page_i];
    let info = &prj.info;
    let fi = Ink::Frame;
    let (y0, y1) = (TITLE_Y0, FRAME_Y1);
    let ym = (y0 + y1) / 2.0;
    d.line(p(FRAME_X0, y0), p(FRAME_X1, y0), 0.5, fi, false);
    let cols = [FRAME_X0, 90.0, 150.0, 210.0, 330.0, 370.0, FRAME_X1];
    for &x in &cols[1..cols.len() - 1] {
        d.line(p(x, y0), p(x, y1), 0.25, fi, false);
    }
    for (x0, x1) in [(90.0, 150.0), (150.0, 210.0), (330.0, 370.0), (370.0, FRAME_X1)] {
        d.line(p(x0, ym), p(x1, ym), 0.25, fi, false);
    }
    let lbl = |d: &mut dyn Draw, x: f32, y: f32, t: &str| {
        d.text(p(x + 1.0, y + 1.8), t, 1.6, Anchor::Left, VAnchor::Middle, Ink::FrameText)
    };
    let val = |d: &mut dyn Draw, x: f32, y: f32, t: &str, size: f32| {
        d.text(p(x + 2.0, y), t, size, Anchor::Left, VAnchor::Middle, Ink::Text)
    };
    lbl(d, FRAME_X0, y0, "Company");
    val(d, FRAME_X0, ym + 2.0, &info.company, 4.0);
    lbl(d, 90.0, y0, "Date");
    val(d, 90.0, y0 + 9.0, &info.date, 2.5);
    lbl(d, 90.0, ym, "Creator");
    val(d, 90.0, ym + 9.0, &info.creator, 2.5);
    lbl(d, 150.0, y0, "Revision");
    val(d, 150.0, y0 + 9.0, &info.revision, 2.5);
    lbl(d, 150.0, ym, "Software");
    val(d, 150.0, ym + 9.0, "OpenECraft", 2.5);
    lbl(d, 210.0, y0, "Project / page description");
    val(d, 210.0, y0 + 9.0, &info.name, 3.5);
    val(d, 210.0, y0 + 21.0, &page.description, 3.0);
    lbl(d, 330.0, y0, "Page");
    val(d, 330.0, y0 + 9.5, &page.name, 4.0);
    lbl(d, 330.0, ym, "Type");
    let kind = match page.kind {
        PageKind::Schematic => "Schematic",
        PageKind::Cover => "Cover",
        PageKind::Report(_) => "Report",
        PageKind::Mounting(_) => "Mounting",
    };
    val(d, 330.0, ym + 9.0, kind, 2.5);
    lbl(d, 370.0, y0, "Sheet");
    val(d, 370.0, y0 + 9.5, &format!("{} / {}", page_i + 1, prj.pages.len()), 3.5);
    lbl(d, 370.0, ym, "Project");
    val(d, 370.0, ym + 9.0, &info.name, 2.2);
}

fn draw_cover(d: &mut dyn Draw, prj: &Project) {
    let info = &prj.info;
    let cx = PAGE_W / 2.0;
    d.text(p(cx, 80.0), &info.name, 10.0, Anchor::Center, VAnchor::Middle, Ink::Text);
    for (i, l) in info.description.lines().enumerate() {
        d.text(p(cx, 100.0 + i as f32 * 8.0), l, 5.0, Anchor::Center, VAnchor::Middle, Ink::Text);
    }
    let rows = [
        ("Company", info.company.as_str()),
        ("Created by", info.creator.as_str()),
        ("Date", info.date.as_str()),
        ("Revision", info.revision.as_str()),
    ];
    let pages = prj.pages.len().to_string();
    let rows = rows.iter().copied().chain(std::iter::once(("Number of pages", pages.as_str())));
    for (i, (k, v)) in rows.enumerate() {
        let y = 160.0 + i as f32 * 9.0;
        d.text(p(cx - 5.0, y), k, 3.5, Anchor::Right, VAnchor::Middle, Ink::FrameText);
        d.text(p(cx + 5.0, y), v, 3.5, Anchor::Left, VAnchor::Middle, Ink::Text);
    }
    d.rect(p(110.0, 150.0), p(310.0, 150.0 + 5.0 * 9.0 + 1.0), 0.25, Ink::Frame, false);
}

pub const REPORT_ROWS_PER_PAGE: usize = 40;

fn draw_report(d: &mut dyn Draw, an: &Analysis, kind: ReportKind) {
    let Some(table) = an.reports.get(&kind) else { return };
    d.text(p(22.0, 24.0), kind.title(), 6.0, Anchor::Left, VAnchor::Middle, Ink::Text);
    let (x0, x1) = (22.0, 400.0);
    let total_w: f32 = table.widths.iter().sum();
    let mut xs = vec![x0];
    for w in &table.widths {
        xs.push(xs.last().unwrap() + w / total_w * (x1 - x0));
    }
    let row_h = 5.0;
    let mut y = 34.0;
    for (i, h) in table.headers.iter().enumerate() {
        d.text(p(xs[i] + 1.2, y + row_h / 2.0), h, 2.4, Anchor::Left, VAnchor::Middle, Ink::Text);
    }
    y += row_h;
    d.line(p(x0, y), p(x1, y), 0.5, Ink::Frame, false);
    for row in table.rows.iter().take(REPORT_ROWS_PER_PAGE) {
        for (i, cell) in row.iter().enumerate() {
            let maxc = ((xs[i + 1] - xs[i]) / (2.2 * 0.6)) as usize;
            let txt: String = if cell.chars().count() > maxc && maxc > 1 {
                cell.chars().take(maxc - 1).collect::<String>() + "…"
            } else {
                cell.clone()
            };
            d.text(p(xs[i] + 1.2, y + row_h / 2.0), &txt, 2.2, Anchor::Left, VAnchor::Middle, Ink::Text);
        }
        y += row_h;
        d.line(p(x0, y), p(x1, y), 0.1, Ink::Frame, false);
    }
    for x in &xs {
        d.line(p(*x, 34.0), p(*x, y), 0.15, Ink::Frame, false);
    }
    d.rect(p(x0, 34.0), p(x1, y), 0.3, Ink::Frame, false);
    if table.rows.len() > REPORT_ROWS_PER_PAGE {
        let more = format!("… {} more rows – see Reports window / CSV export", table.rows.len() - REPORT_ROWS_PER_PAGE);
        d.text(p(x0, y + 5.0), &more, 2.5, Anchor::Left, VAnchor::Middle, Ink::FrameText);
    }
    if table.rows.is_empty() {
        d.text(p(x0 + 2.0, y + 5.0), "(no entries)", 2.5, Anchor::Left, VAnchor::Middle, Ink::FrameText);
    }
}

// ---------------------------------------------------------------------- panel layout

/// Draws an enclosure with its mounting plate in front view. `origin` is the page
/// position of the enclosure's top left corner, `k` the scale (page mm per real mm).
pub fn draw_enclosure(
    d: &mut dyn Draw,
    e: &Enclosure,
    origin: P,
    k: f32,
    selected: &BTreeSet<u64>,
    problems: &BTreeSet<u64>,
) {
    let tf = |x: f32, y: f32| origin + p((x + panel::PLATE_MARGIN) * k, (y + panel::PLATE_MARGIN) * k);
    d.rect(origin, origin + p(e.width * k, e.height * k), 0.5, Ink::Line, false);
    let (pw, ph) = e.plate_size();
    d.rect(tf(0.0, 0.0), tf(pw, ph), 0.0, Ink::Plate, true);
    d.rect(tf(0.0, 0.0), tf(pw, ph), 0.25, Ink::Frame, false);
    // Rails first, then ducts, devices on top.
    let mut items: Vec<&PanelItem> = e.items.iter().collect();
    items.sort_by_key(|i| match i.kind {
        ItemKind::Rail => 0,
        ItemKind::Duct => 1,
        ItemKind::Device => 2,
    });
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
            ItemKind::Device => {
                d.rect(a, b, 0.0, Ink::Fill, true);
                d.rect(a, b, 0.3, ink, false);
                let size = (it.w.min(it.h) * 0.22).clamp(3.0, 9.0) * k;
                let text_ink = if ink == Ink::Line { Ink::Text } else { ink };
                d.text(tf(it.x + it.w / 2.0, it.y + it.h / 2.0), &it.tag, size, Anchor::Center, VAnchor::Middle, text_ink);
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
        let msg = "Enclosure not found - choose another one in the page properties";
        d.text(p(30.0, 40.0), msg, 4.0, Anchor::Left, VAnchor::Middle, Ink::Error);
        return;
    };
    let (x0, y0, x1, y1) = (AREA_X0 + 25.0, AREA_Y0 + 12.0, FRAME_X1 - 15.0, TITLE_Y0 - 22.0);
    let n = drawing_scale((e.width, e.height), (x1 - x0, y1 - y0));
    let k = 1.0 / n;
    let (w, h) = (e.width * k, e.height * k);
    let origin = p(x0 + ((x1 - x0) - w) / 2.0, y0 + ((y1 - y0) - h) / 2.0);
    draw_enclosure(d, e, origin, k, &BTreeSet::new(), &BTreeSet::new());
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
