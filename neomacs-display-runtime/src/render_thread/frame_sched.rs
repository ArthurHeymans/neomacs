//! Pure frame scheduling: typed frame demand, per-window coalescing, and
//! pacing decisions.
//!
//! Stage 1 of the frame scheduling plan
//! (docs/plans/2026-07-11-cross-platform-frame-scheduling-and-animation-architecture.md).
//!
//! This module has no winit, wgpu, or wall-clock dependency. Every method
//! takes `now` (or a [`FrameTick`]) explicitly, so scheduling decisions are
//! deterministic under test: tests anchor one real `Instant` and derive all
//! other times by `Duration` arithmetic.
//!
//! Semantics:
//! - Demand is declared before drawing. Rendering consumes a [`FramePlan`];
//!   it never latches "render me again" as a side effect.
//! - At most one redraw request is outstanding per native window; duplicate
//!   driving demands coalesce into that request.
//! - Deadline demands ([`Cadence::At`], [`Cadence::MaxRate`]) are keyed by
//!   [`DemandReason`]: resubmitting replaces the previous entry, so a caller
//!   can declare its standing demand every pass without accumulation, and
//!   [`FrameCoordinator::retract`] withdraws a reason that no longer applies.
//! - [`Cadence::MaxRate`] keeps a per-reason phase anchor: consuming a tick
//!   advances the anchor by whole periods, so an interleaved one-shot frame
//!   (e.g. an editor commit) never re-anchors an ambient cadence.
//! - Ineligible (occluded/hidden) windows retain demand but are never asked
//!   to present; regaining eligibility issues exactly one recovery request.

use std::collections::BTreeMap;
use std::num::NonZeroU16;
use std::time::{Duration, Instant};

bitflags::bitflags! {
    /// Broad retained composition groups. Deliberately coarse: one bit per
    /// retained group, not one bit per effect.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) struct LayerMask: u32 {
        const ROOT_CONTENT = 1 << 0;
        const CHILD_FRAMES = 1 << 1;
        const CURSOR_EFFECTS = 1 << 2;
        const TRANSIENT_OVERLAYS = 1 << 3;
        const CHROME = 1 << 4;
        const MEDIA = 1 << 5;
        const TRANSITIONS = 1 << 6;
    }
}

/// Damage granularity within a repainted layer. Begins as full-layer only;
/// rectangle lists arrive with retained-layer work. The interface carries
/// damage now so that full-layer repaint never hardens into an implicit
/// invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Damage {
    FullLayer,
}

impl Damage {
    fn combine(self, _other: Damage) -> Damage {
        Damage::FullLayer
    }
}

/// The least expensive category of work capable of producing correct pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Invalidation {
    #[default]
    None,
    /// Recompose existing layer content; sample dynamic state only.
    CompositeOnly { layers: LayerMask },
    /// Repaint the named layers, then compose.
    RepaintLayers { layers: LayerMask, damage: Damage },
    /// A new editor scene generation: rebuild static content.
    RebuildScene,
}

impl Invalidation {
    /// Strongest-wins merge. Equal-strength classes union their layers; a
    /// stronger class absorbs a weaker one because every presented frame
    /// composes all layers anyway.
    pub(crate) fn combine(self, other: Invalidation) -> Invalidation {
        use Invalidation::*;
        match (self, other) {
            (None, x) | (x, None) => x,
            (RebuildScene, _) | (_, RebuildScene) => RebuildScene,
            (
                RepaintLayers {
                    layers: a,
                    damage: da,
                },
                RepaintLayers {
                    layers: b,
                    damage: db,
                },
            ) => RepaintLayers {
                layers: a | b,
                damage: da.combine(db),
            },
            (r @ RepaintLayers { .. }, CompositeOnly { .. })
            | (CompositeOnly { .. }, r @ RepaintLayers { .. }) => r,
            (CompositeOnly { layers: a }, CompositeOnly { layers: b }) => {
                CompositeOnly { layers: a | b }
            }
        }
    }
}

/// When the demanded work should reach the screen.
// Interface variants/fields defined by the scheduling plan; consumed as
// later stages migrate effects onto the coordinator.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cadence {
    /// Fold into whatever frame happens next; never forces a frame.
    OnDemand,
    /// Present as soon as the presentation clock allows.
    NextPresentation,
    /// At most this many frames per second, phase-anchored per reason.
    MaxRate(NonZeroU16),
    /// At a specific deadline (blink timers, scheduled recovery).
    At(Instant),
}

/// Why a frame is wanted. Diagnostic identity, not policy encoded as strings.
/// Deadline demands are keyed by this, so each reason holds at most one
/// scheduled deadline per window.
// Interface variants/fields defined by the scheduling plan; consumed as
// later stages migrate effects onto the coordinator.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DemandReason {
    EditorCommit,
    CursorAnimation,
    /// Infinite ambient compositor-only demand: the cursor color cycle
    /// (Stage 3 tracer bullet). Distinct from CursorAnimation so its MaxRate
    /// phase anchor cannot collide with the blink deadline.
    CursorColorCycle,
    FiniteEffect,
    Transition,
    Video,
    WebKit,
    /// Animated shader surfaces visible in a composited frame
    /// (doc/display-engine/SHADER_SURFACES.md).
    ShaderSurface,
    Terminal,
    Expose,
    DebugCapture,
    /// New editor content or blink toggle needing a repaint.
    Redisplay,
    /// Render-effect families (Stage 6). Each names the group animating so
    /// diagnostics can answer "why is this window still rendering?" without
    /// per-effect logging.
    CursorEffect,
    WindowEffect,
    TextEffect,
    ScrollEffect,
    DecorativeEffect,
    TransientEffect,
}

impl DemandReason {
    /// Every reason, in declaration order. The order is the counter/report
    /// order and matches the derived `Ord`.
    pub(crate) const ALL: [DemandReason; Self::COUNT] = [
        DemandReason::EditorCommit,
        DemandReason::CursorAnimation,
        DemandReason::CursorColorCycle,
        DemandReason::FiniteEffect,
        DemandReason::Transition,
        DemandReason::Video,
        DemandReason::WebKit,
        DemandReason::ShaderSurface,
        DemandReason::Terminal,
        DemandReason::Expose,
        DemandReason::DebugCapture,
        DemandReason::Redisplay,
        DemandReason::CursorEffect,
        DemandReason::WindowEffect,
        DemandReason::TextEffect,
        DemandReason::ScrollEffect,
        DemandReason::DecorativeEffect,
        DemandReason::TransientEffect,
    ];

    /// Number of reasons: the width of [`DemandReason::ALL`] and of every
    /// per-reason counter array.
    pub(crate) const COUNT: usize = 18;

    /// Index into [`DemandReason::ALL`] / the per-reason counter arrays. The
    /// exhaustive match makes a new variant a compile error here; the dense
    /// 0..COUNT range is pinned by `demand_reason_indices_are_dense`.
    pub(crate) const fn index(self) -> usize {
        match self {
            DemandReason::EditorCommit => 0,
            DemandReason::CursorAnimation => 1,
            DemandReason::CursorColorCycle => 2,
            DemandReason::FiniteEffect => 3,
            DemandReason::Transition => 4,
            DemandReason::Video => 5,
            DemandReason::WebKit => 6,
            DemandReason::ShaderSurface => 7,
            DemandReason::Terminal => 8,
            DemandReason::Expose => 9,
            DemandReason::DebugCapture => 10,
            DemandReason::Redisplay => 11,
            DemandReason::CursorEffect => 12,
            DemandReason::WindowEffect => 13,
            DemandReason::TextEffect => 14,
            DemandReason::ScrollEffect => 15,
            DemandReason::DecorativeEffect => 16,
            DemandReason::TransientEffect => 17,
        }
    }

    /// Stable snake_case name for diagnostics output.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            DemandReason::EditorCommit => "editor_commit",
            DemandReason::CursorAnimation => "cursor_animation",
            DemandReason::CursorColorCycle => "cursor_color_cycle",
            DemandReason::FiniteEffect => "finite_effect",
            DemandReason::Transition => "transition",
            DemandReason::Video => "video",
            DemandReason::WebKit => "webkit",
            DemandReason::ShaderSurface => "shader_surface",
            DemandReason::Terminal => "terminal",
            DemandReason::Expose => "expose",
            DemandReason::DebugCapture => "debug_capture",
            DemandReason::Redisplay => "redisplay",
            DemandReason::CursorEffect => "cursor_effect",
            DemandReason::WindowEffect => "window_effect",
            DemandReason::TextEffect => "text_effect",
            DemandReason::ScrollEffect => "scroll_effect",
            DemandReason::DecorativeEffect => "decorative_effect",
            DemandReason::TransientEffect => "transient_effect",
        }
    }
}

/// Set of [`DemandReason`]s, carried by value on a [`FramePlan`] so a frame can
/// be attributed to what asked for it ("why did this present happen?").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct DemandReasonSet(u32);

impl DemandReasonSet {
    pub(crate) const fn empty() -> Self {
        DemandReasonSet(0)
    }

    fn insert(&mut self, reason: DemandReason) {
        self.0 |= 1 << reason.index();
    }

    pub(crate) fn contains(self, reason: DemandReason) -> bool {
        self.0 & (1 << reason.index()) != 0
    }

    pub(crate) fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Reasons in [`DemandReason::ALL`] order.
    pub(crate) fn iter(self) -> impl Iterator<Item = DemandReason> {
        DemandReason::ALL
            .into_iter()
            .filter(move |r| self.contains(*r))
    }
}

impl FromIterator<DemandReason> for DemandReasonSet {
    fn from_iter<I: IntoIterator<Item = DemandReason>>(iter: I) -> Self {
        let mut set = DemandReasonSet::empty();
        for reason in iter {
            set.insert(reason);
        }
        set
    }
}

/// A declaration that pixels need to change, with reason, scope, and cadence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FrameDemand {
    pub invalidation: Invalidation,
    pub cadence: Cadence,
    pub reason: DemandReason,
}

// Interface variants/fields defined by the scheduling plan; consumed as
// later stages migrate effects onto the coordinator.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClockSource {
    Native,
    Synthetic,
}

/// One opportunity to produce a frame: timing input, not an instruction to
/// rebuild editor state.
// Interface variants/fields defined by the scheduling plan; consumed as
// later stages migrate effects onto the coordinator.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct FrameTick {
    pub frame_time: Instant,
    pub target_presentation_time: Instant,
    pub estimated_interval: Duration,
    pub source: ClockSource,
}

/// The scheduler's decision about what work one tick performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RenderWork {
    None,
    CompositeOnly { layers: LayerMask },
    RepaintLayers { layers: LayerMask, damage: Damage },
    RebuildScene,
}

impl RenderWork {
    fn from_invalidation(inv: Invalidation) -> RenderWork {
        match inv {
            Invalidation::None => RenderWork::None,
            Invalidation::CompositeOnly { layers } => RenderWork::CompositeOnly { layers },
            Invalidation::RepaintLayers { layers, damage } => {
                RenderWork::RepaintLayers { layers, damage }
            }
            Invalidation::RebuildScene => RenderWork::RebuildScene,
        }
    }

    fn to_invalidation(self) -> Invalidation {
        match self {
            RenderWork::None => Invalidation::None,
            RenderWork::CompositeOnly { layers } => Invalidation::CompositeOnly { layers },
            RenderWork::RepaintLayers { layers, damage } => {
                Invalidation::RepaintLayers { layers, damage }
            }
            RenderWork::RebuildScene => Invalidation::RebuildScene,
        }
    }
}

/// Pure decision for one tick of one window.
// Interface variants/fields defined by the scheduling plan; consumed as
// later stages migrate effects onto the coordinator.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct FramePlan {
    pub tick: FrameTick,
    pub work: RenderWork,
    pub should_present: bool,
    /// Which demands this frame satisfies. Attribution only — the work class
    /// already encodes what must be drawn.
    pub reasons: DemandReasonSet,
}

/// What the caller should do next for this window. The event loop executes
/// these through a narrow winit adapter; it never derives `ControlFlow` from
/// individual effect fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PacingAction {
    /// Nothing to schedule for this window.
    Sleep,
    /// Ask the platform for one redraw of this window.
    RequestRedraw,
    /// Arm a wake at this deadline (the loop aggregates the earliest).
    WakeAt(Instant),
}

/// Presentation outcome, fed back as scheduling input.
// Interface variants/fields defined by the scheduling plan; consumed as
// later stages migrate effects onto the coordinator.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PresentResult {
    Presented,
    /// Nothing was rendered (no surface yet, warm-up, etc.); the plan's work
    /// was not shown and is re-queued.
    Skipped,
    Occluded,
    SurfaceLost,
    Timeout,
}

/// Visibility/focus state relevant to presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WindowPresentationState {
    pub visible: bool,
    pub occluded: bool,
    pub focused: bool,
}

impl Default for WindowPresentationState {
    fn default() -> Self {
        Self {
            visible: true,
            occluded: false,
            focused: true,
        }
    }
}

/// A native top-level window with its own surface and presentation
/// lifecycle. Child Emacs frames composite into a parent and share its
/// clock; callers map child demand to the parent id before submitting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct NativeWindowId(pub u64);

/// Bounded backoff after a surface acquisition timeout (invariant: surface
/// failure cannot produce an immediate retry storm).
const TIMEOUT_BACKOFF: Duration = Duration::from_millis(50);

#[derive(Debug)]
struct ScheduledDemand {
    reason: DemandReason,
    at: Instant,
    invalidation: Invalidation,
    /// MaxRate period for phase-anchored rescheduling; None for At().
    period: Option<Duration>,
}

#[derive(Debug, Default)]
struct DueDemand {
    invalidation: Invalidation,
    /// Whether the due work by itself justifies requesting a frame.
    /// OnDemand contributions record work without driving.
    driving: bool,
    reasons: DemandReasonSet,
}

impl DueDemand {
    fn merge(&mut self, invalidation: Invalidation, driving: bool, reason: DemandReason) {
        // A demand that requires no work is not demand; merging it must not
        // set the driving flag or record a reason.
        if invalidation == Invalidation::None {
            return;
        }
        self.invalidation = self.invalidation.combine(invalidation);
        self.driving |= driving;
        self.reasons.insert(reason);
    }

    fn is_empty(&self) -> bool {
        self.invalidation == Invalidation::None
    }

    fn take(&mut self) -> DueDemand {
        std::mem::take(self)
    }
}

#[derive(Debug, Default)]
struct WindowSched {
    presentation: WindowPresentationState,
    /// One outstanding platform redraw request (coalescing token).
    request_pending: bool,
    /// Demand consumed by the next begin_frame.
    due: DueDemand,
    /// Future deadline demands, at most one per reason.
    scheduled: Vec<ScheduledDemand>,
    /// Phase anchors for MaxRate reasons: next allowed fire time.
    max_rate_anchor: Vec<(DemandReason, Instant)>,
    last_present_at: Option<Instant>,
}

impl WindowSched {
    fn eligible(&self) -> bool {
        self.presentation.visible && !self.presentation.occluded
    }

    fn earliest_deadline(&self) -> Option<Instant> {
        self.scheduled.iter().map(|s| s.at).min()
    }

    fn has_any_demand(&self) -> bool {
        !self.due.is_empty() || !self.scheduled.is_empty()
    }

    fn anchor_for(&mut self, reason: DemandReason) -> Option<Instant> {
        self.max_rate_anchor
            .iter()
            .find(|(r, _)| *r == reason)
            .map(|(_, at)| *at)
    }

    fn set_anchor(&mut self, reason: DemandReason, at: Instant) {
        if let Some(entry) = self.max_rate_anchor.iter_mut().find(|(r, _)| *r == reason) {
            entry.1 = at;
        } else {
            self.max_rate_anchor.push((reason, at));
        }
    }

    fn schedule(&mut self, demand: ScheduledDemand) {
        if let Some(existing) = self
            .scheduled
            .iter_mut()
            .find(|s| s.reason == demand.reason)
        {
            *existing = demand;
        } else {
            self.scheduled.push(demand);
        }
    }
}

/// Owner of the policy connecting visual demand to presentation, per native
/// window. Pure: no timers, no platform calls; the runtime executes the
/// returned [`PacingAction`]s and feeds ticks and present results back.
#[derive(Debug, Default)]
pub(crate) struct FrameCoordinator {
    windows: BTreeMap<NativeWindowId, WindowSched>,
}

impl FrameCoordinator {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn window(&mut self, id: NativeWindowId) -> &mut WindowSched {
        self.windows.entry(id).or_default()
    }

    pub(crate) fn remove_window(&mut self, id: NativeWindowId) {
        self.windows.remove(&id);
    }

    /// Drop scheduling state for windows that no longer exist, so a
    /// destroyed window's deadlines cannot keep waking the loop.
    pub(crate) fn prune_windows(&mut self, keep: impl Fn(NativeWindowId) -> bool) {
        self.windows.retain(|id, _| keep(*id));
    }

    /// Declare demand. Returns the immediate action; duplicate driving
    /// demands coalesce into one outstanding request per window.
    pub(crate) fn submit_demand(
        &mut self,
        id: NativeWindowId,
        demand: FrameDemand,
        now: Instant,
    ) -> PacingAction {
        if demand.invalidation == Invalidation::None {
            return PacingAction::Sleep;
        }
        tracing::trace!(
            target: "neomacs::demand_trace",
            "submit id={id:?} reason={:?} cadence={:?} inval={:?}",
            demand.reason,
            demand.cadence,
            demand.invalidation
        );
        let ws = self.window(id);
        match demand.cadence {
            Cadence::OnDemand => {
                ws.due.merge(demand.invalidation, false, demand.reason);
                PacingAction::Sleep
            }
            Cadence::NextPresentation => {
                ws.due.merge(demand.invalidation, true, demand.reason);
                Self::drive(ws)
            }
            Cadence::At(at) if at <= now => {
                ws.due.merge(demand.invalidation, true, demand.reason);
                Self::drive(ws)
            }
            Cadence::At(at) => {
                ws.schedule(ScheduledDemand {
                    reason: demand.reason,
                    at,
                    invalidation: demand.invalidation,
                    period: None,
                });
                PacingAction::WakeAt(at)
            }
            Cadence::MaxRate(hz) => {
                let period = Duration::from_secs_f64(1.0 / f64::from(hz.get()));
                match ws.anchor_for(demand.reason) {
                    // First submission (or anchor already reached): fire now
                    // and anchor the phase grid at now + period.
                    None => {
                        ws.set_anchor(demand.reason, now + period);
                        ws.due.merge(demand.invalidation, true, demand.reason);
                        Self::drive(ws)
                    }
                    Some(anchor) if anchor <= now => {
                        let mut next = anchor;
                        while next <= now {
                            next += period;
                        }
                        ws.set_anchor(demand.reason, next);
                        ws.due.merge(demand.invalidation, true, demand.reason);
                        Self::drive(ws)
                    }
                    // Anchor in the future: schedule on the existing phase
                    // grid; resubmission is idempotent.
                    Some(anchor) => {
                        ws.schedule(ScheduledDemand {
                            reason: demand.reason,
                            at: anchor,
                            invalidation: demand.invalidation,
                            period: Some(period),
                        });
                        PacingAction::WakeAt(anchor)
                    }
                }
            }
        }
    }

    fn drive(ws: &mut WindowSched) -> PacingAction {
        if !ws.eligible() {
            return PacingAction::Sleep;
        }
        if ws.request_pending {
            return PacingAction::Sleep;
        }
        ws.request_pending = true;
        PacingAction::RequestRedraw
    }

    /// Withdraw a reason's demand (effect disabled, timer cancelled). Due
    /// work already merged from that reason stays merged; only its standing
    /// deadline and phase anchor are dropped.
    pub(crate) fn retract(&mut self, id: NativeWindowId, reason: DemandReason) {
        let ws = self.window(id);
        ws.scheduled.retain(|s| s.reason != reason);
        ws.max_rate_anchor.retain(|(r, _)| *r != reason);
    }

    /// Consume demand for one tick and decide the work.
    pub(crate) fn begin_frame(&mut self, id: NativeWindowId, tick: FrameTick) -> FramePlan {
        let ws = self.window(id);
        // This tick satisfies the outstanding request, whether it came from
        // our request or was platform-initiated (resize, expose).
        ws.request_pending = false;

        // Fold every ripe scheduled deadline into the due work. A very late
        // tick consumes the whole backlog as one plan; MaxRate anchors
        // advance by whole periods so the phase grid survives.
        let frame_time = tick.frame_time;
        let mut i = 0;
        while i < ws.scheduled.len() {
            if ws.scheduled[i].at <= frame_time {
                let ScheduledDemand {
                    reason,
                    invalidation,
                    period,
                    ..
                } = ws.scheduled.swap_remove(i);
                ws.due.merge(invalidation, true, reason);
                if let Some(period) = period {
                    let mut next = ws.anchor_for(reason).unwrap_or(frame_time);
                    while next <= frame_time {
                        next += period;
                    }
                    ws.set_anchor(reason, next);
                }
            } else {
                i += 1;
            }
        }

        if !ws.eligible() {
            // Retain demand; never present while ineligible.
            return FramePlan {
                tick,
                work: RenderWork::None,
                should_present: false,
                reasons: DemandReasonSet::empty(),
            };
        }

        let due = ws.due.take();
        let work = RenderWork::from_invalidation(due.invalidation);
        FramePlan {
            tick,
            work,
            should_present: work != RenderWork::None,
            reasons: due.reasons,
        }
    }

    /// Record the presentation outcome and decide the next action.
    pub(crate) fn finish_frame(
        &mut self,
        id: NativeWindowId,
        plan: &FramePlan,
        result: PresentResult,
        now: Instant,
    ) -> PacingAction {
        let ws = self.window(id);
        match result {
            PresentResult::Presented => {
                ws.last_present_at = Some(now);
            }
            PresentResult::Skipped => {
                // The plan's work never reached the screen; re-queue it.
                ws.due
                    .merge(plan.work.to_invalidation(), true, DemandReason::Expose);
            }
            PresentResult::Occluded => {
                ws.presentation.occluded = true;
                ws.due
                    .merge(plan.work.to_invalidation(), true, DemandReason::Expose);
                return PacingAction::Sleep;
            }
            PresentResult::SurfaceLost => {
                // Retained content is gone; a full repaint is required once
                // the runtime reconfigures the surface.
                ws.due.merge(
                    Invalidation::RepaintLayers {
                        layers: LayerMask::all(),
                        damage: Damage::FullLayer,
                    },
                    true,
                    DemandReason::Expose,
                );
                ws.due
                    .merge(plan.work.to_invalidation(), true, DemandReason::Expose);
                return Self::drive(ws);
            }
            PresentResult::Timeout => {
                // Bounded retry; never an immediate spin. The retry is
                // scheduled (not just returned) so next_wake_deadline() keeps
                // the recovery alive even if the caller drops the action.
                let invalidation = plan.work.to_invalidation();
                if invalidation != Invalidation::None {
                    ws.schedule(ScheduledDemand {
                        reason: DemandReason::Expose,
                        at: now + TIMEOUT_BACKOFF,
                        invalidation,
                        period: None,
                    });
                }
                return PacingAction::WakeAt(now + TIMEOUT_BACKOFF);
            }
        }
        if ws.due.driving && !ws.due.is_empty() {
            return Self::drive(ws);
        }
        match ws.earliest_deadline() {
            Some(at) => PacingAction::WakeAt(at),
            None => PacingAction::Sleep,
        }
    }

    /// Update visibility/focus/occlusion wholesale. Regaining eligibility with
    /// retained demand issues exactly one recovery request.
    // Full-replace form kept for initialization/tests; the runtime drives
    // per-field transitions through set_occluded/set_focused/set_visible.
    #[allow(dead_code)]
    pub(crate) fn update_window_state(
        &mut self,
        id: NativeWindowId,
        state: WindowPresentationState,
    ) -> PacingAction {
        self.mutate_presentation(id, |p| *p = state)
    }

    /// Mark a window occluded or exposed. Exposure with retained demand
    /// issues one recovery request; occlusion suspends presentation.
    pub(crate) fn set_occluded(&mut self, id: NativeWindowId, occluded: bool) -> PacingAction {
        self.mutate_presentation(id, |p| p.occluded = occluded)
    }

    /// Mark a window minimized/hidden or shown. Same eligibility semantics as
    /// occlusion.
    // Wired when a minimize/hide event source lands (plan Stage 7 policy);
    // Occluded already covers the Wayland/macOS not-showing case. Covered by
    // the scheduler tests.
    #[allow(dead_code)]
    pub(crate) fn set_visible(&mut self, id: NativeWindowId, visible: bool) -> PacingAction {
        self.mutate_presentation(id, |p| p.visible = visible)
    }

    /// Update focus. Focus does not gate presentation, but it is scheduling
    /// input for ambient-effect policy (plan: visibility and power policy).
    pub(crate) fn set_focused(&mut self, id: NativeWindowId, focused: bool) -> PacingAction {
        self.mutate_presentation(id, |p| p.focused = focused)
    }

    /// Whether a window is eligible to present (visible and not occluded).
    pub(crate) fn is_eligible(&self, id: NativeWindowId) -> bool {
        self.windows
            .get(&id)
            .map(|ws| ws.eligible())
            .unwrap_or(true)
    }

    /// Whether a window is focused. Unknown windows default to focused (a
    /// window that has never reported focus should not have ambient effects
    /// suppressed).
    pub(crate) fn is_focused(&self, id: NativeWindowId) -> bool {
        self.windows
            .get(&id)
            .map(|ws| ws.presentation.focused)
            .unwrap_or(true)
    }

    fn mutate_presentation(
        &mut self,
        id: NativeWindowId,
        f: impl FnOnce(&mut WindowPresentationState),
    ) -> PacingAction {
        let ws = self.window(id);
        let was_eligible = ws.eligible();
        f(&mut ws.presentation);
        let now_eligible = ws.eligible();
        if was_eligible && !now_eligible {
            // Any outstanding redraw request is void once the surface is no
            // longer presentable: platforms may drop a pending request when a
            // window is occluded/hidden and never redeliver it on exposure.
            // Clearing it lets the exposure transition issue a fresh one.
            ws.request_pending = false;
        }
        if !was_eligible && now_eligible && ws.has_any_demand() {
            return Self::drive(ws);
        }
        PacingAction::Sleep
    }

    /// Earliest scheduled deadline across eligible windows: the event loop's
    /// WaitUntil aggregation input. None means the loop may Wait indefinitely
    /// as far as frame demand is concerned.
    pub(crate) fn next_wake_deadline(&self) -> Option<Instant> {
        self.windows
            .values()
            .filter(|ws| ws.eligible())
            .filter_map(|ws| ws.earliest_deadline())
            .min()
    }

    /// Active demand reasons for diagnostics ("why is this window still
    /// rendering?").
    // Exposed to the diagnostic snapshot when GUI-test tooling consumes it
    // (plan: Observability); covered by the scheduler tests.
    #[allow(dead_code)]
    pub(crate) fn active_reasons(&self, id: NativeWindowId) -> Vec<DemandReason> {
        let Some(ws) = self.windows.get(&id) else {
            return Vec::new();
        };
        let mut reasons = ws.due.reasons;
        for s in &ws.scheduled {
            reasons.insert(s.reason);
        }
        reasons.iter().collect()
    }

    /// Whether a redraw request is outstanding for this window.
    #[cfg(test)]
    pub(crate) fn request_pending(&self, id: NativeWindowId) -> bool {
        self.windows
            .get(&id)
            .map(|ws| ws.request_pending)
            .unwrap_or(false)
    }
}

#[cfg(test)]
#[path = "frame_sched_test.rs"]
mod frame_sched_test;
