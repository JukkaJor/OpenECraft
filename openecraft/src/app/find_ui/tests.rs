use super::*;

#[test]
fn hits_without_a_page_lead_to_their_pane() {
    assert_eq!(pane_of(Kind::Part), Some(Pane::Tool(ToolPane::Parts)));
    assert_eq!(pane_of(Kind::Cable), Some(Pane::Tool(ToolPane::Cables)));
    assert_eq!(pane_of(Kind::Device), Some(Pane::Navigator(LeftTab::Devices)));
    // Hits of these kinds are always on a page.
    assert_eq!(pane_of(Kind::Text), None);
}
