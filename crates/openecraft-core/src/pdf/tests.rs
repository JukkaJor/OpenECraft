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

fn png(color_type: u8) -> Vec<u8> {
    let mut v = b"\x89PNG\r\n\x1a\n".to_vec();
    let chunk = |v: &mut Vec<u8>, kind: &[u8], data: &[u8]| {
        v.extend_from_slice(&(data.len() as u32).to_be_bytes());
        v.extend_from_slice(kind);
        v.extend_from_slice(data);
        v.extend_from_slice(&[0, 0, 0, 0]);
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&2u32.to_be_bytes());
    ihdr.extend_from_slice(&1u32.to_be_bytes());
    ihdr.extend_from_slice(&[8, color_type, 0, 0, 0]);
    chunk(&mut v, b"IHDR", &ihdr);
    chunk(&mut v, b"IDAT", &[0x78, 0x9C, 0xAB]);
    chunk(&mut v, b"IEND", &[]);
    v
}

#[test]
fn images_become_xobjects() {
    let mut prj = crate::model::Project::default();
    crate::graphics::add_image(&mut prj, 0, crate::geom::p(20.0, 20.0), "logo.png", &png(2), 40.0, "");
    let jpeg = [0xFF, 0xD8, 0xFF, 0xC0, 0, 11, 8, 0, 1, 0, 2, 3, 0, 0, 0, 0xFF, 0xD9];
    crate::graphics::add_image(&mut prj, 0, crate::geom::p(80.0, 20.0), "photo.jpg", &jpeg, 40.0, "");
    // With transparency the image is drawn as a frame with its name.
    crate::graphics::add_image(&mut prj, 0, crate::geom::p(140.0, 20.0), "alpha.png", &png(6), 40.0, "");
    let pdf = document(&prj);
    assert!(pdf.is_ascii());
    assert_eq!(pdf.matches("/Subtype /Image").count(), 2);
    assert!(pdf.contains("/Predictor 15 /Colors 3"));
    assert!(pdf.contains("/DCTDecode"));
    assert!(pdf.contains("/XObject << /Im0 "));
    assert!(pdf.contains("(alpha.png) Tj"));
}
