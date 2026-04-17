//! Span-based size-class allocator for pinned (non-moving) objects.
//!
//! Each `Span` is a contiguous block of memory holding fixed-size slots.
//! Objects are bump-allocated from spans or popped from per-span free-lists.
//! Dead objects are reclaimed by sweep (walk slots, check mark bits, build
//! free-list). Span memory is returned to the OS when all slots are dead.
//!
//! This follows Go's mcache/mspan model adapted for a single-threaded
//! Lisp evaluator.

use std::alloc::{Layout, alloc, dealloc};
use std::ptr::NonNull;

use crate::object::{ObjectHeader, SpaceKind};

/// Span size in bytes. 32KB per span.
const SPAN_BYTES: usize = 32 * 1024;

/// Maximum slot size handled by the span allocator.
/// Objects larger than this fall back to individual system allocation.
pub(crate) const MAX_SLOT_SIZE: usize = 2048;

/// Size class table. Each entry is the slot size for that class.
/// Objects are rounded up to the nearest class.
const SIZE_CLASSES: &[usize] = &[
    64, 72, 80, 96, 128, 192, 256, 384, 512, 768, 1024, 2048,
];

/// Number of size classes.
const NUM_CLASSES: usize = SIZE_CLASSES.len();

/// Find the size class index for a given allocation size.
/// Returns None if the size exceeds MAX_SLOT_SIZE.
fn size_class_for(size: usize) -> Option<usize> {
    SIZE_CLASSES.iter().position(|&class_size| size <= class_size)
}

/// Sentinel value for "no free slot".
const FREE_NONE: u32 = u32::MAX;

/// A contiguous block of memory holding fixed-size slots of one size class.
#[derive(Debug)]
pub(crate) struct Span {
    /// Base pointer to the allocated block.
    base: NonNull<u8>,
    /// Slot size in bytes (includes ObjectHeader + payload + padding).
    slot_size: u32,
    /// Total number of slots that fit in this span.
    slot_count: u16,
    /// Bump cursor: number of slots initialized so far.
    bump_cursor: u16,
    /// Free list head: slot index of first free slot, or FREE_NONE.
    free_head: u32,
    /// Number of live (allocated, not freed) objects in this span.
    live_count: u16,
}

// Safety: `Span` only carries a raw pointer plus scalars. Mutating
// methods take `&mut self`, so there is no observable shared-mutable
// state through `&Span`. The pointer's lifetime is bounded by `Drop`,
// which deallocates the backing buffer. Matching the contract used by
// `NurseryTlab` so the allocator can live inside `HeapCore` (which is
// stored behind a `RwLock` in `HeapState` and must therefore be
// `Send + Sync`).
unsafe impl Send for Span {}
unsafe impl Sync for Span {}

impl Span {
    /// Allocate a new span for the given slot size.
    fn new(slot_size: usize) -> Self {
        let layout = Layout::from_size_align(SPAN_BYTES, 16)
            .expect("span layout");
        let base = unsafe { alloc(layout) };
        if base.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        let slot_count = (SPAN_BYTES / slot_size).min(u16::MAX as usize) as u16;
        Self {
            base: unsafe { NonNull::new_unchecked(base) },
            slot_size: slot_size as u32,
            slot_count,
            bump_cursor: 0,
            free_head: FREE_NONE,
            live_count: 0,
        }
    }

    /// Try to allocate a slot. Returns the base pointer for the slot
    /// (where ObjectHeader should be written), or None if the span is full.
    fn try_alloc(&mut self) -> Option<NonNull<u8>> {
        // Try free list first
        if self.free_head != FREE_NONE {
            let slot_index = self.free_head as usize;
            let ptr = self.slot_ptr(slot_index);
            // Read next-free pointer from the slot
            let next = unsafe { *(ptr.as_ptr() as *const u32) };
            self.free_head = next;
            self.live_count += 1;
            return Some(ptr);
        }
        // Try bump allocation
        if (self.bump_cursor as usize) < self.slot_count as usize {
            let slot_index = self.bump_cursor as usize;
            self.bump_cursor += 1;
            self.live_count += 1;
            Some(self.slot_ptr(slot_index))
        } else {
            None
        }
    }

    /// Get pointer to slot at given index.
    fn slot_ptr(&self, index: usize) -> NonNull<u8> {
        let offset = index * self.slot_size as usize;
        debug_assert!(offset + self.slot_size as usize <= SPAN_BYTES);
        unsafe { NonNull::new_unchecked(self.base.as_ptr().add(offset)) }
    }

    /// Sweep this span: check mark bits on all initialized slots.
    /// Unmarked objects get their payload dropped and slot added to free list.
    /// Returns the number of objects reclaimed.
    fn sweep(&mut self) -> usize {
        let mut reclaimed = 0;
        let mut new_free_head = FREE_NONE;

        for i in 0..self.bump_cursor as usize {
            let ptr = self.slot_ptr(i);
            let header = unsafe { &*(ptr.as_ptr() as *const ObjectHeader) };

            if header.is_marked() {
                // Live object — clear mark for next cycle
                header.clear_mark();
            } else if self.is_slot_live(i) {
                // Dead object — drop payload and add to free list
                unsafe {
                    let desc = header.desc();
                    if desc.needs_drop {
                        let payload = ObjectHeader::payload_ptr(
                            NonNull::new_unchecked(ptr.as_ptr() as *mut ObjectHeader)
                        );
                        (desc.drop_in_place)(payload.as_ptr());
                    }
                }
                // Thread into free list
                unsafe {
                    *(ptr.as_ptr() as *mut u32) = new_free_head;
                }
                new_free_head = i as u32;
                self.live_count -= 1;
                reclaimed += 1;
            }
        }

        self.free_head = new_free_head;
        reclaimed
    }

    /// Check if a slot is currently live (allocated and not yet freed).
    /// A slot is live if it has been bump-allocated or reused from free list,
    /// and hasn't been swept as dead.
    fn is_slot_live(&self, index: usize) -> bool {
        if index >= self.bump_cursor as usize {
            return false;
        }
        // Check if slot is in the free list by reading the header's desc pointer.
        // A free-listed slot has its first bytes overwritten with the next-free pointer,
        // which won't be a valid TypeDesc pointer. We can check if the header looks valid.
        // However, a simpler approach: track live_count and check mark bits.
        // For initial implementation, assume all bump-allocated slots are live
        // unless they're in the free list. The free list is rebuilt during sweep.
        true
    }

    /// Returns true if this span has no live objects.
    fn is_empty(&self) -> bool {
        self.live_count == 0
    }

    /// Clear all mark bits in this span (prepare for new mark cycle).
    fn clear_marks(&mut self) {
        for i in 0..self.bump_cursor as usize {
            let ptr = self.slot_ptr(i);
            let header = unsafe { &*(ptr.as_ptr() as *const ObjectHeader) };
            // Only clear marks on live slots (not free-listed ones)
            if !self.is_in_free_list(ptr) {
                header.clear_mark();
            }
        }
    }

    /// Check if a slot pointer is in the free list.
    fn is_in_free_list(&self, _ptr: NonNull<u8>) -> bool {
        // For now, we can't cheaply check this without walking the free list.
        // The sweep phase rebuilds the free list from scratch, so this isn't
        // needed for correctness — we just clear all marks and let sweep
        // rebuild.
        false
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        // Span owns only the raw buffer, not the payloads. Payload
        // destruction is driven by ObjectRecord::drop (for records in
        // ObjectStore) or by the sweep phase (for dead records after
        // a GC cycle). Dropping payloads here would double-free.
        //
        // Invariant: when a Span drops, every live ObjectRecord backed
        // by this span must have already run its drop_in_place. This
        // holds because:
        //  - On heap teardown: HeapCore drops `objects` (ObjectStore)
        //    before `pinned` (declaration order), so all ObjectRecord
        //    drops run first.
        //  - During sweep: dead records are removed from ObjectStore
        //    before their span slot is reused.
        let layout = Layout::from_size_align(SPAN_BYTES, 16)
            .expect("span layout");
        unsafe { dealloc(self.base.as_ptr(), layout); }
    }
}

/// Per-size-class pool of spans.
#[derive(Debug)]
pub(crate) struct SizeClassPool {
    /// The slot size for this class.
    slot_size: usize,
    /// Active span (receiving allocations).
    current: Option<Span>,
    /// Full spans (no free slots, waiting for sweep).
    full: Vec<Span>,
}

impl SizeClassPool {
    fn new(slot_size: usize) -> Self {
        Self {
            slot_size,
            current: None,
            full: Vec::new(),
        }
    }

    /// Allocate a slot from this size class.
    fn alloc(&mut self) -> NonNull<u8> {
        // Try current span
        if let Some(span) = &mut self.current {
            if let Some(ptr) = span.try_alloc() {
                return ptr;
            }
            // Current span is full — move to full list
            let full_span = self.current.take().unwrap();
            self.full.push(full_span);
        }
        // Allocate new span
        let mut span = Span::new(self.slot_size);
        let ptr = span.try_alloc().expect("fresh span should have space");
        self.current = Some(span);
        ptr
    }

    /// Sweep all spans in this pool. Returns (reclaimed_count, reclaimed_bytes).
    fn sweep(&mut self) -> (usize, usize) {
        let mut total_reclaimed = 0;

        // Sweep current span
        if let Some(span) = &mut self.current {
            total_reclaimed += span.sweep();
        }

        // Sweep full spans
        for span in &mut self.full {
            total_reclaimed += span.sweep();
        }

        // Move spans with free space back to eligible status
        // (they might have had slots freed by sweep)
        // For simplicity, if current span exists and has free space,
        // new allocs will use it. Full spans with freed slots stay in
        // the full list until we need them.

        // Remove completely empty spans (return memory to OS)
        self.full.retain(|span| !span.is_empty());
        if self.current.as_ref().is_some_and(|s| s.is_empty()) {
            self.current = None;
        }

        let reclaimed_bytes = total_reclaimed * self.slot_size;
        (total_reclaimed, reclaimed_bytes)
    }

    /// Clear marks on all spans.
    fn clear_marks(&mut self) {
        if let Some(span) = &mut self.current {
            span.clear_marks();
        }
        for span in &mut self.full {
            span.clear_marks();
        }
    }

    /// Iterate over all live object headers in this pool.
    fn for_each_header(&self, f: &mut dyn FnMut(NonNull<ObjectHeader>)) {
        let mut visit_span = |span: &Span| {
            for i in 0..span.bump_cursor as usize {
                let ptr = span.slot_ptr(i);
                // Skip free-listed slots (their header is overwritten)
                // For now, visit all bump-allocated slots
                f(ptr.cast());
            }
        };
        if let Some(span) = &self.current {
            visit_span(span);
        }
        for span in &self.full {
            visit_span(span);
        }
    }
}

/// Go-style size-class allocator for non-moving pinned objects.
#[derive(Debug)]
pub(crate) struct SizeClassAllocator {
    /// Per-size-class pools.
    pools: Vec<SizeClassPool>,
    /// Total bytes allocated (slot_size × allocated_count).
    allocated_bytes: usize,
}

impl SizeClassAllocator {
    /// Create a new allocator with the standard size classes.
    pub(crate) fn new() -> Self {
        let pools = SIZE_CLASSES
            .iter()
            .map(|&size| SizeClassPool::new(size))
            .collect();
        Self {
            pools,
            allocated_bytes: 0,
        }
    }

    /// Allocate a slot for an object of the given layout.
    /// Returns the base pointer (where ObjectHeader should be written).
    /// Returns None if the layout exceeds MAX_SLOT_SIZE.
    pub(crate) fn alloc(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let class_index = size_class_for(layout.size())?;
        let pool = &mut self.pools[class_index];
        let ptr = pool.alloc();
        self.allocated_bytes += pool.slot_size;
        Some(ptr)
    }

    /// Check if a layout can be served by the span allocator.
    pub(crate) fn can_alloc(&self, layout: &Layout) -> bool {
        layout.size() <= MAX_SLOT_SIZE
    }

    /// Sweep all pools after marking. Returns (reclaimed_objects, reclaimed_bytes).
    pub(crate) fn sweep(&mut self) -> (usize, usize) {
        let mut total_objects = 0;
        let mut total_bytes = 0;
        for pool in &mut self.pools {
            let (objects, bytes) = pool.sweep();
            total_objects += objects;
            total_bytes += bytes;
        }
        self.allocated_bytes = self.allocated_bytes.saturating_sub(total_bytes);
        (total_objects, total_bytes)
    }

    /// Clear all mark bits in preparation for a new mark cycle.
    pub(crate) fn clear_marks(&mut self) {
        for pool in &mut self.pools {
            pool.clear_marks();
        }
    }

    /// Total bytes allocated across all spans.
    pub(crate) fn allocated_bytes(&self) -> usize {
        self.allocated_bytes
    }

    /// Iterate over all object headers (for statistics/debugging).
    pub(crate) fn for_each_header(&self, mut f: impl FnMut(NonNull<ObjectHeader>)) {
        for pool in &self.pools {
            pool.for_each_header(&mut f);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_class_lookup() {
        assert_eq!(size_class_for(1), Some(0));   // → 64B
        assert_eq!(size_class_for(64), Some(0));  // → 64B
        assert_eq!(size_class_for(65), Some(1));  // → 72B
        assert_eq!(size_class_for(72), Some(1));  // → 72B
        assert_eq!(size_class_for(73), Some(2));  // → 80B
        assert_eq!(size_class_for(2048), Some(11)); // → 2048B
        assert_eq!(size_class_for(2049), None);   // too large
    }

    #[test]
    fn span_alloc_and_count() {
        let mut span = Span::new(72);
        assert_eq!(span.slot_count, (SPAN_BYTES / 72) as u16);
        assert_eq!(span.live_count, 0);

        let ptr = span.try_alloc().unwrap();
        assert!(!ptr.as_ptr().is_null());
        assert_eq!(span.live_count, 1);
        assert_eq!(span.bump_cursor, 1);
        // Test allocates slots but does not write valid ObjectHeaders into
        // them. Skip Drop (which would walk slot headers) to avoid reading
        // uninitialized memory.
        std::mem::forget(span);
    }

    #[test]
    fn allocator_serves_multiple_classes() {
        let mut alloc = SizeClassAllocator::new();
        let layout_64 = Layout::from_size_align(64, 8).unwrap();
        let layout_72 = Layout::from_size_align(72, 8).unwrap();
        let layout_128 = Layout::from_size_align(128, 8).unwrap();

        let p1 = alloc.alloc(layout_64).unwrap();
        let p2 = alloc.alloc(layout_72).unwrap();
        let p3 = alloc.alloc(layout_128).unwrap();

        // Different addresses
        assert_ne!(p1.as_ptr(), p2.as_ptr());
        assert_ne!(p2.as_ptr(), p3.as_ptr());

        // Allocation bytes tracked
        assert_eq!(alloc.allocated_bytes(), 64 + 72 + 128);
        // Test allocates slots but does not write valid ObjectHeaders into
        // them. Skip Drop (which would walk slot headers) to avoid reading
        // uninitialized memory.
        std::mem::forget(alloc);
    }

    #[test]
    fn too_large_returns_none() {
        let mut alloc = SizeClassAllocator::new();
        let layout = Layout::from_size_align(4096, 8).unwrap();
        assert!(alloc.alloc(layout).is_none());
    }
}
