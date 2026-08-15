use std::cell::RefCell;
use std::rc::Rc;

use super::CharPos0;
use super::buffer::BufferId;
use crate::emacs_core::value::Value;

/// The point GNU saves for a possible undo point entry, together with the
/// buffer it was saved in.
///
/// GNU keeps these as the pair of globals `point_before_last_command_or_undo`
/// and `buffer_before_last_command_or_undo` (src/keyboard.c:232-233).  Both
/// assignment sites write them together -- the command loop
/// (src/keyboard.c:1536-1537) and `Fundo_boundary` (src/undo.c:278-279) -- and
/// `record_point` reads them together, refusing the entry unless the buffer
/// still matches (src/undo.c:73-75).  Keeping them in one value is what makes
/// "saved the point, forgot the buffer" unrepresentable; a bare `CharPos0` let
/// a point saved in an indirect buffer be spent on an edit in its base, since
/// the two share this state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointBeforeCommand {
    /// The buffer that was current when the point was saved.
    pub buffer: BufferId,
    /// `point_before_last_command_or_undo`.
    pub point: CharPos0,
}

#[derive(Clone)]
pub struct SharedUndoState {
    inner: Rc<RefCell<SharedUndoStateInner>>,
}

#[derive(Clone)]
struct SharedUndoStateInner {
    list: Value,
    in_progress: bool,
    recorded_first_change: bool,
    point_before_command_or_undo: Option<PointBeforeCommand>,
}

impl Default for SharedUndoState {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedUndoState {
    pub fn new() -> Self {
        Self::from_parts(Value::NIL, false, false)
    }

    pub fn from_parts(list: Value, in_progress: bool, recorded_first_change: bool) -> Self {
        Self {
            inner: Rc::new(RefCell::new(SharedUndoStateInner {
                list,
                in_progress,
                recorded_first_change,
                point_before_command_or_undo: None,
            })),
        }
    }

    pub fn shares_with(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }

    pub fn list(&self) -> Value {
        self.inner.borrow().list
    }

    pub fn set_list(&self, list: Value) {
        self.inner.borrow_mut().list = list;
    }

    pub fn in_progress(&self) -> bool {
        self.inner.borrow().in_progress
    }

    pub fn set_in_progress(&self, in_progress: bool) {
        self.inner.borrow_mut().in_progress = in_progress;
    }

    pub fn recorded_first_change(&self) -> bool {
        self.inner.borrow().recorded_first_change
    }

    pub fn set_recorded_first_change(&self, recorded_first_change: bool) {
        self.inner.borrow_mut().recorded_first_change = recorded_first_change;
    }

    pub fn point_before_command_or_undo(&self) -> Option<PointBeforeCommand> {
        self.inner.borrow().point_before_command_or_undo
    }

    pub fn set_point_before_command_or_undo(&self, point: Option<PointBeforeCommand>) {
        self.inner.borrow_mut().point_before_command_or_undo = point;
    }

    pub fn trace_roots(&self, roots: &mut Vec<Value>) {
        roots.push(self.list());
    }
}
