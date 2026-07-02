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
    /// Stage 1b CONCURRENT OBARRAY SCAN: a start-captured snapshot of the obarray's
    /// chunked symbol store. When `Some`, the GC thread scans these symbol cells
    /// ONCE per cycle, feeding each symbol's heap children into `gray` (conses) /
    /// `deferred` (non-cons) like the gray-drain cons branch. Always `Some` for a
    /// concurrent mark — the start handshake captures it.
    obarray: Option<crate::emacs_core::symbol::ObarrayScanSnapshot>,
    /// Stage 2 Tier B CONCURRENT VECTOR SCAN: a start-captured snapshot of every
    /// OWNED/Mapped vector backing (base ptr + len). When `Some`, the GC thread
    /// traces these backings ONCE per cycle, feeding each slot's heap children into
    /// `gray` (conses) / `deferred` (non-cons) like the gray-drain cons branch, so
    /// vectors are marked concurrently instead of deferred to the STW termination.
    /// Always `Some` for a concurrent mark — the start handshake captures it.
    vectors: Option<crate::tagged::header::VectorScanSnapshot>,
    /// CONCURRENT STRING MARKING: count of owned interval-free strings this
    /// cycle's GC thread claimed via `concurrent_try_mark_string` (one per
    /// successful `mark_claim`, Relaxed — single writer). Read by
    /// `join_concurrent_mark` (after the exit handshake's happens-before) into
    /// the cycle stats; sizes how much string work left the STW drain.
    str_claimed: std::sync::Arc<AtomicUsize>,
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

/// CONCURRENT STRING MARKING: try to mark one discovered string on the GC
/// thread. Returns `true` when fully handled here (claimed now, or already
/// marked); `false` means the caller must park the value in `deferred` for the
/// STW termination exactly as before. Called at all three discovery sinks
/// (gray drain, obarray scan, vector-backing scan).
///
/// OWNERSHIP — the same dump-span test the cons drain uses: every mapped
/// (pdump) string is registered via `register_mapped_string_object`, whose
/// only caller (pdump convert) passes `size_of::<StringObj>()` (never 0), so
/// registration ALWAYS `extend_dump_span`s over the object — no
/// "mapped-non-dump string" exists (verified; unlike conses, which have
/// non-dump mapped ranges). `alloc_string` is the only other producer of
/// TAG_STRING values, so outside the span ⇒ owned by this heap. Inside the
/// span ⇒ possibly mapped ⇒ DEFER unchanged: mapped strings are marked via a
/// SEPARATE plain-bool (`MappedStringObject::marked`) that sweep/verify
/// consult — claiming their `GcHeader` bit here would let the termination's
/// `mark_value` skip the mapped mark and the interval trace, a use-after-free
/// of their interval children. (An OWNED string that happens to sit inside
/// the span is merely deferred — safe, identical to today's path.)
///
/// INTERVALS — the hard boundary: the GC thread reads ONLY the interval
/// pointer WORD (`intervals_ptr`), NEVER the table behind it. The mutator can
/// free the table at any instant via `clear_intervals`, so calling
/// `intervals()` / `is_empty()` / `for_each_root()` here is a use-after-free.
/// Any future "trace small interval trees concurrently" extension needs a
/// retire/snapshot scheme like the Tier B vector backings — do not shortcut.
///
/// The null-check runs BEFORE the claim: claiming an interval-BEARING string
/// and then deferring it would make the termination's `mark_value` see the
/// mark bit and return without tracing the intervals. Staleness is safe both
/// ways: a stale non-null word only defers spuriously; a "stale null" can
/// only follow a real `clear_intervals`, whose SATB barrier (enforced inside
/// the `LispString` mutators) logged the dropped children first. A table
/// installed AFTER we claim is also safe: its children are values the mutator
/// obtained from snapshot-reachable places or allocated black — live this
/// cycle under SATB's invariant — and the next cycle re-traces fresh marks.
#[inline]
fn concurrent_try_mark_string(
    val: TaggedValue,
    dump_lo: usize,
    dump_hi: usize,
    str_claimed: &AtomicUsize,
) -> bool {
    debug_assert!(val.is_string());
    let Some(ptr) = val.as_string_ptr() else {
        return false; // malformed value — let the termination's mark_value decide
    };
    let addr = ptr as usize;
    if addr >= dump_lo && addr < dump_hi {
        return false; // inside the dump span: possibly mapped — defer (unchanged path)
    }
    // Owned string. Read the interval pointer WORD only (see doc above).
    if !unsafe { (*ptr).data.intervals_ptr() }.is_null() {
        return false; // interval-bearing: defer so mark_value traces the children
    }
    // Interval-free owned string: zero Lisp children, so claiming the mark bit
    // IS the complete trace. A failed claim means someone already marked it
    // (allocate-black, or an earlier edge this cycle) — equally done.
    if unsafe { (*ptr).header.mark_claim() } {
        str_claimed.fetch_add(1, Ordering::Relaxed);
    }
    true
}

/// The background concurrent-mark loop (Phase 5). Runs on the "neovm-gc" thread
/// with no `&mut TaggedHeap`: it marks conses via atomic block-bitmap ops +
/// atomic car/cdr loads, claims owned INTERVAL-FREE strings via their atomic
/// header mark bit (`concurrent_try_mark_string` — mark-only, zero children),
/// and defers every other non-cons (and non-owned conses) to the mutator's
/// stop-the-world termination. Loops draining its local gray queue and the
/// shared SATB buffer until both are empty and the mutator asks it to stop.
fn run_concurrent_mark(mut job: ConcurrentMarkJob) {
    use std::sync::atomic::Ordering;
    // Stage 1b CONCURRENT OBARRAY SCAN: when an obarray snapshot was handed over,
    // scan the symbol cells ONCE per cycle. Guarded by this local so it runs a
    // single time regardless of how many gray/SATB drain rounds happen. The scan
    // feeds children into `gray` (conses) / `deferred` (non-cons) exactly like the
    // cons-drain branch below, then the outer loop re-drains to a fixpoint.
    let mut obarray_scanned = false;
    // Stage 2 Tier B CONCURRENT VECTOR SCAN: same single-execution guard for the
    // vector-backing scan below.
    let mut vectors_scanned = false;
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
                // Strings first: an owned, interval-free string is claimed right
                // here (mark-only — it has zero Lisp children). Everything else
                // — floats/veclikes whose backing the mutator may reallocate,
                // and interval-bearing or mapped strings, which need the
                // mutator's `mark_value` — is deferred to the STW termination.
                if val.is_string()
                    && concurrent_try_mark_string(val, job.dump_lo, job.dump_hi, &job.str_claimed)
                {
                    continue;
                }
                job.deferred.lock().unwrap().push(val);
            }
        }
        // Stage 1b CONCURRENT OBARRAY SCAN (once per cycle): after the initial
        // gray-drain, scan the snapshotted symbol cells, routing each heap child to
        // gray (conses) / deferred (non-cons) just like the cons-drain branch. We
        // move the snapshot out of `job` (`take`) so the scan closure can borrow the
        // other `job` fields (`gray`, `deferred`) without a borrow conflict; once
        // scanned it stays `None`, so the `obarray_scanned` guard + the empty
        // `job.obarray` both ensure single execution. Pushing into `gray` means the
        // outer loop re-drains the symbol cells' transitive children to a fixpoint.
        if !obarray_scanned {
            obarray_scanned = true;
            if let Some(snap) = job.obarray.take() {
                // Safety: `snap` was captured at this cycle's world-stopped start
                // handshake; its chunk + seq pointers address the live, non-moving
                // obarray storage, and we are on the GC thread.
                unsafe {
                    snap.scan(|child| {
                        if child.is_cons() {
                            job.gray.push(child);
                        } else if !(child.is_string()
                            && concurrent_try_mark_string(
                                child,
                                job.dump_lo,
                                job.dump_hi,
                                &job.str_claimed,
                            ))
                        {
                            job.deferred.lock().unwrap().push(child);
                        }
                    });
                }
                // New children were pushed; loop back to drain them before deciding
                // we are done.
                continue;
            }
        }
        // Stage 2 Tier B CONCURRENT VECTOR SCAN (once per cycle): after the obarray
        // scan, trace the snapshotted vector backings, routing each heap child to gray
        // (conses) / deferred (non-cons) just like the cons-drain branch. We move the
        // snapshot out of `job` (`take`) so the scan closure can borrow the other
        // `job` fields without a borrow conflict; once scanned it stays `None`, so the
        // `vectors_scanned` guard + the empty `job.vectors` both ensure single
        // execution. Pushing into `gray` means the outer loop re-drains the backings'
        // transitive children to a fixpoint. Mirrors the obarray block above.
        if !vectors_scanned {
            vectors_scanned = true;
            if let Some(snap) = job.vectors.take() {
                // Safety: `snap` was captured at this cycle's world-stopped start
                // handshake; each entry's base/len addresses a live, immutable backing
                // (Mapped dump or retired-on-write Owned buffer), and we are on the GC
                // thread.
                unsafe {
                    snap.scan(|child| {
                        if child.is_cons() {
                            job.gray.push(child);
                        } else if !(child.is_string()
                            && concurrent_try_mark_string(
                                child,
                                job.dump_lo,
                                job.dump_hi,
                                &job.str_claimed,
                            ))
                        {
                            job.deferred.lock().unwrap().push(child);
                        }
                    });
                }
                // New children were pushed; loop back to drain them before deciding
                // we are done.
                continue;
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

/// SATB deletion barrier for ROOT-slot overwrites — specifically a symbol's
/// value / function / plist cell. A symbol `TaggedValue` is a `SymId`, not a heap
/// pointer, so symbol-cell writes are ROOT writes that bypass `note_heap_write`
/// (which gates on `owner.is_heap_object()`). Without logging them, the
/// concurrent mark must re-scan the whole obarray at termination to catch any
/// object that became reachable only through a symbol cell.
///
/// Call with the OLD value of the cell BEFORE the store (Yuasa snapshot-at-the-
/// beginning: the value being deleted from the root must be retained for this
/// cycle). No-ops outside a concurrent mark — a single thread-local load + branch,
/// no heap touch — and for non-heap pre-images (fixnum / UNBOUND / nil /
/// symbol-id), so cold-path callers pay essentially nothing when GC is idle.
#[inline]
pub(crate) fn note_root_overwrite(pre_image: TaggedValue) {
    if !pre_image.is_heap_object() {
        return;
    }
    if !TAGGED_HEAP_CONCURRENT_ACTIVE.with(|c| c.get()) {
        return;
    }
    with_tagged_heap(|heap| heap.note_root_overwrite_value(pre_image));
}

/// Whether a concurrent mark is active on this (mutator) thread — the gate the
/// Stage 1b symbol-cell seqlock uses to bracket value-cell ARM changes only
/// while the GC thread might be scanning the obarray. A thread-local load;
/// false (zero cost) off the concurrent path.
#[inline]
pub(crate) fn concurrent_mark_active() -> bool {
    TAGGED_HEAP_CONCURRENT_ACTIVE.with(|c| c.get())
}

/// SATB pre-image sink for STRING interval-table mutations, called from inside
/// the `LispString` interval mutators themselves (`ensure_intervals` /
/// `clear_intervals` in heap_types.rs) so the barrier is enforced at the only
/// mutation choke points — no call site, wrapper or raw, can drop a string's
/// interval children unlogged while the concurrent GC thread may have claimed
/// the string as interval-free. Logs the table's current child VALUES (not an
/// owner) to the shared SATB buffer, deduped once per string address per cycle
/// (`satb_string_preimage_addrs`, cleared at `begin_collection`): the first
/// pre-image is a superset of the start-of-cycle children — the same argument
/// as `push_value_children_to_satb_shared`'s owner dedup. The caller has
/// already checked `concurrent_mark_active()`.
pub(crate) fn note_string_interval_preimage(
    string_addr: usize,
    table: &crate::buffer::text_props::TextPropertyTable,
) {
    with_tagged_heap(|heap| {
        if !heap.satb_string_preimage_addrs.insert(string_addr) {
            return; // this string's full pre-image was already logged this cycle
        }
        let mut shared = heap.satb_shared.lock().unwrap();
        table.for_each_root(|value| {
            if value.is_heap_object() {
                shared.push(value);
            }
        });
    });
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

/// Per-kind breakdown of the values the GC thread parked in `deferred` for the
/// STW termination drain, taken as `join_concurrent_mark` folds the buffer into
/// gray. Sizes the concurrent-tracing extension: which kind a further
/// concurrent tier should take on first (strings are mark-only + intervals;
/// records/closures need atomic slots + snapshot/clone-on-write; weak/growable
/// hash tables stay deferred regardless). Counts are ENTRIES, not unique
/// objects — the GC thread parks a value once per discovered edge, and the
/// termination's `mark_value` dedups. Diagnostics only.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DrainKinds {
    pub string: usize,
    /// Vectors trace concurrently (Stage 2 Tier B scans their BACKINGS), but
    /// the vector VALUE is still parked so the termination sets its header
    /// mark — so this bucket counts header-mark-only work, not child traces.
    pub vector: usize,
    pub record: usize,
    /// Lambda + Macro (interpreted closures).
    pub closure: usize,
    pub bytecode: usize,
    pub hash_table: usize,
    /// CharTable + SubCharTable.
    pub char_table: usize,
    pub float: usize,
    /// Non-owned conses (new-block or mapped) the GC thread could not mark via
    /// its start-of-cycle block snapshot.
    pub cons: usize,
    /// Built-in functions — a large near-constant population (~1.7k registered
    /// at startup), split out so it does not mask the true `other` residue.
    pub subr: usize,
    /// Every remaining veclike (marker/buffer/overlay/bignum/...).
    pub other: usize,
}

impl DrainKinds {
    /// Classify one parked value into its bucket — the same tag dispatch
    /// `mark_value` uses (cons/string/float, then the veclike `type_tag`).
    ///
    /// # Safety
    /// `val` must be a live heap value: `join_concurrent_mark` runs before the
    /// termination drain and sweep, and nothing is freed during a concurrent
    /// mark (allocate-black; sweeps never overlap marking), so every parked
    /// entry's header is still valid.
    unsafe fn note(&mut self, val: TaggedValue) {
        if val.is_cons() {
            self.cons += 1;
        } else if val.is_string() {
            self.string += 1;
        } else if val.is_float() {
            self.float += 1;
        } else if val.is_veclike() {
            let ptr = val.as_veclike_ptr().unwrap();
            match unsafe { (*ptr).type_tag } {
                VecLikeType::Vector => self.vector += 1,
                VecLikeType::Record => self.record += 1,
                VecLikeType::Lambda | VecLikeType::Macro => self.closure += 1,
                VecLikeType::ByteCode => self.bytecode += 1,
                VecLikeType::HashTable => self.hash_table += 1,
                VecLikeType::CharTable | VecLikeType::SubCharTable => self.char_table += 1,
                VecLikeType::Subr => self.subr += 1,
                _ => self.other += 1,
            }
        } else {
            self.other += 1; // unreachable: only heap objects are parked
        }
    }

    /// Fold `cycle`'s per-kind counts into this lifetime per-kind maximum.
    fn merge_max(&mut self, cycle: &DrainKinds) {
        self.string = self.string.max(cycle.string);
        self.vector = self.vector.max(cycle.vector);
        self.record = self.record.max(cycle.record);
        self.closure = self.closure.max(cycle.closure);
        self.bytecode = self.bytecode.max(cycle.bytecode);
        self.hash_table = self.hash_table.max(cycle.hash_table);
        self.char_table = self.char_table.max(cycle.char_table);
        self.float = self.float.max(cycle.float);
        self.cons = self.cons.max(cycle.cons);
        self.subr = self.subr.max(cycle.subr);
        self.other = self.other.max(cycle.other);
    }

    /// Sum of all buckets — equals the deferred-entry count it was built from.
    pub fn total(&self) -> usize {
        self.string
            + self.vector
            + self.record
            + self.closure
            + self.bytecode
            + self.hash_table
            + self.char_table
            + self.float
            + self.cons
            + self.subr
            + self.other
    }
}

impl std::fmt::Display for DrainKinds {
    /// Compact trace-line segment: `str=N vec=N rec=N clo=N bc=N ht=N ct=N f=N
    /// cons=N sub=N other=N`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "str={} vec={} rec={} clo={} bc={} ht={} ct={} f={} cons={} sub={} other={}",
            self.string,
            self.vector,
            self.record,
            self.closure,
            self.bytecode,
            self.hash_table,
            self.char_table,
            self.float,
            self.cons,
            self.subr,
            self.other,
        )
    }
}

/// Snapshot of the deferred-sweep cost accounting plus the concurrent-mark
/// termination drain probe. Diagnostics only: per-cycle fields hold the most
/// recently completed (or in-flight) deferred sweep; lifetime fields aggregate
/// across the heap's life, with the eager STW sweep feeding `lifetime_sweep_us`
/// too so the two sweep paths are comparable.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SweepStats {
    pub sweep_us: u64,
    pub slice_count: usize,
    pub cons_blocks_swept: usize,
    pub noncons_freed: usize,
    pub lifetime_sweep_us: u64,
    pub lifetime_slices: usize,
    pub lifetime_cons_blocks_swept: usize,
    pub lifetime_noncons_freed: usize,
    /// Values `join_concurrent_mark` folded into the termination gray queue:
    /// the GC thread's parked non-cons buffer and the residual SATB log.
    pub last_termination_deferred: usize,
    pub max_termination_deferred: usize,
    pub last_termination_satb: usize,
    /// Per-kind breakdown of `last_termination_deferred`, plus the lifetime
    /// per-kind maximum (each bucket's own max across cycles). Populated in
    /// crate tests and under `NEOVM_GC_TRACE=1`; zero otherwise — the
    /// classification's header reads are not free STW time.
    pub last_termination_kinds: DrainKinds,
    pub max_termination_kinds: DrainKinds,
    /// CONCURRENT STRING MARKING: owned interval-free strings the GC thread
    /// claimed concurrently last cycle — string marks that LEFT the STW drain
    /// (the `kinds.string` bucket keeps counting the strings still parked:
    /// interval-bearing + mapped/dump-span ones). Always populated.
    pub last_concurrent_str_claimed: usize,
    /// Cost of the `join_concurrent_mark` fold itself (taking the SATB +
    /// deferred buffers, classifying, pushing to gray) — the cheap half of the
    /// termination; the mark fixpoint that follows is the trace line's `drain`.
    pub last_termination_fold_us: u64,
    /// Lifetime count of concurrent-mark terminations (`join_concurrent_mark`
    /// calls), so a probe polling between eval chunks can detect a new cycle.
    pub termination_count: usize,
    /// Mark cost of the most recent cycle at `incremental_finish`. For a
    /// concurrent cycle this is exactly the STW termination drain: the counter
    /// resets at `concurrent_begin` and the termination's
    /// `incremental_drain_all` is the only accumulation.
    pub mark_us: u64,
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
    /// Weak hash tables that have become PERMANENT (tenured old generation or
    /// mapped pdump image). The main mark never re-runs `trace_veclike` on a
    /// permanent-black object, so such a table would otherwise never re-register
    /// itself for the weak sweep and its entries would be pinned forever (a
    /// weak-table leak: GNU re-sweeps every weak table on every GC). Populated
    /// at `promote_and_blacken` (tenuring) and at mapped-dump registration;
    /// seeded into `weak_hash_tables` at the start of every `mark_and_sweep_
    /// weak_tables` so permanent weak tables are swept against the CURRENT cycle's
    /// marks exactly like young ones. Permanent, so its pointers never dangle.
    permanent_weak_hash_tables: Vec<*mut HashTableObj>,
    /// Every live finalizer object, registered at allocation — the Rust-side
    /// equivalent of GNU's intrusive `finalizers` list (alloc.c). Scanned at
    /// mark termination by `mark_and_queue_doomed_finalizers`: unmarked
    /// entries leave the registry (the object is swept normally) and their
    /// `function` moves to `doomed_finalizer_functions`. Entries stay valid
    /// because every sweep that could free an unmarked finalizer is preceded
    /// by that scan, which removes it first.
    finalizer_registry: Vec<*mut FinalizerObj>,
    /// Functions of finalizer objects found unreachable, waiting to run —
    /// GNU's `doomed_finalizers` list (we queue only the function; the
    /// finalizer object itself is swept). Re-marked transitively when queued
    /// so the imminent sweep keeps them, and seeded as runtime roots every
    /// cycle so a batch that survives across cycles (e.g. queued during a
    /// finalizer run) stays live. Drained by the evaluator's cycle-completed
    /// block, which calls each with zero args, errors ignored.
    doomed_finalizer_functions: Vec<TaggedValue>,

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
    /// One-time flag: this heap has completed a full stop-the-world collection
    /// (its bootstrap cycle). A dump-less heap runs the concurrent collector
    /// from its second cycle on — the same one-STW-bootstrap-then-concurrent
    /// shape as the dump path; see `should_run_concurrent`.
    bootstrap_collected: bool,

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
    /// Per-cycle dedup for the COARSE (bulk) SATB barrier. A bulk mutator
    /// (`with_hash_table_mut`, `with_vector_data_mut`, char-table, …) hands a
    /// `&mut` to an arbitrary closure, so the barrier — which runs BEFORE the
    /// store and cannot know which slot the closure will touch — conservatively
    /// snapshots the owner's WHOLE pre-image. Doing that on every write is O(n)
    /// per write => O(n²) to build an n-element container (the `(ucs-names)` OOM).
    /// SATB only needs each owner's start-of-cycle child set logged ONCE: at the
    /// owner's FIRST mutation this cycle, all its snapshot-time children are still
    /// present (a child can only be unlinked by a mutation of this owner, which is
    /// itself this first write firing the barrier pre-store), so that single
    /// snapshot is a superset of every child reachable at snapshot time. Later
    /// writes can only overwrite values already logged (or born-black new ones),
    /// so re-snapshotting is pure waste. We record owners snapshotted this cycle
    /// here and skip the re-enumeration. Cleared at every mark start
    /// (`concurrent_begin`/`begin_collection`). Conses (2 children, O(1) barrier)
    /// bypass it; only multi-child veclike/string owners are deduped.
    satb_snapshotted_owners: FxHashSet<usize>,
    /// Veclikes/strings the GC thread reached but did NOT trace (their backing
    /// can be reallocated by the mutator, so reading it concurrently would be a
    /// UAF). They are marked black and parked here, then traced at the
    /// termination handshake while the mutator is stopped.
    deferred_veclikes: std::sync::Arc<std::sync::Mutex<Vec<TaggedValue>>>,
    /// GC thread sets this (Release) when gray + SATB are drained; the mutator
    /// polls it (Acquire) at safe points to decide when to terminate.
    gc_done: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// CONCURRENT STRING MARKING: shared claim counter for the in-flight cycle
    /// (see `ConcurrentMarkJob::str_claimed`). Reset at `launch_concurrent_mark`,
    /// folded into `last_concurrent_str_claimed` at `join_concurrent_mark`.
    concurrent_str_claimed: std::sync::Arc<AtomicUsize>,
    /// Strings the GC thread claimed concurrently in the last completed cycle
    /// (diagnostics; the concurrent counterpart of `last_termination_kinds.string`).
    last_concurrent_str_claimed: usize,
    /// CONCURRENT STRING MARKING: per-cycle dedup for the ENFORCED in-mutator
    /// string interval SATB barrier (`note_string_interval_preimage`), keyed by
    /// `LispString` address — stable for the whole cycle because nothing is
    /// freed while a mark runs. Cleared at `begin_collection`, like
    /// `satb_snapshotted_owners`.
    satb_string_preimage_addrs: FxHashSet<usize>,
    /// Mutator sets this (Release) to ask the GC thread to finish and exit.
    gc_stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Receives when the GC thread has exited its mark loop (so the mutator's
    /// termination can safely take over the gray queue). Set at start.
    gc_exited: Option<std::sync::mpsc::Receiver<()>>,
    /// Stage 1b CONCURRENT OBARRAY SCAN: a start-captured obarray chunk snapshot
    /// staged by the start handshake (`start_concurrent_mark`) just before
    /// `launch_concurrent_mark`, which moves it into the `ConcurrentMarkJob`. The
    /// heap cannot reach the Context-side obarray itself, so the snapshot is built
    /// Context-side and parked here for the launch to consume. `None` except
    /// between a start handshake and the launch consuming it.
    pending_obarray_scan: Option<crate::emacs_core::symbol::ObarrayScanSnapshot>,
    /// Stage 1b: the obarray slot count captured at the start handshake, retained
    /// across the cycle (the snapshot itself is moved into the GC job). At the STW
    /// termination, the residual re-seed covers new symbols in slots `>= this`
    /// (interned mid-cycle, never scanned by the GC thread). `None` outside a
    /// concurrent mark.
    concurrent_obarray_start_slots: Option<usize>,
    /// Stage 2 Tier B CONCURRENT VECTOR SCAN: retired vector backings — the ORIGINAL
    /// `Vec` of each OWNED vector whose backing was clone-on-write replaced during
    /// this concurrent mark (`with_vector_data_mut`). The GC thread's snapshot still
    /// points at these immutable buffers, so they must stay alive until the GC thread
    /// joins. Drained + dropped in `join_concurrent_mark` (the GC thread has provably
    /// exited — the only safe free point). Empty unless a clone-on-write fired.
    retired_vector_buffers: Vec<Vec<TaggedValue>>,
    /// Stage 2 Tier B CONCURRENT VECTOR SCAN: per-cycle clone-on-write dedup set,
    /// keyed on each vector owner's `TaggedValue` bits. On an owner's FIRST bulk
    /// mutation this cycle we clone+retire its OWNED backing once; later mutations of
    /// the same owner skip the clone (they touch the already-cloned live backing the
    /// GC's snapshot does NOT point at). Cleared at every mark start
    /// (`concurrent_begin`/`begin_collection`). Empty unless a clone-on-write fired.
    concurrent_cloned_vectors: FxHashSet<usize>,

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
    /// Per-cycle deferred-sweep cost accumulators (reset when the sweep is
    /// armed at `incremental_finish`) + lifetime totals, and the
    /// concurrent-termination drain probe. Snapshot via `sweep_stats`.
    sweep_slice_us_total: u64,
    sweep_slice_count: usize,
    sweep_cons_blocks_swept: usize,
    sweep_noncons_freed: usize,
    sweep_lifetime_us: u64,
    sweep_lifetime_slices: usize,
    sweep_lifetime_cons_blocks_swept: usize,
    sweep_lifetime_noncons_freed: usize,
    last_termination_deferred: usize,
    max_termination_deferred: usize,
    last_termination_satb: usize,
    last_termination_kinds: DrainKinds,
    max_termination_kinds: DrainKinds,
    last_termination_fold_us: u64,
    termination_count: usize,
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
            permanent_weak_hash_tables: Vec::new(),
            finalizer_registry: Vec::new(),
            doomed_finalizer_functions: Vec::new(),
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
            bootstrap_collected: false,
            mapped_remembered: FxHashSet::default(),
            mark_in_progress: false,
            incremental_mark_us: 0,
            concurrent_mark_running: false,
            satb_shared: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            satb_snapshotted_owners: FxHashSet::default(),
            deferred_veclikes: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            gc_done: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            concurrent_str_claimed: std::sync::Arc::new(AtomicUsize::new(0)),
            last_concurrent_str_claimed: 0,
            satb_string_preimage_addrs: FxHashSet::default(),
            gc_stop: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            gc_exited: None,
            pending_obarray_scan: None,
            concurrent_obarray_start_slots: None,
            retired_vector_buffers: Vec::new(),
            concurrent_cloned_vectors: FxHashSet::default(),
            sweep_in_progress: false,
            sweep_cons_cursor: 0,
            sweep_noncons_pending: std::ptr::null_mut(),
            sweep_noncons_live_bytes: 0,
            sweep_mark_us: 0,
            sweep_bytes_before: 0,
            sweep_slice_us_total: 0,
            sweep_slice_count: 0,
            sweep_cons_blocks_swept: 0,
            sweep_noncons_freed: 0,
            sweep_lifetime_us: 0,
            sweep_lifetime_slices: 0,
            sweep_lifetime_cons_blocks_swept: 0,
            sweep_lifetime_noncons_freed: 0,
            last_termination_deferred: 0,
            max_termination_deferred: 0,
            last_termination_satb: 0,
            last_termination_kinds: DrainKinds::default(),
            max_termination_kinds: DrainKinds::default(),
            last_termination_fold_us: 0,
            termination_count: 0,
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

    /// Deferred-sweep cost + termination-drain instrumentation snapshot.
    pub(crate) fn sweep_stats(&self) -> SweepStats {
        SweepStats {
            sweep_us: self.sweep_slice_us_total,
            slice_count: self.sweep_slice_count,
            cons_blocks_swept: self.sweep_cons_blocks_swept,
            noncons_freed: self.sweep_noncons_freed,
            lifetime_sweep_us: self.sweep_lifetime_us,
            lifetime_slices: self.sweep_lifetime_slices,
            lifetime_cons_blocks_swept: self.sweep_lifetime_cons_blocks_swept,
            lifetime_noncons_freed: self.sweep_lifetime_noncons_freed,
            last_termination_deferred: self.last_termination_deferred,
            max_termination_deferred: self.max_termination_deferred,
            last_termination_satb: self.last_termination_satb,
            last_termination_kinds: self.last_termination_kinds,
            max_termination_kinds: self.max_termination_kinds,
            last_concurrent_str_claimed: self.last_concurrent_str_claimed,
            last_termination_fold_us: self.last_termination_fold_us,
            termination_count: self.termination_count,
            mark_us: self.sweep_mark_us,
        }
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

    /// True when a registered mapped span (a loaded pdump) has activated the
    /// dump-partitioned collector. Diagnostics: lets the drain-kind profiling
    /// probe verify which collector configuration it is measuring.
    #[cfg(test)]
    pub(crate) fn dump_partition_active(&self) -> bool {
        self.partition_dump
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
                        VecLikeType::Record | VecLikeType::WindowConfiguration => {
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
                        VecLikeType::Finalizer => size_of::<FinalizerObj>(),
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

    /// Allocate a window configuration. Structurally a record (`{header, data}`)
    /// but tagged `WindowConfiguration` so it is a distinct pseudovector type.
    pub fn alloc_window_configuration(&mut self, items: Vec<TaggedValue>) -> TaggedValue {
        self.add_memory_use_count(MemoryUseCountSlot::VectorCells, items.len() as u64);
        let obj = Box::new(RecordObj {
            header: VecLikeHeader::new(VecLikeType::WindowConfiguration),
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

    /// Allocate a finalizer object (GNU `Fmake_finalizer`). Registered in
    /// `finalizer_registry` so mark termination can detect when the object
    /// becomes unreachable and queue `function` to run after that cycle.
    /// GNU accepts any object as the function; callers do not validate it.
    pub fn alloc_finalizer(&mut self, function: TaggedValue) -> TaggedValue {
        let obj = Box::new(FinalizerObj {
            header: VecLikeHeader::new(VecLikeType::Finalizer),
            function,
        });
        let ptr = Box::into_raw(obj);
        self.link_veclike(ptr as *mut VecLikeHeader);
        self.finalizer_registry.push(ptr);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<FinalizerObj>());
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
        #[cfg(test)]
        alloc_probe::record(ptr, self.non_cons_object_addrs.len());
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
            #[cfg(test)]
            alloc_probe::record(gc_header, self.non_cons_object_addrs.len());
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
        // New mark cycle: the per-cycle SATB pre-image dedup set must start empty
        // so each owner's full pre-image is snapshotted once for THIS cycle's
        // start-of-cycle reachability (a carried-over entry would wrongly suppress
        // the snapshot of an owner whose children differ this cycle).
        self.satb_snapshotted_owners.clear();
        // CONCURRENT STRING MARKING: same per-cycle reset for the enforced
        // in-mutator string interval pre-image dedup (`note_string_interval_preimage`).
        self.satb_string_preimage_addrs.clear();
        // Stage 2 Tier B CONCURRENT VECTOR SCAN: the per-cycle clone-on-write dedup
        // set must start empty so each vector owner is cloned+retired at most once
        // per cycle (a carried-over entry would wrongly suppress this cycle's clone).
        self.concurrent_cloned_vectors.clear();
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
                // A weak hash table being tenured becomes permanent-black and the
                // main mark will never re-touch it; record it so the weak sweep
                // keeps re-evaluating its entries every GC (GNU sweeps every weak
                // table every GC). See `permanent_weak_hash_tables`.
                if (*obj).kind == HeapObjectKind::VecLike {
                    let vptr = obj as *mut VecLikeHeader;
                    if (*vptr).type_tag == VecLikeType::HashTable {
                        let ht_ptr = vptr as *mut HashTableObj;
                        if (*ht_ptr).table.weakness.is_some()
                            && !self.permanent_weak_hash_tables.contains(&ht_ptr)
                        {
                            self.permanent_weak_hash_tables.push(ht_ptr);
                        }
                    }
                }
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
        // Mapped (pdump) weak hash tables become permanent-black here too (the
        // preloaded image ships several, e.g. `print-number-table` helpers and
        // internal caches). Like tenured weak tables, they would never be
        // re-traced and their entries would be pinned forever; register them so
        // `mark_and_sweep_weak_tables` re-evaluates them every GC.
        let mapped_weak: Vec<*mut HashTableObj> = self
            .mapped_veclike_objects
            .iter()
            .filter_map(|object| {
                let header = object.header;
                // SAFETY: `header` is a live mapped veclike for the dump's lifetime.
                unsafe {
                    if (*header).type_tag == VecLikeType::HashTable {
                        let ht_ptr = header as *mut HashTableObj;
                        if (*ht_ptr).table.weakness.is_some() {
                            return Some(ht_ptr);
                        }
                    }
                }
                None
            })
            .collect();
        for ht_ptr in mapped_weak {
            if !self.permanent_weak_hash_tables.contains(&ht_ptr) {
                self.permanent_weak_hash_tables.push(ht_ptr);
            }
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
                // A dumped/tenured WEAK hash table is permanent-black, so the main
                // mark never re-runs `trace_veclike` on it and it would otherwise
                // never re-register for the weak sweep. Register it here (the
                // remembered-set / SATB / permanent scan is the ONLY path that
                // reaches such a table) and push only its NON-weak children
                // (custom test/hash closures) strongly. Its weak keys/values are
                // deliberately NOT traced here — `mark_and_sweep_weak_tables`
                // (which runs at every mark termination, before
                // `verify_dump_partition`) decides per-entry survival against the
                // current marks and physically removes the dead entries, so the
                // verifier never sees an unmarked weak child. This mirrors GNU's
                // `mark_object` PVEC_HASH_TABLE (alloc.c): weak tables register
                // themselves and do NOT mark their contents.
                if let Some(weak_children) = self.register_weak_hash_table_for_sweep(ptr) {
                    for child in weak_children {
                        if child.is_heap_object() {
                            self.push_gray(child, origin);
                        }
                    }
                } else {
                    // STRONG enumeration for every other veclike (and non-weak
                    // hash tables): the remembered-set / SATB paths and the
                    // dump-partition verifier require every heap child of a
                    // permanent owner to be marked, or it is swept while still
                    // referenced (UAF).
                    for child in self.collect_veclike_children(ptr as *mut VecLikeHeader) {
                        if child.is_heap_object() {
                            self.push_gray(child, origin);
                        }
                    }
                }
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

    /// If `ptr` is a WEAK hash table, register it for this cycle's weak sweep
    /// (deduplicated) and return its NON-weak children — the custom test/hash
    /// closures from `define-hash-table-test`, which must be traced strongly so
    /// they outlive the table. Returns `None` for non-weak tables and every
    /// other veclike, signalling the caller to fall back to the normal strong
    /// child enumeration.
    ///
    /// This is the bridge that lets a dumped/tenured weak table — which the main
    /// mark never re-touches because it is permanent-black — still be swept every
    /// full collection, matching GNU (whose non-generational mark re-encounters
    /// every live weak table every GC and rebuilds `weak_hash_tables`).
    fn register_weak_hash_table_for_sweep(
        &mut self,
        ptr: *const VecLikeHeader,
    ) -> Option<Vec<TaggedValue>> {
        let ht_ptr = ptr as *mut HashTableObj;
        // SAFETY: caller verified `ptr` is a live veclike; the heap is owned
        // exclusively during marking. Reading the immutable weakness / closure
        // fields is race-free.
        let (is_weak, user_cmp, user_hash) = unsafe {
            if (*ptr).type_tag != VecLikeType::HashTable {
                return None;
            }
            let ht = &(*ht_ptr).table;
            (
                ht.weakness.is_some(),
                ht.user_cmp_function,
                ht.user_hash_function,
            )
        };
        if !is_weak {
            return None;
        }
        if !self.weak_hash_tables.contains(&ht_ptr) {
            self.weak_hash_tables.push(ht_ptr);
        }
        let mut nonweak = Vec::new();
        if let Some(f) = user_cmp {
            nonweak.push(f);
        }
        if let Some(f) = user_hash {
            nonweak.push(f);
        }
        Some(nonweak)
    }

    /// Direct children of a mapped vectorlike object (read-only) for the verifier.
    fn collect_veclike_children(&self, ptr: *mut VecLikeHeader) -> Vec<TaggedValue> {
        let mut out = Vec::new();
        unsafe {
            match (*ptr).type_tag {
                VecLikeType::Vector | VecLikeType::Record | VecLikeType::WindowConfiguration => {
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
                    // Custom test/hash closures (`define-hash-table-test`) live
                    // ONLY in these fields and are traced by `trace_veclike`; keep
                    // the two enumerations in sync so the remembered/SATB strong-
                    // trace (which uses this) and the dump-partition verifier both
                    // cover them — otherwise a dumped/tenured custom-test table's
                    // closures are swept while the table still calls them (UAF).
                    if let Some(f) = ht.user_cmp_function {
                        out.push(f);
                    }
                    if let Some(f) = ht.user_hash_function {
                        out.push(f);
                    }
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
                VecLikeType::Finalizer => {
                    out.push((*(ptr as *const FinalizerObj)).function);
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
            // Doomed finalizer functions not yet run must survive any cycle
            // that starts before the evaluator drains them (e.g. one queued
            // during a finalizer run, or an explicit GC before the drain).
            .chain(
                self.doomed_finalizer_functions
                    .iter()
                    .map(|value| (*value, "doomed-finalizer-function")),
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
        // Queue doomed finalizers before the weak sweep (GNU
        // `queue_doomed_finalizers` runs before
        // `mark_and_sweep_weak_table_contents` in `garbage_collect`): their
        // functions are re-marked so both the weak sweep and the object sweep
        // see them as live.
        self.mark_and_queue_doomed_finalizers();
        // Resolve weak hash tables now that the main mark has drained. Both the
        // sync and concurrent paths converge here with the mutator stopped, so
        // this is single-threaded and path-agnostic.
        self.mark_and_sweep_weak_tables();
        let mark_us = mark_t0.elapsed().as_micros() as u64;

        self.finalize_collection(mark_us, bytes_before, t0);
    }

    /// Queue the functions of finalizer objects this cycle found unreachable —
    /// GNU `queue_doomed_finalizers` + `mark_finalizers` (alloc.c). Must run
    /// at BOTH mark terminations (`complete_collection` and
    /// `incremental_finish`), after the main mark drains and before the weak
    /// sweep. A doomed finalizer leaves the registry and is swept normally;
    /// only its `function` is queued, re-marked transitively (same marking
    /// helpers as the weak-table fixpoint) so the imminent sweep keeps
    /// everything it needs. Still-marked finalizers stay registered.
    fn mark_and_queue_doomed_finalizers(&mut self) {
        if self.finalizer_registry.is_empty() {
            return;
        }
        let registry = std::mem::take(&mut self.finalizer_registry);
        let mut doomed = Vec::new();
        for ptr in registry {
            // SAFETY: registered at allocation; every sweep that could free an
            // unmarked finalizer is preceded by this scan, which removes it
            // from the registry first, so `ptr` is live. The world is stopped
            // and marking has drained, so the mark bit is final.
            if unsafe { (*ptr).header.gc.is_marked() } {
                self.finalizer_registry.push(ptr);
            } else {
                doomed.push(unsafe { (*ptr).function });
            }
        }
        if doomed.is_empty() {
            return;
        }
        for function in doomed.iter().copied() {
            if function.is_heap_object() {
                self.push_gray(function, "doomed-finalizer-function");
            }
        }
        self.mark_all();
        self.doomed_finalizer_functions.extend(doomed);
    }

    /// Take every function queued by the doomed-finalizer scans so far. The
    /// evaluator's cycle-completed block calls each with zero args, errors
    /// ignored (GNU `run_finalizers`). Taking the whole batch means a
    /// finalizer created — and doomed — during a finalizer run lands in a
    /// later batch, run after a later cycle.
    pub fn take_doomed_finalizer_functions(&mut self) -> Vec<TaggedValue> {
        std::mem::take(&mut self.doomed_finalizer_functions)
    }

    /// Resolve the weak hash tables discovered during this cycle's mark — GNU
    /// `mark_and_sweep_weak_table_contents` (alloc.c) + `sweep_weak_table`
    /// (fns.c). Runs at the stop-the-world `complete_collection` after the main
    /// mark drains. First a fixpoint marks the key/value of every entry that
    /// survives per its table's weakness — iterate to stability because a value
    /// in one weak table may be a key in another — then non-surviving entries
    /// are removed.
    fn mark_and_sweep_weak_tables(&mut self) {
        // Seed every PERMANENT (tenured/mapped) weak table into this cycle's
        // worklist. The main mark skips permanent-black objects, so these would
        // otherwise never be swept again and their entries would be pinned
        // forever. GNU re-encounters and re-sweeps every live weak table on every
        // GC; this restores that for permanents. Young/runtime weak tables are
        // already registered by `trace_veclike` / `register_weak_hash_table_for_
        // sweep` during this cycle's mark.
        for &tptr in &self.permanent_weak_hash_tables {
            if !self.weak_hash_tables.contains(&tptr) {
                self.weak_hash_tables.push(tptr);
            }
        }

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
        // Eager STW sweep cost feeds the same lifetime total as the deferred
        // slices, so the two sweep paths are comparable.
        self.sweep_lifetime_us += sweep_us;
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

        // A full STW cycle has completed: the heap now has consistent live
        // accounting and an empty gray queue, the baseline the concurrent
        // collector starts from. Dump-less heaps run concurrent marking from
        // the next safe-point collection on (`should_run_concurrent`).
        self.bootstrap_collected = true;
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

    /// True if a concurrent mark should drive THIS collection.
    ///
    /// Dump heaps: a partitioned post-dump heap whose first partition cycle
    /// has promoted + blackened the image (the young/old split bounds what is
    /// traced); that first cycle falls to the STW full path.
    ///
    /// Dump-less heaps: after the first completed STW collection — the same
    /// one-STW-bootstrap-then-concurrent shape as the dump path. Nothing
    /// tenures without a dump, so every cycle re-clears and re-marks the whole
    /// young heap (correct, just unpartitioned), and the concurrent job's dump
    /// checks never match (`dump_addr_lo/hi` stay MAX/0) while the
    /// remembered-set seeding is skipped entirely (`partition_dump` is false).
    ///
    /// A heap that registers a dump AFTER dump-less cycles switches back to
    /// the dump rule: the first partition cycle must be the STW full trace
    /// that promotes + blackens the image, regardless of earlier bootstraps.
    pub fn should_run_concurrent(&self) -> bool {
        if self.partition_dump {
            self.dump_blackened
        } else {
            self.bootstrap_collected
        }
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
    /// Stage 1b: stash the start-captured obarray scan snapshot for the next
    /// `launch_concurrent_mark` to move into the job. Called from
    /// `start_concurrent_mark` at the world-stopped start handshake (once per
    /// concurrent mark).
    pub(crate) fn set_pending_obarray_scan(
        &mut self,
        snap: crate::emacs_core::symbol::ObarrayScanSnapshot,
    ) {
        // Retain the start slot count for the termination residual re-seed before
        // the snapshot is moved into the GC job at `launch_concurrent_mark`.
        self.concurrent_obarray_start_slots = Some(snap.n_slots());
        self.pending_obarray_scan = Some(snap);
    }

    /// Stage 1b: take the start-of-cycle obarray slot count (set at the start
    /// handshake) for the termination residual re-seed. `None` for a cycle with
    /// no concurrent mark (e.g. a stop-the-world full collection).
    pub(crate) fn take_concurrent_obarray_start_slots(&mut self) -> Option<usize> {
        self.concurrent_obarray_start_slots.take()
    }

    pub(crate) fn launch_concurrent_mark(&mut self) {
        // Immutable snapshot of owned cons-block bases — read-only on the GC
        // thread. New blocks allocated during marking are absent, which is fine:
        // their conses allocate-black and never enter the GC's gray queue.
        let mut owned = std::collections::HashSet::with_capacity(self.cons_blocks.len());
        for block in &self.cons_blocks {
            owned.insert(block.base_addr());
        }
        // Stage 2 Tier B CONCURRENT VECTOR SCAN: snapshot every
        // OWNED/Mapped vector backing AT THIS world-stopped point (same instant the
        // cons `owned_bases` snapshot is taken and the roots are seeded), so the GC
        // thread can trace vectors concurrently instead of deferring them to the STW
        // termination. Vectors are heap-side, so capture directly here (no eval.rs
        // seam, unlike the Context-side obarray). `non_cons_object_addrs` holds every
        // owned non-cons object's `GcHeader` addr; pick out the ones tagged
        // `VecLikeType::Vector`. Vectors allocated mid-cycle are absent from this set
        // capture and are covered by allocate-black.
        let vectors = {
            let mut snap = crate::tagged::header::VectorScanSnapshot::with_capacity(
                self.non_cons_object_addrs.len(),
            );
            for &addr in &self.non_cons_object_addrs {
                // Safety: `addr` is an owned non-cons object's live `GcHeader` addr; a
                // VecLike header begins with its `GcHeader`, so casting to
                // `*const VecLikeHeader` and reading `type_tag` is valid. Only when the
                // tag is `Vector` do we cast to `VectorObj` and read its backing.
                let header = addr as *const VecLikeHeader;
                let is_vector = unsafe {
                    (*(addr as *const GcHeader)).kind == HeapObjectKind::VecLike
                        && (*header).type_tag == VecLikeType::Vector
                };
                if is_vector {
                    let obj = unsafe { &*(header as *const VectorObj) };
                    snap.push(obj.data.scan_entry());
                }
            }
            Some(snap)
        };
        let gray = std::mem::take(&mut self.gray_queue);
        let (exited_tx, exited_rx) = std::sync::mpsc::channel();
        self.gc_done
            .store(false, std::sync::atomic::Ordering::Release);
        // Fresh per-cycle concurrent string-claim counter.
        self.concurrent_str_claimed.store(0, Ordering::Relaxed);
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
            // Stage 1b: consume the obarray snapshot the start handshake staged.
            // Take it so it is not left dangling for a later cycle.
            obarray: self.pending_obarray_scan.take(),
            // Stage 2 Tier B: the vector-backing snapshot captured just above.
            vectors,
            str_claimed: self.concurrent_str_claimed.clone(),
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
        // The fold is timed (`last_termination_fold_us`) so the termination's
        // cheap push half is attributable separately from the mark fixpoint.
        let fold_t0 = std::time::Instant::now();
        let satb = std::mem::take(&mut *self.satb_shared.lock().unwrap());
        self.last_termination_satb = satb.len();
        self.gray_queue.extend(satb);
        let deferred = std::mem::take(&mut *self.deferred_veclikes.lock().unwrap());
        self.last_termination_deferred = deferred.len();
        self.max_termination_deferred = self.max_termination_deferred.max(deferred.len());
        // Strings the GC thread claimed concurrently (they never reached
        // `deferred`); the exit handshake above (`rx.recv()`) established the
        // happens-before, so a Relaxed read sees the final count.
        self.last_concurrent_str_claimed = self.concurrent_str_claimed.load(Ordering::Relaxed);
        // Classify what the drain is about to trace, per kind — the measurement
        // that decides which kinds a concurrent-tracing extension should take
        // on. Pure counting (marking behavior is unchanged), but the header
        // reads cost real STW time on a large buffer (~20ns/entry), so outside
        // the crate's own tests it only runs when the trace that prints it is
        // on; the kind buckets stay zero otherwise.
        if cfg!(test) || std::env::var("NEOVM_GC_TRACE").as_deref() == Ok("1") {
            let mut kinds = DrainKinds::default();
            for &val in &deferred {
                // Safety: parked entries are live heap values; nothing has been
                // swept since they were parked (see `DrainKinds::note`).
                unsafe { kinds.note(val) };
            }
            self.last_termination_kinds = kinds;
            self.max_termination_kinds.merge_max(&kinds);
        }
        self.termination_count += 1;
        self.gray_queue.extend(deferred);
        self.last_termination_fold_us = fold_t0.elapsed().as_micros() as u64;
        // Stage 2 Tier B CONCURRENT VECTOR SCAN: the GC thread has provably exited its
        // mark loop (the `rx.recv()` above), so its snapshot pointers into the retired
        // vector backings are no longer in use — this is the ONLY safe free point.
        // Drain + drop the retired originals and clear the per-cycle clone-dedup set.
        // Both are empty unless a clone-on-write fired this cycle.
        let retired = std::mem::take(&mut self.retired_vector_buffers);
        drop(retired);
        self.concurrent_cloned_vectors.clear();
    }

    /// SATB barrier path for concurrent marking: append the owner's current
    /// (pre-overwrite) children to the shared buffer the GC thread drains. Reuses
    /// the gray-queue child enumeration with `self.gray_queue` as scratch (it is
    /// empty during concurrent marking — the snapshot was handed to the thread).
    ///
    /// Per-cycle dedup for multi-child owners (veclike/string): the barrier can't
    /// know which slot the bulk closure will touch, so it logs the owner's WHOLE
    /// pre-image; doing that on every write is O(n) per write => O(n²) to build an
    /// n-element container (hash table, char-table, or a vector filled by `aset`
    /// in a loop — the `(ucs-names)` OOM). SATB only needs each owner's
    /// start-of-cycle child set logged ONCE: at the owner's FIRST mutation this
    /// cycle every snapshot-time child is still present (a child can only be
    /// unlinked by a mutation of THIS owner, i.e. this very first barrier firing
    /// pre-store), so one snapshot is a superset of the snapshot-time children;
    /// later writes overwrite only already-logged values (or born-black new ones,
    /// which need no logging). So re-snapshotting is pure waste — skip it. The
    /// snapshot set is cleared at every mark start (`concurrent_begin`).
    ///
    /// Conses (exactly two children) bypass the dedup: their barrier is already
    /// O(1), and a per-write `HashSet` insert on the hot car/cdr path would cost
    /// more than it saves. Re-logging a cons's 2 children is still SATB-correct.
    fn push_value_children_to_satb_shared(&mut self, owner: TaggedValue) {
        debug_assert!(self.gray_queue.is_empty());
        // Multi-child owners are deduped once per cycle; conses fall through to
        // the cheap direct enumeration below.
        if !owner.is_cons() && !self.satb_snapshotted_owners.insert(owner.bits()) {
            return; // this owner's full pre-image was already logged this cycle
        }
        self.push_value_children_to_gray(owner, "satb-concurrent");
        if !self.gray_queue.is_empty() {
            let mut shared = self.satb_shared.lock().unwrap();
            shared.extend(self.gray_queue.drain(..));
        }
    }

    /// SATB sink for a ROOT-slot overwrite (a symbol value/function/plist cell):
    /// log the pre-image VALUE itself so the concurrent mark grays and traces it
    /// (`join_concurrent_mark` folds `satb_shared` into the gray queue), keeping a
    /// symbol-only-reachable object live across the cycle. Unlike
    /// `push_value_children_to_satb_shared`, the retained thing is the overwritten
    /// value itself, not an owner's children — the symbol cell's "owner" is a
    /// non-heap root. No `concurrent_mark_running` assert: the caller already gated
    /// on the `TAGGED_HEAP_CONCURRENT_ACTIVE` thread-local (the source of truth),
    /// and an extra entry is at worst one cycle of floating garbage.
    fn note_root_overwrite_value(&mut self, pre_image: TaggedValue) {
        self.satb_shared.lock().unwrap().push(pre_image);
    }

    /// Stage 2 Tier B CONCURRENT VECTOR SCAN clone-on-write hook. Called from
    /// `with_vector_data_mut` BEFORE a vector's OWNED backing is bulk-mutated, while a
    /// concurrent mark is active. On the owner's FIRST such
    /// mutation this cycle, if the backing is currently OWNED, replace it with a clone
    /// and RETIRE the original (kept alive to join) so the GC thread's start-of-cycle
    /// snapshot pointer keeps addressing an immutable, live buffer; the closure then
    /// mutates the clone. Idempotent per owner per cycle (dedup set), and a no-op when
    /// the backing is MAPPED (the snapshot points at the immutable dump; `ensure_owned`
    /// will promote it to a fresh OWNED the snapshot never reads, so no clone needed).
    ///
    /// Reachability of the pre-image children is handled separately by the
    /// `note_heap_write(VectorBulk)` SATB barrier the caller fires first; this hook
    /// only preserves the snapshot pointer's buffer for the concurrent READ.
    ///
    /// Safety: `owner` must be a live `VecLikeType::Vector` value on this heap.
    pub(crate) fn concurrent_clone_on_write_vector(&mut self, owner: TaggedValue) {
        // First mutation of this owner this cycle? `insert` returns false if already
        // present, so later mutations of the same owner skip the clone (they touch the
        // already-cloned live backing the snapshot does not point at).
        if !self.concurrent_cloned_vectors.insert(owner.bits()) {
            return;
        }
        let Some(header) = owner.as_veclike_ptr() else {
            return;
        };
        let obj = unsafe { &mut *(header as *mut VectorObj) };
        // Only OWNED backings need cloning: a MAPPED backing reads the immutable dump
        // span the snapshot captured; `ensure_owned` (run by the caller next) promotes
        // it to a brand-new OWNED buffer the snapshot never addresses.
        if !obj.data.is_owned() {
            return;
        }
        // Replace the backing with a clone; retire the original so the GC's snapshot
        // pointer keeps addressing it (immutable + alive) until the join free point.
        let original = obj.data.clone_owned_backing();
        self.retired_vector_buffers.push(original);
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
        // Queue doomed finalizers first (mirrors `complete_collection`; a miss
        // here would mean finalizers silently never run under the concurrent
        // collector). The main mark has drained — the termination handshake
        // already traced the deferred veclikes — so marks are final.
        self.mark_and_queue_doomed_finalizers();
        // Resolve weak hash tables (GNU mark_and_sweep_weak_table_contents): mark
        // entries that survive per their table's weakness, then drop the rest. This
        // mirrors `complete_collection` and MUST run on the concurrent/incremental
        // termination too — otherwise a weak table's only-weakly-reachable entries
        // are neither marked nor removed, so they are swept while still referenced
        // by the table (UAF). The main mark has already drained at this point.
        self.mark_and_sweep_weak_tables();

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
        self.sweep_slice_us_total = 0;
        self.sweep_slice_count = 0;
        self.sweep_cons_blocks_swept = 0;
        self.sweep_noncons_freed = 0;
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
        let mut noncons_freed = 0usize;
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
                    noncons_freed += 1;
                }
            }
            processed += 1;
        }

        let done = self.sweep_cons_cursor >= self.cons_blocks.len()
            && self.sweep_noncons_pending.is_null();
        let slice_us = t0.elapsed().as_micros() as u64;
        self.sweep_slice_us_total += slice_us;
        self.sweep_slice_count += 1;
        self.sweep_cons_blocks_swept += swept_blocks;
        self.sweep_noncons_freed += noncons_freed;
        if std::env::var("NEOVM_GC_TRACE").as_deref() == Ok("1") {
            eprintln!(
                "NEOVM_GC sweep_slice {slice_us}us cons={}/{} noncons_left={} done={done}",
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
        self.sweep_lifetime_us += self.sweep_slice_us_total;
        self.sweep_lifetime_slices += self.sweep_slice_count;
        self.sweep_lifetime_cons_blocks_swept += self.sweep_cons_blocks_swept;
        self.sweep_lifetime_noncons_freed += self.sweep_noncons_freed;
        if std::env::var("NEOVM_GC_TRACE").as_deref() == Ok("1") {
            let (mapped_total, mapped_marked) = self.mapped_object_stats();
            eprintln!(
                "NEOVM_GC gc#{} [incremental mark={}us sweep_total={}us slices={} blocks={} \
                 noncons_freed={}] cons_live={} heap_noncons={} dump_marked={}/{} live={}B",
                self.gc_collections,
                self.sweep_mark_us,
                self.sweep_slice_us_total,
                self.sweep_slice_count,
                self.sweep_cons_blocks_swept,
                self.sweep_noncons_freed,
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
            VecLikeType::Record | VecLikeType::WindowConfiguration => {
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
                    // `mark_and_sweep_weak_table_contents`). The remembered-set /
                    // SATB / permanent-scan paths now also defer weak entries
                    // (`register_weak_hash_table_for_sweep` registers the table
                    // and pushes only its non-weak closures), and a tenured/mapped
                    // weak table is re-registered every cycle via
                    // `permanent_weak_hash_tables`, so weak semantics hold for
                    // young, tenured, and dumped tables alike. The weak sweep runs
                    // before `verify_dump_partition`, so dead entries are removed
                    // before the verifier enumerates — no UAF.
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
            VecLikeType::Finalizer => {
                // A REACHABLE finalizer keeps its function alive (GNU
                // `mark_vectorlike` on PVEC_FINALIZER). Unreachable ones are
                // handled at mark termination by
                // `mark_and_queue_doomed_finalizers`.
                let function = unsafe { (*(ptr as *const FinalizerObj)).function };
                if function.is_heap_object() {
                    self.push_gray(function, "finalizer-function");
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
                    VecLikeType::Record | VecLikeType::WindowConfiguration => unsafe {
                        drop(Box::from_raw(ptr as *mut RecordObj))
                    },
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
                    VecLikeType::Finalizer => unsafe {
                        // The registry entry was already removed by the
                        // mark-termination scan that doomed this object; the
                        // function it queued survives independently.
                        drop(Box::from_raw(ptr as *mut FinalizerObj))
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
        // A live concurrent mark holds start-of-cycle snapshots into this
        // heap (cons blocks + their mark bitmaps, vector backings, the
        // Context obarray) on the GC thread. Reclaim exclusive ownership
        // BEFORE freeing anything it can still read. `tagged_heap` is the
        // first `Context` field, so this join also runs before the obarray
        // drops. No-op when no mark is in flight.
        if self.concurrent_mark_running {
            self.join_concurrent_mark();
        }
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

/// TEST-ONLY allocation-profiling counters for the non-cons allocator
/// modernization probes (size-class arena design inputs): per-kind allocation
/// counts, a size-class histogram over TOTAL object bytes (fixed struct +
/// separately-allocated payload storage, via `object_bytes_from_header`),
/// per-kind byte totals, and the peak `non_cons_object_addrs` population.
/// Compiled ONLY under `cfg(test)` (the consuming probes are in-crate
/// `#[ignore]`d tests), so production builds carry zero instrumentation.
/// Global statics are correct here because nextest runs each probe in its own
/// process, so the counters observe exactly one workload.
#[cfg(test)]
pub(crate) mod alloc_probe {
    use super::{GcHeader, HeapObjectKind, TaggedHeap, VecLikeHeader, VecLikeType};
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

    const N_KINDS: usize = 28;
    const N_BUCKETS: usize = 11;

    /// Dense kind index: String, Float, then every `VecLikeType` variant.
    pub(crate) const KIND_NAMES: [&str; N_KINDS] = [
        "String",
        "Float",
        "Vector",
        "Bignum",
        "Marker",
        "Overlay",
        "Finalizer",
        "SymbolWithPos",
        "UserPtr",
        "Process",
        "Frame",
        "Window",
        "Buffer",
        "HashTable",
        "Obarray",
        "WindowConfig",
        "Subr",
        "Xwidget",
        "XwidgetView",
        "ModuleFunction",
        "Sqlite",
        "Lambda",
        "CharTable",
        "SubCharTable",
        "Record",
        "Macro",
        "ByteCode",
        "Timer",
    ];
    /// Histogram bucket upper bounds (bytes).
    pub(crate) const BUCKET_LABELS: [&str; N_BUCKETS] = [
        "<=16", "<=32", "<=64", "<=128", "<=256", "<=512", "<=1K", "<=4K", "<=16K", "<=64K",
        ">64K",
    ];

    #[allow(clippy::declare_interior_mutable_const)]
    const ZERO: AtomicU64 = AtomicU64::new(0);
    #[allow(clippy::declare_interior_mutable_const)]
    const ROW: [AtomicU64; N_BUCKETS] = [ZERO; N_BUCKETS];
    static COUNTS: [[AtomicU64; N_BUCKETS]; N_KINDS] = [ROW; N_KINDS];
    static TOTAL_BYTES: [AtomicU64; N_KINDS] = [ZERO; N_KINDS];
    static PEAK_ADDR_SET: AtomicUsize = AtomicUsize::new(0);

    fn kind_index(header: *const GcHeader) -> usize {
        match unsafe { (*header).kind } {
            HeapObjectKind::String => 0,
            HeapObjectKind::Float => 1,
            HeapObjectKind::VecLike => {
                2 + match unsafe { (*(header as *const VecLikeHeader)).type_tag } {
                    VecLikeType::Vector => 0,
                    VecLikeType::Bignum => 1,
                    VecLikeType::Marker => 2,
                    VecLikeType::Overlay => 3,
                    VecLikeType::Finalizer => 4,
                    VecLikeType::SymbolWithPos => 5,
                    VecLikeType::UserPtr => 6,
                    VecLikeType::Process => 7,
                    VecLikeType::Frame => 8,
                    VecLikeType::Window => 9,
                    VecLikeType::Buffer => 10,
                    VecLikeType::HashTable => 11,
                    VecLikeType::Obarray => 12,
                    VecLikeType::WindowConfiguration => 13,
                    VecLikeType::Subr => 14,
                    VecLikeType::Xwidget => 15,
                    VecLikeType::XwidgetView => 16,
                    VecLikeType::ModuleFunction => 17,
                    VecLikeType::Sqlite => 18,
                    VecLikeType::Lambda => 19,
                    VecLikeType::CharTable => 20,
                    VecLikeType::SubCharTable => 21,
                    VecLikeType::Record => 22,
                    VecLikeType::Macro => 23,
                    VecLikeType::ByteCode => 24,
                    VecLikeType::Timer => 25,
                }
            }
        }
    }

    fn bucket(bytes: usize) -> usize {
        match bytes {
            0..=16 => 0,
            17..=32 => 1,
            33..=64 => 2,
            65..=128 => 3,
            129..=256 => 4,
            257..=512 => 5,
            513..=1024 => 6,
            1025..=4096 => 7,
            4097..=16384 => 8,
            16385..=65536 => 9,
            _ => 10,
        }
    }

    /// Record one non-cons allocation at link time (`link_object` /
    /// `link_veclike`). The object is fully constructed before it is linked,
    /// so reading its payload sizes here is sound.
    pub(crate) fn record(header: *const GcHeader, addr_set_len: usize) {
        let bytes = TaggedHeap::object_bytes_from_header(header);
        let k = kind_index(header);
        COUNTS[k][bucket(bytes)].fetch_add(1, Ordering::Relaxed);
        TOTAL_BYTES[k].fetch_add(bytes as u64, Ordering::Relaxed);
        PEAK_ADDR_SET.fetch_max(addr_set_len, Ordering::Relaxed);
    }

    /// Zero every counter (start of a probe's measured phase).
    pub(crate) fn reset() {
        for row in &COUNTS {
            for cell in row {
                cell.store(0, Ordering::Relaxed);
            }
        }
        for cell in &TOTAL_BYTES {
            cell.store(0, Ordering::Relaxed);
        }
        PEAK_ADDR_SET.store(0, Ordering::Relaxed);
    }

    /// Peak `non_cons_object_addrs` population observed since reset.
    pub(crate) fn peak_addr_set() -> usize {
        PEAK_ADDR_SET.load(Ordering::Relaxed)
    }

    /// The fixed (arena-resident) struct size per kind index — what a
    /// size-class arena page would actually hold. Payload storage (`Vec`
    /// backings, string text, hash-table internals) stays on the system
    /// allocator either way.
    pub(crate) fn fixed_size(kind: usize) -> usize {
        use std::mem::size_of;
        match kind {
            0 => size_of::<super::StringObj>(),
            1 => size_of::<super::FloatObj>(),
            2 => size_of::<super::VectorObj>(),
            3 => size_of::<super::BignumObj>(),
            4 => size_of::<super::MarkerObj>(),
            5 => size_of::<super::OverlayObj>(),
            6 => size_of::<super::FinalizerObj>(),
            7 => size_of::<super::SymbolWithPosObj>(),
            8 => size_of::<super::UserPtrObj>(),
            9 => size_of::<super::ProcessObj>(),
            10 => size_of::<super::FrameObj>(),
            11 => size_of::<super::WindowObj>(),
            12 => size_of::<super::BufferObj>(),
            13 => size_of::<super::HashTableObj>(),
            14 => size_of::<super::ObarrayObj>(),
            15 => size_of::<super::RecordObj>(), // WindowConfiguration shares RecordObj
            16 => size_of::<super::SubrObj>(),
            17 => size_of::<super::XwidgetObj>(),
            18 => size_of::<super::XwidgetViewObj>(),
            19 => size_of::<super::ModuleFunctionObj>(),
            20 => size_of::<super::SqliteObj>(),
            21 => size_of::<super::LambdaObj>(),
            22 => size_of::<super::CharTableObj>(),
            23 => size_of::<super::SubCharTableObj>(),
            24 => size_of::<super::RecordObj>(),
            25 => size_of::<super::MacroObj>(),
            26 => size_of::<super::ByteCodeObj>(),
            27 => size_of::<super::TimerObj>(),
            _ => 0,
        }
    }

    /// Render the per-kind allocation table: count, total bytes, fixed
    /// (arena-resident) struct size, and the total-bytes histogram row.
    pub(crate) fn report() -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "{:<14} {:>10} {:>13} {:>6}  {}\n",
            "kind",
            "allocs",
            "total_bytes",
            "fixed",
            BUCKET_LABELS.join(" ")
        ));
        let mut grand_allocs = 0u64;
        let mut grand_bytes = 0u64;
        for k in 0..N_KINDS {
            let count: u64 = COUNTS[k].iter().map(|c| c.load(Ordering::Relaxed)).sum();
            if count == 0 {
                continue;
            }
            let bytes = TOTAL_BYTES[k].load(Ordering::Relaxed);
            grand_allocs += count;
            grand_bytes += bytes;
            let histo: Vec<String> = COUNTS[k]
                .iter()
                .map(|c| c.load(Ordering::Relaxed).to_string())
                .collect();
            out.push_str(&format!(
                "{:<14} {:>10} {:>13} {:>6}  {}\n",
                KIND_NAMES[k],
                count,
                bytes,
                fixed_size(k),
                histo.join(" ")
            ));
        }
        out.push_str(&format!(
            "TOTAL allocs={grand_allocs} bytes={grand_bytes} peak_non_cons_object_addrs={}\n",
            peak_addr_set()
        ));
        out
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

    /// Gap 3 instrumentation: a deferred sweep must aggregate per-slice cost
    /// (slice count, total µs, cons blocks, non-cons frees) into `sweep_stats`
    /// and fold the cycle into the lifetime totals at completion.
    #[test]
    fn deferred_sweep_aggregates_slice_stats() {
        crate::test_utils::init_test_tracing();
        let mut heap = TaggedHeap::new();
        set_tagged_heap(&mut heap);

        // A small rooted list plus lots of garbage: dead conses spanning many
        // blocks and dead non-cons objects, so the sliced sweep has real work.
        let mut rooted = TaggedValue::fixnum(0);
        for i in 0..1_000 {
            rooted = heap.alloc_cons(TaggedValue::fixnum(i), rooted);
        }
        for i in 0..400_000 {
            let _ = heap.alloc_cons(TaggedValue::fixnum(i), TaggedValue::fixnum(0));
        }
        for i in 0..4_000 {
            let _ = heap.alloc_float(i as f64);
        }

        // Mark to a fixpoint, arm the deferred sweep (the incremental
        // termination path), then drain it in bounded slices.
        heap.begin_collection();
        heap.seed_root(rooted);
        let bytes_before = heap.live_bytes();
        heap.incremental_drain_all();
        heap.incremental_finish(bytes_before, std::time::Instant::now());
        assert!(heap.sweep_in_progress());
        let mut slices = 1usize;
        while !heap.incremental_sweep_slice(8) {
            slices += 1;
        }
        assert!(!heap.sweep_in_progress());

        let stats = heap.sweep_stats();
        assert_eq!(stats.slice_count, slices);
        assert!(stats.slice_count > 1, "budget 8 must take several slices");
        assert!(stats.sweep_us > 0, "aggregated sweep cost must be non-zero");
        assert!(stats.cons_blocks_swept > 0);
        assert!(
            stats.noncons_freed >= 4_000,
            "the dead floats must be reclaimed by the deferred sweep \
             (freed={})",
            stats.noncons_freed,
        );
        assert_eq!(stats.lifetime_slices, stats.slice_count);
        assert_eq!(stats.lifetime_sweep_us, stats.sweep_us);
        assert_eq!(stats.lifetime_cons_blocks_swept, stats.cons_blocks_swept);
        assert_eq!(stats.lifetime_noncons_freed, stats.noncons_freed);
    }

    /// Gap 3 instrumentation: `join_concurrent_mark` must record how many
    /// GC-thread-parked (deferred) values the STW termination drain was handed
    /// — the number that sizes a records/closures/strings concurrent tier.
    #[test]
    fn concurrent_termination_records_deferred_drain_size() {
        crate::test_utils::init_test_tracing();
        let mut heap = TaggedHeap::new();
        set_tagged_heap(&mut heap);

        // A rooted cons spine carrying non-cons cars: the GC thread marks the
        // owned conses but parks every non-cons in `deferred`, so the
        // termination drain size is deterministically >= the car count.
        let mut list = TaggedValue::fixnum(0);
        for i in 0..1_000 {
            let car = heap.alloc_float(i as f64);
            list = heap.alloc_cons(car, list);
        }
        let root = list;

        heap.concurrent_begin();
        heap.seed_root(root);
        heap.launch_concurrent_mark();
        while !heap.concurrent_mark_done() {
            std::thread::yield_now();
        }
        heap.join_concurrent_mark();

        let stats = heap.sweep_stats();
        assert!(
            stats.last_termination_deferred >= 1_000,
            "every non-cons car must be parked for the termination drain \
             (deferred={})",
            stats.last_termination_deferred,
        );
        assert!(stats.max_termination_deferred >= stats.last_termination_deferred);
        assert_eq!(stats.last_termination_satb, 0, "no mutation ran mid-mark");

        // Finish the cycle cleanly: termination drain + deferred sweep.
        heap.reseed_runtime_and_remembered_roots();
        heap.seed_root(root);
        let bytes_before = heap.live_bytes();
        heap.incremental_drain_all();
        heap.incremental_finish(bytes_before, std::time::Instant::now());
        heap.finish_incremental_sweep_now();
        assert!(!heap.sweep_in_progress());
        assert_eq!(heap.sweep_stats().noncons_freed, 0, "all floats are live");
    }

    /// Termination-drain kind probe: a concurrent cycle over a rooted spine
    /// carrying known counts of strings/records/closures/floats/hash-tables/
    /// vectors must classify every parked entry into the right bucket. Each
    /// value is reachable ONLY through the rooted cons spine, so the GC
    /// thread's cons walk discovers it and parks it in `deferred` (vectors
    /// included — Tier B traces their BACKINGS concurrently, but the vector
    /// VALUE is still parked for its header mark). CONCURRENT STRING MARKING:
    /// interval-FREE strings are now claimed on the GC thread instead of
    /// parked, so the `str` bucket counts only the interval-BEARING ones and
    /// the claim counter covers the rest.
    #[test]
    fn concurrent_termination_classifies_deferred_kinds() {
        use crate::emacs_core::value::{HashTableTest, LispHashTable};

        crate::test_utils::init_test_tracing();
        let mut heap = TaggedHeap::new();
        set_tagged_heap(&mut heap);

        const N_STR: usize = 300;
        const N_STR_PROPS: usize = 40;
        const N_REC: usize = 200;
        const N_LAMBDA: usize = 150;
        const N_MACRO: usize = 30;
        const N_FLT: usize = 120;
        const N_HT: usize = 8;
        const N_VEC: usize = 50;

        let mut list = TaggedValue::fixnum(0);
        for _ in 0..N_STR {
            let s = heap.alloc_string(crate::heap_types::LispString::from_utf8("drain-kind"));
            list = heap.alloc_cons(s, list);
        }
        // Interval-BEARING strings: still parked for the termination drain
        // (their interval children must be traced by `mark_value`).
        for _ in 0..N_STR_PROPS {
            let s = heap.alloc_string(crate::heap_types::LispString::from_utf8("drain-props"));
            let payload = heap.alloc_cons(TaggedValue::fixnum(9), TaggedValue::fixnum(0));
            let ptr = s.as_string_ptr().unwrap() as *mut StringObj;
            // Pre-mark direct install on a just-allocated string (unpublished
            // to any concurrent cycle yet).
            unsafe { *(*ptr).data.intervals_mut() = interval_table_carrying(payload) };
            list = heap.alloc_cons(s, list);
        }
        for i in 0..N_REC {
            let r = heap.alloc_record(vec![TaggedValue::fixnum(i as i64)]);
            list = heap.alloc_cons(r, list);
        }
        for _ in 0..N_LAMBDA {
            let c = heap.alloc_lambda(vec![TaggedValue::fixnum(1)]);
            list = heap.alloc_cons(c, list);
        }
        for _ in 0..N_MACRO {
            let m = heap.alloc_macro(vec![TaggedValue::fixnum(2)]);
            list = heap.alloc_cons(m, list);
        }
        for i in 0..N_FLT {
            let f = heap.alloc_float(i as f64);
            list = heap.alloc_cons(f, list);
        }
        for _ in 0..N_HT {
            let h = heap.alloc_hash_table(LispHashTable::new(HashTableTest::Equal));
            list = heap.alloc_cons(h, list);
        }
        for i in 0..N_VEC {
            let v = heap.alloc_vector(vec![TaggedValue::fixnum(i as i64); 4]);
            list = heap.alloc_cons(v, list);
        }
        let root = list;

        heap.concurrent_begin();
        heap.seed_root(root);
        heap.launch_concurrent_mark();
        while !heap.concurrent_mark_done() {
            std::thread::yield_now();
        }
        heap.join_concurrent_mark();

        let stats = heap.sweep_stats();
        let kinds = stats.last_termination_kinds;
        assert!(
            stats.last_concurrent_str_claimed >= N_STR,
            "interval-free strings are claimed concurrently, not parked \
             (claimed={})",
            stats.last_concurrent_str_claimed,
        );
        assert!(
            kinds.string >= N_STR_PROPS,
            "interval-bearing strings stay parked (str={})",
            kinds.string,
        );
        assert!(
            kinds.string < N_STR,
            "the interval-free majority must have left the parked buffer \
             (str={})",
            kinds.string,
        );
        assert!(kinds.record >= N_REC, "records parked (rec={})", kinds.record);
        assert!(
            kinds.closure >= N_LAMBDA + N_MACRO,
            "lambdas + macros share the closure bucket (clo={})",
            kinds.closure,
        );
        assert!(kinds.float >= N_FLT, "floats parked (f={})", kinds.float);
        assert!(
            kinds.hash_table >= N_HT,
            "hash tables parked (ht={})",
            kinds.hash_table,
        );
        assert!(
            kinds.vector >= N_VEC,
            "vector VALUES are parked for their header mark even though their \
             backings trace concurrently (vec={})",
            kinds.vector,
        );
        assert_eq!(
            kinds.total(),
            stats.last_termination_deferred,
            "every deferred entry lands in exactly one bucket",
        );
        assert_eq!(stats.termination_count, 1);
        assert!(stats.max_termination_kinds.string >= kinds.string);
        assert!(stats.max_termination_kinds.record >= kinds.record);
        assert!(stats.max_termination_kinds.closure >= kinds.closure);

        // Finish the cycle cleanly: termination drain + deferred sweep.
        heap.reseed_runtime_and_remembered_roots();
        heap.seed_root(root);
        let bytes_before = heap.live_bytes();
        heap.incremental_drain_all();
        heap.incremental_finish(bytes_before, std::time::Instant::now());
        heap.finish_incremental_sweep_now();
        assert!(!heap.sweep_in_progress());
        assert_eq!(heap.sweep_stats().noncons_freed, 0, "everything is rooted");
    }

    /// Build an interval table whose sole plist value is `v` (chars [0, 1)).
    /// `for_each_root` yields the plist (a heap cons chain carrying `v`), so
    /// marking the table's roots transitively keeps `v` alive. Allocates the
    /// plist conses on the thread-local tagged heap.
    fn interval_table_carrying(v: TaggedValue) -> crate::buffer::text_props::TextPropertyTable {
        use crate::buffer::text_props::{PropertyInterval, TextPropertyTable};
        let key = TaggedValue::fixnum(1);
        let mut properties = std::collections::HashMap::new();
        properties.insert(key, v);
        TextPropertyTable::from_dump(vec![PropertyInterval {
            start: 0,
            end: 1,
            properties,
            key_order: vec![key],
        }])
    }

    /// Drive one full concurrent cycle to completion: wait for the GC thread,
    /// terminate stop-the-world with `root` re-seeded, and drain the deferred
    /// sweep. Mirrors the driver's state machine (and the other tests here).
    fn finish_concurrent_cycle(heap: &mut TaggedHeap, root: TaggedValue) {
        while !heap.concurrent_mark_done() {
            std::thread::yield_now();
        }
        heap.join_concurrent_mark();
        heap.reseed_runtime_and_remembered_roots();
        heap.seed_root(root);
        let bytes_before = heap.live_bytes();
        heap.incremental_drain_all();
        heap.incremental_finish(bytes_before, std::time::Instant::now());
        heap.finish_incremental_sweep_now();
        assert!(!heap.sweep_in_progress());
    }

    /// CONCURRENT STRING MARKING, load-bearing-barrier proof (production
    /// path): a string S whose interval table is the ONLY reference to value V
    /// has that table dropped MID-MARK through the `mutate.rs` wrapper. V must
    /// survive the cycle purely via the SATB pre-image log — whichever side of
    /// the clear the GC thread observed S on (non-null ⇒ deferred, then the
    /// termination traces an already-empty table; null ⇒ claimed, never
    /// re-traced).
    #[test]
    fn concurrent_string_claim_and_interval_clear_keep_children_alive() {
        crate::test_utils::init_test_tracing();
        let mut heap = TaggedHeap::new();
        set_tagged_heap(&mut heap);

        // V (and its plist chain): reachable ONLY via S's interval table.
        let v = heap.alloc_cons(TaggedValue::fixnum(41), TaggedValue::fixnum(42));
        let s = heap.alloc_string(crate::heap_types::LispString::from_utf8("props"));
        {
            let ptr = s.as_string_ptr().unwrap() as *mut StringObj;
            // Pre-mark install on a fresh string: no barrier needed yet.
            unsafe { *(*ptr).data.intervals_mut() = interval_table_carrying(v) };
        }
        // S2: interval-free — exercises the claim fast path alongside.
        let s2 = heap.alloc_string(crate::heap_types::LispString::from_utf8("plain"));
        // Long spine so the GC thread is (almost certainly) still marking the
        // list when the mutator clears. Both correctness outcomes are asserted
        // identically, so the race direction cannot break the test.
        let mut list = heap.alloc_cons(s2, TaggedValue::fixnum(0));
        list = heap.alloc_cons(s, list);
        for i in 0..300_000 {
            list = heap.alloc_cons(TaggedValue::fixnum(i), list);
        }
        let root = list;

        heap.concurrent_begin();
        heap.seed_root(root);
        heap.launch_concurrent_mark();

        // Mid-mark, on the mutator thread: drop S's whole interval table via
        // the barrier wrapper (fires the StringData SATB pre-image push AND
        // the enforced in-mutator interval barrier).
        let cleared = crate::tagged::mutate::with_lisp_string_mut(s, |ls| ls.clear_intervals());
        assert!(cleared.is_some());

        finish_concurrent_cycle(&mut heap, root);

        // V survived the cycle purely via SATB.
        assert_eq!(
            unsafe { (*v.xcons_ptr()).load_car() }.0,
            TaggedValue::fixnum(41).0,
        );
        assert!(heap.owns_non_cons_object(s.as_string_ptr().unwrap() as *const u8));
        assert!(heap.owns_non_cons_object(s2.as_string_ptr().unwrap() as *const u8));
    }

    /// Same as above, but the mid-mark clear BYPASSES the `mutate.rs` wrappers
    /// entirely (raw `clear_intervals` on the payload) — proving the SATB
    /// barrier is enforced INSIDE the `LispString` mutators and cannot be
    /// skipped by any call site.
    #[test]
    fn concurrent_string_raw_interval_clear_keeps_children_alive() {
        crate::test_utils::init_test_tracing();
        let mut heap = TaggedHeap::new();
        set_tagged_heap(&mut heap);

        let v = heap.alloc_cons(TaggedValue::fixnum(51), TaggedValue::fixnum(52));
        let s = heap.alloc_string(crate::heap_types::LispString::from_utf8("raw-clear"));
        let s_ptr = s.as_string_ptr().unwrap() as *mut StringObj;
        unsafe { *(*s_ptr).data.intervals_mut() = interval_table_carrying(v) };
        let mut list = heap.alloc_cons(s, TaggedValue::fixnum(0));
        for i in 0..300_000 {
            list = heap.alloc_cons(TaggedValue::fixnum(i), list);
        }
        let root = list;

        heap.concurrent_begin();
        heap.seed_root(root);
        heap.launch_concurrent_mark();

        // Raw mutator call — no wrapper, no note_heap_write. The enforced
        // in-mutator barrier inside clear_intervals must log V's plist.
        unsafe { (*s_ptr).data.clear_intervals() };

        finish_concurrent_cycle(&mut heap, root);

        assert_eq!(
            unsafe { (*v.xcons_ptr()).load_car() }.0,
            TaggedValue::fixnum(51).0,
        );
        assert!(heap.owns_non_cons_object(s_ptr as *const u8));
    }

    /// The claim + clear flow under the ARMED partition/tricolor verifiers
    /// (`NEOVM_GC_VERIFY_PARTITION=1`): `verify_incremental_tricolor` is the
    /// oracle that a concurrently-claimed (black) string presents no
    /// black->white edge at termination. The fake dump span only activates the
    /// partition; it maps no objects, so every string stays span-outside
    /// (owned, claim-eligible).
    #[test]
    fn concurrent_string_claim_passes_partition_verifier() {
        crate::test_utils::init_test_tracing();
        unsafe { std::env::set_var("NEOVM_GC_VERIFY_PARTITION", "1") };
        let mut heap = TaggedHeap::new();
        set_tagged_heap(&mut heap);
        heap.extend_dump_span(4096, 16);

        // First partitioned cycle promotes + blackens; verifiers arm after it.
        heap.begin_collection();
        heap.complete_collection();
        assert!(heap.dump_blackened);

        let v = heap.alloc_cons(TaggedValue::fixnum(61), TaggedValue::fixnum(62));
        let s = heap.alloc_string(crate::heap_types::LispString::from_utf8("verified"));
        let s_ptr = s.as_string_ptr().unwrap() as *mut StringObj;
        unsafe { *(*s_ptr).data.intervals_mut() = interval_table_carrying(v) };
        let s2 = heap.alloc_string(crate::heap_types::LispString::from_utf8("verified-free"));
        let mut list = heap.alloc_cons(s2, TaggedValue::fixnum(0));
        list = heap.alloc_cons(s, list);
        for i in 0..200_000 {
            list = heap.alloc_cons(TaggedValue::fixnum(i), list);
        }
        let root = list;

        heap.concurrent_begin();
        heap.seed_root(root);
        heap.launch_concurrent_mark();
        let _ = crate::tagged::mutate::with_lisp_string_mut(s, |ls| ls.clear_intervals());
        // `incremental_finish` (inside) runs verify_dump_partition +
        // verify_incremental_tricolor and panics on any violation.
        finish_concurrent_cycle(&mut heap, root);

        assert_eq!(
            unsafe { (*v.xcons_ptr()).load_car() }.0,
            TaggedValue::fixnum(61).0,
        );
        assert!(heap.owns_non_cons_object(s_ptr as *const u8));
        assert!(heap.owns_non_cons_object(s2.as_string_ptr().unwrap() as *const u8));
    }

    /// MAPPED-STRING CLASSIFICATION (regression guard for the mis-claim UAF):
    /// with the partition span covering a registered mapped string, the GC
    /// thread must DEFER it (its `GcHeader` bit untouched — mapped strings
    /// mark via the `MappedStringObject` side bool) and the termination must
    /// mark it on the mapped path and trace its interval child.
    #[test]
    fn concurrent_mark_defers_mapped_strings_and_marks_their_interval_children() {
        crate::test_utils::init_test_tracing();
        let mut heap = TaggedHeap::new();
        set_tagged_heap(&mut heap);

        // Fake-mapped string: a leaked StringObj registered exactly like the
        // pdump loader registers image objects (extends the dump span over it).
        let mapped = Box::into_raw(Box::new(StringObj {
            header: GcHeader::new(HeapObjectKind::String),
            data: crate::heap_types::LispString::from_utf8("mapped"),
        }));
        unsafe { heap.register_mapped_string_object(mapped, std::mem::size_of::<StringObj>()) };
        // C: heap value reachable ONLY via the mapped string's interval table.
        let c = heap.alloc_cons(TaggedValue::fixnum(7), TaggedValue::fixnum(8));
        unsafe { *(*mapped).data.intervals_mut() = interval_table_carrying(c) };
        let mapped_val = unsafe { TaggedValue::from_string_ptr(mapped) };
        let root = heap.alloc_cons(mapped_val, TaggedValue::fixnum(0));

        // First cycle with a partition is a full trace (dump not blackened):
        // mapped marks were cleared, so the termination must re-mark the
        // mapped string and trace its intervals.
        heap.concurrent_begin();
        heap.seed_root(root);
        heap.launch_concurrent_mark();
        while !heap.concurrent_mark_done() {
            std::thread::yield_now();
        }
        heap.join_concurrent_mark();

        let stats = heap.sweep_stats();
        assert!(
            stats.last_termination_kinds.string >= 1,
            "the mapped string must be parked, not claimed (str={})",
            stats.last_termination_kinds.string,
        );
        assert_eq!(
            stats.last_concurrent_str_claimed, 0,
            "nothing here is claim-eligible",
        );
        assert!(
            unsafe { !(*mapped).header.is_marked() },
            "a mapped string's GcHeader bit must never be claimed by the GC \
             thread (mapped marks live in the side table)",
        );

        heap.reseed_runtime_and_remembered_roots();
        heap.seed_root(root);
        let bytes_before = heap.live_bytes();
        heap.incremental_drain_all();
        heap.incremental_finish(bytes_before, std::time::Instant::now());
        heap.finish_incremental_sweep_now();

        // Termination marked it on the mapped path and traced the child.
        let idx = heap.mapped_string_index_by_addr[&(mapped as usize)];
        assert!(heap.mapped_string_objects[idx].marked);
        assert_eq!(
            unsafe { (*c.xcons_ptr()).load_car() }.0,
            TaggedValue::fixnum(7).0,
        );

        // Free the fake image object after the heap is gone.
        drop(heap);
        let _ = unsafe { Box::from_raw(mapped) };
    }

    /// RACE TEST: the mutator flips strings' interval tables None<->Some in a
    /// loop (through the production wrappers) while the GC thread marks a
    /// large spine. Liveness: every flipped-in value and every string must
    /// survive; run under a data-race detector this is the strings race check
    /// (the seqlock test is the precedent).
    #[test]
    fn concurrent_mark_races_interval_flips_and_retains_live_set() {
        crate::test_utils::init_test_tracing();
        let mut heap = TaggedHeap::new();
        set_tagged_heap(&mut heap);

        const N_STR: usize = 512;
        let mut strings = Vec::with_capacity(N_STR);
        let mut values = Vec::with_capacity(N_STR);
        let mut list = TaggedValue::fixnum(0);
        for i in 0..N_STR {
            let v = heap.alloc_cons(TaggedValue::fixnum(i as i64), TaggedValue::fixnum(-1));
            let s = heap.alloc_string(crate::heap_types::LispString::from_utf8("flip"));
            let ptr = s.as_string_ptr().unwrap() as *mut StringObj;
            unsafe { *(*ptr).data.intervals_mut() = interval_table_carrying(v) };
            strings.push(s);
            values.push(v);
            list = heap.alloc_cons(s, list);
        }
        for i in 0..300_000 {
            list = heap.alloc_cons(TaggedValue::fixnum(i), list);
        }
        let root = list;

        heap.concurrent_begin();
        heap.seed_root(root);
        heap.launch_concurrent_mark();

        // Mutator: clear + reinstall every string's table, twice, while the
        // GC thread walks the spine and claims/defers the strings.
        for round in 0..2 {
            for (i, s) in strings.iter().enumerate() {
                let _ = crate::tagged::mutate::with_lisp_string_mut(*s, |ls| ls.clear_intervals());
                if round == 0 || i % 2 == 0 {
                    let table = interval_table_carrying(values[i]);
                    let _ = crate::tagged::mutate::with_string_text_props_mut(*s, |t| *t = table);
                }
            }
        }

        finish_concurrent_cycle(&mut heap, root);

        for (i, v) in values.iter().enumerate() {
            assert_eq!(
                unsafe { (*v.xcons_ptr()).load_car() }.0,
                TaggedValue::fixnum(i as i64).0,
                "flipped-in interval value #{i} must survive",
            );
        }
        for s in &strings {
            assert!(heap.owns_non_cons_object(s.as_string_ptr().unwrap() as *const u8));
        }
    }

    /// CLAIM-AT-ALL-SINKS (vector sink): strings reachable ONLY through a
    /// vector's slots are discovered by the Tier B backing scan on the GC
    /// thread; the interval-free one must be claimed there (claim counter),
    /// the interval-bearing one parked (str bucket) and its child traced.
    #[test]
    fn concurrent_claim_reaches_vector_slot_strings() {
        crate::test_utils::init_test_tracing();
        let mut heap = TaggedHeap::new();
        set_tagged_heap(&mut heap);

        let s_free = heap.alloc_string(crate::heap_types::LispString::from_utf8("vec-free"));
        let c = heap.alloc_cons(TaggedValue::fixnum(3), TaggedValue::fixnum(4));
        let s_props = heap.alloc_string(crate::heap_types::LispString::from_utf8("vec-props"));
        unsafe {
            *(*(s_props.as_string_ptr().unwrap() as *mut StringObj))
                .data
                .intervals_mut() = interval_table_carrying(c)
        };
        let vec = heap.alloc_vector(vec![s_free, s_props]);
        let root = heap.alloc_cons(vec, TaggedValue::fixnum(0));

        heap.concurrent_begin();
        heap.seed_root(root);
        heap.launch_concurrent_mark();
        while !heap.concurrent_mark_done() {
            std::thread::yield_now();
        }
        heap.join_concurrent_mark();

        let stats = heap.sweep_stats();
        assert!(
            stats.last_concurrent_str_claimed >= 1,
            "the interval-free vector-slot string must be claimed on the GC \
             thread (claimed={})",
            stats.last_concurrent_str_claimed,
        );
        assert!(
            stats.last_termination_kinds.string >= 1,
            "the interval-bearing vector-slot string must be parked (str={})",
            stats.last_termination_kinds.string,
        );

        heap.reseed_runtime_and_remembered_roots();
        heap.seed_root(root);
        let bytes_before = heap.live_bytes();
        heap.incremental_drain_all();
        heap.incremental_finish(bytes_before, std::time::Instant::now());
        heap.finish_incremental_sweep_now();

        assert!(heap.owns_non_cons_object(s_free.as_string_ptr().unwrap() as *const u8));
        assert!(heap.owns_non_cons_object(s_props.as_string_ptr().unwrap() as *const u8));
        assert_eq!(
            unsafe { (*c.xcons_ptr()).load_car() }.0,
            TaggedValue::fixnum(3).0,
        );
    }

    /// CLAIM-AT-ALL-SINKS (obarray sink): a string reachable ONLY through an
    /// obarray symbol's value cell is discovered by the Stage 1b symbol-cell
    /// scan on the GC thread and must be claimed there.
    #[test]
    fn concurrent_claim_reaches_obarray_symbol_value_strings() {
        crate::test_utils::init_test_tracing();
        let mut ev = crate::emacs_core::eval::Context::new();
        set_tagged_heap(&mut ev.tagged_heap);

        // Interval-free string reachable ONLY via the symbol value cell.
        let s = ev
            .tagged_heap
            .alloc_string(crate::heap_types::LispString::from_utf8("obarray-only"));
        ev.obarray.set_symbol_value("neovm--str-claim-probe", s);

        // Stage the obarray snapshot exactly like the start handshake does.
        let snap = ev.obarray.scan_snapshot();
        ev.tagged_heap.set_pending_obarray_scan(snap);
        ev.tagged_heap.concurrent_begin();
        ev.tagged_heap.launch_concurrent_mark();
        while !ev.tagged_heap.concurrent_mark_done() {
            std::thread::yield_now();
        }
        ev.tagged_heap.join_concurrent_mark();

        let stats = ev.tagged_heap.sweep_stats();
        assert!(
            stats.last_concurrent_str_claimed >= 1,
            "the obarray-value string must be claimed via the symbol-cell scan \
             (claimed={})",
            stats.last_concurrent_str_claimed,
        );
        // The claimed string is black.
        assert!(unsafe { (*(s.as_string_ptr().unwrap())).header.is_marked() });
        // No sweep here: this bare-heap driver does not re-seed the Context
        // roots at termination, so sweeping would free live Context objects.
        // Claim + mark are the assertions under test (survival-under-sweep is
        // covered by the vector-sink test); the heap frees everything at drop.
    }

    /// Gap 3: a dump-less heap enables the concurrent collector after its
    /// first completed STW collection (the bootstrap), and a full concurrent
    /// cycle on such a heap retains the rooted live set and reclaims garbage
    /// (mirrors `collect_exact_retains_rooted_and_frees_unrooted`). The dump
    /// span is empty (`dump_addr_lo/hi` = MAX/0), so the GC thread's dump
    /// check must never match and the remembered-set seeding must no-op.
    #[test]
    fn dumpless_heap_enables_concurrent_after_bootstrap_and_collects() {
        crate::test_utils::init_test_tracing();
        let mut heap = TaggedHeap::new();
        set_tagged_heap(&mut heap);

        // Fresh dump-less heap: the first collection must be the STW bootstrap.
        assert!(!heap.should_run_concurrent());

        const N: i64 = 10_000;
        // Rooted list: rooted_head -> cons(N-1) -> ... -> cons(0) -> fixnum(0).
        let mut rooted = TaggedValue::fixnum(0);
        for i in 0..N {
            rooted = heap.alloc_cons(TaggedValue::fixnum(i), rooted);
        }
        let rooted_head = rooted;
        heap.collect_exact(std::iter::once(rooted_head));
        assert!(
            heap.should_run_concurrent(),
            "the completed STW bootstrap must enable concurrent marking"
        );

        // Allocation churn after the bootstrap: garbage for the concurrent
        // cycle to reclaim.
        let mut unrooted = TaggedValue::fixnum(0);
        for i in 0..N {
            unrooted = heap.alloc_cons(TaggedValue::fixnum(1_000_000 + i), unrooted);
        }
        let _unrooted_head = unrooted;
        let before = heap.cons_live_count;

        // One full concurrent cycle, mirroring the driver's state machine:
        // start handshake -> GC thread marks -> STW termination -> deferred
        // sweep drained.
        heap.concurrent_begin();
        heap.seed_root(rooted_head);
        heap.launch_concurrent_mark();
        assert!(heap.concurrent_mark_running());
        while !heap.concurrent_mark_done() {
            std::thread::yield_now();
        }
        heap.join_concurrent_mark();
        heap.reseed_runtime_and_remembered_roots();
        heap.seed_root(rooted_head);
        let bytes_before = heap.live_bytes();
        heap.incremental_drain_all();
        heap.incremental_finish(bytes_before, std::time::Instant::now());
        heap.finish_incremental_sweep_now();

        // The unrooted churn was reclaimed...
        let after = heap.cons_live_count;
        assert!(
            after < before,
            "the concurrent cycle must reclaim garbage (before={before}, after={after})",
        );
        // ...and the rooted spine survives, fully readable.
        let mut node = rooted_head;
        let mut count = 0i64;
        while node.is_cons() {
            let car = unsafe { (*node.xcons_ptr()).load_car() };
            assert_eq!(
                car.0,
                TaggedValue::fixnum(N - 1 - count).0,
                "rooted car intact at index {count}",
            );
            node = unsafe { (*node.xcons_ptr()).load_cdr() };
            count += 1;
        }
        assert_eq!(count, N, "the whole rooted list survived the concurrent cycle");
    }

    /// Gap 3 drop safety: dropping a heap while the GC thread is still
    /// concurrently marking it must stop + join the GC thread before any
    /// storage it can read is freed (dump-less heaps now reach this state at
    /// every safe-point collection after bootstrap, e.g. a test Context
    /// dropped mid-mark).
    #[test]
    fn dropping_heap_mid_concurrent_mark_joins_gc_thread() {
        crate::test_utils::init_test_tracing();
        let mut heap = TaggedHeap::new();
        set_tagged_heap(&mut heap);

        // A long spine so the GC thread is genuinely still marking at drop.
        const N: i64 = 300_000;
        let mut list = TaggedValue::fixnum(0);
        for i in 0..N {
            list = heap.alloc_cons(TaggedValue::fixnum(i), list);
        }
        heap.concurrent_begin();
        heap.seed_root(list);
        heap.launch_concurrent_mark();
        assert!(heap.concurrent_mark_running());
        // Drop with the mark in flight; under TSAN/ASAN a missing join is a
        // use-after-free the sanitizer catches, and the join panics if the GC
        // thread is gone.
        drop(heap);
    }

    /// Workstream A path-collapse safety net (characterization): a forced
    /// `collect_exact` retains a rooted live cons graph and reclaims an unrooted
    /// one, INDEPENDENT of which internal path (concurrent / incremental /
    /// STW-full) runs it. This must keep passing as the incremental slicer + the
    /// `NEOVM_GC_CONCURRENT`/`NEOVM_GC_SATB` env flags are deleted in the collapse.
    #[test]
    fn collect_exact_retains_rooted_and_frees_unrooted() {
        crate::test_utils::init_test_tracing();
        let mut heap = TaggedHeap::new();
        set_tagged_heap(&mut heap);

        const N: i64 = 1_000;
        // Rooted list: rooted_head -> cons(N-1) -> ... -> cons(0) -> fixnum(0).
        let mut rooted = TaggedValue::fixnum(0);
        for i in 0..N {
            rooted = heap.alloc_cons(TaggedValue::fixnum(i), rooted);
        }
        let rooted_head = rooted;
        // Unrooted list (never named in the explicit root set): must be reclaimed.
        // A precise collector roots only the iterator passed to collect_exact, not
        // the Rust stack, so holding this local does NOT keep it alive.
        let mut unrooted = TaggedValue::fixnum(0);
        for i in 0..N {
            unrooted = heap.alloc_cons(TaggedValue::fixnum(1_000_000 + i), unrooted);
        }
        let _unrooted_head = unrooted;
        let before = heap.cons_live_count;

        // Force a full collection with only the rooted list reachable.
        heap.collect_exact(std::iter::once(rooted_head));
        let after = heap.cons_live_count;

        // The unrooted list was reclaimed...
        assert!(
            after < before,
            "unrooted conses must be reclaimed (before={before}, after={after})",
        );
        // ...and the entire rooted spine survives + is readable (a swept cons here
        // would be a use-after-free the asserts / sanitizer catch).
        let mut node = rooted_head;
        let mut count = 0i64;
        while node.is_cons() {
            let car = unsafe { (*node.xcons_ptr()).load_car() };
            assert_eq!(
                car.0,
                TaggedValue::fixnum(N - 1 - count).0,
                "rooted car intact at index {count}",
            );
            node = unsafe { (*node.xcons_ptr()).load_cdr() };
            count += 1;
        }
        assert_eq!(count, N, "the whole rooted list survived collection");
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

    /// Regression test for the O(n²) SATB blow-up: building a large container
    /// (here a hash table) in a loop WHILE a concurrent mark is running must log
    /// each container's pre-image to the SATB buffer at most ONCE per cycle, not
    /// re-enumerate ALL of the container's children on every single mutation.
    ///
    /// Before the per-cycle dedup fix, every `puthash` ran
    /// `push_value_children_to_satb_shared` -> `collect_veclike_children`, which
    /// enumerates `ht.data.values()` + `ht.key_snapshots.values()` — the WHOLE
    /// table. N inserts each snapshot ~k*N values => Θ(N²) entries pushed into
    /// `satb_shared` (and the equivalent memory), which OOMs on a 200K-entry
    /// build like `(ucs-names)`. The fix snapshots the table's full pre-image
    /// once, so the cumulative SATB volume is O(N).
    ///
    /// We drive the SATB barrier directly (set `concurrent_mark_running` without
    /// launching the background GC thread) so nothing drains `satb_shared`
    /// concurrently and the cumulative push count is deterministic.
    #[test]
    fn satb_barrier_on_growing_hash_table_is_linear_not_quadratic() {
        use crate::emacs_core::value::{HashTableTest, LispHashTable};

        crate::test_utils::init_test_tracing();
        let mut heap = TaggedHeap::new();
        set_tagged_heap(&mut heap);

        // An `equal` hash table whose VALUES are heap objects (conses), so the
        // SATB enumeration actually pushes them to the shared buffer.
        let table = heap.alloc_hash_table(LispHashTable::new(HashTableTest::Equal));

        // Arm the SATB barrier exactly as `launch_concurrent_mark` does, but
        // WITHOUT the GC thread, so `satb_shared` is never drained and its length
        // measures the cumulative SATB push volume deterministically.
        heap.concurrent_mark_running = true;
        TAGGED_HEAP_CONCURRENT_ACTIVE.with(|c| c.set(true));

        const N: i64 = 50_000;
        for i in 0..N {
            // Each value is a fresh heap cons (a brand-new key => an INSERT, no
            // prior value at that key for SATB to log).
            let value = heap.alloc_cons(TaggedValue::fixnum(i), TaggedValue::fixnum(0));
            let key = crate::emacs_core::value::HashKey::Int(i);
            let key_snapshot = TaggedValue::fixnum(i);
            crate::tagged::mutate::with_hash_table_mut(table, |ht| {
                ht.data.insert(key.clone(), value);
                ht.key_snapshots.insert(key.clone(), key_snapshot);
                ht.insertion_order.push(key.clone());
                ht.note_hash_key_inserted(key);
            });
        }

        let satb_len = heap.satb_shared.lock().unwrap().len();

        // Disarm before dropping the heap so no later mutation hits the barrier.
        heap.concurrent_mark_running = false;
        TAGGED_HEAP_CONCURRENT_ACTIVE.with(|c| c.set(false));

        // O(n) bound. The full pre-image is snapshotted at most a small constant
        // number of times across the whole cycle (ideally once), so the
        // cumulative pushes are within a small multiple of N. The buggy
        // (re-enumerate-on-every-write) barrier produces ~N²/2 ≈ 1.25e9 pushes
        // for N=50_000, blowing far past this bound.
        let bound = (N as usize) * 4;
        assert!(
            satb_len <= bound,
            "SATB barrier is super-linear: pushed {satb_len} values for {N} inserts \
             (O(n) bound is {bound}); the per-write full-container enumeration was \
             not deduplicated per cycle",
        );
    }

    /// End-to-end correctness for the per-cycle SATB dedup under a REAL concurrent
    /// mark + sweep: a hash table is mutated MANY times during marking (so the
    /// dedup suppresses all but the first per-owner snapshot), values are
    /// OVERWRITTEN (update) and the table is GROWN (insert+resize/rehash), and
    /// churn garbage is allocated and dropped. After termination + sweep:
    ///   * every value reachable through the live table survives and is readable;
    ///   * a value that was OVERWRITTEN before the snapshot-time first mutation is
    ///     retained by the SATB pre-image (Yuasa: it was live at snapshot time);
    ///   * unrooted pre-mark garbage is reclaimed.
    /// If the dedup ever dropped a still-reachable value's pre-image, the sweep
    /// would free a live cons and the readback would observe corruption (and TSan
    /// /ASan would fault). Mirrors `concurrent_mark_overlaps_mutation_and_retains_live_set`
    /// but exercises the deduped multi-child (hash-table) owner path specifically.
    #[test]
    fn concurrent_mark_dedup_retains_hash_table_live_set() {
        use crate::emacs_core::value::{HashKey, HashTableTest, LispHashTable};

        crate::test_utils::init_test_tracing();
        let mut heap = TaggedHeap::new();
        set_tagged_heap(&mut heap);

        // Build the table BEFORE the mark so its initial values are part of the
        // start-of-cycle snapshot. Each value is a heap cons we can read back.
        let table = heap.alloc_hash_table(LispHashTable::new(HashTableTest::Equal));
        const PRE: i64 = 2_000;
        for i in 0..PRE {
            let value = heap.alloc_cons(TaggedValue::fixnum(i), TaggedValue::fixnum(0));
            let key = HashKey::Int(i);
            crate::tagged::mutate::with_hash_table_mut(table, |ht| {
                ht.data.insert(key.clone(), value);
                ht.key_snapshots.insert(key.clone(), TaggedValue::fixnum(i));
                ht.insertion_order.push(key.clone());
                ht.note_hash_key_inserted(key);
            });
        }
        // Pre-mark garbage: reachable from nothing.
        let _garbage = heap.alloc_cons(TaggedValue::fixnum(-99), TaggedValue::fixnum(0));

        // Start a real concurrent mark with the table as the sole root.
        heap.concurrent_begin();
        heap.seed_root(table);
        heap.launch_concurrent_mark();

        // While the GC thread marks: (a) OVERWRITE an existing key's value — the
        // OLD cons leaves the table and must be retained via the SATB pre-image;
        // (b) GROW the table with many new keys (insert + resize/rehash), whose
        // values are born-black; (c) churn-allocate dropped garbage.
        let key0 = HashKey::Int(0);
        let old_value0 =
            crate::tagged::mutate::with_hash_table_mut(table, |ht| ht.data[&key0]).unwrap();
        let new_value0 = heap.alloc_cons(TaggedValue::fixnum(123_456), TaggedValue::fixnum(0));
        crate::tagged::mutate::with_hash_table_mut(table, |ht| {
            *ht.data.get_mut(&key0).unwrap() = new_value0;
        });
        for i in PRE..(PRE + 3_000) {
            let value = heap.alloc_cons(TaggedValue::fixnum(i), TaggedValue::fixnum(0));
            let key = HashKey::Int(i);
            crate::tagged::mutate::with_hash_table_mut(table, |ht| {
                maybe_resize_for_test(ht);
                ht.data.insert(key.clone(), value);
                ht.key_snapshots.insert(key.clone(), TaggedValue::fixnum(i));
                ht.insertion_order.push(key.clone());
                ht.note_hash_key_inserted(key);
            });
        }
        for _ in 0..5_000 {
            let _ = heap.alloc_cons(TaggedValue::fixnum(0), TaggedValue::fixnum(0));
        }

        // Terminate stop-the-world + sweep.
        while !heap.concurrent_mark_done() {
            std::thread::yield_now();
        }
        heap.join_concurrent_mark();
        heap.reseed_runtime_and_remembered_roots();
        heap.seed_root(table);
        let bytes_before = heap.live_bytes();
        heap.incremental_drain_all();
        heap.incremental_finish(bytes_before, std::time::Instant::now());
        heap.finish_incremental_sweep_now();

        // (1) The overwritten OLD value (live at snapshot time, then unlinked) must
        //     still be a readable, non-swept cons (SATB pre-image retained it).
        assert!(old_value0.is_cons());
        assert_eq!(
            unsafe { (*old_value0.xcons_ptr()).load_car() }.0,
            TaggedValue::fixnum(0).0,
            "overwritten pre-snapshot value was swept — dedup dropped a live pre-image",
        );
        // (2) Every value currently in the table is readable (none swept).
        let snapshot = table.with_hash_table_mut(|ht| {
            ht.data
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect::<Vec<_>>()
        });
        let entries = snapshot.expect("hash table");
        assert_eq!(entries.len() as i64, PRE + 3_000);
        for (key, value) in entries {
            assert!(
                value.is_cons(),
                "table value {key:?} is not a cons (swept?)"
            );
            let car = unsafe { (*value.xcons_ptr()).load_car() }.0;
            let expected = match key {
                HashKey::Int(0) => TaggedValue::fixnum(123_456).0, // the updated value
                HashKey::Int(n) => TaggedValue::fixnum(n).0,
                other => panic!("unexpected key {other:?}"),
            };
            assert_eq!(car, expected, "table value {key:?} corrupted/swept");
        }
    }

    /// GNU-parity finalizers, STW path: a finalizer a full collection finds
    /// unreachable leaves the registry, its function is queued + re-marked
    /// (transitively) so the sweep keeps it, and the finalizer object itself
    /// is swept. A queued-but-not-taken function survives later cycles via
    /// the runtime-root seeding.
    #[test]
    fn finalizer_doomed_on_stw_collection_queues_and_keeps_function() {
        crate::test_utils::init_test_tracing();
        let mut heap = TaggedHeap::new();
        set_tagged_heap(&mut heap);

        let payload = heap.alloc_cons(TaggedValue::fixnum(7), TaggedValue::fixnum(8));
        let function = heap.alloc_cons(TaggedValue::fixnum(42), payload);
        let finalizer = heap.alloc_finalizer(function);
        let fin_ptr = finalizer.as_veclike_ptr().unwrap();
        // The verifier enumeration must cover the function slot
        // (`collect_veclike_children` stays a superset of `trace_veclike`).
        let children = heap.collect_veclike_children(fin_ptr as *mut VecLikeHeader);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].0, function.0);

        // Cycle 1: the finalizer is rooted — still registered, nothing queued,
        // and the traced function survives.
        heap.begin_collection();
        heap.seed_root(finalizer);
        heap.complete_collection();
        assert!(heap.doomed_finalizer_functions.is_empty());
        assert_eq!(heap.finalizer_registry.len(), 1);
        assert_eq!(
            unsafe { (*function.xcons_ptr()).load_car() }.0,
            TaggedValue::fixnum(42).0,
        );

        // Cycle 2: nothing roots the finalizer — doomed. The function (and
        // what it reaches) survives the sweep; the finalizer object does not.
        heap.begin_collection();
        heap.complete_collection();
        assert!(heap.finalizer_registry.is_empty());
        assert!(
            !heap.owns_non_cons_object(fin_ptr as *const u8),
            "doomed finalizer object must be swept",
        );
        assert_eq!(
            unsafe { (*function.xcons_ptr()).load_car() }.0,
            TaggedValue::fixnum(42).0,
        );
        assert_eq!(
            unsafe { (*payload.xcons_ptr()).load_car() }.0,
            TaggedValue::fixnum(7).0,
            "everything the queued function reaches must survive",
        );

        // Cycle 3, queue still undrained: the queued function is a runtime
        // root and must survive again.
        heap.begin_collection();
        heap.complete_collection();
        assert_eq!(
            unsafe { (*function.xcons_ptr()).load_car() }.0,
            TaggedValue::fixnum(42).0,
        );

        let doomed = heap.take_doomed_finalizer_functions();
        assert_eq!(doomed.len(), 1);
        assert_eq!(doomed[0].0, function.0);
        assert!(heap.take_doomed_finalizer_functions().is_empty());
    }

    /// GNU-parity finalizers, concurrent path: the doomed-finalizer scan must
    /// run at `incremental_finish` too — a miss there means finalizers never
    /// run under the concurrent collector. Also checks allocate-black: a
    /// finalizer born during the mark survives that cycle and is doomable on
    /// the next one.
    #[test]
    fn finalizer_doomed_on_concurrent_termination_queues_and_keeps_function() {
        crate::test_utils::init_test_tracing();
        let mut heap = TaggedHeap::new();
        set_tagged_heap(&mut heap);

        // A long spine keeps the GC thread marking while the mutator runs.
        const N: i64 = 100_000;
        let mut list = TaggedValue::fixnum(0);
        for i in 0..N {
            list = heap.alloc_cons(TaggedValue::fixnum(i), list);
        }
        let function = heap.alloc_cons(TaggedValue::fixnum(43), TaggedValue::fixnum(0));
        let doomed_fin = heap.alloc_finalizer(function);
        let doomed_ptr = doomed_fin.as_veclike_ptr().unwrap();
        let live_fin = heap.alloc_finalizer(function);

        heap.concurrent_begin();
        heap.seed_root(list);
        heap.seed_root(live_fin); // doomed_fin is unreachable this cycle
        heap.launch_concurrent_mark();

        // Born during the mark: allocate-black, so it survives this cycle
        // even though nothing references it.
        let churn_function = heap.alloc_cons(TaggedValue::fixnum(44), TaggedValue::fixnum(0));
        let churn_fin = heap.alloc_finalizer(churn_function);
        let churn_ptr = churn_fin.as_veclike_ptr().unwrap();

        while !heap.concurrent_mark_done() {
            std::thread::yield_now();
        }
        heap.join_concurrent_mark();
        heap.reseed_runtime_and_remembered_roots();
        heap.seed_root(list);
        heap.seed_root(live_fin);
        let bytes_before = heap.live_bytes();
        heap.incremental_drain_all();
        heap.incremental_finish(bytes_before, std::time::Instant::now());
        heap.finish_incremental_sweep_now();

        assert!(
            !heap.owns_non_cons_object(doomed_ptr as *const u8),
            "doomed finalizer object must be swept",
        );
        assert!(heap.owns_non_cons_object(live_fin.as_veclike_ptr().unwrap() as *const u8));
        assert!(
            heap.owns_non_cons_object(churn_ptr as *const u8),
            "a finalizer born during the mark must survive that cycle",
        );
        assert_eq!(heap.finalizer_registry.len(), 2);
        let doomed = heap.take_doomed_finalizer_functions();
        assert_eq!(doomed.len(), 1);
        assert_eq!(doomed[0].0, function.0);
        assert_eq!(
            unsafe { (*function.xcons_ptr()).load_car() }.0,
            TaggedValue::fixnum(43).0,
        );

        // Next cycle: the born-black churn finalizer (still unreferenced) is
        // doomed now; the rooted one stays registered.
        heap.begin_collection();
        heap.seed_root(live_fin);
        heap.complete_collection();
        assert!(!heap.owns_non_cons_object(churn_ptr as *const u8));
        assert_eq!(heap.finalizer_registry.len(), 1);
        let doomed = heap.take_doomed_finalizer_functions();
        assert_eq!(doomed.len(), 1);
        assert_eq!(doomed[0].0, churn_function.0);
        assert_eq!(
            unsafe { (*churn_function.xcons_ptr()).load_car() }.0,
            TaggedValue::fixnum(44).0,
        );
    }

    /// The dump-partition + tricolor verifiers must accept the finalizer
    /// arms: a LIVE finalizer is enumerated through
    /// `collect_veclike_children`, and a doomed one's re-marked function must
    /// not present a black->white edge. The fake dump span only activates the
    /// partition; it maps no objects.
    #[test]
    fn finalizer_cycle_passes_partition_verifier() {
        crate::test_utils::init_test_tracing();
        unsafe { std::env::set_var("NEOVM_GC_VERIFY_PARTITION", "1") };
        let mut heap = TaggedHeap::new();
        set_tagged_heap(&mut heap);
        heap.extend_dump_span(4096, 16);

        // First partitioned cycle promotes survivors + blackens the (empty)
        // dump; verification gates arm on the cycles after it.
        heap.begin_collection();
        heap.complete_collection();
        assert!(heap.dump_blackened);

        let payload = heap.alloc_cons(TaggedValue::fixnum(5), TaggedValue::fixnum(6));
        let doomed_function = heap.alloc_cons(TaggedValue::fixnum(45), payload);
        let _doomed_fin = heap.alloc_finalizer(doomed_function);
        let live_function = heap.alloc_cons(TaggedValue::fixnum(46), TaggedValue::fixnum(0));
        let live_fin = heap.alloc_finalizer(live_function);

        // Verified cycle: `complete_collection` panics if the finalizer arms
        // break the partition/tricolor invariants.
        heap.begin_collection();
        heap.seed_root(live_fin);
        heap.complete_collection();

        let doomed = heap.take_doomed_finalizer_functions();
        assert_eq!(doomed.len(), 1);
        assert_eq!(doomed[0].0, doomed_function.0);
        assert_eq!(
            unsafe { (*payload.xcons_ptr()).load_car() }.0,
            TaggedValue::fixnum(5).0,
        );
        assert_eq!(heap.finalizer_registry.len(), 1);
        assert_eq!(
            unsafe { (*live_function.xcons_ptr()).load_car() }.0,
            TaggedValue::fixnum(46).0,
        );
    }
}

/// Test-only growth helper mirroring the production insert resize policy closely
/// enough to force rehashes during the concurrent-mark stress test.
#[cfg(test)]
fn maybe_resize_for_test(ht: &mut crate::emacs_core::value::LispHashTable) {
    let len = ht.data.len() as i64;
    if len >= ht.size {
        ht.size = if ht.size == 0 { 6 } else { ht.size * 2 };
        ht.data.reserve(ht.size as usize);
        ht.key_snapshots.reserve(ht.size as usize);
        ht.rebuild_iterable_hash_keys_from_data();
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
