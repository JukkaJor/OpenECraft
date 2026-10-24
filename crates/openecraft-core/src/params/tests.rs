use super::*;

const PARAMS: &[Param] = &[param("page", Kind::Str, "Page"), optional("ids", Kind::Ids, "Items"), optional("points", Kind::Points, "Points"), optional("mirror", Kind::Bool, "Mirror")];

fn map(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

#[test]
fn check_rejects_unknown_missing_and_mistyped_arguments() {
    assert!(check(PARAMS, &map(json!({ "page": "1", "ids": [1, 2] }))).is_ok());
    assert_eq!(check(PARAMS, &map(json!({}))).unwrap_err(), "Missing parameter: page");
    assert_eq!(check(PARAMS, &map(json!({ "page": 1 }))).unwrap_err(), "Parameter page has the wrong type");
    assert_eq!(check(PARAMS, &map(json!({ "page": "1", "x": 1 }))).unwrap_err(), "Unknown parameter: x");
    assert!(check(PARAMS, &map(json!({ "page": "1", "points": [[0, 0], [1]] }))).is_err());
}

#[test]
fn schema_has_types_and_required_parameters() {
    let s = schema(PARAMS);
    assert_eq!(s["required"], json!(["page"]));
    assert_eq!(s["properties"]["ids"]["items"]["type"], "integer");
    assert_eq!(s["properties"]["mirror"]["description"], "Mirror");
}

#[test]
fn values_are_parsed_by_kind() {
    assert_eq!(Kind::Ids.parse("3, 4,5"), Some(json!([3, 4, 5])));
    assert_eq!(Kind::Points.parse("0,0 8,-4"), Some(json!([[0.0, 0.0], [8.0, -4.0]])));
    assert_eq!(Kind::Bool.parse("yes"), Some(json!(true)));
    assert_eq!(Kind::Int.parse("1.5"), None);
    assert_eq!(Kind::Object.parse("[1]"), None);
    let pair = |n: &str, v: Option<&str>| (n.to_string(), v.map(String::from));
    let m = from_cli(PARAMS, Some(r#"{"page": "1"}"#), &[pair("page", Some("2")), pair("mirror", None)]).unwrap();
    assert_eq!(Value::Object(m), json!({ "page": "2", "mirror": true }));
}

#[test]
fn params_of_external_plugins_are_read_from_json() {
    let p: Param = serde_json::from_value(json!({ "name": "length", "kind": "num" })).unwrap();
    assert!(p.required);
    assert_eq!(p.kind, Kind::Num);
    assert!(matches!(p.name, Cow::Owned(_)));
}
