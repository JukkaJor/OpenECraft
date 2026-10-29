use super::*;

#[test]
fn device_tag_field_applies_a_complete_tag_once() {
    let here = Dt { function: "A1".into(), ..Default::default() };
    // Typed relative to the page: the page structure completes it.
    assert_eq!(tag_edit("-K2", "=A1-K1", &here, false), TagEdit::Move("=A1-K2".into()));
    assert_eq!(tag_edit(" -K2 ", "=A1-K1", &here, false), TagEdit::Move("=A1-K2".into()));
    // The same device, Esc and an empty field change nothing.
    assert_eq!(tag_edit("-K1", "=A1-K1", &here, false), TagEdit::Keep);
    assert_eq!(tag_edit("-K2", "=A1-K1", &here, true), TagEdit::Keep);
    assert_eq!(tag_edit("", "=A1-K1", &here, false), TagEdit::Keep);
    assert_eq!(tag_edit("  ", "", &here, false), TagEdit::Keep);
    // Another structure typed in full is kept.
    assert_eq!(tag_edit("=B2-K1", "=A1-K1", &here, false), TagEdit::Move("=B2-K1".into()));
}

#[test]
fn device_tag_note_warns_of_an_empty_tag() {
    let here = Dt::default();
    let idx = Index::default();
    assert!(tag_note_of("", "-K1", &here, &idx).is_some_and(|(_, error)| error));
    assert!(tag_note_of("-K1", "-K1", &here, &idx).is_none());
    assert!(tag_note_of("-K9", "-K1", &here, &idx).is_some_and(|(_, error)| !error));
}
