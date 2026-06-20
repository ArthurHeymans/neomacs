//! Buffer text visible-loop rendering.

use crate::display_buffer_display_property_render::{
    BufferDisplayPropertyTextReplacementApplyOutcome,
    BufferDisplayPropertyTextReplacementRenderState,
    BufferDisplayPropertyTextReplacementResolveRequest,
};
use crate::display_buffer_display_property_source::BufferTextReplacementItem;
use crate::display_buffer_text_face_resolution::*;
use crate::display_buffer_text_item_append::BufferTextRowAppendState;
use crate::display_buffer_text_loop_context::{
    BufferTextWindowConsumedDisplayItemRenderRequest, BufferTextWindowLoopRequestContext,
};
use crate::display_buffer_text_overflow::*;
use crate::display_buffer_text_progress::BufferTextWindowProgressState;
use crate::display_buffer_text_row_lifecycle::*;
use crate::display_buffer_text_row_prelude::BufferTextWindowRowPreludeRequestContext;
use crate::display_buffer_text_source_walk::*;
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
    InvisibleTextScanCheckpoint, LineNumberRenderState, TrailingWhitespaceRenderState,
    WordWrapRenderState,
};
use crate::hit_test::HitRow;
use crate::neovm_bridge::LayoutBufferView;
use crate::types::WindowParams;
use neomacs_display_protocol::types::Color;

pub(crate) struct BufferTextWindowLoopRenderState<'rows, 'emit, 'surface> {
    loop_context: BufferTextWindowLoopRequestContext,
    append_state: &'emit mut BufferTextRowAppendState,
    invisible_text_checkpoint: &'emit mut InvisibleTextScanCheckpoint,
    progress: BufferTextWindowProgressState<'emit>,
    source_render: TextRowSourceRenderState<'emit>,
    row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    box_face: &'emit mut BoxFaceRowState,
    line_numbers: &'emit mut LineNumberRenderState,
    row_geometry: &'emit mut DisplayRowGeometryState,
    row_flags: &'emit mut DisplayRowFlags,
    hit_rows: &'emit mut Vec<HitRow>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BufferTextWindowLoopStepOutcome {
    ContinueBufferWalk,
    StopBufferWalk,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BufferTextWindowPreSourceOutcome {
    ReadyForSourceItem,
    ContinueBufferWalk,
    StopBufferWalk,
}

impl<'rows, 'emit, 'surface> BufferTextWindowLoopRenderState<'rows, 'emit, 'surface> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        loop_context: BufferTextWindowLoopRequestContext,
        append_state: &'emit mut BufferTextRowAppendState,
        invisible_text_checkpoint: &'emit mut InvisibleTextScanCheckpoint,
        progress: BufferTextWindowProgressState<'emit>,
        source_render: TextRowSourceRenderState<'emit>,
        row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &'emit mut BoxFaceRowState,
        line_numbers: &'emit mut LineNumberRenderState,
        row_geometry: &'emit mut DisplayRowGeometryState,
        row_flags: &'emit mut DisplayRowFlags,
        hit_rows: &'emit mut Vec<HitRow>,
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
            invisible_text_checkpoint,
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

    fn hscroll_should_skip(&self) -> bool {
        self.hscroll_skip.should_skip()
    }

    pub(crate) fn render_visible_steps<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'request, B>,
        row_prelude_context: BufferTextWindowRowPreludeRequestContext,
        face_resolution_context: BufferCurrentFaceResolutionContext<'request, B>,
        text: &'request [u8],
        params: &'request WindowParams,
        active_face_state: &mut DisplayRowActiveFaceState,
        buffer: &B,
    ) where
        'surface: 'request,
    {
        while *self.progress.byte_idx < text.len()
            && self
                .row_geometry
                .current_row_is_visible(self.loop_context.row_visibility_limit())
        {
            if matches!(
                self.render_next_step(
                    source_walk,
                    row_prelude_context,
                    face_resolution_context.clone(),
                    text,
                    params,
                    active_face_state,
                    buffer,
                ),
                BufferTextWindowLoopStepOutcome::StopBufferWalk
            ) {
                break;
            }
        }
    }

    fn render_next_step<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'request, B>,
        row_prelude_context: BufferTextWindowRowPreludeRequestContext,
        face_resolution_context: BufferCurrentFaceResolutionContext<'request, B>,
        text: &'request [u8],
        params: &'request WindowParams,
        active_face_state: &mut DisplayRowActiveFaceState,
        buffer: &B,
    ) -> BufferTextWindowLoopStepOutcome
    where
        'surface: 'request,
    {
        match self.render_pre_source_checkpoints_for_context(
            source_walk,
            row_prelude_context,
            face_resolution_context,
            text,
            params,
            active_face_state,
            buffer,
        ) {
            BufferTextWindowPreSourceOutcome::ReadyForSourceItem => {}
            BufferTextWindowPreSourceOutcome::ContinueBufferWalk => {
                return BufferTextWindowLoopStepOutcome::ContinueBufferWalk;
            }
            BufferTextWindowPreSourceOutcome::StopBufferWalk => {
                return BufferTextWindowLoopStepOutcome::StopBufferWalk;
            }
        }

        // One persistent typed source cursor feeds the row walk. The source
        // side owns pending text-run splitting so direct single-character
        // items can still stay typed through render.
        let Some(source_consumption) = source_walk.consume_source_item_for_render(
            &mut self.progress,
            face_resolution_context,
            self.face_ids,
            &mut self.source_render.reborrow(),
            self.row_geometry,
        ) else {
            return BufferTextWindowLoopStepOutcome::StopBufferWalk;
        };

        match source_consumption {
            BufferTextSourceConsumptionItem::DisplayItem(source_item) => {
                return self.render_consumed_display_item_for_context(
                    source_walk,
                    BufferTextWindowConsumedDisplayItemRenderRequest {
                        layout_resolution_context: face_resolution_context
                            .source_item_layout_resolution_context(),
                        source_item,
                        text,
                        active_face_state,
                        params,
                    },
                    buffer,
                );
            }
            BufferTextSourceConsumptionItem::Replacement(replacement) => {
                return self.render_replacement_source_item_for_context(
                    replacement,
                    source_walk,
                    face_resolution_context.source_item_layout_resolution_context(),
                    text,
                    active_face_state,
                    params,
                    buffer,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_pre_source_checkpoints_for_context<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        row_prelude_context: BufferTextWindowRowPreludeRequestContext,
        face_resolution_context: BufferCurrentFaceResolutionContext<'request, B>,
        text: &'request [u8],
        _params: &'request WindowParams,
        active_face_state: &mut DisplayRowActiveFaceState,
        buffer: &B,
    ) -> BufferTextWindowPreSourceOutcome
    where
        'surface: 'request,
    {
        self.render_row_prelude(row_prelude_context, active_face_state, buffer);

        if self
            .render_invisible_text_for_context(source_walk, text, active_face_state, buffer)
            .should_continue_buffer_walk()
        {
            return BufferTextWindowPreSourceOutcome::ContinueBufferWalk;
        }

        if self.hscroll_should_skip() {
            if self
                .render_hscroll_skip_for_context(source_walk, text, active_face_state)
                .should_break()
            {
                return BufferTextWindowPreSourceOutcome::StopBufferWalk;
            }
            return BufferTextWindowPreSourceOutcome::ContinueBufferWalk;
        }

        self.render_face_checkpoint_for_context(face_resolution_context, active_face_state);

        BufferTextWindowPreSourceOutcome::ReadyForSourceItem
    }

    fn render_consumed_display_item_for_context<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        request: BufferTextWindowConsumedDisplayItemRenderRequest<'request>,
        buffer: &B,
    ) -> BufferTextWindowLoopStepOutcome
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
            return BufferTextWindowLoopStepOutcome::StopBufferWalk;
        }
        if selective_display_outcome.should_continue_buffer_walk() {
            return BufferTextWindowLoopStepOutcome::ContinueBufferWalk;
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
                return BufferTextWindowLoopStepOutcome::StopBufferWalk;
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
                return BufferTextWindowLoopStepOutcome::StopBufferWalk;
            }
            if char_render_outcome.should_continue_buffer_walk() {
                return BufferTextWindowLoopStepOutcome::ContinueBufferWalk;
            }
            *self.progress.charpos = (*self.progress.charpos).max(end_charpos);
        }

        BufferTextWindowLoopStepOutcome::ContinueBufferWalk
    }

    fn render_replacement_source_item_for_context<'request, B: LayoutBufferView>(
        &mut self,
        replacement: BufferTextReplacementItem,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'request>,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
        params: &'request WindowParams,
        buffer: &B,
    ) -> BufferTextWindowLoopStepOutcome
    where
        'surface: 'request,
    {
        let request = BufferDisplayPropertyTextReplacementResolveRequest::new(
            replacement,
            self.loop_context.text_start_byte(),
            text,
            self.loop_context.content_x(),
            params,
            0.0,
            self.loop_context.char_height(),
            active_face_state,
            self.loop_context.point_charpos(),
        );
        match request.resolve_and_apply(
            buffer,
            BufferDisplayPropertyTextReplacementRenderState::new(
                text,
                self.source_render.reborrow(),
                self.face_ids,
                self.append_surface,
                self.row_geometry,
                self.cursor_info,
                active_face_state,
                self.progress.reborrow(),
            ),
        ) {
            BufferDisplayPropertyTextReplacementApplyOutcome::Rendered => {
                BufferTextWindowLoopStepOutcome::ContinueBufferWalk
            }
            BufferDisplayPropertyTextReplacementApplyOutcome::Fallback(source_item) => {
                let Some(source_step) = source_walk
                    .consume_fallback_source_item_for_render(source_item, &mut self.progress)
                else {
                    return BufferTextWindowLoopStepOutcome::StopBufferWalk;
                };
                self.render_consumed_display_item_for_context(
                    source_walk,
                    BufferTextWindowConsumedDisplayItemRenderRequest {
                        layout_resolution_context,
                        source_item: source_step,
                        text,
                        active_face_state,
                        params,
                    },
                    buffer,
                )
            }
            BufferDisplayPropertyTextReplacementApplyOutcome::Stop => {
                BufferTextWindowLoopStepOutcome::StopBufferWalk
            }
        }
    }

    pub(crate) fn render_row_prelude<B: LayoutBufferView>(
        &mut self,
        context: BufferTextWindowRowPreludeRequestContext,
        active_face_state: &DisplayRowActiveFaceState,
        buffer: &B,
    ) {
        context
            .line_number_margin_request()
            .render_pending_with_source_state(
                self.line_numbers,
                &mut self.source_render,
                self.face_ids,
                self.row_geometry,
                self.face_scan,
                context.char_width(),
            );

        context
            .line_prefix_request(
                self.append_surface,
                self.row_geometry,
                active_face_state,
                0.0,
                self.progress.row_position(),
            )
            .render_requested_with_source_state_and_apply(
                self.prefix_request,
                &mut self.source_render,
                buffer,
                self.progress.charpos(),
                self.face_ids,
                self.progress.row.x,
                self.progress.row.col,
            );
    }

    fn render_invisible_text_for_context<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
        buffer: &B,
    ) -> BufferInvisibleTextRenderOutcome
    where
        'surface: 'request,
    {
        let request = self.loop_context.invisible_text_request(
            text,
            self.append_surface,
            self.overlay_context,
            active_face_state,
            0.0,
        );
        self.render_invisible_text_at_checkpoint(source_walk, request, buffer)
    }

    fn render_hscroll_skip_for_context<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
    ) -> DisplayRowTransitionContinuation
    where
        'surface: 'request,
    {
        let request =
            self.loop_context
                .hscroll_skip_request(text, self.append_surface, active_face_state);
        self.render_hscroll_skip(source_walk, request)
    }

    fn render_face_checkpoint_for_context<B: LayoutBufferView>(
        &mut self,
        face_resolution_context: BufferCurrentFaceResolutionContext<'_, B>,
        active_face_state: &mut DisplayRowActiveFaceState,
    ) {
        face_resolution_context.resolve_at_checkpoint_with_source_state(
            &mut self.source_render.reborrow(),
            self.face_scan,
            self.face_ids,
            active_face_state,
            self.row_geometry,
            self.row_extend,
            self.box_face,
            *self.progress.row.x,
            self.progress.charpos(),
        );
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

    pub(crate) fn render_line_break_for_context<'request, B: LayoutBufferView>(
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

    fn render_invisible_text_at_checkpoint<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        request: BufferInvisibleTextRenderRequest<'_>,
        buffer: &B,
    ) -> BufferInvisibleTextRenderOutcome {
        request.render_at_checkpoint_and_apply(
            source_walk,
            buffer,
            BufferInvisibleTextRenderRequestState::new(
                self.invisible_text_checkpoint,
                self.progress.reborrow(),
                self.source_render.reborrow(),
                self.row_geometry,
                self.cursor_info,
                self.hit_rows,
                self.hit_row_range,
                self.row_y_positions,
                self.face_ids,
            ),
        )
    }

    fn render_hscroll_skip<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        request: BufferHscrollSkipRenderRequest<'_>,
    ) -> DisplayRowTransitionContinuation {
        request.render_next_and_apply(
            source_walk,
            BufferHscrollSkipRenderState::new(
                self.progress.reborrow(),
                self.hscroll_skip,
                self.row_extend,
                self.source_render.reborrow(),
                self.prefix_request,
                self.line_numbers,
                self.word_wrap,
                self.trailing_whitespace,
                self.row_geometry,
                self.row_flags,
                self.hit_rows,
                self.hit_row_range,
                self.cursor_info,
                self.row_y_positions,
            ),
        )
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
        request: BufferTextConsumedDisplayItemRenderRequest<'_>,
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
