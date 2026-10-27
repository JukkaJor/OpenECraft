use super::*;
use openecraft_core::model::Project;

fn store(name: &str) -> Store {
    let dir = std::env::temp_dir().join(format!("openecraft-autosave-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    Store::at(dir.display().to_string())
}

fn named(name: &str) -> Project {
    let mut prj = Project::default();
    prj.info.name = name.into();
    prj
}

#[test]
fn the_stored_project_and_parts_replace_the_start_up_ones() {
    let store = store("replace");
    let ctx = egui::Context::default();
    let mut parts = PartsDb::default();
    parts.parts.push(openecraft_parts::Part { number: "P1".into(), ..Default::default() });
    // A first run saves.
    let first = Autosave::start(store.clone(), &ctx, &PartsDb::default());
    let mut session = Session::new(named("stored"), None);
    let mut start_parts = PartsDb::default();
    assert!(!first.save(&session, &parts), "nothing is written before the stored values arrived");
    let mut first = first;
    first.poll(&mut session, &mut start_parts);
    assert!(first.save(&session, &parts));

    // The next run starts with another project and gets the stored one.
    let mut next = Autosave::start(store, &ctx, &PartsDb::default());
    let mut session = Session::new(named("start"), None);
    let mut db = PartsDb::default();
    assert!(next.poll(&mut session, &mut db));
    assert_eq!(session.prj.info.name, "stored");
    assert!(db.parts.iter().any(|p| p.number == "P1"));
}

#[test]
fn values_arriving_after_a_change_do_not_replace_it() {
    let store = store("late");
    let ctx = egui::Context::default();
    let saved = Session::new(named("stored"), None);
    store.store(PROJECT, openecraft_core::archive::pack(&saved.prj).into_bytes());
    store.store(PARTS, b"{}".to_vec());
    let mut autosave = Autosave::start(store, &ctx, &PartsDb::default());
    let mut session = Session::new(named("start"), None);
    session.prj.info.name = "edited".into();
    session.commit(true);
    let mut parts = PartsDb::default();
    parts.parts.push(openecraft_parts::Part { number: "new".into(), ..Default::default() });
    assert!(!autosave.poll(&mut session, &mut parts));
    assert_eq!(session.prj.info.name, "edited");
    assert!(parts.parts.iter().any(|p| p.number == "new"));
}
