use super::*;

#[test]
fn part_numbers_of_components() {
    let ts = system("baying").unwrap();
    assert_eq!(body_number(ts, 800.0, 2000.0, 600.0), "GEN.TS-800X2000X600");
    assert_eq!(side_number(ts, 2000.0, 600.0), "GEN.TS-SP-2000X600");
    assert_eq!(rear_number(ts, 800.0, 2000.0), "GEN.TS-RW-800X2000");
    assert_eq!(plinth_number(800.0, 600.0, 100.0), "GEN.PL-800X600-100");
    assert_eq!(gland_plate_number(600.0, 210.0), "GEN.GP-600X210");
    assert!(system("nonsense").is_none());
}

#[test]
fn racks_have_their_units() {
    let rk = system(RACK_SYSTEM).unwrap();
    let units: Vec<u32> = rk.sizes.iter().map(|s| rack_units(s.1)).collect();
    assert_eq!(units, [12, 24, 42, 42, 42, 47]);
    assert!(rk.sizes.iter().all(|s| s.0 - 2.0 * rk.frame >= RACK_WIDTH), "the panels fit between the posts");
    assert_eq!(rack_rail_number(42), "GEN.RK-RAIL-42U");
    assert_eq!(rack_units(80.0), 0);
}

#[test]
fn fasteners() {
    assert_eq!(thread(" m5 "), Some("M5"));
    assert_eq!(thread("M7"), None);
    assert_eq!(screw_number("M5"), "GEN.SCREW-M5X12");
    assert_eq!(rivet_nut_number("M4"), "GEN.RIVNUT-M4");
}

#[test]
fn systems_have_sizes() {
    for s in SYSTEMS {
        assert!(!s.sizes.is_empty(), "{}", s.id);
        assert!(s.wall > 0.0 && s.flange >= s.wall, "{}", s.id);
        assert_eq!(s.bayable, s.frame > 0.0, "{}", s.id);
    }
}
