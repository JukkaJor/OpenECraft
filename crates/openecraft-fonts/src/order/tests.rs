use super::*;

fn names(order: &[Fallback<&'static str>]) -> Vec<&'static str> {
    order
        .iter()
        .map(|f| match f {
            Fallback::Bundled(b) => b.lang,
            Fallback::System(s) => s,
        })
        .collect()
}

#[test]
fn the_font_of_the_language_comes_before_the_system_fonts() {
    assert_eq!(names(&fallback_order("ja", ["sys"])), ["ja", "sys", "zh", "ko"]);
    assert_eq!(names(&fallback_order("ko", ["a", "b"])), ["ko", "a", "b", "zh", "ja"]);
    assert_eq!(names(&fallback_order("de", ["sys"])), ["sys", "zh", "ja", "ko"]);
    assert_eq!(names(&fallback_order::<&str>("zh", [])), ["zh", "ja", "ko"]);
}

#[test]
fn the_fonts_are_truetype() {
    for f in &CJK {
        assert_eq!(&f.data[..4], &[0, 1, 0, 0], "{} is no TrueType font", f.name);
    }
}
