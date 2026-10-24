use super::*;

#[test]
fn base64_round_trips() {
    assert_eq!(base64_encode(b"Man"), "TWFu");
    assert_eq!(base64_encode(b"Ma"), "TWE=");
    assert_eq!(base64_encode(b"M"), "TQ==");
    for data in [&b""[..], b"M", b"Ma", b"Man", b"\x00\xff\x10binary\x80"] {
        assert_eq!(base64_decode(&base64_encode(data)).unwrap(), data);
    }
    assert_eq!(base64_decode("TW\nFu").unwrap(), b"Man");
    assert!(base64_decode("T?Fu").is_none());
}

#[test]
fn documents_know_type_and_size() {
    let doc = Document::new("Datasheet.PDF", &[1, 2, 3, 4, 5]);
    assert_eq!(doc.size(), 5);
    assert_eq!(doc.kind(), Kind::Pdf);
    assert_eq!(doc.mime(), "application/pdf");
    assert_eq!(doc.bytes().unwrap(), vec![1, 2, 3, 4, 5]);
    assert_eq!(Document::new("plan.xlsx", b"").kind(), Kind::Excel);
    assert_eq!(Document::new("notes", b"").kind(), Kind::Other);
    assert_eq!(size_text(2048), "2.0 kB");
    let broken = Document { name: "x.pdf".into(), data: String::from("!!").into() };
    assert!(broken.bytes().unwrap_err().contains("x.pdf"));
}

#[test]
fn document_pages_are_inserted_and_exported() {
    let mut prj = Project::default();
    let at = add_page(&mut prj, 0, "manual.pdf", b"%PDF-1.4 test");
    assert_eq!(at, 1);
    let pg = &prj.pages[at];
    assert_eq!((pg.kind, pg.description.as_str()), (PageKind::Document, "manual"));
    assert!(!pg.is_editable());
    let pdf = crate::pdf::document(&prj);
    assert!(pdf.contains("/Subtype /FileAttachment") && pdf.contains("/EmbeddedFiles"));
    // The file is stored as hex in the PDF.
    assert!(pdf.contains("255044462D312E342074657374>"));
    let svg = crate::svg::page_svg(&prj, at);
    assert!(svg.contains("manual.pdf"));
}
