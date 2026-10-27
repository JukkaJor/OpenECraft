use super::*;
use openecraft_api::{optional, param};
use serde_json::json;

#[test]
fn form_texts_become_typed_arguments() {
    let params = [param("tag", Kind::Str, "Tag"), optional("count", Kind::Int, "Count"), optional("ids", Kind::Ids, "Ids")];
    let texts = |v: [&str; 3]| v.map(String::from).to_vec();
    let args = form_args(&params, &texts(["-K1", "3", ""])).unwrap();
    assert_eq!(Value::Object(args), json!({ "tag": "-K1", "count": 3 }));
    assert_eq!(form_args(&params, &texts(["-K1", "", "1, 2"])).unwrap()["ids"], json!([1, 2]));
    assert!(form_args(&params, &texts(["-K1", "three", ""])).is_err());
}
