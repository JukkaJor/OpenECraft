use super::*;

fn part(number: &str, description: &str, current: &str, tech: &str) -> Part {
    Part { number: number.into(), description: description.into(), current: current.into(), tech_data: tech.into(), ..Default::default() }
}

fn mcb(letter: &str, rated: f32) -> Protection {
    Protection::of_part(&part("X.MCB", "", &format!("{rated} A"), &format!("Characteristic: {letter}")), FnKind::Breaker1).unwrap()
}

#[test]
fn data_from_the_parts() {
    let p = Protection::of_part(&part("GEN.MCB-1P-B10", "Miniature circuit breaker 1P B10", "10 A", ""), FnKind::Breaker1).unwrap();
    assert_eq!((p.characteristic, p.rated, p.breaking, p.instantaneous), (Characteristic::B, 10.0, None, (3.0, 5.0)));
    let p = Protection::of_part(&part("X.Y", "Breaker D16", "16 A", "Characteristic: K\nBreaking capacity: 6 kA"), FnKind::Breaker1).unwrap();
    assert_eq!((p.characteristic, p.breaking), (Characteristic::K, Some(6.0)));
    let p = Protection::of_part(&part("X.Y", "", "16 A", "Icu: 25 kA"), FnKind::Breaker3).unwrap();
    assert_eq!((p.characteristic, p.breaking), (Characteristic::C, Some(25.0)));
    let p = Protection::of_part(&part("X.F", "Fuse", "2 A", ""), FnKind::Fuse).unwrap();
    assert_eq!(p.characteristic, Characteristic::Gg);
    let p = Protection::of_part(&part("X.F", "Fuse 63 A aM", "63 A", ""), FnKind::Fuse3).unwrap();
    assert_eq!(p.characteristic, Characteristic::Am);
    let p = Protection::of_part(&part("X.M", "", "2.5 - 4 A", "Instantaneous trip: 13 x In\nLet-through energy: 2.5e4 A²s"), FnKind::MotorProtection).unwrap();
    assert_eq!((p.characteristic, p.rated, p.let_through), (Characteristic::Motor, 4.0, Some(25000.0)));
    assert!((p.instantaneous.0 - 10.4).abs() < 1e-4 && (p.instantaneous.1 - 15.6).abs() < 1e-4);
    assert!(Protection::of_part(&part("X.Y", "", "", ""), FnKind::Breaker1).is_none());
}

#[test]
fn circuit_breaker_curve() {
    let c16 = mcb("C", 16.0);
    assert_eq!(c16.trip_time(1.1 * 16.0), (NO_TRIP, NO_TRIP));
    // Conventional non-tripping 1.13 In, tripping 1.45 In within an hour.
    assert_eq!(c16.trip_time(1.4 * 16.0).1, NO_TRIP);
    assert!(c16.trip_time(1.5 * 16.0).1 <= 3600.0);
    // 2.55 In: between 1 and 60 s (the lower band at 5 s).
    let (lo, hi) = c16.trip_time(2.55 * 16.0);
    assert!((lo - 5.0).abs() < 1e-3 && (hi - 60.0).abs() < 1e-3);
    // Instantaneous between 5 and 10 In.
    assert_eq!(c16.trip_time(6.0 * 16.0).0, 0.0);
    assert!(c16.trip_time(6.0 * 16.0).1 > 0.4);
    assert_eq!(c16.trip_time(10.0 * 16.0).1, 0.1);
    assert!((c16.current_for(0.4) - 160.0).abs() < 0.5);
    assert!((mcb("B", 16.0).current_for(5.0) - 80.0).abs() < 0.5);
}

#[test]
fn fuse_and_motor_protection_curves() {
    let f = Protection::of_part(&part("X.F", "", "16 A", ""), FnKind::Fuse).unwrap();
    assert_eq!(f.trip_time(1.2 * 16.0), (NO_TRIP, NO_TRIP));
    assert!((f.trip_time(4.06 * 16.0).1 - 5.0).abs() < 1e-3);
    assert!((f.trip_time(2.06 * 16.0).0 - 10.0).abs() < 1e-3);
    assert!(f.trip_time(100.0).0 < f.trip_time(100.0).1);
    let m = Protection::of_part(&part("X.M", "", "4 A", ""), FnKind::MotorProtection).unwrap();
    let (lo, hi) = m.trip_time(7.2 * 4.0);
    assert!((lo - 4.0).abs() < 1e-3 && (hi - 10.0).abs() < 1e-3);
    assert_eq!(m.trip_time(15.0 * 4.0).1, 0.1);
}

#[test]
fn energy_let_through() {
    let c16 = mcb("C", 16.0);
    let (_, t) = c16.trip_time(50.0);
    assert_eq!(c16.energy(50.0), Some(50.0 * 50.0 * t));
    assert_eq!(c16.energy(1000.0), None);
    let mut limited = c16.clone();
    limited.let_through = Some(30000.0);
    assert_eq!(limited.energy(1000.0), Some(30000.0));
    let f = Protection::of_part(&part("X.F", "", "16 A", ""), FnKind::Fuse).unwrap();
    assert!(f.energy(1000.0).is_some());
    assert_eq!(f.energy(10.0), None);
}
