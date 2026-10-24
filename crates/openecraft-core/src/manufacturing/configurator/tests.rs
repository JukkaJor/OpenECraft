use super::*;
use crate::sample::demo_project;

#[test]
fn export_lists_components_and_machining() {
    let prj = demo_project();
    let v = export(&prj, 0);
    assert_eq!(v["format"], FORMAT);
    assert_eq!(v["enclosure"], "+CAB1");
    let machining = v["machining"].as_array().unwrap();
    let e = &prj.layout.enclosures[0];
    let holes: usize = Face::ALL.into_iter().map(|f| holes_on(e, &prj.parts, f).len()).sum();
    let opens: usize = Face::ALL.into_iter().map(|f| openings(e, &prj.parts, f).len()).sum();
    assert_eq!(machining.len(), holes + opens);
    assert!(machining.iter().any(|m| m["face"] == "door" && m["kind"] == "cutout"));
}

#[test]
fn import_creates_and_updates_enclosures() {
    let mut prj = demo_project();
    let text = r#"{ "enclosure": "+CAB2", "system": "baying", "size": [800, 2000, 600], "right_open": true,
        "accessories": [{ "article": "GEN.FAN-148", "face": "left", "x": 300, "y": 1600 }],
        "cutouts": [{ "face": "roof", "x": 400, "y": 300, "w": 200, "h": 100 }, { "face": "bottom", "x": 100, "y": 100, "diameter": 25.5 }, { "face": "bottom" }] }"#;
    let r = import(&mut prj, text).unwrap();
    assert!(r.created);
    assert_eq!((r.accessories, r.cutouts, r.skipped.len()), (1, 2, 1));
    let e = &prj.layout.enclosures[1];
    assert_eq!((e.width, e.height, e.depth, e.build.system.as_str()), (800.0, 2000.0, 600.0, "baying"));
    assert!(e.build.right_open);
    let roof = e.items.iter().find(|i| i.face == Face::Roof).unwrap();
    assert_eq!((roof.x, roof.y, roof.w), (300.0, 600.0 - 300.0 - 50.0, 200.0));
    // A second import adds nothing new.
    let again = import(&mut prj, text).unwrap();
    assert!(!again.created && again.accessories + again.cutouts == 0);
    assert!(import(&mut prj, r#"{ "system": "nope" }"#).is_err());
    assert!(import(&mut prj, "[1]").is_err());
    // Export and import give the same enclosure.
    let n = prj.layout.enclosures[1].items.len();
    let out = export(&prj, 1).to_string();
    import(&mut prj, &out).unwrap();
    assert_eq!(prj.layout.enclosures[1].items.len(), n);
}

#[test]
fn smallest_fitting_size() {
    let prj = demo_project();
    let e = &prj.layout.enclosures[0];
    let (w, h, d) = needed(e);
    assert!(w <= e.width + 1e-3 && h <= e.height + 1e-3, "{w} {h} {d}");
    let s = suggest(e, "baying").unwrap();
    assert!(s.0 >= w && s.1 >= h && s.2 >= d);
    let sizes = SYSTEMS.iter().find(|s| s.id() == "baying").unwrap().data.sizes;
    assert!(sizes.iter().all(|z| z.0 * z.1 >= s.0 * s.1 || z.0 < w || z.1 < h || z.2 < d));
    assert!(suggest(e, "terminal_box").is_none());
    assert!(suggest(e, "nope").is_none());
}
