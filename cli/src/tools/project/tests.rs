use super::*;
use crate::testing::*;

#[test]
fn new_project_saves_and_refuses_to_drop_changes() {
    let dir = temp_dir("project-new");
    let file = dir.join("p.oec");
    let mut s = session();
    let info = run(&mut s, "new_project", json!({ "name": "Plant", "path": file.to_str().unwrap() })).unwrap();
    assert_eq!(info["name"], "Plant");
    assert_eq!(info["unsaved"], false);
    assert!(file.exists());

    run(&mut s, "set_project_info", json!({ "company": "ACME", "properties": { "Line": "3" } })).unwrap();
    let err = run(&mut s, "open_project", json!({ "path": file.to_str().unwrap() })).unwrap_err();
    assert!(err.contains("unsaved changes"), "{err}");
    let info = run(&mut s, "open_project", json!({ "path": file.to_str().unwrap(), "discard": true })).unwrap();
    assert_eq!(info["company"], "");

    let demo = run(&mut s, "new_project", json!({ "demo": true })).unwrap();
    assert!(demo["pages"].as_u64().unwrap() > 1);
    assert_eq!(demo["errors"], 0);

    // An existing file is only replaced on request.
    let err = run(&mut s, "new_project", json!({ "name": "Other", "path": file.to_str().unwrap(), "discard": true })).unwrap_err();
    assert!(err.contains("exists already"), "{err}");
    let info = run(&mut s, "new_project", json!({ "name": "Other", "path": file.to_str().unwrap(), "overwrite": true, "discard": true })).unwrap();
    assert_eq!(info["name"], "Other");
}

#[test]
fn renumbering_and_undo() {
    let mut s = session();
    run(&mut s, "new_project", json!({ "demo": true })).unwrap();
    run(&mut s, "renumber_wires", json!({ "overwrite_fixed": true })).unwrap();
    let before = s.project().unwrap().clone();
    run(&mut s, "add_text", json!({ "page": "2", "x": 0, "y": 0, "text": "x" })).unwrap();
    assert_eq!(run(&mut s, "undo", json!({})).unwrap()["undone"], true);
    assert_eq!(s.project().unwrap(), &before);
    assert_eq!(run(&mut s, "redo", json!({})).unwrap()["redone"], true);
    assert_ne!(s.project().unwrap(), &before);
}

#[test]
fn projects_from_templates() {
    let mut s = session();
    let out = run(&mut s, "new_project", json!({ "template": "Machine", "name": "Line 2" })).unwrap();
    assert_eq!(out["name"], "Line 2");
    assert!(s.project().unwrap().pages.len() > 5);
    assert!(run(&mut s, "new_project", json!({ "template": "no-such-template.oec", "discard": true })).is_err());
}

#[test]
fn examples_are_listed_and_opened() {
    let mut s = session();
    let list = run(&mut s, "list_examples", json!({})).unwrap();
    let names: Vec<&str> = list.as_array().unwrap().iter().map(|e| e["name"].as_str().unwrap()).collect();
    assert_eq!(names.first(), Some(&"lamp_circuit"));
    for name in names {
        let info = run(&mut s, "new_project", json!({ "example": name, "discard": true })).unwrap();
        assert_eq!((info["errors"].as_u64(), info["warnings"].as_u64()), (Some(0), Some(0)), "{name}");
    }
    let err = run(&mut s, "new_project", json!({ "example": "nothing", "discard": true })).unwrap_err();
    assert!(err.contains("list_examples"), "{err}");
}

#[test]
fn templates_by_name_in_any_case_and_properties_of_any_scalar() {
    let mut s = session();
    let info = run(&mut s, "new_project", json!({ "template": "machine" })).unwrap();
    assert!(info["pages"].as_u64().unwrap() > 1);
    let err = run(&mut s, "new_project", json!({ "template": "nosuch", "discard": true })).unwrap_err();
    assert!(err.contains("give a built-in one (") && err.contains("Machine"), "{err}");
    let info = run(&mut s, "set_project_info", json!({ "properties": { "Customer": 5, "Rated": true, "Line": "3" } })).unwrap();
    assert_eq!(info["properties"], json!({ "Customer": "5", "Line": "3", "Rated": "true" }));
    let info = run(&mut s, "set_project_info", json!({ "properties": { "Customer": null, "Line": "" } })).unwrap();
    assert_eq!(info["properties"], json!({ "Rated": "true" }));
    assert!(run(&mut s, "set_project_info", json!({ "properties": { "List": [1] } })).is_err());
    assert_eq!(s.project().unwrap().info.properties.len(), 1, "nothing is changed by a refused call");
}
