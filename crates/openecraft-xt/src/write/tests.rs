use super::*;
use parasolid_core::brep::{map_xt_brep, BodyKind, BrepModel};
use parasolid_core::{parse_xt, BuiltinProfileRegistry, DocumentLimits, SchemaKey};

/// The twelve triangles of a box (outward normals).
fn cuboid(a: [f32; 3], b: [f32; 3]) -> Vec<Tri> {
    let p = |i: usize| [if i & 1 == 0 { a[0] } else { b[0] }, if i & 2 == 0 { a[1] } else { b[1] }, if i & 4 == 0 { a[2] } else { b[2] }];
    let quads = [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [2, 6, 7, 3], [0, 4, 6, 2], [1, 3, 7, 5]];
    quads.iter().flat_map(|q| [[p(q[0]), p(q[1]), p(q[2])], [p(q[0]), p(q[2]), p(q[3])]]).collect()
}

/// Reads the file back with an independent XT parser and checks its topology.
fn read(text: &str) -> BrepModel {
    let key = SchemaKey::parse(SCHEMA).unwrap();
    let registry = BuiltinProfileRegistry::compiled().unwrap();
    let provider = registry.provider_for_key(&key).unwrap();
    let document = parse_xt(text.as_bytes(), &provider, DocumentLimits::default()).unwrap();
    let model = map_xt_brep(&document).unwrap();
    assert!(model.topology.valid, "{:?}\n{:?}", model.topology, model.diagnostics);
    assert!(model.complete, "{:?}", model.diagnostics);
    model
}

#[test]
fn a_box_is_a_solid_with_six_faces() {
    let text = write_xt(&[&cuboid([0.0, 0.0, 0.0], [600.0, 400.0, 200.0])]);
    assert!(text.lines().all(|l| l.len() <= LINE), "{text}");
    assert!(text.contains("SCH_1300000_13006"));
    let model = read(&text);
    assert_eq!(model.bodies.len(), 1);
    assert_eq!(model.bodies[0].kind, BodyKind::Solid);
    assert_eq!((model.faces.len(), model.edges.len(), model.vertices.len()), (6, 12, 8));
    assert_eq!(model.topology.euler_characteristic, 2);
    // Metres: 0.6 × 0.4 × 0.2 m.
    let volume = model.metrics.volume.unwrap();
    assert!((volume - 0.048).abs() < 1e-9, "{volume}");
    let bounds = model.metrics.bounding_box.unwrap();
    assert!((bounds.maximum.x - 0.6).abs() < 1e-9 && (bounds.maximum.z - 0.2).abs() < 1e-9);
}

#[test]
fn a_box_given_inside_out_is_turned_round() {
    let inside_out: Vec<Tri> = cuboid([0.0; 3], [10.0; 3]).into_iter().map(|t| [t[0], t[2], t[1]]).collect();
    let model = read(&write_xt(&[&inside_out]));
    assert_eq!(model.bodies[0].kind, BodyKind::Solid);
    assert!((model.metrics.volume.unwrap() - 1e-6).abs() < 1e-12);
}

#[test]
fn coplanar_triangles_become_one_face_with_its_holes() {
    // A plate with a square hole: the top and bottom faces have an inner loop.
    let (o, i) = (100.0, 40.0);
    let ring = |z: f32| [[-o, -o, z], [o, -o, z], [o, o, z], [-o, o, z], [-i, -i, z], [i, -i, z], [i, i, z], [-i, i, z]];
    let (bottom, top) = (ring(0.0), ring(10.0));
    let mut tris: Vec<Tri> = Vec::new();
    let quad = |t: &mut Vec<Tri>, a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3]| t.extend([[a, b, c], [a, c, d]]);
    for k in 0..4 {
        let (n, m) = ((k + 1) % 4, (k + 1) % 4 + 4);
        // Top and bottom between outer and inner ring.
        quad(&mut tris, top[k], top[n], top[m], top[k + 4]);
        quad(&mut tris, bottom[k], bottom[k + 4], bottom[m], bottom[n]);
        // Outer and inner walls.
        quad(&mut tris, bottom[k], bottom[n], top[n], top[k]);
        quad(&mut tris, bottom[k + 4], top[k + 4], top[m], bottom[m]);
    }
    let model = read(&write_xt(&[&tris]));
    assert_eq!(model.bodies.len(), 1);
    assert_eq!(model.bodies[0].kind, BodyKind::Solid);
    assert_eq!(model.faces.len(), 10);
    assert_eq!(model.loops.len(), 12);
    let volume = model.metrics.volume.unwrap();
    assert!((volume - (0.2 * 0.2 - 0.08 * 0.08) * 0.01).abs() < 1e-9, "{volume}");
}

#[test]
fn meshes_become_several_bodies_and_open_parts_a_sheet() {
    // Two boxes touching along an edge stay two solids; a loose triangle is a sheet.
    let mut tris = cuboid([0.0; 3], [10.0; 3]);
    tris.extend(cuboid([10.0, 10.0, 0.0], [20.0, 20.0, 10.0]));
    let open = [[[0.0, 0.0, 50.0], [10.0, 0.0, 50.0], [0.0, 10.0, 50.0]], [[0.0, 0.0, 60.0], [10.0, 0.0, 60.0], [0.0, 10.0, 60.0]]];
    let degenerate = [[[1.0, 1.0, 1.0], [1.0, 1.0, 1.0], [2.0, 2.0, 2.0]]];
    let text = write_xt(&[&tris, &open, &degenerate]);
    let model = read(&text);
    let kinds: Vec<BodyKind> = model.bodies.iter().map(|b| b.kind).collect();
    assert_eq!(kinds, [BodyKind::Solid, BodyKind::Solid, BodyKind::Sheet]);
    assert_eq!(model.faces.len(), 6 + 6 + 2);
    assert!(text.lines().nth(HEADER.len()).unwrap().starts_with("T51 : TRANSMIT FILE"));
}

#[test]
fn numbers_and_records_follow_the_text_format() {
    assert_eq!(number(0.0), "0");
    assert_eq!(number(-0.0), "0");
    assert_eq!(number(0.25), "0.25");
    assert_eq!(number(1e-8), "1e-8");
    // A separating space at a break begins the next record.
    let stream = format!("{}12 34", "7 ".repeat(39));
    let out = records(&stream);
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines[0].len() <= LINE && !lines[0].ends_with(' '));
    assert_eq!(lines.concat(), stream);
}
