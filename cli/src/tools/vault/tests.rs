use super::*;
use crate::testing::*;

fn connected(vault: &FakeVault) -> Session {
    let mut s = session_with_vault(vault);
    run(&mut s, "vault_connect", json!({ "url": VAULT_URL, "user": "anna", "password": "pw" })).unwrap();
    s
}

#[test]
fn signing_in_and_tokens() {
    let vault = FakeVault::default();
    let mut s = session_with_vault(&vault);
    let st = run(&mut s, "vault_connect", json!({})).unwrap();
    assert_eq!(st["connected"], false);
    let err = run(&mut s, "vault_browse", json!({})).unwrap_err();
    assert!(err.contains("vault_connect"), "{err}");
    assert!(run(&mut s, "vault_connect", json!({ "user": "anna" })).unwrap_err().contains("url"));
    let err = run(&mut s, "vault_connect", json!({ "url": VAULT_URL, "user": "anna", "password": "wrong" })).unwrap_err();
    assert!(err.contains("password is wrong"), "{err}");
    let st = run(&mut s, "vault_connect", json!({ "url": VAULT_URL, "user": "anna", "password": "pw" })).unwrap();
    assert_eq!((st["connected"].as_bool(), st["vault_id"].as_str(), st["user"].as_str()), (Some(true), Some("1"), Some("anna")));
    assert_eq!(st["server"]["version"], "31.0.0");
    assert_eq!(st["token"], VAULT_TOKEN);

    // An access token (e.g. of an Autodesk account) finds its vault through the session.
    let mut t = session_with_vault(&vault);
    let st = run(&mut t, "vault_connect", json!({ "url": format!("{VAULT_URL}/AutodeskDM/Services/api/vault/v2"), "token": VAULT_TOKEN })).unwrap();
    assert_eq!((st["connected"].as_bool(), st["vault"].as_str()), (Some(true), Some("Vault")));
}

#[test]
fn browsing_and_searching() {
    let vault = FakeVault::default();
    vault.folders.borrow_mut().push(("7".into(), "1".into(), "Projects".into()));
    vault.add("1", "Readme.txt", b"hello");
    vault.add("7", "Plant.oec", b"{}");
    let mut s = connected(&vault);
    let root = run(&mut s, "vault_browse", json!({})).unwrap();
    let names: Vec<&str> = root["entries"].as_array().unwrap().iter().filter_map(|e| e["name"].as_str()).collect();
    assert_eq!(names, ["Projects", "Readme.txt"]);
    assert_eq!(root["entries"][0]["kind"], "folder");
    let projects = run(&mut s, "vault_browse", json!({ "folder": "7" })).unwrap();
    let file = &projects["entries"][0];
    assert_eq!((file["kind"].as_str(), file["name"].as_str(), file["version"].as_u64()), (Some("file"), Some("Plant.oec"), Some(1)));
    let found = run(&mut s, "vault_browse", json!({ "query": "plant" })).unwrap();
    assert_eq!(found["entries"].as_array().unwrap().len(), 1);
    let path = temp_dir("vault_download").join("readme.txt");
    let id = root["entries"][1]["version_id"].as_str().unwrap();
    run(&mut s, "vault_download", json!({ "version": id, "path": path.to_str().unwrap() })).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"hello");
}

#[test]
fn a_project_opened_from_the_vault_is_checked_in_as_a_new_version() {
    let vault = FakeVault::default();
    let mut d = session();
    run(&mut d, "new_project", json!({ "name": "Plant 1", "demo": true })).unwrap();
    let file = vault.add("7", "Plant.oec", d.project().unwrap().to_json().as_bytes());
    let version = vault.files.borrow()[0].versions[0].0.clone();
    let mut s = connected(&vault);
    let opened = run(&mut s, "vault_open", json!({ "version": version })).unwrap();
    assert_eq!(opened["project"], "Plant 1");
    let pages = opened["pages"].as_u64().unwrap();
    assert!(pages > 0);
    run(&mut s, "set_project_info", json!({ "description": "Changed in the field" })).unwrap();
    let up = run(&mut s, "vault_upload", json!({ "comment": "field changes" })).unwrap();
    assert_eq!((up["added"].as_bool(), up["file_id"].as_str(), up["version"].as_u64()), (Some(false), Some(file.as_str()), Some(2)));
    let files = vault.files.borrow();
    assert!(!files[0].checked_out);
    let stored = String::from_utf8(files[0].versions[1].1.clone()).unwrap();
    assert!(stored.contains("Changed in the field"));
}

#[test]
fn exports_and_files_are_added() {
    let vault = FakeVault::default();
    let mut s = connected(&vault);
    run(&mut s, "new_project", json!({ "name": "Demo", "demo": true })).unwrap();
    let err = run(&mut s, "vault_upload", json!({})).unwrap_err();
    assert!(err.contains("folder"), "{err}");
    let up = run(&mut s, "vault_upload", json!({ "folder": "root", "format": "wire_list" })).unwrap();
    assert_eq!((up["added"].as_bool(), up["name"].as_str()), (Some(true), Some("Demo.csv")));
    let step = run(&mut s, "vault_upload", json!({ "folder": "root", "format": "step", "name": "Panel.stp" })).unwrap();
    assert_eq!(step["name"], "Panel.stp");
    let up = run(&mut s, "vault_upload", json!({ "folder": "root", "connector": "pdm" })).unwrap();
    assert_eq!(up["name"], "Demo_pdm.json");
    let path = temp_dir("vault_upload").join("notes.txt");
    std::fs::write(&path, "0123456789").unwrap();
    run(&mut s, "vault_upload", json!({ "folder": "root", "path": path.to_str().unwrap() })).unwrap();
    // The same file again is a new version of it.
    let again = run(&mut s, "vault_upload", json!({ "folder": "root", "path": path.to_str().unwrap() })).unwrap();
    assert_eq!(again["added"], false);
    let files = vault.files.borrow();
    let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["Demo.csv", "Panel.stp", "Demo_pdm.json", "notes.txt"]);
    assert!(String::from_utf8_lossy(&files[1].versions[0].1).contains("ISO-10303-21"));
    assert_eq!(files[3].versions.len(), 2);
    assert_eq!(files[3].versions[1].1, b"0123456789");
    drop(files);
    let err = run(&mut s, "vault_upload", json!({ "file": "999", "path": path.to_str().unwrap() })).unwrap_err();
    assert!(err.contains("File not found"), "{err}");
}

#[test]
fn item_links_are_taken_over_from_vault_items() {
    let vault = FakeVault::default();
    let mut s = connected(&vault);
    run(&mut s, "new_project", json!({ "name": "Demo", "demo": true })).unwrap();
    let numbers: Vec<String> = s.project().unwrap().parts.iter().map(|p| p.number.clone()).collect();
    assert!(numbers.len() >= 2);
    vault.items.borrow_mut().extend([
        json!({ "id": "1", "number": numbers[0], "revision": "B", "state": "Released", "version": 3 }),
        json!({ "id": "2", "number": "100023", "revision": "A", "state": "Work in Progress", "version": 1,
                "properties": [{ "propertyDefinitionId": "5", "definition": { "displayName": "Part Number", "systemName": "PartNumber" }, "value": numbers[1] }] }),
    ]);
    let preview = run(&mut s, "vault_sync_items", json!({})).unwrap();
    assert_eq!(preview["items"], 2);
    assert!(preview["applied"].is_null());
    assert!(!preview["changes"].as_array().unwrap().is_empty());
    let applied = run(&mut s, "vault_sync_items", json!({ "apply": true, "date": "2026-10-25" })).unwrap();
    assert!(applied["applied"].as_u64().unwrap() >= 4);
    let items = &s.project().unwrap().integration.items;
    assert_eq!((items[&numbers[0]].item_id.as_str(), items[&numbers[0]].revision.as_str()), (numbers[0].as_str(), "B"));
    assert_eq!((items[&numbers[1]].item_id.as_str(), items[&numbers[1]].lifecycle.as_str()), ("100023", "Work in Progress"));
}
