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

#[test]
fn wires_along_the_plate_go_over_what_stands_in_their_way() {
    // A run 30 mm in front of the plate crosses a busbar 20 .. 70 mm in front of it.
    let busbar = ([100.0, 0.0, 20.0], [200.0, 50.0, 70.0]);
    let way = over(&[[0.0, 25.0, 30.0], [300.0, 25.0, 30.0]], &[busbar], 1.5);
    assert_eq!((way[0], way[way.len() - 1]), ([0.0, 25.0, 30.0], [300.0, 25.0, 30.0]));
    assert!(way.iter().any(|q| q[2] >= 72.5), "in front of it: {way:?}");
    for s in way.windows(2) {
        assert!(vec3::segment_in_box(s[0], s[1], ([98.5, -1.5, 18.5], [201.5, 51.5, 71.5])).is_none(), "{way:?}");
    }
    // Beside it nothing changes.
    let beside = [[0.0, 80.0, 30.0], [300.0, 80.0, 30.0]];
    assert_eq!(over(&beside, &[busbar], 1.5), beside.to_vec());
}
