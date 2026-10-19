use super::*;

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
fn catalog_numbers_unique_and_accessories_exist() {
    let parts = default_catalog();
    for (i, p) in parts.iter().enumerate() {
        assert!(parts[i + 1..].iter().all(|q| q.number != p.number), "duplicate {}", p.number);
        for a in &p.accessories {
            assert!(parts.iter().any(|q| &q.number == a), "{} accessory {a} missing", p.number);
        }
    }
}

#[test]
fn legacy_single_part_is_migrated() {
    let json = r#"{"devices": {"-K1": {"part": "CT-9A-24DC", "description": ""}},
                  "parts": [{"number": "CT-9A-24DC", "manufacturer": "Generic", "description": "old", "price": 1.0}]}"#;
    let mut prj: Project = serde_json::from_str(json).unwrap();
    prj.normalize();
    assert_eq!(prj.devices["-K1"].parts, vec![PartRef { number: "GEN.CT-9A-24DC".into(), count: 1 }]);
    assert!(prj.parts.iter().any(|p| p.number == "GEN.CT-9A-24DC" && !p.functions.is_empty()));
    assert!(!prj.parts.iter().any(|p| p.number == "CT-9A-24DC"));
}

#[test]
fn designations_from_template_and_device_selection() {
    let mut prj = crate::sample::demo_project();
    // Scramble the contact numbers of -K1, then take them from the parts again.
    for pg in &mut prj.pages {
        for s in pg.symbols.iter_mut().filter(|s| s.tag == "-K1" && s.sym == "no") {
            s.pins = vec!["x".into(), "y".into()];
        }
    }
    assert_eq!(apply_designations(&mut prj, None, "-K1"), 3);
    let mut pins: Vec<String> = placed_functions(&prj, "-K1")
        .into_iter()
        .filter(|f| f.0 == FnKind::No)
        .map(|f| f.1.join("/"))
        .collect();
    pins.sort();
    assert_eq!(pins, vec!["13/14", "53/54", "83/84"]);

    let db = PartsDb::default();
    let placed = placed_functions(&prj, "-K1");
    let main = main_kind(&prj, "-K1");
    assert_eq!(main, Some(FnKind::Coil));
    assert!(fits_device(db.get("GEN.CT-9A-24DC").unwrap(), &placed, main));
    assert!(!fits_device(db.get("GEN.MCB-1P-C6").unwrap(), &placed, main));
}

#[test]
fn json_import() {
    let db = PartsDb::default();
    let json = serde_json::to_string(&db).unwrap();
    assert_eq!(import(&json).unwrap().len(), db.parts.len());
}
