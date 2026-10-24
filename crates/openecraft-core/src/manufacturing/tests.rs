use super::*;
use crate::sample::demo_project;

#[test]
fn numbers_names_and_checksums() {
    assert_eq!(num(4.2), "4.2");
    assert_eq!(num(10.0), "10");
    assert_eq!(num(-0.0001), "0");
    assert_eq!(num(12.345), "12.345");
    assert_eq!(file_stem("+CAB1"), "CAB1");
    assert_eq!(file_stem("=A1+S 2"), "A1_S_2");
    assert_eq!(checksum("abc"), checksum("abc"));
    assert_ne!(checksum("abc"), checksum("abd"));
    assert_eq!(checksum("").len(), 16);
}

#[test]
fn default_data_is_not_stored() {
    let mut prj = demo_project();
    let json = serde_json::to_string(&prj).unwrap();
    assert!(!json.contains("\"manufacturing\""));
    prj.layout.manufacturing.face_machines.insert("door".into(), "punch_turret".into());
    assert_eq!(prj.layout.manufacturing.machine_of(Face::Door), "punch_turret");
    assert_eq!(prj.layout.manufacturing.machine_of(Face::Plate), machines::DEFAULT_SHEET);
    let back: Project = serde_json::from_str(&serde_json::to_string(&prj).unwrap()).unwrap();
    assert_eq!(back.layout.manufacturing, prj.layout.manufacturing);
    assert_eq!(enclosure_index(&prj, "+CAB1"), Some(0));
    assert_eq!(enclosure_index(&prj, "+X"), None);
}
