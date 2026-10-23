use super::*;

#[test]
fn locks_are_exclusive() {
    let now = Instant::now();
    let mut s = Sessions::default();
    let anna = s.open("p", "anna", now);
    let ben = s.open("p", "ben", now);
    assert_ne!(anna.session, ben.session);
    s.lock("p", &anna, &[Target::Page(1), Target::Project], now).unwrap();
    s.lock("p", &ben, &[Target::Page(2)], now).unwrap();
    // All or nothing.
    match s.lock("p", &ben, &[Target::Page(3), Target::Page(1)], now) {
        Err(LockError::Held(l)) => assert_eq!((l.user.as_str(), l.target), ("anna", Target::Page(1))),
        other => panic!("{other:?}"),
    }
    let locks = s.locks("p");
    assert_eq!(locks.iter().map(|l| l.target).collect::<Vec<_>>(), vec![Target::Project, Target::Page(1), Target::Page(2)]);
    assert_eq!(s.users("p"), vec!["anna".to_string(), "ben".to_string()]);
    // Locking again is fine.
    s.lock("p", &anna, &[Target::Page(1)], now).unwrap();

    s.release("p", &anna, &[Target::Page(1)], now).unwrap();
    s.lock("p", &ben, &[Target::Page(1)], now).unwrap();
    s.close("p", &anna).unwrap();
    assert_eq!(s.locks("p").len(), 2);
    assert_eq!(s.users("p"), vec!["ben".to_string()]);
}

#[test]
fn sessions_belong_to_user_and_project() {
    let now = Instant::now();
    let mut s = Sessions::default();
    let anna = s.open("p", "anna", now);
    assert_eq!(s.touch("q", &anna, now), Err(LockError::Expired));
    let fake = Session { user: "mallory".into(), session: anna.session.clone() };
    assert_eq!(s.lock("p", &fake, &[Target::Project], now), Err(LockError::Expired));
    assert!(s.locks("q").is_empty());
}

#[test]
fn idle_sessions_expire_with_their_locks() {
    let start = Instant::now();
    let mut s = Sessions::default();
    let anna = s.open("p", "anna", start);
    let ben = s.open("p", "ben", start);
    s.lock("p", &anna, &[Target::Page(1)], start).unwrap();
    let later = start + Duration::from_secs(SESSION_TIMEOUT_SECS - 10);
    s.touch("p", &ben, later).unwrap();
    s.expire(start + Duration::from_secs(SESSION_TIMEOUT_SECS + 1));
    assert!(s.locks("p").is_empty());
    assert_eq!(s.touch("p", &anna, later), Err(LockError::Expired));
    assert!(s.touch("p", &ben, later).is_ok());
}
