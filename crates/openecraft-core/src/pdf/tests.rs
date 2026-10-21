use super::*;
use crate::sample::demo_project;

#[test]
fn document_structure_is_valid() {
    let prj = demo_project();
    let pdf = document(&prj);
    assert!(pdf.starts_with("%PDF-1.4\n") && pdf.ends_with("%%EOF\n"));
    assert!(pdf.is_ascii());
    // startxref points at the cross reference table, whose offsets point at the objects.
    let start: usize = pdf.rsplit("startxref\n").next().unwrap().lines().next().unwrap().parse().unwrap();
    assert!(pdf[start..].starts_with("xref\n"));
    let table: Vec<&str> = pdf[start..].lines().skip(3).take_while(|l| !l.starts_with("trailer")).collect();
    for (i, entry) in table.iter().enumerate() {
        let off: usize = entry[..10].parse().unwrap();
        assert!(pdf[off..].starts_with(&format!("{} 0 obj", i + 1)), "object {}", i + 1);
    }
    // Stream lengths match.
    for part in pdf.split("<< /Length ").skip(1) {
        let len: usize = part[..part.find(' ').unwrap()].parse().unwrap();
        let body = &part[part.find("stream\n").unwrap() + 7..];
        assert!(body[len..].starts_with("endstream"));
    }
    assert_eq!(pdf.matches("/Type /Page ").count(), prj.pages.len());
    assert_eq!(pdf.matches("/Parent 4 0 R").count(), prj.pages.len(), "one bookmark per page");
}

#[test]
fn cross_references_are_links() {
    let prj = demo_project();
    let pdf = document(&prj);
    // Contacts on page 3 refer to the coil, the coil's contact image to page 2 and 3,
    // report rows to their pages.
    assert!(pdf.matches("/Subtype /Link").count() > 10);
    assert!(pdf.contains("(-K1) Tj"));
}

#[test]
fn text_encoding_and_metrics() {
    assert_eq!(pdf_string("a(b)\\"), "a\\(b\\)\\\\");
    assert_eq!(pdf_string("1.5 mm² – x"), "1.5 mm\\262 \\226 x");
    assert_eq!(pdf_string("Ω"), "?");
    assert!((text_width("A", 10.0) - 6.67).abs() < 0.01);
}
