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
    let table = (p.report("project_statistics").unwrap().run)(&prj, &Analysis::build(&prj)).unwrap();
    assert_eq!(table.rows.len(), 8);
    let tags = (p.exporter("device_tags").unwrap().run)(&prj).unwrap();
    assert_eq!(tags.lines().count(), structure::device_tags(&prj).len());
    assert!(p.list().iter().any(|l| l.info.id == "statistics" && l.origin == Origin::BuiltIn));
}
