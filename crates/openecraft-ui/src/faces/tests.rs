use super::*;
use egui::epaint::{TessellationOptions, Tessellator};
use egui::pos2;

/// How far the tessellated shape reaches beyond the bounding box of `pts`.
fn overshoot(shape: Shape, pts: &[Pos2]) -> f32 {
    let mut tess = Tessellator::new(1.0, TessellationOptions::default(), [1, 1], Vec::new());
    let mut mesh = Mesh::default();
    tess.tessellate_shape(shape, &mut mesh);
    let r = egui::Rect::from_points(pts);
    mesh.vertices.iter().map(|v| (r.left() - v.pos.x).max(v.pos.x - r.right()).max(r.top() - v.pos.y).max(v.pos.y - r.bottom())).fold(0.0, f32::max)
}

#[test]
fn slivers_draw_no_spikes() {
    // A box face seen almost edge-on and a thin mesh triangle.
    let quad = vec![pos2(100.0, 100.0), pos2(300.0, 110.0), pos2(310.0, 111.0), pos2(110.0, 101.0)];
    let tri = vec![pos2(0.0, 0.0), pos2(200.0, 3.0), pos2(0.0, 1.0)];
    let stroke = Stroke::new(0.6, Color32::GRAY);
    for pts in [quad, tri] {
        assert!(overshoot(Shape::convex_polygon(pts.clone(), Color32::WHITE, stroke), &pts) > 20.0);
        assert!(overshoot(face(pts.clone(), Color32::WHITE, stroke), &pts) < 2.0);
    }
}

#[test]
fn regular_faces_stay_anti_aliased() {
    let pts = vec![pos2(0.0, 0.0), pos2(50.0, 0.0), pos2(50.0, 30.0), pos2(0.0, 30.0)];
    assert!(matches!(face(pts.clone(), Color32::WHITE, Stroke::NONE), Shape::Path(_)));
    assert!(overshoot(face(pts.clone(), Color32::WHITE, Stroke::new(0.6, Color32::GRAY)), &pts) < 2.0);
}

#[test]
fn degenerate_faces_are_skipped() {
    let line = vec![pos2(0.0, 0.0), pos2(10.0, 0.0), pos2(20.0, 0.0)];
    assert!(matches!(face(line, Color32::WHITE, Stroke::NONE), Shape::Noop));
    let dup = vec![pos2(0.0, 0.0), pos2(0.0, 0.0), pos2(10.0, 0.0), pos2(10.0, 10.0)];
    assert!(matches!(face(dup, Color32::WHITE, Stroke::NONE), Shape::Vec(_)));
}
