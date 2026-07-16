//! Unified memory budget management for all media caches.
//!
//! Provides a shared memory budget across images, video frames, WebKit views,
//! and shader surfaces. Each cache reports its usage; eviction is coordinated
//! centrally.
//!
//! Wiring status: the render thread (`RenderApp`) owns the instance
//! (`NEOMACS_MEDIA_BUDGET_MB` overrides the default limit at init) and so
//! far registers **shader surfaces only** (create/free choke point in
//! `render_thread/asset_commands.rs`). That choke point also runs the
//! eviction driver: an over-budget SurfaceCreate consumes
//! [`MediaBudget::get_eviction_candidates`] and frees *recreatable*
//! (declarative-spec) surfaces until back under the limit. The
//! image/video/webkit caches do not report their usage yet, and nothing
//! calls [`MediaBudget::touch`] on draw (LRU decays to creation order) —
//! that parity work is outstanding.

use std::collections::BTreeMap;

/// Media type for priority ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MediaType {
    /// Static images (lowest priority - can be reloaded)
    Image,
    /// Video frames (medium priority - can be re-decoded)
    Video,
    /// WebKit surfaces (highest priority - expensive to recreate)
    WebKit,
    /// Shader surfaces (evicted last: imperative ids handed to Lisp cannot
    /// be recreated by the renderer; declarative specs re-resolve on the
    /// next redisplay walk. The budget itself cannot tell the two apart —
    /// `RenderApp::surface_recreatable` carries that bit per id and the
    /// eviction driver only frees the recreatable ones)
    Surface,
}

/// Entry in the media budget tracker
#[derive(Debug)]
pub struct BudgetEntry {
    pub media_type: MediaType,
    pub id: u32,
    pub size_bytes: usize,
    pub last_access: u64,
}

/// Unified media budget manager
pub struct MediaBudget {
    /// Maximum total memory in bytes (default: 256MB)
    max_memory: usize,
    /// Current total memory usage
    current_memory: usize,
    /// All tracked entries, ordered by (media_type, last_access)
    entries: BTreeMap<(MediaType, u64, u32), BudgetEntry>,
    /// Global access counter
    access_counter: u64,
}

impl MediaBudget {
    /// Create with default 256MB budget
    pub fn new() -> Self {
        Self::with_max_bytes(256 * 1024 * 1024)
    }

    /// Create with custom memory limit
    pub fn with_max_bytes(max_memory: usize) -> Self {
        Self {
            max_memory,
            current_memory: 0,
            entries: BTreeMap::new(),
            access_counter: 0,
        }
    }

    /// Register a new media item
    pub fn register(&mut self, media_type: MediaType, id: u32, size_bytes: usize) {
        self.access_counter += 1;
        let entry = BudgetEntry {
            media_type,
            id,
            size_bytes,
            last_access: self.access_counter,
        };
        self.entries
            .insert((media_type, self.access_counter, id), entry);
        self.current_memory += size_bytes;

        tracing::trace!(
            "MediaBudget: registered {:?}:{} ({}KB), total={}MB/{}MB",
            media_type,
            id,
            size_bytes / 1024,
            self.current_memory / (1024 * 1024),
            self.max_memory / (1024 * 1024)
        );
    }

    /// Unregister a media item
    pub fn unregister(&mut self, media_type: MediaType, id: u32) {
        let key = self
            .entries
            .iter()
            .find(|(_, e)| e.media_type == media_type && e.id == id)
            .map(|(k, _)| *k);

        if let Some(key) = key
            && let Some(entry) = self.entries.remove(&key)
        {
            self.current_memory = self.current_memory.saturating_sub(entry.size_bytes);
        }
    }

    /// Touch an entry (update last access time)
    pub fn touch(&mut self, media_type: MediaType, id: u32) {
        let old_key = self
            .entries
            .iter()
            .find(|(_, e)| e.media_type == media_type && e.id == id)
            .map(|(k, _)| *k);

        if let Some(old_key) = old_key
            && let Some(mut entry) = self.entries.remove(&old_key)
        {
            self.access_counter += 1;
            entry.last_access = self.access_counter;
            self.entries
                .insert((media_type, self.access_counter, id), entry);
        }
    }

    /// Get items to evict to make room for new_size bytes
    pub fn get_eviction_candidates(&self, new_size: usize) -> Vec<(MediaType, u32)> {
        let mut candidates = Vec::new();
        let target = self.current_memory + new_size;

        if target <= self.max_memory {
            return candidates;
        }

        let mut freed = 0usize;
        let need_to_free = target - self.max_memory;

        for ((media_type, _, id), entry) in &self.entries {
            if freed >= need_to_free {
                break;
            }
            candidates.push((*media_type, *id));
            freed += entry.size_bytes;
        }

        candidates
    }

    /// Check if we're over budget
    pub fn is_over_budget(&self) -> bool {
        self.current_memory > self.max_memory
    }

    /// Get current memory usage
    pub fn current_usage(&self) -> usize {
        self.current_memory
    }

    /// Get max memory limit
    pub fn max_limit(&self) -> usize {
        self.max_memory
    }
}

impl Default for MediaBudget {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "media_budget_test.rs"]
mod tests;
