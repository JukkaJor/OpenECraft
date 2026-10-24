use super::*;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use openecraft_parts::default_catalog;
use openecraft_parts::portal::{ChangeKind, Lifecycle};
use tower::ServiceExt;

const TOKEN: &str = "secret-token";

fn app(token: Option<&str>) -> Router {
    router(state(token))
}

fn state(token: Option<&str>) -> AppState {
    AppState { store: Arc::new(Mutex::new(Store::in_memory(default_catalog()))), token: token.map(String::from) }
}

async fn send(app: &Router, method: &str, uri: &str, token: Option<&str>, body: Vec<u8>) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut req = Request::builder().method(method).uri(uri).header(header::CONTENT_TYPE, "application/json");
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let resp = app.clone().oneshot(req.body(Body::from(body)).unwrap()).await.unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, headers, bytes.to_vec())
}

async fn call(app: &Router, method: &str, uri: &str, token: Option<&str>, body: &str) -> (StatusCode, String) {
    let (status, _, bytes) = send(app, method, uri, token, body.as_bytes().to_vec()).await;
    (status, String::from_utf8_lossy(&bytes).to_string())
}

async fn json<T: serde::de::DeserializeOwned>(app: &Router, method: &str, uri: &str, token: Option<&str>, body: &str) -> T {
    let (status, body) = call(app, method, uri, token, body).await;
    assert!(status.is_success(), "{method} {uri}: {status} {body}");
    serde_json::from_str(&body).unwrap()
}

/// Creates a user with the administrator token and signs in. Returns the token.
async fn sign_up(app: &Router, name: &str, role: &str, manufacturer: &str) -> String {
    let body = format!(r#"{{"role": "{role}", "manufacturer": "{manufacturer}", "password": "pw-{name}"}}"#);
    let (status, text) = call(app, "PUT", &format!("/api/v1/users/{name}"), Some(TOKEN), &body).await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let session: Session = json(app, "POST", "/api/v1/login", None, &format!(r#"{{"name": "{name}", "password": "pw-{name}"}}"#)).await;
    session.token
}

#[tokio::test]
async fn info_list_and_get() {
    let app = app(None);
    let info: ServerInfo = json(&app, "GET", "/api/v1/info", None, "").await;
    assert_eq!(info.parts, default_catalog().len());
    assert!(!info.writable);
    assert_eq!(info.storage, "json");
    assert!(info.user.is_none());

    let list: PartList = json(&app, "GET", "/api/v1/parts?q=contactor&limit=1", None, "").await;
    assert_eq!(list.parts.len(), 1);
    assert!(list.total >= 2);

    let part: Part = json(&app, "GET", "/api/v1/parts/GEN.CT-9A-24DC", None, "").await;
    assert!(!part.functions.is_empty());
    assert_eq!(part.revision, 1);

    let (status, body) = call(&app, "GET", "/api/v1/parts/NOPE", None, "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(serde_json::from_str::<ApiError>(&body).unwrap().error.contains("NOPE"));
}

#[tokio::test]
async fn search_with_facets() {
    let app = app(None);
    let r: SearchResult = json(&app, "POST", "/api/v1/search", None, r#"{"q": "contactor", "brief": true, "limit": 2}"#).await;
    assert!(r.total >= 2 && r.parts.len() == 2);
    assert!(r.facet("manufacturer").is_some_and(|f| !f.values.is_empty()));
    assert!(r.parts.iter().all(|p| p.model_3d.is_none()));
    let r: SearchResult = json(&app, "POST", "/api/v1/search", None, r#"{"numbers": ["GEN.CT-9A-24DC"]}"#).await;
    assert_eq!(r.total, 1);
    // GET list with the new filters in the query string
    let list: PartList = json(&app, "GET", "/api/v1/parts?top_group=Fluid&sort=Price&descending=true&brief=true", None, "").await;
    assert!(list.total > 0 && list.parts.iter().all(|p| p.top_group == openecraft_parts::TopGroup::Fluid));
    let q = PartQuery { lifecycle: Some(Lifecycle::Discontinued), ..Default::default() };
    let list: PartList = json(&app, "GET", &format!("/api/v1/parts{}", q.to_query_string()), None, "").await;
    assert_eq!(list.total, 0);
}

#[tokio::test]
async fn writes_need_the_token() {
    let part = r#"{"number": "TEST.1", "description": "Test"}"#;
    // Read-only server
    let ro = app(None);
    assert_eq!(call(&ro, "PUT", "/api/v1/parts/TEST.1", Some(TOKEN), part).await.0, StatusCode::FORBIDDEN);
    // Writable server: missing / wrong / correct token
    let app = app(Some(TOKEN));
    let info: ServerInfo = json(&app, "GET", "/api/v1/info", Some(TOKEN), "").await;
    assert!(info.writable && info.can_edit());
    assert_eq!(info.user.unwrap().name, ADMIN);
    assert_eq!(call(&app, "PUT", "/api/v1/parts/TEST.1", None, part).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&app, "PUT", "/api/v1/parts/TEST.1", Some("wrong"), part).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&app, "PUT", "/api/v1/parts/TEST.1", Some(TOKEN), part).await.0, StatusCode::CREATED);
    assert_eq!(call(&app, "PUT", "/api/v1/parts/TEST.1", Some(TOKEN), part).await.0, StatusCode::OK);
    // URL and body must agree, no part is its own successor
    assert_eq!(call(&app, "PUT", "/api/v1/parts/OTHER", Some(TOKEN), part).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(call(&app, "PUT", "/api/v1/parts/S", Some(TOKEN), r#"{"successor": "S"}"#).await.0, StatusCode::BAD_REQUEST);
    // Part numbers with special characters are URL encoded
    let odd = r#"{"number": "A B/1"}"#;
    assert_eq!(call(&app, "PUT", "/api/v1/parts/A%20B%2F1", Some(TOKEN), odd).await.0, StatusCode::CREATED);
    assert_eq!(call(&app, "GET", "/api/v1/parts/A%20B%2F1", None, "").await.0, StatusCode::OK);
    assert_eq!(call(&app, "GET", "/api/v1/parts/A%20B%2F1/history", None, "").await.0, StatusCode::OK);

    assert_eq!(call(&app, "DELETE", "/api/v1/parts/TEST.1", None, "").await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&app, "DELETE", "/api/v1/parts/TEST.1", Some(TOKEN), "").await.0, StatusCode::NO_CONTENT);
    assert_eq!(call(&app, "DELETE", "/api/v1/parts/TEST.1", Some(TOKEN), "").await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&app, "GET", "/api/v1/parts/TEST.1", None, "").await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn history_of_changes() {
    let app = app(Some(TOKEN));
    call(&app, "PUT", "/api/v1/parts/H.1", Some(TOKEN), r#"{"description": "one"}"#).await;
    call(&app, "PUT", "/api/v1/parts/H.1", Some(TOKEN), r#"{"description": "two"}"#).await;
    let history: Vec<HistoryEntry> = json(&app, "GET", "/api/v1/parts/H.1/history", None, "").await;
    assert_eq!(history.iter().map(|h| h.change).collect::<Vec<_>>(), [ChangeKind::Changed, ChangeKind::Created]);
    assert_eq!(history[0].user, ADMIN);
    let old: Part = json(&app, "GET", &format!("/api/v1/parts/H.1/history/{}", history[1].revision), None, "").await;
    assert_eq!(old.description, "one");
    assert_eq!(call(&app, "GET", "/api/v1/parts/H.1/history/999", None, "").await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn users_roles_and_sessions() {
    let app = app(Some(TOKEN));
    let editor = sign_up(&app, "eve", "Editor", "").await;
    let acme = sign_up(&app, "max", "Manufacturer", "ACME").await;
    let reader = sign_up(&app, "rita", "Reader", "").await;

    let info: ServerInfo = json(&app, "GET", "/api/v1/info", Some(&acme), "").await;
    assert_eq!(info.user.as_ref().unwrap().manufacturer, "ACME");
    assert!(info.may_edit("acme") && !info.may_edit("Other"));

    // Manufacturer: own parts only.
    assert_eq!(call(&app, "PUT", "/api/v1/parts/ACME.1", Some(&acme), r#"{"manufacturer": "ACME"}"#).await.0, StatusCode::CREATED);
    let (status, body) = call(&app, "PUT", "/api/v1/parts/OTHER.1", Some(&acme), r#"{"manufacturer": "Other"}"#).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(call(&app, "PUT", "/api/v1/parts/GEN.CT-9A-24DC", Some(&acme), r#"{"manufacturer": "ACME"}"#).await.0, StatusCode::FORBIDDEN);
    assert_eq!(call(&app, "DELETE", "/api/v1/parts/GEN.CT-9A-24DC", Some(&acme), "").await.0, StatusCode::FORBIDDEN);
    let csv = "Part number;Manufacturer\nACME.2;ACME\nOTHER.2;Other\n";
    assert_eq!(call(&app, "POST", "/api/v1/import", Some(&acme), csv).await.0, StatusCode::FORBIDDEN);
    // Editor: everything; reader: nothing.
    assert_eq!(call(&app, "PUT", "/api/v1/parts/OTHER.1", Some(&editor), r#"{"manufacturer": "Other"}"#).await.0, StatusCode::CREATED);
    assert_eq!(call(&app, "POST", "/api/v1/import", Some(&editor), csv).await.0, StatusCode::OK);
    assert_eq!(call(&app, "PUT", "/api/v1/parts/R.1", Some(&reader), "{}").await.0, StatusCode::FORBIDDEN);
    assert_eq!(call(&app, "POST", "/api/v1/import", Some(&reader), csv).await.0, StatusCode::FORBIDDEN);
    // Only administrators manage users.
    assert_eq!(call(&app, "GET", "/api/v1/users", Some(&editor), "").await.0, StatusCode::FORBIDDEN);
    let users: Vec<User> = json(&app, "GET", "/api/v1/users", Some(TOKEN), "").await;
    assert_eq!(users.len(), 3);

    // Wrong passwords, logout, new password ends the sessions.
    assert_eq!(call(&app, "POST", "/api/v1/login", None, r#"{"name": "eve", "password": "nope"}"#).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&app, "POST", "/api/v1/logout", Some(&reader), "").await.0, StatusCode::NO_CONTENT);
    assert_eq!(call(&app, "GET", "/api/v1/favorites", Some(&reader), "").await.0, StatusCode::UNAUTHORIZED);
    call(&app, "PUT", "/api/v1/users/eve", Some(TOKEN), r#"{"role": "Editor", "password": "new"}"#).await;
    assert_eq!(call(&app, "GET", "/api/v1/favorites", Some(&editor), "").await.0, StatusCode::UNAUTHORIZED);
    // Changing the role keeps the password.
    call(&app, "PUT", "/api/v1/users/eve", Some(TOKEN), r#"{"role": "Reader"}"#).await;
    let s: Session = json(&app, "POST", "/api/v1/login", None, r#"{"name": "EVE", "password": "new"}"#).await;
    assert_eq!(s.user.role, Role::Reader);

    // Invalid users
    assert_eq!(call(&app, "PUT", "/api/v1/users/new", Some(TOKEN), r#"{"role": "Reader"}"#).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(call(&app, "PUT", "/api/v1/users/m", Some(TOKEN), r#"{"role": "Manufacturer", "password": "x"}"#).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(call(&app, "PUT", "/api/v1/users/admin", Some(TOKEN), r#"{"role": "Reader", "password": "x"}"#).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(call(&app, "DELETE", "/api/v1/users/max", Some(TOKEN), "").await.0, StatusCode::NO_CONTENT);
    assert_eq!(call(&app, "GET", "/api/v1/favorites", Some(&acme), "").await.0, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn last_administrator_stays() {
    let state = state(None);
    {
        let mut store = state.store.lock().unwrap();
        let mut account = Account { user: User { name: "root".into(), role: Role::Admin, manufacturer: String::new() }, ..Default::default() };
        account.password = hash_password("pw").unwrap();
        store.put_account(account).unwrap();
    }
    let app = router(state);
    let s: Session = json(&app, "POST", "/api/v1/login", None, r#"{"name": "root", "password": "pw"}"#).await;
    let info: ServerInfo = json(&app, "GET", "/api/v1/info", None, "").await;
    assert!(info.writable, "administrator accounts make the server writable");
    assert_eq!(call(&app, "PUT", "/api/v1/users/root", Some(&s.token), r#"{"role": "Editor"}"#).await.0, StatusCode::CONFLICT);
    assert_eq!(call(&app, "DELETE", "/api/v1/users/root", Some(&s.token), "").await.0, StatusCode::CONFLICT);
    assert_eq!(call(&app, "PUT", "/api/v1/users/second", Some(&s.token), r#"{"role": "Admin", "password": "x"}"#).await.0, StatusCode::OK);
    assert_eq!(call(&app, "DELETE", "/api/v1/users/root", Some(&s.token), "").await.0, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn favorites_per_user() {
    let app = app(Some(TOKEN));
    let ann = sign_up(&app, "ann", "Reader", "").await;
    let bob = sign_up(&app, "bob", "Reader", "").await;
    let saved: Vec<String> = json(&app, "PUT", "/api/v1/favorites", Some(&ann), r#"["A", "", "A", "B"]"#).await;
    assert_eq!(saved, ["A", "B"]);
    let got: Vec<String> = json(&app, "GET", "/api/v1/favorites", Some(&ann), "").await;
    assert_eq!(got, ["A", "B"]);
    let got: Vec<String> = json(&app, "GET", "/api/v1/favorites", Some(&bob), "").await;
    assert!(got.is_empty());
    assert_eq!(call(&app, "GET", "/api/v1/favorites", None, "").await.0, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn catalogs() {
    let app = app(Some(TOKEN));
    let acme = sign_up(&app, "max", "Manufacturer", "ACME").await;
    let body = r#"{"name": "ACME 2026", "manufacturer": "ACME", "version": "2026.1"}"#;
    let c: Catalog = json(&app, "PUT", "/api/v1/catalogs/acme_2026", Some(&acme), body).await;
    assert_eq!((c.id.as_str(), c.parts), ("acme_2026", 0));
    assert_eq!(call(&app, "PUT", "/api/v1/catalogs/other", Some(&acme), r#"{"manufacturer": "Other"}"#).await.0, StatusCode::FORBIDDEN);
    assert_eq!(call(&app, "PUT", "/api/v1/catalogs/a%20b", Some(TOKEN), "{}").await.0, StatusCode::BAD_REQUEST);
    call(&app, "PUT", "/api/v1/parts/ACME.1", Some(&acme), r#"{"manufacturer": "ACME", "catalog": "acme_2026"}"#).await;
    let list: Vec<Catalog> = json(&app, "GET", "/api/v1/catalogs", None, "").await;
    assert_eq!(list[0].parts, 1);
    let info: ServerInfo = json(&app, "GET", "/api/v1/info", None, "").await;
    assert_eq!(info.catalogs, 1);
    // A catalog with parts is not deleted.
    assert_eq!(call(&app, "DELETE", "/api/v1/catalogs/acme_2026", Some(&acme), "").await.0, StatusCode::CONFLICT);
    call(&app, "DELETE", "/api/v1/parts/ACME.1", Some(&acme), "").await;
    assert_eq!(call(&app, "DELETE", "/api/v1/catalogs/ACME_2026", Some(&acme), "").await.0, StatusCode::NO_CONTENT);
    assert_eq!(call(&app, "GET", "/api/v1/catalogs/acme_2026", None, "").await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn attachments_and_files() {
    let app = app(Some(TOKEN));
    let (status, _, body) = send(&app, "POST", "/api/v1/parts/GEN.CT-9A-24DC/attachments?name=C%3A%5Cdocs%5Csheet.pdf", Some(TOKEN), b"%PDF-1.4".to_vec()).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let part: Part = serde_json::from_slice(&body).unwrap();
    let a = &part.attachments[0];
    assert_eq!(a.name, "sheet.pdf");
    let (status, headers, data) = send(&app, "GET", &openecraft_parts::api::file_path(&a.id), None, Vec::new()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(data, b"%PDF-1.4");
    assert_eq!(headers[header::CONTENT_TYPE], "application/pdf");
    assert!(headers[header::CONTENT_DISPOSITION].to_str().unwrap().contains("sheet.pdf"));
    // Explicit kind, empty files, unknown parts, no token
    let (status, _, body) = send(&app, "POST", "/api/v1/parts/GEN.CT-9A-24DC/attachments?name=m.json&kind=Macro", Some(TOKEN), b"{}".to_vec()).await;
    assert_eq!(status, StatusCode::OK);
    let part: Part = serde_json::from_slice(&body).unwrap();
    assert_eq!(part.attachments.len(), 2);
    assert_eq!(send(&app, "POST", "/api/v1/parts/GEN.CT-9A-24DC/attachments?name=e.pdf", Some(TOKEN), Vec::new()).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(send(&app, "POST", "/api/v1/parts/NOPE/attachments?name=e.pdf", Some(TOKEN), vec![1]).await.0, StatusCode::NOT_FOUND);
    assert_eq!(send(&app, "POST", "/api/v1/parts/GEN.CT-9A-24DC/attachments?name=e.pdf", None, vec![1]).await.0, StatusCode::UNAUTHORIZED);
    let uri = format!("/api/v1/parts/GEN.CT-9A-24DC/attachments/{}", a.id);
    let part: Part = json(&app, "DELETE", &uri, Some(TOKEN), "").await;
    assert_eq!(part.attachments.len(), 1);
    assert_eq!(call(&app, "DELETE", &uri, Some(TOKEN), "").await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&app, "GET", "/api/v1/files/unknown", None, "").await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn import_and_export() {
    let app = app(Some(TOKEN));
    let csv = "Part number;Manufacturer;Description;Functions\nIMP.1;ACME;Imported relay;Coil:A1 A2|CO:11 12 14\n";
    let result: ImportResult = json(&app, "POST", "/api/v1/import", Some(TOKEN), csv).await;
    assert_eq!(result.imported, 1);

    let part: Part = json(&app, "GET", "/api/v1/parts/IMP.1", None, "").await;
    assert_eq!(part.manufacturer, "ACME");
    assert_eq!(part.functions.len(), 2);

    // JSON part list works as well, invalid data is rejected.
    let (status, _) = call(&app, "POST", "/api/v1/import", Some(TOKEN), r#"[{"number": "IMP.2"}]"#).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(&app, "POST", "/api/v1/import", Some(TOKEN), "{ broken").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, csv) = call(&app, "GET", "/api/v1/export.csv", None, "").await;
    assert_eq!(status, StatusCode::OK);
    let parts = openecraft_parts::from_csv(&csv).unwrap();
    assert!(parts.iter().any(|p| p.number == "IMP.1"));
    assert!(parts.iter().any(|p| p.number == "IMP.2"));
}

#[tokio::test]
async fn revision_increases_on_changes() {
    let app = app(Some(TOKEN));
    let rev = |body: String| serde_json::from_str::<ServerInfo>(&body).unwrap().revision;
    let before = rev(call(&app, "GET", "/api/v1/info", None, "").await.1);
    call(&app, "PUT", "/api/v1/parts/X", Some(TOKEN), r#"{"number": "X"}"#).await;
    let after = rev(call(&app, "GET", "/api/v1/info", None, "").await.1);
    assert_eq!(after, before + 1);
    let p: Part = json(&app, "GET", "/api/v1/parts/X", None, "").await;
    assert_eq!(p.revision, after);
}

#[tokio::test]
async fn web_page_and_texts() {
    let app = app(None);
    let (status, headers, page) = send(&app, "GET", "/", None, Vec::new()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(headers[header::CONTENT_TYPE].to_str().unwrap().starts_with("text/html"));
    assert!(String::from_utf8(page).unwrap().contains("/api/v1"));
    let texts: web::WebTexts = json(&app, "GET", "/api/v1/texts?lang=fr-CA", None, "").await;
    assert_eq!(texts.lang, "fr");
    let texts: web::WebTexts = json(&app, "GET", "/api/v1/texts?lang=xx", None, "").await;
    assert_eq!(texts.lang, "en");
}

#[tokio::test]
async fn cors_preflight_allows_authorization_header() {
    let req = Request::builder()
        .method("OPTIONS")
        .uri("/api/v1/import")
        .header(header::ORIGIN, "http://localhost:8080")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "authorization,content-type")
        .body(Body::empty())
        .unwrap();
    let resp = app(Some(TOKEN)).oneshot(req).await.unwrap();
    let h = |name| resp.headers().get(name).and_then(|v| v.to_str().ok()).unwrap_or("").to_lowercase();
    assert_eq!(h(header::ACCESS_CONTROL_ALLOW_ORIGIN), "*");
    // The wildcard would not cover Authorization in browsers.
    assert!(h(header::ACCESS_CONTROL_ALLOW_HEADERS).contains("authorization"));
    assert!(h(header::ACCESS_CONTROL_ALLOW_METHODS).contains("post"));
}
