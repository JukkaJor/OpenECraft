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
fn merge_adds_and_updates() {
    let mut db = PartsDb { parts: vec![Part { number: "A".into(), description: "old".into(), ..Default::default() }] };
    let incoming = vec![
        Part { number: "A".into(), description: "new".into(), ..Default::default() },
        Part { number: "B".into(), ..Default::default() },
    ];
    assert_eq!(merge(&mut db, incoming.clone()), (1, 1));
    assert_eq!(db.get("A").unwrap().description, "new");
    // Same data again: nothing changes.
    assert_eq!(merge(&mut db, incoming), (0, 0));
}
