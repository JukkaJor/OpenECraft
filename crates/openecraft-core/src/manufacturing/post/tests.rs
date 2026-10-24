use super::*;
use crate::manufacturing::machines::builtin;
use crate::manufacturing::strategy::{plan, Strategy};
use crate::panel::Face;
use crate::sample::demo_project;

fn program_of(machine: &str, face: Face) -> (Plan, String) {
    let prj = demo_project();
    let m = builtin().into_iter().find(|m| m.id == machine).unwrap();
    let p = plan(&prj, 0, face, &m, &Strategy::default());
    let text = program(&p, &m);
    (p, text)
}

#[test]
fn iso_program() {
    let (p, text) = program_of("drill_mill_iso", Face::Plate);
    assert!(text.starts_with("%\nO0001 (OPENECRAFT +CAB1 PLATE)"), "{text}");
    assert!(text.contains("G81 X") && text.contains("G84 X") && text.contains("T4 M6 (D4.2)"), "{text}");
    assert!(text.trim_end().ends_with("M30\n%"));
    // Coordinates from the bottom left: the first hole.
    let first = &p.ops[0];
    assert!(text.contains(&format!("X{} Y{}", num(first.pos.x), num(p.height - first.pos.y))));
    let (_, door) = program_of("drill_mill_iso", Face::Door);
    assert!(door.contains("G2 X") || door.contains("G1 X"), "{door}");
    assert_eq!(text, program_of("drill_mill_iso", Face::Plate).1, "programs are deterministic");
}

#[test]
fn heidenhain_punch_and_csv() {
    let (_, h) = program_of("drill_mill_heidenhain", Face::Plate);
    assert!(h.starts_with("0 BEGIN PGM CAB1_PLATE MM\n"), "{h}");
    assert!(h.contains("CYCL DEF 200 DRILLING") && h.contains("CYCL DEF 207 RIGID TAPPING Q200=+2 Q201=-4 Q239=+0.8"), "{h}");
    assert!(h.lines().last().unwrap().ends_with("END PGM CAB1_PLATE MM"));
    let (p, punch) = program_of("punch_turret", Face::Door);
    assert!(punch.contains("G92 X"));
    let hits: usize = p
        .ops
        .iter()
        .map(|o| match &o.kind {
            OpKind::Nibble { hits } => hits.len(),
            OpKind::Punch { .. } | OpKind::Tap { .. } | OpKind::Drill { .. } => 1,
            _ => 0,
        })
        .sum();
    assert_eq!(punch.lines().filter(|l| l.starts_with('X')).count(), hits);
    let (p, csv) = program_of("busbar_cnc", Face::Door);
    assert_eq!(csv.lines().count(), p.ops.len() + 1);
    assert!(csv.starts_with("Enclosure;Face;Machine;Seq;"));
    assert_eq!(thread_pitch("M5"), 0.8);
    let m = builtin().remove(0);
    assert_eq!(file_name(&p, &m), "CAB1_door.nc");
}
