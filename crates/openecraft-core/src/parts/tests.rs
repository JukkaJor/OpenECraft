use super::*;

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
fn every_function_kind_can_be_placed() {
    // Each kind has a symbol, and each template of the catalog fits a symbol of its
    // kind (same number of connection points), so designations can be assigned.
    for k in FnKind::ALL {
        assert!(symbols::library().iter().any(|d| kind_of_symbol(d.key) == Some(k)), "no symbol for {}", k.title());
    }
    for part in catalog() {
        for f in part.functions.iter().filter(|f| f.kind != FnKind::Terminal) {
            let n = f.pin_list().len();
            assert!(
                symbols::library().iter().any(|d| kind_of_symbol(d.key) == Some(f.kind) && d.pins.len() == n),
                "{}: no {} symbol with {n} connection points",
                part.number,
                f.kind.title()
            );
        }
    }
    // Main and auxiliary functions of devices are counted against the templates.
    for d in symbols::library().iter().filter(|d| matches!(d.func, Func::Aux(_))) {
        assert!(kind_of_symbol(d.key).is_some(), "{} has no function kind", d.key);
    }
}

#[test]
fn function_kind_titles_are_translated() {
    let known: std::collections::HashSet<&str> = openecraft_i18n::catalog_entries().into_iter().map(|e| e[0]).collect();
    for k in FnKind::ALL {
        assert!(known.contains(k.title()), "{} missing in the translation catalog", k.title());
    }
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
fn parts_are_added_to_devices() {
    let mut prj = Project::default();
    let db = PartsDb::default();
    let number = db.parts[0].number.clone();
    add_to_device(&mut prj, &db, "-K1", &number, 1);
    add_to_device(&mut prj, &db, "-K1", &number, 2);
    assert_eq!(prj.devices["-K1"].parts, vec![PartRef { number: number.clone(), count: 3 }]);
    assert!(prj.parts.iter().any(|p| p.number == number), "copied into the project");
    // Unknown numbers are referenced all the same (typed by hand).
    add_to_device(&mut prj, &db, "-K1", "OWN.1", 1);
    assert_eq!(prj.devices["-K1"].parts.len(), 2);
}

#[test]
fn generic_parts_come_from_the_catalog_when_missing_in_the_database() {
    let mut prj = Project::default();
    ensure_in_project(&mut prj, &PartsDb { parts: Vec::new() }, "GEN.DUCT-60X80");
    assert!(prj.parts.iter().any(|p| p.number == "GEN.DUCT-60X80" && p.height == 60.0 && p.depth == 80.0));
    ensure_in_project(&mut prj, &PartsDb { parts: Vec::new() }, "UNKNOWN");
    assert_eq!(prj.parts.len(), 1);
}
