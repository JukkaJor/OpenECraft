use super::*;
use crate::{default_catalog, ConnectionPoint, Exit};

#[test]
fn csv_roundtrip() {
    let parts = default_catalog();
    let csv = to_csv(&parts);
    let back = from_csv(&csv).unwrap();
    assert_eq!(parts.len(), back.len());
    for (a, b) in parts.iter().zip(&back) {
        assert_eq!(a.number, b.number);
        assert_eq!(a.functions, b.functions);
        assert_eq!(a.accessories, b.accessories);
        assert_eq!(a.description, b.description);
    }
}

#[test]
fn json_import() {
    let db = PartsDb::default();
    let json = serde_json::to_string(&db).unwrap();
    assert_eq!(import(&json).unwrap().len(), db.parts.len());
}

#[test]
fn connection_points_in_csv() {
    let mut part = Part { number: "OWN.K1".into(), ..Default::default() };
    part.connection_points = vec![
        ConnectionPoint { pin: "A1".into(), pos: [5.5, 0.0, 60.0], exit: Exit::Up },
        ConnectionPoint { pin: "14".into(), pos: [40.0, 74.0, 55.25], exit: Exit::Down },
    ];
    let csv = to_csv(std::slice::from_ref(&part));
    assert!(csv.lines().next().unwrap().ends_with(";Connection points"));
    assert!(csv.contains("A1:5.5/0/60/up|14:40/74/55.25/down"));
    assert_eq!(from_csv(&csv).unwrap()[0].connection_points, part.connection_points);
    // Files without the column (older exports) have no points.
    let old = "Part number;Description
OWN.K2;Relay
";
    assert!(from_csv(old).unwrap()[0].connection_points.is_empty());
}

