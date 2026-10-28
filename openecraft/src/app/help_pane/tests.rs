use super::*;

#[test]
fn tutorial_has_thirteen_numbered_chapters() {
    let chapters = chapters();
    assert_eq!(chapters.len(), 14, "introduction and 13 chapters");
    for (i, (title, _)) in chapters.iter().enumerate().skip(1) {
        assert!(title.starts_with(&format!("{i}. ")), "chapter {i}: {title}");
    }
}

#[test]
fn f1_opens_the_chapter_of_the_pane_in_use() {
    let chapters = chapters();
    let chapter = |active| key_help(active, Pane::Page).map(|c| chapters[c].0.clone()).unwrap_or_default();
    assert!(chapter(Some(Pane::Tool(ToolPane::Reports))).starts_with("9."));
    assert!(chapter(Some(Pane::Navigator(LeftTab::Check))).starts_with("8."));
    assert!(chapter(Some(Pane::LayoutView)).starts_with("10."));
    // Nothing clicked yet: the document worked in.
    assert!(chapter(None).starts_with("3."));
    // In the Help pane F1 keeps the chapter shown.
    assert_eq!(key_help(Some(Pane::Tool(ToolPane::Help)), Pane::Page), None);
}
