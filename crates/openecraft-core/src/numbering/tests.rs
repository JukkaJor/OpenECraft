use super::*;
use crate::geom::p;
use crate::model::{Page, PageKind, SymbolInst};

/// Two pages; coils and contacts placed out of order.
fn project() -> Project {
    let mut prj = Project::default();
    let id = prj.new_id();
    prj.pages.push(Page::new(id, "2", "Second", PageKind::Schematic));
    let frame = crate::frame::of_page(&prj, 0);
    let col = |c: usize| frame.area().min.x + frame.column_width() * c as f32 + 8.0;
    let add = |prj: &mut Project, page: usize, sym: &str, tag: &str, x: f32, y: f32| {
        let id = prj.new_id();
        prj.pages[page].symbols.push(SymbolInst { id, sym: sym.into(), pos: p(x, y), tag: tag.into(), ..Default::default() });
    };
    add(&mut prj, 0, "coil", "-K7", col(3), 100.0);
    add(&mut prj, 0, "coil", "-K3", col(1), 100.0);
    add(&mut prj, 0, "no", "-K3", col(5), 60.0);
    add(&mut prj, 1, "coil", "-K1", col(0), 100.0);
    add(&mut prj, 0, "coil", "=A2-K9", col(2), 100.0);
    add(&mut prj, 0, "term", "-X4", col(4), 140.0);
    prj.pages[0].symbols.last_mut().unwrap().label = "1".into();
    prj.devices.insert("-K3".into(), crate::model::DeviceData { description: "Pump".into(), ..Default::default() });
    prj
}

fn tags(prj: &Project) -> Vec<String> {
    let mut v: Vec<String> = prj.pages.iter().flat_map(|pg| pg.symbols.iter().map(|s| s.tag.clone())).collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn letters_of_device_names() {
    assert_eq!(letters_of("-K12").as_deref(), Some("K"));
    assert_eq!(letters_of("-KF3").as_deref(), Some("KF"));
    assert_eq!(letters_of("-1CR2").as_deref(), Some("CR"));
    assert_eq!(letters_of("-12"), None);
}

#[test]
fn devices_are_numbered_in_schematic_order_per_structure() {
    let mut prj = project();
    let renamed = renumber_devices(&mut prj, &DeviceNumbering::default());
    // -K3 (column 1) comes before -K7 (column 3) on page 1, -K1 on page 2 is last;
    // =A2-K9 is counted in its own structure.
    let pair = |a: &str, b: &str| (a.to_string(), b.to_string());
    assert_eq!(renamed, [pair("-K3", "-K1"), pair("=A2-K9", "=A2-K1"), pair("-K7", "-K2"), pair("-K1", "-K3")]);
    assert_eq!(tags(&prj), ["-K1", "-K2", "-K3", "-X4", "=A2-K1"]);
    // The contact moved with its coil, and so did the device data.
    let contact = prj.pages[0].symbols.iter().find(|s| s.sym == "no").unwrap();
    assert_eq!(contact.tag, "-K1");
    assert_eq!(prj.devices["-K1"].description, "Pump");
    assert!(!prj.devices.contains_key("-K3"));
    // Terminal strips keep their tags unless asked.
    assert_eq!(renumber_devices(&mut prj, &DeviceNumbering::default()), []);
    let all = DeviceNumbering { skip_terminals: false, ..Default::default() };
    assert_eq!(renumber_devices(&mut prj, &all), [("-X4".to_string(), "-X1".to_string())]);
}

#[test]
fn formats_scope_letters_and_released_pages() {
    let mut prj = project();
    let col_format = DeviceNumbering { format: "{letter}{page}{column}".into(), ..Default::default() };
    let plan = plan_devices(&prj, &col_format);
    assert!(plan.contains(&("-K3".into(), "-K12".into())), "{plan:?}");
    assert!(plan.contains(&("-K1".into(), "-K21".into())), "{plan:?}");

    let nfpa = DeviceNumbering { format: "{page}{column}{letter}".into(), ..Default::default() };
    assert!(plan_devices(&prj, &nfpa).contains(&("-K7".into(), "-14K".into())));

    let start = DeviceNumbering { start: 10, step: 5, pages: vec![0], ..Default::default() };
    let plan = plan_devices(&prj, &start);
    let pair = |a: &str, b: &str| (a.to_string(), b.to_string());
    assert_eq!(plan, [pair("-K3", "-K10"), pair("=A2-K9", "=A2-K10"), pair("-K7", "-K15")]);

    let only_q = DeviceNumbering { letters: vec!["Q".into()], ..Default::default() };
    assert!(plan_devices(&prj, &only_q).is_empty());

    // Two devices in one column without a counter: the second gets a suffix.
    let id = prj.new_id();
    let x = prj.pages[0].symbols.iter().find(|s| s.tag == "-K3").unwrap().pos.x;
    prj.pages[0].symbols.push(SymbolInst { id, sym: "coil".into(), pos: p(x, 200.0), tag: "-K8".into(), ..Default::default() });
    let plan = plan_devices(&prj, &col_format);
    assert!(plan.contains(&("-K8".into(), "-K12.2".into())), "{plan:?}");

    prj.pages[1].locked = true;
    assert!(!plan_devices(&prj, &DeviceNumbering::default()).iter().any(|(old, _)| old == "-K1"), "devices on released pages keep their tag");
}

#[test]
fn pages_are_renumbered_with_subpages_and_structures() {
    let mut prj = Project::default();
    for name in ["7", "7.a", "9", "10"] {
        let id = prj.new_id();
        prj.pages.push(Page::new(id, name, "", PageKind::Schematic));
    }
    prj.pages[4].function = "A2".into();
    assert_eq!(renumber_pages(&mut prj, &PageNumbering::default()), 4);
    let names: Vec<&str> = prj.pages.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["1", "2", "2.a", "3", "4"]);

    let opts = PageNumbering { start: 10, step: 10, per_structure: true, subpages: false, ..Default::default() };
    renumber_pages(&mut prj, &opts);
    let names: Vec<&str> = prj.pages.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["10", "20", "30", "40", "10"]);

    prj.pages[0].locked = true;
    let plan = plan_pages(&prj, &PageNumbering { pages: vec![0, 1], ..Default::default() });
    assert_eq!(plan, [(1, "2".to_string())], "released pages keep their name");
}

#[test]
fn pages_are_numbered_per_section_after_the_section_number() {
    let mut prj = Project::default();
    let a = sections::add(&mut prj, 0, "");
    let b = sections::add(&mut prj, a, "");
    sections::set_number(&mut prj, a, "5").unwrap();
    for (name, section) in [("5.4", a), ("5.4.a", a), ("x", a), ("5.1.7", b), ("3", 0)] {
        let id = prj.new_id();
        let mut pg = Page::new(id, name, "", PageKind::Schematic);
        pg.section = section;
        prj.pages.push(pg);
    }
    sections::sort_pages(&mut prj);
    renumber_pages(&mut prj, &PageNumbering::default());
    let names: Vec<&str> = prj.pages.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["1", "2", "5.1", "5.1.a", "5.2", "5.1.1"]);

    let opts = PageNumbering { sections: true, ..Default::default() };
    assert_eq!(plan_pages(&prj, &opts).len(), 4);
    assert_eq!(renumber_pages(&mut prj, &opts), 4);
    let names: Vec<&str> = prj.pages.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["1", "2", "1.1", "1.1.a", "1.2", "1.1.1"]);
}
