use super::*;

/// A closed cylinder (radius 10 mm, height 20 mm, axis z) in V13 transmit text: two
/// planar caps and the side face, joined by two ring edges on circles. The bottom
/// cap's plane points up and its face sense is negative.
fn cylinder() -> String {
    let nodes = [
        "12 1 18 0 0 0 0 0 0 1000 1e-8 0 0 0 1 0 1 1 5 9 18 0 2 16 0 ",
        "19 2 1 0 1 3 0 4 V",
        "19 3 2 0 1 0 2 5 S",
        "13 4 3 0 0 0 0 0 0 2 6 ",
        "13 5 4 0 1 0 6 0 0 3 0 ",
        "14 6 5 0 ?7 0 12 5 9 +0 0 7 0 4 ",
        "14 7 6 0 ?8 6 13 5 10 -0 0 8 6 4 ",
        "14 8 7 0 ?0 7 14 5 11 +0 0 0 7 4 ",
        "50 9 8 0 6 10 0 0 +0 0 0.02 0 0 1 1 0 0 ",
        "50 10 9 0 7 11 9 0 +0 0 0 0 0 1 1 0 0 ",
        "51 11 10 0 8 0 10 0 +0 0 0 0 0 1 0.01 1 0 0 ",
        "15 12 11 0 20 6 0 ",
        "15 13 12 0 22 7 0 ",
        "15 14 13 0 21 8 15 ",
        "15 15 14 0 23 8 0 ",
        "16 16 15 0 ?20 0 17 18 0 0 1 ",
        "16 17 16 0 ?23 16 0 19 0 0 1 ",
        "31 18 17 0 16 19 0 0 +0 0 0.02 0 0 1 1 0 0 0.01 ",
        "31 19 18 0 17 0 18 0 +0 0 0 0 0 1 1 0 0 0.01 ",
        "17 20 0 12 20 20 0 21 16 0 0 +",
        "17 21 0 14 21 21 0 20 16 0 0 -",
        "17 22 0 13 22 22 0 23 17 0 0 -",
        "17 23 0 15 23 23 0 22 17 0 0 +",
        "1 0 ",
    ];
    format!("**PART1;FORMAT=text;\n**PART2;SCH=SCH_1300000_13006;\n**END_OF_HEADER*****\nT51 : TRANSMIT FILE created by modeller version 130000017 SCH_1300000_130060 {}\n", nodes.concat())
}

/// Signed volume of a closed mesh (positive when its triangles face outwards).
fn volume(mesh: &Mesh3d) -> f64 {
    mesh.triangles
        .iter()
        .map(|t| {
            let p = |i: u32| mesh.vertices[i as usize].map(f64::from);
            dot(p(t[0]), cross(p(t[1]), p(t[2]))) / 6.0
        })
        .sum()
}

#[test]
fn curved_faces_are_tessellated_and_face_outwards() {
    let mesh = parse_parasolid(cylinder().as_bytes()).unwrap();
    let s = mesh.size();
    for (k, expected) in [20.0, 20.0, 20.0].into_iter().enumerate() {
        assert!((s[k] - expected).abs() < 0.05, "{s:?}");
    }
    let expected = std::f64::consts::PI * 100.0 * 20.0;
    let v = volume(&mesh);
    assert!((v - expected).abs() < expected * 0.02, "{v} / {expected}");
}

#[test]
fn faceted_bodies_of_the_writer_are_read() {
    // The panel export of an enclosure, and a box written by openecraft-xt.
    let cuboid = |a: [f32; 3], b: [f32; 3]| -> Vec<[[f32; 3]; 3]> {
        let p = |i: usize| [if i & 1 == 0 { a[0] } else { b[0] }, if i & 2 == 0 { a[1] } else { b[1] }, if i & 4 == 0 { a[2] } else { b[2] }];
        let quads = [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [2, 6, 7, 3], [0, 4, 6, 2], [1, 3, 7, 5]];
        quads.iter().flat_map(|q| [[p(q[0]), p(q[1]), p(q[2])], [p(q[0]), p(q[2]), p(q[3])]]).collect()
    };
    let mut prj = openecraft_core::model::Project::default();
    prj.layout.enclosures.push(openecraft_core::panel::Enclosure { name: "+A1".into(), width: 600.0, height: 800.0, depth: 250.0, ..Default::default() });
    let text = openecraft_core::export3d::parasolid(&prj);
    let mesh = parse_parasolid(text.as_bytes()).unwrap();
    let s = mesh.size();
    for (k, expected) in [600.0, 252.0, 800.0].into_iter().enumerate() {
        assert!((s[k] - expected).abs() < 0.01, "{s:?}");
    }
    let boxes = cuboid([0.0; 3], [10.0, 20.0, 30.0]);
    let text = openecraft_xt::write_xt(&[&boxes]);
    let mesh = parse_parasolid(text.as_bytes()).unwrap();
    assert!((volume(&mesh) - 6000.0).abs() < 1e-3, "{}", volume(&mesh));
}

#[test]
fn boundaries_are_triangulated_with_their_holes() {
    let square = |r: f64, z: f64| vec![[-r, -r, z], [r, -r, z], [r, r, z], [-r, r, z]];
    let tris = triangulate(vec![square(1.0, 0.0), square(10.0, 0.0)]);
    let area: f64 = tris.iter().map(|t| length(cross(sub(t[1], t[0]), sub(t[2], t[0]))) / 2.0).sum();
    assert!((area - (400.0 - 4.0)).abs() < 1e-9, "{area}");
    // Wound like the outer ring (counter-clockwise seen from +z).
    assert!(tris.iter().all(|t| cross(sub(t[1], t[0]), sub(t[2], t[0]))[2] > 0.0));
}

#[test]
fn splines_evaluate_like_their_definition() {
    // A clamped quadratic through (0,0,0), (1,1,0) control, (2,0,0).
    let s = Spline { degree: 2, knots: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0], poles: vec![[0.0, 0.0, 0.0, 1.0], [1.0, 1.0, 0.0, 1.0], [2.0, 0.0, 0.0, 1.0]] };
    let mid = s.at(0.5);
    assert!((mid[0] - 1.0).abs() < 1e-12 && (mid[1] - 0.5).abs() < 1e-12);
    assert!((s.parameter([1.0, 0.5, 0.0]) - 0.5).abs() < 1e-6);
    assert!(s.clamped());
}

#[test]
fn ring_edges_are_split_in_two() {
    // The tessellator cannot trim with an edge that starts where it ends.
    let text = cylinder();
    let model = read_model(text.as_bytes()).unwrap();
    assert!(model.complete);
    let ix = Index::new(&model);
    let mut b = Builder::new(&model);
    let all: Vec<usize> = (0..model.faces.len()).collect();
    assert!(b.body(&ix, &all).is_empty());
    assert_eq!(b.ir.model.edges.len(), 4);
    assert!(b.ir.model.edges.iter().all(|e| e.start != e.end));
    let ranges: Vec<[f64; 2]> = b.ir.model.edges.iter().filter_map(|e| e.param_range).collect();
    assert_eq!(ranges[0], [0.0, std::f64::consts::PI]);
    assert_eq!(ranges[1], [std::f64::consts::PI, std::f64::consts::TAU]);
    // Each face on its own meshes too (the fallback when one face fails).
    for f in 0..model.faces.len() {
        let mut single = Builder::new(&model);
        assert!(single.body(&ix, &[f]).is_empty());
        assert!(!mesh_bodies(&single, &ix).unwrap().triangles.is_empty());
    }
}

#[test]
fn other_files_are_refused() {
    assert!(parse_parasolid(b"ISO-10303-21;").unwrap_err().contains("Parasolid"));
    assert!(parse_parasolid(b"").is_err());
}
