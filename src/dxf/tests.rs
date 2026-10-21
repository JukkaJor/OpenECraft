use super::*;
use crate::sample::demo_project;

/// (group code, value) pairs of a DXF file.
fn pairs(dxf: &str) -> Vec<(i32, &str)> {
    let lines: Vec<&str> = dxf.lines().collect();
    assert_eq!(lines.len() % 2, 0, "odd number of lines");
    lines.chunks(2).map(|c| (c[0].trim().parse().expect("group code"), c[1])).collect()
}

#[test]
fn page_as_dxf() {
    let prj = demo_project();
    let dxf = pages_dxf(&prj, &[1]);
    let p = pairs(&dxf);
    assert_eq!(p.last(), Some(&(0, "EOF")));
    let count = |kind: &str| p.iter().filter(|x| **x == (0, kind)).count();
    assert_eq!(count("SECTION"), 3);
    assert_eq!(count("ENDSEC"), 3);
    assert!(count("LINE") > 50);
    assert!(count("TEXT") > 20);
    assert!(count("CIRCLE") > 0);
    assert_eq!(count("POLYLINE"), count("SEQEND"));
    // Every entity is on one of our layers.
    let layers: Vec<&str> = p.windows(2).filter(|w| w[0].0 == 0 && w[1].0 == 8).map(|w| w[1].1).collect();
    assert!(layers.iter().all(|l| LAYERS.iter().any(|(n, _)| n == l)), "{layers:?}");
    assert!(layers.contains(&"OEC_WIRES") && layers.contains(&"OEC_XREF"));
    // Device tags are texts; y is flipped (page top = PAGE_H).
    assert!(p.contains(&(1, "-Q1")));
    let frame_top = p.windows(2).any(|w| w[0] == (20, "287.000") || w[1] == (20, "287.000"));
    assert!(frame_top);
}

#[test]
fn pages_side_by_side_and_unicode() {
    let prj = demo_project();
    let dxf = pages_dxf(&prj, &[1, 2]);
    let max_x = pairs(&dxf).iter().filter(|(c, _)| *c == 10).filter_map(|(_, v)| v.parse::<f32>().ok()).fold(0.0, f32::max);
    assert!(max_x > PAGE_W + 20.0, "{max_x}");
    assert_eq!(dxf_text("1.5 mm²"), "1.5 mm\\U+00B2");
}
