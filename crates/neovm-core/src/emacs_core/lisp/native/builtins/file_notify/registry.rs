//! Evaluator-owned Lisp callback registry.
//!
//! Native watcher threads never hold Lisp values.  All callback lookup and GC
//! rooting stays on the evaluator thread behind this small ownership boundary.

use super::WatchId;
use crate::emacs_core::value::Value;
use hashbrown::HashMap;

#[derive(Default)]
pub(super) struct WatchRegistry {
    callbacks: HashMap<WatchId, Value>,
}

impl WatchRegistry {
    pub(super) fn register(&mut self, watch_id: WatchId, callback: Value) {
        let replaced = self.callbacks.insert(watch_id, callback);
        debug_assert!(replaced.is_none(), "native backend reused a live watch id");
    }

    pub(super) fn unregister(&mut self, watch_id: &WatchId) {
        self.callbacks.remove(watch_id);
    }

    pub(super) fn callback(&self, watch_id: &WatchId) -> Option<Value> {
        self.callbacks.get(watch_id).copied()
    }

    pub(super) fn collect_gc_roots(&self, roots: &mut Vec<Value>) {
        roots.extend(self.callbacks.values().copied());
    }
}
