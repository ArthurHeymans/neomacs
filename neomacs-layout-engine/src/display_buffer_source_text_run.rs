//! Whole text-run rendering for buffer source items.

use crate::coords::layout_i64_char_pos_to_lisp_char_pos;
use crate::display_buffer_source_item_append::BufferSourceRowAppendContext;
use crate::display_buffer_source_item_render::BufferSourceItemRenderOutcome;
use crate::display_cursor::{
    CapturedCursorInfo, CapturedCursorPlacement, CapturedCursorSlotWidth, CursorCaptureState,
    capture_cursor_info,
};
use crate::display_item::DisplaySourcePosition;
use crate::display_row_append_context::DisplayRowAppendKind;
use crate::display_row_builder::{
    DisplayRowAppendProgress, DisplayRowGlyphSlot, DisplayRowPosition,
};
use crate::display_row_face_state::DisplayRowActiveFaceState;
use crate::display_row_geometry::DisplayRowGeometryState;
use crate::display_row_overlay_string::BufferOverlayStringTextRowRenderContext;
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_row_walk_state::{TrailingWhitespaceRenderState, WordWrapRenderState};
use crate::display_source::DisplaySourceStepItem;
use crate::display_source_append_plan::DisplaySourceAppendRenderPolicy;
use crate::display_source_progress::DisplaySourceProgressState;
use crate::neovm_bridge::LayoutBufferView;
use crate::types::LineWrapMode;
use neovm_core::buffer::LispCharPos1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WholeTextRunRenderDecision {
    Render,
    Fallback(WholeTextRunFallbackReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WholeTextRunFallbackReason {
    NotTextRun,
    MissingSourceEnd,
    OverlayString,
    DoesNotFit,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferSourceTextRunRenderRequest<'a> {
    text_start_byte: usize,
    overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
    point_charpos: i64,
    right_edge_px: f32,
    position: DisplayRowPosition,
    geometry: DisplayRowGeometryState,
}

impl<'a> BufferSourceTextRunRenderRequest<'a> {
    pub(crate) fn new(
        text_start_byte: usize,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        point_charpos: i64,
        right_edge_px: f32,
        position: DisplayRowPosition,
        geometry: DisplayRowGeometryState,
    ) -> Self {
        Self {
            text_start_byte,
            overlay_context,
            point_charpos,
            right_edge_px,
            position,
            geometry,
        }
    }

    pub(crate) fn split_at_first_overlay<B: LayoutBufferView>(
        self,
        source_item: &DisplaySourceStepItem,
        buffer: &B,
    ) -> Option<(DisplaySourceStepItem, DisplaySourceStepItem)> {
        let source_end_charpos = source_item.source_end_charpos()?;
        let first_overlay_charpos = self.overlay_context.first_overlay_string_charpos_in_range(
            buffer,
            source_item.source_step_char().start_charpos(),
            source_end_charpos,
        )?;
        source_item
            .clone()
            .split_text_run_at_charpos(first_overlay_charpos, self.text_start_byte)
    }

    pub(crate) fn split_prefix_to_fit<B: LayoutBufferView>(
        self,
        source_item: &DisplaySourceStepItem,
        wrap_mode: LineWrapMode,
        append_context: &BufferSourceRowAppendContext<'_, '_, B>,
        source_render: &mut TextRowSourceRenderState<'_>,
    ) -> Option<(DisplaySourceStepItem, DisplaySourceStepItem)> {
        if wrap_mode != LineWrapMode::Truncate {
            return None;
        }
        let text = source_item.text_run()?;
        let start_charpos = source_item.source_step_char().start_charpos();
        let end_charpos = source_item.source_end_charpos()?;
        let mut last_fit_split_charpos = None;
        for char_offset in 1..text.chars().count() {
            let split_charpos = start_charpos.saturating_add(char_offset as i64);
            if split_charpos >= end_charpos {
                break;
            }
            let (prefix, _) = source_item
                .clone()
                .split_text_run_at_charpos(split_charpos, self.text_start_byte)?;
            if self.source_display_item_fits_text_row(&prefix, append_context, source_render) {
                last_fit_split_charpos = Some(split_charpos);
            } else {
                break;
            }
        }
        source_item
            .clone()
            .split_text_run_at_charpos(last_fit_split_charpos?, self.text_start_byte)
    }

    pub(crate) fn render_if_fits_and_apply<B: LayoutBufferView>(
        self,
        source_item: DisplaySourceStepItem,
        buffer: &B,
        active_face_state: &DisplayRowActiveFaceState,
        append_context: &BufferSourceRowAppendContext<'_, '_, B>,
        cursor_info: &mut CursorCaptureState,
        trailing_whitespace: &mut TrailingWhitespaceRenderState,
        word_wrap: &mut WordWrapRenderState,
        source_render: &mut TextRowSourceRenderState<'_>,
        progress: &mut DisplaySourceProgressState<'_>,
    ) -> Option<BufferSourceItemRenderOutcome> {
        if self.render_decision(&source_item, buffer, append_context, source_render)
            != WholeTextRunRenderDecision::Render
        {
            return None;
        }
        Some(self.render_and_apply(
            source_item,
            active_face_state,
            append_context,
            cursor_info,
            trailing_whitespace,
            word_wrap,
            source_render,
            progress,
        ))
    }

    pub(crate) fn render_and_apply<B: LayoutBufferView + ?Sized>(
        self,
        source_item: DisplaySourceStepItem,
        active_face_state: &DisplayRowActiveFaceState,
        append_context: &BufferSourceRowAppendContext<'_, '_, B>,
        cursor_info: &mut CursorCaptureState,
        trailing_whitespace: &mut TrailingWhitespaceRenderState,
        word_wrap: &mut WordWrapRenderState,
        source_render: &mut TextRowSourceRenderState<'_>,
        progress: &mut DisplaySourceProgressState<'_>,
    ) -> BufferSourceItemRenderOutcome {
        let output_display_point_start = source_render.output_emitter().display_point_len();
        let output_row_positions_start = source_render
            .output_emitter()
            .current_row_display_positions();
        let source_end_charpos = source_item.source_end_charpos();
        let source_end_byte_idx = source_item.source_end_byte_idx();
        let source_text = source_item.raw_text_run().unwrap_or_default().to_owned();
        let (_source_step_char, _, _, source_item) = source_item.into_render_parts();
        let mut render_policy = DisplaySourceAppendRenderPolicy::natural();
        let Some(append_progress) = append_context.append_source_display_item_to_text_row(
            &self.geometry,
            source_render,
            source_item,
            self.position,
            DisplayRowAppendKind::SourceText,
            &mut render_policy,
        ) else {
            return BufferSourceItemRenderOutcome::Stop;
        };
        capture_whole_text_run_cursor_if_point(
            cursor_info,
            active_face_state,
            &self.geometry,
            self.point_charpos,
            &append_progress,
        );
        apply_whole_text_run_trailing_whitespace_state(
            &source_text,
            trailing_whitespace,
            &self.geometry,
            &append_progress,
        );
        apply_whole_text_run_word_wrap_state(
            &source_text,
            word_wrap,
            output_display_point_start,
            output_row_positions_start,
            &append_progress,
        );
        progress.apply_row_position(append_progress.end());
        if let Some(end_charpos) = source_end_charpos {
            progress.max_charpos(end_charpos);
        }
        if let Some(end_byte_idx) = source_end_byte_idx {
            progress.set_byte_idx(end_byte_idx);
        }
        BufferSourceItemRenderOutcome::Rendered
    }

    fn render_decision<B: LayoutBufferView>(
        self,
        source_item: &DisplaySourceStepItem,
        buffer: &B,
        append_context: &BufferSourceRowAppendContext<'_, '_, B>,
        source_render: &mut TextRowSourceRenderState<'_>,
    ) -> WholeTextRunRenderDecision {
        if source_item.text_run().is_none() {
            return WholeTextRunRenderDecision::Fallback(WholeTextRunFallbackReason::NotTextRun);
        }
        let Some(source_end_charpos) = source_item.source_end_charpos() else {
            return WholeTextRunRenderDecision::Fallback(
                WholeTextRunFallbackReason::MissingSourceEnd,
            );
        };
        if source_item.source_end_byte_idx().is_none() {
            return WholeTextRunRenderDecision::Fallback(
                WholeTextRunFallbackReason::MissingSourceEnd,
            );
        }
        if self
            .overlay_context
            .first_overlay_string_charpos_in_range(
                buffer,
                source_item.source_step_char().start_charpos(),
                source_end_charpos,
            )
            .is_some()
        {
            return WholeTextRunRenderDecision::Fallback(WholeTextRunFallbackReason::OverlayString);
        }

        if self.source_display_item_fits_text_row(source_item, append_context, source_render) {
            WholeTextRunRenderDecision::Render
        } else {
            WholeTextRunRenderDecision::Fallback(WholeTextRunFallbackReason::DoesNotFit)
        }
    }

    fn source_display_item_fits_text_row<B: LayoutBufferView>(
        self,
        source_item: &DisplaySourceStepItem,
        append_context: &BufferSourceRowAppendContext<'_, '_, B>,
        source_render: &mut TextRowSourceRenderState<'_>,
    ) -> bool {
        let measured_width = {
            let mut measure = source_render.measure_state();
            append_context.measure_source_display_item_width_naturally(
                &self.geometry,
                &mut measure,
                source_item.item(),
                self.position,
                DisplayRowAppendKind::SourceText,
            )
        };
        measured_width
            .map(|width| self.position.x_px() + width <= self.right_edge_px + f32::EPSILON)
            .unwrap_or(false)
    }
}

fn buffer_slot_matches_charpos(slot: &DisplayRowGlyphSlot, point_charpos: i64) -> bool {
    let DisplaySourcePosition::Buffer { char_pos, .. } = slot.source() else {
        return false;
    };
    char_pos.get() as i64 == point_charpos
}

fn buffer_slot_source_position(slot: &DisplayRowGlyphSlot) -> Option<(usize, i64)> {
    let DisplaySourcePosition::Buffer {
        char_pos, byte_pos, ..
    } = slot.source()
    else {
        return None;
    };
    Some((byte_pos.get(), char_pos.get() as i64))
}

fn capture_whole_text_run_cursor_if_point(
    cursor_info: &mut CursorCaptureState,
    active_face_state: &DisplayRowActiveFaceState,
    geometry: &DisplayRowGeometryState,
    point_charpos: i64,
    append_progress: &DisplayRowAppendProgress,
) {
    if !cursor_info.is_missing() {
        return;
    }
    let Some(slot) = append_progress
        .slots()
        .iter()
        .find(|slot| buffer_slot_matches_charpos(slot, point_charpos))
    else {
        return;
    };
    let DisplaySourcePosition::Buffer { byte_pos, .. } = slot.source() else {
        return;
    };
    capture_cursor_info(
        cursor_info,
        CapturedCursorInfo::from_active_face_state(
            active_face_state,
            CapturedCursorPlacement::from_row_text_position(
                geometry.text_position(slot.x_px(), byte_pos.get(), slot.col()),
                CapturedCursorSlotWidth::Explicit(slot.width_px()),
                false,
            ),
        ),
    );
}

fn apply_whole_text_run_trailing_whitespace_state(
    text: &str,
    trailing_whitespace: &mut TrailingWhitespaceRenderState,
    geometry: &DisplayRowGeometryState,
    append_progress: &DisplayRowAppendProgress,
) {
    if !trailing_whitespace.is_enabled() {
        return;
    }
    for (ch, slot) in text.chars().zip(append_progress.slots()) {
        trailing_whitespace.track_rendered_char(ch, geometry.start_marker_at_x(slot.x_px()));
    }
}

fn apply_whole_text_run_word_wrap_state(
    text: &str,
    word_wrap: &mut WordWrapRenderState,
    output_display_point_start: usize,
    output_row_positions_start: (Option<LispCharPos1>, Option<LispCharPos1>),
    append_progress: &DisplayRowAppendProgress,
) {
    if !word_wrap.is_enabled() {
        return;
    }
    let mut first_run_charpos = output_row_positions_start.0;
    let mut previous_charpos = output_row_positions_start.1;
    for (char_offset, (ch, slot)) in text.chars().zip(append_progress.slots()).enumerate() {
        if let Some((byte_idx, charpos)) = buffer_slot_source_position(slot) {
            let row_first =
                first_run_charpos.or_else(|| Some(layout_i64_char_pos_to_lisp_char_pos(charpos)));
            if word_wrap.can_record_candidate(ch) {
                word_wrap.record_candidate(
                    ch,
                    byte_idx,
                    charpos,
                    output_display_point_start + char_offset,
                    (row_first, previous_charpos),
                );
            }
            first_run_charpos = row_first;
            previous_charpos = Some(layout_i64_char_pos_to_lisp_char_pos(charpos));
        }
        word_wrap.allow_after_current_char(ch);
    }
}
