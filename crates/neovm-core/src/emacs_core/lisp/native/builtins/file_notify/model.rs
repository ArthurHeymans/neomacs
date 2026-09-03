//! Platform-neutral types at the file-notification boundary.
//!
//! A backend chooses its own request and event types through associated types.
//! That keeps platform-specific flag vocabularies out of the common state
//! machine and makes cross-platform request mismatches a compile-time error.

use crate::emacs_core::error::Flow;
use crate::emacs_core::process::WaitNotifier;
use crate::emacs_core::value::Value;
use std::path::{Path, PathBuf};

/// Stable identity for one native watch registration.
///
/// The generation is part of the identity even on backends whose current Lisp
/// representation omits it.  This prevents a future descriptor reuse policy
/// from accidentally delivering a stale event to a newer registration.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct WatchId {
    slot: i64,
    generation: i64,
}

impl WatchId {
    pub(super) fn new(slot: i64, generation: i64) -> Self {
        debug_assert!(slot >= 0);
        debug_assert!(generation >= 0);
        Self { slot, generation }
    }

    pub(super) fn to_inotify_lisp(&self) -> Value {
        Value::cons(Value::fixnum(self.slot()), Value::fixnum(self.generation()))
    }

    pub(super) fn slot(&self) -> i64 {
        self.slot
    }

    pub(super) fn generation(&self) -> i64 {
        self.generation
    }
}

#[derive(Clone, Debug)]
pub(super) struct FileWatch<Request> {
    pub(super) id: WatchId,
    pub(super) path: PathBuf,
    pub(super) request: Request,
}

pub(super) trait BackendEvent {
    fn watch_id(&self) -> &WatchId;
    fn into_lisp(self) -> Value;
}

/// One atomic handoff from a native backend to the evaluator.
///
/// Watch termination is control-plane state, not an attribute of a
/// Lisp-visible event.  Keeping it separate ensures callback roots are
/// released even when a terminal native notification produces no event for
/// the watch's requested action set.
pub(super) struct DrainBatch<Event> {
    pub(super) events: Vec<Event>,
    pub(super) terminated: Vec<WatchId>,
}

pub(super) trait Backend {
    type Request;
    type Event: BackendEvent;

    fn add_watch(
        &mut self,
        path: &Path,
        request: Self::Request,
        notifier: Option<WaitNotifier>,
    ) -> Result<WatchId, Flow>;
    fn remove_watch(&mut self, watch_id: &WatchId) -> Result<bool, Flow>;
    fn valid_p(&self, watch_id: &WatchId) -> bool;
    fn drain_events(&mut self) -> Result<DrainBatch<Self::Event>, Flow>;
    fn has_watches(&self) -> bool;
}
