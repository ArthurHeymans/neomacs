use crate::background::{
    BackgroundCollectionRuntime, BackgroundCollectorConfig, BackgroundWorker,
    BackgroundWorkerConfig, SharedBackgroundError, SharedBackgroundObservation,
    SharedBackgroundService, SharedBackgroundStatus, SharedBackgroundWaitResult,
    SharedCollectorHandle, SharedHeap, SharedHeapError, SharedHeapStatus, SharedRuntimeHandle,
};
use crate::collector_exec::{
    advance_active_major_ephemeron_trace, begin_active_major_ephemeron_trace,
    collect_global_sources, execute_collection_plan, prepare_full_reclaim_prefix_for_plan,
    process_weak_references_for_candidates,
};
use crate::collector_policy::refresh_cached_plans as refresh_cached_collector_plans;
use crate::collector_session;
use crate::collector_state::{CollectorSharedSnapshot, CollectorState, MajorMarkState};
use crate::descriptor::{GcErased, TypeDesc};
use crate::heap::{AllocError, HeapCore, TryCollectorRuntimeError};
use crate::index_state::ForwardingMap;
use crate::object::SpaceKind;
use crate::plan::{
    BackgroundCollectionStatus, CollectionKind, CollectionPhase, CollectionPlan, MajorMarkProgress,
    RuntimeWorkStatus,
};
use crate::reclaim::{
    advance_active_prepared_reclaim_build, advance_prepared_reclaim_commit,
    begin_active_prepared_reclaim_build, begin_prepared_reclaim_commit,
    finish_active_prepared_reclaim_build, finish_prepared_reclaim_cycle_with_state,
};
use crate::stats::{CollectionStats, HeapStats};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

#[cfg(test)]
use crate::root::{HandleScope, Root};

/// Default object budget for one bounded reclaim-prep assist.
pub const DEFAULT_RECLAIM_PREP_SLICE_BUDGET: usize = 256;
const MAX_RECLAIM_PREP_SLICE_BUDGET: usize = 16 * 1024;
/// Default object budget for one bounded reclaim-commit assist.
///
/// Reclaim commit runs stop-the-world, so it deliberately uses a
/// small fixed budget instead of inheriting the throughput-oriented
/// concurrent mark slice size from `CollectionPlan::mark_slice_budget`.
pub const DEFAULT_RECLAIM_COMMIT_SLICE_BUDGET: usize = 64;
const MAX_RECLAIM_COMMIT_SLICE_BUDGET: usize = 4096;
pub(crate) const DEFAULT_MAJOR_MARK_SLICE_BUDGET: usize = 1024;
const DEFAULT_AUTO_COMPACTION_SLICE_BUDGET_BYTES: usize = 256 * 1024;
const MIN_AUTO_COMPACTION_SLICE_BUDGET_BYTES: usize = 64 * 1024;
const MAX_AUTO_COMPACTION_SLICE_BUDGET_BYTES: usize = 8 * 1024 * 1024;

pub(crate) fn bounded_major_mark_plan(mut plan: CollectionPlan) -> CollectionPlan {
    if matches!(plan.kind, CollectionKind::Major | CollectionKind::Full) {
        plan.mark_slice_budget = plan
            .mark_slice_budget
            .max(1)
            .min(DEFAULT_MAJOR_MARK_SLICE_BUDGET);
    }
    plan
}

fn adaptive_pause_target_budget(
    target_pause: Duration,
    observed_units: usize,
    observed_pause_nanos: u64,
    default_budget: usize,
    min_budget: usize,
    max_budget: usize,
) -> usize {
    if observed_units == 0 || observed_pause_nanos == 0 {
        return default_budget.clamp(min_budget, max_budget);
    }
    let target_pause_nanos = elapsed_nanos(target_pause);
    if target_pause_nanos == 0 {
        return min_budget.max(1);
    }

    let projected = ((observed_units as u128) * (target_pause_nanos as u128))
        .div_ceil(observed_pause_nanos as u128)
        .min(usize::MAX as u128) as usize;
    projected.clamp(min_budget, max_budget)
}

/// Collector-side runtime bound to one heap.
///
/// The runtime carries a per-cycle `MutatorLocal` that it
/// either borrows from an outer [`crate::mutator::Mutator`]
/// or owns for the duration of the call. Every collector
/// entry point that needs to walk roots threads that local
/// through.
#[derive(Debug)]
pub struct CollectorRuntime<'heap> {
    heap: &'heap mut HeapCore,
    local: CollectorLocal<'heap>,
}

/// Borrow of the [`crate::mutator::MutatorLocal`] carried by
/// [`CollectorRuntime`]. The runtime always borrows from
/// either an outer [`crate::mutator::Mutator`] (the
/// production path) or a scratch local owned by
/// [`crate::heap::HeapCollectorRuntime`] (the non-mutator
/// path). The runtime itself never owns the local.
#[derive(Debug)]
pub(crate) struct CollectorLocal<'heap> {
    inner: &'heap mut crate::mutator::MutatorLocal,
}

impl CollectorLocal<'_> {
    pub(crate) fn get(&self) -> &crate::mutator::MutatorLocal {
        self.inner
    }

    pub(crate) fn get_mut(&mut self) -> &mut crate::mutator::MutatorLocal {
        self.inner
    }
}

/// Try to bump-allocate `layout` through the caller-supplied
/// nursery TLAB slab. On TLAB miss (including the "no TLAB
/// reserved yet" case and the "generation-stale" case after
/// a minor cycle), refill the slab from the shared from-
/// space cursor via `NurseryState::reserve_tlab` and retry
/// the bump once. Returns `None` if neither the existing
/// TLAB nor the refilled one can service the layout; the
/// caller falls through to the shared-cursor bump path.
///
/// The `tlab_slot` parameter is a `&mut Option<NurseryTlab>`
/// so the caller can own the slab wherever it likes —
/// currently on `MutatorLocal`, so each mutator has its own
/// per-mutator slab without serializing on the shared
/// cursor for the common case.
///
/// `tlab_bytes` comes from
/// [`crate::spaces::NurseryConfig::tlab_bytes`] and controls
/// how much from-space capacity each refill reserves.
pub(crate) fn try_bump_nursery_tlab_or_refill(
    tlab_slot: &mut Option<crate::spaces::nursery_arena::NurseryTlab>,
    nursery: &mut crate::spaces::nursery_arena::NurseryState,
    layout: core::alloc::Layout,
    tlab_bytes: usize,
) -> Option<core::ptr::NonNull<u8>> {
    let current_generation = nursery.generation();

    if let Some(tlab) = tlab_slot.as_mut()
        && let Some(base) = tlab.try_alloc(current_generation, layout)
    {
        return Some(base);
    }

    // The existing TLAB (if any) is either exhausted or stale.
    // Drop it and try to refill from the shared cursor. The
    // refill size is `max(layout.size(), tlab_bytes)` so a
    // single oversized allocation still has a chance of
    // fitting via the TLAB path even when tlab_bytes is set
    // smaller than the object.
    *tlab_slot = None;
    let refill_size = tlab_bytes.max(layout.size());
    if refill_size == 0 {
        return None;
    }
    let mut fresh = nursery.reserve_tlab(refill_size)?;
    let base = fresh.try_alloc(current_generation, layout)?;
    *tlab_slot = Some(fresh);
    Some(base)
}

/// Collector-side runtime bound to one shared heap.
#[derive(Clone, Debug)]
pub struct SharedCollectorRuntime {
    heap: SharedHeap,
    runtime: SharedRuntimeHandle,
    collector: SharedCollectorHandle,
}

impl<'heap> CollectorRuntime<'heap> {
    /// Build a runtime that borrows the supplied mutator
    /// local. Used by [`crate::heap::HeapCollectorRuntime::runtime`]
    /// (which carries a scratch local) and by
    /// [`crate::mutator::Mutator::with_runtime`] (which
    /// borrows the mutator's own local).
    pub(crate) fn with_local(
        heap: &'heap mut HeapCore,
        local: &'heap mut crate::mutator::MutatorLocal,
    ) -> Self {
        Self {
            heap,
            local: CollectorLocal { inner: local },
        }
    }

    /// Return current heap statistics.
    pub fn stats(&self) -> HeapStats {
        self.heap.stats()
    }

    /// Return the most recently completed collection plan, if any.
    pub fn last_completed_plan(&self) -> Option<CollectionPlan> {
        self.heap.last_completed_plan()
    }

    /// Return the number of queued finalizers waiting to run.
    pub fn pending_finalizer_count(&self) -> usize {
        self.heap.pending_finalizer_count()
    }

    /// Return runtime-side follow-up work that remains outside GC commit.
    pub fn runtime_work_status(&self) -> RuntimeWorkStatus {
        self.heap.runtime_work_status()
    }

    /// Run and drain queued finalizers.
    pub fn drain_pending_finalizers(&mut self) -> u64 {
        self.heap.drain_pending_finalizers()
    }

    /// Run at most `max` queued finalizers and return the number
    /// that actually ran. See [`crate::heap::Heap::drain_pending_finalizers_bounded`].
    pub fn drain_pending_finalizers_bounded(&mut self, max: usize) -> u64 {
        self.heap.drain_pending_finalizers_bounded(max)
    }

    /// Recommend the next background concurrent collection plan, if any.
    pub fn recommended_background_plan(&self) -> Option<CollectionPlan> {
        self.heap.recommended_background_plan()
    }

    /// Return the active major-mark plan, if one is in progress.
    pub fn active_major_mark_plan(&self) -> Option<CollectionPlan> {
        self.heap.active_major_mark_plan()
    }

    /// Return progress for the active major-mark session, if any.
    pub fn major_mark_progress(&self) -> Option<MajorMarkProgress> {
        self.heap.major_mark_progress()
    }

    /// Run one stop-the-world collection cycle.
    pub fn collect(&mut self, kind: CollectionKind) -> Result<CollectionStats, AllocError> {
        self.execute_plan(self.heap.plan_for(kind))
    }

    /// Execute one scheduler-provided collection plan.
    pub fn execute_plan(&mut self, plan: CollectionPlan) -> Result<CollectionStats, AllocError> {
        if self.heap.collector_handle().has_active_major_mark() {
            return Err(AllocError::CollectionInProgress);
        }
        if matches!(plan.kind, CollectionKind::Major | CollectionKind::Full) {
            self.heap.clear_pending_auto_compaction();
        }

        let pause_start = Instant::now();
        self.heap.collector_handle().clear_recent_phase_trace();
        let runtime_state = self.heap.runtime_state_handle();
        let mut phases = Vec::new();
        let roots = self.local.get_mut().roots_mut();
        let mut cycle = self.heap.with_flat_store_for_collection(
            |flat,
             old_gen,
             old_config,
             nursery_config,
             stats,
             nursery,
             ext_scanner,
             ext_relocator| {
                execute_collection_plan(
                    &plan,
                    roots,
                    &mut flat.objects,
                    &mut flat.indexes,
                    old_gen,
                    old_config,
                    nursery_config,
                    stats,
                    nursery,
                    &runtime_state,
                    ext_scanner.map(|s| s as &crate::heap::ExternalRootScanner),
                    ext_relocator.map(|r| r as &crate::heap::ExternalRootRelocator),
                    |phase| phases.push(phase),
                )
            },
        )?;
        self.heap.collector_handle().push_phases(phases);
        cycle.pause_nanos = pause_start.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
        self.heap.record_pause_sample(cycle.pause_nanos);
        self.record_completed_cycle(
            cycle,
            CollectionPlan {
                phase: CollectionPhase::Reclaim,
                ..plan
            },
        );
        if matches!(plan.kind, CollectionKind::Major | CollectionKind::Full) {
            self.heap.schedule_auto_compaction_if_enabled();
        } else {
            self.heap.clear_pending_auto_compaction();
        }
        Ok(cycle)
    }

    pub(crate) fn service_allocation_pressure(
        &mut self,
        space: SpaceKind,
        bytes: usize,
    ) -> Result<(), AllocError> {
        if self.heap.collector_handle().has_active_major_mark() {
            return Ok(());
        }
        let Some(plan) = self.heap.allocation_pressure_plan(space, bytes) else {
            return Ok(());
        };
        self.dispatch_collection_plan(plan)
    }

    /// Dispatch one [`CollectionPlan`] either as a background
    /// concurrent major-mark session or as an immediate synchronous
    /// `execute_plan`. Used by both the static pressure path and
    /// the pacer-driven trigger so they pick the same path for the
    /// same plan kind.
    pub(crate) fn dispatch_collection_plan(
        &mut self,
        plan: CollectionPlan,
    ) -> Result<(), AllocError> {
        if plan.concurrent && matches!(plan.kind, CollectionKind::Major | CollectionKind::Full) {
            self.begin_major_mark(plan)
        } else {
            self.execute_plan(plan).map(|_| ())
        }
    }

    pub(crate) fn prepare_typed_allocation<T: crate::descriptor::Trace + 'static>(
        &mut self,
    ) -> Result<(), AllocError> {
        if self.heap.prepared_full_reclaim_active() {
            return Err(AllocError::CollectionInProgress);
        }
        let (_, space, total_bytes) = self.typed_allocation_profile::<T>()?;
        // Layer the adaptive pacer on top of the static thresholds.
        // The pacer never overrides the static path: if the static
        // pressure plan would already collect, that still wins. The
        // pacer only forces an additional collection when its model
        // believes the next major (or early minor) is due.
        //
        // We always advance the pacer's per-allocation accounting
        // here so its EWMA estimates stay current, then we run the
        // static plan, then we re-evaluate the pacer's decision.
        // The re-evaluation matters because the static path may
        // have completed a minor cycle in the meantime — that
        // resets the pacer's nursery counter and turns a stale
        // TriggerMinor back into Continue.
        self.heap.pacer().record_allocation(total_bytes, space);
        self.service_allocation_pressure(space, total_bytes)?;
        let pacer_decision = self.heap.pacer().decision();
        match pacer_decision {
            crate::pacer::PacerDecision::TriggerMajor => {
                if !self.heap.collector_handle().has_active_major_mark()
                    && self
                        .heap
                        .allocation_pressure_plan(space, total_bytes)
                        .is_none()
                {
                    // Honor the heap's `concurrent_mark_workers`
                    // configuration: a pacer-driven major should
                    // start a background mark session when the
                    // static path would have done so.
                    let plan = self.heap.plan_for(CollectionKind::Major);
                    self.dispatch_collection_plan(plan)?;
                    // Only count the trigger after dispatch
                    // succeeds so pacer_triggered_majors agrees
                    // with observed_cycles for the pacer path.
                    self.heap.pacer().record_pacer_triggered_major();
                }
            }
            crate::pacer::PacerDecision::TriggerMinor => {
                if !self.heap.collector_handle().has_active_major_mark()
                    && self
                        .heap
                        .allocation_pressure_plan(space, total_bytes)
                        .is_none()
                {
                    // Minor plans are never concurrent, so this
                    // dispatches to execute_plan via
                    // dispatch_collection_plan unconditionally.
                    let plan = self.heap.plan_for(CollectionKind::Minor);
                    self.dispatch_collection_plan(plan)?;
                    self.heap.pacer().record_pacer_triggered_minor();
                }
            }
            crate::pacer::PacerDecision::Continue => {}
        }
        Ok(())
    }

    /// Try to allocate `layout` bytes of nursery storage
    /// by bumping within the carried local's TLAB. Returns
    /// `None` if both the TLAB refill and the shared
    /// from-space cursor fail.
    #[cfg(test)]
    fn try_alloc_nursery_with_local(
        &mut self,
        layout: core::alloc::Layout,
    ) -> Option<core::ptr::NonNull<u8>> {
        let tlab_bytes = self.heap.config().nursery.tlab_bytes;
        // Split borrow: self.local and self.heap are disjoint
        // fields of CollectorRuntime, so we can mutably
        // reference both at the same time.
        let local: &mut crate::mutator::MutatorLocal = self.local.get_mut();
        let tlab = &mut local.tlab;
        let heap: &mut HeapCore = &mut *self.heap;
        try_bump_nursery_tlab_or_refill(tlab, heap.nursery_mut(), layout, tlab_bytes)
            .or_else(|| heap.nursery_mut().try_alloc(layout))
    }

    /// Allocate a typed managed object through this
    /// runtime's carried `MutatorLocal`.
    ///
    /// Nursery allocations attempt to bump within the
    /// local's TLAB slab via
    /// [`try_bump_nursery_tlab_or_refill`]. On TLAB hit the
    /// allocation never touches the shared from-space
    /// cursor. On TLAB miss the slab is refilled from the
    /// shared cursor; on refill failure the allocation
    /// falls through to the shared-cursor bump path; on
    /// shared-cursor failure the allocation falls back to
    /// the system allocator.
    ///
    /// Direct old-gen allocations bump-allocate from the
    /// block pool with a system-alloc fallback, identical
    /// to the no-TLAB path.
    ///
    /// Pinned, large, and immortal-space allocations always
    /// bypass the TLAB and go through the system allocator.
    #[cfg(test)]
    pub(crate) fn alloc_typed_scoped<
        'scope,
        'handle_heap,
        T: crate::descriptor::Trace + 'static,
    >(
        &mut self,
        scope: &mut HandleScope<'scope, 'handle_heap>,
        value: T,
    ) -> Result<Root<'scope, T>, AllocError> {
        if self.heap.prepared_full_reclaim_active() {
            return Err(AllocError::CollectionInProgress);
        }
        let (desc, space, _) = self.typed_allocation_profile::<T>()?;
        let mut old_reserved_bytes = 0usize;

        let record = match space {
            SpaceKind::Nursery => {
                let (layout, payload_offset) = crate::object::allocation_layout_for::<T>()?;
                let base = self.try_alloc_nursery_with_local(layout);
                match base {
                    Some(base) => unsafe {
                        crate::object::ObjectRecord::allocate_in_arena::<T>(
                            desc,
                            space,
                            base,
                            layout,
                            payload_offset,
                            value,
                        )
                    },
                    None => crate::object::ObjectRecord::allocate(desc, space, value)?,
                }
            }
            SpaceKind::Old => {
                let (layout, payload_offset) = crate::object::allocation_layout_for::<T>()?;
                let old_config = *self.heap.old_config();
                match self
                    .heap
                    .old_gen_mut()
                    .try_alloc_in_block_with_reserved(&old_config, layout)
                {
                    Some((placement, base, reserved_bytes)) => {
                        old_reserved_bytes = reserved_bytes;
                        let mut record = unsafe {
                            crate::object::ObjectRecord::allocate_in_arena::<T>(
                                desc,
                                space,
                                base,
                                layout,
                                payload_offset,
                                value,
                            )
                        };
                        record.set_old_block_placement(placement);
                        record
                    }
                    None => {
                        old_reserved_bytes = self.heap.old_gen().reserved_bytes();
                        crate::object::ObjectRecord::allocate(desc, space, value)?
                    }
                }
            }
            _ => crate::object::ObjectRecord::allocate(desc, space, value)?,
        };
        let local = self.local.get_mut();
        let (publish_local, alloc_counter_local) = local.publish_and_alloc_counter_local_mut();
        let commit = self.heap.commit_allocated_record_shared(
            record,
            old_reserved_bytes,
            publish_local,
            alloc_counter_local,
            false,
        )?;
        if commit.plans_dirty {
            self.heap.refresh_recommended_plans();
        }
        let gc = unsafe { crate::root::Gc::from_erased(commit.gc) };
        Ok(scope.root(gc))
    }

    fn typed_allocation_profile<T: crate::descriptor::Trace + 'static>(
        &mut self,
    ) -> Result<(&'static TypeDesc, SpaceKind, usize), AllocError> {
        let desc = self.heap.descriptor_for::<T>();
        let payload_bytes = core::mem::size_of::<T>();
        let total_bytes = crate::object::estimated_allocation_size::<T>()?;
        let space = crate::collector_policy::select_allocation_space(
            self.heap.config(),
            desc,
            payload_bytes,
        );
        Ok((desc, space, total_bytes))
    }

    pub(crate) fn root_during_active_major_mark(&mut self, object: GcErased) {
        assert!(
            !self.heap.prepared_full_reclaim_active(),
            "cannot add new roots while prepared full reclaim is active"
        );
        let objects = self.heap.mark_objects();
        let _ = self
            .heap
            .collector_handle()
            .record_active_major_reachable_object_and_refresh(
                objects.raw(),
                object,
                self.heap.config().old.mutator_assist_slices,
            )
            .expect("rooting during active major-mark should not fail");
    }

    // NOTE: `record_post_write` was removed after the barrier
    // fast path was moved onto the `HeapCore` read-lock path
    // in `Mutator::post_write_barrier`. The old write-lock
    // variant is no longer needed because the barrier no
    // longer requires exclusive access to the heap core.

    /// Begin a persistent major-mark session for one scheduler-provided plan.
    pub fn begin_major_mark(&mut self, plan: CollectionPlan) -> Result<(), AllocError> {
        self.heap.clear_pending_auto_compaction();
        let objects = self.heap.objects();
        let sources = self
            .heap
            .global_sources_with_roots_from_objects(&self.local.get().roots, &objects);
        self.heap.collector_handle().begin_major_mark_and_refresh(
            objects.raw(),
            plan,
            sources,
            &self.heap.planning_stats(),
            self.heap.old_gen(),
            self.heap.old_config(),
            |kind| self.heap.plan_for(kind),
        )
    }

    /// Advance one scheduler-style concurrent major-mark round using the active plan worker count.
    pub fn poll_active_major_mark(&mut self) -> Result<Option<MajorMarkProgress>, AllocError> {
        let progress = {
            let objects = self.heap.mark_objects();
            self.heap.collector_handle().with_state(|state| {
                let progress =
                    collector_session::poll_active_major_mark_round(state, objects.raw())?;
                state.refresh_cached_active_major_plans();
                Ok(progress)
            })
        }?;
        let auto_prepare_major_reclaim =
            progress.as_ref().is_some_and(|progress| progress.completed)
                && self
                    .heap
                    .collector_handle()
                    .active_reclaim_prep_request()
                    .is_some_and(|request| request.plan.kind == CollectionKind::Major);
        if auto_prepare_major_reclaim {
            let _ = self.prepare_active_reclaim_if_needed()?;
            return Ok(progress);
        }
        Ok(progress)
    }

    /// Advance one slice of the current persistent major-mark session.
    pub fn advance_major_mark(&mut self) -> Result<MajorMarkProgress, AllocError> {
        let progress = self.assist_major_mark(1)?;
        let progress = progress.expect("single-slice assist should require an active session");
        Ok(progress)
    }

    /// Advance up to `max_slices` of the active major-mark session.
    pub fn assist_major_mark(
        &mut self,
        max_slices: usize,
    ) -> Result<Option<MajorMarkProgress>, AllocError> {
        if !self.heap.collector_handle().has_active_major_mark() {
            return Ok(None);
        }
        if max_slices == 0 {
            return Ok(self.heap.major_mark_progress());
        }
        self.heap
            .collector_handle()
            .assist_active_major_mark_slices_and_refresh(self.heap.mark_objects().raw(), max_slices)
    }

    /// Finish the current persistent major-mark session and reclaim.
    pub fn finish_major_collection(&mut self) -> Result<CollectionStats, AllocError> {
        if self.active_major_mark_plan().is_none() {
            return Err(AllocError::NoCollectionInProgress);
        }

        loop {
            if let Some(cycle) = self.finish_active_major_collection_if_ready()? {
                return Ok(cycle);
            }
            let Some(plan) = self.active_major_mark_plan() else {
                return Err(AllocError::NoCollectionInProgress);
            };
            if plan.phase == CollectionPhase::Reclaim {
                continue;
            }
            let Some(progress) = self.poll_active_major_mark()? else {
                return Err(AllocError::NoCollectionInProgress);
            };
            debug_assert!(
                progress.completed || progress.drained_objects > 0,
                "active major mark should either complete or drain at least one object per poll"
            );
        }
    }

    /// Prepare reclaim for the active major collection once mark work is fully drained.
    pub fn prepare_active_reclaim_if_needed(&mut self) -> Result<bool, AllocError> {
        let Some((pause_nanos, scanned_objects)) =
            self.heap.collector_handle().active_reclaim_prep_progress()
        else {
            return self
                .prepare_active_reclaim_if_needed_with_budget(DEFAULT_RECLAIM_PREP_SLICE_BUDGET);
        };
        let budget = adaptive_pause_target_budget(
            self.heap.pacer().config().target_pause,
            scanned_objects,
            pause_nanos,
            DEFAULT_RECLAIM_PREP_SLICE_BUDGET,
            1,
            MAX_RECLAIM_PREP_SLICE_BUDGET,
        );
        self.prepare_active_reclaim_if_needed_with_budget(budget)
    }

    /// Prepare reclaim for the active major collection once mark work is
    /// fully drained, using a bounded object-scan budget.
    pub fn prepare_active_reclaim_if_needed_with_budget(
        &mut self,
        budget: usize,
    ) -> Result<bool, AllocError> {
        let snapshot = self.heap.collector_shared_snapshot();
        if snapshot.active_major_mark_plan.is_none() {
            return Ok(false);
        }
        if snapshot
            .major_mark_progress
            .is_some_and(|progress| !progress.completed)
        {
            return Ok(false);
        }
        let request = self.heap.collector_handle().active_reclaim_prep_request();
        let Some(request) = request else {
            return Ok(false);
        };
        let pause_start = Instant::now();
        let Some(mut state) = self.heap.collector_handle().take_major_mark_state() else {
            return Ok(false);
        };
        if !state.worklist.is_empty() || state.reclaim_prepared {
            self.heap
                .collector_handle()
                .restore_major_mark_state_and_refresh(
                    state,
                    &self.heap.planning_stats(),
                    self.heap.old_gen(),
                    self.heap.old_config(),
                    |kind| self.heap.plan_for(kind),
                );
            return Ok(false);
        }

        if !advance_active_reclaim_ephemerons(self.heap, &mut state, &request, budget) {
            let pause_nanos = elapsed_nanos(pause_start.elapsed());
            state.reclaim_prepare_nanos = state.reclaim_prepare_nanos.saturating_add(pause_nanos);
            self.heap.record_pause_sample(pause_nanos);
            self.heap
                .collector_handle()
                .restore_major_mark_state_and_refresh(
                    state,
                    &self.heap.planning_stats(),
                    self.heap.old_gen(),
                    self.heap.old_config(),
                    |kind| self.heap.plan_for(kind),
                );
            return Ok(false);
        }

        let mut promoted_bytes = 0usize;
        if state.reclaim_prepare_state.is_none() && request.plan.kind == CollectionKind::Full {
            self.heap
                .collector_handle()
                .push_phase(CollectionPhase::Remark);
            let mut phases = Vec::new();
            let roots = self.local.get_mut().roots_mut();
            let prefix = self.heap.with_flat_store_for_collection(
                |flat,
                 old_gen,
                 old_config,
                 nursery_config,
                 stats,
                 nursery,
                 _ext_scanner,
                 ext_relocator| {
                    prepare_full_reclaim_prefix_for_plan(
                        &request.plan,
                        roots,
                        &mut flat.objects,
                        &mut flat.indexes,
                        old_gen,
                        old_config,
                        nursery_config,
                        stats,
                        nursery,
                        ext_relocator,
                        |phase| phases.push(phase),
                    )
                },
            );
            match prefix {
                Ok(bytes) => promoted_bytes = bytes,
                Err(error) => {
                    self.heap
                        .collector_handle()
                        .restore_major_mark_state_and_refresh(
                            state,
                            &self.heap.planning_stats(),
                            self.heap.old_gen(),
                            self.heap.old_config(),
                            |kind| self.heap.plan_for(kind),
                        );
                    return Err(error);
                }
            }
            self.heap.collector_handle().push_phases(phases);
        }

        let completed = if state.reclaim_prepare_state.is_none() {
            let objects = self.heap.objects();
            let raw = objects.raw();
            if request.plan.kind == CollectionKind::Major {
                // Weak slots must be filtered before survivor/candidate lists are
                // snapshotted by the incremental reclaim-prep builder.
                let empty_forwarding = ForwardingMap::default();
                process_weak_references_for_candidates(
                    raw.clone(),
                    &objects.weak_candidates(),
                    request.plan.kind,
                    request.plan.worker_count.max(1),
                    &empty_forwarding,
                );
            }
            state.reclaim_prepare_state = Some(begin_active_prepared_reclaim_build(
                request.plan.kind,
                raw.all_locator_snapshot(),
                objects.finalizable_candidates(),
                objects.weak_candidates(),
                objects.ephemeron_candidates(),
                promoted_bytes,
            ));
            let build = state
                .reclaim_prepare_state
                .as_mut()
                .expect("reclaim prep build state was just initialized");
            advance_active_prepared_reclaim_build(raw, build, budget)
        } else {
            let objects = self.heap.mark_objects();
            let raw = objects.raw();
            let build = state
                .reclaim_prepare_state
                .as_mut()
                .expect("reclaim prep build state should exist");
            advance_active_prepared_reclaim_build(raw, build, budget)
        };
        let pause_nanos = elapsed_nanos(pause_start.elapsed());
        state.reclaim_prepare_nanos = state.reclaim_prepare_nanos.saturating_add(pause_nanos);
        self.heap.record_pause_sample(pause_nanos);
        if completed {
            let prepared_reclaim = finish_active_prepared_reclaim_build(
                state
                    .reclaim_prepare_state
                    .take()
                    .expect("completed reclaim prep should take build state"),
            );
            state.reclaim_prepared = true;
            state.prepared_reclaim = Some(prepared_reclaim);
            state.reclaim_commit_pause_nanos = 0;
            if request.plan.kind == CollectionKind::Major {
                self.heap
                    .collector_handle()
                    .push_phase(CollectionPhase::Remark);
            }
        }
        self.heap
            .collector_handle()
            .restore_major_mark_state_and_refresh(
                state,
                &self.heap.planning_stats(),
                self.heap.old_gen(),
                self.heap.old_config(),
                |kind| self.heap.plan_for(kind),
            );
        Ok(completed)
    }

    /// Finish the active major collection if its mark work is fully drained.
    /// This advances reclaim commit by one bounded pause slice. Large
    /// collections may therefore return `Ok(None)` until callers poll again.
    pub fn finish_active_major_collection_if_ready(
        &mut self,
    ) -> Result<Option<CollectionStats>, AllocError> {
        let snapshot = self.heap.collector_shared_snapshot();
        if snapshot.active_major_mark_plan.is_none() {
            return Ok(None);
        }
        if snapshot
            .major_mark_progress
            .is_some_and(|progress| !progress.completed)
        {
            return Ok(None);
        }
        if snapshot
            .active_major_mark_plan
            .as_ref()
            .is_some_and(|plan| plan.phase != CollectionPhase::Reclaim)
            && self.prepare_active_reclaim_if_needed()?
        {
            return Ok(None);
        }
        let snapshot = self.heap.collector_shared_snapshot();
        if snapshot.active_major_mark_plan.is_none() {
            return Ok(None);
        }
        if snapshot
            .major_mark_progress
            .is_some_and(|progress| !progress.completed)
        {
            return Ok(None);
        }
        if snapshot
            .active_major_mark_plan
            .as_ref()
            .is_some_and(|plan| plan.phase != CollectionPhase::Reclaim)
        {
            return Ok(None);
        }
        self.commit_active_reclaim_if_ready()
    }

    /// Advance the active reclaim commit by one bounded stop-the-world slice.
    pub fn advance_active_reclaim_commit(&mut self) -> Result<Option<CollectionStats>, AllocError> {
        let Some((pause_nanos, scanned_objects)) = self
            .heap
            .collector_handle()
            .active_reclaim_commit_progress()
        else {
            return Ok(None);
        };
        let budget = adaptive_pause_target_budget(
            self.heap.pacer().config().target_pause,
            scanned_objects,
            pause_nanos,
            DEFAULT_RECLAIM_COMMIT_SLICE_BUDGET,
            1,
            MAX_RECLAIM_COMMIT_SLICE_BUDGET,
        );
        self.advance_active_reclaim_commit_with_budget(budget)
    }

    /// Advance the active reclaim commit by one bounded stop-the-world slice
    /// using the provided object budget.
    pub fn advance_active_reclaim_commit_with_budget(
        &mut self,
        budget: usize,
    ) -> Result<Option<CollectionStats>, AllocError> {
        self.advance_active_reclaim_commit_impl(budget)
    }

    /// Advance one deferred post-major auto-compaction slice using
    /// the runtime's adaptive pause budget.
    pub fn advance_auto_compaction(&mut self) -> usize {
        let state = self.heap.auto_compaction_state();
        let budget = adaptive_pause_target_budget(
            self.heap.pacer().config().target_pause,
            state.compacted_bytes,
            state.pause_nanos,
            DEFAULT_AUTO_COMPACTION_SLICE_BUDGET_BYTES,
            MIN_AUTO_COMPACTION_SLICE_BUDGET_BYTES,
            MAX_AUTO_COMPACTION_SLICE_BUDGET_BYTES,
        );
        self.advance_auto_compaction_with_byte_budget(budget)
    }

    /// Advance one deferred post-major auto-compaction slice using
    /// the provided live-byte budget.
    pub fn advance_auto_compaction_with_byte_budget(&mut self, budget_bytes: usize) -> usize {
        let pause_start = Instant::now();
        let advance = {
            let roots = self.local.get_mut().roots_mut();
            self.heap.advance_auto_compaction_slice(roots, budget_bytes)
        };
        if advance.selected_bytes == 0 || advance.moved_records == 0 {
            return 0;
        }

        let pause_nanos = elapsed_nanos(pause_start.elapsed());
        self.heap.record_pause_sample(pause_nanos);
        self.heap.record_auto_compaction_slice(
            advance.selected_bytes,
            pause_nanos,
            advance.remaining,
        );
        advance.moved_records
    }

    /// Advance commit for the active major collection once reclaim has already
    /// been prepared.
    ///
    /// The default commit path is pause-bounded; callers should continue
    /// polling until it returns the completed cycle.
    pub fn commit_active_reclaim_if_ready(
        &mut self,
    ) -> Result<Option<CollectionStats>, AllocError> {
        let snapshot = self.heap.collector_shared_snapshot();
        if snapshot.active_major_mark_plan.is_none() {
            return Ok(None);
        }
        if snapshot
            .major_mark_progress
            .is_some_and(|progress| !progress.completed)
        {
            return Ok(None);
        }
        if snapshot
            .active_major_mark_plan
            .as_ref()
            .is_some_and(|plan| plan.phase != CollectionPhase::Reclaim)
        {
            return Ok(None);
        }
        self.advance_active_reclaim_commit()
    }

    fn advance_active_reclaim_commit_impl(
        &mut self,
        budget: usize,
    ) -> Result<Option<CollectionStats>, AllocError> {
        let snapshot = self.heap.collector_shared_snapshot();
        if snapshot.active_major_mark_plan.is_none() {
            return Ok(None);
        }
        if snapshot
            .major_mark_progress
            .is_some_and(|progress| !progress.completed)
        {
            return Ok(None);
        }
        let Some(active_plan) = snapshot.active_major_mark_plan else {
            return Ok(None);
        };
        if active_plan.phase != CollectionPhase::Reclaim {
            return Ok(None);
        }

        let pause_start = Instant::now();
        let Some(mut state) = self.heap.collector_handle().take_major_mark_state() else {
            return Ok(None);
        };
        let reclaim_plan = CollectionPlan {
            phase: CollectionPhase::Reclaim,
            ..state.plan.clone()
        };
        let started_commit = state.reclaim_commit_state.is_none();
        if started_commit {
            self.heap
                .collector_handle()
                .push_phase(CollectionPhase::Reclaim);
        }
        let before_bytes = match state.reclaim_before_bytes {
            Some(before_bytes) => before_bytes,
            None => {
                let before_bytes = self.heap.planning_stats().total_live_bytes();
                state.reclaim_before_bytes = Some(before_bytes);
                before_bytes
            }
        };
        let runtime_state = self.heap.runtime_state_handle();
        let runtime_state_for_callback = runtime_state.clone();
        let mut completed_cycle = None;
        self.heap
            .with_flat_store_for_reclaim_commit(|flat, old_gen, stats| {
                let prepared_reclaim = state
                    .prepared_reclaim
                    .as_ref()
                    .expect("reclaim-ready session should have prepared reclaim state");
                let commit_state = state.reclaim_commit_state.get_or_insert_with(|| {
                    begin_prepared_reclaim_commit(prepared_reclaim, &flat.objects)
                });
                let completed = advance_prepared_reclaim_commit(
                    flat.objects.as_mut_slice(),
                    &mut flat.indexes,
                    prepared_reclaim,
                    commit_state,
                    budget,
                );
                if completed {
                    let prepared_reclaim = state
                        .prepared_reclaim
                        .take()
                        .expect("completed reclaim commit should take prepared reclaim state");
                    let commit_state = state
                        .reclaim_commit_state
                        .take()
                        .expect("completed reclaim commit should take commit state");
                    completed_cycle = Some(finish_prepared_reclaim_cycle_with_state(
                        &mut flat.objects,
                        &mut flat.indexes,
                        old_gen,
                        stats,
                        &runtime_state,
                        before_bytes,
                        state.mark_steps,
                        state.mark_rounds,
                        state.mark_elapsed_nanos,
                        state.reclaim_prepare_nanos,
                        prepared_reclaim,
                        commit_state,
                        move |object| runtime_state_for_callback.enqueue_pending_finalizer(object),
                    ));
                }
            });

        if let Some(cycle) = completed_cycle {
            return Ok(Some(self.finalize_reclaim_cycle(
                cycle,
                reclaim_plan,
                state.reclaim_commit_pause_nanos,
                pause_start,
            )));
        }

        let pause_nanos = elapsed_nanos(pause_start.elapsed());
        state.reclaim_commit_pause_nanos =
            state.reclaim_commit_pause_nanos.saturating_add(pause_nanos);
        self.heap.record_pause_sample(pause_nanos);
        self.heap
            .collector_handle()
            .restore_major_mark_state_and_refresh(
                state,
                &self.heap.planning_stats(),
                self.heap.old_gen(),
                self.heap.old_config(),
                |kind| self.heap.plan_for(kind),
            );
        Ok(None)
    }

    /// Service one background collection round for the active major-mark session.
    pub fn service_background_collection_round(
        &mut self,
    ) -> Result<BackgroundCollectionStatus, AllocError> {
        if self.active_major_mark_plan().is_none() {
            if self.runtime_work_status().has_pending_auto_compaction() {
                let _ = self.advance_auto_compaction();
            }
            return Ok(BackgroundCollectionStatus::Idle);
        }

        let progress = self
            .poll_active_major_mark()?
            .expect("active major-mark session disappeared during service");
        if progress.completed {
            if let Some(cycle) = self.finish_active_major_collection_if_ready()? {
                Ok(BackgroundCollectionStatus::Finished(cycle))
            } else {
                Ok(BackgroundCollectionStatus::ReadyToFinish(progress))
            }
        } else {
            Ok(BackgroundCollectionStatus::Progress(progress))
        }
    }

    fn finalize_reclaim_cycle(
        &mut self,
        mut cycle: CollectionStats,
        completed_plan: CollectionPlan,
        prior_pause_nanos: u64,
        pause_start: Instant,
    ) -> CollectionStats {
        let current_pause_nanos = elapsed_nanos(pause_start.elapsed());
        self.heap.record_pause_sample(current_pause_nanos);
        cycle.pause_nanos = prior_pause_nanos.saturating_add(current_pause_nanos);
        self.record_completed_cycle(cycle, completed_plan.clone());
        if matches!(
            completed_plan.kind,
            CollectionKind::Major | CollectionKind::Full
        ) {
            self.heap.schedule_auto_compaction_if_enabled();
        } else {
            self.heap.clear_pending_auto_compaction();
        }
        cycle
    }

    fn record_completed_cycle(&mut self, cycle: CollectionStats, completed_plan: CollectionPlan) {
        self.heap.record_collection_stats(cycle);
        // Sync the atomic allocation counters from the
        // post-cycle HeapStats so the hot-path readers see
        // the GC-rebuilt values (apply_space_rebuild rewrites
        // all five per-space live_bytes/reserved_bytes).
        self.heap.sync_alloc_counters();
        self.heap.collector_handle().record_completed_plan(
            completed_plan,
            &self.heap.planning_stats(),
            self.heap.old_gen(),
            self.heap.old_config(),
            |kind| self.heap.plan_for(kind),
        );
    }
}

fn elapsed_nanos(duration: Duration) -> u64 {
    duration.as_nanos().min(u128::from(u64::MAX)) as u64
}

fn advance_active_reclaim_ephemerons(
    core: &HeapCore,
    state: &mut MajorMarkState,
    request: &collector_session::ActiveReclaimPrepRequest,
    budget: usize,
) -> bool {
    if state.ephemerons_processed {
        return true;
    }

    let objects = core.objects();
    let raw = objects.raw();
    let trace = state.ephemeron_trace_state.get_or_insert_with(|| {
        begin_active_major_ephemeron_trace(objects.ephemeron_candidate_snapshot())
    });
    let mark_slice_budget = request.plan.mark_slice_budget.min(budget.max(1)).max(1);
    let progress = advance_active_major_ephemeron_trace(
        raw,
        trace,
        request.plan.worker_count.max(1),
        budget,
        mark_slice_budget,
    );
    state.mark_elapsed_nanos = elapsed_nanos(state.mark_started_at.elapsed());
    state.mark_steps = state.mark_steps.saturating_add(progress.mark_steps_delta);
    state.mark_rounds = state.mark_rounds.saturating_add(progress.mark_rounds_delta);
    if progress.completed {
        state.ephemerons_processed = true;
        state.ephemeron_trace_state = None;
    }
    progress.completed
}

fn prepare_active_major_reclaim_with_read_core(
    core: &HeapCore,
    collector: &mut CollectorState,
    request: &collector_session::ActiveReclaimPrepRequest,
    budget: usize,
) -> bool {
    debug_assert_eq!(request.plan.kind, CollectionKind::Major);
    let pause_start = Instant::now();
    let Some(mut state) = collector.take_major_mark_state() else {
        return false;
    };
    if !state.worklist.is_empty() || state.reclaim_prepared {
        collector.restore_major_mark_state(state);
        refresh_cached_collector_plans(
            collector,
            &core.planning_stats(),
            core.old_gen(),
            core.old_config(),
            |kind| core.plan_for(kind),
        );
        return false;
    }

    if !advance_active_reclaim_ephemerons(core, &mut state, request, budget) {
        let pause_nanos = elapsed_nanos(pause_start.elapsed());
        state.reclaim_prepare_nanos = state.reclaim_prepare_nanos.saturating_add(pause_nanos);
        core.record_pause_sample(pause_nanos);
        collector.restore_major_mark_state(state);
        refresh_cached_collector_plans(
            collector,
            &core.planning_stats(),
            core.old_gen(),
            core.old_config(),
            |kind| core.plan_for(kind),
        );
        return false;
    }

    let completed = if state.reclaim_prepare_state.is_none() {
        let objects = core.objects();
        let raw = objects.raw();
        // Keep the shared/background major path equivalent to the local
        // runtime path: weak slots are filtered before the survivor and
        // candidate-list snapshot is built.
        let empty_forwarding = ForwardingMap::default();
        process_weak_references_for_candidates(
            raw.clone(),
            &objects.weak_candidates(),
            request.plan.kind,
            request.plan.worker_count.max(1),
            &empty_forwarding,
        );
        state.reclaim_prepare_state = Some(begin_active_prepared_reclaim_build(
            request.plan.kind,
            raw.all_locator_snapshot(),
            objects.finalizable_candidates(),
            objects.weak_candidates(),
            objects.ephemeron_candidates(),
            0,
        ));
        let build = state
            .reclaim_prepare_state
            .as_mut()
            .expect("reclaim prep build state was just initialized");
        advance_active_prepared_reclaim_build(raw, build, budget)
    } else {
        let objects = core.mark_objects();
        let raw = objects.raw();
        let build = state
            .reclaim_prepare_state
            .as_mut()
            .expect("reclaim prep build state should exist");
        advance_active_prepared_reclaim_build(raw, build, budget)
    };

    let pause_nanos = elapsed_nanos(pause_start.elapsed());
    state.reclaim_prepare_nanos = state.reclaim_prepare_nanos.saturating_add(pause_nanos);
    core.record_pause_sample(pause_nanos);
    if completed {
        let prepared_reclaim = finish_active_prepared_reclaim_build(
            state
                .reclaim_prepare_state
                .take()
                .expect("completed reclaim prep should take build state"),
        );
        state.reclaim_prepared = true;
        state.prepared_reclaim = Some(prepared_reclaim);
        state.reclaim_commit_state = None;
    }

    collector.restore_major_mark_state(state);
    refresh_cached_collector_plans(
        collector,
        &core.planning_stats(),
        core.old_gen(),
        core.old_config(),
        |kind| core.plan_for(kind),
    );
    completed
}

#[cfg(test)]
#[path = "runtime_test.rs"]
mod tests;

impl SharedCollectorRuntime {
    pub(crate) fn new(heap: SharedHeap) -> Self {
        let runtime = heap.runtime_handle();
        let collector = heap.collector_handle();
        Self {
            heap,
            runtime,
            collector,
        }
    }

    /// Return the shared heap backing this runtime.
    pub fn heap(&self) -> &SharedHeap {
        &self.heap
    }

    /// Create a shared background service loop bound to this runtime.
    pub fn background_service(&self, config: BackgroundCollectorConfig) -> SharedBackgroundService {
        SharedBackgroundService::from_runtime(self.clone(), config)
    }

    /// Spawn a worker-owned background collector thread bound to this runtime.
    pub fn spawn_background_worker(&self, config: BackgroundWorkerConfig) -> BackgroundWorker {
        BackgroundWorker::spawn(self.clone(), config)
    }

    fn map_shared_heap_error(error: SharedHeapError) -> SharedBackgroundError {
        match error {
            SharedHeapError::LockPoisoned => SharedBackgroundError::LockPoisoned,
            SharedHeapError::WouldBlock => SharedBackgroundError::WouldBlock,
        }
    }

    fn publish_collector_snapshot(
        &self,
        next_collector: CollectorSharedSnapshot,
    ) -> Result<(), SharedHeapError> {
        self.runtime.publish_collector_snapshot(next_collector)
    }

    fn with_heap_read<R>(&self, f: impl FnOnce(&HeapCore) -> R) -> Result<R, SharedHeapError> {
        let heap = self
            .heap
            .read()
            .map_err(|_| SharedHeapError::LockPoisoned)?;
        let core = heap.read_core();
        Ok(f(&core))
    }

    fn try_with_heap_read<R>(&self, f: impl FnOnce(&HeapCore) -> R) -> Result<R, SharedHeapError> {
        let heap = self.heap.try_read().map_err(|error| match error {
            std::sync::TryLockError::Poisoned(_) => SharedHeapError::LockPoisoned,
            std::sync::TryLockError::WouldBlock => SharedHeapError::WouldBlock,
        })?;
        let core = heap.read_core();
        Ok(f(&core))
    }

    fn with_heap_read_collector_update<R>(
        &self,
        f: impl FnOnce(&HeapCore, &mut CollectorState) -> Result<R, AllocError>,
    ) -> Result<Result<R, AllocError>, SharedHeapError> {
        let heap = self
            .heap
            .read()
            .map_err(|_| SharedHeapError::LockPoisoned)?;
        let core = heap.read_core();
        let result = self.collector.with_state(|collector| {
            f(&core, collector).map(|value| (value, collector.shared_snapshot()))
        })?;
        match result {
            Ok((value, collector_snapshot)) => {
                drop(core);
                drop(heap);
                self.publish_collector_snapshot(collector_snapshot)?;
                Ok(Ok(value))
            }
            Err(error) => Ok(Err(error)),
        }
    }

    fn try_with_heap_read_collector_update<R>(
        &self,
        f: impl FnOnce(&HeapCore, &mut CollectorState) -> Result<R, AllocError>,
    ) -> Result<Result<R, AllocError>, SharedHeapError> {
        let heap = self.heap.try_read().map_err(|error| match error {
            std::sync::TryLockError::Poisoned(_) => SharedHeapError::LockPoisoned,
            std::sync::TryLockError::WouldBlock => SharedHeapError::WouldBlock,
        })?;
        let core = heap.read_core();
        let result = self.collector.try_with_state(|collector| {
            f(&core, collector).map(|value| (value, collector.shared_snapshot()))
        })?;
        match result {
            Ok((value, collector_snapshot)) => {
                drop(core);
                drop(heap);
                self.publish_collector_snapshot(collector_snapshot)?;
                Ok(Ok(value))
            }
            Err(error) => Ok(Err(error)),
        }
    }

    fn with_runtime_update<R>(
        &self,
        f: impl for<'heap> FnOnce(&mut CollectorRuntime<'heap>) -> Result<R, AllocError>,
    ) -> Result<Result<R, AllocError>, SharedHeapError> {
        let heap = self
            .heap
            .lock()
            .map_err(|_| SharedHeapError::LockPoisoned)?;
        let mut guard = heap.collector_runtime();
        let mut runtime = guard.runtime();
        Ok(f(&mut runtime))
    }

    fn try_with_runtime_update<R>(
        &self,
        f: impl for<'heap> FnOnce(&mut CollectorRuntime<'heap>) -> Result<R, AllocError>,
    ) -> Result<Result<R, AllocError>, SharedHeapError> {
        let heap = self.heap.try_lock().map_err(|error| match error {
            std::sync::TryLockError::Poisoned(_) => SharedHeapError::LockPoisoned,
            std::sync::TryLockError::WouldBlock => SharedHeapError::WouldBlock,
        })?;
        let mut guard = heap.try_collector_runtime().map_err(|error| match error {
            TryCollectorRuntimeError::Poisoned => SharedHeapError::LockPoisoned,
            TryCollectorRuntimeError::WouldBlock => SharedHeapError::WouldBlock,
        })?;
        let mut runtime = guard.runtime();
        Ok(f(&mut runtime))
    }

    fn shared_reclaim_prep_budget(&self) -> Result<usize, SharedBackgroundError> {
        let Some((pause_nanos, scanned_objects)) = self
            .collector
            .active_reclaim_prep_progress()
            .map_err(Self::map_shared_heap_error)?
        else {
            return Ok(DEFAULT_RECLAIM_PREP_SLICE_BUDGET);
        };
        Ok(adaptive_pause_target_budget(
            self.heap.pacer_config().target_pause,
            scanned_objects,
            pause_nanos,
            DEFAULT_RECLAIM_PREP_SLICE_BUDGET,
            1,
            MAX_RECLAIM_PREP_SLICE_BUDGET,
        ))
    }

    fn shared_reclaim_commit_budget(&self) -> Result<usize, SharedBackgroundError> {
        let Some((pause_nanos, scanned_objects)) = self
            .collector
            .active_reclaim_commit_progress()
            .map_err(Self::map_shared_heap_error)?
        else {
            return Ok(DEFAULT_RECLAIM_COMMIT_SLICE_BUDGET);
        };
        Ok(adaptive_pause_target_budget(
            self.heap.pacer_config().target_pause,
            scanned_objects,
            pause_nanos,
            DEFAULT_RECLAIM_COMMIT_SLICE_BUDGET,
            1,
            MAX_RECLAIM_COMMIT_SLICE_BUDGET,
        ))
    }

    fn try_shared_reclaim_commit_budget(&self) -> Result<usize, SharedBackgroundError> {
        let Some((pause_nanos, scanned_objects)) = self
            .collector
            .try_active_reclaim_commit_progress()
            .map_err(Self::map_shared_heap_error)?
        else {
            return Ok(DEFAULT_RECLAIM_COMMIT_SLICE_BUDGET);
        };
        Ok(adaptive_pause_target_budget(
            self.heap.pacer_config().target_pause,
            scanned_objects,
            pause_nanos,
            DEFAULT_RECLAIM_COMMIT_SLICE_BUDGET,
            1,
            MAX_RECLAIM_COMMIT_SLICE_BUDGET,
        ))
    }

    /// Return current heap statistics.
    pub fn stats(&self) -> Result<HeapStats, SharedBackgroundError> {
        self.runtime
            .observe_heap_status()
            .map(|status| status.stats)
            .map_err(Self::map_shared_heap_error)
    }

    /// Recommend the next collection plan from the current shared snapshot.
    pub fn recommended_plan(&self) -> Result<CollectionPlan, SharedBackgroundError> {
        self.collector
            .read_snapshot(|snapshot| snapshot.recommended_plan.clone())
            .map_err(Self::map_shared_heap_error)
    }

    /// Return one consistent shared heap status snapshot for this runtime.
    pub fn status(&self) -> Result<SharedHeapStatus, SharedBackgroundError> {
        self.runtime
            .observe_heap_status()
            .map_err(Self::map_shared_heap_error)
    }

    /// Return the current shared-heap change epoch for this runtime.
    pub fn epoch(&self) -> Result<u64, SharedBackgroundError> {
        self.runtime
            .heap_epoch()
            .map_err(Self::map_shared_heap_error)
    }

    /// Wait for one shared-heap change visible to this runtime.
    pub fn wait_for_change(
        &self,
        observed_epoch: u64,
        timeout: Duration,
    ) -> Result<(u64, bool), SharedBackgroundError> {
        self.runtime
            .wait_for_heap_change(observed_epoch, timeout)
            .map_err(Self::map_shared_heap_error)
    }

    pub(crate) fn notify_waiters(&self) {
        self.runtime.notify_heap();
    }

    pub(crate) fn notify_background_waiters(&self) {
        self.collector.notify();
    }

    /// Return the number of queued finalizers waiting to run.
    pub fn pending_finalizer_count(&self) -> Result<usize, SharedBackgroundError> {
        self.runtime
            .pending_finalizer_count()
            .map_err(Self::map_shared_heap_error)
    }

    /// Return runtime-side follow-up work that remains outside GC commit.
    pub fn runtime_work_status(&self) -> Result<RuntimeWorkStatus, SharedBackgroundError> {
        self.runtime
            .runtime_work_status()
            .map_err(Self::map_shared_heap_error)
    }

    /// Advance one deferred post-major auto-compaction slice.
    pub fn advance_auto_compaction(&self) -> Result<usize, SharedBackgroundError> {
        self.with_runtime_update(|runtime| Ok(runtime.advance_auto_compaction()))
            .map_err(Self::map_shared_heap_error)?
            .map_err(SharedBackgroundError::Collection)
    }

    /// Advance one deferred post-major auto-compaction slice without
    /// blocking on heap contention.
    pub fn try_advance_auto_compaction(&self) -> Result<usize, SharedBackgroundError> {
        self.try_with_runtime_update(|runtime| Ok(runtime.advance_auto_compaction()))
            .map_err(Self::map_shared_heap_error)?
            .map_err(SharedBackgroundError::Collection)
    }

    /// Run and drain queued finalizers.
    pub fn drain_pending_finalizers(&self) -> Result<u64, SharedBackgroundError> {
        self.runtime
            .drain_pending_finalizers()
            .map_err(Self::map_shared_heap_error)
    }

    /// Run at most `max` queued finalizers and return the number
    /// that actually ran. See [`crate::heap::Heap::drain_pending_finalizers_bounded`].
    pub fn drain_pending_finalizers_bounded(
        &self,
        max: usize,
    ) -> Result<u64, SharedBackgroundError> {
        self.runtime
            .drain_pending_finalizers_bounded(max)
            .map_err(Self::map_shared_heap_error)
    }

    /// Run and drain queued finalizers without blocking on heap contention.
    pub fn try_drain_pending_finalizers(&self) -> Result<u64, SharedBackgroundError> {
        self.runtime
            .try_drain_pending_finalizers()
            .map_err(Self::map_shared_heap_error)
    }

    /// Run at most `max` queued finalizers without blocking on
    /// heap contention. See
    /// [`crate::heap::Heap::drain_pending_finalizers_bounded`] for the
    /// blocking variant's semantics.
    pub fn try_drain_pending_finalizers_bounded(
        &self,
        max: usize,
    ) -> Result<u64, SharedBackgroundError> {
        self.runtime
            .try_drain_pending_finalizers_bounded(max)
            .map_err(Self::map_shared_heap_error)
    }

    /// Recommend the next background concurrent collection plan, if any.
    pub fn recommended_background_plan(
        &self,
    ) -> Result<Option<CollectionPlan>, SharedBackgroundError> {
        self.collector
            .read_snapshot(|snapshot| snapshot.recommended_background_plan.clone())
            .map_err(Self::map_shared_heap_error)
    }

    /// Return the active major-mark plan, if one is in progress.
    pub fn active_major_mark_plan(&self) -> Result<Option<CollectionPlan>, SharedBackgroundError> {
        self.collector
            .read_snapshot(|snapshot| snapshot.active_major_mark_plan.clone())
            .map_err(Self::map_shared_heap_error)
    }

    #[cfg(test)]
    pub(crate) fn active_major_mark_has_prepared_reclaim_for_test(
        &self,
    ) -> Result<bool, SharedBackgroundError> {
        self.collector
            .with_state(|state| state.active_major_mark_has_prepared_reclaim())
            .map_err(Self::map_shared_heap_error)
    }

    /// Return progress for the active major-mark session, if any.
    pub fn major_mark_progress(&self) -> Result<Option<MajorMarkProgress>, SharedBackgroundError> {
        self.collector
            .read_snapshot(|snapshot| snapshot.major_mark_progress)
            .map_err(Self::map_shared_heap_error)
    }

    /// Return the last completed collection plan, if any.
    pub fn last_completed_plan(&self) -> Result<Option<CollectionPlan>, SharedBackgroundError> {
        self.collector
            .read_snapshot(|snapshot| snapshot.last_completed_plan.clone())
            .map_err(Self::map_shared_heap_error)
    }

    /// Return one consistent collector-visible shared snapshot.
    pub(crate) fn collector_snapshot(
        &self,
    ) -> Result<CollectorSharedSnapshot, SharedBackgroundError> {
        self.collector
            .snapshot()
            .map_err(Self::map_shared_heap_error)
    }

    pub(crate) fn collector_observation(
        &self,
    ) -> Result<(u64, CollectorSharedSnapshot), SharedBackgroundError> {
        loop {
            let before_epoch = self.background_epoch()?;
            let snapshot = self.collector_snapshot()?;
            let after_epoch = self.background_epoch()?;
            if before_epoch == after_epoch {
                return Ok((after_epoch, snapshot));
            }
        }
    }

    pub(crate) fn wait_for_collector_change(
        &self,
        observed_epoch: &mut u64,
        observed_snapshot: &mut CollectorSharedSnapshot,
        timeout: Duration,
        stop: Option<&AtomicBool>,
    ) -> Result<(bool, bool), SharedBackgroundError> {
        if timeout.is_zero() {
            return Ok((false, false));
        }

        let started_at = Instant::now();
        let mut remaining = timeout;
        let mut signal_changed = false;
        loop {
            let (next_epoch, changed) = self
                .collector
                .wait_for_change(*observed_epoch, remaining)
                .map_err(Self::map_shared_heap_error)?;
            *observed_epoch = next_epoch;
            signal_changed |= changed;

            if stop.is_some_and(|stop| stop.load(std::sync::atomic::Ordering::Acquire)) {
                return Ok((signal_changed, false));
            }

            let next_snapshot = self.collector_snapshot()?;
            if next_snapshot != *observed_snapshot {
                *observed_snapshot = next_snapshot;
                return Ok((signal_changed, true));
            }

            if changed {
                return Ok((signal_changed, false));
            }

            let elapsed = started_at.elapsed();
            if elapsed >= timeout {
                return Ok((signal_changed, false));
            }
            remaining = timeout.saturating_sub(elapsed);
        }
    }

    /// Return the current background-state change epoch for this runtime.
    pub fn background_epoch(&self) -> Result<u64, SharedBackgroundError> {
        self.collector.epoch().map_err(Self::map_shared_heap_error)
    }

    /// Return background-collector-visible shared heap state for this runtime.
    pub fn background_status(&self) -> Result<SharedBackgroundStatus, SharedBackgroundError> {
        self.runtime
            .observe_background_status()
            .map_err(Self::map_shared_heap_error)
    }

    /// Return one consistent observation of background epoch and background-visible shared heap
    /// state for this runtime.
    pub fn background_observation(
        &self,
    ) -> Result<SharedBackgroundObservation, SharedBackgroundError> {
        self.runtime
            .observe_background_status_with_epoch()
            .map(|(epoch, status)| SharedBackgroundObservation { epoch, status })
            .map_err(Self::map_shared_heap_error)
    }

    /// Wait for one background-collector-visible shared heap state change for this runtime.
    pub fn wait_for_background_change(
        &self,
        observed_epoch: u64,
        observed_status: &SharedBackgroundStatus,
        timeout: Duration,
    ) -> Result<SharedBackgroundWaitResult, SharedBackgroundError> {
        let mut observed_epoch = observed_epoch;
        let mut observed_status = observed_status.clone();
        self.runtime
            .wait_for_background_change(&mut observed_epoch, &mut observed_status, timeout, None)
            .map_err(Self::map_shared_heap_error)
    }

    /// Begin a persistent major-mark session for one scheduler-provided plan.
    pub fn begin_major_mark(&self, plan: CollectionPlan) -> Result<(), SharedBackgroundError> {
        self.with_heap_read_collector_update(|core, collector| {
            let objects = core.objects();
            let sources =
                collect_global_sources(&crate::root::RootStack::default(), &objects, None);
            collector_session::begin_major_mark(collector, objects.raw(), plan, sources)?;
            refresh_cached_collector_plans(
                collector,
                &core.planning_stats(),
                core.old_gen(),
                core.old_config(),
                |kind| core.plan_for(kind),
            );
            Ok(())
        })
        .map_err(Self::map_shared_heap_error)?
        .map_err(SharedBackgroundError::Collection)
    }

    /// Begin a persistent major-mark session without blocking on heap contention.
    pub fn try_begin_major_mark(&self, plan: CollectionPlan) -> Result<(), SharedBackgroundError> {
        self.try_with_heap_read_collector_update(|core, collector| {
            let objects = core.objects();
            let sources =
                collect_global_sources(&crate::root::RootStack::default(), &objects, None);
            collector_session::begin_major_mark(collector, objects.raw(), plan, sources)?;
            refresh_cached_collector_plans(
                collector,
                &core.planning_stats(),
                core.old_gen(),
                core.old_config(),
                |kind| core.plan_for(kind),
            );
            Ok(())
        })
        .map_err(Self::map_shared_heap_error)?
        .map_err(SharedBackgroundError::Collection)
    }

    /// Advance one scheduler-style concurrent major-mark round using the active plan worker
    /// count.
    pub fn poll_active_major_mark(
        &self,
    ) -> Result<Option<MajorMarkProgress>, SharedBackgroundError> {
        let (progress, auto_prepare_major_reclaim) = self
            .with_heap_read_collector_update(|core, collector| {
                let objects = core.mark_objects();
                let progress =
                    collector_session::poll_active_major_mark_round(collector, objects.raw())?;
                let auto_prepare_major_reclaim = progress.as_ref().is_some_and(|progress| {
                    progress.completed
                        && collector_session::active_reclaim_prep_request(collector)
                            .is_some_and(|request| request.plan.kind == CollectionKind::Major)
                });
                collector.refresh_cached_active_major_plans();
                Ok((progress, auto_prepare_major_reclaim))
            })
            .map_err(Self::map_shared_heap_error)?
            .map_err(SharedBackgroundError::Collection)?;
        if auto_prepare_major_reclaim {
            match self.try_prepare_active_reclaim_if_needed() {
                Ok(_) | Err(SharedBackgroundError::WouldBlock) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(progress)
    }

    /// Advance one scheduler-style concurrent major-mark round without blocking on heap
    /// contention.
    pub fn try_poll_active_major_mark(
        &self,
    ) -> Result<Option<MajorMarkProgress>, SharedBackgroundError> {
        let (progress, auto_prepare_major_reclaim) = self
            .try_with_heap_read_collector_update(|core, collector| {
                let objects = core.mark_objects();
                let progress =
                    collector_session::poll_active_major_mark_round(collector, objects.raw())?;
                let auto_prepare_major_reclaim = progress.as_ref().is_some_and(|progress| {
                    progress.completed
                        && collector_session::active_reclaim_prep_request(collector)
                            .is_some_and(|request| request.plan.kind == CollectionKind::Major)
                });
                collector.refresh_cached_active_major_plans();
                Ok((progress, auto_prepare_major_reclaim))
            })
            .map_err(Self::map_shared_heap_error)?
            .map_err(SharedBackgroundError::Collection)?;
        if auto_prepare_major_reclaim {
            match self.try_prepare_active_reclaim_if_needed() {
                Ok(_) | Err(SharedBackgroundError::WouldBlock) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(progress)
    }

    /// Prepare reclaim for the active major collection once mark work is fully drained.
    pub fn prepare_active_reclaim_if_needed(&self) -> Result<bool, SharedBackgroundError> {
        let snapshot = self.collector_snapshot()?;
        if snapshot.active_major_mark_plan.is_none() {
            return Ok(false);
        }
        if snapshot
            .major_mark_progress
            .is_some_and(|progress| !progress.completed)
        {
            return Ok(false);
        }
        let request = self.collector.active_reclaim_prep_request();
        let Some(request) = request else {
            return Ok(false);
        };
        if request.plan.kind == CollectionKind::Major {
            let budget = self.shared_reclaim_prep_budget()?;
            let prepared = self
                .with_heap_read_collector_update(|core, collector| {
                    Ok(prepare_active_major_reclaim_with_read_core(
                        core, collector, &request, budget,
                    ))
                })
                .map_err(Self::map_shared_heap_error)?
                .map_err(SharedBackgroundError::Collection)?;
            return Ok(prepared);
        }
        let prepared = self
            .with_runtime_update(|runtime| runtime.prepare_active_reclaim_if_needed())
            .map_err(Self::map_shared_heap_error)?
            .map_err(SharedBackgroundError::Collection)?;
        self.publish_collector_snapshot(self.collector.state_snapshot())
            .map_err(Self::map_shared_heap_error)?;
        Ok(prepared)
    }

    /// Prepare reclaim for the active major collection once mark work is fully drained, without
    /// blocking on heap contention.
    pub fn try_prepare_active_reclaim_if_needed(&self) -> Result<bool, SharedBackgroundError> {
        let snapshot = self.collector_snapshot()?;
        if snapshot.active_major_mark_plan.is_none() {
            return Ok(false);
        }
        if snapshot
            .major_mark_progress
            .is_some_and(|progress| !progress.completed)
        {
            return Ok(false);
        }
        let request = self.collector.active_reclaim_prep_request();
        let Some(request) = request else {
            return Ok(false);
        };
        if request.plan.kind == CollectionKind::Major {
            let budget = self.shared_reclaim_prep_budget()?;
            let prepared = self
                .try_with_heap_read_collector_update(|core, collector| {
                    Ok(prepare_active_major_reclaim_with_read_core(
                        core, collector, &request, budget,
                    ))
                })
                .map_err(Self::map_shared_heap_error)?
                .map_err(SharedBackgroundError::Collection)?;
            return Ok(prepared);
        }
        let prepared = self
            .try_with_runtime_update(|runtime| runtime.prepare_active_reclaim_if_needed())
            .map_err(Self::map_shared_heap_error)?
            .map_err(SharedBackgroundError::Collection)?;
        self.publish_collector_snapshot(self.collector.state_snapshot())
            .map_err(Self::map_shared_heap_error)?;
        Ok(prepared)
    }

    /// Finish the active major collection if its mark work is fully drained.
    pub fn finish_active_major_collection_if_ready(
        &self,
    ) -> Result<Option<CollectionStats>, SharedBackgroundError> {
        let snapshot = self.collector_snapshot()?;
        if snapshot.active_major_mark_plan.is_none() {
            return Ok(None);
        }
        if snapshot
            .major_mark_progress
            .is_some_and(|progress| !progress.completed)
        {
            return Ok(None);
        }
        if snapshot
            .active_major_mark_plan
            .as_ref()
            .is_some_and(|plan| {
                plan.kind == crate::plan::CollectionKind::Major
                    && plan.phase != CollectionPhase::Reclaim
            })
        {
            match self.try_prepare_active_reclaim_if_needed() {
                Ok(_) | Err(SharedBackgroundError::WouldBlock) => return Ok(None),
                Err(error) => return Err(error),
            }
        }
        match self
            .try_with_runtime_update(|runtime| runtime.finish_active_major_collection_if_ready())
        {
            Ok(result) => result.map_err(SharedBackgroundError::Collection),
            Err(SharedHeapError::WouldBlock) => Ok(None),
            Err(error) => Err(Self::map_shared_heap_error(error)),
        }
    }

    /// Advance the active reclaim commit by one bounded stop-the-world slice.
    pub fn advance_active_reclaim_commit(
        &self,
    ) -> Result<Option<CollectionStats>, SharedBackgroundError> {
        let budget = self.shared_reclaim_commit_budget()?;
        self.advance_active_reclaim_commit_with_budget(budget)
    }

    /// Advance the active reclaim commit by one bounded stop-the-world slice
    /// using the provided object budget.
    pub fn advance_active_reclaim_commit_with_budget(
        &self,
        budget: usize,
    ) -> Result<Option<CollectionStats>, SharedBackgroundError> {
        let snapshot = self.collector_snapshot()?;
        if snapshot.active_major_mark_plan.is_none() {
            return Ok(None);
        }
        if snapshot
            .major_mark_progress
            .is_some_and(|progress| !progress.completed)
        {
            return Ok(None);
        }
        if snapshot
            .active_major_mark_plan
            .as_ref()
            .is_some_and(|plan| plan.phase != CollectionPhase::Reclaim)
        {
            return Ok(None);
        }
        match self.try_with_runtime_update(|runtime| {
            runtime.advance_active_reclaim_commit_with_budget(budget)
        }) {
            Ok(result) => result.map_err(SharedBackgroundError::Collection),
            Err(SharedHeapError::WouldBlock) => Ok(None),
            Err(error) => Err(Self::map_shared_heap_error(error)),
        }
    }

    /// Advance commit for the active major collection once reclaim has already
    /// been prepared.
    ///
    /// The default commit path is pause-bounded; callers should continue
    /// polling until it returns the completed cycle.
    pub fn commit_active_reclaim_if_ready(
        &self,
    ) -> Result<Option<CollectionStats>, SharedBackgroundError> {
        let snapshot = self.collector_snapshot()?;
        if snapshot.active_major_mark_plan.is_none() {
            return Ok(None);
        }
        if snapshot
            .major_mark_progress
            .is_some_and(|progress| !progress.completed)
        {
            return Ok(None);
        }
        if snapshot
            .active_major_mark_plan
            .as_ref()
            .is_some_and(|plan| plan.phase != CollectionPhase::Reclaim)
        {
            return Ok(None);
        }
        match self.try_with_runtime_update(|runtime| runtime.commit_active_reclaim_if_ready()) {
            Ok(result) => result.map_err(SharedBackgroundError::Collection),
            Err(SharedHeapError::WouldBlock) => Ok(None),
            Err(error) => Err(Self::map_shared_heap_error(error)),
        }
    }

    /// Finish the active major collection if its mark work is fully drained, without blocking on
    /// heap contention.
    ///
    /// This advances reclaim commit by one bounded pause slice. Large
    /// collections may therefore return `Ok(None)` until callers poll again.
    pub fn try_finish_active_major_collection_if_ready(
        &self,
    ) -> Result<Option<CollectionStats>, SharedBackgroundError> {
        let snapshot = self.collector_snapshot()?;
        if snapshot.active_major_mark_plan.is_none() {
            return Ok(None);
        }
        if snapshot
            .major_mark_progress
            .is_some_and(|progress| !progress.completed)
        {
            return Ok(None);
        }
        if snapshot
            .active_major_mark_plan
            .as_ref()
            .is_some_and(|plan| {
                plan.kind == crate::plan::CollectionKind::Major
                    && plan.phase != CollectionPhase::Reclaim
            })
        {
            match self.try_prepare_active_reclaim_if_needed() {
                Ok(_) | Err(SharedBackgroundError::WouldBlock) => return Ok(None),
                Err(error) => return Err(error),
            }
        }
        self.try_with_runtime_update(|runtime| runtime.finish_active_major_collection_if_ready())
            .map_err(Self::map_shared_heap_error)?
            .map_err(SharedBackgroundError::Collection)
    }

    /// Advance the active reclaim commit by one bounded stop-the-world slice, without blocking on
    /// heap contention.
    pub fn try_advance_active_reclaim_commit(
        &self,
    ) -> Result<Option<CollectionStats>, SharedBackgroundError> {
        let budget = self.try_shared_reclaim_commit_budget()?;
        self.try_advance_active_reclaim_commit_with_budget(budget)
    }

    /// Advance the active reclaim commit by one bounded stop-the-world slice
    /// using the provided object budget, without blocking on heap contention.
    pub fn try_advance_active_reclaim_commit_with_budget(
        &self,
        budget: usize,
    ) -> Result<Option<CollectionStats>, SharedBackgroundError> {
        let snapshot = self.collector_snapshot()?;
        if snapshot.active_major_mark_plan.is_none() {
            return Ok(None);
        }
        if snapshot
            .major_mark_progress
            .is_some_and(|progress| !progress.completed)
        {
            return Ok(None);
        }
        if snapshot
            .active_major_mark_plan
            .as_ref()
            .is_some_and(|plan| plan.phase != CollectionPhase::Reclaim)
        {
            return Ok(None);
        }
        self.try_with_runtime_update(|runtime| {
            runtime.advance_active_reclaim_commit_with_budget(budget)
        })
        .map_err(Self::map_shared_heap_error)?
        .map_err(SharedBackgroundError::Collection)
    }

    /// Advance commit for the active major collection once reclaim has already
    /// been prepared, without blocking on heap contention.
    ///
    /// The default commit path is pause-bounded; callers should continue
    /// polling until it returns the completed cycle.
    pub fn try_commit_active_reclaim_if_ready(
        &self,
    ) -> Result<Option<CollectionStats>, SharedBackgroundError> {
        let snapshot = self.collector_snapshot()?;
        if snapshot.active_major_mark_plan.is_none() {
            return Ok(None);
        }
        if snapshot
            .major_mark_progress
            .is_some_and(|progress| !progress.completed)
        {
            return Ok(None);
        }
        if snapshot
            .active_major_mark_plan
            .as_ref()
            .is_some_and(|plan| plan.phase != CollectionPhase::Reclaim)
        {
            return Ok(None);
        }
        self.try_with_runtime_update(|runtime| runtime.commit_active_reclaim_if_ready())
            .map_err(Self::map_shared_heap_error)?
            .map_err(SharedBackgroundError::Collection)
    }

    /// Service one background collection round for the active major-mark session.
    pub fn service_background_collection_round(
        &self,
    ) -> Result<BackgroundCollectionStatus, SharedBackgroundError> {
        if self.active_major_mark_plan()?.is_none() {
            if self.runtime_work_status()?.has_pending_auto_compaction() {
                let _ = self.advance_auto_compaction()?;
            }
            return Ok(BackgroundCollectionStatus::Idle);
        }

        let Some(progress) = self.poll_active_major_mark()? else {
            return Ok(BackgroundCollectionStatus::Idle);
        };
        if progress.completed {
            match self.try_prepare_active_reclaim_if_needed() {
                Ok(true) => return Ok(BackgroundCollectionStatus::ReadyToFinish(progress)),
                Ok(false) | Err(SharedBackgroundError::WouldBlock) => {}
                Err(error) => return Err(error),
            }
            let commit_budget = self.try_shared_reclaim_commit_budget()?;
            match self.try_advance_active_reclaim_commit_with_budget(commit_budget) {
                Ok(Some(cycle)) => Ok(BackgroundCollectionStatus::Finished(cycle)),
                Ok(None) | Err(SharedBackgroundError::WouldBlock) => {
                    Ok(BackgroundCollectionStatus::ReadyToFinish(progress))
                }
                Err(error) => Err(error),
            }
        } else {
            Ok(BackgroundCollectionStatus::Progress(progress))
        }
    }

    /// Service one background collection round for the active major-mark session without blocking
    /// on heap contention.
    pub fn try_service_background_collection_round(
        &self,
    ) -> Result<BackgroundCollectionStatus, SharedBackgroundError> {
        if self.active_major_mark_plan()?.is_none() {
            if self.runtime_work_status()?.has_pending_auto_compaction() {
                let _ = self.try_advance_auto_compaction()?;
            }
            return Ok(BackgroundCollectionStatus::Idle);
        }

        let Some(progress) = self.try_poll_active_major_mark()? else {
            return Ok(BackgroundCollectionStatus::Idle);
        };
        if progress.completed {
            match self.try_prepare_active_reclaim_if_needed() {
                Ok(true) => Ok(BackgroundCollectionStatus::ReadyToFinish(progress)),
                Ok(false) => {
                    let commit_budget = self.try_shared_reclaim_commit_budget()?;
                    if let Some(cycle) =
                        self.try_advance_active_reclaim_commit_with_budget(commit_budget)?
                    {
                        Ok(BackgroundCollectionStatus::Finished(cycle))
                    } else {
                        Ok(BackgroundCollectionStatus::ReadyToFinish(progress))
                    }
                }
                Err(SharedBackgroundError::WouldBlock) => {
                    Ok(BackgroundCollectionStatus::ReadyToFinish(progress))
                }
                Err(error) => Err(error),
            }
        } else {
            Ok(BackgroundCollectionStatus::Progress(progress))
        }
    }
}

impl BackgroundCollectionRuntime for CollectorRuntime<'_> {
    fn active_major_mark_plan(&self) -> Option<CollectionPlan> {
        self.active_major_mark_plan()
    }

    fn recommended_background_plan(&self) -> Option<CollectionPlan> {
        self.recommended_background_plan()
    }

    fn begin_major_mark(&mut self, plan: CollectionPlan) -> Result<(), AllocError> {
        self.begin_major_mark(plan)
    }

    fn poll_background_mark_round(&mut self) -> Result<Option<MajorMarkProgress>, AllocError> {
        self.poll_active_major_mark()
    }

    fn prepare_active_reclaim_if_needed(&mut self) -> Result<bool, AllocError> {
        self.prepare_active_reclaim_if_needed()
    }

    fn advance_active_reclaim_commit(&mut self) -> Result<Option<CollectionStats>, AllocError> {
        self.advance_active_reclaim_commit()
    }

    fn finish_active_major_collection_if_ready(
        &mut self,
    ) -> Result<Option<CollectionStats>, AllocError> {
        self.finish_active_major_collection_if_ready()
    }

    fn commit_active_reclaim_if_ready(&mut self) -> Result<Option<CollectionStats>, AllocError> {
        self.commit_active_reclaim_if_ready()
    }

    fn runtime_work_status(&self) -> RuntimeWorkStatus {
        self.runtime_work_status()
    }

    fn advance_auto_compaction(&mut self) -> usize {
        self.advance_auto_compaction()
    }
}

impl BackgroundCollectionRuntime for SharedCollectorRuntime {
    fn active_major_mark_plan(&self) -> Option<CollectionPlan> {
        SharedCollectorRuntime::active_major_mark_plan(self)
            .expect("shared collector runtime should not be poisoned")
    }

    fn recommended_background_plan(&self) -> Option<CollectionPlan> {
        SharedCollectorRuntime::recommended_background_plan(self)
            .expect("shared collector runtime should not be poisoned")
    }

    fn begin_major_mark(&mut self, plan: CollectionPlan) -> Result<(), AllocError> {
        SharedCollectorRuntime::begin_major_mark(self, plan).map_err(|error| match error {
            SharedBackgroundError::LockPoisoned | SharedBackgroundError::WouldBlock => {
                AllocError::CollectionInProgress
            }
            SharedBackgroundError::Collection(error) => error,
        })
    }

    fn poll_background_mark_round(&mut self) -> Result<Option<MajorMarkProgress>, AllocError> {
        SharedCollectorRuntime::poll_active_major_mark(self).map_err(|error| match error {
            SharedBackgroundError::LockPoisoned | SharedBackgroundError::WouldBlock => {
                AllocError::CollectionInProgress
            }
            SharedBackgroundError::Collection(error) => error,
        })
    }

    fn prepare_active_reclaim_if_needed(&mut self) -> Result<bool, AllocError> {
        SharedCollectorRuntime::prepare_active_reclaim_if_needed(self).map_err(
            |error| match error {
                SharedBackgroundError::LockPoisoned | SharedBackgroundError::WouldBlock => {
                    AllocError::CollectionInProgress
                }
                SharedBackgroundError::Collection(error) => error,
            },
        )
    }

    fn advance_active_reclaim_commit(&mut self) -> Result<Option<CollectionStats>, AllocError> {
        SharedCollectorRuntime::advance_active_reclaim_commit(self).map_err(|error| match error {
            SharedBackgroundError::LockPoisoned | SharedBackgroundError::WouldBlock => {
                AllocError::CollectionInProgress
            }
            SharedBackgroundError::Collection(error) => error,
        })
    }

    fn finish_active_major_collection_if_ready(
        &mut self,
    ) -> Result<Option<CollectionStats>, AllocError> {
        SharedCollectorRuntime::finish_active_major_collection_if_ready(self).map_err(|error| {
            match error {
                SharedBackgroundError::LockPoisoned | SharedBackgroundError::WouldBlock => {
                    AllocError::CollectionInProgress
                }
                SharedBackgroundError::Collection(error) => error,
            }
        })
    }

    fn commit_active_reclaim_if_ready(&mut self) -> Result<Option<CollectionStats>, AllocError> {
        SharedCollectorRuntime::commit_active_reclaim_if_ready(self).map_err(|error| match error {
            SharedBackgroundError::LockPoisoned | SharedBackgroundError::WouldBlock => {
                AllocError::CollectionInProgress
            }
            SharedBackgroundError::Collection(error) => error,
        })
    }

    fn runtime_work_status(&self) -> RuntimeWorkStatus {
        SharedCollectorRuntime::runtime_work_status(self)
            .expect("shared collector runtime should not be poisoned")
    }

    fn advance_auto_compaction(&mut self) -> usize {
        SharedCollectorRuntime::advance_auto_compaction(self)
            .expect("shared collector runtime should not be poisoned")
    }
}
