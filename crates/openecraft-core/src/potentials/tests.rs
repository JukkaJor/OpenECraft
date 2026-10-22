use super::*;
use crate::geom::p;
use crate::reports::{Analysis, Severity};

fn sym(prj: &mut Project, key: &str, x: f32, y: f32, tag: &str, label: &str) {
    let def = crate::symbols::get(key).unwrap();
    let id = prj.new_id();
    prj.pages[0].symbols.push(SymbolInst {
        id,
        sym: key.into(),
        pos: p(x, y),
        tag: tag.into(),
        label: label.into(),
        pins: def.pins.iter().map(|x| x.label.to_string()).collect(),
        ..Default::default()
    });
}

fn wire(prj: &mut Project, x1: f32, y1: f32, x2: f32, y2: f32) -> u64 {
    let id = prj.new_id();
    prj.pages[0].wires.push(Wire { id, a: p(x1, y1), b: p(x2, y2), ..Default::default() });
    id
}

fn wire_mut(prj: &mut Project, id: u64) -> &mut Wire {
    prj.pages[0].wires.iter_mut().find(|w| w.id == id).unwrap()
}

/// `name` potential at (100, 100) with a wire down to a terminal -X1:1 and a wire
/// from the terminal on to (100, 140); returns (wire to the terminal, wire behind it).
fn feed(prj: &mut Project, key: &str, name: &str) -> (u64, u64) {
    sym(prj, key, 100.0, 96.0, "", name);
    let w1 = wire(prj, 100.0, 100.0, 100.0, 116.0);
    sym(prj, "term", 100.0, 120.0, "-X1", "1");
    let w2 = wire(prj, 100.0, 124.0, 100.0, 140.0);
    (w1, w2)
}

fn props(prj: &Project, wire: u64) -> WireProps {
    let nl = Netlist::build(prj, true);
    let open = Netlist::build(prj, false);
    props_of_wire(prj, &nl, &open, wire)
}

fn messages(prj: &Project) -> Vec<(Severity, String)> {
    Analysis::build(prj).issues.into_iter().map(|i| (i.severity, i.message)).collect()
}

#[test]
fn new_projects_get_the_iec_60204_preset() {
    let prj = Project::new_default();
    assert_eq!(prj.potential("L2").unwrap().color, "BK");
    assert_eq!(prj.potential("PE").unwrap().kind, PotentialKind::Pe);
    assert_eq!(prj.wire_defaults.color, "BU");
    // Old project files have neither: nothing changes for them.
    let old = Project::from_json(&Project::default().to_json()).unwrap();
    assert!(old.potentials.is_empty() && old.wire_defaults.is_default());
}

#[test]
fn preset_replaces_definitions_of_the_same_name() {
    let mut prj = Project::new_default();
    prj.potentials.push(PotentialDef { name: "24V2".into(), ..Default::default() });
    prj.apply_preset(Preset::Iec60445);
    assert_eq!(prj.potential("L1").unwrap().color, "BN");
    assert_eq!(prj.potentials.iter().filter(|p| p.name == "L1").count(), 1);
    assert!(prj.potential("24V2").is_some());
}

#[test]
fn wires_take_the_potential_also_behind_a_terminal() {
    let mut prj = Project::new_default();
    let (w1, w2) = feed(&mut prj, "pot_n", "N");
    for w in [w1, w2] {
        let p = props(&prj, w);
        assert_eq!((p.color.as_str(), p.cross_section.as_str()), ("BU", "2.5 mm²"));
        assert_eq!(p.color_source, Source::Potential("N".into()));
    }
}

#[test]
fn manual_values_win_per_field_and_connection() {
    let mut prj = Project::new_default();
    let (w1, w2) = feed(&mut prj, "pot_l1", "L1");
    let w3 = wire(&mut prj, 100.0, 140.0, 120.0, 140.0);
    wire_mut(&mut prj, w2).color = "RD".into();
    let p = props(&prj, w3);
    assert_eq!((p.color.as_str(), p.color_source), ("RD", Source::Manual));
    assert_eq!((p.cross_section.as_str(), p.cross_section_source), ("2.5 mm²", Source::Potential("L1".into())));
    // The other side of the terminal is another connection.
    assert_eq!(props(&prj, w1).color, "BK");
}

#[test]
fn nets_without_potential_take_the_project_default() {
    let mut prj = Project::new_default();
    sym(&mut prj, "coil", 100.0, 100.0, "-K1", "");
    let w = wire(&mut prj, 100.0, 104.0, 100.0, 140.0);
    let p = props(&prj, w);
    assert_eq!((p.color.as_str(), p.color_source), ("BU", Source::Default));
    prj.wire_defaults = WireDefaults::default();
    assert_eq!(props(&prj, w), WireProps::default());
}

#[test]
fn cable_cores_and_short_circuits_get_nothing_automatic() {
    let mut prj = Project::new_default();
    let (w1, _) = feed(&mut prj, "pot_l1", "L1");
    wire_mut(&mut prj, w1).cable = "-W1".into();
    assert_eq!(props(&prj, w1).color, "");
    let mut prj = Project::new_default();
    let (w1, _) = feed(&mut prj, "pot_l1", "L1");
    sym(&mut prj, "pot_l2", 120.0, 96.0, "", "L2");
    wire(&mut prj, 120.0, 100.0, 100.0, 100.0);
    let p = props(&prj, w1);
    assert_eq!(p.potential, "L1 / L2");
    assert_eq!(p.color, "BU");
    assert_eq!(p.color_source, Source::Default);
}

#[test]
fn connections_carry_the_automatic_values() {
    let mut prj = Project::new_default();
    sym(&mut prj, "pot_24", 100.0, 96.0, "", "+24V");
    wire(&mut prj, 100.0, 100.0, 100.0, 116.0);
    sym(&mut prj, "pb_no", 100.0, 120.0, "-S1", "");
    sym(&mut prj, "coil", 140.0, 120.0, "-K1", "");
    wire(&mut prj, 100.0, 100.0, 140.0, 100.0);
    wire(&mut prj, 140.0, 100.0, 140.0, 116.0);
    let an = Analysis::build(&prj);
    let c = an.connections.iter().find(|c| c.net == "+24V").unwrap();
    assert_eq!((c.color.as_str(), c.cross_section.as_str()), ("BU", "0.75 mm²"));
}

#[test]
fn check_reserves_green_yellow_for_protective_conductors() {
    let mut prj = Project::new_default();
    let (w1, _) = feed(&mut prj, "pot_pe", "PE");
    wire_mut(&mut prj, w1).color = "BK".into();
    assert!(messages(&prj).iter().any(|(s, m)| *s == Severity::Error && m.contains("Protective conductor PE")));
    let mut prj = Project::new_default();
    let (w1, _) = feed(&mut prj, "pot_n", "N");
    wire_mut(&mut prj, w1).color = "gn/ye".into();
    assert!(messages(&prj).iter().any(|(s, m)| *s == Severity::Error && m.contains("reserved for protective")));
    // PE without definition (old project) is recognised by its name.
    let mut prj = Project::default();
    let (w1, _) = feed(&mut prj, "pot_pe", "PE");
    wire_mut(&mut prj, w1).color = "BK".into();
    assert!(messages(&prj).iter().any(|(_, m)| m.contains("Protective conductor PE")));
}

#[test]
fn check_reports_differences_and_missing_definitions() {
    let mut prj = Project::new_default();
    let (w1, _) = feed(&mut prj, "pot_l1", "L1");
    wire_mut(&mut prj, w1).color = "BN".into();
    wire_mut(&mut prj, w1).cross_section = "2,5 mm²".into();
    sym(&mut prj, "pot", 200.0, 96.0, "", "P7");
    prj.potentials.push(PotentialDef { name: "N".into(), ..Default::default() });
    let m = messages(&prj);
    assert!(m.iter().any(|(s, m)| *s == Severity::Warning && m.contains("wire colour BN differs")));
    assert!(!m.iter().any(|(_, m)| m.contains("cross-section 2,5 mm² differs")));
    assert!(m.iter().any(|(s, m)| *s == Severity::Info && m.contains("Potential P7 has no definition")));
    assert!(m.iter().any(|(s, m)| *s == Severity::Error && m.contains("Potential N defined 2 times")));
    // Projects without definitions are not asked for them.
    let mut prj = Project::default();
    sym(&mut prj, "pot", 200.0, 96.0, "", "P7");
    assert!(!messages(&prj).iter().any(|(_, m)| m.contains("no definition")));
}

#[test]
fn two_colour_codes_have_two_parts() {
    assert_eq!(color_parts("GN/YE"), vec![color_rgb("GN").unwrap(), color_rgb("YE").unwrap()]);
    assert_eq!(color_parts("WHBU").len(), 2);
    assert_eq!(color_parts("RD"), vec![color_rgb("RD").unwrap()]);
    assert!(color_parts("1").is_empty() && color_parts("XXYY").is_empty());
}

#[test]
fn wire_colors_cover_every_wire_with_a_colour() {
    let mut prj = Project::new_default();
    let (w1, w2) = feed(&mut prj, "pot_pe", "PE");
    let w3 = wire(&mut prj, 300.0, 100.0, 300.0, 140.0);
    prj.wire_defaults = WireDefaults::default();
    let colors = wire_colors(&prj, &Netlist::build(&prj, true), &Netlist::build(&prj, false));
    assert_eq!(colors.get(&w1).map(String::as_str), Some("GNYE"));
    assert_eq!(colors.get(&w2).map(String::as_str), Some("GNYE"));
    assert!(!colors.contains_key(&w3));
}

#[test]
fn wires_are_exported_in_their_colour_when_enabled() {
    let mut prj = Project::new_default();
    feed(&mut prj, "pot_n", "N");
    sym(&mut prj, "pot_pe", 200.0, 96.0, "", "PE");
    wire(&mut prj, 200.0, 100.0, 200.0, 140.0);
    let svg = |prj: &Project| crate::svg::page_svg(prj, 0);
    assert!(!svg(&prj).contains("#1e5ad2"));
    prj.wire_colors = true;
    let out = svg(&prj);
    assert!(out.contains("#1e5ad2"), "blue N wires");
    assert!(out.contains("#1e9632") && out.contains("#ebcd00"), "green-yellow PE wires");
    assert!(out.contains("stroke-dasharray"));
}

#[test]
fn colour_codes() {
    assert_eq!(normalize_color(" gn-ye "), "GNYE");
    assert_eq!(color_rgb("bu"), Some([30, 90, 210]));
    assert_eq!(color_rgb("XX"), None);
}

#[test]
fn hints_name_the_source() {
    assert!(WireProps::hint("BU", &Source::Potential("N".into())).contains("N"));
    assert!(WireProps::hint("BU", &Source::Manual).starts_with("BU"));
    assert!(WireProps::hint("", &Source::None).is_empty());
}
