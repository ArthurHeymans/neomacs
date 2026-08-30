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
/// so the linear scan in [`PendingCommands::forget`] is not worth indexing.
#[derive(Debug, Default)]
pub(super) struct PendingCommands {
    queue: Vec<PendingCommand>,
    /// One-shot latch: the overflow warning names a runaway, and repeating it
    /// once per dropped command would bury the rest of the log.
    warned_full: bool,
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
    /// At capacity the *newest* is dropped, not the oldest: what survives is
    /// then a coherent prefix of the sequence, where evicting from the front
    /// would drop a `Create` and leave every later command for that id
    /// referring to a view that is never built.
    pub(super) fn push(&mut self, command: PendingCommand) {
        if self.queue.len() >= CAPACITY {
            if !self.warned_full {
                self.warned_full = true;
                tracing::warn!(
                    "wkwebview: more than {CAPACITY} commands queued with no window to \
                     replay them into; dropping the rest"
                );
            }
            return;
        }
        self.queue.push(command);
    }

    /// Drop everything queued for a killed xwidget.
    pub(super) fn forget(&mut self, id: u32) {
        self.queue.retain(|command| command.id() != id);
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
