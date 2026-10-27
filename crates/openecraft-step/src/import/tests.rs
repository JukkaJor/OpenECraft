use super::*;

/// A cuboid of 10 × 20 × 30 mm from the origin, written by monstertruck.
const CUBOID_MM: &str = include_str!("cuboid_mm.step");
/// The same cuboid in metres (`SI_UNIT($,.METRE.)`).
const CUBOID_M: &str = include_str!("cuboid_m.step");

fn assert_size(mesh: &Mesh3d, expected: [f32; 3]) {
    let size = mesh.size();
    for k in 0..3 {
        assert!((size[k] - expected[k]).abs() < 0.01, "size {size:?}, expected {expected:?}");
    }
}

#[test]
fn single_solid_in_millimetres() {
    let mesh = parse_step(CUBOID_MM.as_bytes()).unwrap();
    assert_size(&mesh, [10.0, 20.0, 30.0]);
    assert!(mesh.triangles.len() >= 12, "six faces, at least two triangles each");
    // Moved to the origin like every imported model.
    assert_eq!(mesh.bounds().0, [0.0, 0.0, 0.0]);
}

#[test]
fn lengths_are_converted_to_millimetres() {
    assert_size(&parse_step(CUBOID_M.as_bytes()).unwrap(), [10.0, 20.0, 30.0]);
}

/// The cuboid file plus a top product that uses its part (`#5`, shape
/// representation `#10`) once per offset: the AP214 assembly structure with
/// `NEXT_ASSEMBLY_USAGE_OCCURRENCE` and `ITEM_DEFINED_TRANSFORMATION`.
fn assembly_file(part: &str, offsets: &[f64]) -> String {
    let mut entities = vec![
        "#1000 = PRODUCT('top','top','',(#8));".to_string(),
        "#1001 = PRODUCT_DEFINITION_FORMATION('','',#1000);".into(),
        "#1002 = PRODUCT_DEFINITION('design','',#1001,#9);".into(),
        "#1003 = PRODUCT_DEFINITION_SHAPE('','',#1002);".into(),
        "#1006 = CARTESIAN_POINT('',(0.,0.,0.));".into(),
        "#1007 = DIRECTION('',(0.,0.,1.));".into(),
        "#1008 = DIRECTION('',(1.,0.,0.));".into(),
        "#1009 = AXIS2_PLACEMENT_3D('',#1006,#1007,#1008);".into(),
    ];
    let mut placements = Vec::new();
    for (i, x) in offsets.iter().enumerate() {
        let n = 1100 + 10 * i;
        placements.push(format!("#{}", n + 1));
        entities.extend([
            format!("#{n} = CARTESIAN_POINT('',({x:.1},0.,0.));"),
            format!("#{} = AXIS2_PLACEMENT_3D('',#{n},#1007,#1008);", n + 1),
            format!("#{} = NEXT_ASSEMBLY_USAGE_OCCURRENCE('{i}','block','',#1002,#5,$);", n + 2),
            format!("#{} = PRODUCT_DEFINITION_SHAPE('','',#{});", n + 3, n + 2),
            format!("#{} = ITEM_DEFINED_TRANSFORMATION('','',#1009,#{});", n + 4, n + 1),
            format!(
                "#{} = ( REPRESENTATION_RELATIONSHIP('','',#10,#1004) REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#{}) SHAPE_REPRESENTATION_RELATIONSHIP() );",
                n + 5,
                n + 4
            ),
            format!("#{} = CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#{},#{});", n + 6, n + 5, n + 3),
        ]);
    }
    entities.push(format!("#1004 = SHAPE_REPRESENTATION('top',(#1009,{}),#11);", placements.join(",")));
    entities.push("#1005 = SHAPE_DEFINITION_REPRESENTATION(#1003,#1004);".into());
    let end = part.rfind("ENDSEC;").unwrap();
    format!("{}{}\n{}", &part[..end], entities.join("\n"), &part[end..])
}

#[test]
fn assemblies_place_their_parts() {
    // One part used twice, 50 mm apart.
    let mesh = parse_step(assembly_file(CUBOID_MM, &[0.0, 50.0]).as_bytes()).unwrap();
    assert_size(&mesh, [60.0, 20.0, 30.0]);
    // Used once: only the part.
    assert_size(&parse_step(assembly_file(CUBOID_MM, &[50.0]).as_bytes()).unwrap(), [10.0, 20.0, 30.0]);
}

#[test]
fn files_without_geometry_are_refused() {
    assert!(parse_step(b"not a step file").is_err());
    let empty = "ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\nENDSEC;\nEND-ISO-10303-21;\n";
    assert!(parse_step(empty.as_bytes()).is_err());
}
