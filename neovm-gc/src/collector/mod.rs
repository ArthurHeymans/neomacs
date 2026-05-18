pub(crate) mod exec;
pub(crate) mod session;

// Re-export commonly-used types at the collector module level
pub(crate) use exec::{
    collect_dirty_card_root_indices, collect_dirty_card_root_indices_with_counter,
    collect_dirty_card_root_locators_with_counter, collect_global_sources,
    execute_collection_plan, prepare_full_reclaim_for_plan,
    prepare_major_reclaim_for_plan,
    refresh_block_card_marks_after_minor, trace_collection, trace_major, trace_major_ephemerons,
    trace_major_ephemerons_for_candidates, trace_minor,
    ForwardingRelocator, MajorEphemeronTracer, MajorMarkSession, MarkTracer, MinorTracer,
    ParallelMarkShared,
};
pub(crate) use session::{
    build_prepared_active_reclaim, prepare_active_reclaim, ActiveReclaimPrepRequest,
    FinishedActiveCollection, PreparedActiveReclaim,
};

// ── MarkWorklist (merged from mark.rs) ──
/// Splittable LIFO worklist used by mark tracers.
#[derive(Debug)]
pub(crate) struct MarkWorklist<T> {
    entries: Vec<T>,
}

impl<T> Default for MarkWorklist<T> {
    fn default() -> Self {
        Self { entries: Vec::default() }
    }
}

impl<T> MarkWorklist<T> {
    pub(crate) fn push(&mut self, value: T) { self.entries.push(value); }
    pub(crate) fn pop(&mut self) -> Option<T> { self.entries.pop() }
    pub(crate) fn len(&self) -> usize { self.entries.len() }
    pub(crate) fn is_empty(&self) -> bool { self.entries.is_empty() }
    pub(crate) fn split_half(&mut self) -> Self {
        let split_at = self.entries.len() >> 1;
        Self { entries: self.entries.split_off(split_at) }
    }
    pub(crate) fn append(&mut self, other: &mut Self) {
        self.entries.append(&mut other.entries);
    }
}

#[cfg(test)]
#[path = "mark_test.rs"]
mod mark_tests;
