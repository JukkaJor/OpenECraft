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
        item.legs = legs.into_iter().enumerate().map(|(k, points)| RouteLeg { enclosure: k, points }).collect();
    };
    let mut a = item("A", Class::Control, 6.0, Vec::new());
    leg(&mut a, [vec![[0.0, 0.0, 0.0], [100.0, 0.0, 0.0]], vec![[5000.0, 0.0, 0.0], [5100.0, 0.0, 0.0]]]);
    let mut b = item("B", Class::Control, 6.0, Vec::new());
    leg(&mut b, [vec![[0.0, 50.0, 0.0], [100.0, 0.0, 0.0]], vec![[5000.0, 0.0, 0.0], [5000.0, 300.0, 0.0]]]);
    assert_eq!(a.stretches().len(), 2);
    let (bundles, _) = find(&[a, b], 25.0);
    assert!(bundles.iter().all(|x| x.length < 200.0), "{bundles:?}");
}
