use super::*;
use crate::sample::demo_project;

#[test]
fn steps_follow_the_mounting_order() {
    let prj = demo_project();
    let steps = steps(&prj, Some(0));
    assert!(!steps.is_empty());
    let phases: Vec<Phase> = steps.iter().map(|s| s.phase).collect();
    let mut sorted = phases.clone();
    sorted.sort();
    assert_eq!(phases, sorted, "phase by phase");
    assert!(phases.contains(&Phase::Carriers) && phases.contains(&Phase::Devices));
    let first_device = steps.iter().position(|s| s.phase == Phase::Devices).unwrap();
    let last_carrier = steps.iter().rposition(|s| s.phase == Phase::Carriers).unwrap();
    assert!(last_carrier < first_device, "rails and ducts before the devices");
    assert!(steps.iter().all(|s| s.status == Status::Open));
}

#[test]
fn marks_progress_and_changes() {
    let mut prj = demo_project();
    let it = prj.layout.enclosures[0].items.iter().find(|i| i.kind == ItemKind::Device).unwrap().clone();
    set_mark(&mut prj, &it, Some(Mark::Done), " fitted ");
    assert_eq!(status(&prj, &it), Status::Done);
    assert_eq!(mark_of(&prj, it.id).unwrap().note, "fitted");
    let p = progress(&prj, None);
    assert_eq!((p.done, p.changed), (1, 0));
    assert!(p.fraction() > 0.0 && p.fraction() < 1.0);
    // Moving the item after mounting shows it as changed.
    let moved = PanelItem { x: it.x + 20.0, ..it.clone() };
    assert_eq!(status(&prj, &moved), Status::Changed);
    set_mark(&mut prj, &it, Some(Mark::Problem), "screw missing");
    assert_eq!(status(&prj, &it), Status::Problem);
    set_mark(&mut prj, &it, None, "");
    assert_eq!(status(&prj, &it), Status::Open);

    let steps = steps(&prj, None);
    assert_eq!(next_open(&steps, None), Some(0));
    let csv = csv(&prj, Some(0));
    assert!(csv.starts_with("Enclosure;Phase;Item;Part;Face;X;Y;Width;Height;State;Note\n"));
    assert_eq!(csv.lines().count(), steps.iter().filter(|s| s.enclosure == 0).count() + 1);

    // Marks of deleted items are stale.
    prj.mounting_marks.push(MountingMark { item: 999_999, ..Default::default() });
    assert_eq!(stale(&prj), [999_999]);
    assert_eq!(remove_stale(&mut prj), 1);
    assert!(item_title(&it).contains(&it.tag));
}
