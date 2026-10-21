use super::*;
use crate::model::PageKind;

fn page(function: &str, location: &str) -> Page {
    let mut p = Page::new(1, "3", "", PageKind::Schematic);
    p.function = function.into();
    p.location = location.into();
    p
}

#[test]
fn tags_are_split_into_aspects() {
    let dt = Dt::parse("=A1+CAB1-K1");
    assert_eq!((dt.function.as_str(), dt.location.as_str(), dt.name.as_str()), ("A1", "CAB1", "-K1"));
    assert_eq!(dt.tag(), "=A1+CAB1-K1");
    // Either order, missing aspects, plain names
    assert_eq!(Dt::parse("+CAB1=A1-K1").tag(), "=A1+CAB1-K1");
    assert_eq!(Dt::parse("+CAB1-X1"), Dt { location: "CAB1".into(), name: "-X1".into(), ..Default::default() });
    assert_eq!(Dt::parse("-K1"), Dt { name: "-K1".into(), ..Default::default() });
    assert_eq!(Dt::parse("K1").name, "K1");
    assert_eq!(Dt::parse("=A1.2").function, "A1.2");
    assert_eq!(Dt::parse("=A1").structure(), "=A1");
}

#[test]
fn page_structure_completes_and_shortens_tags() {
    let pg = page("A1", "CAB1");
    assert_eq!(page_full_name(&pg), "=A1+CAB1/3");
    assert_eq!(page_full_name(&page("", "")), "3");
    assert_eq!(complete(&pg, "-K1"), "=A1+CAB1-K1");
    assert_eq!(complete(&pg, "+FIELD-S1"), "=A1+FIELD-S1");
    assert_eq!(complete(&pg, "=B2+X-K1"), "=B2+X-K1");
    assert_eq!(relative(&pg, "=A1+CAB1-K1"), "-K1");
    assert_eq!(relative(&pg, "=A1+FIELD-S1"), "+FIELD-S1");
    assert_eq!(relative(&pg, "=B2+CAB1-K1"), "=B2-K1");
    // Without page structure nothing changes.
    let plain = page("", "");
    assert_eq!(complete(&plain, "-K1"), "-K1");
    assert_eq!(relative(&plain, "=A1-K1"), "=A1-K1");
}

#[test]
fn aspects_are_cleaned() {
    assert_eq!(clean_aspect(" =A1 "), "A1");
    assert_eq!(clean_aspect("+CAB 1"), "CAB1");
    assert_eq!(structure("", "CAB1"), "+CAB1");
}
