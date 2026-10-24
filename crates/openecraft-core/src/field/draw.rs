//! Drawing of the plant layout (canvas and SVG export): outlines, the route network
//! with fill and kind of every segment, nodes, placed enclosures and devices, and the
//! route of one cable highlighted. Plant coordinates are metres; `scale` gives the
//! drawing units per metre.

use super::{Plan, SegmentKind};
use crate::geom::{p, P};
use crate::model::Project;
use crate::render::{Draw, Ink, VAnchor};
use crate::symbols::Anchor;

/// Extent of the plant layout in metres (min, max).
pub fn bounds(prj: &Project) -> Option<(P, P)> {
    let f = &prj.field;
    let pts: Vec<P> = f.nodes.iter().map(|n| p(n.x, n.y)).chain(f.outlines.iter().flat_map(|o| o.points.iter().map(|q| p(q[0], q[1])))).collect();
    let first = *pts.first()?;
    Some(pts.iter().fold((first, first), |(a, b), q| (p(a.x.min(q.x), a.y.min(q.y)), p(b.x.max(q.x), b.y.max(q.y)))))
}

/// Draws the plant layout with `origin` (metres) at drawing position 0/0.
pub fn draw(d: &mut dyn Draw, prj: &Project, plan: &Plan, origin: P, scale: f32, highlight: Option<&str>) {
    let f = &prj.field;
    let at = |x: f32, y: f32| p((x - origin.x) * scale, (y - origin.y) * scale);
    let text = (scale * 0.25).clamp(1.5, 4.0);
    for o in &f.outlines {
        let pts: Vec<P> = o.points.iter().map(|q| at(q[0], q[1])).collect();
        if pts.len() >= 2 {
            d.poly(&pts, true, 0.35, Ink::Frame, false);
            d.text(p(pts[0].x + text * 0.5, pts[0].y + text * 1.2), &o.name, text, Anchor::Left, VAnchor::Bottom, Ink::FrameText);
        }
    }
    let route: &[u64] = highlight.and_then(|c| plan.cable(c)).map(|c| c.segments.as_slice()).unwrap_or(&[]);
    for s in &f.segments {
        let (Some(a), Some(b)) = (f.node(s.a), f.node(s.b)) else { continue };
        let (qa, qb) = (at(a.x, a.y), at(b.x, b.y));
        let over = plan.segment(s.id).is_some_and(|u| u.capacity.is_some_and(|c| u.area > c));
        let ink = if route.contains(&s.id) {
            Ink::Highlight
        } else if over {
            Ink::Error
        } else if s.kind == SegmentKind::DragChain {
            Ink::CrossRef
        } else {
            Ink::Line
        };
        let width = match s.kind {
            SegmentKind::Tray | SegmentKind::Ladder | SegmentKind::Trunking => 1.2,
            SegmentKind::DragChain => 1.6,
            _ => 0.5,
        } * if route.contains(&s.id) { 1.8 } else { 1.0 };
        d.line(qa, qb, width, ink, matches!(s.kind, SegmentKind::Buried | SegmentKind::Free));
        let mid = p((qa.x + qb.x) / 2.0, (qa.y + qb.y) / 2.0 - text * 0.6);
        let mut label = f.segment_name(s.id);
        if let Some(fill) = plan.segment(s.id).and_then(|u| u.fill(s)) {
            label = format!("{label} {fill:.0} %");
        }
        d.text(mid, &label, text * 0.8, Anchor::Center, VAnchor::Bottom, if over { Ink::Error } else { Ink::Text });
    }
    for n in &f.nodes {
        d.circle(at(n.x, n.y), (scale * 0.08).clamp(0.4, 1.5), 0.25, Ink::Line, false);
    }
    for pl in &f.places {
        let Some(n) = f.node(pl.node) else { continue };
        let q = at(n.x, n.y);
        let r = (scale * 0.15).clamp(0.8, 3.0);
        let enclosure = prj.layout.enclosures.iter().any(|e| super::same_location(&e.name, &pl.tag));
        if enclosure {
            d.rect(p(q.x - r, q.y - r), p(q.x + r, q.y + r), 0.35, Ink::Fill, true);
            d.rect(p(q.x - r, q.y - r), p(q.x + r, q.y + r), 0.35, Ink::Line, false);
        } else {
            d.circle(q, r, 0.35, Ink::Pin, true);
        }
        d.text(p(q.x + r * 1.4, q.y + text * 0.35), &pl.tag, text, Anchor::Left, VAnchor::Middle, Ink::Text);
    }
}

/// The plant layout as SVG, fitted to `width` mm.
pub fn svg(prj: &Project, plan: &Plan, highlight: Option<&str>, width: f32) -> String {
    let (min, max) = bounds(prj).unwrap_or((p(0.0, 0.0), p(10.0, 10.0)));
    let margin = 2.0;
    let origin = p(min.x - margin, min.y - margin);
    let (w, h) = ((max.x - min.x + 2.0 * margin).max(1.0), (max.y - min.y + 2.0 * margin).max(1.0));
    let scale = width / w;
    crate::svg::drawing_svg(width, h * scale, |d| draw(d, prj, plan, origin, scale, highlight))
}
