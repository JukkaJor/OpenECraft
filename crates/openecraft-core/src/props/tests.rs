use super::*;
use crate::geom::p;

#[test]
fn values_come_from_device_and_parts() {
    let mut prj = Project::default();
    prj.parts.push(Part { number: "SIE.3RT2015".into(), type_number: "3RT2015-1BB41".into(), manufacturer: "Siemens".into(), ..Default::default() });
    prj.devices.insert(
        "-K1".into(),
        DeviceData { description: "Main contactor".into(), parts: vec![PartRef { number: "SIE.3RT2015".into(), count: 1 }], ..Default::default() },
    );
    let s = SymbolInst { tag: "=A1-K1".into(), pins: vec!["A1".into(), "A2".into()], ..Default::default() };
    assert_eq!(value(&prj, &s, PropKind::Pins), "A1 A2");
    assert_eq!(value(&prj, &s, PropKind::Structure), "=A1");
    assert_eq!(value(&prj, &s, PropKind::Description), "");
    let s = SymbolInst { tag: "-K1".into(), ..s };
    assert_eq!(value(&prj, &s, PropKind::Description), "Main contactor");
    assert_eq!(value(&prj, &s, PropKind::PartNumber), "SIE.3RT2015");
    assert_eq!(value(&prj, &s, PropKind::TypeNumber), "3RT2015-1BB41");
    assert_eq!(value(&prj, &s, PropKind::Manufacturer), "Siemens");
}

#[test]
fn added_properties_stack_downwards() {
    let mut s = SymbolInst::default();
    add(&mut s, PropKind::Description, p(6.0, 2.0));
    add(&mut s, PropKind::PartNumber, p(6.0, 2.0));
    assert_eq!(s.props[0].offset, p(6.0, 2.0));
    assert!((s.props[1].offset.y - 4.8).abs() < 1e-4);
}

#[test]
fn every_kind_has_a_title() {
    for k in PropKind::ALL {
        assert!(!k.title().is_empty());
    }
}
