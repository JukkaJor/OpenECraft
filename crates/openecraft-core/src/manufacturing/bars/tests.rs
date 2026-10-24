use super::*;
use crate::panel::{Enclosure, PanelItem};

fn l_bar() -> CopperBar {
    CopperBar {
        id: 1,
        enclosure: "+CAB1".into(),
        name: "L1".into(),
        width: 20.0,
        thickness: 5.0,
        count: 1,
        segments: vec![100.0, 100.0],
        bends: vec![Bend { angle: 90.0, ..Default::default() }],
        ..Default::default()
    }
}

#[test]
fn developed_length_after_din_6935() {
    assert_eq!(din_factor(5.0, 5.0), 0.65);
    assert_eq!(din_factor(50.0, 5.0), 1.0);
    let (a, sb) = bend_allowance(5.0, 5.0, 90.0, 0.0);
    assert!((a - std::f32::consts::PI * (5.0 + 2.5 * 0.65) / 2.0).abs() < 1e-3);
    assert!((sb - 10.0).abs() < 1e-3);
    let f = l_bar().flat(&BarRules::default());
    assert!((f.length - (180.0 + a)).abs() < 1e-2, "{}", f.length);
    assert_eq!(f.bends.len(), 1);
    assert!((f.bends[0].at - (90.0 + a / 2.0)).abs() < 1e-2);
    assert!((f.bends[0].set_angle - 91.8).abs() < 1e-3);
    assert!(f.problems.is_empty(), "{:?}", f.problems);
    // Edgewise: the width is bent.
    let mut b = l_bar();
    b.bends[0].edgewise = true;
    let e = b.flat(&BarRules::default());
    assert!(e.bends[0].radius == 20.0 && e.length < f.length);
}

#[test]
fn holes_flanges_and_radii_are_checked() {
    let mut b = l_bar();
    b.end_holes = 9.0;
    b.holes.push(BarHole { segment: 1, at: 50.0, diameter: 9.0 });
    let f = b.flat(&BarRules::default());
    assert_eq!(f.holes.len(), 3);
    assert_eq!(f.holes[0], (20.0, 9.0));
    assert!(f.problems.is_empty(), "{:?}", f.problems);
    b.holes.push(BarHole { segment: 1, at: 12.0, diameter: 9.0 });
    b.holes.push(BarHole { segment: 0, at: 40.0, diameter: 25.0 });
    b.segments = vec![100.0, 30.0];
    b.bends[0].radius = 2.0;
    let f = b.flat(&BarRules::default());
    let text = f.problems.join("\n");
    assert!(text.contains("too close to bend 1"), "{text}");
    assert!(text.contains("too large"), "{text}");
    assert!(text.contains("straight"), "{text}");
    assert!(text.contains("radius"), "{text}");
    b.bends.clear();
    assert!(b.flat(&BarRules::default()).problems.iter().any(|p| p.contains("segments")));
}

#[test]
fn bends_are_made_from_the_ends_inwards() {
    let mut b = l_bar();
    b.segments = vec![50.0, 300.0, 300.0, 80.0];
    b.bends = vec![Bend { angle: 90.0, ..Default::default() }, Bend { angle: -90.0, edgewise: true, ..Default::default() }, Bend { angle: 45.0, ..Default::default() }];
    let f = b.flat(&BarRules::default());
    assert_eq!(f.sequence, vec![0, 2, 1]);
}

#[test]
fn cutting_from_stock_lengths() {
    let piece = |l: f32| Piece { label: format!("{l}"), enclosure: "+CAB1".into(), width: 20.0, thickness: 5.0, length: l, system: false };
    let mut list = vec![piece(1000.0), piece(3000.0), piece(900.0), piece(1500.0), piece(5000.0)];
    list.push(Piece { width: 30.0, ..piece(500.0) });
    let (bars, too_long) = cutting(&list, &BarRules::default());
    assert_eq!(too_long.len(), 1);
    let lengths: Vec<Vec<f32>> = bars.iter().map(|b| b.pieces.iter().map(|p| p.length).collect()).collect();
    assert_eq!(lengths, vec![vec![3000.0, 900.0], vec![1500.0, 1000.0], vec![500.0]]);
    assert!((bars[0].rest - 97.0).abs() < 1e-3);
}

#[test]
fn busbar_systems_and_programs() {
    let mut prj = Project::default();
    let mut e = Enclosure::default();
    e.items.push(PanelItem { id: 7, kind: ItemKind::Busbar, part: crate::panel::BUSBAR_PART.into(), w: 400.0, h: 75.0, ..Default::default() });
    prj.layout.enclosures.push(e);
    let mut b = l_bar();
    b.count = 3;
    b.end_holes = 11.0;
    prj.layout.manufacturing.bars.push(b);
    let ps = pieces(&prj);
    assert_eq!(ps.len(), 6);
    assert!(ps.iter().filter(|p| p.system).all(|p| p.length == 400.0 && (p.width, p.thickness) == (12.0, 5.0)));
    let m = crate::manufacturing::machines::find(&prj, crate::manufacturing::machines::DEFAULT_BAR).unwrap();
    let (text, found) = program(&prj, &m, None);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(text.lines().filter(|l| l.contains(";cut;")).count(), 4);
    assert_eq!(text.lines().filter(|l| l.contains(";punch;")).count(), 2);
    assert!(text.lines().any(|l| l.contains(";bend;") && l.contains(";91.8;R5;0")), "{text}");
    assert_eq!(program(&prj, &m, Some("+X")).0.lines().count(), 1);
    assert!(problems(&prj).is_empty());
    assert_eq!(bar_profile(None), (12.0, 5.0));
}

#[test]
fn parsing() {
    assert_eq!(parse_segments("200, 150;300").unwrap(), vec![200.0, 150.0, 300.0]);
    assert!(parse_segments("200, x").is_err());
    let b = parse_bends("90; -90 e; 45 r10").unwrap();
    assert_eq!(b[1], Bend { angle: -90.0, edgewise: true, radius: 0.0 });
    assert_eq!(b[2].radius, 10.0);
    assert!(parse_bends("180").is_err() && parse_bends("90 q").is_err());
    assert_eq!(parse_holes("0:20:9; 2:30:11").unwrap()[1], BarHole { segment: 2, at: 30.0, diameter: 11.0 });
    assert!(parse_holes("0:20").is_err());
}
