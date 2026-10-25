use super::*;
use crate::sample::demo_project;

fn wire(n: &str, a: &str, b: &str, length: f32) -> HarnessWire {
    HarnessWire {
        number: n.into(),
        from: (a.into(), "1".into()),
        to: (b.into(), n.into()),
        color: "BK".into(),
        cross_section: "0.75 mm²".into(),
        length,
        routed: length > 0.0,
        target: None,
    }
}

#[test]
fn branch_lengths_fit_the_wire_lengths() {
    // A–B 1000, A–C 700, B–C 900: a = 400, b = 600, c = 300.
    let b = branch_lengths(3, &[(0, 1, 1000.0), (0, 2, 700.0), (1, 2, 900.0)]);
    let v: Vec<f32> = b.iter().map(|x| x.0).collect();
    assert!((v[0] - 400.0).abs() < 1.0 && (v[1] - 600.0).abs() < 1.0 && (v[2] - 300.0).abs() < 1.0, "{v:?}");
    assert!(b.iter().all(|x| !x.1));
    let two = branch_lengths(2, &[(0, 1, 800.0)]);
    assert!((two[0].0 + two[1].0 - 800.0).abs() < 1.0);
    let unknown = branch_lengths(2, &[(0, 1, 0.0)]);
    assert_eq!(unknown, vec![(DEFAULT_BRANCH, true), (DEFAULT_BRANCH, true)]);
}

#[test]
fn branches_carry_their_pins_and_bundle() {
    let nb = from_wires("-WH9", "Test", "", &[wire("1", "-X1", "-S1", 900.0), wire("2", "-X1", "-S2", 1100.0), wire("3", "-X1", "-S2", 1100.0)]);
    assert_eq!(nb.wires, 3);
    let devices: Vec<&str> = nb.branches.iter().map(|b| b.device.as_str()).collect();
    assert_eq!(devices, ["-S1", "-S2", "-X1"]);
    let x1 = &nb.branches[2];
    assert_eq!(x1.pins.len(), 3);
    assert_eq!(x1.pins[0][4], "-S1:1");
    assert!(x1.diameter > nb.branches[0].diameter, "the shared branch is thicker");
    let angles: std::collections::BTreeSet<i32> = nb.branches.iter().map(|b| b.angle as i32).collect();
    assert_eq!(angles.len(), 3, "spread around the node");
}

#[test]
fn the_scale_fits_the_board_on_the_sheet() {
    let small = from_wires("-W", "", "", &[wire("1", "-A", "-B", 200.0)]);
    let big = from_wires("-W", "", "", &[wire("1", "-A", "-B", 20000.0)]);
    assert!(scale(&small) <= 2.0);
    let n = scale(&big);
    assert!((50.0..=100.0).contains(&n), "{n}");
    let (lo, hi) = extent(&big, n);
    assert!(hi.x - lo.x <= SHEET.0);
}

#[test]
fn the_demo_harness_draws_as_svg() {
    let prj = demo_project();
    let an = Analysis::checked(&prj);
    let nb = layout(&prj, &an, "-WH1").unwrap();
    assert!(nb.branches.len() >= 2);
    assert_eq!(nb.description, "Door harness +CAB1");
    let svg = svg(&nb);
    assert!(svg.starts_with("<svg") && svg.contains("-WH1"));
    for b in &nb.branches {
        assert!(svg.contains(&b.device));
    }
    assert!(layout(&prj, &an, "-WH99").is_none());
}

#[test]
fn short_branches_are_drawn_longer_and_long_and_short_alternate() {
    let nb = from_wires("-W", "", "", &[wire("1", "-A", "-B", 3000.0), wire("2", "-A", "-C", 1600.0), wire("3", "-A", "-D", 1550.0), wire("4", "-B", "-C", 1600.0)]);
    let n = scale(&nb);
    let shortest = nb.branches.iter().map(|b| b.length).fold(f32::MAX, f32::min);
    assert!(shortest / n < MIN_DRAWN);
    assert!(nb.branches.iter().all(|b| drawn(b, n) >= MIN_DRAWN));
    // The two longest branches are not neighbours.
    let mut order: Vec<&Branch> = nb.branches.iter().collect();
    order.sort_by(|a, b| b.angle.total_cmp(&a.angle));
    let longest: Vec<usize> = {
        let mut idx: Vec<usize> = (0..order.len()).collect();
        idx.sort_by(|a, b| order[*b].length.total_cmp(&order[*a].length));
        idx[..2].to_vec()
    };
    let gap = (longest[0] as i32 - longest[1] as i32).unsigned_abs() as usize;
    assert!(gap != 1 && gap != order.len() - 1, "{longest:?}");
}

#[test]
fn tables_do_not_overlap() {
    let prj = demo_project();
    let an = Analysis::checked(&prj);
    let nb = layout(&prj, &an, "-WH1").unwrap();
    let n = scale(&nb);
    let pl = placement(&nb, n);
    for i in 0..pl.len() {
        for j in 0..i {
            let ((wi, hi), (wj, hj)) = (table_size(&nb.branches[i]), table_size(&nb.branches[j]));
            let (a, b) = (pl[i].1, pl[j].1);
            let overlap = a.x < b.x + wj && b.x < a.x + wi && a.y < b.y + hj && b.y < a.y + hi;
            assert!(!overlap, "{} and {}", nb.branches[i].device, nb.branches[j].device);
        }
    }
}
