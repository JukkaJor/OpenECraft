use super::*;

fn response(status: u16, body: &str) -> ehttp::Result<ehttp::Response> {
    Ok(ehttp::Response {
        url: "http://test".into(),
        ok: (200..300).contains(&status),
        status,
        status_text: "Status".into(),
        headers: ehttp::Headers::default(),
        bytes: body.as_bytes().to_vec(),
    })
}

#[test]
fn decode_success_and_errors() {
    let info: ServerInfo = decode(response(200, r#"{"name": "srv", "parts": 3, "writable": true}"#)).unwrap();
    assert_eq!((info.name.as_str(), info.parts, info.writable), ("srv", 3, true));
    // Server error messages are passed through.
    let err = decode::<ServerInfo>(response(401, r#"{"error": "missing or wrong access token"}"#)).unwrap_err();
    assert_eq!(err, "missing or wrong access token");
    // Non-JSON errors fall back to the HTTP status.
    let err = decode::<ServerInfo>(response(502, "Bad gateway")).unwrap_err();
    assert!(err.starts_with("502"));
    let err = decode::<ServerInfo>(Err("refused".into())).unwrap_err();
    assert!(err.contains("refused"));
    let err = decode::<ServerInfo>(response(200, "<html>")).unwrap_err();
    assert!(err.starts_with("unexpected answer"));
}

#[test]
fn empty_answers_are_fine_for_deletes() {
    assert!(check(response(204, "")).is_ok());
    assert_eq!(check(response(404, r#"{"error": "part 'X' not found"}"#)).unwrap_err(), "part 'X' not found");
}

#[test]
fn poll_updates_status_and_info() {
    let mut client = ServerClient { pending: 2, ..Default::default() };
    {
        let mut inbox = client.inbox.lock().unwrap();
        inbox.push(Reply::Info(Ok(ServerInfo { name: "srv".into(), writable: true, ..Default::default() })));
        inbox.push(Reply::Deleted("A".into(), Ok(())));
    }
    let replies = client.poll();
    assert_eq!(replies.len(), 2);
    assert!(!client.busy());
    assert!(client.writable());
    assert_eq!(client.status, "Deleted part A");
    assert!(!client.failed());

    client.inbox.lock().unwrap().push(Reply::Info(Err("connection failed: refused".into())));
    client.poll();
    assert!(client.failed());
    assert!(client.info.is_none());
}

#[test]
fn content_type_replaces_the_default() {
    let req = with_content_type(ehttp::Request::put("http://test", Vec::new()), "application/json");
    let types: Vec<&str> = req.headers.get_all("Content-Type").collect();
    assert_eq!(types, ["application/json"]);
}
