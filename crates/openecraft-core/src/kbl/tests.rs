use super::*;
use crate::harness::{Fixing, FixingKind, PartKind, Protection, ProtectionKind};
use crate::model::harness::{End, ExtraPart};
use crate::sample::demo_project;
use crate::xml::Element;
use std::collections::{BTreeMap, BTreeSet};

/// The KBL 2.4 SR-1 schema of prostep ivip (MIT license, see the file header).
const XSD: &str = include_str!("kbl24_sr1.xsd");

struct Seq {
    name: String,
    ty: String,
    min: usize,
    max: usize,
}

#[derive(Default)]
struct ComplexType {
    base: Option<String>,
    abstract_: bool,
    seq: Vec<Seq>,
    required: Vec<String>,
}

/// The parts of the schema the validator needs: complex types with their element
/// sequence (base types first) and required attributes, enumerations of simple types.
struct Schema {
    types: BTreeMap<String, ComplexType>,
    enums: BTreeMap<String, Vec<String>>,
}

fn local(s: &str) -> String {
    s.rsplit(':').next().unwrap_or(s).to_string()
}

impl Schema {
    fn parse() -> Schema {
        let root = crate::xml::parse(XSD).unwrap();
        let mut types = BTreeMap::new();
        for ct in root.children("complexType") {
            let mut t = ComplexType { abstract_: ct.attr("abstract") == Some("true"), ..Default::default() };
            let ext = ct.descendants("extension").into_iter().next();
            t.base = ext.and_then(|e| e.attr("base")).map(local);
            for e in ct.descendants("element") {
                let max = match e.attr("maxOccurs").unwrap_or("1") {
                    "unbounded" => usize::MAX,
                    n => n.parse().unwrap(),
                };
                t.seq.push(Seq { name: e.attr("name").unwrap().into(), ty: e.attr("type").unwrap_or("xs:string").into(), min: e.attr("minOccurs").unwrap_or("1").parse().unwrap(), max });
            }
            for a in ct.descendants("attribute") {
                if a.attr("use") == Some("required") {
                    t.required.push(a.attr("name").unwrap().into());
                }
            }
            types.insert(ct.attr("name").unwrap().to_string(), t);
        }
        let mut enums = BTreeMap::new();
        for st in root.children("simpleType") {
            enums.insert(st.attr("name").unwrap().to_string(), st.descendants("enumeration").iter().filter_map(|e| e.attr("value")).map(String::from).collect());
        }
        Schema { types, enums }
    }

    /// Sequence of a type with the elements of its base types first.
    fn sequence(&self, ty: &str) -> Vec<&Seq> {
        let t = &self.types[ty];
        let mut v = t.base.as_deref().map(|b| self.sequence(b)).unwrap_or_default();
        v.extend(t.seq.iter());
        v
    }

    fn required(&self, ty: &str) -> Vec<&str> {
        let t = &self.types[ty];
        let mut v = t.base.as_deref().map(|b| self.required(b)).unwrap_or_default();
        v.extend(t.required.iter().map(String::as_str));
        v
    }

    fn derives_from(&self, ty: &str, base: &str) -> bool {
        ty == base || self.types.get(ty).and_then(|t| t.base.as_deref()).is_some_and(|b| self.derives_from(b, base))
    }
}

#[derive(Default)]
struct Validation {
    errors: Vec<String>,
    ids: BTreeSet<String>,
    refs: Vec<(String, String)>,
}

fn is_ncname(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_') && chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn validate(schema: &Schema, el: &Element, declared: &str, path: &str, v: &mut Validation) {
    let ty = match el.attr("type") {
        Some(x) => {
            let x = local(x);
            if !schema.derives_from(&x, declared) {
                v.errors.push(format!("{path}: xsi:type {x} is not a {declared}"));
            }
            x
        }
        None => declared.to_string(),
    };
    if schema.types[&ty].abstract_ {
        v.errors.push(format!("{path}: abstract type {ty} without xsi:type"));
        return;
    }
    for a in schema.required(&ty) {
        match el.attr(a) {
            None => v.errors.push(format!("{path}: missing attribute {a}")),
            Some(id) if a == "id" && (!is_ncname(id) || !v.ids.insert(id.to_string())) => v.errors.push(format!("{path}: id {id} is not unique or not a name")),
            _ => {}
        }
    }
    let seq = schema.sequence(&ty);
    let mut k = 0;
    for s in &seq {
        let mut n = 0;
        while k < el.children.len() && el.children[k].name == s.name {
            let child = &el.children[k];
            let p = format!("{path}/{}", s.name);
            check_value(schema, child, &s.ty, &p, v);
            k += 1;
            n += 1;
        }
        if n < s.min || n > s.max {
            v.errors.push(format!("{path}: {} occurs {n} times ({}..{})", s.name, s.min, if s.max == usize::MAX { "*".into() } else { s.max.to_string() }));
        }
    }
    if k < el.children.len() {
        v.errors.push(format!("{path}: unexpected {} (type {ty})", el.children[k].name));
    }
}

fn check_value(schema: &Schema, el: &Element, ty: &str, path: &str, v: &mut Validation) {
    let text = el.text.trim();
    match ty {
        "xs:string" => {}
        "xs:double" => {
            if text.parse::<f64>().is_err() {
                v.errors.push(format!("{path}: {text} is not a number"));
            }
        }
        "xs:integer" => {
            if text.parse::<i64>().is_err() {
                v.errors.push(format!("{path}: {text} is not an integer"));
            }
        }
        "xs:IDREF" => v.refs.push((path.into(), text.into())),
        "xs:IDREFS" => {
            if text.is_empty() {
                v.errors.push(format!("{path}: empty id list"));
            }
            v.refs.extend(text.split_whitespace().map(|r| (path.to_string(), r.to_string())));
        }
        other => {
            let name = local(other);
            if let Some(values) = schema.enums.get(&name) {
                if !values.is_empty() && !values.iter().any(|x| x == text) {
                    v.errors.push(format!("{path}: {text} is not one of {name}"));
                }
            } else {
                validate(schema, el, &name, path, v);
            }
        }
    }
}

/// Errors of a KBL document against the schema: element order and occurrences,
/// required ids (unique), values of numbers and enumerations, dangling references.
fn errors(kbl: &str) -> Vec<String> {
    let schema = Schema::parse();
    let root = crate::xml::parse(kbl).unwrap();
    assert_eq!(root.name, "KBL_container");
    let mut v = Validation::default();
    validate(&schema, &root, "KBL_container", "KBL_container", &mut v);
    for (path, r) in &v.refs {
        if !v.ids.contains(r) {
            v.errors.push(format!("{path}: reference {r} to nothing"));
        }
    }
    v.errors
}

#[test]
fn the_schema_is_read() {
    let s = Schema::parse();
    assert!(s.types.len() > 90);
    let names: Vec<&str> = s.sequence("Wire_occurrence").iter().map(|x| x.name.as_str()).collect();
    assert_eq!(names, ["Part", "Installation_information", "Length_information", "Wire_number"]);
    assert!(s.enums["Connector_usage"].contains(&"splice".to_string()));
    // The validator finds what is wrong.
    let bad = "<kbl:KBL_container xmlns:kbl=\"x\" id=\"K\" version_id=\"2.4\"><Node id=\"U\"><Id>N</Id><Cartesian_point>CPX</Cartesian_point></Node><Unit id=\"U\"><Si_prefix>mega</Si_prefix></Unit></kbl:KBL_container>";
    let e = errors(bad);
    assert!(e.iter().any(|x| x.contains("mega is not one of SI_prefix")), "{e:?}");
    assert!(e.iter().any(|x| x.contains("id U is not unique")), "{e:?}");
    assert!(e.iter().any(|x| x.contains("reference CPX to nothing")), "{e:?}");
    let order = errors("<kbl:KBL_container xmlns:kbl=\"x\" id=\"K\" version_id=\"2.4\"><Unit id=\"U\"/><Node id=\"N1\"><Id>N</Id><Cartesian_point>U</Cartesian_point></Node></kbl:KBL_container>");
    assert!(order.iter().any(|x| x.contains("unexpected Node")), "{order:?}");
    let missing = errors("<kbl:KBL_container xmlns:kbl=\"x\" id=\"K\" version_id=\"2.4\"><Node id=\"N1\"><Id>N</Id></Node></kbl:KBL_container>");
    assert!(missing.iter().any(|x| x.contains("Cartesian_point occurs 0 times")), "{missing:?}");
}

#[test]
fn the_demo_harness_is_valid_kbl() {
    let prj = demo_project();
    let an = Analysis::checked(&prj);
    let x = export(&prj, &an, None).unwrap();
    assert!(x.starts_with("<?xml"));
    let e = errors(&x);
    assert!(e.is_empty(), "{}", e.join("\n"));
    let ws = harness::wires(&prj, &an, "-WH1");
    let cons = harness::connectors(&ws);
    assert_eq!(x.matches("<Harness ").count(), 1);
    assert_eq!(x.matches("<Connection ").count(), ws.len());
    assert_eq!(x.matches("<Connector_occurrence ").count(), cons.len());
    assert_eq!(x.matches("xsi:type=\"kbl:Wire_occurrence\"").count(), ws.iter().filter(|w| w.cable.is_empty()).count());
    assert!(x.contains("<Id>-WH1</Id>") || x.contains("<Abbreviation>-WH1</Abbreviation>"));
    // 3D topology: nodes, segments with centre curves, the routing of every wire.
    let t = topology::of(&prj, &an, "-WH1").unwrap();
    assert_eq!(x.matches("<Node ").count(), t.nodes.len());
    assert_eq!(x.matches("<Segment ").count(), t.segments.len());
    assert_eq!(x.matches("<Routing ").count(), t.routes.iter().filter(|r| r.is_some()).count());
    assert!(x.contains("<Center_curve "));
    assert!(x.contains("<Unit_component>U_MM</Unit_component>"));
}

fn dressed() -> Project {
    let mut prj = demo_project();
    let an = Analysis::checked(&prj);
    topology::store(&mut prj, &an, "-WH1").unwrap();
    let h = &mut prj.harnesses[0];
    h.part = "WH-4711".into();
    let seg = h.segments[0].id.clone();
    let node = h.nodes[0].id.clone();
    h.nodes[0].bend_radius = 40.0;
    h.protections.push(Protection { id: "P1".into(), kind: ProtectionKind::Tape, part: "TAPE-19".into(), segment: String::new(), start: 0.0, end: 1.0 });
    h.protections.push(Protection { id: "P2".into(), kind: ProtectionKind::CorrugatedTube, part: String::new(), segment: seg.clone(), start: 0.2, end: 0.8 });
    h.fixings.push(Fixing { id: "F1".into(), kind: FixingKind::CableTie, part: "TIE-200".into(), segment: seg, location: 0.5 });
    let device = h.nodes.iter().find(|n| !n.connectors.is_empty()).unwrap().connectors[0].clone();
    h.ends.push(End { device: device.clone(), terminal: "CRIMP-1".into(), seal: "SEAL-1".into(), cavity_plug: "PLUG-1".into(), strip: 5.0, usage: "dangler".into(), ..Default::default() });
    for (kind, part) in [(PartKind::Accessory, "LABEL-1"), (PartKind::CoPack, "BAG-1"), (PartKind::Assembly, "BRK-1"), (PartKind::Component, "DIODE-1")] {
        h.parts.push(ExtraPart { kind, part: part.into(), count: 2, description: "extra".into(), node: node.clone() });
    }
    prj.info.date = "2026-10-25".into();
    prj.languages.targets.push("de".into());
    prj.languages.dictionary.entry(prj.harnesses[0].description.clone()).or_default().insert("de".into(), "Türkabelbaum".into());
    prj
}

#[test]
fn every_kind_of_harness_content_is_valid_kbl() {
    let prj = dressed();
    let an = Analysis::checked(&prj);
    let x = export(&prj, &an, Some("-WH1")).unwrap();
    let e = errors(&x);
    assert!(e.is_empty(), "{}", e.join("\n"));
    for element in [
        "<Wire_protection ", "<Wire_protection_occurrence ", "<Protection_area ", "<Fixing ", "<Fixing_occurrence ", "<Fixing_assignment ", "<General_terminal ", "<Terminal_occurrence ",
        "<Cavity_seal ", "<Cavity_seal_occurrence ", "<Accessory ", "<Accessory_occurrence ", "<Co_pack_part ", "<Co_pack_occurrence ", "<Assembly_part ", "<Assembly_part_occurrence ",
        "<Component ", "<Component_occurrence ", "<Localized_description ", "<Schematic_connection ", "<Dimension_specification ", "<Default_dimension_specification ", "<Placement ",
        "<Bend_radius ", "<Creation ", "<External_reference ",
    ] {
        assert!(x.contains(element), "{element} missing");
    }
    assert!(x.contains("<Usage>dangler</Usage>"));
    assert!(x.contains("<Value>Türkabelbaum</Value>"));
    assert!(x.contains("<Model_year>2026</Model_year>"));
    assert!(x.contains("<Part_number>WH-4711</Part_number>"));
    // Two of each further part.
    assert_eq!(x.matches("<Accessory_occurrence ").count(), 2);
    // The stored topology is used as it is.
    assert_eq!(x.matches("<Node ").count(), prj.harnesses[0].nodes.len());
}

#[test]
fn cables_and_options() {
    let mut prj = dressed();
    crate::testkit::with_cable(&mut prj);
    let an = Analysis::checked(&prj);
    let ws = harness::wires(&prj, &an, "-WH1");
    assert!(ws.iter().any(|w| w.cable == "-W77"));
    let sym = prj.pages.iter().flat_map(|p| p.symbols.iter()).find(|s| s.tag == ws[0].from.0).unwrap().id;
    prj.model_options.options.push(crate::options::ModelOption { name: "LIGHT".into(), description: "Lamp option".into(), members: [sym].into_iter().collect(), ..Default::default() });
    let an = Analysis::checked(&prj);
    let x = export(&prj, &an, None).unwrap();
    let e = errors(&x);
    assert!(e.is_empty(), "{}", e.join("\n"));
    assert!(x.contains("xsi:type=\"kbl:Special_wire_occurrence\""));
    assert!(x.contains("<Core_occurrence "));
    assert!(x.contains("<Cable_designator>-W77</Cable_designator>"));
    assert!(x.contains("<Module "));
    assert!(x.contains("<Harness_configuration "));
}

#[test]
fn several_harnesses() {
    let mut prj = demo_project();
    let id = prj.new_id();
    prj.harnesses.push(Harness { id, tag: "-WH2".into(), ..Default::default() });
    let an = Analysis::checked(&prj);
    assert!(export(&prj, &an, None).unwrap_err().contains("-WH1, -WH2"));
    assert!(export(&prj, &an, Some("-WH9")).is_err());
    // A harness without wires is still a valid container.
    let empty = export(&prj, &an, Some("-WH2")).unwrap();
    assert!(errors(&empty).is_empty(), "{:?}", errors(&empty));
    let files = all(&prj, &an);
    assert_eq!(files.iter().map(|f| f.0.as_str()).collect::<Vec<_>>(), ["WH1.kbl", "WH2.kbl"]);
    let z = crate::zip::read_zip(&zip(&prj, &an)).unwrap();
    assert_eq!(z.len(), 2);
}

#[test]
fn numbers_and_tolerances() {
    assert_eq!(f(1.5), "1.5");
    assert_eq!(f(2.0), "2");
    assert_eq!(f(-0.0001), "0");
    assert_eq!(f(0.1234), "0.123");
    assert_eq!(tolerance(100.0), 5.0);
    assert_eq!(tolerance(2000.0), 15.0);
    assert_eq!(tolerance(1e7), 20.0);
    assert_eq!(year("25.10.2026"), "2026");
    assert_eq!(year(""), "");
}
