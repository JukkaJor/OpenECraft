use super::*;
use crate::sample::demo_project;
use serde_json::{json, Map, Value};

fn args(v: Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}

#[test]
fn every_built_in_export_runs_on_the_demo_project() {
    let prj = demo_project();
    let an = Analysis::build(&prj);
    let mut names = std::collections::HashSet::new();
    for e in builtin() {
        assert!(names.insert(e.name.clone()), "duplicate export {}", e.name);
        assert!(!e.extension.is_empty() && !e.title.is_empty());
        let mut a = if e.params.iter().any(|p| p.name == "page") { args(json!({ "page": "1" })) } else { Map::new() };
        if e.params.iter().any(|p| p.name == "harness") {
            a.insert("harness".into(), json!("-WH1"));
        }
        crate::params::check(&e.params, &a).unwrap();
        let out = (e.run)(&prj, &an, &Args(&a)).unwrap_or_else(|err| panic!("{}: {err}", e.name));
        match out {
            Export::Binary(b) => assert!(e.binary && !b.is_empty(), "{}", e.name),
            Export::Text(t) => assert!(!e.binary && !t.is_empty(), "{}", e.name),
        }
    }
    assert!(names.contains("svg") && names.contains("jt") && names.len() >= 17);
}

#[test]
fn exports_check_their_parameters() {
    let prj = demo_project();
    let an = Analysis::build(&prj);
    let svg = builtin().into_iter().find(|e| e.name == "svg").unwrap();
    assert!((svg.run)(&prj, &an, &Args(&Map::new())).is_err(), "svg needs a page");
    assert!((svg.run)(&prj, &an, &Args(&args(json!({ "page": "99" })))).unwrap_err().contains("99"));
    let step = builtin().into_iter().find(|e| e.name == "step").unwrap();
    let out = (step.run)(&prj, &an, &Args(&args(json!({ "file": "cabinet.stp" })))).unwrap().text().unwrap();
    assert!(out.contains("FILE_NAME('cabinet.stp'"));
    let mut empty = Project::default();
    empty.layout.enclosures.clear();
    let gltf = builtin().into_iter().find(|e| e.name == "gltf").unwrap();
    assert!((gltf.run)(&empty, &Analysis::build(&empty), &Args(&Map::new())).is_err(), "no layout");
}

#[test]
fn exports_say_what_they_lack_and_name_their_language() {
    let mut prj = crate::sample::example("lamp_circuit").unwrap();
    prj.harnesses.clear();
    let an = Analysis::build(&prj);
    let all = builtin();
    let run = |name: &str| (all.iter().find(|e| e.name == name).unwrap().run)(&prj, &an, &Args(&serde_json::Map::new()));
    for name in ["kbl", "kbl_zip", "harness_json", "harness_processing", "harness_gltf", "harness_jt"] {
        assert_eq!(run(name).err().as_deref(), Some("The project has no harnesses"), "{name}");
    }
    let html = run("html").unwrap().text().unwrap();
    assert!(html.starts_with("<!DOCTYPE html><html lang=\"en\">"), "{}", &html[..60]);
    // The wire list opens in spreadsheets with its units intact.
    assert!(run("wire_list").unwrap().text().unwrap().starts_with('\u{feff}'));
}

#[test]
fn the_pinouts_of_one_or_all_plugs_are_exported() {
    let prj = crate::sample::example("connector_handling").unwrap();
    let an = Analysis::build(&prj);
    let all = builtin();
    let e = all.iter().find(|e| e.name == "pinouts").unwrap();
    let run = |args: serde_json::Value| (e.run)(&prj, &an, &Args(args.as_object().unwrap()));
    let every = run(serde_json::json!({})).unwrap().text().unwrap();
    assert!(every.contains("-X11") && every.contains("-X21"));
    let one = run(serde_json::json!({ "plug": "-X15" })).unwrap().text().unwrap();
    assert!(one.contains("dsub 9") && !one.contains("-X11"));
    assert_eq!(run(serde_json::json!({ "plug": "-X99" })).err().as_deref(), Some("No plug -X99"));
}
