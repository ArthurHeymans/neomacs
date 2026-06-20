//! Buffer text source consumption with replacement application.

use crate::display_buffer_display_property_render::{
    BufferDisplayPropertyTextReplacementOutcome, BufferDisplayPropertyTextReplacementRenderOutcome,
    BufferDisplayPropertyTextReplacementRenderState,
    BufferDisplayPropertyTextReplacementResolveRequest,
};
use crate::display_buffer_display_property_source::BufferTextReplacementItem;
use crate::display_buffer_text_face_resolution::BufferCurrentFaceResolutionContext;
use crate::display_buffer_text_face_resolution::BufferSourceItemLayoutResolutionContext;
use crate::display_buffer_text_loop_context::BufferTextWindowLoopRequestContext;
use crate::display_buffer_text_loop_state::BufferTextWindowLoopMutableState;
use crate::display_buffer_text_overflow::{
    BufferTextSourceItemRenderOutcome, BufferTextSourceItemRenderRequestState,
};
use crate::display_buffer_text_row_lifecycle::{
    BufferSelectiveDisplayTailRenderOutcome, BufferSelectiveDisplayTailRenderRequest,
    BufferSelectiveDisplayTailRenderState, BufferTextLineBreakRenderRequest,
    BufferTextLineBreakRenderState,
};
use crate::display_buffer_text_source::BufferTextSourceStepChar;
use crate::display_buffer_text_source_consumption::{
    BufferTextSourceConsumptionItem, BufferTextSourceItem,
};
use crate::display_buffer_text_source_walk::BufferTextWindowSourceWalk;
use crate::display_row::DisplayRowActiveFaceState;
use crate::display_row_transition::DisplayRowTransitionContinuation;
use crate::neovm_bridge::LayoutBufferView;
use crate::types::WindowParams;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BufferTextWindowSourceRenderOutcome {
    ContinueBufferWalk,
    StopBufferWalk,
}

pub(crate) struct BufferTextWindowSourceRenderRequest<'rows, 'request, 'emit, 'surface, 'face> {
    loop_context: BufferTextWindowLoopRequestContext,
    text: &'request [u8],
    params: &'request WindowParams,
    active_face_state: &'face DisplayRowActiveFaceState,
    state: BufferTextWindowLoopMutableState<'rows, 'emit, 'surface>,
}

impl BufferTextWindowSourceRenderOutcome {
    pub(crate) fn should_stop_buffer_walk(&self) -> bool {
        matches!(self, Self::StopBufferWalk)
    }
}

impl<'rows, 'request, 'emit, 'surface, 'face>
    BufferTextWindowSourceRenderRequest<'rows, 'request, 'emit, 'surface, 'face>
{
    pub(crate) fn new(
        loop_context: BufferTextWindowLoopRequestContext,
        text: &'request [u8],
        params: &'request WindowParams,
        active_face_state: &'face DisplayRowActiveFaceState,
        state: BufferTextWindowLoopMutableState<'rows, 'emit, 'surface>,
    ) -> Self {
        Self {
            loop_context,
            text,
            params,
            active_face_state,
            state,
        }
    }

    pub(crate) fn render_next_and_apply<B: LayoutBufferView>(
        mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'request, B>,
        face_resolution_context: BufferCurrentFaceResolutionContext<'request, B>,
        buffer: &B,
    ) -> BufferTextWindowSourceRenderOutcome
    where
        'surface: 'request,
    {
        let layout_resolution_context =
            face_resolution_context.source_item_layout_resolution_context();
        let Some(source_item) = source_walk.consume_source_item_for_render(
            &mut self.state.progress,
            face_resolution_context,
            self.state.face_ids,
            &mut self.state.source_render.reborrow(),
            self.state.row_geometry,
        ) else {
            return BufferTextWindowSourceRenderOutcome::StopBufferWalk;
        };

        match source_item {
            BufferTextSourceConsumptionItem::DisplayItem(source_item) => {
                self.render_source_item(source_walk, layout_resolution_context, source_item, buffer)
            }
            BufferTextSourceConsumptionItem::Replacement(replacement) => self.consume_replacement(
                source_walk,
                layout_resolution_context,
                replacement,
                buffer,
            ),
        }
    }

    fn consume_replacement<B: LayoutBufferView>(
        mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'request, B>,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'request>,
        replacement: BufferTextReplacementItem,
        buffer: &B,
    ) -> BufferTextWindowSourceRenderOutcome
    where
        'surface: 'request,
    {
        let start_charpos = replacement.start_charpos();
        let request = BufferDisplayPropertyTextReplacementResolveRequest::new(
            replacement,
            self.loop_context.text_start_byte(),
            self.text,
            self.loop_context.content_x(),
            self.params,
            0.0,
            self.loop_context.char_height(),
            self.active_face_state,
        );
        let current_x = *self.state.progress.row.x;
        let start_position = self.state.progress.row_position();
        match request.resolve_and_render(
            buffer,
            BufferDisplayPropertyTextReplacementRenderState::new(
                self.state.source_render.reborrow(),
                self.state.face_ids,
                self.state.append_surface,
                self.state.row_geometry,
                self.active_face_state,
            ),
            current_x,
            start_position,
        ) {
            BufferDisplayPropertyTextReplacementRenderOutcome::Rendered(outcome) => {
                self.apply_replacement_outcome(outcome, start_charpos);
                BufferTextWindowSourceRenderOutcome::ContinueBufferWalk
            }
            BufferDisplayPropertyTextReplacementRenderOutcome::Fallback(source_item) => {
                self.render_source_item(source_walk, layout_resolution_context, source_item, buffer)
            }
            BufferDisplayPropertyTextReplacementRenderOutcome::Stop => {
                BufferTextWindowSourceRenderOutcome::StopBufferWalk
            }
        }
    }

    fn apply_replacement_outcome(
        &mut self,
        outcome: BufferDisplayPropertyTextReplacementOutcome,
        start_charpos: i64,
    ) {
        outcome.capture_cursor_info_if_point(
            self.state.cursor_info,
            self.active_face_state,
            self.state.row_geometry,
            self.loop_context.point_charpos(),
            start_charpos,
            *self.state.progress.byte_idx,
        );
        let walk_update = outcome.walk_update(self.text, self.state.progress.source_position());
        self.state
            .progress
            .row
            .apply_position(walk_update.row_position());
        self.state
            .progress
            .apply_source_position(walk_update.source_position());
    }

    fn render_source_item<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'request, B>,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'request>,
        source_item: BufferTextSourceItem,
        buffer: &B,
    ) -> BufferTextWindowSourceRenderOutcome
    where
        'surface: 'request,
    {
        let Some(source_step_char) = source_item.source_step_char() else {
            return BufferTextWindowSourceRenderOutcome::StopBufferWalk;
        };
        let selective_display_outcome =
            self.render_selective_display_tail_for_context(source_walk, source_step_char, buffer);
        if selective_display_outcome.should_break() {
            return BufferTextWindowSourceRenderOutcome::StopBufferWalk;
        }
        if selective_display_outcome.should_continue_buffer_walk() {
            return BufferTextWindowSourceRenderOutcome::ContinueBufferWalk;
        }

        let is_explicit_line_break = source_item.is_explicit_line_break();
        let end_byte_idx = source_item.end_byte_idx(self.loop_context.text_start_byte());
        if is_explicit_line_break {
            if let Some(end_byte_idx) = end_byte_idx {
                *self.state.progress.byte_idx = end_byte_idx;
            }
            if self
                .render_line_break_for_context(source_walk, source_step_char, buffer)
                .should_break()
            {
                return BufferTextWindowSourceRenderOutcome::StopBufferWalk;
            }
            return BufferTextWindowSourceRenderOutcome::ContinueBufferWalk;
        }

        let char_render_outcome = self.render_text_source_item_for_context(
            source_walk,
            layout_resolution_context,
            source_item,
            buffer,
        );
        if char_render_outcome.should_break() {
            return BufferTextWindowSourceRenderOutcome::StopBufferWalk;
        }
        BufferTextWindowSourceRenderOutcome::ContinueBufferWalk
    }

    fn render_selective_display_tail_for_context<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'request, B>,
        source_step_char: BufferTextSourceStepChar,
        buffer: &B,
    ) -> BufferSelectiveDisplayTailRenderOutcome
    where
        'surface: 'request,
    {
        let request = self.loop_context.selective_display_tail_request(
            source_step_char,
            self.text,
            self.state.append_surface,
            self.active_face_state,
            0.0,
        );
        self.render_selective_display_tail(source_walk, request, buffer)
    }

    fn render_line_break_for_context<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'request, B>,
        source_char: BufferTextSourceStepChar,
        buffer: &B,
    ) -> DisplayRowTransitionContinuation
    where
        'surface: 'request,
    {
        let request = self.loop_context.line_break_request(
            source_char,
            self.text,
            self.state.overlay_context,
            self.active_face_state,
        );
        self.render_line_break(source_walk, request, buffer)
    }

    fn render_text_source_item_for_context<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'request, B>,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'request>,
        source_item: BufferTextSourceItem,
        buffer: &B,
    ) -> BufferTextSourceItemRenderOutcome
    where
        'surface: 'request,
    {
        let request = self.loop_context.source_item_request(
            layout_resolution_context,
            source_item,
            self.text,
            self.state.append_surface,
            self.state.overlay_context,
            self.active_face_state,
            self.params,
            0.0,
        );
        request.render_and_apply(
            source_walk,
            buffer,
            BufferTextSourceItemRenderRequestState::new(self.state.reborrow()),
        )
    }

    fn render_selective_display_tail<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'request, B>,
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
        source_walk: &mut BufferTextWindowSourceWalk<'request, B>,
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
}
