//! Overlay before/after-string rendering — GNU `load_overlay_strings` / the
//! `it->overlay_strings` consumption (xdisp.c). Relocated out of
//! display_row_append.rs (pure move, no behavior change).

use crate::display_cursor::{
    CapturedCursorInfo, CapturedCursorPlacement, CapturedCursorSlotWidth,
    CapturedCursorVisualState, CursorCaptureState,
};
use crate::display_origin::OverlayStringKind;
use crate::display_row::DisplayRowRenderStop;
use crate::display_row_append::{
    DisplayRowLineBreakTransitionRequest, LispStringSourceId, LispStringSourceRowAppendSession,
    OverlayStringRenderRowContext, OverlayStringRenderSource, OverlayStringRenderState,
};
use crate::display_row_builder::{DisplayRowGlyphSlot, DisplayRowPosition};
use crate::display_row_geometry::DisplayRowYRecording;
use crate::neovm_bridge::{LayoutBufferView, OverlayDisplayString};
use neovm_core::buffer::CharPos0;
use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::value::get_string_text_properties_table_for_value;

#[derive(Clone, Copy)]
pub(crate) struct OverlayStringRenderBatchSource<'a> {
    overlay_strings: &'a [OverlayDisplayString],
    anchor_charpos: CharPos0,
    kind: OverlayStringKind,
}

impl<'a> OverlayStringRenderBatchSource<'a> {
    pub(crate) fn new(
        overlay_strings: &'a [OverlayDisplayString],
        anchor_charpos: CharPos0,
        kind: OverlayStringKind,
    ) -> Self {
        Self {
            overlay_strings,
            anchor_charpos,
            kind,
        }
    }

    pub(crate) fn is_empty(self) -> bool {
        self.overlay_strings.is_empty()
    }

    pub(crate) fn overlay_strings(self) -> &'a [OverlayDisplayString] {
        self.overlay_strings
    }

    pub(crate) fn source_for(
        self,
        overlay_string: OverlayDisplayString,
    ) -> OverlayStringRenderSource {
        OverlayStringRenderSource::new(overlay_string, self.anchor_charpos, self.kind)
    }
}

pub(crate) fn render_overlay_string_batch<B: LayoutBufferView>(
    buffer: &B,
    source_batch: OverlayStringRenderBatchSource<'_>,
    row_context: OverlayStringRenderRowContext<'_>,
    state: &mut OverlayStringRenderState<'_>,
) {
    if source_batch.is_empty() {
        return;
    }
    for overlay_string in source_batch.overlay_strings() {
        render_overlay_string(
            buffer,
            source_batch.source_for(*overlay_string),
            row_context,
            state,
        );
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

fn render_overlay_string<B: LayoutBufferView>(
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
