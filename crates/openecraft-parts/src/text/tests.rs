use super::*;

#[test]
fn natural_order() {
    let mut v = vec!["-K10", "-K2", "-F1", "-K1"];
    v.sort_by(|a, b| natural_cmp(a, b));
    assert_eq!(v, vec!["-F1", "-K1", "-K2", "-K10"]);
}
