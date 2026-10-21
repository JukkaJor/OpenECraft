use super::*;

#[test]
fn rotate_quarter_turns() {
    let top = p(0.0, -4.0);
    assert!(top.transform(1, false).same(p(4.0, 0.0)));
    assert!(top.transform(2, false).same(p(0.0, 4.0)));
    assert!(top.transform(3, false).same(p(-4.0, 0.0)));
    assert!(top.transform(4, false).same(top));
    assert!(p(2.0, 1.0).transform(0, true).same(p(-2.0, 1.0)));
}

#[test]
fn segment_interior() {
    assert!(on_segment_interior(p(4.0, 0.0), p(0.0, 0.0), p(8.0, 0.0)));
    assert!(!on_segment_interior(p(8.0, 0.0), p(0.0, 0.0), p(8.0, 0.0)));
    assert!(!on_segment_interior(p(4.0, 1.0), p(0.0, 0.0), p(8.0, 0.0)));
}
