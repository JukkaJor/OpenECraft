use super::*;
use crate::write::{write_with_unit, Body};

const TRIS: [[[f32; 3]; 3]; 2] = [[[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 20.0, 0.0]], [[0.0, 0.0, 5.0], [10.0, 0.0, 5.0], [0.0, 20.0, 5.0]]];

fn one_group() -> [Group<'static>; 1] {
    [Group { name: "A", bodies: vec![Body { name: "a", color: [1, 2, 3], transparent: false, triangles: &TRIS }] }]
}

#[test]
fn reads_what_the_writer_wrote() {
    let more = [[[100.0, 0.0, 0.0], [110.0, 0.0, 0.0], [100.0, 10.0, 0.0]]];
    let groups = [
        Group { name: "A", bodies: vec![Body { name: "a", color: [1, 2, 3], transparent: false, triangles: &TRIS }] },
        Group { name: "B", bodies: vec![Body { name: "b", color: [1, 2, 3], transparent: true, triangles: &more }] },
    ];
    let mesh = parse_jt(&write_jt("x", &groups)).unwrap();
    assert_eq!(mesh.triangles.len(), 3);
    assert_eq!(mesh.size(), [110.0, 20.0, 5.0]);
}

#[test]
fn units_are_converted_to_millimetres() {
    for (unit, f) in [("Meters", 1000.0), ("Inches", 25.4), ("Millimeters", 1.0)] {
        let mesh = parse_jt(&write_with_unit("x", &one_group(), unit)).unwrap();
        let s = mesh.size();
        for (k, expected) in [10.0, 20.0, 5.0].into_iter().enumerate() {
            assert!((s[k] - expected * f).abs() < 1e-3, "{unit}: {s:?}");
        }
    }
}

#[test]
fn unsupported_files_are_refused() {
    assert_eq!(parse_jt(b"solid x").unwrap_err(), "not a JT file");
    let mut v10 = format!("{:<80}", "Version 10.0 JT").into_bytes();
    v10.extend([0; 40]);
    assert!(parse_jt(&v10).unwrap_err().contains("JT 10.0"));
    let mut jt = write_jt("x", &one_group());
    jt.truncate(200);
    assert!(parse_jt(&jt).is_err());
    assert!(parse_jt(&write_jt("x", &[])).unwrap_err().contains("no triangle meshes"));
}
