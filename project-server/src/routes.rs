//! HTTP routes of the project server (see `openecraft_core::collab` for the API).

use crate::sessions::{LockError, Sessions};
use crate::store::Store;
use crate::users::{Account, Users};
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use openecraft_auth::{AccessFile, SignInError};
use openecraft_core::collab::{
    merge, valid_name, Access, CreateRequest, Grant, LockRequest, LoginRequest, LoginResponse, OpenRequest, Opened, Owned, Policy, ProjectAccess,
    ProjectRight, ProjectRole, ProjectSummary, ProjectUser, Right, SaveRequest, ServerInfo, Session, Settings, Snapshot, Status, Subject, Target,
    UserUpdate, API_PREFIX, SERVER,
};
use openecraft_parts::api::{bearer_token, token_matches, ApiError};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};

pub struct Server {
    pub store: Store,
    pub sessions: Sessions,
    pub users: Users,
    pub access: AccessFile<ProjectRight>,
}

impl Server {
    /// Without token, accounts and directories everyone may do everything.
    fn open_access(&self, token: &Option<String>) -> bool {
        token.is_none() && self.users.accounts.is_empty() && self.access.config.directories.is_empty()
    }

    /// What a user may do: their role, the grants to them and their groups and what
    /// anonymous requests may do.
    fn access_of(&self, user: &ProjectUser) -> Access<ProjectRight> {
        let policy = &self.access.config.policy;
        policy.access(&user.identity(), &user.grants()).with(policy.anonymous_access())
    }
}

/// Who makes a request and what they may do.
#[derive(Clone, Debug, PartialEq)]
pub struct Caller {
    /// A signed-in user; `None` for the server token, open servers and anonymous requests.
    user: Option<String>,
    access: Access<ProjectRight>,
}

impl Caller {
    fn rights(&self, project: &str) -> BTreeSet<ProjectRight> {
        self.access.rights(project)
    }

    fn may(&self, project: &str, right: ProjectRight) -> bool {
        self.access.allows(project, right)
    }

    /// The user name of a request: a signed-in user's own name, otherwise the one sent.
    fn name(&self, sent: &str) -> Result<String> {
        match &self.user {
            Some(u) => Ok(u.clone()),
            None => user(sent),
        }
    }

    /// A signed-in user may only use their own sessions.
    fn owns(&self, session: &Session) -> Result<()> {
        match &self.user {
            Some(u) if !u.eq_ignore_ascii_case(&session.user) => Err(AppError(StatusCode::FORBIDDEN, "the session belongs to another user".into())),
            _ => Ok(()),
        }
    }

    /// Projects the caller may not view do not exist for them.
    fn see(&self, project: &str) -> Result<()> {
        if self.may(project, ProjectRight::View) {
            Ok(())
        } else {
            Err(not_found(project))
        }
    }

    fn need(&self, project: &str, right: ProjectRight) -> Result<()> {
        self.see(project)?;
        if self.may(project, right) {
            Ok(())
        } else {
            Err(AppError(StatusCode::FORBIDDEN, format!("you lack the right '{}' on project '{project}'", right.name())))
        }
    }

    fn admin(&self) -> Result<()> {
        if self.may(SERVER, ProjectRight::Administer) {
            Ok(())
        } else {
            Err(AppError(StatusCode::FORBIDDEN, "only administrators manage users and access".into()))
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

fn bad(msg: impl Into<String>) -> AppError {
    AppError(StatusCode::BAD_REQUEST, msg.into())
}

pub fn router(state: AppState) -> Router {
    let project = format!("{API_PREFIX}/projects/{{name}}");
    Router::new()
        .route("/", get(index))
        .route(&format!("{API_PREFIX}/info"), get(info))
        .route(&format!("{API_PREFIX}/projects"), get(list).post(create))
        .route(&project, get(snapshot).put(save).delete(delete_project))
        .route(&format!("{project}/open"), post(open))
        .route(&format!("{project}/status"), post(status))
        .route(&format!("{project}/lock"), post(lock))
        .route(&format!("{project}/release"), post(release))
        .route(&format!("{project}/close"), post(close))
        .route(&format!("{project}/access"), get(project_access).put(save_project_access))
        .route(&format!("{API_PREFIX}/login"), post(login))
        .route(&format!("{API_PREFIX}/logout"), post(logout))
        .route(&format!("{API_PREFIX}/users"), get(list_users))
        .route(&format!("{API_PREFIX}/users/{{name}}"), put(save_user).delete(delete_user))
        .route(&format!("{API_PREFIX}/access"), get(access).put(save_access))
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
    headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).and_then(bearer_token).filter(|t| !t.is_empty())
}

fn lock_server(state: &AppState) -> MutexGuard<'_, Server> {
    state.server.lock().unwrap_or_else(|e| e.into_inner())
}

/// Checks the token (the server token or the session of a user) and returns the
/// server with expired sessions removed and who is calling.
fn server<'a>(state: &'a AppState, headers: &HeaderMap) -> Result<(MutexGuard<'a, Server>, Caller)> {
    let mut server = lock_server(state);
    server.sessions.expire(Instant::now());
    let token = presented(headers);
    let caller = if token.is_some_and(|p| state.token.as_ref().is_some_and(|t| token_matches(p, t))) || server.open_access(&state.token) {
        Caller { user: None, access: Access::full() }
    } else if let Some(a) = token.and_then(|t| server.users.by_token(t)) {
        Caller { user: Some(a.user.name.clone()), access: server.access_of(&a.user) }
    } else if token.is_none() && !server.access.config.policy.anonymous_access().is_empty() {
        Caller { user: None, access: server.access.config.policy.anonymous_access() }
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
        return Err(bad("a user name is required"));
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

    fn open(&mut self, name: &str, user: &str, rights: BTreeSet<ProjectRight>) -> Result<Opened> {
        if self.store.get(name).is_none() {
            return Err(not_found(name));
        }
        let first_id = self.store.reserve_ids(name)?;
        let session = self.sessions.open(name, user, Instant::now());
        log::info!("{user} opened {name}");
        Ok(Opened { session, first_id, snapshot: self.snapshot(name)?, rights: rights.into_iter().collect() })
    }

    /// The grants on a project and the roles `caller` may give.
    fn project_access(&self, name: &str, caller: &Caller) -> ProjectAccess {
        let policy = &self.access.config.policy;
        let roles = policy.role_names().into_iter().filter(|r| policy.rights_of(r).is_some_and(|rights| rights.iter().all(|x| caller.may(name, *x)))).collect();
        ProjectAccess { grants: policy.grants_on(name), roles }
    }
}

async fn index(State(state): State<AppState>) -> String {
    let server = lock_server(&state);
    format!(
        "OpenECraft project server {}\n\n{} projects, {}\n\nAPI: {API_PREFIX}/info, {API_PREFIX}/projects, {API_PREFIX}/projects/{{name}}\n",
        env!("CARGO_PKG_VERSION"),
        server.store.projects.len(),
        if server.open_access(&state.token) { "open access" } else { "access with token or user account" }
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
        .filter(|(name, _)| caller.may(name, ProjectRight::View))
        .map(|(name, e)| ProjectSummary {
            name: name.clone(),
            title: e.project.info.name.clone(),
            pages: e.project.pages.len(),
            revision: e.revision,
            users: server.sessions.users(name),
            rights: caller.rights(name).into_iter().collect(),
        })
        .collect();
    Ok(Json(list))
}

async fn create(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<CreateRequest>) -> Result<(StatusCode, Json<Opened>)> {
    let (mut server, mut caller) = server(&state, &headers)?;
    let user = caller.name(&req.user)?;
    if !caller.may(&req.name, ProjectRight::Create) {
        return Err(AppError(StatusCode::FORBIDDEN, format!("you may not create project '{}'", req.name)));
    }
    if !valid_name(&req.name) {
        return Err(bad(format!("'{}' is not a valid project name", req.name)));
    }
    if server.store.get(&req.name).is_some() {
        return Err(AppError(StatusCode::CONFLICT, format!("project '{}' already exists", req.name)));
    }
    server.store.create(&req.name, req.project)?;
    log::info!("{user} created {}", req.name);
    // A signed-in user manages the projects they create.
    if let Some(owner) = caller.user.clone().filter(|_| !caller.may(&req.name, ProjectRight::ManageAccess)) {
        let mut policy = server.access.config.policy.clone();
        let owner_grant = Grant::new(Subject::User(owner), ProjectRole::Manager.name(), &req.name);
        policy.grants.push(owner_grant.clone());
        server.access.set_policy(policy).map_err(|e| AppError(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        caller.access = caller.access.with(Policy::default().access(&Default::default(), &[owner_grant]));
    }
    let rights = caller.rights(&req.name);
    Ok((StatusCode::CREATED, Json(server.open(&req.name, &user, rights)?)))
}

async fn delete_project(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>) -> Result<StatusCode> {
    let (mut server, caller) = server(&state, &headers)?;
    caller.need(&name, ProjectRight::Delete)?;
    server.status(&name)?;
    let users = server.sessions.users(&name);
    if !users.is_empty() {
        return Err(AppError(StatusCode::CONFLICT, format!("project '{name}' is open ({})", users.join(", "))));
    }
    server.store.delete(&name)?;
    let mut policy = server.access.config.policy.clone();
    if !policy.grants_on(&name).is_empty() {
        policy.set_grants_on(&name, Vec::new());
        server.access.set_policy(policy).map_err(|e| AppError(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    }
    log::info!("{} deleted {name}", caller.user.as_deref().unwrap_or("token"));
    Ok(StatusCode::NO_CONTENT)
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
    Ok(Json(server.open(&name, &user, caller.rights(&name))?))
}

/// Merges what the session has locked (and the review records) into the stored project.
async fn save(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>, Json(req): Json<SaveRequest>) -> Result<Json<Snapshot>> {
    let (mut server, caller) = server(&state, &headers)?;
    caller.see(&name)?;
    caller.owns(&req.session)?;
    let rights = caller.rights(&name);
    if ![ProjectRight::Review, ProjectRight::EditPages, ProjectRight::EditProject].iter().any(|r| rights.contains(r)) {
        return Err(AppError(StatusCode::FORBIDDEN, format!("you may only view project '{name}'")));
    }
    server.sessions.touch(&name, &req.session, Instant::now())?;
    let stored = server.store.get(&name).ok_or_else(|| not_found(&name))?.project.clone();
    let locks = server.sessions.locks(&name);
    let owned = Owned::of(&locks, &req.session.session);
    if owned.project {
        caller.need(&name, ProjectRight::EditProject)?;
        let removed = locks.iter().find(|l| match l.target {
            Target::Page(id) => l.session != req.session.session && !req.project.pages.iter().any(|p| p.id == id),
            Target::Project => false,
        });
        if let Some(l) = removed {
            return Err(AppError(StatusCode::CONFLICT, format!("{} cannot be deleted, it is locked by {}", describe(l.target), l.user)));
        }
    }
    if !owned.pages.is_empty() {
        caller.need(&name, ProjectRight::EditPages)?;
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
    caller.see(&name)?;
    caller.owns(&req.session)?;
    for t in &req.targets {
        caller.need(&name, if *t == Target::Project { ProjectRight::EditProject } else { ProjectRight::EditPages })?;
    }
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

async fn project_access(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>) -> Result<Json<ProjectAccess>> {
    let (server, caller) = server(&state, &headers)?;
    caller.need(&name, ProjectRight::ManageAccess)?;
    server.status(&name)?;
    Ok(Json(server.project_access(&name, &caller)))
}

/// Replaces the grants on the project. Nobody gives rights they do not have.
async fn save_project_access(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>, Json(grants): Json<Vec<Grant>>) -> Result<Json<ProjectAccess>> {
    let (mut server, caller) = server(&state, &headers)?;
    caller.need(&name, ProjectRight::ManageAccess)?;
    server.status(&name)?;
    let allowed = server.project_access(&name, &caller).roles;
    let old = server.access.config.policy.grants_on(&name);
    for g in grants.iter().chain(&old) {
        if !allowed.iter().any(|r| r.eq_ignore_ascii_case(g.role.trim())) {
            return Err(AppError(StatusCode::FORBIDDEN, format!("you may not give or take the role '{}'", g.role.trim())));
        }
    }
    let mut policy = server.access.config.policy.clone();
    policy.set_grants_on(&name, grants);
    server.access.set_policy(policy).map_err(bad)?;
    log::info!("access to {name} changed by {}", caller.user.as_deref().unwrap_or("token"));
    Ok(Json(server.project_access(&name, &caller)))
}

/// Signs in with a local account or, if the server has none of that name, with its
/// directories; a directory user gets an account (role, groups, sessions) on the
/// first sign-in.
async fn login(State(state): State<AppState>, Json(req): Json<LoginRequest>) -> Result<Json<LoginResponse>> {
    let refused = || AppError(StatusCode::UNAUTHORIZED, "unknown user or wrong password".into());
    let directories = {
        let mut server = lock_server(&state);
        let local = req.provider.trim().is_empty() && server.users.find_mut(&req.user).is_some_and(|a| a.user.provider.is_empty());
        if local {
            let account = server.users.find_mut(&req.user).ok_or_else(refused)?;
            if !openecraft_auth::verify_password(&req.password, &account.password) {
                log::warn!("failed sign-in of '{}'", req.user.trim());
                return Err(refused());
            }
            let token = openecraft_auth::start_session(&mut account.sessions);
            let user = account.user.clone();
            server.users.save()?;
            log::info!("{} signed in", user.name);
            let access = server.access_of(&user);
            return Ok(Json(LoginResponse { token, user, access }));
        }
        server.access.config.directories.clone()
    };
    let identity = match openecraft_auth::sign_in(&directories, &req.provider, &req.user, &req.password).await {
        Ok(identity) => identity,
        Err(SignInError::Unavailable(e)) => return Err(AppError(StatusCode::SERVICE_UNAVAILABLE, e)),
        Err(_) => {
            log::warn!("failed sign-in of '{}'", req.user.trim());
            return Err(refused());
        }
    };
    let mut server = lock_server(&state);
    let account = match server.users.find_mut(&identity.name) {
        Some(a) if a.user.provider.eq_ignore_ascii_case(&identity.provider) => a,
        Some(_) => return Err(AppError(StatusCode::CONFLICT, format!("the name '{}' belongs to another account", identity.name))),
        None => {
            let user = ProjectUser { name: identity.name.clone(), provider: identity.provider.clone(), ..Default::default() };
            server.users.accounts.push(Account { user, password: String::new(), sessions: Vec::new() });
            server.users.accounts.last_mut().expect("just added")
        }
    };
    account.user.groups = identity.groups;
    let token = openecraft_auth::start_session(&mut account.sessions);
    let user = account.user.clone();
    server.users.save()?;
    log::info!("{} signed in with {}", user.name, user.provider);
    let access = server.access_of(&user);
    Ok(Json(LoginResponse { token, user, access }))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<StatusCode> {
    let token = presented(&headers).ok_or_else(|| AppError(StatusCode::UNAUTHORIZED, "no session".into()))?.to_string();
    let mut server = lock_server(&state);
    let ended = server.users.accounts.iter_mut().any(|a| openecraft_auth::end_session(&mut a.sessions, &token));
    if ended {
        server.users.save()?;
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn list_users(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Vec<ProjectUser>>> {
    let (server, caller) = server(&state, &headers)?;
    caller.admin()?;
    Ok(Json(server.users.accounts.iter().map(|a| a.user.clone()).collect()))
}

async fn save_user(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>, Json(req): Json<UserUpdate>) -> Result<Json<ProjectUser>> {
    let (mut server, caller) = server(&state, &headers)?;
    caller.admin()?;
    let name = user(&name)?;
    let hash = match req.password.as_deref().filter(|p| !p.is_empty()) {
        Some(p) => Some(openecraft_auth::hash_password(p).map_err(|e| AppError(StatusCode::INTERNAL_SERVER_ERROR, e))?),
        None => None,
    };
    let groups = req.groups.map(|g| g.into_iter().map(|g| g.trim().to_string()).filter(|g| !g.is_empty()).collect::<Vec<_>>());
    let user = match server.users.find_mut(&name) {
        Some(a) => {
            if !a.user.provider.is_empty() && hash.is_some() {
                return Err(bad(format!("'{name}' signs in with directory {}, which keeps the password", a.user.provider)));
            }
            a.user.role = req.role;
            a.user.projects = req.projects;
            if let Some(g) = groups.filter(|_| a.user.provider.is_empty()) {
                a.user.groups = g;
            }
            if let Some(h) = hash {
                a.password = h;
                a.sessions.clear();
            }
            a.user.clone()
        }
        None => {
            let password = hash.ok_or_else(|| bad("a new user needs a password"))?;
            let user = ProjectUser { name: name.clone(), role: req.role, projects: req.projects, groups: groups.unwrap_or_default(), ..Default::default() };
            server.users.accounts.push(Account { user: user.clone(), password, sessions: Vec::new() });
            user
        }
    };
    server.users.save()?;
    log::info!("user {name} saved");
    Ok(Json(user))
}

async fn delete_user(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>) -> Result<StatusCode> {
    let (mut server, caller) = server(&state, &headers)?;
    caller.admin()?;
    let before = server.users.accounts.len();
    server.users.accounts.retain(|a| !a.user.name.eq_ignore_ascii_case(name.trim()));
    if server.users.accounts.len() == before {
        return Err(AppError(StatusCode::NOT_FOUND, format!("user '{name}' not found")));
    }
    server.users.save()?;
    Ok(StatusCode::NO_CONTENT)
}

async fn access(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Settings<ProjectRight>>> {
    let (server, caller) = server(&state, &headers)?;
    caller.admin()?;
    Ok(Json(server.access.settings()))
}

async fn save_access(State(state): State<AppState>, headers: HeaderMap, Json(policy): Json<Policy<ProjectRight>>) -> Result<Json<Settings<ProjectRight>>> {
    let (mut server, caller) = server(&state, &headers)?;
    caller.admin()?;
    server.access.set_policy(policy).map_err(bad)?;
    log::info!("access settings changed by {}", caller.user.as_deref().unwrap_or("token"));
    Ok(Json(server.access.settings()))
}

#[cfg(test)]
mod tests;
