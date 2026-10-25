use super::*;
use crate::{Args, Origin, Plugins};
use openecraft_core::sample::demo_project;
use serde_json::Map;

#[test]
fn statistics_counts_the_demo_project() {
    let p = Plugins::new(Vec::<String>::new());
    let mut prj = demo_project();
    let out = p.run(p.command("project_statistics").unwrap(), &mut prj, &Map::new()).unwrap();
    assert_eq!(out["pages"], prj.pages.len());
    assert_eq!(out["errors"], 0);
    let table = (p.report("project_statistics").unwrap().run)(&prj, &Analysis::checked(&prj)).unwrap();
    assert_eq!(table.rows.len(), 8);
    let tags = (p.exporter("device_tags").unwrap().run)(&prj, &Analysis::build(&prj), &Args(&Map::new())).unwrap().text().unwrap();
    assert_eq!(tags.lines().count(), openecraft_core::structure::device_tags(&prj).len());
    assert!(p.exporter("pdf").is_some() && p.exporter("dwg").is_some_and(|e| e.binary), "built-in exports are offered");
    assert!(p.list().iter().any(|l| l.info.id == "statistics" && l.origin == Origin::BuiltIn));
}

#[test]
fn project_checks_are_a_built_in_plugin() {
    let p = Plugins::new(Vec::<String>::new());
    let prj = demo_project();
    let an = Analysis::build(&prj);
    assert!(an.issues.is_empty(), "build leaves the issues to the checks");
    let names: Vec<&str> = p.checks().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"devices") && names.contains(&"panel_layout"), "{names:?}");
    let issues = p.check(&prj, &an);
    assert!(issues.len() >= Analysis::checked(&prj).issues.len());
    assert!(p.list().iter().any(|l| l.info.id == "project_checks"));
}

#[test]
fn macro_boxes_are_listed() {
    let p = Plugins::new(Vec::<String>::new());
    let mut prj = demo_project();
    assert_eq!(p.run(p.command("list_macro_boxes").unwrap(), &mut prj, &Map::new()).unwrap(), serde_json::json!([]));
    let sel: std::collections::BTreeSet<u64> = [prj.pages[1].symbols[0].id].into();
    let m = openecraft_core::macros::from_selection(&prj.pages[1], &sel, "One");
    let values = [("NAME".to_string(), "X".to_string())].into();
    openecraft_core::macro_boxes::place(&mut prj, 1, &m, &values, openecraft_core::geom::p(200.0, 150.0));
    let list = p.run(p.command("list_macro_boxes").unwrap(), &mut prj, &Map::new()).unwrap();
    assert_eq!(list[0]["macro"], "One");
    assert_eq!(list[0]["items"], 1);
    assert_eq!(list[0]["values"]["NAME"], "X");
}
