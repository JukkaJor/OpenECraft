use super::*;
use crate::builtin::testing::*;

#[test]
fn pct_loops_of_the_pre_planning() {
    let mut s = new_session();
    let v = run(&mut s, "set_pct_loop", json!({ "name": "101", "description": "Tank temperature", "devices": "-KF101 -TA101" })).unwrap();
    assert_eq!(v[0]["devices"], json!(["-KF101", "-TA101"]));
    assert_eq!(v[0]["status"], "planned");
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "pid_instrument", "x": 100, "y": 100, "tag": "-BT101", "label": "TIC 101" })).unwrap();
    let v = run(&mut s, "list_pct_loops", json!({})).unwrap();
    assert_eq!(v[0]["functions"], json!(["TIC"]));
    assert_eq!(v[0]["measured_variable"], "Temperature");
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "coil", "x": 200, "y": 100, "tag": "-KF101" })).unwrap();
    assert_eq!(run(&mut s, "list_pct_loops", json!({})).unwrap()[0]["status"], "in_engineering");
    assert!(run(&mut s, "set_pct_loop", json!({ "name": "102", "rename": "101" })).unwrap_err().contains("already exists"));
    run(&mut s, "set_pct_loop", json!({ "name": "101", "rename": "110" })).unwrap();
    let v = run(&mut s, "delete_pct_loop", json!({ "name": "110" })).unwrap();
    // The instrument still names loop 101.
    assert_eq!(v.as_array().unwrap().iter().filter(|l| l["defined"] == true).count(), 0);
    assert!(run(&mut s, "delete_pct_loop", json!({ "name": "110" })).is_err());
}
