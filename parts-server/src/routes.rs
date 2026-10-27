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
use openecraft_auth::{AccessFile, SignInError, Throttle};
use openecraft_parts::access::{Access, Policy, Right, Settings, SERVER};
use openecraft_parts::api::{
    bearer_token, token_matches, ApiError, ImportResult, Login, PartList, PartQuery, ServerInfo, Session, UserUpdate, API_PREFIX,
};
use openecraft_parts::portal::{content_type, AttachmentKind, Catalog, HistoryEntry, PartsRight, Role, SearchResult, User};
use openecraft_parts::{import, to_csv, Part};
use serde::Deserialize;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};

/// Name of the user of the administrator token.
pub const ADMIN: &str = "admin";

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Mutex<Store>>,
    /// Administrator token (`--token`). Without it, user accounts and directories
    /// the server is read-only.
    pub token: Option<String>,
    /// Directories, roles and grants (`--access`). Always locked after `store`.
    pub access: Arc<Mutex<AccessFile<PartsRight>>>,
    /// Failed sign-ins per user name.
    pub throttle: Arc<Mutex<Throttle>>,
}

impl AppState {
    pub fn new(store: Store, token: Option<String>, access: AccessFile<PartsRight>) -> AppState {
        AppState { store: Arc::new(Mutex::new(store)), token, access: Arc::new(Mutex::new(access)), throttle: Arc::default() }
    }
}

/// Access settings of a parts server without a file: anonymous requests may read.
pub fn default_access() -> openecraft_auth::AccessConfig<PartsRight> {
    openecraft_auth::AccessConfig { policy: Policy { anonymous: Role::Reader.name().into(), ..Default::default() }, ..Default::default() }
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
        .route(&api("/access"), get(access).put(save_access))
        .layer(DefaultBodyLimit::max(32 * 1024 * 1024))
        .layer(axum::middleware::map_response(openecraft_auth::json_errors))
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

fn throttle(state: &AppState) -> MutexGuard<'_, Throttle> {
    state.throttle.lock().unwrap_or_else(|e| e.into_inner())
}

fn access_file(state: &AppState) -> MutexGuard<'_, AccessFile<PartsRight>> {
    state.access.lock().unwrap_or_else(|e| e.into_inner())
}

fn presented(headers: &HeaderMap) -> Option<&str> {
    headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).and_then(bearer_token).filter(|t| !t.is_empty())
}

/// Who makes a request and what they may do.
struct Caller {
    /// The signed-in user; the administrator token is the user `admin`.
    user: Option<User>,
    access: Access<PartsRight>,
    /// A token was sent that is neither the administrator token nor a session.
    unknown_token: bool,
}

/// The caller of a request: the administrator token, a session or an anonymous
/// caller (an unknown token is marked: reading with it fails).
fn caller(state: &AppState, headers: &HeaderMap) -> Caller {
    let token = presented(headers);
    if token.is_some_and(|p| state.token.as_deref().is_some_and(|t| token_matches(p, t))) {
        return Caller { user: Some(User { name: ADMIN.into(), role: Role::Admin, ..Default::default() }), access: Access::full(), unknown_token: false };
    }
    let user = token.and_then(|t| store(state).session(t).map(|a| a.user.clone()));
    let file = access_file(state);
    let policy = &file.config.policy;
    match user {
        Some(u) => Caller { access: policy.access(&u.identity(), &u.grants()).with(policy.anonymous_access()), user: Some(u), unknown_token: false },
        None => Caller { user: None, access: policy.anonymous_access(), unknown_token: token.is_some() },
    }
}

fn wrong_token() -> AppError {
    AppError(StatusCode::UNAUTHORIZED, "missing or wrong access token".into())
}

/// True if anybody can change the database.
fn writable(state: &AppState, store: &Store) -> bool {
    state.token.is_some() || !store.accounts().is_empty() || !access_file(state).config.directories.is_empty()
}

/// The signed in user (any role) and their rights.
fn user(state: &AppState, headers: &HeaderMap) -> Result<(User, Access<PartsRight>)> {
    let c = caller(state, headers);
    if let Some(u) = c.user {
        return Ok((u, c.access));
    }
    if !writable(state, &store(state)) {
        return Err(forbidden("server is read-only (start it with --token to allow changes)"));
    }
    Err(wrong_token())
}

/// The signed in user if they have the right on the manufacturer's data.
fn with_right(state: &AppState, headers: &HeaderMap, right: PartsRight, manufacturer: &str) -> Result<(User, Access<PartsRight>)> {
    let (u, access) = user(state, headers)?;
    need(&u, &access, right, manufacturer)?;
    Ok((u, access))
}

fn need(user: &User, access: &Access<PartsRight>, right: PartsRight, manufacturer: &str) -> Result<()> {
    if access.allows(manufacturer.trim(), right) {
        Ok(())
    } else if right.server_wide() {
        Err(forbidden(format!("user '{}' is not allowed to do this", user.name)))
    } else if user.role == Role::Manufacturer && !access.anywhere(right) {
        Err(forbidden(format!("user '{}' may only change data of manufacturer '{}'", user.name, user.manufacturer)))
    } else {
        Err(forbidden(format!("user '{}' lacks the right '{}' for manufacturer '{}'", user.name, right.name(), manufacturer.trim())))
    }
}

/// Reading needs the right to read: anonymous requests have it unless the policy
/// takes it away (`anonymous` role).
fn reader(state: &AppState, headers: &HeaderMap) -> Result<()> {
    let c = caller(state, headers);
    // A wrong or expired token is an error, not an anonymous request.
    if c.unknown_token {
        return Err(wrong_token());
    }
    if c.access.allows(SERVER, PartsRight::Read) {
        Ok(())
    } else if c.user.is_none() {
        Err(AppError(StatusCode::UNAUTHORIZED, "sign in to read the parts".into()))
    } else {
        Err(forbidden("you may not read the parts"))
    }
}

async fn info(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<ServerInfo>> {
    let Caller { user, access, unknown_token } = caller(&state, &headers);
    if unknown_token {
        return Err(wrong_token());
    }
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
        access,
    }))
}

async fn list(State(state): State<AppState>, headers: HeaderMap, Query(query): Query<PartQuery>) -> Result<Json<PartList>> {
    reader(&state, &headers)?;
    let store = store(&state);
    let (total, parts) = store.list(&query)?;
    Ok(Json(PartList { revision: store.revision, total, offset: query.offset.unwrap_or(0), parts }))
}

async fn search(State(state): State<AppState>, headers: HeaderMap, Json(query): Json<PartQuery>) -> Result<Json<SearchResult>> {
    reader(&state, &headers)?;
    Ok(Json(store(&state).search(&query)?))
}

/// Longest part number.
const MAX_NUMBER_LEN: usize = 128;
/// Longest short text of a part (type number, manufacturer, groups, ...).
const MAX_FIELD_LEN: usize = 256;
/// Longest long text of a part (description, note, technical data, ...).
const MAX_TEXT_LEN: usize = 20_000;

/// A part number may not be empty, start or end with blanks, hold control characters
/// or be longer than [`MAX_NUMBER_LEN`].
fn check_number(number: &str) -> Result<()> {
    if number.trim().is_empty() {
        return Err(bad("part number must not be empty"));
    }
    if number.trim() != number || number.chars().any(char::is_control) {
        return Err(bad(format!("part number '{}' must not start or end with blanks", number.escape_debug())));
    }
    if number.chars().count() > MAX_NUMBER_LEN {
        let start: String = number.chars().take(20).collect();
        return Err(bad(format!("part number '{start}...' is longer than {MAX_NUMBER_LEN} characters")));
    }
    Ok(())
}

/// The number and the texts of a part must not exceed their lengths.
fn check_part(part: &Part) -> Result<()> {
    check_number(&part.number)?;
    let short = [
        ("type number", &part.type_number),
        ("order number", &part.order_number),
        ("manufacturer", &part.manufacturer),
        ("supplier", &part.supplier),
        ("product group", &part.group),
        ("product subgroup", &part.subgroup),
        ("identifier", &part.identifier),
        ("currency", &part.currency),
        ("quantity unit", &part.quantity_unit),
        ("voltage", &part.voltage),
        ("current", &part.current),
        ("power", &part.power),
        ("mounting", &part.mounting),
        ("catalog", &part.catalog),
        ("successor", &part.successor),
    ];
    let long = [("description", &part.description), ("note", &part.note), ("technical data", &part.tech_data), ("certificates", &part.certificates), ("datasheet", &part.datasheet)];
    for (max, fields) in [(MAX_FIELD_LEN, &short[..]), (MAX_TEXT_LEN, &long[..])] {
        if let Some((name, _)) = fields.iter().find(|(_, v)| v.chars().count() > max) {
            return Err(bad(format!("the {name} of part '{}' is longer than {max} characters", part.number)));
        }
    }
    Ok(())
}

/// The catalog a part names must exist.
fn check_catalog(store: &Store, part: &Part) -> Result<()> {
    if !part.catalog.trim().is_empty() && store.catalog(part.catalog.trim())?.is_none() {
        return Err(bad(format!("catalog '{}' of part '{}' not found: create the catalog first", part.catalog.trim(), part.number)));
    }
    Ok(())
}

/// A new part must not differ from an existing one only in case.
fn check_new_number(store: &Store, number: &str) -> Result<()> {
    if store.get(number)?.is_none() {
        if let Some(other) = store.other_case(number)? {
            return Err(AppError(StatusCode::CONFLICT, format!("part '{other}' already exists")));
        }
    }
    Ok(())
}

fn part_of(store: &Store, number: &str) -> Result<Part> {
    store.get(number)?.ok_or_else(|| not_found(format!("part '{number}' not found")))
}

async fn get_part(State(state): State<AppState>, headers: HeaderMap, Path(number): Path<String>) -> Result<Json<Part>> {
    reader(&state, &headers)?;
    part_of(&store(&state), &number).map(Json)
}

async fn put_part(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(number): Path<String>,
    Json(mut part): Json<Part>,
) -> Result<(StatusCode, Json<Part>)> {
    let (user, access) = user(&state, &headers)?;
    check_number(&number)?;
    if part.number.is_empty() {
        part.number = number.clone();
    }
    if part.number != number {
        return Err(bad("part number in URL and body differ"));
    }
    if part.successor == part.number {
        return Err(bad("a part cannot be its own successor"));
    }
    check_part(&part)?;
    let mut store = store(&state);
    check_catalog(&store, &part)?;
    need(&user, &access, PartsRight::EditParts, &part.manufacturer)?;
    if let Some(old) = store.get(&number)? {
        need(&user, &access, PartsRight::EditParts, &old.manufacturer)?;
    }
    check_new_number(&store, &number)?;
    if !part.successor.is_empty() && store.get(&part.successor)?.is_none() {
        return Err(bad(format!("successor '{}' not found", part.successor)));
    }
    let (stored, created) = store.put_parts(&user.name, vec![part])?.pop().expect("one part");
    log::info!("{} part {number} ({})", if created { "created" } else { "updated" }, user.name);
    Ok((if created { StatusCode::CREATED } else { StatusCode::OK }, Json(stored)))
}

async fn delete_part(State(state): State<AppState>, headers: HeaderMap, Path(number): Path<String>) -> Result<StatusCode> {
    let (user, access) = user(&state, &headers)?;
    let mut store = store(&state);
    let old = part_of(&store, &number)?;
    need(&user, &access, PartsRight::DeleteParts, &old.manufacturer)?;
    store.delete(&user.name, &number)?;
    log::info!("deleted part {number} ({})", user.name);
    Ok(StatusCode::NO_CONTENT)
}

async fn history(State(state): State<AppState>, headers: HeaderMap, Path(number): Path<String>) -> Result<Json<Vec<HistoryEntry>>> {
    reader(&state, &headers)?;
    let store = store(&state);
    let history = store.history(&number)?;
    if history.is_empty() && store.get(&number)?.is_none() {
        return Err(not_found(format!("part '{number}' not found")));
    }
    Ok(Json(history))
}

async fn history_part(State(state): State<AppState>, headers: HeaderMap, Path((number, revision)): Path<(String, u64)>) -> Result<Json<Part>> {
    reader(&state, &headers)?;
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
    let (user, access) = user(&state, &headers)?;
    let name = file_name(&q.name);
    if name.is_empty() {
        return Err(bad("the file needs a name"));
    }
    if body.is_empty() {
        return Err(bad("the file is empty"));
    }
    let mut store = store(&state);
    need(&user, &access, PartsRight::EditParts, &part_of(&store, &number)?.manufacturer)?;
    let part = store.attach(&user.name, &number, name, q.kind, body.to_vec())?.ok_or_else(|| not_found(format!("part '{number}' not found")))?;
    log::info!("attached {name} to part {number} ({})", user.name);
    Ok(Json(part))
}

async fn detach(State(state): State<AppState>, headers: HeaderMap, Path((number, id)): Path<(String, String)>) -> Result<Json<Part>> {
    let (user, access) = user(&state, &headers)?;
    let mut store = store(&state);
    need(&user, &access, PartsRight::EditParts, &part_of(&store, &number)?.manufacturer)?;
    store.detach(&user.name, &number, &id)?.map(Json).ok_or_else(|| not_found(format!("part '{number}' has no file '{id}'")))
}

async fn file(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>) -> Result<Response> {
    reader(&state, &headers)?;
    let f = store(&state).file(&id)?.ok_or_else(|| not_found(format!("file '{id}' not found")))?;
    let disposition = format!("inline; filename=\"{}\"", f.name.replace(['"', '\\', '\r', '\n'], "_"));
    Ok(([(header::CONTENT_TYPE, content_type(&f.name).to_string()), (header::CONTENT_DISPOSITION, disposition)], f.data).into_response())
}

/// Body: CSV (header names as in the export), JSON (parts database or part list) or BMEcat.
async fn import_parts(State(state): State<AppState>, headers: HeaderMap, body: String) -> Result<Json<ImportResult>> {
    let (user, access) = user(&state, &headers)?;
    if !access.anywhere(PartsRight::EditParts) {
        return Err(forbidden(format!("user '{}' is not allowed to do this", user.name)));
    }
    let parts = import(&body).map_err(|e| bad(format!("invalid parts data: {e}")))?;
    if parts.iter().any(|p| p.number.trim().is_empty()) {
        return Err(bad("every part needs a part number"));
    }
    for p in &parts {
        check_part(p)?;
    }
    let mut seen = std::collections::BTreeSet::new();
    if let Some(dup) = parts.iter().find(|p| !seen.insert(p.number.to_lowercase())) {
        return Err(bad(format!("part '{}' appears more than once in the file", dup.number)));
    }
    let mut store = store(&state);
    for p in &parts {
        check_catalog(&store, p)?;
        need(&user, &access, PartsRight::EditParts, &p.manufacturer)?;
        if let Some(old) = store.get(&p.number)? {
            need(&user, &access, PartsRight::EditParts, &old.manufacturer)?;
        }
        check_new_number(&store, &p.number)?;
        if !p.successor.is_empty() && !parts.iter().any(|q| q.number == p.successor) && store.get(&p.successor)?.is_none() {
            return Err(bad(format!("successor '{}' of part '{}' not found", p.successor, p.number)));
        }
    }
    let imported = store.put_parts(&user.name, parts)?.len();
    log::info!("imported {imported} parts ({})", user.name);
    Ok(Json(ImportResult { imported, revision: store.revision }))
}

/// All parts, or those the query of the address selects (text, groups, manufacturer,
/// catalog, lifecycle, sort order).
async fn export_csv(State(state): State<AppState>, headers: HeaderMap, Query(mut query): Query<PartQuery>) -> Result<impl IntoResponse> {
    reader(&state, &headers)?;
    query.brief = false;
    let parts = store(&state).list(&query)?.1;
    Ok(([(header::CONTENT_TYPE, "text/csv; charset=utf-8"), (header::CONTENT_DISPOSITION, "attachment; filename=\"parts.csv\"")], to_csv(&parts)))
}

async fn catalogs(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Vec<Catalog>>> {
    reader(&state, &headers)?;
    Ok(Json(store(&state).catalogs()?))
}

async fn get_catalog(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>) -> Result<Json<Catalog>> {
    reader(&state, &headers)?;
    store(&state).catalog(&id)?.map(Json).ok_or_else(|| not_found(format!("catalog '{id}' not found")))
}

async fn put_catalog(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>, Json(mut catalog): Json<Catalog>) -> Result<Json<Catalog>> {
    let (user, access) = user(&state, &headers)?;
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
    need(&user, &access, PartsRight::EditCatalogs, &catalog.manufacturer)?;
    if let Some(old) = store.catalog(&id)? {
        need(&user, &access, PartsRight::EditCatalogs, &old.manufacturer)?;
        catalog.id = old.id;
    }
    log::info!("saved catalog {id} ({})", user.name);
    Ok(Json(store.put_catalog(catalog)?))
}

async fn delete_catalog(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>) -> Result<StatusCode> {
    let (user, access) = user(&state, &headers)?;
    let mut store = store(&state);
    let old = store.catalog(&id)?.ok_or_else(|| not_found(format!("catalog '{id}' not found")))?;
    need(&user, &access, PartsRight::EditCatalogs, &old.manufacturer)?;
    if old.parts > 0 {
        return Err(AppError(StatusCode::CONFLICT, format!("catalog '{id}' still has {} parts", old.parts)));
    }
    store.delete_catalog(&old.id)?;
    log::info!("deleted catalog {id} ({})", user.name);
    Ok(StatusCode::NO_CONTENT)
}

/// Signs in with a local account or, if the server has none of that name, with its
/// directories; a directory user gets an account (reader, groups, sessions) on the
/// first sign-in.
async fn login(State(state): State<AppState>, Json(login): Json<Login>) -> Result<Json<Session>> {
    if let Some(wait) = throttle(&state).wait(&login.name, Instant::now()) {
        return Err(AppError(StatusCode::TOO_MANY_REQUESTS, format!("too many failed sign-ins, try again in {wait} s")));
    }
    let refused = || {
        throttle(&state).failed(&login.name, Instant::now());
        AppError(StatusCode::UNAUTHORIZED, "wrong user name or password".into())
    };
    let local = store(&state).account(login.name.trim()).filter(|a| a.user.provider.is_empty() && login.provider.trim().is_empty()).cloned();
    let mut account = match local {
        Some(a) if verify_password(&login.password, &a.password) => a,
        Some(_) => {
            log::warn!("failed login of '{}'", login.name);
            return Err(refused());
        }
        None => {
            let directories = access_file(&state).config.directories.clone();
            let identity = match openecraft_auth::sign_in(&directories, &login.provider, &login.name, &login.password).await {
                Ok(identity) => identity,
                Err(SignInError::Unavailable(e)) => return Err(AppError(StatusCode::SERVICE_UNAVAILABLE, e)),
                Err(_) => {
                    log::warn!("failed login of '{}'", login.name);
                    return Err(refused());
                }
            };
            let mut account = match store(&state).account(&identity.name).cloned() {
                Some(a) if a.user.provider.eq_ignore_ascii_case(&identity.provider) => a,
                Some(_) => return Err(AppError(StatusCode::CONFLICT, format!("the name '{}' belongs to another account", identity.name))),
                None => Account { user: User { name: identity.name.clone(), provider: identity.provider.clone(), ..Default::default() }, ..Default::default() },
            };
            account.user.groups = identity.groups;
            account
        }
    };
    throttle(&state).succeeded(&login.name);
    let token = account.start_session();
    let user = account.user.clone();
    store(&state).put_account(account)?;
    log::info!("{} signed in", user.name);
    let file = access_file(&state);
    let policy = &file.config.policy;
    let access = policy.access(&user.identity(), &user.grants()).with(policy.anonymous_access());
    Ok(Json(Session { token, user, access }))
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
    let (user, _) = with_right(&state, &headers, PartsRight::Read, SERVER)?;
    Ok(Json(store(&state).favorites(&user.name)?))
}

async fn set_favorites(State(state): State<AppState>, headers: HeaderMap, Json(mut numbers): Json<Vec<String>>) -> Result<Json<Vec<String>>> {
    let (user, _) = with_right(&state, &headers, PartsRight::Read, SERVER)?;
    let mut store = store(&state);
    let mut kept: Vec<String> = Vec::new();
    for n in numbers.drain(..) {
        if !kept.contains(&n) && store.get(&n)?.is_some() {
            kept.push(n);
        }
    }
    store.set_favorites(&user.name, kept.clone())?;
    Ok(Json(kept))
}

async fn users(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Vec<User>>> {
    with_right(&state, &headers, PartsRight::Administer, SERVER)?;
    Ok(Json(store(&state).accounts().iter().map(|a| a.user.clone()).collect()))
}

/// True if removing / demoting `name` would leave nobody who can manage users.
fn last_admin(state: &AppState, store: &Store, name: &str) -> bool {
    let file = access_file(state);
    let policy = &file.config.policy;
    let admin = |u: &User| policy.access(&u.identity(), &u.grants()).allows(SERVER, PartsRight::Administer);
    state.token.is_none() && !store.accounts().iter().any(|a| !a.user.name.eq_ignore_ascii_case(name) && admin(&a.user))
}

async fn put_user(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>, Json(update): Json<UserUpdate>) -> Result<Json<User>> {
    let (admin, _) = with_right(&state, &headers, PartsRight::Administer, SERVER)?;
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
    if !account.user.provider.is_empty() && !update.password.is_empty() {
        return Err(bad(format!("'{name}' signs in with directory {}, which keeps the password", account.user.provider)));
    }
    let was_admin = account.user.role == Role::Admin;
    account.user.role = update.role;
    account.user.manufacturer = update.manufacturer.trim().to_string();
    if let Some(groups) = update.groups.filter(|_| account.user.provider.is_empty()) {
        account.user.groups = groups.into_iter().map(|g| g.trim().to_string()).filter(|g| !g.is_empty()).collect();
    }
    if was_admin && update.role != Role::Admin && last_admin(&state, &store, &name) {
        return Err(AppError(StatusCode::CONFLICT, "the last administrator cannot be demoted".into()));
    }
    if !update.password.is_empty() {
        openecraft_auth::check_password(&update.password).map_err(bad)?;
        account.password = hash_password(&update.password).map_err(|e| AppError(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        account.sessions.clear();
    }
    let user = account.user.clone();
    store.put_account(account)?;
    log::info!("saved user {} ({})", user.name, admin.name);
    Ok(Json(user))
}

async fn delete_user(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>) -> Result<StatusCode> {
    let (admin, _) = with_right(&state, &headers, PartsRight::Administer, SERVER)?;
    let mut store = store(&state);
    let account = store.account(&name).cloned().ok_or_else(|| not_found(format!("user '{name}' not found")))?;
    if account.user.role == Role::Admin && last_admin(&state, &store, &account.user.name) {
        return Err(AppError(StatusCode::CONFLICT, "the last administrator cannot be deleted".into()));
    }
    store.delete_account(&account.user.name)?;
    log::info!("deleted user {} ({})", account.user.name, admin.name);
    Ok(StatusCode::NO_CONTENT)
}

async fn access(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Settings<PartsRight>>> {
    with_right(&state, &headers, PartsRight::Administer, SERVER)?;
    Ok(Json(access_file(&state).settings()))
}

async fn save_access(State(state): State<AppState>, headers: HeaderMap, Json(policy): Json<Policy<PartsRight>>) -> Result<Json<Settings<PartsRight>>> {
    let (admin, _) = with_right(&state, &headers, PartsRight::Administer, SERVER)?;
    let mut file = access_file(&state);
    file.set_policy(policy).map_err(bad)?;
    log::info!("access settings changed ({})", admin.name);
    Ok(Json(file.settings()))
}

#[cfg(test)]
mod tests;
