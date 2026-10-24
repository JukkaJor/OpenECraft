use super::*;

#[test]
fn built_in_templates() {
    for (name, _) in BUILTIN {
        let t = builtin(name).unwrap_or_else(|| panic!("{name}"));
        assert!(!t.pages.is_empty());
        assert!(!t.potentials.is_empty(), "IEC 60204-1 potentials");
    }
    assert!(builtin("Nothing").is_none());
    let m = builtin("Machine").unwrap();
    assert_eq!(m.pages.iter().filter(|p| p.kind == PageKind::Schematic).count(), 3);
    assert!(m.pages.iter().any(|p| p.kind == PageKind::Cover));
    assert!(m.structures.entry(crate::structure::Aspect::Function, "POWER").is_some(), "identifiers of the structure template");
    let ids: std::collections::BTreeSet<u64> = m.pages.iter().map(|p| p.id).collect();
    assert_eq!(ids.len(), m.pages.len(), "unique page ids");
}

#[test]
fn templates_drop_what_belongs_to_one_project() {
    let mut prj = crate::sample::demo_project();
    prj.pages[1].locked = true;
    prj.wiring_marks.push(Default::default());
    prj.check_settings.accepted.push(Default::default());
    prj.check_settings.disabled.push("emc".into());
    let t = as_template(&prj);
    assert!(t.wiring_marks.is_empty() && t.check_settings.accepted.is_empty() && t.revisions.is_empty());
    assert_eq!(t.check_settings.disabled, ["emc"], "settings stay");
    assert!(t.pages.iter().all(|p| !p.locked));
    let new = from_template(&t, "Line 2", "2026-10-25");
    assert_eq!((new.info.name.as_str(), new.info.date.as_str()), ("Line 2", "2026-10-25"));
    assert_eq!(new.pages.len(), prj.pages.len());
    assert_eq!(new.potentials, prj.potentials);
}
