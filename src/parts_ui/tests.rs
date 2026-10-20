use super::*;
use crate::sample::demo_project;

fn loaded(purpose: Purpose, name: &str, bytes: &[u8]) -> Loaded {
    Loaded { purpose, name: name.into(), bytes: bytes.to_vec() }
}

#[test]
fn imported_models_go_to_the_requested_part() {
    let mut prj = demo_project();
    let mut db = PartsDb::default();
    let mut ui = PartsUi::default();
    let svg = r#"<svg width="45mm" height="74mm" viewBox="0 0 45 74"><rect x="0" y="0" width="45" height="74"/></svg>"#;
    // Without a requested target nothing happens.
    assert!(ui.apply_model(&loaded(Purpose::Model2d, "front.svg", svg.as_bytes()), &mut db, &mut prj).is_err());

    ui.model_target = Some((Source::Database, "GEN.CT-9A-24DC".into()));
    ui.apply_model(&loaded(Purpose::Model2d, "front.svg", svg.as_bytes()), &mut db, &mut prj).unwrap();
    assert_eq!(db.get("GEN.CT-9A-24DC").unwrap().model_2d.as_ref().unwrap().prims.len(), 1);
    assert!(ui.model_target.is_none());

    // 3D into a project part without mounting data: it takes the model size.
    let obj = "v 0 0 0\nv 30 0 0\nv 30 20 0\nv 0 20 10\nf 1 2 3\nf 1 3 4\n";
    prj.parts.push(Part { number: "NEW.PART".into(), ..Default::default() });
    ui.model_target = Some((Source::Project, "NEW.PART".into()));
    let msg = ui.apply_model(&loaded(Purpose::Model3d, "C:/models/part.obj", obj.as_bytes()), &mut db, &mut prj).unwrap();
    assert!(msg.contains("part.obj"), "{msg}");
    let part = prj.parts.iter().find(|p| p.number == "NEW.PART").unwrap();
    assert_eq!((part.width, part.height, part.depth), (30.0, 20.0, 10.0));
    assert_eq!(part.model_3d.as_ref().unwrap().triangles.len(), 2);

    // Broken files are reported, the part keeps its model.
    ui.model_target = Some((Source::Project, "NEW.PART".into()));
    assert!(ui.apply_model(&loaded(Purpose::Model3d, "bad.stl", b"garbage"), &mut db, &mut prj).is_err());
    assert!(prj.parts.iter().find(|p| p.number == "NEW.PART").unwrap().model_3d.is_some());
}
