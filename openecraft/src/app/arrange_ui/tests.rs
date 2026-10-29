use super::*;

#[test]
fn move_by_has_its_own_distance_of_zero() {
    let st = ArrangeUi::default();
    // A reflexive Move must not shift the selection: the copies' distance is separate.
    assert!(!moves(st.move_by));
    assert!(moves(st.copy_by));
    assert!(moves((0.0, -4.0)) && moves((8.0, 0.0)));
}
