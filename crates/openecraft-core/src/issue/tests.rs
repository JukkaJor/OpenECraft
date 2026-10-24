use super::*;
use openecraft_i18n::{with_lang, Lang};

#[test]
fn messages_are_rendered_in_the_current_language() {
    let i = crate::issue!(Severity::Warning, Some((2, 7)), "Cable {} defined {} times", "-W1", 2);
    assert_eq!(i.message(), "Cable -W1 defined 2 times");
    assert_eq!((i.key(), i.args()), (Some("Cable {} defined {} times"), &["-W1".to_string(), "2".to_string()][..]));
    assert_eq!(i.target, Some((2, 7)));
    let german = with_lang(Lang::De, || i.message());
    assert_ne!(german, i.message());
    assert!(german.contains("-W1") && german.contains('2'), "{german}");
    let plain = Issue::plain(Severity::Info, "as is", None);
    assert_eq!((plain.message().as_str(), plain.key(), plain.args().len()), ("as is", None, 0));
}

#[test]
fn codes_and_fingerprints() {
    let mut a = crate::issue!(Severity::Error, Some((0, 7)), "Cable {} defined {} times", "-W1", 2);
    a.check = "wires".into();
    let mut b = crate::issue!(Severity::Error, Some((3, 9)), "Cable {} defined {} times", "-W2", 3);
    b.check = "wires".into();
    assert_eq!(a.code(), b.code(), "one code per kind of message");
    assert!(a.code().starts_with("wires.") && a.code().len() == 12);
    assert_ne!(a.fingerprint(), b.fingerprint(), "one fingerprint per occurrence");
    let moved = Issue { target: Some((5, 7)), ..a.clone() };
    assert_eq!(a.fingerprint(), moved.fingerprint(), "the page index is not part of it");
    let plain = Issue::plain(Severity::Info, "free text", None);
    assert_eq!(plain.code(), "project");
}
