use super::*;

#[test]
fn table_csv_escapes_cells() {
    let mut t = Table::new(["Tag", "Text"]);
    t.push(vec!["-K1".into(), "a;b".into()], Some((0, 1)));
    assert_eq!(t.to_csv(), "Tag;Text\n-K1;\"a;b\"\n");
    assert_eq!(t.targets, vec![Some((0, 1))]);
}

#[test]
fn registry_lists_all_names() {
    let mut r = Registry::default();
    r.command("a", "A", vec![], |_, _| Ok(Value::Null)).report("b", "B", |_, _| Ok(Table::default())).exporter("c", "C", "txt", |_| Ok(String::new()));
    assert_eq!(r.names().collect::<Vec<_>>(), [("command", "a"), ("report", "b"), ("exporter", "c")]);
}

#[test]
fn built_in_tables_convert() {
    let t = reports::Table { headers: vec!["Tag"], rows: vec![vec!["-K1".into()]], targets: vec![None], ..Default::default() };
    assert_eq!(Table::from(t).to_csv(), "Tag\n-K1\n");
}
