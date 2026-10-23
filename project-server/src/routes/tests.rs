use super::*;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use openecraft_core::model::{Page, PageKind, Project};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tower::ServiceExt;

const TOKEN: &str = "secret-token";

fn app(token: Option<&str>) -> Router {
    let server = Server { store: Store::in_memory(), sessions: Sessions::default() };
    router(AppState { server: Arc::new(Mutex::new(server)), token: token.map(String::from) })
}

async fn call(app: &Router, method: &str, uri: &str, token: Option<&str>, body: String) -> (StatusCode, String) {
    let mut req = Request::builder().method(method).uri(uri).header(header::CONTENT_TYPE, "application/json");
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let resp = app.clone().oneshot(req.body(Body::from(body)).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

/// Request that must succeed, decoded.
async fn ok<T: DeserializeOwned>(app: &Router, method: &str, uri: &str, body: &impl Serialize) -> T {
    let (status, text) = call(app, method, uri, None, serde_json::to_string(body).unwrap()).await;
    assert!(status.is_success(), "{method} {uri}: {status} {text}");
    serde_json::from_str(&text).unwrap()
}

async fn error(app: &Router, method: &str, uri: &str, body: &impl Serialize) -> (StatusCode, String) {
    let (status, text) = call(app, method, uri, None, serde_json::to_string(body).unwrap()).await;
    (status, serde_json::from_str::<ApiError>(&text).map(|e| e.error).unwrap_or(text))
}

fn two_pages() -> Project {
    let mut prj = Project::default();
    let id = prj.new_id();
    prj.pages.push(Page::new(id, "2", "Motors", PageKind::Schematic));
    prj
}

async fn create(app: &Router, user: &str) -> Opened {
    let req = CreateRequest { name: "Plant 1".into(), user: user.into(), project: two_pages() };
    ok(app, "POST", "/api/v1/projects", &req).await
}

async fn open(app: &Router, user: &str) -> Opened {
    ok(app, "POST", "/api/v1/projects/Plant%201/open", &OpenRequest { user: user.into() }).await
}

#[tokio::test]
async fn create_list_and_open() {
    let app = app(None);
    let anna = create(&app, "anna").await;
    assert_eq!(anna.snapshot.revision, 1);
    assert_eq!(anna.snapshot.project.pages.len(), 2);
    let ben = open(&app, "ben").await;
    assert_ne!(anna.session.session, ben.session.session);
    // Every session gets its own ids.
    assert!(ben.first_id >= anna.first_id + openecraft_core::collab::ID_BLOCK);

    let list: Vec<ProjectSummary> = ok(&app, "GET", "/api/v1/projects", &()).await;
    assert_eq!(list.len(), 1);
    assert_eq!((list[0].name.as_str(), list[0].pages), ("Plant 1", 2));
    assert_eq!(list[0].users, vec!["anna".to_string(), "ben".to_string()]);

    let info: ServerInfo = ok(&app, "GET", "/api/v1/info", &()).await;
    assert_eq!(info.projects, 1);

    let req = CreateRequest { name: "Plant 1".into(), user: "ben".into(), project: Project::default() };
    assert_eq!(error(&app, "POST", "/api/v1/projects", &req).await.0, StatusCode::CONFLICT);
    let req = CreateRequest { name: "../x".into(), user: "ben".into(), project: Project::default() };
    assert_eq!(error(&app, "POST", "/api/v1/projects", &req).await.0, StatusCode::BAD_REQUEST);
    let req = CreateRequest { name: "x".into(), user: " ".into(), project: Project::default() };
    assert_eq!(error(&app, "POST", "/api/v1/projects", &req).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(error(&app, "POST", "/api/v1/projects/nope/open", &OpenRequest { user: "ben".into() }).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn two_users_edit_different_pages() {
    let app = app(None);
    let anna = create(&app, "anna").await;
    let ben = open(&app, "ben").await;
    let (p1, p2) = (anna.snapshot.project.pages[0].id, anna.snapshot.project.pages[1].id);
    let path = "/api/v1/projects/Plant%201";

    let lock = |s: &Opened, t: Target| LockRequest { session: s.session.clone(), targets: vec![t] };
    let _: Status = ok(&app, "POST", &format!("{path}/lock"), &lock(&anna, Target::Page(p1))).await;
    let status: Status = ok(&app, "POST", &format!("{path}/lock"), &lock(&ben, Target::Page(p2))).await;
    assert_eq!(status.locks.len(), 2);
    let (code, msg) = error(&app, "POST", &format!("{path}/lock"), &lock(&ben, Target::Page(p1))).await;
    assert_eq!(code, StatusCode::CONFLICT);
    assert!(msg.contains("anna"), "{msg}");
    assert_eq!(error(&app, "POST", &format!("{path}/lock"), &lock(&ben, Target::Page(999_999))).await.0, StatusCode::NOT_FOUND);

    // Both change both pages and the project name; each keeps only what they locked.
    let mut a = anna.snapshot.project.clone();
    a.pages[0].description = "anna 1".into();
    a.pages[1].description = "anna 2".into();
    a.info.name = "anna".into();
    let mut b = ben.snapshot.project.clone();
    b.pages[0].description = "ben 1".into();
    b.pages[1].description = "ben 2".into();
    let saved: Snapshot = ok(&app, "PUT", path, &SaveRequest { session: anna.session.clone(), project: a, release: Vec::new() }).await;
    assert_eq!(saved.revision, 2);
    let saved: Snapshot = ok(&app, "PUT", path, &SaveRequest { session: ben.session.clone(), project: b, release: vec![Target::Page(p2)] }).await;
    assert_eq!(saved.revision, 3);
    assert_eq!(saved.project.pages[0].description, "anna 1");
    assert_eq!(saved.project.pages[1].description, "ben 2");
    assert_eq!(saved.project.info.name, anna.snapshot.project.info.name);
    assert_eq!(saved.locks.len(), 1);

    let snap: Snapshot = ok(&app, "GET", path, &()).await;
    assert_eq!(snap, saved);
    let status: Status = ok(&app, "POST", &format!("{path}/status"), &ben.session).await;
    assert_eq!(status.revision, 3);
}

#[tokio::test]
async fn pages_locked_by_others_cannot_be_deleted() {
    let app = app(None);
    let anna = create(&app, "anna").await;
    let ben = open(&app, "ben").await;
    let p2 = anna.snapshot.project.pages[1].id;
    let path = "/api/v1/projects/Plant%201";
    let _: Status = ok(&app, "POST", &format!("{path}/lock"), &LockRequest { session: anna.session.clone(), targets: vec![Target::Project] }).await;
    let _: Status = ok(&app, "POST", &format!("{path}/lock"), &LockRequest { session: ben.session.clone(), targets: vec![Target::Page(p2)] }).await;
    let mut a = anna.snapshot.project.clone();
    a.pages.pop();
    let req = SaveRequest { session: anna.session.clone(), project: a, release: Vec::new() };
    let (code, msg) = error(&app, "PUT", path, &req).await;
    assert_eq!(code, StatusCode::CONFLICT);
    assert!(msg.contains("ben"), "{msg}");

    // Released: now it can go.
    let _: Status = ok(&app, "POST", &format!("{path}/close"), &ben.session).await;
    let saved: Snapshot = ok(&app, "PUT", path, &req).await;
    assert_eq!(saved.project.pages.len(), 1);
    // The closed session is gone.
    assert_eq!(error(&app, "POST", &format!("{path}/status"), &ben.session).await.0, StatusCode::GONE);
}

#[tokio::test]
async fn token_is_required_for_everything() {
    let app = app(Some(TOKEN));
    assert_eq!(call(&app, "GET", "/api/v1/projects", None, String::new()).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&app, "GET", "/api/v1/projects", Some("wrong"), String::new()).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&app, "GET", "/api/v1/projects", Some(TOKEN), String::new()).await.0, StatusCode::OK);
    let req = serde_json::to_string(&CreateRequest { name: "p".into(), user: "anna".into(), project: Project::default() }).unwrap();
    assert_eq!(call(&app, "POST", "/api/v1/projects", None, req.clone()).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&app, "POST", "/api/v1/projects", Some(TOKEN), req).await.0, StatusCode::CREATED);
}
