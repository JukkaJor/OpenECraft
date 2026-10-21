use super::*;
use crate::sample::demo_project;

fn labels(prj: &Project) -> Vec<String> {
    terminals(prj, "-X1").into_iter().map(|t| t.label).collect()
}

#[test]
fn order_moves_and_renumbering() {
    let mut prj = demo_project();
    assert_eq!(strip_tags(&prj), vec!["-X1".to_string()]);
    assert_eq!(labels(&prj), ["1", "2", "3", "PE"]);
    move_terminal(&mut prj, "-X1", 2, true);
    assert_eq!(labels(&prj), ["1", "3", "2", "PE"]);
    move_terminal(&mut prj, "-X1", 0, true); // already first
    assert_eq!(labels(&prj), ["1", "3", "2", "PE"]);
    // Renumbering follows the strip order, PE keeps its designation; jumpers move along.
    prj.strips.get_mut("-X1").unwrap().jumpers.insert("3".into(), 1);
    assert_eq!(renumber(&mut prj, "-X1"), 2);
    assert_eq!(labels(&prj), ["1", "2", "3", "PE"]);
    assert_eq!(prj.strips["-X1"].jumpers.get("2"), Some(&1));
}

#[test]
fn rows_include_jumpers_and_spares() {
    let mut prj = demo_project();
    let data = prj.strips.entry("-X1".into()).or_default();
    data.spares = 2;
    data.jumpers.insert("1".into(), 4);
    let nl = Netlist::build(&prj, true);
    let open = Netlist::build(&prj, false);
    let rows = rows(&prj, "-X1", &nl, &open);
    assert_eq!(rows.len(), 6);
    assert_eq!(rows[0].jumper, 4);
    assert!(rows[0].internal.contains("-M1:U1"), "{}", rows[0].internal);
    assert_eq!((rows[4].label.as_str(), rows[5].label.as_str()), ("4", "5"));
    assert!(rows[5].terminal.is_none());
    // Spares are in the bill of materials.
    let an = crate::reports::Analysis::build(&prj);
    let tb = an.reports[&ReportKind::BillOfMaterials].rows.iter().find(|r| r[3] == "GEN.TB-2.5").cloned().unwrap();
    assert_eq!(tb[1], "6");
}

#[test]
fn jumpers_join_potentials() {
    let mut prj = demo_project();
    let pot = |prj: &Project, label: &str| {
        let nl = Netlist::build(prj, true);
        let s = prj.pages.iter().flat_map(|p| &p.symbols).find(|s| s.tag == "-X1" && s.label == label).unwrap();
        nl.nets[nl.pin_net[&(s.id, 0)]].name.clone()
    };
    assert_ne!(pot(&prj, "1"), pot(&prj, "2"));
    let data = prj.strips.entry("-X1".into()).or_default();
    data.jumpers.insert("1".into(), 1);
    data.jumpers.insert("2".into(), 1);
    assert_eq!(pot(&prj, "1"), pot(&prj, "2"));
    assert_eq!(jumper_groups(&prj), vec![("-X1".to_string(), vec!["1".to_string(), "2".to_string()])]);
    // The physical connections (wire numbers) stay apart.
    let open = Netlist::build(&prj, false);
    let id = |l: &str| prj.pages.iter().flat_map(|p| &p.symbols).find(|s| s.tag == "-X1" && s.label == l).unwrap().id;
    assert_ne!(open.pin_net[&(id("1"), 0)], open.pin_net[&(id("2"), 0)]);
}
