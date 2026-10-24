use super::*;
use crate::builtin::testing::*;

fn demo() -> Session {
    let mut s = demo_session();
    s.project_mut().unwrap().info.name = "Demo".into();
    s
}

#[test]
fn languages_and_dictionary() {
    let mut s = demo();
    let v = run(&mut s, "set_project_languages", json!({ "source": "EN", "targets": "de pl en" })).unwrap();
    assert_eq!(v["source"], "en");
    assert_eq!(v["targets"].as_array().unwrap().len(), 2);
    assert!(run(&mut s, "set_project_languages", json!({ "targets": "deutsch" })).unwrap_err().contains("deutsch"));
    let c = run(&mut s, "collect_texts", json!({})).unwrap();
    let n = c["added"].as_u64().unwrap();
    assert!(n > 0);
    assert_eq!(c["targets"][0]["untranslated"].as_u64(), Some(n));
    run(&mut s, "set_translation", json!({ "text": "Power supply conveyor motor", "language": "de", "translation": "Einspeisung Förderbandmotor" })).unwrap();
    assert!(run(&mut s, "set_translation", json!({ "text": "x", "language": "en", "translation": "y" })).is_err());
    let missing = run(&mut s, "list_dictionary", json!({ "missing": "de" })).unwrap();
    assert_eq!(missing.as_array().unwrap().len() as u64, n - 1);
    let found = run(&mut s, "list_dictionary", json!({ "query": "förderband" })).unwrap();
    assert_eq!(found[0]["translations"]["de"], "Einspeisung Förderbandmotor");
}
