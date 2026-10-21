use super::*;
use crate::testing::*;

#[test]
fn reports_and_exports() {
    let dir = temp_dir("project-export");
    let mut s = session();
    run(&mut s, "new_project", json!({ "demo": true })).unwrap();
    let bom = run(&mut s, "report", json!({ "kind": "bill_of_materials" })).unwrap();
    assert!(!bom["rows"].as_array().unwrap().is_empty());
    assert!(run(&mut s, "report", json!({ "kind": "device_list", "format": "csv" })).unwrap().as_str().unwrap().contains(';'));
    assert!(run(&mut s, "report", json!({ "kind": "parts" })).is_err());

    let svg = run(&mut s, "export", json!({ "format": "svg", "page": "1" })).unwrap();
    assert!(svg.as_str().unwrap().contains("<svg"));
    assert!(run(&mut s, "export", json!({ "format": "svg" })).is_err(), "svg needs a page");
    let file = dir.join("all.dxf");
    let out = run(&mut s, "export", json!({ "format": "dxf", "path": file.to_str().unwrap() })).unwrap();
    assert!(out["bytes"].as_u64().unwrap() > 0);
    assert!(std::fs::read_to_string(&file).unwrap().contains("SECTION"));
    for format in ["html", "pdf", "wire_list", "plc_table", "kbl"] {
        assert!(run(&mut s, "export", json!({ "format": format })).is_ok(), "{format}");
    }
}
