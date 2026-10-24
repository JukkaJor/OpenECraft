use super::*;
use crate::testing::*;

fn demo() -> Session {
    let mut s = session();
    run(&mut s, "new_project", json!({ "name": "Demo", "demo": true })).unwrap();
    s
}

#[test]
fn connectors_are_listed() {
    let mut s = session();
    let r = run(&mut s, "integration_connectors", json!({})).unwrap();
    let ids: Vec<&str> = r["connectors"].as_array().unwrap().iter().filter_map(|c| c["id"].as_str()).collect();
    for id in ["erp", "pdm", "plm", "ecad", "mcad", "procurement", "wiring_machine"] {
        assert!(ids.contains(&id), "{id}");
    }
}

#[test]
fn export_preview_apply_and_conflicts() {
    let mut s = demo();
    let csv = run(&mut s, "integration_export", json!({ "connector": "erp", "date": "2026-10-24" })).unwrap();
    let csv = csv.as_str().unwrap().to_string();
    let part = csv.lines().nth(1).unwrap().split(';').next().unwrap().to_string();
    run(&mut s, "set_integration_profile", json!({ "name": "SAP", "connector": "erp", "mapping": { "erp_number": "MATNR" } })).unwrap();
    let text = format!("Material;MATNR;Bestand\n{part};100-200;5\n");
    let preview = run(&mut s, "integration_import", json!({ "connector": "erp", "text": text, "profile": "SAP" })).unwrap();
    assert_eq!(preview["changes"].as_array().unwrap().len(), 2);
    assert!(preview["applied"].is_null());
    assert!(s.project().unwrap().integration.items.is_empty());
    let applied = run(&mut s, "integration_import", json!({ "connector": "erp", "text": text, "profile": "SAP", "apply": true })).unwrap();
    assert_eq!(applied["applied"], 2);
    let st = run(&mut s, "integration_status", json!({})).unwrap();
    assert_eq!(st["items"][&part]["erp_number"], "100-200");
    assert_eq!(st["log"].as_array().unwrap().len(), 2);
    let ex = run(&mut s, "integration_export", json!({ "connector": "exchange" })).unwrap();
    assert!(ex.as_str().unwrap().contains("openecraft.exchange/1"));
}

#[test]
fn wrong_input_is_rejected() {
    let mut s = demo();
    assert!(run(&mut s, "integration_export", json!({ "connector": "sap" })).unwrap_err().contains("Unknown connector"));
    assert!(run(&mut s, "integration_import", json!({ "connector": "erp" })).unwrap_err().contains("Missing parameter"));
    assert!(run(&mut s, "integration_import", json!({ "connector": "erp", "text": "a;b\n", "profile": "X" })).unwrap_err().contains("No profile"));
    assert!(run(&mut s, "set_integration_profile", json!({ "name": "X", "connector": "erp", "mapping": { "matnr": "MATNR" } })).unwrap_err().contains("Unknown field"));
    assert_eq!(run(&mut s, "set_integration_profile", json!({ "name": "X", "remove": true })).unwrap()["removed"], 0);
}
