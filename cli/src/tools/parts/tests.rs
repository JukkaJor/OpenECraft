use super::*;
use crate::parts_server::Server;
use openecraft_client::Endpoint;
use crate::testing::{part, run, session_with, temp_dir, FakeServer, URL};
use openecraft_parts::to_csv;

fn server() -> FakeServer {
    let server = FakeServer::default();
    server.parts.borrow_mut().extend([part("ACME.K1", "Contactors"), part("ACME.F1", "Fuses"), part("ACME.K2", "Contactors")]);
    server
}

#[test]
fn search_and_get() {
    let mut s = session_with(&server());
    assert_eq!(run(&mut s, "parts_connect", json!({})).unwrap()["info"]["parts"], 3);
    let hits = run(&mut s, "parts_search", json!({ "group": "Contactors", "limit": 1 })).unwrap();
    assert_eq!(hits["total"], 2);
    assert_eq!(hits["parts"], json!([{ "number": "ACME.K1", "manufacturer": "ACME", "type_number": "", "description": "", "group": "Contactors" }]));
    let full = run(&mut s, "parts_search", json!({ "query": "F1", "full": true })).unwrap();
    assert!(full["parts"][0].get("price").is_some());
    assert_eq!(run(&mut s, "parts_get", json!({ "number": "ACME.F1" })).unwrap()["group"], "Fuses");
    assert_eq!(run(&mut s, "parts_get", json!({ "number": "NOPE" })).unwrap_err(), "Unknown part: NOPE");
}

#[test]
fn save_rename_and_delete() {
    let srv = server();
    let mut s = session_with(&srv);
    run(&mut s, "parts_save", json!({ "part": { "number": "ACME.X1", "description": "New" } })).unwrap();
    assert!(srv.parts.borrow().iter().any(|p| p.number == "ACME.X1" && p.description == "New"));
    assert!(run(&mut s, "parts_save", json!({ "part": { "description": "no number" } })).is_err());
    assert!(run(&mut s, "parts_save", json!({ "part": { "number": "A", "price": "cheap" } })).unwrap_err().starts_with("Not a valid part"));

    assert_eq!(run(&mut s, "parts_rename", json!({ "number": "ACME.X1", "new_number": "ACME.K1" })).unwrap_err(), "Part number already exists");
    run(&mut s, "parts_rename", json!({ "number": "ACME.X1", "new_number": "ACME.X2" })).unwrap();
    let numbers: Vec<String> = srv.parts.borrow().iter().map(|p| p.number.clone()).collect();
    assert!(numbers.contains(&"ACME.X2".to_string()) && !numbers.contains(&"ACME.X1".to_string()));

    run(&mut s, "parts_delete", json!({ "number": "ACME.X2" })).unwrap();
    assert!(run(&mut s, "parts_delete", json!({ "number": "ACME.X2" })).is_err());
}

#[test]
fn changes_need_the_token() {
    let srv = server();
    let mut s = session_with(&srv);
    s.server.endpoint.token = String::new();
    assert_eq!(run(&mut s, "parts_delete", json!({ "number": "ACME.K1" })).unwrap_err(), "invalid token");
    run(&mut s, "parts_connect", json!({ "token": crate::testing::TOKEN })).unwrap();
    run(&mut s, "parts_delete", json!({ "number": "ACME.K1" })).unwrap();
}

#[test]
fn unreachable_server() {
    let mut s = Session::new(Server { endpoint: Endpoint { url: "http://elsewhere".into(), token: String::new() }, transport: Box::new(server()) });
    let err = run(&mut s, "parts_search", json!({})).unwrap_err();
    assert!(err.starts_with("connection failed"), "{err}");
    run(&mut s, "parts_connect", json!({ "url": URL })).unwrap();
}

#[test]
fn import_and_export_files() {
    let dir = temp_dir("parts-files");
    let srv = server();
    let mut s = session_with(&srv);
    let file = dir.join("new.csv");
    std::fs::write(&file, to_csv(&[part("ACME.N1", "New"), part("ACME.N2", "New")])).unwrap();
    assert_eq!(run(&mut s, "parts_import", json!({ "path": file.to_str().unwrap() })).unwrap()["imported"], 2);
    assert_eq!(srv.parts.borrow().len(), 5);

    let csv = run(&mut s, "parts_export", json!({})).unwrap();
    assert!(csv.as_str().unwrap().contains("ACME.N2"));
    let out = dir.join("all.csv");
    run(&mut s, "parts_export", json!({ "path": out.to_str().unwrap() })).unwrap();
    assert_eq!(openecraft_parts::from_csv(&std::fs::read_to_string(out).unwrap()).unwrap().len(), 5);
}

#[test]
fn devices_get_parts_from_the_server() {
    let mut s = session_with(&server());
    run(&mut s, "new_project", json!({})).unwrap();
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "coil", "x": 100, "y": 100 })).unwrap();
    run(&mut s, "assign_part", json!({ "tag": "-K1", "number": "ACME.K2" })).unwrap();
    assert!(s.project().unwrap().parts.iter().any(|p| p.number == "ACME.K2"));
    assert_eq!(run(&mut s, "assign_part", json!({ "tag": "-K1", "number": "NOPE" })).unwrap_err(), "Unknown part: NOPE");
}
