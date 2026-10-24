use super::*;
use crate::builtin::testing::*;

#[test]
fn climate_and_sizing_of_the_demo() {
    let mut s = demo_session();
    let c = run(&mut s, "enclosure_climate", json!({})).unwrap();
    assert_eq!(c[0]["installation"], "wall_mounted");
    assert!(c[0]["power_loss_w"].as_f64().unwrap() > 10.0);
    assert!(c[0]["fan_m3_per_h"].is_null());
    let name = c[0]["enclosure"].as_str().unwrap().to_string();
    let c = run(&mut s, "enclosure_climate", json!({ "enclosure": name, "material": "plastic", "extra_loss": 300.0 })).unwrap();
    assert_eq!(c[0]["material"], "plastic");
    assert!(c[0]["fan_m3_per_h"].as_f64().unwrap() > 0.0);
    assert!(run(&mut s, "enclosure_climate", json!({ "enclosure": name, "material": "wood" })).unwrap_err().contains("Unknown material"));

    let z = run(&mut s, "electrical_sizing", json!({})).unwrap();
    assert_eq!(z["too_thin"], json!([]));
    assert_eq!(z["voltage_drop_limit_percent"], 5.0);
    assert!(!z["cables"].as_array().unwrap().is_empty());
}
