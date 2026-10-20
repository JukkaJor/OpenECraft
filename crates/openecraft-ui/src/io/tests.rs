use super::*;

#[test]
fn short_names_and_mime_types() {
    assert_eq!(short_name("C:\\models\\part.obj"), "part.obj");
    assert_eq!(short_name("/home/x/front.svg"), "front.svg");
    assert_eq!(short_name("plain.csv"), "plain.csv");
    assert_eq!(mime_type("csv"), "text/csv");
    assert_eq!(mime_type("json"), "application/json");
}

#[test]
fn loaded_text_is_lossy() {
    let l = Loaded { purpose: (), name: "a".into(), bytes: vec![b'o', b'k', 0xff] };
    assert!(l.text().starts_with("ok"));
}
