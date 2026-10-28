use super::*;

fn id(n: u32) -> egui::Id {
    egui::Id::unique(("test_dialog", n))
}

#[test]
fn esc_closes_the_dialog_on_top() {
    let (a, b, pane) = (id(1), id(2), id(3));
    assert_eq!(topmost(&[a, pane, b], &[a, b]), Some(b));
    // A dialog clicked last is moved to the top of the window order.
    assert_eq!(topmost(&[b, pane, a], &[a, b]), Some(a));
    // Not ordered yet (opened in the last frame): the one opened last.
    assert_eq!(topmost(&[pane], &[a, b]), Some(b));
    assert_eq!(topmost(&[a, b], &[]), None);
}

#[test]
fn esc_goes_to_one_dialog_only() {
    let (a, b) = (id(1), id(2));
    let mut d = Dialogs::default();
    assert!(!d.begin(true, &[]), "no dialog: Esc stays with the editor");
    assert!(!d.shown(a) && !d.shown(b));
    assert!(d.opens(a), "a was not shown before this frame");
    assert!(d.begin(true, &[b, a]));
    assert!(!d.opens(a));
    assert!(d.shown(a), "Esc closes a");
    assert!(!d.shown(b), "b stays open");
    // a was closed: the next Esc goes to b.
    assert!(d.begin(true, &[b, a]));
    assert!(d.shown(b));
    assert!(!d.begin(false, &[b]));
    assert!(d.opens(b), "b was closed by the Esc before");
}
