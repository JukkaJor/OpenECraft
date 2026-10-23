use super::*;

#[test]
fn natural_order() {
    let mut v = vec!["-K10", "-K2", "-F1", "-K1"];
    v.sort_by(|a, b| natural_cmp(a, b));
    assert_eq!(v, vec!["-F1", "-K1", "-K2", "-K10"]);
}

#[test]
fn dates() {
    assert_eq!(date_from_unix(0), "1970-01-01");
    assert_eq!(date_from_unix(951_782_400), "2000-02-29");
    assert_eq!(date_from_unix(1_792_713_600), "2026-10-23");
}
