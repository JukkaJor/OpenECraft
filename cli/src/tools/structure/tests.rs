use super::*;
use crate::testing::*;

fn structured_session() -> Session {
    let mut s = new_session();
    run(&mut s, "add_page", json!({ "kind": "schematic" })).unwrap();
    run(&mut s, "set_page", json!({ "page": "1", "function": "=A1", "location": "+CAB1" })).unwrap();
    run(&mut s, "set_page", json!({ "page": "2", "function": "A2", "installation": "++HALL1", "location": "FIELD", "product": "-U1" })).unwrap();
    s
}

#[test]
fn identifiers_are_defined_renamed_and_tidied() {
    let mut s = structured_session();
    let v = run(&mut s, "list_structure", json!({})).unwrap();
    assert_eq!(v["aspects"], json!(["function", "location"]));
    let ids: Vec<&str> = v["identifiers"].as_array().unwrap().iter().map(|r| r["identifier"].as_str().unwrap()).collect();
    assert_eq!(ids, vec!["=A1", "=A2", "++HALL1", "+CAB1", "+FIELD", "-U1"]);
    assert_eq!(v["identifiers"][0]["defined"], false);

    let d = run(&mut s, "define_structure", json!({ "aspect": "=", "id": "A1", "description": "Pumps" })).unwrap();
    assert_eq!(d["identifier"], "=A1");
    assert!(run(&mut s, "define_structure", json!({ "aspect": "plant", "id": "A1" })).unwrap_err().contains("Unknown aspect"));
    run(&mut s, "define_structure", json!({ "aspect": "location", "id": "DESK" })).unwrap();

    let r = run(&mut s, "rename_structure", json!({ "aspect": "function", "id": "A1", "new_id": "P1" })).unwrap();
    assert_eq!((r["renamed"].as_u64(), r["identifier"].as_str()), (Some(1), Some("=P1")));
    let pg = &run(&mut s, "list_pages", json!({})).unwrap()[0];
    assert_eq!((pg["function"].as_str(), pg["structure"].as_str()), (Some("P1"), Some("=P1+CAB1")));

    let t = run(&mut s, "tidy_structure", json!({ "remove_unused": true })).unwrap();
    assert_eq!((t["defined"].as_u64(), t["removed"].as_u64()), (Some(5), Some(1)));
    let v = run(&mut s, "list_structure", json!({})).unwrap();
    assert_eq!(v["identifiers"][1]["identifier"], "=P1");
    assert_eq!(v["identifiers"][1]["description"], "Pumps");
    assert!(v["identifiers"].as_array().unwrap().iter().all(|r| r["defined"] == true));
    run(&mut s, "remove_structure", json!({ "aspect": "product", "id": "-U1" })).unwrap();
    assert!(run(&mut s, "remove_structure", json!({ "aspect": "product", "id": "U1" })).unwrap_err().contains("not defined"));
}

#[test]
fn templates_are_applied_and_exported() {
    let mut s = structured_session();
    let list = run(&mut s, "list_structure_templates", json!({})).unwrap();
    assert!(list.as_array().unwrap().iter().any(|t| t["name"] == "Machine (example identifiers)"));
    let r = run(&mut s, "apply_structure_template", json!({ "template": "Process plant (example identifiers)" })).unwrap();
    assert_eq!((r["added"].as_u64(), r["aspects"].clone()), (Some(7), json!(["function", "installation", "location"])));
    assert!(run(&mut s, "apply_structure_template", json!({ "template": "Spaceship" })).unwrap_err().contains("Unknown structure template"));
    assert!(run(&mut s, "apply_structure_template", json!({})).is_err());

    run(&mut s, "set_work_area", json!({ "name": "Hall", "installation": "HALL1" })).unwrap();
    let file = temp_dir("structure-template").join("plant.json");
    run(&mut s, "export_structure_template", json!({ "path": file.to_str().unwrap() })).unwrap();
    let mut other = new_session();
    let r = run(&mut other, "apply_structure_template", json!({ "path": file.to_str().unwrap() })).unwrap();
    assert_eq!(r["added"], 7);
    assert_eq!(run(&mut other, "list_structure", json!({})).unwrap()["work_areas"][0]["filter"], "++HALL1");

    let a = run(&mut s, "set_structure_aspects", json!({ "aspects": "location, product" })).unwrap();
    assert_eq!(a["aspects"], json!(["location", "product"]));
    assert!(run(&mut s, "set_structure_aspects", json!({ "aspects": "colour" })).is_err());
}

#[test]
fn work_areas_limit_the_listed_pages() {
    let mut s = structured_session();
    run(&mut s, "add_page", json!({ "kind": "device_list" })).unwrap();
    let w = run(&mut s, "set_work_area", json!({ "name": "Field", "location": "+FIELD" })).unwrap();
    assert_eq!(w["filter"], "+FIELD");
    let names: Vec<Value> = run(&mut s, "list_pages", json!({ "work_area": "Field" })).unwrap().as_array().unwrap().iter().map(|p| p["name"].clone()).collect();
    assert_eq!(names, vec![json!("2"), json!("3")]);
    // Changing a work area keeps the filters not given.
    run(&mut s, "set_work_area", json!({ "name": "Field", "function": "A*" })).unwrap();
    assert_eq!(run(&mut s, "list_structure", json!({})).unwrap()["work_areas"][0]["filter"], "=A*+FIELD");
    assert!(run(&mut s, "list_pages", json!({ "work_area": "Hall" })).unwrap_err().contains("No work area"));
    run(&mut s, "delete_work_area", json!({ "name": "Field" })).unwrap();
    assert!(run(&mut s, "delete_work_area", json!({ "name": "Field" })).is_err());
}
