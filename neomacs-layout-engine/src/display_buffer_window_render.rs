//! Buffer window source rendering requests and actions.
#[cfg(test)]
pub(crate) use crate::display_buffer_display_property_render::BufferDisplayPropertyTextReplacementOutcome;
use crate::display_buffer_source_body_render::BufferSourceWalkSetupRequest;
pub(crate) use crate::display_buffer_source_render_attempt::{
    BufferSourceRenderAttemptContext, BufferSourceRenderAttemptOutcome,
};
use crate::display_buffer_source_render_plan::{
    BufferSourceDefaultFacePlan, BufferSourceOutputSetup,
};
use crate::display_buffer_text_source::BufferWindowSourceReadRequest;
use crate::display_buffer_window_geometry::{
    BufferWindowChromeHeights, BufferWindowGeometryPlan, BufferWindowGeometryRequest,
    BufferWindowLocalDisplayPolicy,
};
use crate::display_status_line::{
    WindowChromeRowsPlan, max_mini_window_lines, max_mini_window_lines_for_buffer,
};
use crate::neovm_bridge::{LayoutBufferView, RustBufferAccess};
use crate::types::{FrameParams, WindowParams};
use neovm_core::buffer::BufferId;
use neovm_core::window::{FrameId, WindowId};

pub(crate) struct BufferWindowRenderRequest<'a, B>
where
    B: LayoutBufferView,
{
    frame_id: FrameId,
    window_id: WindowId,
    params: &'a WindowParams,
    frame_params: &'a FrameParams,
    buffer_id: BufferId,
    buffer: &'a B,
    buffer_name: &'a str,
    reserve_right_border_col: bool,
}

impl<'a, B> BufferWindowRenderRequest<'a, B>
where
    B: LayoutBufferView,
{
    pub(crate) fn new(
        frame_id: FrameId,
        window_id: WindowId,
        params: &'a WindowParams,
        frame_params: &'a FrameParams,
        buffer_id: BufferId,
        buffer: &'a B,
        buffer_name: &'a str,
        reserve_right_border_col: bool,
    ) -> Self {
        Self {
            frame_id,
            window_id,
            params,
            frame_params,
            buffer_id,
            buffer,
            buffer_name,
            reserve_right_border_col,
        }
    }

    pub(crate) fn render_into(
        self,
        context: BufferSourceRenderAttemptContext<'_, '_>,
        text_buf: &mut Vec<u8>,
        remaining_visibility_retries: usize,
    ) -> BufferSourceRenderAttemptOutcome {
        let Self {
            frame_id,
            window_id,
            params,
            frame_params,
            buffer_id,
            buffer,
            buffer_name,
            reserve_right_border_col,
        } = self;
        let mut state = context;
        let buf_access = RustBufferAccess::new(buffer);
        state.output.install_cursor_effects(params);

        let char_w = params.char_width;
        let char_h = params.char_height;
        let font_ascent = params.font_ascent;
        let local_display_policy = BufferWindowLocalDisplayPolicy::from_buffer(buffer);

        let default_face = BufferSourceDefaultFacePlan::new(
            state.face_resolver,
            &mut *state.font_metrics,
            frame_params.window_system,
            char_w,
            char_h,
            font_ascent,
        );
        let default_resolved = default_face.face();

        tracing::debug!(
            "layout font metrics: family={:?} weight={} italic={} size={} char_w={:.2} char_h={:.2} ascent={:.2} (window char_w={:.2} char_h={:.2})",
            default_resolved.font_family,
            default_resolved.font_weight,
            default_resolved.italic,
            default_resolved.font_size,
            default_face.char_width(),
            default_face.row_height(),
            default_face.ascent(),
            char_w,
            char_h,
        );

        let chrome_plan = WindowChromeRowsPlan::new(
            params,
            state.face_resolver,
            &mut *state.font_metrics,
            char_w,
            default_face.ascent(),
            default_face.row_height(),
        );
        let chrome_heights = BufferWindowChromeHeights::new(
            chrome_plan.mode_line_height(),
            chrome_plan.header_line_height(),
            chrome_plan.tab_line_height(),
        );
        let max_mini_window_rows = {
            let frame_rows = frame_params.height / char_h.max(1.0);
            if params.is_minibuffer() {
                max_mini_window_lines_for_buffer(state.output.evaluator(), buffer, frame_rows)
            } else {
                max_mini_window_lines(state.output.evaluator(), frame_rows)
            }
            .ceil()
            .max(1.0) as usize
        };
        let BufferWindowGeometryPlan {
            geometry,
            line_number_columns,
        } = BufferWindowGeometryRequest::new(
            params,
            char_w,
            char_h,
            chrome_plan.mode_line_height(),
            chrome_plan.header_line_height(),
            chrome_plan.tab_line_height(),
        )
        .with_max_mini_window_rows(max_mini_window_rows)
        .into_window_plan(&local_display_policy, &buf_access);

        let text_source = BufferWindowSourceReadRequest::new(params, geometry.max_rows)
            .read_into(&buf_access, text_buf);
        let bytes_read = text_source.bytes_read();
        let text = if bytes_read > 0 {
            &text_buf[..bytes_read]
        } else {
            &[]
        };
        tracing::debug!(
            "  layout_window_rust id={}: text_y={:.1} text_h={:.1} max_rows={} bytes_read={}",
            params.window_id,
            geometry.text_y,
            geometry.text_height,
            geometry.max_rows,
            bytes_read
        );

        if geometry.text_height <= 0.0 || geometry.text_width <= 0.0 {
            return BufferSourceRenderAttemptOutcome::Skipped;
        }

        let reserve_right_special_col =
            !frame_params.window_system && params.right_fringe_width == 0.0;
        let mut walk_setup = BufferSourceWalkSetupRequest::from_window_geometry(
            text_source,
            params,
            &geometry,
            &local_display_policy,
            &default_face,
            reserve_right_border_col,
            reserve_right_special_col,
        )
        .into_setup();
        let text_append_surface = walk_setup.text_append_surface.clone();
        let output_setup = BufferSourceOutputSetup::from_window_geometry(
            frame_id,
            window_id,
            params,
            &geometry,
            geometry.max_rows,
            &walk_setup,
        );

        output_setup.render_body_attempt(
            &mut walk_setup,
            state,
            chrome_plan.render_request(
                params,
                geometry.mode_line_display_row,
                reserve_right_border_col,
                char_w,
                font_ascent,
                &buffer_name,
            ),
            remaining_visibility_retries,
            local_display_policy,
            line_number_columns,
            &geometry,
            chrome_heights,
            buffer,
            buffer_id,
            text_source,
            params,
            &default_face,
            font_ascent,
            frame_params.window_system,
            params.window_id as u64,
            &text_append_surface,
            reserve_right_special_col,
            reserve_right_border_col,
            text,
            &buf_access,
        )
    }
}
