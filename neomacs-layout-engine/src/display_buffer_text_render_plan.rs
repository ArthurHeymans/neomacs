//! Buffer text render plan construction and completion.

use crate::display_buffer_text_append::{
    BufferTextWindowBeginRequest, BufferTextWindowFinishState,
};
use crate::display_buffer_text_body_render::BufferTextWindowWalkSetup;
use crate::display_buffer_text_face_resolution::*;
use crate::display_buffer_text_loop_context::BufferTextWindowLoopRequestContext;
use crate::display_buffer_text_render_attempt::{
    BufferTextWindowOutputState, BufferTextWindowRedisplayPublishRequest,
    BufferTextWindowRenderAttemptContext, BufferTextWindowRenderAttemptOutcome,
    BufferTextWindowRetryPlan,
};
use crate::display_buffer_text_row_prelude::BufferTextWindowRowPreludeRequestContext;
use crate::display_buffer_text_source::BufferTextWindowSource;
use crate::display_buffer_text_tail_render::{
    BufferTextWindowBodyInstallContext, BufferTextWindowFinishInstallState,
    BufferTextWindowPostLoopRenderOutcome, BufferTextWindowRetryBounds,
    BufferTextWindowTailRequestContext,
};
use crate::display_buffer_text_walk::{
    BufferTextWindowChromeHeights, BufferTextWindowGeometry, BufferTextWindowLocalDisplayPolicy,
};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_row::{
    DisplayRowActiveFaceState, DisplayRowFallbackMetrics, DisplayRowMeasurementPolicy,
};
use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_geometry::{DisplayRowLimit, DisplayRowVisibilityLimit};
use crate::display_row_overlay_string::BufferOverlayStringTextRowRenderContext;
use crate::display_row_walk_state::FaceScanCheckpoint;
use crate::display_status_line::{ChromeRowRenderServices, WindowChromeRowsRenderRequest};
use crate::font_metrics::FontMetricsService;
use crate::hit_test::WindowHitData;
use crate::neovm_bridge::{FaceResolver, LayoutBufferView, ResolvedFace, RustBufferAccess};
use crate::types::WindowParams;
use crate::window_output::{
    TextWindowOutputRetryCheckpoint, TextWindowRedisplayPositions, WindowOutputEmitter,
    render_window_chrome_rows,
};
use neomacs_display_protocol::types::Rect;
use neovm_core::buffer::BufferId;
use neovm_core::window::{FrameId, WindowDisplaySnapshot, WindowId};

pub(crate) struct BufferTextWindowOutputSetup {
    pub(crate) begin_request: BufferTextWindowBeginRequest,
    pub(crate) row_visibility_limit: DisplayRowVisibilityLimit,
    pub(crate) row_limit: DisplayRowLimit,
    pub(crate) body_install_context: BufferTextWindowBodyInstallContext,
    pub(crate) retry_bounds: BufferTextWindowRetryBounds,
}

pub(crate) struct BufferTextWindowDefaultFacePlan {
    face: ResolvedFace,
    char_width: f32,
    row_height: f32,
    ascent: f32,
    measurement_policy: DisplayRowMeasurementPolicy,
}
struct BufferTextWindowRenderedBody<'a> {
    output_emitter: WindowOutputEmitter,
    post_loop: BufferTextWindowPostLoopRenderOutcome,
    retry_bounds: BufferTextWindowRetryBounds,
    publish_request: BufferTextWindowRedisplayPublishRequest,
    tail_context: BufferTextWindowTailRequestContext<'a>,
}

pub(crate) struct BufferTextWindowBodyPlan<'a, 'surface, B>
where
    B: LayoutBufferView,
{
    begin_request: BufferTextWindowBeginRequest,
    retry_bounds: BufferTextWindowRetryBounds,
    publish_request: BufferTextWindowRedisplayPublishRequest,
    local_display_policy: BufferTextWindowLocalDisplayPolicy,
    measurement_policy: DisplayRowMeasurementPolicy,
    default_resolved: &'a ResolvedFace,
    default_face_char_width: f32,
    fallback_metrics: DisplayRowFallbackMetrics,
    row_prelude_context: BufferTextWindowRowPreludeRequestContext,
    loop_context: BufferTextWindowLoopRequestContext,
    face_resolution: BufferCurrentFaceResolutionContext<'a, B>,
    overlay_text_row: BufferOverlayStringTextRowRenderContext<'surface>,
    tail_context: BufferTextWindowTailRequestContext<'a>,
}
impl BufferTextWindowOutputSetup {
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
        walk_setup: &BufferTextWindowWalkSetup,
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
        geometry: &BufferTextWindowGeometry,
        max_rows: usize,
        walk_setup: &BufferTextWindowWalkSetup,
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
        walk_setup: &BufferTextWindowWalkSetup,
    ) -> BufferTextWindowOutputSetup {
        let output_cols = cols.max(1);
        BufferTextWindowOutputSetup {
            begin_request: BufferTextWindowBeginRequest::new(
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
                // `BufferTextWindowGeometry::visibility_bottom_y`).
                bottom_y: visibility_bottom_y,
            },
            row_limit: DisplayRowLimit { max_rows },
            body_install_context: BufferTextWindowBodyInstallContext::new(
                output_window_id,
                display_text_row_base,
                output_cols,
            ),
            retry_bounds: BufferTextWindowRetryBounds::new(
                (text_y - walk_setup.window_top).round() as i64,
                (text_y + text_height - walk_setup.window_top).round() as i64,
            ),
        }
    }
}

impl BufferTextWindowDefaultFacePlan {
    pub(crate) fn new(
        face_resolver: &FaceResolver,
        font_metrics: &mut Option<FontMetricsService>,
        window_system: bool,
        fallback_char_width: f32,
        fallback_row_height: f32,
        fallback_ascent: f32,
    ) -> Self {
        let face = face_resolver.default_face().clone();
        let (char_width, row_height, ascent) = if window_system && let Some(service) = font_metrics
        {
            let metrics = service.font_metrics(
                &face.font_family,
                face.font_weight,
                face.italic,
                face.font_size,
            );
            (metrics.char_width, metrics.line_height, metrics.ascent)
        } else {
            (fallback_char_width, fallback_row_height, fallback_ascent)
        };

        Self {
            face,
            char_width,
            row_height,
            ascent,
            measurement_policy: DisplayRowMeasurementPolicy::for_frame(window_system),
        }
    }

    pub(crate) fn face(&self) -> &ResolvedFace {
        &self.face
    }

    pub(crate) fn char_width(&self) -> f32 {
        self.char_width
    }

    pub(crate) fn row_height(&self) -> f32 {
        self.row_height
    }

    pub(crate) fn ascent(&self) -> f32 {
        self.ascent
    }

    pub(crate) fn measurement_policy(&self) -> DisplayRowMeasurementPolicy {
        self.measurement_policy
    }
}

impl BufferTextWindowOutputSetup {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn into_body_plan<'a, 'surface, B>(
        self,
        walk_setup: &BufferTextWindowWalkSetup,
        local_display_policy: BufferTextWindowLocalDisplayPolicy,
        line_number_cols: i32,
        geometry: &BufferTextWindowGeometry,
        chrome_heights: BufferTextWindowChromeHeights,
        buffer: &'a B,
        buffer_id: BufferId,
        source: BufferTextWindowSource,
        params: &'a WindowParams,
        face_resolver: &'a FaceResolver,
        default_face: &'a BufferTextWindowDefaultFacePlan,
        font_ascent: f32,
        window_system: bool,
        output_window_id: u64,
        append_surface: &'surface DisplayRowAppendSurface,
        reserve_right_special_col: bool,
        reserve_right_border_col: bool,
    ) -> BufferTextWindowBodyPlan<'a, 'surface, B>
    where
        B: LayoutBufferView,
    {
        let has_overlays = !buffer.layout_overlays().is_empty();
        let face_resolution = BufferCurrentFaceResolutionContext::new(
            buffer,
            face_resolver,
            default_face.measurement_policy(),
            default_face.face(),
            default_face.char_width(),
            default_face.ascent(),
            default_face.row_height(),
            geometry.char_width,
            geometry.char_height,
            font_ascent,
            window_system,
        );
        let overlay_text_row = BufferOverlayStringTextRowRenderContext::new(
            has_overlays,
            output_window_id,
            append_surface,
            geometry.char_height,
            default_face.ascent(),
            geometry.text_y,
            self.body_install_context.display_text_row_base(),
            geometry.max_rows,
        );
        let loop_context = BufferTextWindowLoopRequestContext::new(
            buffer_id,
            source.text_start_byte(),
            source.accessible_end(),
            source.point_charpos(),
            params,
            geometry.content_x,
            local_display_policy.has_prefix(),
            default_face.ascent(),
            geometry.char_height,
            geometry.char_width,
            self.row_visibility_limit,
            walk_setup.row_geometry_defaults,
            self.body_install_context.display_text_row_base(),
            geometry.max_rows,
            self.row_limit,
        );
        let row_prelude_context = local_display_policy.row_prelude_context(
            line_number_cols,
            geometry.char_width,
            geometry.char_height,
        );
        let fallback_metrics = DisplayRowFallbackMetrics::from_default_face_extents(
            default_face.char_width(),
            default_face.row_height(),
            default_face.ascent(),
        );
        let tail_context = BufferTextWindowTailRequestContext::new(
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
        let publish_request = BufferTextWindowRedisplayPublishRequest::new(
            self.begin_request.frame_id(),
            self.begin_request.window_id(),
            source.accessible_end_lisp_char(),
            source.accessible_end_emacs_byte(),
        );

        BufferTextWindowBodyPlan {
            begin_request: self.begin_request,
            retry_bounds: self.retry_bounds,
            publish_request,
            local_display_policy,
            measurement_policy: default_face.measurement_policy(),
            default_resolved: default_face.face(),
            default_face_char_width: default_face.char_width(),
            fallback_metrics,
            row_prelude_context,
            loop_context,
            face_resolution,
            overlay_text_row,
            tail_context,
        }
    }
}

impl<'a, 'surface, B> BufferTextWindowBodyPlan<'a, 'surface, B>
where
    B: LayoutBufferView,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_attempt<'buf>(
        self,
        walk_setup: &mut BufferTextWindowWalkSetup,
        state: BufferTextWindowRenderAttemptContext<'_, '_>,
        chrome_request: WindowChromeRowsRenderRequest<'_, '_>,
        remaining_visibility_retries: usize,
        text: &'a [u8],
        params: &'a WindowParams,
        buffer: &B,
        buf_access: &RustBufferAccess<'buf, B>,
    ) -> BufferTextWindowRenderAttemptOutcome {
        let BufferTextWindowRenderAttemptContext {
            mut output,
            font_metrics,
            face_resolver,
            frame_face_id_counter,
            hit_data,
            display_snapshots,
        } = state;
        let retry_checkpoint = output.capture_retry_checkpoint();
        let mut face_ids = FrameFaceIdAllocator::new(*frame_face_id_counter);
        let rendered_body = self.begin_render_body_and_tail(
            walk_setup,
            &mut output,
            font_metrics,
            face_resolver,
            &mut face_ids,
            text,
            params,
            buffer,
            buf_access,
        );
        rendered_body.finish_or_prepare_retry(
            walk_setup,
            chrome_request,
            output,
            font_metrics,
            face_resolver,
            &mut face_ids,
            retry_checkpoint,
            hit_data,
            display_snapshots,
            frame_face_id_counter,
            remaining_visibility_retries,
        )
    }

    fn begin_render_body_and_tail<'buf>(
        self,
        walk_setup: &mut BufferTextWindowWalkSetup,
        output: &mut BufferTextWindowOutputState<'_>,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        text: &'a [u8],
        params: &'a WindowParams,
        buffer: &B,
        buf_access: &RustBufferAccess<'buf, B>,
    ) -> BufferTextWindowRenderedBody<'a> {
        let mut line_numbers = self.local_display_policy.initial_line_numbers(
            buf_access,
            self.tail_context.window_start,
            self.loop_context.point_charpos(),
        );
        let mut face_scan = FaceScanCheckpoint::initial();
        let default_measured_face = self.measurement_policy.measured_face(
            neomacs_display_protocol::face::BasicFaceId::Default.into(),
            self.default_resolved,
            None,
            self.default_face_char_width,
            self.fallback_metrics,
            font_metrics,
        );
        let mut active_face_state =
            DisplayRowActiveFaceState::new(self.default_resolved.clone(), default_measured_face);
        let (output_emitter, post_loop) = walk_setup.begin_render_body_and_tail(
            self.begin_request,
            output,
            font_metrics,
            face_resolver,
            face_ids,
            &mut line_numbers,
            &mut face_scan,
            &mut active_face_state,
            self.row_prelude_context,
            self.loop_context,
            self.face_resolution,
            &self.tail_context,
            text,
            params,
            self.overlay_text_row,
            buffer,
            buf_access,
        );

        BufferTextWindowRenderedBody {
            output_emitter,
            post_loop,
            retry_bounds: self.retry_bounds,
            publish_request: self.publish_request,
            tail_context: self.tail_context,
        }
    }
}

impl<'a> BufferTextWindowRenderedBody<'a> {
    fn retry_plan(&self, walk_setup: &BufferTextWindowWalkSetup) -> BufferTextWindowRetryPlan {
        BufferTextWindowRetryPlan::from_post_loop(
            self.tail_context.params.window_id,
            self.tail_context.window_start,
            self.tail_context.params.point_charpos().get(),
            walk_setup.charpos,
            self.retry_bounds,
            self.post_loop,
        )
    }

    fn install_body_chrome_and_finish(
        mut self,
        walk_setup: &mut BufferTextWindowWalkSetup,
        chrome_request: WindowChromeRowsRenderRequest<'_, '_>,
        output_state: BufferTextWindowOutputState<'_>,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        hit_data: &mut Vec<WindowHitData>,
        display_snapshots: &mut Vec<WindowDisplaySnapshot>,
    ) -> TextWindowRedisplayPositions {
        let BufferTextWindowOutputState {
            mut output,
            evaluator,
        } = output_state;
        let mut render_services =
            ChromeRowRenderServices::new(font_metrics, face_resolver, face_ids);
        let redisplay_positions = walk_setup.install_body_and_publish_redisplay(
            output.reborrow(),
            &mut self.output_emitter,
            evaluator,
            render_services.reborrow(),
            &self.tail_context,
            self.publish_request,
        );
        render_window_chrome_rows(
            output.reborrow(),
            &mut self.output_emitter,
            evaluator,
            chrome_request,
            render_services.reborrow(),
        );
        self.tail_context
            .finish_and_install(BufferTextWindowFinishInstallState {
                finish_state: BufferTextWindowFinishState::new(
                    output,
                    self.output_emitter,
                    evaluator,
                    std::mem::take(&mut walk_setup.hit_rows),
                ),
                hit_data,
                display_snapshots,
            });
        redisplay_positions
    }

    fn finish_or_prepare_retry(
        self,
        walk_setup: &mut BufferTextWindowWalkSetup,
        chrome_request: WindowChromeRowsRenderRequest<'_, '_>,
        mut output: BufferTextWindowOutputState<'_>,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        retry_checkpoint: TextWindowOutputRetryCheckpoint,
        hit_data: &mut Vec<WindowHitData>,
        display_snapshots: &mut Vec<WindowDisplaySnapshot>,
        frame_face_id_counter: &mut u32,
        remaining_visibility_retries: usize,
    ) -> BufferTextWindowRenderAttemptOutcome {
        let retry_plan = self.retry_plan(walk_setup);
        retry_plan.log_visibility_adjustments();

        if let Some(window_start) = retry_plan.should_retry(remaining_visibility_retries) {
            retry_plan.log_retry(window_start, remaining_visibility_retries);
            output.restore_retry_checkpoint(retry_checkpoint);
            *frame_face_id_counter = (*face_ids).finish();
            return BufferTextWindowRenderAttemptOutcome::Retry { window_start };
        }

        let redisplay_positions = self.install_body_chrome_and_finish(
            walk_setup,
            chrome_request,
            output,
            font_metrics,
            face_resolver,
            face_ids,
            hit_data,
            display_snapshots,
        );
        *frame_face_id_counter = (*face_ids).finish();
        BufferTextWindowRenderAttemptOutcome::Finished {
            redisplay_positions,
        }
    }
}
