use super::*;

#[test]
fn models_are_assigned_to_the_part() {
    let mut part = Part { number: "NEW.PART".into(), ..Default::default() };
    let svg = r#"<svg width="45mm" height="74mm" viewBox="0 0 45 74"><rect x="0" y="0" width="45" height="74"/></svg>"#;
    let msg = apply_model(&mut part, ModelKind::Front2d, "C:\\drawings\\front.svg", svg.as_bytes()).unwrap();
    assert!(msg.starts_with("front.svg"), "{msg}");
    assert_eq!(part.model_2d.as_ref().unwrap().prims.len(), 1);

    // Without mounting data the part takes the model size.
    let obj = "v 0 0 0\nv 30 0 0\nv 30 20 0\nv 0 20 10\nf 1 2 3\nf 1 3 4\n";
    let msg = apply_model(&mut part, ModelKind::Mesh3d, "/models/part.obj", obj.as_bytes()).unwrap();
    assert!(msg.contains("part.obj") && msg.contains("NEW.PART"), "{msg}");
    assert_eq!((part.width, part.height, part.depth), (30.0, 20.0, 10.0));
    assert_eq!(part.model_3d.as_ref().unwrap().triangles.len(), 2);

    // Existing mounting data is kept.
    part.width = 45.0;
    apply_model(&mut part, ModelKind::Mesh3d, "part.obj", obj.as_bytes()).unwrap();
    assert_eq!(part.width, 45.0);

    // Broken files are reported, the part keeps its model.
    assert!(apply_model(&mut part, ModelKind::Mesh3d, "bad.stl", b"garbage").is_err());
    assert!(part.model_3d.is_some());
}

#[test]
fn model_file_filters() {
    assert_eq!(ModelKind::Front2d.filter().1, &["svg"]);
    assert_eq!(ModelKind::Mesh3d.filter().1, &["stl", "obj"]);
}
