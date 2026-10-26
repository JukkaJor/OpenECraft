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

#[test]
fn sums_and_distances() {
    assert_eq!(add([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]), [5.0, 7.0, 9.0]);
    assert_eq!(dist([1.0, 2.0, 3.0], [4.0, 6.0, 3.0]), 5.0);
    assert_eq!(manhattan([1.0, 2.0, 3.0], [4.0, 0.0, 3.0]), 5.0);
}

#[test]
fn boxes_are_painted_from_far_to_near() {
    // A long duct along x in front of the plate and a device beside its right end,
    // reaching further out than the duct.
    let duct = ([0.0, 0.0, 0.0], [600.0, 60.0, 60.0]);
    let device = ([620.0, 0.0, 0.0], [680.0, 90.0, 90.0]);
    let pos = |order: Vec<usize>, i: usize| order.iter().position(|&k| k == i).unwrap();
    // Seen from the left the duct is in front of the device, from the right behind it,
    // although the centre of the duct is nearer in both cases.
    let left = paint_order(&[duct, device], [-2000.0, 300.0, 400.0]);
    assert!(pos(left.clone(), 1) < pos(left, 0), "the device first, the duct over it");
    let right = paint_order(&[duct, device], [3000.0, 300.0, 400.0]);
    assert!(pos(right.clone(), 0) < pos(right, 1));
    // From the front, a device on a duct (further out) is painted after it.
    let on = ([100.0, 0.0, 60.0], [160.0, 60.0, 120.0]);
    assert_eq!(paint_order(&[on, duct], [300.0, 300.0, 2000.0]), vec![1, 0]);
}

#[test]
fn overlapping_boxes_and_cycles_still_give_every_box_once() {
    let a = ([0.0, 0.0, 0.0], [10.0, 10.0, 10.0]);
    let b = ([5.0, 5.0, 5.0], [15.0, 15.0, 15.0]);
    let far = ([100.0, 0.0, 0.0], [110.0, 10.0, 10.0]);
    let mut order = paint_order(&[a, b, far], [-50.0, 5.0, 5.0]);
    assert_eq!(order[0], 2, "the farthest one first");
    order.sort();
    assert_eq!(order, vec![0, 1, 2]);
    assert!(paint_order(&[], [0.0; 3]).is_empty());
}

#[test]
fn segments_are_clipped_to_boxes() {
    let duct = ([0.0, 0.0, 0.0], [100.0, 60.0, 60.0]);
    // A wire from a device above the duct down into it and along it.
    assert_eq!(segment_in_box([50.0, 120.0, 30.0], [50.0, 30.0, 30.0], duct), Some((2.0 / 3.0, 1.0)));
    assert_eq!(segment_in_box([10.0, 30.0, 30.0], [90.0, 30.0, 30.0], duct), Some((0.0, 1.0)));
    let (t0, t1) = segment_in_box([-50.0, 30.0, 30.0], [150.0, 30.0, 30.0], duct).unwrap();
    assert!((t0 - 0.25).abs() < 1e-6 && (t1 - 0.75).abs() < 1e-6);
    assert_eq!(segment_in_box([0.0, 100.0, 30.0], [100.0, 100.0, 30.0], duct), None, "above the duct");
    assert_eq!(segment_in_box([-50.0, 30.0, 30.0], [-10.0, 30.0, 30.0], duct), None, "before it");
}

#[test]
fn a_box_hides_another_only_from_its_side() {
    let near = ([0.0, 0.0, 100.0], [10.0, 10.0, 110.0]);
    let far = ([0.0, 0.0, 0.0], [10.0, 10.0, 10.0]);
    let front = [5.0, 5.0, 1000.0];
    assert!(may_hide(&near, &far, front) && !may_hide(&far, &near, front));
    let back = [5.0, 5.0, -1000.0];
    assert!(may_hide(&far, &near, back) && !may_hide(&near, &far, back));
}

#[test]
fn a_wire_is_cut_into_pieces_in_and_between_ducts() {
    let left = ([0.0, 0.0, 0.0], [100.0, 60.0, 60.0]);
    let right = ([200.0, 0.0, 0.0], [300.0, 60.0, 60.0]);
    let pieces = split_by_boxes([-100.0, 30.0, 30.0], [300.0, 30.0, 30.0], &[right, left]);
    let round = |v: f32| (v * 100.0).round() / 100.0;
    let got: Vec<(f32, f32, Option<usize>)> = pieces.into_iter().map(|(a, b, i)| (round(a), round(b), i)).collect();
    assert_eq!(got, vec![(0.0, 0.25, None), (0.25, 0.5, Some(1)), (0.5, 0.75, None), (0.75, 1.0, Some(0))]);
    assert_eq!(split_by_boxes([0.0, 100.0, 0.0], [10.0, 100.0, 0.0], &[left]), vec![(0.0, 1.0, None)]);
}

#[test]
fn wires_go_over_the_ducts_and_under_the_devices() {
    // Paint order: duct 0, device 1, duct 2, device 3.
    let rank = [0, 1, 2, 3];
    // In duct 0 at the junction with duct 2, which is painted later: over both.
    assert_eq!(wire_place([(0, true), (2, true)], &rank), Some((2, true)));
    // Device 3 covers it too: still over the ducts, under the device.
    assert_eq!(wire_place([(0, true), (2, true), (3, false)], &rank), Some((2, true)));
    // A device painted before the last duct covering it wins.
    assert_eq!(wire_place([(2, true), (3, false), (1, false)], &rank), Some((1, false)));
    assert_eq!(wire_place([(3, false)], &rank), Some((3, false)));
    assert_eq!(wire_place([], &rank), None);
}

#[test]
fn a_wire_leaving_a_duct_stays_over_it_seen_from_below() {
    // A duct with a device above it; a wire runs from the duct up into the device.
    let duct = ([0.0, 0.0, 0.0], [100.0, 60.0, 60.0]);
    let device = ([30.0, 80.0, 0.0], [70.0, 160.0, 90.0]);
    let eye = [50.0, -500.0, 1000.0];
    let order = paint_order(&[duct, device], eye);
    let mut rank = [0; 2];
    for (r, &i) in order.iter().enumerate() {
        rank[i] = r;
    }
    assert!(rank[0] > rank[1], "from below the duct is painted over the device");
    let (a, b) = ([50.0, 30.0, 30.0], [50.0, 120.0, 30.0]);
    let hiders = |piece: &(V3, V3)| [(0, true), (1, false)].into_iter().filter(|(i, _)| may_hide(&[duct, device][*i], piece, eye)).collect::<Vec<_>>();
    // Up to the terminal inside the device the piece is under the device and so under the
    // duct painted later.
    let whole = ([50.0, 60.0, 30.0], [50.0, 120.0, 30.0]);
    assert_eq!(wire_place(hiders(&whole), &rank), Some((1, false)));
    // Cut at the device (the part inside it is not seen) it goes over the duct.
    let pieces = split_by_boxes(a, b, &[duct, device]);
    assert_eq!(pieces.iter().map(|p| p.2).collect::<Vec<_>>(), vec![Some(0), None, Some(1)]);
    let (t0, t1, _) = pieces[1];
    let at = |t: f32| [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * t);
    let between = (at(t0), at(t1));
    assert_eq!(wire_place(hiders(&between), &rank), Some((0, true)));
}
