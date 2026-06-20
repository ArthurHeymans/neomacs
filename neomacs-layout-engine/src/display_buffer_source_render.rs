//! Buffer text source consumption with replacement application.

use crate::display_buffer_display_property_render::{
    BufferDisplayPropertyTextReplacementOutcome, BufferDisplayPropertyTextReplacementRenderOutcome,
    BufferDisplayPropertyTextReplacementRenderState,
    BufferDisplayPropertyTextReplacementResolveRequest,
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
use crate::display_buffer_source_walk::BufferSourceWalk;
use crate::display_cursor::capture_cursor_info;
use crate::display_item::BufferDisplayPropertyReplacementItem;
use crate::display_row::{DisplayRowActiveFaceState, DisplayRowFallbackMetrics};
use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_builder::DisplayRowPosition;
use crate::display_row_geometry::{
    DisplayRowGeometryDefaults, DisplayRowLimit, DisplayRowVisibilityLimit,
};
use crate::display_row_overlay_string::{
    BufferOverlayStringTextRowRenderContext, OverlayStringRenderState,
};
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
    let source_step_char = source_item.source_step_char();
    let source_end_charpos = source_item.source_end_charpos();
    let source_end_byte_idx = source_item.source_end_byte_idx();
    let source_item = source_item.into_item();
    render_prepared_source_item_and_apply(
        source_step_char,
        source_end_charpos,
        source_end_byte_idx,
        source_item,
        context,
        source_walk,
        buffer,
        state,
    )
}

fn render_prepared_source_item_and_apply<B: LayoutBufferView>(
    source_step_char: DisplaySourceStepChar,
    source_end_charpos: Option<i64>,
    source_end_byte_idx: Option<usize>,
    source_item: crate::display_item::DisplayItem,
    context: BufferSourceItemRenderContext<'_>,
    source_walk: &mut BufferSourceWalk<'_, B>,
    buffer: &B,
    state: BufferSourceLoopMutableState<'_, '_, '_>,
) -> BufferSourceItemRenderOutcome {
    let mut source_item = source_item;
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
            &mut source_item,
        );
    let ch = source_step_char.ch();
    source_step_char.record_word_wrap_candidate(word_wrap, source_render.output_emitter());

    let buffer_source_char = source_step_char.source_char(context.params.nobreak_char_display);
    let active_face_metrics = active_face_state.metrics();
    let buffer_row_append_context = BufferSourceRowAppendContext::new(
        buffer,
        context.buffer_id,
        context.append_surface,
        &active_face_state,
        context.glyph_y_offset,
        DisplayRowFallbackMetrics::from_default_face_extents(
            active_face_metrics.char_width,
            context.char_h,
            active_face_metrics.ascent,
        ),
    );
    let append_position = DisplayRowPosition {
        x_px: *progress.row.x,
        col: *progress.row.col,
    };
    let append_geometry = *row_geometry;

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
                    *progress.row.x,
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
                *progress.byte_idx = end_byte_idx;
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
        let mut overlay_state = OverlayStringRenderState::from_source_render(
            source_render.reborrow(),
            progress.row.x,
            progress.row.col,
            row_geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
        );
        context.overlay_context.render_at(
            buffer,
            *progress.charpos,
            &active_face_state,
            &mut overlay_state,
        );
    }

    prepared_append.capture_cursor_info_for_main_char_if_point(
        cursor_info,
        &active_face_state,
        row_geometry,
        *progress.row.x,
        source_step_char.start_byte_idx(),
        *progress.row.col,
        ch == '\t',
        *progress.charpos,
        context.point_charpos,
    );
    if cursor_info.is_missing()
        && source_end_charpos.is_some_and(|end| {
            context.point_charpos > *progress.charpos && context.point_charpos < end
        })
    {
        capture_cursor_info(
            cursor_info,
            prepared_append.cursor_info_for_main_char(
                &active_face_state,
                row_geometry.text_position(
                    *progress.row.x,
                    source_step_char.start_byte_idx(),
                    *progress.row.col,
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
        *progress.charpos = (*progress.charpos).max(end_charpos);
    }
    if let Some(end_byte_idx) = source_end_byte_idx {
        *progress.byte_idx = end_byte_idx;
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
        let start_charpos = replacement.start_charpos();
        let active_face_metrics = self.active_face_state.metrics();
        let request = BufferDisplayPropertyTextReplacementResolveRequest::new(
            replacement,
            self.loop_context.text_start_byte(),
            self.text,
            self.loop_context.content_x(),
            self.params,
            0.0,
            DisplayRowFallbackMetrics::from_default_face_extents(
                active_face_metrics.char_width,
                self.loop_context.char_height(),
                active_face_metrics.ascent,
            ),
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
                true
            }
            BufferDisplayPropertyTextReplacementRenderOutcome::Fallback(source_item) => {
                let Some(source_item) =
                    DisplaySourceStepItem::new(source_item, self.loop_context.text_start_byte())
                else {
                    return false;
                };
                self.render_source_item(source_walk, layout_resolution_context, source_item, buffer)
            }
            BufferDisplayPropertyTextReplacementRenderOutcome::Stop => false,
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
                *self.state.progress.byte_idx = end_byte_idx;
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
            BufferSourceItemRenderContext::new(
                layout_resolution_context,
                self.text,
                self.loop_context.text_start_byte(),
                self.loop_context.buffer_id(),
                self.state.append_surface,
                self.state.overlay_context,
                self.active_face_state,
                self.params,
                0.0,
                self.loop_context.char_height(),
                self.loop_context.point_charpos(),
                self.loop_context.row_visibility_limit(),
                self.loop_context.content_x(),
                self.loop_context.has_prefix(),
                self.loop_context.row_geometry_defaults(),
                self.loop_context.display_text_row_base(),
                self.loop_context.max_rows(),
                self.loop_context.row_limit(),
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
