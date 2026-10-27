use super::*;

#[test]
fn delays_after_repeated_failures() {
    let mut t = Throttle::default();
    let now = Instant::now();
    for _ in 0..FREE_ATTEMPTS {
        assert_eq!(t.wait("Anna", now), None);
        t.failed("anna ", now);
    }
    assert_eq!(t.wait("ANNA", now), Some(1));
    assert_eq!(t.wait("anna", now + Duration::from_secs(2)), None);
    t.failed("anna", now);
    assert_eq!(t.wait("anna", now), Some(2));
    // Other names are not affected; a success forgets the failures.
    assert_eq!(t.wait("ben", now), None);
    t.succeeded("Anna");
    assert_eq!(t.wait("anna", now), None);
}

#[test]
fn wait_is_capped() {
    let mut t = Throttle::default();
    let now = Instant::now();
    for _ in 0..40 {
        t.failed("x", now);
    }
    assert_eq!(t.wait("x", now), Some(MAX_WAIT.as_secs()));
}
