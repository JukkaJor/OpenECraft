use super::*;

#[test]
fn table_csv_escapes_cells() {
    let mut t = Table::new(["Tag", "Text"]);
    t.push(vec!["-K1".into(), "a;b".into()], Some((0, 1)));
    assert_eq!(t.to_csv(), "Tag;Text\n-K1;\"a;b\"\n");
    assert_eq!(t.targets, vec![Some((0, 1))]);
    assert_eq!(t.widths, vec![1.0, 1.0]);
}

#[test]
fn headers_are_keys_and_match_their_translations() {
    let t = Table::with_widths(&[("Page", 1.0), ("Description", 4.0)]);
    assert_eq!(t.header_keys(), ["Page", "Description"]);
    assert_eq!(t.column("Description"), Some(1));
    assert_eq!(t.column(translate("Description", Lang::De)), Some(1));
    assert_eq!(t.column("Seite"), Some(0));
    assert_eq!(t.column("nothing"), None);
    let owned = Table::new(vec![String::from("Eigene Spalte")]);
    assert_eq!(owned.column("Eigene Spalte"), Some(0));
    assert_eq!(owned.header_texts(), ["Eigene Spalte"]);
    assert_eq!(openecraft_i18n::with_lang(Lang::De, || t.header_text(0).to_string()), translate("Page", Lang::De));
}
