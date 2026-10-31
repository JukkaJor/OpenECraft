use super::*;

fn labels(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn layouts_are_parsed_and_written_back() {
    for s in ["circle 4", "circle 7+1", "row 8", "zigzag 10", "grid 2x8+PE", "dsub 9"] {
        let pl = PinLayout::parse(s).unwrap_or_else(|| panic!("{s}"));
        assert_eq!(pl.text(), s);
    }
    assert_eq!(PinLayout::parse(" Grid 2X5 + pe").map(|p| p.slots()), Some(11));
    assert_eq!(PinLayout::parse("circle 4+1").map(|p| p.slots()), Some(5));
    for bad in ["", "circle", "circle x", "hexagon 6", "grid 2", "row 0"] {
        assert_eq!(PinLayout::parse(bad), None, "{bad}");
    }
}

#[test]
fn the_layout_and_pitch_come_from_the_technical_data() {
    let part = Part { tech_data: "Pin layout: zigzag 10\nPitch: 2,54 mm".into(), ..Default::default() };
    assert_eq!(PinLayout::of_part(&part).map(|p| p.layout), Some(Layout::Zigzag(10)));
    assert_eq!(pitch_of(&part), Some(2.54));
    assert_eq!(PinLayout::of_part(&Part::default()), None);
}

#[test]
fn circular_faces_start_right_of_the_key_and_run_counter_clockwise() {
    let f = face(PinLayout::parse("circle 4+1").unwrap(), 0.0);
    assert_eq!(f.slots.len(), 5);
    let [x1, y1] = f.slots[0];
    let [x2, y2] = f.slots[1];
    // Male face: pin 1 top right, pin 2 top left (y down), centre pin last.
    assert!(x1 > 0.0 && y1 < 0.0 && x2 < 0.0 && y2 < 0.0);
    let [x3, y3] = f.slots[2];
    assert!(x3 < 0.0 && y3 > 0.0);
    assert_eq!(f.slots[4], [0.0, 0.0]);
    let Outline::Circle { r } = f.outline else { panic!("circle") };
    assert!(f.slots.iter().all(|[x, y]| (x * x + y * y).sqrt() + f.pin_r < r));
    assert_eq!(f.key, Some([0.0, -r]));
}

#[test]
fn zigzag_faces_put_odd_pins_above_even_pins() {
    let f = face(PinLayout::parse("zigzag 10").unwrap(), 2.54);
    assert_eq!(f.slots[0][0], f.slots[1][0]);
    assert!(f.slots[0][1] < f.slots[1][1]);
    assert!((f.slots[2][0] - f.slots[0][0] - 2.54).abs() < 1e-4);
    assert!(f.key.is_some());
}

#[test]
fn dsub_faces_have_the_longer_row_on_top() {
    let f = face(PinLayout::parse("dsub 9").unwrap(), 0.0);
    let top = f.slots.iter().filter(|s| s[1] < 0.0).count();
    assert_eq!((top, f.slots.len() - top), (5, 4));
    let Outline::DSub { top, bottom, .. } = f.outline else { panic!("dsub") };
    assert!(top > bottom);
}

#[test]
fn the_earth_contact_sits_beside_the_pins() {
    let f = face(PinLayout::parse("grid 2x8+PE").unwrap(), 0.0);
    assert_eq!(f.slots.len(), 17);
    let pe = f.slots[16];
    assert!(f.slots[..16].iter().all(|s| s[0] < pe[0]));
    let (w, _) = f.outline.size();
    assert!(pe[0] + f.pin_r < w / 2.0);
}

#[test]
fn designations_are_placed_by_order_or_by_number() {
    assert_eq!(assign(&labels(&["1", "3", "4"]), 4), [Some(0), Some(2), Some(3)]);
    assert_eq!(assign(&labels(&["A", "B", "C"]), 4), [Some(0), Some(1), Some(2)]);
    assert_eq!(assign(&labels(&["2", "1"]), 2), [Some(0), Some(1)]);
    assert_eq!(assign(&labels(&["1", "2", "3"]), 2), [Some(0), Some(1), None]);
}
