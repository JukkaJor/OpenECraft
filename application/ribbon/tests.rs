use super::*;

#[test]
fn tabs_and_groups_have_unique_keys_and_tips() {
    let mut keys = std::collections::BTreeSet::new();
    let mut tips = std::collections::BTreeSet::new();
    for t in TABS {
        assert!(keys.insert(t.key), "tab {} twice", t.key);
        assert!(tips.insert(t.key_tip), "key tip {} twice", t.key_tip);
        let mut groups = std::collections::BTreeSet::new();
        for g in t.groups {
            assert!(groups.insert(g.id), "group {}/{} twice", t.key, g.id);
            assert!(!g.items.is_empty(), "group {}/{} is empty", t.key, g.id);
        }
    }
    assert!(!tips.contains(&'F'), "F is the key tip of the File tab");
    assert_eq!(tab(TabId::Plugins).groups.len(), 0, "the plugin tab is filled from the registry");
}

#[test]
fn context_tabs_are_the_layout_space_and_the_wiring() {
    let ctx: Vec<TabId> = TABS.iter().filter(|t| t.context).map(|t| t.id).collect();
    assert_eq!(ctx, vec![TabId::LayoutSpace, TabId::Wiring]);
}

#[test]
fn the_backstage_offers_files_and_settings() {
    for c in [Cmd::Save, Cmd::SaveAs, Cmd::Settings, Cmd::Quit, Cmd::Open, Cmd::NewProject, Cmd::PrintAll, Cmd::ImportDrawing] {
        assert!(places(c).iter().any(|p| p.starts_with("backstage")), "{c:?}");
    }
    assert!(places(Cmd::Undo).contains(&"quick".to_string()));
}

#[test]
fn key_tips_take_free_letters_of_the_titles() {
    let titles: Vec<String> = ["Paste", "Cut", "Copy", "Paste special", "?"].iter().map(|s| s.to_string()).collect();
    let tips = key_tips(&titles, &['F']);
    assert_eq!(tips[0], 'P');
    assert_eq!(tips[1], 'C');
    assert_eq!(tips[2], 'O', "C is taken, the next letter of Copy");
    assert_eq!(tips[3], 'A');
    assert!(tips[4].is_ascii_alphanumeric());
    let unique: std::collections::BTreeSet<char> = tips.iter().copied().collect();
    assert_eq!(unique.len(), tips.len());
    assert!(!tips.contains(&'F'));
}

#[test]
fn customisation_hides_groups_and_adds_own_ones() {
    let home = tab(TabId::Home);
    let mut c = Custom::default();
    c.set_hidden(home, &home.groups[0], true);
    assert!(c.is_hidden(home, &home.groups[0]));
    c.groups.push(CustomGroup { tab: "home".into(), title: "Mine".into(), commands: vec!["save".into(), "nonsense".into(), "wire_tool".into()] });
    let own = c.groups_of(home);
    assert_eq!(own.len(), 1);
    assert_eq!(own[0].2, vec![Cmd::Save, Cmd::WireTool]);
    let back = Custom::from_json(&c.to_json()).unwrap();
    assert_eq!(back.groups[0].commands, vec!["save".to_string(), "wire_tool".to_string()], "unknown commands are dropped");
    assert!(back.is_hidden(home, &home.groups[0]));
    c.set_hidden(home, &home.groups[0], false);
    assert!(!c.is_hidden(home, &home.groups[0]));
    assert!(Custom::from_json("not json").is_err());
    let foreign = r#"{"hidden":["mars/rover"],"groups":[{"tab":"mars","title":"x","commands":["save"]}]}"#;
    let f = Custom::from_json(foreign).unwrap();
    assert!(f.hidden.is_empty() && f.groups.is_empty());
}
