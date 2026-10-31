use super::*;
use crate::index::Index;
use crate::analysis::Analysis;
use crate::issue::Severity;

#[test]
fn demo_is_consistent() {
    let prj = demo_project();
    let an = Analysis::checked(&prj);
    let errors: Vec<_> = an.issues.iter().filter(|i| i.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let idx = Index::build(&prj);
    assert_eq!(idx.devices["-K1"].mains.len(), 1);
    assert_eq!(idx.devices["-K1"].auxs.len(), 4);
    // Potentials propagate through the interruption points.
    let lamp = prj.pages[2].symbols.iter().find(|s| s.tag == "-P1").unwrap();
    let net = an.netlist.pin_net[&(lamp.id, 1)];
    assert_eq!(an.netlist.nets[net].name, "0V");
    let warnings: Vec<_> = an.issues.iter().filter(|i| i.severity == Severity::Warning).map(|i| i.message()).collect();
    assert!(warnings.is_empty(), "{warnings:#?}");
    // Unused contacts of contactor + auxiliary block are reserves (2 NC of the block).
    let reserve = crate::parts::reserve_functions(&prj, None, "-K1");
    let pins: Vec<_> = reserve.iter().map(|f| f.pins.as_str()).collect();
    assert_eq!(pins, vec!["61 62", "71 72"]);
    // BOM: -X1 has three feed-through terminals and one PE terminal.
    let bom = &an.report(&prj, crate::model::ReportKind::BillOfMaterials).unwrap();
    let qty = |part: &str| bom.rows.iter().find(|r| r[3] == part).map(|r| r[1].clone()).unwrap_or_default();
    assert_eq!((qty("GEN.TB-2.5").as_str(), qty("GEN.TB-PE-2.5").as_str()), ("3", "1"));
}

fn checked(key: &str) -> (Project, Analysis) {
    let prj = example(key).unwrap_or_else(|| panic!("example {key}"));
    let an = Analysis::checked(&prj);
    (prj, an)
}

#[test]
fn examples_are_consistent() {
    let mut keys: Vec<&str> = EXAMPLES.iter().map(|e| e.key).collect();
    keys.dedup();
    assert_eq!(keys.len(), EXAMPLES.len());
    assert!(EXAMPLES.windows(2).all(|w| w[0].level < w[1].level), "levels rise");
    for e in &EXAMPLES {
        let (prj, an) = checked(e.key);
        let problems: Vec<String> =
            an.issues.iter().filter(|i| i.severity <= Severity::Warning).map(|i| format!("{:?}: {}", i.severity, i.message())).collect();
        assert!(problems.is_empty(), "{}: {problems:#?}", e.key);
        // Every device with a function in the schematic has a part (fluid ports and
        // the solenoid coils of the valves aside).
        let idx = Index::build(&prj);
        let missing: Vec<&String> = idx
            .devices
            .keys()
            .filter(|t| !prj.devices.get(*t).is_some_and(|d| !d.parts.is_empty()))
            .filter(|t| !["-BP1", "-GQ1", "-MB1", "-MB2"].contains(&t.as_str()))
            .collect();
        assert!(missing.is_empty(), "{}: devices without part {missing:?}", e.key);
        // Saved and opened again unchanged.
        let json = serde_json::to_string(&prj).unwrap();
        let back: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(serde_json::to_string(&back).unwrap(), json, "{}", e.key);
    }
}

#[test]
fn lamp_circuit_takes_the_potential_colours() {
    let (prj, an) = checked("lamp_circuit");
    assert!(prj.wire_colors);
    let lamp = prj.pages[1].symbols.iter().find(|s| s.tag == "-P1").unwrap();
    assert_eq!(an.netlist.nets[an.netlist.pin_net[&(lamp.id, 1)]].name, "0V");
    let bom = an.report(&prj, ReportKind::BillOfMaterials).unwrap();
    for part in ["GEN.MCB-1P-C6", "GEN.PB-GN-1NO", "GEN.SEL-2POS-1NO", "GEN.LMP-GN-24", "GEN.LMP-RD-24"] {
        assert!(bom.rows.iter().any(|r| r[3] == part), "{part}");
    }
}

#[test]
fn relay_holds_itself_through_the_strip() {
    let (prj, an) = checked("self_holding_relay");
    let idx = Index::build(&prj);
    assert_eq!((idx.devices["-K1"].mains.len(), idx.devices["-K1"].auxs.len()), (1, 3));
    let strip = an.report(&prj, ReportKind::TerminalDiagram).unwrap();
    assert_eq!(strip.rows.iter().filter(|r| r[0] == "-X1" && r[1].parse::<u32>().is_ok()).count(), 4);
    // Wire numbers after page and column.
    assert!(prj.pages[1].wires.iter().any(|w| w.number.starts_with("2.")), "page / column numbers");
}

#[test]
fn star_delta_cable_uses_all_cores() {
    let (prj, an) = checked("star_delta_starter");
    let mut cores: Vec<&str> = prj.pages.iter().flat_map(|p| &p.wires).filter(|w| w.cable == "-W1").map(|w| w.core.as_str()).collect();
    cores.sort();
    assert_eq!(cores, ["1", "2", "3", "4", "5", "6", "GNYE"]);
    let idx = Index::build(&prj);
    for k in ["-K1", "-K2", "-K3", "-K4"] {
        assert_eq!(idx.devices[k].mains.len(), 1, "{k}");
    }
    assert!(an.report(&prj, ReportKind::CableOverview).unwrap().rows.iter().any(|r| r.contains(&"-W1".to_string())));
}

#[test]
fn press_links_fluid_and_electrical_devices() {
    let (prj, an) = checked("pneumatic_press");
    assert_eq!(crate::electrofluid::linked(&prj, "-QM1"), ["-MB1".to_string()]);
    assert_eq!(crate::electrofluid::linked(&prj, "-QM2"), ["-MB2".to_string()]);
    assert_eq!(prj.fluid.groups[0].members, ["-QM1", "-QM2"]);
    assert!(!an.report(&prj, ReportKind::HoseList).unwrap().rows.is_empty());
}

#[test]
fn pump_station_is_structured() {
    let (prj, an) = checked("pump_station");
    let idx = Index::build(&prj);
    for tag in ["=SUP+CAB1-T1", "=P1+CAB1-K1", "=P1+FLD-M1", "=P2+CAB1-T1", "=PLC+CAB1-A3", "=SUP+FLD-X10"] {
        assert!(idx.devices.contains_key(tag), "{tag}");
    }
    assert_eq!(prj.sections.len(), 6);
    assert!(prj.pages.iter().all(|p| p.name.contains('.')), "page names carry the section numbers");
    let plc = an.report(&prj, ReportKind::PlcAssignment).unwrap();
    assert!(plc.rows.iter().any(|r| r.contains(&"IW64".to_string())), "{plc:?}");
    // Planned in the single-line diagram and engineered in the schematic.
    let devices = crate::singleline::devices(&prj, &idx);
    assert!(devices.iter().filter(|d| d.status == crate::singleline::Status::Engineered).count() >= 5);
}

#[test]
fn workflow_marks_the_change_since_the_release() {
    use crate::revision::{pending, status, Status};
    let (prj, _) = checked("engineering_workflow");
    assert_eq!(status(&prj, 0), Status::Released);
    assert_eq!(status(&prj, 1), Status::InReview);
    let changed: Vec<String> = pending(&prj).into_iter().map(|c| c.name).collect();
    assert_eq!(changed, ["3"]);
    assert!(prj.pages.iter().filter(|p| p.name != "3").all(|p| p.locked));
    assert_eq!(prj.review.requests[0].number, "CR-001");
    assert_eq!(prj.baselines.len(), 1);
}

#[test]
fn cabinets_stand_in_a_line_up() {
    use crate::panel::{lineups, locate, ItemKind};
    let (prj, an) = checked("cabinet_lineup");
    let l = &prj.layout;
    assert_eq!(lineups(l), vec![vec![0, 1]]);
    // Every device of the cabinets is placed, the converter in +CAB2.
    let idx = Index::build(&prj);
    for tag in idx.devices.keys().filter(|t| t.contains("+CAB")) {
        assert!(locate(&prj, tag).is_some(), "{tag} not placed");
    }
    assert_eq!(locate(&prj, "=P2+CAB2-T1").map(|(e, _)| e), Some(1));
    let kinds = |e: usize, k: ItemKind| l.enclosures[e].items.iter().filter(|i| i.kind == k).count();
    assert_eq!(kinds(0, ItemKind::Busbar), 1);
    assert_eq!(kinds(0, ItemKind::Cutout) + kinds(1, ItemKind::Cutout), 2, "cable passage");
    assert!(crate::routing3d::accessories(&prj, 0, "Cable gland").len() == 1);
    assert!(!an.report(&prj, ReportKind::DuctFill).unwrap().rows.is_empty());
    assert_eq!(an.report(&prj, ReportKind::Climate).unwrap().rows.len(), 2);
    assert!(prj.pages.iter().filter(|p| matches!(p.kind, PageKind::Mounting(_))).count() == 2);
}

#[test]
fn electrical_room_holds_the_line_up() {
    use crate::model::panel::Opening;
    use crate::panel::{cabinets_in, locate, mount_ok, Face};
    let (prj, _) = checked("electrical_room");
    let l = &prj.layout;
    let r = l.enclosures.iter().position(|e| e.is_room()).expect("room");
    let room = &l.enclosures[r];
    assert_eq!((room.outline.len(), room.inner_walls.len()), (6, 1));
    let openings: Vec<Opening> = room.items.iter().map(|i| i.opening).filter(|o| !o.is_cutout()).collect();
    assert_eq!(openings, [Opening::Door, Opening::Window]);
    assert_eq!(cabinets_in(l, r), [0, 1]);
    assert!(room.items.iter().all(|i| mount_ok(&prj, room, i)));
    assert!(matches!(locate(&prj, "=SUP+ER1-X2"), Some((e, it)) if e == r && it.face == Face::Wall(5)));
    assert!(room.items.iter().any(|i| i.part == room::LIGHT && i.tag.is_empty()));
    assert!(room.items.iter().any(|i| i.part == room::TRAY && i.face == Face::Roof));
}

#[test]
fn pump_room_routes_pipes_and_motor_cables() {
    use crate::panel::{mount_ok, Face};
    use crate::routing3d::RouteKind;
    let (prj, an) = checked("pump_room");
    let l = &prj.layout;
    let r = l.enclosures.iter().position(|e| e.is_room()).expect("room");
    let room = &l.enclosures[r];
    // Vessel, pump and both motors stand on the floor; valves and gauge on the wall.
    let on = |face: Face| room.items.iter().filter(|i| i.face == face && !i.tag.is_empty()).map(|i| i.tag.as_str()).collect::<Vec<_>>();
    assert_eq!(on(Face::Bottom), ["=P1+ER1-CM1", "=P1+ER1-GP1", "=P1+FLD-M1", "=P2+FLD-M1"]);
    assert!(["=P1+ER1-QM1", "=P1+ER1-QM2", "=P1+ER1-BP1"].iter().all(|t| on(Face::Wall(2)).contains(t)));
    assert!(room.items.iter().all(|i| mount_ok(&prj, room, i)));
    assert_eq!(room.paths.iter().filter(|p| p.face.is_some()).count(), 1);
    // The pipes run through the room only, along the pipe runs (the pressure side to
    // the gauge and on to the vessel are two).
    let pipes: Vec<_> = an.routing.items.iter().filter(|i| i.kind == RouteKind::Tube).collect();
    assert_eq!(pipes.len(), 5);
    assert!(pipes.iter().all(|i| i.routed() && i.legs.is_empty() && i.enclosures == [r] && i.diameter == 25.0), "{pipes:#?}");
    assert!(an.routing.paths.iter().any(|u| u.enclosure == r && u.items >= 2), "{:?}", an.routing.paths);
    // The motor cables leave their cabinets through the roof.
    for tag in ["=P1-W1", "=P2-W1"] {
        let c = an.routing.item(tag).expect(tag);
        assert!(c.routed() && c.legs.len() == 2 && c.enclosures.contains(&r), "{tag}: {:?} {:?}", c.status, c.enclosures);
    }
    assert!(!an.issues.iter().any(|i| i.message().contains("enters through the roof")));
}

#[test]
fn building_services_route_from_room_to_room() {
    use crate::model::panel::Opening;
    use crate::routing3d::RouteKind;
    let (prj, an) = checked("building_services");
    let l = &prj.layout;
    let room = |name: &str| l.enclosures.iter().position(|e| e.is_room() && e.name == name).expect(name);
    let (hr, tr) = (room("+HR"), room("+TR"));
    // Manufacturer parts from the catalogs of the parts server.
    for part in ["HAG.LT0057", "HAG.LT056", "HAG.LT253", "HAG.ECR300C", "VIC.PMP482505010", "VIC.PIN482601000", "VIC.BPP900450100", "PYL.US5000", "DEH.952515"] {
        assert!(prj.parts.iter().any(|p| p.number == part), "{part}");
    }
    // Sections in two levels, the pages numbered after them.
    assert!(prj.sections.iter().any(|s| s.parent != 0));
    assert!(prj.pages.iter().any(|p| p.name == "2.1.1") && prj.pages.iter().any(|p| p.name == "7.2.4"), "{:?}", prj.pages.iter().map(|p| &p.name).collect::<Vec<_>>());
    // Two building entries lead into the house connection room, wall penetrations on.
    let entries = l.enclosures[hr].items.iter().filter(|i| i.opening == Opening::Entry).count();
    assert_eq!(entries, 2);
    assert!(crate::panel::wall_links(&prj).iter().filter(|w| (w.from, w.to) == (hr, tr)).count() >= 2);
    // The supply cable runs from its building entry to the house connection box.
    let w1 = an.routing.item("=GRID-W1").expect("W1 routed");
    assert!(w1.routed() && w1.enclosures == [hr], "{:?}", w1.status);
    assert!(w1.ends.iter().any(|e| e.item.is_some_and(|id| l.enclosures[hr].item(id).is_some_and(|i| i.opening == Opening::Entry))));
    // The feeders of the main distribution and the water pipe pass into the technical room.
    for tag in ["=MD-W11", "=MD-W12", "=MD-W13"] {
        let c = an.routing.item(tag).expect(tag);
        assert!(c.routed() && c.enclosures.contains(&hr) && c.enclosures.contains(&tr), "{tag}: {:?} {:?}", c.status, c.enclosures);
    }
    let pipes: Vec<_> = an.routing.items.iter().filter(|i| i.kind == RouteKind::Tube).collect();
    assert!(pipes.iter().all(|i| i.routed()), "{pipes:#?}");
    assert!(pipes.iter().any(|i| i.enclosures == [hr, tr]), "the pipe into the storage tank");
    // Compressed air, heating water and hot water in their own pipes, the compressor
    // and the heat pump fed from the pump cabinet.
    let media: std::collections::BTreeSet<_> = prj.pages.iter().flat_map(|p| &p.wires).filter_map(|w| w.medium).collect();
    use crate::model::fluid::Medium;
    assert!([Medium::Water, Medium::CompressedAir, Medium::HeatingWater, Medium::HotWater].iter().all(|m| media.contains(m)), "{media:?}");
    for tag in ["=AIR-W1", "=HEAT-W1", "=HEAT-W2", "=HEAT-W3"] {
        assert!(an.routing.item(tag).is_some_and(|c| c.routed() && c.enclosures.contains(&tr)), "{tag}");
    }
    assert!(pipes.len() > 30, "{}", pipes.len());
    // Batteries and network in the 19-inch rack.
    let rack = l.enclosures.iter().position(crate::panel::is_rack).expect("rack");
    assert_eq!(crate::panel::rack::occupied(&l.enclosures[rack]).len(), 9);
    // The BMS cable ends at the CAN port of the battery (a connection point of the
    // part, not the middle of the front); the battery cable carries + and - and parts at
    // both ends: to + and - of the battery, to the fuse F1 and the stud N1 of the
    // negative busbar of the Lynx distributor.
    let pos = crate::panel::positions(l);
    let point = |tag: &str, pin: &str| {
        let (ei, it) = crate::panel::locate(&prj, tag).expect(tag);
        let q = crate::panel::device_points(&prj, it).into_iter().find(|p| p.pin == pin).expect(pin).pos;
        crate::parts::vec3::add(crate::panel::face_point(&l.enclosures[ei], it.face, q), pos[ei])
    };
    let bms = an.routing.item("=ESS-W96").expect("=ESS-W96");
    assert!(bms.routed() && bms.ends.iter().any(|e| e.tag == "=ESS+RACK-G1" && e.point == Some(point("=ESS+RACK-G1", "CAN")) && e.cores.is_empty()));
    // So do the DC cables of the inverters.
    for (cable, tag, pins) in [
        ("=ESS-W71", "=ESS+RACK-G1", ["+", "-"]),
        ("=ESS-W71", "=ESS+TR-F1", ["F1", "N1"]),
        ("=ESS-W81", "=ESS+TR-F2", ["F1", "N1"]),
        ("=ESS-W81", "=ESS+TR-T1", ["DC+", "DC-"]),
        ("=ESS-W85", "=ESS+TR-F3", ["F1", "N1"]),
    ] {
        let c = an.routing.item(cable).expect(cable);
        assert!(c.routed(), "{cable}: {:?}", c.status);
        let end = c.ends.iter().find(|e| e.tag == tag).unwrap_or_else(|| panic!("{cable} {tag}: {:?}", c.ends));
        let cores: Vec<_> = pins.iter().map(|p| point(tag, p)).collect();
        assert_eq!(end.cores, cores, "{cable} {tag}");
        let middle = crate::parts::vec3::add(cores[0], cores[1]).map(|v| v / 2.0);
        assert!(end.point.is_some_and(|q| (0..3).all(|k| (q[k] - middle[k]).abs() < 0.01)), "{tag}: {:?}", end.point);
    }
    // The battery cables leave their battery at the front and run past the others, not
    // through them (a device is passable only where a route leaves it).
    let racked = &l.enclosures[rack];
    let batteries: Vec<_> = racked.items.iter().filter(|i| i.tag.contains("RACK-G")).map(|i| crate::panel::item_box(racked, i)).collect();
    for cable in ["=ESS-W71", "=ESS-W72", "=ESS-W73", "=ESS-W74"] {
        let c = an.routing.item(cable).expect(cable);
        let (_, points) = c.stretches().into_iter().find(|(leg, _)| *leg == Some(rack)).expect("rack leg");
        for seg in points[1..].windows(2) {
            let mid = [0, 1, 2].map(|k| (seg[0][k] + seg[1][k]) / 2.0 - pos[rack][k]);
            assert!(!batteries.iter().any(|(lo, hi)| (0..3).all(|k| mid[k] > lo[k] && mid[k] < hi[k])), "{cable}: {seg:?}");
        }
    }
    // The plate view of a cabinet shows the legs of its line-up only: the cable from
    // the main distribution to the PLC cabinet stays on the plate of each.
    let w11 = an.routing.item("=MD-W11").expect("=MD-W11");
    assert_eq!(w11.legs.len(), 4, "{:?}", w11.enclosures);
    for name in ["+MD", "+CC1"] {
        let ei = l.enclosures.iter().position(|e| e.name == name).expect(name);
        let lineup = crate::panel::lineup_of(l, ei);
        let x = (pos[lineup[0]][0], lineup.iter().map(|&k| pos[k][0] + l.enclosures[k].width).fold(0.0, f32::max));
        let points: Vec<_> = w11.stretches_in_lineup(l, ei).concat();
        assert!(!points.is_empty() && points.iter().all(|q| q[0] >= x.0 && q[0] <= x.1), "{name}: {points:?}");
    }
    // PLC cards and EtherCAT terminals, NH fuse switches of three sizes.
    let plc = an.report(&prj, ReportKind::PlcAssignment).unwrap();
    for addr in ["I0.0", "Q0.1", "IW66", "DI4", "DO2"] {
        assert!(plc.rows.iter().any(|r| r.contains(&addr.to_string())), "{addr}");
    }
    let idx = Index::build(&prj);
    let devices = crate::singleline::devices(&prj, &idx);
    assert!(devices.iter().filter(|d| d.status == crate::singleline::Status::Engineered).count() >= 12);
}

#[test]
fn terminal_strips_are_drawn_with_their_models() {
    use crate::panel::{part_by_number, strip_cells, ItemKind};
    let mut strips = 0;
    for ex in &EXAMPLES {
        let prj = example(ex.key).unwrap();
        for it in prj.layout.enclosures.iter().flat_map(|e| &e.items).filter(|i| i.kind == ItemKind::Device) {
            let cells = strip_cells(&prj, it);
            if cells.is_empty() {
                continue;
            }
            strips += 1;
            // Every terminal, end plate and end bracket has its 3D model (no plain box).
            for c in &cells {
                assert!(part_by_number(&prj, &c.part).is_some_and(|p| p.model_3d.is_some()), "{}: {} {:?}", ex.key, it.tag, c.part);
            }
            // Side by side on the rail: the strip is as deep as its deepest part.
            let deepest = cells.iter().filter_map(|c| part_by_number(&prj, &c.part).and_then(|p| p.size())).map(|s| s[2]).fold(0.0, f32::max);
            assert!((it.d - deepest).abs() < 0.01, "{}: {} {} deep, parts {}", ex.key, it.tag, it.d, deepest);
            // Strips on a rail end in end brackets.
            if part_by_number(&prj, &it.part).is_some_and(|p| p.mounting.starts_with("Mounting rail")) {
                let bracket = |c: &crate::panel::Cell| part_by_number(&prj, &c.part).is_some_and(|p| p.subgroup == "End bracket");
                assert!(bracket(&cells[0]) && bracket(&cells[cells.len() - 1]), "{}: {}", ex.key, it.tag);
            }
        }
    }
    assert!(strips > 20, "{strips}");
}
