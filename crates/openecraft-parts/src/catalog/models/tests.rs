use super::*;
use crate::{default_catalog, FnKind, FunctionTemplate};

fn part(group: &str, subgroup: &str, dims: (f32, f32, f32)) -> Part {
    Part { group: group.into(), subgroup: subgroup.into(), width: dims.0, height: dims.1, depth: dims.2, ..Default::default() }
}

fn floor(mut p: Part) -> Part {
    p.mounting_data.floor_mount = true;
    p
}

#[test]
fn floor_standing_parts_are_modelled_upright() {
    // A vessel: a round body as tall as the part, its top above the middle of its
    // footprint.
    let tank = floor(part("Fluid power components", "Vessel", (800.0, 1750.0, 800.0)));
    let (front, mesh) = model(&tank).unwrap();
    assert_eq!(mesh.size(), [800.0, 1750.0, 800.0]);
    assert_eq!((front.width, front.height), (800.0, 1750.0));
    let top = mesh.vertices.iter().copied().fold([0.0f32; 3], |a, v| if v[1] > a[1] { v } else { a });
    assert!((top[0] - 400.0).abs() < 60.0 && (top[2] - 400.0).abs() < 60.0, "{top:?}");
    // The same part on a wall is no vessel model.
    assert!(model(&part("Fluid power components", "Vessel", (800.0, 1750.0, 800.0))).is_none());
}

#[test]
fn models_are_painted_and_fill_the_part() {
    let mut pump = floor(part("Fluid power components", "Pump", (260.0, 720.0, 360.0)));
    pump.functions = vec![FunctionTemplate { kind: FnKind::Pump, ..Default::default() }];
    let mut valve = part("Fluid power components", "Shut-off valve", (180.0, 130.0, 150.0));
    valve.description = "Motorised ball valve with actuator".into();
    for p in [pump, valve, part("Safety devices", "Fuse switch disconnector", (106.0, 210.0, 115.0)), part("Fluid power components", "Service unit", (80.0, 220.0, 90.0))] {
        let (_, mesh) = model(&p).unwrap_or_else(|| panic!("{}", p.subgroup));
        assert_eq!(mesh.colors.len(), mesh.triangles.len(), "{}", p.subgroup);
        assert!(mesh.colors.iter().any(|c| *c != mesh.colors[0]), "{} has details in other colours", p.subgroup);
        let (lo, hi) = mesh.bounds();
        assert_eq!((lo, hi), ([0.0; 3], [p.width, p.height, p.depth]), "{} fills its box", p.subgroup);
    }
}

#[test]
fn round_bodies_face_outwards() {
    let mut m = Mesh3d::default();
    m.add_cylinder([0.0, 0.0, 0.0], 2, 10.0, 20.0, 12, BODY);
    // Every triangle's normal points away from the middle of the cylinder.
    for t in &m.triangles {
        let [a, b, c] = t.map(|i| m.vertices[i as usize]);
        let n = crate::vec3::cross(crate::vec3::sub(b, a), crate::vec3::sub(c, a));
        let centre = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0];
        let out = [centre[0], centre[1], centre[2] - 10.0];
        assert!(crate::vec3::dot(n, out) > 0.0, "{t:?}");
    }
}

#[test]
fn catalog_devices_get_their_models() {
    let parts = default_catalog();
    let get = |n: &str| parts.iter().find(|p| p.number == n).unwrap_or_else(|| panic!("{n}"));
    for n in ["GEN.PUMP-BOOST-DN32", "GEN.TANK-1000L", "GEN.HP-AW-12KW", "GEN.COMP-PISTON-450", "GEN.VLV-BALL-DN32", "GEN.FR-G38-WALL", "GEN.MANIFOLD-DN32-4"] {
        let p = get(n);
        assert!(kind_of(p).is_some(), "{n}");
        assert!(p.model_3d.as_ref().is_some_and(|m| !m.colors.is_empty()), "{n}");
    }
    // DIN rail devices keep their shape, painted.
    let din = parts.iter().find(|p| p.subgroup == "Circuit breaker").unwrap();
    assert!(kind_of(din).is_none() && din.model_3d.as_ref().is_some_and(|m| m.colors.len() == m.triangles.len()));
}

#[test]
fn devices_of_the_bundled_catalogs_look_like_their_photos() {
    let parts = crate::bundled::default_parts();
    let get = |n: &str| parts.iter().find(|p| p.number == n).unwrap_or_else(|| panic!("{n}"));
    for (n, brand) in [
        ("VIC.PMP482505010", brands::Brand::MultiPlus),
        ("VIC.PIN482601000", brands::Brand::InverterRs),
        ("PYL.US5000", brands::Brand::Pylontech),
        ("HAG.LT253", brands::Brand::HagerNh),
        ("HAG.ECR380D", brands::Brand::HagerMeter),
        ("DEH.952405", brands::Brand::Dehn(4)),
        ("BECK.EL1008", brands::Brand::BeckhoffIo),
        ("PXC.2904601", brands::Brand::PhoenixPsu),
    ] {
        let p = get(n);
        assert_eq!(brands::brand_of(p), Some(brand), "{n}");
        let m = p.model_3d.as_ref().unwrap();
        assert_eq!(m.bounds(), ([0.0; 3], [p.width, p.height, p.depth]), "{n} fills its box");
        assert_eq!(m.colors.len(), m.triangles.len(), "{n}");
        let front = p.model_2d.as_ref().unwrap();
        assert!(front.prims.len() > 3, "{n} shows its front details");
    }
    // Victron's blue and the orange band, the black front of the battery.
    let colors = |n: &str| get(n).model_3d.as_ref().unwrap().colors.clone();
    assert!(colors("VIC.PMP482505010").contains(&VICTRON) && colors("VIC.PMP482505010").contains(&[232, 122, 40]));
    assert!(colors("PYL.US5000").contains(&BLACK));
    // Every device of the bundled catalogs with dimensions has a model.
    for p in parts.iter().filter(|p| p.width > 0.0 && p.height > 0.0 && p.depth > 0.0) {
        assert!(p.model_3d.is_some(), "{}", p.number);
    }
}
