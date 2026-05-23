pub(crate) mod exec;
pub(crate) mod session;

// Re-export types used across crate boundaries or between collector submodules
pub(crate) use exec::{
    ForwardingRelocator, MarkTracer, collect_dirty_card_root_locators_with_counter,
    collect_global_sources, execute_collection_plan, prepare_full_reclaim_for_plan,
    prepare_major_reclaim_for_plan, trace_major_ephemerons_for_candidates,
};
pub(crate) use session::{
    ActiveReclaimPrepRequest, FinishedActiveCollection, PreparedActiveReclaim,
    build_prepared_active_reclaim, prepare_active_reclaim,
};

// ── MarkWorklist (merged from mark.rs) ──
/// Splittable LIFO worklist used by mark tracers.
#[derive(Debug)]
pub(crate) struct MarkWorklist<T> {
    entries: Vec<T>,
}

impl<T> Default for MarkWorklist<T> {
    fn default() -> Self {
        Self {
            entries: Vec::default(),
        }
    }
}

impl<T> MarkWorklist<T> {
    pub(crate) fn push(&mut self, value: T) {
        self.entries.push(value);
    }
    pub(crate) fn pop(&mut self) -> Option<T> {
        self.entries.pop()
    }
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub(crate) fn split_half(&mut self) -> Self {
        let split_at = self.entries.len() >> 1;
        Self {
            entries: self.entries.split_off(split_at),
        }
    }
    pub(crate) fn append(&mut self, other: &mut Self) {
        self.entries.append(&mut other.entries);
    }
}

#[cfg(test)]
#[path = "mark_test.rs"]
mod mark_tests;
