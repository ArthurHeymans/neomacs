//! Buffer text source consumption with replacement application.

use crate::display_buffer_display_property_render::{
    BufferDisplayPropertyTextReplacementApplyOutcome,
    BufferDisplayPropertyTextReplacementRenderContext,
    BufferDisplayPropertyTextReplacementRenderState,
};
use crate::display_buffer_source_consumption::BufferSourceConsumedItem;
use crate::display_buffer_source_face_resolution::BufferSourceFaceResolutionContext;
use crate::display_buffer_source_face_resolution::BufferSourceItemLayoutResolutionContext;
use crate::display_buffer_source_item_render::{
    BufferSourceItemRenderOutcome, BufferSourceItemRenderRequest,
};
use crate::display_buffer_source_loop_context::BufferSourceLoopRequestContext;
use crate::display_buffer_source_loop_state::BufferSourceLoopMutableState;
use crate::display_buffer_source_row_lifecycle::{
    BufferSourceLineBreakRenderRequest, BufferSourceSelectiveDisplayTailRenderOutcome,
    BufferSourceSelectiveDisplayTailRenderRequest,
};
use crate::display_buffer_source_walk::BufferSourceWalk;
use crate::display_item::BufferDisplayPropertyReplacementItem;
use crate::display_row::DisplayRowActiveFaceState;
use crate::display_row_transition::DisplayRowTransitionContinuation;
use crate::display_source::DisplaySourceStepChar;
use crate::display_source::DisplaySourceStepItem;
use crate::neovm_bridge::LayoutBufferView;
use crate::types::WindowParams;

pub(crate) struct BufferSourceRenderRequest<'rows, 'request, 'emit, 'surface, 'face> {
    loop_context: BufferSourceLoopRequestContext,
    text: &'request [u8],
    params: &'request WindowParams,
    active_face_state: &'face DisplayRowActiveFaceState,
    state: BufferSourceLoopMutableState<'rows, 'emit, 'surface>,
}

impl<'rows, 'request, 'emit, 'surface, 'face>
    BufferSourceRenderRequest<'rows, 'request, 'emit, 'surface, 'face>
{
    pub(crate) fn new(
        loop_context: BufferSourceLoopRequestContext,
        text: &'request [u8],
        params: &'request WindowParams,
        active_face_state: &'face DisplayRowActiveFaceState,
        state: BufferSourceLoopMutableState<'rows, 'emit, 'surface>,
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
        source_walk: &mut BufferSourceWalk<'request, B>,
        face_resolution_context: BufferSourceFaceResolutionContext<'request, B>,
        buffer: &B,
    ) -> bool
    where
        'surface: 'request,
    {
        let layout_resolution_context =
            face_resolution_context.source_item_layout_resolution_context();
        let Some(consumed_item) = source_walk.consume_source_item_for_render(
            &mut self.state.progress,
            face_resolution_context,
            self.state.face_ids,
            &mut self.state.source_render.reborrow(),
            self.state.row_geometry,
        ) else {
            return false;
        };

        match consumed_item {
            BufferSourceConsumedItem::DisplayPropertyReplacement(replacement) => self
                .consume_replacement(source_walk, layout_resolution_context, replacement, buffer),
            BufferSourceConsumedItem::Renderable(source_item) => {
                self.render_source_item(source_walk, layout_resolution_context, source_item, buffer)
            }
        }
    }

    fn consume_replacement<B: LayoutBufferView>(
        mut self,
        source_walk: &mut BufferSourceWalk<'request, B>,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'request>,
        replacement: BufferDisplayPropertyReplacementItem,
        buffer: &B,
    ) -> bool
    where
        'surface: 'request,
    {
        let replacement_context = BufferDisplayPropertyTextReplacementRenderContext::new(
            replacement,
            self.loop_context.text_start_byte(),
            self.text,
            self.loop_context.content_x(),
            self.params,
            0.0,
            self.loop_context.char_height(),
            self.active_face_state,
            self.state.progress.row_progress().x(),
            self.state.progress.row_position(),
        );
        match replacement_context.render_and_apply(
            buffer,
            BufferDisplayPropertyTextReplacementRenderState::new(
                self.state.source_render.reborrow(),
                self.state.face_ids,
                self.state.append_surface,
                self.state.row_geometry,
                self.active_face_state,
            ),
            &mut self.state.progress,
            self.state.cursor_info,
            self.loop_context.point_charpos(),
        ) {
            BufferDisplayPropertyTextReplacementApplyOutcome::Applied => true,
            BufferDisplayPropertyTextReplacementApplyOutcome::Fallback(source_item) => {
                self.render_source_item(source_walk, layout_resolution_context, source_item, buffer)
            }
            BufferDisplayPropertyTextReplacementApplyOutcome::Stop => false,
        }
    }

    fn render_source_item<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferSourceWalk<'request, B>,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'request>,
        source_item: DisplaySourceStepItem,
        buffer: &B,
    ) -> bool
    where
        'surface: 'request,
    {
        let source_step_char = source_item.source_step_char();
        let selective_display_outcome =
            self.render_selective_display_tail_for_context(source_walk, source_step_char, buffer);
        if selective_display_outcome.should_break() {
            return false;
        }
        if selective_display_outcome.should_continue_buffer_walk() {
            return true;
        }

        let is_explicit_line_break = source_item.is_explicit_line_break();
        let end_byte_idx = source_item.source_end_byte_idx();
        if is_explicit_line_break {
            if let Some(end_byte_idx) = end_byte_idx {
                self.state.progress.set_byte_idx(end_byte_idx);
            }
            if self
                .render_line_break_for_context(source_walk, source_step_char, buffer)
                .should_break()
            {
                return false;
            }
            return true;
        }

        let char_render_outcome = self.render_text_source_item_for_context(
            source_walk,
            layout_resolution_context,
            source_item,
            buffer,
        );
        if char_render_outcome.should_break() {
            return false;
        }
        true
    }

    fn render_selective_display_tail_for_context<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferSourceWalk<'request, B>,
        source_step_char: DisplaySourceStepChar,
        buffer: &B,
    ) -> BufferSourceSelectiveDisplayTailRenderOutcome
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
        source_walk: &mut BufferSourceWalk<'request, B>,
        source_char: DisplaySourceStepChar,
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
        source_walk: &mut BufferSourceWalk<'request, B>,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'request>,
        source_item: DisplaySourceStepItem,
        buffer: &B,
    ) -> BufferSourceItemRenderOutcome
    where
        'surface: 'request,
    {
        BufferSourceItemRenderRequest::from_loop_context(
            layout_resolution_context,
            self.loop_context,
            self.text,
            self.state.append_surface,
            self.state.overlay_context,
            self.active_face_state,
            self.params,
        )
        .render_and_apply(source_item, source_walk, buffer, self.state.reborrow())
    }

    fn render_selective_display_tail<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferSourceWalk<'request, B>,
        request: BufferSourceSelectiveDisplayTailRenderRequest<'_>,
        buffer: &B,
    ) -> BufferSourceSelectiveDisplayTailRenderOutcome {
        request.render_if_needed_and_apply(source_walk, buffer, self.state.reborrow())
    }

    fn render_line_break<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferSourceWalk<'request, B>,
        request: BufferSourceLineBreakRenderRequest<'_>,
        buffer: &B,
    ) -> DisplayRowTransitionContinuation {
        request.render_and_apply(source_walk, buffer, self.state.reborrow())
    }
}
