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
    let headers: Vec<Text> = ["Tag", "Name", "Part"].map(Text::Borrowed).to_vec();
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
    prj.pages.push(crate::model::Page::new(id, "99", "Devices", PageKind::Report(ReportKind::DeviceList.into())));
    let page = prj.pages.len() - 1;
    let form = ReportForm { title: "Our device list".into(), columns: vec!["Part number".into(), "Device tag".into()], rows: 10, ..Default::default() };
    prj.forms.reports.insert(ReportKind::DeviceList.into(), form);
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
    let mut lib = FormLibrary { title_blocks: vec![own.clone()], ..Default::default() };
    assert_eq!(merge(&mut prj, FormLibrary::from_json(&serde_json::to_string(&lib).unwrap()).unwrap()), (1, 0));
    own.items.clear();
    lib.title_blocks = vec![own];
    merge(&mut prj, lib);
    assert_eq!(prj.forms.title_blocks.len(), before + 1);
    assert!(prj.forms.title_blocks.iter().find(|f| f.name == "Company").unwrap().items.is_empty());
    assert!(FormLibrary::from_json("42").is_err());
}

#[test]
fn merging_report_schemes_replaces_by_report_and_name() {
    let mut prj = Project::default();
    let scheme = |kind: ReportKind, title: &str| ReportScheme { name: "Workshop".into(), kind: kind.into(), form: ReportForm { title: title.into(), ..Default::default() } };
    let lib = FormLibrary { schemes: vec![scheme(ReportKind::DeviceList, "a"), scheme(ReportKind::TerminalDiagram, "b")], ..Default::default() };
    let json = serde_json::to_string(&lib).unwrap();
    assert_eq!(merge(&mut prj, FormLibrary::from_json(&json).unwrap()), (0, 2));
    merge(&mut prj, FormLibrary { schemes: vec![scheme(ReportKind::DeviceList, "c")], ..Default::default() });
    assert_eq!(prj.forms.schemes.len(), 2);
    assert_eq!(report_form(&prj, ReportKind::DeviceList, "Workshop").unwrap().title, "c");
    // Old title block files without schemes still load.
    assert!(FormLibrary::from_json(r#"{"title_blocks":[]}"#).unwrap().schemes.is_empty());
}

#[test]
fn property_database_values_fill_placeholders() {
    use crate::properties::{set, Object};
    let mut prj = demo_project();
    set(&mut prj, &Object::Project, "project.customer", "ACME").unwrap();
    set(&mut prj, &Object::Project, "project.mains_voltage", "400").unwrap();
    let page = Object::Page(prj.pages[1].id);
    set(&mut prj, &page, "page.drawing_number", "D-17").unwrap();
    assert_eq!(fill("{prop:project.customer} {prop:project.mains_voltage}", &prj, 1), "ACME 400 V");
    assert_eq!(fill("{prop:page.drawing_number}", &prj, 1), "D-17");
    assert_eq!(fill("{prop:page.drawing_number}", &prj, 0), "");
    assert_eq!(fill("{prop:project.pages}", &prj, 0), prj.pages.len().to_string());
}

#[test]
fn row_placeholders_take_the_cells_by_key_or_title() {
    let keys = vec!["Tag".to_string(), "Part".to_string()];
    let headers = vec!["Kennz.".to_string(), "Artikel".to_string()];
    let row = vec!["-K1".to_string(), "{odd}".to_string()];
    assert_eq!(fill_row("{Tag}: {Artikel}", &keys, &headers, &row), "-K1: {{odd}}");
    assert_eq!(fill_row("{project} {Missing} {{x}}", &keys, &headers, &row), "{project} {Missing} {{x}}");
    assert_eq!(fill_row("open {", &keys, &headers, &row), "open {");
}

#[test]
fn a_drawn_form_replaces_the_table_of_a_report_page() {
    let mut prj = demo_project();
    let page = prj.pages.iter().position(|p| p.kind == PageKind::Report(ReportKind::DeviceList.into())).unwrap();
    let an = crate::analysis::Analysis::with_builtin(&prj);
    let table = an.report(&prj, ReportKind::DeviceList).unwrap();
    let keys = table.header_keys();
    let mut layout = starting_layout(&keys, &table.header_texts(), &table.widths, 380.0);
    // A mark only the drawn form has, with a column and a project placeholder.
    layout.row.push(FormItem::Text { pos: p(300.0, 3.0), text: format!("#{{{}}}@{{project}}", keys[0]), size: 2.0, anchor: Anchor::Left, label: false, max_width: 0.0 });
    prj.forms.reports.insert(ReportKind::DeviceList.into(), ReportForm { layout: Some(layout.clone()), ..Default::default() });
    let t = crate::report_blocks::page_table(&prj, &an, page).unwrap();
    assert_eq!(t.layout.as_ref(), Some(&layout));
    assert_eq!(t.keys.len(), t.headers.len());
    let first = &t.rows[0][0];
    let svg = crate::svg::page_svg_with(&prj, &an, page);
    assert!(svg.contains(&format!("#{first}@Demo conveyor")), "row placeholders filled");
    assert!(svg.contains(&tr("Device list").to_string()), "{{report}} is the page heading");
    let (a, b) = items_bounds(&layout.row).unwrap();
    assert!(a.x <= 0.0 && b.x >= 300.0);
    // Without the layout the page is the table again.
    prj.forms.reports.clear();
    assert!(!crate::svg::page_svg_with(&prj, &an, page).contains(&format!("#{first}@")));
}

#[test]
fn the_starting_layout_has_a_placeholder_per_column() {
    let layout = starting_layout(&["Tag", "Part"], &["Kennz."], &[1.0, 3.0], 200.0);
    let texts: Vec<&str> = layout.row.iter().filter_map(|i| match i { FormItem::Text { text, .. } => Some(text.as_str()), _ => None }).collect();
    assert_eq!(texts, ["{Tag}", "{Part}"]);
    let labels: Vec<&str> = layout.header.iter().filter_map(|i| match i { FormItem::Text { text, label: true, .. } => Some(text.as_str()), _ => None }).collect();
    assert_eq!(labels, ["Kennz.", "Part"], "titles in the current language, keys where none is given");
    assert_eq!(layout.step, p(0.0, 6.0));
    let json = serde_json::to_string(&ReportForm { layout: Some(layout.clone()), ..Default::default() }).unwrap();
    assert_eq!(serde_json::from_str::<ReportForm>(&json).unwrap().layout, Some(layout));
    assert!(!serde_json::to_string(&ReportForm::default()).unwrap().contains("layout"));
}
