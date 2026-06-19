//! Buffer-text-window-specific append surface construction.
//!
//! This module holds helpers that translate a buffer text window's geometry
//! and chrome reservation policy into a generic `DisplayRowAppendSurface`.
//! Keeping it separate from `display_row_append.rs` lets the append layer stay
//! source-agnostic while the buffer text walker owns its own setup logic.

use crate::display_row::insert_resolved_display_row_face;
use crate::display_row_append_context::{DisplayRowAppendArea, DisplayRowAppendSurface};
use crate::display_row_builder::DisplayTabPolicy;
use crate::display_row_geometry::{
    DisplayRowFlags, DisplayRowGeometryState, DisplayRowLimit, DisplayRowYPositions,
};
use crate::display_row_source_render::TextRowOutputRenderState;
use crate::display_row_special_glyphs::{
    install_last_window_right_border_from_source_requests,
    install_right_edge_markers_from_source_requests,
};
use crate::display_row_walk_state::{
    HitRowRangeTracker, next_window_start_for_partially_visible_point_row,
    next_window_start_for_point_line_continuation, next_window_start_from_visible_rows,
};
use crate::display_status_line::ChromeRowRenderServices;
use crate::hit_test::{HitRow, WindowHitData};
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::neovm_bridge::{LayoutBufferView, RustBufferAccess};
use crate::types::WindowParams;
use crate::window_output::{
    TextMatrixRowBegin, TextWindowBegin, TextWindowBodyOutputInstall, TextWindowCursorEffects,
    TextWindowOutputInstaller, TextWindowPendingRowFinish, TextWindowRedisplayPositions,
    TextWindowRightBorder, TextWindowRightEdgeMarkers, TextWindowRowLifecycleInstaller,
    WindowOutputEmitter, close_text_window_output, install_text_window_cursor_effects,
};
use neomacs_display_protocol::effect_config::EffectsConfig;
use neovm_core::buffer::LispCharPos1;
use neovm_core::emacs_core::Context;
use neovm_core::window::{DisplayRowSnapshot, FrameId, WindowDisplaySnapshot, WindowId};

use crate::coords::layout_i64_char_pos_to_lisp_char_pos;
use crate::display_cursor::{
    CapturedTextWindowCursorPublishContext, CapturedTextWindowCursorPublishOutcome,
    CursorCaptureState, VisualTextWindowCursorPublishContext, VisualTextWindowCursorPublishSummary,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextWindowAppendSurfaceRequest<'a> {
    content_x: f32,
    text_width: f32,
    line_number_width: f32,
    reserve_right_border_col: bool,
    reserve_right_special_col: bool,
    char_width: f32,
    tab_width: i32,
    tab_stop_list: &'a [i32],
}

impl<'a> TextWindowAppendSurfaceRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        content_x: f32,
        text_width: f32,
        line_number_width: f32,
        reserve_right_border_col: bool,
        reserve_right_special_col: bool,
        char_width: f32,
        tab_width: i32,
        tab_stop_list: &'a [i32],
    ) -> Self {
        Self {
            content_x,
            text_width,
            line_number_width,
            reserve_right_border_col,
            reserve_right_special_col,
            char_width,
            tab_width,
            tab_stop_list,
        }
    }

    fn reserved_width(self) -> f32 {
        let right_border = if self.reserve_right_border_col {
            self.char_width
        } else {
            0.0
        };
        let right_special = if self.reserve_right_special_col {
            self.char_width
        } else {
            0.0
        };
        right_border + right_special
    }

    fn append_width(self) -> f32 {
        (self.text_width - self.line_number_width - self.reserved_width()).max(self.char_width)
    }

    pub(crate) fn into_surface(self) -> DisplayRowAppendSurface {
        DisplayRowAppendSurface::new(
            DisplayRowAppendArea::new(
                self.content_x,
                self.append_width(),
                self.text_width,
                self.line_number_width,
            ),
            DisplayTabPolicy::from_tab_width_and_stops(
                self.content_x,
                self.tab_width,
                self.tab_stop_list,
            ),
        )
    }
}

pub(crate) struct BufferTextWindowCursorEffectsRequest {
    window_id: i64,
    effects: Option<EffectsConfig>,
}

impl BufferTextWindowCursorEffectsRequest {
    pub(crate) fn new(window_id: i64, effects: Option<EffectsConfig>) -> Self {
        Self { window_id, effects }
    }

    pub(crate) fn install_and_apply(self, builder: &mut GlyphMatrixBuilder) -> bool {
        let Some(effects) = self.effects else {
            return false;
        };
        install_text_window_cursor_effects(
            builder,
            TextWindowCursorEffects {
                window_id: self.window_id,
                effects,
            },
        );
        true
    }
}

pub(crate) struct BufferTextWindowTerminalRightBorderRequest {
    ch: char,
    face_name: &'static str,
    char_width: f32,
}

impl BufferTextWindowTerminalRightBorderRequest {
    pub(crate) fn new(char_width: f32) -> Self {
        Self {
            ch: '|',
            face_name: "vertical-border",
            char_width,
        }
    }

    pub(crate) fn install_and_apply(
        self,
        builder: &mut GlyphMatrixBuilder,
        mut render_services: ChromeRowRenderServices<'_, '_>,
    ) -> u32 {
        let border_face = render_services
            .face_resolver()
            .resolve_named_face(self.face_name);
        // GNU draws every realized face id from the single per-frame face cache
        // counter (`face_cache->used`, xfaces.c `lookup_face`). Allocate the
        // border's id from the frame-scoped allocator (reconciled into
        // `frame_face_id_counter` by the decoration render, engine.rs) rather than
        // a separate `FaceResolver` counter that could collide with it.
        let border_face_id = render_services.face_ids().allocate();
        insert_resolved_display_row_face(builder, border_face_id, &border_face, None);
        install_last_window_right_border_from_source_requests(
            builder,
            render_services.reborrow(),
            TextWindowRightBorder {
                ch: self.ch,
                face_id: border_face_id,
                char_width: self.char_width,
            },
            &border_face,
        );
        border_face_id
    }
}

pub(crate) struct BufferTextWindowBeginRequest {
    frame_id: FrameId,
    window_id: WindowId,
    text_matrix_row_base: usize,
    text_area_left: f32,
    window_top: f32,
    matrix_window_id: u64,
    matrix_rows: usize,
    matrix_cols: usize,
    bounds: neomacs_display_protocol::types::Rect,
    text_bounds: neomacs_display_protocol::types::Rect,
    selected: bool,
    first_row: TextMatrixRowBegin,
}

pub(crate) struct BufferTextWindowBeginState<'a> {
    pub(crate) builder: &'a mut GlyphMatrixBuilder,
    pub(crate) evaluator: &'a mut Context,
}

pub(crate) struct BufferTextWindowTailFinalizeRequest<'a> {
    context: BufferTextWindowTailFinalizeContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferTextWindowTailFinalizeContext<'a> {
    pub(crate) params: &'a WindowParams,
    pub(crate) text: &'a [u8],
    pub(crate) text_matrix_row_base: usize,
    pub(crate) text_area_left: f32,
    pub(crate) window_top: f32,
    pub(crate) text_y: f32,
    pub(crate) text_height: f32,
    pub(crate) char_w: f32,
    pub(crate) char_h: f32,
    pub(crate) window_start: i64,
    pub(crate) point_charpos: i64,
    pub(crate) charpos: i64,
    pub(crate) point_is_visible_eob: bool,
    pub(crate) row_limit: DisplayRowLimit,
}

pub(crate) struct BufferTextWindowTailFinalizeState<'a, 'emit> {
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) row_geometry: &'a DisplayRowGeometryState,
    pub(crate) row_y_positions: &'a DisplayRowYPositions,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) output_render: TextRowOutputRenderState<'emit>,
}

pub(crate) struct BufferTextWindowBodyInstallRequest<'a> {
    context: BufferTextWindowBodyInstallRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferTextWindowBodyInstallRenderContext<'a> {
    pub(crate) window_id: u64,
    pub(crate) window_start: i64,
    pub(crate) text_start_byte: usize,
    pub(crate) byte_idx: usize,
    pub(crate) reserve_right_special_col: bool,
    pub(crate) reserve_right_border_col: bool,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) matrix_cols: usize,
    pub(crate) row_flags: &'a DisplayRowFlags,
    pub(crate) right_edge_face_id: u32,
    pub(crate) char_w: f32,
}

pub(crate) struct BufferTextWindowBodyInstallState<'a, 'emit, 'face> {
    pub(crate) builder: &'emit mut GlyphMatrixBuilder,
    pub(crate) output_emitter: &'a WindowOutputEmitter,
    pub(crate) render_services: ChromeRowRenderServices<'emit, 'face>,
}

pub(crate) struct BufferTextWindowVisibilityRetryRequest<'a, 'buf, B: LayoutBufferView> {
    rows: &'a [DisplayRowSnapshot],
    window_start: i64,
    accessible_start: i64,
    accessible_end: i64,
    point_charpos: i64,
    charpos: i64,
    point_is_visible_eob: bool,
    text_area_top: i64,
    text_area_bottom: i64,
    buf_access: &'a RustBufferAccess<'buf, B>,
}

pub(crate) struct BufferTextWindowFinishRequest {
    window_id: i64,
    content_x: f32,
    char_w: f32,
    text_area_left_offset: i64,
    mode_line_height: i64,
    header_line_height: i64,
    tab_line_height: i64,
}

pub(crate) struct BufferTextWindowFinishState<'a> {
    pub(crate) builder: &'a mut GlyphMatrixBuilder,
    pub(crate) output_emitter: WindowOutputEmitter,
    pub(crate) evaluator: &'a mut Context,
    pub(crate) hit_rows: Vec<HitRow>,
}

pub(crate) struct BufferTextWindowFinishOutput {
    pub(crate) hit_data: WindowHitData,
    pub(crate) snapshot: WindowDisplaySnapshot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowVisibilityRetryOutcome {
    visible_end_lisp: Option<LispCharPos1>,
    visible_progress: i64,
    point_beyond_visible_span: bool,
    scroll_down_window_start: Option<i64>,
    point_row_window_start: Option<i64>,
    point_line_window_start: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowTailFinalizeOutcome {
    cursor_requested: bool,
    cursor_publish_status: BufferTextWindowCursorPublishStatus,
    visual_cursor_summary: VisualTextWindowCursorPublishSummary,
    pending_row_finished: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextWindowCursorPublishStatus {
    NotRequested,
    MissingCapture,
    NoWindowCursor,
    Clipped,
    Published,
}

impl From<CapturedTextWindowCursorPublishOutcome> for BufferTextWindowCursorPublishStatus {
    fn from(outcome: CapturedTextWindowCursorPublishOutcome) -> Self {
        match outcome {
            CapturedTextWindowCursorPublishOutcome::NoWindowCursor => Self::NoWindowCursor,
            CapturedTextWindowCursorPublishOutcome::Clipped => Self::Clipped,
            CapturedTextWindowCursorPublishOutcome::Published => Self::Published,
        }
    }
}

impl BufferTextWindowTailFinalizeOutcome {
    #[cfg(test)]
    pub(crate) fn cursor_requested(self) -> bool {
        self.cursor_requested
    }

    #[cfg(test)]
    pub(crate) fn cursor_published(self) -> bool {
        matches!(
            self.cursor_publish_status,
            BufferTextWindowCursorPublishStatus::Published
        )
    }

    #[cfg(test)]
    pub(crate) fn cursor_publish_status(self) -> BufferTextWindowCursorPublishStatus {
        self.cursor_publish_status
    }

    #[cfg(test)]
    pub(crate) fn visual_cursor_summary(self) -> VisualTextWindowCursorPublishSummary {
        self.visual_cursor_summary
    }

    #[cfg(test)]
    pub(crate) fn pending_row_finished(self) -> bool {
        self.pending_row_finished
    }
}

impl BufferTextWindowBeginRequest {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        frame_id: FrameId,
        window_id: WindowId,
        text_matrix_row_base: usize,
        text_area_left: f32,
        window_top: f32,
        matrix_window_id: u64,
        matrix_rows: usize,
        matrix_cols: usize,
        bounds: neomacs_display_protocol::types::Rect,
        text_bounds: neomacs_display_protocol::types::Rect,
        selected: bool,
        first_row: TextMatrixRowBegin,
    ) -> Self {
        Self {
            frame_id,
            window_id,
            text_matrix_row_base,
            text_area_left,
            window_top,
            matrix_window_id,
            matrix_rows,
            matrix_cols,
            bounds,
            text_bounds,
            selected,
            first_row,
        }
    }

    pub(crate) fn begin_and_apply(
        self,
        state: BufferTextWindowBeginState<'_>,
    ) -> WindowOutputEmitter {
        let mut output_emitter = WindowOutputEmitter::new(
            self.frame_id,
            self.window_id,
            self.text_matrix_row_base,
            self.text_area_left,
            self.window_top,
        );
        output_emitter.begin_update(state.evaluator);
        TextWindowRowLifecycleInstaller::new(state.builder, &mut output_emitter, state.evaluator)
            .begin_text_window_output(TextWindowBegin {
                window_id: self.matrix_window_id,
                rows: self.matrix_rows,
                cols: self.matrix_cols,
                bounds: self.bounds,
                text_bounds: self.text_bounds,
                selected: self.selected,
                first_row: self.first_row,
            });
        output_emitter
    }

    pub(crate) fn frame_id(&self) -> FrameId {
        self.frame_id
    }

    pub(crate) fn window_id(&self) -> WindowId {
        self.window_id
    }
}

impl<'a> BufferTextWindowTailFinalizeRequest<'a> {
    pub(crate) fn new(context: BufferTextWindowTailFinalizeContext<'a>) -> Self {
        Self { context }
    }

    pub(crate) fn finalize_and_apply(
        self,
        state: BufferTextWindowTailFinalizeState<'_, '_>,
    ) -> BufferTextWindowTailFinalizeOutcome {
        let BufferTextWindowTailFinalizeState {
            cursor_info,
            row_geometry,
            row_y_positions,
            hit_row_range,
            hit_rows,
            output_render,
        } = state;
        let (builder, output_emitter, evaluator) = output_render.into_parts();
        let context = self.context;

        let cursor_requested = context.point_charpos >= context.window_start
            && (context.point_charpos <= context.charpos || context.point_is_visible_eob);
        let mut cursor_publish_status = if cursor_requested {
            BufferTextWindowCursorPublishStatus::MissingCapture
        } else {
            BufferTextWindowCursorPublishStatus::NotRequested
        };

        if cursor_requested {
            if let Some(cursor) = cursor_info.captured() {
                let cursor_row_metrics = output_emitter.row_metrics().to_vec();
                cursor_publish_status = CapturedTextWindowCursorPublishContext::new(
                    context.params,
                    context.text,
                    context.text_matrix_row_base,
                    context.text_area_left,
                    context.window_top,
                    context.text_y,
                    context.text_height,
                    context.char_w,
                    context.char_h,
                    context.point_charpos,
                    context.point_is_visible_eob,
                )
                .publish_captured_cursor(
                    cursor,
                    &cursor_row_metrics,
                    row_geometry.row_metrics_snapshot(context.text_matrix_row_base),
                    builder,
                    output_emitter,
                )
                .into();
            } else {
                tracing::debug!(
                    "layout_window_rust: no explicit cursor capture for point={} window_start={} charpos_end={}",
                    context.point_charpos,
                    context.window_start,
                    context.charpos
                );
            }
        }

        let pending_row_finished =
            TextWindowRowLifecycleInstaller::new(builder, output_emitter, evaluator)
                .finish_pending_row(TextWindowPendingRowFinish {
                    row_geometry,
                    row_limit: context.row_limit,
                    row_y_positions,
                    text_y: context.text_y,
                    char_height: context.char_h,
                    charpos: context.charpos,
                    hit_row_range,
                    hit_rows,
                });

        let visual_cursor_summary = VisualTextWindowCursorPublishContext::new(
            context.params,
            context.text_area_left,
            context.window_top,
            context.text_y,
            context.text_height,
            context.char_w,
        )
        .publish_visual_cursors(builder, output_emitter);

        BufferTextWindowTailFinalizeOutcome {
            cursor_requested,
            cursor_publish_status,
            visual_cursor_summary,
            pending_row_finished,
        }
    }
}

impl<'a> BufferTextWindowBodyInstallRequest<'a> {
    pub(crate) fn new(context: BufferTextWindowBodyInstallRenderContext<'a>) -> Self {
        Self { context }
    }

    pub(crate) fn install_and_apply(
        self,
        state: BufferTextWindowBodyInstallState<'_, '_, '_>,
    ) -> TextWindowRedisplayPositions {
        let context = self.context;
        let right_edge_markers = TextWindowRightEdgeMarkers::for_reserved_special_column(
            context.reserve_right_special_col,
            context.reserve_right_border_col,
            context.text_matrix_row_base,
            context.matrix_cols,
            context.row_flags,
            context.right_edge_face_id,
            context.char_w,
        );

        let redisplay_positions =
            TextWindowOutputInstaller::new(state.builder, state.output_emitter)
                .install_body_output(TextWindowBodyOutputInstall {
                    window_id: context.window_id,
                    window_start: context.window_start,
                    text_start_byte: context.text_start_byte,
                    byte_idx: context.byte_idx,
                });
        if let Some(markers) = right_edge_markers {
            install_right_edge_markers_from_source_requests(
                state.builder,
                state.render_services,
                markers,
            );
        }
        redisplay_positions
    }
}

impl<'a, 'buf, B: LayoutBufferView> BufferTextWindowVisibilityRetryRequest<'a, 'buf, B> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        rows: &'a [DisplayRowSnapshot],
        window_start: i64,
        accessible_start: i64,
        accessible_end: i64,
        point_charpos: i64,
        charpos: i64,
        point_is_visible_eob: bool,
        text_area_top: i64,
        text_area_bottom: i64,
        buf_access: &'a RustBufferAccess<'buf, B>,
    ) -> Self {
        Self {
            rows,
            window_start,
            accessible_start,
            accessible_end,
            point_charpos,
            charpos,
            point_is_visible_eob,
            text_area_top,
            text_area_bottom,
            buf_access,
        }
    }

    pub(crate) fn decide(self) -> BufferTextWindowVisibilityRetryOutcome {
        let point_lisp = layout_i64_char_pos_to_lisp_char_pos(self.point_charpos);
        let visible_end_lisp = self.rows.iter().rev().find_map(|row| row.end_buffer_pos);
        let visible_end_lisp = if self.point_is_visible_eob {
            Some(visible_end_lisp.unwrap_or(point_lisp).max(point_lisp))
        } else {
            visible_end_lisp
        };
        let visible_progress = visible_end_lisp
            .map(LispCharPos1::as_i64)
            .unwrap_or(self.charpos);
        let point_beyond_visible_span = visible_end_lisp
            .map(|end_lisp| point_lisp > end_lisp)
            .unwrap_or(self.point_charpos > self.charpos);

        // GNU `resize_mini_window` (src/xdisp.c:13361) scrolls a mini-window
        // whose measured content exceeds `max-mini-window-height` so the END
        // (where point sits in an active fido/vertico minibuffer) shows.  This
        // is the same point-driven scroll an ordinary window uses, so the
        // minibuffer is no longer excluded.  An inactive echo-area mini-window
        // has point reset to BEGV, so `point_beyond_visible_span` is false and
        // it never scrolls.
        let scroll_down_window_start =
            if point_beyond_visible_span && visible_progress > self.window_start {
                next_window_start_from_visible_rows(self.rows, self.window_start)
                    .map(|new_ws| new_ws.min(self.point_charpos.max(self.accessible_start)))
            } else {
                None
            };
        let point_row_window_start = next_window_start_for_partially_visible_point_row(
            self.rows,
            self.point_charpos,
            self.text_area_top,
            self.text_area_bottom,
            self.window_start,
        );
        let point_line_window_start = next_window_start_for_point_line_continuation(
            self.rows,
            self.point_charpos,
            self.window_start,
            self.buf_access,
            self.accessible_end,
        );

        BufferTextWindowVisibilityRetryOutcome {
            visible_end_lisp,
            visible_progress,
            point_beyond_visible_span,
            scroll_down_window_start,
            point_row_window_start,
            point_line_window_start,
        }
    }
}

impl BufferTextWindowFinishRequest {
    pub(crate) fn new(
        window_id: i64,
        content_x: f32,
        char_w: f32,
        text_area_left_offset: i64,
        mode_line_height: i64,
        header_line_height: i64,
        tab_line_height: i64,
    ) -> Self {
        Self {
            window_id,
            content_x,
            char_w,
            text_area_left_offset,
            mode_line_height,
            header_line_height,
            tab_line_height,
        }
    }

    pub(crate) fn finish_and_snapshot(
        self,
        state: BufferTextWindowFinishState<'_>,
    ) -> BufferTextWindowFinishOutput {
        close_text_window_output(state.builder);
        let hit_data = WindowHitData {
            window_id: self.window_id,
            content_x: self.content_x,
            char_w: self.char_w,
            rows: state.hit_rows,
        };
        let snapshot = state.output_emitter.finish_snapshot(
            state.evaluator,
            self.text_area_left_offset,
            self.mode_line_height,
            self.header_line_height,
            self.tab_line_height,
        );

        BufferTextWindowFinishOutput { hit_data, snapshot }
    }
}

impl BufferTextWindowVisibilityRetryOutcome {
    pub(crate) fn visible_end_lisp(self) -> Option<LispCharPos1> {
        self.visible_end_lisp
    }

    #[cfg(test)]
    pub(crate) fn point_beyond_visible_span(self) -> bool {
        self.point_beyond_visible_span
    }

    pub(crate) fn scroll_down_window_start(self) -> Option<i64> {
        self.scroll_down_window_start
    }

    pub(crate) fn point_row_window_start(self) -> Option<i64> {
        self.point_row_window_start
    }

    pub(crate) fn point_line_window_start(self) -> Option<i64> {
        self.point_line_window_start
    }

    pub(crate) fn retry_window_start(self) -> Option<i64> {
        self.scroll_down_window_start
            .or(self.point_row_window_start)
            .or(self.point_line_window_start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_window_append_surface_request_reserves_right_columns() {
        let tab_stops = vec![4, 12];
        let surface =
            TextWindowAppendSurfaceRequest::new(20.0, 200.0, 16.0, true, true, 8.0, 6, &tab_stops)
                .into_surface();

        assert_eq!(surface.content_x(), 20.0);
        assert_eq!(surface.right_edge(), 188.0);
        assert_eq!(surface.full_text_right_edge(), 204.0);
    }
}
