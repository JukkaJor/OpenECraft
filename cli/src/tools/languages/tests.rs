use super::*;
use crate::testing::*;

fn demo() -> Session {
    let mut s = session();
    run(&mut s, "new_project", json!({ "name": "Demo", "demo": true })).unwrap();
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

#[test]
fn export_per_language_and_csv_round_trip() {
    let mut s = demo();
    run(&mut s, "set_translation", json!({ "text": "Power supply conveyor motor", "language": "de", "translation": "Einspeisung Förderbandmotor" })).unwrap();
    let page = s.project().unwrap().pages.iter().find(|p| p.description == "Power supply conveyor motor").unwrap().name.clone();
    let svg = run(&mut s, "export", json!({ "format": "svg", "page": page, "language": "de" })).unwrap();
    assert!(svg.as_str().unwrap().contains("Einspeisung Förderbandmotor"));
    assert!(run(&mut s, "export", json!({ "format": "pdf", "language": "it" })).unwrap_err().contains("not a language of the project"));
    let csv = run(&mut s, "export", json!({ "format": "dictionary" })).unwrap();
    assert!(csv.as_str().unwrap().starts_with("en;de\n"));
    let v = run(&mut s, "import_dictionary", json!({ "csv": "en;fr\nPower supply conveyor motor;Alimentation du moteur\n" })).unwrap();
    assert_eq!(v["imported"], 1);
    assert_eq!(v["targets"].as_array().unwrap().len(), 2);
    assert!(run(&mut s, "import_dictionary", json!({})).is_err());
}
