use super::*;

#[test]
fn catalog_numbers_unique_and_accessories_exist() {
    let parts = default_catalog();
    for (i, p) in parts.iter().enumerate() {
        assert!(parts[i + 1..].iter().all(|q| q.number != p.number), "duplicate {}", p.number);
        for a in &p.accessories {
            assert!(parts.iter().any(|q| &q.number == a), "{} accessory {a} missing", p.number);
        }
    }
}

#[test]
fn generated_models_match_mounting_data() {
    let parts = default_catalog();
    let mut with_models = 0;
    for p in &parts {
        match (&p.model_2d, &p.model_3d) {
            (Some(m2), Some(m3)) => {
                with_models += 1;
                let s = m3.size();
                for (got, want) in [(s[0], p.width), (s[1], p.height), (s[2], p.depth)] {
                    assert!((got - want).abs() < 0.01, "{}: model {s:?} vs {} x {} x {}", p.number, p.width, p.height, p.depth);
                }
                assert_eq!((m2.width, m2.height), (p.width, p.height), "{}", p.number);
                assert!(m3.triangles.iter().flatten().all(|&i| (i as usize) < m3.vertices.len()));
            }
            (None, None) => {
                // Only parts without dimensions have no model.
                assert!(p.width == 0.0 || p.height == 0.0 || p.depth == 0.0, "{} has no model", p.number);
            }
            _ => panic!("{}: only one of the models", p.number),
        }
    }
    assert!(with_models >= 25, "{with_models}");
}

#[test]
fn generated_models_keep_existing_models() {
    let mut p = Part { number: "X.1".into(), group: "Relays, contactors".into(), width: 45.0, height: 57.5, depth: 73.0, ..Default::default() };
    add_generated_models(&mut p);
    assert_eq!(p.model_3d.as_ref().unwrap().size(), [45.0, 57.5, 73.0]);
    let own = Model2d { width: 1.0, height: 1.0, prims: Vec::new() };
    let mut q = Part { model_2d: Some(own.clone().into()), ..p.clone() };
    q.model_3d = None;
    add_generated_models(&mut q);
    assert_eq!((q.model_2d, q.model_3d), (Some(own.into()), None));
    let mut none = Part { number: "X.2".into(), ..Default::default() };
    add_generated_models(&mut none);
    assert!(none.model_2d.is_none());
}

#[test]
fn mounting_material_has_models_in_proportion() {
    let parts = default_catalog();
    let mounting: Vec<&Part> = parts.iter().filter(|p| p.group == "Mounting" && p.width > 0.0 && p.height > 0.0 && p.depth > 0.0).collect();
    assert!(mounting.len() > 60, "{}", mounting.len());
    for p in mounting {
        let (m2, m3) = (p.model_2d.as_ref().unwrap(), p.model_3d.as_ref().unwrap());
        // Small enough to be sent with every part and drawn at once.
        assert!(m3.triangles.len() <= 600 && m2.prims.len() <= 400, "{}: {} triangles, {} lines", p.number, m3.triangles.len(), m2.prims.len());
        let inside = |q: [f32; 2]| (-0.01..=p.width + 0.01).contains(&q[0]) && (-0.01..=p.height + 0.01).contains(&q[1]);
        for prim in &m2.prims {
            let ok = match prim {
                Prim2d::Line { a, b } => inside(*a) && inside(*b),
                Prim2d::Rect { min, max, .. } => inside(*min) && inside(*max) && min[0] <= max[0] && min[1] <= max[1],
                Prim2d::Circle { c, r, .. } => inside([c[0] - r, c[1] - r]) && inside([c[0] + r, c[1] + r]),
                Prim2d::Polyline { pts, .. } => pts.iter().all(|q| inside(*q)),
                Prim2d::Text { .. } => true,
            };
            assert!(ok, "{}: {prim:?} outside {} x {}", p.number, p.width, p.height);
        }
    }
    // The shapes differ: a duct is no rail.
    let tris = |n: &str| parts.iter().find(|p| p.number == n).and_then(|p| p.model_3d.as_ref()).map(|m| m.triangles.len());
    assert_ne!(tris("GEN.RAIL-TS35-1M"), tris("GEN.DUCT-40X60"));
}

#[test]
fn the_catalog_has_a_part_for_wall_mounting() {
    let parts = default_catalog();
    let light = parts.iter().find(|p| p.number == "GEN.LIGHT-EM-3W").expect("emergency light");
    assert!(light.wall_mountable() && light.width > 0.0 && light.height > 0.0);
}
