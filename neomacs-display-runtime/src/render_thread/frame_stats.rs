//! Frame-scheduling evidence counters (Stage 0 of the frame scheduling plan,
//! docs/plans/2026-07-11-cross-platform-frame-scheduling-and-animation-architecture.md).
//!
//! Process-wide monotonic counters incremented from the render thread's
//! scheduling and presentation hot points. They change no policy; they exist
//! so scheduling changes can be judged by structural evidence (wakeups,
//! requests, commits, glyph-build passes, presents, commit-to-present
//! latency) instead of by CPU percentages alone.
//!
//! All counters are relaxed atomics: every writer runs on the render thread,
//! and readers only need eventually-consistent totals.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// One event-loop iteration (about_to_wait or user-event wake).
pub(super) static EVENT_LOOP_WAKEUPS: AtomicU64 = AtomicU64::new(0);
/// Redraw requests issued to winit windows.
pub(super) static REDRAW_REQUESTS: AtomicU64 = AtomicU64::new(0);
/// RedrawRequested events received from winit.
pub(super) static REDRAW_EVENTS: AtomicU64 = AtomicU64::new(0);
/// Editor scene commits ingested from the evaluator channel.
pub(super) static SCENE_COMMITS: AtomicU64 = AtomicU64::new(0);
/// Root glyph build/render passes (the expensive static-scene path).
pub(super) static ROOT_GLYPH_PASSES: AtomicU64 = AtomicU64::new(0);
/// Surface presents on top-level frame windows.
pub(super) static SURFACE_PRESENTS: AtomicU64 = AtomicU64::new(0);
/// Frame plans by render work class (frame scheduling plan, Stage 2).
pub(super) static PLAN_NONE: AtomicU64 = AtomicU64::new(0);
pub(super) static PLAN_COMPOSITE_ONLY: AtomicU64 = AtomicU64::new(0);
pub(super) static PLAN_REPAINT_LAYERS: AtomicU64 = AtomicU64::new(0);
pub(super) static PLAN_REBUILD_SCENE: AtomicU64 = AtomicU64::new(0);
/// Microseconds from the most recently consumed scene commit to its present.
pub(super) static LAST_COMMIT_TO_PRESENT_US: AtomicU64 = AtomicU64::new(0);
/// Worst observed commit-to-present latency in microseconds.
pub(super) static MAX_COMMIT_TO_PRESENT_US: AtomicU64 = AtomicU64::new(0);

/// Monotonic anchor for converting `Instant`s to storable microsecond ticks.
static EPOCH: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
/// Microsecond tick (vs EPOCH) of the oldest scene commit not yet presented;
/// 0 = none pending. Approximate under multiple windows, which is acceptable
/// for Stage 0 evidence.
static PENDING_COMMIT_TICK_US: AtomicU64 = AtomicU64::new(0);
/// Microsecond tick of the last periodic snapshot log.
static LAST_LOG_TICK_US: AtomicU64 = AtomicU64::new(0);
/// Wakeup total at the last periodic snapshot log.
static LAST_LOG_WAKEUPS: AtomicU64 = AtomicU64::new(0);

fn tick_us(now: Instant) -> u64 {
    let epoch = *EPOCH.get_or_init(|| now);
    // Saturates at 0 for the first caller; monotonic afterwards. +1 keeps a
    // real tick from colliding with the 0 = "none pending" sentinel.
    now.saturating_duration_since(epoch).as_micros() as u64 + 1
}

pub(super) fn count(counter: &AtomicU64) {
    counter.fetch_add(1, Ordering::Relaxed);
}

/// Record a frame plan's render work class.
pub(super) fn count_plan(work: &super::frame_sched::RenderWork) {
    use super::frame_sched::RenderWork;
    let counter = match work {
        RenderWork::None => &PLAN_NONE,
        RenderWork::CompositeOnly { .. } => &PLAN_COMPOSITE_ONLY,
        RenderWork::RepaintLayers { .. } => &PLAN_REPAINT_LAYERS,
        RenderWork::RebuildScene => &PLAN_REBUILD_SCENE,
    };
    count(counter);
}

/// Record that a scene commit arrived; starts the commit-to-present clock if
/// no earlier commit is still waiting to reach the screen.
pub(super) fn note_scene_commit(now: Instant) {
    count(&SCENE_COMMITS);
    let tick = tick_us(now);
    let _ = PENDING_COMMIT_TICK_US.compare_exchange(0, tick, Ordering::Relaxed, Ordering::Relaxed);
}

/// Record a top-level present; closes the commit-to-present measurement when
/// a commit is pending.
pub(super) fn note_present(now: Instant) {
    count(&SURFACE_PRESENTS);
    let pending = PENDING_COMMIT_TICK_US.swap(0, Ordering::Relaxed);
    if pending != 0 {
        let latency_us = tick_us(now).saturating_sub(pending);
        LAST_COMMIT_TO_PRESENT_US.store(latency_us, Ordering::Relaxed);
        MAX_COMMIT_TO_PRESENT_US.fetch_max(latency_us, Ordering::Relaxed);
    }
}

/// Point-in-time copy of every counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FrameSchedSnapshot {
    pub wakeups: u64,
    pub redraw_requests: u64,
    pub redraw_events: u64,
    pub scene_commits: u64,
    pub root_glyph_passes: u64,
    pub presents: u64,
    pub plan_none: u64,
    pub plan_composite_only: u64,
    pub plan_repaint_layers: u64,
    pub plan_rebuild_scene: u64,
    pub last_commit_to_present_us: u64,
    pub max_commit_to_present_us: u64,
}

pub(crate) fn snapshot() -> FrameSchedSnapshot {
    FrameSchedSnapshot {
        wakeups: EVENT_LOOP_WAKEUPS.load(Ordering::Relaxed),
        redraw_requests: REDRAW_REQUESTS.load(Ordering::Relaxed),
        redraw_events: REDRAW_EVENTS.load(Ordering::Relaxed),
        scene_commits: SCENE_COMMITS.load(Ordering::Relaxed),
        root_glyph_passes: ROOT_GLYPH_PASSES.load(Ordering::Relaxed),
        presents: SURFACE_PRESENTS.load(Ordering::Relaxed),
        plan_none: PLAN_NONE.load(Ordering::Relaxed),
        plan_composite_only: PLAN_COMPOSITE_ONLY.load(Ordering::Relaxed),
        plan_repaint_layers: PLAN_REPAINT_LAYERS.load(Ordering::Relaxed),
        plan_rebuild_scene: PLAN_REBUILD_SCENE.load(Ordering::Relaxed),
        last_commit_to_present_us: LAST_COMMIT_TO_PRESENT_US.load(Ordering::Relaxed),
        max_commit_to_present_us: MAX_COMMIT_TO_PRESENT_US.load(Ordering::Relaxed),
    }
}

const LOG_INTERVAL_US: u64 = 5_000_000;

/// Log a snapshot at debug level at most every 5 seconds, and only when the
/// loop actually woke since the previous log. Called from about_to_wait, so a
/// fully idle loop logs nothing at all.
pub(super) fn maybe_log_snapshot(now: Instant) {
    let tick = tick_us(now);
    let last = LAST_LOG_TICK_US.load(Ordering::Relaxed);
    if tick.saturating_sub(last) < LOG_INTERVAL_US {
        return;
    }
    if LAST_LOG_TICK_US
        .compare_exchange(last, tick, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        return;
    }
    let snap = snapshot();
    let last_wakeups = LAST_LOG_WAKEUPS.swap(snap.wakeups, Ordering::Relaxed);
    if snap.wakeups == last_wakeups {
        return;
    }
    let elapsed_s = tick.saturating_sub(last) as f64 / 1_000_000.0;
    tracing::debug!(
        wakeups = snap.wakeups,
        wakeups_per_s = format!("{:.1}", (snap.wakeups - last_wakeups) as f64 / elapsed_s),
        redraw_requests = snap.redraw_requests,
        redraw_events = snap.redraw_events,
        scene_commits = snap.scene_commits,
        root_glyph_passes = snap.root_glyph_passes,
        presents = snap.presents,
        plan_none = snap.plan_none,
        plan_composite_only = snap.plan_composite_only,
        plan_repaint_layers = snap.plan_repaint_layers,
        plan_rebuild_scene = snap.plan_rebuild_scene,
        last_commit_to_present_us = snap.last_commit_to_present_us,
        max_commit_to_present_us = snap.max_commit_to_present_us,
        "frame_sched_stats"
    );
}
