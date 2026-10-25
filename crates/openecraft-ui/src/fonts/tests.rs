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
fn installing_without_system_fonts_changes_nothing() {
    let ctx = egui::Context::default();
    // Whatever the machine has, the count stays within the limit.
    assert!(install_system_fallbacks(&ctx) <= MAX_FALLBACKS);
}
