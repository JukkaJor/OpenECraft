//! HTTP routes of the parts server (see `openecraft_parts::api` for the API).

use crate::store::Store;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use openecraft_parts::api::{bearer_token, token_matches, ApiError, ImportResult, PartList, PartQuery, ServerInfo, API_PREFIX};
use openecraft_parts::{import, to_csv, Part};
use std::sync::{Arc, RwLock};
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<RwLock<Store>>,
    /// Bearer token for write requests. `None` = read-only server.
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

type Result<T> = std::result::Result<T, AppError>;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route(&format!("{API_PREFIX}/info"), get(info))
        .route(&format!("{API_PREFIX}/parts"), get(list))
        .route(&format!("{API_PREFIX}/parts/{{number}}"), get(get_part).put(put_part).delete(delete_part))
        .route(&format!("{API_PREFIX}/import"), post(import_parts))
        .route(&format!("{API_PREFIX}/export.csv"), get(export_csv))
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

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<()> {
    let Some(token) = &state.token else {
        return Err(AppError(StatusCode::FORBIDDEN, "server is read-only (start it with --token to allow changes)".into()));
    };
    let presented = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(bearer_token);
    match presented {
        Some(p) if token_matches(p, token) => Ok(()),
        _ => Err(AppError(StatusCode::UNAUTHORIZED, "missing or wrong access token".into())),
    }
}

fn read(state: &AppState) -> std::sync::RwLockReadGuard<'_, Store> {
    state.store.read().unwrap_or_else(|e| e.into_inner())
}

fn write(state: &AppState) -> std::sync::RwLockWriteGuard<'_, Store> {
    state.store.write().unwrap_or_else(|e| e.into_inner())
}

async fn index(State(state): State<AppState>) -> String {
    let store = read(&state);
    format!(
        "OpenECraft parts server {}\n\n{} parts, revision {}, {}\n\nAPI: {API_PREFIX}/info, {API_PREFIX}/parts, {API_PREFIX}/parts/{{number}}, {API_PREFIX}/import, {API_PREFIX}/export.csv\n",
        env!("CARGO_PKG_VERSION"),
        store.parts.len(),
        store.revision,
        if state.token.is_some() { "writable with token" } else { "read-only" }
    )
}

async fn info(State(state): State<AppState>) -> Json<ServerInfo> {
    let store = read(&state);
    Json(ServerInfo {
        name: "OpenECraft parts server".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        parts: store.parts.len(),
        revision: store.revision,
        writable: state.token.is_some(),
    })
}

async fn list(State(state): State<AppState>, Query(query): Query<PartQuery>) -> Json<PartList> {
    let store = read(&state);
    let (total, parts) = query.apply(&store.parts);
    Json(PartList { revision: store.revision, total, offset: query.offset.unwrap_or(0), parts })
}

async fn get_part(State(state): State<AppState>, Path(number): Path<String>) -> Result<Json<Part>> {
    let store = read(&state);
    store.get(&number).cloned().map(Json).ok_or_else(|| AppError(StatusCode::NOT_FOUND, format!("part '{number}' not found")))
}

async fn put_part(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(number): Path<String>,
    Json(mut part): Json<Part>,
) -> Result<(StatusCode, Json<Part>)> {
    authorize(&state, &headers)?;
    if number.trim().is_empty() {
        return Err(AppError(StatusCode::BAD_REQUEST, "part number must not be empty".into()));
    }
    if part.number.is_empty() {
        part.number = number.clone();
    }
    if part.number != number {
        return Err(AppError(StatusCode::BAD_REQUEST, "part number in URL and body differ".into()));
    }
    let created = write(&state).upsert(part.clone())?;
    log::info!("{} part {number}", if created { "created" } else { "updated" });
    Ok((if created { StatusCode::CREATED } else { StatusCode::OK }, Json(part)))
}

async fn delete_part(State(state): State<AppState>, headers: HeaderMap, Path(number): Path<String>) -> Result<StatusCode> {
    authorize(&state, &headers)?;
    if write(&state).delete(&number)? {
        log::info!("deleted part {number}");
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError(StatusCode::NOT_FOUND, format!("part '{number}' not found")))
    }
}

/// Body: CSV (header names as in the export) or JSON (parts database or part list).
async fn import_parts(State(state): State<AppState>, headers: HeaderMap, body: String) -> Result<Json<ImportResult>> {
    authorize(&state, &headers)?;
    let parts = import(&body).map_err(|e| AppError(StatusCode::BAD_REQUEST, format!("invalid parts data: {e}")))?;
    if parts.iter().any(|p| p.number.trim().is_empty()) {
        return Err(AppError(StatusCode::BAD_REQUEST, "every part needs a part number".into()));
    }
    let mut store = write(&state);
    let imported = store.import(parts)?;
    log::info!("imported {imported} parts");
    Ok(Json(ImportResult { imported, revision: store.revision }))
}

async fn export_csv(State(state): State<AppState>) -> impl IntoResponse {
    let store = read(&state);
    (
        [(header::CONTENT_TYPE, "text/csv; charset=utf-8"), (header::CONTENT_DISPOSITION, "attachment; filename=\"parts.csv\"")],
        to_csv(&store.parts),
    )
}

#[cfg(test)]
mod tests;
