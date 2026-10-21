use super::*;

#[test]
fn columns() {
    assert_eq!(column_of(p(AREA_X0 + 1.0, 50.0)), 1);
    assert_eq!(column_of(p(FRAME_X1 - 1.0, 50.0)), COLUMNS);
    assert_eq!(row_of(p(100.0, AREA_Y0 + 1.0)), 'A');
}

#[test]
fn files_without_structure_cables_and_numbers_still_load() {
    let old = r#"{"pages":[{"id":1,"name":"1","kind":"Schematic","wires":[{"id":2,"a":{"x":0,"y":0},"b":{"x":0,"y":8},"color":"","cross_section":""}]}],"next_id":3}"#;
    let prj: Project = serde_json::from_str(old).unwrap();
    assert!(prj.cables.is_empty());
    assert_eq!(prj.wire_numbering, WireNumbering::default());
    assert!(prj.pages[0].function.is_empty() && prj.pages[0].wires[0].number.is_empty());
    // Empty new wire fields are not written.
    let json = serde_json::to_string(&prj.pages[0].wires[0]).unwrap();
    assert!(!json.contains("number") && !json.contains("cable"), "{json}");
}

#[test]
fn next_tag_counts_per_structure_and_cables() {
    let mut prj = Project::default();
    prj.cables.push(Cable { tag: "=A1-W1".into(), ..Default::default() });
    assert_eq!(prj.next_tag("=A1", "W"), "=A1-W2");
    assert_eq!(prj.next_tag("", "W"), "-W1");
    assert_eq!(prj.cable("=A1-W1").map(|c| c.core_list().len()), Some(0));
}

#[test]
fn project_file_round_trip() {
    let prj = crate::sample::demo_project();
    let back = Project::from_json(&prj.to_json()).unwrap();
    assert_eq!(back, prj);
    assert!(Project::from_json("42").unwrap_err().starts_with("Not a valid project file"));
    // Ids are made unique again for hand edited files.
    let mut low = prj.clone();
    low.next_id = 1;
    assert!(Project::from_json(&low.to_json()).unwrap().next_id > 1);
}

#[test]
fn only_unreleased_schematic_pages_are_editable() {
    let mut pg = Page::new(1, "1", "", PageKind::Schematic);
    assert!(pg.is_editable());
    pg.locked = true;
    assert!(!pg.is_editable());
    assert!(!Page::new(2, "2", "", PageKind::Report(ReportKind::DeviceList)).is_editable());
    assert!(!Page::new(3, "3", "", PageKind::Cover).is_editable());
}
