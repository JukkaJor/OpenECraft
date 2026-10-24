use super::*;
use crate::sample::demo_project;

#[test]
fn tool_lists() {
    let t = parse_tools("D4.2 T4; M5; F10 T30\nR22,5 T101; Q92x92 T120; Q10").unwrap();
    assert_eq!(t.len(), 6);
    assert_eq!((t[0].kind, t[0].diameter, t[0].number), (ToolKind::Drill, 4.2, 4));
    assert_eq!((t[1].kind, t[1].number), (ToolKind::Tap, 5));
    assert_eq!((t[3].kind, t[3].diameter), (ToolKind::PunchRound, 22.5));
    assert_eq!((t[4].diameter, t[4].height), (92.0, 92.0));
    assert_eq!((t[5].diameter, t[5].height, t[5].number), (10.0, 10.0, 121));
    assert_eq!(parse_tools(&tools_text(&t)).unwrap(), t);
    assert!(parse_tools("X4").is_err());
    assert!(parse_tools("D0").is_err());
    assert!(parse_tools("D4 Tx").is_err());
    assert!(t[4].fits_rect(92.0, 92.0) && !t[4].fits_rect(92.0, 45.0));
}

#[test]
fn builtin_machines_and_project_machines() {
    for m in builtin() {
        let mut numbers: Vec<u32> = m.tools.iter().map(|t| t.number).collect();
        numbers.sort();
        numbers.dedup();
        assert_eq!(numbers.len(), m.tools.len(), "{}", m.id);
        assert!(!m.tools.is_empty(), "{}", m.id);
        assert!(m.id.chars().all(|c| c.is_ascii_lowercase() || c == '_'), "{}", m.id);
    }
    let m = &builtin()[0];
    assert_eq!(m.tool(ToolKind::Drill, 4.25, 0.1).map(|t| t.diameter), Some(4.2));
    assert!(m.tool(ToolKind::Drill, 22.5, 0.1).is_none());
    assert_eq!(m.largest(ToolKind::Mill, 9.0).map(|t| t.diameter), Some(6.0));
    let mut prj = demo_project();
    assert!(find(&prj, DEFAULT_BAR).is_some());
    assert!(get(&prj, "nope").is_err());
    prj.layout.manufacturing.machines.push(Machine { id: DEFAULT_SHEET.into(), name: "Own".into(), ..Default::default() });
    assert_eq!(find(&prj, DEFAULT_SHEET).unwrap().name, "Own");
    assert_eq!(all(&prj).len(), builtin().len());
    assert_eq!(MachineKind::from_key("punching"), Some(MachineKind::Punching));
    assert_eq!(Dialect::from_key("heidenhain").map(Dialect::extension), Some("h"));
}
