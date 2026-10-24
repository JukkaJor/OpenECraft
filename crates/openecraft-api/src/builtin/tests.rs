use super::*;
use crate::{Origin, Plugins};
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
    let tags = (p.exporter("device_tags").unwrap().run)(&prj).unwrap();
    assert_eq!(tags.lines().count(), structure::device_tags(&prj).len());
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
