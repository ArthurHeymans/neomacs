//! Buffer source render-attempt state, retry, and publish lifecycle.

use crate::coords::layout_i64_char_pos_to_lisp_char_pos;
use crate::display_buffer_source_tail_render::{
    BufferSourcePostLoopRenderOutcome, BufferSourceRetryBounds,
};
use crate::display_frame_output::FrameOutputOwner;
use crate::display_row_source_render::{TextRowOutputRenderState, TextRowSourceRenderState};
use crate::display_text_window_row_lifecycle::{
    TextWindowBeginRequest, TextWindowCursorEffectsRequest, TextWindowVisibilityRetryOutcome,
};
use crate::font_metrics::FontMetricsService;
use crate::frame_face_arena::FrameFaceAttempt;
use crate::neovm_bridge::FaceResolver;
use crate::types::WindowParams;
use crate::window_output::{
    TextWindowOutputRetryCheckpoint, TextWindowOutputTarget, TextWindowRedisplayPositions,
    WindowOutputEmitter, capture_text_window_retry_checkpoint,
    restore_text_window_retry_checkpoint,
};
use neovm_core::buffer::{EmacsBytePos, LispCharPos1};
use neovm_core::emacs_core::Context;
use neovm_core::window::{FrameId, WindowDisplaySnapshot, WindowId};

pub(crate) struct BufferSourceOutputState<'emit> {
    output: TextWindowOutputTarget<'emit>,
    evaluator: &'emit mut Context,
}

pub(crate) struct BufferSourceRenderAttemptContext<'a, 'face> {
    output: BufferSourceOutputState<'a>,
    font_metrics: &'a mut Option<FontMetricsService>,
    face_resolver: &'face FaceResolver,
    face_attempt: FrameFaceAttempt,
    window_snapshots: &'a mut Vec<WindowDisplaySnapshot>,
}

/// Which live window state may be published by a completed row walk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum WindowPositionPublication {
    #[default]
    Redisplay,
    /// Redisplay's preliminary GNU `resize_mini_window` measurement.
    ///
    /// The row walk is lifted to `max-mini-window-height`. If it reaches ZV,
    /// point visibility relative to the old physical one-line allocation must
    /// not scroll the start chosen by the resize measurement.
    RedisplayMinibufferMeasurement,
    SynchronousQueryEnd,
}

impl WindowPositionPublication {
    /// A logical `window-end` query walks from the live start marker exactly.
    ///
    /// Redisplay may resolve a different source start to keep point visible;
    /// GNU `Fwindow_end` does not run that viewport policy.
    pub(crate) const fn uses_exact_window_start(self) -> bool {
        matches!(self, Self::SynchronousQueryEnd)
    }

    pub(crate) const fn keeps_complete_minibuffer_measurement_start(self) -> bool {
        matches!(self, Self::RedisplayMinibufferMeasurement)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSourceRedisplayPublishRequest {
    frame_id: FrameId,
    window_id: WindowId,
    accessible_end_lisp_char: usize,
    accessible_end_emacs_byte: usize,
    publication: WindowPositionPublication,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferSourceRenderAttemptOutcome {
    Skipped,
    Retry {
        window_start: i64,
    },
    /// The window start is forced (GNU `w->force_start`): keep it and re-lay
    /// with POINT moved to the last fully-visible position instead of
    /// recomputing the start around point (GNU redisplay_window's
    /// force_start branch moves point when the cursor row is off-window).
    RetryPointIntoWindow {
        /// Layout 0-based charpos point should move to.
        point_charpos: i64,
    },
    Finished {
        redisplay_positions: TextWindowRedisplayPositions,
        window_end_record: neovm_core::window::WindowEndRecord,
        /// Whether this window took the Phase 1 cursor-only fast path (body rows
        /// reused verbatim) rather than a full body walk.
        cursor_only: bool,
        /// `Some(reused_row_count)` when this window took the Phase 2 pure-scroll
        /// fast path (that many overlapping rows reused shifted; only the
        /// newly-exposed rows walked); `None` otherwise.
        scroll_reused_rows: Option<usize>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSourceRetryPlan {
    window_id: i64,
    window_start: i64,
    point_charpos: i64,
    charpos_end: i64,
    rendered_rows_len: usize,
    retry_bounds: BufferSourceRetryBounds,
    retry: TextWindowVisibilityRetryOutcome,
}

impl<'emit> BufferSourceOutputState<'emit> {
    pub(crate) fn from_parts(
        output: TextWindowOutputTarget<'emit>,
        evaluator: &'emit mut Context,
    ) -> Self {
        Self { output, evaluator }
    }

    pub(crate) fn capture_retry_checkpoint(&mut self) -> TextWindowOutputRetryCheckpoint {
        capture_text_window_retry_checkpoint(self.output.reborrow())
    }

    pub(crate) fn restore_retry_checkpoint(&mut self, checkpoint: TextWindowOutputRetryCheckpoint) {
        restore_text_window_retry_checkpoint(self.output.reborrow(), checkpoint);
    }

    pub(crate) fn evaluator(&mut self) -> &mut Context {
        self.evaluator
    }

    pub(crate) fn into_parts(self) -> (TextWindowOutputTarget<'emit>, &'emit mut Context) {
        (self.output, self.evaluator)
    }

    pub(crate) fn install_cursor_effects(&mut self, params: &WindowParams) -> bool {
        TextWindowCursorEffectsRequest::new(params.window_id, params.cursor_effects.clone())
            .install_and_apply(self.output.reborrow())
    }

    pub(crate) fn begin_text_window_output(
        &mut self,
        begin_request: TextWindowBeginRequest,
    ) -> WindowOutputEmitter {
        begin_request.begin_and_apply(self.output.reborrow(), self.evaluator)
    }

    pub(crate) fn source_render_state<'output>(
        &'output mut self,
        output_emitter: &'output mut WindowOutputEmitter,
        font_metrics: &'output mut Option<FontMetricsService>,
        window_system: bool,
        face_resolver: &'output FaceResolver,
    ) -> TextRowSourceRenderState<'output> {
        TextRowSourceRenderState::from_output_render(
            TextRowOutputRenderState::from_parts(
                self.output.reborrow(),
                output_emitter,
                self.evaluator,
            ),
            font_metrics,
            window_system,
            face_resolver,
        )
    }
}

impl<'a, 'face> BufferSourceRenderAttemptContext<'a, 'face> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        output: TextWindowOutputTarget<'a>,
        evaluator: &'a mut Context,
        font_metrics: &'a mut Option<FontMetricsService>,
        face_resolver: &'face FaceResolver,
        face_attempt: FrameFaceAttempt,
        window_snapshots: &'a mut Vec<WindowDisplaySnapshot>,
    ) -> Self {
        Self {
            output: BufferSourceOutputState::from_parts(output, evaluator),
            font_metrics,
            face_resolver,
            face_attempt,
            window_snapshots,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_frame_output_owner(
        frame_output: &'a mut FrameOutputOwner,
        evaluator: &'a mut Context,
        font_metrics: &'a mut Option<FontMetricsService>,
        face_resolver: &'face FaceResolver,
        face_attempt: FrameFaceAttempt,
        window_snapshots: &'a mut Vec<WindowDisplaySnapshot>,
    ) -> Self {
        Self::new(
            frame_output.text_window_output_target(),
            evaluator,
            font_metrics,
            face_resolver,
            face_attempt,
            window_snapshots,
        )
    }

    pub(crate) fn output_mut(&mut self) -> &mut BufferSourceOutputState<'a> {
        &mut self.output
    }

    pub(crate) fn with_face_services<R>(
        &mut self,
        f: impl FnOnce(&FaceResolver, &mut Option<FontMetricsService>) -> R,
    ) -> R {
        f(self.face_resolver, self.font_metrics)
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        BufferSourceOutputState<'a>,
        &'a mut Option<FontMetricsService>,
        &'face FaceResolver,
        FrameFaceAttempt,
        &'a mut Vec<WindowDisplaySnapshot>,
    ) {
        (
            self.output,
            self.font_metrics,
            self.face_resolver,
            self.face_attempt,
            self.window_snapshots,
        )
    }
}

impl BufferSourceRedisplayPublishRequest {
    pub(crate) fn new(
        frame_id: FrameId,
        window_id: WindowId,
        accessible_end_lisp_char: usize,
        accessible_end_emacs_byte: usize,
        publication: WindowPositionPublication,
    ) -> Self {
        Self {
            frame_id,
            window_id,
            accessible_end_lisp_char,
            accessible_end_emacs_byte,
            publication,
        }
    }

    pub(crate) fn publish(self, evaluator: &mut Context, positions: TextWindowRedisplayPositions) {
        let buffer_z_char = LispCharPos1::from_one_based_usize(self.accessible_end_lisp_char);
        let buffer_z_byte = EmacsBytePos::new(self.accessible_end_emacs_byte);
        match self.publication {
            WindowPositionPublication::Redisplay
            | WindowPositionPublication::RedisplayMinibufferMeasurement => {
                evaluator.publish_redisplay_window_positions(
                    self.frame_id,
                    self.window_id,
                    positions.window_start,
                    buffer_z_char,
                    buffer_z_byte,
                    positions.window_end,
                    positions.window_end_byte,
                    positions.window_end_vpos,
                );
            }
            WindowPositionPublication::SynchronousQueryEnd => {
                evaluator.publish_window_layout_query_end(
                    self.frame_id,
                    self.window_id,
                    buffer_z_char,
                    buffer_z_byte,
                    positions.window_end,
                    positions.window_end_byte,
                    positions.window_end_vpos,
                );
            }
        }
    }

    pub(crate) fn window_end_record(
        self,
        positions: TextWindowRedisplayPositions,
    ) -> neovm_core::window::WindowEndRecord {
        neovm_core::window::WindowEndRecord::from_positions(
            LispCharPos1::from_one_based_usize(self.accessible_end_lisp_char),
            EmacsBytePos::new(self.accessible_end_emacs_byte),
            positions.window_end,
            positions.window_end_byte,
            neovm_core::window::MatrixRow0::new(positions.window_end_vpos),
        )
    }
}

impl BufferSourceRetryPlan {
    pub(crate) fn from_post_loop(
        window_id: i64,
        window_start: i64,
        point_charpos: i64,
        charpos_end: i64,
        retry_bounds: BufferSourceRetryBounds,
        post_loop: BufferSourcePostLoopRenderOutcome,
    ) -> Self {
        Self {
            window_id,
            window_start,
            point_charpos,
            charpos_end,
            rendered_rows_len: post_loop.rendered_rows_len,
            retry_bounds,
            retry: post_loop.retry,
        }
    }

    pub(crate) fn log_visibility_adjustments(self) {
        if self.retry.scroll_down_window_start().is_some() {
            tracing::debug!(
                "layout_window_rust: point={} beyond visible_end={:?} (charpos_end={}), visible_rows={}, new_window_start={:?}",
                layout_i64_char_pos_to_lisp_char_pos(self.point_charpos).as_i64(),
                self.retry.visible_end_lisp(),
                self.charpos_end,
                self.rendered_rows_len,
                self.retry.scroll_down_window_start()
            );
        }
        if self.retry.point_row_window_start().is_some() {
            tracing::debug!(
                "layout_window_rust: point={} row partially visible within {}..{}, new_window_start={:?}",
                self.point_charpos,
                self.retry_bounds.text_area_top(),
                self.retry_bounds.text_area_bottom(),
                self.retry.point_row_window_start()
            );
        }
        if self.retry.point_line_window_start().is_some() {
            tracing::debug!(
                "layout_window_rust: point={} line continues below final visible row, new_window_start={:?}",
                self.point_charpos,
                self.retry.point_line_window_start()
            );
        }
    }

    pub(crate) fn retry_window_start(self) -> Option<i64> {
        self.retry.retry_window_start()
    }

    /// Target for GNU's force_start point move: the last fully-visible
    /// buffer position of the attempt just laid out (layout 0-based), i.e.
    /// point lands on the final visible row of the kept window start.
    pub(crate) fn forced_start_point_target(self) -> Option<i64> {
        self.retry
            .visible_end_lisp()
            .map(|pos| pos.as_i64() - 1)
            .filter(|charpos| *charpos >= 0)
    }

    pub(crate) fn should_retry(self, remaining_visibility_retries: usize) -> Option<i64> {
        self.retry_window_start().filter(|new_window_start| {
            remaining_visibility_retries > 0 && *new_window_start > self.window_start
        })
    }

    pub(crate) fn log_retry(self, new_window_start: i64, remaining_visibility_retries: usize) {
        tracing::debug!(
            "layout_window_rust: retrying window {} with adjusted window_start {} -> {} (remaining={})",
            self.window_id,
            self.window_start,
            new_window_start,
            remaining_visibility_retries
        );
    }
}
