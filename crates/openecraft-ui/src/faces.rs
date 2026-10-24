//! Projected faces of the 3D views (panel layout, model previews).
//!
//! egui feathers and strokes a polygon with miter joins that grow without limit at
//! sharp corners (egui issue #1226): a sliver – a face seen nearly edge-on while
//! orbiting, a thin mesh triangle – then shoots spikes across the view. Such faces are
//! filled without feathering and outlined with single segments instead.

use egui::{Color32, Mesh, Pos2, Shape, Stroke};

/// Smallest interior angle (radians) drawn as an anti-aliased polygon; the miter of
/// its corners is at most 1 / sin(angle / 2) ≈ 3 times the stroke width.
const MIN_ANGLE: f32 = 0.35;

/// A convex face; `stroke` outlines it (`Stroke::NONE` for none).
pub fn face(pts: Vec<Pos2>, fill: Color32, stroke: Stroke) -> Shape {
    let n = pts.len();
    if n < 3 || area(&pts).abs() < 0.05 {
        return Shape::Noop;
    }
    if min_angle(&pts) >= MIN_ANGLE {
        return Shape::convex_polygon(pts, fill, stroke);
    }
    let mut mesh = Mesh::default();
    for &q in &pts {
        mesh.colored_vertex(q, fill);
    }
    for i in 1..n as u32 - 1 {
        mesh.add_triangle(0, i, i + 1);
    }
    let mut shapes = vec![Shape::mesh(mesh)];
    if !stroke.is_empty() {
        shapes.extend((0..n).map(|i| Shape::line_segment([pts[i], pts[(i + 1) % n]], stroke)));
    }
    Shape::Vec(shapes)
}

fn area(pts: &[Pos2]) -> f32 {
    let n = pts.len();
    (0..n).map(|i| pts[i].x * pts[(i + 1) % n].y - pts[(i + 1) % n].x * pts[i].y).sum::<f32>() / 2.0
}

/// Smallest interior angle; 0 at repeated points.
fn min_angle(pts: &[Pos2]) -> f32 {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let (a, b) = (pts[(i + n - 1) % n] - pts[i], pts[(i + 1) % n] - pts[i]);
            let (la, lb) = (a.length(), b.length());
            if la < 1e-3 || lb < 1e-3 {
                return 0.0;
            }
            (a.dot(b) / (la * lb)).clamp(-1.0, 1.0).acos()
        })
        .fold(f32::INFINITY, f32::min)
}

#[cfg(test)]
mod tests;
