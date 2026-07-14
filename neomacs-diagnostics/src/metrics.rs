//! The metrics data model exposed over `/metrics` and `/live`.
//!
//! Field names are the JSON API contract — later phases and external consumers
//! (agents, Perfetto, dashboards) depend on them exactly.

use serde::Serialize;

/// A point-in-time snapshot of neomacs performance metrics.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct MetricsSnapshot {
    pub frame: FrameMetrics,
    pub gc: GcMetrics,
}

/// Render / frame-scheduling counters. Mirrors the display runtime's
/// `FrameSchedSnapshot`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct FrameMetrics {
    pub presents: u64,
    pub scene_commits: u64,
    pub wakeups: u64,
    /// Latency from scene commit to present for the last frame (microseconds).
    pub last_commit_to_present_us: u64,
    /// Worst commit-to-present latency observed (microseconds).
    pub max_commit_to_present_us: u64,
    pub composite_only_frames: u64,
    pub retained_static_builds: u64,
}

/// Lisp GC / heap counters. Mirrors the published `GcStatsSnapshot`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct GcMetrics {
    pub collections: u64,
    pub live_bytes: u64,
    pub total_allocated_bytes: u64,
    pub cons_cells: u64,
    pub strings: u64,
    pub vector_cells: u64,
}
