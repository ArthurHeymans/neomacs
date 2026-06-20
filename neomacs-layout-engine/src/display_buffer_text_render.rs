//! Buffer-text source rendering requests and actions.
#[cfg(test)]
pub(crate) use crate::display_buffer_display_property_render::BufferDisplayPropertyTextReplacementOutcome;
pub(crate) use crate::display_buffer_display_property_render::{
    BufferDisplayPropertyTextReplacementRenderState,
    BufferDisplayPropertyTextReplacementResolveOutcome,
    BufferDisplayPropertyTextReplacementResolveRequest,
};
use crate::display_buffer_display_property_source::BufferTextReplacementItem;
use crate::display_buffer_text_append::{
    BufferTextWindowBeginRequest, BufferTextWindowBodyInstallRenderContext,
    BufferTextWindowBodyInstallRequest, BufferTextWindowBodyInstallState,
    BufferTextWindowFinishRequest, BufferTextWindowFinishState,
    BufferTextWindowTailFinalizeContext, BufferTextWindowTailFinalizeOutcome,
    BufferTextWindowTailFinalizeRequest, BufferTextWindowTailFinalizeState,
    BufferTextWindowVisibilityRetryOutcome, BufferTextWindowVisibilityRetryRequest,
    TextWindowAppendSurfaceRequest,
};
use crate::display_buffer_text_face_resolution::*;
use crate::display_buffer_text_item_append::BufferTextRowAppendState;
use crate::display_buffer_text_output_session::{
    BufferTextWindowBodyInstallPublishState, BufferTextWindowBodyInstallRenderState,
    BufferTextWindowBodyPassOutcome, BufferTextWindowBodyPassState, BufferTextWindowOutputSession,
    BufferTextWindowOutputState, BufferTextWindowRedisplayPublishRequest,
    BufferTextWindowRenderedBodyCompleteState, BufferTextWindowRenderedBodyFinishState,
    BufferTextWindowRetryPlan,
};
pub(crate) use crate::display_buffer_text_output_session::{
    BufferTextWindowRenderAttemptContext, BufferTextWindowRenderAttemptOutcome,
};
use crate::display_buffer_text_overflow::*;
pub(crate) use crate::display_buffer_text_progress::{
    BufferTextWindowProgressState, BufferTextWindowRowProgressState,
};
use crate::display_buffer_text_row_lifecycle::*;
use crate::display_buffer_text_row_prelude::BufferTextWindowRowPreludeRequestContext;
use crate::display_buffer_text_source::{
    BufferTextWindowSource, BufferTextWindowSourceReadRequest,
};
use crate::display_buffer_text_source_walk::*;
use crate::display_buffer_text_tail_decoration::{
    BufferTextWindowTailDecorationOutcome, BufferTextWindowTailDecorationRequest,
    BufferTextWindowTailDecorationState,
};
use crate::display_buffer_text_walk::{
    BufferTextWindowChromeHeights, BufferTextWindowGeometry, BufferTextWindowGeometryPlan,
    BufferTextWindowGeometryRequest, BufferTextWindowLocalDisplayPolicy,
};
use crate::display_cursor::CursorCaptureState;
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_row::{
    DisplayRowActiveFaceState, DisplayRowFallbackMetrics, DisplayRowMeasurementPolicy,
};
use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_geometry::{
    DisplayRowFlags, DisplayRowGeometryDefaults, DisplayRowGeometryState, DisplayRowLimit,
    DisplayRowScopedValue, DisplayRowVisibilityLimit, DisplayRowYPositions,
};
use crate::display_row_lisp_string::DisplayRowPrefixRequest;
use crate::display_row_overlay_string::BufferOverlayStringTextRowRenderContext;
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_row_transition::DisplayRowTransitionContinuation;
use crate::display_row_walk_state::{
    BoxFaceRowState, FaceScanCheckpoint, HitRowRangeTracker, HorizontalScrollSkipState,
    InvisibleTextScanCheckpoint, LineNumberRenderState, TrailingWhitespaceRenderState,
    WordWrapRenderState,
};
use crate::display_status_line::{
    WindowChromeRowsPlan, WindowChromeRowsRenderRequest, max_mini_window_lines,
    max_mini_window_lines_for_buffer,
};
use crate::font_metrics::FontMetricsService;
use crate::hit_test::{HitRow, WindowHitData};
use crate::neovm_bridge::{FaceResolver, LayoutBufferView, ResolvedFace, RustBufferAccess};
use crate::types::{FrameParams, LineWrapMode, WindowParams};
use crate::window_output::{
    TextWindowRedisplayPositions, WindowOutputEmitter, render_window_chrome_rows,
};
use neomacs_display_protocol::types::{Color, Rect};
use neovm_core::buffer::BufferId;
use neovm_core::window::{DisplayRowSnapshot, FrameId, WindowDisplaySnapshot, WindowId};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextWindowLoopRequestContext {
    buffer_id: BufferId,
    text_start_byte: usize,
    accessible_end: i64,
    point_charpos: i64,
    selective_display: i32,
    tab_width: i32,
    extra_line_spacing: f32,
    content_x: f32,
    has_prefix: bool,
    default_face_ascent: f32,
    char_height: f32,
    char_width: f32,
    row_visibility_limit: DisplayRowVisibilityLimit,
    row_geometry_defaults: DisplayRowGeometryDefaults,
    display_text_row_base: usize,
    max_rows: usize,
    row_limit: DisplayRowLimit,
}

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
    cols: usize,
    char_width: f32,
    char_height: f32,
    default_fg: Color,
    max_rows: usize,
    row_limit: DisplayRowLimit,
    row_geometry_defaults: DisplayRowGeometryDefaults,
    retry_bounds: BufferTextWindowRetryBounds,
    body_install_context: BufferTextWindowBodyInstallContext,
    reserve_right_special_col: bool,
    reserve_right_border_col: bool,
    mode_line_height: f32,
    header_line_height: f32,
    tab_line_height: f32,
}

pub(crate) struct BufferTextWindowFinishInstallState<'a> {
    pub(crate) finish_state: BufferTextWindowFinishState<'a>,
    pub(crate) hit_data: &'a mut Vec<WindowHitData>,
    pub(crate) display_snapshots: &'a mut Vec<WindowDisplaySnapshot>,
}

struct BufferTextWindowPostLoopState<'rows, 'emit, 'surface> {
    loop_context: BufferTextWindowLoopRequestContext,
    source_render: TextRowSourceRenderState<'emit>,
    row_progress: BufferTextWindowRowProgressState<'emit>,
    row_geometry: &'emit mut DisplayRowGeometryState,
    cursor_info: &'emit mut CursorCaptureState,
    hit_rows: &'emit mut Vec<HitRow>,
    hit_row_range: &'emit mut HitRowRangeTracker,
    row_y_positions: &'rows mut DisplayRowYPositions,
    face_ids: &'emit mut FrameFaceIdAllocator,
    row_flags: &'emit DisplayRowFlags,
    row_extend: &'emit DisplayRowScopedValue<(Color, u32)>,
    box_face: &'emit BoxFaceRowState,
    text_append_surface: &'surface DisplayRowAppendSurface,
    overlay_context: BufferOverlayStringTextRowRenderContext<'surface>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowPostLoopRenderOutcome {
    pub(crate) retry: BufferTextWindowVisibilityRetryOutcome,
    pub(crate) rendered_rows_len: usize,
}

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

pub(crate) struct BufferTextWindowOutputSetupRequest {
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
}

pub(crate) struct BufferTextWindowOutputSetup {
    pub(crate) begin_request: BufferTextWindowBeginRequest,
    pub(crate) row_visibility_limit: DisplayRowVisibilityLimit,
    pub(crate) row_limit: DisplayRowLimit,
    pub(crate) body_install_context: BufferTextWindowBodyInstallContext,
    pub(crate) retry_bounds: BufferTextWindowRetryBounds,
}

pub(crate) struct BufferTextWindowDefaultFacePlan {
    face: ResolvedFace,
    foreground: Color,
    char_width: f32,
    row_height: f32,
    ascent: f32,
    measurement_policy: DisplayRowMeasurementPolicy,
}

struct BufferTextWindowWalkRenderState<'emit> {
    source_render: TextRowSourceRenderState<'emit>,
    line_numbers: &'emit mut LineNumberRenderState,
    face_scan: &'emit mut FaceScanCheckpoint,
    active_face_state: &'emit mut DisplayRowActiveFaceState,
    face_ids: &'emit mut FrameFaceIdAllocator,
}

struct BufferTextWindowPostLoopRenderState<'emit> {
    source_render: TextRowSourceRenderState<'emit>,
    face_ids: &'emit mut FrameFaceIdAllocator,
}

struct BufferTextWindowBodyRenderState<'emit> {
    source_render: TextRowSourceRenderState<'emit>,
    line_numbers: &'emit mut LineNumberRenderState,
    face_scan: &'emit mut FaceScanCheckpoint,
    active_face_state: &'emit mut DisplayRowActiveFaceState,
    face_ids: &'emit mut FrameFaceIdAllocator,
}

pub(crate) struct BufferTextWindowRenderRequest<'a, B>
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

struct BufferTextWindowRenderedBody<'a> {
    output_emitter: WindowOutputEmitter,
    post_loop: BufferTextWindowPostLoopRenderOutcome,
    retry_bounds: BufferTextWindowRetryBounds,
    publish_request: BufferTextWindowRedisplayPublishRequest,
    tail_context: BufferTextWindowTailRequestContext<'a>,
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

impl<'emit> BufferTextWindowPostLoopRenderState<'emit> {
    fn new(
        source_render: TextRowSourceRenderState<'emit>,
        face_ids: &'emit mut FrameFaceIdAllocator,
    ) -> Self {
        Self {
            source_render,
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

struct BufferTextWindowRenderContextsRequest<'a, 'surface, B>
where
    B: LayoutBufferView,
{
    buffer: &'a B,
    face_resolver: &'a FaceResolver,
    measurement_policy: DisplayRowMeasurementPolicy,
    default_resolved: &'a ResolvedFace,
    default_face_char_width: f32,
    default_face_ascent: f32,
    default_face_height: f32,
    char_width: f32,
    char_height: f32,
    font_ascent: f32,
    window_system: bool,
    output_window_id: u64,
    append_surface: &'surface DisplayRowAppendSurface,
    text_y: f32,
    display_text_row_base: usize,
    max_rows: usize,
}

struct BufferTextWindowRenderContexts<'a, 'surface, B>
where
    B: LayoutBufferView,
{
    pub(crate) face_resolution: BufferCurrentFaceResolutionContext<'a, B>,
    pub(crate) overlay_text_row: BufferOverlayStringTextRowRenderContext<'surface>,
}

struct BufferTextWindowBodyPlan<'a, 'surface, B>
where
    B: LayoutBufferView,
{
    begin_request: BufferTextWindowBeginRequest,
    retry_bounds: BufferTextWindowRetryBounds,
    publish_request: BufferTextWindowRedisplayPublishRequest,
    local_display_policy: BufferTextWindowLocalDisplayPolicy,
    initial_face_state: BufferTextWindowInitialFaceStateRequest<'a>,
    row_prelude_context: BufferTextWindowRowPreludeRequestContext,
    loop_context: BufferTextWindowLoopRequestContext,
    render_contexts: BufferTextWindowRenderContexts<'a, 'surface, B>,
    tail_context: BufferTextWindowTailRequestContext<'a>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BufferTextWindowInitialFaceStateRequest<'a> {
    measurement_policy: DisplayRowMeasurementPolicy,
    default_resolved: &'a ResolvedFace,
    default_face_char_width: f32,
    fallback_metrics: DisplayRowFallbackMetrics,
}

struct BufferTextWindowLoopRenderState<'rows, 'emit, 'surface> {
    loop_context: BufferTextWindowLoopRequestContext,
    append_state: &'emit mut BufferTextRowAppendState,
    invisible_text_checkpoint: &'emit mut InvisibleTextScanCheckpoint,
    progress: BufferTextWindowProgressState<'emit>,
    source_render: TextRowSourceRenderState<'emit>,
    row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    box_face: &'emit mut BoxFaceRowState,
    line_numbers: &'emit mut LineNumberRenderState,
    row_geometry: &'emit mut DisplayRowGeometryState,
    row_flags: &'emit mut DisplayRowFlags,
    hit_rows: &'emit mut Vec<HitRow>,
    hit_row_range: &'emit mut HitRowRangeTracker,
    prefix_request: &'emit mut DisplayRowPrefixRequest,
    hscroll_skip: &'emit mut HorizontalScrollSkipState,
    word_wrap: &'emit mut WordWrapRenderState,
    trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    face_scan: &'emit mut FaceScanCheckpoint,
    row_y_positions: &'rows mut DisplayRowYPositions,
    cursor_info: &'emit mut CursorCaptureState,
    face_ids: &'emit mut FrameFaceIdAllocator,
    append_surface: &'surface DisplayRowAppendSurface,
    overlay_context: BufferOverlayStringTextRowRenderContext<'surface>,
}

struct BufferTextWindowConsumedDisplayItemRenderRequest<'a> {
    layout_resolution_context: BufferSourceItemLayoutResolutionContext<'a>,
    source_item: BufferTextConsumedDisplayItem,
    text: &'a [u8],
    active_face_state: &'a DisplayRowActiveFaceState,
    params: &'a WindowParams,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BufferTextWindowLoopStepOutcome {
    ContinueBufferWalk,
    StopBufferWalk,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BufferTextWindowPreSourceOutcome {
    ReadyForSourceItem,
    ContinueBufferWalk,
    StopBufferWalk,
}

impl BufferTextWindowLoopRequestContext {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        buffer_id: BufferId,
        text_start_byte: usize,
        accessible_end: i64,
        point_charpos: i64,
        params: &WindowParams,
        content_x: f32,
        has_prefix: bool,
        default_face_ascent: f32,
        char_height: f32,
        char_width: f32,
        row_visibility_limit: DisplayRowVisibilityLimit,
        row_geometry_defaults: DisplayRowGeometryDefaults,
        display_text_row_base: usize,
        max_rows: usize,
        row_limit: DisplayRowLimit,
    ) -> Self {
        Self {
            buffer_id,
            text_start_byte,
            accessible_end,
            point_charpos,
            selective_display: params.selective_display,
            tab_width: params.tab_width,
            extra_line_spacing: params.extra_line_spacing,
            content_x,
            has_prefix,
            default_face_ascent,
            char_height,
            char_width,
            row_visibility_limit,
            row_geometry_defaults,
            display_text_row_base,
            max_rows,
            row_limit,
        }
    }

    pub(crate) fn invisible_text_request<'a>(
        self,
        text: &'a [u8],
        append_surface: &'a DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
    ) -> BufferInvisibleTextRenderRequest<'a> {
        BufferInvisibleTextRenderRequest::new(BufferInvisibleTextRenderContext::new(
            text,
            self.accessible_end,
            self.point_charpos,
            append_surface,
            overlay_context,
            active_face_state,
            glyph_y_offset,
            self.default_face_ascent,
            self.char_height,
            self.char_width,
        ))
    }

    pub(crate) fn hscroll_skip_request<'a>(
        self,
        text: &'a [u8],
        append_surface: &'a DisplayRowAppendSurface,
        active_face_state: &'a DisplayRowActiveFaceState,
    ) -> BufferHscrollSkipRenderRequest<'a> {
        BufferHscrollSkipRenderRequest::new(BufferHscrollSkipRenderContext::new(
            text,
            self.tab_width,
            self.content_x,
            append_surface,
            active_face_state,
            self.default_face_ascent,
            self.char_height,
            self.char_width,
            self.point_charpos,
            self.has_prefix,
            self.row_geometry_defaults,
            self.display_text_row_base,
            self.max_rows,
            self.row_limit,
        ))
    }

    pub(crate) fn selective_display_tail_request<'a>(
        self,
        source_step_char: BufferTextSourceStepChar,
        text: &'a [u8],
        append_surface: &'a DisplayRowAppendSurface,
        active_face_state: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
    ) -> BufferSelectiveDisplayTailRenderRequest<'a> {
        BufferSelectiveDisplayTailRenderRequest::new(
            source_step_char,
            BufferSelectiveDisplayTailRenderContext::new(
                text,
                self.text_start_byte,
                self.selective_display,
                self.tab_width,
                append_surface,
                active_face_state,
                glyph_y_offset,
                self.default_face_ascent,
                self.char_height,
                self.char_width,
                self.content_x,
                self.has_prefix,
                self.row_geometry_defaults,
                self.display_text_row_base,
                self.max_rows,
                self.row_limit,
            ),
        )
    }

    pub(crate) fn line_break_request<'a>(
        self,
        source_char: BufferTextSourceStepChar,
        text: &'a [u8],
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
    ) -> BufferTextLineBreakRenderRequest<'a> {
        BufferTextLineBreakRenderRequest::new(
            source_char,
            BufferTextLineBreakRenderContext::new(
                text,
                self.text_start_byte,
                self.selective_display,
                self.tab_width,
                active_face_state,
                self.point_charpos,
                self.char_height,
                self.extra_line_spacing,
                self.content_x,
                self.has_prefix,
                self.row_geometry_defaults,
                self.display_text_row_base,
                self.max_rows,
                self.row_limit,
                overlay_context,
            ),
        )
    }

    pub(crate) fn consumed_display_item_request<'a>(
        self,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'a>,
        source_item: BufferTextConsumedDisplayItem,
        text: &'a [u8],
        append_surface: &'a DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
        params: &'a WindowParams,
        glyph_y_offset: f32,
    ) -> BufferTextConsumedDisplayItemRenderRequest<'a> {
        BufferTextConsumedDisplayItemRenderRequest::new(
            source_item,
            BufferTextConsumedDisplayItemRenderContext::new(
                layout_resolution_context,
                text,
                self.text_start_byte,
                self.buffer_id,
                append_surface,
                overlay_context,
                active_face_state,
                params,
                glyph_y_offset,
                self.char_height,
                self.point_charpos,
                self.row_visibility_limit,
                self.content_x,
                self.has_prefix,
                self.row_geometry_defaults,
                self.display_text_row_base,
                self.max_rows,
                self.row_limit,
            ),
        )
    }

    pub(crate) fn end_of_buffer_tail_request<'a>(
        self,
        byte_idx: usize,
        charpos: i64,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
    ) -> BufferEndOfBufferTailRenderRequest<'a> {
        BufferEndOfBufferTailRenderRequest::new(BufferEndOfBufferTailRenderContext::new(
            byte_idx,
            charpos,
            self.accessible_end,
            self.point_charpos,
            overlay_context,
            active_face_state,
        ))
    }

    pub(crate) fn buffer_id(self) -> BufferId {
        self.buffer_id
    }

    pub(crate) fn text_start_byte(self) -> usize {
        self.text_start_byte
    }

    pub(crate) fn content_x(self) -> f32 {
        self.content_x
    }

    pub(crate) fn char_height(self) -> f32 {
        self.char_height
    }

    pub(crate) fn point_charpos(self) -> i64 {
        self.point_charpos
    }

    pub(crate) fn row_visibility_limit(self) -> DisplayRowVisibilityLimit {
        self.row_visibility_limit
    }

    #[cfg(test)]
    pub(crate) fn accessible_end(self) -> i64 {
        self.accessible_end
    }

    #[cfg(test)]
    pub(crate) fn selective_display(self) -> i32 {
        self.selective_display
    }

    #[cfg(test)]
    pub(crate) fn tab_width(self) -> i32 {
        self.tab_width
    }

    #[cfg(test)]
    pub(crate) fn row_limit(self) -> DisplayRowLimit {
        self.row_limit
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

impl<'a, B> BufferTextWindowRenderRequest<'a, B>
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
        context: BufferTextWindowRenderAttemptContext<'_, '_>,
        text_buf: &mut Vec<u8>,
        remaining_visibility_retries: usize,
    ) -> BufferTextWindowRenderAttemptOutcome {
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
        let local_display_policy = BufferTextWindowLocalDisplayPolicy::from_buffer(buffer);

        let default_face = BufferTextWindowDefaultFacePlan::new(
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
        let chrome_heights = BufferTextWindowChromeHeights::new(
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
        let BufferTextWindowGeometryPlan {
            geometry,
            line_number_columns,
        } = BufferTextWindowGeometryRequest::new(
            params,
            char_w,
            char_h,
            chrome_plan.mode_line_height(),
            chrome_plan.header_line_height(),
            chrome_plan.tab_line_height(),
        )
        .with_max_mini_window_rows(max_mini_window_rows)
        .into_window_plan(&local_display_policy, &buf_access);

        let text_source = BufferTextWindowSourceReadRequest::new(params, geometry.max_rows)
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
            return BufferTextWindowRenderAttemptOutcome::Skipped;
        }

        let reserve_right_special_col =
            !frame_params.window_system && params.right_fringe_width == 0.0;
        let mut walk_setup = BufferTextWindowWalkSetupRequest::from_window_geometry(
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
        let output_setup = BufferTextWindowOutputSetupRequest::from_window_geometry(
            frame_id, window_id, params, &geometry,
        )
        .into_setup(geometry.max_rows, &walk_setup);

        let body_plan = output_setup.into_body_plan(
            &walk_setup,
            local_display_policy,
            line_number_columns,
            &geometry,
            chrome_heights,
            buffer,
            buffer_id,
            text_source,
            params,
            state.face_resolver,
            &default_face,
            font_ascent,
            frame_params.window_system,
            params.window_id as u64,
            &text_append_surface,
            reserve_right_special_col,
            reserve_right_border_col,
        );
        body_plan.render_attempt(
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
            text,
            params,
            buffer,
            &buf_access,
        )
    }
}

impl<'emit, 'face> BufferTextWindowRenderedBodyCompleteState<'emit, 'face> {
    pub(crate) fn install_body_and_publish_redisplay(
        &mut self,
        output_emitter: &mut WindowOutputEmitter,
        walk_setup: &mut BufferTextWindowWalkSetup,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
        publish_request: BufferTextWindowRedisplayPublishRequest,
    ) -> TextWindowRedisplayPositions {
        let BufferTextWindowOutputState { output, evaluator } = &mut self.output;
        walk_setup.install_body_and_publish_redisplay(
            BufferTextWindowBodyInstallPublishState::new(
                output.reborrow(),
                output_emitter,
                evaluator,
                self.render_services.reborrow(),
            ),
            tail_context,
            publish_request,
        )
    }

    pub(crate) fn render_chrome_rows(
        &mut self,
        output_emitter: &mut WindowOutputEmitter,
        request: WindowChromeRowsRenderRequest<'_, '_>,
    ) {
        let BufferTextWindowOutputState { output, evaluator } = &mut self.output;
        render_window_chrome_rows(
            output.reborrow(),
            output_emitter,
            evaluator,
            request,
            self.render_services.reborrow(),
        );
    }
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
        state: BufferTextWindowBodyInstallRenderState<'_, '_, '_>,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
    ) -> TextWindowRedisplayPositions {
        tail_context
            .body_install_request(self.byte_idx, &self.row_flags)
            .install_and_apply(BufferTextWindowBodyInstallState::new(
                state.output,
                state.output_emitter,
                state.render_services,
            ))
    }

    fn install_body_and_publish_redisplay(
        &mut self,
        state: BufferTextWindowBodyInstallPublishState<'_, '_, '_>,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
        publish_request: BufferTextWindowRedisplayPublishRequest,
    ) -> TextWindowRedisplayPositions {
        let BufferTextWindowBodyInstallPublishState {
            output,
            output_emitter,
            evaluator,
            render_services,
        } = state;
        let redisplay_positions = self.install_body(
            BufferTextWindowBodyInstallRenderState::new(output, output_emitter, render_services),
            tail_context,
        );
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
    fn begin_render_body_and_tail<'request, 'buf, B: LayoutBufferView>(
        &mut self,
        begin_request: BufferTextWindowBeginRequest,
        state: &mut BufferTextWindowBodyPassState<'_>,
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
    ) -> BufferTextWindowBodyPassOutcome {
        let mut output_emitter = state.output.begin_text_window_output(begin_request);
        let post_loop = self.render_body_and_tail(
            &mut BufferTextWindowBodyRenderState::new(
                state.output.source_render_state(&mut output_emitter),
                line_numbers,
                face_scan,
                active_face_state,
                state.face_ids,
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
        BufferTextWindowBodyPassOutcome {
            output_emitter,
            post_loop,
        }
    }

    fn finish_window_and_install(
        &mut self,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
        state: BufferTextWindowRenderedBodyFinishState<'_>,
        output_emitter: WindowOutputEmitter,
    ) {
        tail_context.finish_and_install(
            state.finish_install_state(output_emitter, std::mem::take(&mut self.hit_rows)),
        );
    }
}

impl BufferTextWindowOutputSetupRequest {
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
    ) -> Self {
        Self {
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
        }
    }

    pub(crate) fn from_window_geometry(
        frame_id: FrameId,
        window_id: WindowId,
        params: &WindowParams,
        geometry: &BufferTextWindowGeometry,
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
        )
    }

    pub(crate) fn into_setup(
        self,
        max_rows: usize,
        walk_setup: &BufferTextWindowWalkSetup,
    ) -> BufferTextWindowOutputSetup {
        let output_cols = self.cols.max(1);
        BufferTextWindowOutputSetup {
            begin_request: BufferTextWindowBeginRequest::new(
                self.frame_id,
                self.window_id,
                self.display_text_row_base,
                walk_setup.text_area_left,
                walk_setup.window_top,
                self.output_window_id,
                self.display_text_row_base + self.display_text_rows + self.bottom_chrome_rows,
                output_cols,
                self.bounds,
                self.text_bounds,
                self.selected,
                walk_setup.row_geometry.display_text_row_begin(
                    self.display_text_row_base,
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
                bottom_y: self.visibility_bottom_y,
            },
            row_limit: DisplayRowLimit { max_rows },
            body_install_context: BufferTextWindowBodyInstallContext::new(
                self.output_window_id,
                self.display_text_row_base,
                output_cols,
            ),
            retry_bounds: BufferTextWindowRetryBounds::new(
                (self.text_y - walk_setup.window_top).round() as i64,
                (self.text_y + self.text_height - walk_setup.window_top).round() as i64,
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
            foreground: Color::from_pixel(face.fg),
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

    pub(crate) fn foreground(&self) -> Color {
        self.foreground
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
    fn into_body_plan<'a, 'surface, B>(
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
        let render_contexts = BufferTextWindowRenderContextsRequest::new(
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
            output_window_id,
            append_surface,
            geometry.text_y,
            self.body_install_context.display_text_row_base(),
            geometry.max_rows,
        )
        .into_contexts();
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
        let initial_face_state = BufferTextWindowInitialFaceStateRequest::new(
            default_face.measurement_policy(),
            default_face.face(),
            default_face.char_width(),
            DisplayRowFallbackMetrics::from_default_face_extents(
                default_face.char_width(),
                default_face.row_height(),
                default_face.ascent(),
            ),
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
            geometry.cols,
            geometry.char_width,
            geometry.char_height,
            default_face.foreground(),
            geometry.max_rows,
            self.row_limit,
            walk_setup.row_geometry_defaults,
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
            initial_face_state,
            row_prelude_context,
            loop_context,
            render_contexts,
            tail_context,
        }
    }
}

impl<'a, 'surface, B> BufferTextWindowBodyPlan<'a, 'surface, B>
where
    B: LayoutBufferView,
{
    #[allow(clippy::too_many_arguments)]
    fn render_attempt<'buf>(
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
            output,
            font_metrics,
            face_resolver,
            frame_face_id_counter,
            hit_data,
            display_snapshots,
        } = state;
        let mut output_session = BufferTextWindowOutputSession::from_output_state(
            output,
            font_metrics,
            face_resolver,
            *frame_face_id_counter,
        );
        let rendered_body = self.begin_render_body_and_tail(
            walk_setup,
            &mut output_session,
            text,
            params,
            buffer,
            buf_access,
        );
        rendered_body.finish_or_prepare_retry(
            walk_setup,
            chrome_request,
            &mut output_session,
            hit_data,
            display_snapshots,
            frame_face_id_counter,
            remaining_visibility_retries,
        )
    }

    fn begin_render_body_and_tail<'buf>(
        self,
        walk_setup: &mut BufferTextWindowWalkSetup,
        output_session: &mut BufferTextWindowOutputSession<'_>,
        text: &'a [u8],
        params: &'a WindowParams,
        buffer: &B,
        buf_access: &RustBufferAccess<'buf, B>,
    ) -> BufferTextWindowRenderedBody<'a> {
        let BufferTextWindowRenderContexts {
            face_resolution,
            overlay_text_row,
        } = self.render_contexts;
        let mut line_numbers = self.local_display_policy.initial_line_numbers(
            buf_access,
            self.tail_context.window_start,
            self.loop_context.point_charpos(),
        );
        let mut face_scan = FaceScanCheckpoint::initial();
        let mut active_face_state =
            output_session.initial_active_face_state(self.initial_face_state);
        let mut body_pass_state = output_session.body_pass_state();
        let BufferTextWindowBodyPassOutcome {
            output_emitter,
            post_loop,
        } = walk_setup.begin_render_body_and_tail(
            self.begin_request,
            &mut body_pass_state,
            &mut line_numbers,
            &mut face_scan,
            &mut active_face_state,
            self.row_prelude_context,
            self.loop_context,
            face_resolution,
            &self.tail_context,
            text,
            params,
            overlay_text_row,
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

    pub(crate) fn install_body_and_publish_redisplay(
        &mut self,
        walk_setup: &mut BufferTextWindowWalkSetup,
        state: &mut BufferTextWindowRenderedBodyCompleteState<'_, '_>,
    ) -> TextWindowRedisplayPositions {
        state.install_body_and_publish_redisplay(
            &mut self.output_emitter,
            walk_setup,
            &self.tail_context,
            self.publish_request,
        )
    }

    pub(crate) fn render_chrome_rows(
        &mut self,
        request: WindowChromeRowsRenderRequest<'_, '_>,
        state: &mut BufferTextWindowRenderedBodyCompleteState<'_, '_>,
    ) {
        state.render_chrome_rows(&mut self.output_emitter, request);
    }

    pub(crate) fn finish_window_and_install(
        self,
        walk_setup: &mut BufferTextWindowWalkSetup,
        state: BufferTextWindowRenderedBodyFinishState<'_>,
    ) {
        walk_setup.finish_window_and_install(&self.tail_context, state, self.output_emitter);
    }

    fn install_body_chrome_and_finish(
        mut self,
        walk_setup: &mut BufferTextWindowWalkSetup,
        chrome_request: WindowChromeRowsRenderRequest<'_, '_>,
        output_session: &mut BufferTextWindowOutputSession<'_>,
        hit_data: &mut Vec<WindowHitData>,
        display_snapshots: &mut Vec<WindowDisplaySnapshot>,
    ) -> TextWindowRedisplayPositions {
        let mut state = output_session.rendered_body_complete_state(hit_data, display_snapshots);
        let redisplay_positions = self.install_body_and_publish_redisplay(walk_setup, &mut state);
        self.render_chrome_rows(chrome_request, &mut state);
        self.finish_window_and_install(walk_setup, state.finish_state());
        redisplay_positions
    }

    fn finish_or_prepare_retry(
        self,
        walk_setup: &mut BufferTextWindowWalkSetup,
        chrome_request: WindowChromeRowsRenderRequest<'_, '_>,
        output_session: &mut BufferTextWindowOutputSession<'_>,
        hit_data: &mut Vec<WindowHitData>,
        display_snapshots: &mut Vec<WindowDisplaySnapshot>,
        frame_face_id_counter: &mut u32,
        remaining_visibility_retries: usize,
    ) -> BufferTextWindowRenderAttemptOutcome {
        let retry_plan = self.retry_plan(walk_setup);
        retry_plan.log_visibility_adjustments();

        if let Some(window_start) = retry_plan.should_retry(remaining_visibility_retries) {
            retry_plan.log_retry(window_start, remaining_visibility_retries);
            output_session.prepare_retry(frame_face_id_counter);
            return BufferTextWindowRenderAttemptOutcome::Retry { window_start };
        }

        let redisplay_positions = self.install_body_chrome_and_finish(
            walk_setup,
            chrome_request,
            output_session,
            hit_data,
            display_snapshots,
        );
        output_session.publish_face_ids(frame_face_id_counter);
        BufferTextWindowRenderAttemptOutcome::Finished {
            redisplay_positions,
        }
    }
}

impl<'a, 'surface, B> BufferTextWindowRenderContextsRequest<'a, 'surface, B>
where
    B: LayoutBufferView,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        buffer: &'a B,
        face_resolver: &'a FaceResolver,
        measurement_policy: DisplayRowMeasurementPolicy,
        default_resolved: &'a ResolvedFace,
        default_face_char_width: f32,
        default_face_ascent: f32,
        default_face_height: f32,
        char_width: f32,
        char_height: f32,
        font_ascent: f32,
        window_system: bool,
        output_window_id: u64,
        append_surface: &'surface DisplayRowAppendSurface,
        text_y: f32,
        display_text_row_base: usize,
        max_rows: usize,
    ) -> Self {
        Self {
            buffer,
            face_resolver,
            measurement_policy,
            default_resolved,
            default_face_char_width,
            default_face_ascent,
            default_face_height,
            char_width,
            char_height,
            font_ascent,
            window_system,
            output_window_id,
            append_surface,
            text_y,
            display_text_row_base,
            max_rows,
        }
    }

    pub(crate) fn into_contexts(self) -> BufferTextWindowRenderContexts<'a, 'surface, B> {
        let has_overlays = !self.buffer.layout_overlays().is_empty();
        BufferTextWindowRenderContexts {
            face_resolution: BufferCurrentFaceResolutionContext::new(
                self.buffer,
                self.face_resolver,
                self.measurement_policy,
                self.default_resolved,
                self.default_face_char_width,
                self.default_face_ascent,
                self.default_face_height,
                self.char_width,
                self.char_height,
                self.font_ascent,
                self.window_system,
            ),
            overlay_text_row: BufferOverlayStringTextRowRenderContext::new(
                has_overlays,
                self.output_window_id,
                self.append_surface,
                self.char_height,
                self.default_face_ascent,
                self.text_y,
                self.display_text_row_base,
                self.max_rows,
            ),
        }
    }
}

impl<'a> BufferTextWindowInitialFaceStateRequest<'a> {
    pub(crate) fn new(
        measurement_policy: DisplayRowMeasurementPolicy,
        default_resolved: &'a ResolvedFace,
        default_face_char_width: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
    ) -> Self {
        Self {
            measurement_policy,
            default_resolved,
            default_face_char_width,
            fallback_metrics,
        }
    }

    pub(crate) fn into_active_face_state(
        self,
        font_metrics: &mut Option<FontMetricsService>,
    ) -> DisplayRowActiveFaceState {
        let default_measured_face = self.measurement_policy.measured_face(
            neomacs_display_protocol::face::BasicFaceId::Default.into(),
            self.default_resolved,
            None,
            self.default_face_char_width,
            self.fallback_metrics,
            font_metrics,
        );
        DisplayRowActiveFaceState::new(self.default_resolved.clone(), default_measured_face)
    }
}

impl<'rows, 'emit, 'surface> BufferTextWindowLoopRenderState<'rows, 'emit, 'surface> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        loop_context: BufferTextWindowLoopRequestContext,
        append_state: &'emit mut BufferTextRowAppendState,
        invisible_text_checkpoint: &'emit mut InvisibleTextScanCheckpoint,
        progress: BufferTextWindowProgressState<'emit>,
        source_render: TextRowSourceRenderState<'emit>,
        row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &'emit mut BoxFaceRowState,
        line_numbers: &'emit mut LineNumberRenderState,
        row_geometry: &'emit mut DisplayRowGeometryState,
        row_flags: &'emit mut DisplayRowFlags,
        hit_rows: &'emit mut Vec<HitRow>,
        hit_row_range: &'emit mut HitRowRangeTracker,
        prefix_request: &'emit mut DisplayRowPrefixRequest,
        hscroll_skip: &'emit mut HorizontalScrollSkipState,
        word_wrap: &'emit mut WordWrapRenderState,
        trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
        face_scan: &'emit mut FaceScanCheckpoint,
        row_y_positions: &'rows mut DisplayRowYPositions,
        cursor_info: &'emit mut CursorCaptureState,
        face_ids: &'emit mut FrameFaceIdAllocator,
        append_surface: &'surface DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'surface>,
    ) -> Self {
        Self {
            loop_context,
            append_state,
            invisible_text_checkpoint,
            progress,
            source_render,
            row_extend,
            box_face,
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
            append_surface,
            overlay_context,
        }
    }

    fn hscroll_should_skip(&self) -> bool {
        self.hscroll_skip.should_skip()
    }

    fn render_visible_steps<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'request, B>,
        row_prelude_context: BufferTextWindowRowPreludeRequestContext,
        face_resolution_context: BufferCurrentFaceResolutionContext<'request, B>,
        text: &'request [u8],
        params: &'request WindowParams,
        active_face_state: &mut DisplayRowActiveFaceState,
        buffer: &B,
    ) where
        'surface: 'request,
    {
        while *self.progress.byte_idx < text.len()
            && self
                .row_geometry
                .current_row_is_visible(self.loop_context.row_visibility_limit())
        {
            if matches!(
                self.render_next_step(
                    source_walk,
                    row_prelude_context,
                    face_resolution_context.clone(),
                    text,
                    params,
                    active_face_state,
                    buffer,
                ),
                BufferTextWindowLoopStepOutcome::StopBufferWalk
            ) {
                break;
            }
        }
    }

    fn render_next_step<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'request, B>,
        row_prelude_context: BufferTextWindowRowPreludeRequestContext,
        face_resolution_context: BufferCurrentFaceResolutionContext<'request, B>,
        text: &'request [u8],
        params: &'request WindowParams,
        active_face_state: &mut DisplayRowActiveFaceState,
        buffer: &B,
    ) -> BufferTextWindowLoopStepOutcome
    where
        'surface: 'request,
    {
        match self.render_pre_source_checkpoints_for_context(
            source_walk,
            row_prelude_context,
            face_resolution_context,
            text,
            params,
            active_face_state,
            buffer,
        ) {
            BufferTextWindowPreSourceOutcome::ReadyForSourceItem => {}
            BufferTextWindowPreSourceOutcome::ContinueBufferWalk => {
                return BufferTextWindowLoopStepOutcome::ContinueBufferWalk;
            }
            BufferTextWindowPreSourceOutcome::StopBufferWalk => {
                return BufferTextWindowLoopStepOutcome::StopBufferWalk;
            }
        }

        let Some(source_consumption) =
            self.consume_source_item(source_walk, face_resolution_context)
        else {
            return BufferTextWindowLoopStepOutcome::StopBufferWalk;
        };

        match source_consumption {
            BufferTextSourceConsumptionItem::DisplayItem(source_item) => {
                return self.render_consumed_display_item_for_context(
                    source_walk,
                    BufferTextWindowConsumedDisplayItemRenderRequest {
                        layout_resolution_context: face_resolution_context
                            .source_item_layout_resolution_context(),
                        source_item,
                        text,
                        active_face_state,
                        params,
                    },
                    buffer,
                );
            }
            BufferTextSourceConsumptionItem::Replacement(replacement) => {
                return self.render_replacement_source_item_for_context(
                    replacement,
                    source_walk,
                    face_resolution_context.source_item_layout_resolution_context(),
                    text,
                    active_face_state,
                    params,
                    buffer,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_pre_source_checkpoints_for_context<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        row_prelude_context: BufferTextWindowRowPreludeRequestContext,
        face_resolution_context: BufferCurrentFaceResolutionContext<'request, B>,
        text: &'request [u8],
        _params: &'request WindowParams,
        active_face_state: &mut DisplayRowActiveFaceState,
        buffer: &B,
    ) -> BufferTextWindowPreSourceOutcome
    where
        'surface: 'request,
    {
        self.render_row_prelude(row_prelude_context, active_face_state, buffer);

        if self
            .render_invisible_text_for_context(source_walk, text, active_face_state, buffer)
            .should_continue_buffer_walk()
        {
            return BufferTextWindowPreSourceOutcome::ContinueBufferWalk;
        }

        if self.hscroll_should_skip() {
            if self
                .render_hscroll_skip_for_context(source_walk, text, active_face_state)
                .should_break()
            {
                return BufferTextWindowPreSourceOutcome::StopBufferWalk;
            }
            return BufferTextWindowPreSourceOutcome::ContinueBufferWalk;
        }

        self.render_face_checkpoint_for_context(face_resolution_context, active_face_state);

        BufferTextWindowPreSourceOutcome::ReadyForSourceItem
    }

    fn render_consumed_display_item_for_context<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        request: BufferTextWindowConsumedDisplayItemRenderRequest<'request>,
        buffer: &B,
    ) -> BufferTextWindowLoopStepOutcome
    where
        'surface: 'request,
    {
        let BufferTextWindowConsumedDisplayItemRenderRequest {
            layout_resolution_context,
            source_item,
            text,
            active_face_state,
            params,
        } = request;
        let selective_display_outcome = self.render_selective_display_tail_for_context(
            source_walk,
            source_item.source_char(),
            text,
            active_face_state,
            buffer,
        );
        if selective_display_outcome.should_break() {
            return BufferTextWindowLoopStepOutcome::StopBufferWalk;
        }
        if selective_display_outcome.should_continue_buffer_walk() {
            return BufferTextWindowLoopStepOutcome::ContinueBufferWalk;
        }

        let is_explicit_line_break = source_item.is_explicit_line_break();
        let end_charpos = source_item.end_charpos();
        let source_char = source_item.source_char();
        if is_explicit_line_break {
            if self
                .render_line_break_for_context(
                    source_walk,
                    source_char,
                    text,
                    active_face_state,
                    buffer,
                )
                .should_break()
            {
                return BufferTextWindowLoopStepOutcome::StopBufferWalk;
            }
        } else {
            let char_render_outcome = self.render_text_consumed_display_item_for_context(
                source_walk,
                layout_resolution_context,
                source_item,
                text,
                active_face_state,
                params,
                buffer,
            );
            if char_render_outcome.should_break() {
                return BufferTextWindowLoopStepOutcome::StopBufferWalk;
            }
            if char_render_outcome.should_continue_buffer_walk() {
                return BufferTextWindowLoopStepOutcome::ContinueBufferWalk;
            }
            *self.progress.charpos = (*self.progress.charpos).max(end_charpos);
        }

        BufferTextWindowLoopStepOutcome::ContinueBufferWalk
    }

    fn render_replacement_source_item_for_context<'request, B: LayoutBufferView>(
        &mut self,
        replacement: BufferTextReplacementItem,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'request>,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
        params: &'request WindowParams,
        buffer: &B,
    ) -> BufferTextWindowLoopStepOutcome
    where
        'surface: 'request,
    {
        let request = BufferDisplayPropertyTextReplacementResolveRequest::new(
            replacement,
            self.loop_context.text_start_byte(),
            text,
            self.loop_context.content_x(),
            params,
            0.0,
            self.loop_context.char_height(),
            active_face_state,
            self.loop_context.point_charpos(),
        );
        let resolve_outcome =
            self.source_render
                .with_font_metrics_and_display_host(|font_metrics, host| {
                    request.resolve(
                        font_metrics,
                        host,
                        *self.progress.row.x,
                        self.progress.row_position(),
                    )
                });
        match resolve_outcome {
            BufferDisplayPropertyTextReplacementResolveOutcome::Resolved(request) => {
                let update = request.render_and_apply(
                    buffer,
                    BufferDisplayPropertyTextReplacementRenderState::new(
                        text,
                        self.source_render.reborrow(),
                        self.face_ids,
                        self.append_surface,
                        self.row_geometry,
                        self.cursor_info,
                        active_face_state,
                        self.progress.reborrow(),
                    ),
                );
                source_walk
                    .commit_display_property_replacement(update)
                    .apply_to_progress(&mut self.progress);
                BufferTextWindowLoopStepOutcome::ContinueBufferWalk
            }
            BufferDisplayPropertyTextReplacementResolveOutcome::Fallback(source_item) => {
                let Some(source_step) = source_walk
                    .consume_fallback_source_item(source_item, self.progress.source_position())
                    .apply_to_progress(&mut self.progress)
                else {
                    return BufferTextWindowLoopStepOutcome::StopBufferWalk;
                };
                self.render_consumed_display_item_for_context(
                    source_walk,
                    BufferTextWindowConsumedDisplayItemRenderRequest {
                        layout_resolution_context,
                        source_item: source_step,
                        text,
                        active_face_state,
                        params,
                    },
                    buffer,
                )
            }
            BufferDisplayPropertyTextReplacementResolveOutcome::Stop => {
                BufferTextWindowLoopStepOutcome::StopBufferWalk
            }
        }
    }

    pub(crate) fn render_row_prelude<B: LayoutBufferView>(
        &mut self,
        context: BufferTextWindowRowPreludeRequestContext,
        active_face_state: &DisplayRowActiveFaceState,
        buffer: &B,
    ) {
        context
            .line_number_margin_request()
            .render_pending_with_source_state(
                self.line_numbers,
                &mut self.source_render,
                self.face_ids,
                self.row_geometry,
                self.face_scan,
                context.char_width(),
            );

        context
            .line_prefix_request(
                self.append_surface,
                self.row_geometry,
                active_face_state,
                0.0,
                self.progress.row_position(),
            )
            .render_requested_with_source_state_and_apply(
                self.prefix_request,
                &mut self.source_render,
                buffer,
                self.progress.charpos(),
                self.face_ids,
                self.progress.row.x,
                self.progress.row.col,
            );
    }

    fn consume_source_item<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        face_resolution_context: BufferCurrentFaceResolutionContext<'_, B>,
    ) -> Option<BufferTextSourceConsumptionItem> {
        // One persistent typed source cursor feeds the row walk. The source
        // side owns pending text-run splitting so direct single-character
        // items can still stay typed through render.
        let (source_item, pending_faces) = source_walk
            .consume_source_item(
                self.progress.source_position(),
                face_resolution_context,
                self.face_ids,
            )
            .apply_to_progress(&mut self.progress);
        {
            let mut source_render = self.source_render.reborrow();
            face_resolution_context.install_pending_source_faces(
                &mut source_render,
                self.row_geometry,
                pending_faces,
            );
        }
        source_item
    }

    fn render_invisible_text_for_context<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
        buffer: &B,
    ) -> BufferInvisibleTextRenderOutcome
    where
        'surface: 'request,
    {
        let request = self.loop_context.invisible_text_request(
            text,
            self.append_surface,
            self.overlay_context,
            active_face_state,
            0.0,
        );
        self.render_invisible_text_at_checkpoint(source_walk, request, buffer)
    }

    fn render_hscroll_skip_for_context<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
    ) -> DisplayRowTransitionContinuation
    where
        'surface: 'request,
    {
        let request =
            self.loop_context
                .hscroll_skip_request(text, self.append_surface, active_face_state);
        self.render_hscroll_skip(source_walk, request)
    }

    fn render_face_checkpoint_for_context<B: LayoutBufferView>(
        &mut self,
        face_resolution_context: BufferCurrentFaceResolutionContext<'_, B>,
        active_face_state: &mut DisplayRowActiveFaceState,
    ) {
        face_resolution_context.resolve_at_checkpoint_with_source_state(
            &mut self.source_render.reborrow(),
            self.face_scan,
            self.face_ids,
            active_face_state,
            self.row_geometry,
            self.row_extend,
            self.box_face,
            *self.progress.row.x,
            self.progress.charpos(),
        );
    }

    fn render_selective_display_tail_for_context<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        source_step_char: BufferTextSourceStepChar,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
        buffer: &B,
    ) -> BufferSelectiveDisplayTailRenderOutcome
    where
        'surface: 'request,
    {
        let request = self.loop_context.selective_display_tail_request(
            source_step_char,
            text,
            self.append_surface,
            active_face_state,
            0.0,
        );
        self.render_selective_display_tail(source_walk, request, buffer)
    }

    pub(crate) fn render_line_break_for_context<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        source_char: BufferTextSourceStepChar,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
        buffer: &B,
    ) -> DisplayRowTransitionContinuation
    where
        'surface: 'request,
    {
        let request = self.loop_context.line_break_request(
            source_char,
            text,
            self.overlay_context,
            active_face_state,
        );
        self.render_line_break(source_walk, request, buffer)
    }

    fn render_text_consumed_display_item_for_context<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'request>,
        source_item: BufferTextConsumedDisplayItem,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
        params: &'request WindowParams,
        buffer: &B,
    ) -> BufferTextConsumedDisplayItemRenderOutcome
    where
        'surface: 'request,
    {
        let request = self.loop_context.consumed_display_item_request(
            layout_resolution_context,
            source_item,
            text,
            self.append_surface,
            self.overlay_context,
            active_face_state,
            params,
            0.0,
        );
        self.render_text_consumed_display_item(source_walk, request, buffer)
    }

    fn render_invisible_text_at_checkpoint<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        request: BufferInvisibleTextRenderRequest<'_>,
        buffer: &B,
    ) -> BufferInvisibleTextRenderOutcome {
        request.render_at_checkpoint_and_apply(
            source_walk,
            buffer,
            BufferInvisibleTextRenderRequestState::new(
                self.invisible_text_checkpoint,
                self.progress.reborrow(),
                self.source_render.reborrow(),
                self.row_geometry,
                self.cursor_info,
                self.hit_rows,
                self.hit_row_range,
                self.row_y_positions,
                self.face_ids,
            ),
        )
    }

    fn render_hscroll_skip<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        request: BufferHscrollSkipRenderRequest<'_>,
    ) -> DisplayRowTransitionContinuation {
        request.render_next_and_apply(
            source_walk,
            BufferHscrollSkipRenderState::new(
                self.progress.reborrow(),
                self.hscroll_skip,
                self.row_extend,
                self.source_render.reborrow(),
                self.prefix_request,
                self.line_numbers,
                self.word_wrap,
                self.trailing_whitespace,
                self.row_geometry,
                self.row_flags,
                self.hit_rows,
                self.hit_row_range,
                self.cursor_info,
                self.row_y_positions,
            ),
        )
    }

    fn render_selective_display_tail<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        request: BufferSelectiveDisplayTailRenderRequest<'_>,
        buffer: &B,
    ) -> BufferSelectiveDisplayTailRenderOutcome {
        request.render_if_needed_and_apply(
            source_walk,
            buffer,
            BufferSelectiveDisplayTailRenderState::new(
                self.progress.reborrow(),
                self.source_render.reborrow(),
                self.row_extend,
                self.box_face,
                self.line_numbers,
                self.row_geometry,
                self.row_flags,
                self.hit_rows,
                self.hit_row_range,
                self.prefix_request,
                self.hscroll_skip,
                self.word_wrap,
                self.trailing_whitespace,
                self.row_y_positions,
            ),
        )
    }

    fn render_line_break<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        request: BufferTextLineBreakRenderRequest<'_>,
        buffer: &B,
    ) -> DisplayRowTransitionContinuation {
        request.render_and_apply(
            source_walk,
            buffer,
            BufferTextLineBreakRenderState::new(
                self.progress.reborrow(),
                self.cursor_info,
                self.row_geometry,
                self.trailing_whitespace,
                self.row_extend,
                self.box_face,
                self.source_render.reborrow(),
                self.prefix_request,
                self.line_numbers,
                self.hscroll_skip,
                self.word_wrap,
                self.row_flags,
                self.hit_rows,
                self.hit_row_range,
                self.row_y_positions,
                self.face_ids,
            ),
        )
    }

    fn render_text_consumed_display_item<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'_, B>,
        request: BufferTextConsumedDisplayItemRenderRequest<'_>,
        buffer: &B,
    ) -> BufferTextConsumedDisplayItemRenderOutcome {
        request.render_and_apply(
            source_walk,
            buffer,
            BufferTextConsumedDisplayItemRenderRequestState::new(
                self.append_state,
                self.progress.reborrow(),
                self.source_render.reborrow(),
                self.row_extend,
                self.line_numbers,
                self.row_geometry,
                self.row_flags,
                self.hit_rows,
                self.hit_row_range,
                self.prefix_request,
                self.hscroll_skip,
                self.word_wrap,
                self.trailing_whitespace,
                self.face_scan,
                self.row_y_positions,
                self.cursor_info,
                self.face_ids,
            ),
        )
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
        cols: usize,
        char_width: f32,
        char_height: f32,
        default_fg: Color,
        max_rows: usize,
        row_limit: DisplayRowLimit,
        row_geometry_defaults: DisplayRowGeometryDefaults,
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
            cols,
            char_width,
            char_height,
            default_fg,
            max_rows,
            row_limit,
            row_geometry_defaults,
            retry_bounds,
            body_install_context,
            reserve_right_special_col,
            reserve_right_border_col,
            mode_line_height,
            header_line_height,
            tab_line_height,
        }
    }

    pub(crate) fn tail_decoration_request(&self) -> BufferTextWindowTailDecorationRequest<'a> {
        BufferTextWindowTailDecorationRequest::new(
            self.params,
            self.content_x,
            self.cols,
            self.text_y,
            self.text_height,
            self.char_width,
            self.char_height,
            self.default_fg,
            self.max_rows,
            self.row_limit,
            self.row_geometry_defaults,
        )
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

    fn finish_and_install(&self, state: BufferTextWindowFinishInstallState<'_>) {
        let finished_window = self
            .finish_request()
            .finish_and_snapshot(state.finish_state);
        state.hit_data.push(finished_window.hit_data);
        state.display_snapshots.push(finished_window.snapshot);
    }

    #[cfg(test)]
    pub(crate) fn window_start(&self) -> i64 {
        self.window_start
    }

    #[cfg(test)]
    pub(crate) fn accessible_range(&self) -> (i64, i64) {
        (self.accessible_start, self.accessible_end)
    }
}

impl<'rows, 'emit, 'surface> BufferTextWindowPostLoopState<'rows, 'emit, 'surface> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        loop_context: BufferTextWindowLoopRequestContext,
        source_render: TextRowSourceRenderState<'emit>,
        row_progress: BufferTextWindowRowProgressState<'emit>,
        row_geometry: &'emit mut DisplayRowGeometryState,
        cursor_info: &'emit mut CursorCaptureState,
        hit_rows: &'emit mut Vec<HitRow>,
        hit_row_range: &'emit mut HitRowRangeTracker,
        row_y_positions: &'rows mut DisplayRowYPositions,
        face_ids: &'emit mut FrameFaceIdAllocator,
        row_flags: &'emit DisplayRowFlags,
        row_extend: &'emit DisplayRowScopedValue<(Color, u32)>,
        box_face: &'emit BoxFaceRowState,
        text_append_surface: &'surface DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'surface>,
    ) -> Self {
        Self {
            loop_context,
            source_render,
            row_progress,
            row_geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
            row_flags,
            row_extend,
            box_face,
            text_append_surface,
            overlay_context,
        }
    }

    pub(crate) fn render_end_of_buffer_tail<'request, B: LayoutBufferView>(
        &mut self,
        byte_idx: usize,
        charpos: i64,
        active_face_state: &'request DisplayRowActiveFaceState,
        buffer: &B,
    ) -> bool
    where
        'surface: 'request,
    {
        self.loop_context
            .end_of_buffer_tail_request(byte_idx, charpos, self.overlay_context, active_face_state)
            .render_and_apply(
                buffer,
                BufferEndOfBufferTailRenderState::new(
                    self.source_render.reborrow(),
                    self.row_progress.reborrow(),
                    self.row_geometry,
                    self.cursor_info,
                    self.hit_rows,
                    self.hit_row_range,
                    self.row_y_positions,
                    self.face_ids,
                ),
            )
            .point_is_visible_eob()
    }

    pub(crate) fn apply_tail_decorations(
        &self,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
    ) -> BufferTextWindowTailDecorationOutcome {
        tail_context
            .tail_decoration_request()
            .apply(BufferTextWindowTailDecorationState::new(
                *self.row_progress.x,
                self.text_append_surface,
                self.row_geometry,
                self.row_y_positions,
                self.row_flags,
                self.row_extend,
                self.box_face,
            ))
    }

    pub(crate) fn finalize_tail(
        &mut self,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
        text: &[u8],
        charpos: i64,
        point_is_visible_eob: bool,
    ) -> BufferTextWindowTailFinalizeOutcome {
        tail_context
            .tail_finalize_request(text, charpos, point_is_visible_eob)
            .finalize_and_apply(BufferTextWindowTailFinalizeState::new(
                self.cursor_info,
                self.row_geometry,
                self.row_y_positions,
                self.hit_row_range,
                self.hit_rows,
                self.source_render.output_render(),
            ))
    }

    pub(crate) fn decide_visibility_retry<'buf, B: LayoutBufferView>(
        &self,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
        charpos: i64,
        point_is_visible_eob: bool,
        buf_access: &'rows RustBufferAccess<'buf, B>,
    ) -> BufferTextWindowVisibilityRetryOutcome {
        tail_context
            .visibility_retry_request(
                self.source_render.output_rows(),
                charpos,
                point_is_visible_eob,
                buf_access,
            )
            .decide()
    }

    pub(crate) fn rendered_rows_len(&self) -> usize {
        self.source_render.output_rows_len()
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_tail_and_decide_retry<'request, 'buf, B: LayoutBufferView>(
        &mut self,
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
        let point_is_visible_eob =
            self.render_end_of_buffer_tail(byte_idx, charpos, active_face_state, buffer);

        self.apply_tail_decorations(tail_context);
        self.finalize_tail(tail_context, text, charpos, point_is_visible_eob);

        // GNU redisplay keeps iterating until point visibility converges or no
        // further progress can be made. Advance by actual rendered row spans
        // from this pass, since wrapped and variable-height lines are exactly
        // where newline-based retry selection goes wrong.
        let retry =
            self.decide_visibility_retry(tail_context, charpos, point_is_visible_eob, buf_access);
        BufferTextWindowPostLoopRenderOutcome {
            retry,
            rendered_rows_len: self.rendered_rows_len(),
        }
    }
}
