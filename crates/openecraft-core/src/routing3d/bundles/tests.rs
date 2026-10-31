use super::*;
use crate::routing3d::{End, RouteKind, RouteLeg, Status};

fn item(key: &str, class: Class, diameter: f32, points: Vec<V3>) -> RoutedItem {
    RoutedItem {
        kind: RouteKind::Cable,
        key: key.into(),
        label: key.into(),
        class,
        ends: [End::default(), End::default()],
        enclosures: vec![0],
        points,
        radii: Vec::new(),
        legs: Vec::new(),
        length: 0.0,
        cut: 0.0,
        diameter,
        free: 0.0,
        bends: 0,
        shielded: false,
        status: Status::Routed,
        wire: None,
    }
}

#[test]
fn shared_stretches_become_bundles_with_branches() {
    let items = vec![
        // A and B run together from x 0 to 500, then A goes up and B down.
        item("A", Class::Power, 6.0, vec![[0.0, 0.0, 0.0], [500.0, 0.0, 0.0], [500.0, -300.0, 0.0]]),
        item("B", Class::Signal, 8.0, vec![[0.0, 0.0, 0.0], [500.0, 0.0, 0.0], [500.0, 300.0, 0.0]]),
        // C joins them from x 250 on.
        item("C", Class::Control, 6.0, vec![[250.0, 200.0, 0.0], [250.0, 0.0, 0.0], [500.0, 0.0, 0.0], [500.0, 300.0, 0.0]]),
        // D runs on its own.
        item("D", Class::Control, 6.0, vec![[0.0, 1000.0, 0.0], [500.0, 1000.0, 0.0]]),
    ];
    let (bundles, branches) = find(&items, 25.0);
    let find_b = |members: &[usize]| bundles.iter().find(|b| b.items == members).unwrap_or_else(|| panic!("{members:?} in {bundles:?}"));
    let ab = find_b(&[0, 1]);
    assert_eq!(ab.length, 250.0);
    assert!((ab.diameter - 1.15 * 10.0).abs() < 0.01);
    assert_eq!(ab.classes, vec![Class::Power, Class::Signal]);
    let abc = find_b(&[0, 1, 2]);
    assert_eq!(abc.length, 250.0);
    // B and C go on downwards together.
    assert_eq!(find_b(&[1, 2]).length, 300.0);
    assert_eq!(bundles.len(), 3);
    assert!(!bundles.iter().any(|b| b.items.contains(&3)));
    // Branches where C joins and where A leaves.
    let at = |q: V3| branches.iter().find(|b| (0..3).all(|i| (b.at[i] - q[i]).abs() < 1e-3));
    assert_eq!(at([250.0, 0.0, 0.0]).map(|b| b.items), Some(3));
    let split = at([500.0, 0.0, 0.0]).expect("branch at x 500");
    assert_eq!((split.items, split.bundles.len()), (3, 2));
    assert_eq!(branches.len(), 2);
    // Unrouted items take no part.
    let mut unrouted = items.clone();
    unrouted[1].status = Status::Unrouted("x".into());
    assert!(find(&unrouted, 25.0).0.iter().all(|b| !b.items.contains(&1)));
    assert!((diameter([3.0, 4.0].into_iter()) - 5.75).abs() < 1e-4);
}

#[test]
fn legs_are_bundled_on_their_own() {
    // Two routes leave a cabinet (x 0 – 100) through the same gland and go on in a room
    // elsewhere in the layout (x 5000 – 5100): the jump between the legs is no stretch.
    let leg = |item: &mut RoutedItem, legs: [Vec<V3>; 2]| {
        item.points = legs.concat();
        item.legs = legs.into_iter().enumerate().map(|(k, points)| RouteLeg { enclosure: k, points, radii: Vec::new() }).collect();
    };
    let mut a = item("A", Class::Control, 6.0, Vec::new());
    leg(&mut a, [vec![[0.0, 0.0, 0.0], [100.0, 0.0, 0.0]], vec![[5000.0, 0.0, 0.0], [5100.0, 0.0, 0.0]]]);
    let mut b = item("B", Class::Control, 6.0, Vec::new());
    leg(&mut b, [vec![[0.0, 50.0, 0.0], [100.0, 0.0, 0.0]], vec![[5000.0, 0.0, 0.0], [5000.0, 300.0, 0.0]]]);
    assert_eq!(a.stretches().len(), 2);
    let (bundles, _) = find(&[a, b], 25.0);
    assert!(bundles.iter().all(|x| x.length < 200.0), "{bundles:?}");
}

#[test]
fn items_of_a_bundle_lie_side_by_side_and_do_not_cross() {
    // A and B run the same way round two corners; C on its own.
    let route = vec![[0.0, -300.0, 0.0], [0.0, 0.0, 0.0], [500.0, 0.0, 0.0], [500.0, 500.0, 0.0], [900.0, 500.0, 0.0]];
    let items = [
        item("A", Class::Power, 10.0, route.clone()),
        item("B", Class::Power, 10.0, route.clone()),
        item("C", Class::Power, 10.0, vec![[0.0, 2000.0, 0.0], [500.0, 2000.0, 0.0], [500.0, 2500.0, 0.0]]),
    ];
    let ways: Vec<Way> = items.iter().map(|it| Way { points: &it.points, radii: &it.radii, diameter: it.diameter, rest: [0.0; 3], below: f32::INFINITY }).collect();
    let laid = spread_ways(&ways, 25.0);
    let (a, b) = (&laid[0].0, &laid[1].0);
    assert_eq!(laid[2].0, items[2].points, "alone it stays as it is");
    // The ends and the stretches at the ends stay where they are.
    assert_eq!((a[0], a[a.len() - 1], b[0], a[1], b[1]), (route[0], route[4], route[0], route[1], route[1]));
    // Along the shared stretches (x 0 .. 500, then y 0 .. 500) they run a lane apart (the
    // thickest item and 1 mm), parallel: side by side, never crossing.
    let middle = |p: &[V3], axis: usize| {
        let s = p.windows(2).find(|s| (s[1][axis] - s[0][axis]).abs() > 400.0).unwrap_or_else(|| panic!("{p:?}"));
        assert_eq!((0..3).filter(|&k| (s[0][k] - s[1][k]).abs() > 1e-3).count(), 1, "{p:?}");
        [0, 1, 2].map(|k| (s[0][k] + s[1][k]) / 2.0)
    };
    for axis in [0, 1] {
        let d = crate::parts::vec3::dist(middle(a, axis), middle(b, axis));
        assert!((d - 11.0).abs() < 0.01, "{axis}: {a:?} {b:?}");
    }
}

#[test]
fn lanes_keep_off_the_surface_the_ways_run_along() {
    // Five wires lying 5 mm in front of a plate (z 0) that may come 3 mm nearer to it:
    // two rows away from it, side by side along it.
    let line = vec![[0.0, 0.0, 5.0], [500.0, 0.0, 5.0], [500.0, 300.0, 5.0], [800.0, 300.0, 5.0]];
    let ways: Vec<Way> = (0..5).map(|_| Way { points: &line, radii: &[], diameter: 3.0, rest: [0.0, 0.0, 1.0], below: 3.0 }).collect();
    let laid = spread_ways(&ways, 8.0);
    for (points, _) in &laid {
        assert!(points.iter().all(|q| q[2] >= 2.0 - 0.01), "{points:?}");
    }
    // Along the middle stretch apart from each other.
    let middles: Vec<V3> = laid
        .iter()
        .map(|(p, _)| {
            let s = p.windows(2).find(|s| (s[1][1] - s[0][1]).abs() > 250.0).unwrap_or_else(|| panic!("{p:?}"));
            [0, 1, 2].map(|k| (s[0][k] + s[1][k]) / 2.0)
        })
        .collect();
    for (i, a) in middles.iter().enumerate() {
        assert!(middles[i + 1..].iter().all(|b| crate::parts::vec3::dist(*a, *b) >= 3.99), "{middles:?}");
    }
    let rows: std::collections::BTreeSet<i32> = middles.iter().map(|q| (q[2] * 10.0).round() as i32).collect();
    assert_eq!(rows.len(), 2, "{middles:?}");
}
