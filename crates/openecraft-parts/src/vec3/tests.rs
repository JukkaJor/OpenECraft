use super::*;

fn close(a: V3, b: V3) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5)
}

#[test]
fn vector_basics() {
    assert_eq!(dot([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]), 32.0);
    assert_eq!(cross([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]), [0.0, 0.0, 1.0]);
    assert!(close(norm([3.0, 0.0, 4.0]), [0.6, 0.0, 0.8]));
}

#[test]
fn orbit_from_the_front_looks_down_minus_z() {
    let cam = Orbit { yaw: 0.0, pitch: 0.0, dist: 10.0, target: [1.0, 2.0, 3.0] };
    let (eye, right, up, fwd) = cam.basis();
    assert!(close(eye, [1.0, 2.0, 13.0]));
    assert!(close(fwd, [0.0, 0.0, -1.0]));
    assert!(close(right, [1.0, 0.0, 0.0]));
    assert!(close(up, [0.0, 1.0, 0.0]));
}

#[test]
fn segments_and_triangles() {
    let t = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 10.0, 0.0]];
    assert!(segment_crosses_triangle([2.0, 2.0, -1.0], [2.0, 2.0, 1.0], &t));
    // Ending on the plane, outside, or lying in the plane does not count.
    assert!(!segment_crosses_triangle([2.0, 2.0, 0.0], [2.0, 2.0, 1.0], &t));
    assert!(!segment_crosses_triangle([8.0, 8.0, -1.0], [8.0, 8.0, 1.0], &t));
    assert!(!segment_crosses_triangle([1.0, 1.0, 0.0], [3.0, 1.0, 0.0], &t));
}

#[test]
fn box_surfaces() {
    let a = box_triangles([0.0; 3], [10.0; 3]);
    assert!(point_inside([5.0, 5.0, 5.0], &a));
    assert!(!point_inside([15.0, 5.0, 5.0], &a));
    // Overlapping, touching, apart, contained.
    assert!(surfaces_intersect(&a, &box_triangles([8.0, 2.0, 2.0], [20.0, 8.0, 8.0])));
    assert!(!surfaces_intersect(&a, &box_triangles([10.0, 0.0, 0.0], [20.0, 10.0, 10.0])));
    assert!(!surfaces_intersect(&a, &box_triangles([12.0, 0.0, 0.0], [20.0, 10.0, 10.0])));
    assert!(surfaces_intersect(&a, &box_triangles([2.0; 3], [4.0; 3])));
    assert!(surfaces_intersect(&box_triangles([2.0; 3], [4.0; 3]), &a));
    // Crossing bars (no corner inside the other box).
    let x = box_triangles([-5.0, 4.0, 4.0], [15.0, 6.0, 6.0]);
    assert!(surfaces_intersect(&a, &x));
}
