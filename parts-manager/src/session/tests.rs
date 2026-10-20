use super::*;
use openecraft_parts::api::{ImportResult, PartList, ServerInfo};

fn part(number: &str) -> Part {
    Part { number: number.into(), ..Default::default() }
}

fn loaded(numbers: &[&str]) -> Session {
    let mut s = Session::default();
    let list = PartList { revision: 4, parts: numbers.iter().map(|n| part(n)).collect(), ..Default::default() };
    assert_eq!(s.apply(&Reply::Parts(Ok(list))), None);
    s
}

#[test]
fn connecting_loads_the_parts_once_per_revision() {
    let mut s = Session::default();
    let info = |revision| Reply::Info(Ok(ServerInfo { revision, ..Default::default() }));
    assert_eq!(s.apply(&info(4)), Some(FollowUp::Reload));
    let list = PartList { revision: 4, parts: vec![part("B10"), part("B9")], ..Default::default() };
    s.apply(&Reply::Parts(Ok(list)));
    assert!(s.loaded);
    assert_eq!(s.parts[0].number, "B9", "natural sort order");
    assert_eq!(s.apply(&info(4)), None);
    assert_eq!(s.apply(&info(5)), Some(FollowUp::Reload));
}

#[test]
fn editing_marks_the_draft_dirty_until_reverted() {
    let mut s = loaded(&["A", "B"]);
    assert!(!s.dirty());
    s.select("A");
    assert_eq!(s.selected(), Some("A"));
    assert!(!s.dirty());
    s.draft.as_mut().unwrap().part.description = "changed".into();
    assert!(s.dirty());
    s.revert();
    assert!(!s.dirty());
    assert_eq!(s.draft.as_ref().unwrap().part.description, "");
}

#[test]
fn new_parts_get_unique_numbers_and_are_dirty() {
    let mut s = loaded(&["NEW.PART"]);
    s.new_part();
    let d = s.draft.as_ref().unwrap();
    assert_eq!((d.original.clone(), d.part.number.as_str()), (None, "NEW.PART_1"));
    assert!(s.dirty());
    assert_eq!(s.save_problem(), None);
    s.copy_part();
    assert_eq!(s.draft.as_ref().unwrap().part.number, "NEW.PART_1_COPY");
    s.revert();
    assert!(s.draft.is_none());
}

#[test]
fn existing_or_empty_numbers_cannot_be_saved() {
    let mut s = loaded(&["A", "B"]);
    s.select("A");
    s.draft.as_mut().unwrap().part.number = "B".into();
    assert!(s.number_taken());
    assert!(s.save_problem().is_some());
    s.draft.as_mut().unwrap().part.number = " ".into();
    assert!(s.save_problem().is_some());
    s.draft.as_mut().unwrap().part.number = "A".into();
    assert_eq!(s.save_problem(), None);
}

#[test]
fn saving_a_renamed_part_deletes_the_old_number() {
    let mut s = loaded(&["A"]);
    s.select("A");
    s.draft.as_mut().unwrap().part.number = "A2".into();
    let follow = s.apply(&Reply::Saved(Ok(Box::new(part("A2")))));
    assert_eq!(follow, Some(FollowUp::Delete("A".into())));
    assert_eq!(s.draft.as_ref().unwrap().original.as_deref(), Some("A2"));
    // Deleting the old number keeps the renamed part in the editor.
    s.apply(&Reply::Deleted("A".into(), Ok(())));
    assert_eq!(s.parts.iter().map(|p| p.number.as_str()).collect::<Vec<_>>(), ["A2"]);
    assert_eq!(s.selected(), Some("A2"));
    assert!(!s.dirty());
}

#[test]
fn saving_a_new_part_adds_it() {
    let mut s = loaded(&["A"]);
    s.new_part();
    let number = s.draft.as_ref().unwrap().part.number.clone();
    assert_eq!(s.apply(&Reply::Saved(Ok(Box::new(part(&number))))), None);
    assert!(s.get(&number).is_some());
    assert!(!s.dirty());
}

#[test]
fn deleting_the_selected_part_closes_the_editor() {
    let mut s = loaded(&["A", "B"]);
    s.select("B");
    s.apply(&Reply::Deleted("B".into(), Ok(())));
    assert!(s.draft.is_none());
    assert_eq!(s.parts.len(), 1);
    // Failed requests change nothing.
    s.select("A");
    s.apply(&Reply::Deleted("A".into(), Err("missing or wrong access token".into())));
    assert_eq!(s.selected(), Some("A"));
}

#[test]
fn reloading_keeps_unsaved_changes() {
    let mut s = loaded(&["A"]);
    s.select("A");
    s.draft.as_mut().unwrap().part.description = "mine".into();
    let mut theirs = part("A");
    theirs.description = "theirs".into();
    s.apply(&Reply::Parts(Ok(PartList { revision: 5, parts: vec![theirs.clone()], ..Default::default() })));
    assert_eq!(s.draft.as_ref().unwrap().part.description, "mine");
    // Without changes the editor shows the new server state.
    s.revert();
    assert_eq!(s.draft.as_ref().unwrap().part.description, "theirs");
    assert_eq!(s.apply(&Reply::Imported(Ok(ImportResult { imported: 3, revision: 6 }))), Some(FollowUp::Reload));
}
