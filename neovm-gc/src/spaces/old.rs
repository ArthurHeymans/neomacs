use core::sync::atomic::{AtomicU32, AtomicU8, AtomicUsize, Ordering};

use crate::card_table::CardTable;
use crate::object::{ObjectRecord, OldBlockPlacement};
use crate::stats::OldRegionStats;

/// Sentinel value for [`OldBlock::object_starts`] meaning "no object
/// starts in this card."
const OBJECT_START_NONE: u32 = u32::MAX;

/// Old-generation configuration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OldGenConfig {
    /// Region size in bytes.
    pub region_bytes: usize,
    /// Line size in bytes for occupancy tracking.
    pub line_bytes: usize,
    /// Maximum number of old regions to target in one planned compaction cycle.
    pub compaction_candidate_limit: usize,
    /// Minimum reclaimable bytes required for a region to become a compaction candidate.
    pub selective_reclaim_threshold_bytes: usize,
    /// Maximum live bytes selected for compaction in one planned cycle.
    pub max_compaction_bytes_per_cycle: usize,
    /// Maximum number of concurrent mark workers.
    pub concurrent_mark_workers: usize,
    /// Number of major-mark slices one mutator operation should assist.
    pub mutator_assist_slices: usize,
    /// Density threshold below which an `OldBlock` becomes a
    /// physical-compaction candidate during a major cycle.
    /// Expressed as a ratio in `[0.0, 1.0]`. A block whose
    /// post-mark `live_bytes / capacity_bytes` is at or below
    /// this threshold has every surviving record evacuated into
    /// a freshly-created target block, after which the now-empty
    /// source block is reclaimed by the block-pool sweep.
    ///
    /// The default is `0.0` — physical compaction is opt-in.
    /// At `0.0` the threshold never fires (density is always
    /// `> 0.0` for blocks that still hold live records), so the
    /// post-major commit hook never moves any record. Setting
    /// this to e.g. `0.3` enables physical compaction of any
    /// block that is 30% full or less.
    pub physical_compaction_density_threshold: f64,
}

impl Default for OldGenConfig {
    fn default() -> Self {
        Self {
            region_bytes: 4 * 1024 * 1024,
            line_bytes: 256,
            compaction_candidate_limit: 8,
            selective_reclaim_threshold_bytes: 1,
            max_compaction_bytes_per_cycle: usize::MAX,
            concurrent_mark_workers: 1,
            mutator_assist_slices: 1,
            physical_compaction_density_threshold: 0.0,
        }
    }
}

/// A single old-generation Immix-style block.
///
/// Each block owns a contiguous backing buffer divided into fixed-size
/// lines. The block tracks per-line occupancy with `line_marks` so the
/// post-sweep allocator can find runs of free lines (Immix hole filling)
/// before falling back to a fresh block.
///
/// `cursor` is a hint into the byte buffer where the next allocation scan
/// starts. After a sweep the cursor is reset to zero so the allocator can
/// see freshly opened holes near the front of the block.
#[derive(Debug)]
pub(crate) struct OldBlock {
    buffer: Box<[u8]>,
    line_marks: Box<[AtomicU8]>,
    line_bytes: usize,
    /// Bump cursor into the backing buffer.  Atomic so `try_alloc`
    /// can run under `&self` — the fast hole-filling search and
    /// placement use CAS to claim space without a lock.
    cursor: AtomicUsize,
    /// High-water mark: the largest offset any allocation has ever
    /// advanced the cursor to. Tracks the logical "region used bytes"
    /// concept that Step 1 of the OldRegion → OldBlock unification is
    /// lifting out of `OldRegion`. Reset to 0 when the block is
    /// cleared (currently unused — blocks are dropped, not cleared,
    /// on reclaim — but preserved for future sweep paths).
    used_bytes: AtomicUsize,
    /// Sum of `total_size` over every live object currently placed
    /// in the block. Mirrors `OldRegion::live_bytes`. Updated by
    /// `record_object_accounting` and cleared by
    /// `clear_live_accounting`.
    live_bytes: AtomicUsize,
    /// Count of live objects currently placed in the block. Mirrors
    /// `OldRegion::object_count`. Updated alongside `live_bytes`.
    object_count: AtomicUsize,
    /// One byte per line. `1` means at least one live object overlaps
    /// that line in the current accounting snapshot.
    /// Distinct from `line_marks`: the atomic `line_marks` array is
    /// set only during sweep (by the post-sweep survivor walk), while
    /// these flags mirror the exact semantics of
    /// `OldRegion::occupied_lines` — updated at allocation time by
    /// `record_object_accounting` so `region_stats` can report
    /// `occupied_lines` without a sweep having run yet.
    occupied_lines: Box<[AtomicU8]>,
    /// Cached count of lines whose corresponding `occupied_lines`
    /// byte is currently non-zero.
    occupied_line_count: AtomicUsize,
    /// Per-block card table covering the backing buffer. The write barrier
    /// dirties cards through this table when the owner of an old-to-young
    /// edge lives inside the block; the minor GC root scan walks the dirty
    /// cards to find old-gen objects that may reference young targets.
    card_table: CardTable,
    /// One entry per card in this block. Each entry is the offset (from
    /// the start of the buffer) of the FIRST object header that lies in
    /// that card. `None` if no object starts in that card. The minor GC
    /// dirty-card root scan uses this index to walk dirty cards in
    /// O(dirty_cards) instead of doing a linear pass over every record
    /// in `Heap::objects` per dirty card. Subsequent objects in the same
    /// card are reached by walking forward from the first one via the
    /// per-object total_size header field.
    /// Per-card index into the backing buffer: `AtomicU32` where
    /// `u32::MAX` means no object starts in this card, otherwise
    /// the byte offset (from buffer start) of the first object
    /// header in the card.  Atomic so `try_alloc` / `record_object_start`
    /// can update the index under `&self`.
    object_starts: Box<[AtomicU32]>,
}

impl OldBlock {
    /// Construct a new block whose backing buffer is at least
    /// `capacity_bytes` long, rounded up to a whole number of `line_bytes`
    /// lines (and at least one line so degenerate configurations stay
    /// well-defined).
    pub(crate) fn new(capacity_bytes: usize, line_bytes: usize) -> Self {
        let line_bytes = line_bytes.max(1);
        let line_count = capacity_bytes.div_ceil(line_bytes).max(1);
        let buffer_len = line_count.saturating_mul(line_bytes);
        let buffer: Box<[u8]> = vec![0u8; buffer_len].into_boxed_slice();
        let mut marks = Vec::with_capacity(line_count);
        let mut occupied = Vec::with_capacity(line_count);
        for _ in 0..line_count {
            marks.push(AtomicU8::new(0));
            occupied.push(AtomicU8::new(0));
        }
        let base_addr = buffer.as_ptr() as usize;
        let card_table = CardTable::with_default_card_size(base_addr, buffer_len);
        let object_starts: Box<[AtomicU32]> = (0..card_table.card_count())
            .map(|_| AtomicU32::new(OBJECT_START_NONE))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self {
            buffer,
            line_marks: marks.into_boxed_slice(),
            line_bytes,
            cursor: AtomicUsize::new(0),
            used_bytes: AtomicUsize::new(0),
            live_bytes: AtomicUsize::new(0),
            object_count: AtomicUsize::new(0),
            occupied_lines: occupied.into_boxed_slice(),
            occupied_line_count: AtomicUsize::new(0),
            card_table,
            object_starts,
        }
    }

    /// Accumulated size of every live object currently placed in
    /// this block. Mirrors `OldRegion::live_bytes`. Updated by
    /// [`record_object_accounting`] and reset by
    /// [`clear_live_accounting`].
    pub(crate) fn live_bytes(&self) -> usize {
        self.live_bytes.load(Ordering::Relaxed)
    }

    /// Count of live objects currently placed in this block.
    /// Mirrors `OldRegion::object_count`. Updated by
    /// [`record_object_accounting`] and reset by
    /// [`clear_live_accounting`].
    pub(crate) fn object_count(&self) -> usize {
        self.object_count.load(Ordering::Relaxed)
    }

    /// Allocation high-water mark: the largest buffer offset any
    /// `try_alloc` has advanced through. Mirrors
    /// `OldRegion::used_bytes`. Distinct from [`cursor`] because
    /// sweep resets the cursor for hole-filling but keeps
    /// `used_bytes` as the upper bound of ever-allocated space
    /// (useful for stats and compaction planning).
    pub(crate) fn used_bytes(&self) -> usize {
        self.used_bytes.load(Ordering::Relaxed)
    }

    /// Number of lines currently containing at least one live
    /// object, as tracked by the allocation-time `occupied_lines`
    /// set. Distinct from [`line_marks`]: this reflects what
    /// `record_object_accounting` has recorded, while `line_marks`
    /// is populated only during post-sweep rebuild.
    pub(crate) fn occupied_line_count(&self) -> usize {
        self.occupied_line_count.load(Ordering::Relaxed)
    }

    /// Exclusive accounting path used while the caller already
    /// holds mutable access to the block (e.g. sweep rebuild and
    /// serial nursery promotion). Avoids atomic RMW traffic.
    pub(crate) fn record_object_accounting(&mut self, offset: usize, size: usize) {
        if size == 0 {
            return;
        }
        *self.live_bytes.get_mut() = self.live_bytes().saturating_add(size);
        *self.object_count.get_mut() = self.object_count().saturating_add(1);
        let end = offset.saturating_add(size);
        let bounded_end = end.min(self.buffer.len());
        if bounded_end > self.used_bytes() {
            *self.used_bytes.get_mut() = bounded_end;
        }
        let first_line = offset / self.line_bytes;
        let last_byte = end.saturating_sub(1);
        let last_line = last_byte / self.line_bytes;
        for line in first_line..=last_line {
            let Some(slot) = self.occupied_lines.get_mut(line) else {
                continue;
            };
            if *slot.get_mut() == 0 {
                *slot.get_mut() = 1;
                *self.occupied_line_count.get_mut() = self.occupied_line_count().saturating_add(1);
            }
        }
    }

    /// Shared accounting path used by the concurrent old-allocation
    /// commit once the record is already published. Uses atomics so
    /// the caller only needs `&self`.
    pub(crate) fn record_object_accounting_shared(&self, offset: usize, size: usize) {
        if size == 0 {
            return;
        }
        self.live_bytes.fetch_add(size, Ordering::Relaxed);
        self.object_count.fetch_add(1, Ordering::Relaxed);
        let end = offset.saturating_add(size);
        let bounded_end = end.min(self.buffer.len());
        let mut observed = self.used_bytes.load(Ordering::Relaxed);
        while bounded_end > observed {
            match self.used_bytes.compare_exchange_weak(
                observed,
                bounded_end,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(next) => observed = next,
            }
        }
        let first_line = offset / self.line_bytes;
        let last_byte = end.saturating_sub(1);
        let last_line = last_byte / self.line_bytes;
        for line in first_line..=last_line {
            let Some(slot) = self.occupied_lines.get(line) else {
                continue;
            };
            if slot
                .compare_exchange(0, 1, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                self.occupied_line_count.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Clear the live-object accounting counters without touching
    /// the backing buffer or line marks. Invoked at the start of a
    /// sweep-driven rebuild, right before the survivor walk
    /// re-populates them via [`record_object_accounting`]. Leaves
    /// `used_bytes` at its current high-water mark — the block's
    /// physical layout does not shrink even after dead objects are
    /// reclaimed.
    pub(crate) fn clear_live_accounting(&mut self) {
        self.live_bytes.store(0, Ordering::Relaxed);
        self.object_count.store(0, Ordering::Relaxed);
        self.occupied_line_count.store(0, Ordering::Relaxed);
        for line in self.occupied_lines.iter_mut() {
            line.store(0, Ordering::Relaxed);
        }
    }

    /// Read-only view of the per-card object-start index. Each entry is
    /// the byte offset (relative to the buffer base) of the first object
    /// header that lies in that card, or `None` if no object starts in
    /// the card. Used by the minor GC dirty-card scan and by tests.
    pub(crate) fn object_starts(&self) -> &[AtomicU32] {
        &self.object_starts
    }

    /// Load the object-start offset for `card_index`, returning
    /// `None` when no object starts in that card.
    #[inline]
    pub(crate) fn object_start_for_card(&self, card_index: usize) -> Option<u32> {
        let raw = self
            .object_starts
            .get(card_index)?
            .load(Ordering::Relaxed);
        if raw == OBJECT_START_NONE {
            None
        } else {
            Some(raw)
        }
    }

    /// Reset every per-card object-start entry to `None`. Called from
    /// the post-sweep rebuild before walking surviving records to repopulate
    /// the index.
    pub(crate) fn clear_object_starts(&mut self) {
        for slot in self.object_starts.iter_mut() {
            *slot.get_mut() = OBJECT_START_NONE;
        }
    }

    /// Record an object start at the given buffer offset. The per-card
    /// object-start entry tracks the SMALLEST offset whose header lies
    /// in that card; subsequent objects in the same card can be discovered
    /// by walking forward via the per-object total_size header field.
    /// We track the smallest offset (rather than the first call to win)
    /// because the post-sweep rebuild may visit surviving records out of
    /// allocation order. Out-of-range offsets are silently ignored.
    ///
    /// Takes `&self` — uses CAS on the per-card `AtomicU32`.
    pub(crate) fn record_object_start(&self, offset: usize) {
        let card_size = self.card_table.card_size();
        let card_idx = offset / card_size;
        let offset_u32 = offset as u32;
        let Some(slot) = self.object_starts.get(card_idx) else {
            return;
        };
        let mut current = slot.load(Ordering::Relaxed);
        loop {
            if current != OBJECT_START_NONE && current <= offset_u32 {
                break;
            }
            match slot.compare_exchange_weak(
                current,
                offset_u32,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
    }

    /// Read-only access to the per-block card table. The remembered-set
    /// write barrier and the minor GC root scan use this to dirty/scan
    /// cards covering the block buffer.
    pub(crate) fn card_table(&self) -> &CardTable {
        &self.card_table
    }

    /// Total backing buffer length in bytes.
    pub(crate) fn capacity_bytes(&self) -> usize {
        self.buffer.len()
    }

    /// Number of lines in the block.
    pub(crate) fn line_count(&self) -> usize {
        self.line_marks.len()
    }

    /// Bytes per line.
    pub(crate) fn line_bytes(&self) -> usize {
        self.line_bytes
    }

    /// Base pointer of the backing buffer (read-only). The pointer remains
    /// valid for the lifetime of the block.
    pub(crate) fn base_ptr(&self) -> *const u8 {
        self.buffer.as_ptr()
    }

    /// True if the byte at `addr` (an absolute pointer-as-usize) lies
    /// inside this block's backing buffer.
    pub(crate) fn contains_addr(&self, addr: usize) -> bool {
        let base = self.base_ptr() as usize;
        addr >= base && addr < base + self.buffer.len()
    }

    /// Mark the line at `index` as occupied. Out-of-range indices are
    /// silently ignored.
    pub(crate) fn mark_line(&self, index: usize) {
        if let Some(slot) = self.line_marks.get(index) {
            slot.store(1, Ordering::Relaxed);
        }
    }

    /// Test whether the line at `index` is currently marked occupied.
    pub(crate) fn is_line_marked(&self, index: usize) -> bool {
        self.line_marks
            .get(index)
            .map(|slot| slot.load(Ordering::Relaxed) != 0)
            .unwrap_or(false)
    }

    /// Mark every line covered by the byte range `[offset, offset + size)`
    /// as occupied. Sweep walks surviving block-backed records and calls
    /// this for each one to rebuild the line occupancy map.
    pub(crate) fn mark_lines_for_range(&self, offset: usize, size: usize) {
        if size == 0 {
            return;
        }
        let start_line = offset / self.line_bytes;
        let end_byte = offset.saturating_add(size).saturating_sub(1);
        let end_line = end_byte / self.line_bytes;
        let last_line = self.line_count().saturating_sub(1);
        let end_line = end_line.min(last_line);
        for line in start_line..=end_line {
            self.mark_line(line);
        }
    }

    /// Clear every line mark in the block.
    pub(crate) fn clear_line_marks(&self) {
        for slot in self.line_marks.iter() {
            slot.store(0, Ordering::Relaxed);
        }
    }

    /// True when no line is marked as occupied. Empty blocks are reclaimed
    /// after the sweep.
    pub(crate) fn is_empty(&self) -> bool {
        self.line_marks
            .iter()
            .all(|slot| slot.load(Ordering::Relaxed) == 0)
    }

    /// Reset the bump cursor back to the start of the block.
    /// GC-only — requires `&mut self` for exclusive access.
    pub(crate) fn reset_cursor(&mut self) {
        *self.cursor.get_mut() = 0;
    }

    /// Try to allocate `layout.size()` bytes from the block using
    /// hole-filling. The implementation scans `line_marks` starting at
    /// the current cursor for the first run of `ceil(size / line_bytes)`
    /// consecutive free lines. On success the cursor advances past the
    /// allocation and the function returns the offset of the placement
    /// inside the buffer plus a `NonNull<u8>` to that slot.
    ///
    /// Takes `&self` — the bump cursor is atomic so multiple threads
    /// can allocate concurrently from the same block.  When contention
    /// is rare (the common case) the CAS retry loop never spins.
    pub(crate) fn try_alloc(
        &self,
        layout: core::alloc::Layout,
    ) -> Option<(usize, core::ptr::NonNull<u8>)> {
        let size = layout.size();
        if size == 0 {
            return None;
        }
        if size > self.buffer.len() {
            return None;
        }
        let lines_needed = size.div_ceil(self.line_bytes).max(1);
        let line_count = self.line_count();
        if lines_needed > line_count {
            return None;
        }

        let mut cursor = self.cursor.load(Ordering::Acquire);
        loop {
            let cursor_line = cursor.div_ceil(self.line_bytes);
            let mut search_line = cursor_line;
            // Find a free run of `lines_needed` consecutive lines.
            let mut found = false;
            while search_line + lines_needed <= line_count {
                while search_line + lines_needed <= line_count && self.is_line_marked(search_line) {
                    search_line += 1;
                }
                if search_line + lines_needed > line_count {
                    break;
                }
                let mut run_end = search_line;
                while run_end < line_count
                    && !self.is_line_marked(run_end)
                    && run_end - search_line < lines_needed
                {
                    run_end += 1;
                }
                if run_end - search_line >= lines_needed {
                    let offset = search_line * self.line_bytes;
                    let alloc_end = offset + size;
                    if alloc_end > self.buffer.len() {
                        return None;
                    }
                    let base_addr = self.buffer.as_ptr() as usize;
                    let slot_addr = base_addr + offset;
                    if !slot_addr.is_multiple_of(layout.align().max(1)) {
                        search_line = run_end;
                        continue;
                    }
                    let after_lines = offset + lines_needed * self.line_bytes;
                    let next_cursor = after_lines.min(self.buffer.len());
                    match self.cursor.compare_exchange_weak(
                        cursor,
                        next_cursor,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => {
                            if next_cursor > self.used_bytes.load(Ordering::Relaxed) {
                                self.used_bytes.store(next_cursor, Ordering::Relaxed);
                            }
                            self.record_object_start(offset);
                            // SAFETY: offset is in-range; the buffer outlives the block.
                            let raw =
                                unsafe { (self.buffer.as_ptr() as *mut u8).add(offset) };
                            let ptr = core::ptr::NonNull::new(raw)?;
                            return Some((offset, ptr));
                        }
                        Err(actual) => {
                            cursor = actual;
                            found = true; // break to outer retry
                            break;
                        }
                    }
                }
                search_line = run_end;
            }
            if !found {
                return None;
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct OldGenState {
    /// Block buffer pool. Blocks are allocated on demand when direct old-gen
    /// allocation or nursery promotion needs fresh backing storage, and the
    /// post-sweep reclaim path drops blocks whose line marks are entirely
    /// empty (Immix-style block reclaim).
    ///
    /// Wrapped in a `parking_lot::Mutex` so the allocation hot path
    /// (`try_alloc_in_block`) can run under `&self`, avoiding the
    /// `HeapCore` write lock.  GC paths also acquire this lock, but
    /// since the safepoint write lock excludes all mutators during
    /// collection, the acquisition is always uncontended.
    /// Block pool protected by an RwLock.  The barrier hot path takes
    /// only a read lock (find_block_for_addr / record_write_barrier),
    /// avoiding contention with other concurrent barriers.  Allocation
    /// takes the write lock.
    blocks: parking_lot::RwLock<Vec<OldBlock>>,
    reserved_bytes: AtomicUsize,
    total_used_bytes: AtomicUsize,
    /// Cache of the last block index found by `record_write_barrier`.
    /// Eliminates the binary search for repeated writes to the same
    /// block (common in tight loops).  `usize::MAX` means no cache.
    last_block_index: AtomicUsize,
}

impl Default for OldGenState {
    fn default() -> Self {
        Self {
            blocks: parking_lot::RwLock::new(Vec::new()),
            reserved_bytes: AtomicUsize::new(0),
            total_used_bytes: AtomicUsize::new(0),
            last_block_index: AtomicUsize::new(usize::MAX),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct OldGenPlanSelection {
    pub(crate) candidates: Vec<OldRegionStats>,
    pub(crate) estimated_compaction_bytes: usize,
    pub(crate) estimated_reclaim_bytes: usize,
}

#[derive(Debug, Default)]
pub(crate) struct PreparedOldGenReclaim {
    pub(crate) region_stats: OldRegionCollectionStats,
}

impl OldGenState {
    pub(crate) fn is_empty(&self) -> bool {
        self.blocks.read().is_empty()
    }

    pub(crate) fn reserved_bytes(&self) -> usize {
        self.reserved_bytes.load(Ordering::Relaxed)
    }

    /// Total bytes the block-pool bump allocator has consumed
    /// across every block (`sum(block.used_bytes)`). This is the
    /// denominator of the old-gen fragmentation ratio and is
    /// cached into `HeapStats::old_gen_used_bytes` so observers
    /// that want the ratio lock-free can read it from the shared
    /// snapshot.
    pub(crate) fn total_used_bytes(&self) -> usize {
        self.total_used_bytes.load(Ordering::Relaxed)
    }

    fn refresh_cached_layout_totals(&self) {
        let blocks = self.blocks.read();
        let reserved_bytes: usize = blocks.iter().map(|block| block.capacity_bytes()).sum();
        let total_used_bytes: usize = blocks.iter().map(|block| block.used_bytes()).sum();
        self.reserved_bytes.store(reserved_bytes, Ordering::Relaxed);
        self.total_used_bytes
            .store(total_used_bytes, Ordering::Relaxed);
    }

    /// Immix-style block allocation. Walks every block looking
    /// for a hole large enough to fit `layout` (hole-filling), and
    /// on failure allocates a fresh block sized to the larger of
    /// `config.region_bytes` and `layout.size()`. Returns the
    /// placement (block index plus byte offset) and a
    /// `NonNull<u8>` to the placement slot.
    ///
    /// Takes `&self` — acquires the internal block-pool lock.
    pub(crate) fn try_alloc_in_block(
        &self,
        config: &OldGenConfig,
        layout: core::alloc::Layout,
    ) -> Option<(OldBlockPlacement, core::ptr::NonNull<u8>)> {
        let mut blocks = self.blocks.write();
        // Try every existing block from oldest to newest. Hot allocation
        // benefits from staying in the most recently used block first, but
        // hole filling improves overall density at the cost of one extra
        // pass — start the search at the beginning so we always re-use the
        // earliest available hole, mirroring the Immix paper recommendation.
        for index in 0..blocks.len() {
            let used_before = blocks[index].used_bytes();
            if let Some((offset, ptr)) = blocks[index].try_alloc(layout) {
                let used_after = blocks[index].used_bytes();
                drop(blocks);
                self.total_used_bytes
                    .fetch_add(used_after.saturating_sub(used_before), Ordering::Relaxed);
                let placement = OldBlockPlacement {
                    block_index: index,
                    offset_bytes: offset,
                    total_size: layout.size(),
                };
                return Some((placement, ptr));
            }
        }

        // No existing block had room — allocate a new block sized to
        // the larger of the configured region size and the requested
        // layout.
        let capacity = config.region_bytes.max(layout.size());
        let line_bytes = config.line_bytes.max(1);
        let mut block = OldBlock::new(capacity, line_bytes);
        let (offset, ptr) = block.try_alloc(layout)?;
        let used_bytes = block.used_bytes();
        let block_index = blocks.len();
        blocks.push(block);
        drop(blocks);
        self.reserved_bytes.fetch_add(capacity, Ordering::Relaxed);
        self.total_used_bytes
            .fetch_add(used_bytes, Ordering::Relaxed);
        Some((
            OldBlockPlacement {
                block_index,
                offset_bytes: offset,
                total_size: layout.size(),
            },
            ptr,
        ))
    }

    pub(crate) fn try_alloc_in_block_with_reserved(
        &self,
        config: &OldGenConfig,
        layout: core::alloc::Layout,
    ) -> Option<(OldBlockPlacement, core::ptr::NonNull<u8>, usize)> {
        let (placement, ptr) = self.try_alloc_in_block(config, layout)?;
        let reserved = self.reserved_bytes();
        Some((placement, ptr, reserved))
    }

    /// Compaction target allocator: try to place `layout` into the
    /// block at `target_hint`; if that fails (or `target_hint` is
    /// `None`), create a fresh block and place the layout there.
    /// Returns the `(placement, pointer, new_target_hint)` tuple;
    /// the new_target_hint is the block index the allocation
    /// landed in, which the caller should thread into the next
    /// call so multiple survivors from the compaction pass share
    /// the same target block.
    ///
    /// Returns `None` only if even the fresh-block path cannot
    /// service the layout (e.g. `layout.size() == 0`). Callers
    /// treat that as "skip this survivor."
    pub(crate) fn alloc_for_compaction_into_target(
        &self,
        config: &OldGenConfig,
        layout: core::alloc::Layout,
        target_hint: Option<usize>,
    ) -> Option<(OldBlockPlacement, core::ptr::NonNull<u8>, usize)> {
        let mut blocks = self.blocks.write();
        if let Some(index) = target_hint
            && let Some(block) = blocks.get(index)
            && let Some((offset, ptr)) = block.try_alloc(layout)
        {
            return Some((
                OldBlockPlacement {
                    block_index: index,
                    offset_bytes: offset,
                    total_size: layout.size(),
                },
                ptr,
                index,
            ));
        }
        drop(blocks);
        let (placement, ptr) = self.alloc_in_fresh_block(config, layout)?;
        let new_target = placement.block_index;
        Some((placement, ptr, new_target))
    }

    /// Allocate directly into a newly-created block, bypassing the
    /// hole-filling search over existing blocks that
    /// [`try_alloc_in_block`] performs.
    ///
    /// Takes `&self` — acquires the internal block-pool lock.
    pub(crate) fn alloc_in_fresh_block(
        &self,
        config: &OldGenConfig,
        layout: core::alloc::Layout,
    ) -> Option<(OldBlockPlacement, core::ptr::NonNull<u8>)> {
        let capacity = config.region_bytes.max(layout.size());
        let line_bytes = config.line_bytes.max(1);
        let mut block = OldBlock::new(capacity, line_bytes);
        let (offset, ptr) = block.try_alloc(layout)?;
        let used_bytes = block.used_bytes();
        let mut blocks = self.blocks.write();
        let block_index = blocks.len();
        blocks.push(block);
        drop(blocks);
        self.reserved_bytes.fetch_add(capacity, Ordering::Relaxed);
        self.total_used_bytes
            .fetch_add(used_bytes, Ordering::Relaxed);
        Some((
            OldBlockPlacement {
                block_index,
                offset_bytes: offset,
                total_size: layout.size(),
            },
            ptr,
        ))
    }

    /// Number of physical blocks currently in the pool.
    pub(crate) fn block_count(&self) -> usize {
        self.blocks.read().len()
    }

    /// Iterate over every block in the pool.
    /// Returns a guard that derefs to `&[OldBlock]`.
    pub(crate) fn blocks(&self) -> parking_lot::RwLockReadGuard<'_, Vec<OldBlock>> {
        self.blocks.read()
    }

    /// Find the index of the block whose backing buffer contains `addr`.
    ///
    /// Uses binary search over the block pool, which is maintained in
    /// address order: blocks are only ever appended (`push`) or filtered
    /// (`retain`, which preserves relative order), so their base
    /// addresses are monotonically non-decreasing.
    pub(crate) fn find_block_for_addr(&self, addr: usize) -> Option<usize> {
        self.blocks
            .read()
            .binary_search_by(|block| {
                let base = block.base_ptr() as usize;
                let end = base.saturating_add(block.buffer.len());
                if addr < base {
                    std::cmp::Ordering::Greater
                } else if addr >= end {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .ok()
    }

    /// Mark the card containing `owner_addr` as dirty if
    /// `owner_addr` falls inside one of the old-gen blocks.
    /// Returns true if a card was set; false when the owner is
    /// not block-backed (pinned record, large-object space, or
    /// a system-allocated fallback record). Callers must fall
    /// back to the explicit-edge `RememberedSetState` for the
    /// false case so the minor GC's old-to-young scan still
    /// covers the owner.
    pub(crate) fn record_write_barrier(&self, owner_addr: usize) -> bool {
        // Check the cached block index first.  In tight loops most
        // writes hit the same block, so this avoids the O(log n)
        // binary search on the hot path.
        let cached = self.last_block_index.load(Ordering::Relaxed);
        if cached != usize::MAX {
            let blocks = self.blocks.read();
            if let Some(block) = blocks.get(cached)
                && block.contains_addr(owner_addr)
            {
                block.card_table().record_write(owner_addr);
                return true;
            }
        }
        // Cache miss — do the full search under a single read lock.
        let blocks = self.blocks.read();
        let Some(index) = blocks
            .binary_search_by(|block| {
                let base = block.base_ptr() as usize;
                let end = base.saturating_add(block.buffer.len());
                if owner_addr < base {
                    std::cmp::Ordering::Greater
                } else if owner_addr >= end {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .ok()
        else {
            return false;
        };
        self.last_block_index.store(index, Ordering::Relaxed);
        blocks[index].card_table().record_write(owner_addr);
        true
    }

    /// Reset every per-block card table back to clean. Invoked at the
    /// end of a minor collection after the dirty-card scan has produced
    /// roots for the trace.
    pub(crate) fn clear_all_dirty_cards(&self) {
        for block in self.blocks.read().iter() {
            block.card_table().clear_all();
        }
    }

    /// Total number of dirty cards across every block in the pool.
    pub(crate) fn dirty_card_count(&self) -> usize {
        self.blocks
            .read()
            .iter()
            .map(|block| block.card_table().dirty_card_count())
            .sum()
    }

    /// Clear every line mark across every block.
    pub(crate) fn clear_all_block_line_marks(&self) {
        for block in self.blocks.read().iter() {
            block.clear_line_marks();
        }
    }

    /// Clear every per-card object-start entry across every block.
    pub(crate) fn clear_all_block_object_starts(&self) {
        for block in self.blocks.write().iter_mut() {
            block.clear_object_starts();
        }
    }

    /// Clear the live-object accounting counters on every block.
    pub(crate) fn clear_all_block_live_accounting(&self) {
        for block in self.blocks.write().iter_mut() {
            block.clear_live_accounting();
        }
    }

    /// Record a surviving block-backed object into the block's
    /// live accounting counters (`live_bytes`, `object_count`,
    /// `occupied_lines`). Mirrors
    /// `record_block_object_start_for_placement` and
    /// `mark_block_lines_for_placement`: these three helpers are
    /// called in lockstep by the post-sweep rebuild for every
    /// surviving record.
    pub(crate) fn record_block_object_accounting_for_placement(
        &self,
        placement: OldBlockPlacement,
    ) {
        if let Some(block) = self.blocks.write().get_mut(placement.block_index) {
            block.record_object_accounting(placement.offset_bytes, placement.total_size);
        }
    }

    pub(crate) fn record_block_object_accounting_for_placement_shared(
        &self,
        placement: OldBlockPlacement,
    ) {
        if let Some(block) = self.blocks.read().get(placement.block_index) {
            block.record_object_accounting_shared(placement.offset_bytes, placement.total_size);
        }
    }

    /// Record an object start in the block identified by `placement`.
    pub(crate) fn record_block_object_start_for_placement(&self, placement: OldBlockPlacement) {
        if let Some(block) = self.blocks.read().get(placement.block_index) {
            block.record_object_start(placement.offset_bytes);
        }
    }

    /// Mark the lines covered by `placement` in the corresponding block.
    pub(crate) fn mark_block_lines_for_placement(&self, placement: OldBlockPlacement) {
        if let Some(block) = self.blocks.read().get(placement.block_index) {
            block.mark_lines_for_range(placement.offset_bytes, placement.total_size);
        }
    }

    /// Reset every block's bump cursor without dropping any blocks.
    pub(crate) fn reset_block_cursors(&self) {
        for block in self.blocks.write().iter_mut() {
            block.reset_cursor();
        }
    }

    /// Compute a remapping that drops empty blocks and rewrites surviving
    /// indices into a contiguous 0..N range. Returns `(new_indices, dropped)`
    /// where `new_indices[old] == Some(new)` if the block survives or `None`
    /// if it was dropped.
    pub(crate) fn compute_block_index_remap(&self) -> (Vec<Option<usize>>, usize) {
        let blocks = self.blocks.read();
        let mut new_indices = Vec::with_capacity(blocks.len());
        let mut next = 0usize;
        let mut dropped = 0usize;
        for block in blocks.iter() {
            if block.is_empty() {
                new_indices.push(None);
                dropped += 1;
            } else {
                new_indices.push(Some(next));
                next += 1;
            }
        }
        (new_indices, dropped)
    }

    /// Drop blocks whose line marks are completely empty after a sweep.
    /// Returns a remap from old block indices to new block indices (or
    /// `None` if the block was dropped) so the caller can rebind any
    /// surviving `OldBlockPlacement::block_index` values that were stored
    /// outside the block pool.
    ///
    /// IMPORTANT: callers must mark the lines of every record that anchors
    /// a block — including pending finalizers — before invoking this
    /// function. A block is reclaimed iff none of its lines are marked.
    pub(crate) fn drop_unused_blocks_with_remap(&self) -> Vec<Option<usize>> {
        let (remap, dropped) = self.compute_block_index_remap();
        if dropped == 0 {
            self.reset_block_cursors();
            return remap;
        }

        let mut next = 0usize;
        let mut keep_mask = Vec::with_capacity(self.blocks.read().len());
        for entry in &remap {
            keep_mask.push(entry.is_some());
        }
        self.blocks.write().retain(|_| {
            let keep = keep_mask[next];
            next += 1;
            keep
        });
        self.refresh_cached_layout_totals();
        self.reset_block_cursors();
        remap
    }

    pub(crate) fn record_object(&self, object: &ObjectRecord) {
        if let Some(block_placement) = object.old_block_placement()
            && let Some(block) = self.blocks.read().get(block_placement.block_index)
        {
            block
                .record_object_accounting_shared(block_placement.offset_bytes, object.total_size());
        }
    }

    /// Per-block region-stats reader. Aliases
    /// [`OldGenState::block_region_stats`]; both methods
    /// produce the same result. Kept under both names so older
    /// internal call sites and tests that grep for
    /// `region_stats` continue to work.
    pub(crate) fn region_stats(&self) -> Vec<OldRegionStats> {
        self.block_region_stats()
    }

    /// Block-backed stats view. Each `OldBlock` maps to one
    /// `OldRegionStats` entry (using the block index as the
    /// pseudo region index). The per-block counters are
    /// maintained by the sweep rebuild and the bump allocator
    /// directly.
    ///
    /// Field semantics:
    /// * `region_index` — block index in allocation order.
    /// * `reserved_bytes` — block capacity.
    /// * `used_bytes` — high-water mark of the bump cursor in
    ///   the block. Only shrinks when bytes are physically
    ///   moved (via physical compaction).
    /// * `live_bytes` — sum of survivor sizes after the most
    ///   recent sweep rebuild.
    /// * `hole_bytes` — interior gaps (`used_bytes - live_bytes`).
    /// * `tail_bytes` — unused tail at the end of the block.
    /// * `object_count` — number of survivors in the block.
    /// * `occupied_lines` — number of Immix lines containing
    ///   live data.
    pub(crate) fn block_region_stats(&self) -> Vec<OldRegionStats> {
        self.blocks
            .read()
            .iter()
            .enumerate()
            .map(|(index, block)| {
                let reserved_bytes = block.capacity_bytes();
                let used_bytes = block.used_bytes();
                let live_bytes = block.live_bytes();
                OldRegionStats {
                    region_index: index,
                    reserved_bytes,
                    used_bytes,
                    live_bytes,
                    free_bytes: reserved_bytes.saturating_sub(live_bytes),
                    hole_bytes: used_bytes.saturating_sub(live_bytes),
                    tail_bytes: reserved_bytes.saturating_sub(used_bytes),
                    object_count: block.object_count(),
                    occupied_lines: block.occupied_line_count(),
                }
            })
            .collect()
    }

    /// Old-gen compaction candidate selector. Reads from the
    /// per-block view (the legacy logical-region selector was
    /// retired alongside the rebuild path). The estimator
    /// outputs (`estimated_compaction_bytes`,
    /// `estimated_reclaim_bytes`) feed `CollectionPlan` fields
    /// of the same names, which the runtime accumulates into
    /// `Heap::stats().collections`.
    pub(crate) fn block_plan_selection(&self, config: &OldGenConfig) -> OldGenPlanSelection {
        Self::run_major_plan_selection(self.block_region_stats(), config)
    }

    fn run_major_plan_selection(
        stats: Vec<OldRegionStats>,
        config: &OldGenConfig,
    ) -> OldGenPlanSelection {
        let mut candidates: Vec<_> = stats
            .into_iter()
            .filter(|region| {
                region.object_count > 0
                    && region.hole_bytes > 0
                    && region.hole_bytes >= config.selective_reclaim_threshold_bytes
            })
            .collect();
        candidates.sort_by(compare_compaction_candidate_priority);

        let max_regions = config.compaction_candidate_limit;
        let max_bytes = config.max_compaction_bytes_per_cycle;
        let mut selected = Vec::new();
        let mut selected_bytes = 0usize;
        for candidate in candidates {
            if selected.len() >= max_regions {
                break;
            }
            if selected_bytes.saturating_add(candidate.live_bytes) > max_bytes {
                continue;
            }
            selected_bytes = selected_bytes.saturating_add(candidate.live_bytes);
            selected.push(candidate);
        }

        OldGenPlanSelection {
            estimated_compaction_bytes: selected.iter().map(|region| region.live_bytes).sum(),
            estimated_reclaim_bytes: selected.iter().map(|region| region.hole_bytes).sum(),
            candidates: selected,
        }
    }

    /// Apply a prepared reclaim. The major-cycle reclaim
    /// pipeline now defers all physical mutation to the
    /// `rebuild_line_marks_and_reclaim_empty_old_blocks` pass
    /// that runs at the end of the major commit, so this entry
    /// point exists only to thread the prepared region stats
    /// out of the prepared-reclaim phase and into the cycle
    /// stats.
    pub(crate) fn apply_prepared_reclaim(
        &self,
        prepared: PreparedOldGenReclaim,
    ) -> OldRegionCollectionStats {
        prepared.region_stats
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct OldRegionCollectionStats {
    pub(crate) compacted_regions: u64,
    pub(crate) reclaimed_regions: u64,
}

pub(crate) fn compare_compaction_candidate_priority(
    left: &OldRegionStats,
    right: &OldRegionStats,
) -> core::cmp::Ordering {
    let left_live = left.live_bytes.max(1) as u128;
    let right_live = right.live_bytes.max(1) as u128;
    let left_efficiency = (left.hole_bytes as u128).saturating_mul(right_live);
    let right_efficiency = (right.hole_bytes as u128).saturating_mul(left_live);

    right_efficiency
        .cmp(&left_efficiency)
        .then_with(|| right.hole_bytes.cmp(&left.hole_bytes))
        .then_with(|| left.live_bytes.cmp(&right.live_bytes))
        .then_with(|| right.free_bytes.cmp(&left.free_bytes))
        .then_with(|| left.object_count.cmp(&right.object_count))
        .then_with(|| left.region_index.cmp(&right.region_index))
}

#[cfg(test)]
#[path = "old_test.rs"]
mod tests;
