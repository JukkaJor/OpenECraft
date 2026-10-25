use super::*;

#[test]
fn the_first_readable_fonts_are_taken() {
    let read = |p: &str| match p {
        "a" => None,
        "b" => Some(vec![1u8; 2048]),
        "c" => Some(vec![1u8; 10]),
        "d" | "e" => Some(vec![2u8; 4096]),
        _ => None,
    };
    let found = load_first(&["a", "b", "c", "d", "e"], 2, read);
    let paths: Vec<&str> = found.iter().map(|f| f.0.as_str()).collect();
    assert_eq!(paths, ["b", "d"], "missing and truncated files are skipped, at most two");
}

#[test]
fn bundled_and_system_fonts_follow_the_default_fonts() {
    let system = [("sys.ttf".to_string(), Arc::new(FontData::from_static(&[])))];
    let defs = definitions(Lang::Ja, &system);
    let family = &defs.families[&FontFamily::Proportional];
    let default = FontDefinitions::default().families[&FontFamily::Proportional].len();
    assert_eq!(&family[default..], ["bundled:Noto Sans JP", "system:sys.ttf", "bundled:Noto Sans SC", "bundled:Noto Sans KR"]);
    assert!(defs.families[&FontFamily::Monospace].contains(&"bundled:Noto Sans KR".to_string()));
    assert_eq!(definitions(Lang::De, &system).families[&FontFamily::Proportional][default], "system:sys.ttf");
}

#[test]
fn installing_without_system_fonts_changes_nothing() {
    let ctx = egui::Context::default();
    // Whatever the machine has, the count stays within the limit.
    assert!(install_system_fallbacks(&ctx) <= MAX_FALLBACKS);
}
