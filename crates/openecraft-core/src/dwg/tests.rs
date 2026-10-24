use super::*;
use crate::panel::Face;
use crate::sample::demo_project;

fn k1_page(prj: &Project) -> usize {
    prj.pages.iter().position(|p| p.symbols.iter().any(|s| s.tag == "-K1")).unwrap()
}

#[test]
fn pages_as_dwg() {
    let demo = demo_project();
    let dwg = pages_dwg(&demo, &[k1_page(&demo)]).unwrap();
    assert!(dwg.starts_with(b"AC1018"));
    assert!(is_dwg(&dwg));
    let dxf = to_dxf(&dwg).unwrap();
    assert!(dxf.contains("\nOEC_WIRES\n") && dxf.contains("\nOEC_FRAME\n"));
}

#[test]
fn exported_pages_come_back() {
    let demo = demo_project();
    let page = k1_page(&demo);
    let mut prj = Project::default();
    let from_dxf = dxf_import::import_page(&mut prj, &crate::dxf::pages_dxf(&demo, &[page]), "x", 0, DxfOptions::default()).unwrap();
    let mut prj = Project::default();
    let r = import_page(&mut prj, &pages_dwg(&demo, &[page]).unwrap(), "Old drawing", 0, DxfOptions::default()).unwrap();
    assert_eq!((r.lines, r.texts), (from_dxf.lines, from_dxf.texts), "{r:?}");
    assert!((r.scale - from_dxf.scale).abs() < 1e-3);
    assert_eq!(prj.pages[r.page].description, "Old drawing");
    assert!(prj.pages[r.page].texts.iter().any(|t| t.text.contains("-K1")));
}

#[test]
fn nc_data_as_dwg() {
    let demo = demo_project();
    let dxf = to_dxf(&nc_dwg(&demo, 0, Face::Plate).unwrap()).unwrap();
    assert!(dxf.contains("\nPLATE\n") && dxf.contains("HOLE_D"));
}

#[test]
fn drawings_are_told_apart_by_content() {
    let demo = demo_project();
    let page = k1_page(&demo);
    let mut prj = Project::default();
    let a = import_drawing(&mut prj, &pages_dwg(&demo, &[page]).unwrap(), "a", 0, DxfOptions::default()).unwrap();
    let b = import_drawing(&mut prj, crate::dxf::pages_dxf(&demo, &[page]).as_bytes(), "b", 0, DxfOptions::default()).unwrap();
    assert_eq!(a.lines, b.lines);
    assert_eq!(prj.pages.len(), 3);
}

#[test]
fn files_that_are_no_dwg_are_refused() {
    assert!(!is_dwg(b"0\nSECTION\n"));
    assert!(!is_dwg(b"AC1"));
    assert!(to_dxf(b"hello").unwrap_err().contains("DWG"));
    assert!(to_dxf(b"AC1018 broken").is_err());
    let mut prj = Project::default();
    assert!(import_page(&mut prj, b"AC1015\0\0\0\0\0", "x", 0, DxfOptions::default()).is_err());
    assert_eq!(prj.pages.len(), 1);
}
