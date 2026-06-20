//! Buffer text output session, retry, and publish lifecycle.

use crate::coords::layout_i64_char_pos_to_lisp_char_pos;
use crate::display_buffer_text_append::{
    BufferTextWindowBeginRequest, BufferTextWindowCursorEffectsRequest,
    BufferTextWindowFinishState, BufferTextWindowVisibilityRetryOutcome,
};
use crate::display_buffer_text_render::{
    BufferTextWindowFinishInstallState, BufferTextWindowInitialFaceStateRequest,
    BufferTextWindowPostLoopRenderOutcome, BufferTextWindowRetryBounds,
};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_frame_output::FrameOutputOwner;
use crate::display_row::DisplayRowActiveFaceState;
use crate::display_row_source_render::{TextRowOutputRenderState, TextRowSourceRenderState};
use crate::display_status_line::ChromeRowRenderServices;
use crate::font_metrics::FontMetricsService;
use crate::hit_test::{HitRow, WindowHitData};
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

pub(crate) struct BufferTextWindowOutputState<'emit> {
    pub(crate) output: TextWindowOutputTarget<'emit>,
    pub(crate) evaluator: &'emit mut Context,
}

pub(crate) struct BufferTextWindowBodyOutputRenderState<'emit> {
    output: BufferTextWindowOutputState<'emit>,
    font_metrics: &'emit mut Option<FontMetricsService>,
    face_resolver: &'emit FaceResolver,
}

pub(crate) struct BufferTextWindowBodyPassState<'emit> {
    pub(crate) output: BufferTextWindowBodyOutputRenderState<'emit>,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
}

pub(crate) struct BufferTextWindowOutputSession<'emit> {
    output: BufferTextWindowOutputState<'emit>,
    font_metrics: &'emit mut Option<FontMetricsService>,
    face_resolver: &'emit FaceResolver,
    face_ids: FrameFaceIdAllocator,
    retry_checkpoint: TextWindowOutputRetryCheckpoint,
}

pub(crate) struct BufferTextWindowRenderAttemptContext<'a, 'face> {
    pub(crate) output: BufferTextWindowOutputState<'a>,
    pub(crate) font_metrics: &'a mut Option<FontMetricsService>,
    pub(crate) face_resolver: &'face FaceResolver,
    pub(crate) frame_face_id_counter: &'a mut u32,
    pub(crate) hit_data: &'a mut Vec<WindowHitData>,
    pub(crate) display_snapshots: &'a mut Vec<WindowDisplaySnapshot>,
}

pub(crate) struct BufferTextWindowBodyInstallRenderState<'emit, 'output, 'face> {
    pub(crate) output: TextWindowOutputTarget<'output>,
    pub(crate) output_emitter: &'output mut WindowOutputEmitter,
    pub(crate) render_services: ChromeRowRenderServices<'emit, 'face>,
}

pub(crate) struct BufferTextWindowBodyInstallPublishState<'emit, 'output, 'face> {
    pub(crate) output: TextWindowOutputTarget<'output>,
    pub(crate) output_emitter: &'output mut WindowOutputEmitter,
    pub(crate) evaluator: &'output mut Context,
    pub(crate) render_services: ChromeRowRenderServices<'emit, 'face>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowRedisplayPublishRequest {
    frame_id: FrameId,
    window_id: WindowId,
    accessible_end_lisp_char: usize,
    accessible_end_emacs_byte: usize,
}

pub(crate) struct BufferTextWindowBodyPassOutcome {
    pub(crate) output_emitter: WindowOutputEmitter,
    pub(crate) post_loop: BufferTextWindowPostLoopRenderOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextWindowRenderAttemptOutcome {
    Skipped,
    Retry {
        window_start: i64,
    },
    Finished {
        redisplay_positions: TextWindowRedisplayPositions,
    },
}

pub(crate) struct BufferTextWindowRenderedBodyFinishState<'a> {
    output: BufferTextWindowOutputState<'a>,
    hit_data: &'a mut Vec<WindowHitData>,
    display_snapshots: &'a mut Vec<WindowDisplaySnapshot>,
}

pub(crate) struct BufferTextWindowRenderedBodyCompleteState<'emit, 'face> {
    pub(crate) output: BufferTextWindowOutputState<'emit>,
    pub(crate) render_services: ChromeRowRenderServices<'emit, 'face>,
    hit_data: &'emit mut Vec<WindowHitData>,
    display_snapshots: &'emit mut Vec<WindowDisplaySnapshot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowRetryPlan {
    window_id: i64,
    window_start: i64,
    point_charpos: i64,
    charpos_end: i64,
    rendered_rows_len: usize,
    retry_bounds: BufferTextWindowRetryBounds,
    retry: BufferTextWindowVisibilityRetryOutcome,
}

impl<'emit> BufferTextWindowOutputState<'emit> {
    pub(crate) fn from_parts(
        output: TextWindowOutputTarget<'emit>,
        evaluator: &'emit mut Context,
    ) -> Self {
        Self { output, evaluator }
    }

    pub(crate) fn reborrow(&mut self) -> BufferTextWindowOutputState<'_> {
        BufferTextWindowOutputState::from_parts(self.output.reborrow(), self.evaluator)
    }

    fn capture_retry_checkpoint(&mut self) -> TextWindowOutputRetryCheckpoint {
        capture_text_window_retry_checkpoint(self.output.reborrow())
    }

    fn restore_retry_checkpoint(&mut self, checkpoint: TextWindowOutputRetryCheckpoint) {
        restore_text_window_retry_checkpoint(self.output.reborrow(), checkpoint);
    }

    pub(crate) fn evaluator(&mut self) -> &mut Context {
        self.evaluator
    }

    pub(crate) fn install_cursor_effects(&mut self, params: &WindowParams) -> bool {
        BufferTextWindowCursorEffectsRequest::new(params.window_id, params.cursor_effects.clone())
            .install_and_apply(self.output.reborrow())
    }

    fn begin_text_window_output(
        &mut self,
        begin_request: BufferTextWindowBeginRequest,
    ) -> WindowOutputEmitter {
        begin_request.begin_and_apply(self.output.reborrow(), self.evaluator)
    }

    fn source_render_state<'output>(
        &'output mut self,
        output_emitter: &'output mut WindowOutputEmitter,
        font_metrics: &'output mut Option<FontMetricsService>,
        face_resolver: &'output FaceResolver,
    ) -> TextRowSourceRenderState<'output> {
        TextRowSourceRenderState::from_output_render(
            TextRowOutputRenderState::from_parts(
                self.output.reborrow(),
                output_emitter,
                self.evaluator,
            ),
            font_metrics,
            face_resolver,
        )
    }

    fn into_finish_state(
        self,
        output_emitter: WindowOutputEmitter,
        hit_rows: Vec<HitRow>,
    ) -> BufferTextWindowFinishState<'emit> {
        BufferTextWindowFinishState::new(self.output, output_emitter, self.evaluator, hit_rows)
    }
}

impl<'emit> BufferTextWindowBodyOutputRenderState<'emit> {
    fn new(
        output: BufferTextWindowOutputState<'emit>,
        font_metrics: &'emit mut Option<FontMetricsService>,
        face_resolver: &'emit FaceResolver,
    ) -> Self {
        Self {
            output,
            font_metrics,
            face_resolver,
        }
    }

    pub(crate) fn begin_text_window_output(
        &mut self,
        begin_request: BufferTextWindowBeginRequest,
    ) -> WindowOutputEmitter {
        self.output.begin_text_window_output(begin_request)
    }

    pub(crate) fn source_render_state<'output>(
        &'output mut self,
        output_emitter: &'output mut WindowOutputEmitter,
    ) -> TextRowSourceRenderState<'output> {
        self.output
            .source_render_state(output_emitter, self.font_metrics, self.face_resolver)
    }
}

impl<'emit> BufferTextWindowBodyPassState<'emit> {
    fn new(
        output: BufferTextWindowBodyOutputRenderState<'emit>,
        face_ids: &'emit mut FrameFaceIdAllocator,
    ) -> Self {
        Self { output, face_ids }
    }
}

impl<'emit, 'output, 'face> BufferTextWindowBodyInstallRenderState<'emit, 'output, 'face> {
    pub(crate) fn new(
        output: TextWindowOutputTarget<'output>,
        output_emitter: &'output mut WindowOutputEmitter,
        render_services: ChromeRowRenderServices<'emit, 'face>,
    ) -> Self {
        Self {
            output,
            output_emitter,
            render_services,
        }
    }
}

impl<'emit, 'output, 'face> BufferTextWindowBodyInstallPublishState<'emit, 'output, 'face> {
    pub(crate) fn new(
        output: TextWindowOutputTarget<'output>,
        output_emitter: &'output mut WindowOutputEmitter,
        evaluator: &'output mut Context,
        render_services: ChromeRowRenderServices<'emit, 'face>,
    ) -> Self {
        Self {
            output,
            output_emitter,
            evaluator,
            render_services,
        }
    }
}

impl<'emit, 'face> BufferTextWindowRenderedBodyCompleteState<'emit, 'face> {
    fn new(
        output: BufferTextWindowOutputState<'emit>,
        render_services: ChromeRowRenderServices<'emit, 'face>,
        hit_data: &'emit mut Vec<WindowHitData>,
        display_snapshots: &'emit mut Vec<WindowDisplaySnapshot>,
    ) -> Self {
        Self {
            output,
            render_services,
            hit_data,
            display_snapshots,
        }
    }

    pub(crate) fn finish_state(self) -> BufferTextWindowRenderedBodyFinishState<'emit> {
        BufferTextWindowRenderedBodyFinishState {
            output: self.output,
            hit_data: self.hit_data,
            display_snapshots: self.display_snapshots,
        }
    }
}

impl<'a> BufferTextWindowRenderedBodyFinishState<'a> {
    pub(crate) fn finish_install_state(
        self,
        output_emitter: WindowOutputEmitter,
        hit_rows: Vec<HitRow>,
    ) -> BufferTextWindowFinishInstallState<'a> {
        BufferTextWindowFinishInstallState {
            finish_state: self.output.into_finish_state(output_emitter, hit_rows),
            hit_data: self.hit_data,
            display_snapshots: self.display_snapshots,
        }
    }
}

impl<'a, 'face> BufferTextWindowRenderAttemptContext<'a, 'face> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        output: TextWindowOutputTarget<'a>,
        evaluator: &'a mut Context,
        font_metrics: &'a mut Option<FontMetricsService>,
        face_resolver: &'face FaceResolver,
        frame_face_id_counter: &'a mut u32,
        hit_data: &'a mut Vec<WindowHitData>,
        display_snapshots: &'a mut Vec<WindowDisplaySnapshot>,
    ) -> Self {
        Self {
            output: BufferTextWindowOutputState::from_parts(output, evaluator),
            font_metrics,
            face_resolver,
            frame_face_id_counter,
            hit_data,
            display_snapshots,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_frame_output_owner(
        frame_output: &'a mut FrameOutputOwner,
        evaluator: &'a mut Context,
        font_metrics: &'a mut Option<FontMetricsService>,
        face_resolver: &'face FaceResolver,
        frame_face_id_counter: &'a mut u32,
        hit_data: &'a mut Vec<WindowHitData>,
        display_snapshots: &'a mut Vec<WindowDisplaySnapshot>,
    ) -> Self {
        Self::new(
            frame_output.text_window_output_target(),
            evaluator,
            font_metrics,
            face_resolver,
            frame_face_id_counter,
            hit_data,
            display_snapshots,
        )
    }
}

impl<'emit> BufferTextWindowOutputSession<'emit> {
    pub(crate) fn from_output_state(
        mut output: BufferTextWindowOutputState<'emit>,
        font_metrics: &'emit mut Option<FontMetricsService>,
        face_resolver: &'emit FaceResolver,
        frame_face_id_counter: u32,
    ) -> Self {
        let retry_checkpoint = output.capture_retry_checkpoint();
        Self {
            output,
            font_metrics,
            face_resolver,
            face_ids: FrameFaceIdAllocator::new(frame_face_id_counter),
            retry_checkpoint,
        }
    }

    pub(crate) fn body_pass_state(&mut self) -> BufferTextWindowBodyPassState<'_> {
        BufferTextWindowBodyPassState::new(
            BufferTextWindowBodyOutputRenderState::new(
                self.output.reborrow(),
                self.font_metrics,
                self.face_resolver,
            ),
            &mut self.face_ids,
        )
    }

    pub(crate) fn rendered_body_complete_state<'hit>(
        &'hit mut self,
        hit_data: &'hit mut Vec<WindowHitData>,
        display_snapshots: &'hit mut Vec<WindowDisplaySnapshot>,
    ) -> BufferTextWindowRenderedBodyCompleteState<'hit, 'hit> {
        BufferTextWindowRenderedBodyCompleteState::new(
            self.output.reborrow(),
            ChromeRowRenderServices::new(self.font_metrics, self.face_resolver, &mut self.face_ids),
            hit_data,
            display_snapshots,
        )
    }

    pub(crate) fn initial_active_face_state(
        &mut self,
        request: BufferTextWindowInitialFaceStateRequest<'_>,
    ) -> DisplayRowActiveFaceState {
        request.into_active_face_state(self.font_metrics)
    }

    pub(crate) fn prepare_retry(&mut self, frame_counter: &mut u32) {
        self.output.restore_retry_checkpoint(self.retry_checkpoint);
        self.publish_face_ids(frame_counter);
    }

    pub(crate) fn publish_face_ids(&self, frame_counter: &mut u32) {
        *frame_counter = self.face_ids.finish();
    }
}

impl BufferTextWindowRedisplayPublishRequest {
    pub(crate) fn new(
        frame_id: FrameId,
        window_id: WindowId,
        accessible_end_lisp_char: usize,
        accessible_end_emacs_byte: usize,
    ) -> Self {
        Self {
            frame_id,
            window_id,
            accessible_end_lisp_char,
            accessible_end_emacs_byte,
        }
    }

    pub(crate) fn publish(self, evaluator: &mut Context, positions: TextWindowRedisplayPositions) {
        evaluator.publish_redisplay_window_positions(
            self.frame_id,
            self.window_id,
            positions.window_start,
            LispCharPos1::from_one_based_usize(self.accessible_end_lisp_char),
            EmacsBytePos::new(self.accessible_end_emacs_byte),
            positions.window_end,
            positions.window_end_byte,
            positions.window_end_vpos,
        );
    }
}

impl BufferTextWindowRetryPlan {
    pub(crate) fn from_post_loop(
        window_id: i64,
        window_start: i64,
        point_charpos: i64,
        charpos_end: i64,
        retry_bounds: BufferTextWindowRetryBounds,
        post_loop: BufferTextWindowPostLoopRenderOutcome,
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
