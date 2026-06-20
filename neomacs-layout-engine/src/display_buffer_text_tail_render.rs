//! Buffer text post-loop tail rendering and install context.

use crate::display_buffer_text_append::{
    BufferTextWindowBodyInstallRenderContext, BufferTextWindowBodyInstallRequest,
    BufferTextWindowFinishRequest, BufferTextWindowFinishState,
    BufferTextWindowTailFinalizeContext, BufferTextWindowTailFinalizeRequest,
    BufferTextWindowTailFinalizeState, BufferTextWindowVisibilityRetryOutcome,
    BufferTextWindowVisibilityRetryRequest,
};
use crate::display_buffer_text_loop_context::BufferTextWindowLoopRequestContext;
use crate::display_buffer_text_progress::BufferTextWindowRowProgressState;
use crate::display_cursor::CursorCaptureState;
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_row::DisplayRowActiveFaceState;
use crate::display_row_geometry::{
    DisplayRowFlags, DisplayRowGeometryState, DisplayRowLimit, DisplayRowYPositions,
};
use crate::display_row_overlay_string::BufferOverlayStringTextRowRenderContext;
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_row_walk_state::HitRowRangeTracker;
use crate::hit_test::{HitRow, WindowHitData};
use crate::neovm_bridge::{LayoutBufferView, RustBufferAccess};
use crate::types::WindowParams;
use neovm_core::window::{DisplayRowSnapshot, WindowDisplaySnapshot};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowBodyInstallContext {
    output_window_id: u64,
    display_text_row_base: usize,
    output_cols: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowRetryBounds {
    text_area_top: i64,
    text_area_bottom: i64,
}

pub(crate) struct BufferTextWindowTailRequestContext<'a> {
    pub(crate) params: &'a WindowParams,
    pub(crate) window_start: i64,
    accessible_start: i64,
    accessible_end: i64,
    text_start_byte: usize,
    display_text_row_base: usize,
    text_area_left: f32,
    window_top: f32,
    text_y: f32,
    text_height: f32,
    content_x: f32,
    char_width: f32,
    char_height: f32,
    row_limit: DisplayRowLimit,
    retry_bounds: BufferTextWindowRetryBounds,
    body_install_context: BufferTextWindowBodyInstallContext,
    reserve_right_special_col: bool,
    reserve_right_border_col: bool,
    mode_line_height: f32,
    header_line_height: f32,
    tab_line_height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowPostLoopRenderOutcome {
    pub(crate) retry: BufferTextWindowVisibilityRetryOutcome,
    pub(crate) rendered_rows_len: usize,
}

impl BufferTextWindowBodyInstallContext {
    pub(crate) fn new(
        output_window_id: u64,
        display_text_row_base: usize,
        output_cols: usize,
    ) -> Self {
        Self {
            output_window_id,
            display_text_row_base,
            output_cols,
        }
    }

    pub(crate) fn display_text_row_base(self) -> usize {
        self.display_text_row_base
    }

    pub(crate) fn request(
        self,
        window_start: i64,
        text_start_byte: usize,
        byte_idx: usize,
        reserve_right_special_col: bool,
        reserve_right_border_col: bool,
        row_flags: &DisplayRowFlags,
        char_width: f32,
    ) -> BufferTextWindowBodyInstallRequest<'_> {
        BufferTextWindowBodyInstallRequest::new(BufferTextWindowBodyInstallRenderContext::new(
            self.output_window_id,
            window_start,
            text_start_byte,
            byte_idx,
            reserve_right_special_col,
            reserve_right_border_col,
            self.display_text_row_base,
            self.output_cols,
            row_flags,
            0,
            char_width,
        ))
    }

    #[cfg(test)]
    pub(crate) fn output_cols(self) -> usize {
        self.output_cols
    }
}

impl BufferTextWindowRetryBounds {
    pub(crate) fn new(text_area_top: i64, text_area_bottom: i64) -> Self {
        Self {
            text_area_top,
            text_area_bottom,
        }
    }

    pub(crate) fn text_area_top(self) -> i64 {
        self.text_area_top
    }

    pub(crate) fn text_area_bottom(self) -> i64 {
        self.text_area_bottom
    }
}

impl<'a> BufferTextWindowTailRequestContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        params: &'a WindowParams,
        window_start: i64,
        accessible_start: i64,
        accessible_end: i64,
        text_start_byte: usize,
        display_text_row_base: usize,
        text_area_left: f32,
        window_top: f32,
        text_y: f32,
        text_height: f32,
        content_x: f32,
        char_width: f32,
        char_height: f32,
        row_limit: DisplayRowLimit,
        retry_bounds: BufferTextWindowRetryBounds,
        body_install_context: BufferTextWindowBodyInstallContext,
        reserve_right_special_col: bool,
        reserve_right_border_col: bool,
        mode_line_height: f32,
        header_line_height: f32,
        tab_line_height: f32,
    ) -> Self {
        Self {
            params,
            window_start,
            accessible_start,
            accessible_end,
            text_start_byte,
            display_text_row_base,
            text_area_left,
            window_top,
            text_y,
            text_height,
            content_x,
            char_width,
            char_height,
            row_limit,
            retry_bounds,
            body_install_context,
            reserve_right_special_col,
            reserve_right_border_col,
            mode_line_height,
            header_line_height,
            tab_line_height,
        }
    }

    pub(crate) fn tail_finalize_request<'request>(
        &'request self,
        text: &'request [u8],
        charpos: i64,
        point_is_visible_eob: bool,
    ) -> BufferTextWindowTailFinalizeRequest<'request> {
        BufferTextWindowTailFinalizeRequest::new(BufferTextWindowTailFinalizeContext::new(
            self.params,
            text,
            self.display_text_row_base,
            self.text_area_left,
            self.window_top,
            self.text_y,
            self.text_height,
            self.char_width,
            self.char_height,
            self.window_start,
            self.params.point_charpos().get(),
            charpos,
            point_is_visible_eob,
            self.row_limit,
        ))
    }

    pub(crate) fn visibility_retry_request<'rows, 'buf, B>(
        &self,
        rows: &'rows [DisplayRowSnapshot],
        charpos: i64,
        point_is_visible_eob: bool,
        buf_access: &'rows RustBufferAccess<'buf, B>,
    ) -> BufferTextWindowVisibilityRetryRequest<'rows, 'buf, B>
    where
        B: LayoutBufferView,
    {
        BufferTextWindowVisibilityRetryRequest::new(
            rows,
            self.window_start,
            self.accessible_start,
            self.accessible_end,
            self.params.point_charpos().get(),
            charpos,
            point_is_visible_eob,
            self.retry_bounds.text_area_top(),
            self.retry_bounds.text_area_bottom(),
            buf_access,
        )
    }

    pub(crate) fn body_install_request<'flags>(
        &self,
        byte_idx: usize,
        row_flags: &'flags DisplayRowFlags,
    ) -> BufferTextWindowBodyInstallRequest<'flags> {
        self.body_install_context.request(
            self.window_start,
            self.text_start_byte,
            byte_idx,
            self.reserve_right_special_col,
            self.reserve_right_border_col,
            row_flags,
            self.char_width,
        )
    }

    pub(crate) fn finish_request(&self) -> BufferTextWindowFinishRequest {
        BufferTextWindowFinishRequest::new(
            self.params.window_id,
            self.content_x,
            self.char_width,
            (self.text_area_left - self.params.bounds.x).round() as i64,
            self.mode_line_height.round() as i64,
            self.header_line_height.round() as i64,
            self.tab_line_height.round() as i64,
        )
    }

    pub(crate) fn finish_and_install(
        &self,
        finish_state: BufferTextWindowFinishState<'_>,
        hit_data: &mut Vec<WindowHitData>,
        display_snapshots: &mut Vec<WindowDisplaySnapshot>,
    ) {
        let finished_window = self.finish_request().finish_and_snapshot(finish_state);
        hit_data.push(finished_window.hit_data);
        display_snapshots.push(finished_window.snapshot);
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_buffer_text_window_tail_and_decide_retry<
    'request,
    'rows,
    'emit,
    'surface,
    'buf,
    B: LayoutBufferView,
>(
    loop_context: BufferTextWindowLoopRequestContext,
    mut source_render: TextRowSourceRenderState<'emit>,
    mut row_progress: BufferTextWindowRowProgressState<'emit>,
    row_geometry: &'emit mut DisplayRowGeometryState,
    cursor_info: &'emit mut CursorCaptureState,
    hit_rows: &'emit mut Vec<HitRow>,
    hit_row_range: &'emit mut HitRowRangeTracker,
    row_y_positions: &'rows mut DisplayRowYPositions,
    face_ids: &'emit mut FrameFaceIdAllocator,
    overlay_context: BufferOverlayStringTextRowRenderContext<'surface>,
    tail_context: &BufferTextWindowTailRequestContext<'_>,
    text: &'request [u8],
    byte_idx: usize,
    charpos: i64,
    active_face_state: &'request DisplayRowActiveFaceState,
    buffer: &B,
    buf_access: &'rows RustBufferAccess<'buf, B>,
) -> BufferTextWindowPostLoopRenderOutcome
where
    'surface: 'request,
{
    let point_is_visible_eob = loop_context
        .end_of_buffer_tail_request(byte_idx, charpos, overlay_context, active_face_state)
        .render_and_apply(
            buffer,
            source_render.reborrow(),
            row_progress.reborrow(),
            row_geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
        )
        .point_is_visible_eob();

    tail_context
        .tail_finalize_request(text, charpos, point_is_visible_eob)
        .finalize_and_apply(BufferTextWindowTailFinalizeState::new(
            cursor_info,
            row_geometry,
            row_y_positions,
            hit_row_range,
            hit_rows,
            source_render.output_render(),
        ));

    // GNU redisplay keeps iterating until point visibility converges or no
    // further progress can be made. Advance by actual rendered row spans
    // from this pass, since wrapped and variable-height lines are exactly
    // where newline-based retry selection goes wrong.
    let retry = tail_context
        .visibility_retry_request(
            source_render.output_rows(),
            charpos,
            point_is_visible_eob,
            buf_access,
        )
        .decide();
    BufferTextWindowPostLoopRenderOutcome {
        retry,
        rendered_rows_len: source_render.output_rows_len(),
    }
}
