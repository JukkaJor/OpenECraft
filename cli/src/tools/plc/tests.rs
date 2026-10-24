use super::*;
use crate::testing::*;

fn demo() -> Session {
    let mut s = session();
    run(&mut s, "new_project", json!({ "demo": true })).unwrap();
    s
}

#[test]
fn aml_and_iec_exports_and_aml_import() {
    let mut s = demo();
    run(&mut s, "configure_plc_hardware", json!({})).unwrap();
    let aml = run(&mut s, "export", json!({ "format": "aml" })).unwrap();
    let aml = aml.as_str().unwrap().to_string();
    assert!(aml.contains("<CAEXFile"));
    let gvl = run(&mut s, "export", json!({ "format": "plc_variables" })).unwrap();
    assert!(gvl.as_str().unwrap().contains("AT %IX0.0 : BOOL;"));

    let mut other = demo();
    let r = run(&mut other, "import_aml", json!({ "text": aml })).unwrap();
    assert_eq!(r["imported"]["stations"], 1);
    assert_eq!(r["stations"][0]["modules"].as_array().unwrap().len(), 2);
    assert!(run(&mut other, "import_aml", json!({})).is_err());
}

#[test]
fn device_descriptions_are_listed_and_imported() {
    let mut s = demo();
    let list = run(&mut s, "list_device_descriptions", json!({ "filter": "cpu" })).unwrap();
    assert!(list.as_array().unwrap().iter().any(|d| d["id"] == "GEN.CPU-PN" && d["source"] == "built-in"));
    let dir = temp_dir("esi");
    let path = dir.join("acme.xml");
    std::fs::write(&path, "<EtherCATInfo><Vendor><Name>ACME</Name></Vendor><Descriptions><Devices><Device><Type>EK1100</Type><Name>Coupler</Name></Device></Devices></Descriptions></EtherCATInfo>").unwrap();
    let r = run(&mut s, "import_device_descriptions", json!({ "path": path.to_str().unwrap() })).unwrap();
    assert_eq!(r["added"], 1);
    assert_eq!(r["descriptions"][0]["source"], "acme.xml");
    let list = run(&mut s, "list_device_descriptions", json!({ "filter": "EK1100" })).unwrap();
    assert_eq!(list[0]["kind"], "Coupler");
}
