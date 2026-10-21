use super::*;
use monstertruck_io::step::save::{CompleteStepDisplay, SiPrefix, StepHeaderDescriptor, StepMeasurementContext, StepModel};
use monstertruck_modeling::{primitive, CompressedSolid, Solid};

/// A cuboid from the origin to `size`.
fn cuboid(size: [f64; 3]) -> CompressedSolid {
    let bdd = BoundingBox::from_iter([Point3::new(0.0, 0.0, 0.0), Point3::new(size[0], size[1], size[2])]);
    let solid: Solid = primitive::cuboid(bdd);
    solid.compress()
}

fn step_file(solid: &CompressedSolid, prefix: SiPrefix) -> String {
    let context = StepMeasurementContext { length_prefix: prefix, ..Default::default() };
    CompleteStepDisplay::new(StepModel::from(solid).with_measurement_context(context), StepHeaderDescriptor::default()).to_string()
}

fn assert_size(mesh: &Mesh3d, expected: [f32; 3]) {
    let size = mesh.size();
    for k in 0..3 {
        assert!((size[k] - expected[k]).abs() < 0.01, "size {size:?}, expected {expected:?}");
    }
}

#[test]
fn single_solid_in_millimetres() {
    let text = step_file(&cuboid([10.0, 20.0, 30.0]), SiPrefix::Milli);
    let mesh = parse_step(text.as_bytes()).unwrap();
    assert_size(&mesh, [10.0, 20.0, 30.0]);
    assert!(mesh.triangles.len() >= 12, "six faces, at least two triangles each");
    // Moved to the origin like every imported model.
    assert_eq!(mesh.bounds().0, [0.0, 0.0, 0.0]);
}

#[test]
fn lengths_are_converted_to_millimetres() {
    let text = step_file(&cuboid([0.01, 0.02, 0.03]), SiPrefix::None);
    assert_eq!(length_unit_mm(&text), 1000.0);
    assert_size(&parse_step(text.as_bytes()).unwrap(), [10.0, 20.0, 30.0]);
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
    format!("{}{}
{}", &part[..end], entities.join("
"), &part[end..])
}

#[test]
fn assemblies_place_their_parts() {
    // One part used twice, 50 mm apart.
    let part = step_file(&cuboid([10.0, 10.0, 10.0]), SiPrefix::Milli);
    let mesh = parse_step(assembly_file(&part, &[0.0, 50.0]).as_bytes()).unwrap();
    assert_size(&mesh, [60.0, 10.0, 10.0]);
    // Used once: only the part.
    assert_size(&parse_step(assembly_file(&part, &[50.0]).as_bytes()).unwrap(), [10.0, 10.0, 10.0]);
}

#[test]
fn length_units_of_step_files() {
    let unit = |s: &str| length_unit_mm(&format!("DATA;\n#1 = {s};\nENDSEC;"));
    assert_eq!(unit("( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) )"), 1.0);
    assert_eq!(unit("( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT($,.METRE.) )"), 1000.0);
    assert_eq!(unit("( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.CENTI., .METRE.) )"), 10.0);
    assert_eq!(unit("( CONVERSION_BASED_UNIT('INCH',#2) LENGTH_UNIT() NAMED_UNIT(#3) )"), 25.4);
    // Plane angle units do not count; no length unit: millimetres.
    assert_eq!(unit("( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) )"), 1.0);
}

#[test]
fn files_without_geometry_are_refused() {
    assert!(parse_step(b"not a step file").is_err());
    let empty = "ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\nENDSEC;\nEND-ISO-10303-21;\n";
    assert!(parse_step(empty.as_bytes()).is_err());
}

