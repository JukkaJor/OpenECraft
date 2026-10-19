use super::*;

#[test]
fn natural_order() {
    let mut v = vec!["-K10", "-K2", "-F1", "-K1"];
    v.sort_by(|a, b| natural_cmp(a, b));
    assert_eq!(v, vec!["-F1", "-K1", "-K2", "-K10"]);
}

#[test]
fn columns() {
    assert_eq!(column_of(p(AREA_X0 + 1.0, 50.0)), 1);
    assert_eq!(column_of(p(FRAME_X1 - 1.0, 50.0)), COLUMNS);
    assert_eq!(row_of(p(100.0, AREA_Y0 + 1.0)), 'A');
}
