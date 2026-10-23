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

#[test]
fn panel_layout_in_3d() {
    let mut s = session();
    run(&mut s, "new_project", json!({ "demo": true })).unwrap();
    let gltf = run(&mut s, "export", json!({ "format": "gltf" })).unwrap();
    let v: Value = serde_json::from_str(gltf.as_str().unwrap()).unwrap();
    assert_eq!(v["asset"]["version"], "2.0");
    let file = temp_dir("export-3d").join("panel.stp");
    let r = run(&mut s, "export", json!({ "format": "step", "path": file.to_str().unwrap() })).unwrap();
    assert!(r["bytes"].as_u64().unwrap() > 1000);
    let step = std::fs::read_to_string(&file).unwrap();
    assert!(step.starts_with("ISO-10303-21;") && step.contains("FILE_NAME('panel.stp'"));
}
