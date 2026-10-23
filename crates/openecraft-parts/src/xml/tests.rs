use super::*;

#[test]
fn elements_attributes_text_and_entities() {
    let doc = parse(
        "\u{feff}<?xml version=\"1.0\"?>\n<!DOCTYPE BMECAT SYSTEM \"bmecat.dtd\">\n<!-- c -->\
         <bm:ROOT xmlns:bm=\"x\" a='1' b=\"x &amp; y\"><A>T&lt;1&#x41;&#66;</A><B/><C><![CDATA[<raw>]]></C></bm:ROOT>",
    )
    .unwrap();
    assert_eq!(doc.name, "ROOT");
    assert_eq!((doc.attribute("a"), doc.attribute("b")), (Some("1"), Some("x & y")));
    assert_eq!(doc.text_of("A"), "T<1AB");
    assert!(doc.child("B").is_some());
    assert_eq!(doc.text_of("C"), "<raw>");
    assert_eq!(doc.text_of("D"), "");
    let mut v = Vec::new();
    doc.descendants("A", &mut v);
    assert_eq!(v.len(), 1);
}

#[test]
fn broken_documents_are_refused() {
    assert!(parse("<A><B></A>").is_err());
    assert!(parse("<A>").is_err());
    assert!(parse("just text").is_err());
    assert_eq!(unescape("a &unknown; b &amp"), "a &unknown; b &amp");
}
