use super::*;

fn unit_box() -> Mesh3d {
    let mut m = Mesh3d::default();
    m.add_box([0.0, 0.0, 0.0], [10.0, 20.0, 30.0]);
    m
}

fn close(a: [f32; 3], b: [f32; 3]) -> bool {
    (0..3).all(|k| (a[k] - b[k]).abs() < 1e-4)
}

/// Every triangle of a closed box faces outwards (normal points away from the centre).
fn outward(m: &Mesh3d) -> bool {
    let (min, max) = m.bounds();
    let c = [(min[0] + max[0]) / 2.0, (min[1] + max[1]) / 2.0, (min[2] + max[2]) / 2.0];
    m.triangles.iter().all(|t| {
        let [a, b, d] = [m.vertices[t[0] as usize], m.vertices[t[1] as usize], m.vertices[t[2] as usize]];
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [d[0] - a[0], d[1] - a[1], d[2] - a[2]];
        let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        let to_face = [a[0] - c[0], a[1] - c[1], a[2] - c[2]];
        n[0] * to_face[0] + n[1] * to_face[1] + n[2] * to_face[2] > 0.0
    })
}

#[test]
fn box_mesh_and_transforms() {
    let mut m = unit_box();
    assert_eq!((m.vertices.len(), m.triangles.len()), (8, 12));
    assert!(close(m.size(), [10.0, 20.0, 30.0]));
    assert!(outward(&m));
    m.rotate90(2); // around z: width and height swap
    assert!(close(m.size(), [20.0, 10.0, 30.0]));
    assert!(close(m.bounds().0, [0.0, 0.0, 0.0]));
    m.rotate90(0); // around x: height and depth swap
    assert!(close(m.size(), [20.0, 30.0, 10.0]));
    assert!(outward(&m));
    m.scale(2.0);
    assert!(close(m.size(), [40.0, 60.0, 20.0]));
}

fn ascii_stl(m: &Mesh3d) -> String {
    let mut s = String::from("solid test\n");
    for t in &m.triangles {
        s.push_str("  facet normal 0 0 0\n    outer loop\n");
        for &i in t {
            let v = m.vertices[i as usize];
            s.push_str(&format!("      vertex {} {} {}\n", v[0], v[1], v[2]));
        }
        s.push_str("    endloop\n  endfacet\n");
    }
    s.push_str("endsolid test\n");
    s
}

fn binary_stl(m: &Mesh3d) -> Vec<u8> {
    let mut b = vec![0u8; 80];
    b.extend((m.triangles.len() as u32).to_le_bytes());
    for t in &m.triangles {
        b.extend([0u8; 12]);
        for &i in t {
            for c in m.vertices[i as usize] {
                b.extend(c.to_le_bytes());
            }
        }
        b.extend([0u8; 2]);
    }
    b
}

#[test]
fn stl_ascii_and_binary() {
    let m = unit_box();
    for parsed in [parse_stl(ascii_stl(&m).as_bytes()).unwrap(), parse_stl(&binary_stl(&m)).unwrap()] {
        // Vertices of neighbouring triangles are merged again.
        assert_eq!((parsed.vertices.len(), parsed.triangles.len()), (8, 12));
        assert!(close(parsed.size(), [10.0, 20.0, 30.0]));
        assert!(outward(&parsed));
    }
    assert!(parse_stl(b"solid x\nendsolid x\n").is_err());
    let mut truncated = binary_stl(&m);
    truncated.truncate(200);
    assert!(parse_stl(&truncated).unwrap_err().contains("truncated"));
    assert!(parse_mesh("model.STL", ascii_stl(&m).as_bytes()).is_ok());
    assert!(parse_mesh("model.step", b"").is_err());
}

#[test]
fn obj_with_quads_and_negative_indices() {
    let obj = "# cube face\nv 0 0 0\nv 10 0 0\nv 10 10 0\nv 0 10 0\nvt 0 0\nf 1/1/1 2/1/1 3/1/1 4/1/1\nv 0 0 5\nf -1 1 2\n";
    let m = parse_obj(obj).unwrap();
    assert_eq!(m.triangles.len(), 3);
    assert!(close(m.size(), [10.0, 10.0, 5.0]));
    assert!(parse_obj("v 0 0 0\nf 1 2 3\n").unwrap_err().contains("out of range"));
    assert!(parse_obj("o nothing\n").is_err());
}

#[test]
fn svg_elements_units_and_paths() {
    let svg = r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="45mm" height="90mm" viewBox="0 0 450 900">
  <rect x="0" y="0" width="450" height="900" fill="none" stroke="black"/>
  <rect x="50" y="100" width="350" height="200" style="fill:#ccc"/>
  <line x1="0" y1="450" x2="450" y2="450"/>
  <circle cx="100" cy="50" r="20"/>
  <polygon points="10,10 40,10 40,40"/>
  <path d="M 10 800 l 100 0 v 50 h -100 z m 200 0 L 300 850"/>
  <text x="225" y="600" font-size="40">-K1</text>
</svg>"#;
    let m = parse_svg(svg).unwrap();
    assert_eq!((m.width, m.height), (45.0, 90.0));
    // viewBox units are scaled to millimetres (1/10)
    assert_eq!(m.prims[1], Prim2d::Rect { min: [5.0, 10.0], max: [40.0, 30.0], fill: true });
    assert_eq!(m.prims[0], Prim2d::Rect { min: [0.0, 0.0], max: [45.0, 90.0], fill: false });
    assert_eq!(m.prims[3], Prim2d::Circle { c: [10.0, 5.0], r: 2.0, fill: false });
    // Relative path commands and closing
    let closed = m.prims.iter().find_map(|p| match p {
        Prim2d::Polyline { pts, closed: true, .. } if pts.len() == 4 => Some(pts.clone()),
        _ => None,
    });
    assert_eq!(closed.unwrap(), vec![[1.0, 80.0], [11.0, 80.0], [11.0, 85.0], [1.0, 85.0]]);
    assert!(m.prims.iter().any(|p| matches!(p, Prim2d::Polyline { pts, closed: false, .. } if pts == &vec![[21.0, 80.0], [30.0, 85.0]])));
    assert!(m.prims.iter().any(|p| matches!(p, Prim2d::Text { text, size, .. } if text == "-K1" && (*size - 4.0).abs() < 1e-4)));
    // Without units one user unit is one millimetre
    let plain = parse_svg(r#"<svg viewBox="0 0 20 10"><line x1="0" y1="0" x2="20" y2="10"/></svg>"#).unwrap();
    assert_eq!((plain.width, plain.height), (20.0, 10.0));
    assert!(parse_svg("<html></html>").is_err());
    assert!(parse_svg("<svg width='10' height='10'></svg>").is_err());
}

#[test]
fn number_lists() {
    assert_eq!(numbers("10,20 -5-6 1e2 .5.5"), vec![10.0, 20.0, -5.0, -6.0, 100.0, 0.5, 0.5]);
}
