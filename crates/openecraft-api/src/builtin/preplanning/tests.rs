use super::*;
use crate::Plugins;
use openecraft_core::analysis::Analysis;
use openecraft_core::sample::demo_project;
use serde_json::Map;

fn run(p: &Plugins, prj: &mut Project, name: &str, args: Value) -> Result<Value, String> {
    let args: Map<String, Value> = args.as_object().cloned().unwrap_or_default();
    p.run(p.command(name).ok_or("no command")?, prj, &args)
}

#[test]
fn segments_and_planned_devices_through_commands() {
    let p = Plugins::new(Vec::<String>::new());
    let mut prj = demo_project();
    run(&p, &mut prj, "add_segment", json!({ "name": "Plant" })).unwrap();
    let area = run(&p, &mut prj, "add_segment", json!({ "name": "Area 1", "parent": "Plant" })).unwrap();
    assert_eq!(area["path"], "Plant/Area 1");
    assert!(run(&p, &mut prj, "add_segment", json!({ "name": "Area 1", "parent": "Plant" })).is_err());
    assert!(run(&p, &mut prj, "add_segment", json!({ "name": "A/B" })).is_err());
    run(&p, &mut prj, "set_segment", json!({ "segment": "Plant/Area 1", "function": "=A5", "location": "+L2" })).unwrap();
    let d = run(&p, &mut prj, "plan_device", json!({ "segment": "Plant/Area 1", "tag": "-M1", "kind": "Motor", "power": 5.5 })).unwrap();
    assert_eq!(d["tag"], "=A5+L2-M1");
    assert_eq!(d["segment_power"], 5.5);
    run(&p, &mut prj, "save_segment_template", json!({ "segment": "Plant/Area 1", "name": "Drive" })).unwrap();
    run(&p, &mut prj, "add_segment", json!({ "name": "Area 2", "parent": "Plant" })).unwrap();
    assert_eq!(run(&p, &mut prj, "apply_segment_template", json!({ "segment": "Plant/Area 2", "template": "Drive" })).unwrap()["added"], 1);
    let tree = run(&p, &mut prj, "preplanning", json!({})).unwrap();
    assert_eq!(tree["segments"].as_array().unwrap().len(), 3);
    assert_eq!(tree["segments"][0]["power"], 11.0);
    assert_eq!(tree["segments"][1]["devices"][0]["state"], "planned");
    assert!(run(&p, &mut prj, "set_segment", json!({ "segment": "Plant", "parent": "Plant/Area 1" })).is_err());
    let table = (p.report("preplanning").unwrap().run)(&prj, &Analysis::build(&prj)).unwrap();
    assert_eq!(table.rows.len(), 2);
    run(&p, &mut prj, "remove_planned_device", json!({ "segment": "Plant/Area 2", "tag": "-M1" })).unwrap();
    assert!(run(&p, &mut prj, "remove_planned_device", json!({ "segment": "Plant/Area 2", "tag": "-M1" })).is_err());
    assert_eq!(run(&p, &mut prj, "remove_segment", json!({ "segment": "Plant" })).unwrap()["removed"], 3);
    assert!(run(&p, &mut prj, "remove_segment", json!({ "segment": "Plant" })).is_err());
}
