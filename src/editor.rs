//! Interactive schematic canvas: pan/zoom, selection, moving, placing symbols,
//! drawing wires, texts and graphics.

use crate::geom::{dist_to_segment, p, P, R, GRID};
use crate::index::Index;
use crate::model::*;
use crate::ops::{self, Clip, PlaceCtx};
use crate::render::{self, Draw, Ink, Opts, VAnchor};
use crate::reports::Analysis;
use crate::symbols::{self, Anchor};
use egui::{pos2, vec2, Align2, Color32, FontId, Key, Painter, PointerButton, Pos2, Rect, Sense, Shape as EShape, Stroke, Vec2};
use std::collections::{BTreeSet, HashSet};

#[derive(Clone, Debug, PartialEq)]
pub enum Tool {
    Select,
    Place(&'static str),
    Wire,
    Text,
    Line,
    Rect,
}

impl Tool {
    pub fn hint(&self) -> &'static str {
        match self {
            Tool::Select => "Click to select, drag to move, drag on empty space for window selection. Right/middle drag pans, wheel zooms.",
            Tool::Place(_) => "Click to place · R rotate · M mirror · Esc finish",
            Tool::Wire => "Click start and corner points · Space toggles corner direction · ends automatically on a connection · Right click / Esc finish",
            Tool::Text => "Click to place a text",
            Tool::Line | Tool::Rect => "Click start point, click end point",
        }
    }
}

enum Drag {
    None,
    Move { start: P, applied: P, attached: Vec<(u64, bool)> },
    Box { start: Pos2 },
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
        }
    }
}

/// Renders page geometry with an egui painter.
pub struct EguiDraw<'a> {
    pub painter: &'a Painter,
    pub origin: Pos2,
    pub zoom: f32,
    pub pal: &'a Palette,
    /// Forces every element to this ink (ghost previews).
    pub force: Option<Ink>,
}

impl EguiDraw<'_> {
    fn w2s(&self, q: P) -> Pos2 {
        pos2(self.origin.x + q.x * self.zoom, self.origin.y + q.y * self.zoom)
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
            self.painter.add(EShape::convex_polygon(v, self.color(ink), st));
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
    wire_start: Option<P>,
    wire_hfirst: bool,
    shape_start: Option<P>,
    drag: Drag,
    canvas_size: Vec2,
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
            wire_start: None,
            wire_hfirst: true,
            shape_start: None,
            drag: Drag::None,
            canvas_size: vec2(800.0, 600.0),
        }
    }
}

/// What the canvas wants the application to do.
#[derive(Default)]
pub struct CanvasOutput {
    /// The user double clicked an item (show its properties).
    pub open_properties: bool,
}

impl Editor {
    pub fn set_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.wire_start = None;
        self.shape_start = None;
        if self.tool != Tool::Select {
            self.selection.clear();
        }
    }

    /// True while a multi-click operation (wire, rectangle) is in progress.
    pub fn busy(&self) -> bool {
        self.wire_start.is_some() || self.shape_start.is_some() || !matches!(self.drag, Drag::None)
    }

    pub fn cancel(&mut self) {
        self.wire_start = None;
        self.shape_start = None;
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
        self.zoom = ((size.x - 2.0 * margin) / PAGE_W).min((size.y - 2.0 * margin) / PAGE_H).max(0.3);
        self.offset = vec2((size.x - PAGE_W * self.zoom) / 2.0, (size.y - PAGE_H * self.zoom) / 2.0);
    }

    fn zoom_at(&mut self, rect: Rect, screen: Pos2, factor: f32) {
        let before = self.s2w(rect, screen);
        self.zoom = (self.zoom * factor).clamp(0.4, 80.0);
        let after = self.w2s(rect, before);
        self.offset += screen - after;
    }

    /// Hit test: returns the id of the item under `q` (world), preferring texts and symbols.
    pub fn hit(&self, page: &Page, q: P) -> Option<u64> {
        let tol = (8.0 / self.zoom).max(0.4);
        for t in page.texts.iter().rev() {
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
        for w in page.wires.iter().rev() {
            if dist_to_segment(q, w.a, w.b) <= tol {
                return Some(w.id);
            }
        }
        for s in page.shapes.iter().rev() {
            let hit = match s.kind {
                ShapeKind::Line => dist_to_segment(q, s.a, s.b) <= tol,
                ShapeKind::Rect => {
                    let (a, b) = (s.a, s.b);
                    let c = [a, p(b.x, a.y), b, p(a.x, b.y), a];
                    c.windows(2).any(|e| dist_to_segment(q, e[0], e[1]) <= tol)
                }
            };
            if hit {
                return Some(s.id);
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

    fn wire_path(&self, a: P, b: P) -> Vec<P> {
        if a.x == b.x || a.y == b.y {
            return vec![a, b];
        }
        let corner = if self.wire_hfirst { p(b.x, a.y) } else { p(a.x, b.y) };
        vec![a, corner, b]
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, prj: &mut Project, page_i: usize, idx: &Index, an: &Analysis) -> CanvasOutput {
        let mut out = CanvasOutput::default();
        let dark = ui.visuals().dark_mode;
        let pal = Palette::new(dark);
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = resp.rect;
        self.canvas_size = rect.size();
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

        let page_kind = prj.pages[page_i].kind;
        let editable = page_kind == PageKind::Schematic;
        let ptr = ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p));
        let world = ptr.map(|s| self.s2w(rect, s));
        let snapped = world.map(|w| w.snap(GRID));
        self.hover = world;

        // ------------------------------------------------------------ keyboard
        let kb = !ui.ctx().egui_wants_keyboard_input();
        if kb {
            self.keyboard(ui, prj, page_i, snapped);
        }

        // ------------------------------------------------------------ mouse
        if editable {
            self.mouse(ui, &resp, prj, page_i, rect, world, snapped, &mut out);
        } else if resp.clicked() {
            self.selection.clear();
        }

        // ------------------------------------------------------------ drawing
        painter.rect_filled(rect, 0.0, pal.canvas);
        let page_rect = Rect::from_min_max(self.w2s(rect, p(0.0, 0.0)), self.w2s(rect, p(PAGE_W, PAGE_H)));
        painter.rect_filled(page_rect.translate(vec2(3.0, 3.0)), 0.0, Color32::from_black_alpha(60));
        painter.rect_filled(page_rect, 0.0, pal.paper);
        if self.show_grid && editable && self.zoom * GRID >= 7.0 {
            self.draw_grid(&painter, rect, &pal);
        }

        let mut opts = Opts { selected: self.selection.clone(), show_pins: self.show_pins || self.tool == Tool::Wire, ..Default::default() };
        // Highlight the complete potential of a selected wire.
        if self.selection.len() == 1 {
            let id = *self.selection.iter().next().unwrap();
            if let Some(net) = an.netlist.net_of_wire(id) {
                opts.highlight_wires = net.wires.iter().map(|w| w.1).collect::<HashSet<u64>>();
            }
        }
        let origin = rect.min + self.offset;
        let mut d = EguiDraw { painter: &painter, origin, zoom: self.zoom, pal: &pal, force: None };
        render::draw_page(&mut d, prj, idx, an, page_i, &opts);

        // Previews
        let mut g = EguiDraw { painter: &painter, origin, zoom: self.zoom, pal: &pal, force: Some(Ink::Ghost) };
        if let (Some(sn), true) = (snapped, editable) {
            match &self.tool {
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
                        let path = self.wire_path(a, sn);
                        for w in path.windows(2) {
                            g.line(w[0], w[1], render::WIRE_W, Ink::Ghost, false);
                        }
                    }
                }
                Tool::Line | Tool::Rect => {
                    if let Some(a) = self.shape_start {
                        let s = Shape { kind: if self.tool == Tool::Line { ShapeKind::Line } else { ShapeKind::Rect }, a, b: sn, ..Default::default() };
                        render::draw_shape(&mut g, &s, Ink::Ghost);
                    }
                }
                _ => {}
            }
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
        if !editable {
            painter.text(
                rect.left_top() + vec2(10.0, 10.0),
                Align2::LEFT_TOP,
                "Generated page – edit via page properties",
                FontId::proportional(13.0),
                pal.frame,
            );
        }

        // Status line
        self.status = match (world, snapped) {
            (Some(w), Some(_)) => {
                let mut s = format!("X {:7.1}  Y {:7.1} mm   Column {}  Row {}", w.x, w.y, column_of(w), row_of(w));
                if let Some(id) = self.hit(&prj.pages[page_i], w) {
                    let page = &prj.pages[page_i];
                    if let Some(sym) = page.symbols.iter().find(|s| s.id == id) {
                        let name = symbols::get(&sym.sym).map(|d| d.name).unwrap_or("?");
                        s += &format!("   |  {} {} {}", sym.tag, name, sym.label);
                    } else if let Some(net) = an.netlist.net_of_wire(id) {
                        s += &format!("   |  Potential {}", net.name);
                    }
                }
                s
            }
            _ => String::new(),
        };
        out
    }

    fn draw_grid(&self, painter: &Painter, rect: Rect, pal: &Palette) {
        let tl = self.s2w(rect, rect.min);
        let br = self.s2w(rect, rect.max);
        let x0 = (tl.x.max(0.0) / GRID).ceil() as i32;
        let x1 = (br.x.min(PAGE_W) / GRID).floor() as i32;
        let y0 = (tl.y.max(0.0) / GRID).ceil() as i32;
        let y1 = (br.y.min(PAGE_H) / GRID).floor() as i32;
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

    fn keyboard(&mut self, ui: &egui::Ui, prj: &mut Project, page_i: usize, snapped: Option<P>) {
        let editable = prj.pages[page_i].kind == PageKind::Schematic;
        let (esc, r, m, del, space, fit, all, arrows, events) = ui.input(|i| {
            let k = |key| i.key_pressed(key) && i.modifiers.is_none();
            (
                i.key_pressed(Key::Escape),
                k(Key::R),
                k(Key::M),
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
            if self.wire_start.is_some() || self.shape_start.is_some() {
                self.wire_start = None;
                self.shape_start = None;
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
            self.selection = ops::all_ids(page).into_iter().collect();
        }
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
                            let at = snapped.unwrap_or(p(AREA_X0 + 20.0, AREA_Y0 + 20.0));
                            self.selection = ops::paste(prj, page_i, &clip, at);
                            self.set_tool(Tool::Select);
                        }
                    }
                    return;
                }
                _ => {}
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
        out: &mut CanvasOutput,
    ) {
        let shift = ui.input(|i| i.modifiers.shift);
        let secondary_click = resp.clicked_by(PointerButton::Secondary);
        if secondary_click {
            if self.wire_start.is_some() || self.shape_start.is_some() {
                self.wire_start = None;
                self.shape_start = None;
            } else if self.tool != Tool::Select {
                self.set_tool(Tool::Select);
            }
        }
        match self.tool.clone() {
            Tool::Select => {
                if resp.drag_started_by(PointerButton::Primary) {
                    let origin = ui.input(|i| i.pointer.press_origin()).unwrap_or(rect.center());
                    let ow = self.s2w(rect, origin);
                    let page = &prj.pages[page_i];
                    if let Some(id) = self.hit(page, ow) {
                        if !self.selection.contains(&id) {
                            if !shift {
                                self.selection.clear();
                            }
                            self.selection.insert(id);
                        }
                        let attached = ops::attached_wire_ends(page, &self.selection);
                        self.drag = Drag::Move { start: ow, applied: p(0.0, 0.0), attached };
                    } else {
                        self.drag = Drag::Box { start: origin };
                    }
                }
                if resp.dragged_by(PointerButton::Primary) {
                    if let (Drag::Move { start, applied, attached }, Some(w)) = (&mut self.drag, world) {
                        let total = (w - *start).snap(GRID);
                        let delta = total - *applied;
                        if delta.x != 0.0 || delta.y != 0.0 {
                            ops::translate(&mut prj.pages[page_i], &self.selection, attached, delta);
                            *applied = total;
                        }
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
                                    if let Some(bb) = ops::item_bounds(page, id) {
                                        if sel.contains_rect(&bb) {
                                            self.selection.insert(id);
                                        }
                                    }
                                }
                            }
                        }
                        Drag::Move { .. } => ops::cleanup_wires(&mut prj.pages[page_i]),
                        Drag::None => {}
                    }
                }
                if resp.clicked_by(PointerButton::Primary) {
                    if let Some(w) = world {
                        let page = &prj.pages[page_i];
                        match self.hit(page, w) {
                            Some(id) => {
                                if shift {
                                    if !self.selection.remove(&id) {
                                        self.selection.insert(id);
                                    }
                                } else {
                                    self.selection.clear();
                                    self.selection.insert(id);
                                }
                                if let Some(s) = page.symbols.iter().find(|s| s.id == id) {
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
            Tool::Place(key) => {
                if let (true, Some(sn)) = (resp.clicked_by(PointerButton::Primary), snapped) {
                    if let Some(def) = symbols::get(key) {
                        let s = ops::new_symbol(prj, &self.place_ctx, def, sn, self.place_rot, self.place_mirror);
                        self.place_ctx.remember(&s);
                        prj.pages[page_i].symbols.push(s);
                    }
                }
            }
            Tool::Wire => {
                let finish = resp.double_clicked();
                if let (true, Some(sn)) = (resp.clicked_by(PointerButton::Primary), snapped) {
                    match self.wire_start {
                        None => self.wire_start = Some(sn),
                        Some(a) if a.same(sn) => self.wire_start = None,
                        Some(a) => {
                            let ends_on_connection = Self::is_connection(&prj.pages[page_i], sn);
                            let path = self.wire_path(a, sn);
                            for seg in path.windows(2) {
                                let id = prj.new_id();
                                prj.pages[page_i].wires.push(Wire { id, a: seg[0], b: seg[1], ..Default::default() });
                            }
                            ops::cleanup_wires(&mut prj.pages[page_i]);
                            self.wire_start = if ends_on_connection { None } else { Some(sn) };
                        }
                    }
                }
                if finish {
                    self.wire_start = None;
                }
            }
            Tool::Text => {
                if let (true, Some(sn)) = (resp.clicked_by(PointerButton::Primary), snapped) {
                    let id = prj.new_id();
                    prj.pages[page_i].texts.push(TextItem { id, pos: sn, text: "Text".into(), size: 3.0 });
                    self.set_tool(Tool::Select);
                    self.selection.insert(id);
                    out.open_properties = true;
                }
            }
            Tool::Line | Tool::Rect => {
                if let (true, Some(sn)) = (resp.clicked_by(PointerButton::Primary), snapped) {
                    match self.shape_start.take() {
                        None => self.shape_start = Some(sn),
                        Some(a) if !a.same(sn) => {
                            let id = prj.new_id();
                            let kind = if self.tool == Tool::Line { ShapeKind::Line } else { ShapeKind::Rect };
                            prj.pages[page_i].shapes.push(Shape { id, kind, a, b: sn, dashed: false });
                        }
                        Some(_) => {}
                    }
                }
            }
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
    let mut d = EguiDraw { painter: &painter, origin, zoom, pal: &pal, force: None };
    let inst = SymbolInst { sym: key.into(), label: def.default_label.into(), ..Default::default() };
    render::draw_symbol(&mut d, def, &inst, Ink::Line);
    resp
}
