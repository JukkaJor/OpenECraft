use super::*;
use crate::testing::*;

#[test]
fn pages_are_added_renamed_and_deleted() {
    let mut s = new_session();
    assert_eq!(run(&mut s, "add_page", json!({ "kind": "schematic" })).unwrap()["name"], "2");
    let bom = run(&mut s, "add_page", json!({ "kind": "bill-of-materials", "after": "1", "name": "BOM" })).unwrap();
    assert_eq!(bom["kind"], "bill_of_materials");
    let names: Vec<Value> = run(&mut s, "list_pages", json!({})).unwrap().as_array().unwrap().iter().map(|p| p["name"].clone()).collect();
    assert_eq!(names, vec![json!("1"), json!("BOM"), json!("2")]);

    assert!(run(&mut s, "add_page", json!({ "kind": "drawing" })).is_err());
    assert!(run(&mut s, "set_page", json!({ "page": "2", "name": "BOM" })).unwrap_err().contains("already exists"));
    let pg = run(&mut s, "set_page", json!({ "page": "2", "name": "10", "function": "=A1" })).unwrap();
    assert_eq!(pg["function"], "A1");

    run(&mut s, "delete_page", json!({ "page": "BOM" })).unwrap();
    run(&mut s, "delete_page", json!({ "page": "10" })).unwrap();
    assert!(run(&mut s, "delete_page", json!({ "page": "1" })).unwrap_err().contains("last page"));
}

#[test]
fn released_pages_are_not_deleted() {
    let mut s = new_session();
    run(&mut s, "add_page", json!({ "kind": "schematic" })).unwrap();
    s.project_mut().unwrap().pages[1].locked = true;
    assert_eq!(run(&mut s, "delete_page", json!({ "page": "2" })).unwrap_err(), "Released pages cannot be deleted");
    assert!(run(&mut s, "set_page", json!({ "page": "2", "name": "9" })).unwrap_err().contains("locked"));
}

