use super::*;
use crate::sample::demo_project;

#[test]
fn placeholders_are_filled() {
    let mut prj = demo_project();
    prj.info.properties.insert("Customer".into(), "ACME".into());
    let n = prj.pages.len();
    assert_eq!(fill("{project} – {sheet}/{sheets}", &prj, 1), format!("Demo conveyor – 2/{n}"));
    assert_eq!(fill("Customer: {prop:Customer}", &prj, 0), "Customer: ACME");
    assert_eq!(fill("{prop:Missing}", &prj, 0), "");
    // Unknown placeholders and escaped braces stay readable.
    assert_eq!(fill("{nothing} {{x}} {", &prj, 0), "{nothing} {x} {");
    assert_eq!(fill("{page.type}", &prj, 0), "Cover");
    assert_eq!(fill("ä {company} ö", &prj, 0), "ä OpenECraft ö");
}

#[test]
fn every_offered_placeholder_is_known() {
    let prj = demo_project();
    for (ph, _) in PLACEHOLDERS {
        let name = ph.trim_start_matches('{').trim_end_matches('}');
        if name.starts_with('{') || name.trim().is_empty() {
            continue;
        }
        let name = if name.starts_with("prop:") { "prop:Name" } else { name };
        assert!(placeholder(&prj, 0, name).is_some(), "{ph}");
    }
}

#[test]
fn title_block_choice() {
    let mut prj = demo_project();
    assert_eq!(title_block(&prj, 1).name, STANDARD);
    let mut own = find(&prj, COMPACT).into_owned();
    own.name = "Company".into();
    prj.forms.title_blocks.push(own);
    prj.forms.default_title_block = "Company".into();
    assert_eq!(title_block(&prj, 1).name, "Company");
    prj.pages[1].title_block = NONE.into();
    assert_eq!(title_block(&prj, 1).name, NONE);
    assert!(title_block(&prj, 1).items.is_empty());
    // Unknown names fall back to the standard title block.
    prj.pages[1].title_block = "Deleted".into();
    assert_eq!(title_block(&prj, 1).name, STANDARD);
    assert_eq!(names(&prj), vec!["Company", STANDARD, COMPACT, NONE]);
}

#[test]
fn title_blocks_are_drawn_with_page_values() {
    let mut prj = demo_project();
    let svg = crate::svg::page_svg(&prj, 1);
    assert!(svg.contains("Demo conveyor"));
    assert!(svg.contains(&format!("2 / {}", prj.pages.len())));
    // A custom title block with a project property.
    prj.info.properties.insert("Order".into(), "4711-XY".into());
    let item = FormItem::Text { pos: p(10.0, 10.0), text: "Order {prop:Order}".into(), size: 3.0, anchor: Anchor::Left, label: false, max_width: 0.0 };
    prj.forms.title_blocks.push(Form { name: "Mine".into(), items: vec![item] });
    prj.pages[1].title_block = "Mine".into();
    let svg = crate::svg::page_svg(&prj, 1);
    assert!(svg.contains("Order 4711-XY"));
    assert!(!svg.contains("Demo conveyor"), "standard title block is not drawn");
}

#[test]
fn long_values_shrink_to_their_width() {
    struct Sizes(Vec<f32>);
    impl Draw for Sizes {
        fn line(&mut self, _: P, _: P, _: f32, _: Ink, _: bool) {}
        fn poly(&mut self, _: &[P], _: bool, _: f32, _: Ink, _: bool) {}
        fn circle(&mut self, _: P, _: f32, _: f32, _: Ink, _: bool) {}
        fn text(&mut self, _: P, _: &str, size: f32, _: Anchor, _: VAnchor, _: Ink) {
            self.0.push(size);
        }
    }
    let mut prj = demo_project();
    prj.info.name = "A very long project name".into();
    let item = FormItem::Text { pos: p(0.0, 0.0), text: "{project}".into(), size: 4.0, anchor: Anchor::Left, label: false, max_width: 20.0 };
    let mut d = Sizes(Vec::new());
    draw(&mut d, &Form { name: "x".into(), items: vec![item] }, &prj, 0);
    assert!(d.0[0] < 4.0);
    assert!(text_width(&prj.info.name, d.0[0]) <= 20.01);
}

#[test]
fn report_columns_follow_the_form() {
    let headers = ["Tag", "Name", "Part"];
    assert_eq!(report_columns(None, &headers), vec![0, 1, 2]);
    let f = ReportForm { columns: vec!["Part".into(), "Unknown".into(), "Tag".into()], ..Default::default() };
    assert_eq!(report_columns(Some(&f), &headers), vec![2, 0]);
    let f = ReportForm { columns: vec!["Unknown".into()], ..Default::default() };
    assert_eq!(report_columns(Some(&f), &headers), vec![0, 1, 2]);
}

#[test]
fn report_pages_use_their_form() {
    let mut prj = demo_project();
    let id = prj.new_id();
    prj.pages.push(crate::model::Page::new(id, "99", "Devices", PageKind::Report(ReportKind::DeviceList)));
    let page = prj.pages.len() - 1;
    let form = ReportForm { title: "Our device list".into(), columns: vec!["Part number".into(), "Device tag".into()], rows: 10 };
    prj.forms.reports.insert(ReportKind::DeviceList, form);
    let svg = crate::svg::page_svg(&prj, page);
    assert!(svg.contains("Our device list"));
    let (part, tag) = (svg.find(">Part number<").unwrap(), svg.find(">Device tag<").unwrap());
    assert!(part < tag, "columns in the order of the form");
    assert!(!svg.contains(">Function text<"));
    // The forms are saved with the project (report kinds as keys).
    let json = serde_json::to_string(&prj).unwrap();
    let back: Project = serde_json::from_str(&json).unwrap();
    assert_eq!(back.forms, prj.forms);
}

#[test]
fn merging_title_blocks_replaces_by_name() {
    let mut prj = Project::default();
    let before = prj.forms.title_blocks.len();
    let mut own = builtin().remove(0);
    own.name = "Company".into();
    let mut lib = FormLibrary { title_blocks: vec![own.clone()] };
    assert_eq!(merge(&mut prj, FormLibrary::from_json(&serde_json::to_string(&lib).unwrap()).unwrap()), 1);
    own.items.clear();
    lib.title_blocks = vec![own];
    merge(&mut prj, lib);
    assert_eq!(prj.forms.title_blocks.len(), before + 1);
    assert!(prj.forms.title_blocks.iter().find(|f| f.name == "Company").unwrap().items.is_empty());
    assert!(FormLibrary::from_json("42").is_err());
}
