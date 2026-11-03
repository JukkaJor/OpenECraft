use super::*;
use crate::sample::example;

fn bar_with(prj: &Project, tag: &str) -> u64 {
    let (_, it) = crate::panel::locate(prj, tag).unwrap_or_else(|| panic!("{tag}"));
    let e = prj.layout.enclosures.iter().find(|e| e.items.iter().any(|i| i.id == it.id)).unwrap();
    e.items.iter().find(|b| b.kind == ItemKind::Busbar && b.y <= it.y + it.h && it.y <= b.y + b.h && b.x <= it.x && it.x <= b.x + b.w).unwrap().id
}

#[test]
fn each_bar_carries_one_conductor() {
    let prj = example("industrial_main_distribution").unwrap();
    let nl = Netlist::build(&prj, true);
    let uses = uses(&prj, &nl);
    // The supply busbar: the phases of both supply cables and the isolation strip.
    let supply = uses.iter().find(|u| u.busbar == bar_with(&prj, "=GRID+HV1-X0")).unwrap();
    assert_eq!((supply.bars, supply.nets.len()), (3, 3), "{supply:?}");
    assert!(supply.devices.contains(&"=MS+HV1-F10".to_string()));
    // The strips behind the coupling switch have a busbar of their own.
    let pv = uses.iter().find(|u| u.busbar == bar_with(&prj, "=PV+HV1-Q41")).unwrap();
    assert!(!pv.devices.contains(&"=DIST+HV1-Q25".to_string()), "{pv:?}");
    // The measured strips: the conductors through their transformers.
    let dist = uses.iter().find(|u| u.busbar == bar_with(&prj, "=DIST+HV2-Q21")).unwrap();
    assert_eq!((dist.devices.len(), dist.nets.len()), (4, 3), "{dist:?}");
    assert!(problems(&prj, &nl).is_empty());
}

#[test]
fn a_pen_terminal_on_the_phase_bars_is_a_short_circuit() {
    let mut prj = example("industrial_main_distribution").unwrap();
    // The PEN terminals moved onto the busbar of the phases.
    let (ei, x0) = crate::panel::locate(&prj, "=GRID+HV1-X0").map(|(ei, it)| (ei, it.clone())).unwrap();
    let x1 = prj.layout.enclosures[ei].items.iter_mut().find(|i| i.tag == "=GRID+HV1-X1").unwrap();
    (x1.x, x1.y) = (x0.x + x0.w + 20.0, x0.y);
    let nl = Netlist::build(&prj, true);
    let issues = problems(&prj, &nl);
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert!(issues[0].message().contains("3 bars joins 4 conductors"), "{}", issues[0].message());
}

#[test]
fn the_bars_are_drawn_apart() {
    let prj = example("industrial_main_distribution").unwrap();
    let id = bar_with(&prj, "=DIST+HV2-Q21");
    let it = prj.layout.enclosures.iter().flat_map(|e| &e.items).find(|i| i.id == id).unwrap();
    let tris = mesh(&prj, it);
    // Three copper bars across the system (y), apart from each other.
    let mut ys: Vec<f32> = tris.iter().filter(|t| t.1 == Some([200, 120, 60])).flat_map(|t| t.0.iter().map(|v| v[1])).collect();
    ys.sort_by(f32::total_cmp);
    ys.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    assert_eq!(ys.len(), 6, "{ys:?}");
    assert!(ys.windows(2).skip(1).step_by(2).all(|w| w[1] - w[0] > 50.0), "gaps between the bars: {ys:?}");
}
