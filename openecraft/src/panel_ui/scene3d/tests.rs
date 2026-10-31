use super::*;

#[test]
fn duct_fingers_a_tube_passes_through_are_left_out() {
    // Two fingers of a duct; a cable passes through the first, beside the second.
    let duct = ([0.0, 0.0, 0.0], [100.0, 60.0, 60.0]);
    let fingers = vec![(0, 0..6, ([10.0, 0.0, 20.0], [16.0, 2.0, 60.0])), (0, 6..12, ([60.0, 0.0, 20.0], [66.0, 2.0, 60.0]))];
    let through = vec![([13.0, -50.0, 40.0], [13.0, 30.0, 40.0])].into_iter().map(|(a, b)| (vec![a, b], 4.0)).collect::<Vec<_>>();
    assert_eq!(fingers_hit(&fingers, &through, &[duct]), vec![0]);
    // Touching it with its thickness is enough; far away nothing.
    let touching = vec![(vec![[21.0, -50.0, 40.0], [21.0, 30.0, 40.0]], 6.0)];
    assert_eq!(fingers_hit(&fingers, &touching, &[duct]), vec![0]);
    let away = vec![(vec![[40.0, -50.0, 40.0], [40.0, 30.0, 40.0]], 4.0)];
    assert!(fingers_hit(&fingers, &away, &[duct]).is_empty());
}
