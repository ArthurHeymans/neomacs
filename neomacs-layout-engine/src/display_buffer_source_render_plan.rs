//! Buffer source render plan construction and completion.

use crate::display_buffer_source_body_render::BufferSourceWalkSetup;
use crate::display_buffer_source_face_resolution::*;
use crate::display_buffer_source_loop_context::BufferSourceLoopRequestContext;
use crate::display_buffer_source_render_attempt::{
    BufferSourceRedisplayPublishRequest, BufferSourceRenderAttemptContext,
    BufferSourceRenderAttemptOutcome, BufferSourceRetryPlan,
};
use crate::display_buffer_source_tail_render::{
    BufferSourceBodyInstallContext, BufferSourceRetryBounds, BufferSourceTailRequestContext,
};
use crate::display_buffer_window_geometry::{
    BufferWindowChromeHeights, BufferWindowGeometry, BufferWindowLocalDisplayPolicy,
};
use crate::display_buffer_window_source::BufferWindowSource;
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_row::{
    DisplayRowActiveFaceState, DisplayRowFallbackMetrics, DisplayRowMeasurementPolicy,
};
use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_geometry::{DisplayRowLimit, DisplayRowVisibilityLimit};
use crate::display_row_overlay_string::BufferOverlayStringTextRowRenderContext;
use crate::display_row_walk_state::FaceScanCheckpoint;
use crate::display_status_line::{ChromeRowRenderServices, WindowChromeRowsRenderRequest};
use crate::display_text_window_row_lifecycle::{TextWindowBeginRequest, TextWindowFinishState};
use crate::font_metrics::FontMetricsService;
use crate::neovm_bridge::{FaceResolver, LayoutBufferView, ResolvedFace, RustBufferAccess};
use crate::types::WindowParams;
use crate::window_output::render_window_chrome_rows;
use neomacs_display_protocol::types::Rect;
use neovm_core::buffer::BufferId;
use neovm_core::window::{FrameId, WindowId};

pub(crate) struct BufferSourceOutputSetup {
    begin_request: TextWindowBeginRequest,
    row_visibility_limit: DisplayRowVisibilityLimit,
    row_limit: DisplayRowLimit,
    body_install_context: BufferSourceBodyInstallContext,
    retry_bounds: BufferSourceRetryBounds,
}

pub(crate) struct BufferSourceDefaultFacePlan {
    face: ResolvedFace,
    metrics: DisplayRowFallbackMetrics,
    measurement_policy: DisplayRowMeasurementPolicy,
}
impl BufferSourceOutputSetup {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        frame_id: FrameId,
        window_id: WindowId,
        output_window_id: u64,
        display_text_row_base: usize,
        display_text_rows: usize,
        bottom_chrome_rows: usize,
        cols: usize,
        bounds: Rect,
        text_bounds: Rect,
        selected: bool,
        text_y: f32,
        text_height: f32,
        visibility_bottom_y: f32,
        max_rows: usize,
        walk_setup: &BufferSourceWalkSetup,
    ) -> Self {
        Self::from_parts(
            frame_id,
            window_id,
            output_window_id,
            display_text_row_base,
            display_text_rows,
            bottom_chrome_rows,
            cols,
            bounds,
            text_bounds,
            selected,
            text_y,
            text_height,
            visibility_bottom_y,
            max_rows,
            walk_setup,
        )
    }

    pub(crate) fn from_window_geometry(
        frame_id: FrameId,
        window_id: WindowId,
        params: &WindowParams,
        geometry: &BufferWindowGeometry,
        max_rows: usize,
        walk_setup: &BufferSourceWalkSetup,
    ) -> Self {
        Self::new(
            frame_id,
            window_id,
            params.window_id as u64,
            geometry.display_text_row_base,
            geometry.display_text_rows,
            geometry.bottom_chrome_rows,
            geometry.cols,
            params.bounds,
            params.text_bounds,
            params.selected,
            geometry.text_y,
            geometry.text_height,
            geometry.visibility_bottom_y,
            max_rows,
            walk_setup,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_parts(
        frame_id: FrameId,
        window_id: WindowId,
        output_window_id: u64,
        display_text_row_base: usize,
        display_text_rows: usize,
        bottom_chrome_rows: usize,
        cols: usize,
        bounds: Rect,
        text_bounds: Rect,
        selected: bool,
        text_y: f32,
        text_height: f32,
        visibility_bottom_y: f32,
        max_rows: usize,
        walk_setup: &BufferSourceWalkSetup,
    ) -> BufferSourceOutputSetup {
        let output_cols = cols.max(1);
        BufferSourceOutputSetup {
            begin_request: TextWindowBeginRequest::new(
                frame_id,
                window_id,
                display_text_row_base,
                walk_setup.text_area_left,
                walk_setup.window_top,
                output_window_id,
                display_text_row_base + display_text_rows + bottom_chrome_rows,
                output_cols,
                bounds,
                text_bounds,
                selected,
                walk_setup.row_geometry.display_text_row_begin(
                    display_text_row_base,
                    walk_setup.col,
                    walk_setup.x,
                ),
            ),
            row_visibility_limit: DisplayRowVisibilityLimit {
                max_rows,
                // Lifted to span `max_rows` for a minibuffer so the unclamped
                // GNU `resize_mini_window` measurement can emit content rows
                // beyond the window's current physical height (see
                // `BufferWindowGeometry::visibility_bottom_y`).
                bottom_y: visibility_bottom_y,
            },
            row_limit: DisplayRowLimit { max_rows },
            body_install_context: BufferSourceBodyInstallContext::new(
                output_window_id,
                display_text_row_base,
                output_cols,
            ),
            retry_bounds: BufferSourceRetryBounds::new(
                (text_y - walk_setup.window_top).round() as i64,
                (text_y + text_height - walk_setup.window_top).round() as i64,
            ),
        }
    }
}

impl BufferSourceDefaultFacePlan {
    pub(crate) fn new(
        face_resolver: &FaceResolver,
        font_metrics: &mut Option<FontMetricsService>,
        window_system: bool,
        fallback_metrics: DisplayRowFallbackMetrics,
    ) -> Self {
        let face = face_resolver.default_face().clone();
        let metrics = if window_system && let Some(service) = font_metrics {
            let metrics = service.font_metrics(
                &face.font_family,
                face.font_weight,
                face.italic,
                face.font_size,
            );
            DisplayRowFallbackMetrics::from_font_metrics(metrics)
        } else {
            fallback_metrics
        };

        Self {
            face,
            metrics,
            measurement_policy: DisplayRowMeasurementPolicy::for_frame(window_system),
        }
    }

    pub(crate) fn face(&self) -> &ResolvedFace {
        &self.face
    }

    pub(crate) fn char_width(&self) -> f32 {
        self.metrics.char_width()
    }

    pub(crate) fn row_height(&self) -> f32 {
        self.metrics.row_height()
    }

    pub(crate) fn ascent(&self) -> f32 {
        self.metrics.ascent()
    }

    pub(crate) fn metrics(&self) -> DisplayRowFallbackMetrics {
        self.metrics
    }

    pub(crate) fn row_metrics_for_extents(
        &self,
        char_width: f32,
        row_height: f32,
    ) -> DisplayRowFallbackMetrics {
        self.metrics.with_extents(char_width, row_height)
    }

    pub(crate) fn row_metrics_for_default_width(
        &self,
        row_height: f32,
    ) -> DisplayRowFallbackMetrics {
        self.row_metrics_for_extents(self.metrics.char_width(), row_height)
    }

    pub(crate) fn measurement_policy(&self) -> DisplayRowMeasurementPolicy {
        self.measurement_policy
    }
}

impl BufferSourceOutputSetup {
    #[cfg(test)]
    pub(crate) fn row_visibility_limit(&self) -> DisplayRowVisibilityLimit {
        self.row_visibility_limit
    }

    #[cfg(test)]
    pub(crate) fn row_limit(&self) -> DisplayRowLimit {
        self.row_limit
    }

    #[cfg(test)]
    pub(crate) fn body_install_context(&self) -> BufferSourceBodyInstallContext {
        self.body_install_context
    }

    #[cfg(test)]
    pub(crate) fn retry_bounds(&self) -> BufferSourceRetryBounds {
        self.retry_bounds
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_body_attempt<'a, 'surface, 'buf, B>(
        self,
        walk_setup: &mut BufferSourceWalkSetup,
        state: BufferSourceRenderAttemptContext<'_, '_>,
        chrome_request: WindowChromeRowsRenderRequest<'_, '_>,
        remaining_visibility_retries: usize,
        local_display_policy: BufferWindowLocalDisplayPolicy,
        line_number_cols: i32,
        geometry: &BufferWindowGeometry,
        chrome_heights: BufferWindowChromeHeights,
        buffer: &'a B,
        buffer_id: BufferId,
        source: BufferWindowSource,
        params: &'a WindowParams,
        default_face: &'a BufferSourceDefaultFacePlan,
        window_metrics: DisplayRowFallbackMetrics,
        window_system: bool,
        output_window_id: u64,
        append_surface: &'surface DisplayRowAppendSurface,
        reserve_right_special_col: bool,
        reserve_right_border_col: bool,
        text: &'a [u8],
        buf_access: &RustBufferAccess<'buf, B>,
    ) -> BufferSourceRenderAttemptOutcome
    where
        B: LayoutBufferView,
    {
        let (
            mut output,
            font_metrics,
            face_resolver,
            frame_face_id_counter,
            hit_data,
            display_snapshots,
        ) = state.into_parts();
        let retry_checkpoint = output.capture_retry_checkpoint();
        let mut face_ids = FrameFaceIdAllocator::new(*frame_face_id_counter);

        let has_overlays = !buffer.layout_overlays().is_empty();
        let face_resolution = BufferSourceFaceResolutionContext::new(
            buffer,
            face_resolver,
            default_face.measurement_policy(),
            default_face.face(),
            default_face.metrics(),
            window_metrics,
            window_system,
        );
        let overlay_text_row = BufferOverlayStringTextRowRenderContext::new(
            has_overlays,
            output_window_id,
            append_surface,
            default_face.row_metrics_for_default_width(geometry.char_height),
            geometry.text_y,
            self.body_install_context.display_text_row_base(),
            geometry.max_rows,
        );
        let row_fallback_metrics =
            default_face.row_metrics_for_extents(geometry.char_width, geometry.char_height);
        let loop_context = BufferSourceLoopRequestContext::new(
            buffer_id,
            source.text_start_byte(),
            source.accessible_end(),
            source.point_charpos(),
            params,
            geometry.content_x,
            local_display_policy.has_prefix(),
            row_fallback_metrics,
            self.row_visibility_limit,
            walk_setup.row_geometry_defaults,
            self.body_install_context.display_text_row_base(),
            geometry.max_rows,
            self.row_limit,
        );
        let row_prelude_context =
            local_display_policy.row_prelude_context(line_number_cols, row_fallback_metrics);
        let fallback_metrics = default_face.metrics();
        let tail_context = BufferSourceTailRequestContext::new(
            params,
            source.window_start(),
            source.accessible_start(),
            source.accessible_end(),
            source.text_start_byte(),
            self.body_install_context.display_text_row_base(),
            walk_setup.text_area_left,
            walk_setup.window_top,
            geometry.text_y,
            geometry.text_height,
            geometry.content_x,
            geometry.char_width,
            geometry.char_height,
            self.row_limit,
            self.retry_bounds,
            self.body_install_context,
            reserve_right_special_col,
            reserve_right_border_col,
            chrome_heights.mode_line,
            chrome_heights.header_line,
            chrome_heights.tab_line,
        );
        let publish_request = BufferSourceRedisplayPublishRequest::new(
            self.begin_request.frame_id(),
            self.begin_request.window_id(),
            source.accessible_end_lisp_char(),
            source.accessible_end_emacs_byte(),
        );

        let mut line_numbers = local_display_policy.initial_line_numbers(
            buf_access,
            tail_context.window_start,
            loop_context.point_charpos(),
        );
        let mut face_scan = FaceScanCheckpoint::initial();
        let default_measured_face = default_face.measurement_policy().measured_face(
            neomacs_display_protocol::face::BasicFaceId::Default.into(),
            default_face.face(),
            None,
            default_face.char_width(),
            fallback_metrics,
            font_metrics,
        );
        let mut active_face_state =
            DisplayRowActiveFaceState::new(default_face.face().clone(), default_measured_face);
        let (output_emitter, post_loop) = walk_setup.begin_render_body_and_tail(
            self.begin_request,
            &mut output,
            font_metrics,
            face_resolver,
            &mut face_ids,
            &mut line_numbers,
            &mut face_scan,
            &mut active_face_state,
            row_prelude_context,
            loop_context,
            face_resolution,
            &tail_context,
            text,
            params,
            overlay_text_row,
            buffer,
            buf_access,
        );

        let retry_plan = BufferSourceRetryPlan::from_post_loop(
            tail_context.params.window_id,
            tail_context.window_start,
            tail_context.params.point_charpos().get(),
            walk_setup.charpos,
            self.retry_bounds,
            post_loop,
        );
        retry_plan.log_visibility_adjustments();

        if let Some(window_start) = retry_plan.should_retry(remaining_visibility_retries) {
            retry_plan.log_retry(window_start, remaining_visibility_retries);
            output.restore_retry_checkpoint(retry_checkpoint);
            *frame_face_id_counter = face_ids.finish();
            return BufferSourceRenderAttemptOutcome::Retry { window_start };
        }

        let (mut output, evaluator) = output.into_parts();
        let mut render_services =
            ChromeRowRenderServices::new(font_metrics, face_resolver, &mut face_ids);
        let mut output_emitter = output_emitter;
        let redisplay_positions = walk_setup.install_body_and_publish_redisplay(
            output.reborrow(),
            &mut output_emitter,
            evaluator,
            render_services.reborrow(),
            &tail_context,
            publish_request,
        );
        render_window_chrome_rows(
            output.reborrow(),
            &mut output_emitter,
            evaluator,
            chrome_request,
            render_services.reborrow(),
        );
        tail_context.finish_and_install(
            TextWindowFinishState::new(
                output,
                output_emitter,
                evaluator,
                std::mem::take(&mut walk_setup.hit_rows),
            ),
            hit_data,
            display_snapshots,
        );
        drop(render_services);
        *frame_face_id_counter = face_ids.finish();
        BufferSourceRenderAttemptOutcome::Finished {
            redisplay_positions,
        }
    }
}
