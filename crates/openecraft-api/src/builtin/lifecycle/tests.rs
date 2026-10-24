use super::*;
use crate::{Origin, Plugins};
use openecraft_core::parts::Part;
use openecraft_core::analysis::Analysis;
use openecraft_core::sample::demo_project;
use serde_json::Map;

fn run(p: &Plugins, prj: &mut Project, name: &str, args: Value) -> Result<Value, String> {
    let args: Map<String, Value> = args.as_object().cloned().unwrap_or_default();
    p.run(p.command(name).ok_or("no command")?, prj, &args)
}

/// The demo project with its first used part discontinued; returns its number.
fn discontinued(prj: &mut Project, successor: &str) -> (String, String) {
    let (tag, number) = prj.devices.iter().find_map(|(t, d)| d.parts.first().map(|r| (t.clone(), r.number.clone()))).unwrap();
    let p = prj.parts.iter_mut().find(|p| p.number == number).unwrap();
    p.lifecycle = Lifecycle::Discontinued;
    p.successor = successor.into();
    (tag, number)
}

#[test]
fn active_parts_give_no_findings() {
    let p = Plugins::new(Vec::<String>::new());
    assert!(p.list().iter().any(|l| l.info.id == "part_lifecycle" && l.origin == Origin::BuiltIn));
    let prj = demo_project();
    assert!(issues(&prj).is_empty());
    assert!((p.report("part_lifecycle").unwrap().run)(&prj, &Analysis::build(&prj)).unwrap().rows.is_empty());
}

#[test]
fn discontinued_parts_are_reported_per_device() {
    let p = Plugins::new(Vec::<String>::new());
    let mut prj = demo_project();
    let (tag, number) = discontinued(&mut prj, "NEW.1");
    let found: Vec<Issue> = p.check(&prj, &Analysis::build(&prj)).into_iter().filter(|i| i.message().contains(&number)).collect();
    assert!(!found.is_empty());
    assert!(found.iter().all(|i| i.severity == Severity::Warning));
    assert!(found.iter().any(|i| i.message().starts_with(&tag) && i.message().ends_with("NEW.1")));
    let table = (p.report("part_lifecycle").unwrap().run)(&prj, &Analysis::build(&prj)).unwrap();
    assert_eq!(table.rows[0][0], number);
    assert!(table.rows[0][3].contains(&tag));
    // Phase-out is a note only.
    prj.parts.iter_mut().find(|p| p.number == number).unwrap().lifecycle = Lifecycle::PhaseOut;
    assert!(issues(&prj).iter().all(|i| i.severity == Severity::Info));
}

#[test]
fn devices_change_over_to_successors() {
    let p = Plugins::new(Vec::<String>::new());
    let mut prj = demo_project();
    let (tag, number) = discontinued(&mut prj, "NEW.1");
    // The successor is not a project part yet.
    let out = run(&p, &mut prj, "use_successor_parts", json!({})).unwrap();
    assert_eq!(out["missing_successors"], json!(["NEW.1"]));
    assert!(out["changed"].as_array().unwrap().is_empty());
    prj.parts.push(Part { number: "NEW.1".into(), ..Default::default() });
    assert!(run(&p, &mut prj, "use_successor_parts", json!({ "number": "OTHER" })).unwrap()["changed"].as_array().unwrap().is_empty());
    let out = run(&p, &mut prj, "use_successor_parts", json!({ "number": number })).unwrap();
    assert!(out["changed"].as_array().unwrap().iter().any(|c| c["device"] == tag.as_str()));
    assert!(prj.devices[&tag].parts.iter().any(|r| r.number == "NEW.1"));
    assert!(issues(&prj).iter().all(|i| !i.message().contains(&number)));
}
