//! Buffer text consumed-item lifecycle rendering.

use crate::display_buffer_text_face_resolution::BufferSourceItemLayoutResolutionContext;
use crate::display_buffer_text_item_append::BufferTextRowAppendState;
use crate::display_buffer_text_loop_context::{
    BufferTextWindowConsumedDisplayItemRenderRequest, BufferTextWindowLoopRequestContext,
};
use crate::display_buffer_text_overflow::{
    BufferTextConsumedDisplayItemRenderOutcome, BufferTextConsumedDisplayItemRenderRequestState,
};
use crate::display_buffer_text_progress::BufferTextWindowProgressState;
use crate::display_buffer_text_row_lifecycle::{
    BufferSelectiveDisplayTailRenderOutcome, BufferSelectiveDisplayTailRenderRequest,
    BufferSelectiveDisplayTailRenderState, BufferTextLineBreakRenderRequest,
    BufferTextLineBreakRenderState,
};
use crate::display_buffer_text_source_consumption::{
    BufferTextConsumedDisplayItem, BufferTextSourceStepChar,
};
use crate::display_buffer_text_source_walk::BufferTextWindowSourceWalk;
use crate::display_cursor::CursorCaptureState;
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_row::DisplayRowActiveFaceState;
use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_geometry::{
    DisplayRowFlags, DisplayRowGeometryState, DisplayRowScopedValue, DisplayRowYPositions,
};
use crate::display_row_lisp_string::DisplayRowPrefixRequest;
use crate::display_row_overlay_string::BufferOverlayStringTextRowRenderContext;
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_row_transition::DisplayRowTransitionContinuation;
use crate::display_row_walk_state::{
    BoxFaceRowState, FaceScanCheckpoint, HitRowRangeTracker, HorizontalScrollSkipState,
    LineNumberRenderState, TrailingWhitespaceRenderState, WordWrapRenderState,
};
use crate::neovm_bridge::LayoutBufferView;
use crate::types::WindowParams;
use neomacs_display_protocol::types::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextWindowConsumedRenderOutcome {
    ContinueBufferWalk,
    StopBufferWalk,
}

pub(crate) struct BufferTextWindowConsumedRenderState<'rows, 'emit, 'surface> {
    loop_context: BufferTextWindowLoopRequestContext,
    append_state: &'emit mut BufferTextRowAppendState,
    progress: BufferTextWindowProgressState<'emit>,
    source_render: TextRowSourceRenderState<'emit>,
    row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    box_face: &'emit mut BoxFaceRowState,
    line_numbers: &'emit mut LineNumberRenderState,
    row_geometry: &'emit mut DisplayRowGeometryState,
    row_flags: &'emit mut DisplayRowFlags,
    hit_rows: &'emit mut Vec<crate::hit_test::HitRow>,
    hit_row_range: &'emit mut HitRowRangeTracker,
    prefix_request: &'emit mut DisplayRowPrefixRequest,
    hscroll_skip: &'emit mut HorizontalScrollSkipState,
    word_wrap: &'emit mut WordWrapRenderState,
    trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    face_scan: &'emit mut FaceScanCheckpoint,
    row_y_positions: &'rows mut DisplayRowYPositions,
    cursor_info: &'emit mut CursorCaptureState,
    face_ids: &'emit mut FrameFaceIdAllocator,
    append_surface: &'surface DisplayRowAppendSurface,
    overlay_context: BufferOverlayStringTextRowRenderContext<'surface>,
}

impl BufferTextWindowConsumedRenderOutcome {
    pub(crate) fn should_stop_buffer_walk(self) -> bool {
        matches!(self, Self::StopBufferWalk)
    }
}

impl<'rows, 'emit, 'surface> BufferTextWindowConsumedRenderState<'rows, 'emit, 'surface> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        loop_context: BufferTextWindowLoopRequestContext,
        append_state: &'emit mut BufferTextRowAppendState,
        progress: BufferTextWindowProgressState<'emit>,
        source_render: TextRowSourceRenderState<'emit>,
        row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &'emit mut BoxFaceRowState,
        line_numbers: &'emit mut LineNumberRenderState,
        row_geometry: &'emit mut DisplayRowGeometryState,
        row_flags: &'emit mut DisplayRowFlags,
        hit_rows: &'emit mut Vec<crate::hit_test::HitRow>,
        hit_row_range: &'emit mut HitRowRangeTracker,
        prefix_request: &'emit mut DisplayRowPrefixRequest,
        hscroll_skip: &'emit mut HorizontalScrollSkipState,
        word_wrap: &'emit mut WordWrapRenderState,
        trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
        face_scan: &'emit mut FaceScanCheckpoint,
        row_y_positions: &'rows mut DisplayRowYPositions,
        cursor_info: &'emit mut CursorCaptureState,
        face_ids: &'emit mut FrameFaceIdAllocator,
        append_surface: &'surface DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'surface>,
    ) -> Self {
        Self {
            loop_context,
            append_state,
            progress,
            source_render,
            row_extend,
            box_face,
            line_numbers,
            row_geometry,
            row_flags,
            hit_rows,
            hit_row_range,
            prefix_request,
            hscroll_skip,
            word_wrap,
            trailing_whitespace,
            face_scan,
            row_y_positions,
            cursor_info,
            face_ids,
            append_surface,
            overlay_context,
        }
    }

    pub(crate) fn render_for_context<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        request: BufferTextWindowConsumedDisplayItemRenderRequest<'request>,
        buffer: &B,
    ) -> BufferTextWindowConsumedRenderOutcome
    where
        'surface: 'request,
    {
        let BufferTextWindowConsumedDisplayItemRenderRequest {
            layout_resolution_context,
            source_item,
            text,
            active_face_state,
            params,
        } = request;
        let selective_display_outcome = self.render_selective_display_tail_for_context(
            source_walk,
            source_item.source_char(),
            text,
            active_face_state,
            buffer,
        );
        if selective_display_outcome.should_break() {
            return BufferTextWindowConsumedRenderOutcome::StopBufferWalk;
        }
        if selective_display_outcome.should_continue_buffer_walk() {
            return BufferTextWindowConsumedRenderOutcome::ContinueBufferWalk;
        }

        let is_explicit_line_break = source_item.is_explicit_line_break();
        let end_charpos = source_item.end_charpos();
        let source_char = source_item.source_char();
        if is_explicit_line_break {
            if self
                .render_line_break_for_context(
                    source_walk,
                    source_char,
                    text,
                    active_face_state,
                    buffer,
                )
                .should_break()
            {
                return BufferTextWindowConsumedRenderOutcome::StopBufferWalk;
            }
        } else {
            let char_render_outcome = self.render_text_consumed_display_item_for_context(
                source_walk,
                layout_resolution_context,
                source_item,
                text,
                active_face_state,
                params,
                buffer,
            );
            if char_render_outcome.should_break() {
                return BufferTextWindowConsumedRenderOutcome::StopBufferWalk;
            }
            if char_render_outcome.should_continue_buffer_walk() {
                return BufferTextWindowConsumedRenderOutcome::ContinueBufferWalk;
            }
            *self.progress.charpos = (*self.progress.charpos).max(end_charpos);
        }

        BufferTextWindowConsumedRenderOutcome::ContinueBufferWalk
    }

    fn render_selective_display_tail_for_context<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        source_step_char: BufferTextSourceStepChar,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
        buffer: &B,
    ) -> BufferSelectiveDisplayTailRenderOutcome
    where
        'surface: 'request,
    {
        let request = self.loop_context.selective_display_tail_request(
            source_step_char,
            text,
            self.append_surface,
            active_face_state,
            0.0,
        );
        self.render_selective_display_tail(source_walk, request, buffer)
    }

    fn render_line_break_for_context<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        source_char: BufferTextSourceStepChar,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
        buffer: &B,
    ) -> DisplayRowTransitionContinuation
    where
        'surface: 'request,
    {
        let request = self.loop_context.line_break_request(
            source_char,
            text,
            self.overlay_context,
            active_face_state,
        );
        self.render_line_break(source_walk, request, buffer)
    }

    fn render_text_consumed_display_item_for_context<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'request>,
        source_item: BufferTextConsumedDisplayItem,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
        params: &'request WindowParams,
        buffer: &B,
    ) -> BufferTextConsumedDisplayItemRenderOutcome
    where
        'surface: 'request,
    {
        let request = self.loop_context.consumed_display_item_request(
            layout_resolution_context,
            source_item,
            text,
            self.append_surface,
            self.overlay_context,
            active_face_state,
            params,
            0.0,
        );
        self.render_text_consumed_display_item(source_walk, request, buffer)
    }

    fn render_selective_display_tail<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        request: BufferSelectiveDisplayTailRenderRequest<'_>,
        buffer: &B,
    ) -> BufferSelectiveDisplayTailRenderOutcome {
        request.render_if_needed_and_apply(
            source_walk,
            buffer,
            BufferSelectiveDisplayTailRenderState::new(
                self.progress.reborrow(),
                self.source_render.reborrow(),
                self.row_extend,
                self.box_face,
                self.line_numbers,
                self.row_geometry,
                self.row_flags,
                self.hit_rows,
                self.hit_row_range,
                self.prefix_request,
                self.hscroll_skip,
                self.word_wrap,
                self.trailing_whitespace,
                self.row_y_positions,
            ),
        )
    }

    fn render_line_break<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        request: BufferTextLineBreakRenderRequest<'_>,
        buffer: &B,
    ) -> DisplayRowTransitionContinuation {
        request.render_and_apply(
            source_walk,
            buffer,
            BufferTextLineBreakRenderState::new(
                self.progress.reborrow(),
                self.cursor_info,
                self.row_geometry,
                self.trailing_whitespace,
                self.row_extend,
                self.box_face,
                self.source_render.reborrow(),
                self.prefix_request,
                self.line_numbers,
                self.hscroll_skip,
                self.word_wrap,
                self.row_flags,
                self.hit_rows,
                self.hit_row_range,
                self.row_y_positions,
                self.face_ids,
            ),
        )
    }

    fn render_text_consumed_display_item<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        request: crate::display_buffer_text_overflow::BufferTextConsumedDisplayItemRenderRequest<
            '_,
        >,
        buffer: &B,
    ) -> BufferTextConsumedDisplayItemRenderOutcome {
        request.render_and_apply(
            source_walk,
            buffer,
            BufferTextConsumedDisplayItemRenderRequestState::new(
                self.append_state,
                self.progress.reborrow(),
                self.source_render.reborrow(),
                self.row_extend,
                self.line_numbers,
                self.row_geometry,
                self.row_flags,
                self.hit_rows,
                self.hit_row_range,
                self.prefix_request,
                self.hscroll_skip,
                self.word_wrap,
                self.trailing_whitespace,
                self.face_scan,
                self.row_y_positions,
                self.cursor_info,
                self.face_ids,
            ),
        )
    }
}
