use super::*;

fn font() -> Font<'static> {
    Font::parse(epaint_default_fonts::UBUNTU_LIGHT).expect("the bundled font parses")
}

#[test]
fn glyphs_and_metrics() {
    let f = font();
    assert_eq!(f.units_per_em, 1000);
    assert!(f.num_glyphs > 1000);
    for c in ['A', 'ä', 'Ł', 'ş', 'Ω', 'Ж', '€'] {
        assert_ne!(f.glyph(c), 0, "{c} is in the font");
    }
    assert_eq!(f.glyph('\u{4e2d}'), 0, "no CJK");
    assert!(f.advance(f.glyph('M')) > f.advance(f.glyph('i')));
    assert!(f.ascent > 0 && f.descent < 0 && f.cap_height > 0);
    assert!(f.bbox[2] > f.bbox[0]);
}

#[test]
fn subsets_keep_the_used_outlines() {
    let f = font();
    let used: BTreeSet<u16> = "Łódź Ω".chars().map(|c| f.glyph(c)).collect();
    let sub = f.subset(&used);
    assert!(sub.len() < epaint_default_fonts::UBUNTU_LIGHT.len() / 4, "{} bytes", sub.len());
    let s = Font::parse(&sub).expect("the subset is a font");
    assert_eq!(s.num_glyphs, f.num_glyphs, "glyph ids stay");
    let len = |f: &Font, g: u16| f.glyph_range(g).map(|(a, b)| b - a);
    for g in &used {
        let (kept, original) = (len(&s, *g).unwrap(), len(&f, *g).unwrap());
        assert!(kept >= original && kept < original + 4, "glyph {g} kept (padded to 4 bytes)");
    }
    assert!(used.iter().any(|g| len(&s, *g).unwrap_or(0) > 0));
    let unused = f.glyph('Q');
    assert_eq!(s.glyph_range(unused).map(|(a, b)| b - a), Some(0), "unused glyph emptied");
    assert_eq!(checksum(&sub), 0xB1B0_AFBA, "head checksum adjustment");
    assert_eq!(s.glyph('A'), 0, "a subset has no character map");
}
