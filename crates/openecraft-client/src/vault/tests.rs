use super::*;
use std::collections::VecDeque;

fn endpoint() -> VaultEndpoint {
    VaultEndpoint { url: "https://vault.example.com/".into(), token: " V:abc ".into(), vault: "101".into() }
}

fn response(status: u16, body: Value) -> ehttp::Response {
    ehttp::Response { url: String::new(), ok: (200..300).contains(&status), status, status_text: "Error".into(), headers: Default::default(), bytes: serde_json::to_vec(&body).unwrap() }
}

/// Answers requests in order; each expected request is a method and a part of its URL.
struct Script {
    steps: VecDeque<(ehttp::Method, &'static str, u16, Value)>,
    seen: Vec<ehttp::Request>,
}

impl Script {
    fn new(steps: Vec<(ehttp::Method, &'static str, u16, Value)>) -> Script {
        Script { steps: steps.into(), seen: Vec::new() }
    }

    fn fetch(&mut self, req: &ehttp::Request) -> ehttp::Result<ehttp::Response> {
        let (method, url, status, body) = self.steps.pop_front().unwrap_or_else(|| panic!("unexpected request {}", req.url));
        assert_eq!(req.method, method, "{}", req.url);
        assert!(req.url.contains(url), "{} does not contain {url}", req.url);
        self.seen.push(req.clone());
        Ok(response(status, body))
    }
}

fn body(req: &ehttp::Request) -> Value {
    serde_json::from_slice(&req.body).unwrap()
}

#[test]
fn requests_of_the_api() {
    let e = endpoint();
    assert_eq!(e.server_info().url, "https://vault.example.com/AutodeskDM/Services/api/vault/v2/server-info");
    assert_eq!(e.session().url, "https://vault.example.com/AutodeskDM/Services/api/vault/v2/sessions/@current");
    assert!(e.contents(ROOT).url.starts_with("https://vault.example.com/AutodeskDM/Services/api/vault/v2/vaults/101/folders/root/contents?option[latestOnly]=true"));
    assert!(e.search("Plant 1", Some("7")).url.contains("/vaults/101/folders/7/contents?q=Plant%201&option[searchSubFolders]=true"));
    assert!(e.search("motor", None).url.contains("/vaults/101/search-results?q=motor"));
    assert!(e.items("GEN.CT-9").url.contains("/vaults/101/item-versions?q=GEN.CT-9&option[propDefIds]=all"));
    assert!(e.download("55").url.ends_with("/vaults/101/file-versions/55/content"));
    assert!(e.checkout("12").url.ends_with("/vaults/101/files/12:checkout"));
    assert!(e.undo_checkout("12").url.ends_with("/vaults/101/files/12:undo-checkout"));
    // Every request carries the token; the API path is not doubled when it was pasted.
    assert_eq!(e.contents(ROOT).headers.get("Authorization"), Some("Bearer V:abc"));
    let pasted = VaultEndpoint { url: "https://vault.example.com/AutodeskDM/Services/api/vault/v2/".into(), ..endpoint() };
    assert_eq!(pasted.server_info().url, e.server_info().url);
    // Next pages come as server paths.
    assert_eq!(e.next("/AutodeskDM/Services/api/vault/v2/vaults/101/item-versions?cursorState=x").url, "https://vault.example.com/AutodeskDM/Services/api/vault/v2/vaults/101/item-versions?cursorState=x");
    // The token goes to the Vault server only.
    assert!(e.next("/vaults/101/item-versions?cursorState=x").headers.get("Authorization").is_some());
    assert!(e.next("https://vault.example.com/AutodeskDM/x").headers.get("Authorization").is_some());
    let elsewhere = e.next("https://evil.example/collect");
    assert_eq!(elsewhere.url, "https://evil.example/collect");
    assert_eq!(elsewhere.headers.get("Authorization"), None);
    // Actions have no body and no content type.
    assert!(e.checkout("12").body.is_empty());
    assert_eq!(e.checkout("12").headers.get("Content-Type"), None);
}

#[test]
fn signing_in_sends_the_vault_account() {
    let req = endpoint().sign_in(" Vault ", "anna", "pw");
    assert_eq!(req.method, ehttp::Method::POST);
    assert!(req.url.ends_with("/v2/sessions"));
    assert_eq!(req.headers.get("Authorization"), None);
    assert_eq!(req.headers.get_all("Content-Type").collect::<Vec<_>>(), ["application/json"]);
    assert_eq!(body(&req), json!({ "input": { "vault": "Vault", "userName": "anna", "password": "pw", "appCode": APP_CODE } }));
    let session: Session = decode(Ok(response(200, json!({ "id": "9", "accessToken": "V:xyz", "vaultInformation": { "id": "101", "name": "Vault" }, "userInformation": { "name": "anna" } })))).unwrap();
    assert_eq!(session.access_token, "V:xyz");
    assert_eq!(session.vault_information.unwrap().id, "101");
}

#[test]
fn entries_of_folders_and_searches() {
    let page = json!({
        "pagination": { "limit": 100, "totalResults": 5, "nextUrl": "/next" },
        "results": [
            { "entityType": "Folder", "id": "7", "name": "Projects", "fullName": "$/Projects" },
            { "entityType": "FileVersion", "id": "55", "name": "Plant.oec", "version": 3, "revision": "B", "state": "Work in Progress", "isCheckedOut": true, "file": { "id": "12" } },
            { "entityType": "ItemVersion", "id": "8", "number": "100023", "title": "Contactor", "revision": "A", "version": 2 },
            { "entityType": "ChangeOrder", "id": "3", "number": "ECO-1" },
            { "id": "56", "name": "Panel.stp", "parentFolderId": "7" }
        ]
    });
    let e = entries(Ok(response(200, page))).unwrap();
    assert_eq!(e.pagination.total_results, 5);
    assert_eq!(e.pagination.next_url.as_deref(), Some("/next"));
    let names: Vec<&str> = e.results.iter().map(Entry::name).collect();
    assert_eq!(names, ["Projects", "Plant.oec", "100023", "Panel.stp"]);
    let Entry::File(f) = &e.results[1] else { panic!("not a file") };
    assert_eq!((f.file_id(), f.version_number(), f.is_checked_out), ("12", 3, true));
    assert!(matches!(&e.results[0], Entry::Folder(f) if f.full_name == "$/Projects"));
}

#[test]
fn errors_are_readable() {
    let err = decode::<Session>(Ok(response(401, json!({ "statusCode": 401, "errorCode": "Unauthorized", "title": "Unauthorized", "detail": "The user name or password is wrong." })))).unwrap_err();
    assert_eq!(err, "Unauthorized: The user name or password is wrong.");
    let plain = ehttp::Response { bytes: b"<html>".to_vec(), ..response(502, json!(null)) };
    assert_eq!(check(Ok(plain)).unwrap_err(), "502 Error");
    assert!(check(Err("no route".into())).unwrap_err().contains("no route"));
}

fn session(id: &str, token: &str, part_size: u64) -> Value {
    json!({ "uploadId": id, "uploadSessionToken": token, "state": "open", "maxPartSize": part_size })
}

#[test]
fn a_new_file_is_added_in_parts() {
    let data: Vec<u8> = (0..20).collect();
    let upload = Upload::new(endpoint(), Target::Folder("7".into()), "Plant.oec", data.clone(), "first release");
    let other = json!({ "pagination": { "nextUrl": "/AutodeskDM/Services/api/vault/v2/vaults/101/folders/7/contents?cursorState=2" }, "results": [{ "entityType": "FileVersion", "id": "1", "name": "Other.oec", "file": { "id": "2" } }] });
    let mut script = Script::new(vec![
        (ehttp::Method::GET, "/folders/7/contents", 200, other),
        (ehttp::Method::GET, "cursorState=2", 200, json!({ "results": [] })),
        (ehttp::Method::POST, "/vaults/101/file-uploads", 201, session("u1", "t0", 8)),
        (ehttp::Method::PUT, "/file-uploads/u1/parts/0", 200, session("u1", "t1", 8)),
        (ehttp::Method::PUT, "/file-uploads/u1/parts/1", 200, session("u1", "t2", 8)),
        (ehttp::Method::PUT, "/file-uploads/u1/parts/2", 200, session("u1", "t3", 8)),
        (ehttp::Method::POST, "/file-uploads/u1:complete", 200, json!({ "uploadId": "u1", "state": "completed", "uploadTicket": "ticket" })),
        (ehttp::Method::POST, "/vaults/101/files", 201, json!({ "id": "40", "fileVersion": { "id": "41", "name": "Plant.oec", "version": 1 } })),
    ]);
    assert!(upload.status().contains("Plant.oec"));
    let done = run_blocking(upload, |r| script.fetch(r)).unwrap();
    assert!(done.added);
    assert_eq!((done.version.id.as_str(), done.version.file_id()), ("41", "40"));
    let seen = &script.seen;
    // Every part continues the session of the answer before.
    let tokens: Vec<&str> = seen[3..7].iter().map(|r| r.headers.get("X-Vault-Upload-Session").unwrap()).collect();
    assert_eq!(tokens, ["t0", "t1", "t2", "t3"]);
    let parts: Vec<u8> = seen[3..6].iter().flat_map(|r| r.body.clone()).collect();
    assert_eq!(parts, data);
    assert_eq!(seen[3].headers.get("Content-Type"), Some("application/octet-stream"));
    assert_eq!(body(&seen[2]), json!({ "fileName": "Plant.oec" }));
    assert_eq!(body(&seen[7]), json!({ "folderId": 7, "uploadTicket": "ticket", "name": "Plant.oec", "comment": "first release" }));
}

#[test]
fn a_file_of_the_same_name_gets_a_new_version() {
    let upload = Upload::new(endpoint(), Target::Folder(ROOT.into()), "plant.OEC", b"{}".to_vec(), "");
    let listing = json!({ "results": [{ "entityType": "FileVersion", "id": "55", "name": "Plant.oec", "isCheckedOut": false, "file": { "id": "12" } }] });
    let mut script = Script::new(vec![
        (ehttp::Method::GET, "/folders/root/contents", 200, listing),
        (ehttp::Method::POST, "/files/12:checkout", 200, json!({ "id": "55" })),
        (ehttp::Method::POST, "/file-uploads", 201, session("u2", "a", 1 << 30)),
        (ehttp::Method::PUT, "/file-uploads/u2/parts/0", 200, session("u2", "b", 1 << 30)),
        (ehttp::Method::POST, "/file-uploads/u2:complete", 200, json!({ "uploadTicket": "t" })),
        (ehttp::Method::POST, "/files/12:checkin", 201, json!({ "id": "57", "name": "Plant.oec", "version": 4, "file": { "id": "12" } })),
    ]);
    let done = run_blocking(upload, |r| script.fetch(r)).unwrap();
    assert!(!done.added);
    assert_eq!(done.version.version_number(), 4);
    assert_eq!(body(script.seen.last().unwrap()), json!({ "uploadTicket": "t", "keepCheckedOut": false }));
}

#[test]
fn a_failed_upload_undoes_the_checkout() {
    let upload = Upload::new(endpoint(), Target::File("12".into()), "", b"x".to_vec(), "");
    let mut script = Script::new(vec![
        (ehttp::Method::GET, "/vaults/101/files/12", 200, json!({ "id": "12", "fileVersion": { "id": "55", "name": "Panel.stp", "isCheckedOut": false } })),
        (ehttp::Method::POST, "/files/12:checkout", 200, json!({})),
        (ehttp::Method::POST, "/file-uploads", 507, json!({ "title": "Insufficient Storage", "detail": "The file store is full." })),
        (ehttp::Method::POST, "/files/12:undo-checkout", 200, json!({})),
    ]);
    let err = run_blocking(upload, |r| script.fetch(r)).unwrap_err();
    assert_eq!(err, "Insufficient Storage: The file store is full.");
    // The name of the file in the vault is used when none is given.
    assert_eq!(body(&script.seen[2]), json!({ "fileName": "Panel.stp" }));
    assert!(script.steps.is_empty());
}

#[test]
fn a_file_checked_out_already_is_not_checked_out_again() {
    let upload = Upload::new(endpoint(), Target::File("12".into()), "Panel.stp", Vec::new(), "");
    let mut script = Script::new(vec![
        (ehttp::Method::GET, "/files/12", 200, json!({ "id": "12", "fileVersion": { "isCheckedOut": true } })),
        (ehttp::Method::POST, "/file-uploads", 201, session("u3", "a", 100)),
        // An empty file is one empty part.
        (ehttp::Method::PUT, "/parts/0", 200, session("u3", "b", 100)),
        (ehttp::Method::POST, ":complete", 200, json!({ "uploadTicket": "t" })),
        (ehttp::Method::POST, "/files/12:checkin", 200, json!({ "id": "55" })),
    ]);
    run_blocking(upload, |r| script.fetch(r)).unwrap();
    assert!(script.seen[2].body.is_empty());
}

fn item(number: &str, version: f64, part_number: Option<&str>) -> Value {
    let props: Vec<Value> = part_number.map(|p| json!({ "propertyDefinitionId": "5", "definition": { "displayName": "Part Number", "systemName": "PartNumber" }, "value": p })).into_iter().collect();
    json!({ "id": format!("{number}-{version}"), "number": number, "revision": format!("R{version}"), "state": "Released", "version": version, "properties": props })
}

#[test]
fn items_are_found_by_number_or_part_number() {
    let job = ItemLookup::new(endpoint(), ["B-2".to_string(), "A-1".into(), " ".into(), "C-3".into(), "A-1".into()]).unwrap();
    assert!(job.status().contains("0 of 3"));
    let mut script = Script::new(vec![
        (ehttp::Method::GET, "q=A-1", 200, json!({ "results": [item("A-1", 1.0, None), item("A-1", 2.0, None), item("A-10", 1.0, None)] })),
        (ehttp::Method::GET, "q=B-2", 200, json!({ "results": [item("100023", 1.0, Some("B-2"))] })),
        (ehttp::Method::GET, "q=C-3", 200, json!({ "results": [item("100024", 1.0, Some("C-30"))] })),
    ]);
    let found = run_blocking(job, |r| script.fetch(r)).unwrap();
    let pairs: Vec<(&str, &str, &str)> = found.iter().map(|(n, i)| (n.as_str(), i.number.as_str(), i.revision.as_str())).collect();
    assert_eq!(pairs, [("A-1", "A-1", "R2"), ("B-2", "100023", "R1")]);
    let records: Value = serde_json::from_str(&item_records(&found)).unwrap();
    assert_eq!(records[1], json!({ "part_number": "B-2", "item_id": "100023", "revision": "R1", "lifecycle": "Released" }));
    assert!(ItemLookup::new(endpoint(), Vec::new()).is_err());
}
