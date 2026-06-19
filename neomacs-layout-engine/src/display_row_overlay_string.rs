//! Overlay before/after-string rendering — GNU `load_overlay_strings` / the
//! `it->overlay_strings` consumption (xdisp.c). Relocated out of
//! display_row_append.rs (pure move, no behavior change).

use crate::display_cursor::{
    CapturedCursorInfo, CapturedCursorPlacement, CapturedCursorSlotWidth,
    CapturedCursorVisualState, CursorCaptureState,
};
use crate::display_face_id::FrameFaceIdAllocator;
#[cfg(test)]
use crate::display_face_policy::BaseFacePolicy;
use crate::display_origin::{DisplayOrigin, OverlayStringKind};
use crate::display_row::{DisplayRowActiveFaceState, DisplayRowRenderStop};
use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_builder::{DisplayRowGlyphSlot, DisplayRowPosition};
use crate::display_row_geometry::{
    DisplayRowGeometryDefaults, DisplayRowGeometryState, DisplayRowLimit, DisplayRowYPositions,
    DisplayRowYRecording,
};
use crate::display_row_lisp_string::{
    LispStringSourceAppendRequest, LispStringSourceId, LispStringSourceRowAppendSession,
};
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_row_transition::DisplayRowLineBreakTransitionRequest;
use crate::display_row_walk_state::HitRowRangeTracker;
use crate::hit_test::HitRow;
use crate::neovm_bridge::{
    LayoutBufferView, OverlayDisplayString, ResolvedFace, RustTextPropAccess,
};
use neovm_core::buffer::CharPos0;
use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::value::get_string_text_properties_table_for_value;

#[derive(Clone, Copy)]
pub(crate) struct OverlayStringRenderSource {
    value: Value,
    overlay_id: Value,
    anchor_charpos: CharPos0,
    kind: OverlayStringKind,
}

impl OverlayStringRenderSource {
    pub(crate) fn new(
        overlay_string: OverlayDisplayString,
        anchor_charpos: CharPos0,
        kind: OverlayStringKind,
    ) -> Self {
        Self {
            value: overlay_string.string,
            overlay_id: overlay_string.overlay_id,
            anchor_charpos,
            kind,
        }
    }

    pub(crate) fn anchor_i64(self) -> i64 {
        self.anchor_charpos.get() as i64
    }

    pub(crate) fn value(self) -> Value {
        self.value
    }

    pub(crate) fn origin(self) -> DisplayOrigin {
        DisplayOrigin::OverlayString {
            overlay_id: self.overlay_id,
            anchor_charpos: self.anchor_charpos,
            kind: self.kind,
        }
    }

    #[cfg(test)]
    pub(crate) fn base_face_policy(self) -> BaseFacePolicy {
        self.origin().default_base_face_policy()
    }

    pub(crate) fn append_request(
        self,
        position: DisplayRowPosition,
    ) -> LispStringSourceAppendRequest {
        LispStringSourceAppendRequest::new(position, LispStringSourceId::OVERLAY_STRING, self.value)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct OverlayStringRenderRowContext<'a> {
    pub(crate) append_surface: &'a DisplayRowAppendSurface,
    pub(crate) face_char_w: f32,
    pub(crate) char_h: f32,
    pub(crate) default_row_ascent: f32,
    text_y: f32,
    pub(crate) row_base: usize,
    pub(crate) max_rows: usize,
}

impl<'a> OverlayStringRenderRowContext<'a> {
    pub(crate) fn new(
        append_surface: &'a DisplayRowAppendSurface,
        active_face_state: &DisplayRowActiveFaceState,
        char_h: f32,
        default_row_ascent: f32,
        text_y: f32,
        row_base: usize,
        max_rows: usize,
    ) -> Self {
        Self {
            append_surface,
            face_char_w: active_face_state.metrics().char_width,
            char_h,
            default_row_ascent,
            text_y,
            row_base,
            max_rows,
        }
    }

    pub(crate) fn content_x(self) -> f32 {
        self.append_surface.content_x()
    }

    pub(crate) fn right_edge(self) -> f32 {
        self.append_surface.right_edge()
    }

    pub(crate) fn geometry_defaults(self) -> DisplayRowGeometryDefaults {
        DisplayRowGeometryDefaults::new(self.text_y, self.char_h, self.default_row_ascent)
    }

    pub(crate) fn row_limit(self) -> DisplayRowLimit {
        DisplayRowLimit {
            max_rows: self.max_rows,
        }
    }

    pub(crate) fn cursor_visual_state(self, base_face: &ResolvedFace) -> CapturedCursorVisualState {
        CapturedCursorVisualState {
            face_width: self.face_char_w,
            face_height: self.char_h,
            face_ascent: self.default_row_ascent,
            background: neomacs_display_protocol::types::Color::from_pixel(base_face.bg),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BufferOverlayStringRenderContext<'a> {
    enabled: bool,
    window_id: u64,
    row_context: OverlayStringRenderRowContext<'a>,
}

pub(crate) struct OverlayStringRenderState<'a> {
    pub(crate) source_render: TextRowSourceRenderState<'a>,
    pub(crate) x: &'a mut f32,
    pub(crate) col: &'a mut usize,
    pub(crate) geometry: &'a mut DisplayRowGeometryState,
    pub(crate) cursor_info: &'a mut CursorCaptureState,
    pub(crate) hit_rows: &'a mut Vec<HitRow>,
    pub(crate) hit_row_range: &'a mut HitRowRangeTracker,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
    pub(crate) face_ids: &'a mut FrameFaceIdAllocator,
}

impl<'a> OverlayStringRenderState<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_source_render(
        source_render: TextRowSourceRenderState<'a>,
        x: &'a mut f32,
        col: &'a mut usize,
        geometry: &'a mut DisplayRowGeometryState,
        cursor_info: &'a mut CursorCaptureState,
        hit_rows: &'a mut Vec<HitRow>,
        hit_row_range: &'a mut HitRowRangeTracker,
        row_y_positions: &'a mut DisplayRowYPositions,
        face_ids: &'a mut FrameFaceIdAllocator,
    ) -> Self {
        Self {
            source_render,
            x,
            col,
            geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BufferOverlayStringTextRowRenderContext<'a> {
    enabled: bool,
    window_id: u64,
    append_surface: &'a DisplayRowAppendSurface,
    char_h: f32,
    default_row_ascent: f32,
    text_y: f32,
    row_base: usize,
    max_rows: usize,
}

impl<'a> BufferOverlayStringTextRowRenderContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        enabled: bool,
        window_id: u64,
        append_surface: &'a DisplayRowAppendSurface,
        char_h: f32,
        default_row_ascent: f32,
        text_y: f32,
        row_base: usize,
        max_rows: usize,
    ) -> Self {
        Self {
            enabled,
            window_id,
            append_surface,
            char_h,
            default_row_ascent,
            text_y,
            row_base,
            max_rows,
        }
    }

    pub(crate) fn is_enabled(self) -> bool {
        self.enabled
    }

    fn overlay_context(
        self,
        active_face_state: &DisplayRowActiveFaceState,
    ) -> BufferOverlayStringRenderContext<'a> {
        BufferOverlayStringRenderContext::for_text_row(
            self.enabled,
            self.window_id,
            self.append_surface,
            active_face_state,
            self.char_h,
            self.default_row_ascent,
            self.text_y,
            self.row_base,
            self.max_rows,
        )
    }

    pub(crate) fn render_before_at<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        active_face_state: &DisplayRowActiveFaceState,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        self.overlay_context(active_face_state)
            .render_before_at(buffer, anchor_charpos, state);
    }

    pub(crate) fn render_after_at<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        active_face_state: &DisplayRowActiveFaceState,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        self.overlay_context(active_face_state)
            .render_after_at(buffer, anchor_charpos, state);
    }

    pub(crate) fn render_both_at<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        active_face_state: &DisplayRowActiveFaceState,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        self.overlay_context(active_face_state)
            .render_both_at(buffer, anchor_charpos, state);
    }
}

impl<'a> BufferOverlayStringRenderContext<'a> {
    pub(crate) fn new(
        enabled: bool,
        window_id: u64,
        row_context: OverlayStringRenderRowContext<'a>,
    ) -> Self {
        Self {
            enabled,
            window_id,
            row_context,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_text_row(
        enabled: bool,
        window_id: u64,
        append_surface: &'a DisplayRowAppendSurface,
        active_face_state: &DisplayRowActiveFaceState,
        char_h: f32,
        default_row_ascent: f32,
        text_y: f32,
        row_base: usize,
        max_rows: usize,
    ) -> Self {
        Self::new(
            enabled,
            window_id,
            OverlayStringRenderRowContext::new(
                append_surface,
                active_face_state,
                char_h,
                default_row_ascent,
                text_y,
                row_base,
                max_rows,
            ),
        )
    }

    pub(crate) fn render_before_at<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        self.render_at_kind(buffer, anchor_charpos, OverlayStringKind::Before, state);
    }

    pub(crate) fn render_after_at<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        self.render_at_kind(buffer, anchor_charpos, OverlayStringKind::After, state);
    }

    pub(crate) fn render_both_at<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        self.render_before_at(buffer, anchor_charpos, state);
        self.render_after_at(buffer, anchor_charpos, state);
    }

    fn render_at_kind<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        kind: OverlayStringKind,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        if !self.enabled {
            return;
        }
        let text_props = RustTextPropAccess::new_for_window(buffer, self.window_id);
        // overlay_strings_at now returns one GNU-ordered interleaved list; pick
        // the entries of this kind (within-kind order is preserved, so this is
        // behavior-neutral vs the old per-kind sort).
        let want_after = matches!(kind, OverlayStringKind::After);
        let overlay_strings: Vec<_> = text_props
            .overlay_strings_at(anchor_charpos)
            .into_iter()
            .filter(|entry| entry.after_string_p == want_after)
            .collect();
        for overlay_string in overlay_strings {
            render_overlay_string(
                buffer,
                OverlayStringRenderSource::new(
                    overlay_string,
                    CharPos0::new(anchor_charpos as usize),
                    kind,
                ),
                self.row_context,
                state,
            );
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct OverlayStringRowBreakRenderContext<'a> {
    anchor_charpos: i64,
    row_context: OverlayStringRenderRowContext<'a>,
}

impl<'a> OverlayStringRowBreakRenderContext<'a> {
    pub(crate) fn new(anchor_charpos: i64, row_context: OverlayStringRenderRowContext<'a>) -> Self {
        Self {
            anchor_charpos,
            row_context,
        }
    }

    pub(crate) fn finish_row(self, state: &mut OverlayStringRenderState<'_>) -> bool {
        let content_x = self.row_context.content_x();
        let geometry_transition = DisplayRowLineBreakTransitionRequest::new(
            state.hit_row_range.range_to(self.anchor_charpos),
            self.row_context.geometry_defaults(),
            self.row_context.row_base,
            0,
            content_x,
            0.0,
            DisplayRowYRecording::None,
            self.row_context.max_rows,
        )
        .finish_geometry(state.geometry, state.hit_rows);

        state.hit_row_range.advance_to(self.anchor_charpos);
        if !state
            .geometry
            .is_within_row_limit(self.row_context.row_limit())
        {
            state
                .source_render
                .output_render()
                .finish_and_end_text_matrix_row_output(geometry_transition.finished_row);
            return false;
        }

        state.geometry.record_current_row_y(state.row_y_positions);
        *state.x = content_x;
        *state.col = 0;
        state
            .source_render
            .output_render()
            .emit_text_matrix_row_transition(geometry_transition);
        true
    }
}

pub(crate) fn render_overlay_string<B: LayoutBufferView>(
    buffer: &B,
    source_request: OverlayStringRenderSource,
    row_context: OverlayStringRenderRowContext<'_>,
    state: &mut OverlayStringRenderState<'_>,
) {
    let anchor_charpos = source_request.anchor_i64();
    let text_value = source_request.value();
    if text_value.as_lisp_string().is_none() {
        return;
    }
    let text_props = get_string_text_properties_table_for_value(text_value);
    let base_face = state.source_render.default_display_string_base_face(
        buffer,
        source_request.origin(),
        state.face_ids,
    );
    let max_x = row_context.right_edge();
    let row_limit = row_context.row_limit();
    let row_break_context = OverlayStringRowBreakRenderContext::new(anchor_charpos, row_context);
    let append_request = source_request.append_request(DisplayRowPosition {
        x_px: *state.x,
        col: *state.col,
    });
    let Some(mut source_context) = LispStringSourceRowAppendSession::new(
        append_request,
        base_face.face_id(),
        base_face.face(),
        row_context.append_surface,
        0.0,
        row_context.char_h,
        row_context.default_row_ascent,
        row_context.face_char_w,
        row_context.char_h,
    ) else {
        return;
    };

    while state.geometry.is_within_row_limit(row_limit) {
        if *state.x >= max_x {
            break;
        }

        let Some(outcome) = source_context.render_to_text_row_and_emit(
            &mut state.source_render,
            state.face_ids,
            state.geometry,
            DisplayRowPosition {
                x_px: *state.x,
                col: *state.col,
            },
        ) else {
            break;
        };
        let stop = outcome.stop();
        outcome.include_vertical_metrics(state.geometry);
        let overlay_cursor_visual_state = row_context.cursor_visual_state(base_face.face());
        for slot in outcome.source_slots() {
            capture_overlay_string_cursor_at_slot(
                text_props.as_ref(),
                slot,
                state.cursor_info,
                state.geometry.y(),
                state.geometry.row(),
                overlay_cursor_visual_state,
            );
        }
        let end = outcome.end_position();
        *state.x = end.x_px;
        *state.col = end.col;

        if stop == DisplayRowRenderStop::RowBreak {
            if !row_break_context.finish_row(state) {
                break;
            }
            continue;
        }
        match stop {
            DisplayRowRenderStop::SourceExhausted => break,
            DisplayRowRenderStop::Clipped => {
                if source_context.discard_pending_until_row_break() {
                    if !row_break_context.finish_row(state) {
                        break;
                    }
                    continue;
                }
                break;
            }
            DisplayRowRenderStop::RowBreak => unreachable!("row break handled above"),
        }
    }
}

fn root_lisp_position_char(source: &crate::display_item::DisplaySourcePosition) -> Option<usize> {
    match source {
        crate::display_item::DisplaySourcePosition::LispString {
            source_id,
            char_index,
            ..
        } if source_id.get() == LispStringSourceId::OVERLAY_STRING.raw() => Some(*char_index),
        _ => None,
    }
}

fn capture_overlay_string_cursor_at_slot(
    text_props: Option<&neovm_core::buffer::text_props::TextPropertyTable>,
    slot: &DisplayRowGlyphSlot,
    cursor_info: &mut CursorCaptureState,
    y: f32,
    matrix_row: usize,
    visual_state: CapturedCursorVisualState,
) {
    let Some(char_idx) = root_lisp_position_char(&slot.source) else {
        return;
    };
    capture_overlay_string_cursor(
        text_props,
        char_idx,
        cursor_info,
        slot.x_px,
        y,
        slot.col,
        matrix_row,
        visual_state,
        CapturedCursorSlotWidth::Explicit(slot.width_px),
    );
}

#[allow(clippy::too_many_arguments)]
fn capture_overlay_string_cursor(
    text_props: Option<&neovm_core::buffer::text_props::TextPropertyTable>,
    char_idx: usize,
    cursor_info: &mut CursorCaptureState,
    x: f32,
    y: f32,
    col: usize,
    matrix_row: usize,
    visual_state: CapturedCursorVisualState,
    slot_width: CapturedCursorSlotWidth,
) {
    let Some(props) = text_props else {
        return;
    };
    let Some(cursor_prop) =
        props.get_property_at_char_pos(CharPos0::new(char_idx), Value::symbol("cursor"))
    else {
        return;
    };
    if cursor_prop.is_nil() {
        return;
    }

    let info = CapturedCursorInfo::from_visual_state(
        visual_state,
        CapturedCursorPlacement {
            x,
            y,
            byte_idx: 0,
            col,
            matrix_row,
            slot_width,
            stretch_like: false,
        },
    );
    cursor_info.capture_string_cursor_property(info);
}
