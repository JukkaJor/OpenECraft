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
                // Mounting material and parts without dimensions have no model.
                assert!(p.group == "Mounting" || p.width == 0.0 || p.depth == 0.0, "{} has no model", p.number);
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
    let mut q = Part { model_2d: Some(own.clone()), ..p.clone() };
    q.model_3d = None;
    add_generated_models(&mut q);
    assert_eq!((q.model_2d, q.model_3d), (Some(own), None));
    let mut none = Part { number: "X.2".into(), ..Default::default() };
    add_generated_models(&mut none);
    assert!(none.model_2d.is_none());
}
