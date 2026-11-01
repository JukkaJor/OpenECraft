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
    let ways: Vec<Way> = items.iter().map(|it| Way { points: &it.points, radii: &it.radii, diameter: it.diameter, rest: [0.0; 3], below: f32::INFINITY, steps: 2 }).collect();
    let laid = spread_ways(&ways, 25.0, &[], &[], &[]);
    let (a, b) = (&laid[0].0, &laid[1].0);
    assert_eq!(laid[2].0, items[2].points, "alone it stays as it is");
    // The ends stay where they are; the stretches at the ends lie side by side too and
    // run from their lanes to the end points.
    assert_eq!((a[0], a[a.len() - 1], b[0], b[b.len() - 1]), (route[0], route[4], route[0], route[4]));
    assert!(crate::parts::vec3::dist(a[1], b[1]) > 10.0, "{a:?} {b:?}");
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
    let ways: Vec<Way> = (0..5).map(|_| Way { points: &line, radii: &[], diameter: 3.0, rest: [0.0, 0.0, 1.0], below: 3.0, steps: 2 }).collect();
    let laid = spread_ways(&ways, 8.0, &[], &[], &[]);
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

#[test]
fn lanes_keep_out_of_devices_beside_them() {
    // Three cables run together along x; a device lies right beside them on one side
    // (y below -6): centred, a lane would run through it, so they go to the other side.
    let line = vec![[0.0, -300.0, 0.0], [0.0, 0.0, 0.0], [500.0, 0.0, 0.0], [500.0, 300.0, 0.0]];
    let ways: Vec<Way> = (0..3).map(|_| Way { points: &line, radii: &[], diameter: 10.0, rest: [0.0; 3], below: f32::INFINITY, steps: 2 }).collect();
    let device = ([100.0, -100.0, -50.0], [400.0, -6.0, 50.0]);
    let middle = |p: &[V3]| p.windows(2).find(|s| (s[1][0] - s[0][0]).abs() > 400.0).map(|s| [0, 1, 2].map(|k| (s[0][k] + s[1][k]) / 2.0)).unwrap();
    let free = spread_ways(&ways, 25.0, &[], &[], &[]);
    let kept = spread_ways(&ways, 25.0, &[device], &[], &[]);
    let inside = |q: V3| (0..3).all(|k| q[k] > device.0[k] - 5.0 && q[k] < device.1[k] + 5.0);
    assert!(free.iter().any(|(p, _)| inside(middle(p))), "centred one lane runs through it");
    assert!(kept.iter().all(|(p, _)| !inside(middle(p))), "{kept:?}");
    // Still apart from each other.
    let middles: Vec<V3> = kept.iter().map(|(p, _)| middle(p)).collect();
    for (i, a) in middles.iter().enumerate() {
        assert!(middles[i + 1..].iter().all(|b| crate::parts::vec3::dist(*a, *b) >= 10.99), "{middles:?}");
    }
    // Hemmed in on both sides they stay as they are.
    let other = ([100.0, 6.0, -50.0], [400.0, 100.0, 50.0]);
    let below = ([100.0, -100.0, -100.0], [400.0, 100.0, -6.0]);
    let above = ([100.0, -100.0, 6.0], [400.0, 100.0, 100.0]);
    let stuck = spread_ways(&ways, 25.0, &[device, other, below, above], &[], &[]);
    assert!(stuck.iter().all(|(p, _)| middle(p)[1..] == middle(&line)[1..]), "{stuck:?}");
}

#[test]
fn ways_along_a_duct_lie_inside_it() {
    // Six wires along a duct (x 0 .. 500), inside 40 wide (y) and 30 deep (z) on a
    // plate at z 0; all on the duct's middle line.
    let line = vec![[-50.0, 20.0, -100.0], [-50.0, 20.0, 15.0], [550.0, 20.0, 15.0], [550.0, 200.0, 15.0]];
    let ways: Vec<Way> = (0..6).map(|_| Way { points: &line, radii: &[], diameter: 8.0, rest: [0.0; 3], below: f32::INFINITY, steps: 2 }).collect();
    let duct = Channel { inside: ([0.0, 0.0, 0.0], [500.0, 40.0, 30.0]), floor: [0.0, 0.0, 1.0], rim: 0.0 };
    let laid = spread_ways(&ways, 8.0, &[], &[duct], &[]);
    let middles: Vec<V3> = laid.iter().map(|(p, _)| p.windows(2).find(|s| (s[1][0] - s[0][0]).abs() > 500.0).map(|s| [0, 1, 2].map(|k| (s[0][k] + s[1][k]) / 2.0)).unwrap()).collect();
    for q in &middles {
        assert!(q[1] >= 4.0 - 0.01 && q[1] <= 36.0 + 0.01 && q[2] >= 4.0 - 0.01 && q[2] <= 26.0 + 0.01, "{middles:?}");
    }
    for (i, a) in middles.iter().enumerate() {
        assert!(middles[i + 1..].iter().all(|b| crate::parts::vec3::dist(*a, *b) >= 8.0), "{middles:?}");
    }
}

#[test]
fn strands_through_each_other_are_pushed_apart() {
    // Least distance between the two, but within 30 mm of the ends (x 0 and 400).
    let gap = |a: &[V3], b: &[V3]| {
        let mut least = f32::INFINITY;
        for s in a.windows(2) {
            for t in b.windows(2) {
                for k in 0..=20 {
                    let p = crate::parts::vec3::add(s[0], crate::parts::vec3::sub(s[1], s[0]).map(|v| v * k as f32 / 20.0));
                    if p[0] < 30.0 || p[0] > 370.0 {
                        continue;
                    }
                    let ab = crate::parts::vec3::sub(t[1], t[0]);
                    let u = (dot(crate::parts::vec3::sub(p, t[0]), ab) / dot(ab, ab)).clamp(0.0, 1.0);
                    least = least.min(crate::parts::vec3::dist(p, crate::parts::vec3::add(t[0], ab.map(|v| v * u))));
                }
            }
        }
        least
    };
    // Two wires 6 mm thick lying on a plate (z 0, may not come nearer): one along x,
    // one across it, both at z 3.
    fn strand(points: &[V3], rest: V3, below: f32) -> Strand<'_> {
        Strand { points, diameter: 6.0, rest, below, tails: [0.0; 2] }
    }
    let along = [[0.0, 0.0, 3.0], [400.0, 0.0, 3.0]];
    let across = [[200.0, -200.0, 3.0], [200.0, 200.0, 3.0]];
    let on_plate = ([0.0, 0.0, 1.0], 0.0);
    let apart = separate(&[strand(&along, on_plate.0, on_plate.1), strand(&across, on_plate.0, on_plate.1)], &[], &[], &[]);
    assert!(gap(&apart[0], &apart[1]) >= 6.0 - 0.1, "{apart:?}");
    // One goes over the other, none into the plate; the ends stay.
    assert!(apart.iter().flatten().all(|q| q[2] >= 3.0 - 1e-3));
    assert_eq!((apart[0][0], *apart[0].last().unwrap()), (along[0], along[1]));
    // Where they did not move they stay straight.
    assert!(apart[0].len() < 20 && apart[1].len() < 20, "{apart:?}");

    // Two lying on each other in a channel (60 wide, 20 deep) move apart inside it.
    let same = [[0.0, 30.0, 10.0], [400.0, 30.0, 10.0]];
    let channel = || Channel { inside: ([-10.0, 0.0, 0.0], [410.0, 60.0, 20.0]), floor: [0.0, 0.0, 1.0], rim: 0.0 };
    let free = [strand(&same, [0.0; 3], f32::INFINITY), strand(&same, [0.0; 3], f32::INFINITY)];
    let apart = separate(&free, &[], &[channel()], &[]);
    assert!(gap(&apart[0], &apart[1]) >= 6.0 - 0.1, "{apart:?}");
    assert!(apart.iter().flatten().all(|q| q[1] >= 3.0 - 1e-3 && q[1] <= 57.0 + 1e-3 && q[2] >= 3.0 - 1e-3 && q[2] <= 17.0 + 1e-3), "{apart:?}");

    // A device in the way is kept out of.
    let device = ([150.0, 33.5, 0.0], [250.0, 100.0, 50.0]);
    let apart = separate(&free, &[device], &[channel()], &[]);
    for q in apart.iter().flatten() {
        assert!(!(q[0] > 150.0 && q[0] < 250.0 && q[1] + 3.0 > 33.5 + 1e-3), "{q:?}");
    }
}

#[test]
fn wires_on_terminals_closer_than_they_are_thick_leave_them_straight() {
    // Three wires 10 mm thick on terminals 6 mm apart at the front (z 100): straight back
    // to z 20, along y to 400 and forward to terminals as close at the other end.
    let lines: Vec<Vec<V3>> = (0..3)
        .map(|i| {
            let x = 6.0 * i as f32;
            vec![[x, 0.0, 100.0], [x, 0.0, 20.0], [x, 400.0, 20.0], [x, 400.0, 100.0]]
        })
        .collect();
    let radii = [0.0, 20.0, 20.0, 0.0];
    let ways: Vec<Way> = lines.iter().map(|l| Way { points: l, radii: &radii, diameter: 10.0, rest: [0.0; 3], below: f32::INFINITY, steps: 4 }).collect();
    let rounds = std::cell::Cell::new(0);
    let laid = lay_out(&ways, 8.0, &[], &[], &[], &|n, total| {
        assert_eq!(total, ROUNDS);
        rounds.set(n + 1);
        true
    })
    .expect("not stopped");
    assert!(rounds.get() < ROUNDS, "pushed apart until the end: {}", rounds.get());
    for (l, p) in lines.iter().zip(&laid) {
        assert_eq!((p[0], p[p.len() - 1]), (l[0], l[3]));
        // From each terminal straight down to its lane: no hook where they come together.
        let n = p.len();
        assert!(p[1][2] < 60.0 && p[n - 2][2] < 60.0, "{p:?}");
    }
    // Along y apart from each other.
    let at = |p: &[V3]| p.iter().copied().find(|q| (q[1] - 200.0).abs() < 1.0).or_else(|| p.windows(2).find(|s| s[0][1] < 200.0 && s[1][1] > 200.0).map(|s| s[0]));
    let middles: Vec<V3> = laid.iter().map(|p| at(p).unwrap_or_else(|| panic!("{p:?}"))).collect();
    for (i, a) in middles.iter().enumerate() {
        for b in &middles[i + 1..] {
            let across = ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
            assert!(across >= 10.0 - 0.1, "{middles:?}");
        }
    }
    // Stopped, there is none.
    assert!(lay_out(&ways, 8.0, &[], &[], &[], &|_, _| false).is_none());
}

#[test]
fn strands_are_pushed_no_further_than_a_few_times_their_thickness() {
    // Three strands 30 mm thick lying on each other in free air, with an obstacle round
    // them they started in (so nothing holds them): none goes far from where it was.
    let same = [[0.0, 0.0, 0.0], [1000.0, 0.0, 0.0]];
    let strand = || Strand { points: &same, diameter: 30.0, rest: [0.0; 3], below: f32::INFINITY, tails: [0.0; 2] };
    let apart = separate(&[strand(), strand(), strand()], &[], &[], &[]);
    let most = MOST_MOVE * 30.0 + MOST_MOVE_EXTRA + 0.01;
    for q in apart.iter().flatten() {
        assert!(q[1].hypot(q[2]) <= most, "{q:?}");
    }
}

#[test]
fn ways_cross_the_walls_of_a_duct_above_their_rim() {
    // A duct along x (inside y 0 .. 40, z 0 .. 70 on the plate z 0) with its walls closed
    // up to 20 above its floor; a wire 10 mm thick lying in it at z 5 leaves it across
    // the wall at y 40.
    let line = vec![[100.0, 20.0, 5.0], [300.0, 20.0, 5.0], [300.0, 200.0, 5.0]];
    let way = Way { points: &line, radii: &[0.0, 30.0, 0.0], diameter: 10.0, rest: [0.0; 3], below: f32::INFINITY, steps: 2 };
    let duct = Channel { inside: ([0.0, 0.0, 0.0], [500.0, 40.0, 70.0]), floor: [0.0, 0.0, 1.0], rim: 20.0 };
    let (points, _) = spread_ways(&[way], 8.0, &[], &[duct], &[]).remove(0);
    // Where it crosses y 40 it lies 25 above the floor at least; its ends stay.
    let crossing = points.windows(2).find(|s| s[0][1] != s[1][1] && (s[0][1] - 40.0) * (s[1][1] - 40.0) <= 0.0).expect("crosses the wall");
    let t = (40.0 - crossing[0][1]) / (crossing[1][1] - crossing[0][1]);
    let z = crossing[0][2] + (crossing[1][2] - crossing[0][2]) * t;
    assert!(z >= 25.0 - 0.01, "{points:?}");
    assert_eq!((points[0], *points.last().unwrap()), (line[0], line[2]));
    // Without a rim it stays as it is.
    let open = Channel { rim: 0.0, ..duct };
    let way = Way { points: &line, radii: &[0.0, 30.0, 0.0], diameter: 10.0, rest: [0.0; 3], below: f32::INFINITY, steps: 2 };
    assert_eq!(spread_ways(&[way], 8.0, &[], &[open], &[]).remove(0).0, line);
}

#[test]
fn strands_over_the_rim_of_a_duct_stay_over_it() {
    // Two wires 10 mm thick lying on each other in a duct (floor z 0, walls closed up to
    // 20), both up over the rim and out across the wall at y 40: pushed apart, the parts
    // inside the duct before the wall stay 25 above the floor at least.
    let duct = Channel { inside: ([0.0, 0.0, 0.0], [500.0, 40.0, 70.0]), floor: [0.0, 0.0, 1.0], rim: 20.0 };
    let line = vec![[100.0, 20.0, 5.0], [300.0, 20.0, 5.0], [300.0, 20.0, 25.0], [300.0, 46.0, 25.0], [300.0, 200.0, 25.0]];
    fn strand(points: &[V3]) -> Strand<'_> {
        Strand { points, diameter: 10.0, rest: [0.0; 3], below: f32::INFINITY, tails: [0.0; 2] }
    }
    let apart = separate(&[strand(&line), strand(&line)], &[], &[duct], &[]);
    for s in &apart {
        let crossing = s.windows(2).find(|w| w[0][1] < 40.0 && w[1][1] >= 40.0).expect("crosses the wall");
        let t = (40.0 - crossing[0][1]) / (crossing[1][1] - crossing[0][1]);
        assert!(crossing[0][2] + (crossing[1][2] - crossing[0][2]) * t >= 25.0 - 0.01, "{apart:?}");
    }
    // A way through a duct from wall to wall (across y) low in it rises over both rims.
    let through = vec![[200.0, -100.0, 5.0], [200.0, 150.0, 5.0]];
    let way = Way { points: &through, radii: &[0.0, 0.0], diameter: 10.0, rest: [0.0; 3], below: f32::INFINITY, steps: 2 };
    let (points, _) = spread_ways(&[way], 8.0, &[], &[duct], &[]).remove(0);
    for wall in [0.0, 40.0] {
        let crossing = points.windows(2).find(|s| s[0][1] != s[1][1] && (s[0][1] - wall) * (s[1][1] - wall) <= 0.0).expect("crosses the wall");
        let t = (wall - crossing[0][1]) / (crossing[1][1] - crossing[0][1]);
        assert!(crossing[0][2] + (crossing[1][2] - crossing[0][2]) * t >= 25.0 - 0.01, "{points:?}");
    }
}
