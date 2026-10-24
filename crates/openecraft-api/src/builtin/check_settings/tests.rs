use super::*;
use crate::builtin::testing::*;
use openecraft_core::analysis::Analysis;

fn issues(s: &Session) -> Vec<openecraft_core::issue::Issue> {
    let prj = s.project().unwrap();
    s.plugins.check(prj, &Analysis::build(prj))
}

#[test]
fn checks_levels_and_accepted_messages() {
    let mut s = demo_session();
    // A device without part gives a message of the `devices` check.
    run(&mut s, "place_symbol", json!({ "page": "2", "symbol": "coil", "x": 380, "y": 240, "tag": "-K99" })).unwrap();
    let found = issues(&s);
    let first = found.iter().find(|i| i.check == "devices" && i.key().is_some()).expect("a devices message").clone();
    let code = first.code();
    assert!(code.starts_with("devices.") && code.len() == "devices.".len() + 6, "{code}");
    assert_ne!(first.fingerprint(), code);

    // Changing the level of the kind of message changes every occurrence.
    run(&mut s, "set_message_level", json!({ "code": code, "level": "info" })).unwrap();
    assert!(issues(&s).iter().filter(|i| i.code() == code).all(|i| i.severity == openecraft_core::issue::Severity::Info));
    run(&mut s, "set_message_level", json!({ "code": code, "level": "off" })).unwrap();
    assert!(!issues(&s).iter().any(|i| i.code() == code));
    assert!(run(&mut s, "set_message_level", json!({ "code": code, "level": "loud" })).is_err());
    let shown = run(&mut s, "set_message_level", json!({ "code": code, "level": "default" })).unwrap();
    assert_eq!(shown["levels"], json!({}));

    // Accepted: still listed, but marked.
    run(&mut s, "accept_message", json!({ "fingerprint": first.fingerprint(), "note": "spare coil", "user": "nils" })).unwrap();
    let again = issues(&s);
    assert!(again.iter().any(|i| i.fingerprint() == first.fingerprint() && i.accepted));
    assert_eq!(again.iter().filter(|i| i.accepted).count(), 1);
    let shown = run(&mut s, "accept_message", json!({ "fingerprint": first.fingerprint(), "accept": false })).unwrap();
    assert_eq!(shown["accepted"], json!([]));

    // Switched off checks do not run.
    run(&mut s, "set_check", json!({ "name": "devices", "enabled": false })).unwrap();
    assert!(!issues(&s).iter().any(|i| i.check == "devices"));
    let shown = run(&mut s, "set_check", json!({ "name": "devices", "enabled": true })).unwrap();
    assert_eq!(shown["disabled"], json!([]));
}

#[test]
fn stale_acceptances_and_csv() {
    let mut s = demo_session();
    run(&mut s, "place_symbol", json!({ "page": "2", "symbol": "coil", "x": 380, "y": 240, "tag": "-K99" })).unwrap();
    let found = issues(&s);
    let first = found.iter().find(|i| i.check == "devices").unwrap().clone();
    let prj = s.project_mut().unwrap();
    openecraft_core::checks::accept(prj, &first, true, "", "", "");
    assert!(openecraft_core::checks::stale_accepted(prj, &found).is_empty());
    assert_eq!(openecraft_core::checks::stale_accepted(prj, &[]).len(), 1);
    let csv = openecraft_core::checks::issues_csv(prj, &found);
    assert!(csv.starts_with("Severity;Code;Message;Page;Item;Accepted\n"));
    assert!(csv.contains(&first.code()));
}
