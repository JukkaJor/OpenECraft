use super::*;
use crate::macros::{from_page, MacroKind};
use crate::sample::demo_project;

fn device(tag: &str, power: f32) -> PlannedDevice {
    PlannedDevice { tag: tag.into(), kind: "Motor".into(), description: format!("Drive {tag}"), power, voltage: "400 V".into(), ..Default::default() }
}

#[test]
fn segments_form_a_tree_with_inherited_structure() {
    let mut prj = demo_project();
    let plant = add_segment(&mut prj, 0, "Plant");
    let area = add_segment(&mut prj, plant, "Area 1");
    let unit = add_segment(&mut prj, area, "Conveyor");
    segment_mut(&mut prj, plant).unwrap().location = "L1".into();
    segment_mut(&mut prj, area).unwrap().function = "A7".into();
    segment_mut(&mut prj, unit).unwrap().devices.push(device("-M9", 1.5));
    segment_mut(&mut prj, area).unwrap().devices.push(device("-M8", 4.0));
    assert_eq!(path(&prj, unit), "Plant/Area 1/Conveyor");
    assert_eq!(structure(&prj, unit), ("A7".to_string(), "L1".to_string()));
    let d = segment(&prj, unit).unwrap().devices[0].clone();
    assert_eq!(full_tag(&prj, unit, &d), "=A7+L1-M9");
    assert!((power(&prj, plant) - 5.5).abs() < 1e-4);
    let t: Vec<(usize, String)> = tree(&prj).into_iter().map(|(d, s)| (d, s.name.clone())).collect();
    assert_eq!(t, vec![(0, "Plant".into()), (1, "Area 1".into()), (2, "Conveyor".into())]);
    assert!(!move_segment(&mut prj, plant, unit), "not below its own child");
    assert_eq!(remove_segment(&mut prj, area), 2);
    assert_eq!(prj.preplanning.segments.len(), 1);
}

#[test]
fn planned_devices_are_compared_with_the_schematic() {
    let mut prj = demo_project();
    let idx = Index::build(&prj);
    let existing = idx.sorted_tags().into_iter().find(|t| prj.devices.get(*t).is_some_and(|d| !d.parts.is_empty())).unwrap().clone();
    let seg = add_segment(&mut prj, 0, "Plant");
    let dt = Dt::parse(&existing);
    {
        let s = segment_mut(&mut prj, seg).unwrap();
        s.function = dt.function.clone();
        s.location = dt.location.clone();
        s.devices.push(PlannedDevice { tag: dt.name.clone(), part: "OTHER.PART".into(), ..Default::default() });
        s.devices.push(device("-M77", 2.0));
        s.devices.push(device("-M77", 2.0));
    }
    let idx = Index::build(&prj);
    let r = rows(&prj, &idx);
    assert_eq!(r[0].tag, existing);
    assert_eq!(r[0].state, State::PartDiffers);
    assert_eq!(r[1].state, State::Planned);
    let issues = issues(&prj, &idx);
    assert!(issues.iter().any(|i| i.severity == Severity::Warning));
    assert!(issues.iter().any(|i| i.severity == Severity::Error), "planned twice");
    assert!(issues.iter().any(|i| i.severity == Severity::Info));
}

#[test]
fn templates_add_their_devices_once() {
    let mut prj = demo_project();
    let a = add_segment(&mut prj, 0, "A");
    let b = add_segment(&mut prj, 0, "B");
    segment_mut(&mut prj, a).unwrap().devices = vec![device("-M1", 1.0), device("-B1", 0.0)];
    save_template(&mut prj, a, "Drive").unwrap();
    assert_eq!(apply_template(&mut prj, b, "Drive").unwrap(), 2);
    assert_eq!(apply_template(&mut prj, b, "Drive").unwrap(), 0);
    assert!(apply_template(&mut prj, b, "Nope").is_err());
}

#[test]
fn csv_round_trip_creates_segments_and_devices() {
    let mut prj = demo_project();
    let unit = {
        let p = add_segment(&mut prj, 0, "Plant");
        add_segment(&mut prj, p, "Pump station")
    };
    segment_mut(&mut prj, unit).unwrap().devices.push(PlannedDevice { macro_name: "Motor DOL".into(), part: "GEN.M".into(), ..device("-M3", 7.5) });
    let rows = sheet(&prj);
    let mut other = demo_project();
    assert_eq!(import_rows(&mut other, &rows).unwrap(), (2, 1, 0));
    assert_eq!(import_rows(&mut other, &rows).unwrap(), (0, 0, 0), "nothing changes the second time");
    assert_eq!(csv(&other), csv(&prj));
    let csv_rows = crate::xlsx::rows_of_csv(&csv(&prj));
    let mut third = demo_project();
    assert_eq!(import_rows(&mut third, &csv_rows).unwrap().1, 1);
    assert!(import_rows(&mut third, &[vec!["Tag".into()]]).is_err());
}

#[test]
fn pages_are_generated_from_page_macros() {
    let mut prj = demo_project();
    let mut m = from_page(&prj.pages[1], "Motor DOL");
    m.kind = MacroKind::Page;
    let lib = MacroLibrary { macros: vec![m] };
    let seg = add_segment(&mut prj, 0, "Plant");
    {
        let s = segment_mut(&mut prj, seg).unwrap();
        s.function = "A9".into();
        s.devices.push(PlannedDevice { macro_name: "Motor DOL".into(), ..device("-M50", 3.0) });
        s.devices.push(PlannedDevice { macro_name: "Missing".into(), ..device("-M51", 3.0) });
        s.devices.push(device("-M52", 1.0));
    }
    let pages = prj.pages.len();
    let (new, missing) = generate(&mut prj, seg, &lib);
    assert_eq!(new.len(), 1);
    assert_eq!(missing, vec!["Missing".to_string()]);
    assert_eq!(prj.pages.len(), pages + 1);
    let pg = prj.pages.iter().find(|p| p.name == new[0]).unwrap();
    assert_eq!(pg.function, "A9");
}
