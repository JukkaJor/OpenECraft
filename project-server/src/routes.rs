//! HTTP routes of the project server (see `openecraft_core::collab` for the API).

use crate::sessions::{LockError, Sessions};
use crate::store::Store;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use openecraft_core::collab::{
    merge, valid_name, CreateRequest, LockRequest, OpenRequest, Opened, Owned, ProjectSummary, SaveRequest, ServerInfo, Session, Snapshot,
    Status, Target, API_PREFIX,
};
use openecraft_parts::api::{bearer_token, token_matches, ApiError};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};

pub struct Server {
    pub store: Store,
    pub sessions: Sessions,
}

#[derive(Clone)]
pub struct AppState {
    pub server: Arc<Mutex<Server>>,
    /// Bearer token required for every request. `None` = open server.
    pub token: Option<String>,
}

pub struct AppError(StatusCode, String);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.0, Json(ApiError { error: self.1 })).into_response()
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        log::error!("storage error: {e}");
        AppError(StatusCode::INTERNAL_SERVER_ERROR, format!("storage error: {e}"))
    }
}

impl From<LockError> for AppError {
    fn from(e: LockError) -> Self {
        match e {
            LockError::Expired => AppError(StatusCode::GONE, "the session has expired, open the project again".into()),
            LockError::Held(l) => AppError(StatusCode::CONFLICT, format!("{} is locked by {}", describe(l.target), l.user)),
        }
    }
}

type Result<T> = std::result::Result<T, AppError>;

fn describe(target: Target) -> String {
    match target {
        Target::Project => "the project data".into(),
        Target::Page(id) => format!("page {id}"),
    }
}

pub fn router(state: AppState) -> Router {
    let project = format!("{API_PREFIX}/projects/{{name}}");
    Router::new()
        .route("/", get(index))
        .route(&format!("{API_PREFIX}/info"), get(info))
        .route(&format!("{API_PREFIX}/projects"), get(list).post(create))
        .route(&project, get(snapshot).put(save))
        .route(&format!("{project}/open"), post(open))
        .route(&format!("{project}/status"), post(status))
        .route(&format!("{project}/lock"), post(lock))
        .route(&format!("{project}/release"), post(release))
        .route(&format!("{project}/close"), post(close))
        .layer(DefaultBodyLimit::max(256 * 1024 * 1024))
        .layer(cors())
        .with_state(state)
}

/// Any origin may call the API (e.g. the web app): authentication uses bearer
/// tokens, not cookies.
fn cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::PUT, Method::POST])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
}

/// Checks the token and returns the server with expired sessions removed.
fn server<'a>(state: &'a AppState, headers: &HeaderMap) -> Result<MutexGuard<'a, Server>> {
    if let Some(token) = &state.token {
        let presented = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(bearer_token);
        if !presented.is_some_and(|p| token_matches(p, token)) {
            return Err(AppError(StatusCode::UNAUTHORIZED, "missing or wrong access token".into()));
        }
    }
    let mut server = state.server.lock().unwrap_or_else(|e| e.into_inner());
    server.sessions.expire(Instant::now());
    Ok(server)
}

fn not_found(name: &str) -> AppError {
    AppError(StatusCode::NOT_FOUND, format!("project '{name}' not found"))
}

fn user(name: &str) -> Result<String> {
    let user = name.trim();
    if user.is_empty() {
        return Err(AppError(StatusCode::BAD_REQUEST, "a user name is required".into()));
    }
    Ok(user.to_string())
}

impl Server {
    fn snapshot(&self, name: &str) -> Result<Snapshot> {
        let entry = self.store.get(name).ok_or_else(|| not_found(name))?;
        Ok(Snapshot { revision: entry.revision, project: entry.project.clone(), locks: self.sessions.locks(name) })
    }

    fn status(&self, name: &str) -> Result<Status> {
        let entry = self.store.get(name).ok_or_else(|| not_found(name))?;
        Ok(Status { revision: entry.revision, locks: self.sessions.locks(name) })
    }

    fn open(&mut self, name: &str, user: &str) -> Result<Opened> {
        if self.store.get(name).is_none() {
            return Err(not_found(name));
        }
        let first_id = self.store.reserve_ids(name)?;
        let session = self.sessions.open(name, user, Instant::now());
        log::info!("{user} opened {name}");
        Ok(Opened { session, first_id, snapshot: self.snapshot(name)? })
    }
}

async fn index(State(state): State<AppState>) -> String {
    let server = state.server.lock().unwrap_or_else(|e| e.into_inner());
    format!(
        "OpenECraft project server {}\n\n{} projects, {}\n\nAPI: {API_PREFIX}/info, {API_PREFIX}/projects, {API_PREFIX}/projects/{{name}}\n",
        env!("CARGO_PKG_VERSION"),
        server.store.projects.len(),
        if state.token.is_some() { "access with token" } else { "open access" }
    )
}

async fn info(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<ServerInfo>> {
    let server = server(&state, &headers)?;
    Ok(Json(ServerInfo {
        name: "OpenECraft project server".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        projects: server.store.projects.len(),
    }))
}

async fn list(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Vec<ProjectSummary>>> {
    let server = server(&state, &headers)?;
    let list = server
        .store
        .projects
        .iter()
        .map(|(name, e)| ProjectSummary {
            name: name.clone(),
            title: e.project.info.name.clone(),
            pages: e.project.pages.len(),
            revision: e.revision,
            users: server.sessions.users(name),
        })
        .collect();
    Ok(Json(list))
}

async fn create(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<CreateRequest>) -> Result<(StatusCode, Json<Opened>)> {
    let mut server = server(&state, &headers)?;
    let user = user(&req.user)?;
    if !valid_name(&req.name) {
        return Err(AppError(StatusCode::BAD_REQUEST, format!("'{}' is not a valid project name", req.name)));
    }
    if server.store.get(&req.name).is_some() {
        return Err(AppError(StatusCode::CONFLICT, format!("project '{}' already exists", req.name)));
    }
    server.store.create(&req.name, req.project)?;
    log::info!("{user} created {}", req.name);
    Ok((StatusCode::CREATED, Json(server.open(&req.name, &user)?)))
}

async fn snapshot(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>) -> Result<Json<Snapshot>> {
    Ok(Json(server(&state, &headers)?.snapshot(&name)?))
}

async fn open(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>, Json(req): Json<OpenRequest>) -> Result<Json<Opened>> {
    let user = user(&req.user)?;
    Ok(Json(server(&state, &headers)?.open(&name, &user)?))
}

/// Merges what the session has locked into the stored project.
async fn save(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>, Json(req): Json<SaveRequest>) -> Result<Json<Snapshot>> {
    let mut server = server(&state, &headers)?;
    server.sessions.touch(&name, &req.session, Instant::now())?;
    let stored = server.store.get(&name).ok_or_else(|| not_found(&name))?.project.clone();
    let locks = server.sessions.locks(&name);
    let owned = Owned::of(&locks, &req.session.session);
    if owned.project {
        let removed = locks.iter().find(|l| match l.target {
            Target::Page(id) => l.session != req.session.session && !req.project.pages.iter().any(|p| p.id == id),
            Target::Project => false,
        });
        if let Some(l) = removed {
            return Err(AppError(StatusCode::CONFLICT, format!("{} cannot be deleted, it is locked by {}", describe(l.target), l.user)));
        }
    }
    let merged = merge(&stored, &req.project, &owned);
    let revision = server.store.save(&name, merged)?;
    log::info!("{} saved {name} (revision {revision})", req.session.user);
    server.sessions.release(&name, &req.session, &req.release, Instant::now())?;
    Ok(Json(server.snapshot(&name)?))
}

async fn status(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>, Json(session): Json<Session>) -> Result<Json<Status>> {
    let mut server = server(&state, &headers)?;
    server.status(&name)?;
    server.sessions.touch(&name, &session, Instant::now())?;
    Ok(Json(server.status(&name)?))
}

async fn lock(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>, Json(req): Json<LockRequest>) -> Result<Json<Status>> {
    let mut server = server(&state, &headers)?;
    let entry = server.store.get(&name).ok_or_else(|| not_found(&name))?;
    if let Some(missing) = req.targets.iter().find(|t| match t {
        Target::Page(id) => !entry.project.pages.iter().any(|p| p.id == *id),
        Target::Project => false,
    }) {
        return Err(AppError(StatusCode::NOT_FOUND, format!("{} does not exist on the server", describe(*missing))));
    }
    server.sessions.lock(&name, &req.session, &req.targets, Instant::now())?;
    Ok(Json(server.status(&name)?))
}

async fn release(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>, Json(req): Json<LockRequest>) -> Result<Json<Status>> {
    let mut server = server(&state, &headers)?;
    server.status(&name)?;
    server.sessions.release(&name, &req.session, &req.targets, Instant::now())?;
    Ok(Json(server.status(&name)?))
}

async fn close(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>, Json(session): Json<Session>) -> Result<Json<Status>> {
    let mut server = server(&state, &headers)?;
    server.status(&name)?;
    server.sessions.close(&name, &session)?;
    log::info!("{} closed {name}", session.user);
    Ok(Json(server.status(&name)?))
}

#[cfg(test)]
mod tests;
