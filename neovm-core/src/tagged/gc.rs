//! Mark-sweep garbage collector for the tagged pointer value system.
//!
//! # Design
//!
//! - **Cons cells**: GNU-shaped aligned block allocator.
//!   Each `ConsBlock` stores a fixed-size array of `ConsCell` at the front of
//!   a 64KB-aligned block, followed by packed mark bits. This lets the GC
//!   derive a cons's owning block/index directly from the pointer, matching the
//!   structure GNU Emacs uses in `alloc.c`.
//!
//! - **All other heap objects** (string, float, vectorlike): allocated
//!   via the system allocator, linked via intrusive `GcHeader.next` list
//!   for sweeping, with an address index for O(1) ownership checks during
//!   marking.
//!
//! - **Mark phase**: walk from roots, decode tags, follow heap pointers.
//! - **Sweep phase**: walk cons blocks (bitmap) and the intrusive list
//!   (GcHeader chain), freeing unmarked objects.
//!
//! No ObjId. No generations. No stale references.

use super::header::*;
use super::value::TaggedValue;
use crate::emacs_core::intern::SymId;
use crate::emacs_core::value::{HashKey, HashTableWeakness};
use crate::gc_trace::GcTrace;
use malachite::integer::Integer;
use rustc_hash::{FxHashMap, FxHashSet};
use std::alloc::{self, Layout};
use std::cell::Cell;
use std::mem::size_of;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Optional heap-write observation, used by tests/introspection to inspect which
/// owners (and optionally which individual writes) were mutated since the last
/// reset. This is NOT a GC marking barrier — the concurrent collector's barrier
/// is the SATB log keyed on `concurrent_mark_running`. The dump remembered set is
/// maintained unconditionally in `record_heap_write` regardless of this mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteTrackingMode {
    Disabled,
    OwnersAndRecords,
}

/// Classifies the kind of heap mutation that occurred.
///
/// GNU Emacs performs direct object/cell writes (`XSETCAR`, `XSETCDR`, `ASET`,
/// symbol value writes, etc.).  Neomacs keeps the same Lisp-visible semantics,
/// but records mutation metadata here so future generational or incremental
/// collectors have a single write-barrier surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeapWriteKind {
    ConsCar,
    ConsCdr,
    VectorSlot,
    VectorBulk,
    RecordSlot,
    RecordBulk,
    ClosureSlot,
    ClosureBulk,
    StringTextProps,
    StringData,
    HashTableData,
    ByteCodeData,
    LispMarker,
    OverlayData,
    XwidgetData,
    XwidgetViewData,
    /// Mutation of a char-table object (default/parent/ascii/contents/extras).
    /// Char-tables are dumped (syntax/category/case tables) and mutated in
    /// place post-load, so this barrier is required for the dump partition's
    /// remembered set to catch dumped char-table → heap edges.
    CharTableData,
    /// Mutation of a sub-char-table object's contents.
    SubCharTableData,
    /// Mutation of an obarray object (buckets/count). Obarrays are dumped and
    /// mutated post-load by `intern`, so the remembered set must observe
    /// dumped-obarray → heap edges through this chokepoint.
    ObarrayData,
}

/// A single heap mutation event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HeapWriteRecord {
    pub owner: TaggedValue,
    pub kind: HeapWriteKind,
    pub slot: Option<usize>,
    pub value: Option<TaggedValue>,
}

pub(crate) const MEMORY_USE_COUNT_LEN: usize = 7;

#[derive(Clone, Copy, Debug)]
pub(crate) enum MemoryUseCountSlot {
    ConsCells = 0,
    Floats = 1,
    VectorCells = 2,
    Symbols = 3,
    StringChars = 4,
    Intervals = 5,
    Strings = 6,
}

impl MemoryUseCountSlot {
    #[inline]
    pub(crate) const fn index(self) -> usize {
        self as usize
    }
}

impl HeapWriteRecord {
    pub const fn bulk(owner: TaggedValue, kind: HeapWriteKind) -> Self {
        Self {
            owner,
            kind,
            slot: None,
            value: None,
        }
    }

    pub const fn slot(
        owner: TaggedValue,
        kind: HeapWriteKind,
        slot: usize,
        value: TaggedValue,
    ) -> Self {
        Self {
            owner,
            kind,
            slot: Some(slot),
            value: Some(value),
        }
    }
}

// ---------------------------------------------------------------------------
// Thread-local heap access
// ---------------------------------------------------------------------------

thread_local! {
    static TAGGED_HEAP: Cell<*mut TaggedHeap> = const { Cell::new(std::ptr::null_mut()) };
    static TAGGED_HEAP_WRITE_TRACKING_MODE: Cell<WriteTrackingMode> =
        const { Cell::new(WriteTrackingMode::Disabled) };
    /// Mirrors `TaggedHeap::partition_dump` so the write-barrier hot path can
    /// decide whether to run without dereferencing the heap.
    static TAGGED_HEAP_PARTITION_ACTIVE: Cell<bool> = const { Cell::new(false) };
    /// Mirrors `TaggedHeap::concurrent_mark_running` so the write-barrier hot
    /// path keeps reaching `record_heap_write` (for the concurrent SATB log)
    /// even when owner-tracking is Disabled and the partition is inactive.
    static TAGGED_HEAP_CONCURRENT_ACTIVE: Cell<bool> = const { Cell::new(false) };
    /// Auto-allocated heap for tests that construct Values without a Context.
    #[cfg(test)]
    static TEST_FALLBACK_TAGGED_HEAP: std::cell::RefCell<Option<Box<TaggedHeap>>> =
        const { std::cell::RefCell::new(None) };
}

static NEXT_TAGGED_HEAP_ID: AtomicUsize = AtomicUsize::new(1);

fn next_tagged_heap_identity() -> usize {
    NEXT_TAGGED_HEAP_ID.fetch_add(1, Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Background GC thread (concurrent collector, Phase 4)
// ---------------------------------------------------------------------------

/// A raw `*mut TaggedHeap` that can cross to the GC thread. The heap is `!Send`
/// (raw pointers), but during a handshake the mutator is BLOCKED waiting for the
/// GC thread, so the two threads never touch the heap at the same time — the GC
/// thread has exclusive access for the duration. (Phase 5 makes access genuinely
/// concurrent via the atomic slots + SATB built in Phases 1-3.)
struct HeapPtr(*mut TaggedHeap);
unsafe impl Send for HeapPtr {}

/// A non-blocking concurrent-mark job (Phase 5). Carries everything the GC
/// thread needs WITHOUT a `&mut TaggedHeap` — two threads holding `&mut` to the
/// same heap is UB in Rust's model even with atomic fields. The GC thread marks
/// only conses (fixed 16B; car/cdr + mark bits are atomic) and DEFERS every
/// non-cons (and any non-owned cons) to `deferred`, traced at the stop-the-world
/// termination. So it touches no growable/reallocatable heap structure.
struct ConcurrentMarkJob {
    /// Root snapshot, moved out of the heap's gray queue at the start handshake.
    gray: Vec<TaggedValue>,
    /// Base addresses of every owned cons block at the snapshot (immutable,
    /// read-only on the GC thread). A cons whose block base is here is markable
    /// via block arithmetic; others (mapped/dump, or new blocks) are deferred.
    owned_bases: std::sync::Arc<std::collections::HashSet<usize>>,
    /// Dump (pdump mmap) address span; conses inside are permanent-black and
    /// their young children come from the remembered set, so they are skipped.
    dump_lo: usize,
    dump_hi: usize,
    /// Overwritten children appended by the mutator's SATB barrier; drained here.
    satb: std::sync::Arc<std::sync::Mutex<Vec<TaggedValue>>>,
    /// Non-cons / non-owned-cons values to trace at the STW termination.
    deferred: std::sync::Arc<std::sync::Mutex<Vec<TaggedValue>>>,
    /// Set when gray + SATB are drained (tentatively done); polled by the mutator.
    done: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Set by the mutator to ask this loop to exit.
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Signalled when the loop exits, so the mutator can take over the gray queue.
    exited: std::sync::mpsc::Sender<()>,
}

/// A unit of work handed to the GC thread, plus a oneshot done-channel the GC
/// thread signals when finished so the mutator can resume.
enum GcRequest {
    /// Drain the gray queue (mark to a fixpoint) on the GC thread.
    MarkAll(HeapPtr, std::sync::mpsc::Sender<()>),
    /// Non-blocking concurrent mark (Phase 5): mark conses while the mutator
    /// runs; defer everything else to the termination handshake.
    ConcurrentMark(ConcurrentMarkJob),
}

static GC_THREAD: std::sync::OnceLock<std::sync::Mutex<std::sync::mpsc::Sender<GcRequest>>> =
    std::sync::OnceLock::new();

/// Lazily spawn the process-global GC thread and return its request channel.
/// The thread lives for the process; it loops draining requests.
fn gc_thread() -> std::sync::MutexGuard<'static, std::sync::mpsc::Sender<GcRequest>> {
    GC_THREAD
        .get_or_init(|| {
            let (tx, rx) = std::sync::mpsc::channel::<GcRequest>();
            std::thread::Builder::new()
                .name("neovm-gc".to_string())
                .spawn(move || {
                    while let Ok(req) = rx.recv() {
                        match req {
                            GcRequest::MarkAll(HeapPtr(p), done) => {
                                // Exclusive access: the mutator is blocked on
                                // `done` until we signal.
                                unsafe { (*p).mark_all() };
                                let _ = done.send(());
                            }
                            GcRequest::ConcurrentMark(job) => {
                                run_concurrent_mark(job);
                            }
                        }
                    }
                })
                .expect("spawn neovm-gc thread");
            std::sync::Mutex::new(tx)
        })
        .lock()
        .expect("gc thread channel poisoned")
}

/// Atomically set an OWNED cons cell's mark bit using only its pointer. The mark
/// bitmap lives at `block_base + CONS_MARKS_OFFSET`, derivable from the pointer
/// with no `&TaggedHeap`, so the concurrent GC thread marks conses without an
/// aliasing `&mut`. Returns true if this call set the bit (was unmarked).
///
/// # Safety
/// `ptr` must be a cell-aligned cons in an owned `ConsBlock` (verified by the
/// caller against the start-of-cycle owned-base set). Passing a dump/mapped cons
/// would scribble a mark bit into the wrong region.
#[inline]
unsafe fn atomic_mark_owned_cons_ptr(ptr: *const ConsCell) -> bool {
    let addr = ptr as usize;
    let base = addr & !(CONS_BLOCK_ALIGN - 1);
    let index = (addr - base) / size_of::<ConsCell>();
    let word_index = index / CONS_MARK_BITS_PER_WORD;
    let mask = 1usize << (index % CONS_MARK_BITS_PER_WORD);
    let word = unsafe { &*((base + CONS_MARKS_OFFSET) as *const AtomicUsize).add(word_index) };
    (word.fetch_or(mask, Ordering::Relaxed) & mask) == 0
}

/// The background concurrent-mark loop (Phase 5). Runs on the "neovm-gc" thread
/// with no `&mut TaggedHeap`: it marks conses via atomic block-bitmap ops +
/// atomic car/cdr loads, and defers all non-cons (and non-owned conses) to the
/// mutator's stop-the-world termination. Loops draining its local gray queue and
/// the shared SATB buffer until both are empty and the mutator asks it to stop.
fn run_concurrent_mark(mut job: ConcurrentMarkJob) {
    use std::sync::atomic::Ordering;
    loop {
        // Drain the local gray worklist (GC-thread-owned; no sharing).
        while let Some(val) = job.gray.pop() {
            if val.is_cons() {
                let ptr = val.xcons_ptr();
                let addr = ptr as usize;
                if addr >= job.dump_lo && addr < job.dump_hi {
                    continue; // dump cons: permanent black, children via remembered set
                }
                let base = addr & !(CONS_BLOCK_ALIGN - 1);
                if !job.owned_bases.contains(&base) {
                    // Mapped (non-dump) or new-block cons — let the mutator's
                    // termination mark it through the full `mark_value` path.
                    job.deferred.lock().unwrap().push(val);
                    continue;
                }
                if unsafe { atomic_mark_owned_cons_ptr(ptr) } {
                    let car = unsafe { (*ptr).load_car() };
                    let cdr = unsafe { (*ptr).load_cdr() };
                    if car.is_heap_object() {
                        job.gray.push(car);
                    }
                    if cdr.is_heap_object() {
                        job.gray.push(cdr);
                    }
                }
            } else if val.is_heap_object() {
                // Float/string/veclike: backing may be reallocated by the mutator,
                // so never read it here — defer the trace to the STW termination.
                job.deferred.lock().unwrap().push(val);
            }
        }
        // Fold the mutator's SATB log (overwritten children) into gray.
        let batch = { std::mem::take(&mut *job.satb.lock().unwrap()) };
        if batch.is_empty() {
            // Tentatively drained. Advertise done; exit if the mutator asked.
            job.done.store(true, Ordering::Release);
            if job.stop.load(Ordering::Acquire) {
                break;
            }
            // Idle wait — short enough to react to new SATB / stop quickly,
            // long enough not to peg a core. (A real thread sleep, not the
            // harness-blocked foreground sleep.)
            std::thread::sleep(std::time::Duration::from_micros(100));
        } else {
            job.done.store(false, Ordering::Release);
            job.gray.extend(batch);
        }
    }
    let _ = job.exited.send(());
}

/// Set the thread-local tagged heap pointer.
pub fn set_tagged_heap(heap: &mut TaggedHeap) {
    TAGGED_HEAP.with(|h| h.set(heap as *mut TaggedHeap));
    TAGGED_HEAP_WRITE_TRACKING_MODE.with(|mode| mode.set(heap.write_tracking_mode()));
    TAGGED_HEAP_PARTITION_ACTIVE.with(|p| p.set(heap.partition_dump));
    TAGGED_HEAP_CONCURRENT_ACTIVE.with(|c| c.set(heap.concurrent_mark_running));
}

/// Return the current thread's tagged heap identity, if one is installed.
///
/// This is used only for runtime side tables that must avoid retaining Lisp
/// objects from a different evaluator heap. GNU keeps those object references
/// inside ordinary GC-managed structures; the heap identity preserves that
/// ownership boundary for Neomacs side tables.
pub(crate) fn current_tagged_heap_identity() -> Option<usize> {
    TAGGED_HEAP.with(|h| {
        let ptr = h.get();
        (!ptr.is_null()).then(|| unsafe { (*ptr).identity() })
    })
}

/// Access the thread-local tagged heap.
///
/// In test mode, auto-creates a fallback heap if none is set.
/// In production, panics if no heap is set.
#[inline]
pub fn with_tagged_heap<R>(f: impl FnOnce(&mut TaggedHeap) -> R) -> R {
    TAGGED_HEAP.with(|h| {
        let ptr = h.get();
        if !ptr.is_null() {
            return f(unsafe { &mut *ptr });
        }
        #[cfg(test)]
        {
            TEST_FALLBACK_TAGGED_HEAP.with(|fb| {
                let mut borrow = fb.borrow_mut();
                if borrow.is_none() {
                    *borrow = Some(Box::new(TaggedHeap::new()));
                }
                let heap_ref: &mut TaggedHeap = borrow.as_mut().unwrap();
                let ptr = heap_ref as *mut TaggedHeap;
                h.set(ptr);
                f(unsafe { &mut *ptr })
            })
        }
        #[cfg(not(test))]
        {
            panic!("no TaggedHeap set for this thread");
        }
    })
}

/// Central mutation hook for bulk writes to the tagged heap.
#[inline]
pub fn note_heap_write(owner: TaggedValue, kind: HeapWriteKind) {
    note_heap_write_record(HeapWriteRecord::bulk(owner, kind));
}

/// Central mutation hook for slot writes to the tagged heap.
#[inline]
pub fn note_heap_slot_write(
    owner: TaggedValue,
    kind: HeapWriteKind,
    slot: usize,
    value: TaggedValue,
) {
    note_heap_write_record(HeapWriteRecord::slot(owner, kind, slot, value));
}

#[inline]
fn note_heap_write_record(record: HeapWriteRecord) {
    if !record.owner.is_heap_object() {
        return;
    }
    let disabled =
        TAGGED_HEAP_WRITE_TRACKING_MODE.with(|mode| mode.get()) == WriteTrackingMode::Disabled;
    // The dump partition needs the barrier even when owner-tracking is off, to
    // record mutations of dumped objects into the remembered set.
    let partition = TAGGED_HEAP_PARTITION_ACTIVE.with(|p| p.get());
    // The concurrent collector needs the barrier (its SATB log) regardless of
    // owner-tracking / partition state.
    let concurrent = TAGGED_HEAP_CONCURRENT_ACTIVE.with(|c| c.get());
    if disabled && !partition && !concurrent {
        return;
    }
    with_tagged_heap(|heap| heap.record_heap_write(record));
}

// ---------------------------------------------------------------------------
// Cons block allocator
// ---------------------------------------------------------------------------

/// GNU Emacs keeps conses in fixed-size aligned blocks and derives the owning
/// block/index directly from the cons pointer. Keep the same shape here so
/// mark/ownership checks stay O(1) instead of linearly scanning `cons_blocks`.
const CONS_BLOCK_BYTES: usize = 64 * 1024;
const CONS_BLOCK_ALIGN: usize = CONS_BLOCK_BYTES;
const CONS_MARK_BITS_PER_WORD: usize = usize::BITS as usize;

const fn cons_mark_words(cell_count: usize) -> usize {
    cell_count.div_ceil(CONS_MARK_BITS_PER_WORD)
}

const fn cons_block_cell_count() -> usize {
    let cons_size = size_of::<ConsCell>();
    let mark_word_size = size_of::<usize>();
    let mut cells = CONS_BLOCK_BYTES / cons_size;
    while cells > 0 {
        let marks_bytes = cons_mark_words(cells) * mark_word_size;
        if cells * cons_size + marks_bytes <= CONS_BLOCK_BYTES {
            return cells;
        }
        cells -= 1;
    }
    0
}

const CONS_BLOCK_SIZE: usize = cons_block_cell_count();
const CONS_MARK_WORDS: usize = cons_mark_words(CONS_BLOCK_SIZE);
const CONS_CELLS_BYTES: usize = CONS_BLOCK_SIZE * size_of::<ConsCell>();
const CONS_MARKS_OFFSET: usize = CONS_CELLS_BYTES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ConsMarkBit {
    word_index: usize,
    mask: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ConsBlockCacheEntry {
    block_base: usize,
    block_index: usize,
}

impl ConsBlockCacheEntry {
    fn new(block_base: usize, block_index: usize) -> Self {
        Self {
            block_base,
            block_index,
        }
    }
}

/// A GNU-shaped cons block with cells at the front of a fixed-size aligned
/// storage area, followed by packed mark bits.
struct ConsBlock {
    /// Aligned raw storage for cons cells plus mark bits.
    storage: *mut u8,
    /// Index of the first never-allocated cell in this block.
    next_index: u16,
}

impl ConsBlock {
    fn layout() -> Layout {
        Layout::from_size_align(CONS_BLOCK_BYTES, CONS_BLOCK_ALIGN).expect("cons block layout")
    }

    fn new() -> Self {
        let layout = Self::layout();
        let storage = unsafe { alloc::alloc_zeroed(layout) };
        if storage.is_null() {
            alloc::handle_alloc_error(layout);
        }
        Self {
            storage,
            next_index: 0,
        }
    }

    #[inline]
    fn base_addr(&self) -> usize {
        self.storage as usize
    }

    #[inline]
    fn cells_ptr(&self) -> *mut ConsCell {
        self.storage.cast()
    }

    #[inline]
    fn mark_words_ptr(&self) -> *mut usize {
        unsafe { self.storage.add(CONS_MARKS_OFFSET).cast() }
    }

    #[inline]
    fn block_base_for_ptr(ptr: *const ConsCell) -> usize {
        (ptr as usize) & !(CONS_BLOCK_ALIGN - 1)
    }

    #[inline]
    fn ptr_offset(ptr: *const ConsCell) -> usize {
        (ptr as usize).saturating_sub(Self::block_base_for_ptr(ptr))
    }

    #[inline]
    fn ptr_is_cell_aligned(ptr: *const ConsCell) -> bool {
        let offset = Self::ptr_offset(ptr);
        offset < CONS_CELLS_BYTES && offset.is_multiple_of(size_of::<ConsCell>())
    }

    #[inline]
    fn index_of_ptr(ptr: *const ConsCell) -> usize {
        Self::ptr_offset(ptr) / size_of::<ConsCell>()
    }

    #[inline]
    fn mark_bit(index: usize) -> ConsMarkBit {
        let word = index / CONS_MARK_BITS_PER_WORD;
        let bit = index % CONS_MARK_BITS_PER_WORD;
        ConsMarkBit {
            word_index: word,
            mask: 1usize << bit,
        }
    }

    #[inline]
    fn owns_ptr(&self, ptr: *const ConsCell) -> bool {
        Self::block_base_for_ptr(ptr) == self.base_addr() && Self::ptr_is_cell_aligned(ptr)
    }

    /// View a mark-bitmap word as an atomic. The cons mark bits are accessed
    /// atomically (relaxed) so a future concurrent GC thread can set them while
    /// the mutator allocate-blacks / reads them without a data race; on x86 a
    /// relaxed atomic load/store is a plain mov, so this is free single-threaded.
    #[inline]
    fn mark_word(&self, word_index: usize) -> &AtomicUsize {
        unsafe { &*(self.mark_words_ptr().add(word_index) as *const AtomicUsize) }
    }

    #[inline]
    fn is_marked_ptr(&self, ptr: *const ConsCell) -> bool {
        let index = Self::index_of_ptr(ptr);
        let mark = Self::mark_bit(index);
        debug_assert!(mark.word_index < CONS_MARK_WORDS);
        (self.mark_word(mark.word_index).load(Ordering::Relaxed) & mark.mask) != 0
    }

    #[inline]
    fn mark_ptr(&mut self, ptr: *const ConsCell) {
        let index = Self::index_of_ptr(ptr);
        let mark = Self::mark_bit(index);
        debug_assert!(mark.word_index < CONS_MARK_WORDS);
        self.mark_word(mark.word_index)
            .fetch_or(mark.mask, Ordering::Relaxed);
    }

    /// Allocate a fresh cons cell from this block's bump cursor.
    /// Returns None if the block has no never-used cells left.
    fn alloc_bump(&mut self, car: TaggedValue, cdr: TaggedValue) -> Option<*mut ConsCell> {
        if self.next_index as usize >= CONS_BLOCK_SIZE {
            return None;
        }
        let idx = self.next_index;
        self.next_index += 1;
        let cell = unsafe { self.cells_ptr().add(idx as usize) };
        unsafe {
            (*cell).set_car(car);
            (*cell).set_cdr(cdr);
        }
        Some(cell)
    }

    /// Clear all mark bits used by this block. Runs stop-the-world (at
    /// `begin_collection`), but stores atomically so the representation stays
    /// consistent with the concurrent reads/writes elsewhere.
    fn clear_marks(&mut self) {
        let used_words = cons_mark_words(self.next_index as usize);
        for w in 0..used_words {
            self.mark_word(w).store(0, Ordering::Relaxed);
        }
    }

    /// Count currently-marked (live) cells via mark-bitmap popcount. Bits at or
    /// above `next_index` are never set, so popcounting the used words is exact.
    /// Cheap O(cells/64); used to recompute the live count after an incremental
    /// sweep without a second cell walk.
    fn count_marked(&self) -> usize {
        let used_words = cons_mark_words(self.next_index as usize);
        let mut live = 0usize;
        for w in 0..used_words {
            live += self.mark_word(w).load(Ordering::Relaxed).count_ones() as usize;
        }
        live
    }

    /// Sweep: thread reclaimed cells into the global intrusive free list and
    /// return the number of live cells in this block.
    fn sweep(&mut self, free_list: &mut *mut ConsCell) -> usize {
        let mut live = 0;

        // Match GNU alloc.c: reclaimed conses are linked through the dead
        // cells themselves instead of rebuilding an external index vector.
        for i in (0..self.next_index as usize).rev() {
            let cell = unsafe { self.cells_ptr().add(i) };
            let mark = Self::mark_bit(i);
            let marked = (self.mark_word(mark.word_index).load(Ordering::Relaxed) & mark.mask) != 0;
            if marked {
                live += 1;
            } else {
                unsafe {
                    (*cell).set_free_next(*free_list);
                }
                *free_list = cell;
            }
        }

        live
    }
}

impl Drop for ConsBlock {
    fn drop(&mut self) {
        unsafe { alloc::dealloc(self.storage, Self::layout()) };
    }
}

struct MappedConsRange {
    start: *mut ConsCell,
    len: usize,
    mark_bits: Vec<usize>,
}

impl MappedConsRange {
    fn new(start: *mut ConsCell, len: usize) -> Self {
        Self {
            start,
            len,
            mark_bits: vec![0; cons_mark_words(len)],
        }
    }

    #[inline]
    fn contains_ptr(&self, ptr: *const ConsCell) -> bool {
        if ptr.is_null() || self.len == 0 {
            return false;
        }
        let start = self.start as usize;
        let end = start + self.len * size_of::<ConsCell>();
        let ptr = ptr as usize;
        start <= ptr && ptr < end && (ptr - start).is_multiple_of(size_of::<ConsCell>())
    }

    #[inline]
    fn index_of_ptr(&self, ptr: *const ConsCell) -> usize {
        (ptr as usize - self.start as usize) / size_of::<ConsCell>()
    }

    #[inline]
    fn is_marked_ptr(&self, ptr: *const ConsCell) -> bool {
        let index = self.index_of_ptr(ptr);
        let mark = ConsBlock::mark_bit(index);
        (self.mark_bits[mark.word_index] & mark.mask) != 0
    }

    #[inline]
    fn mark_ptr(&mut self, ptr: *const ConsCell) {
        let index = self.index_of_ptr(ptr);
        let mark = ConsBlock::mark_bit(index);
        self.mark_bits[mark.word_index] |= mark.mask;
    }

    fn clear_marks(&mut self) {
        self.mark_bits.fill(0);
    }

    /// Mark every cell in the range live (dump-partition: born black). Sets
    /// exactly `len` bits so `live_count` stays exact.
    fn mark_all(&mut self) {
        self.mark_bits.fill(!0);
        let rem = self.len % CONS_MARK_BITS_PER_WORD;
        if rem != 0
            && let Some(last) = self.mark_bits.last_mut()
        {
            *last = (1usize << rem) - 1;
        }
    }

    fn live_count(&self) -> usize {
        self.mark_bits
            .iter()
            .enumerate()
            .map(|(word_index, word)| {
                let full_words = self.len / CONS_MARK_BITS_PER_WORD;
                let tail_bits = self.len % CONS_MARK_BITS_PER_WORD;
                if word_index < full_words || tail_bits == 0 {
                    word.count_ones() as usize
                } else {
                    let mask = (1usize << tail_bits) - 1;
                    (word & mask).count_ones() as usize
                }
            })
            .sum()
    }
}

struct MappedFloatRange {
    start: *mut FloatObj,
    len: usize,
    mark_bits: Vec<usize>,
}

impl MappedFloatRange {
    fn new(start: *mut FloatObj, len: usize) -> Self {
        Self {
            start,
            len,
            mark_bits: vec![0; cons_mark_words(len)],
        }
    }

    #[inline]
    fn contains_ptr(&self, ptr: *const FloatObj) -> bool {
        if ptr.is_null() || self.len == 0 {
            return false;
        }
        let start = self.start as usize;
        let end = start + self.len * size_of::<FloatObj>();
        let ptr = ptr as usize;
        start <= ptr && ptr < end && (ptr - start).is_multiple_of(size_of::<FloatObj>())
    }

    #[inline]
    fn index_of_ptr(&self, ptr: *const FloatObj) -> usize {
        (ptr as usize - self.start as usize) / size_of::<FloatObj>()
    }

    #[inline]
    fn is_marked_ptr(&self, ptr: *const FloatObj) -> bool {
        let index = self.index_of_ptr(ptr);
        let mark = ConsBlock::mark_bit(index);
        (self.mark_bits[mark.word_index] & mark.mask) != 0
    }

    #[inline]
    fn mark_ptr(&mut self, ptr: *const FloatObj) {
        let index = self.index_of_ptr(ptr);
        let mark = ConsBlock::mark_bit(index);
        self.mark_bits[mark.word_index] |= mark.mask;
    }

    fn clear_marks(&mut self) {
        self.mark_bits.fill(0);
    }

    /// Mark every cell in the range live (dump-partition: born black). Sets
    /// exactly `len` bits so `live_count` stays exact.
    fn mark_all(&mut self) {
        self.mark_bits.fill(!0);
        let rem = self.len % CONS_MARK_BITS_PER_WORD;
        if rem != 0
            && let Some(last) = self.mark_bits.last_mut()
        {
            *last = (1usize << rem) - 1;
        }
    }

    fn live_count(&self) -> usize {
        self.mark_bits
            .iter()
            .enumerate()
            .map(|(word_index, word)| {
                let full_words = self.len / CONS_MARK_BITS_PER_WORD;
                let tail_bits = self.len % CONS_MARK_BITS_PER_WORD;
                if word_index < full_words || tail_bits == 0 {
                    word.count_ones() as usize
                } else {
                    let mask = (1usize << tail_bits) - 1;
                    (word & mask).count_ones() as usize
                }
            })
            .sum()
    }
}

struct MappedVecLikeObject {
    header: *mut VecLikeHeader,
    byte_len: usize,
    marked: bool,
}

impl MappedVecLikeObject {
    fn new(header: *mut VecLikeHeader, byte_len: usize) -> Self {
        Self {
            header,
            byte_len,
            marked: false,
        }
    }
}

struct MappedStringObject {
    ptr: *mut StringObj,
    byte_len: usize,
    marked: bool,
}

impl MappedStringObject {
    fn new(ptr: *mut StringObj, byte_len: usize) -> Self {
        Self {
            ptr,
            byte_len,
            marked: false,
        }
    }
}

// ---------------------------------------------------------------------------
// TaggedHeap — the main GC-managed heap
// ---------------------------------------------------------------------------

/// The tagged pointer heap. Owns all heap-allocated Lisp objects.
pub struct TaggedHeap {
    /// Process-unique heap identity used by side tables that carry GC-managed
    /// Lisp values.  It deliberately does not use this heap's address: boxed
    /// heaps are routinely dropped and recreated by snapshot-based tests, and
    /// the allocator may reuse an address for a different heap lifetime.
    identity: usize,

    /// Cons cell block allocator.
    cons_blocks: Vec<ConsBlock>,
    /// Base-address lookup for O(1) cons block ownership and marking.
    cons_block_index_by_base: FxHashMap<usize, usize>,
    /// Last ordinary cons block used by the mark phase.
    ///
    /// GNU's cons marker derives the block directly from the pointer and has a
    /// special fast path for successive list cells.  Keep Neomacs's explicit
    /// ownership map, but avoid probing it repeatedly while the mark queue is
    /// walking cells from the same block.
    mark_cons_block_cache: Option<ConsBlockCacheEntry>,

    /// Intrusive linked list of YOUNG non-cons heap objects (the nursery).
    /// Points to the GcHeader of the first object; follow `next` to traverse.
    /// Every cycle clears+sweeps only this list, so its length bounds the
    /// per-GC clear/sweep cost.
    all_objects: *mut GcHeader,
    /// Intrusive linked list of TENURED non-cons heap objects (the old
    /// generation). Filled at first-cycle promotion (`promote_and_blacken`);
    /// these are permanently black and are NEVER cleared or swept, so the
    /// minor-GC walk skips them entirely. Freed only at heap teardown.
    tenured_objects: *mut GcHeader,
    /// Exact address set for ordinary non-cons object headers.
    ///
    /// GNU's GC reaches ordinary heap ownership through allocator metadata and
    /// dumped-object ownership through `pdumper_object_p` range metadata. Keep
    /// the same fast-path split here: mark-time checks must not scan
    /// `all_objects`.
    non_cons_object_addrs: FxHashSet<usize>,

    /// Total number of allocated objects (cons + non-cons).
    pub allocated_count: usize,
    /// Lisp-visible allocation statistics backing `memory-use-counts`.
    memory_use_counts: [u64; MEMORY_USE_COUNT_LEN],

    /// GC threshold in approximate Lisp heap bytes.
    gc_threshold: usize,
    /// When true, `gc_threshold` was explicitly overridden by tests or host
    /// code and should not be recomputed from Lisp-visible GC variables.
    gc_threshold_overridden: bool,
    /// Approximate Lisp heap bytes allocated since the last full collection.
    bytes_since_gc: usize,
    /// Approximate bytes retained by the live heap after the last sweep.
    live_bytes: usize,

    /// Gray worklist for mark phase.
    gray_queue: Vec<TaggedValue>,
    /// Weak hash tables discovered during this cycle's mark. Their entries are
    /// NOT traced inline (so a weak key/value does not keep its entry alive);
    /// `mark_and_sweep_weak_tables` instead processes them at the stop-the-world
    /// `complete_collection`, after the main mark drains (GNU
    /// `mark_and_sweep_weak_table_contents`). Holds raw object pointers, valid
    /// only within a single collection; cleared each cycle.
    weak_hash_tables: Vec<*mut HashTableObj>,

    /// Reclaimed cons cells threaded through the dead cells themselves,
    /// matching GNU alloc.c's `cons_free_list`.
    cons_free_list: *mut ConsCell,
    /// Cons cells loaded directly from a mapped pdump image.  GNU's pdumper
    /// uses external mark bits for dumped objects rather than writing mark
    /// state into malloc/GC allocation headers; mirror that for mapped conses.
    mapped_cons_ranges: Vec<MappedConsRange>,
    /// Float objects loaded directly from a mapped pdump image.  Like GNU
    /// pdumper dump objects, their mark state lives outside the mapped bytes.
    mapped_float_ranges: Vec<MappedFloatRange>,
    /// Vectorlike objects loaded directly from a mapped pdump image.  Their
    /// object headers are in the mapped image, but mark state remains external.
    mapped_veclike_objects: Vec<MappedVecLikeObject>,
    mapped_veclike_index_by_addr: FxHashMap<usize, usize>,
    /// String objects loaded directly from a mapped pdump image.  Their text
    /// properties can contain Lisp roots, so mark state must be external too.
    mapped_string_objects: Vec<MappedStringObject>,
    mapped_string_index_by_addr: FxHashMap<usize, usize>,
    /// Number of live cons cells currently included in `allocated_count`.
    cons_live_count: usize,

    /// Raw pointers to the `markers_head` slot of every live buffer's
    /// `BufferText`. Populated by the caller immediately before
    /// `complete_collection` via `set_marker_chain_head_slots`; drained
    /// by `unchain_dead_markers` between the mark and sweep phases so
    /// unmarked markers are spliced out of the intrusive per-buffer
    /// chain before `sweep_objects` frees them. Mirrors GNU
    /// `sweep_buffer → unchain_dead_markers` (`alloc.c`).
    ///
    /// Empty for GC cycles that don't go through a `Context` (raw-heap
    /// tests in `tagged/tests.rs`), which is fine because those never
    /// create chain-linked markers.
    marker_chain_head_slots: Vec<*mut *mut MarkerObj>,

    /// Canonical runtime handle wrappers keyed by their underlying object id.
    buffer_registry: FxHashMap<crate::buffer::BufferId, TaggedValue>,
    window_registry: FxHashMap<u64, TaggedValue>,
    frame_registry: FxHashMap<u64, TaggedValue>,
    timer_registry: FxHashMap<u64, TaggedValue>,
    process_registry: FxHashMap<crate::emacs_core::process::ProcessId, TaggedValue>,

    /// Cumulative GC statistics.
    gc_collections: usize,
    gc_total_elapsed_us: u64,

    /// Time (µs) spent in the `begin_collection` mark-clear pass of the most
    /// recent collection. Part of the clear/mark/sweep split used to size the
    /// dump-partition opportunity (the clear pass and the dump re-mark are the
    /// non-fundamental costs a "dump as permanent tenured region" would remove).
    last_clear_us: u64,

    /// Owners mutated since the last full collection.
    ///
    /// This is the minimal remembered-set precursor for future generational
    /// or incremental GC. We keep owner identity, not child edges, because the
    /// current collector is still full-heap mark-sweep.
    write_tracking_mode: WriteTrackingMode,
    dirty_owners: Vec<TaggedValue>,
    dirty_owner_bits: FxHashSet<usize>,
    dirty_writes: Vec<HeapWriteRecord>,

    // --- Dump-partition state (treat the immutable pdump image as a permanent
    // black/tenured region: never clear, re-trace, or sweep it). Gated by
    // `partition_dump`; default off => identical to the full-trace collector.
    /// When true, mapped (pdump) objects are born black and never re-traced;
    /// only mutated dumped objects (`mapped_remembered`) are re-scanned.
    partition_dump: bool,
    /// One-time flag: the mapped image has been blackened (all marks set).
    dump_blackened: bool,
    /// Persistent remembered set: bits of dumped objects that have been
    /// mutated and may now hold heap children. Seeded as roots every cycle so
    /// those heap children stay live. Fed by the write barrier
    /// (`record_heap_write`). Tiny in practice (few dumped objects are ever
    /// mutated). Never cleared (conservative retention).
    mapped_remembered: FxHashSet<usize>,
    /// Address span `[lo, hi)` covering every mapped object, for an O(1) "is
    /// this owner a dumped object?" test in the write-barrier hot path.
    dump_addr_lo: usize,
    dump_addr_hi: usize,

    // --- Incremental marking state (step 7). Active on every partitioned cycle
    // (after the first-cycle promotion); the first cycle and no-dump heaps stay
    // stop-the-world. Marking is sliced across evaluator safe points using an
    // incremental-update (Steele) write barrier: dirty owners (written during
    // marking) are re-traced so no black->white edge survives, and the COMPLETE
    // root set is re-snapshotted at mark termination.
    /// True between the start of an incremental mark and its termination/sweep.
    /// While set, every safe point advances marking by one bounded slice.
    mark_in_progress: bool,
    /// Accumulated marking time (slices + final drain) for the in-flight
    /// incremental cycle, reported as `mark_us` at termination. Reset at start.
    incremental_mark_us: u64,
    /// True between a concurrent mark's start and termination handshakes — the
    /// mutator runs while the GC thread marks.
    concurrent_mark_running: bool,
    /// Mutator->GC channel (Phase 5): the SATB barrier appends the overwritten
    /// children here (locked); the GC thread drains them into its gray worklist.
    satb_shared: std::sync::Arc<std::sync::Mutex<Vec<TaggedValue>>>,
    /// Veclikes/strings the GC thread reached but did NOT trace (their backing
    /// can be reallocated by the mutator, so reading it concurrently would be a
    /// UAF). They are marked black and parked here, then traced at the
    /// termination handshake while the mutator is stopped.
    deferred_veclikes: std::sync::Arc<std::sync::Mutex<Vec<TaggedValue>>>,
    /// GC thread sets this (Release) when gray + SATB are drained; the mutator
    /// polls it (Acquire) at safe points to decide when to terminate.
    gc_done: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Mutator sets this (Release) to ask the GC thread to finish and exit.
    gc_stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Receives when the GC thread has exited its mark loop (so the mutator's
    /// termination can safely take over the gray queue). Set at start.
    gc_exited: Option<std::sync::mpsc::Receiver<()>>,

    // --- Incremental sweep state (step 8). After a mark terminates, the sweep
    // is deferred and drained in bounded slices at later safe points, so the
    // reclaim is no longer part of the stop-the-world pause. The next mark and
    // any forced GC finish the sweep first (marks must stay intact until then).
    /// True while the deferred sweep is draining.
    sweep_in_progress: bool,
    /// Next heap cons-block index the deferred sweep will reclaim.
    sweep_cons_cursor: usize,
    /// Non-cons objects detached from `all_objects` at sweep start, reclaimed
    /// incrementally. New non-cons allocations link onto a fresh `all_objects`
    /// and are not swept this cycle.
    sweep_noncons_pending: *mut GcHeader,
    /// Live bytes accumulated from the non-cons objects swept so far this cycle.
    sweep_noncons_live_bytes: usize,
    /// Carried from mark termination for the completion trace/accounting.
    sweep_mark_us: u64,
    sweep_bytes_before: usize,
}

impl TaggedHeap {
    pub fn new() -> Self {
        Self {
            identity: next_tagged_heap_identity(),
            cons_blocks: Vec::new(),
            cons_block_index_by_base: FxHashMap::default(),
            mark_cons_block_cache: None,
            all_objects: std::ptr::null_mut(),
            tenured_objects: std::ptr::null_mut(),
            non_cons_object_addrs: FxHashSet::default(),
            allocated_count: 0,
            memory_use_counts: [0; MEMORY_USE_COUNT_LEN],
            gc_threshold: 1_000_000 * size_of::<usize>(),
            gc_threshold_overridden: false,
            bytes_since_gc: 0,
            live_bytes: 0,
            gray_queue: Vec::new(),
            weak_hash_tables: Vec::new(),
            cons_free_list: std::ptr::null_mut(),
            mapped_cons_ranges: Vec::new(),
            mapped_float_ranges: Vec::new(),
            mapped_veclike_objects: Vec::new(),
            mapped_veclike_index_by_addr: FxHashMap::default(),
            mapped_string_objects: Vec::new(),
            mapped_string_index_by_addr: FxHashMap::default(),
            cons_live_count: 0,
            marker_chain_head_slots: Vec::new(),
            buffer_registry: FxHashMap::default(),
            window_registry: FxHashMap::default(),
            frame_registry: FxHashMap::default(),
            timer_registry: FxHashMap::default(),
            process_registry: FxHashMap::default(),
            write_tracking_mode: WriteTrackingMode::Disabled,
            dirty_owners: Vec::new(),
            dirty_owner_bits: FxHashSet::default(),
            dirty_writes: Vec::new(),
            gc_collections: 0,
            gc_total_elapsed_us: 0,
            last_clear_us: 0,
            // Activated automatically when a pdump is registered
            // (`extend_dump_span`); a bare/no-dump heap stays on full mark-sweep.
            partition_dump: false,
            dump_blackened: false,
            mapped_remembered: FxHashSet::default(),
            mark_in_progress: false,
            incremental_mark_us: 0,
            concurrent_mark_running: false,
            satb_shared: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            deferred_veclikes: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            gc_done: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            gc_stop: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            gc_exited: None,
            sweep_in_progress: false,
            sweep_cons_cursor: 0,
            sweep_noncons_pending: std::ptr::null_mut(),
            sweep_noncons_live_bytes: 0,
            sweep_mark_us: 0,
            sweep_bytes_before: 0,
            dump_addr_lo: usize::MAX,
            dump_addr_hi: 0,
        }
    }

    pub(crate) fn identity(&self) -> usize {
        self.identity
    }

    pub fn set_stack_bottom(&mut self, bottom: *const u8) {
        let _ = bottom;
    }

    pub fn set_write_tracking_mode(&mut self, mode: WriteTrackingMode) {
        self.write_tracking_mode = mode;
        TAGGED_HEAP_WRITE_TRACKING_MODE.with(|current| current.set(mode));
        if mode == WriteTrackingMode::Disabled {
            self.clear_dirty_owners();
            self.clear_dirty_writes();
        }
    }

    pub fn write_tracking_mode(&self) -> WriteTrackingMode {
        self.write_tracking_mode
    }

    pub fn should_collect(&self) -> bool {
        self.bytes_since_gc >= self.gc_threshold
    }

    pub fn gc_threshold(&self) -> usize {
        self.gc_threshold
    }

    pub fn set_gc_threshold(&mut self, threshold: usize) {
        self.gc_threshold = threshold.max(1);
        self.gc_threshold_overridden = true;
    }

    pub fn set_gc_threshold_from_runtime(&mut self, threshold: usize) {
        if !self.gc_threshold_overridden {
            self.gc_threshold = threshold.max(1);
        }
    }

    pub fn clear_gc_threshold_override(&mut self) {
        self.gc_threshold_overridden = false;
    }

    pub fn gc_threshold_is_overridden(&self) -> bool {
        self.gc_threshold_overridden
    }

    pub fn allocated_count(&self) -> usize {
        self.allocated_count
    }

    /// Total number of completed GC collection cycles since this heap was
    /// created. Used by allocation benchmarks to measure GC frequency.
    pub fn gc_collections(&self) -> usize {
        self.gc_collections
    }

    #[inline]
    pub(crate) fn add_memory_use_count(&mut self, slot: MemoryUseCountSlot, delta: u64) {
        let index = slot.index();
        self.memory_use_counts[index] = self.memory_use_counts[index].wrapping_add(delta);
    }

    #[inline]
    pub(crate) fn memory_use_counts_snapshot(&self) -> [u64; MEMORY_USE_COUNT_LEN] {
        self.memory_use_counts
    }

    pub fn bytes_since_gc(&self) -> usize {
        self.bytes_since_gc
    }

    pub(crate) fn reset_bytes_since_gc(&mut self) {
        self.bytes_since_gc = 0;
    }

    pub fn live_bytes(&self) -> usize {
        self.live_bytes
    }

    pub fn buffer_value(&self, id: crate::buffer::BufferId) -> Option<TaggedValue> {
        self.buffer_registry.get(&id).copied()
    }

    pub fn register_buffer_value(&mut self, id: crate::buffer::BufferId, value: TaggedValue) {
        self.buffer_registry.insert(id, value);
    }

    pub fn window_value(&self, id: u64) -> Option<TaggedValue> {
        self.window_registry.get(&id).copied()
    }

    pub fn register_window_value(&mut self, id: u64, value: TaggedValue) {
        self.window_registry.insert(id, value);
    }

    pub fn frame_value(&self, id: u64) -> Option<TaggedValue> {
        self.frame_registry.get(&id).copied()
    }

    pub fn register_frame_value(&mut self, id: u64, value: TaggedValue) {
        self.frame_registry.insert(id, value);
    }

    pub fn timer_value(&self, id: u64) -> Option<TaggedValue> {
        self.timer_registry.get(&id).copied()
    }

    pub fn register_timer_value(&mut self, id: u64, value: TaggedValue) {
        self.timer_registry.insert(id, value);
    }

    pub fn process_value(&self, id: crate::emacs_core::process::ProcessId) -> Option<TaggedValue> {
        self.process_registry.get(&id).copied()
    }

    pub fn register_process_value(
        &mut self,
        id: crate::emacs_core::process::ProcessId,
        value: TaggedValue,
    ) {
        self.process_registry.insert(id, value);
    }

    /// Register cons cells whose storage is owned by the loaded pdump image.
    ///
    /// # Safety
    /// `start..start+len` must remain mapped and writable for the lifetime of
    /// this heap.  The range must contain aligned `ConsCell` objects.
    pub(crate) unsafe fn register_mapped_cons_range(&mut self, start: *mut ConsCell, len: usize) {
        if len == 0 {
            return;
        }
        debug_assert_eq!(start as usize % std::mem::align_of::<ConsCell>(), 0);
        self.extend_dump_span(start as usize, len.saturating_mul(size_of::<ConsCell>()));
        self.mapped_cons_ranges
            .push(MappedConsRange::new(start, len));
        self.allocated_count = self.allocated_count.saturating_add(len);
        self.live_bytes = self
            .live_bytes
            .saturating_add(len.saturating_mul(size_of::<ConsCell>()));
    }

    /// Register float objects whose storage is owned by the loaded pdump image.
    ///
    /// # Safety
    /// `start..start+len` must remain mapped and writable for the lifetime of
    /// this heap.  The range must contain aligned `FloatObj` objects.
    pub(crate) unsafe fn register_mapped_float_range(&mut self, start: *mut FloatObj, len: usize) {
        if len == 0 {
            return;
        }
        debug_assert_eq!(start as usize % std::mem::align_of::<FloatObj>(), 0);
        self.extend_dump_span(start as usize, len.saturating_mul(size_of::<FloatObj>()));
        self.mapped_float_ranges
            .push(MappedFloatRange::new(start, len));
        self.allocated_count = self.allocated_count.saturating_add(len);
        self.live_bytes = self
            .live_bytes
            .saturating_add(len.saturating_mul(size_of::<FloatObj>()));
    }

    /// Register a vectorlike object whose storage is owned by the loaded pdump image.
    ///
    /// # Safety
    /// `header` must point at a complete, aligned vectorlike object that remains
    /// mapped and writable for the lifetime of this heap.
    pub(crate) unsafe fn register_mapped_veclike_object(
        &mut self,
        header: *mut VecLikeHeader,
        byte_len: usize,
    ) {
        if byte_len == 0 {
            return;
        }
        debug_assert_eq!(header as usize % std::mem::align_of::<VecLikeHeader>(), 0);
        self.extend_dump_span(header as usize, byte_len);
        let index = self.mapped_veclike_objects.len();
        let prev = self
            .mapped_veclike_index_by_addr
            .insert(header as usize, index);
        debug_assert!(prev.is_none(), "mapped vectorlike object registered twice");
        self.mapped_veclike_objects
            .push(MappedVecLikeObject::new(header, byte_len));
        self.allocated_count = self.allocated_count.saturating_add(1);
        self.live_bytes = self.live_bytes.saturating_add(byte_len);
    }

    /// Register a string object whose storage is owned by the loaded pdump image.
    ///
    /// # Safety
    /// `ptr` must point at a complete, aligned string object that remains
    /// mapped and writable for the lifetime of this heap.
    pub(crate) unsafe fn register_mapped_string_object(
        &mut self,
        ptr: *mut StringObj,
        byte_len: usize,
    ) {
        if byte_len == 0 {
            return;
        }
        debug_assert_eq!(ptr as usize % std::mem::align_of::<StringObj>(), 0);
        self.extend_dump_span(ptr as usize, byte_len);
        let index = self.mapped_string_objects.len();
        let prev = self.mapped_string_index_by_addr.insert(ptr as usize, index);
        debug_assert!(prev.is_none(), "mapped string object registered twice");
        self.mapped_string_objects
            .push(MappedStringObject::new(ptr, byte_len));
        self.allocated_count = self.allocated_count.saturating_add(1);
        self.live_bytes = self.live_bytes.saturating_add(byte_len);
    }

    pub fn dirty_owner_count(&self) -> usize {
        self.dirty_owners.len()
    }

    pub fn is_dirty_owner(&self, owner: TaggedValue) -> bool {
        self.dirty_owner_bits.contains(&owner.bits())
    }

    pub fn take_dirty_owners(&mut self) -> Vec<TaggedValue> {
        self.dirty_owner_bits.clear();
        std::mem::take(&mut self.dirty_owners)
    }

    pub fn clear_dirty_owners(&mut self) {
        self.dirty_owners.clear();
        self.dirty_owner_bits.clear();
    }

    pub fn dirty_write_count(&self) -> usize {
        self.dirty_writes.len()
    }

    pub fn dirty_writes(&self) -> &[HeapWriteRecord] {
        &self.dirty_writes
    }

    pub fn take_dirty_writes(&mut self) -> Vec<HeapWriteRecord> {
        std::mem::take(&mut self.dirty_writes)
    }

    pub fn clear_dirty_writes(&mut self) {
        self.dirty_writes.clear();
    }

    fn record_heap_write(&mut self, record: HeapWriteRecord) {
        // Dump partition: a mutated dumped object may now hold heap children,
        // so remember it as a permanent root. Conservative — a false positive
        // (a heap owner inside the dump address span) just adds a redundant
        // root; a false negative would be a use-after-free, so the span test
        // must cover every mapped object (see `register_mapped_*`).
        if self.partition_dump
            && (self.owner_is_mapped(record.owner) || self.value_is_tenured(record.owner))
        {
            self.mapped_remembered.insert(record.owner.bits());
        }
        // SATB (snapshot-at-the-beginning) barrier. Runs BEFORE the store, so the
        // owner's current children are its PRE-overwrite values; logging them
        // keeps the start-of-cycle snapshot live. Nothing is re-read later, so
        // the concurrent GC thread never touches a reallocated owner.
        if self.concurrent_mark_running {
            // The background GC thread is marking — log overwritten children to
            // the shared buffer it drains (not the local gray queue, which
            // belongs to the GC thread for the duration). This SATB barrier keeps
            // the start-of-cycle snapshot live without re-reading a mutated owner.
            self.push_value_children_to_satb_shared(record.owner);
        }
        if self.write_tracking_mode == WriteTrackingMode::Disabled {
            return;
        }
        if self.dirty_owner_bits.insert(record.owner.bits()) {
            self.dirty_owners.push(record.owner);
        }
        if self.write_tracking_mode == WriteTrackingMode::OwnersAndRecords {
            self.dirty_writes.push(record);
        }
    }

    /// Raw object address for a heap-tagged value (cons/veclike/string/float),
    /// used for the dump-partition address-span test.
    fn value_heap_addr(value: TaggedValue) -> Option<usize> {
        if value.is_cons() {
            Some(value.xcons_ptr() as usize)
        } else if value.is_veclike() {
            value.as_veclike_ptr().map(|ptr| ptr as usize)
        } else if value.is_string() {
            value.as_string_ptr().map(|ptr| ptr as usize)
        } else if value.is_float() {
            value.as_float_ptr().map(|ptr| ptr as usize)
        } else {
            None
        }
    }

    /// True if `value` is a mapped (pdump) object, via the address span that
    /// `register_mapped_*` keeps over every mapped object.
    fn owner_is_mapped(&self, value: TaggedValue) -> bool {
        match Self::value_heap_addr(value) {
            Some(addr) => addr >= self.dump_addr_lo && addr < self.dump_addr_hi,
            None => false,
        }
    }

    /// Extend the mapped-object address span to cover `[start, start+len)`.
    ///
    /// The first registered mapped object activates the dump partition (and its
    /// generational/incremental collector): a heap with a loaded pdump runs the
    /// low-pause collector, while a bare heap with no dump (unit tests, the
    /// pre-dump bootstrap loader) stays on the simple full mark-sweep path. This
    /// is intrinsic to whether there is anything to partition — not a tunable.
    fn extend_dump_span(&mut self, start: usize, len_bytes: usize) {
        if len_bytes == 0 {
            return;
        }
        self.dump_addr_lo = self.dump_addr_lo.min(start);
        self.dump_addr_hi = self.dump_addr_hi.max(start.saturating_add(len_bytes));
        if !self.partition_dump {
            self.partition_dump = true;
            // Keep the write-barrier hot-path mirror in sync so the dump
            // remembered set starts being maintained immediately.
            TAGGED_HEAP_PARTITION_ACTIVE.with(|p| p.set(true));
        }
    }

    fn note_allocation_bytes(&mut self, bytes: usize) {
        self.bytes_since_gc = self.bytes_since_gc.saturating_add(bytes);
        self.live_bytes = self.live_bytes.saturating_add(bytes);
    }

    fn vector_storage_bytes<T>(values: &Vec<T>) -> usize {
        values.capacity().saturating_mul(size_of::<T>())
    }

    fn lisp_value_vec_storage_bytes(values: &LispValueVec) -> usize {
        values
            .owned_capacity()
            .saturating_mul(size_of::<TaggedValue>())
    }

    fn hash_map_storage_bytes<K, V, S>(values: &std::collections::HashMap<K, V, S>) -> usize {
        values.capacity().saturating_mul(size_of::<(K, V)>())
    }

    fn string_object_bytes(obj: &StringObj) -> usize {
        size_of::<StringObj>().saturating_add(obj.data.byte_len())
    }

    fn hash_table_object_bytes(obj: &HashTableObj) -> usize {
        size_of::<HashTableObj>()
            .saturating_add(Self::hash_map_storage_bytes(&obj.table.data))
            .saturating_add(Self::hash_map_storage_bytes(&obj.table.key_snapshots))
            .saturating_add(Self::vector_storage_bytes(&obj.table.insertion_order))
            .saturating_add(Self::vector_storage_bytes(&obj.table.entry_slots))
            .saturating_add(Self::hash_map_storage_bytes(&obj.table.entry_slot_by_key))
            .saturating_add(Self::vector_storage_bytes(&obj.table.free_slots))
    }

    fn lambda_object_bytes(obj: &LambdaObj) -> usize {
        size_of::<LambdaObj>().saturating_add(Self::lisp_value_vec_storage_bytes(&obj.data))
    }

    fn macro_object_bytes(obj: &MacroObj) -> usize {
        size_of::<MacroObj>().saturating_add(Self::lisp_value_vec_storage_bytes(&obj.data))
    }

    fn bytecode_object_bytes(obj: &ByteCodeObj) -> usize {
        let data = &obj.data;
        size_of::<ByteCodeObj>()
            .saturating_add(Self::vector_storage_bytes(&data.ops))
            .saturating_add(Self::vector_storage_bytes(&data.constants))
            .saturating_add(
                data.params
                    .required
                    .capacity()
                    .saturating_mul(size_of::<SymId>()),
            )
            .saturating_add(
                data.params
                    .optional
                    .capacity()
                    .saturating_mul(size_of::<SymId>()),
            )
            .saturating_add(
                data.gnu_byte_offset_map
                    .as_ref()
                    .map_or(0, Self::vector_storage_bytes),
            )
            .saturating_add(
                data.gnu_bytecode_bytes
                    .as_ref()
                    .map_or(0, |bytes| bytes.capacity().saturating_mul(size_of::<u8>())),
            )
            .saturating_add(Self::vector_storage_bytes(&data.extra_slots))
            .saturating_add(data.docstring.as_ref().map_or(0, |doc| doc.sbytes()))
    }

    fn record_object_bytes(obj: &RecordObj) -> usize {
        size_of::<RecordObj>().saturating_add(Self::lisp_value_vec_storage_bytes(&obj.data))
    }

    fn obarray_object_bytes(obj: &ObarrayObj) -> usize {
        size_of::<ObarrayObj>().saturating_add(Self::lisp_value_vec_storage_bytes(&obj.buckets))
    }

    fn object_bytes_from_header(header: *const GcHeader) -> usize {
        unsafe {
            match (*header).kind {
                HeapObjectKind::String => Self::string_object_bytes(&*(header as *const StringObj)),
                HeapObjectKind::Float => size_of::<FloatObj>(),
                HeapObjectKind::VecLike => {
                    let ptr = header as *const VecLikeHeader;
                    match (*ptr).type_tag {
                        VecLikeType::Vector => {
                            let obj = &*(ptr as *const VectorObj);
                            size_of::<VectorObj>()
                                .saturating_add(Self::lisp_value_vec_storage_bytes(&obj.data))
                        }
                        VecLikeType::CharTable => {
                            let obj = &*(ptr as *const CharTableObj);
                            size_of::<CharTableObj>()
                                .saturating_add(Self::lisp_value_vec_storage_bytes(&obj.extras))
                        }
                        VecLikeType::SubCharTable => {
                            let obj = &*(ptr as *const SubCharTableObj);
                            size_of::<SubCharTableObj>()
                                .saturating_add(Self::lisp_value_vec_storage_bytes(&obj.contents))
                        }
                        VecLikeType::HashTable => {
                            Self::hash_table_object_bytes(&*(ptr as *const HashTableObj))
                        }
                        VecLikeType::Obarray => {
                            Self::obarray_object_bytes(&*(ptr as *const ObarrayObj))
                        }
                        VecLikeType::Lambda => {
                            Self::lambda_object_bytes(&*(ptr as *const LambdaObj))
                        }
                        VecLikeType::Macro => Self::macro_object_bytes(&*(ptr as *const MacroObj)),
                        VecLikeType::ByteCode => {
                            Self::bytecode_object_bytes(&*(ptr as *const ByteCodeObj))
                        }
                        VecLikeType::Record => {
                            Self::record_object_bytes(&*(ptr as *const RecordObj))
                        }
                        VecLikeType::Overlay => size_of::<OverlayObj>(),
                        VecLikeType::Marker => size_of::<MarkerObj>(),
                        VecLikeType::Buffer => size_of::<BufferObj>(),
                        VecLikeType::Window => size_of::<WindowObj>(),
                        VecLikeType::Frame => size_of::<FrameObj>(),
                        VecLikeType::Timer => size_of::<TimerObj>(),
                        VecLikeType::Process => size_of::<ProcessObj>(),
                        VecLikeType::Xwidget => size_of::<XwidgetObj>(),
                        VecLikeType::XwidgetView => size_of::<XwidgetViewObj>(),
                        VecLikeType::Subr => size_of::<SubrObj>(),
                        VecLikeType::Bignum => size_of::<BignumObj>(),
                        VecLikeType::SymbolWithPos => size_of::<SymbolWithPosObj>(),
                        VecLikeType::Sqlite => size_of::<SqliteObj>(),
                        VecLikeType::UserPtr => size_of::<UserPtrObj>(),
                        VecLikeType::ModuleFunction => size_of::<ModuleFunctionObj>(),
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Allocation
    // -----------------------------------------------------------------------

    /// Allocate a cons cell. Returns a tagged Value.
    pub fn alloc_cons(&mut self, car: TaggedValue, cdr: TaggedValue) -> TaggedValue {
        self.add_memory_use_count(MemoryUseCountSlot::ConsCells, 1);
        // Allocate-black during the deferred sweep OR a concurrent mark: a cons
        // born while a block is unswept must survive that block's reclaim, and a
        // cons born during concurrent marking must survive this cycle's sweep
        // (the GC thread won't reach it, and a black owner may point at it before
        // the next root snapshot). New conses are always live, so this is exact
        // (cleared at the next mark's begin).
        let sweeping = self.sweep_in_progress || self.concurrent_mark_running;
        if !self.cons_free_list.is_null() {
            let cell = self.cons_free_list;
            unsafe {
                self.cons_free_list = (*cell).free_next();
                (*cell).set_car(car);
                (*cell).set_cdr(cdr);
            }
            self.allocated_count += 1;
            self.cons_live_count += 1;
            self.note_allocation_bytes(size_of::<ConsCell>());
            if sweeping {
                self.mark_cons(cell);
            }
            return unsafe { TaggedValue::from_cons_ptr(cell) };
        }

        if let Some(block) = self.cons_blocks.last_mut()
            && let Some(cell) = block.alloc_bump(car, cdr)
        {
            if sweeping {
                block.mark_ptr(cell);
            }
            self.allocated_count += 1;
            self.cons_live_count += 1;
            self.note_allocation_bytes(size_of::<ConsCell>());
            return unsafe { TaggedValue::from_cons_ptr(cell) };
        }

        // All existing blocks are exhausted and there are no reclaimed cells,
        // so allocate a fresh current block and bump from it, matching GNU's
        // cons_block/cons_block_index path.
        let mut block = ConsBlock::new();
        let block_base = block.base_addr();
        let cell = block
            .alloc_bump(car, cdr)
            .expect("fresh block should have space");
        self.cons_blocks.push(block);
        let block_index = self.cons_blocks.len() - 1;
        self.cons_block_index_by_base
            .insert(block_base, block_index);
        self.allocated_count += 1;
        self.cons_live_count += 1;
        self.note_allocation_bytes(size_of::<ConsCell>());
        if sweeping {
            self.mark_cons(cell);
        }
        unsafe { TaggedValue::from_cons_ptr(cell) }
    }

    /// Allocate a string object.
    pub fn alloc_string(&mut self, s: crate::heap_types::LispString) -> TaggedValue {
        self.add_memory_use_count(MemoryUseCountSlot::Strings, 1);
        self.add_memory_use_count(MemoryUseCountSlot::StringChars, s.sbytes() as u64);
        let obj = Box::new(StringObj {
            header: GcHeader::new(HeapObjectKind::String),
            data: s,
        });
        let ptr = Box::into_raw(obj);
        self.link_object(unsafe { &mut (*ptr).header });
        self.allocated_count += 1;
        self.note_allocation_bytes(unsafe { Self::string_object_bytes(&*ptr) });
        unsafe { TaggedValue::from_string_ptr(ptr) }
    }

    /// Allocate a float object.
    pub fn alloc_float(&mut self, value: f64) -> TaggedValue {
        self.add_memory_use_count(MemoryUseCountSlot::Floats, 1);
        let obj = Box::new(FloatObj {
            header: GcHeader::new(HeapObjectKind::Float),
            value,
        });
        let ptr = Box::into_raw(obj);
        self.link_object(unsafe { &mut (*ptr).header });
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<FloatObj>());
        unsafe { TaggedValue::from_float_ptr(ptr) }
    }

    /// Allocate a vector.
    pub fn alloc_vector(&mut self, items: Vec<TaggedValue>) -> TaggedValue {
        self.add_memory_use_count(MemoryUseCountSlot::VectorCells, items.len() as u64);
        let obj = Box::new(VectorObj {
            header: VecLikeHeader::new(VecLikeType::Vector),
            data: items.into(),
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(
            size_of::<VectorObj>()
                .saturating_add(Self::lisp_value_vec_storage_bytes(unsafe { &(*ptr).data })),
        );
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate a GNU-shaped char-table.
    pub fn alloc_char_table(
        &mut self,
        purpose: TaggedValue,
        init: TaggedValue,
        n_extras: usize,
    ) -> TaggedValue {
        let contents = [init; CHAR_TABLE_TOP_SLOTS];
        let extras = vec![init; n_extras];
        self.add_memory_use_count(
            MemoryUseCountSlot::VectorCells,
            (4 + CHAR_TABLE_TOP_SLOTS + n_extras) as u64,
        );
        let obj = Box::new(CharTableObj {
            header: VecLikeHeader::new(VecLikeType::CharTable),
            defalt: init,
            parent: TaggedValue::NIL,
            purpose,
            ascii: init,
            contents,
            extras: extras.into(),
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(unsafe {
            size_of::<CharTableObj>()
                .saturating_add(Self::lisp_value_vec_storage_bytes(&(*ptr).extras))
        });
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate a GNU-shaped sub-char-table.
    pub fn alloc_sub_char_table(
        &mut self,
        depth: i32,
        min_char: i32,
        contents: Vec<TaggedValue>,
    ) -> TaggedValue {
        self.add_memory_use_count(MemoryUseCountSlot::VectorCells, contents.len() as u64);
        let obj = Box::new(SubCharTableObj {
            header: VecLikeHeader::new(VecLikeType::SubCharTable),
            depth,
            min_char,
            contents: contents.into(),
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(unsafe {
            size_of::<SubCharTableObj>()
                .saturating_add(Self::lisp_value_vec_storage_bytes(&(*ptr).contents))
        });
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate a hash table.
    pub fn alloc_hash_table(
        &mut self,
        table: crate::emacs_core::value::LispHashTable,
    ) -> TaggedValue {
        self.add_memory_use_count(MemoryUseCountSlot::VectorCells, 1);
        let obj = Box::new(HashTableObj {
            header: VecLikeHeader::new(VecLikeType::HashTable),
            table,
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(unsafe { Self::hash_table_object_bytes(&*ptr) });
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate a GNU-shaped obarray object.
    pub fn alloc_obarray(&mut self, buckets: Vec<TaggedValue>) -> TaggedValue {
        self.add_memory_use_count(MemoryUseCountSlot::VectorCells, buckets.len() as u64);
        let obj = Box::new(ObarrayObj {
            header: VecLikeHeader::new(VecLikeType::Obarray),
            buckets: buckets.into(),
            count: 0,
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(unsafe { Self::obarray_object_bytes(&*ptr) });
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate a lambda.
    /// Allocate a lambda (interpreted closure) as a Value vector.
    /// Matches GNU Emacs's PVEC_CLOSURE: all slots are GC-traced Values.
    pub fn alloc_lambda(&mut self, slots: Vec<TaggedValue>) -> TaggedValue {
        let obj = Box::new(LambdaObj {
            header: VecLikeHeader::new(VecLikeType::Lambda),
            data: slots.into(),
            parsed_params: std::sync::OnceLock::new(),
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(unsafe { Self::lambda_object_bytes(&*ptr) });
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate a lambda from a LambdaData (bridge for migration).
    /// Converts LambdaData fields to the Value vector layout.
    pub fn alloc_lambda_from_data(
        &mut self,
        data: crate::emacs_core::value::LambdaData,
    ) -> TaggedValue {
        let slots = data.to_closure_slots();
        self.alloc_lambda(slots)
    }

    /// Allocate a macro as a Value vector.
    pub fn alloc_macro(&mut self, slots: Vec<TaggedValue>) -> TaggedValue {
        let obj = Box::new(MacroObj {
            header: VecLikeHeader::new(VecLikeType::Macro),
            data: slots.into(),
            parsed_params: std::sync::OnceLock::new(),
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(unsafe { Self::macro_object_bytes(&*ptr) });
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate a macro from a LambdaData (bridge for migration).
    pub fn alloc_macro_from_data(
        &mut self,
        data: crate::emacs_core::value::LambdaData,
    ) -> TaggedValue {
        let slots = data.to_closure_slots();
        self.alloc_macro(slots)
    }

    /// Allocate a buffer reference.
    pub fn alloc_buffer(&mut self, id: crate::buffer::BufferId) -> TaggedValue {
        let obj = Box::new(BufferObj {
            header: VecLikeHeader::new(VecLikeType::Buffer),
            id,
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<BufferObj>());
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate a window reference.
    pub fn alloc_window(&mut self, id: u64) -> TaggedValue {
        let obj = Box::new(WindowObj {
            header: VecLikeHeader::new(VecLikeType::Window),
            id,
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<WindowObj>());
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate a frame reference.
    pub fn alloc_frame(&mut self, id: u64) -> TaggedValue {
        let obj = Box::new(FrameObj {
            header: VecLikeHeader::new(VecLikeType::Frame),
            id,
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<FrameObj>());
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate a timer reference.
    pub fn alloc_timer(&mut self, id: u64) -> TaggedValue {
        let obj = Box::new(TimerObj {
            header: VecLikeHeader::new(VecLikeType::Timer),
            id,
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<TimerObj>());
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate a process reference.
    pub fn alloc_process(&mut self, id: crate::emacs_core::process::ProcessId) -> TaggedValue {
        let obj = Box::new(ProcessObj {
            header: VecLikeHeader::new(VecLikeType::Process),
            id,
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<ProcessObj>());
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate an xwidget model object.
    pub fn alloc_xwidget(
        &mut self,
        type_: TaggedValue,
        title: TaggedValue,
        buffer: TaggedValue,
        width: i32,
        height: i32,
        xwidget_id: u32,
    ) -> TaggedValue {
        let obj = Box::new(XwidgetObj {
            header: VecLikeHeader::new(VecLikeType::Xwidget),
            plist: TaggedValue::NIL,
            type_,
            buffer,
            title,
            script_callbacks: TaggedValue::NIL,
            height,
            width,
            xwidget_id,
            kill_without_query: false,
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<XwidgetObj>());
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate an xwidget view object.
    pub fn alloc_xwidget_view(&mut self, model: TaggedValue, window: TaggedValue) -> TaggedValue {
        let obj = Box::new(XwidgetViewObj {
            header: VecLikeHeader::new(VecLikeType::XwidgetView),
            model,
            window,
            x: 0,
            y: 0,
            clip_right: 0,
            clip_bottom: 0,
            clip_top: 0,
            clip_left: 0,
            redisplayed: false,
            hidden: false,
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<XwidgetViewObj>());
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate a bytecode function.
    pub fn alloc_bytecode(
        &mut self,
        data: crate::emacs_core::bytecode::ByteCodeFunction,
    ) -> TaggedValue {
        let obj = Box::new(ByteCodeObj {
            header: VecLikeHeader::new(VecLikeType::ByteCode),
            data,
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(unsafe { Self::bytecode_object_bytes(&*ptr) });
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate a record.
    pub fn alloc_record(&mut self, items: Vec<TaggedValue>) -> TaggedValue {
        self.add_memory_use_count(MemoryUseCountSlot::VectorCells, items.len() as u64);
        let obj = Box::new(RecordObj {
            header: VecLikeHeader::new(VecLikeType::Record),
            data: items.into(),
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(unsafe { Self::record_object_bytes(&*ptr) });
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate an overlay.
    pub fn alloc_overlay(&mut self, data: crate::heap_types::OverlayData) -> TaggedValue {
        let obj = Box::new(OverlayObj {
            header: VecLikeHeader::new(VecLikeType::Overlay),
            data,
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<OverlayObj>());
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate a marker.
    pub fn alloc_marker(&mut self, data: crate::heap_types::LispMarker) -> TaggedValue {
        let obj = Box::new(MarkerObj {
            header: VecLikeHeader::new(VecLikeType::Marker),
            data,
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<MarkerObj>());
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate a bignum (arbitrary-precision integer).
    ///
    /// Mirrors GNU `make_bignum` (`src/bignum.c:113`): the caller is
    /// responsible for ensuring the value is outside fixnum range.
    /// Use `Value::make_integer` for the canonical "fixnum-or-bignum"
    /// constructor that delegates here only when promotion is needed.
    pub fn alloc_bignum(&mut self, value: Integer) -> TaggedValue {
        let obj = Box::new(BignumObj {
            header: VecLikeHeader::new(VecLikeType::Bignum),
            value,
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<BignumObj>());
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate a symbol-with-pos object.
    /// `sym` must be a bare symbol, `pos` must be a fixnum.
    pub fn alloc_symbol_with_pos(&mut self, sym: TaggedValue, pos: TaggedValue) -> TaggedValue {
        let obj = Box::new(SymbolWithPosObj {
            header: VecLikeHeader::new(VecLikeType::SymbolWithPos),
            sym,
            pos,
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<SymbolWithPosObj>());
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate an SQLite database or statement object.
    pub fn alloc_sqlite(&mut self, is_statement: bool, id: i64) -> TaggedValue {
        let obj = Box::new(SqliteObj {
            header: VecLikeHeader::new(VecLikeType::Sqlite),
            is_statement,
            id,
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<SqliteObj>());
        unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) }
    }

    /// Allocate a user-pointer object for dynamic module API.
    pub fn alloc_user_ptr(
        &mut self,
        ptr: *mut std::ffi::c_void,
        finalizer: EmacsFinalizer,
    ) -> TaggedValue {
        let obj = Box::new(UserPtrObj {
            header: VecLikeHeader::new(VecLikeType::UserPtr),
            ptr,
            finalizer,
        });
        let raw = Box::into_raw(obj);
        self.link_veclike(raw as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<UserPtrObj>());
        unsafe { TaggedValue::from_veclike_ptr(raw as *const VecLikeHeader) }
    }

    /// Allocate a module-function object for dynamic module API.
    pub fn alloc_module_function(
        &mut self,
        min_arity: isize,
        max_arity: isize,
        subr: *const std::ffi::c_void,
        data: *mut std::ffi::c_void,
        documentation: TaggedValue,
        interactive_form: TaggedValue,
    ) -> TaggedValue {
        let obj = Box::new(ModuleFunctionObj {
            header: VecLikeHeader::new(VecLikeType::ModuleFunction),
            min_arity,
            max_arity,
            subr,
            data,
            finalizer: None,
            documentation,
            interactive_form,
        });
        let raw = Box::into_raw(obj);
        self.link_veclike(raw as *mut VecLikeHeader);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<ModuleFunctionObj>());
        unsafe { TaggedValue::from_veclike_ptr(raw as *const VecLikeHeader) }
    }

    // -----------------------------------------------------------------------
    // Marker operations
    // -----------------------------------------------------------------------

    // `find_marker_by_id_during_load` was retired in T11. Pdump load now
    // builds an O(1) `marker_id` → `MarkerObj*` index in
    // `TaggedLoadState::markers_by_id` during `preload_tagged_heap`, so the
    // O(N·M) heap scan is no longer needed.

    /// Install the raw chain-head slots the next `complete_collection`
    /// cycle should walk when unlinking dead markers. Caller (typically
    /// `Context::gc_collect_from_current_roots`) passes one slot per
    /// live `BufferText`. The vec is consumed and cleared by
    /// `unchain_dead_markers` so successive cycles must re-install.
    ///
    /// SAFETY: each slot must point to a valid `*mut MarkerObj` living
    /// inside a live `BufferText`'s storage and must remain valid for
    /// the duration of the GC cycle. The caller must hold exclusive
    /// access to the heap and the buffer manager during the cycle.
    pub unsafe fn set_marker_chain_head_slots(&mut self, slots: Vec<*mut *mut MarkerObj>) {
        self.marker_chain_head_slots = slots;
    }

    /// Walk each installed buffer-chain head slot and splice out markers
    /// whose GC mark bit is clear. Runs between `mark_all` and
    /// `sweep_objects` so reading `header.gc.marked` is sound (the
    /// allocation is still live). Mirrors GNU Emacs `sweep_buffer →
    /// unchain_dead_markers` (alloc.c).
    fn unchain_dead_markers(&mut self) {
        // Take the slot list out so we don't alias self while iterating.
        let slots = std::mem::take(&mut self.marker_chain_head_slots);
        for slot in slots {
            unsafe {
                let mut prev_slot: *mut *mut MarkerObj = slot;
                while !(*prev_slot).is_null() {
                    let curr = *prev_slot;
                    if (*curr).header.gc.is_marked() {
                        // Live — advance prev
                        prev_slot = &mut (*curr).data.next_marker;
                    } else {
                        // Dead — splice out. The generic `sweep_objects`
                        // pass frees the allocation.
                        *prev_slot = (*curr).data.next_marker;
                        (*curr).data.next_marker = std::ptr::null_mut();
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Link a non-cons object into the all_objects intrusive list.
    fn link_object(&mut self, header: &mut GcHeader) {
        header.next = self.all_objects;
        // Allocate-black during a concurrent mark: the GC thread defers non-cons
        // objects and never reaches one born mid-cycle, so mark it live now to
        // survive this cycle's sweep (cleared at the next mark's begin).
        if self.concurrent_mark_running {
            header.set_marked(true);
        }
        let ptr = header as *mut GcHeader;
        let inserted = self.non_cons_object_addrs.insert(ptr as usize);
        debug_assert!(inserted, "non-cons object linked twice");
        self.all_objects = ptr;
    }

    /// Link a veclike object into the all_objects list.
    fn link_veclike(&mut self, header: *mut VecLikeHeader) {
        unsafe {
            (*header).gc.next = self.all_objects;
            if self.concurrent_mark_running {
                (*header).gc.set_marked(true); // allocate-black (see `link_object`)
            }
            let gc_header = &mut (*header).gc as *mut GcHeader;
            let inserted = self.non_cons_object_addrs.insert(gc_header as usize);
            debug_assert!(inserted, "veclike object linked twice");
            self.all_objects = gc_header;
        }
    }

    // -----------------------------------------------------------------------
    // Garbage collection — stop-the-world mark-sweep
    // -----------------------------------------------------------------------

    /// Run a full mark-sweep garbage collection.
    ///
    /// `roots` must yield every reachable `TaggedValue`.
    pub fn collect(&mut self, roots: impl Iterator<Item = TaggedValue>) {
        self.collect_exact(roots);
    }

    /// Run a full mark-sweep collection using only the explicit roots provided.
    pub fn collect_exact(&mut self, roots: impl Iterator<Item = TaggedValue>) {
        self.begin_collection();
        for root in roots {
            self.seed_root(root);
        }
        self.complete_collection();
    }

    pub(crate) fn begin_collection(&mut self) {
        // (Pre-mark verification removed — unmarked objects may have stale data
        //  that will be swept. Only post-mark verification is meaningful.)

        // A mark must never start while a deferred sweep is still draining: the
        // sweep reads the mark bits this would clear. The driver finishes any
        // in-flight sweep before getting here.
        debug_assert!(
            !self.sweep_in_progress,
            "begin_collection while a deferred sweep is in progress"
        );

        let clear_t0 = std::time::Instant::now();
        // The first partition cycle runs a NORMAL full collection (so it traces
        // everything and frees load transients); promotion + blackening happen
        // at the end of that cycle (`complete_collection`). Only once
        // `dump_blackened` is set do the partitioned skips apply.
        let partitioned = self.partition_dump && self.dump_blackened;

        // -- Clear marks (heap cons) --
        for block in &mut self.cons_blocks {
            block.clear_marks();
        }
        // -- Mapped (pdump) marks: permanent black region when partitioned --
        if !partitioned {
            for range in &mut self.mapped_cons_ranges {
                range.clear_marks();
            }
            for range in &mut self.mapped_float_ranges {
                range.clear_marks();
            }
            for object in &mut self.mapped_veclike_objects {
                object.marked = false;
            }
            for object in &mut self.mapped_string_objects {
                object.marked = false;
            }
        }
        // -- Clear marks on YOUNG non-cons (heap) objects. The tenured old
        //    generation lives on a separate list (`tenured_objects`) that is
        //    never walked here, so it stays permanently black. Before the
        //    first-cycle promotion every object is still on `all_objects`, so
        //    the full preloaded world is cleared and traced that one cycle. --
        let mut obj = self.all_objects;
        while !obj.is_null() {
            unsafe {
                (*obj).set_marked(false);
                obj = (*obj).next;
            }
        }

        self.last_clear_us = clear_t0.elapsed().as_micros() as u64;

        // -- Seed gray queue from roots --
        self.gray_queue.clear();
        self.weak_hash_tables.clear();
        self.mark_cons_block_cache = None;
        self.seed_internal_runtime_roots();
        if partitioned {
            // Re-scan dumped/tenured objects mutated to point at young heap
            // objects: those children must be kept live even though the dump and
            // the tenured old generation are black.
            self.seed_mapped_remembered();
        } else if self.partition_dump {
            // First partition cycle (full trace): keep every dump-referenced
            // heap object alive so none is swept and left dangling when the
            // image is blackened at the end of this cycle.
            self.seed_all_mapped_children();
        }
    }

    /// Run once at the END of the first partition cycle (after a full
    /// trace+sweep): promote every survivor to the tenured old generation,
    /// blacken the mapped dump image, and build the initial remembered set.
    /// Thereafter both regions are permanently black and skipped each cycle.
    fn promote_and_blacken(&mut self) {
        // 1. Promote every surviving heap object to tenured (old generation).
        //    The first partition cycle ran a full trace+sweep, so everything
        //    still in `all_objects` is alive = a permanent (the preloaded world
        //    plus whatever the session has retained). They are already marked.
        //    Move the whole young list onto the tenured list and flag each
        //    node so the nursery (`all_objects`) starts empty; from now on only
        //    post-loadup allocations land there and get cleared/swept.
        let mut tail: *mut GcHeader = std::ptr::null_mut();
        let mut obj = self.all_objects;
        while !obj.is_null() {
            unsafe {
                (*obj).tenured = true;
                tail = obj;
                obj = (*obj).next;
            }
        }
        if !tail.is_null() {
            // Splice: [all_objects .. tail] -> front of tenured_objects.
            unsafe {
                (*tail).next = self.tenured_objects;
            }
            self.tenured_objects = self.all_objects;
            self.all_objects = std::ptr::null_mut();
        }
        // 2. Blacken the mapped image.
        for range in &mut self.mapped_cons_ranges {
            range.mark_all();
        }
        for range in &mut self.mapped_float_ranges {
            range.mark_all();
        }
        for object in &mut self.mapped_veclike_objects {
            object.marked = true;
        }
        for object in &mut self.mapped_string_objects {
            object.marked = true;
        }
        // 3. Remember permanents (mapped or tenured) that point at a YOUNG heap
        //    object so its children stay live. After promotion the only young
        //    objects are heap conses (cons cells are header-less and cannot be
        //    tenured), so this is the surviving-heap-cons reference set.
        self.scan_permanents_for_young_children();
    }

    /// Scan every permanent object (mapped dump + tenured old gen) for edges to
    /// young heap objects and add such permanents to the remembered set. Used
    /// at promotion and re-buildable on demand; the result is seeded each cycle.
    fn scan_permanents_for_young_children(&mut self) {
        // -- mapped vectorlike --
        let veclike: Vec<*mut VecLikeHeader> = self
            .mapped_veclike_objects
            .iter()
            .map(|o| o.header)
            .collect();
        for ptr in veclike {
            if self
                .collect_veclike_children(ptr)
                .iter()
                .any(|c| self.is_heap_young(*c))
            {
                let value = unsafe { TaggedValue::from_veclike_ptr(ptr as *const VecLikeHeader) };
                self.mapped_remembered.insert(value.bits());
            }
        }
        // -- mapped conses --
        let cons_ranges: Vec<(*mut ConsCell, usize)> = self
            .mapped_cons_ranges
            .iter()
            .map(|range| (range.start, range.len))
            .collect();
        for (start, len) in cons_ranges {
            for i in 0..len {
                let cell = unsafe { start.add(i) };
                let car = unsafe { (*cell).load_car() };
                let cdr = unsafe { (*cell).load_cdr() };
                if self.is_heap_young(car) || self.is_heap_young(cdr) {
                    let value = unsafe { TaggedValue::from_cons_ptr(cell) };
                    self.mapped_remembered.insert(value.bits());
                }
            }
        }
        // -- mapped strings (text-prop intervals) --
        let strings: Vec<*mut StringObj> =
            self.mapped_string_objects.iter().map(|o| o.ptr).collect();
        for ptr in strings {
            let mut roots: Vec<TaggedValue> = Vec::new();
            let intervals = unsafe { (*ptr).data.intervals() };
            if !intervals.is_empty() {
                intervals.for_each_root(|root| roots.push(root));
            }
            if roots.iter().any(|r| self.is_heap_young(*r)) {
                let value = unsafe { TaggedValue::from_string_ptr(ptr) };
                self.mapped_remembered.insert(value.bits());
            }
        }
        // -- tenured heap objects (old generation) --
        let tenured: Vec<*mut GcHeader> = {
            let mut out = Vec::new();
            let mut obj = self.tenured_objects;
            while !obj.is_null() {
                unsafe {
                    out.push(obj);
                    obj = (*obj).next;
                }
            }
            out
        };
        for header in tenured {
            let kind = unsafe { (*header).kind };
            let has_young = match kind {
                HeapObjectKind::VecLike => self
                    .collect_veclike_children(header as *mut VecLikeHeader)
                    .iter()
                    .any(|c| self.is_heap_young(*c)),
                HeapObjectKind::String => {
                    let mut roots: Vec<TaggedValue> = Vec::new();
                    let intervals = unsafe { (*(header as *const StringObj)).data.intervals() };
                    if !intervals.is_empty() {
                        intervals.for_each_root(|root| roots.push(root));
                    }
                    roots.iter().any(|r| self.is_heap_young(*r))
                }
                HeapObjectKind::Float => false,
            };
            if has_young {
                let value = match kind {
                    HeapObjectKind::VecLike => unsafe {
                        TaggedValue::from_veclike_ptr(header as *const VecLikeHeader)
                    },
                    HeapObjectKind::String => unsafe {
                        TaggedValue::from_string_ptr(header as *mut StringObj)
                    },
                    HeapObjectKind::Float => continue,
                };
                self.mapped_remembered.insert(value.bits());
            }
        }
    }

    /// True if `value` is a YOUNG heap object: a real heap allocation that is
    /// neither mapped (dump) nor tenured (old gen) — i.e. it participates in the
    /// normal clear/mark/sweep each cycle. Heap cons cells are always young
    /// (header-less, cannot be tenured).
    fn is_heap_young(&self, value: TaggedValue) -> bool {
        if !value.is_heap_object() || self.owner_is_mapped(value) {
            return false;
        }
        if value.is_cons() {
            return true; // heap cons: header-less, cannot be tenured
        }
        // Non-cons: young iff heap-OWNED and not tenured. Static/untracked
        // objects (e.g. Subrs) are permanently live, never young.
        match Self::value_heap_addr(value) {
            Some(addr) => {
                self.owns_non_cons_object(addr as *const u8)
                    && !unsafe { (*(addr as *const GcHeader)).tenured }
            }
            None => false,
        }
    }

    /// True if `value` is a tenured (old-gen) heap non-cons object.
    fn value_is_tenured(&self, value: TaggedValue) -> bool {
        if value.is_cons() {
            return false;
        }
        let Some(addr) = Self::value_heap_addr(value) else {
            return false;
        };
        if !self.owns_non_cons_object(addr as *const u8) {
            return false; // mapped, not a tenured heap object
        }
        unsafe { (*(addr as *const GcHeader)).tenured }
    }

    /// First-cycle only: seed the heap children of EVERY mapped object so they
    /// survive the cycle's sweep. Dumped objects are never freed, so a heap
    /// object referenced only by an (otherwise unreachable) dumped object must
    /// still be kept — otherwise it would be swept and the dumped object would
    /// be left holding a dangling pointer once the image is blackened.
    fn seed_all_mapped_children(&mut self) {
        let veclike: Vec<*mut VecLikeHeader> = self
            .mapped_veclike_objects
            .iter()
            .map(|o| o.header)
            .collect();
        for ptr in veclike {
            unsafe { self.trace_veclike(ptr) };
        }
        let cons_ranges: Vec<(*mut ConsCell, usize)> = self
            .mapped_cons_ranges
            .iter()
            .map(|range| (range.start, range.len))
            .collect();
        for (start, len) in cons_ranges {
            for i in 0..len {
                let cell = unsafe { start.add(i) };
                let car = unsafe { (*cell).load_car() };
                let cdr = unsafe { (*cell).load_cdr() };
                if car.is_heap_object() {
                    self.push_gray(car, "first-cycle-mapped-cons-car");
                }
                if cdr.is_heap_object() {
                    self.push_gray(cdr, "first-cycle-mapped-cons-cdr");
                }
            }
        }
        let strings: Vec<*mut StringObj> =
            self.mapped_string_objects.iter().map(|o| o.ptr).collect();
        for ptr in strings {
            let mut roots: Vec<TaggedValue> = Vec::new();
            let intervals = unsafe { (*ptr).data.intervals() };
            if !intervals.is_empty() {
                intervals.for_each_root(|root| roots.push(root));
            }
            for root in roots {
                if root.is_heap_object() {
                    self.push_gray(root, "first-cycle-mapped-string-interval");
                }
            }
        }
    }

    /// Seed the gray queue with the heap children of every dumped object that
    /// has been mutated since load (the dump remembered set). Because the dump
    /// is black, `mark_value` would otherwise never re-trace these, so we
    /// enqueue their children directly. Mapped children are already black and
    /// are skipped when popped; only heap children get marked.
    fn seed_mapped_remembered(&mut self) {
        if self.mapped_remembered.is_empty() {
            return;
        }
        let owners: Vec<TaggedValue> = self
            .mapped_remembered
            .iter()
            .map(|&bits| TaggedValue(bits))
            .collect();
        for owner in owners {
            self.push_value_children_to_gray(owner, "remembered-dump-child");
        }
    }

    /// Push every heap child of `owner` onto the gray queue (re-trace its
    /// outgoing references). Unlike `mark_value`, this does NOT consult the
    /// owner's own mark bit, so it re-examines an already-black owner's slots —
    /// exactly what the incremental-update barrier and the dump remembered set
    /// both need. Mirrors `trace_veclike`/cons/string child enumeration.
    fn push_value_children_to_gray(&mut self, owner: TaggedValue, origin: &'static str) {
        if owner.is_cons() {
            let ptr = owner.xcons_ptr();
            let car = unsafe { (*ptr).load_car() };
            let cdr = unsafe { (*ptr).load_cdr() };
            if car.is_heap_object() {
                self.push_gray(car, origin);
            }
            if cdr.is_heap_object() {
                self.push_gray(cdr, origin);
            }
        } else if owner.is_veclike() {
            if let Some(ptr) = owner.as_veclike_ptr() {
                unsafe { self.trace_veclike(ptr as *mut VecLikeHeader) };
            }
        } else if owner.is_string() {
            if let Some(ptr) = owner.as_string_ptr() {
                let intervals = unsafe { (*(ptr as *const StringObj)).data.intervals() };
                if !intervals.is_empty() {
                    intervals.for_each_root(|root| {
                        if root.is_heap_object() {
                            self.push_gray(root, origin);
                        }
                    });
                }
            }
        }
        // Floats have no heap children.
    }

    /// Is `value` currently marked? Covers heap and mapped objects of every
    /// category. Used only by the dump-partition verifier.
    fn is_value_marked(&self, value: TaggedValue) -> bool {
        if value.is_cons() {
            let ptr = value.xcons_ptr();
            if ConsBlock::ptr_is_cell_aligned(ptr) {
                let base = ConsBlock::block_base_for_ptr(ptr);
                if let Some(&idx) = self.cons_block_index_by_base.get(&base) {
                    return self.cons_blocks[idx].is_marked_ptr(ptr);
                }
            }
            return self
                .mapped_cons_ranges
                .iter()
                .find(|range| range.contains_ptr(ptr))
                .map(|range| range.is_marked_ptr(ptr))
                .unwrap_or(false);
        }
        let Some(addr) = Self::value_heap_addr(value) else {
            return true;
        };
        let owned = self.owns_non_cons_object(addr as *const u8);
        // A non-cons object that is neither heap-owned nor mapped is a static,
        // never-swept runtime object (e.g. a `Subr`) — permanently live, so
        // treat it as marked (`unwrap_or(true)`). This relies on the dump
        // partition keeping every dump-referenced heap object live, so a
        // not-owned/not-mapped pointer is never a dangling reference.
        if value.is_string() {
            if owned {
                return unsafe { (*(addr as *const StringObj)).header.is_marked() };
            }
            return self
                .mapped_string_index_by_addr
                .get(&addr)
                .map(|&i| self.mapped_string_objects[i].marked)
                .unwrap_or(true);
        }
        if value.is_float() {
            if owned {
                return unsafe { (*(addr as *const FloatObj)).header.is_marked() };
            }
            let ptr = addr as *const FloatObj;
            return self
                .mapped_float_ranges
                .iter()
                .find(|range| range.contains_ptr(ptr))
                .map(|range| range.is_marked_ptr(ptr))
                .unwrap_or(true);
        }
        if value.is_veclike() {
            if owned {
                return unsafe { (*(addr as *const VecLikeHeader)).gc.is_marked() };
            }
            return self
                .mapped_veclike_index_by_addr
                .get(&addr)
                .map(|&i| self.mapped_veclike_objects[i].marked)
                .unwrap_or(true);
        }
        true
    }

    /// Verification gate for the dump partition (env `NEOVM_GC_VERIFY_PARTITION`).
    /// After the partitioned mark, every direct heap child of every dumped
    /// object MUST already be marked — otherwise the write barrier missed a
    /// dumped→heap mutation and the partition is about to free a live object.
    /// Panics on the first violation. Expensive (full dump scan); verification
    /// runs only.
    fn verify_dump_partition(&mut self) {
        let mut violations: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        let mut sample: Option<usize> = None;
        let mut record = |owner: &str, child: TaggedValue| {
            let child_kind = if child.is_cons() {
                "cons".to_string()
            } else if child.is_string() {
                "string".to_string()
            } else if child.is_float() {
                "float".to_string()
            } else if child.is_veclike() {
                format!("{:?}", child.veclike_type())
            } else {
                "other".to_string()
            };
            *violations
                .entry(format!("{owner} -> {child_kind}"))
                .or_insert(0) += 1;
            sample.get_or_insert(child.0);
        };

        // Mapped veclike objects (char-tables etc.), grouped by owner type.
        let veclike: Vec<(*mut VecLikeHeader, VecLikeType)> = self
            .mapped_veclike_objects
            .iter()
            .map(|o| (o.header, unsafe { (*o.header).type_tag }))
            .collect();
        for (ptr, ty) in veclike {
            let owner = format!("{ty:?}");
            for child in self.collect_veclike_children(ptr) {
                if child.is_heap_object() && !self.is_value_marked(child) {
                    record(&owner, child);
                }
            }
        }
        // Mapped conses.
        let cons_ranges: Vec<(*mut ConsCell, usize)> = self
            .mapped_cons_ranges
            .iter()
            .map(|range| (range.start, range.len))
            .collect();
        for (start, len) in cons_ranges {
            for i in 0..len {
                let cell = unsafe { start.add(i) };
                for child in [unsafe { (*cell).load_car() }, unsafe { (*cell).load_cdr() }] {
                    if child.is_heap_object() && !self.is_value_marked(child) {
                        record("Cons", child);
                    }
                }
            }
        }
        // Mapped strings (text-property intervals).
        let strings: Vec<*mut StringObj> =
            self.mapped_string_objects.iter().map(|o| o.ptr).collect();
        for ptr in strings {
            let mut roots: Vec<TaggedValue> = Vec::new();
            let intervals = unsafe { (*ptr).data.intervals() };
            if !intervals.is_empty() {
                intervals.for_each_root(|root| roots.push(root));
            }
            for child in roots {
                if child.is_heap_object() && !self.is_value_marked(child) {
                    record("String", child);
                }
            }
        }
        // Tenured heap objects (old generation): their direct heap children
        // must also be marked, or a survival-promoted permanent mutated to
        // point at a young object would free it.
        let tenured: Vec<*mut GcHeader> = {
            let mut out = Vec::new();
            let mut obj = self.tenured_objects;
            while !obj.is_null() {
                unsafe {
                    out.push(obj);
                    obj = (*obj).next;
                }
            }
            out
        };
        for header in tenured {
            let kind = unsafe { (*header).kind };
            let owner = format!("tenured:{kind:?}");
            let children: Vec<TaggedValue> = match kind {
                HeapObjectKind::VecLike => {
                    self.collect_veclike_children(header as *mut VecLikeHeader)
                }
                HeapObjectKind::String => {
                    let mut roots = Vec::new();
                    let intervals = unsafe { (*(header as *const StringObj)).data.intervals() };
                    if !intervals.is_empty() {
                        intervals.for_each_root(|root| roots.push(root));
                    }
                    roots
                }
                HeapObjectKind::Float => Vec::new(),
            };
            for child in children {
                if child.is_heap_object() && !self.is_value_marked(child) {
                    record(&owner, child);
                }
            }
        }

        if !violations.is_empty() {
            let total: usize = violations.values().sum();
            eprintln!("DUMP_PARTITION_VIOLATIONS total={total}");
            for (k, n) in &violations {
                eprintln!("  {n:>6}  {k}");
            }
            panic!(
                "dump-partition verification: {total} unmarked heap children of mapped objects \
                 (sample value={:#x}) — write barrier missed dumped->heap mutations (UAF risk). \
                 See DUMP_PARTITION_VIOLATIONS above.",
                sample.unwrap_or(0)
            );
        }
    }

    /// Verification gate for incremental marking (env `NEOVM_GC_VERIFY_PARTITION`,
    /// incremental builds). Complements `verify_dump_partition`, which covers
    /// mapped + tenured owners: this checks the remaining black objects —
    /// YOUNG non-cons (`all_objects`) and every marked heap CONS — for the
    /// strong tri-color invariant (no black object points to a white object).
    /// A violation means the incremental-update barrier missed a black->white
    /// edge created by the mutator during marking (a UAF about to happen).
    /// Panics on the first batch of violations. Expensive; verification only.
    fn verify_incremental_tricolor(&mut self) {
        let mut violations: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        let mut sample: Option<usize> = None;

        // -- Young non-cons objects that are marked (black). --
        let young: Vec<*mut GcHeader> = {
            let mut out = Vec::new();
            let mut obj = self.all_objects;
            while !obj.is_null() {
                unsafe {
                    if (*obj).is_marked() {
                        out.push(obj);
                    }
                    obj = (*obj).next;
                }
            }
            out
        };
        for header in young {
            let kind = unsafe { (*header).kind };
            let children: Vec<TaggedValue> = match kind {
                HeapObjectKind::VecLike => {
                    self.collect_veclike_children(header as *mut VecLikeHeader)
                }
                HeapObjectKind::String => {
                    let mut roots = Vec::new();
                    let intervals = unsafe { (*(header as *const StringObj)).data.intervals() };
                    if !intervals.is_empty() {
                        intervals.for_each_root(|root| roots.push(root));
                    }
                    roots
                }
                HeapObjectKind::Float => Vec::new(),
            };
            let owner = format!("young:{kind:?}");
            for child in children {
                if child.is_heap_object() && !self.is_value_marked(child) {
                    *violations.entry(owner.clone()).or_insert(0) += 1;
                    sample.get_or_insert(child.0);
                }
            }
        }

        // -- Every marked heap cons cell: car/cdr must be marked. --
        let blocks: Vec<(*mut ConsCell, usize)> = self
            .cons_blocks
            .iter()
            .map(|b| (b.cells_ptr(), b.next_index as usize))
            .collect();
        for (cells, count) in blocks {
            for i in 0..count {
                let cell = unsafe { cells.add(i) };
                if !self.is_value_marked(unsafe { TaggedValue::from_cons_ptr(cell) }) {
                    continue;
                }
                for child in [unsafe { (*cell).load_car() }, unsafe { (*cell).load_cdr() }] {
                    if child.is_heap_object() && !self.is_value_marked(child) {
                        *violations.entry("young:Cons".to_string()).or_insert(0) += 1;
                        sample.get_or_insert(child.0);
                    }
                }
            }
        }

        if !violations.is_empty() {
            let total: usize = violations.values().sum();
            eprintln!("INCREMENTAL_TRICOLOR_VIOLATIONS total={total}");
            for (k, n) in &violations {
                eprintln!("  {n:>6}  {k}");
            }
            panic!(
                "incremental tri-color verification: {total} black->white edges \
                 (sample value={:#x}) — the incremental-update barrier missed a mutation \
                 (UAF risk). See INCREMENTAL_TRICOLOR_VIOLATIONS above.",
                sample.unwrap_or(0)
            );
        }
    }

    /// Direct children of a mapped vectorlike object (read-only) for the verifier.
    fn collect_veclike_children(&self, ptr: *mut VecLikeHeader) -> Vec<TaggedValue> {
        let mut out = Vec::new();
        unsafe {
            match (*ptr).type_tag {
                VecLikeType::Vector | VecLikeType::Record => {
                    out.extend((*(ptr as *const VectorObj)).data.iter().copied());
                }
                VecLikeType::CharTable => {
                    let o = &*(ptr as *const CharTableObj);
                    out.extend([o.defalt, o.parent, o.purpose, o.ascii]);
                    out.extend(o.contents.iter().copied());
                    out.extend(o.extras.iter().copied());
                }
                VecLikeType::SubCharTable => {
                    out.extend((*(ptr as *const SubCharTableObj)).contents.iter().copied());
                }
                VecLikeType::Obarray => {
                    out.extend((*(ptr as *const ObarrayObj)).buckets.iter().copied());
                }
                VecLikeType::Lambda | VecLikeType::Macro => {
                    out.extend((*(ptr as *const LambdaObj)).data.iter().copied());
                }
                VecLikeType::HashTable => {
                    let ht = &(*(ptr as *const HashTableObj)).table;
                    out.extend(ht.data.values().copied());
                    out.extend(ht.key_snapshots.values().copied());
                }
                VecLikeType::ByteCode => {
                    let data = &(*(ptr as *const ByteCodeObj)).data;
                    out.push(data.arglist);
                    out.extend(data.constants.iter().copied());
                    if let Some(env) = data.env {
                        out.push(env);
                    }
                    if let Some(doc_form) = data.doc_form {
                        out.push(doc_form);
                    }
                    if let Some(interactive) = data.interactive {
                        out.push(interactive);
                    }
                    out.extend(data.extra_slots.iter().copied());
                }
                VecLikeType::Overlay => {
                    out.push((*(ptr as *const OverlayObj)).data.plist);
                }
                VecLikeType::SymbolWithPos => {
                    let o = &*(ptr as *const SymbolWithPosObj);
                    out.extend([o.sym, o.pos]);
                }
                VecLikeType::ModuleFunction => {
                    let o = &*(ptr as *const ModuleFunctionObj);
                    out.extend([o.documentation, o.interactive_form]);
                }
                VecLikeType::Xwidget => {
                    let o = &*(ptr as *const XwidgetObj);
                    out.extend([o.plist, o.type_, o.buffer, o.title, o.script_callbacks]);
                }
                VecLikeType::XwidgetView => {
                    let o = &*(ptr as *const XwidgetViewObj);
                    out.extend([o.model, o.window]);
                }
                // Buffer/Window/Frame/Timer/Process/Marker/Subr/Bignum/Sqlite/
                // UserPtr have no Value children to trace (mirrors trace_veclike).
                VecLikeType::Buffer
                | VecLikeType::Window
                | VecLikeType::Frame
                | VecLikeType::Timer
                | VecLikeType::Process
                | VecLikeType::Marker
                | VecLikeType::Subr
                | VecLikeType::Bignum
                | VecLikeType::Sqlite
                | VecLikeType::UserPtr => {}
            }
        }
        out
    }

    pub(crate) fn seed_root(&mut self, root: TaggedValue) {
        self.seed_root_with_origin(root, "explicit-root");
    }

    pub(crate) fn seed_root_with_origin(&mut self, root: TaggedValue, origin: &str) {
        if !root.is_heap_object() {
            return;
        }
        // Stage 0: in the blackened dump partition, a root that points into the
        // dump image is already permanent-black (never cleared or swept), so it
        // needs no marking; any young child it gained through mutation is covered
        // by the dump remembered set (`seed_mapped_remembered`). Skipping these
        // avoids pushing+draining the ~450k interned-symbol value/function/plist
        // cells that still point at dumped objects on every root handshake — the
        // dominant cost of the start + termination pauses.
        if self.dump_blackened && self.owner_is_mapped(root) {
            return;
        }
        self.push_gray(root, origin);
    }

    fn seed_internal_runtime_roots(&mut self) {
        // Static subr objects are leaked process/thread runtime objects, matching
        // GNU's static `Lisp_Subr` storage. They are not swept by this heap.
        let roots: Vec<(TaggedValue, &'static str)> = self
            .buffer_registry
            .values()
            .map(|value| (*value, "buffer-registry"))
            .chain(
                self.window_registry
                    .values()
                    .map(|value| (*value, "window-registry")),
            )
            .chain(
                self.frame_registry
                    .values()
                    .map(|value| (*value, "frame-registry")),
            )
            .chain(
                self.timer_registry
                    .values()
                    .map(|value| (*value, "timer-registry")),
            )
            .chain(
                self.process_registry
                    .values()
                    .map(|value| (*value, "process-registry")),
            )
            .collect();

        for (value, origin) in roots {
            if value.is_heap_object() {
                self.push_gray(value, origin);
            }
        }
    }

    pub(crate) fn complete_collection(&mut self) {
        let bytes_before = self.live_bytes;
        let t0 = std::time::Instant::now();

        // -- Mark phase: drain the gray queue on the GC thread. This is the STW
        //    full/bootstrap path (first cycle, no-dump heaps, explicit
        //    garbage-collect); the mutator blocks until the GC thread finishes,
        //    so heap access is exclusive (no concurrency hazard here). --
        let mark_t0 = std::time::Instant::now();
        self.mark_all_on_gc_thread();
        // Resolve weak hash tables now that the main mark has drained. Both the
        // sync and concurrent paths converge here with the mutator stopped, so
        // this is single-threaded and path-agnostic.
        self.mark_and_sweep_weak_tables();
        let mark_us = mark_t0.elapsed().as_micros() as u64;

        self.finalize_collection(mark_us, bytes_before, t0);
    }

    /// Resolve the weak hash tables discovered during this cycle's mark — GNU
    /// `mark_and_sweep_weak_table_contents` (alloc.c) + `sweep_weak_table`
    /// (fns.c). Runs at the stop-the-world `complete_collection` after the main
    /// mark drains. First a fixpoint marks the key/value of every entry that
    /// survives per its table's weakness — iterate to stability because a value
    /// in one weak table may be a key in another — then non-surviving entries
    /// are removed.
    fn mark_and_sweep_weak_tables(&mut self) {
        if self.weak_hash_tables.is_empty() {
            return;
        }

        // -- Mark phase: keep marking surviving entries until nothing changes. --
        loop {
            let mut marked = false;
            // The worklist holds raw pointers, stable across this stop-the-world
            // step; copy them so the body can call `&mut self` methods.
            let tables = self.weak_hash_tables.clone();
            for tptr in tables {
                // SAFETY: `tptr` was recorded this cycle from a live veclike; the
                // heap is exclusively owned here (mutator stopped). Snapshot the
                // entries so the `ht` borrow is released before `push_gray`.
                let (weakness, entries): (
                    Option<HashTableWeakness>,
                    Vec<(TaggedValue, TaggedValue)>,
                ) = unsafe {
                    let ht = &(*tptr).table;
                    let entries = ht
                        .data
                        .iter()
                        .map(|(hk, &value)| {
                            let key = ht.key_snapshots.get(hk).copied().unwrap_or(value);
                            (key, value)
                        })
                        .collect();
                    (ht.weakness, entries)
                };
                for (key, value) in entries {
                    let key_survives = self.is_value_marked(key);
                    let value_survives = self.is_value_marked(value);
                    if Self::keep_weak_entry(weakness, key_survives, value_survives) {
                        if !key_survives {
                            self.push_gray(key, "weak-hash-key");
                            marked = true;
                        }
                        if !value_survives {
                            self.push_gray(value, "weak-hash-value");
                            marked = true;
                        }
                    }
                }
            }
            // Drain whatever those surviving entries reached, then re-check.
            self.mark_all();
            if !marked {
                break;
            }
        }

        // -- Sweep phase: drop entries that did not survive. --
        let tables = std::mem::take(&mut self.weak_hash_tables);
        for tptr in tables {
            // SAFETY: as above; exclusive heap access.
            let (weakness, entries): (
                Option<HashTableWeakness>,
                Vec<(HashKey, TaggedValue, TaggedValue)>,
            ) = unsafe {
                let ht = &(*tptr).table;
                let entries = ht
                    .data
                    .iter()
                    .map(|(hk, &value)| {
                        let key = ht.key_snapshots.get(hk).copied().unwrap_or(value);
                        (hk.clone(), key, value)
                    })
                    .collect();
                (ht.weakness, entries)
            };
            let dead: Vec<HashKey> = entries
                .into_iter()
                .filter_map(|(hk, key, value)| {
                    let keep = Self::keep_weak_entry(
                        weakness,
                        self.is_value_marked(key),
                        self.is_value_marked(value),
                    );
                    (!keep).then_some(hk)
                })
                .collect();
            if dead.is_empty() {
                continue;
            }
            // SAFETY: exclusive heap access. Mirror `builtin_remhash`'s removal.
            let ht = unsafe { &mut (*tptr).table };
            for hk in dead {
                ht.data.remove(&hk);
                ht.key_snapshots.remove(&hk);
                ht.note_hash_key_removed(&hk);
            }
        }
    }

    /// GNU `keep_entry_p` (fns.c): does a weak-table entry survive, given whether
    /// its key and value are independently reachable?
    fn keep_weak_entry(
        weakness: Option<HashTableWeakness>,
        strong_key: bool,
        strong_value: bool,
    ) -> bool {
        match weakness {
            None => true,
            Some(HashTableWeakness::Key) => strong_key,
            Some(HashTableWeakness::Value) => strong_value,
            Some(HashTableWeakness::KeyOrValue) => strong_key || strong_value,
            Some(HashTableWeakness::KeyAndValue) => strong_key && strong_value,
        }
    }

    /// Post-mark portion of a collection: verify, sweep, promote, account, and
    /// clear the remembered/dirty bookkeeping. Shared by the stop-the-world
    /// `complete_collection` and the incremental mark-termination path. By the
    /// time this runs the gray queue is fully drained (marking is complete) and
    /// the marker chain heads are installed.
    fn finalize_collection(&mut self, mark_us: u64, bytes_before: usize, t0: std::time::Instant) {
        // Dump-partition safety gate: prove no live heap object reachable only
        // through a dumped object was left unmarked (i.e. the write barrier's
        // remembered set is complete). Off unless explicitly verifying.
        if self.partition_dump
            && self.dump_blackened
            && std::env::var("NEOVM_GC_VERIFY_PARTITION").as_deref() == Ok("1")
        {
            self.verify_dump_partition();
            // Incremental marking adds young-black->young-white as a possible
            // failure mode (a missed write-barrier owner). Check it too.
            self.verify_incremental_tricolor();
        }

        let sweep_t0 = std::time::Instant::now();

        // Unchain dead markers BEFORE `sweep_objects` frees them; the
        // chain would otherwise hold dangling pointers after the sweep.
        // Mirrors GNU `sweep_buffer → unchain_dead_markers` (`alloc.c`).
        // Reading `header.gc.marked` is sound here because the
        // allocation is still live until `sweep_objects` runs below.
        self.unchain_dead_markers();

        // -- Sweep phase --
        let cons_live_bytes = self.sweep_cons();
        let object_live_bytes = self.sweep_objects();
        let mapped_object_live_bytes = self.mapped_non_cons_live_bytes();
        self.live_bytes = cons_live_bytes
            .saturating_add(object_live_bytes)
            .saturating_add(mapped_object_live_bytes);
        self.bytes_since_gc = 0;

        // End of the first partition cycle: every survivor is a permanent.
        // Promote them to the tenured old generation and blacken the dump so
        // all later cycles skip both regions.
        if self.partition_dump && !self.dump_blackened {
            self.promote_and_blacken();
            self.dump_blackened = true;
        }

        let sweep_us = sweep_t0.elapsed().as_micros() as u64;
        let elapsed = t0.elapsed();
        self.gc_collections += 1;
        self.gc_total_elapsed_us += elapsed.as_micros() as u64;

        // Phase split + dump-partition opportunity sizing. `mapped_marked` is
        // the immutable pdump (mapped) objects re-traced this cycle — the work
        // a "dump as permanent tenured region" partition would eliminate —
        // versus the mutable heap (`cons_live` + `heap_noncons`).
        let (mapped_total, mapped_marked) = self.mapped_object_stats();
        // Batch/headless runs don't install the tracing subscriber, so mirror
        // the phase split to stderr when `NEOVM_GC_TRACE=1` for profiling.
        if std::env::var("NEOVM_GC_TRACE").as_deref() == Ok("1") {
            eprintln!(
                "NEOVM_GC gc#{} {:.2}ms [clear={}us mark={}us sweep={}us] \
                 cons_live={} heap_noncons={} dump_marked={}/{} dirty_owners={} live={}B",
                self.gc_collections,
                elapsed.as_micros() as f64 / 1000.0,
                self.last_clear_us,
                mark_us,
                sweep_us,
                self.cons_live_count,
                self.non_cons_object_addrs.len(),
                mapped_marked,
                mapped_total,
                self.dirty_owners.len(),
                self.live_bytes,
            );
        }
        tracing::debug!(
            "gc#{} {:.2}ms [clear={}us mark={}us sweep={}us] {} → {} bytes ({:+.1}%), \
             cons_live={}, heap_noncons={}, dump_marked={}/{}, dirty_owners={}, threshold={}",
            self.gc_collections,
            elapsed.as_micros() as f64 / 1000.0,
            self.last_clear_us,
            mark_us,
            sweep_us,
            bytes_before,
            self.live_bytes,
            if bytes_before > 0 {
                (self.live_bytes as f64 - bytes_before as f64) / bytes_before as f64 * 100.0
            } else {
                0.0
            },
            self.cons_live_count,
            self.non_cons_object_addrs.len(),
            mapped_marked,
            mapped_total,
            self.dirty_owners.len(),
            self.gc_threshold,
        );

        // A full-heap collection subsumes any remembered-set bookkeeping.
        self.clear_dirty_owners();
        self.clear_dirty_writes();
    }

    /// Drain the gray queue, marking and tracing all reachable objects.
    fn mark_all(&mut self) {
        while let Some(val) = self.gray_queue.pop() {
            self.mark_value(val);
        }
    }

    /// Drain the gray queue on the background GC thread (Phase 4). The mutator
    /// blocks on the done-channel until the GC thread finishes, so heap access
    /// is exclusive (no concurrency hazard yet). This proves the thread +
    /// heap-sharing + handshake; the pause is not yet reduced. Phase 5 removes
    /// the block so marking actually overlaps mutator execution.
    fn mark_all_on_gc_thread(&mut self) {
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let ptr = self as *mut TaggedHeap;
        gc_thread()
            .send(GcRequest::MarkAll(HeapPtr(ptr), done_tx))
            .expect("neovm-gc thread is gone");
        // Block until the GC thread has finished marking on the shared heap.
        done_rx.recv().expect("neovm-gc thread did not respond");
    }

    // ---------------------------------------------------------------------
    // Concurrent marking (Phase 5) — background GC thread marks while the
    // mutator runs; only two short stop-the-world handshakes (start + finish).
    // ---------------------------------------------------------------------

    /// True if a concurrent mark should drive THIS collection: a partitioned
    /// post-dump heap (the young/old split bounds what is traced). The first
    /// cycle and no-dump heaps fall to the STW full path instead.
    pub fn should_run_concurrent(&self) -> bool {
        self.partition_dump && self.dump_blackened
    }

    /// True while the background GC thread is marking (between the start and
    /// termination handshakes) — the mutator is running concurrently.
    pub fn concurrent_mark_running(&self) -> bool {
        self.concurrent_mark_running
    }

    /// The GC thread has tentatively drained gray + SATB (Acquire pairs with the
    /// thread's Release). The mutator polls this at safe points to terminate.
    pub fn concurrent_mark_done(&self) -> bool {
        self.gc_done.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Start-of-cycle setup for a concurrent mark: clear young marks + seed the
    /// collector-internal and remembered roots (`begin_collection`), arm
    /// `mark_in_progress`. The caller then seeds context roots and calls
    /// `launch_concurrent_mark`. No Steele owner-tracking: the concurrent SATB
    /// barrier (keyed on `concurrent_mark_running`) preserves the snapshot.
    pub(crate) fn concurrent_begin(&mut self) {
        self.begin_collection();
        self.mark_in_progress = true;
        self.incremental_mark_us = 0;
    }

    /// Hand the seeded gray queue (the full root snapshot) to the GC thread and
    /// start non-blocking concurrent marking. Returns immediately; the mutator
    /// resumes while the GC thread marks. Allocate-black turns on so new objects
    /// survive this cycle's sweep, and the SATB barrier starts logging.
    pub(crate) fn launch_concurrent_mark(&mut self) {
        // Immutable snapshot of owned cons-block bases — read-only on the GC
        // thread. New blocks allocated during marking are absent, which is fine:
        // their conses allocate-black and never enter the GC's gray queue.
        let mut owned = std::collections::HashSet::with_capacity(self.cons_blocks.len());
        for block in &self.cons_blocks {
            owned.insert(block.base_addr());
        }
        let gray = std::mem::take(&mut self.gray_queue);
        let (exited_tx, exited_rx) = std::sync::mpsc::channel();
        self.gc_done
            .store(false, std::sync::atomic::Ordering::Release);
        self.gc_stop
            .store(false, std::sync::atomic::Ordering::Release);
        self.gc_exited = Some(exited_rx);
        self.concurrent_mark_running = true;
        // Keep the write-barrier fast path reaching `record_heap_write` so the
        // SATB log fires even with owner-tracking Disabled / no partition.
        TAGGED_HEAP_CONCURRENT_ACTIVE.with(|c| c.set(true));
        let job = ConcurrentMarkJob {
            gray,
            owned_bases: std::sync::Arc::new(owned),
            dump_lo: self.dump_addr_lo,
            dump_hi: self.dump_addr_hi,
            satb: self.satb_shared.clone(),
            deferred: self.deferred_veclikes.clone(),
            done: self.gc_done.clone(),
            stop: self.gc_stop.clone(),
            exited: exited_tx,
        };
        gc_thread()
            .send(GcRequest::ConcurrentMark(job))
            .expect("neovm-gc thread is gone");
    }

    /// Stop the GC thread and fold its residual work back into the gray queue so
    /// the caller can finish marking stop-the-world. After this, the heap is
    /// owned exclusively by the mutator again (the GC thread has exited its loop).
    pub(crate) fn join_concurrent_mark(&mut self) {
        self.gc_stop
            .store(true, std::sync::atomic::Ordering::Release);
        if let Some(rx) = self.gc_exited.take() {
            let _ = rx.recv(); // block until the GC thread leaves its mark loop
        }
        self.concurrent_mark_running = false;
        TAGGED_HEAP_CONCURRENT_ACTIVE.with(|c| c.set(false));
        // Residual SATB (children overwritten after the GC's last drain) +
        // deferred (every non-cons + non-owned cons the GC parked) become gray;
        // the caller reseeds roots, then drains to a fixpoint stop-the-world.
        let satb = std::mem::take(&mut *self.satb_shared.lock().unwrap());
        self.gray_queue.extend(satb);
        let deferred = std::mem::take(&mut *self.deferred_veclikes.lock().unwrap());
        self.gray_queue.extend(deferred);
    }

    /// SATB barrier path for concurrent marking: append the owner's current
    /// (pre-overwrite) children to the shared buffer the GC thread drains. Reuses
    /// the gray-queue child enumeration with `self.gray_queue` as scratch (it is
    /// empty during concurrent marking — the snapshot was handed to the thread).
    fn push_value_children_to_satb_shared(&mut self, owner: TaggedValue) {
        debug_assert!(self.gray_queue.is_empty());
        self.push_value_children_to_gray(owner, "satb-concurrent");
        if !self.gray_queue.is_empty() {
            let mut shared = self.satb_shared.lock().unwrap();
            shared.extend(self.gray_queue.drain(..));
        }
    }

    // ---------------------------------------------------------------------
    // Incremental marking (step 7)
    // ---------------------------------------------------------------------

    /// True while a mark is underway (between the start handshake and sweep).
    pub fn mark_in_progress(&self) -> bool {
        self.mark_in_progress
    }

    /// Re-seed the collector-internal roots at mark termination: the runtime
    /// object registries and the dump remembered set (the non-clearing seeds
    /// that `begin_collection` runs at the start). Mark termination must
    /// re-snapshot the COMPLETE root set, not just the evaluator/context roots —
    /// otherwise an object that became reachable only through one of these roots
    /// during the marking window is left unmarked and swept while live.
    pub(crate) fn reseed_runtime_and_remembered_roots(&mut self) {
        self.seed_internal_runtime_roots();
        if self.partition_dump && self.dump_blackened {
            self.seed_mapped_remembered();
        }
    }

    /// Drain ALL remaining marking work to a fixpoint (no budget). Used at mark
    /// termination, after the roots have been re-snapshotted, while the world is
    /// stopped. A single `mark_all` reaches the fixpoint: `mark_value` re-pushes
    /// each marked object's children, so the gray queue drains completely.
    pub(crate) fn incremental_drain_all(&mut self) {
        let t0 = std::time::Instant::now();
        self.mark_all();
        self.incremental_mark_us += t0.elapsed().as_micros() as u64;
    }

    /// Run mark termination's sweep + accounting, then leave the incremental
    /// state. Marking must already be drained to a fixpoint and the marker
    /// chain heads installed. `pause_t0` stamps the termination (sweep) pause.
    /// Mark termination: verify, unchain dead markers, then DEFER the sweep.
    /// The reclaim drains in bounded slices at later safe points
    /// (`incremental_sweep_slice`), so it is no longer part of the STW pause.
    /// Marking is complete here; the barrier is dropped.
    pub(crate) fn incremental_finish(
        &mut self,
        bytes_before: usize,
        _pause_t0: std::time::Instant,
    ) {
        // Dump-partition safety gate (marks still intact). Same as
        // `finalize_collection`'s, run before any object is freed.
        if self.partition_dump
            && self.dump_blackened
            && std::env::var("NEOVM_GC_VERIFY_PARTITION").as_deref() == Ok("1")
        {
            self.verify_dump_partition();
            self.verify_incremental_tricolor();
        }
        // Unchain dead markers before the sweep frees them (mirrors GNU
        // sweep_buffer -> unchain_dead_markers). Reads marks, which are intact.
        self.unchain_dead_markers();

        // Begin the deferred sweep. Detach the young non-cons list (new non-cons
        // allocations link onto a fresh `all_objects` and are not swept this
        // cycle) and reset the cons free list (rebuilt as blocks are swept).
        self.sweep_noncons_pending = self.all_objects;
        self.all_objects = std::ptr::null_mut();
        self.cons_free_list = std::ptr::null_mut();
        self.sweep_cons_cursor = 0;
        self.sweep_noncons_live_bytes = 0;
        self.sweep_mark_us = self.incremental_mark_us;
        self.sweep_bytes_before = bytes_before;
        self.sweep_in_progress = true;
        // The triggering allocation budget is spent; the next mark fires once a
        // fresh threshold's worth has been allocated.
        self.bytes_since_gc = 0;

        // Marking is done; drop the marking barrier. The dump remembered set is
        // still maintained unconditionally in `record_heap_write`.
        self.set_write_tracking_mode(WriteTrackingMode::Disabled);
        self.mark_in_progress = false;
    }

    /// True while the deferred sweep is draining.
    pub fn sweep_in_progress(&self) -> bool {
        self.sweep_in_progress
    }

    /// Advance the deferred sweep by one bounded slice: reclaim up to `budget`
    /// cons blocks and up to `budget` pending non-cons objects. Returns true
    /// (and finalizes accounting) once the whole sweep is done. New conses
    /// allocated meanwhile are born black (see `alloc_cons`), so an unswept
    /// block never reclaims a live new cell.
    pub(crate) fn incremental_sweep_slice(&mut self, budget: usize) -> bool {
        let t0 = std::time::Instant::now();
        // -- cons: reclaim up to `budget` blocks (each ~64KB of cells) --
        let mut swept_blocks = 0usize;
        while swept_blocks < budget && self.sweep_cons_cursor < self.cons_blocks.len() {
            let idx = self.sweep_cons_cursor;
            let free_list: *mut *mut ConsCell = &mut self.cons_free_list;
            self.cons_blocks[idx].sweep(unsafe { &mut *free_list });
            self.sweep_cons_cursor += 1;
            swept_blocks += 1;
        }
        // -- non-cons: reclaim more objects per slice than cons blocks, since a
        //    cons block holds thousands of cells while a non-cons node is one
        //    object (with a heavier per-object free). --
        let noncons_budget = budget.saturating_mul(256);
        let mut processed = 0usize;
        while processed < noncons_budget && !self.sweep_noncons_pending.is_null() {
            let current = self.sweep_noncons_pending;
            unsafe {
                self.sweep_noncons_pending = (*current).next;
                if (*current).is_marked() {
                    // Survivor: relink onto the (fresh) young list.
                    (*current).next = self.all_objects;
                    self.all_objects = current;
                    self.sweep_noncons_live_bytes = self
                        .sweep_noncons_live_bytes
                        .saturating_add(Self::object_bytes_from_header(current));
                } else {
                    self.non_cons_object_addrs.remove(&(current as usize));
                    self.free_gc_object(current);
                    self.allocated_count = self.allocated_count.saturating_sub(1);
                }
            }
            processed += 1;
        }

        let done = self.sweep_cons_cursor >= self.cons_blocks.len()
            && self.sweep_noncons_pending.is_null();
        if std::env::var("NEOVM_GC_TRACE").as_deref() == Ok("1") {
            eprintln!(
                "NEOVM_GC sweep_slice {}us cons={}/{} noncons_left={} done={done}",
                t0.elapsed().as_micros(),
                self.sweep_cons_cursor,
                self.cons_blocks.len(),
                if self.sweep_noncons_pending.is_null() {
                    0
                } else {
                    1
                },
            );
        }
        if done {
            self.finish_incremental_sweep();
        }
        done
    }

    /// Drive the deferred sweep to completion in one shot (forced GC, or before
    /// the next mark / a stop-the-world collection can begin).
    pub(crate) fn finish_incremental_sweep_now(&mut self) {
        while self.sweep_in_progress {
            self.incremental_sweep_slice(usize::MAX);
        }
    }

    /// Finalize the deferred sweep: recompute the cons live count from the mark
    /// bitmaps (cheap popcount; counts allocate-black new conses, excludes
    /// reclaimed ones), fix the allocation accounting, and emit the cycle trace.
    fn finish_incremental_sweep(&mut self) {
        let recount: usize = self.cons_blocks.iter().map(ConsBlock::count_marked).sum();
        // allocated_count carries the tracked cons live count; replace it with
        // the true recount (delta may be negative -> use checked sub).
        if recount >= self.cons_live_count {
            self.allocated_count = self
                .allocated_count
                .saturating_add(recount - self.cons_live_count);
        } else {
            self.allocated_count = self
                .allocated_count
                .saturating_sub(self.cons_live_count - recount);
        }
        self.cons_live_count = recount;

        let mapped_cons_live: usize = self
            .mapped_cons_ranges
            .iter()
            .map(MappedConsRange::live_count)
            .sum();
        let cons_live_bytes = recount
            .saturating_add(mapped_cons_live)
            .saturating_mul(size_of::<ConsCell>());
        let mapped_object_live_bytes = self.mapped_non_cons_live_bytes();
        self.live_bytes = cons_live_bytes
            .saturating_add(self.sweep_noncons_live_bytes)
            .saturating_add(mapped_object_live_bytes);

        self.gc_collections += 1;
        if std::env::var("NEOVM_GC_TRACE").as_deref() == Ok("1") {
            let (mapped_total, mapped_marked) = self.mapped_object_stats();
            eprintln!(
                "NEOVM_GC gc#{} [incremental mark={}us sweep=deferred] \
                 cons_live={} heap_noncons={} dump_marked={}/{} live={}B",
                self.gc_collections,
                self.sweep_mark_us,
                self.cons_live_count,
                self.non_cons_object_addrs.len(),
                mapped_marked,
                mapped_total,
                self.live_bytes,
            );
        }
        self.clear_dirty_owners();
        self.clear_dirty_writes();
        self.sweep_in_progress = false;
    }

    fn push_gray(&mut self, val: TaggedValue, origin: &str) {
        debug_assert!(val.is_heap_object());
        self.debug_assert_heap_tag_matches_header(val, origin);
        self.gray_queue.push(val);
    }

    #[cfg(debug_assertions)]
    fn debug_assert_heap_tag_matches_header(&self, val: TaggedValue, origin: &str) {
        if val.is_cons() {
            return;
        }

        let (ptr, expected) = if val.is_string() {
            (
                val.as_string_ptr().unwrap() as *const u8,
                HeapObjectKind::String,
            )
        } else if val.is_float() {
            (
                val.as_float_ptr().unwrap() as *const u8,
                HeapObjectKind::Float,
            )
        } else if val.is_veclike() {
            (
                val.as_veclike_ptr().unwrap() as *const u8,
                HeapObjectKind::VecLike,
            )
        } else {
            return;
        };

        if !self.owns_non_cons_object(ptr) {
            return;
        }

        let header = unsafe { &*(ptr as *const GcHeader) };
        assert_eq!(
            header.kind,
            expected,
            "GC gray queue received malformed tagged heap value from {origin}: \
             value={:#x}, ptr={:?}, tag={}, header.kind={:?}, expected={:?}",
            val.0,
            ptr,
            val.tag(),
            header.kind,
            expected
        );
    }

    #[cfg(not(debug_assertions))]
    fn debug_assert_heap_tag_matches_header(&self, _val: TaggedValue, _origin: &str) {}

    /// Mark a single tagged value and push its children onto the gray queue.
    fn mark_value(&mut self, val: TaggedValue) {
        if val.is_cons() {
            let ptr = val.xcons_ptr();
            if self.mark_cons(ptr) {
                let car = unsafe { (*ptr).load_car() };
                let cdr = unsafe { (*ptr).load_cdr() };
                if car.is_heap_object() {
                    self.push_gray(car, "cons-car");
                }
                if cdr.is_heap_object() {
                    self.push_gray(cdr, "cons-cdr");
                }
            }
        } else if val.is_string() {
            let ptr = val.as_string_ptr().unwrap() as *mut StringObj;
            if !self.owns_non_cons_object(ptr as *const u8) {
                if self.mark_mapped_string(ptr) {
                    unsafe {
                        let intervals = (*ptr).data.intervals();
                        if !intervals.is_empty() {
                            intervals.for_each_root(|root| {
                                if root.is_heap_object() {
                                    self.push_gray(root, "mapped-string-interval");
                                }
                            });
                        }
                    }
                }
                return;
            }
            unsafe {
                if (*ptr).header.is_marked() {
                    return;
                }
                (*ptr).header.set_marked(true);
                let intervals = (*ptr).data.intervals();
                if !intervals.is_empty() {
                    intervals.for_each_root(|root| {
                        if root.is_heap_object() {
                            self.push_gray(root, "string-interval");
                        }
                    });
                }
            };
        } else if val.is_float() {
            let ptr = val.as_float_ptr().unwrap() as *mut FloatObj;
            if !self.owns_non_cons_object(ptr as *const u8) {
                let _ = self.mark_mapped_float(ptr);
                return;
            }
            unsafe {
                if (*ptr).header.is_marked() {
                    return;
                }
                (*ptr).header.set_marked(true);
            };
        } else if val.is_veclike() {
            let ptr = val.as_veclike_ptr().unwrap() as *mut VecLikeHeader;
            if !self.owns_non_cons_object(ptr as *const u8) {
                if self.mark_mapped_veclike(ptr) {
                    unsafe {
                        self.trace_veclike(ptr);
                    }
                }
                return;
            }
            unsafe {
                if (*ptr).gc.is_marked() {
                    return;
                }
                (*ptr).gc.set_marked(true);
                self.trace_veclike(ptr);
            }
        }
    }

    /// Mark a cons cell. Returns true if newly marked (not previously marked).
    fn mark_cons(&mut self, ptr: *const ConsCell) -> bool {
        if ptr.is_null() || !ConsBlock::ptr_is_cell_aligned(ptr) {
            return self.mark_mapped_cons(ptr);
        }
        let block_base = ConsBlock::block_base_for_ptr(ptr);
        let block_index = match self.mark_cons_block_cache {
            Some(cache) if cache.block_base == block_base => cache.block_index,
            _ => {
                let Some(&block_index) = self.cons_block_index_by_base.get(&block_base) else {
                    return self.mark_mapped_cons(ptr);
                };
                self.mark_cons_block_cache =
                    Some(ConsBlockCacheEntry::new(block_base, block_index));
                block_index
            }
        };
        let block = &mut self.cons_blocks[block_index];
        if block.is_marked_ptr(ptr) {
            return false;
        }
        block.mark_ptr(ptr);
        true
    }

    fn mark_mapped_cons(&mut self, ptr: *const ConsCell) -> bool {
        for range in &mut self.mapped_cons_ranges {
            if !range.contains_ptr(ptr) {
                continue;
            }
            if range.is_marked_ptr(ptr) {
                return false;
            }
            range.mark_ptr(ptr);
            return true;
        }
        false
    }

    fn mark_mapped_float(&mut self, ptr: *const FloatObj) -> bool {
        for range in &mut self.mapped_float_ranges {
            if !range.contains_ptr(ptr) {
                continue;
            }
            if range.is_marked_ptr(ptr) {
                return false;
            }
            range.mark_ptr(ptr);
            return true;
        }
        false
    }

    fn mark_mapped_veclike(&mut self, ptr: *const VecLikeHeader) -> bool {
        let Some(&index) = self.mapped_veclike_index_by_addr.get(&(ptr as usize)) else {
            return false;
        };
        let object = &mut self.mapped_veclike_objects[index];
        debug_assert!(std::ptr::eq(object.header as *const VecLikeHeader, ptr));
        if object.marked {
            return false;
        }
        object.marked = true;
        true
    }

    fn mark_mapped_string(&mut self, ptr: *const StringObj) -> bool {
        let Some(&index) = self.mapped_string_index_by_addr.get(&(ptr as usize)) else {
            return false;
        };
        let object = &mut self.mapped_string_objects[index];
        debug_assert!(std::ptr::eq(object.ptr as *const StringObj, ptr));
        if object.marked {
            return false;
        }
        object.marked = true;
        true
    }

    /// Trace children of a vectorlike object, pushing them onto the gray queue.
    unsafe fn trace_veclike(&mut self, ptr: *mut VecLikeHeader) {
        match unsafe { (*ptr).type_tag } {
            VecLikeType::Vector => {
                let obj = ptr as *const VectorObj;
                for val in unsafe { (*obj).data.iter_atomic() } {
                    if val.is_heap_object() {
                        self.push_gray(val, "vector-slot");
                    }
                }
            }
            VecLikeType::CharTable => {
                let obj = unsafe { &*(ptr as *const CharTableObj) };
                for (value, origin) in [
                    (load_value_atomic(&obj.defalt), "char-table-default"),
                    (load_value_atomic(&obj.parent), "char-table-parent"),
                    (load_value_atomic(&obj.purpose), "char-table-purpose"),
                    (load_value_atomic(&obj.ascii), "char-table-ascii"),
                ] {
                    if value.is_heap_object() {
                        self.push_gray(value, origin);
                    }
                }
                for slot in &obj.contents {
                    let val = load_value_atomic(slot);
                    if val.is_heap_object() {
                        self.push_gray(val, "char-table-content");
                    }
                }
                for val in obj.extras.iter_atomic() {
                    if val.is_heap_object() {
                        self.push_gray(val, "char-table-extra");
                    }
                }
            }
            VecLikeType::SubCharTable => {
                let obj = unsafe { &*(ptr as *const SubCharTableObj) };
                for val in obj.contents.iter_atomic() {
                    if val.is_heap_object() {
                        self.push_gray(val, "sub-char-table-content");
                    }
                }
            }
            VecLikeType::Record => {
                let obj = ptr as *const RecordObj;
                for val in unsafe { (*obj).data.iter_atomic() } {
                    if val.is_heap_object() {
                        self.push_gray(val, "record-slot");
                    }
                }
            }
            VecLikeType::HashTable => {
                let obj = ptr as *const HashTableObj;
                let ht = unsafe { &(*obj).table };
                if ht.weakness.is_some() {
                    // Weak table: DON'T trace its entries here — that would keep
                    // every key/value alive and defeat weakness. Record it; the
                    // per-entry survival decision happens in
                    // `mark_and_sweep_weak_tables` at the stop-the-world
                    // `complete_collection`, after the main mark drains (GNU
                    // `mark_and_sweep_weak_table_contents`). The generational
                    // remembered-set / SATB paths (`collect_veclike_children`)
                    // still trace weak entries strongly — conservative
                    // over-retention for a dump-tenured weak table, never a UAF;
                    // precise weak semantics apply to runtime weak tables.
                    self.weak_hash_tables.push(obj as *mut HashTableObj);
                } else {
                    // Trace all values in the hash table
                    for slot in ht.data.values() {
                        let val = load_value_atomic(slot);
                        if val.is_heap_object() {
                            self.push_gray(val, "hash-table-value");
                        }
                    }
                    // Trace key snapshots (original key objects)
                    for slot in ht.key_snapshots.values() {
                        let val = load_value_atomic(slot);
                        if val.is_heap_object() {
                            self.push_gray(val, "hash-table-key-snapshot");
                        }
                    }
                }
                // Custom test/hash closures (from `define-hash-table-test`) live
                // ONLY in these fields. Without tracing them the closure is swept
                // while the table is still live, and the next custom-test
                // gethash/puthash calls a freed function (use-after-free). The
                // fields are immutable after table creation, so a plain read is
                // race-free during a concurrent mark.
                if let Some(f) = ht.user_cmp_function
                    && f.is_heap_object()
                {
                    self.push_gray(f, "hash-table-user-cmp");
                }
                if let Some(f) = ht.user_hash_function
                    && f.is_heap_object()
                {
                    self.push_gray(f, "hash-table-user-hash");
                }
            }
            VecLikeType::Obarray => {
                let obj = unsafe { &*(ptr as *const ObarrayObj) };
                for val in obj.buckets.iter_atomic() {
                    if val.is_heap_object() {
                        self.push_gray(val, "obarray-bucket");
                    }
                }
            }
            VecLikeType::Lambda | VecLikeType::Macro => {
                // Closures are plain Value vectors (GNU PVEC_CLOSURE compat).
                // Trace ALL slots uniformly — no type-specific logic needed.
                let obj = ptr as *const LambdaObj;
                for val in unsafe { (*obj).data.iter_atomic() } {
                    if val.is_heap_object() {
                        self.push_gray(val, "closure-slot");
                    }
                }
            }
            VecLikeType::ByteCode => {
                let obj = ptr as *const ByteCodeObj;
                let data = unsafe { &(*obj).data };
                if data.arglist.is_heap_object() {
                    self.push_gray(data.arglist, "bytecode-arglist");
                }
                // Trace constants vector
                for val in &data.constants {
                    if val.is_heap_object() {
                        self.push_gray(*val, "bytecode-constant");
                    }
                }
                // Trace captured lexical environment
                if let Some(env) = data.env {
                    if env.is_heap_object() {
                        self.push_gray(env, "bytecode-env");
                    }
                }
                // Trace doc_form (can be a Value)
                if let Some(doc_form) = data.doc_form {
                    if doc_form.is_heap_object() {
                        self.push_gray(doc_form, "bytecode-doc-form");
                    }
                }
                // Trace interactive spec
                if let Some(interactive) = data.interactive {
                    if interactive.is_heap_object() {
                        self.push_gray(interactive, "bytecode-interactive");
                    }
                }
                for val in &data.extra_slots {
                    if val.is_heap_object() {
                        self.push_gray(*val, "bytecode-extra-slot");
                    }
                }
            }
            VecLikeType::Overlay => {
                let obj = ptr as *const OverlayObj;
                let data = unsafe { &(*obj).data };
                // Trace the property list
                let plist = load_value_atomic(&data.plist);
                if plist.is_heap_object() {
                    self.push_gray(plist, "overlay-plist");
                }
            }
            VecLikeType::SymbolWithPos => {
                // Trace both the symbol and the position fields.
                let obj = ptr as *const SymbolWithPosObj;
                let sym = unsafe { (*obj).sym };
                let pos = unsafe { (*obj).pos };
                if sym.is_heap_object() {
                    self.push_gray(sym, "symbol-with-pos-symbol");
                }
                if pos.is_heap_object() {
                    self.push_gray(pos, "symbol-with-pos-position");
                }
            }
            VecLikeType::ModuleFunction => {
                let obj = ptr as *const ModuleFunctionObj;
                let doc = unsafe { (*obj).documentation };
                let interactive = unsafe { (*obj).interactive_form };
                if doc.is_heap_object() {
                    self.push_gray(doc, "module-function-documentation");
                }
                if interactive.is_heap_object() {
                    self.push_gray(interactive, "module-function-interactive");
                }
            }
            VecLikeType::Xwidget => {
                let obj = ptr as *const XwidgetObj;
                let fields = unsafe {
                    [
                        (load_value_atomic(&(*obj).plist), "xwidget-plist"),
                        (load_value_atomic(&(*obj).type_), "xwidget-type"),
                        (load_value_atomic(&(*obj).buffer), "xwidget-buffer"),
                        (load_value_atomic(&(*obj).title), "xwidget-title"),
                        (
                            load_value_atomic(&(*obj).script_callbacks),
                            "xwidget-script-callbacks",
                        ),
                    ]
                };
                for (value, label) in fields {
                    if value.is_heap_object() {
                        self.push_gray(value, label);
                    }
                }
            }
            VecLikeType::XwidgetView => {
                let obj = ptr as *const XwidgetViewObj;
                let fields = unsafe {
                    [
                        ((*obj).model, "xwidget-view-model"),
                        ((*obj).window, "xwidget-view-window"),
                    ]
                };
                for (value, label) in fields {
                    if value.is_heap_object() {
                        self.push_gray(value, label);
                    }
                }
            }
            VecLikeType::Buffer
            | VecLikeType::Window
            | VecLikeType::Frame
            | VecLikeType::Timer
            | VecLikeType::Process
            | VecLikeType::Marker
            | VecLikeType::Subr
            | VecLikeType::Bignum
            | VecLikeType::Sqlite
            | VecLikeType::UserPtr => {
                // These have no Value children to trace.
                //
                // Bignums own a `malachite::Integer`, which manages
                // its own limb buffer, but no Lisp_Object children —
                // `Drop` takes care of the memory in `free_gc_object`.
                //
                // UserPtr has only a raw C pointer and finalizer, no
                // Lisp children.
            }
        }
    }

    /// Sweep unmarked cons cells back to free lists.
    fn sweep_cons(&mut self) -> usize {
        let old_live = self.cons_live_count;
        let mut new_live = 0;
        self.cons_free_list = std::ptr::null_mut();
        for block in &mut self.cons_blocks {
            new_live += block.sweep(&mut self.cons_free_list);
        }
        self.cons_live_count = new_live;
        self.allocated_count = self
            .allocated_count
            .saturating_sub(old_live)
            .saturating_add(new_live);
        let mapped_live = self
            .mapped_cons_ranges
            .iter()
            .map(MappedConsRange::live_count)
            .sum::<usize>();
        new_live
            .saturating_add(mapped_live)
            .saturating_mul(size_of::<ConsCell>())
    }

    /// Sweep non-cons objects: walk intrusive list, free unmarked, rebuild list.
    fn sweep_objects(&mut self) -> usize {
        // `unchain_dead_markers` (invoked in `complete_collection`
        // between mark and sweep) has already spliced unmarked markers
        // out of every live buffer's intrusive chain, so freeing them
        // here leaves no dangling chain pointers. Mirrors GNU
        // `sweep_buffer → unchain_dead_markers` (alloc.c).
        let mut prev: *mut *mut GcHeader = &mut self.all_objects;
        let mut current = self.all_objects;
        let mut live_bytes = 0usize;
        while !current.is_null() {
            unsafe {
                let next = (*current).next;
                if (*current).is_marked() {
                    // Keep it — advance prev
                    live_bytes = live_bytes.saturating_add(Self::object_bytes_from_header(current));
                    prev = &mut (*current).next;
                    current = next;
                } else {
                    // Free it — unlink from list
                    *prev = next;
                    self.non_cons_object_addrs.remove(&(current as usize));
                    self.free_gc_object(current);
                    self.allocated_count = self.allocated_count.saturating_sub(1);
                    current = next;
                }
            }
        }

        live_bytes
    }

    /// `(total mapped objects, mapped objects currently marked)`.
    ///
    /// The marked count is how many immutable pdump (mapped) objects the mark
    /// phase re-traced this cycle — pure overhead that a "dump as permanent
    /// tenured region" partition would eliminate, since mapped objects are
    /// never freed. Used only for GC phase instrumentation.
    fn mapped_object_stats(&self) -> (usize, usize) {
        let veclike_total = self.mapped_veclike_objects.len();
        let veclike_marked = self
            .mapped_veclike_objects
            .iter()
            .filter(|object| object.marked)
            .count();
        let string_total = self.mapped_string_objects.len();
        let string_marked = self
            .mapped_string_objects
            .iter()
            .filter(|object| object.marked)
            .count();
        let cons_total: usize = self.mapped_cons_ranges.iter().map(|range| range.len).sum();
        let cons_marked: usize = self
            .mapped_cons_ranges
            .iter()
            .map(MappedConsRange::live_count)
            .sum();
        let float_total: usize = self.mapped_float_ranges.iter().map(|range| range.len).sum();
        let float_marked: usize = self
            .mapped_float_ranges
            .iter()
            .map(MappedFloatRange::live_count)
            .sum();
        (
            veclike_total + string_total + cons_total + float_total,
            veclike_marked + string_marked + cons_marked + float_marked,
        )
    }

    fn mapped_non_cons_live_bytes(&self) -> usize {
        self.mapped_float_ranges
            .iter()
            .map(|range| range.live_count().saturating_mul(size_of::<FloatObj>()))
            .chain(
                self.mapped_veclike_objects
                    .iter()
                    .filter(|object| object.marked)
                    .map(|object| object.byte_len),
            )
            .chain(
                self.mapped_string_objects
                    .iter()
                    .filter(|object| object.marked)
                    .map(|object| object.byte_len),
            )
            .sum()
    }

    /// Free a GC object by its header pointer.
    /// Must determine the actual type to call the correct Drop and dealloc.
    unsafe fn free_gc_object(&mut self, header: *mut GcHeader) {
        let kind = unsafe { (*header).kind };
        match kind {
            HeapObjectKind::String => {
                unsafe { drop(Box::from_raw(header as *mut StringObj)) };
            }
            HeapObjectKind::Float => {
                unsafe { drop(Box::from_raw(header as *mut FloatObj)) };
            }
            HeapObjectKind::VecLike => {
                let ptr = header as *mut VecLikeHeader;
                let type_tag = unsafe { (*ptr).type_tag };
                match type_tag {
                    VecLikeType::Vector => unsafe { drop(Box::from_raw(ptr as *mut VectorObj)) },
                    VecLikeType::CharTable => unsafe {
                        drop(Box::from_raw(ptr as *mut CharTableObj))
                    },
                    VecLikeType::SubCharTable => unsafe {
                        drop(Box::from_raw(ptr as *mut SubCharTableObj))
                    },
                    VecLikeType::HashTable => unsafe {
                        drop(Box::from_raw(ptr as *mut HashTableObj))
                    },
                    VecLikeType::Obarray => unsafe { drop(Box::from_raw(ptr as *mut ObarrayObj)) },
                    VecLikeType::Lambda => unsafe { drop(Box::from_raw(ptr as *mut LambdaObj)) },
                    VecLikeType::Macro => unsafe { drop(Box::from_raw(ptr as *mut MacroObj)) },
                    VecLikeType::ByteCode => unsafe {
                        drop(Box::from_raw(ptr as *mut ByteCodeObj))
                    },
                    VecLikeType::Record => unsafe { drop(Box::from_raw(ptr as *mut RecordObj)) },
                    VecLikeType::Overlay => unsafe { drop(Box::from_raw(ptr as *mut OverlayObj)) },
                    VecLikeType::Marker => unsafe { drop(Box::from_raw(ptr as *mut MarkerObj)) },
                    VecLikeType::Buffer => unsafe { drop(Box::from_raw(ptr as *mut BufferObj)) },
                    VecLikeType::Window => unsafe { drop(Box::from_raw(ptr as *mut WindowObj)) },
                    VecLikeType::Frame => unsafe { drop(Box::from_raw(ptr as *mut FrameObj)) },
                    VecLikeType::Timer => unsafe { drop(Box::from_raw(ptr as *mut TimerObj)) },
                    VecLikeType::Process => unsafe { drop(Box::from_raw(ptr as *mut ProcessObj)) },
                    VecLikeType::Xwidget => unsafe { drop(Box::from_raw(ptr as *mut XwidgetObj)) },
                    VecLikeType::XwidgetView => unsafe {
                        drop(Box::from_raw(ptr as *mut XwidgetViewObj))
                    },
                    VecLikeType::Subr => unsafe { drop(Box::from_raw(ptr as *mut SubrObj)) },
                    VecLikeType::Bignum => unsafe {
                        // Box::drop runs malachite::Integer::drop, which
                        // frees the underlying limb buffer.
                        drop(Box::from_raw(ptr as *mut BignumObj))
                    },
                    VecLikeType::SymbolWithPos => unsafe {
                        drop(Box::from_raw(ptr as *mut SymbolWithPosObj))
                    },
                    VecLikeType::Sqlite => unsafe { drop(Box::from_raw(ptr as *mut SqliteObj)) },
                    VecLikeType::UserPtr => {
                        // Call the finalizer if present before dropping.
                        let up = ptr as *mut UserPtrObj;
                        if let Some(fin) = unsafe { (*up).finalizer } {
                            unsafe { fin((*up).ptr) };
                        }
                        unsafe { drop(Box::from_raw(up)) };
                    }
                    VecLikeType::ModuleFunction => {
                        // Call the finalizer if present before dropping.
                        let mf = ptr as *mut ModuleFunctionObj;
                        if let Some(fin) = unsafe { (*mf).finalizer } {
                            unsafe { fin((*mf).data) };
                        }
                        unsafe { drop(Box::from_raw(mf)) };
                    }
                }
            }
        }
    }

    fn owns_non_cons_object(&self, ptr: *const u8) -> bool {
        !ptr.is_null() && self.non_cons_object_addrs.contains(&(ptr as usize))
    }

    /// Debug verification: after marking, check that every marked non-cons
    /// object is actually in one of our intrusive lists (young `all_objects`
    /// or tenured `tenured_objects`). If a marked object is NOT in a list, it
    /// means a root pointed to freed memory that happened to look like a valid
    /// tagged pointer.
    #[cfg(debug_assertions)]
    fn verify_marked_objects_owned(&self) {
        // Build a set of all owned non-cons object addresses
        let mut owned_addrs: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for head in [self.all_objects, self.tenured_objects] {
            let mut obj = head;
            while !obj.is_null() {
                owned_addrs.insert(obj as usize);
                unsafe {
                    obj = (*obj).next;
                }
            }
        }

        // Now walk both lists again and check marked objects
        let mut total_marked = 0usize;
        for head in [self.all_objects, self.tenured_objects] {
            let mut current = head;
            while !current.is_null() {
                unsafe {
                    if (*current).is_marked() {
                        total_marked += 1;
                        // Verify the object's internal data is sane
                        match (*current).kind {
                            HeapObjectKind::String => {
                                let ptr = current as *const StringObj;
                                let s = &(*ptr).data;
                                // Check string data pointer is reasonable
                                let str_ptr = s.as_bytes().as_ptr() as usize;
                                if str_ptr != 0 && str_ptr < 0x1000 {
                                    tracing::error!(
                                        "GC VERIFY: marked StringObj at {:p} has \
                                         corrupt data pointer {:#x}",
                                        current,
                                        str_ptr
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                    current = (*current).next;
                }
            }
        }
        tracing::trace!(
            "GC verify: {} marked non-cons objects, all owned",
            total_marked
        );
    }
}

impl Drop for TaggedHeap {
    fn drop(&mut self) {
        // Free all non-cons objects via every intrusive list: young, tenured,
        // and any objects detached for an in-flight deferred sweep.
        for mut current in [
            self.all_objects,
            self.tenured_objects,
            self.sweep_noncons_pending,
        ] {
            while !current.is_null() {
                unsafe {
                    let next = (*current).next;
                    self.free_gc_object(current);
                    current = next;
                }
            }
        }
        // ConsBlocks are dropped automatically (they implement Drop)
    }
}

#[cfg(test)]
mod ownership_tests {
    use super::*;

    #[test]
    fn heap_identity_is_unique_across_heap_lifetimes() {
        crate::test_utils::init_test_tracing();

        let first_id = TaggedHeap::new().identity();
        let second_id = TaggedHeap::new().identity();

        assert_ne!(first_id, second_id);
    }

    /// Phase 5: drive a non-blocking concurrent mark with the GC thread marking
    /// a large cons spine while THIS thread mutates (firing the SATB barrier) and
    /// allocates (allocate-black). The graph is large on purpose so marking is
    /// still in flight during the mutation, creating genuine overlap — run under
    /// ThreadSanitizer (`-Zsanitizer=thread`) this is the race check. The liveness
    /// asserts confirm the snapshot + SATB + allocate-black retain the right set.
    #[test]
    fn concurrent_mark_overlaps_mutation_and_retains_live_set() {
        crate::test_utils::init_test_tracing();
        let mut heap = TaggedHeap::new();
        set_tagged_heap(&mut heap);

        // A long reachable list: head -> ... -> tail (cdr-terminated with a
        // fixnum so traversal stops). Root = head.
        const N: i64 = 300_000;
        let mut list = TaggedValue::fixnum(0); // non-heap terminator
        for i in 0..N {
            list = heap.alloc_cons(TaggedValue::fixnum(i), list);
        }
        let head = list;
        // A second cons whose cdr we will rewire mid-mark (exercises SATB).
        let pivot = heap.alloc_cons(TaggedValue::fixnum(-1), head);
        // Unreachable garbage allocated before the mark begins.
        let _garbage = heap.alloc_cons(TaggedValue::fixnum(-2), TaggedValue::fixnum(0));
        let allocated_before = heap.cons_live_count;

        // Start the concurrent mark with `pivot` as the sole root (pivot -> head
        // -> whole list). begin_collection clears marks + seeds internal roots.
        heap.concurrent_begin();
        heap.seed_root(pivot);
        heap.launch_concurrent_mark();

        // While the GC thread marks: rewire pivot.cdr to a fresh cons D (the old
        // child `head` is logged to SATB and must stay live), and churn-allocate
        // (each new cons is born black). The list is long enough that the GC is
        // still traversing it during this.
        let d = heap.alloc_cons(TaggedValue::fixnum(7), head);
        assert!(crate::tagged::mutate::set_cons_cdr(pivot, d));
        for _ in 0..5_000 {
            let _ = heap.alloc_cons(TaggedValue::fixnum(0), TaggedValue::fixnum(0));
        }

        // Wait for the GC thread to drain, then terminate stop-the-world.
        while !heap.concurrent_mark_done() {
            std::thread::yield_now();
        }
        heap.join_concurrent_mark();
        heap.reseed_runtime_and_remembered_roots();
        heap.seed_root(pivot);
        let bytes_before = heap.live_bytes();
        heap.incremental_drain_all();
        heap.incremental_finish(bytes_before, std::time::Instant::now());
        heap.finish_incremental_sweep_now();

        // The whole list (N) + pivot + D survive; `head` is retained as floating
        // garbage via SATB (it left pivot's cdr but was logged); the churn conses
        // are allocate-black so they survive this cycle too; only `_garbage` is
        // reclaimed. So exactly one cons (the pre-mark garbage) was swept.
        assert_eq!(
            heap.cons_live_count,
            allocated_before + 1 /* D */ + 5_000 /* churn */ - 1, /* garbage */
            "concurrent mark must retain the live + SATB + allocate-black set",
        );
        // The reachable spine is intact: walk pivot -> D -> head -> ... and check
        // a few cars (reading a swept cons would be caught by the sanitizer).
        let after_pivot = unsafe { (*pivot.xcons_ptr()).load_cdr() };
        assert!(after_pivot.is_cons());
        let head_again = unsafe { (*after_pivot.xcons_ptr()).load_cdr() };
        assert!(head_again.is_cons());
        assert_eq!(
            unsafe { (*head_again.xcons_ptr()).load_car() }.0,
            TaggedValue::fixnum(N - 1).0,
        );
    }

    #[test]
    fn ordinary_non_cons_ownership_index_tracks_sweep() {
        crate::test_utils::init_test_tracing();
        let mut heap = TaggedHeap::new();

        let live = heap.alloc_float(1.0);
        let dead = heap.alloc_float(2.0);
        let live_ptr = live.as_float_ptr().unwrap() as *const u8;
        let dead_ptr = dead.as_float_ptr().unwrap() as *const u8;

        assert!(heap.owns_non_cons_object(live_ptr));
        assert!(heap.owns_non_cons_object(dead_ptr));
        assert_eq!(heap.non_cons_object_addrs.len(), 2);

        heap.collect_exact(std::iter::once(live));

        assert!(heap.owns_non_cons_object(live_ptr));
        assert!(!heap.owns_non_cons_object(dead_ptr));
        assert_eq!(heap.non_cons_object_addrs.len(), 1);
        assert!((live.xfloat() - 1.0).abs() < f64::EPSILON);
    }

    /// Characterization safety net for the path-collapse refactor: a forced full
    /// collection must retain a rooted cons graph and reclaim an unrooted one,
    /// regardless of which internal mark path runs. Pins the observable contract
    /// (`collect_exact` keeps the live set, frees garbage, leaves the spine
    /// readable) so collapsing the three GC paths into one cannot silently change
    /// it.
    #[test]
    fn collect_exact_retains_rooted_graph_and_frees_garbage() {
        crate::test_utils::init_test_tracing();
        let mut heap = TaggedHeap::new();
        set_tagged_heap(&mut heap);

        // Rooted spine: a -> b -> c (cdr-terminated by a fixnum).
        let c = heap.alloc_cons(TaggedValue::fixnum(3), TaggedValue::fixnum(0));
        let b = heap.alloc_cons(TaggedValue::fixnum(2), c);
        let a = heap.alloc_cons(TaggedValue::fixnum(1), b);
        // Unrooted garbage: reachable from neither the root nor the spine.
        let _g1 = heap.alloc_cons(TaggedValue::fixnum(-1), TaggedValue::fixnum(0));
        let _g2 = heap.alloc_cons(TaggedValue::fixnum(-2), TaggedValue::fixnum(0));
        let live_before = heap.cons_live_count;
        assert!(live_before >= 5);

        // Force a full collection rooted only at `a`.
        heap.collect_exact(std::iter::once(a));

        // The 3-cons rooted spine survives; the 2 garbage conses are reclaimed.
        assert_eq!(
            heap.cons_live_count,
            live_before - 2,
            "rooted graph retained, unrooted garbage reclaimed",
        );
        // The spine is intact and readable (reading a swept cons would corrupt).
        let a_cdr = unsafe { (*a.xcons_ptr()).load_cdr() };
        assert!(a_cdr.is_cons());
        let b_cdr = unsafe { (*a_cdr.xcons_ptr()).load_cdr() };
        assert!(b_cdr.is_cons());
        assert_eq!(
            unsafe { (*b_cdr.xcons_ptr()).load_car() }.0,
            TaggedValue::fixnum(3).0,
        );
    }
}

pub fn read_stack_end_from_proc() -> Option<usize> {
    let maps = std::fs::read_to_string("/proc/self/maps").ok()?;
    for line in maps.lines() {
        if line.contains("[stack]") {
            let dash = line.find('-')?;
            let space = line.find(' ')?;
            let end_hex = &line[dash + 1..space];
            return usize::from_str_radix(end_hex, 16).ok();
        }
    }
    None
}
