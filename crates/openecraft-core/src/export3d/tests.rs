use super::*;
use crate::sample::demo_project;
use std::collections::HashSet;

#[test]
fn the_scene_has_walls_door_plate_and_items() {
    let prj = demo_project();
    let s = scene(&prj);
    assert_eq!(s.len(), prj.layout.enclosures.len());
    let e = &prj.layout.enclosures[0];
    let solids = &s[0].solids;
    let parts = panel::components(e).iter().filter(|c| !c.boxes.is_empty()).count();
    assert_eq!(parts, 7, "door, five walls, mounting plate");
    assert_eq!(solids.len(), parts + e.items.iter().filter(|i| i.kind != ItemKind::Cutout).count());
    assert!(solids.iter().any(|x| x.transparent && x.name.ends_with("door")));
    assert!(solids.iter().any(|x| x.name == "-K1"));
    // Everything lies inside the enclosure (the door in front of it).
    for x in solids {
        for v in x.triangles.iter().flatten() {
            assert!(v[0] >= -0.01 && v[0] <= e.width + 0.01 && v[1] >= -0.01 && v[1] <= e.height + 0.01, "{} {v:?}", x.name);
            assert!(v[2] >= -0.01 && v[2] <= e.depth + crate::panel::WALL + 0.01, "{} {v:?}", x.name);
        }
    }
}

#[test]
fn base64_and_step_reals() {
    assert_eq!(base64(b"Man"), "TWFu");
    assert_eq!(base64(b"Ma"), "TWE=");
    assert_eq!(base64(b"M"), "TQ==");
    assert_eq!(real(12.0), "12.");
    assert_eq!(real(-0.5), "-0.5");
    assert_eq!(real(1e-9), "0.");
}

#[test]
fn gltf_is_consistent() {
    let prj = demo_project();
    let text = gltf(&prj);
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["asset"]["version"], "2.0");
    let buffer = &v["buffers"][0];
    let len = buffer["byteLength"].as_u64().unwrap() as usize;
    let uri = buffer["uri"].as_str().unwrap();
    let data = uri.strip_prefix("data:application/octet-stream;base64,").unwrap();
    assert_eq!(data.len(), len.div_ceil(3) * 4);
    let views = v["bufferViews"].as_array().unwrap();
    let total: u64 = views.iter().map(|b| b["byteLength"].as_u64().unwrap()).sum();
    assert_eq!(total as usize, len);
    for a in v["accessors"].as_array().unwrap() {
        let view = &views[a["bufferView"].as_u64().unwrap() as usize];
        assert_eq!(a["count"].as_u64().unwrap() * 12, view["byteLength"].as_u64().unwrap());
    }
    // A root node per enclosure, a mesh node per solid.
    let solids: usize = scene(&prj).iter().map(|e| e.solids.len()).sum();
    assert_eq!(v["meshes"].as_array().unwrap().len(), solids);
    assert_eq!(v["scenes"][0]["nodes"].as_array().unwrap().len(), prj.layout.enclosures.len());
    assert!(v["materials"].as_array().unwrap().iter().any(|m| m["alphaMode"] == "BLEND"));
}

#[test]
fn step_references_only_defined_entities() {
    let prj = demo_project();
    let text = step(&prj, "panel.stp");
    assert!(text.starts_with("ISO-10303-21;\nHEADER;") && text.ends_with("END-ISO-10303-21;\n"));
    assert!(text.contains("FILE_SCHEMA(('AUTOMOTIVE_DESIGN"));
    let mut defined = HashSet::new();
    let mut used = HashSet::new();
    for line in text.lines().filter(|l| l.starts_with('#')) {
        let (id, body) = line.split_once('=').unwrap();
        assert!(defined.insert(id.to_string()), "{id} twice");
        let mut rest = body;
        while let Some(i) = rest.find('#') {
            let digits: String = rest[i + 1..].chars().take_while(|c| c.is_ascii_digit()).collect();
            used.insert(format!("#{digits}"));
            rest = &rest[i + 1 + digits.len()..];
        }
        assert!(line.ends_with(';'));
    }
    let missing: Vec<_> = used.difference(&defined).collect();
    assert!(missing.is_empty(), "{missing:?}");
    let solids: usize = scene(&prj).iter().map(|e| e.solids.iter().filter(|s| !s.triangles.is_empty()).count()).sum();
    assert_eq!(text.matches("=FACETED_BREP(").count(), solids);
    assert_eq!(text.matches("=STYLED_ITEM(").count(), solids);
    assert!(text.contains("FACETED_BREP('-K1'"));
}

#[test]
fn empty_layouts_export_empty_files() {
    let prj = Project::default();
    let v: serde_json::Value = serde_json::from_str(&gltf(&prj)).unwrap();
    assert_eq!(v["meshes"].as_array().unwrap().len(), 0);
    assert!(step(&prj, "x.stp").contains("FACETED_BREP_SHAPE_REPRESENTATION"));
}

#[test]
fn floor_standing_system_with_frame_and_plinth() {
    let mut prj = demo_project();
    panel::apply_system(&mut prj.layout.enclosures[0], "baying", Some((800.0, 2000.0, 600.0)));
    let s = scene(&prj);
    let solids = &s[0].solids;
    let plinth = solids.iter().find(|x| x.name.contains("plinth")).unwrap();
    // y up: the plinth is below the enclosure.
    assert!(plinth.triangles.iter().flatten().all(|v| v[1] <= 0.01 && v[1] >= -100.01));
    assert!(solids.iter().any(|x| x.name.contains("GEN.TS-800X2000X600")));
    assert!(solids.iter().any(|x| x.name.contains("GEN.TS-SP-2000X600")));
}
