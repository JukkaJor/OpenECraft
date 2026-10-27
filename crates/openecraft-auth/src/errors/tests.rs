use super::*;
use axum::response::IntoResponse;

async fn body(resp: Response) -> (StatusCode, Option<String>, String) {
    let status = resp.status();
    let ctype = resp.headers().get(header::CONTENT_TYPE).map(|v| v.to_str().unwrap().to_string());
    let bytes = to_bytes(resp.into_body(), MAX_TEXT).await.unwrap();
    (status, ctype, String::from_utf8(bytes.to_vec()).unwrap())
}

#[tokio::test]
async fn plain_errors_become_json() {
    let resp = json_errors((StatusCode::BAD_REQUEST, "Failed to parse the request body").into_response()).await;
    let (status, ctype, text) = body(resp).await;
    assert_eq!((status, ctype.as_deref()), (StatusCode::BAD_REQUEST, Some("application/json")));
    assert_eq!(text, r#"{"error":"Failed to parse the request body"}"#);

    let (_, _, text) = body(json_errors(StatusCode::NOT_FOUND.into_response()).await).await;
    assert_eq!(text, r#"{"error":"not found"}"#);
}

#[tokio::test]
async fn other_answers_stay() {
    let (status, _, text) = body(json_errors((StatusCode::OK, "fine").into_response()).await).await;
    assert_eq!((status, text.as_str()), (StatusCode::OK, "fine"));
    let json = ([(header::CONTENT_TYPE, "application/json")], r#"{"error":"x"}"#);
    let (_, _, text) = body(json_errors((StatusCode::CONFLICT, json).into_response()).await).await;
    assert_eq!(text, r#"{"error":"x"}"#);
}
