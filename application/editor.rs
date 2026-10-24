//! Interactive schematic canvas: pan/zoom, selection, moving, placing symbols,
//! drawing wires, texts, graphics, dimensions and connection / cable definitions.

use openecraft_i18n::{tr, tr_fmt};
use crate::geom::{dist_to_segment, p, P, R, GRID};
use crate::frame::PlotFrame;
use crate::index::Index;
use crate::model::*;
use crate::ops::{self, Clip, PlaceCtx};
use crate::render::{self, Draw, Ink, Opts, VAnchor};
use crate::reports::Analysis;
use crate::review::{Hit, MarkupKind, Pen};
use crate::symbols::{self, Anchor};
use crate::{cabledef, conndef, graphics, routing};
use egui::{pos2, vec2, Align2, Color32, FontId, Key, Painter, PointerButton, Pos2, Rect, Sense, Shape as EShape, Stroke, Vec2};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Clone, Debug, PartialEq)]
pub enum Tool {
    Select,
    Place(&'static str),
    Wire,
    Text,
    Line,
    Rect,
    Circle,
    Ellipse,
    Arc,
    Polyline,
    Polygon,
    Dimension(DimKind),
    /// Connection definition point on a wire.
    ConnDef,
    /// Cable definition line across wires.
    CableDef,
    /// Place the items of `Editor::insert` (macro, image).
    Insert,
    /// Place a review comment.
    Comment,
    /// Draw redline / greenline markup.
    Markup(MarkupKind, Pen),
}

impl Tool {
    pub fn hint(&self) -> &'static str {
        match self {
            Tool::Select => tr("Click to select, drag to move, drag on empty space for window selection. Drag the tag or a property text of a selected symbol to place it. Right/middle drag pans, wheel zooms."),
            Tool::Place(_) => tr("Click to place · R rotate · M mirror · Esc finish"),
            Tool::Wire => tr("Click start and corner points · Space toggles corner direction · A toggles routing around symbols · ends automatically on a connection · Right click / Esc finish"),
            Tool::Text => tr("Click to place a text"),
            Tool::Line | Tool::Rect | Tool::Ellipse => tr("Click start point, click end point"),
            Tool::Circle => tr("Click the centre, then a point of the circle"),
            Tool::Arc => tr("Click the centre, the start point and the end point (counter-clockwise)"),
            Tool::Polyline | Tool::Polygon => tr("Click the corner points · double click, Enter or right click finishes"),
            Tool::Dimension(_) => tr("Click the two points to measure, then the position of the dimension line"),
            Tool::ConnDef => tr("Click on a wire: the point defines number, colour, cross-section and cable core of the connection"),
            Tool::CableDef => tr("Draw a line across the wires that are cores of the cable"),
            Tool::Insert => tr("Click to insert · Esc cancel"),
            Tool::Comment => tr("Click to place a comment"),
            Tool::Markup(MarkupKind::Text, _) => tr("Click to place the markup text"),
            Tool::Markup(..) => tr("Click start point, click end point"),
        }
    }

    /// Tools of the review layer: they work on every page.
    pub fn is_review(&self) -> bool {
        matches!(self, Tool::Comment | Tool::Markup(..))
    }

    /// Clicks needed by a graphic tool (None: open ended).
    fn points(&self) -> Option<usize> {
        match self {
            Tool::Line | Tool::Rect | Tool::Circle | Tool::Ellipse | Tool::CableDef => Some(2),
            Tool::Arc | Tool::Dimension(_) => Some(3),
            _ => None,
        }
    }

    fn shape_kind(&self) -> Option<ShapeKind> {
        Some(match self {
            Tool::Line => ShapeKind::Line,
            Tool::Rect => ShapeKind::Rect,
            Tool::Circle => ShapeKind::Circle,
            Tool::Ellipse => ShapeKind::Ellipse,
            Tool::Arc => ShapeKind::Arc,
            Tool::Polyline => ShapeKind::Polyline,
            Tool::Polygon => ShapeKind::Polygon,
            _ => return None,
        })
    }
}

/// Part of a symbol that can be dragged on its own: the tag block (None) or a
/// placed property (index).
type PropHandle = (u64, Option<usize>);

enum Drag {
    None,
    Move { start: P, applied: P, attached: Vec<(u64, bool)> },
    Box { start: Pos2 },
    Prop { handle: PropHandle, start: P, orig: P },
}

pub struct Palette {
    pub canvas: Color32,
    pub paper: Color32,
    pub grid: Color32,
    pub line: Color32,
    pub wire: Color32,
    pub text: Color32,
    pub frame: Color32,
    pub selected: Color32,
    pub highlight: Color32,
    pub ghost: Color32,
    pub pin: Color32,
    pub crossref: Color32,
    pub error: Color32,
    pub plate: Color32,
    pub fill: Color32,
}

impl Palette {
    pub fn new(dark: bool) -> Palette {
        if dark {
            Palette {
                canvas: Color32::from_rgb(18, 19, 22),
                paper: Color32::from_rgb(32, 34, 38),
                grid: Color32::from_rgb(70, 72, 78),
                line: Color32::from_rgb(225, 225, 225),
                wire: Color32::from_rgb(120, 175, 255),
                text: Color32::from_rgb(232, 232, 232),
                frame: Color32::from_rgb(130, 133, 140),
                selected: Color32::from_rgb(255, 150, 40),
                highlight: Color32::from_rgb(255, 80, 220),
                ghost: Color32::from_rgba_unmultiplied(80, 170, 255, 170),
                pin: Color32::from_rgb(255, 90, 90),
                crossref: Color32::from_rgb(140, 180, 255),
                error: Color32::from_rgb(255, 90, 90),
                plate: Color32::from_rgb(52, 55, 60),
                fill: Color32::from_rgb(78, 82, 90),
            }
        } else {
            Palette {
                canvas: Color32::from_rgb(190, 193, 198),
                paper: Color32::WHITE,
                grid: Color32::from_rgb(190, 190, 190),
                line: Color32::BLACK,
                wire: Color32::from_rgb(0, 40, 160),
                text: Color32::BLACK,
                frame: Color32::from_rgb(90, 90, 90),
                selected: Color32::from_rgb(235, 110, 0),
                highlight: Color32::from_rgb(210, 0, 170),
                ghost: Color32::from_rgba_unmultiplied(0, 110, 230, 170),
                pin: Color32::from_rgb(220, 0, 0),
                crossref: Color32::from_rgb(20, 60, 170),
                error: Color32::from_rgb(220, 0, 0),
                plate: Color32::from_rgb(236, 238, 241),
                fill: Color32::from_rgb(214, 218, 224),
            }
        }
    }

    fn ink(&self, ink: Ink) -> Color32 {
        match ink {
            Ink::Line => self.line,
            Ink::Wire => self.wire,
            Ink::Text => self.text,
            Ink::Frame | Ink::FrameText => self.frame,
            Ink::Selected => self.selected,
            Ink::Highlight => self.highlight,
            Ink::Ghost => self.ghost,
            Ink::Pin => self.pin,
            Ink::CrossRef => self.crossref,
            Ink::Error => self.error,
            Ink::Plate => self.plate,
            Ink::Fill => self.fill,
            Ink::Color([r, g, b]) => Color32::from_rgb(r, g, b),
        }
    }
}

/// Textures of the images on a page: image id -> (fingerprint of the file, texture;
/// None when the file cannot be decoded).
pub type Textures = HashMap<u64, (u64, Option<egui::TextureHandle>)>;

/// Renders page geometry with an egui painter.
pub struct EguiDraw<'a> {
    pub painter: &'a Painter,
    pub origin: Pos2,
    pub zoom: f32,
    pub pal: &'a Palette,
    /// Forces every element to this ink (ghost previews).
    pub force: Option<Ink>,
    /// Mirrors the drawing at `x = mirror / 2` (inside view of a door).
    pub mirror: Option<f32>,
    /// Decoded images (bitmaps are drawn as frames without).
    pub textures: Option<&'a Textures>,
}

impl EguiDraw<'_> {
    fn w2s(&self, q: P) -> Pos2 {
        let x = self.mirror.map(|m| m - q.x).unwrap_or(q.x);
        pos2(self.origin.x + x * self.zoom, self.origin.y + q.y * self.zoom)
    }
    fn color(&self, ink: Ink) -> Color32 {
        self.pal.ink(self.force.unwrap_or(ink))
    }
    fn stroke(&self, width: f32, ink: Ink) -> Stroke {
        Stroke::new((width * self.zoom).max(1.0), self.color(ink))
    }
}

impl Draw for EguiDraw<'_> {
    fn line(&mut self, a: P, b: P, width: f32, ink: Ink, dashed: bool) {
        let pts = [self.w2s(a), self.w2s(b)];
        let st = self.stroke(width, ink);
        if dashed {
            let z = self.zoom.max(0.5);
            self.painter.extend(EShape::dashed_line(&pts, st, 1.5 * z, 1.0 * z));
        } else {
            self.painter.line_segment(pts, st);
        }
    }

    fn poly(&mut self, pts: &[P], closed: bool, width: f32, ink: Ink, fill: bool) {
        let v: Vec<Pos2> = pts.iter().map(|q| self.w2s(*q)).collect();
        if fill {
            let st = if width > 0.0 { self.stroke(width, ink) } else { Stroke::NONE };
            if pts.len() > 3 && !convex(pts) {
                // Fan triangles of the even-odd filled area.
                self.painter.add(EShape::mesh(fill_mesh(pts, &v, self.color(ink))));
                if width > 0.0 {
                    self.painter.add(EShape::closed_line(v, st));
                }
            } else {
                self.painter.add(EShape::convex_polygon(v, self.color(ink), st));
            }
        } else if closed {
            self.painter.add(EShape::closed_line(v, self.stroke(width, ink)));
        } else {
            self.painter.add(EShape::line(v, self.stroke(width, ink)));
        }
    }

    fn circle(&mut self, c: P, r: f32, width: f32, ink: Ink, fill: bool) {
        let c = self.w2s(c);
        let r = (r * self.zoom).max(1.0);
        if fill {
            self.painter.circle_filled(c, r, self.color(ink));
        } else {
            self.painter.circle_stroke(c, r, self.stroke(width, ink));
        }
    }

    fn text(&mut self, pos: P, text: &str, size: f32, h: Anchor, v: VAnchor, ink: Ink) {
        let px = size * self.zoom * 1.25;
        if px < 3.0 || text.is_empty() {
            return;
        }
        let align = Align2([
            match h {
                Anchor::Left => egui::Align::Min,
                Anchor::Center => egui::Align::Center,
                Anchor::Right => egui::Align::Max,
            },
            match v {
                VAnchor::Middle => egui::Align::Center,
                VAnchor::Bottom => egui::Align::Max,
            },
        ]);
        self.painter.text(self.w2s(pos), align, text, FontId::proportional(px), self.color(ink));
    }

    fn image(&mut self, img: &ImageItem) {
        let tex = self.textures.and_then(|t| t.get(&img.id)).and_then(|t| t.1.as_ref());
        match (tex, self.force) {
            (Some(tex), None) => {
                let r = Rect::from_two_pos(self.w2s(img.a), self.w2s(img.b));
                let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                self.painter.image(tex.id(), r, uv, Color32::WHITE);
            }
            _ => render::image_placeholder(self, img),
        }
    }
}

fn convex(pts: &[P]) -> bool {
    let n = pts.len();
    let mut sign = 0.0f32;
    for i in 0..n {
        let (a, b, c) = (pts[i], pts[(i + 1) % n], pts[(i + 2) % n]);
        let z = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
        if z.abs() > 1e-6 {
            if sign != 0.0 && z.signum() != sign {
                return false;
            }
            sign = z.signum();
        }
    }
    true
}

/// Ear clipping of a simple polygon (screen points `v`, page points `pts`).
fn fill_mesh(pts: &[P], v: &[Pos2], color: Color32) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    for q in v {
        mesh.colored_vertex(*q, color);
    }
    let area: f32 = (0..pts.len()).map(|i| {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        a.x * b.y - b.x * a.y
    }).sum();
    let mut idx: Vec<usize> = (0..pts.len()).collect();
    let cross = |a: P, b: P, c: P| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let mut guard = 0;
    while idx.len() > 3 && guard < 10_000 {
        guard += 1;
        let n = idx.len();
        let mut cut = None;
        for i in 0..n {
            let (ia, ib, ic) = (idx[(i + n - 1) % n], idx[i], idx[(i + 1) % n]);
            let (a, b, c) = (pts[ia], pts[ib], pts[ic]);
            if cross(a, b, c) * area <= 0.0 {
                continue;
            }
            let inside = idx.iter().filter(|&&k| k != ia && k != ib && k != ic).any(|&k| {
                let q = pts[k];
                let (d1, d2, d3) = (cross(a, b, q), cross(b, c, q), cross(c, a, q));
                (d1 * area >= 0.0) && (d2 * area >= 0.0) && (d3 * area >= 0.0)
            });
            if !inside {
                cut = Some(i);
                break;
            }
        }
        let Some(i) = cut else { break };
        let n = idx.len();
        mesh.add_triangle(idx[(i + n - 1) % n] as u32, idx[i] as u32, idx[(i + 1) % n] as u32);
        idx.remove(i);
    }
    if idx.len() == 3 {
        mesh.add_triangle(idx[0] as u32, idx[1] as u32, idx[2] as u32);
    }
    mesh
}

pub struct Editor {
    pub tool: Tool,
    /// Screen offset of page origin relative to the canvas rect.
    pub offset: Vec2,
    /// Pixels per millimetre.
    pub zoom: f32,
    pub selection: BTreeSet<u64>,
    pub place_rot: u8,
    pub place_mirror: bool,
    pub show_grid: bool,
    pub show_pins: bool,
    pub place_ctx: PlaceCtx,
    pub fit_pending: bool,
    pub hover: Option<P>,
    pub status: String,
    pub center_on: Option<P>,
    /// Items placed with `Tool::Insert`.
    pub insert: Option<Clip>,
    pub show_revision_marks: bool,
    /// Layer of new graphics, texts, dimensions and images.
    pub active_layer: String,
    /// Connect facing connection points after placing or moving symbols.
    pub autoconnect: bool,
    /// The wire tool routes around symbols and other wires.
    pub autoroute: bool,
    /// Cable of new cable definition lines; empty = a new cable.
    pub cable_target: String,
    /// Device tag of the next symbols placed (navigator: place a function).
    pub place_tag: Option<String>,
    /// The page is locked by another user of a shared project.
    pub read_only: bool,
    /// Comments and markup are drawn.
    pub review_layer: bool,
    /// Selected comment or markup.
    pub review_selected: Option<Hit>,
    wire_start: Option<P>,
    wire_hfirst: bool,
    /// Points clicked so far by a graphic tool.
    pts: Vec<P>,
    drag: Drag,
    canvas_size: Vec2,
    /// Plot frame of the page shown.
    frame: PlotFrame,
    textures: Textures,
    /// Wire routing preview: from, to and the path.
    route_cache: Option<(P, P, Vec<P>)>,
}

impl Default for Editor {
    fn default() -> Self {
        Editor {
            tool: Tool::Select,
            offset: vec2(20.0, 20.0),
            zoom: 3.0,
            selection: BTreeSet::new(),
            place_rot: 0,
            place_mirror: false,
            show_grid: true,
            show_pins: false,
            place_ctx: PlaceCtx::default(),
            fit_pending: true,
            hover: None,
            status: String::new(),
            center_on: None,
            insert: None,
            show_revision_marks: true,
            active_layer: String::new(),
            autoconnect: true,
            autoroute: false,
            cable_target: String::new(),
            place_tag: None,
            read_only: false,
            review_layer: true,
            review_selected: None,
            wire_start: None,
            wire_hfirst: true,
            pts: Vec::new(),
            drag: Drag::None,
            canvas_size: vec2(800.0, 600.0),
            frame: PlotFrame::default(),
            textures: HashMap::new(),
            route_cache: None,
        }
    }
}

/// What the canvas wants the application to do.
#[derive(Default)]
pub struct CanvasOutput {
    /// The user double clicked an item (show its properties).
    pub open_properties: bool,
    /// Message for the status line.
    pub message: Option<String>,
    /// A comment or markup to add (review tools).
    pub review: Option<ReviewInput>,
    /// Click with the select tool (world position), for picking comments and markup.
    pub clicked: Option<P>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReviewInput {
    Comment(P),
    Markup(MarkupKind, Pen, P, P),
}

/// Cheap fingerprint of an embedded file (changes when it is replaced).
fn fingerprint(doc: &crate::document::Document) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    doc.data.len().hash(&mut h);
    doc.data.get(..256.min(doc.data.len())).hash(&mut h);
    doc.data.get(doc.data.len().saturating_sub(256)..).hash(&mut h);
    h.finish()
}

fn decode_image(doc: &crate::document::Document) -> Option<egui::ColorImage> {
    let bytes = doc.bytes().ok()?;
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()))
}

fn ghost_shape(g: &mut dyn Draw, kind: ShapeKind, a: P, b: P, pts: Vec<P>, sweep_deg: f32) {
    let s = Shape { kind, a, b, pts, sweep: sweep_deg, ..Default::default() };
    render::draw_shape(g, &s, Ink::Ghost);
}

/// Counter-clockwise angle in degrees from `start` to `end` around `c`.
fn sweep(c: P, start: P, end: P) -> f32 {
    let ang = |q: P| (-(q.y - c.y)).atan2(q.x - c.x).to_degrees();
    (ang(end) - ang(start)).rem_euclid(360.0)
}

impl Editor {
    pub fn set_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.wire_start = None;
        self.pts.clear();
        if self.tool != Tool::Select {
            self.selection.clear();
        }
    }

    /// True while a multi-click operation (wire, rectangle) is in progress.
    pub fn busy(&self) -> bool {
        self.wire_start.is_some() || !self.pts.is_empty() || !matches!(self.drag, Drag::None)
    }

    pub fn cancel(&mut self) {
        self.wire_start = None;
        self.pts.clear();
        self.drag = Drag::None;
    }

    fn s2w(&self, rect: Rect, s: Pos2) -> P {
        p((s.x - rect.min.x - self.offset.x) / self.zoom, (s.y - rect.min.y - self.offset.y) / self.zoom)
    }

    fn w2s(&self, rect: Rect, q: P) -> Pos2 {
        pos2(rect.min.x + self.offset.x + q.x * self.zoom, rect.min.y + self.offset.y + q.y * self.zoom)
    }

    pub fn fit(&mut self, size: Vec2) {
        let margin = 16.0;
        let (w, h) = self.frame.size();
        self.zoom = ((size.x - 2.0 * margin) / w).min((size.y - 2.0 * margin) / h).max(0.1);
        self.offset = vec2((size.x - w * self.zoom) / 2.0, (size.y - h * self.zoom) / 2.0);
    }

    fn zoom_at(&mut self, rect: Rect, screen: Pos2, factor: f32) {
        let before = self.s2w(rect, screen);
        self.zoom = (self.zoom * factor).clamp(0.4, 80.0);
        let after = self.w2s(rect, before);
        self.offset += screen - after;
    }

    /// Hit test: returns the id of the item under `q` (world), preferring texts and
    /// symbols. Items on hidden or locked layers are skipped.
    pub fn hit(&self, prj: &Project, page_i: usize, q: P) -> Option<u64> {
        let page = &prj.pages[page_i];
        let tol = (8.0 / self.zoom).max(0.4);
        let usable = |layer: &str| graphics::shown(prj, layer, false) && !graphics::locked(prj, layer);
        for t in page.texts.iter().rev().filter(|t| usable(&t.layer)) {
            if ops::text_bounds(t).expand(tol * 0.5).contains(q) {
                return Some(t.id);
            }
        }
        let mut best: Option<(f32, u64)> = None;
        for s in page.symbols.iter().rev() {
            let Some(def) = symbols::get(&s.sym) else { continue };
            let b = def.world_bounds(s.pos, s.rot, s.mirror);
            if b.expand(tol * 0.5).contains(q) {
                let d = b.center().dist(q) / (b.width() + b.height()).max(1.0);
                if best.map(|(bd, _)| d < bd).unwrap_or(true) {
                    best = Some((d, s.id));
                }
            }
        }
        if let Some((_, id)) = best {
            return Some(id);
        }
        if let Some(c) = page.conn_defs.iter().rev().find(|c| c.pos.dist(q) <= tol.max(1.2)) {
            return Some(c.id);
        }
        for w in page.wires.iter().rev() {
            if dist_to_segment(q, w.a, w.b) <= tol {
                return Some(w.id);
            }
        }
        if let Some(c) = page.cable_defs.iter().rev().find(|c| dist_to_segment(q, c.a, c.b) <= tol) {
            return Some(c.id);
        }
        if let Some(d) = page.dimensions.iter().rev().find(|d| usable(&d.layer) && graphics::dim_hit(d, q, tol)) {
            return Some(d.id);
        }
        if let Some(s) = page.shapes.iter().rev().find(|s| usable(&s.layer) && graphics::shape_hit(s, q, tol)) {
            return Some(s.id);
        }
        page.images.iter().rev().find(|i| usable(&i.layer) && R::from_points(i.a, i.b).contains(q)).map(|i| i.id)
    }

    /// Tag block or placed property of a selected symbol under `q`.
    fn prop_hit(&self, prj: &Project, page_i: usize, q: P) -> Option<PropHandle> {
        let page = &prj.pages[page_i];
        for s in page.symbols.iter().filter(|s| self.selection.contains(&s.id)) {
            for (i, pp) in s.props.iter().enumerate() {
                if render::prop_bounds(prj, page_i, s, pp).expand(0.3).contains(q) {
                    return Some((s.id, Some(i)));
                }
            }
            let Some(def) = symbols::get(&s.sym) else { continue };
            if render::tag_block_bounds(prj, page_i, def, s).is_some_and(|r| r.expand(0.3).contains(q)) {
                return Some((s.id, None));
            }
        }
        None
    }

    /// Does `q` touch a connection point or an existing wire (to end a wire there)?
    fn is_connection(page: &Page, q: P) -> bool {
        let pin = page.symbols.iter().any(|s| {
            symbols::get(&s.sym)
                .map(|d| (0..d.pins.len()).any(|i| d.pin_world(i, s.pos, s.rot, s.mirror).same(q)))
                .unwrap_or(false)
        });
        pin || page.wires.iter().any(|w| dist_to_segment(q, w.a, w.b) < 0.01)
    }

    fn wire_path(&mut self, page: &Page, a: P, b: P) -> Vec<P> {
        if self.autoroute {
            if let Some((ca, cb, path)) = &self.route_cache {
                if ca.same(a) && cb.same(b) {
                    return path.clone();
                }
            }
            let path = routing::route(page, self.frame.area(), a, b);
            self.route_cache = Some((a, b, path.clone()));
            return path;
        }
        if a.x == b.x || a.y == b.y {
            return vec![a, b];
        }
        let corner = if self.wire_hfirst { p(b.x, a.y) } else { p(a.x, b.y) };
        vec![a, corner, b]
    }

    /// Point on a wire for a connection definition: the grid point if it is on a
    /// wire, otherwise the nearest point of a wire close to the pointer.
    fn wire_point(&self, page: &Page, world: P, snapped: P) -> Option<P> {
        if conndef::wire_at(page, snapped).is_some() {
            return Some(snapped);
        }
        let tol = (8.0 / self.zoom).max(0.4);
        let w = page.wires.iter().find(|w| dist_to_segment(world, w.a, w.b) <= tol)?;
        let ab = w.b - w.a;
        let l2 = ab.x * ab.x + ab.y * ab.y;
        let t = if l2 > 0.0 { (((world.x - w.a.x) * ab.x + (world.y - w.a.y) * ab.y) / l2).clamp(0.0, 1.0) } else { 0.0 };
        let q = w.a + ab * t;
        // Whole millimetres along axis parallel wires.
        Some(if w.a.x == w.b.x { p(w.a.x, q.y.round()) } else if w.a.y == w.b.y { p(q.x.round(), w.a.y) } else { q })
    }

    /// Loads textures of the page's images that are new or changed.
    fn update_textures(&mut self, ctx: &egui::Context, page: &Page) {
        for img in &page.images {
            let fp = fingerprint(&img.file);
            if self.textures.get(&img.id).is_some_and(|t| t.0 == fp) {
                continue;
            }
            let tex = decode_image(&img.file).map(|ci| ctx.load_texture(format!("page-image-{}", img.id), ci, egui::TextureOptions::LINEAR));
            self.textures.insert(img.id, (fp, tex));
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, prj: &mut Project, page_i: usize, idx: &Index, an: &Analysis) -> CanvasOutput {
        let mut out = CanvasOutput::default();
        let dark = ui.visuals().dark_mode;
        let pal = Palette::new(dark);
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = resp.rect;
        self.canvas_size = rect.size();
        let frame = crate::frame::of_page(prj, page_i);
        if frame.size() != self.frame.size() {
            self.fit_pending = true;
        }
        self.frame = frame;
        // Wait for a sensible canvas size (the first web frame can be tiny).
        if self.fit_pending && rect.width() > 200.0 && rect.height() > 150.0 {
            self.fit(rect.size());
            self.fit_pending = false;
        }
        if let Some(c) = self.center_on.take() {
            self.offset = rect.size() / 2.0 - vec2(c.x, c.y) * self.zoom;
        }

        // ------------------------------------------------------------ navigation
        if resp.hovered() {
            // Raw wheel events: CAD style zoom around the cursor, no smoothing.
            let (wheel, zoom_delta, ptr) = ui.input(|i| {
                let mut wheel = 0.0;
                for e in &i.events {
                    if let egui::Event::MouseWheel { unit, delta, modifiers, .. } = e {
                        if modifiers.ctrl || modifiers.command {
                            continue; // already contained in zoom_delta()
                        }
                        wheel += match unit {
                            egui::MouseWheelUnit::Point => delta.y,
                            egui::MouseWheelUnit::Line => delta.y * 40.0,
                            egui::MouseWheelUnit::Page => delta.y * 400.0,
                        };
                    }
                }
                (wheel, i.zoom_delta(), i.pointer.hover_pos())
            });
            if let Some(ptr) = ptr {
                let factor = zoom_delta * (wheel * 0.0025).exp();
                if (factor - 1.0).abs() > 1e-4 {
                    self.zoom_at(rect, ptr, factor);
                }
            }
        }
        if resp.dragged_by(PointerButton::Middle) || resp.dragged_by(PointerButton::Secondary) {
            self.offset += resp.drag_delta();
        }

        let locked = prj.pages[page_i].locked;
        let editable = prj.pages[page_i].is_editable() && !self.read_only;
        let ptr = ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p));
        let world = ptr.map(|s| self.s2w(rect, s));
        let snapped = world.map(|w| w.snap(GRID));
        self.hover = world;

        // ------------------------------------------------------------ keyboard
        let kb = !ui.ctx().egui_wants_keyboard_input();
        if kb {
            self.keyboard(ui, prj, page_i, snapped, &mut out);
        }

        // ------------------------------------------------------------ mouse
        if self.tool == Tool::Select && resp.clicked_by(PointerButton::Primary) {
            out.clicked = world;
        }
        if self.tool.is_review() {
            self.review_mouse(&resp, world, &mut out);
        } else if editable {
            self.mouse(ui, &resp, prj, page_i, rect, world, snapped, an, &mut out);
        } else if resp.clicked() {
            self.selection.clear();
        }

        // ------------------------------------------------------------ drawing
        self.update_textures(ui.ctx(), &prj.pages[page_i]);
        painter.rect_filled(rect, 0.0, pal.canvas);
        let page_rect = Rect::from_min_max(self.w2s(rect, p(0.0, 0.0)), self.w2s(rect, p(self.frame.width(), self.frame.height())));
        painter.rect_filled(page_rect.translate(vec2(3.0, 3.0)), 0.0, Color32::from_black_alpha(60));
        painter.rect_filled(page_rect, 0.0, pal.paper);
        if self.show_grid && editable && self.zoom * GRID >= 7.0 {
            self.draw_grid(&painter, rect, &pal);
        }

        let mut opts = Opts {
            selected: self.selection.clone(),
            show_pins: self.show_pins || self.tool == Tool::Wire,
            hide_revision_marks: !self.show_revision_marks,
            review: self.review_layer || self.tool.is_review(),
            review_selected: self.review_selected,
            screen: true,
            ..Default::default()
        };
        // Highlight the complete potential of a selected wire.
        if self.selection.len() == 1 {
            let id = *self.selection.iter().next().unwrap();
            if let Some(net) = an.netlist.net_of_wire(id) {
                opts.highlight_wires = net.wires.iter().map(|w| w.1).collect::<HashSet<u64>>();
            }
        }
        let origin = rect.min + self.offset;
        let mut d = EguiDraw { painter: &painter, origin, zoom: self.zoom, pal: &pal, force: None, mirror: None, textures: Some(&self.textures) };
        render::draw_page(&mut d, prj, idx, an, page_i, &opts);

        // Previews
        if let (Tool::Markup(kind, _), Some(&a), Some(b)) = (&self.tool, self.pts.first(), world) {
            let mut g = EguiDraw { painter: &painter, origin, zoom: self.zoom, pal: &pal, force: Some(Ink::Ghost), mirror: None, textures: None };
            match kind {
                MarkupKind::Line => g.line(a, b, 0.5, Ink::Ghost, true),
                _ => g.rect(a, b, 0.5, Ink::Ghost, false),
            }
        }
        if let (Some(sn), Some(w), true) = (snapped, world, editable) {
            self.previews(&painter, &pal, origin, &prj.pages[page_i], w, sn);
            if self.tool != Tool::Select {
                let c = self.w2s(rect, sn);
                let st = Stroke::new(1.0, pal.ghost);
                painter.line_segment([c - vec2(8.0, 0.0), c + vec2(8.0, 0.0)], st);
                painter.line_segment([c - vec2(0.0, 8.0), c + vec2(0.0, 8.0)], st);
            }
        }
        if let Drag::Box { start } = self.drag {
            if let Some(ptr) = ptr {
                let r = Rect::from_two_pos(start, ptr);
                painter.rect_filled(r, 0.0, pal.ghost.gamma_multiply(0.15));
                painter.rect_stroke(r, 0.0, Stroke::new(1.0, pal.ghost), egui::StrokeKind::Inside);
            }
        }
        // Handles of the movable texts of a single selected symbol.
        if self.tool == Tool::Select && self.selection.len() == 1 {
            let page = &prj.pages[page_i];
            if let Some(s) = page.symbols.iter().find(|s| self.selection.contains(&s.id)) {
                let mut boxes: Vec<R> = s.props.iter().map(|pp| render::prop_bounds(prj, page_i, s, pp)).collect();
                if let Some(r) = symbols::get(&s.sym).and_then(|def| render::tag_block_bounds(prj, page_i, def, s)) {
                    boxes.push(r);
                }
                for r in boxes {
                    let sr = Rect::from_min_max(self.w2s(rect, r.min), self.w2s(rect, r.max)).expand(1.0);
                    painter.rect_stroke(sr, 0.0, Stroke::new(1.0, pal.ghost), egui::StrokeKind::Outside);
                }
            }
        }
        if !editable {
            let msg = if locked { tr("Released page – unlock it in Project › Revisions to change it") } else { tr("Generated page – edit via page properties") };
            painter.text(rect.left_top() + vec2(10.0, 10.0), Align2::LEFT_TOP, msg, FontId::proportional(13.0), pal.frame);
        }

        // Status line
        self.status = match (world, snapped) {
            (Some(w), Some(_)) => {
                let mut s = tr_fmt!("X {}  Y {} mm   Column {}  Row {}", format!("{:7.1}", w.x), format!("{:7.1}", w.y), self.frame.column_of(w), self.frame.row_of(w));
                if let Some(id) = self.hit(prj, page_i, w) {
                    let page = &prj.pages[page_i];
                    if let Some(sym) = page.symbols.iter().find(|s| s.id == id) {
                        let name = symbols::get(&sym.sym).map(|d| tr(d.name)).unwrap_or("?");
                        s += &format!("   |  {} {} {}", sym.tag, name, sym.label);
                    } else if let Some(net) = an.netlist.net_of_wire(id) {
                        s += &tr_fmt!("   |  Potential {}", net.name);
                    } else if let Some(dm) = page.dimensions.iter().find(|d| d.id == id) {
                        s += &format!("   |  {}", graphics::dim_text(dm));
                    } else if let Some(c) = page.cable_defs.iter().find(|c| c.id == id) {
                        s += &format!("   |  {}", c.cable);
                    }
                }
                if self.tool == Tool::Wire && self.autoroute {
                    s += &format!("   |  {}", tr("Routing around symbols"));
                }
                s
            }
            _ => String::new(),
        };
        out
    }

    fn previews(&mut self, painter: &Painter, pal: &Palette, origin: Pos2, page: &Page, world: P, sn: P) {
        let mut g = EguiDraw { painter, origin, zoom: self.zoom, pal, force: Some(Ink::Ghost), mirror: None, textures: None };
        match self.tool.clone() {
            Tool::Place(key) => {
                if let Some(def) = symbols::get(key) {
                    let inst = SymbolInst {
                        sym: def.key.into(),
                        pos: sn,
                        rot: self.place_rot,
                        mirror: self.place_mirror,
                        label: def.default_label.into(),
                        ..Default::default()
                    };
                    render::draw_symbol(&mut g, def, &inst, Ink::Ghost);
                }
            }
            Tool::Wire => {
                if let Some(a) = self.wire_start {
                    let path = self.wire_path(page, a, sn);
                    let mut g = EguiDraw { painter, origin, zoom: self.zoom, pal, force: Some(Ink::Ghost), mirror: None, textures: None };
                    for w in path.windows(2) {
                        g.line(w[0], w[1], render::WIRE_W, Ink::Ghost, false);
                    }
                }
            }
            Tool::Line | Tool::Rect | Tool::Circle | Tool::Ellipse => {
                if let (Some(&a), Some(kind)) = (self.pts.first(), self.tool.shape_kind()) {
                    ghost_shape(&mut g, kind, a, sn, Vec::new(), 0.0);
                }
            }
            Tool::Arc => match self.pts.as_slice() {
                [c] => g.line(*c, sn, render::SYMBOL_W, Ink::Ghost, true),
                [c, s] => ghost_shape(&mut g, ShapeKind::Arc, *c, *s, Vec::new(), sweep(*c, *s, sn)),
                _ => {}
            },
            Tool::Polyline | Tool::Polygon => {
                if !self.pts.is_empty() {
                    let mut pts = self.pts.clone();
                    pts.push(sn);
                    let kind = self.tool.shape_kind().unwrap_or_default();
                    ghost_shape(&mut g, kind, P::default(), P::default(), pts, 0.0);
                }
            }
            Tool::Dimension(kind) => match self.pts.as_slice() {
                [a] => g.line(*a, sn, render::SYMBOL_W * 0.7, Ink::Ghost, true),
                [a, b] => {
                    let mut dm = Dimension { kind, a: *a, b: *b, ..Default::default() };
                    dm.offset = graphics::dim_offset_through(&dm, sn);
                    render::draw_dimension(&mut g, &dm, Ink::Ghost);
                }
                _ => {}
            },
            Tool::CableDef => {
                if let Some(&a) = self.pts.first() {
                    g.line(a, sn, render::SYMBOL_W, Ink::Ghost, false);
                    for id in cabledef::crossed_wires(page, a, sn) {
                        if let Some(w) = page.wires.iter().find(|w| w.id == id) {
                            g.line(w.a, w.b, render::WIRE_W * 2.0, Ink::Ghost, false);
                        }
                    }
                }
            }
            Tool::ConnDef => {
                if let Some(q) = self.wire_point(page, world, sn) {
                    let r = 0.9;
                    g.poly(&[q + p(0.0, -r), q + p(r, 0.0), q + p(0.0, r), q + p(-r, 0.0)], true, render::SYMBOL_W, Ink::Ghost, true);
                }
            }
            Tool::Comment | Tool::Markup(..) => {}
            Tool::Insert => {
                if let Some(clip) = &self.insert {
                    // Same offset as ops::paste
                    let d = (sn - clip.bounds().min).snap(GRID);
                    for s in &clip.symbols {
                        if let Some(def) = symbols::get(&s.sym) {
                            let inst = SymbolInst { pos: s.pos + d, ..s.clone() };
                            render::draw_symbol(&mut g, def, &inst, Ink::Ghost);
                        }
                    }
                    for w in &clip.wires {
                        g.line(w.a + d, w.b + d, render::WIRE_W, Ink::Ghost, false);
                    }
                    for s in &clip.shapes {
                        let mut s = s.clone();
                        graphics::map_shape(&mut s, |q| q + d);
                        render::draw_shape(&mut g, &s, Ink::Ghost);
                    }
                    for i in &clip.images {
                        g.rect(i.a + d, i.b + d, render::SYMBOL_W, Ink::Ghost, false);
                    }
                }
            }
            Tool::Select | Tool::Text => {}
        }
    }

    fn draw_grid(&self, painter: &Painter, rect: Rect, pal: &Palette) {
        let tl = self.s2w(rect, rect.min);
        let br = self.s2w(rect, rect.max);
        let x0 = (tl.x.max(0.0) / GRID).ceil() as i32;
        let x1 = (br.x.min(self.frame.width()) / GRID).floor() as i32;
        let y0 = (tl.y.max(0.0) / GRID).ceil() as i32;
        let y1 = (br.y.min(self.frame.height()) / GRID).floor() as i32;
        let mut mesh = egui::Mesh::default();
        let r = if self.zoom * GRID > 20.0 { 1.0 } else { 0.6 };
        for gx in x0..=x1 {
            for gy in y0..=y1 {
                let c = self.w2s(rect, p(gx as f32 * GRID, gy as f32 * GRID));
                mesh.add_colored_rect(Rect::from_center_size(c, vec2(r * 2.0, r * 2.0)), pal.grid);
            }
        }
        painter.add(EShape::mesh(mesh));
    }

    fn keyboard(&mut self, ui: &egui::Ui, prj: &mut Project, page_i: usize, snapped: Option<P>, out: &mut CanvasOutput) {
        let editable = prj.pages[page_i].is_editable() && !self.read_only;
        let (esc, enter, r, m, a, del, space, fit, all, arrows, events) = ui.input(|i| {
            let k = |key| i.key_pressed(key) && i.modifiers.is_none();
            (
                i.key_pressed(Key::Escape),
                k(Key::Enter),
                k(Key::R),
                k(Key::M),
                k(Key::A),
                i.key_pressed(Key::Delete) || k(Key::Backspace),
                k(Key::Space),
                k(Key::F) || k(Key::Home),
                i.modifiers.command && i.key_pressed(Key::A),
                [
                    (i.key_pressed(Key::ArrowLeft), p(-GRID, 0.0)),
                    (i.key_pressed(Key::ArrowRight), p(GRID, 0.0)),
                    (i.key_pressed(Key::ArrowUp), p(0.0, -GRID)),
                    (i.key_pressed(Key::ArrowDown), p(0.0, GRID)),
                ],
                i.events.clone(),
            )
        });
        if esc {
            if self.wire_start.is_some() || !self.pts.is_empty() {
                self.wire_start = None;
                self.pts.clear();
            } else if self.tool != Tool::Select {
                self.set_tool(Tool::Select);
            } else {
                self.selection.clear();
            }
        }
        if fit {
            self.fit(self.canvas_size);
        }
        if !editable {
            return;
        }
        if enter {
            self.finish_poly(prj, page_i);
        }
        if a && self.tool == Tool::Wire {
            self.autoroute = !self.autoroute;
            self.route_cache = None;
        }
        let page = &mut prj.pages[page_i];
        if r {
            if matches!(self.tool, Tool::Place(_)) {
                self.place_rot = (self.place_rot + 1) % 4;
            } else if !self.selection.is_empty() {
                ops::rotate(page, &self.selection);
            }
        }
        if m {
            if matches!(self.tool, Tool::Place(_)) {
                self.place_mirror = !self.place_mirror;
            } else {
                ops::mirror(page, &self.selection);
            }
        }
        if space && self.tool == Tool::Wire {
            self.wire_hfirst = !self.wire_hfirst;
        }
        if del && !self.selection.is_empty() {
            ops::delete(page, &self.selection);
            self.selection.clear();
        }
        if all {
            let page = &prj.pages[page_i];
            self.selection = ops::all_ids(page).into_iter().filter(|id| graphics::item_layer(page, *id).is_none_or(|l| !graphics::locked(prj, l))).collect();
        }
        let page = &mut prj.pages[page_i];
        if !self.selection.is_empty() {
            for (pressed, d) in arrows {
                if pressed {
                    let att = ops::attached_wire_ends(page, &self.selection);
                    ops::translate(page, &self.selection, &att, d);
                }
            }
        }
        for ev in events {
            match ev {
                egui::Event::Copy | egui::Event::Cut if !self.selection.is_empty() => {
                    let clip = ops::copy(page, &self.selection);
                    ui.ctx().copy_text(clip.to_text());
                    if ev == egui::Event::Cut {
                        ops::delete(page, &self.selection);
                        self.selection.clear();
                    }
                }
                egui::Event::Paste(text) => {
                    if let Some(clip) = Clip::from_text(&text) {
                        if !clip.is_empty() {
                            let at = snapped.unwrap_or(self.frame.area().min + p(20.0, 20.0));
                            self.selection = ops::paste(prj, page_i, &clip, at);
                            self.set_tool(Tool::Select);
                            out.message = Some(tr_fmt!("{} objects pasted", self.selection.len()));
                        }
                    }
                    return;
                }
                _ => {}
            }
        }
    }

    /// Ends a polyline / polygon (if it has enough points).
    fn finish_poly(&mut self, prj: &mut Project, page_i: usize) {
        let Some(kind) = self.tool.shape_kind().filter(|k| matches!(k, ShapeKind::Polyline | ShapeKind::Polygon)) else { return };
        let mut pts = std::mem::take(&mut self.pts);
        pts.dedup_by(|a, b| a.same(*b));
        let needed = if kind == ShapeKind::Polygon { 3 } else { 2 };
        if pts.len() >= needed {
            let id = prj.new_id();
            prj.pages[page_i].shapes.push(Shape { id, kind, pts, layer: self.active_layer.clone(), ..Default::default() });
        }
    }

    /// Adds the shape, dimension or definition of a graphic tool once all its points
    /// are clicked.
    fn complete(&mut self, prj: &mut Project, page_i: usize, out: &mut CanvasOutput) {
        let pts = std::mem::take(&mut self.pts);
        let layer = self.active_layer.clone();
        match self.tool.clone() {
            Tool::Dimension(kind) => {
                let mut dm = Dimension { kind, a: pts[0], b: pts[1], layer, ..Default::default() };
                dm.offset = graphics::dim_offset_through(&dm, pts[2]);
                if graphics::dim_length(&dm).abs() > 0.01 {
                    dm.id = prj.new_id();
                    prj.pages[page_i].dimensions.push(dm);
                }
            }
            Tool::CableDef => {
                let id = cabledef::add(prj, page_i, pts[0], pts[1], &self.cable_target);
                if let Some(c) = prj.pages[page_i].cable_defs.iter().find(|c| c.id == id) {
                    let n = prj.pages[page_i].wires.iter().filter(|w| w.cable == c.cable).count();
                    out.message = Some(tr_fmt!("Cable {}: {} cores assigned", c.cable, n));
                    self.cable_target = c.cable.clone();
                }
                self.set_tool(Tool::Select);
                self.selection.insert(id);
                out.open_properties = true;
            }
            tool => {
                let Some(kind) = tool.shape_kind() else { return };
                let (a, b) = (pts[0], pts[1]);
                let sweep_deg = if kind == ShapeKind::Arc { sweep(a, b, pts[2]) } else { 0.0 };
                if a.same(b) || (kind == ShapeKind::Arc && sweep_deg == 0.0) {
                    return;
                }
                let id = prj.new_id();
                prj.pages[page_i].shapes.push(Shape { id, kind, a, b, sweep: sweep_deg, layer, ..Default::default() });
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn mouse(
        &mut self,
        ui: &egui::Ui,
        resp: &egui::Response,
        prj: &mut Project,
        page_i: usize,
        rect: Rect,
        world: Option<P>,
        snapped: Option<P>,
        an: &Analysis,
        out: &mut CanvasOutput,
    ) {
        let shift = ui.input(|i| i.modifiers.shift);
        let secondary_click = resp.clicked_by(PointerButton::Secondary);
        if secondary_click {
            if matches!(self.tool, Tool::Polyline | Tool::Polygon) && !self.pts.is_empty() {
                self.finish_poly(prj, page_i);
            } else if self.wire_start.is_some() || !self.pts.is_empty() {
                self.wire_start = None;
                self.pts.clear();
            } else if self.tool != Tool::Select {
                self.set_tool(Tool::Select);
            }
        }
        let click = resp.clicked_by(PointerButton::Primary).then_some(snapped).flatten();
        match self.tool.clone() {
            Tool::Select => self.select_mouse(ui, resp, prj, page_i, rect, world, shift, out),
            Tool::Insert => {
                if let (Some(sn), Some(clip)) = (click, click.and_then(|_| self.insert.take())) {
                    let ids = ops::paste(prj, page_i, &clip, sn);
                    self.set_tool(Tool::Select);
                    self.selection = ids;
                }
            }
            Tool::Place(key) => {
                if let (Some(sn), Some(def)) = (click, symbols::get(key)) {
                    let mut ctx = self.place_ctx.clone();
                    if let Some(tag) = &self.place_tag {
                        ctx.last_main.insert(def.letter.to_string(), tag.clone());
                    }
                    let mut s = ops::new_symbol(prj, page_i, &ctx, def, sn, self.place_rot, self.place_mirror);
                    if let (Some(tag), true) = (&self.place_tag, def.func.has_tag()) {
                        s.tag = tag.clone();
                    }
                    self.place_ctx.remember(&s);
                    let id = s.id;
                    prj.pages[page_i].symbols.push(s);
                    if self.autoconnect {
                        let n = routing::apply_autoconnect(prj, page_i, &BTreeSet::from([id]));
                        if n > 0 {
                            out.message = Some(tr_fmt!("{} connections drawn automatically", n));
                        }
                    }
                }
            }
            Tool::Wire => {
                let finish = resp.double_clicked();
                if let Some(sn) = click {
                    match self.wire_start {
                        None => self.wire_start = Some(sn),
                        Some(a) if a.same(sn) => self.wire_start = None,
                        Some(a) => {
                            let ends_on_connection = Self::is_connection(&prj.pages[page_i], sn);
                            let path = self.wire_path(&prj.pages[page_i], a, sn);
                            for seg in path.windows(2) {
                                let id = prj.new_id();
                                prj.pages[page_i].wires.push(Wire { id, a: seg[0], b: seg[1], ..Default::default() });
                            }
                            ops::cleanup_wires(&mut prj.pages[page_i]);
                            self.route_cache = None;
                            self.wire_start = if ends_on_connection { None } else { Some(sn) };
                        }
                    }
                }
                if finish {
                    self.wire_start = None;
                }
            }
            Tool::Text => {
                if let Some(sn) = click {
                    let id = prj.new_id();
                    prj.pages[page_i].texts.push(TextItem { id, pos: sn, text: "Text".into(), size: 3.0, layer: self.active_layer.clone(), color: None });
                    self.set_tool(Tool::Select);
                    self.selection.insert(id);
                    out.open_properties = true;
                }
            }
            Tool::ConnDef => {
                let target = resp.clicked_by(PointerButton::Primary).then(|| world.zip(snapped)).flatten();
                if let Some((w, sn)) = target {
                    match self.wire_point(&prj.pages[page_i], w, sn) {
                        Some(q) => {
                            let mut cd = conndef::adopt(prj, &an.open, page_i, q);
                            cd.id = prj.new_id();
                            let id = cd.id;
                            prj.pages[page_i].conn_defs.push(cd);
                            self.set_tool(Tool::Select);
                            self.selection.insert(id);
                            out.open_properties = true;
                        }
                        None => out.message = Some(tr("Click on a wire to place the connection definition point").to_string()),
                    }
                }
            }
            Tool::Polyline | Tool::Polygon => {
                if let Some(sn) = click {
                    if !self.pts.last().is_some_and(|l| l.same(sn)) {
                        self.pts.push(sn);
                    }
                }
                if resp.double_clicked() {
                    self.finish_poly(prj, page_i);
                }
            }
            tool => {
                let Some(needed) = tool.points() else { return };
                if let Some(sn) = click {
                    if self.pts.last().is_some_and(|l| l.same(sn)) && tool != Tool::Arc {
                        return;
                    }
                    self.pts.push(sn);
                    if self.pts.len() >= needed {
                        self.complete(prj, page_i, out);
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn select_mouse(&mut self, ui: &egui::Ui, resp: &egui::Response, prj: &mut Project, page_i: usize, rect: Rect, world: Option<P>, shift: bool, out: &mut CanvasOutput) {
        if resp.drag_started_by(PointerButton::Primary) {
            let origin = ui.input(|i| i.pointer.press_origin()).unwrap_or(rect.center());
            let ow = self.s2w(rect, origin);
            if let Some(handle) = self.prop_hit(prj, page_i, ow) {
                let page = &prj.pages[page_i];
                let s = page.symbols.iter().find(|s| s.id == handle.0);
                let orig = match (s, handle.1) {
                    (Some(s), Some(i)) => s.props[i].offset,
                    (Some(s), None) => s.tag_offset.unwrap_or_else(|| {
                        symbols::get(&s.sym).and_then(|def| render::tag_block_bounds(prj, page_i, def, s)).map(|r| r.min - s.pos).unwrap_or_default()
                    }),
                    _ => P::default(),
                };
                self.drag = Drag::Prop { handle, start: ow, orig };
            } else if let Some(id) = self.hit(prj, page_i, ow) {
                if !self.selection.contains(&id) {
                    if !shift {
                        self.selection.clear();
                    }
                    self.selection.insert(id);
                }
                let attached = ops::attached_wire_ends(&prj.pages[page_i], &self.selection);
                self.drag = Drag::Move { start: ow, applied: p(0.0, 0.0), attached };
            } else {
                self.drag = Drag::Box { start: origin };
            }
        }
        if resp.dragged_by(PointerButton::Primary) {
            match (&mut self.drag, world) {
                (Drag::Move { start, applied, attached }, Some(w)) => {
                    let total = (w - *start).snap(GRID);
                    let delta = total - *applied;
                    if delta.x != 0.0 || delta.y != 0.0 {
                        ops::translate(&mut prj.pages[page_i], &self.selection, attached, delta);
                        *applied = total;
                    }
                }
                (Drag::Prop { handle, start, orig }, Some(w)) => {
                    let to = *orig + (w - *start).snap(0.5);
                    if let Some(s) = prj.pages[page_i].symbols.iter_mut().find(|s| s.id == handle.0) {
                        match handle.1 {
                            Some(i) => s.props[i].offset = to,
                            None => s.tag_offset = Some(to),
                        }
                    }
                }
                _ => {}
            }
        }
        if resp.drag_stopped() {
            match std::mem::replace(&mut self.drag, Drag::None) {
                Drag::Box { start } => {
                    if let Some(end) = resp.interact_pointer_pos().or(ui.input(|i| i.pointer.hover_pos())) {
                        let a = self.s2w(rect, start);
                        let b = self.s2w(rect, end);
                        let sel = R::from_points(a, b);
                        if !shift {
                            self.selection.clear();
                        }
                        let page = &prj.pages[page_i];
                        for id in ops::all_ids(page) {
                            let usable = graphics::item_layer(page, id).is_none_or(|l| graphics::shown(prj, l, false) && !graphics::locked(prj, l));
                            if usable && ops::item_bounds(page, id).is_some_and(|bb| sel.contains_rect(&bb)) {
                                self.selection.insert(id);
                            }
                        }
                    }
                }
                Drag::Move { applied, .. } => {
                    ops::cleanup_wires(&mut prj.pages[page_i]);
                    let moved_symbols = prj.pages[page_i].symbols.iter().any(|s| self.selection.contains(&s.id));
                    if self.autoconnect && moved_symbols && (applied.x != 0.0 || applied.y != 0.0) {
                        let n = routing::apply_autoconnect(prj, page_i, &self.selection);
                        if n > 0 {
                            out.message = Some(tr_fmt!("{} connections drawn automatically", n));
                        }
                    }
                }
                Drag::Prop { .. } | Drag::None => {}
            }
        }
        if resp.clicked_by(PointerButton::Primary) {
            if let Some(w) = world {
                match self.hit(prj, page_i, w) {
                    Some(id) => {
                        if shift {
                            if !self.selection.remove(&id) {
                                self.selection.insert(id);
                            }
                        } else {
                            self.selection.clear();
                            self.selection.insert(id);
                        }
                        if let Some(s) = prj.pages[page_i].symbols.iter().find(|s| s.id == id) {
                            self.place_ctx.remember(s);
                        }
                    }
                    None if !shift => self.selection.clear(),
                    None => {}
                }
            }
        }
        if resp.double_clicked() && !self.selection.is_empty() {
            out.open_properties = true;
        }
    }
}

impl Editor {
    /// Comment and markup tools: two clicks for lines and rectangles, one otherwise.
    fn review_mouse(&mut self, resp: &egui::Response, world: Option<P>, out: &mut CanvasOutput) {
        let (true, Some(w)) = (resp.clicked_by(PointerButton::Primary), world) else { return };
        match self.tool {
            Tool::Comment => out.review = Some(ReviewInput::Comment(w)),
            Tool::Markup(MarkupKind::Text, pen) => out.review = Some(ReviewInput::Markup(MarkupKind::Text, pen, w, w)),
            Tool::Markup(kind, pen) => match self.pts.pop() {
                None => self.pts.push(w),
                Some(a) if a.dist(w) > 0.5 => out.review = Some(ReviewInput::Markup(kind, pen, a, w)),
                Some(_) => {}
            },
            _ => {}
        }
    }
}

/// Draws a symbol preview into a small rectangle (symbol library).
pub fn symbol_icon(ui: &mut egui::Ui, key: &str, size: Vec2) -> egui::Response {
    let (resp, painter) = ui.allocate_painter(size, Sense::click());
    let Some(def) = symbols::get(key) else { return resp };
    let pal = Palette::new(ui.visuals().dark_mode);
    let b = def.bounds;
    let zoom = ((size.x - 6.0) / b.width()).min((size.y - 6.0) / b.height()).min(3.2);
    let center = resp.rect.center();
    let origin = center - vec2(b.center().x, b.center().y) * zoom;
    let mut d = EguiDraw { painter: &painter, origin, zoom, pal: &pal, force: None, mirror: None, textures: None };
    let inst = SymbolInst { sym: key.into(), label: def.default_label.into(), ..Default::default() };
    render::draw_symbol(&mut d, def, &inst, Ink::Line);
    resp
}

#[cfg(test)]
mod tests;
