//! Buffer text overflow and consumed-item rendering.
//!
//! This module owns the buffer-specific overflow lifecycle for main text and
//! special display items while delegating actual item appends to the shared row
//! source append pipeline.

use crate::display_buffer_text_face_resolution::BufferSourceItemLayoutResolutionContext;
use crate::display_buffer_text_item_append::{
    BufferTextPreparedSourceCharAppend, BufferTextRowAppendContext,
    BufferTextSourceCharPreparationState, BufferTextSourceCharPreparedAppend,
    BufferTextSourceDisplayItemPreparationRequest, BufferTextSpecialSourceCharPreparedAppend,
};
use crate::display_buffer_text_loop_state::BufferTextWindowLoopMutableState;
use crate::display_buffer_text_progress::BufferTextWindowProgressState;
use crate::display_buffer_text_source::BufferTextSourcePosition;
use crate::display_buffer_text_source_consumption::{
    BufferTextConsumedDisplayItem, BufferTextSourceStepChar,
};
use crate::display_buffer_text_source_walk::BufferTextWindowSourceWalk;
use crate::display_cursor::capture_cursor_info;
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_row::DisplayRowActiveFaceState;
use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_builder::DisplayRowPosition;
use crate::display_row_geometry::{
    DisplayRowGeometryDefaults, DisplayRowGeometryState, DisplayRowHitRange, DisplayRowLimit,
    DisplayRowScopedValue, DisplayRowVisibilityLimit,
};
use crate::display_row_overlay_string::{
    BufferOverlayStringTextRowRenderContext, OverlayStringRenderState,
};
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_row_transition::{
    DisplayRowOverflowTransitionPlan, DisplayRowTextWindowEmitContext,
    DisplayRowTransitionContinuation, DisplayRowTransitionRenderState,
};
use crate::display_row_walk_state::{
    BufferTextRowOverflowDecision, FaceScanCheckpoint, HitRowRangeTracker, LineNumberRenderState,
    SpecialTextRowOverflowDecision, TextRowTransitionStatePolicy, TrailingWhitespaceRenderState,
    WordWrapBreakCandidate, WordWrapRenderState,
};
use crate::neovm_bridge::LayoutBufferView;
use crate::types::{LineWrapMode, WindowParams};
use crate::window_output::{DisplayTextRowTransition, WindowOutputEmitter};
use neomacs_display_protocol::types::Color;
use neovm_core::buffer::{BufferId, EmacsBytePos};

pub(crate) struct BufferTextOverflowRenderState<'rows, 'emit, 'surface> {
    state: BufferTextWindowLoopMutableState<'rows, 'emit, 'surface>,
}

pub(crate) struct BufferTextSpecialOverflowRenderState<'rows, 'emit, 'surface> {
    state: BufferTextWindowLoopMutableState<'rows, 'emit, 'surface>,
}

pub(crate) struct BufferTextConsumedDisplayItemRenderRequest<'a> {
    source_item: BufferTextConsumedDisplayItem,
    context: BufferTextConsumedDisplayItemRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferTextConsumedDisplayItemRenderContext<'a> {
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

pub(crate) struct BufferTextConsumedDisplayItemRenderRequestState<'rows, 'emit, 'surface> {
    state: BufferTextWindowLoopMutableState<'rows, 'emit, 'surface>,
}

impl<'rows, 'emit, 'surface> BufferTextOverflowRenderState<'rows, 'emit, 'surface> {
    pub(crate) fn new(state: BufferTextWindowLoopMutableState<'rows, 'emit, 'surface>) -> Self {
        Self { state }
    }
}

impl<'rows, 'emit, 'surface> BufferTextSpecialOverflowRenderState<'rows, 'emit, 'surface> {
    pub(crate) fn new(state: BufferTextWindowLoopMutableState<'rows, 'emit, 'surface>) -> Self {
        Self { state }
    }
}

impl<'rows, 'emit, 'surface>
    BufferTextConsumedDisplayItemRenderRequestState<'rows, 'emit, 'surface>
{
    pub(crate) fn new(state: BufferTextWindowLoopMutableState<'rows, 'emit, 'surface>) -> Self {
        Self { state }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum BufferTextSourceCharOverflowAction {
    Fits,
    Truncate {
        transition: DisplayRowOverflowTransitionPlan,
    },
    WordWrap {
        break_candidate: WordWrapBreakCandidate,
        transition: DisplayRowOverflowTransitionPlan,
    },
    CharacterWrap {
        transition: DisplayRowOverflowTransitionPlan,
    },
}

impl BufferTextSourceCharOverflowAction {
    pub(crate) fn for_decision(decision: BufferTextRowOverflowDecision) -> Self {
        match decision {
            BufferTextRowOverflowDecision::Fits => Self::Fits,
            BufferTextRowOverflowDecision::Truncate => Self::Truncate {
                transition: DisplayRowOverflowTransitionPlan::truncation(
                    TextRowTransitionStatePolicy::truncation(),
                ),
            },
            BufferTextRowOverflowDecision::WordWrap { break_candidate } => Self::WordWrap {
                break_candidate,
                transition: DisplayRowOverflowTransitionPlan::visual_wrap(
                    TextRowTransitionStatePolicy::visual_wrap(),
                ),
            },
            BufferTextRowOverflowDecision::CharacterWrap => Self::CharacterWrap {
                transition: DisplayRowOverflowTransitionPlan::visual_wrap(
                    TextRowTransitionStatePolicy::character_wrap(),
                ),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextOverflowRenderRequest<'a> {
    prepared_append: &'a BufferTextSourceCharPreparedAppend,
    source_step_char: BufferTextSourceStepChar,
    context: BufferTextOverflowRenderContext,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextOverflowRenderContext {
    ch: char,
    right_edge_px: f32,
    wrap_mode: LineWrapMode,
    word_wrap: WordWrapRenderState,
    row_visibility_limit: DisplayRowVisibilityLimit,
    content_x: f32,
    has_prefix: bool,
    row_geometry_defaults: DisplayRowGeometryDefaults,
    display_text_row_base: usize,
    max_rows: usize,
    row_limit: DisplayRowLimit,
}

impl BufferTextOverflowRenderContext {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        ch: char,
        right_edge_px: f32,
        wrap_mode: LineWrapMode,
        word_wrap: WordWrapRenderState,
        row_visibility_limit: DisplayRowVisibilityLimit,
        content_x: f32,
        has_prefix: bool,
        row_geometry_defaults: DisplayRowGeometryDefaults,
        display_text_row_base: usize,
        max_rows: usize,
        row_limit: DisplayRowLimit,
    ) -> Self {
        Self {
            ch,
            right_edge_px,
            wrap_mode,
            word_wrap,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextOverflowRenderOutcome {
    Fits,
    Transition(DisplayRowTransitionContinuation),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextSourceAppendContinuation {
    Rendered,
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextConsumedDisplayItemRenderOutcome {
    Rendered,
    ContinueBufferWalk,
    Stop,
}

pub(crate) struct BufferTextConsumedDisplayItemRenderState<'a> {
    pub(crate) source_render: TextRowSourceRenderState<'a>,
    pub(crate) trailing_whitespace: &'a mut TrailingWhitespaceRenderState,
    pub(crate) word_wrap: &'a mut WordWrapRenderState,
    pub(crate) progress: BufferTextWindowProgressState<'a>,
}

impl<'a> BufferTextConsumedDisplayItemRenderState<'a> {
    pub(crate) fn new(
        source_render: TextRowSourceRenderState<'a>,
        trailing_whitespace: &'a mut TrailingWhitespaceRenderState,
        word_wrap: &'a mut WordWrapRenderState,
        progress: BufferTextWindowProgressState<'a>,
    ) -> Self {
        Self {
            source_render,
            trailing_whitespace,
            word_wrap,
            progress,
        }
    }
}

pub(crate) struct BufferTextSpecialSourceCharRenderState<'a> {
    pub(crate) face_ids: &'a mut FrameFaceIdAllocator,
    pub(crate) source_render: TextRowSourceRenderState<'a>,
    pub(crate) face_scan: &'a mut FaceScanCheckpoint,
    pub(crate) word_wrap: &'a mut WordWrapRenderState,
    pub(crate) progress: BufferTextWindowProgressState<'a>,
}

impl<'a> BufferTextSpecialSourceCharRenderState<'a> {
    pub(crate) fn new(
        face_ids: &'a mut FrameFaceIdAllocator,
        source_render: TextRowSourceRenderState<'a>,
        face_scan: &'a mut FaceScanCheckpoint,
        word_wrap: &'a mut WordWrapRenderState,
        progress: BufferTextWindowProgressState<'a>,
    ) -> Self {
        Self {
            face_ids,
            source_render,
            face_scan,
            word_wrap,
            progress,
        }
    }
}

impl BufferTextSourceAppendContinuation {
    pub(crate) fn should_break(self) -> bool {
        matches!(self, Self::Stopped)
    }
}

impl BufferTextConsumedDisplayItemRenderOutcome {
    pub(crate) fn should_break(self) -> bool {
        matches!(self, Self::Stop)
    }

    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(self, Self::ContinueBufferWalk)
    }
}

impl BufferTextOverflowRenderOutcome {
    pub(crate) fn should_break(self) -> bool {
        matches!(
            self,
            Self::Transition(
                DisplayRowTransitionContinuation::Exhausted
                    | DisplayRowTransitionContinuation::Hidden
            )
        )
    }

    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(
            self,
            Self::Transition(DisplayRowTransitionContinuation::Continue)
        )
    }
}

impl<'a> BufferTextConsumedDisplayItemRenderContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
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

impl<'a> BufferTextConsumedDisplayItemRenderRequest<'a> {
    pub(crate) fn new(
        source_item: BufferTextConsumedDisplayItem,
        context: BufferTextConsumedDisplayItemRenderContext<'a>,
    ) -> Self {
        debug_assert_ne!(source_item.source_char().ch(), '\n');
        Self {
            source_item,
            context,
        }
    }

    pub(crate) fn render_and_apply<B: LayoutBufferView>(
        self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        buffer: &B,
        state: BufferTextConsumedDisplayItemRenderRequestState<'_, '_, '_>,
    ) -> BufferTextConsumedDisplayItemRenderOutcome {
        let BufferTextConsumedDisplayItemRenderRequestState { state } = state;
        let BufferTextWindowLoopMutableState {
            append_state,
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
        let context = self.context;
        let (source_step_char, mut source_item) = self.source_item.into_parts();
        let active_face_state = context
            .layout_resolution_context
            .resolve_source_item_layout_for_active_face(
                &mut source_render,
                face_ids,
                row_geometry,
                context.active_face_state,
                &mut source_item,
            );
        let source_end_charpos = source_item
            .span
            .buffer_end_charpos()
            .map(|char_pos| char_pos.get() as i64);

        let ch = source_step_char.ch();
        source_step_char.record_word_wrap_candidate(word_wrap, source_render.output_emitter());

        let buffer_source_char = source_step_char.source_char(context.params.nobreak_char_display);
        let buffer_row_append_context = BufferTextRowAppendContext::new(
            buffer,
            context.buffer_id,
            context.append_surface,
            &active_face_state,
            context.glyph_y_offset,
            context.char_h,
        );
        let append_position = DisplayRowPosition {
            x_px: *progress.row.x,
            col: *progress.row.col,
        };
        let append_geometry = *row_geometry;

        let prepared_append = {
            let mut preparation_state = BufferTextSourceCharPreparationState::from_source_render(
                append_state,
                &mut source_render,
            );
            buffer_row_append_context.prepare_source_item_for_current_text_row(
                BufferTextSourceDisplayItemPreparationRequest::new(
                    append_geometry,
                    &buffer_source_char,
                    context.text,
                    source_step_char.start_byte_idx(),
                    append_position,
                    &source_item,
                ),
                &mut preparation_state,
            )
        };

        let prepared_append = match prepared_append {
            BufferTextPreparedSourceCharAppend::Special(special_prepared_append) => {
                let special_overflow_outcome = BufferTextSpecialOverflowRenderRequest::new(
                    &special_prepared_append,
                    BufferTextSpecialOverflowRenderContext::new(
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
                    BufferTextSpecialOverflowRenderState::new(
                        BufferTextWindowLoopMutableState::new(
                            append_state,
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
                    ),
                );
                if special_overflow_outcome.should_break() {
                    return BufferTextConsumedDisplayItemRenderOutcome::Stop;
                }
                if special_overflow_outcome.should_continue_buffer_walk() {
                    return BufferTextConsumedDisplayItemRenderOutcome::ContinueBufferWalk;
                }

                if special_prepared_append
                    .append_to_text_row_and_apply(
                        &buffer_row_append_context,
                        row_geometry,
                        context.params,
                        &mut BufferTextSpecialSourceCharRenderState::new(
                            face_ids,
                            source_render.reborrow(),
                            face_scan,
                            word_wrap,
                            progress.reborrow(),
                        ),
                    )
                    .should_break()
                {
                    return BufferTextConsumedDisplayItemRenderOutcome::Stop;
                }
                return BufferTextConsumedDisplayItemRenderOutcome::ContinueBufferWalk;
            }
            BufferTextPreparedSourceCharAppend::Text(prepared_append) => prepared_append,
        };

        prepared_append
            .update_cursor_info_for_main_char(cursor_info, source_step_char.start_byte_idx());
        let overflow_outcome = BufferTextOverflowRenderRequest::new(
            &prepared_append,
            source_step_char,
            BufferTextOverflowRenderContext::new(
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
            BufferTextOverflowRenderState::new(BufferTextWindowLoopMutableState::new(
                append_state,
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
            )),
        );
        if overflow_outcome.should_break() {
            return BufferTextConsumedDisplayItemRenderOutcome::Stop;
        }
        if overflow_outcome.should_continue_buffer_walk() {
            return BufferTextConsumedDisplayItemRenderOutcome::ContinueBufferWalk;
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
                &mut BufferTextConsumedDisplayItemRenderState::new(
                    source_render.reborrow(),
                    trailing_whitespace,
                    word_wrap,
                    progress.reborrow(),
                ),
            )
            .should_break()
        {
            return BufferTextConsumedDisplayItemRenderOutcome::Stop;
        }
        if let Some(end_charpos) = source_end_charpos {
            *progress.charpos = (*progress.charpos).max(end_charpos);
        }

        BufferTextConsumedDisplayItemRenderOutcome::Rendered
    }
}

impl<'a> BufferTextOverflowRenderRequest<'a> {
    pub(crate) fn new(
        prepared_append: &'a BufferTextSourceCharPreparedAppend,
        source_step_char: BufferTextSourceStepChar,
        context: BufferTextOverflowRenderContext,
    ) -> Self {
        Self {
            prepared_append,
            source_step_char,
            context,
        }
    }

    pub(crate) fn render_if_needed_and_apply<B: LayoutBufferView>(
        self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        text: &[u8],
        state: BufferTextOverflowRenderState<'_, '_, '_>,
    ) -> BufferTextOverflowRenderOutcome {
        let BufferTextOverflowRenderState { state } = state;
        let BufferTextWindowLoopMutableState {
            mut progress,
            source_render,
            row_extend,
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
            ..
        } = state;
        let mut source_render = source_render;
        let context = self.context;

        match self.prepared_append.overflow_action(
            context.ch,
            context.right_edge_px,
            context.wrap_mode,
            context.word_wrap,
        ) {
            BufferTextSourceCharOverflowAction::Fits => BufferTextOverflowRenderOutcome::Fits,
            BufferTextSourceCharOverflowAction::Truncate { transition } => {
                let truncation_skip = source_walk
                    .consume_truncation_skip(text, progress.source_position())
                    .apply_to_progress(&mut progress);
                truncation_skip.apply_before_row_transition(
                    line_numbers,
                    row_extend,
                    progress.row.x,
                    context.content_x,
                );
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    context.row_geometry_defaults,
                    context.display_text_row_base,
                    row_y_positions,
                    context.max_rows,
                    row_geometry,
                    row_flags,
                    context.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow_then_row_start(
                    transition,
                    hit_row_range.range_to(progress.charpos()),
                    DisplayRowPosition {
                        x_px: *progress.row.x,
                        col: *progress.row.col,
                    },
                    DisplayRowTransitionRenderState::new(
                        prefix_request,
                        context.has_prefix,
                        line_numbers,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                    ),
                    progress.row.col,
                );
                BufferTextOverflowRenderOutcome::Transition(
                    truncation_skip.transition_continuation(row_transition),
                )
            }
            BufferTextSourceCharOverflowAction::WordWrap {
                break_candidate: wrap_break,
                transition,
            } => {
                let word_wrap_action = BufferTextWordWrapSourceAction::new(wrap_break);
                let mut source_position = progress.source_position();
                word_wrap_action.apply_before_row_transition(
                    source_render.output_emitter(),
                    &mut source_position,
                    progress.row.col,
                    row_extend,
                    progress.row.x,
                    context.content_x,
                );
                source_walk
                    .source_position_update(source_position)
                    .apply_to_progress(&mut progress);
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    context.row_geometry_defaults,
                    context.display_text_row_base,
                    row_y_positions,
                    context.max_rows,
                    row_geometry,
                    row_flags,
                    context.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow(
                    transition,
                    hit_row_range.range_to(progress.charpos()),
                    DisplayRowPosition {
                        x_px: *progress.row.x,
                        col: *progress.row.col,
                    },
                );
                let continuation = word_wrap_action.apply_after_row_transition_and_prefix(
                    row_transition,
                    transition,
                    &mut source_position,
                    hit_row_range,
                    face_scan,
                    row_geometry,
                    context.row_visibility_limit,
                    DisplayRowTransitionRenderState::new(
                        prefix_request,
                        context.has_prefix,
                        line_numbers,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                    ),
                );
                source_walk
                    .source_position_update(source_position)
                    .apply_to_progress(&mut progress);
                BufferTextOverflowRenderOutcome::Transition(continuation)
            }
            BufferTextSourceCharOverflowAction::CharacterWrap { transition } => {
                let character_wrap_action =
                    BufferTextCharacterWrapSourceAction::from_source_step_char(
                        self.source_step_char,
                    );
                character_wrap_action.apply_before_row_transition(
                    row_extend,
                    progress.row.x,
                    context.content_x,
                );
                let mut source_position = progress.source_position();
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    context.row_geometry_defaults,
                    context.display_text_row_base,
                    row_y_positions,
                    context.max_rows,
                    row_geometry,
                    row_flags,
                    context.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow_then_row_start(
                    transition,
                    hit_row_range.range_to(progress.charpos()),
                    DisplayRowPosition {
                        x_px: *progress.row.x,
                        col: *progress.row.col,
                    },
                    DisplayRowTransitionRenderState::new(
                        prefix_request,
                        context.has_prefix,
                        line_numbers,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                    ),
                    progress.row.col,
                );
                let continuation = character_wrap_action.apply_after_visible_row_transition(
                    row_transition,
                    &mut source_position,
                    hit_row_range,
                    face_scan,
                    row_geometry,
                    context.row_visibility_limit,
                );
                source_walk
                    .source_position_update(source_position)
                    .apply_to_progress(&mut progress);
                BufferTextOverflowRenderOutcome::Transition(continuation)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextTruncationSkipAction {
    pub(crate) charpos: i64,
    pub(crate) reached_line_break: bool,
    pub(crate) source_position: BufferTextSourcePosition,
}

impl BufferTextTruncationSkipAction {
    pub(crate) fn consume_source_step_char_and_rest_of_line(
        text: &[u8],
        position: &mut BufferTextSourcePosition,
    ) -> Self {
        position.advance_charpos_by_one();
        let mut reached_line_break = false;
        while position.byte_idx() < text.len() {
            let Some(source_char) = BufferTextSourceStepChar::consume_from_position(text, position)
            else {
                break;
            };
            if source_char.ch() == '\n' {
                reached_line_break = true;
                break;
            }
        }
        Self {
            charpos: position.charpos(),
            reached_line_break,
            source_position: *position,
        }
    }

    #[cfg(test)]
    pub(crate) fn charpos(self) -> i64 {
        self.charpos
    }

    pub(crate) fn source_position(self) -> BufferTextSourcePosition {
        self.source_position
    }

    pub(crate) fn reached_line_break(self) -> bool {
        self.reached_line_break
    }

    pub(crate) fn apply_before_row_transition(
        self,
        line_numbers: &mut LineNumberRenderState,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        x: &mut f32,
        content_x: f32,
    ) {
        if self.reached_line_break() {
            line_numbers.advance_line();
        }
        *x = content_x;
        row_extend.clear();
    }

    pub(crate) fn sync_after_row_transition(
        synced_charpos: i64,
        position: &mut BufferTextSourcePosition,
        hit_row_range: &mut HitRowRangeTracker,
    ) {
        *position = position.with_charpos(synced_charpos);
        hit_row_range.advance_to(position.charpos());
    }

    pub(crate) fn transition_continuation(
        self,
        row_transition: DisplayTextRowTransition,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            DisplayRowTransitionContinuation::Exhausted
        } else {
            DisplayRowTransitionContinuation::Continue
        }
    }

    pub(crate) fn sync_after_row_transition_if_visible(
        self,
        row_transition: DisplayTextRowTransition,
        synced_charpos: i64,
        position: &mut BufferTextSourcePosition,
        hit_row_range: &mut HitRowRangeTracker,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        Self::sync_after_row_transition(synced_charpos, position, hit_row_range);
        DisplayRowTransitionContinuation::Continue
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWordWrapSourceAction {
    break_candidate: WordWrapBreakCandidate,
}

impl BufferTextWordWrapSourceAction {
    pub(crate) fn new(break_candidate: WordWrapBreakCandidate) -> Self {
        Self { break_candidate }
    }

    pub(crate) fn restore_row_output_progress(self, output_emitter: &mut WindowOutputEmitter) {
        output_emitter.truncate_display_points(self.break_candidate.display_point_count());
        let (row_first_display_pos, row_last_display_pos) =
            self.break_candidate.row_display_positions();
        output_emitter
            .restore_current_row_display_positions(row_first_display_pos, row_last_display_pos);
    }

    pub(crate) fn source_position(self) -> BufferTextSourcePosition {
        BufferTextSourcePosition::new(
            self.break_candidate.byte_idx(),
            self.break_candidate.charpos(),
        )
    }

    pub(crate) fn rewind_source_state(
        self,
        position: &mut BufferTextSourcePosition,
        col: &mut usize,
    ) {
        *position = self.source_position();
        *col = 0;
    }

    pub(crate) fn apply_before_row_transition(
        self,
        output_emitter: &mut WindowOutputEmitter,
        position: &mut BufferTextSourcePosition,
        col: &mut usize,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        x: &mut f32,
        content_x: f32,
    ) {
        self.restore_row_output_progress(output_emitter);
        self.rewind_source_state(position, col);
        *x = content_x;
        row_extend.clear();
    }

    pub(crate) fn apply_after_row_transition(
        self,
        position: &mut BufferTextSourcePosition,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
    ) {
        *position = self.source_position();
        hit_row_range.advance_to(position.charpos());
        face_scan.invalidate();
    }

    pub(crate) fn apply_after_row_transition_and_prefix(
        self,
        row_transition: DisplayTextRowTransition,
        transition: DisplayRowOverflowTransitionPlan,
        position: &mut BufferTextSourcePosition,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
        row_geometry: &DisplayRowGeometryState,
        row_visibility_limit: DisplayRowVisibilityLimit,
        render_state: DisplayRowTransitionRenderState<'_>,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        self.apply_after_row_transition(position, hit_row_range, face_scan);
        render_state.apply_overflow_prefix(transition);
        DisplayRowTransitionContinuation::after_visible_row_transition(
            row_transition,
            row_geometry,
            row_visibility_limit,
        )
    }

    #[cfg(test)]
    pub(crate) fn byte_idx(self) -> usize {
        self.break_candidate.byte_idx()
    }

    #[cfg(test)]
    pub(crate) fn charpos(self) -> i64 {
        self.break_candidate.charpos()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextSpecialWrapSourceAction {
    charpos: i64,
}

impl BufferTextSpecialWrapSourceAction {
    pub(crate) fn new(charpos: i64) -> Self {
        Self { charpos }
    }

    pub(crate) fn apply_before_row_transition(
        self,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        x: &mut f32,
        content_x: f32,
    ) {
        *x = content_x;
        row_extend.clear();
    }

    pub(crate) fn hit_range_and_advance(
        self,
        hit_row_range: &mut HitRowRangeTracker,
    ) -> DisplayRowHitRange {
        let hit_range = hit_row_range.range_to(self.charpos);
        hit_row_range.advance_to(self.charpos);
        hit_range
    }

    pub(crate) fn transition_continuation(
        self,
        row_transition: DisplayTextRowTransition,
        row_geometry: &DisplayRowGeometryState,
        row_visibility_limit: DisplayRowVisibilityLimit,
    ) -> DisplayRowTransitionContinuation {
        DisplayRowTransitionContinuation::after_visible_row_transition(
            row_transition,
            row_geometry,
            row_visibility_limit,
        )
    }

    #[cfg(test)]
    pub(crate) fn charpos(self) -> i64 {
        self.charpos
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextCharacterWrapSourceAction {
    ch_start_byte_idx: usize,
    ch_start_charpos: i64,
}

impl BufferTextCharacterWrapSourceAction {
    pub(crate) fn new(ch_start_byte_idx: usize, ch_start_charpos: i64) -> Self {
        Self {
            ch_start_byte_idx,
            ch_start_charpos,
        }
    }

    pub(crate) fn from_source_step_char(source_char: BufferTextSourceStepChar) -> Self {
        Self::new(source_char.start_byte_idx(), source_char.start_charpos())
    }

    pub(crate) fn source_position(self) -> BufferTextSourcePosition {
        BufferTextSourcePosition::new(self.ch_start_byte_idx, self.ch_start_charpos)
    }

    pub(crate) fn rewind_source_state(self, position: &mut BufferTextSourcePosition) {
        *position = self.source_position();
    }

    pub(crate) fn apply_before_row_transition(
        self,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        x: &mut f32,
        content_x: f32,
    ) {
        *x = content_x;
        row_extend.clear();
    }

    pub(crate) fn apply_after_row_transition(
        self,
        position: &mut BufferTextSourcePosition,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
    ) {
        self.rewind_source_state(position);
        hit_row_range.advance_to(position.charpos());
        face_scan.invalidate();
    }

    pub(crate) fn apply_after_visible_row_transition(
        self,
        row_transition: DisplayTextRowTransition,
        position: &mut BufferTextSourcePosition,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
        row_geometry: &DisplayRowGeometryState,
        row_visibility_limit: DisplayRowVisibilityLimit,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        self.apply_after_row_transition(position, hit_row_range, face_scan);
        DisplayRowTransitionContinuation::after_visible_row_transition(
            row_transition,
            row_geometry,
            row_visibility_limit,
        )
    }
}

pub(crate) struct BufferTextSpecialOverflowRenderRequest<'a> {
    prepared_append: &'a BufferTextSpecialSourceCharPreparedAppend,
    context: BufferTextSpecialOverflowRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferTextSpecialOverflowRenderContext<'a> {
    text: &'a [u8],
    text_start_byte: usize,
    x_px: f32,
    right_edge_px: f32,
    wrap_mode: LineWrapMode,
    row_visibility_limit: DisplayRowVisibilityLimit,
    content_x: f32,
    has_prefix: bool,
    row_geometry_defaults: DisplayRowGeometryDefaults,
    display_text_row_base: usize,
    max_rows: usize,
    row_limit: DisplayRowLimit,
}

impl<'a> BufferTextSpecialOverflowRenderContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        text: &'a [u8],
        text_start_byte: usize,
        x_px: f32,
        right_edge_px: f32,
        wrap_mode: LineWrapMode,
        row_visibility_limit: DisplayRowVisibilityLimit,
        content_x: f32,
        has_prefix: bool,
        row_geometry_defaults: DisplayRowGeometryDefaults,
        display_text_row_base: usize,
        max_rows: usize,
        row_limit: DisplayRowLimit,
    ) -> Self {
        Self {
            text,
            text_start_byte,
            x_px,
            right_edge_px,
            wrap_mode,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextSpecialOverflowRenderOutcome {
    Fits,
    AppendPrepared(DisplayRowTransitionContinuation),
    ContinueBufferWalk(DisplayRowTransitionContinuation),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum BufferTextSpecialSourceCharOverflowAction {
    Fits,
    Truncate {
        transition: DisplayRowOverflowTransitionPlan,
    },
    Wrap {
        transition: DisplayRowOverflowTransitionPlan,
    },
}

impl BufferTextSpecialSourceCharOverflowAction {
    pub(crate) fn for_decision(decision: SpecialTextRowOverflowDecision) -> Self {
        match decision {
            SpecialTextRowOverflowDecision::Fits => Self::Fits,
            SpecialTextRowOverflowDecision::Truncate => Self::Truncate {
                transition: DisplayRowOverflowTransitionPlan::truncation(
                    TextRowTransitionStatePolicy::special_truncation(),
                ),
            },
            SpecialTextRowOverflowDecision::Wrap => Self::Wrap {
                transition: DisplayRowOverflowTransitionPlan::visual_wrap(
                    TextRowTransitionStatePolicy::special_visual_wrap(),
                ),
            },
        }
    }
}

impl BufferTextSpecialOverflowRenderOutcome {
    pub(crate) fn should_break(self) -> bool {
        matches!(
            self,
            Self::AppendPrepared(
                DisplayRowTransitionContinuation::Exhausted
                    | DisplayRowTransitionContinuation::Hidden
            ) | Self::ContinueBufferWalk(
                DisplayRowTransitionContinuation::Exhausted
                    | DisplayRowTransitionContinuation::Hidden
            )
        )
    }

    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(
            self,
            Self::ContinueBufferWalk(DisplayRowTransitionContinuation::Continue)
        )
    }
}

impl<'a> BufferTextSpecialOverflowRenderRequest<'a> {
    pub(crate) fn new(
        prepared_append: &'a BufferTextSpecialSourceCharPreparedAppend,
        context: BufferTextSpecialOverflowRenderContext<'a>,
    ) -> Self {
        Self {
            prepared_append,
            context,
        }
    }

    pub(crate) fn render_if_needed_and_apply<B: LayoutBufferView>(
        self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        buffer: &B,
        state: BufferTextSpecialOverflowRenderState<'_, '_, '_>,
    ) -> BufferTextSpecialOverflowRenderOutcome {
        let BufferTextSpecialOverflowRenderState { state } = state;
        let BufferTextWindowLoopMutableState {
            mut progress,
            source_render,
            row_extend,
            line_numbers,
            row_geometry,
            row_flags,
            hit_rows,
            hit_row_range,
            prefix_request,
            hscroll_skip,
            word_wrap,
            trailing_whitespace,
            row_y_positions,
            ..
        } = state;
        let mut source_render = source_render;
        let context = self.context;

        match self.prepared_append.overflow_action(
            context.x_px,
            context.right_edge_px,
            context.wrap_mode,
        ) {
            None | Some(BufferTextSpecialSourceCharOverflowAction::Fits) => {
                BufferTextSpecialOverflowRenderOutcome::Fits
            }
            Some(BufferTextSpecialSourceCharOverflowAction::Truncate { transition }) => {
                let truncation_skip = source_walk
                    .consume_truncation_skip(context.text, progress.source_position())
                    .apply_to_progress(&mut progress);
                let mut source_position = truncation_skip.source_position();
                truncation_skip.apply_before_row_transition(
                    line_numbers,
                    row_extend,
                    progress.row.x,
                    context.content_x,
                );
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    context.row_geometry_defaults,
                    context.display_text_row_base,
                    row_y_positions,
                    context.max_rows,
                    row_geometry,
                    row_flags,
                    context.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow_then_row_start(
                    transition,
                    hit_row_range.range_to(progress.charpos()),
                    DisplayRowPosition {
                        x_px: *progress.row.x,
                        col: *progress.row.col,
                    },
                    DisplayRowTransitionRenderState::new(
                        prefix_request,
                        context.has_prefix,
                        line_numbers,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                    ),
                    progress.row.col,
                );
                let synced_charpos = buffer
                    .layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(
                        context.text_start_byte + source_position.byte_idx(),
                    ))
                    .get() as i64;
                let continuation = truncation_skip.sync_after_row_transition_if_visible(
                    row_transition,
                    synced_charpos,
                    &mut source_position,
                    hit_row_range,
                );
                source_walk
                    .source_position_update(source_position)
                    .apply_to_progress(&mut progress);
                BufferTextSpecialOverflowRenderOutcome::ContinueBufferWalk(continuation)
            }
            Some(BufferTextSpecialSourceCharOverflowAction::Wrap { transition }) => {
                let special_wrap_action =
                    BufferTextSpecialWrapSourceAction::new(progress.charpos());
                special_wrap_action.apply_before_row_transition(
                    row_extend,
                    progress.row.x,
                    context.content_x,
                );
                let hit_range = special_wrap_action.hit_range_and_advance(hit_row_range);
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    context.row_geometry_defaults,
                    context.display_text_row_base,
                    row_y_positions,
                    context.max_rows,
                    row_geometry,
                    row_flags,
                    context.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow_then_row_start(
                    transition,
                    hit_range,
                    DisplayRowPosition {
                        x_px: *progress.row.x,
                        col: *progress.row.col,
                    },
                    DisplayRowTransitionRenderState::new(
                        prefix_request,
                        context.has_prefix,
                        line_numbers,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                    ),
                    progress.row.col,
                );
                BufferTextSpecialOverflowRenderOutcome::AppendPrepared(
                    special_wrap_action.transition_continuation(
                        row_transition,
                        row_geometry,
                        context.row_visibility_limit,
                    ),
                )
            }
        }
    }
}
