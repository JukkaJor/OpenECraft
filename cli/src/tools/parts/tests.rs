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
fn search_with_portal_filters_and_facets() {
    let srv = server();
    {
        let mut parts = srv.parts.borrow_mut();
        parts[0].voltage = "24 V DC".into();
        parts[0].catalog = "acme_2026".into();
        parts[2].lifecycle = openecraft_parts::portal::Lifecycle::Discontinued;
        parts[2].successor = "ACME.K1".into();
    }
    let mut s = session_with(&srv);
    let hits = run(&mut s, "parts_search", json!({ "attributes": { "Voltage": "24 V DC" } })).unwrap();
    assert_eq!(hits["total"], 1);
    assert_eq!(hits["parts"][0]["catalog"], "acme_2026");
    let hits = run(&mut s, "parts_search", json!({ "lifecycle": "discontinued" })).unwrap();
    assert_eq!(hits["parts"][0]["successor"], "ACME.K1");
    let hits = run(&mut s, "parts_search", json!({ "sort": "Number", "descending": true, "facets": true })).unwrap();
    assert_eq!(hits["parts"][0]["number"], "ACME.K2");
    assert_eq!(hits["facets"]["group"], json!([["Contactors", 2], ["Fuses", 1]]));
    assert_eq!(hits["attributes"]["Voltage"], json!([["24 V DC", 1]]));
    let err = run(&mut s, "parts_search", json!({ "lifecycle": "old" })).unwrap_err();
    assert!(err.contains("Active, PhaseOut, Discontinued"), "{err}");
}

#[test]
fn sign_in_favorites_and_users() {
    let srv = server();
    let mut s = session_with(&srv);
    s.server.endpoint.token = String::new();
    assert_eq!(run(&mut s, "parts_connect", json!({ "user": "ann", "password": "no" })).unwrap_err(), "wrong user name or password");
    run(&mut s, "parts_connect", json!({ "user": "ann", "password": "pw" })).unwrap();
    assert_eq!(s.server.endpoint.token, crate::testing::TOKEN);
    assert_eq!(run(&mut s, "parts_favorites", json!({ "add": "ACME.K1" })).unwrap(), json!(["ACME.K1"]));
    assert_eq!(run(&mut s, "parts_favorites", json!({ "add": "ACME.K2", "remove": "ACME.K1" })).unwrap(), json!(["ACME.K2"]));
    assert_eq!(run(&mut s, "parts_favorites", json!({})).unwrap(), json!(["ACME.K2"]));

    let u = run(&mut s, "parts_user_save", json!({ "name": "max", "role": "manufacturer", "manufacturer": "ACME", "password": "x" })).unwrap();
    assert_eq!(u["role"], "Manufacturer");
    assert!(run(&mut s, "parts_user_save", json!({ "name": "max", "role": "boss" })).is_err());
    assert_eq!(run(&mut s, "parts_users", json!({})).unwrap()[0]["name"], "max");
    run(&mut s, "parts_user_delete", json!({ "name": "max" })).unwrap();
    assert!(srv.users.borrow().is_empty());
}

#[test]
fn catalogs_files_and_history() {
    let dir = temp_dir("parts-portal");
    let srv = server();
    let mut s = session_with(&srv);
    let c = run(&mut s, "parts_catalog_save", json!({ "catalog": { "name": "ACME 2026", "manufacturer": "ACME" } })).unwrap();
    assert_eq!(c["id"], "acme_2026");
    assert!(run(&mut s, "parts_catalog_save", json!({ "catalog": {} })).is_err());
    assert_eq!(run(&mut s, "parts_catalogs", json!({})).unwrap()[0]["name"], "ACME 2026");
    run(&mut s, "parts_catalog_delete", json!({ "id": "acme_2026" })).unwrap();
    assert!(srv.catalogs.borrow().is_empty());

    let sheet = dir.join("k1 sheet.pdf");
    std::fs::write(&sheet, b"%PDF").unwrap();
    let r = run(&mut s, "parts_attach", json!({ "number": "ACME.K1", "path": sheet.to_str().unwrap() })).unwrap();
    assert_eq!(r["attachments"][0]["name"], "k1 sheet.pdf");
    assert_eq!(r["attachments"][0]["kind"], "Datasheet");
    let id = r["attachments"][0]["id"].as_str().unwrap().to_string();
    let out = dir.join("copy.pdf");
    assert_eq!(run(&mut s, "parts_file", json!({ "id": id, "path": out.to_str().unwrap() })).unwrap()["bytes"], 4);
    assert_eq!(std::fs::read(&out).unwrap(), b"%PDF");
    let r = run(&mut s, "parts_attach", json!({ "number": "ACME.K1", "path": sheet.to_str().unwrap(), "kind": "manual" })).unwrap();
    assert_eq!(r["attachments"][1]["kind"], "Manual");
    let r = run(&mut s, "parts_detach", json!({ "number": "ACME.K1", "id": id })).unwrap();
    assert_eq!(r["attachments"].as_array().unwrap().len(), 1);

    assert_eq!(run(&mut s, "parts_history", json!({ "number": "ACME.K1" })).unwrap()[0]["change"], "Created");
    assert_eq!(run(&mut s, "parts_history", json!({ "number": "ACME.K1", "revision": 1 })).unwrap()["number"], "ACME.K1");
    assert!(run(&mut s, "parts_history", json!({ "number": "ACME.K1", "revision": 7 })).is_err());
}

#[test]
fn project_parts_are_checked_for_updates() {
    let srv = server();
    let mut s = session_with(&srv);
    run(&mut s, "new_project", json!({})).unwrap();
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "coil", "x": 100, "y": 100 })).unwrap();
    run(&mut s, "assign_part", json!({ "tag": "-K1", "number": "ACME.K2" })).unwrap();
    assert_eq!(run(&mut s, "parts_check_updates", json!({})).unwrap()["updates"], json!([]));
    {
        let mut parts = srv.parts.borrow_mut();
        let k2 = parts.iter_mut().find(|p| p.number == "ACME.K2").unwrap();
        k2.description = "newer".into();
        k2.lifecycle = openecraft_parts::portal::Lifecycle::PhaseOut;
        k2.successor = "ACME.K1".into();
        k2.revision = 5;
    }
    let r = run(&mut s, "parts_check_updates", json!({})).unwrap();
    assert_eq!(r["updates"][0]["kind"], "PhaseOut");
    assert_eq!(r["updates"][0]["successor"], "ACME.K1");
    assert_eq!(r["applied"], 0);
    assert_eq!(run(&mut s, "parts_check_updates", json!({ "apply": true })).unwrap()["applied"], 1);
    let k2 = s.project().unwrap().parts.iter().find(|p| p.number == "ACME.K2").unwrap().clone();
    assert_eq!(k2.description, "newer");
    assert_eq!(run(&mut s, "parts_check_updates", json!({})).unwrap()["updates"], json!([]));
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
