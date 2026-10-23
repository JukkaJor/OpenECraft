use super::*;

/// An IFC4 file: units (`unit` is the IFCSIUNIT / conversion based unit entity
/// with number #100), a product with placement #20 and the shape items `items`
/// (entity numbers from #30) plus `extra` entities.
fn file(unit: &str, items: &[&str], extra: &str) -> String {
    let refs = (0..items.len()).map(|i| format!("#{}", 30 + i)).collect::<Vec<_>>().join(",");
    let mut data = vec![
        "#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,$,#101);".to_string(),
        format!("#100={unit};"),
        "#101=IFCUNITASSIGNMENT((#100,#102));".into(),
        "#102=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);".into(),
        "#10=IFCCARTESIANPOINT((0.,0.,0.));".into(),
        "#11=IFCAXIS2PLACEMENT3D(#10,$,$);".into(),
        "#12=IFCCARTESIANPOINT((100.,0.,0.));".into(),
        "#13=IFCAXIS2PLACEMENT3D(#12,$,$);".into(),
        "#19=IFCLOCALPLACEMENT($,#11);".into(),
        "#20=IFCLOCALPLACEMENT(#19,#13);".into(),
        format!("#25=IFCSHAPEREPRESENTATION($,'Body','Tessellation',({refs}));"),
        "#26=IFCSHAPEREPRESENTATION($,'Box','BoundingBox',(#90));".into(),
        "#27=IFCPRODUCTDEFINITIONSHAPE($,$,(#25,#26));".into(),
        "#28=IFCBUILDINGELEMENTPROXY('1YvctVUKr0kugbFTf53O9L',$,'-K1',$,$,#20,#27,$,$);".into(),
        "#90=IFCBLOCK(#11,1000.,1000.,1000.);".into(),
    ];
    data.extend(items.iter().enumerate().map(|(i, it)| format!("#{}={it};", 30 + i)));
    data.push(extra.to_string());
    format!("ISO-10303-21;\nHEADER;\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n{}\nENDSEC;\nEND-ISO-10303-21;\n", data.join("\n"))
}

const MM: &str = "IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.)";

fn assert_size(mesh: &Mesh3d, expected: [f32; 3]) {
    let s = mesh.size();
    for k in 0..3 {
        assert!((s[k] - expected[k]).abs() < 0.01, "size {s:?}, expected {expected:?}");
    }
}

#[test]
fn triangulated_face_sets() {
    let text = file(
        MM,
        &["IFCTRIANGULATEDFACESET(#40,$,.T.,((1,2,3),(1,3,4)),$)"],
        "#40=IFCCARTESIANPOINTLIST3D(((0.,0.,0.),(10.,0.,0.),(10.,20.,0.),(0.,20.,5.)));",
    );
    let mesh = parse_ifc(text.as_bytes()).unwrap();
    assert_eq!(mesh.triangles.len(), 2);
    // The bounding box representation is not read.
    assert_size(&mesh, [10.0, 20.0, 5.0]);
}

#[test]
fn extrusions_in_metres() {
    let text = file(
        "IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.)",
        &["IFCEXTRUDEDAREASOLID(#41,#11,#42,0.03)", "IFCEXTRUDEDAREASOLID(#43,#11,#42,0.01)"],
        "#41=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,0.01,0.02);\n#42=IFCDIRECTION((0.,0.,1.));\n#43=IFCCIRCLEPROFILEDEF(.AREA.,$,$,0.002);",
    );
    let mesh = parse_ifc(text.as_bytes()).unwrap();
    assert_size(&mesh, [10.0, 20.0, 30.0]);
    // Box: 2 × 2 caps + 4 × 2 sides; cylinder: 2 × 30 caps + 32 × 2 sides.
    assert_eq!(mesh.triangles.len(), 12 + 60 + 64);
}

#[test]
fn faceted_breps_and_inch() {
    let extra = [
        "#50=IFCCARTESIANPOINT((0.,0.,0.));",
        "#51=IFCCARTESIANPOINT((1.,0.,0.));",
        "#52=IFCCARTESIANPOINT((1.,2.,0.));",
        "#53=IFCCARTESIANPOINT((0.,2.,0.));",
        "#60=IFCCLOSEDSHELL((#63));",
        "#61=IFCPOLYLOOP((#50,#51,#52,#53));",
        "#62=IFCFACEOUTERBOUND(#61,.T.);",
        "#63=IFCFACE((#62));",
        "#103=IFCMEASUREWITHUNIT(IFCLENGTHMEASURE(0.0254),#104);",
        "#104=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);",
    ]
    .join("\n");
    let text = file("IFCCONVERSIONBASEDUNIT(#105,.LENGTHUNIT.,'INCH',#103)", &["IFCFACETEDBREP(#60)"], &extra);
    let mesh = parse_ifc(text.as_bytes()).unwrap();
    assert_eq!(mesh.triangles.len(), 2);
    assert_size(&mesh, [25.4, 50.8, 0.0]);
}

#[test]
fn mapped_items_are_placed() {
    let text = file(
        MM,
        &["IFCMAPPEDITEM(#44,#45)", "IFCMAPPEDITEM(#44,#46)"],
        "#40=IFCCARTESIANPOINTLIST3D(((0.,0.,0.),(10.,0.,0.),(0.,10.,0.)));\n#41=IFCTRIANGULATEDFACESET(#40,$,$,((1,2,3)),$);\n#43=IFCSHAPEREPRESENTATION($,'Body','Tessellation',(#41));\n#44=IFCREPRESENTATIONMAP(#11,#43);\n#45=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#10,$,$);\n#46=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#47,2.,$);\n#47=IFCCARTESIANPOINT((0.,0.,50.));",
    );
    let mesh = parse_ifc(text.as_bytes()).unwrap();
    assert_eq!(mesh.triangles.len(), 2);
    assert_size(&mesh, [20.0, 20.0, 50.0]);
}

#[test]
fn placements_chain() {
    let ifc = Ifc { e: parse_entities(file(MM, &[], "").as_bytes()).unwrap(), mesh: MeshBuilder::new(), scale: 1.0, triangles: 0 };
    let tf = ifc.object_placement(&Value::Ref(20), 0);
    assert_eq!(tf.point([1.0, 2.0, 3.0]), [101.0, 2.0, 3.0]);
    // Rotated: z along x, x along y.
    let rotated = "#14=IFCDIRECTION((1.,0.,0.));\n#15=IFCDIRECTION((0.,1.,0.));\n#16=IFCAXIS2PLACEMENT3D(#10,#14,#15);";
    let ifc = Ifc { e: parse_entities(file(MM, &[], rotated).as_bytes()).unwrap(), mesh: MeshBuilder::new(), scale: 1.0, triangles: 0 };
    let tf = ifc.placement(&Value::Ref(16));
    let p = tf.point([1.0, 2.0, 3.0]);
    assert!((p[0] - 3.0).abs() < 1e-9 && (p[1] - 1.0).abs() < 1e-9 && (p[2] - 2.0).abs() < 1e-9, "{p:?}");
}

#[test]
fn strings_comments_and_typed_values() {
    let mut p = Parser { s: b" /* c */ ('It''s;', .T., IFCLABEL('x'), -1.5E-3, #12, $, *)", pos: 0 };
    let v = p.list().unwrap();
    assert_eq!(v[0], Value::Str("It's;".into()));
    assert_eq!(v[1], Value::Enum("T".into()));
    assert_eq!(v[2], Value::Typed("IFCLABEL".into(), Box::new(Value::Str("x".into()))));
    assert_eq!(v[3], Value::Num(-0.0015));
    assert_eq!(v[4], Value::Ref(12));
    assert_eq!(&v[5..], [Value::Null, Value::Null]);
}

#[test]
fn concave_polygons_are_clipped() {
    // An L shape: 6 corners, 4 triangles covering 3 unit squares.
    let l = [[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [1.0, 1.0], [1.0, 2.0], [0.0, 2.0]];
    for poly in [l.to_vec(), l.iter().rev().copied().collect()] {
        let tris = ear_clip(&poly);
        assert_eq!(tris.len(), 4);
        let area: f64 = tris.iter().map(|t| ((poly[t[1]][0] - poly[t[0]][0]) * (poly[t[2]][1] - poly[t[0]][1]) - (poly[t[1]][1] - poly[t[0]][1]) * (poly[t[2]][0] - poly[t[0]][0])) / 2.0).sum::<f64>().abs();
        assert!((area - 3.0).abs() < 1e-9);
    }
}

#[test]
fn other_files_are_refused() {
    assert!(parse_ifc(b"solid x").unwrap_err().contains("not an IFC file"));
    assert!(parse_ifc(file(MM, &[], "").as_bytes()).unwrap_err().contains("no supported body geometry"));
    assert!(parse_mesh_by_name().is_ok());
}

fn parse_mesh_by_name() -> Result<Mesh3d, String> {
    let text = file(MM, &["IFCBLOCK(#11,1.,2.,3.)"], "");
    crate::parse_mesh("part.IFC", text.as_bytes())
}
