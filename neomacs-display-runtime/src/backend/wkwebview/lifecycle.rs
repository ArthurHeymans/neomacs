//! What happens to a WebKit command, decided without AppKit.
//!
//! `WkWebViewHost` binds to winit's content view lazily, because a
//! `make-xwidget` can be evaluated before the primary frame is realized
//! (`FrameLifecycle::Pending`). The render thread drains its command channel
//! unconditionally, independent of frame presentation, so anything sent in
//! that window is gone by the time a view could be built for it -- a
//! `Create` followed by a `LoadUri` would leave a blank `WKWebView` behind
//! while Lisp reported the URI as loaded.
//!
//! So commands are kept and replayed once the host arrives. Every decision
//! about *whether* and *when* -- defer, replay, refuse, bind -- is made here
//! and returned as [`Action`]s; the host only executes them. That is what
//! makes the lifecycle testable: the sequences a review can name (a pending
//! xwidget killed the moment the window arrives; a flood of ids with no
//! window ever) are one `dispatch` call each.

use std::collections::BTreeSet;

use super::command::WebKitViewCommand;

/// How many deferred commands to keep.
///
/// A host that never arrives -- a headless run, or a window that fails to
/// realize -- would otherwise let this grow for the life of the process.
pub(super) const CAPACITY: usize = 256;

/// What the host has to do for one dispatched command.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Action {
    /// Retain the window's content view; emitted once, before any `Apply`.
    Bind,
    Apply(WebKitViewCommand),
}

/// The per-window WebKit lifecycle: pending queue, refusals, and binding.
///
/// Two invariants, both per xwidget rather than per command, because a view
/// that replays with its `Create` but without the `LoadUri` behind it is a
/// blank view -- the failure this whole module exists to prevent:
///
/// - **every id in the queue is whole or absent**, so overflow evicts an id's
///   earlier commands along with the one that would not fit;
/// - **a refused id stays refused until `Destroy`**, before and after binding,
///   so a later command for a lifecycle that was partly dropped cannot build
///   an orphan.
///
/// And one bound: the queue is capped at [`CAPACITY`] and stops accepting
/// once it has been full, so the refusal set can only ever hold ids evicted
/// from that one full queue. Retained state never exceeds the cap however
/// many ids arrive.
#[derive(Debug, Default)]
pub(super) struct Lifecycle {
    bound: bool,
    queue: Vec<WebKitViewCommand>,
    /// Ids that lost a queued command to overflow.
    rejected: BTreeSet<u32>,
    /// Latched the first time the queue is full. From then on new ids are
    /// dropped without being tracked -- tracking them is what would make
    /// memory and the log grow without bound.
    saturated: bool,
}

impl Lifecycle {
    pub(super) fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub(super) fn is_bound(&self) -> bool {
        self.bound
    }

    /// Nothing queued and nothing live to place -- the host uses this to
    /// skip frames, so it has to count pending work.
    pub(super) fn has_pending(&self) -> bool {
        !self.queue.is_empty()
    }

    /// The host has a window and nothing else to do: bind now and replay.
    /// Idempotent; the render pass calls this every presented frame.
    pub(super) fn bind(&mut self) -> Vec<Action> {
        if self.bound {
            return Vec::new();
        }
        self.bound = true;
        let mut actions = vec![Action::Bind];
        actions.extend(
            std::mem::take(&mut self.queue)
                .into_iter()
                .map(Action::Apply),
        );
        actions
    }

    /// Decide one command. `window_available` says whether the host *could*
    /// bind right now; binding is only worth doing for a command that will
    /// then be applied, which `Destroy` never is.
    pub(super) fn dispatch(
        &mut self,
        command: WebKitViewCommand,
        window_available: bool,
    ) -> Vec<Action> {
        let id = command.id();

        if let WebKitViewCommand::Destroy { .. } = command {
            // Never bind for a kill. Binding first would replay the queued
            // Create and LoadUri of the very xwidget being destroyed, which
            // could start a network request before the view is torn down.
            self.queue.retain(|queued| queued.id() != id);
            self.rejected.remove(&id);
            return if self.bound {
                vec![Action::Apply(command)]
            } else {
                Vec::new()
            };
        }

        if self.rejected.contains(&id) {
            tracing::debug!("wkwebview: refusing command for xwidget {id}, dropped at overflow");
            return Vec::new();
        }

        if self.bound {
            return vec![Action::Apply(command)];
        }
        if window_available {
            let mut actions = self.bind();
            actions.push(Action::Apply(command));
            return actions;
        }
        self.defer(command);
        Vec::new()
    }

    /// Queue a command for replay, or refuse it.
    ///
    /// Overflow is a latch. Once the queue has been full, nothing more is
    /// queued until a window arrives -- not even after a destroy or an
    /// eviction frees a slot. Refilling looked harmless and is not: every
    /// refill-then-evict cycle adds an id to the refusal set, so a stream of
    /// `Create`/`LoadUri` pairs would grow it by one per xwidget with no
    /// bound at all. Latched, the refusal set can only hold ids that were in
    /// the queue when it filled, and the two together never exceed the cap.
    ///
    /// While saturated, an arriving command whose id has queued commands
    /// evicts them and refuses the id -- replaying the earlier half of a
    /// lifecycle is the blank-view failure -- and an id with nothing queued
    /// is dropped and forgotten, since there is nothing to keep consistent.
    fn defer(&mut self, command: WebKitViewCommand) {
        let id = command.id();
        if !self.saturated && self.queue.len() < CAPACITY {
            self.queue.push(command);
            return;
        }
        if !self.saturated {
            self.saturated = true;
            tracing::warn!(
                "wkwebview: {CAPACITY} commands queued with no window to replay them into; \
                 dropping further xwidget commands until one arrives"
            );
        }
        let before = self.queue.len();
        self.queue.retain(|queued| queued.id() != id);
        if self.queue.len() != before {
            self.rejected.insert(id);
        }
    }

    /// Everything this type holds on to, for the bounded-memory test.
    #[cfg(test)]
    pub(super) fn retained(&self) -> usize {
        self.queue.len() + self.rejected.len()
    }
}

#[cfg(test)]
#[path = "lifecycle_test.rs"]
mod tests;
