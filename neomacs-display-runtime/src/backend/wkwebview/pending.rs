//! Operations that arrived before the native host view existed.
//!
//! `WkWebViewHost` binds to winit's content view lazily, because a
//! `make-xwidget` can be evaluated before the primary frame is realized
//! (`FrameLifecycle::Pending`). The render thread drains its command channel
//! unconditionally, independent of frame presentation, so anything sent in
//! that window is gone by the time a view could be built for it -- a
//! `Create` followed by a `LoadUri` would leave a blank `WKWebView` behind
//! while Lisp reported the URI as loaded.
//!
//! So the commands are kept, not just the sizes, and replayed in arrival
//! order once the host arrives.

/// One deferred operation.
///
/// These mirror the `AssetCommand::WebKit*` variants that reach
/// [`super::WkWebViewHost`], minus `Destroy`, which is applied to the queue
/// itself rather than deferred.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum PendingCommand {
    Create { id: u32, width: f64, height: f64 },
    LoadUri { id: u32, url: String },
    Resize { id: u32, width: f64, height: f64 },
    Script { id: u32, script: String },
}

impl PendingCommand {
    pub(super) fn id(&self) -> u32 {
        match *self {
            Self::Create { id, .. }
            | Self::LoadUri { id, .. }
            | Self::Resize { id, .. }
            | Self::Script { id, .. } => id,
        }
    }
}

use std::collections::BTreeSet;

/// How many deferred commands to keep.
///
/// A host that never arrives -- a headless run, or a window that fails to
/// realize -- would otherwise let this grow for the life of the process.
const CAPACITY: usize = 256;

/// Deferred commands, oldest first.
///
/// One queue for every id rather than one per id: ordering matters *across*
/// ids as well as within one, and a single vector gets that for free. The
/// counts here are small -- a handful of web views, a few commands each --
/// so the linear scans in [`PendingCommands::forget`] and overflow are not
/// worth indexing.
///
/// The invariant is per xwidget: **every id in the queue is whole or absent.**
/// A view that replays with its `Create` but without the `LoadUri` behind it
/// is a blank view, which is the failure this queue exists to prevent, so
/// overflow is atomic per id rather than per command.
#[derive(Debug, Default)]
pub(super) struct PendingCommands {
    queue: Vec<PendingCommand>,
    /// Ids that lost a command to overflow. Nothing further is accepted for
    /// them until `Destroy`, because a later `LoadUri` for a `Create` that was
    /// never queued would be an orphan.
    rejected: BTreeSet<u32>,
}

impl PendingCommands {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Defer one command.
    ///
    /// At capacity the arriving command's whole id is dropped -- its earlier
    /// queued commands too -- and the id is refused until `Destroy`. Dropping
    /// only the newest command looked sufficient (the oldest are the creates,
    /// so the front is the part worth keeping) but is not atomic at an id
    /// boundary: a `Create` accepted as the last entry with its `LoadUri`
    /// dropped replays into exactly the blank view being guarded against.
    pub(super) fn push(&mut self, command: PendingCommand) {
        let id = command.id();
        if self.rejected.contains(&id) {
            return;
        }
        if self.queue.len() >= CAPACITY {
            tracing::warn!(
                "wkwebview: more than {CAPACITY} commands queued with no window to \
                 replay them into; dropping xwidget {id} until it is killed"
            );
            self.queue.retain(|queued| queued.id() != id);
            self.rejected.insert(id);
            return;
        }
        self.queue.push(command);
    }

    /// Drop everything queued for a killed xwidget, and let a later lifecycle
    /// under the same id start clean.
    pub(super) fn forget(&mut self, id: u32) {
        self.queue.retain(|command| command.id() != id);
        self.rejected.remove(&id);
    }

    /// Take the queue for replay. The caller re-dispatches each command
    /// through the normal path, so this must leave the queue empty -- a
    /// replayed command that finds no host again would otherwise re-queue
    /// itself forever.
    pub(super) fn take(&mut self) -> Vec<PendingCommand> {
        std::mem::take(&mut self.queue)
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
#[path = "pending_test.rs"]
mod tests;
