use super::*;

#[test]
fn every_icon_parses_and_stays_on_the_grid() {
    for icon in Icon::ALL {
        let prims = parse(icon.path()).unwrap_or_else(|| panic!("{icon:?} cannot be read"));
        for prim in prims {
            let pts = match prim {
                Prim::Lines(p, _) | Prim::Fill(p) => p,
                Prim::Circle(c, r, _) => vec![c - vec2(r, r), c + vec2(r, r)],
            };
            for p in pts {
                assert!((-0.5..=16.5).contains(&p.x) && (-0.5..=16.5).contains(&p.y), "{icon:?} leaves the grid at {p:?}");
            }
        }
    }
}

#[test]
fn icons_are_distinct() {
    for (i, a) in Icon::ALL.iter().enumerate() {
        assert_eq!(Icon::ALL.iter().filter(|b| *b == a).count(), 1, "{a:?} twice in ALL");
        for b in &Icon::ALL[i + 1..] {
            assert_ne!(a.path(), b.path(), "{a:?} and {b:?} look the same");
        }
    }
}

#[test]
fn shapes_are_read() {
    assert_eq!(parse("M 0 0 1 1"), Some(vec![Prim::Lines(vec![pos2(0.0, 0.0), pos2(1.0, 1.0)], false)]));
    assert_eq!(parse("D 8 8 2"), Some(vec![Prim::Circle(pos2(8.0, 8.0), 2.0, true)]));
    assert!(matches!(parse("R 1 1 2 2").as_deref(), Some([Prim::Lines(p, true)]) if p.len() == 4));
    assert_eq!(parse("M 0 0 1"), None);
    assert_eq!(parse("X 1 2"), None);
    assert_eq!(parse(""), None);
}
