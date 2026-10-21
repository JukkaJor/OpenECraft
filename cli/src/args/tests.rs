use super::*;

fn argv(s: &str) -> Vec<String> {
    s.split_whitespace().map(String::from).collect()
}

#[test]
fn options_and_parameters_in_any_order() {
    let inv = parse(&argv("--project a.oec rename_device --old -K1 --new=-K2 --server http://x -p b.oec --discard")).unwrap();
    assert_eq!(inv.command.as_deref(), Some("rename_device"));
    assert_eq!(inv.options.project.as_deref(), Some("b.oec"));
    assert_eq!(inv.options.server.as_deref(), Some("http://x"));
    assert_eq!(
        inv.params,
        vec![("old".into(), Some("-K1".into())), ("new".into(), Some("-K2".into())), ("discard".into(), None)]
    );
}

#[test]
fn dashes_in_parameter_names_become_underscores() {
    let inv = parse(&argv("place_symbol --function-text Motor")).unwrap();
    assert_eq!(inv.params, vec![("function_text".into(), Some("Motor".into()))]);
}

#[test]
fn help_and_further_words() {
    let inv = parse(&argv("help add_wire")).unwrap();
    assert_eq!(inv.command.as_deref(), Some("help"));
    assert_eq!(inv.words, vec!["add_wire".to_string()]);
    assert!(parse(&argv("check -h")).unwrap().options.help);
}

#[test]
fn options_need_a_value() {
    assert!(parse(&argv("check --project")).is_err());
    assert!(parse(&argv("check --token --project a.oec")).is_err());
}
