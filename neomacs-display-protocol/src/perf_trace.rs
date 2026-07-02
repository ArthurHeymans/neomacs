//! Env-gated display performance tracing shared by layout, runtime, and renderer.
//!
//! Set `NEOMACS_DISPLAY_TRACE=1` to emit one summary line for each rendered
//! frame and detailed phase counters gathered along the display pipeline.

use std::cell::RefCell;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static DISPLAY_TRACE_ENABLED: OnceLock<bool> = OnceLock::new();
static NEXT_INPUT_TRACE_ID: AtomicU64 = AtomicU64::new(1);
static LATEST_INPUT_TRACE: Mutex<Option<DisplayInputTrace>> = Mutex::new(None);

thread_local! {
    static PHASE_COUNTERS: RefCell<DisplayPhaseCounters> =
        RefCell::new(DisplayPhaseCounters::default());
}

#[derive(Clone, Copy, Debug)]
pub struct DisplayInputTrace {
    pub id: u64,
    pub event_name: &'static str,
    pub render_thread_received_at: Instant,
    pub bridge_delivered_at: Option<Instant>,
}

#[derive(Clone, Debug, Default)]
pub struct DisplayFramePerfTrace {
    pub input: Option<DisplayInputTrace>,
    pub publish_started_at: Option<Instant>,
    pub frame_sent_at: Option<Instant>,
    pub materialize_started_at: Option<Instant>,
    pub render_started_at: Option<Instant>,
    pub presented_at: Option<Instant>,
    pub layout_total_ns: u64,
    pub materialize_ns: u64,
    pub render_window_ns: u64,
    pub present_call_ns: u64,
    pub layout_window_count: u32,
    pub layout_window_ns: u64,
    pub status_line_eval_count: u32,
    pub status_line_eval_ns: u64,
    pub glyph_render_ns: u64,
    pub glyph_submit_ns: u64,
    pub gpu_main_glyph_pass_ns: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DisplayPhaseCounters {
    pub layout_window_count: u32,
    pub layout_window_ns: u64,
    pub status_line_eval_count: u32,
    pub status_line_eval_ns: u64,
    pub glyph_render_ns: u64,
    pub glyph_submit_ns: u64,
    pub gpu_main_glyph_pass_ns: u64,
}

#[derive(Clone, Copy, Debug)]
pub enum DisplayPhase {
    LayoutWindow,
    StatusLineEval,
    GlyphRender,
    GlyphSubmit,
}

pub struct DisplayPhaseTimer {
    phase: DisplayPhase,
    start: Instant,
    enabled: bool,
}

impl Drop for DisplayPhaseTimer {
    fn drop(&mut self) {
        if self.enabled {
            record_phase_duration(self.phase, self.start.elapsed());
        }
    }
}

pub fn enabled() -> bool {
    *DISPLAY_TRACE_ENABLED.get_or_init(|| {
        std::env::var("NEOMACS_DISPLAY_TRACE")
            .ok()
            .is_some_and(|value| {
                let value = value.trim();
                !value.is_empty()
                    && !value.eq_ignore_ascii_case("0")
                    && !value.eq_ignore_ascii_case("false")
                    && !value.eq_ignore_ascii_case("off")
            })
    })
}

pub fn begin_input_trace(event_name: &'static str) -> Option<DisplayInputTrace> {
    if !enabled() {
        return None;
    }
    Some(DisplayInputTrace {
        id: NEXT_INPUT_TRACE_ID.fetch_add(1, Ordering::Relaxed),
        event_name,
        render_thread_received_at: Instant::now(),
        bridge_delivered_at: None,
    })
}

pub fn mark_input_delivered(mut trace: DisplayInputTrace) {
    if !enabled() {
        return;
    }
    trace.bridge_delivered_at = Some(Instant::now());
    if let Ok(mut latest) = LATEST_INPUT_TRACE.lock() {
        *latest = Some(trace);
    }
}

pub fn take_latest_input_trace() -> Option<DisplayInputTrace> {
    if !enabled() {
        return None;
    }
    LATEST_INPUT_TRACE
        .lock()
        .ok()
        .and_then(|mut latest| latest.take())
}

pub fn phase_timer(phase: DisplayPhase) -> DisplayPhaseTimer {
    DisplayPhaseTimer {
        phase,
        start: Instant::now(),
        enabled: enabled(),
    }
}

pub fn reset_phase_counters() {
    if enabled() {
        PHASE_COUNTERS.with(|counters| *counters.borrow_mut() = DisplayPhaseCounters::default());
    }
}

pub fn take_phase_counters() -> DisplayPhaseCounters {
    if !enabled() {
        return DisplayPhaseCounters::default();
    }
    PHASE_COUNTERS.with(|counters| std::mem::take(&mut *counters.borrow_mut()))
}

pub fn duration_ns(duration: Duration) -> u64 {
    duration.as_nanos().min(u128::from(u64::MAX)) as u64
}

pub fn merge_phase_counters(trace: &mut DisplayFramePerfTrace, counters: DisplayPhaseCounters) {
    trace.layout_window_count = trace
        .layout_window_count
        .saturating_add(counters.layout_window_count);
    trace.layout_window_ns = trace
        .layout_window_ns
        .saturating_add(counters.layout_window_ns);
    trace.status_line_eval_count = trace
        .status_line_eval_count
        .saturating_add(counters.status_line_eval_count);
    trace.status_line_eval_ns = trace
        .status_line_eval_ns
        .saturating_add(counters.status_line_eval_ns);
    trace.glyph_render_ns = trace
        .glyph_render_ns
        .saturating_add(counters.glyph_render_ns);
    trace.glyph_submit_ns = trace
        .glyph_submit_ns
        .saturating_add(counters.glyph_submit_ns);
    trace.gpu_main_glyph_pass_ns = trace
        .gpu_main_glyph_pass_ns
        .saturating_add(counters.gpu_main_glyph_pass_ns);
}

pub fn record_gpu_main_glyph_pass_ms(ms: f64) {
    if !enabled() || !ms.is_finite() || ms < 0.0 {
        return;
    }
    let ns = (ms * 1_000_000.0).min(u64::MAX as f64) as u64;
    PHASE_COUNTERS.with(|counters| {
        let mut counters = counters.borrow_mut();
        counters.gpu_main_glyph_pass_ns = counters.gpu_main_glyph_pass_ns.saturating_add(ns);
    });
}

pub fn log_frame_summary(frame_id: u64, glyph_count: usize, trace: &DisplayFramePerfTrace) {
    if !enabled() {
        return;
    }

    let input_to_present_ms = trace.input.and_then(|input| {
        trace.presented_at.map(|presented| {
            presented
                .duration_since(input.render_thread_received_at)
                .as_secs_f64()
                * 1000.0
        })
    });
    let bridge_to_present_ms = trace.input.and_then(|input| {
        input.bridge_delivered_at.and_then(|delivered| {
            trace
                .presented_at
                .map(|presented| presented.duration_since(delivered).as_secs_f64() * 1000.0)
        })
    });

    tracing::info!(
        target: "neomacs_display_trace",
        frame_id = frame_id,
        glyphs = glyph_count,
        input_id = trace.input.map(|input| input.id),
        input_event = trace.input.map(|input| input.event_name).unwrap_or("none"),
        input_to_present_ms = input_to_present_ms,
        bridge_to_present_ms = bridge_to_present_ms,
        layout_total_ms = ns_to_ms(trace.layout_total_ns),
        layout_windows = trace.layout_window_count,
        layout_windows_ms = ns_to_ms(trace.layout_window_ns),
        status_line_evals = trace.status_line_eval_count,
        status_line_eval_ms = ns_to_ms(trace.status_line_eval_ns),
        materialize_ms = ns_to_ms(trace.materialize_ns),
        render_window_ms = ns_to_ms(trace.render_window_ns),
        glyph_render_ms = ns_to_ms(trace.glyph_render_ns),
        glyph_submit_ms = ns_to_ms(trace.glyph_submit_ns),
        gpu_main_glyph_pass_ms = ns_to_ms(trace.gpu_main_glyph_pass_ns),
        present_call_ms = ns_to_ms(trace.present_call_ns),
        "display frame performance"
    );
}

fn record_phase_duration(phase: DisplayPhase, duration: Duration) {
    let ns = duration_ns(duration);
    PHASE_COUNTERS.with(|counters| {
        let mut counters = counters.borrow_mut();
        match phase {
            DisplayPhase::LayoutWindow => {
                counters.layout_window_count = counters.layout_window_count.saturating_add(1);
                counters.layout_window_ns = counters.layout_window_ns.saturating_add(ns);
            }
            DisplayPhase::StatusLineEval => {
                counters.status_line_eval_count = counters.status_line_eval_count.saturating_add(1);
                counters.status_line_eval_ns = counters.status_line_eval_ns.saturating_add(ns);
            }
            DisplayPhase::GlyphRender => {
                counters.glyph_render_ns = counters.glyph_render_ns.saturating_add(ns);
            }
            DisplayPhase::GlyphSubmit => {
                counters.glyph_submit_ns = counters.glyph_submit_ns.saturating_add(ns);
            }
        }
    });
}

fn ns_to_ms(ns: u64) -> f64 {
    ns as f64 / 1_000_000.0
}
