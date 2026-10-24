use super::*;
use crate::testing::*;

fn demo_session() -> Session {
    let mut s = session();
    run(&mut s, "new_project", json!({ "demo": true })).unwrap();
    s
}

fn save_project(dir: &Path, file: &str, name: &str, company: &str) -> String {
    let mut prj = Project::new_default();
    prj.info.name = name.into();
    prj.info.company = company.into();
    let path = dir.join(file);
    std::fs::write(&path, prj.to_json()).unwrap();
    path.display().to_string()
}

#[test]
fn projects_are_indexed_searched_and_changed_in_batch() {
    let dir = temp_dir("mgmt-index");
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    let a = save_project(&dir, "a.oec", "Line A", "ACME");
    save_project(&dir.join("sub"), "b.oec", "Line B", "Other");
    std::fs::write(dir.join("notes.txt"), "x").unwrap();
    let mut s = session();
    let db = dir.join("db.json");
    let out = run(&mut s, "index_projects", json!({ "folder": dir.display().to_string(), "database": db.display().to_string(), "verify": true })).unwrap();
    assert_eq!(out["total"], 2);
    assert_eq!(out["projects"][0]["verified"], json!([0, 0]));

    let found = run(&mut s, "search_projects", json!({ "database": db.display().to_string(), "query": "company=acme" })).unwrap();
    assert_eq!(found.as_array().unwrap().len(), 1);
    assert_eq!(found[0]["path"], a);
    let found = run(&mut s, "search_projects", json!({ "folder": dir.display().to_string(), "query": "line" })).unwrap();
    assert_eq!(found.as_array().unwrap().len(), 2);
    assert!(run(&mut s, "search_projects", json!({})).is_err());

    let dry = run(&mut s, "batch_project_properties", json!({ "folder": dir.display().to_string(), "set": { "creator": "NS", "plant": "North" }, "dry_run": true })).unwrap();
    assert_eq!(dry.as_array().unwrap().len(), 2);
    assert_eq!(dry[0]["written"], false);
    let out = run(&mut s, "batch_project_properties", json!({ "folder": dir.display().to_string(), "query": "company=ACME", "set": { "creator": "NS" }, "replace": "name", "find": "Line", "with": "Conveyor" })).unwrap();
    assert_eq!(out.as_array().unwrap().len(), 1);
    assert_eq!(out[0]["written"], true);
    assert_eq!(out[0]["changes"].as_array().unwrap().len(), 2);
    let changed = Project::from_json(&std::fs::read_to_string(&a).unwrap()).unwrap();
    assert_eq!((changed.info.name.as_str(), changed.info.creator.as_str()), ("Conveyor A", "NS"));
    assert!(run(&mut s, "batch_project_properties", json!({ "folder": dir.display().to_string() })).is_err());
    assert!(run(&mut s, "batch_project_properties", json!({ "set": { "a": "b" } })).is_err());
}

#[test]
fn projects_are_archived_converted_and_verified() {
    let dir = temp_dir("mgmt-archive");
    let mut s = demo_session();
    let v = run(&mut s, "check_project_data", json!({})).unwrap();
    assert_eq!(v["passed"], true);
    s.project_mut().unwrap().devices.insert("-Q99".into(), Default::default());
    let v = run(&mut s, "check_project_data", json!({ "repair": true })).unwrap();
    assert_eq!(v["repaired"].as_array().unwrap().len(), 1);

    let path = dir.join("demo.oecz").display().to_string();
    let out = run(&mut s, "archive_project", json!({ "path": path, "comment": "Handover", "compact": true })).unwrap();
    assert_eq!(out["files"][0], "project.oec");
    let info = run(&mut s, "archive_info", json!({ "path": path })).unwrap();
    assert_eq!((info["name"].as_str(), info["comment"].as_str()), (Some("Demo conveyor"), Some("Handover")));
    let opened = run(&mut s, "open_project", json!({ "path": path, "discard": true })).unwrap();
    assert_eq!(opened["file"], Value::Null);
    assert!(opened.get("conversion").is_none());

    let old = dir.join("old.oec");
    std::fs::write(&old, r#"{ "info": { "name": "Old" }, "pages": [], "next_id": 1 }"#).unwrap();
    let c = run(&mut s, "convert_project", json!({ "path": old.display().to_string() })).unwrap();
    assert_eq!((c["from"].as_u64(), c["to"].as_u64()), (Some(0), Some(1)));
    assert!(std::fs::read_to_string(&old).unwrap().contains("\"format\": 1"));
    let c = run(&mut s, "convert_project", json!({ "path": path })).unwrap();
    assert!(c["output"].as_str().unwrap().ends_with("demo.oec"));
}

#[test]
fn subprojects_are_extracted_and_inserted() {
    let dir = temp_dir("mgmt-sub");
    let mut s = demo_session();
    let path = dir.join("power.oec").display().to_string();
    let out = run(&mut s, "extract_subproject", json!({ "name": "Power", "path": path, "pages": "2" })).unwrap();
    assert_eq!(out["pages"], json!(["2"]));
    let list = run(&mut s, "list_subprojects", json!({})).unwrap();
    assert_eq!(list["subprojects"][0]["out"], true);
    assert!(s.project().unwrap().pages[1].locked);

    let mut sub = session();
    run(&mut sub, "open_project", json!({ "path": path })).unwrap();
    assert_eq!(run(&mut sub, "list_subprojects", json!({})).unwrap()["parent"]["name"], "Power");
    run(&mut sub, "set_project_info", json!({ "description": "edited" })).unwrap();
    run(&mut sub, "save_project", json!({})).unwrap();

    let r = run(&mut s, "insert_subproject", json!({ "path": path })).unwrap();
    assert_eq!((r["pages_removed"].as_u64(), r["pages_inserted"].as_u64()), (Some(1), Some(1)));
    assert!(!s.project().unwrap().pages[1].locked);
    run(&mut s, "extract_subproject", json!({ "name": "Ctrl", "path": dir.join("c.oec").display().to_string(), "pages": "3" })).unwrap();
    let list = run(&mut s, "discard_subproject", json!({ "name": "Ctrl" })).unwrap();
    assert_eq!(list["subprojects"].as_array().unwrap().len(), 1);
    assert!(run(&mut s, "extract_subproject", json!({ "name": "X", "path": dir.join("x.oec").display().to_string() })).is_err());
}

#[test]
fn model_options_hide_and_show_content() {
    let mut s = demo_session();
    run(&mut s, "define_model_option", json!({ "name": "PLC", "description": "With PLC" })).unwrap();
    let pages = s.project().unwrap().pages.len();
    let out = run(&mut s, "assign_model_option", json!({ "name": "PLC", "pages": "4" })).unwrap();
    assert_eq!(out["options"][0]["pages"], 1);
    let out = run(&mut s, "set_model_option", json!({ "name": "PLC", "active": false })).unwrap();
    assert_eq!(out["moved"], 1);
    assert_eq!(s.project().unwrap().pages.len(), pages - 1);
    run(&mut s, "undo", json!({})).unwrap();
    assert_eq!(s.project().unwrap().pages.len(), pages);
    run(&mut s, "set_model_option", json!({ "name": "PLC", "active": false })).unwrap();
    let out = run(&mut s, "unassign_model_option", json!({ "pages": "4" })).unwrap();
    assert_eq!(out["unassigned"], 1);
    assert_eq!(s.project().unwrap().pages.len(), pages);

    run(&mut s, "define_model_option", json!({ "name": "PLC", "condition": "" })).unwrap();
    assert_eq!(run(&mut s, "list_model_options", json!({})).unwrap()[0]["description"], "With PLC");
    assert!(run(&mut s, "define_model_option", json!({ "name": "PLC", "condition": "NOPE=1" })).is_err());
    assert_eq!(run(&mut s, "apply_model_options", json!({})).unwrap()["switched"], json!([]));
    assert!(run(&mut s, "assign_model_option", json!({ "name": "PLC" })).is_err());
    run(&mut s, "remove_model_option", json!({ "name": "PLC" })).unwrap();
    assert_eq!(run(&mut s, "list_model_options", json!({})).unwrap(), json!([]));
}
