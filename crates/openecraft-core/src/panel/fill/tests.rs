use super::*;
use crate::panel::{DuctRun, EndKind, Enclosure, PanelItem, Route};
use crate::reports::{Analysis, Severity};

fn project_with_duct(part: &str) -> Project {
    let mut prj = Project::default();
    let mut e = Enclosure::default();
    e.items.push(PanelItem { id: 7, kind: ItemKind::Duct, part: part.into(), x: 0.0, y: 0.0, w: 400.0, h: 40.0, d: 60.0, ..Default::default() });
    prj.layout.enclosures.push(e);
    prj
}

fn wire(cross_section: &str, runs: &[(f32, f32)]) -> RoutedWire {
    RoutedWire {
        enclosure: 0,
        from: "-K1:13".into(),
        to: "-K2:14".into(),
        net: String::new(),
        color: String::new(),
        cross_section: cross_section.into(),
        number: String::new(),
        route: Route {
            points: Vec::new(),
            plate_length: 0.0,
            via_ducts: true,
            runs: runs.iter().map(|&(from, to)| DuctRun { duct: 7, from, to }).collect(),
        },
        length: 0.0,
        items: (1, 2),
        ends: (EndKind::Generated, EndKind::Generated),
    }
}

#[test]
fn fullest_stretch_of_overlapping_wires() {
    let prj = project_with_duct("NONE");
    let fill = duct_fill(&prj, &[wire("1.5 mm²", &[(0.0, 100.0)]), wire("1.5 mm²", &[(50.0, 150.0)])]);
    assert_eq!(fill.len(), 1);
    let f = &fill[0];
    // Default duct profile 40 x 60 mm.
    assert_eq!(f.capacity, 2400.0);
    assert_eq!((f.from, f.to, f.wires, f.total_wires), (50.0, 100.0, 2, 2));
    let expected = 2.0 * wire_area("1.5 mm²") / 2400.0 * 100.0;
    assert!((f.percent - expected).abs() < 1e-3, "{} vs {expected}", f.percent);
}

#[test]
fn wires_that_only_touch_are_not_side_by_side() {
    let prj = project_with_duct("NONE");
    let fill = duct_fill(&prj, &[wire("1.5", &[(0.0, 50.0)]), wire("1.5", &[(50.0, 100.0)])]);
    assert_eq!(fill[0].wires, 1);
    // The fullest stretch reaches over both, they are equally full.
    assert_eq!((fill[0].from, fill[0].to), (0.0, 100.0));
}

#[test]
fn capacity_from_the_duct_part_and_wire_sizes() {
    let mut prj = project_with_duct("OWN.DUCT");
    prj.parts.push(crate::parts::Part { number: "OWN.DUCT".into(), width: 1000.0, height: 60.0, depth: 80.0, ..Default::default() });
    let fill = duct_fill(&prj, &[wire("2.5", &[(0.0, 10.0)])]);
    assert_eq!(fill[0].capacity, 4800.0);
    assert!(wire_area("2.5") > wire_area("1.5"));
    assert!(wire_area("0.75") < wire_area("1.5"));
    // Unused ducts and zero length stretches are not listed.
    assert!(duct_fill(&prj, &[wire("1.5", &[(10.0, 10.0)])]).is_empty());
    assert!(duct_fill(&prj, &[]).is_empty());
}

#[test]
fn demo_ducts_in_check_and_report() {
    let mut prj = crate::sample::demo_project();
    let an = Analysis::build(&prj);
    assert!(!an.duct_fill.is_empty());
    assert!(an.duct_fill.iter().all(|f| f.percent > 0.0 && f.percent < prj.layout.duct_fill_limit));
    let report = &an.reports[&crate::model::ReportKind::DuctFill];
    assert_eq!(report.rows.len(), an.duct_fill.len());
    assert!(!an.issues.iter().any(|i| i.message.contains("cable duct at")), "demo stays below the limit");

    let fullest = an.duct_fill.iter().map(|f| f.percent).fold(0.0, f32::max);
    prj.layout.duct_fill_limit = fullest / 2.0;
    let an = Analysis::build(&prj);
    let warnings: Vec<&str> = an.issues.iter().filter(|i| i.severity == Severity::Warning).map(|i| i.message.as_str()).filter(|m| m.contains("cable duct at")).collect();
    assert!(!warnings.is_empty());
    assert!(warnings[0].starts_with("+CAB1: cable duct at "), "{}", warnings[0]);
    assert!(warnings[0].ends_with(&format!("(limit {:.0} %)", fullest / 2.0)));
}
