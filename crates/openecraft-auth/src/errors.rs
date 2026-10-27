//! Errors of the servers as JSON: every answer with an error status carries
//! `{"error": "…"}` (`openecraft_parts::api::ApiError`), also the plain text answers
//! of the router (unknown route, wrong method, a body or query that does not parse).

use axum::body::{to_bytes, Body};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::Response;

/// Largest plain text error kept.
const MAX_TEXT: usize = 16 * 1024;

/// Middleware for `axum::middleware::map_response`: turns error answers that are not
/// JSON into `{"error": "…"}`.
pub async fn json_errors(resp: Response) -> Response {
    let status = resp.status();
    let json = resp.headers().get(header::CONTENT_TYPE).is_some_and(|v| v.as_bytes().starts_with(b"application/json"));
    if !(status.is_client_error() || status.is_server_error()) || json {
        return resp;
    }
    let (mut parts, body) = resp.into_parts();
    let text = to_bytes(body, MAX_TEXT).await.map(|b| String::from_utf8_lossy(&b).trim().to_string()).unwrap_or_default();
    let body = serde_json::json!({ "error": error_text(status, &text) }).to_string();
    parts.headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
    parts.headers.remove(header::CONTENT_LENGTH);
    Response::from_parts(parts, Body::from(body))
}

/// The message of an error answer: its text, or the reason of the status.
pub fn error_text(status: StatusCode, text: &str) -> String {
    if text.is_empty() {
        status.canonical_reason().unwrap_or("error").to_lowercase()
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests;
