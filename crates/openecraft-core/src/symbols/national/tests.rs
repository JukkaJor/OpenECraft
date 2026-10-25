use super::*;
use crate::symbols::{builtin, library};

#[test]
fn every_entry_is_drawn_like_its_iec_symbol() {
    for entries in [GB, GOST, JIS] {
        for &(key, iec, letter, _) in entries {
            let d = builtin(key).unwrap_or_else(|| panic!("{key} missing (no IEC symbol {iec})"));
            let base = builtin(iec).unwrap();
            assert_eq!(d.prims, base.prims, "{key}");
            assert_eq!(d.func, base.func, "{key}");
            assert_eq!(d.pins.len(), base.pins.len(), "{key}");
            assert_eq!(d.letter, letter);
            assert_eq!(iec_equivalent(key), Some(iec));
            assert_eq!(crate::parts::kind_of_symbol(key), crate::parts::kind_of_symbol(iec), "{key}");
        }
    }
}

#[test]
fn each_set_has_the_national_letters() {
    let letters = |cat: Category| library().iter().filter(|d| d.category == cat).map(|d| d.letter).collect::<Vec<_>>();
    for (cat, expected) in [(Category::Gb, ["KM", "QF", "SB", "HL", "FU"]), (Category::Gost, ["KM", "QF", "SB", "HL", "KK"]), (Category::Jis, ["MC", "MCCB", "PBS", "PL", "THR"])] {
        let have = letters(cat);
        assert!(have.len() >= 20, "{}", cat.title());
        for l in expected {
            assert!(have.contains(&l), "{} lacks {l}", cat.title());
        }
    }
}

#[test]
fn placed_national_symbols_are_numbered_with_their_letters() {
    use crate::model::{Page, PageKind, Project};
    let mut prj = Project { pages: vec![Page::new(1, "1", "", PageKind::Schematic)], next_id: 10, ..Default::default() };
    let ctx = crate::ops::PlaceCtx::default();
    let place = |prj: &mut Project, key: &str| {
        let s = crate::ops::new_symbol(prj, 0, &ctx, builtin(key).unwrap(), crate::geom::p(40.0, 40.0), 0, false);
        prj.pages[0].symbols.push(s.clone());
        s.tag
    };
    assert_eq!(place(&mut prj, "gb_coil_km"), "-KM1");
    assert_eq!(place(&mut prj, "gb_coil_km"), "-KM2");
    assert_eq!(place(&mut prj, "jis_cb3"), "-MCCB1");
    assert_eq!(place(&mut prj, "gost_ol3"), "-KK1");
    assert_eq!(place(&mut prj, "gb_term"), "-XT1");
    assert_eq!(place(&mut prj, "term"), "-X1");
}
