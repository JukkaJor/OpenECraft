use super::*;
use serde::Deserialize;

#[derive(Debug, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Settings {
    bind: Option<String>,
    data: Option<PathBuf>,
    install_catalogs: Vec<String>,
}

#[test]
fn settings_are_read_by_their_option_names() {
    let s: Settings = parse("bind = \"0.0.0.0:8090\"\n# a comment\ninstall_catalogs = [\"siemens\", \"abb\"]\n").unwrap();
    assert_eq!(s.bind.as_deref(), Some("0.0.0.0:8090"));
    assert_eq!(s.install_catalogs, vec!["siemens", "abb"]);
    assert_eq!(s.data, None, "missing settings keep their defaults");
    assert_eq!(parse::<Settings>("").unwrap(), Settings::default());
}

#[test]
fn misspelt_settings_and_wrong_types_are_refused() {
    let e = parse::<Settings>("bnid = \"0.0.0.0:8090\"\n").unwrap_err();
    assert!(e.contains("bnid"), "{e}");
    assert!(parse::<Settings>("install_catalogs = \"all\"\n").is_err());
}

#[test]
fn files_are_read_and_relative_paths_follow_the_file() {
    let dir = std::env::temp_dir().join(format!("openecraft-config-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("server.toml");
    std::fs::write(&path, "data = \"db/parts.sqlite\"\n").unwrap();
    let s: Settings = load(&path).unwrap();
    assert_eq!(relative_to(&path, s.data.unwrap()), dir.join("db/parts.sqlite"));
    let absolute = std::env::temp_dir().join("elsewhere.sqlite");
    assert_eq!(relative_to(&path, absolute.clone()), absolute);
    assert!(load::<Settings>(&dir.join("missing.toml")).unwrap_err().contains("missing.toml"));
    std::fs::remove_dir_all(&dir).ok();
}
