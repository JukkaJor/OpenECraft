use super::*;
use crate::sample::demo_project;

/// A PDF of `objs` (number, body) with a classic cross reference table.
fn pdf(objs: &[(u32, String)]) -> Vec<u8> {
    let mut out = String::from("%PDF-1.4\n");
    for (n, body) in objs {
        out.push_str(&format!("{n} 0 obj\n{body}\nendobj\n"));
    }
    out.push_str("trailer\n<< /Root 1 0 R >>\n%%EOF\n");
    out.into_bytes()
}

/// Two A4 landscape pages; page 1 has a note with a reply, a rectangle, a line, a
/// highlight without text and a link.
fn commented() -> Vec<(u32, String)> {
    vec![
        (1, "<< /Type /Catalog /Pages 2 0 R >>".into()),
        (2, "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".into()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 841.89 595.28] /Annots [10 0 R 11 0 R 12 0 R 13 0 R 14 0 R 15 0 R 16 0 R] >>".into()),
        (4, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 841.89 595.28] >>".into()),
        (10, "<< /Type /Annot /Subtype /Text /Rect [72 523.28 92 543.28] /Contents (Use a 2.5 mm\\262 wire \\(K1\\)) /T (Anna) /M (D:20261024101500+02'00') /Popup 11 0 R >>".into()),
        (11, "<< /Type /Annot /Subtype /Popup /Rect [100 400 300 500] /Parent 10 0 R >>".into()),
        (12, "<< /Type /Annot /Subtype /Text /Rect [72 523.28 92 543.28] /Contents <FEFF004F004B00200020D83DDC4D> /T (Ben) /IRT 10 0 R >>".into()),
        (13, "<< /Type /Annot /Subtype /Square /Rect [141.73 311.81 283.46 453.54] /Contents (Move this) /T (Anna) /CreationDate (D:20261023) >>".into()),
        (14, "<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /L [0 595.28 28.35 566.93] /T (Anna) >>".into()),
        (15, "<< /Type /Annot /Subtype /Highlight /Rect [10 10 20 20] >>".into()),
        (16, "<< /Type /Annot /Subtype /Link /Rect [0 0 10 10] /A << /S /URI /URI (https://example.com) >> >>".into()),
    ]
}

#[test]
fn reads_the_comments_of_the_pages_in_millimetres() {
    let a = read(&pdf(&commented())).unwrap();
    let kinds: Vec<&str> = a.iter().map(|x| x.kind.as_str()).collect();
    assert_eq!(kinds, ["Text", "Text", "Square", "Line", "Highlight"], "popups and links are no comments");
    let note = &a[0];
    assert_eq!(note.text, "Use a 2.5 mm² wire (K1)");
    assert_eq!((note.author.as_str(), note.date.as_str()), ("Anna", "2026-10-24"));
    assert!((note.rect[0] - 25.4).abs() < 0.01 && (note.rect[1] - 18.34).abs() < 0.05, "{:?}", note.rect);
    assert_eq!(a[1].text, "OK  \u{1F44D}", "UTF-16 with surrogates");
    assert_eq!(a[1].reply_to, Some(10));
    assert_eq!(a[2].date, "2026-10-23");
    let sq = a[2].rect;
    assert!((sq[0] - 50.0).abs() < 0.05 && (sq[2] - 100.0).abs() < 0.05 && sq[1] < sq[3]);
    let l = a[3].line.unwrap();
    assert!(l[0].abs() < 0.01 && l[1].abs() < 0.01 && (l[2] - 10.0).abs() < 0.01 && (l[3] - 10.0).abs() < 0.01);
}

#[test]
fn imports_comments_replies_and_shapes_once() {
    let mut prj = demo_project();
    let bytes = pdf(&commented());
    let res = import(&mut prj, &bytes, "me", "2026-10-25").unwrap();
    assert_eq!(res, Imported { comments: 4, replies: 1, markups: 2, skipped: 0, outside: 0 });
    let page = prj.pages[0].id;
    let c: Vec<_> = prj.review.live_comments().filter(|c| c.page == page).collect();
    assert_eq!(c.len(), 4);
    let note = c.iter().find(|c| c.author == "Anna" && c.text.starts_with("Use")).unwrap();
    assert_eq!(note.replies.len(), 1);
    assert_eq!(note.replies[0].author, "Ben");
    let hl = c.iter().find(|c| c.text == "Highlight").unwrap();
    assert_eq!((hl.author.as_str(), hl.date.as_str()), ("me", "2026-10-25"), "defaults where the PDF has none");
    assert_eq!(prj.review.live_markups().count(), 2);
    let again = import(&mut prj, &bytes, "me", "2026-10-25").unwrap();
    assert_eq!(again, Imported { comments: 0, replies: 0, markups: 0, skipped: 4, outside: 0 });
}

#[test]
fn comments_on_pages_the_project_does_not_have_are_counted() {
    let mut prj = demo_project();
    prj.pages.clear();
    let res = import(&mut prj, &pdf(&commented()), "me", "2026-10-25").unwrap();
    assert_eq!(res.outside, 4);
    assert_eq!(res.comments, 0);
}

#[test]
fn incremental_updates_replace_objects() {
    let mut objs = commented();
    // An update that removes all comments of page 1 but the square.
    objs.push((3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 841.89 595.28] /Annots [13 0 R] >>".into()));
    let a = read(&pdf(&objs)).unwrap();
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].kind, "Square");
}

#[test]
fn compressed_object_streams_are_read() {
    let all = commented();
    let (plain, packed): (Vec<_>, Vec<_>) = all.into_iter().partition(|(n, _)| *n < 10);
    let mut header = String::new();
    let mut body = String::new();
    for (n, b) in &packed {
        header.push_str(&format!("{n} {} ", body.len()));
        body.push_str(b);
        body.push('\n');
    }
    let data = format!("{header}\n{body}");
    let first = header.len() + 1;
    let z = miniz_oxide::deflate::compress_to_vec_zlib(data.as_bytes(), 6);
    let mut out = pdf(&plain);
    let stm = format!("20 0 obj\n<< /Type /ObjStm /N {} /First {first} /Filter /FlateDecode /Length {} >>\nstream\n", packed.len(), z.len());
    out.extend_from_slice(stm.as_bytes());
    out.extend_from_slice(&z);
    out.extend_from_slice(b"\nendstream\nendobj\n%%EOF\n");
    let a = read(&out).unwrap();
    assert_eq!(a.len(), 5);
    assert_eq!(a[2].text, "Move this");
}

#[test]
fn the_pdf_export_reads_back_without_comments() {
    let prj = demo_project();
    let an = crate::analysis::Analysis::checked(&prj);
    let pdf = crate::pdf::document_with(&prj, &an);
    let a = read(pdf.as_bytes()).unwrap();
    assert!(a.is_empty(), "cross reference links are no comments: {a:?}");
}

#[test]
fn other_files_are_refused() {
    assert!(read(b"PK\x03\x04").is_err());
    assert!(read(b"%PDF-1.4\n%%EOF").is_err());
}

#[test]
fn dates_and_strings() {
    assert_eq!(date_of("D:20261025143000+01'00'"), "2026-10-25");
    assert_eq!(date_of("yesterday"), "");
    assert_eq!(text_of(b"a\r\nb\rc"), "a\nb\nc");
    assert_eq!(text_of(&[0xEF, 0xBB, 0xBF, b'x']), "x");
    let mut p = Parser { b: b"/A#20B", i: 0 };
    assert_eq!(p.obj(0), Some(Obj::Name("A B".into())));
}
