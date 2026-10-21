use super::*;
use openecraft_parts::api::{ImportResult, ServerInfo};

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

fn endpoint() -> Endpoint {
    Endpoint { url: "http://host:8090/".into(), token: " secret ".into() }
}

#[test]
fn requests_of_the_api() {
    let e = endpoint();
    assert_eq!(e.info().url, "http://host:8090/api/v1/info");
    let q = PartQuery { group: Some("Fuses".into()), limit: Some(5), ..Default::default() };
    assert_eq!(e.list(&q).url, "http://host:8090/api/v1/parts?group=Fuses&limit=5");
    assert_eq!(e.part("A B").url, "http://host:8090/api/v1/parts/A%20B");
    assert_eq!(e.export().url, "http://host:8090/api/v1/export.csv");
    // Reading needs no token.
    assert_eq!(e.info().headers.get("Authorization"), None);
}

#[test]
fn changes_carry_the_token_and_one_content_type() {
    let e = endpoint();
    let part = Part { number: "GEN.X".into(), ..Default::default() };
    let save = e.save(&part);
    assert_eq!(save.method, ehttp::Method::PUT);
    assert_eq!(save.url, "http://host:8090/api/v1/parts/GEN.X");
    assert_eq!(save.headers.get_all("Content-Type").collect::<Vec<_>>(), ["application/json"]);
    assert_eq!(save.headers.get("Authorization"), Some("Bearer secret"));
    assert_eq!(serde_json::from_slice::<Part>(&save.body).unwrap(), part);

    let import = e.import(b"[]".to_vec(), "application/json");
    assert_eq!(import.method, ehttp::Method::POST);
    assert_eq!(import.headers.get_all("Content-Type").collect::<Vec<_>>(), ["application/json"]);
    assert_eq!(e.delete("GEN.X").headers.get("Authorization"), Some("Bearer secret"));

    let anonymous = Endpoint { token: "  ".into(), ..endpoint() };
    assert_eq!(anonymous.delete("GEN.X").headers.get("Authorization"), None);
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
fn missing_resources_and_empty_answers() {
    assert!(check(response(204, "")).is_ok());
    assert_eq!(check(response(404, r#"{"error": "part 'X' not found"}"#)).unwrap_err(), "part 'X' not found");
    assert_eq!(decode_found::<Part>(response(404, r#"{"error": "part 'X' not found"}"#)).unwrap(), None);
    assert!(decode_found::<Part>(response(500, "")).is_err());
    assert_eq!(decode_found::<ImportResult>(response(200, r#"{"imported": 2}"#)).unwrap().unwrap().imported, 2);
    assert_eq!(text(response(200, "a;b\n")).unwrap(), "a;b\n");
}
