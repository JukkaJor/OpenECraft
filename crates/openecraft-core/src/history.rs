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

    /// Applies `f` to the project and all undo / redo steps (changes of other users
    /// of a shared project, which undo must not take back).
    pub fn rebase(&mut self, prj: &mut Project, f: impl Fn(&Project) -> Project) {
        for p in self.undo.iter_mut().chain(self.redo.iter_mut()) {
            *p = f(p);
        }
        self.committed = f(&self.committed);
        *prj = f(prj);
        self.rev += 1;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Steps that can be undone (the undo list of the quick access row).
    pub fn undo_steps(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_steps(&self) -> usize {
        self.redo.len()
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
mod tests;
