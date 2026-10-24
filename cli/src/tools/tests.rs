use super::*;
use crate::testing::session;
use serde_json::json;

fn args(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

#[test]
fn tool_and_parameter_names_are_unique_snake_case() {
    let snake = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    let mut names = std::collections::HashSet::new();
    for tool in all() {
        assert!(snake(tool.name), "{}", tool.name);
        assert!(names.insert(tool.name), "duplicate tool {}", tool.name);
        assert!(!tool.help.is_empty());
        let mut params = std::collections::HashSet::new();
        for p in tool.params {
            assert!(snake(&p.name), "{}.{}", tool.name, p.name);
            assert!(params.insert(p.name.as_ref()), "duplicate parameter {}.{}", tool.name, p.name);
            assert!(!p.help.is_empty());
        }
    }
    assert!(names.len() > 30);
}

#[test]
fn schema_lists_types_and_required_parameters() {
    let s = schema(find("add_wire").unwrap());
    assert_eq!(s["type"], "object");
    assert_eq!(s["additionalProperties"], false);
    assert_eq!(s["required"], json!(["page", "points"]));
    assert_eq!(s["properties"]["page"]["type"], "string");
    assert_eq!(s["properties"]["points"]["items"]["maxItems"], 2);
    assert!(s["properties"]["points"]["description"].is_string());
}

#[test]
fn call_checks_the_parameters() {
    let mut s = session();
    let tool = find("open_project").unwrap();
    assert_eq!(call(&mut s, tool, &args(json!({}))).unwrap_err(), "Missing parameter: path");
    assert_eq!(call(&mut s, tool, &args(json!({ "path": 3 }))).unwrap_err(), "Parameter path has the wrong type");
    assert_eq!(call(&mut s, tool, &args(json!({ "path": "a", "x": 1 }))).unwrap_err(), "Unknown parameter: x");
    let err = call(&mut s, find("list_pages").unwrap(), &Map::new()).unwrap_err();
    assert!(err.starts_with("No project is open"), "{err}");
}

#[test]
fn command_line_values_follow_the_parameter_type() {
    let pair = |n: &str, v: Option<&str>| (n.to_string(), v.map(String::from));
    let place = find("place_symbol").unwrap();
    let map = from_cli(place, None, &[pair("x", Some("12.5")), pair("rotation", Some("90")), pair("mirror", None), pair("tag", Some("-K1"))]).unwrap();
    assert_eq!(Value::Object(map), json!({ "x": 12.5, "rotation": 90, "mirror": true, "tag": "-K1" }));
    assert!(from_cli(place, None, &[pair("x", Some("left"))]).is_err());
    assert!(from_cli(place, None, &[pair("tag", None)]).is_err());
    assert!(from_cli(place, None, &[pair("colour", Some("red"))]).is_err());

    let del = find("delete_items").unwrap();
    assert_eq!(from_cli(del, None, &[pair("ids", Some("3, 4,5"))]).unwrap()["ids"], json!([3, 4, 5]));
    assert_eq!(from_cli(del, None, &[pair("ids", Some("[6]"))]).unwrap()["ids"], json!([6]));

    let wire = find("add_wire").unwrap();
    assert_eq!(from_cli(wire, None, &[pair("points", Some("0,0 8,-4"))]).unwrap()["points"], json!([[0.0, 0.0], [8.0, -4.0]]));
    let map = from_cli(wire, Some(r#"{"page": "1", "points": [[0, 0], [4, 0]]}"#), &[pair("page", Some("2"))]).unwrap();
    assert_eq!(map["page"], "2", "--name overrides --json");
    assert!(from_cli(wire, Some("[1]"), &[]).is_err());
}

#[test]
fn help_shows_all_parameters() {
    let text = help(find("connect").unwrap());
    assert!(text.starts_with("openecraft-cli connect --project <file>"));
    for p in find("connect").unwrap().params {
        assert!(text.contains(&format!("--{}", p.name)));
    }
    assert!(text.contains("(optional)"));
}
