//! Undo / redo by snapshot comparison: every committed change of the project
//! (from the canvas, the property panel, dialogs, ...) becomes one undo step.

use crate::model::Project;

const MAX_STEPS: usize = 200;

pub struct History {
    undo: Vec<Project>,
    redo: Vec<Project>,
    committed: Project,
    /// Incremented on every change; used to invalidate caches.
    pub rev: u64,
}

impl History {
    pub fn new(prj: &Project) -> History {
        History { undo: Vec::new(), redo: Vec::new(), committed: prj.clone(), rev: 1 }
    }

    /// Call once per frame. `can_commit` is false while the user is still dragging
    /// or typing, so that a whole gesture becomes one undo step.
    pub fn track(&mut self, prj: &Project, can_commit: bool) {
        if can_commit && *prj != self.committed {
            let prev = std::mem::replace(&mut self.committed, prj.clone());
            self.undo.push(prev);
            if self.undo.len() > MAX_STEPS {
                self.undo.remove(0);
            }
            self.redo.clear();
            self.rev += 1;
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self, prj: &mut Project) {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.committed, prev));
            *prj = self.committed.clone();
            self.rev += 1;
        }
    }

    pub fn redo(&mut self, prj: &mut Project) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.committed, next));
            *prj = self.committed.clone();
            self.rev += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_redo_roundtrip() {
        let mut prj = Project::default();
        let mut h = History::new(&prj);
        prj.info.name = "A".into();
        h.track(&prj, false);
        assert!(!h.can_undo());
        h.track(&prj, true);
        prj.info.name = "B".into();
        h.track(&prj, true);
        h.undo(&mut prj);
        assert_eq!(prj.info.name, "A");
        h.redo(&mut prj);
        assert_eq!(prj.info.name, "B");
    }
}
