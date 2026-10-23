use super::*;
use crate::panel::Face;
use crate::sample::demo_project;

fn item(id: u64, kind: ItemKind, x: f32, y: f32, w: f32, h: f32) -> PanelItem {
    PanelItem { id, kind, x, y, w, h, d: 20.0, ..Default::default() }
}

#[test]
fn align_kinds_place_and_capture() {
    assert_eq!(Align::Start(20.0).place(500.0, 0.0, 100.0), (20.0, 100.0));
    assert_eq!(Align::Center(10.0).place(500.0, 0.0, 100.0), (210.0, 100.0));
    assert_eq!(Align::End(20.0).place(500.0, 0.0, 100.0), (380.0, 100.0));
    assert_eq!(Align::Stretch(40.0, 60.0).place(500.0, 0.0, 100.0), (40.0, 400.0));
    assert_eq!(Align::Free.place(500.0, 7.0, 100.0), (7.0, 100.0));
    for a in [Align::Start(20.0), Align::Center(-15.0), Align::End(5.0), Align::Stretch(10.0, 30.0)] {
        let (pos, size) = a.place(800.0, 0.0, 50.0);
        assert_eq!(a.capture(800.0, pos, size), a);
        assert_eq!(Align::parse(&a.text()), Some(a));
    }
    assert_eq!(Align::parse("stretch:10"), Some(Align::Stretch(10.0, 10.0)));
    assert_eq!(Align::parse("middle:1"), None);
}

#[test]
fn anchored_items_follow_the_enclosure_size() {
    let mut e = Enclosure::default();
    let mut duct = item(1, ItemKind::Duct, 0.0, 0.0, 100.0, 40.0);
    duct.anchor = Anchor { x: Align::Stretch(0.0, 0.0), y: Align::End(0.0) };
    let mut fan = item(2, ItemKind::Accessory, 0.0, 0.0, 204.0, 204.0);
    fan.face = Face::Left;
    fan.anchor = Anchor { x: Align::Center(0.0), y: Align::End(150.0) };
    e.items = vec![duct, fan];
    apply_anchors(&mut e);
    let (pw, ph) = e.plate_size();
    let d = e.item(1).unwrap();
    assert_eq!((d.x, d.w, d.y), (0.0, pw, ph - 40.0));
    e.width = 800.0;
    e.height = 1000.0;
    e.depth = 300.0;
    apply_anchors(&mut e);
    let (pw, ph) = e.plate_size();
    let d = e.item(1).unwrap();
    assert_eq!((d.w, d.y), (pw, ph - 40.0));
    let f = e.item(2).unwrap();
    assert_eq!((f.x, f.y), ((300.0 - 204.0) / 2.0, 1000.0 - 150.0 - 204.0));
    // Moving an anchored item keeps the kind of anchor with the new distance.
    e.items[1].y -= 100.0;
    capture_anchors(&mut e, &[2]);
    assert_eq!(e.item(2).unwrap().anchor.y, Align::End(250.0));
}

#[test]
fn pattern_of_copies() {
    let mut prj = demo_project();
    prj.layout.enclosures[0].items.clear();
    let id = prj.new_id();
    prj.layout.enclosures[0].items.push(item(id, ItemKind::Cutout, 10.0, 20.0, 30.0, 30.0));
    let copies = pattern_copy(&mut prj, 0, &[id], 3, 2, 50.0, 60.0);
    assert_eq!(copies.len(), 5);
    let e = &prj.layout.enclosures[0];
    let last = e.item(*copies.last().unwrap()).unwrap();
    assert_eq!((last.x, last.y), (110.0, 80.0));
    // Devices are not copied.
    let (_, k1) = demo_project().layout.find_device("-K1").map(|(i, it)| (i, it.id)).unwrap();
    let mut prj = demo_project();
    assert!(pattern_copy(&mut prj, 0, &[k1], 2, 1, 50.0, 0.0).is_empty());
}

#[test]
fn align_distribute_and_rail() {
    let items = vec![
        item(1, ItemKind::Device, 10.0, 100.0, 40.0, 60.0),
        item(2, ItemKind::Device, 100.0, 120.0, 20.0, 40.0),
        item(3, ItemKind::Device, 300.0, 90.0, 40.0, 50.0),
    ];
    let mut e = Enclosure { items, ..Default::default() };
    align(&mut e, &[1, 2, 3], AlignTo::Top);
    assert!(e.items.iter().all(|i| i.y == 90.0));
    align(&mut e, &[1, 2, 3], AlignTo::Bottom);
    assert!(e.items.iter().all(|i| i.y + i.h == 150.0));
    distribute(&mut e, &[1, 2, 3], true);
    // Gap: (340 - 10 - 100) / 2 = 115
    assert_eq!(e.item(2).unwrap().x, 10.0 + 40.0 + 115.0);
    // Line up on a rail at y 300 (centre 317.5), 5 mm apart.
    e.items.push(PanelItem { id: 9, kind: ItemKind::Rail, x: 0.0, y: 300.0, w: 200.0, h: 35.0, d: 7.5, ..Default::default() });
    assert!(arrange_on_rail(&mut e, 9, &[1, 2, 3], 5.0));
    let xs: Vec<f32> = [1, 2, 3].iter().map(|id| e.item(*id).unwrap().x).collect();
    assert_eq!(xs, vec![5.0, 50.0, 75.0]);
    assert!(e.items.iter().filter(|i| i.kind == ItemKind::Device).all(|i| i.z == 7.5 && (i.center().y - 317.5).abs() < 0.01));
    e.items[0].w = 100.0;
    assert!(!arrange_on_rail(&mut e, 9, &[1, 2, 3], 50.0));
}
