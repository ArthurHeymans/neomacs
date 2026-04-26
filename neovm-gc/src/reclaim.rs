use crate::descriptor::ObjectKey;
#[cfg(test)]
use crate::heap::AllocError;
use crate::index_state::{
    ForwardingMap, HeapIndexState, ObjectIndex, ObjectKeyBuildHasher, ObjectLocator,
    PreparedIndexReclaim,
};
use crate::object::{ObjectRecord, OldBlockPlacement, PendingFinalizer, SpaceKind};
use crate::object_store::ObjectReadRaw;
use crate::plan::{CollectionKind, CollectionPlan};
use crate::runtime_state::RuntimeStateHandle;
use crate::spaces::{
    OldBlock, OldGenConfig, OldGenState, OldRegionCollectionStats, PreparedOldGenReclaim,
};
use crate::stats::{CollectionStats, HeapStats, PreparedHeapStats};
use std::collections::HashSet;

/// Physical old-gen compaction helper (physical-compaction step 3).
///
/// Walks `objects` and computes the sum of `total_size` for every
/// live block-backed object, grouped by `OldBlockPlacement::block_index`.
/// Only objects that a previous pass has identified as survivors
/// (i.e. still present in the slice after mark processing) are
/// counted. The returned vector has one entry per block in
/// `blocks`; entries for blocks that hold no surviving records
/// stay at zero.
pub(crate) fn compute_per_block_live_bytes(
    objects: &[ObjectRecord],
    block_count: usize,
) -> Vec<usize> {
    let mut live_by_block = vec![0usize; block_count];
    for object in objects {
        if object.space() != SpaceKind::Old {
            continue;
        }
        let Some(placement) = object.old_block_placement() else {
            continue;
        };
        if let Some(slot) = live_by_block.get_mut(placement.block_index) {
            *slot = slot.saturating_add(object.total_size());
        }
    }
    live_by_block
}

/// Physical old-gen compaction helper (physical-compaction step 3).
///
/// Identify the indices of blocks whose current live-byte density
/// falls at or below `density_threshold` relative to their
/// capacity. These are the candidates the compaction pass will
/// evacuate from: copying their survivors into fresh target
/// blocks leaves the source blocks empty so the existing
/// block-reclaim path can drop them.
///
/// `density_threshold` is in the range `[0.0, 1.0]`. A value of
/// `0.3` means "blocks with 30% or less live fill are candidates."
/// Blocks that are empty are excluded (nothing to evacuate).
///
/// The returned vec is sorted by *ascending density*: the
/// emptiest blocks come first. This gives the compaction loop
/// the best-bang-for-buck ordering — moving a single survivor
/// out of a 1%-full block reclaims more space than moving it
/// out of a 50%-full block, and the compaction target packing
/// works best when we evacuate the most-wasted blocks first.
pub(crate) fn find_sparse_old_block_candidates(
    live_by_block: &[usize],
    blocks: &[OldBlock],
    density_threshold: f64,
) -> Vec<usize> {
    let mut candidates: Vec<(usize, f64)> = Vec::new();
    for (index, block) in blocks.iter().enumerate() {
        let live = live_by_block.get(index).copied().unwrap_or(0);
        if live == 0 {
            continue; // empty blocks get dropped by drop_unused_blocks_with_remap.
        }
        let capacity = block.capacity_bytes();
        if capacity == 0 {
            continue;
        }
        let density = (live as f64) / (capacity as f64);
        if density <= density_threshold {
            candidates.push((index, density));
        }
    }
    // Sort by ascending density so the emptiest blocks come
    // first. Density values come from (live_bytes / capacity)
    // and are bounded in [0.0, 1.0]; partial_cmp is safe here
    // because the inputs are real, finite, non-NaN.
    candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    candidates.into_iter().map(|(index, _)| index).collect()
}

/// Physical old-gen compaction pass (physical-compaction step 4).
///
/// Walks `objects`, identifies sparse OldBlock candidates whose
/// live density is at or below `density_threshold`, evacuates
/// every surviving record in those blocks into freshly-created
/// target blocks via [`evacuate_old_object_to_fresh_block`], and
/// replaces the source records in `objects` with the evacuated
/// ones. Returns a [`ForwardingMap`] of
/// `(old_object_key, new_GcErased)` entries that a subsequent
/// relocation pass can feed through [`ForwardingRelocator`] to
/// rewrite any inbound reference.
///
/// This function does NOT touch the source block contents: the
/// source blocks become empty (no surviving records point into
/// them), and the existing
/// [`rebuild_line_marks_and_reclaim_empty_old_blocks`] pass drops
/// them as part of the post-sweep rebuild.
///
/// If no blocks qualify as sparse, the function returns an empty
/// forwarding map and leaves `objects` untouched. Allocation
/// failures during evacuation are treated per-object: the
/// failing record is left in place, the forwarding map omits it,
/// and the pass moves on. Callers get a best-effort compaction.
pub(crate) fn compact_sparse_old_blocks(
    objects: &mut [ObjectRecord],
    old_gen: &mut OldGenState,
    config: &OldGenConfig,
    density_threshold: f64,
) -> ForwardingMap {
    if objects.is_empty() || old_gen.block_count() == 0 {
        return ForwardingMap::default();
    }

    // Phase A: compute per-block live_bytes from the post-mark
    // object slice and pick sparse candidates.
    let live_by_block = compute_per_block_live_bytes(objects, old_gen.block_count());
    let candidates =
        find_sparse_old_block_candidates(&live_by_block, old_gen.blocks(), density_threshold);
    if candidates.is_empty() {
        return ForwardingMap::default();
    }
    let candidate_set: std::collections::HashSet<usize> = candidates.into_iter().collect();
    compact_specific_old_blocks(objects, old_gen, config, &candidate_set)
}

/// Physical old-gen compaction pass that operates on a
/// caller-supplied set of block indices instead of computing the
/// set via density-thresholding.
///
/// Used by the future block-indexed manual-plan path: callers
/// pass the exact block indices they want compacted, the
/// function evacuates every surviving record in those blocks
/// into freshly-created target blocks, and the existing
/// post-compact rebuild drops the now-empty source blocks.
///
/// Returns the same `ForwardingMap` shape as
/// [`compact_sparse_old_blocks`]; pass it to a relocator pass to
/// rewrite inbound references.
pub(crate) fn compact_specific_old_blocks(
    objects: &mut [ObjectRecord],
    old_gen: &mut OldGenState,
    config: &OldGenConfig,
    candidate_set: &std::collections::HashSet<usize>,
) -> ForwardingMap {
    let mut forwarding = ForwardingMap::default();
    if objects.is_empty() || old_gen.block_count() == 0 || candidate_set.is_empty() {
        return forwarding;
    }

    // Phase B: walk every record; for each one whose block is a
    // candidate, evacuate it into a compaction target block and
    // swap the record in place. Multiple survivors share the
    // same target block until it fills up, at which point
    // alloc_for_compaction_into_target rolls over to a fresh
    // one. This packs survivors tight instead of creating one
    // new block per evacuated record.
    let mut target_hint: Option<usize> = None;
    let total = objects.len();
    #[allow(clippy::needless_range_loop)]
    for slot_index in 0..total {
        let (is_candidate, object_key) = {
            let object = &objects[slot_index];
            if object.space() != SpaceKind::Old {
                (false, None)
            } else {
                match object.old_block_placement() {
                    Some(placement) if candidate_set.contains(&placement.block_index) => {
                        (true, Some(object.object_key()))
                    }
                    _ => (false, None),
                }
            }
        };
        if !is_candidate {
            continue;
        }
        let Some(object_key) = object_key else {
            continue;
        };
        // Derive the allocation layout from the source record.
        let layout = {
            let source = &objects[slot_index];
            match core::alloc::Layout::from_size_align(source.total_size(), source.layout_align()) {
                Ok(l) => l,
                Err(_) => continue,
            }
        };
        // Allocate the target slot via the compaction-aware
        // allocator: prefer the current target block, fall
        // forward to a fresh block when the current one is full.
        let Some((placement, base, new_target)) =
            old_gen.alloc_for_compaction_into_target(config, layout, target_hint)
        else {
            continue;
        };
        target_hint = Some(new_target);
        // Copy the payload and install the forwarding pointer.
        let evacuated = {
            let source = &objects[slot_index];
            // SAFETY: `alloc_for_compaction_into_target` returned
            // a freshly-reserved slot in a non-source block
            // backed by a buffer owned by the pool. The layout
            // matches what evacuate_to_arena_slot expects.
            let mut record = match unsafe { source.evacuate_to_arena_slot(SpaceKind::Old, base) } {
                Ok(r) => r,
                Err(_) => continue,
            };
            record.set_old_block_placement(placement);
            record
        };
        let new_erased = evacuated.erased();
        // Replace the source record in place with the evacuated
        // one. The source block's bytes remain in the pool buffer
        // but no record now references them; the post-compact
        // rebuild will drop the source block because none of
        // its lines are marked any more.
        objects[slot_index] = evacuated;
        forwarding.insert(object_key, new_erased);
    }

    forwarding
}

/// Physical old-gen compaction helper (physical-compaction step 2).
///
/// Copies the payload of `source` into a freshly-created OldBlock
/// target slot and returns a new `ObjectRecord` owning that slot.
/// `evacuate_to_arena_slot` installs a forwarding pointer on the
/// source header as a side effect, so a later relocation pass can
/// rewrite every inbound reference.
///
/// The target slot is allocated via
/// [`OldGenState::alloc_in_fresh_block`] to guarantee the target
/// block is NOT one of the sparse source blocks we are evacuating
/// from — a fresh block had zero live bytes before this call, so it
/// cannot collide with any source placement.
///
/// Returns `Err(AllocError)` if the target allocation or the
/// payload layout cannot be satisfied. Callers should treat such
/// failures as "skip this evacuation, leave the source record in
/// place" rather than aborting the whole reclaim cycle.
///
/// Currently consumed only by the unit test that exercises the
/// single-record evacuation primitive in isolation. The
/// production compaction loop in `compact_sparse_old_blocks`
/// inlines `alloc_for_compaction_into_target` plus
/// `evacuate_to_arena_slot` directly so it can share a packed
/// target block across multiple survivors. Kept here as a
/// reference implementation for the simpler one-shot path.
#[cfg(test)]
pub(crate) fn evacuate_old_object_to_fresh_block(
    old_gen: &mut OldGenState,
    config: &OldGenConfig,
    source: &ObjectRecord,
) -> Result<ObjectRecord, AllocError> {
    let total_size = source.total_size();
    let align = source.layout_align();
    let layout = core::alloc::Layout::from_size_align(total_size, align)
        .map_err(|_| AllocError::LayoutOverflow)?;
    let (placement, base) = old_gen
        .alloc_in_fresh_block(config, layout)
        .ok_or(AllocError::LayoutOverflow)?;
    // SAFETY: `alloc_in_fresh_block` returned a freshly-created
    // slot in a brand-new OldBlock whose backing buffer outlives
    // the pool. The layout matches what evacuate_to_arena_slot
    // expects. The source record's payload remains live until we
    // replace it in the objects vec.
    let mut evacuated = unsafe { source.evacuate_to_arena_slot(SpaceKind::Old, base)? };
    evacuated.set_old_block_placement(placement);
    Ok(evacuated)
}

#[derive(Debug)]
pub(crate) struct PreparedReclaimSurvivor {
    /// Original index in `Heap::objects` before reclaim commit.
    pub(crate) object_index: usize,
}

#[derive(Debug)]
pub(crate) struct PreparedReclaim {
    /// Object count when reclaim preparation ran. Objects allocated after this
    /// snapshot are outside the current collection and must survive commit.
    pub(crate) prepared_object_count: usize,
    pub(crate) promoted_bytes: usize,
    pub(crate) old_gen: PreparedOldGenReclaim,
    /// Per-subsystem reclaim state assembled under `HeapIndexState`.
    /// This is the single source of truth for finalize_indices and the
    /// rebuilt candidate lists — `commit_prepared_reclaim_objects` reads
    /// `indexes.finalize_indices` directly rather than duplicating it at
    /// the top level.
    pub(crate) indexes: PreparedIndexReclaim,
    /// Survivors in ascending original `object_index` order.
    ///
    /// `commit_prepared_reclaim_objects` drains this in lockstep with the original
    /// `objects` vector, so ordering is part of the prepared-state contract.
    pub(crate) survivors: Vec<PreparedReclaimSurvivor>,
    pub(crate) stats: PreparedHeapStats,
}

#[derive(Debug)]
pub(crate) struct PreparedReclaimBuildState {
    kind: CollectionKind,
    prepared_object_count: usize,
    promoted_bytes: usize,
    scan_index: usize,
    locators: Vec<ObjectLocator>,
    finalizable_candidate_set: HashSet<ObjectKey, ObjectKeyBuildHasher>,
    weak_candidate_set: HashSet<ObjectKey, ObjectKeyBuildHasher>,
    ephemeron_candidate_set: HashSet<ObjectKey, ObjectKeyBuildHasher>,
    rebuilt_object_index: ObjectIndex,
    finalize_indices: Vec<usize>,
    finalizable_candidates: Vec<ObjectKey>,
    weak_candidates: Vec<ObjectKey>,
    ephemeron_candidates: Vec<ObjectKey>,
    survivors: Vec<PreparedReclaimSurvivor>,
    stats: PreparedHeapStats,
}

impl PreparedReclaimBuildState {
    pub(crate) fn scanned_objects(&self) -> usize {
        self.scan_index
    }
}

#[derive(Debug)]
pub(crate) struct PreparedReclaimCommitState {
    scan_index: usize,
    survivor_cursor: usize,
    finalize_cursor: usize,
    survivor_write_index: usize,
    finalize_keys: std::collections::HashSet<ObjectKey, ObjectKeyBuildHasher>,
}

impl PreparedReclaimCommitState {
    pub(crate) fn scanned_objects(&self) -> usize {
        self.scan_index
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MinorRebuildResult {
    pub(crate) queued_finalizers: u64,
    pub(crate) old_region_stats: OldRegionCollectionStats,
    pub(crate) after_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ReclaimCommitResult {
    pub(crate) queued_finalizers: u64,
    pub(crate) old_region_stats: OldRegionCollectionStats,
    pub(crate) after_bytes: usize,
}

pub(crate) fn prepare_reclaim(
    objects: &[ObjectRecord],
    indexes: &HeapIndexState,
    old_gen: &OldGenState,
    old_config: &OldGenConfig,
    kind: CollectionKind,
    plan: &CollectionPlan,
) -> PreparedReclaim {
    let _ = (old_gen, old_config, plan);
    let mut survivors = Vec::new();
    let mut prepared_stats = PreparedHeapStats::default();

    for (object_index, object) in objects.iter().enumerate() {
        if !keep_object_for_collection(kind, object) {
            continue;
        }
        let total_size = object.total_size();
        // Logical-region renumbering is retired; physical
        // compaction (Heap::compact_old_gen_blocks /
        // compact_old_gen_physical) is the only mechanism that
        // moves bytes, and that updates old_block_placement
        // directly.
        survivors.push(PreparedReclaimSurvivor { object_index });
        prepared_stats.record_live_object(object.space(), total_size);
    }

    let prepared_indexes = indexes.prepare_reclaim_state(objects, &survivors, kind);
    PreparedReclaim {
        prepared_object_count: objects.len(),
        promoted_bytes: 0,
        old_gen: PreparedOldGenReclaim::default(),
        indexes: prepared_indexes,
        survivors,
        stats: prepared_stats,
    }
}

pub(crate) fn begin_active_prepared_reclaim_build(
    kind: CollectionKind,
    locators: Vec<ObjectLocator>,
    finalizable_candidates: Vec<ObjectKey>,
    weak_candidates: Vec<ObjectKey>,
    ephemeron_candidates: Vec<ObjectKey>,
    promoted_bytes: usize,
) -> PreparedReclaimBuildState {
    let prepared_object_count = locators.len();
    PreparedReclaimBuildState {
        kind,
        prepared_object_count,
        promoted_bytes,
        scan_index: 0,
        locators,
        finalizable_candidate_set: finalizable_candidates
            .into_iter()
            .collect::<HashSet<_, ObjectKeyBuildHasher>>(),
        weak_candidate_set: weak_candidates
            .into_iter()
            .collect::<HashSet<_, ObjectKeyBuildHasher>>(),
        ephemeron_candidate_set: ephemeron_candidates
            .into_iter()
            .collect::<HashSet<_, ObjectKeyBuildHasher>>(),
        rebuilt_object_index: ObjectIndex::with_capacity_and_hasher(
            prepared_object_count,
            ObjectKeyBuildHasher,
        ),
        finalize_indices: Vec::new(),
        finalizable_candidates: Vec::new(),
        weak_candidates: Vec::new(),
        ephemeron_candidates: Vec::new(),
        survivors: Vec::new(),
        stats: PreparedHeapStats::default(),
    }
}

pub(crate) fn advance_active_prepared_reclaim_build(
    objects: ObjectReadRaw<'_>,
    build: &mut PreparedReclaimBuildState,
    budget: usize,
) -> bool {
    let target_scan_index = build
        .prepared_object_count
        .min(build.scan_index.saturating_add(budget.max(1)));
    while build.scan_index < target_scan_index {
        let object_index = build.scan_index;
        let object = objects.get(build.locators[object_index]);
        let object_key = object.object_key();
        if keep_object_for_collection(build.kind, object) {
            let rebuilt_index = build.survivors.len();
            build
                .rebuilt_object_index
                .insert(object_key, ObjectLocator::flat(rebuilt_index));
            build
                .survivors
                .push(PreparedReclaimSurvivor { object_index });
            build
                .stats
                .record_live_object(object.space(), object.total_size());
            if build.finalizable_candidate_set.contains(&object_key) {
                build.finalizable_candidates.push(object_key);
            }
            if build.weak_candidate_set.contains(&object_key) {
                build.weak_candidates.push(object_key);
            }
            if build.ephemeron_candidate_set.contains(&object_key) {
                build.ephemeron_candidates.push(object_key);
            }
        } else if build.finalizable_candidate_set.contains(&object_key)
            && !object.header().is_moved_out()
        {
            build.finalize_indices.push(object_index);
        }
        build.scan_index = build.scan_index.saturating_add(1);
    }
    build.scan_index >= build.prepared_object_count
}

pub(crate) fn finish_active_prepared_reclaim_build(
    build: PreparedReclaimBuildState,
) -> PreparedReclaim {
    let PreparedReclaimBuildState {
        prepared_object_count,
        promoted_bytes,
        rebuilt_object_index,
        finalize_indices,
        finalizable_candidates,
        weak_candidates,
        ephemeron_candidates,
        survivors,
        stats,
        ..
    } = build;
    PreparedReclaim {
        prepared_object_count,
        promoted_bytes,
        old_gen: PreparedOldGenReclaim::default(),
        indexes: PreparedIndexReclaim {
            rebuilt_object_index,
            finalize_indices,
            finalizable_candidates,
            weak_candidates,
            ephemeron_candidates,
            remembered_owners: Vec::new(),
        },
        survivors,
        stats,
    }
}

pub(crate) fn prepare_major_reclaim(
    plan: &CollectionPlan,
    process_weak_references: impl FnOnce(&CollectionPlan),
    prepare_reclaim: impl FnOnce(&CollectionPlan) -> PreparedReclaim,
) -> PreparedReclaim {
    process_weak_references(plan);
    prepare_reclaim(plan)
}

#[cfg(test)]
pub(crate) fn prepare_full_reclaim<Heap, Forwarding>(
    heap: &mut Heap,
    plan: &CollectionPlan,
    evacuate_marked_nursery: impl FnOnce(&mut Heap) -> Result<(Forwarding, usize), AllocError>,
    relocate_roots_and_edges: impl FnOnce(&mut Heap, &Forwarding),
    process_weak_references: impl FnOnce(&mut Heap, &CollectionPlan, &Forwarding),
    prepare_reclaim: impl FnOnce(&Heap, &CollectionPlan) -> PreparedReclaim,
) -> Result<PreparedReclaim, AllocError> {
    let (forwarding, promoted_bytes) = evacuate_marked_nursery(heap)?;
    relocate_roots_and_edges(heap, &forwarding);
    process_weak_references(heap, plan, &forwarding);
    Ok(PreparedReclaim {
        promoted_bytes,
        ..prepare_reclaim(heap, plan)
    })
}

pub(crate) fn begin_prepared_reclaim_commit(
    prepared_reclaim: &PreparedReclaim,
    objects: &[ObjectRecord],
) -> PreparedReclaimCommitState {
    let prepared_object_count = prepared_reclaim.prepared_object_count.min(objects.len());
    let finalize_keys = prepared_reclaim
        .indexes
        .finalize_indices
        .iter()
        .copied()
        .filter(|&index| index < prepared_object_count)
        .map(|index| objects[index].object_key())
        .collect();
    PreparedReclaimCommitState {
        scan_index: 0,
        survivor_cursor: 0,
        finalize_cursor: 0,
        survivor_write_index: 0,
        finalize_keys,
    }
}

pub(crate) fn advance_prepared_reclaim_commit(
    objects: &mut [ObjectRecord],
    indexes: &mut HeapIndexState,
    prepared_reclaim: &PreparedReclaim,
    commit_state: &mut PreparedReclaimCommitState,
    budget: usize,
) -> bool {
    debug_assert!(
        prepared_reclaim
            .survivors
            .windows(2)
            .all(|window| window[0].object_index < window[1].object_index),
        "prepared reclaim survivors must stay sorted by original object index"
    );
    debug_assert!(
        prepared_reclaim
            .indexes
            .finalize_indices
            .windows(2)
            .all(|window| window[0] < window[1]),
        "prepared reclaim finalizer indices must stay sorted by original object index"
    );

    let prepared_object_count = prepared_reclaim.prepared_object_count.min(objects.len());
    let target_scan_index =
        prepared_object_count.min(commit_state.scan_index.saturating_add(budget.max(1)));
    while commit_state.scan_index < target_scan_index {
        let current_index = commit_state.scan_index;
        let should_finalize = prepared_reclaim
            .indexes
            .finalize_indices
            .get(commit_state.finalize_cursor)
            .is_some_and(|&pending_index| pending_index == current_index);
        let is_survivor = prepared_reclaim
            .survivors
            .get(commit_state.survivor_cursor)
            .is_some_and(|survivor| survivor.object_index == current_index);

        debug_assert!(
            !(should_finalize && is_survivor),
            "prepared reclaim object cannot be both live and queued for finalization"
        );

        if should_finalize {
            let object_key = objects[current_index].object_key();
            indexes.object_index.remove(&object_key);
            commit_state.finalize_cursor = commit_state.finalize_cursor.saturating_add(1);
        } else if is_survivor {
            if commit_state.survivor_write_index != current_index {
                let object_key = objects[current_index].object_key();
                objects.swap(commit_state.survivor_write_index, current_index);
                indexes.object_index.insert(
                    object_key,
                    ObjectLocator::flat(commit_state.survivor_write_index),
                );
            }
            commit_state.survivor_cursor = commit_state.survivor_cursor.saturating_add(1);
            commit_state.survivor_write_index = commit_state.survivor_write_index.saturating_add(1);
        } else {
            let object_key = objects[current_index].object_key();
            indexes.object_index.remove(&object_key);
        }
        commit_state.scan_index = commit_state.scan_index.saturating_add(1);
    }

    commit_state.scan_index >= prepared_object_count
}

/// Rebuild old-block line marks from currently surviving records and pending
/// finalizers, then drop blocks whose line marks are entirely empty. Surviving
/// records' `OldBlockPlacement::block_index` values are rebound through the
/// new block index map after empty blocks are dropped.
pub(crate) fn rebuild_line_marks_and_reclaim_empty_old_blocks(
    objects: &mut [ObjectRecord],
    old_gen: &mut OldGenState,
    runtime_state: &RuntimeStateHandle,
) -> usize {
    // Snapshot pending finalizer placements first so blocks they pin stay
    // marked even though their owning records are no longer in `objects`.
    let pending_placements = runtime_state.snapshot_pending_finalizer_block_placements();
    old_gen.clear_all_block_line_marks();
    // Rebuild the per-card object-start index from surviving
    // block-backed records so the next minor cycle's dirty-card
    // root scan can iterate dirty cards in O(dirty_cards) instead
    // of doing a linear pass over every record per dirty card.
    old_gen.clear_all_block_object_starts();
    // Reset per-block live_bytes / object_count / occupied_lines
    // so the survivor walk below can re-populate them. Without
    // this the counters stay at their pre-sweep monotonic values
    // and over-report live bytes.
    old_gen.clear_all_block_live_accounting();
    for object in objects.iter() {
        if let Some(placement) = object.old_block_placement() {
            old_gen.mark_block_lines_for_placement(placement);
            old_gen.record_block_object_start_for_placement(placement);
            old_gen.record_block_object_accounting_for_placement(placement);
        }
    }
    for placement in &pending_placements {
        old_gen.mark_block_lines_for_placement(*placement);
        old_gen.record_block_object_start_for_placement(*placement);
        old_gen.record_block_object_accounting_for_placement(*placement);
    }

    let remap = old_gen.drop_unused_blocks_with_remap();
    let dropped = remap.iter().filter(|entry| entry.is_none()).count();
    if dropped == 0 {
        return 0;
    }

    // Apply remap to surviving records.
    for object in objects.iter_mut() {
        let Some(placement) = object.old_block_placement() else {
            continue;
        };
        let Some(&Some(new_index)) = remap.get(placement.block_index) else {
            // Should not happen for live records, but stay defensive.
            continue;
        };
        if new_index != placement.block_index {
            object.set_old_block_placement(OldBlockPlacement {
                block_index: new_index,
                ..placement
            });
        }
    }
    runtime_state.rebind_pending_finalizer_block_indices(&remap);
    dropped
}

fn compute_rebuilt_heap_stats(objects: &[ObjectRecord]) -> PreparedHeapStats {
    let mut rebuilt_stats = PreparedHeapStats::default();
    for object in objects {
        rebuilt_stats.record_live_object(object.space(), object.total_size());
    }
    rebuilt_stats
}

fn finish_prepared_reclaim_commit(
    objects: &mut Vec<ObjectRecord>,
    indexes: &mut HeapIndexState,
    old_gen: &mut OldGenState,
    stats: &mut HeapStats,
    runtime_state: &RuntimeStateHandle,
    prepared_reclaim: PreparedReclaim,
    mut commit_state: PreparedReclaimCommitState,
    mut enqueue_pending_finalizer: impl FnMut(PendingFinalizer) -> u64,
) -> ReclaimCommitResult {
    let _ = advance_prepared_reclaim_commit(
        objects.as_mut_slice(),
        indexes,
        &prepared_reclaim,
        &mut commit_state,
        usize::MAX,
    );

    let prepared_object_count = prepared_reclaim.prepared_object_count.min(objects.len());
    let mut queued_finalizers = 0u64;
    for object in objects.drain(commit_state.survivor_write_index..prepared_object_count) {
        if commit_state.finalize_keys.contains(&object.object_key()) {
            queued_finalizers = queued_finalizers
                .saturating_add(enqueue_pending_finalizer(PendingFinalizer::new(object)));
        }
    }
    for object in objects.iter() {
        object.clear_mark();
    }

    let PreparedReclaim {
        old_gen: prepared_old_gen,
        ..
    } = prepared_reclaim;
    let mut old_region_stats = old_gen.apply_prepared_reclaim(prepared_old_gen);
    // Rebuild every index structure from the committed survivor set so
    // post-prepare allocations remain visible after the reclaim snapshot
    // closes.
    indexes.rebuild_from_objects(objects);
    let dropped_blocks =
        rebuild_line_marks_and_reclaim_empty_old_blocks(objects, old_gen, runtime_state);
    old_region_stats.reclaimed_regions = old_region_stats
        .reclaimed_regions
        .saturating_add(dropped_blocks as u64);
    let after_bytes =
        compute_rebuilt_heap_stats(objects).apply_space_rebuild(stats, old_gen.reserved_bytes());
    ReclaimCommitResult {
        queued_finalizers,
        old_region_stats,
        after_bytes,
    }
}

pub(crate) fn apply_prepared_reclaim(
    objects: &mut Vec<ObjectRecord>,
    indexes: &mut HeapIndexState,
    old_gen: &mut OldGenState,
    stats: &mut HeapStats,
    runtime_state: &RuntimeStateHandle,
    prepared_reclaim: PreparedReclaim,
    enqueue_pending_finalizer: impl FnMut(PendingFinalizer) -> u64,
) -> ReclaimCommitResult {
    let commit_state = begin_prepared_reclaim_commit(&prepared_reclaim, objects);
    finish_prepared_reclaim_commit(
        objects,
        indexes,
        old_gen,
        stats,
        runtime_state,
        prepared_reclaim,
        commit_state,
        enqueue_pending_finalizer,
    )
}

pub(crate) fn finish_prepared_reclaim_cycle(
    objects: &mut Vec<ObjectRecord>,
    indexes: &mut HeapIndexState,
    old_gen: &mut OldGenState,
    stats: &mut HeapStats,
    runtime_state: &RuntimeStateHandle,
    before_bytes: usize,
    mark_steps: u64,
    mark_rounds: u64,
    mark_elapsed_nanos: u64,
    reclaim_prepare_nanos: u64,
    prepared_reclaim: PreparedReclaim,
    enqueue_pending_finalizer: impl FnMut(PendingFinalizer) -> u64,
) -> CollectionStats {
    let commit_state = begin_prepared_reclaim_commit(&prepared_reclaim, objects);
    finish_prepared_reclaim_cycle_with_state(
        objects,
        indexes,
        old_gen,
        stats,
        runtime_state,
        before_bytes,
        mark_steps,
        mark_rounds,
        mark_elapsed_nanos,
        reclaim_prepare_nanos,
        prepared_reclaim,
        commit_state,
        enqueue_pending_finalizer,
    )
}

pub(crate) fn finish_prepared_reclaim_cycle_with_state(
    objects: &mut Vec<ObjectRecord>,
    indexes: &mut HeapIndexState,
    old_gen: &mut OldGenState,
    stats: &mut HeapStats,
    runtime_state: &RuntimeStateHandle,
    before_bytes: usize,
    mark_steps: u64,
    mark_rounds: u64,
    mark_elapsed_nanos: u64,
    reclaim_prepare_nanos: u64,
    prepared_reclaim: PreparedReclaim,
    commit_state: PreparedReclaimCommitState,
    enqueue_pending_finalizer: impl FnMut(PendingFinalizer) -> u64,
) -> CollectionStats {
    let promoted_bytes = prepared_reclaim.promoted_bytes;
    let commit = finish_prepared_reclaim_commit(
        objects,
        indexes,
        old_gen,
        stats,
        runtime_state,
        prepared_reclaim,
        commit_state,
        enqueue_pending_finalizer,
    );
    CollectionStats::completed_old_gen_cycle(
        mark_steps,
        mark_rounds,
        promoted_bytes,
        mark_elapsed_nanos,
        reclaim_prepare_nanos,
        before_bytes,
        commit.after_bytes,
        commit.queued_finalizers,
        commit.old_region_stats,
    )
}

pub(crate) fn sweep_minor_and_rebuild_post_collection(
    objects: &mut Vec<ObjectRecord>,
    indexes: &mut HeapIndexState,
    old_gen: &mut OldGenState,
    old_config: &OldGenConfig,
    stats: &mut HeapStats,
    runtime_state: &RuntimeStateHandle,
    kind: CollectionKind,
    completed_plan: Option<CollectionPlan>,
    mut enqueue_pending_finalizer: impl FnMut(PendingFinalizer) -> u64,
) -> MinorRebuildResult {
    let _ = (old_config, completed_plan);
    let old_objects = core::mem::take(objects);
    let post_sweep_indexes = indexes.begin_post_sweep_rebuild(old_objects.len());
    let mut rebuilt_stats = PreparedHeapStats::default();

    let mut rebuilt_objects = Vec::with_capacity(old_objects.len());
    let mut queued_finalizers = 0u64;
    for object in old_objects {
        if !keep_object_for_collection(kind, &object) {
            if post_sweep_indexes.should_enqueue_finalizer(&object) {
                let pending = PendingFinalizer::new(object);
                queued_finalizers =
                    queued_finalizers.saturating_add(enqueue_pending_finalizer(pending));
            }
            continue;
        }

        object.clear_mark();
        let object_key = object.object_key();
        let desc = object.header().desc();
        let space = object.space();
        let total_size = object.total_size();
        let index = rebuilt_objects.len();
        rebuilt_objects.push(object);
        indexes.record_allocated_object(object_key, ObjectLocator::flat(index), desc);
        rebuilt_stats.record_live_object(space, total_size);
    }

    *objects = rebuilt_objects;
    // Rebuild block-level line marks from surviving records (and pending
    // finalizers) and reclaim any block whose lines remain entirely free.
    let dropped_blocks =
        rebuild_line_marks_and_reclaim_empty_old_blocks(objects, old_gen, runtime_state);
    let after_bytes = rebuilt_stats.apply_space_rebuild(stats, old_gen.reserved_bytes());
    indexes.refresh_remembered_owners_for_post_sweep_objects(objects);
    MinorRebuildResult {
        queued_finalizers,
        // reclaimed_regions reports the number of empty
        // old-gen blocks the post-minor sweep rebuild dropped.
        old_region_stats: OldRegionCollectionStats {
            compacted_regions: 0,
            reclaimed_regions: dropped_blocks as u64,
        },
        after_bytes,
    }
}

fn keep_object_for_collection(kind: CollectionKind, object: &ObjectRecord) -> bool {
    match kind {
        CollectionKind::Minor => {
            object.space() == SpaceKind::Immortal
                || object.space() != SpaceKind::Nursery
                || (object.is_marked() && !object.header().is_moved_out())
        }
        CollectionKind::Major | CollectionKind::Full => {
            object.space() == SpaceKind::Immortal
                || (object.is_marked() && !object.header().is_moved_out())
        }
    }
}

#[cfg(test)]
#[path = "reclaim_test.rs"]
mod tests;
