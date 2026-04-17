use crate::emacs_core::value::Value;

/// Trait for types that hold GC-managed `Value` references.
///
/// Each runtime subsystem implements this to enumerate all `Value`s it holds,
/// so the collector can discover every live object reachable from explicit
/// runtime roots.
pub trait GcTrace {
    /// Push all `Value` references held by `self` into `roots`.
    fn trace_roots(&self, roots: &mut Vec<Value>);

    /// Visit all `Value` references held by `self` without requiring the
    /// caller to materialize an intermediate root vector.
    fn trace_roots_with(&self, visit: &mut dyn FnMut(Value)) {
        let mut roots = Vec::new();
        self.trace_roots(&mut roots);
        for root in roots {
            visit(root);
        }
    }

    /// Visit all `Value` references held by `self` with mutable access,
    /// so a moving collector can rewrite each pointer after an
    /// evacuation copy. Phase δ of the moving-nursery roadmap:
    /// concrete impls override this to expose `&mut Value` for every
    /// slot they store.
    ///
    /// Default implementation is a read-only fallback that cannot
    /// rewrite — it delegates to `trace_roots_with` and drops the
    /// Values on the floor. Sub-systems that hold types in a
    /// `Space::Nursery` must override this with real `&mut Value`
    /// visits before their types can flip to `MovePolicy::Movable`.
    ///
    /// Until Phase δ is complete, overriding this is optional; the
    /// default keeps the trait backward-compatible and lets the
    /// collector's evacuation path no-op harmlessly on subsystems
    /// that haven't migrated yet.
    fn trace_roots_mut(&mut self, visit: &mut dyn FnMut(&mut Value)) {
        // Backwards-compatible fallback: visit values by value (copy).
        // The `visit` callback receives &mut Value but any rewrite
        // applies only to the local copy, so this path cannot
        // actually track moved objects. Overriders must traverse
        // their internal storage mutably.
        let _ = visit;
    }
}
