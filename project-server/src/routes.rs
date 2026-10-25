//! HTTP routes of the project server (see `openecraft_core::collab` for the API).

use crate::sessions::{LockError, Sessions};
use crate::store::Store;
use crate::users::{Account, Users};
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use openecraft_core::collab::{
    merge, valid_name, CreateRequest, LockRequest, LoginRequest, LoginResponse, OpenRequest, Opened, Owned, ProjectRole, ProjectSummary,
    ProjectUser, SaveRequest, ServerInfo, Session, Snapshot, Status, Target, UserUpdate, API_PREFIX,
};
use openecraft_parts::api::{bearer_token, token_matches, ApiError};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};

pub struct Server {
    pub store: Store,
    pub sessions: Sessions,
    pub users: Users,
}

/// Who makes a request.
#[derive(Clone, Debug, PartialEq)]
pub enum Caller {
    /// The server token: full access.
    Admin,
    /// A signed-in user.
    User(ProjectUser),
    /// A server without token and accounts: everyone may do everything.
    Open,
}

impl Caller {
    fn may_see(&self, project: &str) -> bool {
        match self {
            Caller::User(u) => u.may_see(project),
            _ => true,
        }
    }

    fn may_edit(&self, project: &str) -> bool {
        match self {
            Caller::User(u) => u.may_edit(project),
            _ => true,
        }
    }

    fn is_admin(&self) -> bool {
        match self {
            Caller::User(u) => u.role == ProjectRole::Admin,
            _ => true,
        }
    }

    /// The user name of a request: a signed-in user's own name, otherwise the one sent.
    fn name(&self, sent: &str) -> Result<String> {
        match self {
            Caller::User(u) => Ok(u.name.clone()),
            _ => user(sent),
        }
    }

    /// A signed-in user may only use their own sessions.
    fn owns(&self, session: &Session) -> Result<()> {
        match self {
            Caller::User(u) if !u.name.eq_ignore_ascii_case(&session.user) => Err(AppError(StatusCode::FORBIDDEN, "the session belongs to another user".into())),
            _ => Ok(()),
        }
    }

    fn see(&self, project: &str) -> Result<()> {
        if self.may_see(project) {
            Ok(())
        } else {
            Err(not_found(project))
        }
    }

    fn edit(&self, project: &str) -> Result<()> {
        self.see(project)?;
        if self.may_edit(project) {
            Ok(())
        } else {
            Err(AppError(StatusCode::FORBIDDEN, format!("you may only view project '{project}'")))
        }
    }
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
        .route(&format!("{API_PREFIX}/login"), post(login))
        .route(&format!("{API_PREFIX}/logout"), post(logout))
        .route(&format!("{API_PREFIX}/users"), get(list_users))
        .route(&format!("{API_PREFIX}/users/{{name}}"), put(save_user).delete(delete_user))
        .layer(DefaultBodyLimit::max(256 * 1024 * 1024))
        .layer(cors())
        .with_state(state)
}

/// Any origin may call the API (e.g. the web app): authentication uses bearer
/// tokens, not cookies.
fn cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::PUT, Method::POST, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
}

fn presented(headers: &HeaderMap) -> Option<&str> {
    headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).and_then(bearer_token)
}

/// Checks the token (the server token or the session of a user) and returns the
/// server with expired sessions removed and who is calling.
fn server<'a>(state: &'a AppState, headers: &HeaderMap) -> Result<(MutexGuard<'a, Server>, Caller)> {
    let mut server = state.server.lock().unwrap_or_else(|e| e.into_inner());
    server.sessions.expire(Instant::now());
    let token = presented(headers);
    let caller = if token.is_some_and(|p| state.token.as_ref().is_some_and(|t| token_matches(p, t))) {
        Caller::Admin
    } else if let Some(a) = token.and_then(|t| server.users.by_token(t)) {
        Caller::User(a.user.clone())
    } else if state.token.is_none() && server.users.accounts.is_empty() {
        Caller::Open
    } else {
        return Err(AppError(StatusCode::UNAUTHORIZED, "missing or wrong access token".into()));
    };
    Ok((server, caller))
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
        if state.token.is_some() || !server.users.accounts.is_empty() { "access with token or user account" } else { "open access" }
    )
}

async fn info(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<ServerInfo>> {
    let (server, _) = server(&state, &headers)?;
    Ok(Json(ServerInfo {
        name: "OpenECraft project server".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        projects: server.store.projects.len(),
    }))
}

async fn list(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Vec<ProjectSummary>>> {
    let (server, caller) = server(&state, &headers)?;
    let list = server
        .store
        .projects
        .iter()
        .filter(|(name, _)| caller.may_see(name))
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
    let (mut server, caller) = server(&state, &headers)?;
    let user = caller.name(&req.user)?;
    caller.edit(&req.name)?;
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
    let (server, caller) = server(&state, &headers)?;
    caller.see(&name)?;
    Ok(Json(server.snapshot(&name)?))
}

async fn open(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>, Json(req): Json<OpenRequest>) -> Result<Json<Opened>> {
    let (mut server, caller) = server(&state, &headers)?;
    caller.see(&name)?;
    let user = caller.name(&req.user)?;
    Ok(Json(server.open(&name, &user)?))
}

/// Merges what the session has locked into the stored project.
async fn save(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>, Json(req): Json<SaveRequest>) -> Result<Json<Snapshot>> {
    let (mut server, caller) = server(&state, &headers)?;
    caller.edit(&name)?;
    caller.owns(&req.session)?;
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
    let (mut server, caller) = server(&state, &headers)?;
    caller.see(&name)?;
    caller.owns(&session)?;
    server.status(&name)?;
    server.sessions.touch(&name, &session, Instant::now())?;
    Ok(Json(server.status(&name)?))
}

async fn lock(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>, Json(req): Json<LockRequest>) -> Result<Json<Status>> {
    let (mut server, caller) = server(&state, &headers)?;
    caller.edit(&name)?;
    caller.owns(&req.session)?;
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
    let (mut server, caller) = server(&state, &headers)?;
    caller.see(&name)?;
    caller.owns(&req.session)?;
    server.status(&name)?;
    server.sessions.release(&name, &req.session, &req.targets, Instant::now())?;
    Ok(Json(server.status(&name)?))
}

async fn close(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>, Json(session): Json<Session>) -> Result<Json<Status>> {
    let (mut server, caller) = server(&state, &headers)?;
    caller.see(&name)?;
    caller.owns(&session)?;
    server.status(&name)?;
    server.sessions.close(&name, &session)?;
    log::info!("{} closed {name}", session.user);
    Ok(Json(server.status(&name)?))
}

async fn login(State(state): State<AppState>, Json(req): Json<LoginRequest>) -> Result<Json<LoginResponse>> {
    let mut server = state.server.lock().unwrap_or_else(|e| e.into_inner());
    let refused = || AppError(StatusCode::UNAUTHORIZED, "unknown user or wrong password".into());
    let account = server.users.find_mut(&req.user).ok_or_else(refused)?;
    if !openecraft_auth::verify_password(&req.password, &account.password) {
        return Err(refused());
    }
    let token = openecraft_auth::start_session(&mut account.sessions);
    let user = account.user.clone();
    server.users.save()?;
    log::info!("{} signed in", user.name);
    Ok(Json(LoginResponse { token, user }))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<StatusCode> {
    let token = presented(&headers).ok_or_else(|| AppError(StatusCode::UNAUTHORIZED, "no session".into()))?.to_string();
    let mut server = state.server.lock().unwrap_or_else(|e| e.into_inner());
    let ended = server.users.accounts.iter_mut().any(|a| openecraft_auth::end_session(&mut a.sessions, &token));
    if ended {
        server.users.save()?;
    }
    Ok(StatusCode::NO_CONTENT)
}

fn admin_only(caller: &Caller) -> Result<()> {
    if caller.is_admin() {
        Ok(())
    } else {
        Err(AppError(StatusCode::FORBIDDEN, "only administrators manage users".into()))
    }
}

async fn list_users(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Vec<ProjectUser>>> {
    let (server, caller) = server(&state, &headers)?;
    admin_only(&caller)?;
    Ok(Json(server.users.accounts.iter().map(|a| a.user.clone()).collect()))
}

async fn save_user(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>, Json(req): Json<UserUpdate>) -> Result<Json<ProjectUser>> {
    let (mut server, caller) = server(&state, &headers)?;
    admin_only(&caller)?;
    let name = user(&name)?;
    let hash = match req.password.as_deref().filter(|p| !p.is_empty()) {
        Some(p) => Some(openecraft_auth::hash_password(p).map_err(|e| AppError(StatusCode::INTERNAL_SERVER_ERROR, e))?),
        None => None,
    };
    let user = ProjectUser { name: name.clone(), role: req.role, projects: req.projects };
    match server.users.find_mut(&name) {
        Some(a) => {
            a.user = user.clone();
            if let Some(h) = hash {
                a.password = h;
                a.sessions.clear();
            }
        }
        None => {
            let password = hash.ok_or_else(|| AppError(StatusCode::BAD_REQUEST, "a new user needs a password".into()))?;
            server.users.accounts.push(Account { user: user.clone(), password, sessions: Vec::new() });
        }
    }
    server.users.save()?;
    log::info!("user {name} saved");
    Ok(Json(user))
}

async fn delete_user(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>) -> Result<StatusCode> {
    let (mut server, caller) = server(&state, &headers)?;
    admin_only(&caller)?;
    let before = server.users.accounts.len();
    server.users.accounts.retain(|a| !a.user.name.eq_ignore_ascii_case(name.trim()));
    if server.users.accounts.len() == before {
        return Err(AppError(StatusCode::NOT_FOUND, format!("user '{name}' not found")));
    }
    server.users.save()?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
