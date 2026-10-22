use super::*;
use crate::reports::Analysis;
use crate::sample::demo_project;

/// The demo with numbered wires, its markers and its analysis.
fn demo() -> (Project, Analysis) {
    let mut prj = demo_project();
    let an = Analysis::build(&prj);
    crate::wirenum::renumber(&mut prj, &an.open, &an.netlist, true);
    let an = Analysis::build(&prj);
    (prj, an)
}

fn of(markers: &[Marker], category: Category) -> Vec<&Marker> {
    markers.iter().filter(|m| m.category == category).collect()
}

#[test]
fn devices_get_markers_door_devices_legend_plates() {
    let (prj, an) = demo();
    let markers = markers(&prj, &an.connections);
    let devices = of(&markers, Category::Device);
    let strips = crate::strips::strip_tags(&prj);
    assert_eq!(devices.len(), Index::build(&prj).devices.len() - strips.len());
    let k1 = devices.iter().find(|m| m.group == "-K1").unwrap();
    assert_eq!((k1.text.as_str(), k1.material.as_str(), k1.enclosure.as_str()), ("-K1", "GEN.MARK-DEV", "+CAB1"));
    let s1 = devices.iter().find(|m| m.group == "-S1").unwrap();
    assert_eq!(s1.material, "GEN.MARK-LEG", "door device");
    let m1 = devices.iter().find(|m| m.group == "-M1").unwrap();
    assert_eq!(m1.enclosure, "", "field device");
}

#[test]
fn terminal_markers_in_rail_order() {
    let (mut prj, _) = demo();
    prj.strips.entry("-X1".into()).or_default().spares = 1;
    let markers = markers(&prj, &[]);
    let texts: Vec<&str> = of(&markers, Category::Terminal).iter().map(|m| m.text.as_str()).collect();
    assert_eq!(texts, ["-X1", "1", "2", "3", "PE", "4"]);
}

#[test]
fn wire_markers_by_number_or_destination() {
    let (mut prj, an) = demo();
    let wires: Vec<&Connection> = an.connections.iter().filter(|c| c.cable.is_empty()).collect();
    assert!(!wires.is_empty() && wires.iter().all(|c| !c.number.is_empty()));
    let markers = markers(&prj, &an.connections);
    let numbers = of(&markers, Category::Wire);
    assert_eq!(numbers.len(), wires.len(), "one entry per connection");
    assert!(numbers.iter().all(|m| m.count == 2 && m.material == "GEN.MARK-WIRE"));
    // Cable cores are marked by their cable.
    assert!(an.connections.iter().any(|c| !c.cable.is_empty()));
    let cables = of(&markers, Category::Cable);
    assert!(cables.iter().any(|m| m.text == "-W1" && m.count == 2));

    prj.marking.wire_text = WireText::Destination;
    let destination = super::markers(&prj, &an.connections);
    let ends = of(&destination, Category::Wire);
    assert_eq!(ends.len(), 2 * wires.len());
    let c = wires[0];
    let (from, to) = (pin_designation(&prj, c.a), pin_designation(&prj, c.b));
    assert_eq!((ends[0].text.as_str(), ends[1].text.as_str()), (to.as_str(), from.as_str()), "each end shows the other one");
}

#[test]
fn tags_leave_out_the_enclosure() {
    assert_eq!(marked_tag("=A1+CAB1-K1", "+CAB1"), "=A1-K1");
    assert_eq!(marked_tag("+CAB1-K1", "+CAB1"), "-K1");
    assert_eq!(marked_tag("+CAB2-K1", "+CAB1"), "+CAB2-K1");
    assert_eq!(marked_tag("-K1", ""), "-K1");
}

#[test]
fn csv_grouped_by_material() {
    let m = |category, material: &str, text: &str| Marker { category, material: material.into(), text: text.into(), count: 1, enclosure: "+CAB1".into(), group: "g".into() };
    let markers = [m(Category::Device, "DEV", "-K1"), m(Category::Wire, "WIRE", "1;2"), m(Category::Device, "DEV", "-K2")];
    let csv = csv(&markers);
    assert!(csv.starts_with("\u{feff}Material;Category;Text;Count;Enclosure;Group\n"));
    let rows: Vec<&str> = csv.lines().skip(1).collect();
    assert_eq!(rows, ["DEV;device;-K1;1;+CAB1;g", "DEV;device;-K2;1;+CAB1;g", "WIRE;wire;\"1;2\";1;+CAB1;g"]);
    assert_eq!(material(&markers), [("DEV".to_string(), 2), ("WIRE".to_string(), 1)]);
}

#[test]
fn marker_material_in_the_bill_of_materials_and_report() {
    let (prj, an) = demo();
    let markers = markers(&prj, &an.connections);
    let bom = &an.reports[&crate::model::ReportKind::BillOfMaterials];
    for (part, n) in material(&markers) {
        let row = bom.rows.iter().find(|r| r[3] == part).unwrap_or_else(|| panic!("{part} in the BOM"));
        assert_eq!(row[1], n.to_string(), "{part}");
    }
    assert_eq!(an.reports[&crate::model::ReportKind::Marking].rows.len(), markers.len());
}

#[test]
fn settings_are_stored_only_when_changed() {
    let mut prj = demo_project();
    assert!(!prj.to_json().contains("\"marking\""));
    prj.marking.wire_text = WireText::Destination;
    prj.marking.wire_material = "OWN.WIRE".into();
    let back = Project::from_json(&prj.to_json()).unwrap();
    assert_eq!(back.marking, prj.marking);
}
