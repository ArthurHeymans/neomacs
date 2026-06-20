//! Buffer text pre-source checkpoint rendering.

use crate::display_buffer_text_face_resolution::BufferCurrentFaceResolutionContext;
use crate::display_buffer_text_loop_context::BufferTextWindowLoopRequestContext;
use crate::display_buffer_text_loop_state::BufferTextWindowLoopMutableState;
use crate::display_buffer_text_row_lifecycle::{
    BufferHscrollSkipRenderRequest, BufferHscrollSkipRenderState, BufferInvisibleTextRenderOutcome,
    BufferInvisibleTextRenderRequest, BufferInvisibleTextRenderRequestState,
};
use crate::display_buffer_text_row_prelude::BufferTextWindowRowPreludeRequestContext;
use crate::display_buffer_text_source_walk::BufferTextWindowSourceWalk;
use crate::display_row::DisplayRowActiveFaceState;
use crate::display_row_transition::DisplayRowTransitionContinuation;
use crate::neovm_bridge::LayoutBufferView;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextWindowPreSourceOutcome {
    ReadyForSourceItem,
    ContinueBufferWalk,
    StopBufferWalk,
}

pub(crate) struct BufferTextWindowPreSourceRenderState<'rows, 'emit, 'surface> {
    loop_context: BufferTextWindowLoopRequestContext,
    state: BufferTextWindowLoopMutableState<'rows, 'emit, 'surface>,
}

impl<'rows, 'emit, 'surface> BufferTextWindowPreSourceRenderState<'rows, 'emit, 'surface> {
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
        row_prelude_context: BufferTextWindowRowPreludeRequestContext,
        face_resolution_context: BufferCurrentFaceResolutionContext<'request, B>,
        text: &'request [u8],
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

        if self.state.hscroll_skip.should_skip() {
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

    fn render_row_prelude<B: LayoutBufferView>(
        &mut self,
        context: BufferTextWindowRowPreludeRequestContext,
        active_face_state: &DisplayRowActiveFaceState,
        buffer: &B,
    ) {
        context
            .line_number_margin_request()
            .render_pending_with_source_state(
                self.state.line_numbers,
                &mut self.state.source_render,
                self.state.face_ids,
                self.state.row_geometry,
                self.state.face_scan,
                context.char_width(),
            );

        context
            .line_prefix_request(
                self.state.append_surface,
                self.state.row_geometry,
                active_face_state,
                0.0,
                self.state.progress.row_position(),
            )
            .render_requested_with_source_state_and_apply(
                self.state.prefix_request,
                &mut self.state.source_render,
                buffer,
                self.state.progress.charpos(),
                self.state.face_ids,
                self.state.progress.row.x,
                self.state.progress.row.col,
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
            self.state.append_surface,
            self.state.overlay_context,
            active_face_state,
            0.0,
        );
        self.render_invisible_text_at_checkpoint(source_walk, request, buffer)
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
                self.state.invisible_text_checkpoint,
                self.state.progress.reborrow(),
                self.state.source_render.reborrow(),
                self.state.row_geometry,
                self.state.cursor_info,
                self.state.hit_rows,
                self.state.hit_row_range,
                self.state.row_y_positions,
                self.state.face_ids,
            ),
        )
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
        let request = self.loop_context.hscroll_skip_request(
            text,
            self.state.append_surface,
            active_face_state,
        );
        self.render_hscroll_skip(source_walk, request)
    }

    fn render_hscroll_skip<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        request: BufferHscrollSkipRenderRequest<'_>,
    ) -> DisplayRowTransitionContinuation {
        request.render_next_and_apply(
            source_walk,
            BufferHscrollSkipRenderState::new(
                self.state.progress.reborrow(),
                self.state.hscroll_skip,
                self.state.row_extend,
                self.state.source_render.reborrow(),
                self.state.prefix_request,
                self.state.line_numbers,
                self.state.word_wrap,
                self.state.trailing_whitespace,
                self.state.row_geometry,
                self.state.row_flags,
                self.state.hit_rows,
                self.state.hit_row_range,
                self.state.cursor_info,
                self.state.row_y_positions,
            ),
        )
    }

    fn render_face_checkpoint_for_context<B: LayoutBufferView>(
        &mut self,
        face_resolution_context: BufferCurrentFaceResolutionContext<'_, B>,
        active_face_state: &mut DisplayRowActiveFaceState,
    ) {
        face_resolution_context.resolve_at_checkpoint_with_source_state(
            &mut self.state.source_render.reborrow(),
            self.state.face_scan,
            self.state.face_ids,
            active_face_state,
            self.state.row_geometry,
            self.state.row_extend,
            self.state.box_face,
            *self.state.progress.row.x,
            self.state.progress.charpos(),
        );
    }
}
