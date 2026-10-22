use super::*;
use crate::testing::*;

fn demo() -> Session {
    let mut s = session();
    run(&mut s, "new_project", json!({ "demo": true })).unwrap();
    s
}

#[test]
fn tables_are_read_changed_and_round_tripped() {
    let mut s = demo();
    let t = run(&mut s, "table_get", json!({ "table": "devices", "filter": "-K1" })).unwrap();
    assert_eq!(t["rows"][0]["key"], "-K1");
    assert_eq!(t["columns"][0], json!({ "name": "Device tag", "editable": true }));
    let r = run(&mut s, "table_set", json!({ "table": "devices", "key": "-K1", "column": "description", "value": "Main contactor" })).unwrap();
    assert_eq!(r["changed"], true);
    assert!(run(&mut s, "table_set", json!({ "table": "devices", "key": "-K1", "column": "Colour", "value": "x" })).unwrap_err().contains("Unknown column"));
    assert!(run(&mut s, "table_get", json!({ "table": "cables" })).unwrap_err().contains("Unknown table"));

    let r = run(&mut s, "table_replace", json!({ "table": "devices", "column": "Description", "find": "Main", "replace": "Motor" })).unwrap();
    assert_eq!(r["changed"], 1);
    assert_eq!(s.project().unwrap().devices["-K1"].description, "Motor contactor");

    let dir = temp_dir("tables");
    let file = dir.join("devices.csv");
    run(&mut s, "table_get", json!({ "table": "devices", "path": file.to_str().unwrap() })).unwrap();
    let csv = std::fs::read_to_string(&file).unwrap().replace("Motor contactor", "Spare contactor");
    std::fs::write(&file, csv).unwrap();
    let r = run(&mut s, "table_import", json!({ "table": "devices", "path": file.to_str().unwrap() })).unwrap();
    assert_eq!((r["changed"].as_u64(), r["problems"].as_array().map(|p| p.len())), (Some(1), Some(0)));
    assert_eq!(s.project().unwrap().devices["-K1"].description, "Spare contactor");
}
