//! Buffer-text source rendering requests and actions.

use crate::display_buffer_text_source::{
    BufferTextDecodedSourceChar, BufferTextLineBreakSourceEvent,
};
use crate::display_cursor::{
    CapturedCursorInfo, CapturedCursorPlacement, CapturedCursorSlotWidth, CursorCaptureState,
    capture_cursor_info,
};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_row::DisplayRowActiveFaceState;
use crate::display_row_append::{
    BufferOverlayStringTextRowRenderContext, BufferSyntheticTextRenderContext,
    BufferSyntheticTextRenderState, DisplayRowAppendSurface, DisplayRowLineBreakTransitionPlan,
    DisplayRowPrefixRequest, DisplayRowTextWindowEmitContext, DisplayRowTransitionContinuation,
    DisplayRowTransitionRenderState, OverlayStringRenderState, SyntheticTextAppendRequest,
    SyntheticTextMarker,
};
use crate::display_row_builder::DisplayRowPosition;
use crate::display_row_geometry::{
    DisplayRowFlags, DisplayRowGeometryDefaults, DisplayRowGeometryState, DisplayRowHitRange,
    DisplayRowLimit, DisplayRowScopedValue, DisplayRowYPositions,
};
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_row_walk_state::{
    BoxFaceRowState, HitRowRangeTracker, HorizontalScrollSkipState, LineNumberRenderState,
    TextPropertyScanCheckpoints, TrailingWhitespaceRenderState, WordWrapRenderState,
    skip_text_to_charpos,
};
use crate::hit_test::HitRow;
use crate::neovm_bridge::{LayoutBufferView, RustTextPropAccess};
use crate::unicode::{decode_utf8, is_wide_char};
use crate::window_output::{TextMatrixRowTransition, WindowOutputEmitter};
use neomacs_display_protocol::types::Color;
use neovm_core::buffer::{EmacsBytePos, LispCharPos1};

pub(crate) struct BufferHscrollSkipRenderState<'a, 'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
}

pub(crate) struct BufferEndOfBufferTailRenderRequest<'a> {
    context: BufferEndOfBufferTailRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferEndOfBufferTailRenderContext<'a> {
    pub(crate) byte_idx: usize,
    pub(crate) charpos: i64,
    pub(crate) accessible_end: i64,
    pub(crate) point_charpos: i64,
    pub(crate) has_overlays: bool,
    pub(crate) overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
    pub(crate) active_face_state: &'a DisplayRowActiveFaceState,
    pub(crate) row_limit: DisplayRowLimit,
}

pub(crate) struct BufferEndOfBufferTailRenderState<'emit> {
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) row_y_positions: &'emit mut DisplayRowYPositions,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
}

pub(crate) struct BufferEndOfBufferTailRenderOutcome {
    point_is_visible_eob: bool,
}

impl BufferEndOfBufferTailRenderOutcome {
    pub(crate) fn point_is_visible_eob(self) -> bool {
        self.point_is_visible_eob
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferHscrollSkipAction {
    LineBreak {
        ch_start_byte_idx: usize,
        charpos: i64,
    },
    Text {
        ch_start_byte_idx: usize,
        charpos: i64,
        show_left_truncation: bool,
    },
}

impl BufferHscrollSkipAction {
    pub(crate) fn is_line_break(self) -> bool {
        matches!(self, Self::LineBreak { .. })
    }

    pub(crate) fn ch_start_byte_idx(self) -> usize {
        match self {
            Self::LineBreak {
                ch_start_byte_idx, ..
            }
            | Self::Text {
                ch_start_byte_idx, ..
            } => ch_start_byte_idx,
        }
    }

    pub(crate) fn charpos(self) -> i64 {
        match self {
            Self::LineBreak { charpos, .. } | Self::Text { charpos, .. } => charpos,
        }
    }

    pub(crate) fn should_show_left_truncation(self) -> bool {
        matches!(
            self,
            Self::Text {
                show_left_truncation: true,
                ..
            }
        )
    }

    pub(crate) fn apply_line_break_before_row_transition(
        self,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        output_emitter: &mut WindowOutputEmitter,
        x: &mut f32,
        content_x: f32,
    ) {
        if self.is_line_break() {
            *x = content_x;
            output_emitter.note_display_buffer_pos(LispCharPos1::new(self.charpos()));
            row_extend.clear();
        }
    }

    pub(crate) fn line_break_hit_range(
        self,
        hit_row_range: &mut HitRowRangeTracker,
    ) -> Option<DisplayRowHitRange> {
        if !self.is_line_break() {
            return None;
        }
        let hit_range = hit_row_range.range_to(self.charpos());
        hit_row_range.advance_to(self.charpos());
        Some(hit_range)
    }

    pub(crate) fn capture_line_break_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        x: f32,
        col: usize,
        char_h: f32,
    ) {
        if !target.is_missing() || point_charpos != self.charpos() {
            return;
        }
        capture_cursor_info(
            target,
            CapturedCursorInfo::line_break_from_active_face_state(
                active_face_state,
                CapturedCursorPlacement::from_row_text_position(
                    row_geometry.text_position(x, self.ch_start_byte_idx(), col),
                    CapturedCursorSlotWidth::FaceChar,
                    false,
                ),
                char_h,
            ),
        );
    }

    pub(crate) fn apply_after_line_break_row_transition(
        self,
        row_transition: TextMatrixRowTransition,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        x: f32,
        col: usize,
        char_h: f32,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        self.capture_line_break_cursor_if_point(
            target,
            active_face_state,
            row_geometry,
            point_charpos,
            x,
            col,
            char_h,
        );
        DisplayRowTransitionContinuation::Continue
    }

    pub(crate) fn capture_text_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        x: f32,
        col: usize,
    ) {
        if !target.is_missing() || point_charpos != self.charpos() {
            return;
        }
        capture_cursor_info(
            target,
            CapturedCursorInfo::from_active_face_state(
                active_face_state,
                CapturedCursorPlacement::from_row_text_position(
                    row_geometry.text_position(x, self.ch_start_byte_idx(), col),
                    CapturedCursorSlotWidth::FaceChar,
                    false,
                ),
            ),
        );
    }

    pub(crate) fn append_left_truncation_marker_to_text_row_and_apply<'ctx>(
        self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        state: &mut BufferSyntheticTextRenderState<'_>,
        content_x: f32,
    ) {
        if !self.should_show_left_truncation() {
            return;
        }
        state.append_hscroll_truncation_marker_to_text_row(render_context, row_geometry, content_x);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferEndOfBufferCursorAction {
    byte_idx: usize,
    charpos: i64,
    accessible_end: i64,
    point_charpos: i64,
}

impl BufferEndOfBufferCursorAction {
    pub(crate) fn new(
        byte_idx: usize,
        charpos: i64,
        accessible_end: i64,
        point_charpos: i64,
    ) -> Self {
        Self {
            byte_idx,
            charpos,
            accessible_end,
            point_charpos,
        }
    }

    fn point_is_visible_eob(self) -> bool {
        self.point_charpos == self.accessible_end && self.charpos == self.accessible_end
    }

    pub(crate) fn capture_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        x: f32,
        col: usize,
    ) {
        if !target.is_missing()
            || (self.charpos != self.point_charpos && !self.point_is_visible_eob())
        {
            return;
        }
        if self.point_is_visible_eob() {
            tracing::debug!(
                "layout_window_rust: capturing EOB cursor at x={:.1} y={:.1} point={} point-max={}",
                x,
                row_geometry.glyph_y(0.0),
                self.point_charpos,
                self.accessible_end
            );
        }
        capture_cursor_info(
            target,
            CapturedCursorInfo::from_active_face_state(
                active_face_state,
                CapturedCursorPlacement::from_row_text_position(
                    row_geometry.text_position(x, self.byte_idx, col),
                    CapturedCursorSlotWidth::FaceChar,
                    false,
                ),
            ),
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferEndOfBufferTailAction {
    cursor: BufferEndOfBufferCursorAction,
    has_overlays: bool,
}

impl BufferEndOfBufferTailAction {
    pub(crate) fn new(
        byte_idx: usize,
        charpos: i64,
        accessible_end: i64,
        point_charpos: i64,
        has_overlays: bool,
    ) -> Self {
        Self {
            cursor: BufferEndOfBufferCursorAction::new(
                byte_idx,
                charpos,
                accessible_end,
                point_charpos,
            ),
            has_overlays,
        }
    }

    pub(crate) fn point_is_visible_eob(self) -> bool {
        self.cursor.point_is_visible_eob()
    }

    pub(crate) fn capture_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        x: f32,
        col: usize,
    ) {
        self.cursor
            .capture_cursor_if_point(target, active_face_state, row_geometry, x, col);
    }

    pub(crate) fn should_render_overlay_strings(
        self,
        row_geometry: &DisplayRowGeometryState,
        row_limit: DisplayRowLimit,
    ) -> bool {
        self.has_overlays && row_geometry.is_within_row_limit(row_limit)
    }

    pub(crate) fn render_overlay_strings_at_eob<B: LayoutBufferView>(
        self,
        buffer: &B,
        render_context: BufferOverlayStringTextRowRenderContext<'_>,
        active_face_state: &DisplayRowActiveFaceState,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        render_context.render_both_at(buffer, self.cursor.charpos, active_face_state, state);
    }
}

impl<'a> BufferEndOfBufferTailRenderRequest<'a> {
    pub(crate) fn new(context: BufferEndOfBufferTailRenderContext<'a>) -> Self {
        Self { context }
    }

    pub(crate) fn render_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferEndOfBufferTailRenderState<'_>,
    ) -> BufferEndOfBufferTailRenderOutcome {
        let BufferEndOfBufferTailRenderState {
            source_render,
            x,
            col,
            row_geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
        } = state;
        let mut source_render = source_render;
        let context = self.context;

        let tail = BufferEndOfBufferTailAction::new(
            context.byte_idx,
            context.charpos,
            context.accessible_end,
            context.point_charpos,
            context.has_overlays,
        );
        let point_is_visible_eob = tail.point_is_visible_eob();
        tail.capture_cursor_if_point(
            cursor_info,
            context.active_face_state,
            row_geometry,
            *x,
            *col,
        );

        if tail.should_render_overlay_strings(row_geometry, context.row_limit) {
            let mut overlay_state = OverlayStringRenderState::from_source_render(
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
            tail.render_overlay_strings_at_eob(
                buffer,
                context.overlay_context,
                context.active_face_state,
                &mut overlay_state,
            );
        }

        BufferEndOfBufferTailRenderOutcome {
            point_is_visible_eob,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferHscrollSkipSourceChar {
    ch_start_byte_idx: usize,
    ch: char,
    charpos: i64,
}

pub(crate) struct BufferHscrollSkipRenderRequest<'a> {
    context: BufferHscrollSkipRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferHscrollSkipRenderContext<'a> {
    pub(crate) text: &'a [u8],
    pub(crate) tab_width: i32,
    pub(crate) content_x: f32,
    pub(crate) append_surface: &'a DisplayRowAppendSurface,
    pub(crate) active_face_state: &'a DisplayRowActiveFaceState,
    pub(crate) default_face_ascent: f32,
    pub(crate) char_h: f32,
    pub(crate) char_w: f32,
    pub(crate) point_charpos: i64,
    pub(crate) has_prefix: bool,
    pub(crate) row_geometry_defaults: DisplayRowGeometryDefaults,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) max_rows: usize,
    pub(crate) row_limit: DisplayRowLimit,
}

impl BufferHscrollSkipSourceChar {
    fn new(ch_start_byte_idx: usize, ch: char, charpos: i64) -> Self {
        Self {
            ch_start_byte_idx,
            ch,
            charpos,
        }
    }

    pub(crate) fn consume_from_text(
        text: &[u8],
        byte_idx: &mut usize,
        charpos: &mut i64,
        hscroll_skip: &mut HorizontalScrollSkipState,
        tab_width: i32,
    ) -> Option<BufferHscrollSkipAction> {
        if *byte_idx >= text.len() {
            return None;
        }

        let ch_start_byte_idx = *byte_idx;
        let (ch, ch_len) = decode_utf8(&text[*byte_idx..]);
        *byte_idx += ch_len;
        *charpos += 1;

        Some(
            Self::new(ch_start_byte_idx, ch, *charpos).consume_for_hscroll(hscroll_skip, tab_width),
        )
    }

    fn consume_for_hscroll(
        self,
        hscroll_skip: &mut HorizontalScrollSkipState,
        tab_width: i32,
    ) -> BufferHscrollSkipAction {
        if self.ch == '\n' {
            return BufferHscrollSkipAction::LineBreak {
                ch_start_byte_idx: self.ch_start_byte_idx,
                charpos: self.charpos,
            };
        }

        hscroll_skip.consume_columns(self.column_width(tab_width, hscroll_skip.consumed_columns()));
        BufferHscrollSkipAction::Text {
            ch_start_byte_idx: self.ch_start_byte_idx,
            charpos: self.charpos,
            show_left_truncation: !hscroll_skip.should_skip()
                && hscroll_skip.should_show_left_truncation(),
        }
    }

    fn column_width(self, tab_width: i32, consumed_columns: i32) -> i32 {
        if self.ch == '\t' {
            let tab_width = tab_width.max(1);
            return ((consumed_columns / tab_width + 1) * tab_width) - consumed_columns;
        }

        if is_wide_char(self.ch) { 2 } else { 1 }
    }
}

impl<'a> BufferHscrollSkipRenderRequest<'a> {
    pub(crate) fn new(context: BufferHscrollSkipRenderContext<'a>) -> Self {
        Self { context }
    }

    pub(crate) fn render_next_and_apply(
        self,
        state: BufferHscrollSkipRenderState<'_, '_>,
    ) -> DisplayRowTransitionContinuation {
        let BufferHscrollSkipRenderState {
            byte_idx,
            charpos,
            hscroll_skip,
            row_extend,
            source_render,
            x,
            col,
            prefix_request,
            line_numbers,
            word_wrap,
            trailing_whitespace,
            row_geometry,
            row_flags,
            hit_rows,
            hit_row_range,
            cursor_info,
            row_y_positions,
        } = state;
        let mut source_render = source_render;
        let context = self.context;

        let Some(hscroll_action) = BufferHscrollSkipSourceChar::consume_from_text(
            context.text,
            byte_idx,
            charpos,
            hscroll_skip,
            context.tab_width,
        ) else {
            return DisplayRowTransitionContinuation::Exhausted;
        };

        if hscroll_action.is_line_break() {
            hscroll_action.apply_line_break_before_row_transition(
                row_extend,
                source_render.output_emitter(),
                x,
                context.content_x,
            );
            let line_break_transition = DisplayRowLineBreakTransitionPlan::hscroll_line_break();
            let hit_range = hscroll_action
                .line_break_hit_range(hit_row_range)
                .expect("hscroll line break hit range");
            let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                context.row_geometry_defaults,
                context.text_matrix_row_base,
                row_y_positions,
                context.max_rows,
                row_geometry,
                row_flags,
                context.row_limit,
                hit_rows,
                &mut source_render,
            )
            .emit_line_break_then_row_start(
                line_break_transition,
                hit_range,
                DisplayRowPosition {
                    x_px: *x,
                    col: *col,
                },
                0.0,
                DisplayRowTransitionRenderState::new(
                    prefix_request,
                    context.has_prefix,
                    line_numbers,
                    hscroll_skip,
                    word_wrap,
                    trailing_whitespace,
                ),
                col,
            );
            return hscroll_action.apply_after_line_break_row_transition(
                row_transition,
                cursor_info,
                context.active_face_state,
                row_geometry,
                context.point_charpos,
                *x,
                *col,
                context.char_h,
            );
        }

        let mut synthetic_text_state =
            BufferSyntheticTextRenderState::new(source_render.reborrow(), x, col);
        hscroll_action.append_left_truncation_marker_to_text_row_and_apply(
            BufferSyntheticTextRenderContext::new(
                context.append_surface,
                context.active_face_state,
                0.0,
                context.char_h,
                context.default_face_ascent,
                context.char_w,
            ),
            row_geometry,
            &mut synthetic_text_state,
            context.content_x,
        );
        hscroll_action.capture_text_cursor_if_point(
            cursor_info,
            context.active_face_state,
            row_geometry,
            context.point_charpos,
            *x,
            *col,
        );
        DisplayRowTransitionContinuation::Continue
    }
}

pub(crate) struct BufferTextLineBreakRenderState<'a, 'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) box_face: &'emit mut BoxFaceRowState,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
}

pub(crate) struct BufferSelectiveDisplayTailRenderRequest<'a> {
    source_char: BufferTextDecodedSourceChar,
    context: BufferSelectiveDisplayTailRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferSelectiveDisplayTailRenderContext<'a> {
    pub(crate) text: &'a [u8],
    pub(crate) text_start_byte: usize,
    pub(crate) selective_display: i32,
    pub(crate) tab_width: i32,
    pub(crate) append_surface: &'a DisplayRowAppendSurface,
    pub(crate) active_face_state: &'a DisplayRowActiveFaceState,
    pub(crate) glyph_y_offset: f32,
    pub(crate) default_face_ascent: f32,
    pub(crate) char_h: f32,
    pub(crate) char_w: f32,
    pub(crate) content_x: f32,
    pub(crate) has_prefix: bool,
    pub(crate) row_geometry_defaults: DisplayRowGeometryDefaults,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) max_rows: usize,
    pub(crate) row_limit: DisplayRowLimit,
}

pub(crate) struct BufferSelectiveDisplayTailRenderState<'a, 'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) col: &'emit mut usize,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) box_face: &'emit mut BoxFaceRowState,
    pub(crate) x: &'emit mut f32,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
}

pub(crate) struct BufferInvisibleTextRenderRequest<'a> {
    context: BufferInvisibleTextRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferInvisibleTextRenderContext<'a> {
    pub(crate) text: &'a [u8],
    pub(crate) accessible_end: i64,
    pub(crate) point_charpos: i64,
    pub(crate) append_surface: &'a DisplayRowAppendSurface,
    pub(crate) overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
    pub(crate) active_face_state: &'a DisplayRowActiveFaceState,
    pub(crate) glyph_y_offset: f32,
    pub(crate) default_face_ascent: f32,
    pub(crate) char_h: f32,
    pub(crate) char_w: f32,
}

pub(crate) struct BufferInvisibleTextRenderRequestState<'a, 'emit> {
    pub(crate) checkpoints: &'emit mut TextPropertyScanCheckpoints,
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferSelectiveDisplayTailRenderOutcome {
    NotHidden,
    ContinueBufferWalk,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferInvisibleTextRenderOutcome {
    Visible,
    ContinueBufferWalk,
}

impl BufferSelectiveDisplayTailRenderOutcome {
    pub(crate) fn should_break(self) -> bool {
        matches!(self, Self::Stop)
    }

    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(self, Self::ContinueBufferWalk)
    }
}

impl BufferInvisibleTextRenderOutcome {
    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(self, Self::ContinueBufferWalk)
    }
}

impl<'a> BufferSelectiveDisplayTailRenderRequest<'a> {
    pub(crate) fn new(
        source_char: BufferTextDecodedSourceChar,
        context: BufferSelectiveDisplayTailRenderContext<'a>,
    ) -> Self {
        Self {
            source_char,
            context,
        }
    }

    pub(crate) fn render_if_needed_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferSelectiveDisplayTailRenderState<'_, '_>,
    ) -> BufferSelectiveDisplayTailRenderOutcome {
        let context = self.context;
        let selective_display = BufferSelectiveDisplayContext::new(
            context.text,
            context.selective_display,
            context.tab_width,
        );
        let Some(marker) = selective_display.carriage_return_tail_marker(self.source_char.ch())
        else {
            return BufferSelectiveDisplayTailRenderOutcome::NotHidden;
        };

        let BufferSelectiveDisplayTailRenderState {
            byte_idx,
            charpos,
            col,
            source_render,
            row_extend,
            box_face,
            x,
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
        } = state;
        let mut source_render = source_render;

        let mut synthetic_text_state =
            BufferSyntheticTextRenderState::new(source_render.reborrow(), x, col);
        marker.append_to_text_row_and_apply(
            BufferSyntheticTextRenderContext::new(
                context.append_surface,
                context.active_face_state,
                context.glyph_y_offset,
                context.char_h,
                context.default_face_ascent,
                context.char_w,
            ),
            row_geometry,
            &mut synthetic_text_state,
        );

        let tail_action =
            selective_display.skip_rest_of_line_after_carriage_return(byte_idx, charpos);
        if !tail_action.is_line_break() {
            return BufferSelectiveDisplayTailRenderOutcome::ContinueBufferWalk;
        }

        tail_action.apply_hidden_line_break_row_state(
            row_geometry,
            row_extend,
            box_face,
            context.content_x,
            x,
        );
        let line_break_transition = DisplayRowLineBreakTransitionPlan::hidden_line_break();
        let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
            context.row_geometry_defaults,
            context.text_matrix_row_base,
            row_y_positions,
            context.max_rows,
            row_geometry,
            row_flags,
            context.row_limit,
            hit_rows,
            &mut source_render,
        )
        .emit_line_break_then_row_start(
            line_break_transition,
            hit_row_range.range_to(*charpos),
            DisplayRowPosition {
                x_px: *x,
                col: *col,
            },
            0.0,
            DisplayRowTransitionRenderState::new(
                prefix_request,
                context.has_prefix,
                line_numbers,
                hscroll_skip,
                word_wrap,
                trailing_whitespace,
            ),
            col,
        );
        let synced_charpos = buffer
            .layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(
                context.text_start_byte + *byte_idx,
            ))
            .get() as i64;
        if tail_action
            .apply_after_hidden_line_break_transition(
                row_transition,
                synced_charpos,
                charpos,
                hit_row_range,
            )
            .should_break()
        {
            return BufferSelectiveDisplayTailRenderOutcome::Stop;
        }

        BufferSelectiveDisplayTailRenderOutcome::ContinueBufferWalk
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferInvisibleTextScanAction {
    Unchecked,
    Visible { next_visible: i64 },
    Hidden(BufferInvisibleTextSkip),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferInvisibleTextSkip {
    start_byte_idx: usize,
    start_charpos: i64,
    skip_to: i64,
    next_visible: i64,
    point_in_hidden_region: bool,
    ellipsis: bool,
}

impl BufferInvisibleTextSkip {
    pub(crate) fn new(
        start_byte_idx: usize,
        start_charpos: i64,
        skip_to: i64,
        next_visible: i64,
        point_in_hidden_region: bool,
        ellipsis: bool,
    ) -> Self {
        Self {
            start_byte_idx,
            start_charpos,
            skip_to,
            next_visible,
            point_in_hidden_region,
            ellipsis,
        }
    }

    #[cfg(test)]
    pub(crate) fn start_byte_idx(self) -> usize {
        self.start_byte_idx
    }

    #[cfg(test)]
    pub(crate) fn start_charpos(self) -> i64 {
        self.start_charpos
    }

    #[cfg(test)]
    pub(crate) fn skip_to(self) -> i64 {
        self.skip_to
    }

    #[cfg(test)]
    pub(crate) fn next_visible(self) -> i64 {
        self.next_visible
    }

    #[cfg(test)]
    pub(crate) fn point_in_hidden_region(self) -> bool {
        self.point_in_hidden_region
    }

    #[cfg(test)]
    pub(crate) fn ellipsis(self) -> bool {
        self.ellipsis
    }

    pub(crate) fn capture_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        x: f32,
        col: usize,
    ) {
        if !self.point_in_hidden_region {
            return;
        }
        capture_cursor_info(
            target,
            CapturedCursorInfo::from_active_face_state(
                active_face_state,
                CapturedCursorPlacement::from_row_text_position(
                    row_geometry.text_position(x, self.start_byte_idx, col),
                    CapturedCursorSlotWidth::FaceChar,
                    false,
                ),
            ),
        );
    }

    pub(crate) fn ellipsis_append_request(
        self,
        position: DisplayRowPosition,
    ) -> Option<SyntheticTextAppendRequest> {
        self.ellipsis.then(|| {
            SyntheticTextAppendRequest::active_marker(
                position,
                SyntheticTextMarker::InvisibleEllipsis,
            )
        })
    }

    pub(crate) fn append_to_text_row_and_apply<'ctx>(
        self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        state: &mut BufferInvisibleTextRenderState<'_>,
    ) {
        let position = state.synthetic_text.position();
        self.capture_cursor_if_point(
            state.cursor_info,
            render_context.active_face(),
            row_geometry,
            position.x_px,
            position.col,
        );

        let Some(request) = self.ellipsis_append_request(position) else {
            return;
        };
        state
            .synthetic_text
            .append_request_to_text_row(render_context, row_geometry, request);
    }
}

pub(crate) struct BufferInvisibleTextRenderState<'a> {
    synthetic_text: BufferSyntheticTextRenderState<'a>,
    cursor_info: &'a mut CursorCaptureState,
}

impl<'a> BufferInvisibleTextRenderState<'a> {
    pub(crate) fn new(
        source_render: TextRowSourceRenderState<'a>,
        cursor_info: &'a mut CursorCaptureState,
        x: &'a mut f32,
        col: &'a mut usize,
    ) -> Self {
        Self {
            synthetic_text: BufferSyntheticTextRenderState::new(source_render, x, col),
            cursor_info,
        }
    }
}

impl<'a> BufferInvisibleTextRenderRequest<'a> {
    pub(crate) fn new(context: BufferInvisibleTextRenderContext<'a>) -> Self {
        Self { context }
    }

    pub(crate) fn render_at_checkpoint_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferInvisibleTextRenderRequestState<'_, '_>,
    ) -> BufferInvisibleTextRenderOutcome {
        let BufferInvisibleTextRenderRequestState {
            checkpoints,
            byte_idx,
            charpos,
            source_render,
            x,
            col,
            row_geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
        } = state;
        let mut source_render = source_render;
        let context = self.context;

        let action = BufferInvisibleTextScanContext::new(
            context.text,
            context.accessible_end,
            context.point_charpos,
            cursor_info.is_missing(),
        )
        .consume_at_checkpoint(buffer, checkpoints, byte_idx, charpos);
        let BufferInvisibleTextScanAction::Hidden(hidden_text) = action else {
            return BufferInvisibleTextRenderOutcome::Visible;
        };

        let mut hidden_text_state =
            BufferInvisibleTextRenderState::new(source_render.reborrow(), cursor_info, x, col);
        hidden_text.append_to_text_row_and_apply(
            BufferSyntheticTextRenderContext::new(
                context.append_surface,
                context.active_face_state,
                context.glyph_y_offset,
                context.char_h,
                context.default_face_ascent,
                context.char_w,
            ),
            row_geometry,
            &mut hidden_text_state,
        );

        let mut overlay_state = OverlayStringRenderState::from_source_render(
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
        context.overlay_context.render_after_at(
            buffer,
            *charpos,
            context.active_face_state,
            &mut overlay_state,
        );
        BufferInvisibleTextRenderOutcome::ContinueBufferWalk
    }
}

pub(crate) struct BufferInvisibleTextScanContext<'a> {
    text: &'a [u8],
    accessible_end: i64,
    point_charpos: i64,
    cursor_missing: bool,
}

impl<'a> BufferInvisibleTextScanContext<'a> {
    pub(crate) fn new(
        text: &'a [u8],
        accessible_end: i64,
        point_charpos: i64,
        cursor_missing: bool,
    ) -> Self {
        Self {
            text,
            accessible_end,
            point_charpos,
            cursor_missing,
        }
    }

    pub(crate) fn consume_at_checkpoint<B: LayoutBufferView>(
        &self,
        buffer: &B,
        checkpoints: &mut TextPropertyScanCheckpoints,
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) -> BufferInvisibleTextScanAction {
        if !checkpoints.should_check_invisible(*charpos) {
            return BufferInvisibleTextScanAction::Unchecked;
        }

        let start_byte_idx = *byte_idx;
        let start_charpos = *charpos;
        let text_props = RustTextPropAccess::new(buffer);
        let (invisible, next_visible) = text_props.check_invisible(start_charpos);
        checkpoints.record_invisible_next(next_visible);

        if !invisible.hidden {
            return BufferInvisibleTextScanAction::Visible { next_visible };
        }

        let skip_to = next_visible.min(self.accessible_end);
        let point_in_hidden_region = self.cursor_missing
            && self.point_charpos >= start_charpos
            && self.point_charpos < skip_to;
        skip_text_to_charpos(self.text, byte_idx, charpos, skip_to);

        BufferInvisibleTextScanAction::Hidden(BufferInvisibleTextSkip::new(
            start_byte_idx,
            start_charpos,
            skip_to,
            next_visible,
            point_in_hidden_region,
            invisible.ellipsis,
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferSelectiveDisplayLineTailAction {
    Exhausted,
    LineBreak { charpos: i64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSelectiveDisplayLineTailMarker;

impl BufferSelectiveDisplayLineTailMarker {
    pub(crate) fn ellipsis_append_request(
        self,
        position: DisplayRowPosition,
    ) -> SyntheticTextAppendRequest {
        SyntheticTextAppendRequest::active_marker(position, SyntheticTextMarker::SelectiveEllipsis)
    }

    pub(crate) fn append_to_text_row_and_apply<'ctx>(
        self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        state: &mut BufferSyntheticTextRenderState<'_>,
    ) {
        let request = self.ellipsis_append_request(state.position());
        state.append_request_to_text_row(render_context, row_geometry, request);
    }
}

impl BufferSelectiveDisplayLineTailAction {
    pub(crate) fn is_line_break(self) -> bool {
        matches!(self, Self::LineBreak { .. })
    }

    pub(crate) fn apply_hidden_line_break_row_state(
        self,
        row_geometry: &DisplayRowGeometryState,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &mut BoxFaceRowState,
        content_x: f32,
        x: &mut f32,
    ) {
        if self.is_line_break() {
            *x = content_x;
            row_extend.clear();
            box_face.continue_on_row(row_geometry.next_row_marker(), content_x);
        }
    }

    pub(crate) fn sync_after_hidden_line_break_transition(
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) {
        *charpos = synced_charpos;
        hit_row_range.advance_to(*charpos);
    }

    pub(crate) fn apply_after_hidden_line_break_transition(
        self,
        row_transition: TextMatrixRowTransition,
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        Self::sync_after_hidden_line_break_transition(synced_charpos, charpos, hit_row_range);
        DisplayRowTransitionContinuation::Continue
    }

    #[cfg(test)]
    pub(crate) fn charpos(self) -> Option<i64> {
        match self {
            Self::LineBreak { charpos } => Some(charpos),
            Self::Exhausted => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSelectiveDisplayHiddenLines {
    hidden_line_count: usize,
}

impl BufferSelectiveDisplayHiddenLines {
    fn new(hidden_line_count: usize) -> Self {
        Self { hidden_line_count }
    }

    #[cfg(test)]
    pub(crate) fn hidden_line_count(self) -> usize {
        self.hidden_line_count
    }

    pub(crate) fn apply_to_line_numbers(self, line_numbers: &mut LineNumberRenderState) {
        for _ in 0..self.hidden_line_count {
            line_numbers.advance_hidden_line();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSelectiveDisplayContext<'a> {
    text: &'a [u8],
    selective_display: i32,
    tab_width: i32,
}

impl<'a> BufferSelectiveDisplayContext<'a> {
    pub(crate) fn new(text: &'a [u8], selective_display: i32, tab_width: i32) -> Self {
        Self {
            text,
            selective_display,
            tab_width: tab_width.max(1),
        }
    }

    pub(crate) fn hides_carriage_return_tail(self, ch: char) -> bool {
        self.selective_display > 0 && ch == '\r'
    }

    pub(crate) fn carriage_return_tail_marker(
        self,
        ch: char,
    ) -> Option<BufferSelectiveDisplayLineTailMarker> {
        self.hides_carriage_return_tail(ch)
            .then_some(BufferSelectiveDisplayLineTailMarker)
    }

    pub(crate) fn hides_indented_lines_after_line_break(self, byte_idx: usize) -> bool {
        self.selective_display > 0
            && self.selective_display < i32::MAX
            && byte_idx < self.text.len()
    }

    pub(crate) fn skip_rest_of_line_after_carriage_return(
        self,
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) -> BufferSelectiveDisplayLineTailAction {
        *charpos += 1;
        while *byte_idx < self.text.len() {
            let (skip_ch, skip_len) = decode_utf8(&self.text[*byte_idx..]);
            if skip_len == 0 {
                break;
            }
            *byte_idx += skip_len;
            *charpos += 1;
            if skip_ch == '\n' {
                return BufferSelectiveDisplayLineTailAction::LineBreak { charpos: *charpos };
            }
        }

        BufferSelectiveDisplayLineTailAction::Exhausted
    }

    pub(crate) fn skip_hidden_indented_lines_after_line_break(
        self,
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) -> BufferSelectiveDisplayHiddenLines {
        let mut hidden_line_count = 0;
        while *byte_idx < self.text.len() {
            let Some(indent) = self.indentation_columns_at(*byte_idx) else {
                break;
            };
            if indent <= self.selective_display {
                break;
            }

            if self.skip_line(byte_idx, charpos) {
                hidden_line_count += 1;
            }
        }

        BufferSelectiveDisplayHiddenLines::new(hidden_line_count)
    }

    pub(crate) fn apply_hidden_indented_lines_after_line_break(
        self,
        byte_idx: &mut usize,
        charpos: &mut i64,
        line_numbers: &mut LineNumberRenderState,
    ) -> BufferSelectiveDisplayHiddenLines {
        if !self.hides_indented_lines_after_line_break(*byte_idx) {
            return BufferSelectiveDisplayHiddenLines::new(0);
        }
        let hidden_lines = self.skip_hidden_indented_lines_after_line_break(byte_idx, charpos);
        hidden_lines.apply_to_line_numbers(line_numbers);
        hidden_lines
    }

    fn indentation_columns_at(self, mut byte_idx: usize) -> Option<i32> {
        if byte_idx >= self.text.len() {
            return None;
        }

        let mut indent = 0i32;
        while byte_idx < self.text.len() {
            match self.text[byte_idx] {
                b' ' => {
                    indent += 1;
                    byte_idx += 1;
                }
                b'\t' => {
                    indent = ((indent / self.tab_width) + 1) * self.tab_width;
                    byte_idx += 1;
                }
                _ => break,
            }
        }
        Some(indent)
    }

    fn skip_line(self, byte_idx: &mut usize, charpos: &mut i64) -> bool {
        while *byte_idx < self.text.len() {
            let (skip_ch, skip_len) = decode_utf8(&self.text[*byte_idx..]);
            if skip_len == 0 {
                break;
            }
            *byte_idx += skip_len;
            *charpos += 1;
            if skip_ch == '\n' {
                return true;
            }
        }
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextLineBreakSourceAction {
    ch_start_byte_idx: usize,
    charpos: i64,
    next_charpos: i64,
    line_spacing: f32,
}

pub(crate) struct BufferTextLineBreakRenderRequest<'a> {
    source_event: BufferTextLineBreakSourceEvent,
    context: BufferTextLineBreakRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferTextLineBreakRenderContext<'a> {
    pub(crate) text: &'a [u8],
    pub(crate) text_start_byte: usize,
    pub(crate) selective_display: i32,
    pub(crate) tab_width: i32,
    pub(crate) active_face_state: &'a DisplayRowActiveFaceState,
    pub(crate) point_charpos: i64,
    pub(crate) char_h: f32,
    pub(crate) extra_line_spacing: f32,
    pub(crate) content_x: f32,
    pub(crate) has_prefix: bool,
    pub(crate) row_geometry_defaults: DisplayRowGeometryDefaults,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) max_rows: usize,
    pub(crate) row_limit: DisplayRowLimit,
}

impl<'a> BufferTextLineBreakRenderRequest<'a> {
    #[cfg(test)]
    pub(crate) fn new(
        source_char: BufferTextDecodedSourceChar,
        context: BufferTextLineBreakRenderContext<'a>,
    ) -> Self {
        Self::from_source_event(BufferTextLineBreakSourceEvent::new(source_char), context)
    }

    pub(crate) fn from_source_event(
        source_event: BufferTextLineBreakSourceEvent,
        context: BufferTextLineBreakRenderContext<'a>,
    ) -> Self {
        Self {
            source_event,
            context,
        }
    }

    pub(crate) fn render_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferTextLineBreakRenderState<'_, '_>,
    ) -> DisplayRowTransitionContinuation {
        let BufferTextLineBreakRenderState {
            byte_idx,
            charpos,
            cursor_info,
            row_geometry,
            trailing_whitespace,
            row_extend,
            box_face,
            source_render,
            x,
            col,
            prefix_request,
            line_numbers,
            hscroll_skip,
            word_wrap,
            row_flags,
            hit_rows,
            hit_row_range,
            row_y_positions,
        } = state;
        let mut source_render = source_render;
        let context = self.context;

        let line_break_action = BufferTextLineBreakSourceAction::for_decoded_newline(
            buffer,
            self.source_event.decoded_char(),
            context.char_h,
            context.extra_line_spacing,
        );
        line_break_action.capture_cursor_if_point(
            cursor_info,
            context.active_face_state,
            row_geometry,
            context.point_charpos,
            *x,
            *col,
        );
        line_break_action.apply_before_row_transition(
            row_geometry,
            trailing_whitespace,
            row_extend,
            box_face,
            source_render.output_emitter(),
            context.content_x,
            x,
            charpos,
        );

        let line_break_transition = DisplayRowLineBreakTransitionPlan::line_break();
        let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
            context.row_geometry_defaults,
            context.text_matrix_row_base,
            row_y_positions,
            context.max_rows,
            row_geometry,
            row_flags,
            context.row_limit,
            hit_rows,
            &mut source_render,
        )
        .emit_line_break_then_row_start(
            line_break_transition,
            hit_row_range.range_to(*charpos),
            DisplayRowPosition {
                x_px: *x,
                col: *col,
            },
            line_break_action.line_spacing(),
            DisplayRowTransitionRenderState::new(
                prefix_request,
                context.has_prefix,
                line_numbers,
                hscroll_skip,
                word_wrap,
                trailing_whitespace,
            ),
            col,
        );

        let synced_charpos = buffer
            .layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(
                context.text_start_byte + *byte_idx,
            ))
            .get() as i64;
        let continuation = line_break_action.apply_after_line_break_row_transition(
            row_transition,
            synced_charpos,
            charpos,
            hit_row_range,
            row_geometry,
            box_face,
            context.content_x,
        );
        if continuation.should_break() {
            return continuation;
        }

        BufferSelectiveDisplayContext::new(
            context.text,
            context.selective_display,
            context.tab_width,
        )
        .apply_hidden_indented_lines_after_line_break(byte_idx, charpos, line_numbers);
        DisplayRowTransitionContinuation::Continue
    }
}

impl BufferTextLineBreakSourceAction {
    pub(crate) fn for_newline<B: LayoutBufferView>(
        buffer: &B,
        charpos: i64,
        ch_start_byte_idx: usize,
        char_h: f32,
        extra_line_spacing: f32,
    ) -> Self {
        let text_prop_spacing = RustTextPropAccess::new(buffer).check_line_spacing(charpos, char_h);
        let line_spacing = if text_prop_spacing > 0.0 {
            text_prop_spacing
        } else if extra_line_spacing > 0.0 {
            extra_line_spacing
        } else {
            0.0
        };
        Self {
            ch_start_byte_idx,
            charpos,
            next_charpos: charpos + 1,
            line_spacing,
        }
    }

    pub(crate) fn for_decoded_newline<B: LayoutBufferView>(
        buffer: &B,
        source_char: BufferTextDecodedSourceChar,
        char_h: f32,
        extra_line_spacing: f32,
    ) -> Self {
        Self::for_newline(
            buffer,
            source_char.start_charpos(),
            source_char.start_byte_idx(),
            char_h,
            extra_line_spacing,
        )
    }

    pub(crate) fn point_matches(self, point_charpos: i64) -> bool {
        point_charpos == self.charpos
    }

    pub(crate) fn next_charpos(self) -> i64 {
        self.next_charpos
    }

    pub(crate) fn line_spacing(self) -> f32 {
        self.line_spacing
    }

    pub(crate) fn apply_before_row_transition(
        self,
        row_geometry: &DisplayRowGeometryState,
        trailing_whitespace: &mut TrailingWhitespaceRenderState,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &mut BoxFaceRowState,
        output_emitter: &mut WindowOutputEmitter,
        content_x: f32,
        x: &mut f32,
        charpos: &mut i64,
    ) {
        trailing_whitespace.reset_after_row_transition();
        row_extend.clear();
        box_face.continue_on_row(row_geometry.current_row_marker(), content_x);
        *charpos = self.next_charpos();
        *x = content_x;
        output_emitter.note_display_buffer_pos(LispCharPos1::new(*charpos));
    }

    pub(crate) fn apply_after_row_transition(
        self,
        row_geometry: &DisplayRowGeometryState,
        box_face: &mut BoxFaceRowState,
        content_x: f32,
    ) {
        box_face.continue_on_row(row_geometry.current_row_marker(), content_x);
    }

    pub(crate) fn apply_after_line_break_row_transition(
        self,
        row_transition: TextMatrixRowTransition,
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        row_geometry: &DisplayRowGeometryState,
        box_face: &mut BoxFaceRowState,
        content_x: f32,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        Self::sync_after_row_transition(synced_charpos, charpos, hit_row_range);
        self.apply_after_row_transition(row_geometry, box_face, content_x);
        DisplayRowTransitionContinuation::Continue
    }

    pub(crate) fn sync_after_row_transition(
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) {
        *charpos = synced_charpos;
        hit_row_range.advance_to(*charpos);
    }

    pub(crate) fn cursor_info(
        self,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        x: f32,
        col: usize,
    ) -> CapturedCursorInfo {
        CapturedCursorInfo::from_active_face_state(
            active_face_state,
            CapturedCursorPlacement::from_row_text_position(
                row_geometry.text_position(x, self.ch_start_byte_idx, col),
                CapturedCursorSlotWidth::FaceChar,
                false,
            ),
        )
    }

    pub(crate) fn capture_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        x: f32,
        col: usize,
    ) {
        if !target.is_missing() || !self.point_matches(point_charpos) {
            return;
        }
        capture_cursor_info(
            target,
            self.cursor_info(active_face_state, row_geometry, x, col),
        );
    }
}
