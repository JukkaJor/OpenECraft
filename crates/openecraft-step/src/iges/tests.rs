use super::*;
use openecraft_core::export3d;
use openecraft_core::model::Project;
use openecraft_core::panel::Enclosure;

#[test]
fn reads_the_iges_export_of_an_enclosure() {
    let mut prj = Project::default();
    prj.layout.enclosures.push(Enclosure { name: "+A1".into(), width: 600.0, height: 800.0, depth: 250.0, ..Default::default() });
    let text = export3d::iges(&prj, "panel.igs");
    let mesh = parse_iges(text.as_bytes()).unwrap();
    // Walls, door and mounting plate; z up, the door in front (-y).
    let s = mesh.size();
    for (k, expected) in [600.0, 252.0, 800.0].into_iter().enumerate() {
        assert!((s[k] - expected).abs() < 0.01, "{s:?}");
    }
}

#[test]
fn other_files_are_refused() {
    assert!(parse_iges(b"solid x\nendsolid x\n").unwrap_err().contains("IGES"));
    assert!(parse_iges(b"").is_err());
}
