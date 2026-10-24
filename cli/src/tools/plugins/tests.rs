use crate::testing::*;
use crate::tools::{lookup, Entry};
use serde_json::json;

#[test]
fn plugins_are_listed_and_can_be_disabled() {
    let mut s = new_session();
    let out = run(&mut s, "plugins", json!({})).unwrap();
    let stats = out["plugins"].as_array().unwrap().iter().find(|p| p["id"] == "statistics").unwrap();
    assert_eq!(stats["enabled"], true);
    assert_eq!(stats["commands"], json!(["project_statistics"]));
    assert!(matches!(lookup(&s, "project_statistics"), Some(Entry::Plugin(_))));

    run(&mut s, "plugins", json!({ "disable": "statistics" })).unwrap();
    assert!(lookup(&s, "project_statistics").is_none());
    assert!(run(&mut s, "plugins", json!({ "enable": "nothing" })).is_err());
}

#[test]
fn plugin_commands_reports_and_exports_are_tools() {
    let mut s = new_session();
    run(&mut s, "new_project", json!({ "demo": true, "discard": true })).unwrap();
    let cmd = lookup(&s, "project_statistics").unwrap();
    let stats = crate::tools::call(&mut s, cmd, &Default::default()).unwrap();
    assert!(stats["pages"].as_u64().unwrap() > 0);
    let report = run(&mut s, "report", json!({ "kind": "project_statistics" })).unwrap();
    assert_eq!(report["headers"], json!(["Figure", "Count"]));
    let tags = run(&mut s, "export", json!({ "format": "device_tags" })).unwrap();
    assert!(tags.as_str().unwrap().lines().count() > 1);
}
