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
