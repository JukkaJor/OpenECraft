use super::*;

#[test]
fn short_names_and_mime_types() {
    assert_eq!(short_name("C:\\models\\part.obj"), "part.obj");
    assert_eq!(short_name("/home/x/front.svg"), "front.svg");
    assert_eq!(short_name("plain.csv"), "plain.csv");
    assert_eq!(mime_type("csv"), "text/csv");
    assert_eq!(mime_type("json"), "application/json");
    assert_eq!(mime_type("igs"), "model/iges");
    assert_eq!(mime_type("jt"), "application/octet-stream");
}

#[test]
fn loaded_text_is_lossy() {
    let l = Loaded { purpose: (), name: "a".into(), bytes: vec![b'o', b'k', 0xff] };
    assert!(l.text().starts_with("ok"));
}

#[test]
fn files_are_found_read_and_written_by_path() {
    let dir = std::env::temp_dir().join(format!("openecraft-ui-files-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    let a = dir.join("a.oec").display().to_string();
    let b = dir.join("sub").join("b.oec").display().to_string();
    write_file(&a, b"1").unwrap();
    write_file(&b, b"2").unwrap();
    write_file(&dir.join("c.txt").display().to_string(), b"3").unwrap();
    let found = files_in(&dir.display().to_string(), |p| p.ends_with(".oec"));
    assert_eq!(found, vec![a.clone(), b]);
    assert_eq!(read_file(&a).unwrap(), b"1");
    assert!(read_file(&dir.join("none").display().to_string()).is_err());
    assert!(has_file_system());
    assert_eq!(mime_type("oecz"), "application/octet-stream");
}
