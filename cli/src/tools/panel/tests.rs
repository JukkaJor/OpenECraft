use super::*;
use crate::testing::*;

fn demo() -> Session {
    let mut s = session();
    run(&mut s, "new_project", json!({ "name": "Demo", "demo": true })).unwrap();
    s
}

#[test]
fn enclosure_of_a_system_with_components() {
    let mut s = demo();
    let systems = run(&mut s, "enclosure_systems", json!({})).unwrap();
    assert!(systems.as_array().unwrap().iter().any(|x| x["system"] == "baying" && x["bayable"] == true));
    let e = run(&mut s, "add_enclosure", json!({ "name": "+CAB2", "system": "baying" })).unwrap();
    assert_eq!(e["part"], "GEN.TS-600X1800X500");
    assert_eq!(e["plinth"], 100.0);
    let e = run(&mut s, "set_enclosure", json!({ "enclosure": "+CAB2", "width": 800, "height": 2000, "right_open": true, "gland_plate": true })).unwrap();
    assert_eq!(e["part"], "GEN.TS-800X2000X500");
    let parts: Vec<&str> = e["components"].as_array().unwrap().iter().filter_map(|c| c["part"].as_str()).collect();
    assert!(parts.contains(&"GEN.TS-SP-2000X500") && parts.contains(&"GEN.GP-800X500") && parts.contains(&"GEN.TS-BAY"), "{parts:?}");
    assert!(run(&mut s, "add_enclosure", json!({ "name": "+CAB2" })).unwrap_err().contains("exists"));
    assert!(run(&mut s, "set_enclosure", json!({ "enclosure": "+CAB2", "system": "tent" })).unwrap_err().contains("Unknown enclosure system"));
}
