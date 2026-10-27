use super::*;

#[test]
fn recently_used_commands_come_first_and_are_limited() {
    let mut p = PluginCommands::default();
    for i in 0..10 {
        p.used(&format!("cmd_{i}"));
    }
    p.used("cmd_5");
    assert_eq!(p.recent.len(), RECENT);
    assert_eq!(p.recent[0], "cmd_5");
    assert_eq!(p.recent.iter().filter(|r| *r == "cmd_5").count(), 1);
}

#[test]
fn favorites_are_toggled_and_left_out_of_the_recent_ones() {
    let mut p = PluginCommands::default();
    p.used("set_wire_defaults");
    p.used("renumber_pages");
    p.toggle_favorite("renumber_pages");
    assert!(p.is_favorite("renumber_pages"));
    assert_eq!(p.recent_only(), vec!["set_wire_defaults".to_string()]);
    p.toggle_favorite("renumber_pages");
    assert!(!p.is_favorite("renumber_pages"));
    assert_eq!(p.recent_only().len(), 2);
}

#[test]
fn search_and_pick() {
    let mut p = PluginCommands { query: "wire def*".into(), ..Default::default() };
    assert!(p.matches(&["Set wire defaults", "Potentials"]));
    assert!(!p.matches(&["Renumber pages", "Pages"]));
    p.query.clear();
    assert!(p.matches(&["anything"]));

    let all = ["a", "b", "c"];
    let names = vec!["c".to_string(), "gone".to_string(), "a".to_string()];
    assert_eq!(pick(&names, &all, |s| s), vec![&"c", &"a"]);
}
