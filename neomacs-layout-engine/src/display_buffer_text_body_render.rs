//! Buffer text body walk setup and render pass driver.

use crate::display_buffer_text_append::{
    BufferTextWindowBeginRequest, BufferTextWindowBodyInstallState, TextWindowAppendSurfaceRequest,
};
use crate::display_buffer_text_face_resolution::*;
use crate::display_buffer_text_item_append::BufferTextRowAppendState;
use crate::display_buffer_text_loop_context::BufferTextWindowLoopRequestContext;
use crate::display_buffer_text_loop_render::BufferTextWindowLoopRenderState;
use crate::display_buffer_text_output_session::{
    BufferTextWindowOutputState, BufferTextWindowRedisplayPublishRequest,
};
use crate::display_buffer_text_progress::{
    BufferTextWindowProgressState, BufferTextWindowRowProgressState,
};
use crate::display_buffer_text_render_plan::BufferTextWindowDefaultFacePlan;
use crate::display_buffer_text_row_prelude::BufferTextWindowRowPreludeRequestContext;
use crate::display_buffer_text_source::BufferTextWindowSource;
use crate::display_buffer_text_source_walk::BufferTextWindowSourceWalk;
use crate::display_buffer_text_tail_render::{
    BufferTextWindowPostLoopRenderOutcome, BufferTextWindowPostLoopRenderState,
    BufferTextWindowPostLoopState, BufferTextWindowTailRequestContext,
};
use crate::display_buffer_text_walk::{
    BufferTextWindowGeometry, BufferTextWindowLocalDisplayPolicy,
};
use crate::display_cursor::CursorCaptureState;
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_row::DisplayRowActiveFaceState;
use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_geometry::{
    DisplayRowFlags, DisplayRowGeometryDefaults, DisplayRowGeometryState, DisplayRowScopedValue,
    DisplayRowYPositions,
};
use crate::display_row_lisp_string::DisplayRowPrefixRequest;
use crate::display_row_overlay_string::BufferOverlayStringTextRowRenderContext;
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_row_walk_state::{
    BoxFaceRowState, FaceScanCheckpoint, HitRowRangeTracker, HorizontalScrollSkipState,
    InvisibleTextScanCheckpoint, LineNumberRenderState, TrailingWhitespaceRenderState,
    WordWrapRenderState,
};
use crate::display_status_line::ChromeRowRenderServices;
use crate::font_metrics::FontMetricsService;
use crate::hit_test::HitRow;
use crate::neovm_bridge::{FaceResolver, LayoutBufferView, RustBufferAccess};
use crate::types::{LineWrapMode, WindowParams};
use crate::window_output::{
    TextWindowOutputTarget, TextWindowRedisplayPositions, WindowOutputEmitter,
};
use neomacs_display_protocol::types::Color;
use neovm_core::emacs_core::Context;

pub(crate) struct BufferTextWindowWalkSetupRequest<'a> {
    window_start: i64,
    content_x: f32,
    text_x: f32,
    text_width: f32,
    text_y: f32,
    window_top: f32,
    line_number_pixel_width: f32,
    max_rows: usize,
    char_width: f32,
    char_height: f32,
    default_face_ascent: f32,
    wrap_mode: LineWrapMode,
    hscroll: i32,
    word_wrap: bool,
    has_prefix: bool,
    has_line_default_prefix: bool,
    reserve_right_border_col: bool,
    reserve_right_special_col: bool,
    tab_width: i32,
    tab_stop_list: &'a [i32],
    trailing_whitespace_enabled: bool,
    trailing_whitespace_bg: u32,
}

pub(crate) struct BufferTextWindowWalkSetup {
    pub(crate) x: f32,
    pub(crate) col: usize,
    pub(crate) byte_idx: usize,
    pub(crate) charpos: i64,
    pub(crate) text_area_left: f32,
    pub(crate) window_top: f32,
    pub(crate) invisible_text_checkpoint: InvisibleTextScanCheckpoint,
    pub(crate) row_flags: DisplayRowFlags,
    pub(crate) hscroll_skip: HorizontalScrollSkipState,
    pub(crate) word_wrap: WordWrapRenderState,
    pub(crate) prefix_request: DisplayRowPrefixRequest,
    pub(crate) text_append_surface: DisplayRowAppendSurface,
    pub(crate) row_geometry_defaults: DisplayRowGeometryDefaults,
    pub(crate) row_geometry: DisplayRowGeometryState,
    pub(crate) row_y_positions: DisplayRowYPositions,
    pub(crate) trailing_whitespace: TrailingWhitespaceRenderState,
    pub(crate) buffer_text_append_state: BufferTextRowAppendState,
    pub(crate) row_extend: DisplayRowScopedValue<(Color, u32)>,
    pub(crate) box_face: BoxFaceRowState,
    pub(crate) cursor_info: CursorCaptureState,
    pub(crate) hit_rows: Vec<HitRow>,
    pub(crate) hit_row_range: HitRowRangeTracker,
}

struct BufferTextWindowWalkRenderState<'emit> {
    source_render: TextRowSourceRenderState<'emit>,
    line_numbers: &'emit mut LineNumberRenderState,
    face_scan: &'emit mut FaceScanCheckpoint,
    active_face_state: &'emit mut DisplayRowActiveFaceState,
    face_ids: &'emit mut FrameFaceIdAllocator,
}

struct BufferTextWindowBodyRenderState<'emit> {
    source_render: TextRowSourceRenderState<'emit>,
    line_numbers: &'emit mut LineNumberRenderState,
    face_scan: &'emit mut FaceScanCheckpoint,
    active_face_state: &'emit mut DisplayRowActiveFaceState,
    face_ids: &'emit mut FrameFaceIdAllocator,
}

impl<'emit> BufferTextWindowWalkRenderState<'emit> {
    fn new(
        source_render: TextRowSourceRenderState<'emit>,
        line_numbers: &'emit mut LineNumberRenderState,
        face_scan: &'emit mut FaceScanCheckpoint,
        active_face_state: &'emit mut DisplayRowActiveFaceState,
        face_ids: &'emit mut FrameFaceIdAllocator,
    ) -> Self {
        Self {
            source_render,
            line_numbers,
            face_scan,
            active_face_state,
            face_ids,
        }
    }
}

impl<'emit> BufferTextWindowBodyRenderState<'emit> {
    fn new(
        source_render: TextRowSourceRenderState<'emit>,
        line_numbers: &'emit mut LineNumberRenderState,
        face_scan: &'emit mut FaceScanCheckpoint,
        active_face_state: &'emit mut DisplayRowActiveFaceState,
        face_ids: &'emit mut FrameFaceIdAllocator,
    ) -> Self {
        Self {
            source_render,
            line_numbers,
            face_scan,
            active_face_state,
            face_ids,
        }
    }
}

impl<'a> BufferTextWindowWalkSetupRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        window_start: i64,
        content_x: f32,
        text_x: f32,
        text_width: f32,
        text_y: f32,
        window_top: f32,
        line_number_pixel_width: f32,
        max_rows: usize,
        char_width: f32,
        char_height: f32,
        default_face_ascent: f32,
        wrap_mode: LineWrapMode,
        hscroll: i32,
        word_wrap: bool,
        has_prefix: bool,
        has_line_default_prefix: bool,
        reserve_right_border_col: bool,
        reserve_right_special_col: bool,
        tab_width: i32,
        tab_stop_list: &'a [i32],
        trailing_whitespace_enabled: bool,
        trailing_whitespace_bg: u32,
    ) -> Self {
        Self {
            window_start,
            content_x,
            text_x,
            text_width,
            text_y,
            window_top,
            line_number_pixel_width,
            max_rows,
            char_width,
            char_height,
            default_face_ascent,
            wrap_mode,
            hscroll,
            word_wrap,
            has_prefix,
            has_line_default_prefix,
            reserve_right_border_col,
            reserve_right_special_col,
            tab_width,
            tab_stop_list,
            trailing_whitespace_enabled,
            trailing_whitespace_bg,
        }
    }

    pub(crate) fn from_window_geometry(
        source: BufferTextWindowSource,
        params: &'a WindowParams,
        geometry: &BufferTextWindowGeometry,
        local_display_policy: &BufferTextWindowLocalDisplayPolicy,
        default_face: &BufferTextWindowDefaultFacePlan,
        reserve_right_border_col: bool,
        reserve_right_special_col: bool,
    ) -> Self {
        Self::new(
            source.window_start(),
            geometry.content_x,
            geometry.text_x,
            geometry.text_width,
            geometry.text_y,
            params.bounds.y,
            geometry.line_number_pixel_width,
            geometry.max_rows,
            geometry.char_width,
            geometry.char_height,
            default_face.ascent(),
            params.wrap_mode,
            params.hscroll,
            params.word_wrap,
            local_display_policy.has_prefix(),
            local_display_policy.has_line_default_prefix(),
            reserve_right_border_col,
            reserve_right_special_col,
            params.tab_width,
            &params.tab_stop_list,
            params.show_trailing_whitespace,
            params.trailing_ws_bg,
        )
    }

    pub(crate) fn into_setup(self) -> BufferTextWindowWalkSetup {
        let row_geometry_defaults = DisplayRowGeometryDefaults::new(
            self.text_y,
            self.char_height,
            self.default_face_ascent,
        );

        BufferTextWindowWalkSetup {
            x: self.content_x,
            col: 0,
            byte_idx: 0,
            charpos: self.window_start,
            text_area_left: self.text_x,
            window_top: self.window_top,
            invisible_text_checkpoint: InvisibleTextScanCheckpoint::new(self.window_start),
            row_flags: DisplayRowFlags::new(self.max_rows),
            hscroll_skip: HorizontalScrollSkipState::new(self.wrap_mode, self.hscroll),
            word_wrap: WordWrapRenderState::new(self.word_wrap),
            prefix_request: DisplayRowPrefixRequest::initial(
                self.has_prefix,
                self.has_line_default_prefix,
            ),
            text_append_surface: TextWindowAppendSurfaceRequest::new(
                self.content_x,
                self.text_width,
                self.line_number_pixel_width,
                self.reserve_right_border_col,
                self.reserve_right_special_col,
                self.char_width,
                self.tab_width,
                self.tab_stop_list,
            )
            .into_surface(),
            row_geometry_defaults,
            row_geometry: row_geometry_defaults.initial_state(),
            row_y_positions: DisplayRowYPositions::with_capacity_and_first_row(
                self.max_rows,
                self.text_y,
            ),
            trailing_whitespace: TrailingWhitespaceRenderState::new(
                self.trailing_whitespace_enabled,
                self.trailing_whitespace_bg,
            ),
            buffer_text_append_state: BufferTextRowAppendState::default(),
            row_extend: DisplayRowScopedValue::inactive(),
            box_face: BoxFaceRowState::inactive(),
            cursor_info: CursorCaptureState::new(),
            hit_rows: Vec::new(),
            hit_row_range: HitRowRangeTracker::new(self.window_start),
        }
    }
}

impl BufferTextWindowWalkSetup {
    #[allow(clippy::too_many_arguments)]
    fn render_visible_steps<'request, B: LayoutBufferView>(
        &mut self,
        state: &mut BufferTextWindowWalkRenderState<'_>,
        row_prelude_context: BufferTextWindowRowPreludeRequestContext,
        loop_context: BufferTextWindowLoopRequestContext,
        face_resolution_context: BufferCurrentFaceResolutionContext<'request, B>,
        text: &'request [u8],
        params: &WindowParams,
        overlay_text_row_context: BufferOverlayStringTextRowRenderContext<'request>,
        buffer: &B,
    ) {
        let mut source_walk = BufferTextWindowSourceWalk::new(
            loop_context.buffer_id(),
            buffer,
            self.charpos,
            loop_context.text_start_byte(),
        );

        BufferTextWindowLoopRenderState::new(
            loop_context,
            &mut self.buffer_text_append_state,
            &mut self.invisible_text_checkpoint,
            BufferTextWindowProgressState::new(
                &mut self.byte_idx,
                &mut self.charpos,
                &mut self.x,
                &mut self.col,
            ),
            state.source_render.reborrow(),
            &mut self.row_extend,
            &mut self.box_face,
            state.line_numbers,
            &mut self.row_geometry,
            &mut self.row_flags,
            &mut self.hit_rows,
            &mut self.hit_row_range,
            &mut self.prefix_request,
            &mut self.hscroll_skip,
            &mut self.word_wrap,
            &mut self.trailing_whitespace,
            state.face_scan,
            &mut self.row_y_positions,
            &mut self.cursor_info,
            state.face_ids,
            &self.text_append_surface,
            overlay_text_row_context,
        )
        .render_visible_steps(
            &mut source_walk,
            row_prelude_context,
            face_resolution_context,
            text,
            params,
            state.active_face_state,
            buffer,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn render_tail_and_decide_retry<'request, 'buf, B: LayoutBufferView>(
        &mut self,
        state: &mut BufferTextWindowPostLoopRenderState<'_>,
        loop_context: BufferTextWindowLoopRequestContext,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
        text: &'request [u8],
        overlay_context: BufferOverlayStringTextRowRenderContext<'request>,
        active_face_state: &'request DisplayRowActiveFaceState,
        buffer: &B,
        buf_access: &RustBufferAccess<'buf, B>,
    ) -> BufferTextWindowPostLoopRenderOutcome {
        BufferTextWindowPostLoopState::new(
            loop_context,
            state.source_render.reborrow(),
            BufferTextWindowRowProgressState::new(&mut self.x, &mut self.col),
            &mut self.row_geometry,
            &mut self.cursor_info,
            &mut self.hit_rows,
            &mut self.hit_row_range,
            &mut self.row_y_positions,
            state.face_ids,
            &self.row_flags,
            &self.row_extend,
            &self.box_face,
            &self.text_append_surface,
            overlay_context,
        )
        .render_tail_and_decide_retry(
            tail_context,
            text,
            self.byte_idx,
            self.charpos,
            active_face_state,
            buffer,
            buf_access,
        )
    }

    fn install_body(
        &mut self,
        output: TextWindowOutputTarget<'_>,
        output_emitter: &mut WindowOutputEmitter,
        render_services: ChromeRowRenderServices<'_, '_>,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
    ) -> TextWindowRedisplayPositions {
        tail_context
            .body_install_request(self.byte_idx, &self.row_flags)
            .install_and_apply(BufferTextWindowBodyInstallState::new(
                output,
                output_emitter,
                render_services,
            ))
    }

    pub(crate) fn install_body_and_publish_redisplay(
        &mut self,
        output: TextWindowOutputTarget<'_>,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        render_services: ChromeRowRenderServices<'_, '_>,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
        publish_request: BufferTextWindowRedisplayPublishRequest,
    ) -> TextWindowRedisplayPositions {
        let redisplay_positions =
            self.install_body(output, output_emitter, render_services, tail_context);
        // GNU status-line percent specs read the live window state from the
        // just-produced redisplay. Publish before chrome rows are evaluated.
        publish_request.publish(evaluator, redisplay_positions);
        redisplay_positions
    }

    #[allow(clippy::too_many_arguments)]
    fn render_body_and_tail<'request, 'buf, B: LayoutBufferView>(
        &mut self,
        state: &mut BufferTextWindowBodyRenderState<'_>,
        row_prelude_context: BufferTextWindowRowPreludeRequestContext,
        loop_context: BufferTextWindowLoopRequestContext,
        face_resolution_context: BufferCurrentFaceResolutionContext<'request, B>,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
        text: &'request [u8],
        params: &'request WindowParams,
        overlay_text_row_context: BufferOverlayStringTextRowRenderContext<'request>,
        buffer: &B,
        buf_access: &RustBufferAccess<'buf, B>,
    ) -> BufferTextWindowPostLoopRenderOutcome {
        self.render_visible_steps(
            &mut BufferTextWindowWalkRenderState::new(
                state.source_render.reborrow(),
                state.line_numbers,
                state.face_scan,
                state.active_face_state,
                state.face_ids,
            ),
            row_prelude_context,
            loop_context,
            face_resolution_context,
            text,
            params,
            overlay_text_row_context,
            buffer,
        );

        self.render_tail_and_decide_retry(
            &mut BufferTextWindowPostLoopRenderState::new(
                state.source_render.reborrow(),
                state.face_ids,
            ),
            loop_context,
            tail_context,
            text,
            overlay_text_row_context,
            state.active_face_state,
            buffer,
            buf_access,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn begin_render_body_and_tail<'request, 'buf, B: LayoutBufferView>(
        &mut self,
        begin_request: BufferTextWindowBeginRequest,
        output: &mut BufferTextWindowOutputState<'_>,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        line_numbers: &mut LineNumberRenderState,
        face_scan: &mut FaceScanCheckpoint,
        active_face_state: &mut DisplayRowActiveFaceState,
        row_prelude_context: BufferTextWindowRowPreludeRequestContext,
        loop_context: BufferTextWindowLoopRequestContext,
        face_resolution_context: BufferCurrentFaceResolutionContext<'request, B>,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
        text: &'request [u8],
        params: &'request WindowParams,
        overlay_text_row_context: BufferOverlayStringTextRowRenderContext<'request>,
        buffer: &B,
        buf_access: &RustBufferAccess<'buf, B>,
    ) -> (WindowOutputEmitter, BufferTextWindowPostLoopRenderOutcome) {
        let mut output_emitter = output.begin_text_window_output(begin_request);
        let source_render =
            output.source_render_state(&mut output_emitter, font_metrics, face_resolver);
        let post_loop = self.render_body_and_tail(
            &mut BufferTextWindowBodyRenderState::new(
                source_render,
                line_numbers,
                face_scan,
                active_face_state,
                face_ids,
            ),
            row_prelude_context,
            loop_context,
            face_resolution_context,
            tail_context,
            text,
            params,
            overlay_text_row_context,
            buffer,
            buf_access,
        );
        (output_emitter, post_loop)
    }
}
