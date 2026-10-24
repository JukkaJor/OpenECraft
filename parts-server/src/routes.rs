//! HTTP routes of the parts server (see `openecraft_parts::api` for the API).

use crate::auth::{hash_password, verify_password, Account};
use crate::storage::Store;
use crate::web;
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use openecraft_parts::api::{
    bearer_token, token_matches, ApiError, ImportResult, Login, PartList, PartQuery, ServerInfo, Session, UserUpdate, API_PREFIX,
};
use openecraft_parts::portal::{content_type, AttachmentKind, Catalog, HistoryEntry, Role, SearchResult, User};
use openecraft_parts::{import, to_csv, Part};
use serde::Deserialize;
use std::sync::{Arc, Mutex, MutexGuard};
use tower_http::cors::{Any, CorsLayer};

/// Name of the user of the administrator token.
pub const ADMIN: &str = "admin";

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Mutex<Store>>,
    /// Administrator token (`--token`). Without it and without user accounts the
    /// server is read-only.
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

fn bad(msg: impl Into<String>) -> AppError {
    AppError(StatusCode::BAD_REQUEST, msg.into())
}

fn not_found(msg: impl Into<String>) -> AppError {
    AppError(StatusCode::NOT_FOUND, msg.into())
}

fn forbidden(msg: impl Into<String>) -> AppError {
    AppError(StatusCode::FORBIDDEN, msg.into())
}

type Result<T> = std::result::Result<T, AppError>;

pub fn router(state: AppState) -> Router {
    let api = |path: &str| format!("{API_PREFIX}{path}");
    Router::new()
        .route("/", get(web::index))
        .route(&api("/texts"), get(web::texts))
        .route(&api("/info"), get(info))
        .route(&api("/parts"), get(list))
        .route(&api("/search"), post(search))
        .route(&api("/parts/{number}"), get(get_part).put(put_part).delete(delete_part))
        .route(&api("/parts/{number}/history"), get(history))
        .route(&api("/parts/{number}/history/{revision}"), get(history_part))
        .route(&api("/parts/{number}/attachments"), post(attach))
        .route(&api("/parts/{number}/attachments/{id}"), delete(detach))
        .route(&api("/files/{id}"), get(file))
        .route(&api("/import"), post(import_parts))
        .route(&api("/export.csv"), get(export_csv))
        .route(&api("/catalogs"), get(catalogs))
        .route(&api("/catalogs/{id}"), get(get_catalog).put(put_catalog).delete(delete_catalog))
        .route(&api("/login"), post(login))
        .route(&api("/logout"), post(logout))
        .route(&api("/favorites"), get(favorites).put(set_favorites))
        .route(&api("/users"), get(users))
        .route(&api("/users/{name}"), axum::routing::put(put_user).delete(delete_user))
        .layer(DefaultBodyLimit::max(32 * 1024 * 1024))
        .layer(cors())
        .with_state(state)
}

/// Any origin may call the API (e.g. the web app): authentication uses bearer
/// tokens, not cookies. `Authorization` must be listed explicitly – browsers do not
/// cover it with the `*` wildcard.
fn cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::PUT, Method::POST, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
}

fn store(state: &AppState) -> MutexGuard<'_, Store> {
    state.store.lock().unwrap_or_else(|e| e.into_inner())
}

fn presented(headers: &HeaderMap) -> Option<&str> {
    headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).and_then(bearer_token).filter(|t| !t.is_empty())
}

/// The user of the request's token: the administrator token or a session.
fn caller(state: &AppState, headers: &HeaderMap) -> Option<User> {
    let token = presented(headers)?;
    if state.token.as_deref().is_some_and(|t| token_matches(token, t)) {
        return Some(User { name: ADMIN.into(), role: Role::Admin, manufacturer: String::new() });
    }
    store(state).session(token).map(|a| a.user.clone())
}

/// True if anybody can change the database.
fn writable(state: &AppState, store: &Store) -> bool {
    state.token.is_some() || store.accounts().iter().any(|a| a.user.role >= Role::Manufacturer)
}

/// The signed in user (any role).
fn user(state: &AppState, headers: &HeaderMap) -> Result<User> {
    if let Some(u) = caller(state, headers) {
        return Ok(u);
    }
    if state.token.is_none() && store(state).accounts().is_empty() {
        return Err(forbidden("server is read-only (start it with --token to allow changes)"));
    }
    Err(AppError(StatusCode::UNAUTHORIZED, "missing or wrong access token".into()))
}

fn with_role(state: &AppState, headers: &HeaderMap, role: Role) -> Result<User> {
    let u = user(state, headers)?;
    if u.role < role {
        return Err(forbidden(format!("user '{}' is not allowed to do this", u.name)));
    }
    Ok(u)
}

fn may_edit(user: &User, manufacturer: &str) -> Result<()> {
    if user.may_edit(manufacturer) {
        Ok(())
    } else if user.role == Role::Manufacturer {
        Err(forbidden(format!("user '{}' may only change data of manufacturer '{}'", user.name, user.manufacturer)))
    } else {
        Err(forbidden(format!("user '{}' may not change parts", user.name)))
    }
}

async fn info(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<ServerInfo>> {
    let user = caller(&state, &headers);
    let store = store(&state);
    Ok(Json(ServerInfo {
        name: "OpenECraft parts server".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        parts: store.count()?,
        revision: store.revision,
        writable: writable(&state, &store),
        catalogs: store.catalogs()?.len(),
        storage: store.kind().into(),
        user,
    }))
}

async fn list(State(state): State<AppState>, Query(query): Query<PartQuery>) -> Result<Json<PartList>> {
    let store = store(&state);
    let (total, parts) = store.list(&query)?;
    Ok(Json(PartList { revision: store.revision, total, offset: query.offset.unwrap_or(0), parts }))
}

async fn search(State(state): State<AppState>, Json(query): Json<PartQuery>) -> Result<Json<SearchResult>> {
    Ok(Json(store(&state).search(&query)?))
}

fn part_of(store: &Store, number: &str) -> Result<Part> {
    store.get(number)?.ok_or_else(|| not_found(format!("part '{number}' not found")))
}

async fn get_part(State(state): State<AppState>, Path(number): Path<String>) -> Result<Json<Part>> {
    part_of(&store(&state), &number).map(Json)
}

async fn put_part(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(number): Path<String>,
    Json(mut part): Json<Part>,
) -> Result<(StatusCode, Json<Part>)> {
    let user = user(&state, &headers)?;
    if number.trim().is_empty() {
        return Err(bad("part number must not be empty"));
    }
    if part.number.is_empty() {
        part.number = number.clone();
    }
    if part.number != number {
        return Err(bad("part number in URL and body differ"));
    }
    if part.successor == part.number {
        return Err(bad("a part cannot be its own successor"));
    }
    let mut store = store(&state);
    may_edit(&user, &part.manufacturer)?;
    if let Some(old) = store.get(&number)? {
        may_edit(&user, &old.manufacturer)?;
    }
    let (stored, created) = store.put_parts(&user.name, vec![part])?.pop().expect("one part");
    log::info!("{} part {number} ({})", if created { "created" } else { "updated" }, user.name);
    Ok((if created { StatusCode::CREATED } else { StatusCode::OK }, Json(stored)))
}

async fn delete_part(State(state): State<AppState>, headers: HeaderMap, Path(number): Path<String>) -> Result<StatusCode> {
    let user = user(&state, &headers)?;
    let mut store = store(&state);
    let old = part_of(&store, &number)?;
    may_edit(&user, &old.manufacturer)?;
    store.delete(&user.name, &number)?;
    log::info!("deleted part {number} ({})", user.name);
    Ok(StatusCode::NO_CONTENT)
}

async fn history(State(state): State<AppState>, Path(number): Path<String>) -> Result<Json<Vec<HistoryEntry>>> {
    Ok(Json(store(&state).history(&number)?))
}

async fn history_part(State(state): State<AppState>, Path((number, revision)): Path<(String, u64)>) -> Result<Json<Part>> {
    store(&state).history_part(&number, revision)?.map(Json).ok_or_else(|| not_found(format!("no version {revision} of part '{number}'")))
}

#[derive(Deserialize)]
struct AttachQuery {
    name: String,
    kind: Option<AttachmentKind>,
}

/// File name without folders.
fn file_name(name: &str) -> &str {
    name.rsplit(['/', '\\']).next().unwrap_or(name).trim()
}

async fn attach(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(number): Path<String>,
    Query(q): Query<AttachQuery>,
    body: Bytes,
) -> Result<Json<Part>> {
    let user = user(&state, &headers)?;
    let name = file_name(&q.name);
    if name.is_empty() {
        return Err(bad("the file needs a name"));
    }
    if body.is_empty() {
        return Err(bad("the file is empty"));
    }
    let mut store = store(&state);
    may_edit(&user, &part_of(&store, &number)?.manufacturer)?;
    let part = store.attach(&user.name, &number, name, q.kind, body.to_vec())?.ok_or_else(|| not_found(format!("part '{number}' not found")))?;
    log::info!("attached {name} to part {number} ({})", user.name);
    Ok(Json(part))
}

async fn detach(State(state): State<AppState>, headers: HeaderMap, Path((number, id)): Path<(String, String)>) -> Result<Json<Part>> {
    let user = user(&state, &headers)?;
    let mut store = store(&state);
    may_edit(&user, &part_of(&store, &number)?.manufacturer)?;
    store.detach(&user.name, &number, &id)?.map(Json).ok_or_else(|| not_found(format!("part '{number}' has no file '{id}'")))
}

async fn file(State(state): State<AppState>, Path(id): Path<String>) -> Result<Response> {
    let f = store(&state).file(&id)?.ok_or_else(|| not_found(format!("file '{id}' not found")))?;
    let disposition = format!("inline; filename=\"{}\"", f.name.replace(['"', '\\', '\r', '\n'], "_"));
    Ok(([(header::CONTENT_TYPE, content_type(&f.name).to_string()), (header::CONTENT_DISPOSITION, disposition)], f.data).into_response())
}

/// Body: CSV (header names as in the export), JSON (parts database or part list) or BMEcat.
async fn import_parts(State(state): State<AppState>, headers: HeaderMap, body: String) -> Result<Json<ImportResult>> {
    let user = with_role(&state, &headers, Role::Manufacturer)?;
    let parts = import(&body).map_err(|e| bad(format!("invalid parts data: {e}")))?;
    if parts.iter().any(|p| p.number.trim().is_empty()) {
        return Err(bad("every part needs a part number"));
    }
    let mut store = store(&state);
    for p in &parts {
        may_edit(&user, &p.manufacturer)?;
        if let Some(old) = store.get(&p.number)? {
            may_edit(&user, &old.manufacturer)?;
        }
    }
    let imported = store.put_parts(&user.name, parts)?.len();
    log::info!("imported {imported} parts ({})", user.name);
    Ok(Json(ImportResult { imported, revision: store.revision }))
}

async fn export_csv(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let parts = store(&state).all()?;
    Ok(([(header::CONTENT_TYPE, "text/csv; charset=utf-8"), (header::CONTENT_DISPOSITION, "attachment; filename=\"parts.csv\"")], to_csv(&parts)))
}

async fn catalogs(State(state): State<AppState>) -> Result<Json<Vec<Catalog>>> {
    Ok(Json(store(&state).catalogs()?))
}

async fn get_catalog(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<Catalog>> {
    store(&state).catalog(&id)?.map(Json).ok_or_else(|| not_found(format!("catalog '{id}' not found")))
}

async fn put_catalog(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>, Json(mut catalog): Json<Catalog>) -> Result<Json<Catalog>> {
    let user = user(&state, &headers)?;
    if id.trim().is_empty() || id.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(bad("a catalog id must not be empty or contain blanks"));
    }
    if catalog.id.is_empty() {
        catalog.id = id.clone();
    }
    if !catalog.id.eq_ignore_ascii_case(&id) {
        return Err(bad("catalog id in URL and body differ"));
    }
    if catalog.name.trim().is_empty() {
        catalog.name = catalog.id.clone();
    }
    let mut store = store(&state);
    may_edit(&user, &catalog.manufacturer)?;
    if let Some(old) = store.catalog(&id)? {
        may_edit(&user, &old.manufacturer)?;
        catalog.id = old.id;
    }
    log::info!("saved catalog {id} ({})", user.name);
    Ok(Json(store.put_catalog(catalog)?))
}

async fn delete_catalog(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>) -> Result<StatusCode> {
    let user = user(&state, &headers)?;
    let mut store = store(&state);
    let old = store.catalog(&id)?.ok_or_else(|| not_found(format!("catalog '{id}' not found")))?;
    may_edit(&user, &old.manufacturer)?;
    if old.parts > 0 {
        return Err(AppError(StatusCode::CONFLICT, format!("catalog '{id}' still has {} parts", old.parts)));
    }
    store.delete_catalog(&old.id)?;
    log::info!("deleted catalog {id} ({})", user.name);
    Ok(StatusCode::NO_CONTENT)
}

async fn login(State(state): State<AppState>, Json(login): Json<Login>) -> Result<Json<Session>> {
    let mut store = store(&state);
    let account = store.account(login.name.trim()).filter(|a| verify_password(&login.password, &a.password)).cloned();
    let Some(mut account) = account else {
        log::warn!("failed login of '{}'", login.name);
        return Err(AppError(StatusCode::UNAUTHORIZED, "wrong user name or password".into()));
    };
    let token = account.start_session();
    let user = account.user.clone();
    store.put_account(account)?;
    log::info!("{} signed in", user.name);
    Ok(Json(Session { token, user }))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<StatusCode> {
    let token = presented(&headers).unwrap_or("").to_string();
    let mut store = store(&state);
    if let Some(mut account) = store.session(&token).cloned() {
        account.end_session(&token);
        store.put_account(account)?;
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn favorites(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Vec<String>>> {
    let user = user(&state, &headers)?;
    Ok(Json(store(&state).favorites(&user.name)?))
}

async fn set_favorites(State(state): State<AppState>, headers: HeaderMap, Json(mut numbers): Json<Vec<String>>) -> Result<Json<Vec<String>>> {
    let user = user(&state, &headers)?;
    numbers.retain(|n| !n.trim().is_empty());
    numbers.dedup();
    store(&state).set_favorites(&user.name, numbers.clone())?;
    Ok(Json(numbers))
}

async fn users(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Vec<User>>> {
    with_role(&state, &headers, Role::Admin)?;
    Ok(Json(store(&state).accounts().iter().map(|a| a.user.clone()).collect()))
}

/// True if removing / demoting `name` would leave nobody who can manage users.
fn last_admin(state: &AppState, store: &Store, name: &str) -> bool {
    state.token.is_none() && !store.accounts().iter().any(|a| a.user.role == Role::Admin && !a.user.name.eq_ignore_ascii_case(name))
}

async fn put_user(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>, Json(update): Json<UserUpdate>) -> Result<Json<User>> {
    let admin = with_role(&state, &headers, Role::Admin)?;
    let name = name.trim().to_string();
    if name.is_empty() || name.chars().any(|c| c.is_whitespace() || c.is_control()) || name.eq_ignore_ascii_case(ADMIN) && state.token.is_some() {
        return Err(bad(format!("'{name}' cannot be used as a user name")));
    }
    if update.role == Role::Manufacturer && update.manufacturer.trim().is_empty() {
        return Err(bad("a manufacturer user needs a manufacturer"));
    }
    let mut store = store(&state);
    let mut account = match store.account(&name) {
        Some(a) => a.clone(),
        None if update.password.is_empty() => return Err(bad("a new user needs a password")),
        None => Account { user: User { name: name.clone(), ..Default::default() }, ..Default::default() },
    };
    if account.user.role == Role::Admin && update.role != Role::Admin && last_admin(&state, &store, &name) {
        return Err(AppError(StatusCode::CONFLICT, "the last administrator cannot be demoted".into()));
    }
    account.user.role = update.role;
    account.user.manufacturer = update.manufacturer.trim().to_string();
    if !update.password.is_empty() {
        account.password = hash_password(&update.password).map_err(|e| AppError(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        account.sessions.clear();
    }
    let user = account.user.clone();
    store.put_account(account)?;
    log::info!("saved user {} ({})", user.name, admin.name);
    Ok(Json(user))
}

async fn delete_user(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>) -> Result<StatusCode> {
    let admin = with_role(&state, &headers, Role::Admin)?;
    let mut store = store(&state);
    let account = store.account(&name).cloned().ok_or_else(|| not_found(format!("user '{name}' not found")))?;
    if account.user.role == Role::Admin && last_admin(&state, &store, &account.user.name) {
        return Err(AppError(StatusCode::CONFLICT, "the last administrator cannot be deleted".into()));
    }
    store.delete_account(&account.user.name)?;
    log::info!("deleted user {} ({})", account.user.name, admin.name);
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
