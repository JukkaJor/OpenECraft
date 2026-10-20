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
