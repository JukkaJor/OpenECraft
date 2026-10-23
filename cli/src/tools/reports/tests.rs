use super::*;
use crate::testing::*;

#[test]
fn report_blocks_and_schemes() {
    let mut s = session();
    run(&mut s, "new_project", json!({ "demo": true })).unwrap();
    assert!(run(&mut s, "set_report_block", json!({ "name": "Strips" })).is_err(), "a new block needs a kind");
    let form = json!({ "rows": 2, "sort": [{ "column": "Core", "descending": true }] });
    run(&mut s, "set_report_scheme", json!({ "name": "Short", "kind": "cable_connections", "form": form })).unwrap();
    let schemes = run(&mut s, "list_report_schemes", json!({})).unwrap();
    assert_eq!(schemes[0]["form"]["rows"], 2);
    assert!(run(&mut s, "set_report_block", json!({ "name": "Cables", "kind": "cable_connections", "scheme": "Nope" })).is_err());
    let b = run(&mut s, "set_report_block", json!({ "name": "Cables", "kind": "cable_connections", "scheme": "Short", "function": "=A1" })).unwrap();
    assert_eq!((b["scheme"].as_str(), b["function"].as_str()), (Some("Short"), Some("A1")));
    let r = run(&mut s, "generate_report_blocks", json!({ "name": "Cables" })).unwrap();
    assert_eq!(r["pages"], 2, "four cores of -W1, two rows per page");
    let blocks = run(&mut s, "list_report_blocks", json!({})).unwrap();
    assert_eq!(blocks[0]["pages"].as_array().unwrap().len(), 2);
    run(&mut s, "set_report_block", json!({ "name": "Lineup", "kind": "terminal_lineup", "groups": "-X1" })).unwrap();
    assert_eq!(run(&mut s, "generate_report_blocks", json!({})).unwrap()["pages"], 3);
    run(&mut s, "remove_report_block", json!({ "name": "Lineup" })).unwrap();
    assert!(run(&mut s, "remove_report_block", json!({ "name": "Lineup" })).is_err());
}

#[test]
fn report_pages_are_output_into_another_project() {
    let dir = temp_dir("output-reports");
    let target = dir.join("plant.oec");
    let mut s = session();
    run(&mut s, "new_project", json!({})).unwrap();
    run(&mut s, "save_project", json!({ "path": target.to_str().unwrap() })).unwrap();
    run(&mut s, "new_project", json!({ "demo": true })).unwrap();
    run(&mut s, "set_report_block", json!({ "name": "Strips", "kind": "terminal_strip_overview", "per_group": false })).unwrap();
    run(&mut s, "generate_report_blocks", json!({})).unwrap();
    let path = target.to_str().unwrap();
    assert!(run(&mut s, "output_reports", json!({ "target": path, "block": "Other" })).is_err());
    let r = run(&mut s, "output_reports", json!({ "target": path, "block": "Strips" })).unwrap();
    assert_eq!(r["pages"], 1);
    let all = run(&mut s, "output_reports", json!({ "target": path })).unwrap();
    assert!(all["pages"].as_u64().unwrap() > 1);
    let prj = openecraft_core::model::Project::from_json(&std::fs::read_to_string(&target).unwrap()).unwrap();
    assert_eq!(prj.pages.len() as u64, 1 + all["pages"].as_u64().unwrap(), "replaced, not added");
}
