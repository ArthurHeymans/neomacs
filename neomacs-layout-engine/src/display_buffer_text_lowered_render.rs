//! Buffer text lowered-item lifecycle rendering.

use crate::display_buffer_text_face_resolution::BufferSourceItemLayoutResolutionContext;
use crate::display_buffer_text_loop_context::{
    BufferTextWindowLoopRequestContext, BufferTextWindowLoweredDisplayItemRenderRequest,
};
use crate::display_buffer_text_loop_state::BufferTextWindowLoopMutableState;
use crate::display_buffer_text_overflow::{
    BufferTextLoweredDisplayItemRenderOutcome, BufferTextLoweredDisplayItemRenderRequestState,
};
use crate::display_buffer_text_row_lifecycle::{
    BufferSelectiveDisplayTailRenderOutcome, BufferSelectiveDisplayTailRenderRequest,
    BufferSelectiveDisplayTailRenderState, BufferTextLineBreakRenderRequest,
    BufferTextLineBreakRenderState,
};
use crate::display_buffer_text_source_lowering::{
    BufferTextLoweredDisplayItem, BufferTextSourceStepChar,
};
use crate::display_buffer_text_source_walk::BufferTextWindowSourceWalk;
use crate::display_row::DisplayRowActiveFaceState;
use crate::display_row_transition::DisplayRowTransitionContinuation;
use crate::neovm_bridge::LayoutBufferView;
use crate::types::WindowParams;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextWindowLoweredRenderOutcome {
    ContinueBufferWalk,
    StopBufferWalk,
}

pub(crate) struct BufferTextWindowLoweredRenderState<'rows, 'emit, 'surface> {
    loop_context: BufferTextWindowLoopRequestContext,
    state: BufferTextWindowLoopMutableState<'rows, 'emit, 'surface>,
}

impl BufferTextWindowLoweredRenderOutcome {
    pub(crate) fn should_stop_buffer_walk(self) -> bool {
        matches!(self, Self::StopBufferWalk)
    }
}

impl<'rows, 'emit, 'surface> BufferTextWindowLoweredRenderState<'rows, 'emit, 'surface> {
    pub(crate) fn new(
        loop_context: BufferTextWindowLoopRequestContext,
        state: BufferTextWindowLoopMutableState<'rows, 'emit, 'surface>,
    ) -> Self {
        Self {
            loop_context,
            state,
        }
    }

    pub(crate) fn render_for_context<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        request: BufferTextWindowLoweredDisplayItemRenderRequest<'request>,
        buffer: &B,
    ) -> BufferTextWindowLoweredRenderOutcome
    where
        'surface: 'request,
    {
        let BufferTextWindowLoweredDisplayItemRenderRequest {
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
            return BufferTextWindowLoweredRenderOutcome::StopBufferWalk;
        }
        if selective_display_outcome.should_continue_buffer_walk() {
            return BufferTextWindowLoweredRenderOutcome::ContinueBufferWalk;
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
                return BufferTextWindowLoweredRenderOutcome::StopBufferWalk;
            }
        } else {
            let char_render_outcome = self.render_text_lowered_display_item_for_context(
                source_walk,
                layout_resolution_context,
                source_item,
                text,
                active_face_state,
                params,
                buffer,
            );
            if char_render_outcome.should_break() {
                return BufferTextWindowLoweredRenderOutcome::StopBufferWalk;
            }
            if char_render_outcome.should_continue_buffer_walk() {
                return BufferTextWindowLoweredRenderOutcome::ContinueBufferWalk;
            }
            *self.state.progress.charpos = (*self.state.progress.charpos).max(end_charpos);
        }

        BufferTextWindowLoweredRenderOutcome::ContinueBufferWalk
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
            self.state.append_surface,
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
            self.state.overlay_context,
            active_face_state,
        );
        self.render_line_break(source_walk, request, buffer)
    }

    fn render_text_lowered_display_item_for_context<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'request>,
        source_item: BufferTextLoweredDisplayItem,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
        params: &'request WindowParams,
        buffer: &B,
    ) -> BufferTextLoweredDisplayItemRenderOutcome
    where
        'surface: 'request,
    {
        let request = self.loop_context.lowered_display_item_request(
            layout_resolution_context,
            source_item,
            text,
            self.state.append_surface,
            self.state.overlay_context,
            active_face_state,
            params,
            0.0,
        );
        self.render_text_lowered_display_item(source_walk, request, buffer)
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
                self.state.progress.reborrow(),
                self.state.source_render.reborrow(),
                self.state.row_extend,
                self.state.box_face,
                self.state.line_numbers,
                self.state.row_geometry,
                self.state.row_flags,
                self.state.hit_rows,
                self.state.hit_row_range,
                self.state.prefix_request,
                self.state.hscroll_skip,
                self.state.word_wrap,
                self.state.trailing_whitespace,
                self.state.row_y_positions,
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
                self.state.progress.reborrow(),
                self.state.cursor_info,
                self.state.row_geometry,
                self.state.trailing_whitespace,
                self.state.row_extend,
                self.state.box_face,
                self.state.source_render.reborrow(),
                self.state.prefix_request,
                self.state.line_numbers,
                self.state.hscroll_skip,
                self.state.word_wrap,
                self.state.row_flags,
                self.state.hit_rows,
                self.state.hit_row_range,
                self.state.row_y_positions,
                self.state.face_ids,
            ),
        )
    }

    fn render_text_lowered_display_item<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        request: crate::display_buffer_text_overflow::BufferTextLoweredDisplayItemRenderRequest<'_>,
        buffer: &B,
    ) -> BufferTextLoweredDisplayItemRenderOutcome {
        request.render_and_apply(
            source_walk,
            buffer,
            BufferTextLoweredDisplayItemRenderRequestState::new(self.state.reborrow()),
        )
    }
}
