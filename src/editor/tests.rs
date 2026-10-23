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

const SCREEN: Rect = Rect { min: pos2(0.0, 0.0), max: pos2(1200.0, 800.0) };

/// Clicks at the page point `q`.
fn click(ed: &mut Editor, ctx: &egui::Context, prj: &mut Project, page: usize, q: P) {
    let at = ed.w2s(SCREEN, q);
    frame(ed, ctx, prj, page, vec![Event::PointerMoved(at)]);
    frame(ed, ctx, prj, page, vec![button(at, true)]);
    frame(ed, ctx, prj, page, vec![button(at, false)]);
    // Moving away avoids double clicks with the next click.
    frame(ed, ctx, prj, page, vec![Event::PointerMoved(at + vec2(300.0, 0.0))]);
}

fn ready(ed: &mut Editor, ctx: &egui::Context, prj: &mut Project) {
    frame(ed, ctx, prj, 0, vec![]);
    frame(ed, ctx, prj, 0, vec![]);
}

#[test]
fn dimension_tool_takes_three_clicks() {
    let mut prj = Project::default();
    let ctx = egui::Context::default();
    let mut ed = Editor::default();
    ready(&mut ed, &ctx, &mut prj);
    ed.set_tool(Tool::Dimension(DimKind::Horizontal));
    click(&mut ed, &ctx, &mut prj, 0, p(40.0, 40.0));
    click(&mut ed, &ctx, &mut prj, 0, p(80.0, 48.0));
    assert!(prj.pages[0].dimensions.is_empty());
    click(&mut ed, &ctx, &mut prj, 0, p(60.0, 28.0));
    let d = &prj.pages[0].dimensions[0];
    assert_eq!((d.a, d.b), (p(40.0, 40.0), p(80.0, 48.0)));
    assert!((d.offset - 12.0).abs() < 1e-4);
    assert_eq!(crate::graphics::dim_text(d), "40");
}

#[test]
fn cable_definition_tool_makes_cores() {
    let mut prj = Project::default();
    for (i, x) in [40.0, 48.0].into_iter().enumerate() {
        prj.pages[0].wires.push(Wire { id: 100 + i as u64, a: p(x, 40.0), b: p(x, 100.0), ..Default::default() });
    }
    prj.fix_next_id();
    let ctx = egui::Context::default();
    let mut ed = Editor::default();
    ready(&mut ed, &ctx, &mut prj);
    ed.set_tool(Tool::CableDef);
    click(&mut ed, &ctx, &mut prj, 0, p(36.0, 60.0));
    click(&mut ed, &ctx, &mut prj, 0, p(52.0, 60.0));
    assert_eq!(prj.cables.len(), 1);
    assert_eq!(prj.pages[0].cable_defs.len(), 1);
    assert!(prj.pages[0].wires.iter().all(|w| w.cable == prj.cables[0].tag && !w.core.is_empty()));
    assert_eq!(ed.tool, Tool::Select);
    assert_eq!(ed.cable_target, prj.cables[0].tag);
}

#[test]
fn polygon_ends_with_enter() {
    let mut prj = Project::default();
    let ctx = egui::Context::default();
    let mut ed = Editor::default();
    ready(&mut ed, &ctx, &mut prj);
    ed.set_tool(Tool::Polygon);
    for q in [p(40.0, 40.0), p(80.0, 40.0), p(60.0, 80.0)] {
        click(&mut ed, &ctx, &mut prj, 0, q);
    }
    let enter = Event::Key { key: Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::default() };
    frame(&mut ed, &ctx, &mut prj, 0, vec![enter]);
    let s = &prj.pages[0].shapes[0];
    assert_eq!((s.kind, s.pts.len()), (ShapeKind::Polygon, 3));
}

#[test]
fn tag_block_is_dragged_into_place() {
    let mut prj = Project::default();
    prj.pages[0].symbols.push(SymbolInst { id: 7, sym: "coil".into(), pos: p(100.0, 100.0), tag: "-K1".into(), pins: vec!["A1".into(), "A2".into()], ..Default::default() });
    prj.fix_next_id();
    let ctx = egui::Context::default();
    let mut ed = Editor::default();
    ready(&mut ed, &ctx, &mut prj);
    ed.selection.insert(7);
    let def = symbols::get("coil").unwrap();
    let block = render::tag_block_bounds(&prj, 0, def, &prj.pages[0].symbols[0]).unwrap();
    let from = ed.w2s(SCREEN, block.center());
    let to = ed.w2s(SCREEN, block.center() + p(-20.0, 10.0));
    frame(&mut ed, &ctx, &mut prj, 0, vec![Event::PointerMoved(from)]);
    frame(&mut ed, &ctx, &mut prj, 0, vec![button(from, true)]);
    for i in 1..=5 {
        let t = i as f32 / 5.0;
        frame(&mut ed, &ctx, &mut prj, 0, vec![Event::PointerMoved(from + (to - from) * t)]);
    }
    frame(&mut ed, &ctx, &mut prj, 0, vec![button(to, false)]);
    let s = &prj.pages[0].symbols[0];
    assert_eq!(s.pos, p(100.0, 100.0), "the symbol stays");
    let o = s.tag_offset.expect("tag placed");
    let expect = block.min - s.pos + p(-20.0, 10.0);
    assert!(o.dist(expect) < 0.6, "{o:?} {expect:?}");
}
