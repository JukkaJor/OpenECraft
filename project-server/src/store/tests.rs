use super::*;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("openecraft-project-server-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn create_save_and_reload() {
    let dir = temp_dir("reload");
    let mut store = Store::open(&dir).unwrap();
    assert!(store.projects.is_empty());
    let mut prj = Project::default();
    prj.info.name = "Plant".into();
    store.create("Plant 1", prj.clone()).unwrap();
    assert_eq!(store.get("Plant 1").unwrap().revision, 1);
    let first = store.reserve_ids("Plant 1").unwrap();
    assert_eq!(store.get("Plant 1").unwrap().next_id, first + ID_BLOCK);

    // Unchanged project: same revision.
    let same = store.get("Plant 1").unwrap().project.clone();
    assert_eq!(store.save("Plant 1", same).unwrap(), 1);
    prj.info.name = "Plant (changed)".into();
    assert_eq!(store.save("Plant 1", prj).unwrap(), 2);
    assert!(store.save("missing", Project::default()).is_err());

    let reloaded = Store::open(&dir).unwrap();
    let entry = reloaded.get("Plant 1").unwrap();
    assert_eq!(entry.revision, 2);
    assert_eq!(entry.next_id, first + ID_BLOCK);
    assert_eq!(entry.project.info.name, "Plant (changed)");
    // The project file is a normal project file.
    let text = std::fs::read_to_string(dir.join("Plant 1.oec")).unwrap();
    assert_eq!(Project::from_json(&text).unwrap().info.name, "Plant (changed)");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ignores_other_files() {
    let dir = temp_dir("other");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("notes.txt"), "x").unwrap();
    std::fs::write(dir.join(".hidden.oec"), "{}").unwrap();
    let store = Store::open(&dir).unwrap();
    assert!(store.projects.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ids_never_go_below_the_project() {
    let mut store = Store::in_memory();
    let prj = Project { next_id: 500, ..Default::default() };
    store.create("p", prj).unwrap();
    let first = store.reserve_ids("p").unwrap();
    assert!(first >= 500);
    assert!(store.reserve_ids("p").unwrap() >= first + ID_BLOCK);
}
