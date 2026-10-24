use super::*;

#[test]
fn registry_lists_all_names() {
    let mut r = Registry::default();
    r.command("a", "A", vec![], |_, _| Ok(Value::Null)).report("b", "B", |_, _| Ok(Table::default())).exporter("c", "C", "txt", |_| Ok(String::new()));
    assert_eq!(r.names().collect::<Vec<_>>(), [("command", "a"), ("report", "b"), ("exporter", "c")]);
}
