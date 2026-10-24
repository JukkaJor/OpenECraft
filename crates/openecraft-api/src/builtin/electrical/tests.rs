use super::*;
use crate::builtin::testing::*;

#[test]
fn settings_and_calculations_of_the_demo() {
    let mut s = demo_session();
    let e = run(&mut s, "electrical_settings", json!({})).unwrap();
    assert_eq!((e["voltage"].as_f64(), e["earthing"].as_str(), e["capacity_factor"].as_f64()), (Some(400.0), Some("tn_s"), Some(1.0)));
    let e = run(&mut s, "electrical_settings", json!({ "earthing": "tt", "ik_max": 6.0, "ambient": 50.0, "insulation": "xlpe" })).unwrap();
    assert_eq!((e["earthing"].as_str(), e["ik_max_ka"].as_f64(), e["insulation"].as_str()), (Some("tt"), Some(6.0), Some("xlpe")));
    assert!(run(&mut s, "electrical_settings", json!({ "earthing": "tx" })).unwrap_err().contains("Unknown earthing system"));

    let sc = run(&mut s, "short_circuit", json!({})).unwrap();
    let devices = sc["devices"].as_array().unwrap();
    assert!(!devices.is_empty());
    assert!(devices.iter().all(|d| d["breaks"] == true && d["ik_max_a"].as_f64() == Some(6000.0)));
    // TT: final circuits within 0.2 s, by a residual current device.
    assert!(devices.iter().all(|d| (d["permitted_s"].as_f64().unwrap() - 0.2).abs() < 1e-6 && d["disconnects"] == false));

    let l = run(&mut s, "load_calculation", json!({})).unwrap();
    assert!(l["installed_kw"].as_f64().unwrap() > 0.5);
    assert_eq!(l["power_supplies"][0]["without_data"], json!([]));
    let emc = run(&mut s, "emc_check", json!({})).unwrap();
    assert_eq!(emc["problems"], json!([]));
}

#[test]
fn safety_functions_and_rule_sets() {
    let mut s = demo_session();
    let f = json!({
        "name": "SF1",
        "required": "C",
        "subsystems": [{ "name": "Output", "devices": ["-K1"], "category": "One", "operations": 1000.0 }],
    });
    let r = run(&mut s, "safety_functions", json!({ "set": f })).unwrap();
    assert_eq!(r[0]["achieved"], "c");
    assert_eq!((r[0]["sil"].as_u64(), r[0]["met"].as_bool()), (Some(1), Some(true)));
    assert!(run(&mut s, "safety_functions", json!({ "remove": "SF9" })).is_err());
    assert_eq!(run(&mut s, "safety_functions", json!({ "remove": "SF1" })).unwrap(), json!([]));

    let r = run(&mut s, "rule_sets", json!({ "enable": "ul508a" })).unwrap();
    assert_eq!(r["libraries"].as_array().unwrap().iter().filter(|l| l["enabled"] == true).count(), 1);
    assert_eq!(r["limits"]["min_cross_section_power_mm2"].as_f64(), Some(2.08f32 as f64));
    let v = run(&mut s, "verify_project", json!({})).unwrap();
    assert!(v["failed_checks"].as_u64().unwrap() > 0);
    let custom = json!({ "name": "ACME", "active": true, "checks": [{ "scope": "Project", "require": { "field": "company", "op": "Equals", "value": "ACME" }, "level": "Error", "message": "Made by ACME" }] });
    let r = run(&mut s, "rule_sets", json!({ "add": custom, "disable": "ul508a" })).unwrap();
    assert_eq!(r["rule_sets"][0]["active"], true);
    let v = run(&mut s, "verify_project", json!({})).unwrap();
    assert_eq!(v["checks"].as_array().unwrap().len(), 1);
    assert!(v["problems"][0].as_str().unwrap().ends_with("Made by ACME (ACME)"));
    run(&mut s, "rule_sets", json!({ "remove": "ACME" })).unwrap();
    assert!(run(&mut s, "rule_sets", json!({ "enable": "nfpa" })).is_err());
}
