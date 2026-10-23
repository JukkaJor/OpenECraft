use super::*;
use crate::reports::Analysis;
use crate::sample::demo_project;
use egui::{Event, Modifiers, RawInput};

/// Runs one editor frame on page `page` with the given input events.
fn frame(ed: &mut Editor, ctx: &egui::Context, prj: &mut Project, page: usize, events: Vec<Event>) -> CanvasOutput {
    let idx = Index::build(prj);
    let an = Analysis::build(prj);
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1200.0, 800.0))),
        events,
        ..Default::default()
    };
    let mut canvas = CanvasOutput::default();
    let mut out = ctx.run_ui(input, |ui| {
        canvas = ed.ui(ui, prj, page, &idx, &an);
    });
    out.textures_delta.clear();
    canvas
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::default() }
}

#[test]
fn macro_insertion_waits_for_the_click() {
    let mut prj = demo_project();
    let page = 1;
    let sel: BTreeSet<u64> = prj.pages[page].symbols.iter().filter(|s| s.tag == "-K1").map(|s| s.id).collect();
    let clip = ops::copy(&prj.pages[page], &sel);
    let before = prj.pages[page].symbols.len();
    let ctx = egui::Context::default();
    let mut ed = Editor { insert: Some(clip), ..Default::default() };
    ed.set_tool(Tool::Insert);
    // Frames without a click keep the macro (regression: it was taken on the first frame).
    let at = pos2(600.0, 400.0);
    frame(&mut ed, &ctx, &mut prj, page, vec![]);
    frame(&mut ed, &ctx, &mut prj, page, vec![Event::PointerMoved(at)]);
    assert!(ed.insert.is_some());
    assert_eq!(prj.pages[page].symbols.len(), before);
    // A click inserts it once and returns to the select tool.
    frame(&mut ed, &ctx, &mut prj, page, vec![button(at, true)]);
    frame(&mut ed, &ctx, &mut prj, page, vec![button(at, false)]);
    frame(&mut ed, &ctx, &mut prj, page, vec![]);
    assert!(ed.insert.is_none());
    assert_eq!(ed.tool, Tool::Select);
    assert_eq!(prj.pages[page].symbols.len(), before + sel.len());
    assert_eq!(ed.selection.len(), sel.len());
}

#[test]
fn released_pages_are_not_edited() {
    let mut prj = demo_project();
    let page = 1;
    prj.pages[page].locked = true;
    let ctx = egui::Context::default();
    let mut ed = Editor::default();
    ed.set_tool(Tool::Place("coil"));
    let before = prj.pages[page].symbols.len();
    let at = pos2(600.0, 400.0);
    frame(&mut ed, &ctx, &mut prj, page, vec![Event::PointerMoved(at)]);
    frame(&mut ed, &ctx, &mut prj, page, vec![button(at, true)]);
    frame(&mut ed, &ctx, &mut prj, page, vec![button(at, false)]);
    assert_eq!(prj.pages[page].symbols.len(), before);
    // Unlocked, the same click places the coil.
    prj.pages[page].locked = false;
    frame(&mut ed, &ctx, &mut prj, page, vec![button(at, true)]);
    frame(&mut ed, &ctx, &mut prj, page, vec![button(at, false)]);
    assert_eq!(prj.pages[page].symbols.len(), before + 1);
}

#[test]
fn pages_locked_by_other_users_are_not_edited() {
    let mut prj = demo_project();
    let page = 1;
    let ctx = egui::Context::default();
    let mut ed = Editor { read_only: true, ..Editor::default() };
    ed.set_tool(Tool::Place("coil"));
    let before = prj.pages[page].symbols.len();
    let at = pos2(600.0, 400.0);
    frame(&mut ed, &ctx, &mut prj, page, vec![Event::PointerMoved(at)]);
    frame(&mut ed, &ctx, &mut prj, page, vec![button(at, true)]);
    frame(&mut ed, &ctx, &mut prj, page, vec![button(at, false)]);
    assert_eq!(prj.pages[page].symbols.len(), before);
    ed.read_only = false;
    frame(&mut ed, &ctx, &mut prj, page, vec![button(at, true)]);
    frame(&mut ed, &ctx, &mut prj, page, vec![button(at, false)]);
    assert_eq!(prj.pages[page].symbols.len(), before + 1);
}

#[test]
fn review_tools_work_on_released_pages() {
    let mut prj = demo_project();
    let page = 1;
    prj.pages[page].locked = true;
    let ctx = egui::Context::default();
    let mut ed = Editor { read_only: true, ..Editor::default() };
    ed.set_tool(Tool::Comment);
    let at = pos2(600.0, 400.0);
    frame(&mut ed, &ctx, &mut prj, page, vec![Event::PointerMoved(at)]);
    frame(&mut ed, &ctx, &mut prj, page, vec![button(at, true)]);
    let out = frame(&mut ed, &ctx, &mut prj, page, vec![button(at, false)]);
    assert!(matches!(out.review, Some(ReviewInput::Comment(_))));

    // Markup rectangles take two clicks.
    ed.set_tool(Tool::Markup(MarkupKind::Rect, Pen::Green));
    let b = pos2(700.0, 450.0);
    frame(&mut ed, &ctx, &mut prj, page, vec![button(at, true)]);
    let out = frame(&mut ed, &ctx, &mut prj, page, vec![button(at, false)]);
    assert!(out.review.is_none());
    frame(&mut ed, &ctx, &mut prj, page, vec![Event::PointerMoved(b), button(b, true)]);
    let out = frame(&mut ed, &ctx, &mut prj, page, vec![button(b, false)]);
    match out.review {
        Some(ReviewInput::Markup(MarkupKind::Rect, Pen::Green, a, b)) => assert!(b.x > a.x && b.y > a.y),
        other => panic!("{other:?}"),
    }
}
