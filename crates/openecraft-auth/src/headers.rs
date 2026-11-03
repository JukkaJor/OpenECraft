//! What every answer of the servers carries and what they know of the client: headers
//! that keep browsers from guessing types, framing pages or passing addresses on, and
//! the address of the client (failed sign-ins are counted per client).

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::request::Parts;
use axum::http::{header, HeaderValue};
use axum::response::Response;
use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};

/// Content security policy of answers without one of their own: nothing is loaded or
/// run, and the answer is not shown in frames.
pub const DEFAULT_CSP: &str = "default-src 'none'; frame-ancestors 'none'";

/// Middleware for `axum::middleware::map_response`: adds `X-Content-Type-Options`,
/// `X-Frame-Options`, `Referrer-Policy` and, if the answer has none,
/// [`DEFAULT_CSP`].
pub async fn security_headers(mut resp: Response) -> Response {
    let h = resp.headers_mut();
    h.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    h.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    h.insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    if !h.contains_key(header::CONTENT_SECURITY_POLICY) {
        h.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(DEFAULT_CSP));
    }
    resp
}

/// The address of the client, if the server was started with
/// `into_make_service_with_connect_info::<SocketAddr>()` (not in tests).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClientIp(pub Option<IpAddr>);

impl<S: Send + Sync> FromRequestParts<S> for ClientIp {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(ClientIp(parts.extensions.get::<ConnectInfo<SocketAddr>>().map(|c| c.0.ip())))
    }
}

#[cfg(test)]
mod tests;
