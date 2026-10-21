use super::*;
use crate::sample::demo_project;

#[test]
fn door_harness_of_the_demo() {
    let prj = demo_project();
    let an = Analysis::build(&prj);
    let ws = wires(&prj, &an, "-WH1");
    assert!(!ws.is_empty());
    // Every wire has a door device at least on one connection of its potential, and
    // all of them have a length (routed in the panel or the harness length).
    assert!(ws.iter().all(|w| w.length > 0.0));
    assert!(ws.iter().any(|w| w.routed));
    let cons = connectors(&ws);
    assert!(cons.iter().any(|c| c.device == "-S1"));
    // Each wire appears at both of its ends.
    assert_eq!(cons.iter().map(|c| c.pins.len()).sum::<usize>(), 2 * ws.len());
    let d = bundle_diameter(&ws);
    assert!(d > outer_diameter("1.5 mm²") && d < 50.0);
    // Report and check.
    let t = &an.reports[&crate::model::ReportKind::HarnessData];
    assert_eq!(t.rows.len(), ws.len() + 1);
    assert!(!an.issues.iter().any(|i| i.message.contains("harness")));
}

#[test]
fn wire_diameters() {
    assert!((outer_diameter("1.5 mm²") - 2.96).abs() < 0.01);
    assert!(outer_diameter("2.5") > outer_diameter("0.75"));
}

#[test]
fn only_the_assigned_connections_belong_to_the_harness() {
    let prj = demo_project();
    let an = Analysis::build(&prj);
    let door = ["-S1", "-S2", "-S3", "-S4", "-P1", "-P2"];
    let ws = wires(&prj, &an, "-WH1");
    // Every harness wire has a door device at one end ...
    assert!(ws.iter().all(|w| door.contains(&w.from.0.as_str()) || door.contains(&w.to.0.as_str())));
    // ... while other connections of the same potential sections stay outside.
    let section_mates = an
        .connections
        .iter()
        .filter(|c| c.harness.is_empty())
        .filter(|c| an.connections.iter().any(|d| d.harness == "-WH1" && (d.a == c.a || d.b == c.a || d.a == c.b || d.b == c.b)))
        .count();
    assert!(section_mates > 0, "the demo has connections next to harness wires that are not in the harness");
}

#[test]
fn assignment_by_selected_wires() {
    let mut prj = demo_project();
    for h in &mut prj.harnesses {
        h.connections.clear();
    }
    let an = Analysis::build(&prj);
    // The wires of one connection that lies on a single page.
    let ci = an.connections.iter().position(|c| c.a.page == c.b.page).unwrap();
    let c = an.connections[ci].clone();
    let page = c.a.page;
    let sel: BTreeSet<u64> = prj.pages[page].wires.iter().map(|w| w.id).collect();
    let found = connections_at(&prj, &an, page, &sel);
    assert!(found.contains(&ci));
    // No wires selected: no connection.
    assert!(connections_at(&prj, &an, page, &BTreeSet::new()).is_empty());
    let (a, b) = (pin_designation(&prj, c.a), pin_designation(&prj, c.b));
    assign(&mut prj, &a, &b, "-WH1");
    assert_eq!(of(&prj, &b, &a), "-WH1");
    let an = Analysis::build(&prj);
    assert_eq!(an.connections.iter().filter(|c| c.harness == "-WH1").count(), 1);
    // The connections of a wire: those of its potential section.
    let wire = prj.pages[page].wires[0].id;
    assert!(!connections_of_wire(&an, wire).is_empty());
    assign(&mut prj, &a, &b, "");
    assert_eq!(of(&prj, &a, &b), "");
}

#[test]
fn renamed_devices_keep_and_stale_assignments_are_reported() {
    let mut prj = demo_project();
    let before = wires(&prj, &Analysis::build(&prj), "-WH1").len();
    crate::ops::rename_device(&mut prj, "-S1", "-S11");
    let an = Analysis::build(&prj);
    assert_eq!(wires(&prj, &an, "-WH1").len(), before);
    assert!(stale(&prj, &an).is_empty());
    // An assignment of a connection that does not exist.
    prj.harnesses[0].connections.push(key("-X9:1", "-K9:A1"));
    let an = Analysis::build(&prj);
    assert_eq!(stale(&prj, &an), vec![("-WH1".to_string(), key("-K9:A1", "-X9:1"))]);
    assert!(an.issues.iter().any(|i| i.message.contains("-K9:A1 – -X9:1 does not exist")));
    // A harness without connections.
    prj.harnesses.push(Harness { id: 999, tag: "-WH2".into(), ..Default::default() });
    let an = Analysis::build(&prj);
    assert!(an.issues.iter().any(|i| i.message == "Harness -WH2 has no wires"));
}

#[test]
fn kbl_export() {
    let prj = demo_project();
    let an = Analysis::build(&prj);
    let ws = wires(&prj, &an, "-WH1");
    let x = kbl(&prj, &an);
    assert!(x.starts_with("<?xml"));
    assert!(x.contains("<Id>-WH1</Id>"));
    assert_eq!(x.matches("<Connection ").count(), ws.len());
    assert_eq!(x.matches("xsi:type=\"kbl:Wire_occurrence\"").count(), ws.len());
    assert_eq!(x.matches("<Connector_occurrence ").count(), connectors(&ws).len());
    // Every contact point a connection refers to exists.
    for cap in x.split("<Contact_point>").skip(1) {
        let id = &cap[..cap.find('<').unwrap()];
        assert!(x.contains(&format!("<Contact_point id=\"{id}\">")), "{id}");
    }
    assert_eq!(x.matches("<Harness ").count(), x.matches("</Harness>").count());
    assert_eq!(xml("a<b&\"c\""), "a&lt;b&amp;&quot;c&quot;");
}
