use super::*;
use crate::default_catalog;

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
