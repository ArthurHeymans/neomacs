//! Buffer-text source rendering requests and actions.

use crate::display_buffer_text_item_append::{
    BufferTextPreparedSourceCharAppend, BufferTextRowAppendContext, BufferTextRowAppendState,
    BufferTextSourceCharPreparationState, BufferTextSourceCharPreparedAppend,
    BufferTextSourceDisplayItemPreparationRequest, BufferTextSpecialSourceCharPreparedAppend,
};
use crate::display_buffer_text_source::{BufferTextSourceItem, BufferTextSourceStepChar};
use crate::display_cursor::{
    CapturedCursorInfo, CapturedCursorPlacement, CapturedCursorSlotWidth, CursorCaptureState,
    capture_cursor_info,
};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_face_layout::{DisplayHeightFaceBasis, height_adjusted_face};
use crate::display_item::{DisplayItem, DisplayItemKind, RenderFaceRef};
use crate::display_origin::DisplayOrigin;
use crate::display_property::classify_display_property;
use crate::display_row::{
    DisplayRowActiveFaceState, DisplayRowFallbackMetrics, DisplayRowMeasurementPolicy,
};
use crate::display_row_append_context::{
    DisplayRowActiveFaceAppendContext, DisplayRowAppendFrame, DisplayRowAppendKind,
    DisplayRowAppendSurface,
};
use crate::display_row_builder::{
    DisplayRowAppendProgress, DisplayRowAppendStatus, DisplayRowPosition,
};
use crate::display_row_geometry::{
    DisplayRowFlags, DisplayRowGeometryDefaults, DisplayRowGeometryState, DisplayRowHitRange,
    DisplayRowLimit, DisplayRowScopedValue, DisplayRowTextPosition, DisplayRowVisibilityLimit,
    DisplayRowYPositions,
};
use crate::display_row_lisp_string::{
    DisplayRowPrefixRequest, DisplayRowPrefixValues, LispStringRowAppendContext,
};
use crate::display_row_overlay_string::{
    BufferOverlayStringTextRowRenderContext, OverlayStringRenderState,
};
use crate::display_row_replacement::DisplayPropertyReplacementAppendOutcome;
use crate::display_row_source_append::append_synthetic_text_to_display_row;
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_row_transition::{
    DisplayRowLineBreakTransitionPlan, DisplayRowOverflowTransitionPlan,
    DisplayRowTextWindowEmitContext, DisplayRowTransitionContinuation,
    DisplayRowTransitionRenderState,
};
use crate::display_row_walk_state::{
    BoxFaceRowState, BufferTextRowOverflowDecision, FaceScanCheckpoint, HitRowRangeTracker,
    HorizontalScrollSkipState, LineNumberRenderState, SpecialTextRowOverflowDecision,
    TextPropertyScanCheckpoints, TextRowTransitionStatePolicy, TrailingWhitespaceRenderState,
    WordWrapBreakCandidate, WordWrapRenderState, skip_text_to_charpos, skip_to_newline,
};
use crate::display_source::{BufferDisplayPropertyTextSourceEvent, SyntheticTextItemSource};
use crate::display_source_resolver::DisplayPropertyReplacementAppendRequestResolver;
use crate::hit_test::HitRow;
use crate::neovm_bridge::{FaceResolver, LayoutBufferView, ResolvedFace, RustTextPropAccess};
use crate::types::{LineWrapMode, WindowParams};
use crate::unicode::{decode_utf8, is_wide_char};
use crate::window_output::{TextMatrixRowTransition, WindowOutputEmitter};
use neomacs_display_protocol::face::BasicFaceId;
use neomacs_display_protocol::types::Color;
use neovm_core::buffer::{BufferId, CharPos0, EmacsBytePos, LispCharPos1};
use neovm_core::emacs_core::Value;

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
    source_char: BufferTextSourceStepChar,
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
        source_char: BufferTextSourceStepChar,
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
    source_char: BufferTextSourceStepChar,
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
    pub(crate) fn new(
        source_char: BufferTextSourceStepChar,
        context: BufferTextLineBreakRenderContext<'a>,
    ) -> Self {
        debug_assert_eq!(source_char.ch(), '\n');
        Self {
            source_char,
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

        let line_break_action = BufferTextLineBreakSourceAction::for_source_step_newline(
            buffer,
            self.source_char,
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

    pub(crate) fn for_source_step_newline<B: LayoutBufferView>(
        buffer: &B,
        source_char: BufferTextSourceStepChar,
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

pub(crate) const SYNTHETIC_SOURCE_INVISIBLE_ELLIPSIS: u64 = 3;
pub(crate) const SYNTHETIC_SOURCE_HSCROLL_TRUNCATION: u64 = 4;
pub(crate) const SYNTHETIC_SOURCE_SELECTIVE_ELLIPSIS: u64 = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SyntheticTextSource {
    pub(crate) source_id: u64,
    pub(crate) text: Box<str>,
}

impl SyntheticTextSource {
    #[cfg(test)]
    pub(crate) fn new(source_id: u64, text: impl Into<Box<str>>) -> Self {
        Self {
            source_id,
            text: text.into(),
        }
    }

    fn marker(marker: SyntheticTextMarker) -> Self {
        Self {
            source_id: marker.source_id(),
            text: marker.text().into(),
        }
    }

    pub(crate) fn into_item_source(self, face_id: u32) -> SyntheticTextItemSource {
        SyntheticTextItemSource::new(self.source_id, self.text, RenderFaceRef::FaceId(face_id), 0)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SyntheticTextAppendRequest {
    position: DisplayRowPosition,
    source: SyntheticTextSource,
    face: SyntheticTextAppendFace,
}

#[derive(Clone, Debug)]
pub(crate) enum SyntheticTextAppendFace {
    ActiveFace,
    TextRowMetrics {
        face_id: u32,
        base_face: ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    },
}

impl SyntheticTextAppendRequest {
    #[cfg(test)]
    pub(crate) fn active_source(position: DisplayRowPosition, source: SyntheticTextSource) -> Self {
        Self {
            position,
            source,
            face: SyntheticTextAppendFace::ActiveFace,
        }
    }

    pub(crate) fn active_marker(position: DisplayRowPosition, marker: SyntheticTextMarker) -> Self {
        Self {
            position,
            source: SyntheticTextSource::marker(marker),
            face: SyntheticTextAppendFace::ActiveFace,
        }
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_row_metrics_source(
        position: DisplayRowPosition,
        source: SyntheticTextSource,
        face_id: u32,
        base_face: &ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    ) -> Self {
        Self {
            position,
            source,
            face: SyntheticTextAppendFace::TextRowMetrics {
                face_id,
                base_face: base_face.clone(),
                height_px,
                ascent_px,
                char_width_px,
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_row_metrics_marker(
        position: DisplayRowPosition,
        marker: SyntheticTextMarker,
        face_id: u32,
        base_face: &ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    ) -> Self {
        Self {
            position,
            source: SyntheticTextSource::marker(marker),
            face: SyntheticTextAppendFace::TextRowMetrics {
                face_id,
                base_face: base_face.clone(),
                height_px,
                ascent_px,
                char_width_px,
            },
        }
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        DisplayRowPosition,
        SyntheticTextSource,
        SyntheticTextAppendFace,
    ) {
        (self.position, self.source, self.face)
    }
}

#[derive(Clone)]
pub(crate) struct SyntheticTextAppendContext<'a> {
    face_id: u32,
    base_face: &'a ResolvedFace,
    frame: DisplayRowAppendFrame,
}

impl<'a> SyntheticTextAppendContext<'a> {
    pub(crate) fn new(
        face_id: u32,
        base_face: &'a ResolvedFace,
        frame: DisplayRowAppendFrame,
    ) -> Self {
        Self {
            face_id,
            base_face,
            frame,
        }
    }

    pub(crate) fn append_to_text_row_and_emit(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        position: DisplayRowPosition,
        source: SyntheticTextSource,
    ) -> Option<DisplayRowAppendProgress> {
        append_synthetic_text_to_display_row(
            state,
            self.base_face,
            self.frame.clone(),
            position,
            source,
            self.face_id,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SyntheticTextMarker {
    InvisibleEllipsis,
    HscrollTruncation,
    SelectiveEllipsis,
}

impl SyntheticTextMarker {
    pub(crate) fn source_id(self) -> u64 {
        match self {
            Self::InvisibleEllipsis => SYNTHETIC_SOURCE_INVISIBLE_ELLIPSIS,
            Self::HscrollTruncation => SYNTHETIC_SOURCE_HSCROLL_TRUNCATION,
            Self::SelectiveEllipsis => SYNTHETIC_SOURCE_SELECTIVE_ELLIPSIS,
        }
    }

    pub(crate) fn text(self) -> &'static str {
        match self {
            Self::InvisibleEllipsis | Self::SelectiveEllipsis => "...",
            Self::HscrollTruncation => "$",
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SyntheticTextRowAppendContext<'a> {
    active_face_context: DisplayRowActiveFaceAppendContext<'a, 'a>,
}

impl<'a> SyntheticTextRowAppendContext<'a> {
    pub(crate) fn new(
        append_surface: &'a DisplayRowAppendSurface,
        geometry: &'a DisplayRowGeometryState,
        active_face: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            active_face_context: DisplayRowActiveFaceAppendContext::new(
                append_surface,
                geometry,
                active_face,
                glyph_y_offset,
                default_row_height,
            ),
        }
    }

    fn active_face(
        self,
        face_id: u32,
        base_face: &'a ResolvedFace,
    ) -> SyntheticTextAppendContext<'a> {
        SyntheticTextAppendContext::new(
            face_id,
            base_face,
            self.active_face_context.active_face_frame(),
        )
    }

    fn text_row<'face>(
        self,
        face_id: u32,
        base_face: &'face ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    ) -> SyntheticTextAppendContext<'face> {
        SyntheticTextAppendContext::new(
            face_id,
            base_face,
            self.active_face_context
                .text_row_frame(height_px, ascent_px, char_width_px),
        )
    }

    pub(crate) fn append_request_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        request: SyntheticTextAppendRequest,
    ) -> Option<DisplayRowAppendProgress> {
        let (position, source, face) = request.into_parts();
        match face {
            SyntheticTextAppendFace::ActiveFace => {
                let active_face = self.active_face_context.active_face();
                self.active_face(active_face.face_id(), active_face.resolved_face())
                    .append_to_text_row_and_emit(state, position, source)
            }
            SyntheticTextAppendFace::TextRowMetrics {
                face_id,
                base_face,
                height_px,
                ascent_px,
                char_width_px,
            } => self
                .text_row(face_id, &base_face, height_px, ascent_px, char_width_px)
                .append_to_text_row_and_emit(state, position, source),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BufferSyntheticTextRenderContext<'a> {
    append_surface: &'a DisplayRowAppendSurface,
    active_face: &'a DisplayRowActiveFaceState,
    glyph_y_offset: f32,
    default_row_height: f32,
    default_row_ascent: f32,
    default_char_width: f32,
}

impl<'a> BufferSyntheticTextRenderContext<'a> {
    pub(crate) fn new(
        append_surface: &'a DisplayRowAppendSurface,
        active_face: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
        default_row_ascent: f32,
        default_char_width: f32,
    ) -> Self {
        Self {
            append_surface,
            active_face,
            glyph_y_offset,
            default_row_height,
            default_row_ascent,
            default_char_width,
        }
    }

    pub(crate) fn active_face(self) -> &'a DisplayRowActiveFaceState {
        self.active_face
    }

    fn row_context(
        self,
        geometry: &'a DisplayRowGeometryState,
    ) -> SyntheticTextRowAppendContext<'a> {
        SyntheticTextRowAppendContext::new(
            self.append_surface,
            geometry,
            self.active_face,
            self.glyph_y_offset,
            self.default_row_height,
        )
    }

    pub(crate) fn render_request_to_text_row<'face>(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        geometry: &'a DisplayRowGeometryState,
        request: SyntheticTextAppendRequest,
    ) -> Option<DisplayRowAppendProgress> {
        self.row_context(geometry)
            .append_request_to_text_row_and_emit(state, request)
    }

    #[cfg(test)]
    pub(crate) fn render_active_marker_to_text_row(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        geometry: &'a DisplayRowGeometryState,
        position: DisplayRowPosition,
        marker: SyntheticTextMarker,
    ) -> Option<DisplayRowPosition> {
        self.render_request_to_text_row(
            state,
            geometry,
            SyntheticTextAppendRequest::active_marker(position, marker),
        )
        .map(|progress| progress.end)
    }

    pub(crate) fn hscroll_truncation_request(
        self,
        base_face: ResolvedFace,
        content_x: f32,
    ) -> SyntheticTextAppendRequest {
        SyntheticTextAppendRequest::text_row_metrics_marker(
            DisplayRowPosition {
                x_px: content_x,
                col: 0,
            },
            SyntheticTextMarker::HscrollTruncation,
            BasicFaceId::Default.into(),
            &base_face,
            self.default_row_height,
            self.default_row_ascent,
            self.default_char_width,
        )
    }

    #[cfg(test)]
    pub(crate) fn render_hscroll_truncation_marker_to_text_row(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        geometry: &'a DisplayRowGeometryState,
        content_x: f32,
    ) -> Option<DisplayRowPosition> {
        let request = self.hscroll_truncation_request(state.default_face(), content_x);
        self.render_request_to_text_row(state, geometry, request)
            .map(|progress| progress.end)
    }
}

pub(crate) struct BufferSyntheticTextRenderState<'a> {
    source_render: TextRowSourceRenderState<'a>,
    x: &'a mut f32,
    col: &'a mut usize,
}

impl<'a> BufferSyntheticTextRenderState<'a> {
    pub(crate) fn new(
        source_render: TextRowSourceRenderState<'a>,
        x: &'a mut f32,
        col: &'a mut usize,
    ) -> Self {
        Self {
            source_render,
            x,
            col,
        }
    }

    pub(crate) fn position(&self) -> DisplayRowPosition {
        DisplayRowPosition {
            x_px: *self.x,
            col: *self.col,
        }
    }

    pub(crate) fn append_request_to_text_row<'ctx>(
        &mut self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        request: SyntheticTextAppendRequest,
    ) {
        let Some(progress) = render_context.render_request_to_text_row(
            &mut self.source_render,
            row_geometry,
            request,
        ) else {
            return;
        };
        *self.x = progress.end.x_px;
        *self.col = progress.end.col;
    }

    pub(crate) fn append_hscroll_truncation_marker_to_text_row<'ctx>(
        &mut self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        content_x: f32,
    ) {
        let request =
            render_context.hscroll_truncation_request(self.source_render.default_face(), content_x);
        self.append_request_to_text_row(render_context, row_geometry, request);
        self.source_render.mark_current_text_row_truncated_left();
    }
}

pub(crate) struct BufferLinePrefixRenderContext<'a> {
    values: DisplayRowPrefixValues,
    append_surface: &'a DisplayRowAppendSurface,
    row_geometry: &'a DisplayRowGeometryState,
    active_face_state: &'a DisplayRowActiveFaceState,
    glyph_y_offset: f32,
    default_row_height: f32,
}

pub(crate) struct BufferLinePrefixRenderRequest<'a> {
    context: BufferLinePrefixRenderContext<'a>,
    position: DisplayRowPosition,
}

impl<'a> BufferLinePrefixRenderContext<'a> {
    pub(crate) fn new(
        values: DisplayRowPrefixValues,
        append_surface: &'a DisplayRowAppendSurface,
        row_geometry: &'a DisplayRowGeometryState,
        active_face_state: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            values,
            append_surface,
            row_geometry,
            active_face_state,
            glyph_y_offset,
            default_row_height,
        }
    }

    pub(crate) fn render_requested_to_text_row_and_emit<B: LayoutBufferView>(
        self,
        request: &mut DisplayRowPrefixRequest,
        state: &mut TextRowSourceRenderState<'_>,
        buffer: &B,
        anchor_charpos: i64,
        face_ids: &mut FrameFaceIdAllocator,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        if !request.is_requested() {
            return position;
        }

        let text_props = RustTextPropAccess::new(buffer);
        let line_property = text_props.get_property(anchor_charpos, Value::symbol("line-prefix"));
        let wrap_property = text_props.get_property(anchor_charpos, Value::symbol("wrap-prefix"));
        let source = request.source_from_values(
            self.values.with_properties(line_property, wrap_property),
            CharPos0::new(anchor_charpos as usize),
        );
        request.clear();

        let Some(prefix_source) = source else {
            return position;
        };

        let prefix_base_face =
            state.default_display_string_base_face(buffer, prefix_source.origin(), face_ids);
        LispStringRowAppendContext::new(
            self.append_surface,
            self.row_geometry,
            self.active_face_state,
            self.glyph_y_offset,
            self.default_row_height,
        )
        .render_prefix_source_to_text_row_and_emit(
            state,
            face_ids,
            &prefix_base_face,
            prefix_source,
            position,
        )
    }
}

impl<'a> BufferLinePrefixRenderRequest<'a> {
    pub(crate) fn new(
        context: BufferLinePrefixRenderContext<'a>,
        position: DisplayRowPosition,
    ) -> Self {
        Self { context, position }
    }

    pub(crate) fn render_requested_with_source_state_and_apply<B: LayoutBufferView>(
        self,
        request: &mut DisplayRowPrefixRequest,
        source_render: &mut TextRowSourceRenderState<'_>,
        buffer: &B,
        anchor_charpos: i64,
        face_ids: &mut FrameFaceIdAllocator,
        x: &mut f32,
        col: &mut usize,
    ) {
        let position = self.context.render_requested_to_text_row_and_emit(
            request,
            source_render,
            buffer,
            anchor_charpos,
            face_ids,
            self.position,
        );
        *x = position.x_px;
        *col = position.col;
    }
}

pub(crate) struct BufferCurrentFaceResolutionContext<'a, B: LayoutBufferView> {
    buffer: &'a B,
    face_resolver: &'a FaceResolver,
    measurement_policy: DisplayRowMeasurementPolicy,
    default_resolved: &'a ResolvedFace,
    default_face_char_w: f32,
    default_face_ascent: f32,
    default_face_h: f32,
    char_w: f32,
    char_h: f32,
    font_ascent: f32,
    window_system: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferSourceItemLayoutResolutionContext<'a> {
    measurement_policy: DisplayRowMeasurementPolicy,
    default_resolved: &'a ResolvedFace,
    default_face_char_w: f32,
    default_face_ascent: f32,
    default_face_h: f32,
    char_w: f32,
    char_h: f32,
    font_ascent: f32,
    window_system: bool,
}

impl<'a, B: LayoutBufferView> Clone for BufferCurrentFaceResolutionContext<'a, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, B: LayoutBufferView> Copy for BufferCurrentFaceResolutionContext<'a, B> {}

impl<'a, B: LayoutBufferView> BufferCurrentFaceResolutionContext<'a, B> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        buffer: &'a B,
        face_resolver: &'a FaceResolver,
        measurement_policy: DisplayRowMeasurementPolicy,
        default_resolved: &'a ResolvedFace,
        default_face_char_w: f32,
        default_face_ascent: f32,
        default_face_h: f32,
        char_w: f32,
        char_h: f32,
        font_ascent: f32,
        window_system: bool,
    ) -> Self {
        Self {
            buffer,
            face_resolver,
            measurement_policy,
            default_resolved,
            default_face_char_w,
            default_face_ascent,
            default_face_h,
            char_w,
            char_h,
            font_ascent,
            window_system,
        }
    }

    pub(crate) fn resolve_at_checkpoint(
        &self,
        state: &mut BufferCurrentFaceResolutionState<'_, '_>,
        charpos: i64,
    ) -> bool {
        if !state.face_scan.should_resolve_at(charpos as usize) {
            return false;
        }

        let origin = DisplayOrigin::BufferText {
            charpos: neovm_core::buffer::CharPos0::new(charpos as usize),
        };
        let resolved = self.face_resolver.default_base_face_for_origin(
            Some(self.buffer),
            &origin,
            state.face_scan.next_check_mut(),
        );
        let face_id = state.face_ids.allocate();
        let resolved_extend = resolved.extend;
        let resolved_bg = resolved.bg;
        let resolved_box_type = resolved.box_type;
        *state.active_face_state = state.source_render.resolve_and_install_measured_face(
            self.measurement_policy,
            face_id,
            resolved,
            self.window_system,
            self.char_w,
            DisplayRowFallbackMetrics::from_default_face_extents(
                self.char_w,
                self.char_h,
                self.font_ascent,
            ),
        );
        let face_metrics = state.active_face_state.metrics();
        state
            .row_geometry
            .include_row_extents(face_metrics.row_height, face_metrics.ascent);

        if resolved_extend {
            let ext_bg = Color::from_pixel(resolved_bg);
            state
                .row_extend
                .activate(state.row_geometry.current_row_marker(), (ext_bg, face_id));
        }

        if state.box_face.is_active() && resolved_box_type == 0 {
            state.box_face.clear();
        }
        if resolved_box_type > 0 {
            state
                .box_face
                .activate(state.row_geometry.current_row_marker(), state.x);
        }
        true
    }

    pub(crate) fn source_item_layout_resolution_context(
        self,
    ) -> BufferSourceItemLayoutResolutionContext<'a> {
        BufferSourceItemLayoutResolutionContext {
            measurement_policy: self.measurement_policy,
            default_resolved: self.default_resolved,
            default_face_char_w: self.default_face_char_w,
            default_face_ascent: self.default_face_ascent,
            default_face_h: self.default_face_h,
            char_w: self.char_w,
            char_h: self.char_h,
            font_ascent: self.font_ascent,
            window_system: self.window_system,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_at_checkpoint_with_source_state(
        &self,
        source_render: &mut TextRowSourceRenderState<'_>,
        face_scan: &mut FaceScanCheckpoint,
        face_ids: &mut FrameFaceIdAllocator,
        active_face_state: &mut DisplayRowActiveFaceState,
        row_geometry: &mut DisplayRowGeometryState,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &mut BoxFaceRowState,
        x: f32,
        charpos: i64,
    ) -> bool {
        self.resolve_at_checkpoint(
            &mut BufferCurrentFaceResolutionState::new(
                source_render,
                face_scan,
                face_ids,
                active_face_state,
                row_geometry,
                row_extend,
                box_face,
                x,
            ),
            charpos,
        )
    }
}

impl BufferSourceItemLayoutResolutionContext<'_> {
    pub(crate) fn resolve_source_item_layout_for_active_face(
        &self,
        source_render: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        row_geometry: &mut DisplayRowGeometryState,
        active_face_state: &DisplayRowActiveFaceState,
        item: &mut DisplayItem,
    ) -> DisplayRowActiveFaceState {
        if matches!(item.face, RenderFaceRef::Inherit) {
            item.face = RenderFaceRef::FaceId(active_face_state.face_id());
        }

        let Some(factor) = item
            .layout
            .height
            .filter(|factor| factor.is_finite() && *factor > 0.0)
        else {
            return active_face_state.clone();
        };

        item.layout.height = None;
        let Some(resolved) = height_adjusted_face(
            active_face_state.resolved_face(),
            DisplayHeightFaceBasis {
                canonical_face: self.default_resolved,
                base_face: self.default_resolved,
                fallback_char_width: self.default_face_char_w,
                fallback_ascent: self.default_face_ascent,
                fallback_row_height: self.default_face_h,
            },
            factor,
        ) else {
            return active_face_state.clone();
        };

        let face_id = face_ids.allocate();
        item.face = RenderFaceRef::FaceId(face_id);
        let resolved_active_face = source_render.resolve_and_install_measured_face(
            self.measurement_policy,
            face_id,
            resolved,
            self.window_system,
            self.char_w,
            DisplayRowFallbackMetrics::from_default_face_extents(
                self.char_w,
                self.char_h,
                self.font_ascent,
            ),
        );
        let metrics = resolved_active_face.metrics();
        row_geometry.include_row_extents(metrics.row_height, metrics.ascent);
        resolved_active_face
    }
}

pub(crate) struct BufferCurrentFaceResolutionState<'a, 'source> {
    source_render: &'a mut TextRowSourceRenderState<'source>,
    face_scan: &'a mut FaceScanCheckpoint,
    face_ids: &'a mut FrameFaceIdAllocator,
    active_face_state: &'a mut DisplayRowActiveFaceState,
    row_geometry: &'a mut DisplayRowGeometryState,
    row_extend: &'a mut DisplayRowScopedValue<(Color, u32)>,
    box_face: &'a mut BoxFaceRowState,
    x: f32,
}

impl<'a, 'source> BufferCurrentFaceResolutionState<'a, 'source> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        source_render: &'a mut TextRowSourceRenderState<'source>,
        face_scan: &'a mut FaceScanCheckpoint,
        face_ids: &'a mut FrameFaceIdAllocator,
        active_face_state: &'a mut DisplayRowActiveFaceState,
        row_geometry: &'a mut DisplayRowGeometryState,
        row_extend: &'a mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &'a mut BoxFaceRowState,
        x: f32,
    ) -> Self {
        Self {
            source_render,
            face_scan,
            face_ids,
            active_face_state,
            row_geometry,
            row_extend,
            box_face,
            x,
        }
    }
}

pub(crate) enum BufferDisplayPropertyTextAppendAction {
    Replacement(BufferDisplayPropertyTextReplacementOutcome),
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferDisplayPropertyTextWalkOutcome {
    Continue,
    ReplacementConsumed,
}

pub(crate) struct BufferDisplayPropertyTextAppendRequest<'a> {
    source_event: BufferDisplayPropertyTextSourceEvent<'a>,
    context: BufferDisplayPropertyTextAppendContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferDisplayPropertyTextAppendContext<'a> {
    buffer_id: BufferId,
    active_face_state: &'a DisplayRowActiveFaceState,
    current_x: f32,
    content_x: f32,
    params: &'a WindowParams,
    glyph_y_offset: f32,
    default_row_height: f32,
    start_position: DisplayRowPosition,
}

pub(crate) struct BufferDisplayPropertyTextRenderContext<'a> {
    buffer_id: BufferId,
    text_start_byte: usize,
    text: &'a [u8],
    active_face_state: &'a DisplayRowActiveFaceState,
    current_x: f32,
    content_x: f32,
    params: &'a WindowParams,
    glyph_y_offset: f32,
    default_row_height: f32,
    start_position: DisplayRowPosition,
}

pub(crate) struct BufferDisplayPropertyCheckpointRenderRequest<'a, B: LayoutBufferView> {
    context: BufferDisplayPropertyCheckpointRenderContext<'a, B>,
}

pub(crate) struct BufferDisplayPropertyCheckpointRenderContext<'a, B: LayoutBufferView> {
    pub(crate) buffer: &'a B,
    pub(crate) buffer_id: BufferId,
    pub(crate) text_start_byte: usize,
    pub(crate) text: &'a [u8],
    pub(crate) current_x: f32,
    pub(crate) content_x: f32,
    pub(crate) params: &'a WindowParams,
    pub(crate) glyph_y_offset: f32,
    pub(crate) default_row_height: f32,
    pub(crate) start_position: DisplayRowPosition,
    pub(crate) charpos: i64,
    pub(crate) byte_idx: usize,
    pub(crate) accessible_end: i64,
}

pub(crate) struct BufferDisplayPropertyCheckpointRenderState<'a, 'emit> {
    source_render: TextRowSourceRenderState<'emit>,
    face_ids: &'emit mut FrameFaceIdAllocator,
    append_surface: &'a DisplayRowAppendSurface,
    row_geometry: &'emit mut DisplayRowGeometryState,
    checkpoints: &'emit mut TextPropertyScanCheckpoints,
    active_face_state: &'emit mut DisplayRowActiveFaceState,
    byte_idx: &'emit mut usize,
    charpos: &'emit mut i64,
    x: &'emit mut f32,
    col: &'emit mut usize,
    cursor_info: &'emit mut CursorCaptureState,
    point_charpos: i64,
}

impl<'a, 'emit> BufferDisplayPropertyCheckpointRenderState<'a, 'emit> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        source_render: TextRowSourceRenderState<'emit>,
        face_ids: &'emit mut FrameFaceIdAllocator,
        append_surface: &'a DisplayRowAppendSurface,
        row_geometry: &'emit mut DisplayRowGeometryState,
        checkpoints: &'emit mut TextPropertyScanCheckpoints,
        active_face_state: &'emit mut DisplayRowActiveFaceState,
        byte_idx: &'emit mut usize,
        charpos: &'emit mut i64,
        x: &'emit mut f32,
        col: &'emit mut usize,
        cursor_info: &'emit mut CursorCaptureState,
        point_charpos: i64,
    ) -> Self {
        Self {
            source_render,
            face_ids,
            append_surface,
            row_geometry,
            checkpoints,
            active_face_state,
            byte_idx,
            charpos,
            x,
            col,
            cursor_info,
            point_charpos,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferDisplayPropertyTextReplacementOutcome {
    pub(crate) replacement: DisplayPropertyReplacementAppendOutcome,
    pub(crate) skip_to: i64,
}

impl BufferDisplayPropertyTextWalkOutcome {
    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(self, Self::ReplacementConsumed)
    }
}

impl BufferDisplayPropertyTextAppendAction {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_to_buffer_walk_state(
        self,
        text: &[u8],
        byte_idx: &mut usize,
        charpos: &mut i64,
        x: &mut f32,
        col: &mut usize,
        cursor_info: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
    ) -> BufferDisplayPropertyTextWalkOutcome {
        match self {
            Self::Replacement(replacement_outcome) => {
                replacement_outcome.capture_cursor_info_if_point(
                    cursor_info,
                    active_face_state,
                    row_geometry,
                    point_charpos,
                    *charpos,
                    *byte_idx,
                );
                replacement_outcome.apply_to_walk_state(text, byte_idx, charpos, x, col);
                BufferDisplayPropertyTextWalkOutcome::ReplacementConsumed
            }
            Self::None => BufferDisplayPropertyTextWalkOutcome::Continue,
        }
    }
}

impl<'a> BufferDisplayPropertyTextRenderContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        buffer_id: BufferId,
        text_start_byte: usize,
        text: &'a [u8],
        active_face_state: &'a DisplayRowActiveFaceState,
        current_x: f32,
        content_x: f32,
        params: &'a WindowParams,
        glyph_y_offset: f32,
        default_row_height: f32,
        start_position: DisplayRowPosition,
    ) -> Self {
        Self {
            buffer_id,
            text_start_byte,
            text,
            active_face_state,
            current_x,
            content_x,
            params,
            glyph_y_offset,
            default_row_height,
            start_position,
        }
    }

    pub(crate) fn resolve_and_append_at_checkpoint<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        append_surface: &DisplayRowAppendSurface,
        row_geometry: &mut DisplayRowGeometryState,
        checkpoints: &mut TextPropertyScanCheckpoints,
        charpos: i64,
        byte_idx: usize,
        accessible_end: i64,
    ) -> BufferDisplayPropertyTextAppendAction {
        if !checkpoints.should_check_display(charpos) {
            return BufferDisplayPropertyTextAppendAction::None;
        }

        let text_props = RustTextPropAccess::new(buffer);
        let (display_property, next_change) = text_props.check_display_prop(charpos);
        checkpoints.record_display_next(next_change);
        let Some(value) = display_property else {
            return BufferDisplayPropertyTextAppendAction::None;
        };

        let source_event = BufferDisplayPropertyTextSourceEvent::new(
            value,
            self.text_start_byte,
            self.text,
            charpos,
            byte_idx,
            checkpoints.display_skip_to(accessible_end),
        );
        BufferDisplayPropertyTextAppendRequest::for_source_event(
            source_event,
            BufferDisplayPropertyTextAppendContext {
                buffer_id: self.buffer_id,
                active_face_state: self.active_face_state,
                current_x: self.current_x,
                content_x: self.content_x,
                params: self.params,
                glyph_y_offset: self.glyph_y_offset,
                default_row_height: self.default_row_height,
                start_position: self.start_position,
            },
        )
        .resolve_and_append_to_text_row(
            buffer,
            state,
            face_ids,
            append_surface,
            row_geometry,
        )
    }
}

impl<'a, B: LayoutBufferView> BufferDisplayPropertyCheckpointRenderRequest<'a, B> {
    pub(crate) fn new(context: BufferDisplayPropertyCheckpointRenderContext<'a, B>) -> Self {
        Self { context }
    }

    pub(crate) fn render_and_apply(
        self,
        state: BufferDisplayPropertyCheckpointRenderState<'_, '_>,
    ) -> BufferDisplayPropertyTextWalkOutcome {
        let BufferDisplayPropertyCheckpointRenderState {
            mut source_render,
            face_ids,
            append_surface,
            row_geometry,
            checkpoints,
            active_face_state,
            byte_idx,
            charpos,
            x,
            col,
            cursor_info,
            point_charpos,
        } = state;
        let context = self.context;

        let action = BufferDisplayPropertyTextRenderContext::new(
            context.buffer_id,
            context.text_start_byte,
            context.text,
            active_face_state,
            context.current_x,
            context.content_x,
            context.params,
            context.glyph_y_offset,
            context.default_row_height,
            context.start_position,
        )
        .resolve_and_append_at_checkpoint(
            context.buffer,
            &mut source_render,
            face_ids,
            append_surface,
            row_geometry,
            checkpoints,
            context.charpos,
            context.byte_idx,
            context.accessible_end,
        );
        let outcome = action.apply_to_buffer_walk_state(
            context.text,
            byte_idx,
            charpos,
            x,
            col,
            cursor_info,
            active_face_state,
            row_geometry,
            point_charpos,
        );
        outcome
    }
}

impl<'a> BufferDisplayPropertyTextAppendRequest<'a> {
    pub(crate) fn for_source_event(
        source_event: BufferDisplayPropertyTextSourceEvent<'a>,
        context: BufferDisplayPropertyTextAppendContext<'a>,
    ) -> Self {
        Self {
            source_event,
            context,
        }
    }

    pub(crate) fn resolve_and_append_to_text_row<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        append_surface: &DisplayRowAppendSurface,
        row_geometry: &mut DisplayRowGeometryState,
    ) -> BufferDisplayPropertyTextAppendAction {
        let context = self.context;
        let display_property = classify_display_property(self.source_event.value());
        let replacement_request = state.with_font_metrics_and_display_host(|font_metrics, host| {
            DisplayPropertyReplacementAppendRequestResolver::for_source_event(
                &display_property,
                context.buffer_id,
                self.source_event,
                context.active_face_state,
                context.current_x,
                context.content_x,
                context.params,
                context.glyph_y_offset,
                context.default_row_height,
                context.start_position,
            )
            .resolve(font_metrics, host)
        });
        if let Some(request) = replacement_request {
            let replacement = request.append_to_text_row(
                buffer,
                state,
                face_ids,
                append_surface,
                row_geometry,
                context.active_face_state,
            );
            return BufferDisplayPropertyTextAppendAction::Replacement(
                BufferDisplayPropertyTextReplacementOutcome {
                    replacement,
                    skip_to: self.source_event.skip_to(),
                },
            );
        }

        BufferDisplayPropertyTextAppendAction::None
    }
}

impl BufferDisplayPropertyTextReplacementOutcome {
    pub(crate) fn point_in_replacement(self, point_charpos: i64, start_charpos: i64) -> bool {
        point_charpos >= start_charpos && point_charpos < self.skip_to
    }

    pub(crate) fn start_position(self) -> DisplayRowPosition {
        self.replacement.start_position()
    }

    pub(crate) fn end_position(self) -> DisplayRowPosition {
        self.replacement.end_position()
    }

    pub(crate) fn skip_covered_buffer_text(
        self,
        text: &[u8],
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) {
        skip_text_to_charpos(text, byte_idx, charpos, self.skip_to);
    }

    pub(crate) fn capture_cursor_info_if_point(
        self,
        cursor_info: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        start_charpos: i64,
        byte_idx: usize,
    ) {
        if cursor_info.is_missing() && self.point_in_replacement(point_charpos, start_charpos) {
            let start_position = self.start_position();
            capture_cursor_info(
                cursor_info,
                self.cursor_info(
                    active_face_state,
                    row_geometry.text_position(start_position.x_px, byte_idx, start_position.col),
                ),
            );
        }
    }

    pub(crate) fn apply_to_walk_state(
        self,
        text: &[u8],
        byte_idx: &mut usize,
        charpos: &mut i64,
        x: &mut f32,
        col: &mut usize,
    ) {
        let position = self.end_position();
        *x = position.x_px;
        *col = position.col;
        self.skip_covered_buffer_text(text, byte_idx, charpos);
    }

    #[cfg(test)]
    pub(crate) fn skip_to(self) -> i64 {
        self.skip_to
    }

    pub(crate) fn cursor_info(
        self,
        active_face_state: &DisplayRowActiveFaceState,
        position: DisplayRowTextPosition,
    ) -> CapturedCursorInfo {
        self.replacement.cursor_info(active_face_state, position)
    }
}

pub(crate) struct BufferTextOverflowRenderState<'a, 'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) col: &'emit mut usize,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
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
    pub(crate) face_scan: &'emit mut FaceScanCheckpoint,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
}

pub(crate) struct BufferTextSpecialOverflowRenderState<'a, 'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) col: &'emit mut usize,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
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

pub(crate) struct BufferTextSourceCharRenderRequest<'a> {
    source_char: BufferTextSourceStepChar,
    source_item: DisplayItem,
    context: BufferTextSourceCharRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferTextSourceCharRenderContext<'a> {
    pub(crate) layout_resolution_context: BufferSourceItemLayoutResolutionContext<'a>,
    pub(crate) text: &'a [u8],
    pub(crate) text_start_byte: usize,
    pub(crate) buffer_id: BufferId,
    pub(crate) append_surface: &'a DisplayRowAppendSurface,
    pub(crate) overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
    pub(crate) active_face_state: &'a DisplayRowActiveFaceState,
    pub(crate) params: &'a WindowParams,
    pub(crate) glyph_y_offset: f32,
    pub(crate) char_h: f32,
    pub(crate) point_charpos: i64,
    pub(crate) row_visibility_limit: DisplayRowVisibilityLimit,
    pub(crate) content_x: f32,
    pub(crate) has_prefix: bool,
    pub(crate) row_geometry_defaults: DisplayRowGeometryDefaults,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) max_rows: usize,
    pub(crate) row_limit: DisplayRowLimit,
}

pub(crate) struct BufferTextSourceCharRenderRequestState<'a, 'emit> {
    pub(crate) append_state: &'emit mut BufferTextRowAppendState,
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) col: &'emit mut usize,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
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
    pub(crate) face_scan: &'emit mut FaceScanCheckpoint,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
}

pub(crate) struct BufferTextPlainRunRenderRequest<'a> {
    source_item: &'a BufferTextSourceItem,
    context: BufferTextPlainRunRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferTextPlainRunRenderContext<'a> {
    pub(crate) buffer_id: BufferId,
    pub(crate) append_surface: &'a DisplayRowAppendSurface,
    pub(crate) overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
    pub(crate) active_face_state: &'a DisplayRowActiveFaceState,
    pub(crate) params: &'a WindowParams,
    pub(crate) char_h: f32,
    pub(crate) point_charpos: i64,
}

pub(crate) struct BufferTextPlainRunRenderState<'emit> {
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextPlainRunRenderOutcome {
    Rendered,
    Stop,
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
    pub(crate) ch: char,
    pub(crate) right_edge_px: f32,
    pub(crate) wrap_mode: LineWrapMode,
    pub(crate) word_wrap: WordWrapRenderState,
    pub(crate) row_visibility_limit: DisplayRowVisibilityLimit,
    pub(crate) content_x: f32,
    pub(crate) has_prefix: bool,
    pub(crate) row_geometry_defaults: DisplayRowGeometryDefaults,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) max_rows: usize,
    pub(crate) row_limit: DisplayRowLimit,
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
pub(crate) enum BufferTextSourceCharRenderOutcome {
    Rendered,
    ContinueBufferWalk,
    Stop,
}

impl<'a> BufferTextPlainRunRenderRequest<'a> {
    pub(crate) fn new(
        source_item: &'a BufferTextSourceItem,
        context: BufferTextPlainRunRenderContext<'a>,
    ) -> Self {
        Self {
            source_item,
            context,
        }
    }

    pub(crate) fn render_if_eligible_and_apply<B: LayoutBufferView + ?Sized>(
        self,
        buffer: &B,
        state: BufferTextPlainRunRenderState<'_>,
    ) -> Option<BufferTextPlainRunRenderOutcome> {
        let BufferTextPlainRunRenderState {
            source_render,
            row_geometry,
            byte_idx,
            charpos,
            x,
            col,
            trailing_whitespace,
            word_wrap,
        } = state;
        let mut source_render = source_render;
        let context = self.context;
        let item = self.source_item.item();
        if item.face != RenderFaceRef::Inherit
            || item.layout.height.is_some()
            || item.layout.raise.is_some()
            || context.overlay_context.is_enabled()
            || context.params.word_wrap
            || context.params.wrap_mode != LineWrapMode::Truncate
        {
            return None;
        }

        let DisplayItemKind::TextRun(run) = &item.kind else {
            return None;
        };
        let text = run.text.as_ref();
        if text.is_empty() || text.chars().any(char::is_whitespace) {
            return None;
        }

        let char_count = i64::try_from(text.chars().count()).ok()?;
        if char_count <= 1 {
            return None;
        }
        let start_charpos = self.source_item.start_charpos();
        let end_charpos = start_charpos.checked_add(char_count)?;
        if context.point_charpos >= start_charpos && context.point_charpos < end_charpos {
            return None;
        }

        let mut direct_item = item.clone();
        direct_item.face = RenderFaceRef::FaceId(context.active_face_state.face_id());
        let append_position = DisplayRowPosition {
            x_px: *x,
            col: *col,
        };
        let row_append_context = BufferTextRowAppendContext::new(
            buffer,
            context.buffer_id,
            context.append_surface,
            context.active_face_state,
            0.0,
            context.char_h,
        );
        let measured_width = row_append_context.measure_source_display_item_width_to_text_row(
            row_geometry,
            &mut source_render.measure_state(),
            &direct_item,
            append_position,
            DisplayRowAppendKind::SourceText,
        )?;
        if append_position.x_px + measured_width > context.append_surface.right_edge() {
            return None;
        }

        let progress = row_append_context.append_source_display_item_naturally_to_text_row(
            row_geometry,
            &mut source_render,
            direct_item,
            append_position,
            DisplayRowAppendKind::SourceText,
        )?;
        if progress.status != DisplayRowAppendStatus::Complete {
            return Some(BufferTextPlainRunRenderOutcome::Stop);
        }

        trailing_whitespace.reset_after_row_transition();
        if let Some(last_char) = text.chars().last() {
            word_wrap.allow_after_current_char(last_char);
        }
        *byte_idx = self.source_item.start_byte_idx() + text.len();
        *charpos = end_charpos;
        *x = progress.end.x_px;
        *col = progress.end.col;
        Some(BufferTextPlainRunRenderOutcome::Rendered)
    }
}

pub(crate) struct BufferTextSourceCharRenderState<'a> {
    pub(crate) source_render: TextRowSourceRenderState<'a>,
    pub(crate) trailing_whitespace: &'a mut TrailingWhitespaceRenderState,
    pub(crate) word_wrap: &'a mut WordWrapRenderState,
    pub(crate) x: &'a mut f32,
    pub(crate) col: &'a mut usize,
    pub(crate) charpos: &'a mut i64,
}

impl<'a> BufferTextSourceCharRenderState<'a> {
    pub(crate) fn new(
        source_render: TextRowSourceRenderState<'a>,
        trailing_whitespace: &'a mut TrailingWhitespaceRenderState,
        word_wrap: &'a mut WordWrapRenderState,
        x: &'a mut f32,
        col: &'a mut usize,
        charpos: &'a mut i64,
    ) -> Self {
        Self {
            source_render,
            trailing_whitespace,
            word_wrap,
            x,
            col,
            charpos,
        }
    }
}

pub(crate) struct BufferTextSpecialSourceCharRenderState<'a> {
    pub(crate) face_ids: &'a mut FrameFaceIdAllocator,
    pub(crate) source_render: TextRowSourceRenderState<'a>,
    pub(crate) face_scan: &'a mut FaceScanCheckpoint,
    pub(crate) word_wrap: &'a mut WordWrapRenderState,
    pub(crate) x: &'a mut f32,
    pub(crate) col: &'a mut usize,
    pub(crate) charpos: &'a mut i64,
}

impl<'a> BufferTextSpecialSourceCharRenderState<'a> {
    pub(crate) fn new(
        face_ids: &'a mut FrameFaceIdAllocator,
        source_render: TextRowSourceRenderState<'a>,
        face_scan: &'a mut FaceScanCheckpoint,
        word_wrap: &'a mut WordWrapRenderState,
        x: &'a mut f32,
        col: &'a mut usize,
        charpos: &'a mut i64,
    ) -> Self {
        Self {
            face_ids,
            source_render,
            face_scan,
            word_wrap,
            x,
            col,
            charpos,
        }
    }
}

impl BufferTextSourceAppendContinuation {
    pub(crate) fn should_break(self) -> bool {
        matches!(self, Self::Stopped)
    }
}

impl BufferTextSourceCharRenderOutcome {
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

impl<'a> BufferTextSourceCharRenderRequest<'a> {
    pub(crate) fn new(
        source_step_char: BufferTextSourceStepChar,
        source_item: DisplayItem,
        context: BufferTextSourceCharRenderContext<'a>,
    ) -> Self {
        debug_assert_ne!(source_step_char.ch(), '\n');
        Self {
            source_char: source_step_char,
            source_item,
            context,
        }
    }

    pub(crate) fn render_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferTextSourceCharRenderRequestState<'_, '_>,
    ) -> BufferTextSourceCharRenderOutcome {
        let BufferTextSourceCharRenderRequestState {
            append_state,
            byte_idx,
            charpos,
            col,
            source_render,
            row_extend,
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
            face_scan,
            row_y_positions,
            cursor_info,
            face_ids,
        } = state;
        let mut source_render = source_render;
        let context = self.context;
        let mut source_item = self.source_item;
        let active_face_state = context
            .layout_resolution_context
            .resolve_source_item_layout_for_active_face(
                &mut source_render,
                face_ids,
                row_geometry,
                context.active_face_state,
                &mut source_item,
            );

        let source_step_char = self.source_char;
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
            x_px: *x,
            col: *col,
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
                    BufferTextSpecialOverflowRenderContext {
                        text: context.text,
                        text_start_byte: context.text_start_byte,
                        x_px: *x,
                        right_edge_px: context.append_surface.full_text_right_edge(),
                        wrap_mode: context.params.wrap_mode,
                        row_visibility_limit: context.row_visibility_limit,
                        content_x: context.content_x,
                        has_prefix: context.has_prefix,
                        row_geometry_defaults: context.row_geometry_defaults,
                        text_matrix_row_base: context.text_matrix_row_base,
                        max_rows: context.max_rows,
                        row_limit: context.row_limit,
                    },
                )
                .render_if_needed_and_apply(
                    buffer,
                    BufferTextSpecialOverflowRenderState {
                        byte_idx,
                        charpos,
                        col,
                        source_render: source_render.reborrow(),
                        row_extend,
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
                    },
                );
                if special_overflow_outcome.should_break() {
                    return BufferTextSourceCharRenderOutcome::Stop;
                }
                if special_overflow_outcome.should_continue_buffer_walk() {
                    return BufferTextSourceCharRenderOutcome::ContinueBufferWalk;
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
                            x,
                            col,
                            charpos,
                        ),
                    )
                    .should_break()
                {
                    return BufferTextSourceCharRenderOutcome::Stop;
                }
                return BufferTextSourceCharRenderOutcome::ContinueBufferWalk;
            }
            BufferTextPreparedSourceCharAppend::Text(prepared_append) => prepared_append,
        };

        prepared_append
            .update_cursor_info_for_main_char(cursor_info, source_step_char.start_byte_idx());
        let overflow_outcome = BufferTextOverflowRenderRequest::new(
            &prepared_append,
            source_step_char,
            BufferTextOverflowRenderContext {
                ch,
                right_edge_px: context.append_surface.right_edge(),
                wrap_mode: context.params.wrap_mode,
                word_wrap: *word_wrap,
                row_visibility_limit: context.row_visibility_limit,
                content_x: context.content_x,
                has_prefix: context.has_prefix,
                row_geometry_defaults: context.row_geometry_defaults,
                text_matrix_row_base: context.text_matrix_row_base,
                max_rows: context.max_rows,
                row_limit: context.row_limit,
            },
        )
        .render_if_needed_and_apply(
            context.text,
            BufferTextOverflowRenderState {
                byte_idx,
                charpos,
                col,
                source_render: source_render.reborrow(),
                row_extend,
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
                face_scan,
                row_y_positions,
            },
        );
        if overflow_outcome.should_break() {
            return BufferTextSourceCharRenderOutcome::Stop;
        }
        if overflow_outcome.should_continue_buffer_walk() {
            return BufferTextSourceCharRenderOutcome::ContinueBufferWalk;
        }

        {
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
            context.overlay_context.render_before_at(
                buffer,
                *charpos,
                &active_face_state,
                &mut overlay_state,
            );
        }

        prepared_append.capture_cursor_info_for_main_char_if_point(
            cursor_info,
            &active_face_state,
            row_geometry,
            *x,
            source_step_char.start_byte_idx(),
            *col,
            ch == '\t',
            *charpos,
            context.point_charpos,
        );

        if prepared_append
            .append_to_text_row_and_apply(
                &buffer_row_append_context,
                &append_geometry,
                ch,
                &mut BufferTextSourceCharRenderState::new(
                    source_render.reborrow(),
                    trailing_whitespace,
                    word_wrap,
                    x,
                    col,
                    charpos,
                ),
            )
            .should_break()
        {
            return BufferTextSourceCharRenderOutcome::Stop;
        }

        {
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
                &active_face_state,
                &mut overlay_state,
            );
        }

        BufferTextSourceCharRenderOutcome::Rendered
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

    pub(crate) fn render_if_needed_and_apply(
        self,
        text: &[u8],
        state: BufferTextOverflowRenderState<'_, '_>,
    ) -> BufferTextOverflowRenderOutcome {
        let BufferTextOverflowRenderState {
            byte_idx,
            charpos,
            col,
            source_render,
            row_extend,
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
            face_scan,
            row_y_positions,
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
                let truncation_skip =
                    BufferTextTruncationSkipAction::consume_source_step_char_and_rest_of_line(
                        text, byte_idx, charpos,
                    );
                truncation_skip.apply_before_row_transition(
                    line_numbers,
                    row_extend,
                    x,
                    context.content_x,
                );
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
                .emit_overflow_then_row_start(
                    transition,
                    hit_row_range.range_to(*charpos),
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
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
                BufferTextOverflowRenderOutcome::Transition(
                    truncation_skip.transition_continuation(row_transition),
                )
            }
            BufferTextSourceCharOverflowAction::WordWrap {
                break_candidate: wrap_break,
                transition,
            } => {
                let word_wrap_action = BufferTextWordWrapSourceAction::new(wrap_break);
                word_wrap_action.apply_before_row_transition(
                    source_render.output_emitter(),
                    byte_idx,
                    charpos,
                    col,
                    row_extend,
                    x,
                    context.content_x,
                );
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
                .emit_overflow(
                    transition,
                    hit_row_range.range_to(*charpos),
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
                );
                BufferTextOverflowRenderOutcome::Transition(
                    word_wrap_action.apply_after_row_transition_and_prefix(
                        row_transition,
                        transition,
                        charpos,
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
                    ),
                )
            }
            BufferTextSourceCharOverflowAction::CharacterWrap { transition } => {
                let character_wrap_action =
                    BufferTextCharacterWrapSourceAction::from_source_step_char(
                        self.source_step_char,
                    );
                character_wrap_action.apply_before_row_transition(row_extend, x, context.content_x);
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
                .emit_overflow_then_row_start(
                    transition,
                    hit_row_range.range_to(*charpos),
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
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
                BufferTextOverflowRenderOutcome::Transition(
                    character_wrap_action.apply_after_visible_row_transition(
                        row_transition,
                        byte_idx,
                        charpos,
                        hit_row_range,
                        face_scan,
                        row_geometry,
                        context.row_visibility_limit,
                    ),
                )
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextTruncationSkipAction {
    pub(crate) charpos: i64,
    pub(crate) reached_line_break: bool,
}

impl BufferTextTruncationSkipAction {
    pub(crate) fn consume_source_step_char_and_rest_of_line(
        text: &[u8],
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) -> Self {
        *charpos += 1;
        let reached_line_break = skip_to_newline(text, byte_idx, charpos);
        Self {
            charpos: *charpos,
            reached_line_break,
        }
    }

    #[cfg(test)]
    pub(crate) fn charpos(self) -> i64 {
        self.charpos
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
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) {
        *charpos = synced_charpos;
        hit_row_range.advance_to(*charpos);
    }

    pub(crate) fn transition_continuation(
        self,
        row_transition: TextMatrixRowTransition,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            DisplayRowTransitionContinuation::Exhausted
        } else {
            DisplayRowTransitionContinuation::Continue
        }
    }

    pub(crate) fn sync_after_row_transition_if_visible(
        self,
        row_transition: TextMatrixRowTransition,
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        Self::sync_after_row_transition(synced_charpos, charpos, hit_row_range);
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

    pub(crate) fn rewind_source_state(
        self,
        byte_idx: &mut usize,
        charpos: &mut i64,
        col: &mut usize,
    ) {
        *byte_idx = self.break_candidate.byte_idx();
        *charpos = self.break_candidate.charpos();
        *col = 0;
    }

    pub(crate) fn apply_before_row_transition(
        self,
        output_emitter: &mut WindowOutputEmitter,
        byte_idx: &mut usize,
        charpos: &mut i64,
        col: &mut usize,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        x: &mut f32,
        content_x: f32,
    ) {
        self.restore_row_output_progress(output_emitter);
        self.rewind_source_state(byte_idx, charpos, col);
        *x = content_x;
        row_extend.clear();
    }

    pub(crate) fn apply_after_row_transition(
        self,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
    ) {
        *charpos = self.charpos();
        hit_row_range.advance_to(*charpos);
        face_scan.invalidate();
    }

    pub(crate) fn apply_after_row_transition_and_prefix(
        self,
        row_transition: TextMatrixRowTransition,
        transition: DisplayRowOverflowTransitionPlan,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
        row_geometry: &DisplayRowGeometryState,
        row_visibility_limit: DisplayRowVisibilityLimit,
        render_state: DisplayRowTransitionRenderState<'_>,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        self.apply_after_row_transition(charpos, hit_row_range, face_scan);
        render_state.apply_overflow_prefix(transition);
        DisplayRowTransitionContinuation::after_visible_row_transition(
            row_transition,
            row_geometry,
            row_visibility_limit,
        )
    }

    pub(crate) fn charpos(self) -> i64 {
        self.break_candidate.charpos()
    }

    #[cfg(test)]
    pub(crate) fn byte_idx(self) -> usize {
        self.break_candidate.byte_idx()
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
        row_transition: TextMatrixRowTransition,
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

    pub(crate) fn rewind_source_state(self, byte_idx: &mut usize, charpos: &mut i64) {
        *byte_idx = self.ch_start_byte_idx;
        *charpos = self.ch_start_charpos;
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
        byte_idx: &mut usize,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
    ) {
        self.rewind_source_state(byte_idx, charpos);
        hit_row_range.advance_to(*charpos);
        face_scan.invalidate();
    }

    pub(crate) fn apply_after_visible_row_transition(
        self,
        row_transition: TextMatrixRowTransition,
        byte_idx: &mut usize,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
        row_geometry: &DisplayRowGeometryState,
        row_visibility_limit: DisplayRowVisibilityLimit,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        self.apply_after_row_transition(byte_idx, charpos, hit_row_range, face_scan);
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
    pub(crate) text: &'a [u8],
    pub(crate) text_start_byte: usize,
    pub(crate) x_px: f32,
    pub(crate) right_edge_px: f32,
    pub(crate) wrap_mode: LineWrapMode,
    pub(crate) row_visibility_limit: DisplayRowVisibilityLimit,
    pub(crate) content_x: f32,
    pub(crate) has_prefix: bool,
    pub(crate) row_geometry_defaults: DisplayRowGeometryDefaults,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) max_rows: usize,
    pub(crate) row_limit: DisplayRowLimit,
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
        buffer: &B,
        state: BufferTextSpecialOverflowRenderState<'_, '_>,
    ) -> BufferTextSpecialOverflowRenderOutcome {
        let BufferTextSpecialOverflowRenderState {
            byte_idx,
            charpos,
            col,
            source_render,
            row_extend,
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
                let truncation_skip =
                    BufferTextTruncationSkipAction::consume_source_step_char_and_rest_of_line(
                        context.text,
                        byte_idx,
                        charpos,
                    );
                truncation_skip.apply_before_row_transition(
                    line_numbers,
                    row_extend,
                    x,
                    context.content_x,
                );
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
                .emit_overflow_then_row_start(
                    transition,
                    hit_row_range.range_to(*charpos),
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
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
                BufferTextSpecialOverflowRenderOutcome::ContinueBufferWalk(
                    truncation_skip.sync_after_row_transition_if_visible(
                        row_transition,
                        synced_charpos,
                        charpos,
                        hit_row_range,
                    ),
                )
            }
            Some(BufferTextSpecialSourceCharOverflowAction::Wrap { transition }) => {
                let special_wrap_action = BufferTextSpecialWrapSourceAction::new(*charpos);
                special_wrap_action.apply_before_row_transition(row_extend, x, context.content_x);
                let hit_range = special_wrap_action.hit_range_and_advance(hit_row_range);
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
                .emit_overflow_then_row_start(
                    transition,
                    hit_range,
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
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
