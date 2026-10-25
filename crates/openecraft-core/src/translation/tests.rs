use super::*;
use crate::model::ReportKind;
use crate::sample::demo_project;

fn page_of(prj: &Project, kind: ReportKind) -> usize {
    prj.pages.iter().position(|p| p.kind == crate::model::PageKind::Report(kind.into())).unwrap()
}

#[test]
fn codes_are_normalized() {
    assert_eq!(normalize_code(" DE "), Some("de".into()));
    assert_eq!(normalize_code("pt_BR"), Some("pt-br".into()));
    assert_eq!(normalize_code("zh-Hans"), Some("zh-hans".into()));
    assert_eq!(normalize_code("german"), None);
    assert_eq!(normalize_code("d"), None);
    assert_eq!(language_name("de"), "Deutsch (de)");
    assert_eq!(language_name("pl"), "Polski (pl)");
    assert_eq!(language_name("el"), "el");
}

#[test]
fn texts_are_translated_line_by_line() {
    let mut langs = Languages { targets: vec!["de".into()], ..Default::default() };
    langs.dictionary.insert("Motor".into(), [("de".to_string(), "Motor".to_string())].into());
    langs.dictionary.insert("conveyor belt".into(), [("de".to_string(), "Förderband".to_string())].into());
    assert_eq!(langs.translate("Motor\n  conveyor belt\nspare", "de"), "Motor\n  Förderband\nspare");
    assert_eq!(langs.translate("conveyor belt", "en"), "conveyor belt");
    assert_eq!(langs.translate("conveyor belt", "fr"), "conveyor belt");
    assert_eq!(langs.translate("24", "de"), "24");
}

#[test]
fn dictionary_is_collected_and_cleaned() {
    let mut prj = demo_project();
    let lines = source_lines(&prj);
    assert!(lines.contains("Power supply conveyor motor"), "{lines:?}");
    assert_eq!(collect(&mut prj), lines.len());
    assert_eq!(collect(&mut prj), 0);
    prj.languages.targets.push("de".into());
    assert_eq!(missing(&prj, "de").len(), lines.len());
    assert!(missing(&prj, "en").is_empty());
    set(&mut prj, "Power supply conveyor motor", "de", "Einspeisung Förderbandmotor");
    assert_eq!(missing(&prj, "de").len(), lines.len() - 1);
    set(&mut prj, "Power supply conveyor motor", "de", "");
    assert_eq!(missing(&prj, "de").len(), lines.len());
    prj.languages.dictionary.insert("old text".into(), BTreeMap::new());
    assert_eq!(remove_unused(&mut prj), 1);
    let an = crate::analysis::Analysis::checked(&prj);
    assert!(an.issues.iter().any(|i| i.message().contains("not translated into Deutsch")));
}

#[test]
fn output_is_generated_in_the_target_language() {
    let mut prj = demo_project();
    prj.languages.targets = vec!["de".into(), "el".into()];
    set(&mut prj, "Power supply conveyor motor", "de", "Einspeisung Förderbandmotor");
    set(&mut prj, "Power supply conveyor motor", "el", "Τροφοδοσία κινητήρα μεταφορέα");
    let page = prj.pages.iter().position(|p| p.description == "Power supply conveyor motor").unwrap();
    let de = with_language(&prj, "de", |p| crate::svg::page_svg(p, page));
    assert!(de.contains("Einspeisung Förderbandmotor"));
    // Generated texts follow the language; the project itself is unchanged.
    let report = page_of(&prj, ReportKind::DeviceList);
    let title = openecraft_i18n::translate("Device list", Lang::De);
    assert_ne!(title, "Device list");
    assert!(with_language(&prj, "de", |p| crate::svg::page_svg(p, report)).contains(title));
    assert!(with_language(&prj, "el", |p| crate::svg::page_svg(p, report)).contains("Device list"));
    assert_eq!(prj.pages[page].description, "Power supply conveyor motor");
    assert!(with_language(&prj, "en", |p| crate::svg::page_svg(p, page)).contains("Power supply conveyor motor"));
    let pdf = with_language(&prj, "de", crate::pdf::document);
    assert!(pdf.starts_with("%PDF"));
}

#[test]
fn csv_round_trip_for_translators() {
    let mut prj = demo_project();
    prj.languages.targets = vec!["de".into()];
    collect(&mut prj);
    set(&mut prj, "Power supply conveyor motor", "de", "Einspeisung; Motor");
    let csv = to_csv(&prj);
    assert!(csv.starts_with("en;de\n"));
    assert!(csv.contains("Power supply conveyor motor;\"Einspeisung; Motor\""));
    let mut other = demo_project();
    let text = format!("{csv}\"Power supply conveyor motor\";;Alimentation moteur\n").replace("en;de\n", "en;de;FR\n");
    let n = from_csv(&mut other, &text).unwrap();
    assert_eq!(n, 2);
    assert_eq!(other.languages.targets, ["de", "fr"]);
    assert_eq!(other.languages.lookup("Power supply conveyor motor", "de"), Some("Einspeisung; Motor"));
    assert_eq!(other.languages.lookup("Power supply conveyor motor", "fr"), Some("Alimentation moteur"));
    assert!(from_csv(&mut other, "en;Deutsch\n").unwrap_err().contains("Deutsch"));
    assert!(from_csv(&mut other, "").is_err());
}
