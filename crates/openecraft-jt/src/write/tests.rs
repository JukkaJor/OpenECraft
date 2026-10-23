use super::*;

fn cube() -> Vec<Tri> {
    let p = |x: f32, y: f32, z: f32| [x * 10.0, y * 20.0, z * 30.0];
    let quads = [
        [p(0., 0., 0.), p(0., 1., 0.), p(1., 1., 0.), p(1., 0., 0.)],
        [p(0., 0., 1.), p(1., 0., 1.), p(1., 1., 1.), p(0., 1., 1.)],
        [p(0., 0., 0.), p(1., 0., 0.), p(1., 0., 1.), p(0., 0., 1.)],
        [p(0., 1., 0.), p(0., 1., 1.), p(1., 1., 1.), p(1., 1., 0.)],
        [p(0., 0., 0.), p(0., 0., 1.), p(0., 1., 1.), p(0., 1., 0.)],
        [p(1., 0., 0.), p(1., 1., 0.), p(1., 1., 1.), p(1., 0., 1.)],
    ];
    quads.iter().flat_map(|q| [[q[0], q[1], q[2]], [q[0], q[2], q[3]]]).collect()
}

fn sample() -> Vec<u8> {
    let tris = cube();
    let groups = [Group { name: "+A1 Enclosure", bodies: vec![Body { name: "-K1", color: [200, 100, 50], transparent: false, triangles: &tris }, Body { name: "empty", color: [0; 3], transparent: true, triangles: &[] }] }];
    write_jt("Panel", &groups)
}

#[test]
fn header_and_table_of_contents() {
    let jt = sample();
    assert!(jt.starts_with(b"Version 8.1 JT "));
    assert_eq!(jt[80], 0, "little endian");
    let toc = i32::from_le_bytes(jt[85..89].try_into().unwrap()) as usize;
    let entries = i32::from_le_bytes(jt[toc..toc + 4].try_into().unwrap());
    assert_eq!(entries, 2, "scene graph and one shape LOD (empty bodies are left out)");
    assert_eq!(toc + 4 + 2 * (16 + 4 + 4 + 4), jt.len());
    // The LSG segment is the first one and named in the header.
    assert_eq!(jt[89..105], jt[HEADER_LEN..HEADER_LEN + 16]);
    let attributes = u32::from_le_bytes(jt[toc + 4 + 24..toc + 4 + 28].try_into().unwrap());
    assert_eq!(attributes >> 24, SEGMENT_LSG);
}

#[test]
fn strings_are_utf16() {
    let mut b = Buf::default();
    b.mb_string("Aü");
    assert_eq!(b.0, [2, 0, 0, 0, b'A', 0, 0xfc, 0]);
}

#[test]
fn lod_data_holds_one_strip_per_triangle() {
    let tris = cube();
    let lod = lod_data(&tris);
    let size = i32::from_le_bytes(lod[20..24].try_into().unwrap()) as usize;
    assert_eq!(size, 4 + 13 * 4 + 12 * 3 * 6 * 4);
}
