use super::*;

#[test]
fn clones_share_the_allocation_and_compare_by_pointer_first() {
    let a: Shared<Vec<f32>> = Shared::new(vec![1.0, 2.0]);
    let b = a.clone();
    assert!(a.ptr_eq(&b));
    assert_eq!(a, b);
    // Equal content in another allocation is still equal (NaN aside).
    let c: Shared<Vec<f32>> = vec![1.0, 2.0].into();
    assert!(!a.ptr_eq(&c));
    assert_eq!(a, c);
    assert_ne!(a, Shared::new(vec![1.0]));
}

#[test]
fn make_mut_copies_only_when_shared() {
    let mut a = Shared::new(String::from("x"));
    let b = a.clone();
    a.make_mut().push('y');
    assert_eq!(*a, "xy");
    assert_eq!(*b, "x");
    assert!(!a.ptr_eq(&b));
}

#[test]
fn json_form_is_the_value_itself() {
    let s: Shared<String> = Shared::new("data".into());
    assert_eq!(serde_json::to_string(&s).unwrap(), "\"data\"");
    let back: Shared<String> = serde_json::from_str("\"data\"").unwrap();
    assert_eq!(back, s);
    assert_eq!(s.to_string(), "data");
    assert_eq!(s.len(), 4);
}
