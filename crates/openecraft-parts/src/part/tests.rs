use super::*;

#[test]
fn merge_adds_and_updates() {
    let mut db = PartsDb { parts: vec![Part { number: "A".into(), description: "old".into(), ..Default::default() }] };
    let incoming = vec![
        Part { number: "A".into(), description: "new".into(), ..Default::default() },
        Part { number: "B".into(), ..Default::default() },
    ];
    assert_eq!(db.merge(incoming.clone()), (1, 1));
    assert_eq!(db.get("A").unwrap().description, "new");
    // Same data again: nothing changes.
    assert_eq!(db.merge(incoming), (0, 0));
}

#[test]
fn unique_numbers() {
    let list = vec![Part { number: "X".into(), ..Default::default() }, Part { number: "X_1".into(), ..Default::default() }];
    assert_eq!(unique_number(&list, "Y"), "Y");
    assert_eq!(unique_number(&list, "X"), "X_2");
}

#[test]
fn technical_data_values() {
    let p = crate::default_catalog().into_iter().find(|p| p.number == "GEN.CBL-4G1.5").unwrap();
    assert_eq!(p.tech_value("cores"), Some("BN BK GY GNYE"));
    assert_eq!(p.tech_value("Cross-section"), Some("1.5 mm²"));
    assert_eq!(p.tech_value("Voltage"), None);
    assert_eq!(p.quantity_unit, "m");
}
