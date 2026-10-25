use super::*;
use crate::testing::*;

#[test]
fn pages_are_added_renamed_and_deleted() {
    let mut s = new_session();
    assert_eq!(run(&mut s, "add_page", json!({ "kind": "schematic" })).unwrap()["name"], "2");
    let bom = run(&mut s, "add_page", json!({ "kind": "bill-of-materials", "after": "1", "name": "BOM" })).unwrap();
    assert_eq!(bom["kind"], "bill_of_materials");
    let names: Vec<Value> = run(&mut s, "list_pages", json!({})).unwrap().as_array().unwrap().iter().map(|p| p["name"].clone()).collect();
    assert_eq!(names, vec![json!("1"), json!("BOM"), json!("2")]);

    assert!(run(&mut s, "add_page", json!({ "kind": "drawing" })).is_err());
    assert!(run(&mut s, "set_page", json!({ "page": "2", "name": "BOM" })).unwrap_err().contains("already exists"));
    let pg = run(&mut s, "set_page", json!({ "page": "2", "name": "10", "function": "=A1" })).unwrap();
    assert_eq!(pg["function"], "A1");

    run(&mut s, "delete_page", json!({ "page": "BOM" })).unwrap();
    run(&mut s, "delete_page", json!({ "page": "10" })).unwrap();
    assert!(run(&mut s, "delete_page", json!({ "page": "1" })).unwrap_err().contains("last page"));
}

#[test]
fn released_pages_are_not_deleted() {
    let mut s = new_session();
    run(&mut s, "add_page", json!({ "kind": "schematic" })).unwrap();
    s.project_mut().unwrap().pages[1].locked = true;
    assert_eq!(run(&mut s, "delete_page", json!({ "page": "2" })).unwrap_err(), "Released pages cannot be deleted");
    assert!(run(&mut s, "set_page", json!({ "page": "2", "name": "9" })).unwrap_err().contains("locked"));
}

#[test]
fn dxf_drawings_are_imported_as_pages() {
    let mut s = session();
    run(&mut s, "new_project", json!({ "demo": true })).unwrap();
    let file = temp_dir("dxf-import").join("old drawing.dxf");
    run(&mut s, "export", json!({ "format": "dxf", "page": "2", "path": file.to_str().unwrap() })).unwrap();
    let pages = s.project().unwrap().pages.len();
    let r = run(&mut s, "import_dxf", json!({ "path": file.to_str().unwrap(), "after": "1" })).unwrap();
    assert_eq!((r["name"].as_str(), r["description"].as_str()), (Some(s.project().unwrap().pages[1].name.as_str()), Some("old drawing")));
    assert!(r["lines"].as_u64().unwrap() > 50 && r["texts"].as_u64().unwrap() > 5);
    assert_eq!(s.project().unwrap().pages.len(), pages + 1);
    std::fs::write(&file, "not a drawing").unwrap();
    assert!(run(&mut s, "import_dxf", json!({ "path": file.to_str().unwrap() })).unwrap_err().contains("Not a DXF file"));
}

#[test]
fn dwg_drawings_are_imported_as_pages() {
    let mut s = session();
    run(&mut s, "new_project", json!({ "demo": true })).unwrap();
    let dir = temp_dir("dwg-import");
    let (dwg, dxf) = (dir.join("old drawing.dwg"), dir.join("old drawing.dxf"));
    run(&mut s, "export", json!({ "format": "dwg", "page": "2", "path": dwg.to_str().unwrap() })).unwrap();
    run(&mut s, "export", json!({ "format": "dxf", "page": "2", "path": dxf.to_str().unwrap() })).unwrap();
    let pages = s.project().unwrap().pages.len();
    let r = run(&mut s, "import_dwg", json!({ "path": dwg.to_str().unwrap(), "after": "1", "description": "Plant" })).unwrap();
    assert_eq!(r["description"], "Plant");
    let from_dxf = run(&mut s, "import_dxf", json!({ "path": dxf.to_str().unwrap() })).unwrap();
    assert_eq!((&r["lines"], &r["texts"]), (&from_dxf["lines"], &from_dxf["texts"]));
    assert_eq!(s.project().unwrap().pages.len(), pages + 2);
    std::fs::write(&dwg, b"AC1018 damaged").unwrap();
    assert!(run(&mut s, "import_dwg", json!({ "path": dwg.to_str().unwrap() })).unwrap_err().contains("DWG"));
}

#[test]
fn pdf_comments_are_imported_into_the_review() {
    let mut s = session();
    run(&mut s, "new_project", json!({ "demo": true })).unwrap();
    let file = temp_dir("pdf-comments").join("reviewed.pdf");
    let body = "%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 841.89 595.28] /Annots [4 0 R] >>\nendobj\n\
4 0 obj\n<< /Type /Annot /Subtype /Text /Rect [72 500 92 520] /Contents (Check fuse) /T (Anna) >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n";
    std::fs::write(&file, body).unwrap();
    let r = run(&mut s, "import_pdf_comments", json!({ "path": file.to_str().unwrap(), "author": "me" })).unwrap();
    assert_eq!((r["comments"].as_u64(), r["skipped"].as_u64()), (Some(1), Some(0)));
    let again = run(&mut s, "import_pdf_comments", json!({ "path": file.to_str().unwrap() })).unwrap();
    assert_eq!(again["skipped"], 1);
    std::fs::write(&file, "no pdf").unwrap();
    assert!(run(&mut s, "import_pdf_comments", json!({ "path": file.to_str().unwrap() })).unwrap_err().contains("Not a PDF file"));
}
