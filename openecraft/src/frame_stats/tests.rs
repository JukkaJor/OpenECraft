use super::*;

#[test]
fn frame_time_and_rate_are_averaged_twice_a_second() {
    let mut s = FrameStats::default();
    assert_eq!(s.text(), None);
    // 60 frames per second taking 4 ms each, then 10 frames per second taking 20 ms.
    for k in 0..=30 {
        s.record(f64::from(k) / 60.0, Some(0.004));
    }
    assert_eq!(s.text().as_deref(), Some("4.0 ms · 60 FPS"));
    for k in 1..=5 {
        s.record(0.5 + f64::from(k) / 10.0, Some(0.02));
    }
    assert_eq!(s.text().as_deref(), Some("20.0 ms · 10 FPS"));
}

#[test]
fn the_shown_values_wait_for_the_interval() {
    let mut s = FrameStats::default();
    for k in 0..10 {
        s.record(f64::from(k) * 0.01, Some(0.001));
    }
    assert_eq!(s.text(), None, "less than half a second");
    // An idle window: one frame after a pause.
    s.record(3.0, None);
    assert_eq!(s.text().as_deref(), Some("0.9 ms · 3 FPS"), "10 frames in 3 s");
}
