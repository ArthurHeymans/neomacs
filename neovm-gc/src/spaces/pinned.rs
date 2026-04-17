use crate::spaces::pinned_span::SizeClassAllocator;

/// Pinned-space configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PinnedSpaceConfig {
    /// Initial pinned-space capacity.
    pub reserved_bytes: usize,
}

impl Default for PinnedSpaceConfig {
    fn default() -> Self {
        Self {
            reserved_bytes: 8 * 1024 * 1024,
        }
    }
}

/// Runtime state for the pinned (non-moving) space.
///
/// Holds the span-based size-class allocator that serves objects
/// up to [`crate::spaces::pinned_span::MAX_SLOT_SIZE`] bytes.
/// Larger objects fall back to the system allocator via
/// `ObjectRecord::allocate`.
#[derive(Debug)]
pub(crate) struct PinnedSpaceState {
    allocator: SizeClassAllocator,
}

impl PinnedSpaceState {
    pub(crate) fn new() -> Self {
        Self {
            allocator: SizeClassAllocator::new(),
        }
    }

    pub(crate) fn allocator_mut(&mut self) -> &mut SizeClassAllocator {
        &mut self.allocator
    }

    pub(crate) fn allocator(&self) -> &SizeClassAllocator {
        &self.allocator
    }

    /// Bytes currently allocated across all span pools.
    pub(crate) fn allocated_bytes(&self) -> usize {
        self.allocator.allocated_bytes()
    }
}

impl Default for PinnedSpaceState {
    fn default() -> Self {
        Self::new()
    }
}
