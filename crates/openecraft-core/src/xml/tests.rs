use super::*;

#[test]
fn parses_elements_attributes_and_text() {
    let doc = "\u{feff}<?xml version=\"1.0\"?>\n<!DOCTYPE x [<!ENTITY a \"b\">]>\n<!-- c -->\n\
               <caex:Root xmlns:caex=\"urn:x\" a='1 &amp; 2'><Item Name=\"x&#62;y\"/><Item><![CDATA[<raw>]]> &lt;t&gt;</Item></caex:Root>";
    let root = parse(doc).unwrap();
    assert_eq!(root.name, "Root");
    assert_eq!(root.attr("a"), Some("1 & 2"));
    assert_eq!(root.attrs.len(), 1, "namespace declarations are dropped");
    let items: Vec<&Element> = root.children("Item").collect();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].attr("Name"), Some("x>y"));
    assert_eq!(items[1].text, "<raw> <t>");
    assert_eq!(root.descendants("Item").len(), 2);
}

#[test]
fn reports_broken_documents() {
    assert!(parse("<a><b></a>").is_err());
    assert!(parse("<a>").is_err());
    assert!(parse("text only").is_err());
}

#[test]
fn writer_output_parses_back() {
    let mut w = Writer::new();
    w.open("A", &[("Name", "x \"y\" & <z>")]);
    w.text("B", &[], "line 1\nline 2");
    w.empty("C", &[("v", "1")]);
    w.close("A");
    let root = parse(&w.out).unwrap();
    assert_eq!(root.attr("Name"), Some("x \"y\" & <z>"));
    assert_eq!(root.child("B").unwrap().text, "line 1\nline 2");
    assert_eq!(root.child("C").unwrap().attr("v"), Some("1"));
}

#[test]
fn deep_nesting_is_refused() {
    let doc = |n: usize| format!("{}{}", "<a>".repeat(n), "</a>".repeat(n));
    assert!(parse(&doc(MAX_DEPTH)).is_ok());
    assert!(parse(&doc(MAX_DEPTH + 1)).unwrap_err().contains("nested"));
    assert!(parse(&doc(1_000_000)).is_err());
}
