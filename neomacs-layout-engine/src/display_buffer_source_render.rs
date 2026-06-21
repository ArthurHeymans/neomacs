//! Buffer text source consumption with replacement application.

use crate::display_buffer_display_property_render::{
    BufferDisplayPropertyTextReplacementRenderContext,
    BufferDisplayPropertyTextReplacementRenderOutcome,
    BufferDisplayPropertyTextReplacementRenderState,
};
use crate::display_buffer_source_consumption::BufferSourceConsumedItem;
use crate::display_buffer_source_face_resolution::BufferSourceFaceResolutionContext;
use crate::display_buffer_source_face_resolution::BufferSourceItemLayoutResolutionContext;
use crate::display_buffer_source_item_append::BufferSourceRowAppendContext;
use crate::display_buffer_source_loop_context::BufferSourceLoopRequestContext;
use crate::display_buffer_source_loop_state::BufferSourceLoopMutableState;
use crate::display_buffer_source_overflow::{
    BufferSourceOverflowRenderContext, BufferSourceOverflowRenderRequest,
    BufferSourceSpecialOverflowRenderContext, BufferSourceSpecialOverflowRenderRequest,
};
use crate::display_buffer_source_row_lifecycle::{
    BufferSourceLineBreakRenderRequest, BufferSourceSelectiveDisplayTailRenderOutcome,
    BufferSourceSelectiveDisplayTailRenderRequest,
};
use crate::display_buffer_source_text_run::BufferSourceTextRunRenderRequest;
use crate::display_buffer_source_walk::BufferSourceWalk;
use crate::display_cursor::capture_cursor_info;
use crate::display_item::BufferDisplayPropertyReplacementItem;
use crate::display_row::DisplayRowActiveFaceState;
use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_geometry::{
    DisplayRowGeometryDefaults, DisplayRowLimit, DisplayRowVisibilityLimit,
};
use crate::display_row_overlay_string::BufferOverlayStringTextRowRenderContext;
use crate::display_row_transition::DisplayRowTransitionContinuation;
use crate::display_source::DisplaySourceStepChar;
use crate::display_source::DisplaySourceStepItem;
use crate::display_source_item_append::DisplaySourcePreparedCharAppend;
use crate::neovm_bridge::LayoutBufferView;
use crate::types::WindowParams;
use neovm_core::buffer::BufferId;

#[derive(Clone, Copy)]
struct BufferSourceItemRenderContext<'a> {
    layout_resolution_context: BufferSourceItemLayoutResolutionContext<'a>,
    text: &'a [u8],
    text_start_byte: usize,
    buffer_id: BufferId,
    append_surface: &'a DisplayRowAppendSurface,
    overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
    active_face_state: &'a DisplayRowActiveFaceState,
    params: &'a WindowParams,
    glyph_y_offset: f32,
    char_h: f32,
    point_charpos: i64,
    row_visibility_limit: DisplayRowVisibilityLimit,
    content_x: f32,
    has_prefix: bool,
    row_geometry_defaults: DisplayRowGeometryDefaults,
    display_text_row_base: usize,
    max_rows: usize,
    row_limit: DisplayRowLimit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferSourceItemRenderOutcome {
    Rendered,
    ContinueBufferWalk,
    Stop,
}

impl BufferSourceItemRenderOutcome {
    pub(crate) fn should_break(self) -> bool {
        matches!(self, Self::Stop)
    }
}

pub(crate) struct BufferSourceRenderRequest<'rows, 'request, 'emit, 'surface, 'face> {
    loop_context: BufferSourceLoopRequestContext,
    text: &'request [u8],
    params: &'request WindowParams,
    active_face_state: &'face DisplayRowActiveFaceState,
    state: BufferSourceLoopMutableState<'rows, 'emit, 'surface>,
}

impl<'a> BufferSourceItemRenderContext<'a> {
    fn from_loop_context(
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'a>,
        loop_context: BufferSourceLoopRequestContext,
        text: &'a [u8],
        append_surface: &'a DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
        params: &'a WindowParams,
    ) -> Self {
        Self::new(
            layout_resolution_context,
            text,
            loop_context.text_start_byte(),
            loop_context.buffer_id(),
            append_surface,
            overlay_context,
            active_face_state,
            params,
            0.0,
            loop_context.char_height(),
            loop_context.point_charpos(),
            loop_context.row_visibility_limit(),
            loop_context.content_x(),
            loop_context.has_prefix(),
            loop_context.row_geometry_defaults(),
            loop_context.display_text_row_base(),
            loop_context.max_rows(),
            loop_context.row_limit(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'a>,
        text: &'a [u8],
        text_start_byte: usize,
        buffer_id: BufferId,
        append_surface: &'a DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
        params: &'a WindowParams,
        glyph_y_offset: f32,
        char_h: f32,
        point_charpos: i64,
        row_visibility_limit: DisplayRowVisibilityLimit,
        content_x: f32,
        has_prefix: bool,
        row_geometry_defaults: DisplayRowGeometryDefaults,
        display_text_row_base: usize,
        max_rows: usize,
        row_limit: DisplayRowLimit,
    ) -> Self {
        Self {
            layout_resolution_context,
            text,
            text_start_byte,
            buffer_id,
            append_surface,
            overlay_context,
            active_face_state,
            params,
            glyph_y_offset,
            char_h,
            point_charpos,
            row_visibility_limit,
            content_x,
            has_prefix,
            row_geometry_defaults,
            display_text_row_base,
            max_rows,
            row_limit,
        }
    }
}

fn render_source_item_and_apply<B: LayoutBufferView>(
    source_item: DisplaySourceStepItem,
    context: BufferSourceItemRenderContext<'_>,
    source_walk: &mut BufferSourceWalk<'_, B>,
    buffer: &B,
    state: BufferSourceLoopMutableState<'_, '_, '_>,
) -> BufferSourceItemRenderOutcome {
    debug_assert_ne!(source_item.source_step_char().ch(), '\n');
    render_prepared_source_item_and_apply(source_item, context, source_walk, buffer, state)
}

fn render_prepared_source_item_and_apply<B: LayoutBufferView>(
    mut source_item: DisplaySourceStepItem,
    context: BufferSourceItemRenderContext<'_>,
    source_walk: &mut BufferSourceWalk<'_, B>,
    buffer: &B,
    state: BufferSourceLoopMutableState<'_, '_, '_>,
) -> BufferSourceItemRenderOutcome {
    let BufferSourceLoopMutableState {
        invisible_text_checkpoint,
        mut progress,
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
    } = state;
    let mut source_render = source_render;
    let active_face_state = context
        .layout_resolution_context
        .resolve_source_item_layout_for_active_face(
            &mut source_render,
            face_ids,
            row_geometry,
            context.active_face_state,
            source_item.item_mut(),
        );
    let buffer_row_append_context = BufferSourceRowAppendContext::from_active_face_row(
        buffer,
        context.buffer_id,
        context.append_surface,
        &active_face_state,
        context.glyph_y_offset,
        context.char_h,
    );
    let append_position = progress.row_position();
    let append_geometry = *row_geometry;
    let text_run_request = BufferSourceTextRunRenderRequest::new(
        context.text_start_byte,
        context.overlay_context,
        context.point_charpos,
        context.append_surface.right_edge(),
        append_position,
        append_geometry,
    );

    if let Some((prefix, suffix)) = text_run_request.split_at_first_overlay(&source_item, buffer) {
        source_walk.prepend_pending_render_items(vec![suffix]);
        source_item = prefix;
    }

    if let Some(outcome) = text_run_request.render_if_fits_and_apply(
        source_item.clone(),
        buffer,
        &active_face_state,
        &buffer_row_append_context,
        cursor_info,
        trailing_whitespace,
        word_wrap,
        &mut source_render,
        &mut progress,
    ) {
        return outcome;
    }

    if let Some((prefix, suffix)) = text_run_request.split_prefix_to_fit(
        &source_item,
        context.params.wrap_mode,
        &buffer_row_append_context,
        &mut source_render,
    ) {
        source_walk.prepend_pending_render_items(vec![suffix]);
        return text_run_request.render_and_apply(
            prefix,
            &active_face_state,
            &buffer_row_append_context,
            cursor_info,
            trailing_whitespace,
            word_wrap,
            &mut source_render,
            &mut progress,
        );
    }

    if source_item.is_multi_char_text_run()
        && let Some((first, pending)) = source_item
            .clone()
            .split_text_run_items(context.text_start_byte)
    {
        source_walk.prepend_pending_render_items(pending);
        source_item = first;
    }

    let (source_step_char, source_end_charpos, source_end_byte_idx, source_item) =
        source_item.into_render_parts();
    let ch = source_step_char.ch();
    source_step_char.record_word_wrap_candidate(word_wrap, source_render.output_emitter());

    let buffer_source_char = source_step_char.source_char(context.params.nobreak_char_display);
    let prepared_append = buffer_row_append_context.prepare_source_item_for_current_text_row(
        append_geometry,
        source_walk.append_state(),
        &mut source_render,
        &buffer_source_char,
        context.text,
        source_step_char.start_byte_idx(),
        append_position,
        &source_item,
    );

    let prepared_append = match prepared_append {
        DisplaySourcePreparedCharAppend::Special(special_prepared_append) => {
            let special_overflow_outcome = BufferSourceSpecialOverflowRenderRequest::new(
                &special_prepared_append,
                BufferSourceSpecialOverflowRenderContext::new(
                    context.text,
                    context.text_start_byte,
                    progress.row_progress().x(),
                    context.append_surface.full_text_right_edge(),
                    context.params.wrap_mode,
                    context.row_visibility_limit,
                    context.content_x,
                    context.has_prefix,
                    context.row_geometry_defaults,
                    context.display_text_row_base,
                    context.max_rows,
                    context.row_limit,
                ),
            )
            .render_if_needed_and_apply(
                source_walk,
                buffer,
                BufferSourceLoopMutableState::new(
                    invisible_text_checkpoint,
                    progress.reborrow(),
                    source_render.reborrow(),
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
                ),
            );
            if special_overflow_outcome.should_break() {
                return BufferSourceItemRenderOutcome::Stop;
            }
            if special_overflow_outcome.should_continue_buffer_walk() {
                return BufferSourceItemRenderOutcome::ContinueBufferWalk;
            }

            if special_prepared_append
                .append_to_text_row_and_apply(
                    &buffer_row_append_context,
                    row_geometry,
                    context.params,
                    face_ids,
                    &mut source_render.reborrow(),
                    face_scan,
                    word_wrap,
                    &mut progress.reborrow(),
                )
                .should_break()
            {
                return BufferSourceItemRenderOutcome::Stop;
            }
            if let Some(end_byte_idx) = source_end_byte_idx {
                progress.set_byte_idx(end_byte_idx);
            }
            return BufferSourceItemRenderOutcome::ContinueBufferWalk;
        }
        DisplaySourcePreparedCharAppend::Text(prepared_append) => prepared_append,
    };

    prepared_append
        .update_cursor_info_for_main_char(cursor_info, source_step_char.start_byte_idx());
    let overflow_outcome = BufferSourceOverflowRenderRequest::new(
        &prepared_append,
        source_step_char,
        BufferSourceOverflowRenderContext::new(
            ch,
            context.append_surface.right_edge(),
            context.params.wrap_mode,
            *word_wrap,
            context.row_visibility_limit,
            context.content_x,
            context.has_prefix,
            context.row_geometry_defaults,
            context.display_text_row_base,
            context.max_rows,
            context.row_limit,
        ),
    )
    .render_if_needed_and_apply(
        source_walk,
        context.text,
        BufferSourceLoopMutableState::new(
            invisible_text_checkpoint,
            progress.reborrow(),
            source_render.reborrow(),
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
        ),
    );
    if overflow_outcome.should_break() {
        return BufferSourceItemRenderOutcome::Stop;
    }
    if overflow_outcome.should_continue_buffer_walk() {
        return BufferSourceItemRenderOutcome::ContinueBufferWalk;
    }

    {
        let overlay_charpos = progress.charpos();
        let (x, col) = progress.row_progress_mut().coordinates_mut();
        context.overlay_context.render_at_text_row(
            buffer,
            overlay_charpos,
            &active_face_state,
            source_render.reborrow(),
            x,
            col,
            row_geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
        );
    }

    let row_position = progress.row_position();
    prepared_append.capture_cursor_info_for_main_char_if_point(
        cursor_info,
        &active_face_state,
        row_geometry,
        row_position.x_px(),
        source_step_char.start_byte_idx(),
        row_position.col(),
        ch == '\t',
        progress.charpos(),
        context.point_charpos,
    );
    if cursor_info.is_missing()
        && source_end_charpos.is_some_and(|end| {
            context.point_charpos > progress.charpos() && context.point_charpos < end
        })
    {
        capture_cursor_info(
            cursor_info,
            prepared_append.cursor_info_for_main_char(
                &active_face_state,
                row_geometry.text_position(
                    row_position.x_px(),
                    source_step_char.start_byte_idx(),
                    row_position.col(),
                ),
                ch == '\t',
            ),
        );
    }

    if prepared_append
        .append_to_text_row_and_apply(
            &buffer_row_append_context,
            &append_geometry,
            ch,
            &mut source_render.reborrow(),
            trailing_whitespace,
            word_wrap,
            &mut progress.reborrow(),
        )
        .should_break()
    {
        return BufferSourceItemRenderOutcome::Stop;
    }
    if let Some(end_charpos) = source_end_charpos {
        progress.max_charpos(end_charpos);
    }
    if let Some(end_byte_idx) = source_end_byte_idx {
        progress.set_byte_idx(end_byte_idx);
    }

    BufferSourceItemRenderOutcome::Rendered
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
        match replacement_context.render(
            buffer,
            BufferDisplayPropertyTextReplacementRenderState::new(
                self.state.source_render.reborrow(),
                self.state.face_ids,
                self.state.append_surface,
                self.state.row_geometry,
                self.active_face_state,
            ),
        ) {
            BufferDisplayPropertyTextReplacementRenderOutcome::Rendered(outcome) => {
                replacement_context.apply_rendered_outcome(
                    outcome,
                    &mut self.state.progress,
                    self.state.cursor_info,
                    self.state.row_geometry,
                    self.loop_context.point_charpos(),
                );
                true
            }
            BufferDisplayPropertyTextReplacementRenderOutcome::Fallback(source_item) => {
                self.render_source_item(source_walk, layout_resolution_context, source_item, buffer)
            }
            BufferDisplayPropertyTextReplacementRenderOutcome::Stop => false,
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
        render_source_item_and_apply(
            source_item,
            BufferSourceItemRenderContext::from_loop_context(
                layout_resolution_context,
                self.loop_context,
                self.text,
                self.state.append_surface,
                self.state.overlay_context,
                self.active_face_state,
                self.params,
            ),
            source_walk,
            buffer,
            self.state.reborrow(),
        )
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
