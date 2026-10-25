use super::*;

#[test]
fn keys_round_trip() {
    for e in [Entry::Symbol("coil".into()), Entry::Macro("Motor star-delta".into()), Entry::Part("3RT2016:1BB41".into())] {
        assert_eq!(Entry::from_key(&e.key()), Some(e));
    }
    assert_eq!(Entry::from_key("symbol:"), None);
    assert_eq!(Entry::from_key("rocket:x"), None);
    assert_eq!(Entry::from_key("nonsense"), None);
}

#[test]
fn recent_entries_come_first_and_are_limited() {
    let mut ic = InsertCenter::default();
    for i in 0..25 {
        ic.used(&Entry::Symbol(format!("s{i}")));
    }
    assert_eq!(ic.recent.len(), RECENT);
    assert_eq!(ic.recent[0], "symbol:s24");
    ic.used(&Entry::Symbol("s10".into()));
    assert_eq!(ic.recent[0], "symbol:s10");
    assert_eq!(ic.recent.iter().filter(|k| *k == "symbol:s10").count(), 1);
}

#[test]
fn favorites_toggle_and_are_exchanged() {
    let mut ic = InsertCenter::default();
    let coil = Entry::Symbol("coil".into());
    ic.toggle_favorite(&coil);
    assert!(ic.is_favorite(&coil));
    ic.toggle_favorite(&Entry::Macro("M1".into()));
    let json = ic.favorites_json();
    ic.toggle_favorite(&coil);
    assert!(!ic.is_favorite(&coil));
    assert_eq!(ic.favorites(), vec![Entry::Macro("M1".into())]);
    assert_eq!(ic.import_favorites(&json), Ok(1), "only the coil is new");
    assert!(ic.import_favorites("{").is_err());
}

#[test]
fn search_uses_all_words_and_placeholders() {
    let mut ic = InsertCenter { query: "con no".into(), ..Default::default() };
    assert!(ic.matches(&["Contact NO", "K"]));
    assert!(!ic.matches(&["Contact NC", "K"]));
    ic.query = "c?il".into();
    assert!(ic.matches(&["Relay coil"]));
    ic.query = String::new();
    assert!(ic.matches(&["anything"]));
}
