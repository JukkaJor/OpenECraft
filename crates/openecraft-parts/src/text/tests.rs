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
    assert_eq!(unix_from_date("1970-01-01"), Some(0));
    assert_eq!(unix_from_date("2000-02-29"), Some(951_782_400));
    assert_eq!(unix_from_date("2026-10-23T01:02:03+02:00"), Some(1_792_713_600 + 3723));
    assert_eq!(unix_from_date("2026-10-23 01:02:03"), Some(1_792_713_600 + 3723));
    assert_eq!(unix_from_date("23.10.2026"), None);
    assert_eq!(unix_from_date("2026-13-01"), None);
    assert_eq!(unix_from_date(""), None);
}
