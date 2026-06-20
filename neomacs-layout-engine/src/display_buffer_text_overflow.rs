//! Buffer text overflow rendering.
//!
//! This module owns the buffer-specific overflow lifecycle for main text and
//! special display items while delegating actual item appends to the shared row
//! source append pipeline.

use crate::display_buffer_text_loop_state::BufferTextWindowLoopMutableState;
use crate::display_buffer_text_source_walk::BufferTextWindowSourceWalk;
use crate::display_row_builder::DisplayRowPosition;
use crate::display_row_geometry::{
    DisplayRowGeometryDefaults, DisplayRowGeometryState, DisplayRowHitRange, DisplayRowLimit,
    DisplayRowScopedValue, DisplayRowVisibilityLimit,
};
use crate::display_row_transition::{
    DisplayRowOverflowTransitionPlan, DisplayRowTextWindowEmitContext,
    DisplayRowTransitionContinuation, DisplayRowTransitionRenderState,
};
use crate::display_row_walk_state::{
    FaceScanCheckpoint, HitRowRangeTracker, LineNumberRenderState, WordWrapBreakCandidate,
    WordWrapRenderState,
};
use crate::display_source::{DisplaySourceStepChar, DisplaySourceTextPosition};
use crate::display_source_item_append::{
    DisplaySourceSpecialCharPreparedAppend, DisplaySourceTextCharPreparedAppend,
};
use crate::display_source_overflow::{
    DisplaySourceSpecialCharOverflowAction, DisplaySourceTextCharOverflowAction,
};
use crate::neovm_bridge::LayoutBufferView;
use crate::types::LineWrapMode;
use crate::window_output::{DisplayTextRowTransition, WindowOutputEmitter};
use neomacs_display_protocol::types::Color;
use neovm_core::buffer::EmacsBytePos;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextOverflowRenderRequest<'a> {
    prepared_append: &'a DisplaySourceTextCharPreparedAppend,
    source_step_char: DisplaySourceStepChar,
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

impl<'a> BufferTextOverflowRenderRequest<'a> {
    pub(crate) fn new(
        prepared_append: &'a DisplaySourceTextCharPreparedAppend,
        source_step_char: DisplaySourceStepChar,
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
        state: BufferTextWindowLoopMutableState<'_, '_, '_>,
    ) -> BufferTextOverflowRenderOutcome {
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
            DisplaySourceTextCharOverflowAction::Fits => BufferTextOverflowRenderOutcome::Fits,
            DisplaySourceTextCharOverflowAction::Truncate { transition } => {
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
            DisplaySourceTextCharOverflowAction::WordWrap {
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
            DisplaySourceTextCharOverflowAction::CharacterWrap { transition } => {
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
    pub(crate) source_position: DisplaySourceTextPosition,
}

impl BufferTextTruncationSkipAction {
    pub(crate) fn consume_source_step_char_and_rest_of_line(
        text: &[u8],
        position: &mut DisplaySourceTextPosition,
    ) -> Self {
        let reached_line_break = position.consume_one_then_until_line_break(text);
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

    pub(crate) fn source_position(self) -> DisplaySourceTextPosition {
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
        position: &mut DisplaySourceTextPosition,
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
        position: &mut DisplaySourceTextPosition,
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

    pub(crate) fn source_position(self) -> DisplaySourceTextPosition {
        DisplaySourceTextPosition::new(
            self.break_candidate.byte_idx(),
            self.break_candidate.charpos(),
        )
    }

    pub(crate) fn rewind_source_state(
        self,
        position: &mut DisplaySourceTextPosition,
        col: &mut usize,
    ) {
        *position = self.source_position();
        *col = 0;
    }

    pub(crate) fn apply_before_row_transition(
        self,
        output_emitter: &mut WindowOutputEmitter,
        position: &mut DisplaySourceTextPosition,
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
        position: &mut DisplaySourceTextPosition,
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
        position: &mut DisplaySourceTextPosition,
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

    pub(crate) fn from_source_step_char(source_char: DisplaySourceStepChar) -> Self {
        Self::new(source_char.start_byte_idx(), source_char.start_charpos())
    }

    pub(crate) fn source_position(self) -> DisplaySourceTextPosition {
        DisplaySourceTextPosition::new(self.ch_start_byte_idx, self.ch_start_charpos)
    }

    pub(crate) fn rewind_source_state(self, position: &mut DisplaySourceTextPosition) {
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
        position: &mut DisplaySourceTextPosition,
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
        position: &mut DisplaySourceTextPosition,
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
    prepared_append: &'a DisplaySourceSpecialCharPreparedAppend,
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
        prepared_append: &'a DisplaySourceSpecialCharPreparedAppend,
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
        state: BufferTextWindowLoopMutableState<'_, '_, '_>,
    ) -> BufferTextSpecialOverflowRenderOutcome {
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
            None | Some(DisplaySourceSpecialCharOverflowAction::Fits) => {
                BufferTextSpecialOverflowRenderOutcome::Fits
            }
            Some(DisplaySourceSpecialCharOverflowAction::Truncate { transition }) => {
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
            Some(DisplaySourceSpecialCharOverflowAction::Wrap { transition }) => {
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
