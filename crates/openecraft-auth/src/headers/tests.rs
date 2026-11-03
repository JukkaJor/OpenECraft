use super::*;
use axum::body::Body;
use axum::http::Request;

#[tokio::test]
async fn answers_get_the_security_headers() {
    let resp = security_headers(Response::new(Body::from("x"))).await;
    let h = resp.headers();
    assert_eq!(h[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    assert_eq!(h[header::X_FRAME_OPTIONS], "DENY");
    assert_eq!(h[header::REFERRER_POLICY], "no-referrer");
    assert_eq!(h[header::CONTENT_SECURITY_POLICY], DEFAULT_CSP);
    // A policy of the answer is kept.
    let mut own = Response::new(Body::from("x"));
    own.headers_mut().insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'self'"));
    assert_eq!(security_headers(own).await.headers()[header::CONTENT_SECURITY_POLICY], "default-src 'self'");
}

#[tokio::test]
async fn client_address() {
    let (mut parts, _) = Request::new(Body::empty()).into_parts();
    assert_eq!(ClientIp::from_request_parts(&mut parts, &()).await.unwrap(), ClientIp(None));
    parts.extensions.insert(ConnectInfo(SocketAddr::from(([192, 0, 2, 7], 4711))));
    assert_eq!(ClientIp::from_request_parts(&mut parts, &()).await.unwrap(), ClientIp(Some(IpAddr::from([192, 0, 2, 7]))));
}
