use super::*;

#[test]
fn column_names_and_references() {
    assert_eq!(column_name(0), "A");
    assert_eq!(column_name(25), "Z");
    assert_eq!(column_name(26), "AA");
    assert_eq!(column_name(27), "AB");
    assert_eq!(column_name(701), "ZZ");
    assert_eq!(column_name(702), "AAA");
    assert_eq!(column_of("B7"), Some(1));
    assert_eq!(column_of("AA10"), Some(26));
    assert_eq!(column_of("7"), None);
}

#[test]
fn numbers_stay_numbers_and_codes_stay_texts() {
    for n in ["0", "12", "-3.5", "0.25", "1500"] {
        assert!(is_number(n), "{n}");
    }
    for t in ["007", "+5", "1e3", "1.5 mm²", "", " 1", "-K1", "3.", "1.2.3", "1234567890123456"] {
        assert!(!is_number(t), "{t}");
    }
    assert_eq!(number_text("2.0"), "2");
    assert_eq!(number_text("0.30000000000000004"), "0.30000000000000004");
    assert_eq!(number_text("1.5"), "1.5");
}

#[test]
fn round_trip_of_sheets() {
    let rows = vec![
        vec!["Key".to_string(), "Description".into(), "Length".into()],
        vec!["-K1".into(), "Pump & fan <main>\nline 2".into(), "12".into()],
        vec!["007".into(), String::new(), "1.5".into()],
    ];
    let bytes = write(&[("Devices".into(), rows.clone()), ("Bad:/name".into(), vec![vec!["a".into()]])]);
    let sheets = read(&bytes).unwrap();
    assert_eq!(sheets.len(), 2);
    assert_eq!(sheets[0].0, "Devices");
    assert_eq!(sheets[1].0, "Bad__name");
    let mut expected = rows.clone();
    expected[2][1] = String::new();
    assert_eq!(sheets[0].1, expected);
    let csv = first_sheet_csv(&bytes).unwrap();
    assert!(csv.starts_with("Key;Description;Length\n-K1;\"Pump & fan <main>\nline 2\";12\n007;;1.5\n"), "{csv}");
    assert_eq!(rows_of_csv(&csv)[1][1], "Pump & fan <main>\nline 2");
    assert!(read(b"not a zip").is_err());
    assert_eq!(sheet_name("Devices", &["devices".into()]), "Devices 2");
}

#[test]
fn shared_strings_written_by_excel_are_read() {
    let mut z = ZipWriter::new("2026-01-01");
    z.add("xl/workbook.xml", br#"<workbook xmlns:r="x"><sheets><sheet name="Tab" sheetId="1" r:id="rId1"/></sheets></workbook>"#);
    z.add("xl/_rels/workbook.xml.rels", br#"<Relationships><Relationship Id="rId1" Target="worksheets/sheet1.xml"/></Relationships>"#);
    z.add("xl/sharedStrings.xml", br#"<sst><si><t>Tag</t></si><si><r><t>Rich </t></r><r><t>text</t></r></si></sst>"#);
    z.add("xl/worksheets/sheet1.xml", br#"<worksheet><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="C1" t="s"><v>1</v></c></row><row r="3"><c r="B3"><v>4.0</v></c><c r="C3" t="b"><v>1</v></c></row></sheetData></worksheet>"#);
    let sheets = read(&z.finish()).unwrap();
    assert_eq!(sheets[0].1, vec![vec!["Tag".to_string(), String::new(), "Rich text".into()], vec![], vec![String::new(), "4".into(), "TRUE".into()]]);
}
