use super::*;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use openecraft_parts::default_catalog;
use tower::ServiceExt;

const TOKEN: &str = "secret-token";

fn app(token: Option<&str>) -> Router {
    router(AppState {
        store: Arc::new(RwLock::new(Store::in_memory(default_catalog()))),
        token: token.map(String::from),
    })
}

async fn call(app: &Router, method: &str, uri: &str, token: Option<&str>, body: &str) -> (StatusCode, String) {
    let mut req = Request::builder().method(method).uri(uri).header(header::CONTENT_TYPE, "application/json");
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let resp = app.clone().oneshot(req.body(Body::from(body.to_string())).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

#[tokio::test]
async fn info_list_and_get() {
    let app = app(None);
    let (status, body) = call(&app, "GET", "/api/v1/info", None, "").await;
    assert_eq!(status, StatusCode::OK);
    let info: ServerInfo = serde_json::from_str(&body).unwrap();
    assert_eq!(info.parts, default_catalog().len());
    assert!(!info.writable);

    let (status, body) = call(&app, "GET", "/api/v1/parts?q=contactor&limit=1", None, "").await;
    assert_eq!(status, StatusCode::OK);
    let list: PartList = serde_json::from_str(&body).unwrap();
    assert_eq!(list.parts.len(), 1);
    assert!(list.total >= 2);

    let (status, body) = call(&app, "GET", "/api/v1/parts/GEN.CT-9A-24DC", None, "").await;
    assert_eq!(status, StatusCode::OK);
    let part: Part = serde_json::from_str(&body).unwrap();
    assert!(!part.functions.is_empty());

    let (status, body) = call(&app, "GET", "/api/v1/parts/NOPE", None, "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(serde_json::from_str::<ApiError>(&body).unwrap().error.contains("NOPE"));
}

#[tokio::test]
async fn writes_need_the_token() {
    let part = r#"{"number": "TEST.1", "description": "Test"}"#;
    // Read-only server
    let ro = app(None);
    assert_eq!(call(&ro, "PUT", "/api/v1/parts/TEST.1", Some(TOKEN), part).await.0, StatusCode::FORBIDDEN);
    // Writable server: missing / wrong / correct token
    let app = app(Some(TOKEN));
    assert_eq!(call(&app, "PUT", "/api/v1/parts/TEST.1", None, part).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&app, "PUT", "/api/v1/parts/TEST.1", Some("wrong"), part).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&app, "PUT", "/api/v1/parts/TEST.1", Some(TOKEN), part).await.0, StatusCode::CREATED);
    assert_eq!(call(&app, "PUT", "/api/v1/parts/TEST.1", Some(TOKEN), part).await.0, StatusCode::OK);
    // URL and body must agree
    assert_eq!(call(&app, "PUT", "/api/v1/parts/OTHER", Some(TOKEN), part).await.0, StatusCode::BAD_REQUEST);
    // Part numbers with special characters are URL encoded
    let odd = r#"{"number": "A B/1"}"#;
    assert_eq!(call(&app, "PUT", "/api/v1/parts/A%20B%2F1", Some(TOKEN), odd).await.0, StatusCode::CREATED);
    assert_eq!(call(&app, "GET", "/api/v1/parts/A%20B%2F1", None, "").await.0, StatusCode::OK);

    assert_eq!(call(&app, "DELETE", "/api/v1/parts/TEST.1", None, "").await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&app, "DELETE", "/api/v1/parts/TEST.1", Some(TOKEN), "").await.0, StatusCode::NO_CONTENT);
    assert_eq!(call(&app, "DELETE", "/api/v1/parts/TEST.1", Some(TOKEN), "").await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&app, "GET", "/api/v1/parts/TEST.1", None, "").await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn import_and_export() {
    let app = app(Some(TOKEN));
    let csv = "Part number;Manufacturer;Description;Functions\nIMP.1;ACME;Imported relay;Coil:A1 A2|CO:11 12 14\n";
    let (status, body) = call(&app, "POST", "/api/v1/import", Some(TOKEN), csv).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let result: ImportResult = serde_json::from_str(&body).unwrap();
    assert_eq!(result.imported, 1);

    let (_, body) = call(&app, "GET", "/api/v1/parts/IMP.1", None, "").await;
    let part: Part = serde_json::from_str(&body).unwrap();
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

#[test]
fn token_comparison() {
    assert!(same("abc", "abc"));
    assert!(!same("abc", "abd"));
    assert!(!same("abc", "abcd"));
}
