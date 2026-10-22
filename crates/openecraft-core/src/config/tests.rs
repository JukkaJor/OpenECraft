use super::*;
use crate::macros::{from_page, Macro};
use crate::sample::demo_project;

/// Library with the demo's motor page as page macro "Motor" in two variants and its
/// control page as "Control".
fn library() -> MacroLibrary {
    let prj = demo_project();
    let mut lib = MacroLibrary::default();
    let mut motor = from_page(&prj.pages[1], "Motor");
    motor.page_description = "Motor {NAME}".into();
    lib.upsert_variant(motor.clone(), "");
    let mut star_delta: Macro = motor.clone();
    star_delta.page_description = "Star-delta {NAME}".into();
    lib.upsert_variant(star_delta, "B");
    lib.upsert_variant(from_page(&prj.pages[2], "Control"), "");
    lib
}

fn option(name: &str, choices: &[&str], value: &str) -> ConfigOption {
    ConfigOption { name: name.into(), choices: choices.iter().map(|c| c.to_string()).collect(), value: value.into() }
}

fn rule(m: &str, variant: &str, values: &str, condition: &str) -> Rule {
    Rule { macro_name: m.into(), variant: variant.into(), values: values.into(), condition: condition.into() }
}

#[test]
fn conditions() {
    let cfg = Configuration { options: vec![option("STARTER", &["DOL", "SD"], "SD"), option("LINE", &[], "2")], rules: vec![] };
    assert_eq!(cfg.holds(""), Ok(true));
    assert_eq!(cfg.holds("STARTER=SD"), Ok(true));
    assert_eq!(cfg.holds("STARTER=DOL"), Ok(false));
    assert_eq!(cfg.holds("STARTER!=DOL & LINE=1|2"), Ok(true));
    assert_eq!(cfg.holds("STARTER=SD & LINE=3"), Ok(false));
    assert!(cfg.holds("COLOUR=red").unwrap_err().contains("Unknown option COLOUR"));
    assert!(cfg.holds("STARTER").is_err());
    assert_eq!(parse_values(" NAME = Pump 1 ; X=; =y"), BTreeMap::from([("NAME".into(), "Pump 1".into()), ("X".into(), "".into())]));
}

#[test]
fn generation_follows_the_options_and_replaces_its_pages() {
    let lib = library();
    let mut prj = Project::new_default();
    prj.configuration = Configuration {
        options: vec![option("STARTER", &["DOL", "SD"], "DOL"), option("LINE", &[], "L3")],
        rules: vec![
            rule("Motor", "", "NAME=Conveyor {LINE}", "STARTER=DOL"),
            rule("Motor", "B", "NAME=Conveyor {LINE}", "STARTER=SD"),
            rule("Control", "", "", ""),
        ],
    };
    let g = generate(&mut prj, &lib).unwrap();
    assert_eq!((g.removed, g.pages.len()), (0, 2));
    assert_eq!(prj.pages.len(), 3);
    assert_eq!(prj.pages[1].description, "Motor Conveyor L3");
    assert!(prj.pages[1].generated && prj.pages[2].generated && !prj.pages[0].generated);
    assert!(!prj.pages[1].symbols.is_empty());

    // Other option: the generated pages are replaced, the own page stays.
    prj.configuration.options[0].value = "SD".into();
    let g = generate(&mut prj, &lib).unwrap();
    assert_eq!((g.removed, g.pages.len()), (2, 2));
    assert_eq!(prj.pages.len(), 3);
    assert_eq!(prj.pages[1].description, "Star-delta Conveyor L3");
    // Device tags stay unique over the generated pages.
    let an = crate::reports::Analysis::build(&prj);
    assert!(!an.issues.iter().any(|i| i.message.contains("main function placed")), "{:?}", an.issues.iter().map(|i| &i.message).collect::<Vec<_>>());
}

#[test]
fn problems_stop_the_generation() {
    let lib = library();
    let mut prj = Project::new_default();
    prj.configuration = Configuration { options: vec![option("STARTER", &["DOL", "SD"], "Y-D")], rules: vec![rule("Motor", "C", "", ""), rule("Pump", "", "", "X=1")] };
    let p = problems(&prj.configuration, &lib);
    assert_eq!(p, vec!["Rule 1: Motor has no variant C", "Rule 2: no macro Pump", "Rule 2: Unknown option X", "Option STARTER: Y-D is not one of its choices"]);
    let before = prj.clone();
    assert!(generate(&mut prj, &lib).is_err());
    assert_eq!(prj, before);
    // Released generated pages are not thrown away.
    prj.configuration = Configuration { options: vec![], rules: vec![rule("Control", "", "", "")] };
    generate(&mut prj, &lib).unwrap();
    prj.pages[1].locked = true;
    assert!(generate(&mut prj, &lib).unwrap_err().contains("released"));
}
