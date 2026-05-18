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
