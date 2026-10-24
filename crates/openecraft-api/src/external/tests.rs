use super::*;
use crate::{Args, Origin, Plugins};
use serde_json::Map;
use openecraft_core::analysis::Analysis;
use openecraft_core::sample::demo_project;
use std::sync::Mutex;

/// Plugin answering like an external program; records the requests.
fn fake(log: Arc<Mutex<Vec<Value>>>) -> External {
    let info = Info { id: "fake".into(), name: Text::Owned("Fake".into()), version: "1".into(), description: Text::Owned(String::new()) };
    External::new(
        info,
        Arc::new(move |req: &Value| {
            log.lock().unwrap().push(req.clone());
            Ok(match (req["method"].as_str().unwrap(), req["name"].as_str()) {
                ("describe", _) => json!({
                    "commands": [{ "name": "set_title", "help": "Sets the title", "params": [{ "name": "title", "kind": "str" }] }, { "name": "fail" }],
                    "checks": [{ "name": "titled", "title": "Title set" }],
                    "reports": [{ "name": "titles", "title": "Titles" }],
                    "exporters": [{ "name": "title_txt", "title": "Title", "extension": "txt" }],
                }),
                ("command", Some("fail")) => json!({ "error": "failed on purpose" }),
                ("command", _) => {
                    let mut prj = req["project"].clone();
                    prj["info"]["name"] = req["params"]["title"].clone();
                    json!({ "result": "ok", "project": prj })
                }
                ("check", _) => json!({ "issues": [{ "severity": "error", "message": "no title", "page": 0, "item": 7 }, { "message": "hint" }] }),
                ("report", _) => json!({ "headers": ["Title"], "rows": [[req["project"]["info"]["name"].clone()]] }),
                ("export", _) => json!({ "content": "title" }),
                _ => json!({}),
            })
        }),
    )
}

#[test]
fn external_plugins_answer_json_requests() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut p = Plugins::default();
    p.add(&fake(log.clone()), Origin::External("fake".into())).unwrap();
    assert_eq!(log.lock().unwrap()[0], json!({ "method": "describe", "api": API_VERSION }));

    let mut prj = demo_project();
    let args = json!({ "title": "Plant 7" }).as_object().unwrap().clone();
    assert_eq!(p.run(p.command("set_title").unwrap(), &mut prj, &args).unwrap(), "ok");
    assert_eq!(prj.info.name, "Plant 7");
    assert_eq!(p.run(p.command("fail").unwrap(), &mut prj, &Default::default()).unwrap_err(), "failed on purpose");

    let issues = p.check(&prj, &Analysis::default());
    assert_eq!(issues[0].severity, Severity::Error);
    assert_eq!(issues[0].target, Some((0, 7)));
    assert_eq!((issues[1].severity, issues[1].target), (Severity::Warning, None));

    let table = (p.report("titles").unwrap().run)(&prj, &Analysis::default()).unwrap();
    assert_eq!(table.rows, [["Plant 7"]]);
    assert_eq!((p.exporter("title_txt").unwrap().run)(&prj, &Analysis::default(), &Args(&Map::new())).unwrap().text().unwrap(), "title");
}

#[test]
fn manifests_are_read_from_the_plugin_folder() {
    let dir = std::env::temp_dir().join(format!("openecraft-api-manifest-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(MANIFEST), r#"{ "id": "x", "name": "X", "api": 99, "run": ["x"] }"#).unwrap();
    assert!(External::open(&dir).err().unwrap().contains("API 99"));
    std::fs::write(dir.join(MANIFEST), r#"{ "id": "x", "name": "X", "run": [] }"#).unwrap();
    assert!(External::open(&dir).is_err());
    std::fs::write(dir.join(MANIFEST), r#"{ "id": "x", "name": "X", "run": ["openecraft-no-such-program"] }"#).unwrap();
    let plugin = External::open(&dir).unwrap();
    assert_eq!(plugin.info().id, "x");
    assert!(Plugins::default().add(&plugin, Origin::External(dir.clone())).unwrap_err().contains("openecraft-no-such-program"));
    std::fs::remove_dir_all(&dir).unwrap();
}
