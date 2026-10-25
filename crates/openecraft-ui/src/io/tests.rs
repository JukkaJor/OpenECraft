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
fn a_file_opened_by_path_is_polled() {
    let path = std::env::temp_dir().join(format!("openecraft-ui-open-{}.oec", std::process::id())).display().to_string();
    write_file(&path, b"{}").unwrap();
    let io = FileIo::<()>::default();
    io.open_path(&path, ());
    io.open_path(&format!("{path}.missing"), ());
    let loaded = io.poll().unwrap();
    assert_eq!((loaded.name.as_str(), loaded.bytes.as_slice()), (path.as_str(), b"{}".as_slice()));
    assert!(io.poll().is_none());
    std::fs::remove_file(&path).unwrap();
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

#[test]
fn the_store_keeps_values_by_key() {
    let dir = std::env::temp_dir().join(format!("openecraft-ui-store-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = Store::at(dir.display().to_string());
    let ctx = egui::Context::default();
    assert!(!store.works());
    store.store("project", b"one".to_vec());
    store.store("project", b"two".to_vec());
    store.store("parts/db", vec![0, 255]);
    assert!(store.works());
    store.load(&ctx, "project");
    store.load(&ctx, "parts/db");
    store.load(&ctx, "none");
    assert_eq!(store.poll(), Some(("project".into(), Some(b"two".to_vec()))));
    assert_eq!(store.poll(), Some(("parts/db".into(), Some(vec![0, 255]))));
    assert_eq!(store.poll(), Some(("none".into(), None)));
    assert_eq!(store.poll(), None);
    // Another store of the same place sees the values (next start of the app).
    let again = Store::at(dir.display().to_string());
    again.load(&ctx, "project");
    assert_eq!(again.poll().and_then(|(_, v)| v), Some(b"two".to_vec()));
    assert!(data_dir("OpenECraft").ends_with(std::path::Path::new("OpenECraft").join("data")));
}

#[test]
fn the_print_script_goes_before_the_end_of_the_body() {
    let html = with_print_script("<html><body><p>x</p></body></html>");
    assert!(html.contains("window.print()"));
    assert!(html.find("window.print()").unwrap() < html.find("</body>").unwrap());
    assert!(with_print_script("<p>x</p>").ends_with("</script>"));
}
