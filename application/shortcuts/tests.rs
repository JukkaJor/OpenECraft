use super::*;

#[test]
fn shortcuts_parse_and_print() {
    let s = Shortcut::parse("Ctrl+Shift+Z").unwrap();
    assert!(s.command && s.shift && !s.alt && s.key == Key::Z);
    assert_eq!(s.text(), "Ctrl+Shift+Z");
    assert_eq!(Shortcut::parse("PageDown").unwrap().text(), "PageDown");
    assert_eq!(Shortcut::parse("alt + F1").unwrap().text(), "Alt+F1");
    assert_eq!(Shortcut::parse("Hyper+Z"), None);
    assert_eq!(Shortcut::parse("Ctrl+"), None);
    for a in Action::ALL {
        for k in a.defaults() {
            assert_eq!(Shortcut::parse(&k.text()), Some(k), "{a:?}");
        }
    }
}

#[test]
fn modifiers_must_match_exactly() {
    let undo = Shortcut::parse("Ctrl+Z").unwrap();
    assert!(undo.held(Modifiers::COMMAND));
    assert!(!undo.held(Modifiers::COMMAND | Modifiers::SHIFT), "Ctrl+Shift+Z is redo, not undo");
    assert!(!Shortcut::parse("R").unwrap().held(Modifiers::COMMAND));
}

#[test]
fn defaults_have_no_conflicts_and_ids_are_unique() {
    assert!(Shortcuts::default().conflicts().is_empty());
    let ids: std::collections::BTreeSet<&str> = Action::ALL.iter().map(|a| a.id()).collect();
    assert_eq!(ids.len(), Action::ALL.len());
}

#[test]
fn changed_keys_are_stored_and_conflicts_found() {
    let mut s = Shortcuts::default();
    s.set(Action::Rotate, vec![Shortcut::parse("Ctrl+R").unwrap()]);
    s.set(Action::TextTool, vec![Shortcut::parse("W").unwrap()]);
    s.set(Action::Autoroute, vec![]);
    assert_eq!(s.text(Action::Rotate), "Ctrl+R");
    assert_eq!(s.text(Action::Autoroute), "");
    assert_eq!(s.text(Action::Redo), "Ctrl+Y / Ctrl+Shift+Z");
    let conflicts = s.conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].1, vec![Action::WireTool, Action::TextTool]);
    let stored = s.to_storage();
    assert_eq!(stored.len(), 3);
    assert_eq!(Shortcuts::from_storage(&stored), s);
    s.set(Action::Rotate, Action::Rotate.defaults());
    assert!(s.is_default(Action::Rotate), "setting the defaults again removes the change");
    s.reset_all();
    assert_eq!(s, Shortcuts::default());
}

#[test]
fn unknown_stored_entries_are_ignored() {
    let stored: BTreeMap<String, Vec<String>> = [("teleport".to_string(), vec!["X".to_string()]), ("rotate".to_string(), vec!["Bogus+R".to_string(), "E".to_string()])].into();
    let s = Shortcuts::from_storage(&stored);
    assert_eq!(s.text(Action::Rotate), "E");
}
