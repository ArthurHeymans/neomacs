//! Buffer-text source rendering requests and actions.

use crate::coords::layout_i64_char_pos_to_lisp_char_pos;
use crate::display_buffer_text_append::{
    BufferTextWindowBeginRequest, BufferTextWindowBeginState,
    BufferTextWindowBodyInstallRenderContext, BufferTextWindowBodyInstallRequest,
    BufferTextWindowBodyInstallState, BufferTextWindowFinishRequest, BufferTextWindowFinishState,
    BufferTextWindowTailFinalizeContext, BufferTextWindowTailFinalizeOutcome,
    BufferTextWindowTailFinalizeRequest, BufferTextWindowTailFinalizeState,
    BufferTextWindowVisibilityRetryOutcome, BufferTextWindowVisibilityRetryRequest,
    TextWindowAppendSurfaceRequest,
};
use crate::display_buffer_text_item_append::{
    BufferTextPreparedSourceCharAppend, BufferTextRowAppendContext, BufferTextRowAppendState,
    BufferTextSourceCharPreparationState, BufferTextSourceCharPreparedAppend,
    BufferTextSourceDisplayItemPreparationRequest, BufferTextSpecialSourceCharPreparedAppend,
};
use crate::display_buffer_text_source::{
    BufferTextConsumedSourceItem, BufferTextReplacementItem, BufferTextSourceCursor,
    BufferTextSourceItem, BufferTextSourceItemStep, BufferTextSourceItemStepper,
    BufferTextSourceStepChar, BufferTextWindowSource,
};
use crate::display_buffer_text_walk::{
    BufferTextWindowChromeHeights, BufferTextWindowGeometry, BufferTextWindowLocalDisplayPolicy,
};
use crate::display_cursor::{
    CapturedCursorInfo, CapturedCursorPlacement, CapturedCursorSlotWidth, CursorCaptureState,
    capture_cursor_info,
};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_face_layout::{DisplayHeightFaceBasis, height_adjusted_face};
use crate::display_item::{DisplayItem, RenderFaceRef};
use crate::display_origin::DisplayOrigin;
use crate::display_property::classify_display_property;
use crate::display_row::{
    DisplayRowActiveFaceState, DisplayRowFallbackMetrics, DisplayRowMeasurementPolicy,
};
use crate::display_row_append_context::{
    DisplayRowActiveFaceAppendContext, DisplayRowAppendFrame, DisplayRowAppendSurface,
};
use crate::display_row_builder::{DisplayRowAppendProgress, DisplayRowPosition};
use crate::display_row_geometry::{
    DisplayRowFlagKind, DisplayRowFlags, DisplayRowGeometryDefaults, DisplayRowGeometryState,
    DisplayRowHitRange, DisplayRowLimit, DisplayRowScopedValue, DisplayRowTextPosition,
    DisplayRowVisibilityLimit, DisplayRowYPositions,
};
use crate::display_row_line_number_margin::BufferLineNumberMarginRenderRequest;
use crate::display_row_lisp_string::{
    DisplayRowPrefixRequest, DisplayRowPrefixValues, LispStringRowAppendContext,
};
use crate::display_row_overlay_string::{
    BufferOverlayStringTextRowRenderContext, OverlayStringRenderState,
};
use crate::display_row_replacement::DisplayPropertyReplacementAppendOutcome;
use crate::display_row_source_append::append_synthetic_text_to_display_row;
use crate::display_row_source_render::{TextRowOutputRenderState, TextRowSourceRenderState};
use crate::display_row_transition::{
    DisplayRowLineBreakTransitionPlan, DisplayRowOverflowTransitionPlan,
    DisplayRowTextWindowEmitContext, DisplayRowTransitionContinuation,
    DisplayRowTransitionRenderState,
};
use crate::display_row_walk_state::{
    BoxFaceRowState, BufferTextRowOverflowDecision, FaceScanCheckpoint, HitRowRangeTracker,
    HorizontalScrollSkipState, LineNumberRenderState, SpecialTextRowOverflowDecision,
    TextPropertyScanCheckpoints, TextRowTransitionStatePolicy, TrailingWhitespaceRenderState,
    WordWrapBreakCandidate, WordWrapRenderState, skip_text_to_charpos, skip_to_newline,
};
use crate::display_source::{DisplaySourceContext, SyntheticTextItemSource};
use crate::display_source_resolver::{
    DisplayPropertyReplacementAppendRequestResolver, DisplaySourceFaceBasis,
    DisplaySourceFallbackMetrics, DisplaySourcePropertyResolver, DisplaySourceResolveParams,
    DisplaySourceResolveState, PendingDisplaySourceFace,
};
use crate::display_status_line::{ChromeRowRenderServices, WindowChromeRowsRenderRequest};
use crate::font_metrics::FontMetricsService;
use crate::hit_test::{HitRow, WindowHitData};
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::neovm_bridge::{
    FaceResolver, LayoutBufferView, ResolvedFace, RustBufferAccess, RustTextPropAccess,
};
use crate::types::{LineWrapMode, WindowParams};
use crate::unicode::is_wide_char;
use crate::window_output::{
    TextMatrixRowTransition, TextWindowOutputRenderState, TextWindowOutputRetryCheckpoint,
    TextWindowRedisplayPositions, WindowOutputEmitter,
};
use neomacs_display_protocol::face::BasicFaceId;
use neomacs_display_protocol::types::{Color, Rect};
use neovm_core::buffer::{BufferId, CharPos0, EmacsBytePos, LispCharPos1};
use neovm_core::emacs_core::eval::DisplayHost;
use neovm_core::emacs_core::{Context, Value};
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
    text_matrix_row_base: usize,
    max_rows: usize,
    row_limit: DisplayRowLimit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextWindowRowPreludeRequestContext {
    line_number_mode: u8,
    line_number_current_absolute: bool,
    line_number_offset: i64,
    line_number_major_tick: i32,
    line_number_cols: i32,
    prefix_values: DisplayRowPrefixValues,
    char_width: f32,
    char_height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowBodyInstallContext {
    pub(crate) matrix_window_id: u64,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) matrix_cols: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowRetryBounds {
    pub(crate) text_area_top: i64,
    pub(crate) text_area_bottom: i64,
}

pub(crate) struct BufferTextWindowTailRequestContext<'a> {
    pub(crate) params: &'a WindowParams,
    pub(crate) window_start: i64,
    accessible_start: i64,
    accessible_end: i64,
    text_start_byte: usize,
    text_matrix_row_base: usize,
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

pub(crate) struct BufferTextWindowPostLoopState<'rows, 'emit, 'surface> {
    loop_context: BufferTextWindowLoopRequestContext,
    source_render: TextRowSourceRenderState<'emit>,
    x: &'emit mut f32,
    col: &'emit mut usize,
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

pub(crate) struct BufferTextWindowTailDecorationRequest<'a> {
    params: &'a WindowParams,
    content_x: f32,
    cols: usize,
    text_y: f32,
    text_height: f32,
    char_width: f32,
    char_height: f32,
    default_fg: Color,
    max_rows: usize,
    row_limit: DisplayRowLimit,
    row_geometry_defaults: DisplayRowGeometryDefaults,
}

pub(crate) struct BufferTextWindowTailDecorationState<'a> {
    pub(crate) x: f32,
    pub(crate) text_append_surface: &'a DisplayRowAppendSurface,
    pub(crate) row_geometry: &'a DisplayRowGeometryState,
    pub(crate) row_y_positions: &'a DisplayRowYPositions,
    pub(crate) row_flags: &'a DisplayRowFlags,
    pub(crate) row_extend: &'a DisplayRowScopedValue<(Color, u32)>,
    pub(crate) box_face: &'a BoxFaceRowState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BufferTextWindowTailDecorationOutcome {
    pub(crate) box_face_active: bool,
    pub(crate) row_extend_active: bool,
    pub(crate) current_row_extended: bool,
    pub(crate) empty_extend_rows: usize,
    pub(crate) fringe_rows: usize,
    pub(crate) right_continuation_rows: usize,
    pub(crate) right_truncation_rows: usize,
    pub(crate) left_continuation_rows: usize,
    pub(crate) empty_line_fringe_rows: usize,
    pub(crate) fill_column_rows: usize,
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
    pub(crate) text_property_checkpoints: TextPropertyScanCheckpoints,
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
    matrix_window_id: u64,
    text_matrix_row_base: usize,
    text_matrix_rows: usize,
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

pub(crate) struct BufferTextWindowWalkRenderState<'emit> {
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) face_scan: &'emit mut FaceScanCheckpoint,
    pub(crate) active_face_state: &'emit mut DisplayRowActiveFaceState,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
}

pub(crate) struct BufferTextWindowPostLoopRenderState<'emit> {
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
}

pub(crate) struct BufferTextWindowBodyRenderState<'emit> {
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) face_scan: &'emit mut FaceScanCheckpoint,
    pub(crate) active_face_state: &'emit mut DisplayRowActiveFaceState,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
}

pub(crate) struct BufferTextWindowBodyPassState<'emit> {
    pub(crate) output: BufferTextWindowBodyOutputState<'emit>,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
}

pub(crate) struct BufferTextWindowBodyOutputState<'emit> {
    builder: &'emit mut GlyphMatrixBuilder,
    evaluator: &'emit mut Context,
    font_metrics: &'emit mut Option<FontMetricsService>,
    face_resolver: &'emit FaceResolver,
}

pub(crate) struct BufferTextWindowOutputSession<'emit> {
    builder: &'emit mut GlyphMatrixBuilder,
    evaluator: &'emit mut Context,
    font_metrics: &'emit mut Option<FontMetricsService>,
    face_resolver: &'emit FaceResolver,
    face_ids: &'emit mut FrameFaceIdAllocator,
}

pub(crate) struct BufferTextWindowBodyInstallRenderState<'emit, 'output, 'face> {
    pub(crate) output: TextWindowOutputRenderState<'emit, 'output>,
    pub(crate) render_services: ChromeRowRenderServices<'emit, 'face>,
}

pub(crate) struct BufferTextWindowBodyInstallPublishState<'emit, 'output, 'face> {
    pub(crate) output: TextWindowOutputRenderState<'emit, 'output>,
    pub(crate) evaluator: &'emit mut Context,
    pub(crate) render_services: ChromeRowRenderServices<'emit, 'face>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowRedisplayPublishRequest {
    frame_id: FrameId,
    window_id: WindowId,
    accessible_end_lisp_char: usize,
    accessible_end_emacs_byte: usize,
}

pub(crate) struct BufferTextWindowBodyPassOutcome {
    pub(crate) output_emitter: WindowOutputEmitter,
    pub(crate) post_loop: BufferTextWindowPostLoopRenderOutcome,
}

pub(crate) struct BufferTextWindowRenderedBody<'a> {
    output_emitter: WindowOutputEmitter,
    post_loop: BufferTextWindowPostLoopRenderOutcome,
    retry_bounds: BufferTextWindowRetryBounds,
    publish_request: BufferTextWindowRedisplayPublishRequest,
    tail_context: BufferTextWindowTailRequestContext<'a>,
}

pub(crate) struct BufferTextWindowRenderedBodyFinishState<'a> {
    builder: &'a mut GlyphMatrixBuilder,
    evaluator: &'a mut Context,
    hit_data: &'a mut Vec<WindowHitData>,
    display_snapshots: &'a mut Vec<WindowDisplaySnapshot>,
}

pub(crate) struct BufferTextWindowRenderedBodyCompleteState<'emit, 'face> {
    builder: &'emit mut GlyphMatrixBuilder,
    evaluator: &'emit mut Context,
    render_services: ChromeRowRenderServices<'emit, 'face>,
    hit_data: &'emit mut Vec<WindowHitData>,
    display_snapshots: &'emit mut Vec<WindowDisplaySnapshot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowRetryPlan {
    window_id: i64,
    window_start: i64,
    point_charpos: i64,
    charpos_end: i64,
    rendered_rows_len: usize,
    retry_bounds: BufferTextWindowRetryBounds,
    retry: BufferTextWindowVisibilityRetryOutcome,
}

pub(crate) struct BufferTextWindowRenderContextsRequest<'a, 'surface, B>
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
    matrix_window_id: u64,
    append_surface: &'surface DisplayRowAppendSurface,
    text_y: f32,
    text_matrix_row_base: usize,
    max_rows: usize,
}

pub(crate) struct BufferTextWindowRenderContexts<'a, 'surface, B>
where
    B: LayoutBufferView,
{
    pub(crate) has_overlays: bool,
    pub(crate) face_resolution: BufferCurrentFaceResolutionContext<'a, B>,
    pub(crate) overlay_text_row: BufferOverlayStringTextRowRenderContext<'surface>,
}

pub(crate) struct BufferTextWindowBodyPlan<'a, 'surface, B>
where
    B: LayoutBufferView,
{
    pub(crate) begin_request: BufferTextWindowBeginRequest,
    pub(crate) retry_bounds: BufferTextWindowRetryBounds,
    pub(crate) publish_request: BufferTextWindowRedisplayPublishRequest,
    pub(crate) local_display_policy: BufferTextWindowLocalDisplayPolicy,
    pub(crate) initial_face_state: BufferTextWindowInitialFaceStateRequest<'a>,
    pub(crate) row_prelude_context: BufferTextWindowRowPreludeRequestContext,
    pub(crate) loop_context: BufferTextWindowLoopRequestContext,
    pub(crate) render_contexts: BufferTextWindowRenderContexts<'a, 'surface, B>,
    pub(crate) tail_context: BufferTextWindowTailRequestContext<'a>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BufferTextWindowInitialFaceStateRequest<'a> {
    measurement_policy: DisplayRowMeasurementPolicy,
    default_resolved: &'a ResolvedFace,
    default_face_char_width: f32,
    fallback_metrics: DisplayRowFallbackMetrics,
}

pub(crate) struct BufferTextWindowProgressState<'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
}

impl<'emit> BufferTextWindowProgressState<'emit> {
    pub(crate) fn new(
        byte_idx: &'emit mut usize,
        charpos: &'emit mut i64,
        x: &'emit mut f32,
        col: &'emit mut usize,
    ) -> Self {
        Self {
            byte_idx,
            charpos,
            x,
            col,
        }
    }

    pub(crate) fn row_position(&self) -> DisplayRowPosition {
        DisplayRowPosition {
            x_px: *self.x,
            col: *self.col,
        }
    }

    pub(crate) fn charpos(&self) -> i64 {
        *self.charpos
    }
}

pub(crate) struct BufferTextWindowLoopRenderState<'rows, 'emit, 'surface> {
    loop_context: BufferTextWindowLoopRequestContext,
    append_state: &'emit mut BufferTextRowAppendState,
    text_property_checkpoints: &'emit mut TextPropertyScanCheckpoints,
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

pub(crate) struct BufferTextSourceItemStepRenderRequest<'a> {
    layout_resolution_context: BufferSourceItemLayoutResolutionContext<'a>,
    source_step: BufferTextSourceItemStep,
    text: &'a [u8],
    active_face_state: &'a DisplayRowActiveFaceState,
    params: &'a WindowParams,
}

pub(crate) struct BufferTextSourceItemRenderRequest<'a> {
    layout_resolution_context: BufferSourceItemLayoutResolutionContext<'a>,
    source_item: BufferTextSourceItem,
    item_stepper: &'a mut BufferTextSourceItemStepper,
    text: &'a [u8],
    active_face_state: &'a DisplayRowActiveFaceState,
    params: &'a WindowParams,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextWindowLoopStepOutcome {
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
        text_matrix_row_base: usize,
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
            text_matrix_row_base,
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
        BufferInvisibleTextRenderRequest::new(BufferInvisibleTextRenderContext {
            text,
            accessible_end: self.accessible_end,
            point_charpos: self.point_charpos,
            append_surface,
            overlay_context,
            active_face_state,
            glyph_y_offset,
            default_face_ascent: self.default_face_ascent,
            char_h: self.char_height,
            char_w: self.char_width,
        })
    }

    pub(crate) fn hscroll_skip_request<'a>(
        self,
        text: &'a [u8],
        append_surface: &'a DisplayRowAppendSurface,
        active_face_state: &'a DisplayRowActiveFaceState,
    ) -> BufferHscrollSkipRenderRequest<'a> {
        BufferHscrollSkipRenderRequest::new(BufferHscrollSkipRenderContext {
            text,
            tab_width: self.tab_width,
            content_x: self.content_x,
            append_surface,
            active_face_state,
            default_face_ascent: self.default_face_ascent,
            char_h: self.char_height,
            char_w: self.char_width,
            point_charpos: self.point_charpos,
            has_prefix: self.has_prefix,
            row_geometry_defaults: self.row_geometry_defaults,
            text_matrix_row_base: self.text_matrix_row_base,
            max_rows: self.max_rows,
            row_limit: self.row_limit,
        })
    }

    pub(crate) fn display_property_checkpoint_request<'a, B>(
        self,
        buffer: &'a B,
        charpos: i64,
    ) -> BufferDisplayPropertyCheckpointRenderRequest<'a, B>
    where
        B: LayoutBufferView,
    {
        BufferDisplayPropertyCheckpointRenderRequest::new(
            BufferDisplayPropertyCheckpointRenderContext { buffer, charpos },
        )
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
            BufferSelectiveDisplayTailRenderContext {
                text,
                text_start_byte: self.text_start_byte,
                selective_display: self.selective_display,
                tab_width: self.tab_width,
                append_surface,
                active_face_state,
                glyph_y_offset,
                default_face_ascent: self.default_face_ascent,
                char_h: self.char_height,
                char_w: self.char_width,
                content_x: self.content_x,
                has_prefix: self.has_prefix,
                row_geometry_defaults: self.row_geometry_defaults,
                text_matrix_row_base: self.text_matrix_row_base,
                max_rows: self.max_rows,
                row_limit: self.row_limit,
            },
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
            BufferTextLineBreakRenderContext {
                text,
                text_start_byte: self.text_start_byte,
                selective_display: self.selective_display,
                tab_width: self.tab_width,
                active_face_state,
                point_charpos: self.point_charpos,
                char_h: self.char_height,
                extra_line_spacing: self.extra_line_spacing,
                content_x: self.content_x,
                has_prefix: self.has_prefix,
                row_geometry_defaults: self.row_geometry_defaults,
                text_matrix_row_base: self.text_matrix_row_base,
                max_rows: self.max_rows,
                row_limit: self.row_limit,
                overlay_context,
            },
        )
    }

    pub(crate) fn source_char_request<'a>(
        self,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'a>,
        source_char: BufferTextSourceStepChar,
        source_item: DisplayItem,
        text: &'a [u8],
        append_surface: &'a DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
        params: &'a WindowParams,
        glyph_y_offset: f32,
    ) -> BufferTextSourceCharRenderRequest<'a> {
        BufferTextSourceCharRenderRequest::new(
            source_char,
            source_item,
            BufferTextSourceCharRenderContext {
                layout_resolution_context,
                text,
                text_start_byte: self.text_start_byte,
                buffer_id: self.buffer_id,
                append_surface,
                overlay_context,
                active_face_state,
                params,
                glyph_y_offset,
                char_h: self.char_height,
                point_charpos: self.point_charpos,
                row_visibility_limit: self.row_visibility_limit,
                content_x: self.content_x,
                has_prefix: self.has_prefix,
                row_geometry_defaults: self.row_geometry_defaults,
                text_matrix_row_base: self.text_matrix_row_base,
                max_rows: self.max_rows,
                row_limit: self.row_limit,
            },
        )
    }

    pub(crate) fn end_of_buffer_tail_request<'a>(
        self,
        byte_idx: usize,
        charpos: i64,
        has_overlays: bool,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
    ) -> BufferEndOfBufferTailRenderRequest<'a> {
        BufferEndOfBufferTailRenderRequest::new(BufferEndOfBufferTailRenderContext {
            byte_idx,
            charpos,
            accessible_end: self.accessible_end,
            point_charpos: self.point_charpos,
            has_overlays,
            overlay_context,
            active_face_state,
            row_limit: self.row_limit,
        })
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

impl<'a> BufferTextWindowTailDecorationRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        params: &'a WindowParams,
        content_x: f32,
        cols: usize,
        text_y: f32,
        text_height: f32,
        char_width: f32,
        char_height: f32,
        default_fg: Color,
        max_rows: usize,
        row_limit: DisplayRowLimit,
        row_geometry_defaults: DisplayRowGeometryDefaults,
    ) -> Self {
        Self {
            params,
            content_x,
            cols,
            text_y,
            text_height,
            char_width,
            char_height,
            default_fg,
            max_rows,
            row_limit,
            row_geometry_defaults,
        }
    }

    pub(crate) fn apply(
        self,
        state: BufferTextWindowTailDecorationState<'_>,
    ) -> BufferTextWindowTailDecorationOutcome {
        let mut outcome = BufferTextWindowTailDecorationOutcome::default();

        if state.box_face.is_active() {
            outcome.box_face_active = true;
            let _ = (state.box_face.start_x(), state.box_face.row());
        }

        if let Some((_ext_bg, _ext_face_id)) = state.row_extend.value() {
            outcome.row_extend_active = true;
            let right_edge = state.text_append_surface.right_edge();
            if state.x < right_edge && state.row_geometry.is_within_row_limit(self.row_limit) {
                outcome.current_row_extended = true;
                let _ = state.row_geometry.current_row_y(
                    state.row_y_positions,
                    self.text_y,
                    self.char_height,
                );
            }

            let start_row = state.row_geometry.first_row_below_current(self.row_limit);
            for r in start_row..self.max_rows {
                let ry = state.row_geometry.row_y(
                    r,
                    state.row_y_positions,
                    self.text_y,
                    self.char_height,
                );
                if ry + self.char_height > self.text_y + self.text_height {
                    break;
                }
                outcome.empty_extend_rows += 1;
            }
        }

        if self.params.left_fringe_width > 0.0 || self.params.right_fringe_width > 0.0 {
            let _fringe_char_w = self
                .params
                .left_fringe_width
                .min(self.char_width)
                .max(self.char_width * 0.5);

            for r in 0..state.row_geometry.rendered_row_count(self.row_limit) {
                outcome.fringe_rows += 1;
                let _gy = state
                    .row_y_positions
                    .y_for_row(r, self.row_geometry_defaults.row_y_fallback(0.0));

                if self.params.right_fringe_width > 0.0
                    && state.row_flags.is_set(r, DisplayRowFlagKind::Continued)
                {
                    outcome.right_continuation_rows += 1;
                }

                if self.params.right_fringe_width > 0.0
                    && state.row_flags.is_set(r, DisplayRowFlagKind::Truncated)
                {
                    outcome.right_truncation_rows += 1;
                }

                if self.params.left_fringe_width > 0.0
                    && state.row_flags.is_set(r, DisplayRowFlagKind::Continuation)
                {
                    outcome.left_continuation_rows += 1;
                }
            }

            if self.params.indicate_empty_lines > 0 {
                let eob_start = state.row_geometry.rendered_row_count(self.row_limit);
                for r in eob_start..self.max_rows {
                    let _gy = state.row_geometry.row_y(
                        r,
                        state.row_y_positions,
                        self.text_y,
                        self.char_height,
                    );
                    let left_fringe_x = self.params.text_bounds.x - self.params.left_fringe_width;
                    let right_fringe_x = self.params.text_bounds.x + self.params.text_bounds.width;
                    let _fringe_x = if self.params.indicate_empty_lines == 2 {
                        right_fringe_x
                    } else {
                        left_fringe_x
                    };
                    let fringe_w = if self.params.indicate_empty_lines == 2 {
                        self.params.right_fringe_width
                    } else {
                        self.params.left_fringe_width
                    };
                    if fringe_w > 0.0 {
                        outcome.empty_line_fringe_rows += 1;
                    }
                }
            }
        }

        if self.params.fill_column_indicator >= 0 {
            let fci_col = self.params.fill_column_indicator;
            let _fci_char = self.params.fill_column_indicator_char;
            let _fci_fg = if self.params.fill_column_indicator_fg != 0 {
                Color::from_pixel(self.params.fill_column_indicator_fg)
            } else {
                self.default_fg
            };

            if (fci_col as usize) < self.cols {
                let indicator_x = self.content_x + fci_col as f32 * self.char_width;
                let total_rows = state.row_geometry.rendered_row_count(self.row_limit);
                for r in 0..total_rows {
                    let _gy = state
                        .row_y_positions
                        .y_for_row(r, self.row_geometry_defaults.row_y_fallback(0.0));
                    if indicator_x < state.text_append_surface.right_edge() {
                        outcome.fill_column_rows += 1;
                    }
                }
            }
        }

        outcome
    }
}

impl BufferTextWindowRowPreludeRequestContext {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        line_number_mode: u8,
        line_number_current_absolute: bool,
        line_number_offset: i64,
        line_number_major_tick: i32,
        line_number_cols: i32,
        prefix_values: DisplayRowPrefixValues,
        char_width: f32,
        char_height: f32,
    ) -> Self {
        Self {
            line_number_mode,
            line_number_current_absolute,
            line_number_offset,
            line_number_major_tick,
            line_number_cols,
            prefix_values,
            char_width,
            char_height,
        }
    }

    pub(crate) fn line_number_margin_request(self) -> BufferLineNumberMarginRenderRequest {
        BufferLineNumberMarginRenderRequest::new(
            self.line_number_mode,
            self.line_number_current_absolute,
            self.line_number_offset,
            self.line_number_major_tick,
            self.line_number_cols,
        )
    }

    pub(crate) fn line_prefix_request<'a>(
        self,
        append_surface: &'a DisplayRowAppendSurface,
        row_geometry: &'a DisplayRowGeometryState,
        active_face_state: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        position: DisplayRowPosition,
    ) -> BufferLinePrefixRenderRequest<'a> {
        BufferLinePrefixRenderRequest::new(
            BufferLinePrefixRenderContext::new(
                self.prefix_values,
                append_surface,
                row_geometry,
                active_face_state,
                glyph_y_offset,
                self.char_height,
            ),
            position,
        )
    }

    pub(crate) fn char_width(self) -> f32 {
        self.char_width
    }

    #[cfg(test)]
    pub(crate) fn line_number_mode(self) -> u8 {
        self.line_number_mode
    }

    #[cfg(test)]
    pub(crate) fn prefix_values(self) -> DisplayRowPrefixValues {
        self.prefix_values
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
            text_property_checkpoints: TextPropertyScanCheckpoints::new(self.window_start),
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

impl<'emit> BufferTextWindowBodyOutputState<'emit> {
    pub(crate) fn begin_text_window_output(
        &mut self,
        begin_request: BufferTextWindowBeginRequest,
    ) -> WindowOutputEmitter {
        begin_request.begin_and_apply(BufferTextWindowBeginState::new(
            self.builder,
            self.evaluator,
        ))
    }

    pub(crate) fn source_render_state<'output>(
        &'output mut self,
        output_emitter: &'output mut WindowOutputEmitter,
    ) -> TextRowSourceRenderState<'output> {
        TextRowSourceRenderState::from_output_render(
            TextRowOutputRenderState::new(self.builder, output_emitter, self.evaluator),
            self.font_metrics,
            self.face_resolver,
        )
    }
}

impl<'emit> BufferTextWindowOutputSession<'emit> {
    pub(crate) fn new(
        builder: &'emit mut GlyphMatrixBuilder,
        evaluator: &'emit mut Context,
        font_metrics: &'emit mut Option<FontMetricsService>,
        face_resolver: &'emit FaceResolver,
        face_ids: &'emit mut FrameFaceIdAllocator,
    ) -> Self {
        Self {
            builder,
            evaluator,
            font_metrics,
            face_resolver,
            face_ids,
        }
    }

    pub(crate) fn body_pass_state(&mut self) -> BufferTextWindowBodyPassState<'_> {
        BufferTextWindowBodyPassState {
            output: BufferTextWindowBodyOutputState {
                builder: self.builder,
                evaluator: self.evaluator,
                font_metrics: self.font_metrics,
                face_resolver: self.face_resolver,
            },
            face_ids: self.face_ids,
        }
    }

    pub(crate) fn rendered_body_complete_state<'hit>(
        &'hit mut self,
        hit_data: &'hit mut Vec<WindowHitData>,
        display_snapshots: &'hit mut Vec<WindowDisplaySnapshot>,
    ) -> BufferTextWindowRenderedBodyCompleteState<'hit, 'hit> {
        BufferTextWindowRenderedBodyCompleteState {
            builder: self.builder,
            evaluator: self.evaluator,
            render_services: ChromeRowRenderServices::new(
                self.font_metrics,
                self.face_resolver,
                self.face_ids,
            ),
            hit_data,
            display_snapshots,
        }
    }

    pub(crate) fn initial_active_face_state(
        &mut self,
        request: BufferTextWindowInitialFaceStateRequest<'_>,
    ) -> DisplayRowActiveFaceState {
        request.into_active_face_state(self.font_metrics)
    }

    pub(crate) fn restore_retry_checkpoint(&mut self, checkpoint: TextWindowOutputRetryCheckpoint) {
        checkpoint.restore(self.builder);
    }

    pub(crate) fn publish_face_ids(&self, frame_counter: &mut u32) {
        *frame_counter = self.face_ids.finish();
    }
}

impl<'emit, 'face> BufferTextWindowRenderedBodyCompleteState<'emit, 'face> {
    pub(crate) fn finish_state(self) -> BufferTextWindowRenderedBodyFinishState<'emit> {
        BufferTextWindowRenderedBodyFinishState {
            builder: self.builder,
            evaluator: self.evaluator,
            hit_data: self.hit_data,
            display_snapshots: self.display_snapshots,
        }
    }

    pub(crate) fn install_body_and_publish_redisplay(
        &mut self,
        output_emitter: &mut WindowOutputEmitter,
        walk_setup: &mut BufferTextWindowWalkSetup,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
        publish_request: BufferTextWindowRedisplayPublishRequest,
    ) -> TextWindowRedisplayPositions {
        walk_setup.install_body_and_publish_redisplay(
            BufferTextWindowBodyInstallPublishState {
                output: TextWindowOutputRenderState::new(self.builder, output_emitter),
                evaluator: self.evaluator,
                render_services: self.render_services.reborrow(),
            },
            tail_context,
            publish_request,
        )
    }

    pub(crate) fn render_chrome_rows(
        &mut self,
        output_emitter: &mut WindowOutputEmitter,
        request: WindowChromeRowsRenderRequest<'_, '_>,
    ) {
        TextWindowOutputRenderState::new(self.builder, output_emitter).render_chrome_rows(
            self.evaluator,
            request,
            self.render_services.reborrow(),
        );
    }
}

impl<'a> BufferTextWindowRenderedBodyFinishState<'a> {
    pub(crate) fn finish_install_state(
        self,
        output_emitter: WindowOutputEmitter,
        hit_rows: Vec<HitRow>,
    ) -> BufferTextWindowFinishInstallState<'a> {
        BufferTextWindowFinishInstallState {
            finish_state: BufferTextWindowFinishState::new(
                self.builder,
                output_emitter,
                self.evaluator,
                hit_rows,
            ),
            hit_data: self.hit_data,
            display_snapshots: self.display_snapshots,
        }
    }
}

impl BufferTextWindowRedisplayPublishRequest {
    pub(crate) fn new(
        frame_id: FrameId,
        window_id: WindowId,
        accessible_end_lisp_char: usize,
        accessible_end_emacs_byte: usize,
    ) -> Self {
        Self {
            frame_id,
            window_id,
            accessible_end_lisp_char,
            accessible_end_emacs_byte,
        }
    }

    pub(crate) fn publish(self, evaluator: &mut Context, positions: TextWindowRedisplayPositions) {
        evaluator.publish_redisplay_window_positions(
            self.frame_id,
            self.window_id,
            positions.window_start,
            LispCharPos1::from_one_based_usize(self.accessible_end_lisp_char),
            EmacsBytePos::new(self.accessible_end_emacs_byte),
            positions.window_end,
            positions.window_end_byte,
            positions.window_end_vpos,
        );
    }
}

impl BufferTextWindowBodyInstallContext {
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
        BufferTextWindowBodyInstallRequest::new(BufferTextWindowBodyInstallRenderContext {
            window_id: self.matrix_window_id,
            window_start,
            text_start_byte,
            byte_idx,
            reserve_right_special_col,
            reserve_right_border_col,
            text_matrix_row_base: self.text_matrix_row_base,
            matrix_cols: self.matrix_cols,
            row_flags,
            right_edge_face_id: 0,
            char_w: char_width,
        })
    }

    #[cfg(test)]
    pub(crate) fn matrix_cols(self) -> usize {
        self.matrix_cols
    }
}

impl BufferTextWindowWalkSetup {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_visible_steps<'request, B: LayoutBufferView>(
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
        let mut source_cursor = BufferTextSourceCursor::new(
            loop_context.buffer_id(),
            buffer,
            CharPos0::new(self.charpos.max(0) as usize),
            CharPos0::new(usize::MAX),
            RenderFaceRef::Inherit,
        );
        let mut source_resolve_state = DisplaySourceResolveState::default();
        let mut item_stepper = BufferTextSourceItemStepper::new(loop_context.text_start_byte());

        BufferTextWindowLoopRenderState::new(
            loop_context,
            &mut self.buffer_text_append_state,
            &mut self.text_property_checkpoints,
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
            &mut source_cursor,
            &mut source_resolve_state,
            &mut item_stepper,
            row_prelude_context,
            face_resolution_context,
            text,
            params,
            state.active_face_state,
            buffer,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_tail_and_decide_retry<'request, 'buf, B: LayoutBufferView>(
        &mut self,
        state: &mut BufferTextWindowPostLoopRenderState<'_>,
        loop_context: BufferTextWindowLoopRequestContext,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
        text: &'request [u8],
        has_overlays: bool,
        overlay_context: BufferOverlayStringTextRowRenderContext<'request>,
        active_face_state: &'request DisplayRowActiveFaceState,
        buffer: &B,
        buf_access: &RustBufferAccess<'buf, B>,
    ) -> BufferTextWindowPostLoopRenderOutcome {
        BufferTextWindowPostLoopState::new(
            loop_context,
            state.source_render.reborrow(),
            &mut self.x,
            &mut self.col,
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
            has_overlays,
            active_face_state,
            buffer,
            buf_access,
        )
    }

    pub(crate) fn install_body(
        &mut self,
        state: BufferTextWindowBodyInstallRenderState<'_, '_, '_>,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
    ) -> TextWindowRedisplayPositions {
        tail_context
            .body_install_request(self.byte_idx, &self.row_flags)
            .install_and_apply(BufferTextWindowBodyInstallState::new(
                state.output,
                state.render_services,
            ))
    }

    pub(crate) fn install_body_and_publish_redisplay(
        &mut self,
        state: BufferTextWindowBodyInstallPublishState<'_, '_, '_>,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
        publish_request: BufferTextWindowRedisplayPublishRequest,
    ) -> TextWindowRedisplayPositions {
        let BufferTextWindowBodyInstallPublishState {
            output,
            evaluator,
            render_services,
        } = state;
        let redisplay_positions = self.install_body(
            BufferTextWindowBodyInstallRenderState {
                output,
                render_services,
            },
            tail_context,
        );
        // GNU status-line percent specs read the live window state from the
        // just-produced redisplay. Publish before chrome rows are evaluated.
        publish_request.publish(evaluator, redisplay_positions);
        redisplay_positions
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_body_and_tail<'request, 'buf, B: LayoutBufferView>(
        &mut self,
        state: &mut BufferTextWindowBodyRenderState<'_>,
        row_prelude_context: BufferTextWindowRowPreludeRequestContext,
        loop_context: BufferTextWindowLoopRequestContext,
        face_resolution_context: BufferCurrentFaceResolutionContext<'request, B>,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
        text: &'request [u8],
        params: &'request WindowParams,
        has_overlays: bool,
        overlay_text_row_context: BufferOverlayStringTextRowRenderContext<'request>,
        buffer: &B,
        buf_access: &RustBufferAccess<'buf, B>,
    ) -> BufferTextWindowPostLoopRenderOutcome {
        self.render_visible_steps(
            &mut BufferTextWindowWalkRenderState {
                source_render: state.source_render.reborrow(),
                line_numbers: state.line_numbers,
                face_scan: state.face_scan,
                active_face_state: state.active_face_state,
                face_ids: state.face_ids,
            },
            row_prelude_context,
            loop_context,
            face_resolution_context,
            text,
            params,
            overlay_text_row_context,
            buffer,
        );

        self.render_tail_and_decide_retry(
            &mut BufferTextWindowPostLoopRenderState {
                source_render: state.source_render.reborrow(),
                face_ids: state.face_ids,
            },
            loop_context,
            tail_context,
            text,
            has_overlays,
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
        has_overlays: bool,
        overlay_text_row_context: BufferOverlayStringTextRowRenderContext<'request>,
        buffer: &B,
        buf_access: &RustBufferAccess<'buf, B>,
    ) -> BufferTextWindowBodyPassOutcome {
        let mut output_emitter = state.output.begin_text_window_output(begin_request);
        let post_loop = self.render_body_and_tail(
            &mut BufferTextWindowBodyRenderState {
                source_render: state.output.source_render_state(&mut output_emitter),
                line_numbers,
                face_scan,
                active_face_state,
                face_ids: state.face_ids,
            },
            row_prelude_context,
            loop_context,
            face_resolution_context,
            tail_context,
            text,
            params,
            has_overlays,
            overlay_text_row_context,
            buffer,
            buf_access,
        );
        BufferTextWindowBodyPassOutcome {
            output_emitter,
            post_loop,
        }
    }

    pub(crate) fn finish_window_and_install(
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
        matrix_window_id: u64,
        text_matrix_row_base: usize,
        text_matrix_rows: usize,
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
            matrix_window_id,
            text_matrix_row_base,
            text_matrix_rows,
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
            geometry.text_matrix_row_base,
            geometry.text_matrix_rows,
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
        let matrix_cols = self.cols.max(1);
        BufferTextWindowOutputSetup {
            begin_request: BufferTextWindowBeginRequest::new(
                self.frame_id,
                self.window_id,
                self.text_matrix_row_base,
                walk_setup.text_area_left,
                walk_setup.window_top,
                self.matrix_window_id,
                self.text_matrix_row_base + self.text_matrix_rows + self.bottom_chrome_rows,
                matrix_cols,
                self.bounds,
                self.text_bounds,
                self.selected,
                walk_setup.row_geometry.text_matrix_row_begin(
                    self.text_matrix_row_base,
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
            body_install_context: BufferTextWindowBodyInstallContext {
                matrix_window_id: self.matrix_window_id,
                text_matrix_row_base: self.text_matrix_row_base,
                matrix_cols,
            },
            retry_bounds: BufferTextWindowRetryBounds {
                text_area_top: (self.text_y - walk_setup.window_top).round() as i64,
                text_area_bottom: (self.text_y + self.text_height - walk_setup.window_top).round()
                    as i64,
            },
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
        matrix_window_id: u64,
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
            matrix_window_id,
            append_surface,
            geometry.text_y,
            self.body_install_context.text_matrix_row_base,
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
            self.body_install_context.text_matrix_row_base,
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
            self.body_install_context.text_matrix_row_base,
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
    pub(crate) fn begin_render_body_and_tail<'buf>(
        self,
        walk_setup: &mut BufferTextWindowWalkSetup,
        output_session: &mut BufferTextWindowOutputSession<'_>,
        text: &'a [u8],
        params: &'a WindowParams,
        buffer: &B,
        buf_access: &RustBufferAccess<'buf, B>,
    ) -> BufferTextWindowRenderedBody<'a> {
        let BufferTextWindowRenderContexts {
            has_overlays,
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
            has_overlays,
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
    pub(crate) fn retry_plan(
        &self,
        walk_setup: &BufferTextWindowWalkSetup,
    ) -> BufferTextWindowRetryPlan {
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

    pub(crate) fn install_body_chrome_and_finish(
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
        matrix_window_id: u64,
        append_surface: &'surface DisplayRowAppendSurface,
        text_y: f32,
        text_matrix_row_base: usize,
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
            matrix_window_id,
            append_surface,
            text_y,
            text_matrix_row_base,
            max_rows,
        }
    }

    pub(crate) fn into_contexts(self) -> BufferTextWindowRenderContexts<'a, 'surface, B> {
        let has_overlays = !self.buffer.layout_overlays().is_empty();
        BufferTextWindowRenderContexts {
            has_overlays,
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
                self.matrix_window_id,
                self.append_surface,
                self.char_height,
                self.default_face_ascent,
                self.text_y,
                self.text_matrix_row_base,
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

impl BufferTextWindowRetryPlan {
    pub(crate) fn from_post_loop(
        window_id: i64,
        window_start: i64,
        point_charpos: i64,
        charpos_end: i64,
        retry_bounds: BufferTextWindowRetryBounds,
        post_loop: BufferTextWindowPostLoopRenderOutcome,
    ) -> Self {
        Self {
            window_id,
            window_start,
            point_charpos,
            charpos_end,
            rendered_rows_len: post_loop.rendered_rows_len,
            retry_bounds,
            retry: post_loop.retry,
        }
    }

    pub(crate) fn log_visibility_adjustments(self) {
        if self.retry.scroll_down_window_start().is_some() {
            tracing::debug!(
                "layout_window_rust: point={} beyond visible_end={:?} (charpos_end={}), visible_rows={}, new_window_start={:?}",
                layout_i64_char_pos_to_lisp_char_pos(self.point_charpos).as_i64(),
                self.retry.visible_end_lisp(),
                self.charpos_end,
                self.rendered_rows_len,
                self.retry.scroll_down_window_start()
            );
        }
        if self.retry.point_row_window_start().is_some() {
            tracing::debug!(
                "layout_window_rust: point={} row partially visible within {}..{}, new_window_start={:?}",
                self.point_charpos,
                self.retry_bounds.text_area_top,
                self.retry_bounds.text_area_bottom,
                self.retry.point_row_window_start()
            );
        }
        if self.retry.point_line_window_start().is_some() {
            tracing::debug!(
                "layout_window_rust: point={} line continues below final visible row, new_window_start={:?}",
                self.point_charpos,
                self.retry.point_line_window_start()
            );
        }
    }

    pub(crate) fn retry_window_start(self) -> Option<i64> {
        self.retry.retry_window_start()
    }

    pub(crate) fn should_retry(self, remaining_visibility_retries: usize) -> Option<i64> {
        self.retry_window_start().filter(|new_window_start| {
            remaining_visibility_retries > 0 && *new_window_start > self.window_start
        })
    }

    pub(crate) fn log_retry(self, new_window_start: i64, remaining_visibility_retries: usize) {
        tracing::debug!(
            "layout_window_rust: retrying window {} with adjusted window_start {} -> {} (remaining={})",
            self.window_id,
            self.window_start,
            new_window_start,
            remaining_visibility_retries
        );
    }
}

impl<'rows, 'emit, 'surface> BufferTextWindowLoopRenderState<'rows, 'emit, 'surface> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        loop_context: BufferTextWindowLoopRequestContext,
        append_state: &'emit mut BufferTextRowAppendState,
        text_property_checkpoints: &'emit mut TextPropertyScanCheckpoints,
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
            text_property_checkpoints,
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

    pub(crate) fn hscroll_should_skip(&self) -> bool {
        self.hscroll_skip.should_skip()
    }

    pub(crate) fn render_visible_steps<'request, B: LayoutBufferView>(
        &mut self,
        source_cursor: &mut BufferTextSourceCursor<'request, B>,
        source_resolve_state: &mut DisplaySourceResolveState,
        item_stepper: &mut BufferTextSourceItemStepper,
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
                    source_cursor,
                    source_resolve_state,
                    item_stepper,
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

    pub(crate) fn render_next_step<'request, B: LayoutBufferView>(
        &mut self,
        source_cursor: &mut BufferTextSourceCursor<'request, B>,
        source_resolve_state: &mut DisplaySourceResolveState,
        item_stepper: &mut BufferTextSourceItemStepper,
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

        let Some(consumed_source) = self.consume_source_item(
            source_cursor,
            source_resolve_state,
            item_stepper,
            face_resolution_context,
            active_face_state,
        ) else {
            return BufferTextWindowLoopStepOutcome::StopBufferWalk;
        };

        let source_item = match consumed_source {
            BufferTextConsumedSourceItem::PendingStep(source_step) => {
                return self.render_source_item_step_for_context(
                    BufferTextSourceItemStepRenderRequest {
                        layout_resolution_context: face_resolution_context
                            .source_item_layout_resolution_context(),
                        source_step,
                        text,
                        active_face_state,
                        params,
                    },
                    buffer,
                );
            }
            BufferTextConsumedSourceItem::SourceItem(source_item) => source_item,
            BufferTextConsumedSourceItem::Replacement(replacement) => {
                return self.render_replacement_source_item_for_context(
                    replacement,
                    item_stepper,
                    face_resolution_context.source_item_layout_resolution_context(),
                    text,
                    active_face_state,
                    params,
                    buffer,
                );
            }
        };

        self.render_source_item_for_context(
            BufferTextSourceItemRenderRequest {
                layout_resolution_context: face_resolution_context
                    .source_item_layout_resolution_context(),
                source_item,
                item_stepper,
                text,
                active_face_state,
                params,
            },
            buffer,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_pre_source_checkpoints_for_context<'request, B: LayoutBufferView>(
        &mut self,
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
            .render_invisible_text_for_context(text, active_face_state, buffer)
            .should_continue_buffer_walk()
        {
            return BufferTextWindowPreSourceOutcome::ContinueBufferWalk;
        }

        if self.hscroll_should_skip() {
            if self
                .render_hscroll_skip_for_context(text, active_face_state)
                .should_break()
            {
                return BufferTextWindowPreSourceOutcome::StopBufferWalk;
            }
            return BufferTextWindowPreSourceOutcome::ContinueBufferWalk;
        }

        self.render_face_checkpoint_for_context(face_resolution_context, active_face_state);

        let display_property_walk = self.render_display_property_checkpoint_for_context(buffer);
        if display_property_walk.should_continue_buffer_walk() {
            return BufferTextWindowPreSourceOutcome::ContinueBufferWalk;
        }

        BufferTextWindowPreSourceOutcome::ReadyForSourceItem
    }

    pub(crate) fn render_source_item_step_for_context<'request, B: LayoutBufferView>(
        &mut self,
        request: BufferTextSourceItemStepRenderRequest<'request>,
        buffer: &B,
    ) -> BufferTextWindowLoopStepOutcome {
        let BufferTextSourceItemStepRenderRequest {
            layout_resolution_context,
            source_step,
            text,
            active_face_state,
            params,
        } = request;
        let selective_display_outcome = self.render_selective_display_tail_for_context(
            source_step.source_char(),
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

        let is_explicit_line_break = source_step.is_explicit_line_break();
        let end_charpos = source_step.end_charpos();
        let (source_char, source_item) = source_step.into_parts();
        if is_explicit_line_break {
            if self
                .render_line_break_for_context(source_char, text, active_face_state, buffer)
                .should_break()
            {
                return BufferTextWindowLoopStepOutcome::StopBufferWalk;
            }
        } else {
            let char_render_outcome = self.render_source_char_for_context(
                layout_resolution_context,
                source_char,
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

    pub(crate) fn render_source_item_for_context<'request, B: LayoutBufferView>(
        &mut self,
        request: BufferTextSourceItemRenderRequest<'request>,
        buffer: &B,
    ) -> BufferTextWindowLoopStepOutcome {
        let BufferTextSourceItemRenderRequest {
            layout_resolution_context,
            source_item,
            item_stepper,
            text,
            active_face_state,
            params,
        } = request;

        let Some(source_step) = item_stepper.item_step_from_source_item(
            source_item,
            self.progress.byte_idx,
            *self.progress.charpos,
        ) else {
            return BufferTextWindowLoopStepOutcome::StopBufferWalk;
        };

        self.render_source_item_step_for_context(
            BufferTextSourceItemStepRenderRequest {
                layout_resolution_context,
                source_step,
                text,
                active_face_state,
                params,
            },
            buffer,
        )
    }

    pub(crate) fn render_replacement_source_item_for_context<'request, B: LayoutBufferView>(
        &mut self,
        replacement: BufferTextReplacementItem,
        item_stepper: &mut BufferTextSourceItemStepper,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'request>,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
        params: &'request WindowParams,
        buffer: &B,
    ) -> BufferTextWindowLoopStepOutcome
    where
        'surface: 'request,
    {
        let request = BufferDisplayPropertyTextReplacementRenderRequest::new(
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
        match request.render_and_apply(
            buffer,
            BufferDisplayPropertyTextReplacementRenderState {
                source_render: self.source_render.reborrow(),
                face_ids: self.face_ids,
                append_surface: self.append_surface,
                row_geometry: self.row_geometry,
                cursor_info: self.cursor_info,
                byte_idx: self.progress.byte_idx,
                charpos: self.progress.charpos,
                x: self.progress.x,
                col: self.progress.col,
            },
        ) {
            BufferDisplayPropertyTextReplacementRenderOutcome::Continue => {
                BufferTextWindowLoopStepOutcome::ContinueBufferWalk
            }
            BufferDisplayPropertyTextReplacementRenderOutcome::Fallback(source_item) => self
                .render_source_item_for_context(
                    BufferTextSourceItemRenderRequest {
                        layout_resolution_context,
                        source_item,
                        item_stepper,
                        text,
                        active_face_state,
                        params,
                    },
                    buffer,
                ),
            BufferDisplayPropertyTextReplacementRenderOutcome::Stop => {
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
                self.progress.x,
                self.progress.col,
            );
    }

    pub(crate) fn consume_source_item<B: LayoutBufferView>(
        &mut self,
        source_cursor: &mut BufferTextSourceCursor<'_, B>,
        source_resolve_state: &mut DisplaySourceResolveState,
        item_stepper: &mut BufferTextSourceItemStepper,
        face_resolution_context: BufferCurrentFaceResolutionContext<'_, B>,
        _active_face_state: &DisplayRowActiveFaceState,
    ) -> Option<BufferTextConsumedSourceItem> {
        // One persistent typed source cursor feeds the row walk. The source
        // side owns pending text-run splitting so direct single-character
        // items can still stay typed through render.
        let mut pending_faces = Vec::new();
        let source_item = {
            let params = face_resolution_context.source_resolve_params(None);
            let mut resolver = DisplaySourcePropertyResolver::new(
                params,
                source_resolve_state,
                self.face_ids,
                &mut pending_faces,
            );
            let mut source_context = DisplaySourceContext::with_face_resolver(&mut resolver);
            item_stepper.next_consumed_source_item(
                source_cursor,
                &mut source_context,
                self.progress.byte_idx,
                self.progress.charpos(),
            )
        };
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

    pub(crate) fn render_invisible_text_for_context<'request, B: LayoutBufferView>(
        &mut self,
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
        self.render_invisible_text_at_checkpoint(request, buffer)
    }

    pub(crate) fn render_hscroll_skip_for_context<'request>(
        &mut self,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
    ) -> DisplayRowTransitionContinuation
    where
        'surface: 'request,
    {
        let request =
            self.loop_context
                .hscroll_skip_request(text, self.append_surface, active_face_state);
        self.render_hscroll_skip(request)
    }

    pub(crate) fn render_face_checkpoint_for_context<B: LayoutBufferView>(
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
            *self.progress.x,
            self.progress.charpos(),
        );
    }

    pub(crate) fn render_display_property_checkpoint_for_context<'request, B: LayoutBufferView>(
        &mut self,
        buffer: &'request B,
    ) -> BufferDisplayPropertyTextWalkOutcome {
        let request = self
            .loop_context
            .display_property_checkpoint_request(buffer, self.progress.charpos());
        self.render_display_property_checkpoint(request)
    }

    pub(crate) fn render_selective_display_tail_for_context<'request, B: LayoutBufferView>(
        &mut self,
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
        self.render_selective_display_tail(request, buffer)
    }

    pub(crate) fn render_line_break_for_context<'request, B: LayoutBufferView>(
        &mut self,
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
        self.render_line_break(request, buffer)
    }

    pub(crate) fn render_source_char_for_context<'request, B: LayoutBufferView>(
        &mut self,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'request>,
        source_char: BufferTextSourceStepChar,
        source_item: DisplayItem,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
        params: &'request WindowParams,
        buffer: &B,
    ) -> BufferTextSourceCharRenderOutcome
    where
        'surface: 'request,
    {
        let request = self.loop_context.source_char_request(
            layout_resolution_context,
            source_char,
            source_item,
            text,
            self.append_surface,
            self.overlay_context,
            active_face_state,
            params,
            0.0,
        );
        self.render_source_char(request, buffer)
    }

    fn render_invisible_text_at_checkpoint<B: LayoutBufferView>(
        &mut self,
        request: BufferInvisibleTextRenderRequest<'_>,
        buffer: &B,
    ) -> BufferInvisibleTextRenderOutcome {
        request.render_at_checkpoint_and_apply(
            buffer,
            BufferInvisibleTextRenderRequestState {
                checkpoints: self.text_property_checkpoints,
                byte_idx: self.progress.byte_idx,
                charpos: self.progress.charpos,
                source_render: self.source_render.reborrow(),
                x: self.progress.x,
                col: self.progress.col,
                row_geometry: self.row_geometry,
                cursor_info: self.cursor_info,
                hit_rows: self.hit_rows,
                hit_row_range: self.hit_row_range,
                row_y_positions: self.row_y_positions,
                face_ids: self.face_ids,
            },
        )
    }

    fn render_hscroll_skip(
        &mut self,
        request: BufferHscrollSkipRenderRequest<'_>,
    ) -> DisplayRowTransitionContinuation {
        request.render_next_and_apply(BufferHscrollSkipRenderState {
            byte_idx: self.progress.byte_idx,
            charpos: self.progress.charpos,
            hscroll_skip: self.hscroll_skip,
            row_extend: self.row_extend,
            source_render: self.source_render.reborrow(),
            x: self.progress.x,
            col: self.progress.col,
            prefix_request: self.prefix_request,
            line_numbers: self.line_numbers,
            word_wrap: self.word_wrap,
            trailing_whitespace: self.trailing_whitespace,
            row_geometry: self.row_geometry,
            row_flags: self.row_flags,
            hit_rows: self.hit_rows,
            hit_row_range: self.hit_row_range,
            cursor_info: self.cursor_info,
            row_y_positions: self.row_y_positions,
        })
    }

    fn render_display_property_checkpoint<B: LayoutBufferView>(
        &mut self,
        request: BufferDisplayPropertyCheckpointRenderRequest<'_, B>,
    ) -> BufferDisplayPropertyTextWalkOutcome {
        request.render_and_apply(BufferDisplayPropertyCheckpointRenderState::new(
            self.text_property_checkpoints,
        ))
    }

    fn render_selective_display_tail<B: LayoutBufferView>(
        &mut self,
        request: BufferSelectiveDisplayTailRenderRequest<'_>,
        buffer: &B,
    ) -> BufferSelectiveDisplayTailRenderOutcome {
        request.render_if_needed_and_apply(
            buffer,
            BufferSelectiveDisplayTailRenderState {
                byte_idx: self.progress.byte_idx,
                charpos: self.progress.charpos,
                col: self.progress.col,
                source_render: self.source_render.reborrow(),
                row_extend: self.row_extend,
                box_face: self.box_face,
                x: self.progress.x,
                line_numbers: self.line_numbers,
                row_geometry: self.row_geometry,
                row_flags: self.row_flags,
                hit_rows: self.hit_rows,
                hit_row_range: self.hit_row_range,
                prefix_request: self.prefix_request,
                hscroll_skip: self.hscroll_skip,
                word_wrap: self.word_wrap,
                trailing_whitespace: self.trailing_whitespace,
                row_y_positions: self.row_y_positions,
            },
        )
    }

    fn render_line_break<B: LayoutBufferView>(
        &mut self,
        request: BufferTextLineBreakRenderRequest<'_>,
        buffer: &B,
    ) -> DisplayRowTransitionContinuation {
        request.render_and_apply(
            buffer,
            BufferTextLineBreakRenderState {
                byte_idx: self.progress.byte_idx,
                charpos: self.progress.charpos,
                cursor_info: self.cursor_info,
                row_geometry: self.row_geometry,
                trailing_whitespace: self.trailing_whitespace,
                row_extend: self.row_extend,
                box_face: self.box_face,
                source_render: self.source_render.reborrow(),
                x: self.progress.x,
                col: self.progress.col,
                prefix_request: self.prefix_request,
                line_numbers: self.line_numbers,
                hscroll_skip: self.hscroll_skip,
                word_wrap: self.word_wrap,
                row_flags: self.row_flags,
                hit_rows: self.hit_rows,
                hit_row_range: self.hit_row_range,
                row_y_positions: self.row_y_positions,
                face_ids: self.face_ids,
            },
        )
    }

    fn render_source_char<B: LayoutBufferView>(
        &mut self,
        request: BufferTextSourceCharRenderRequest<'_>,
        buffer: &B,
    ) -> BufferTextSourceCharRenderOutcome {
        request.render_and_apply(
            buffer,
            BufferTextSourceCharRenderRequestState {
                append_state: self.append_state,
                byte_idx: self.progress.byte_idx,
                charpos: self.progress.charpos,
                col: self.progress.col,
                source_render: self.source_render.reborrow(),
                row_extend: self.row_extend,
                x: self.progress.x,
                line_numbers: self.line_numbers,
                row_geometry: self.row_geometry,
                row_flags: self.row_flags,
                hit_rows: self.hit_rows,
                hit_row_range: self.hit_row_range,
                prefix_request: self.prefix_request,
                hscroll_skip: self.hscroll_skip,
                word_wrap: self.word_wrap,
                trailing_whitespace: self.trailing_whitespace,
                face_scan: self.face_scan,
                row_y_positions: self.row_y_positions,
                cursor_info: self.cursor_info,
                face_ids: self.face_ids,
            },
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
        text_matrix_row_base: usize,
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
            text_matrix_row_base,
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
        BufferTextWindowTailFinalizeRequest::new(BufferTextWindowTailFinalizeContext {
            params: self.params,
            text,
            text_matrix_row_base: self.text_matrix_row_base,
            text_area_left: self.text_area_left,
            window_top: self.window_top,
            text_y: self.text_y,
            text_height: self.text_height,
            char_w: self.char_width,
            char_h: self.char_height,
            window_start: self.window_start,
            point_charpos: self.params.point_charpos().get(),
            charpos,
            point_is_visible_eob,
            row_limit: self.row_limit,
        })
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
            self.retry_bounds.text_area_top,
            self.retry_bounds.text_area_bottom,
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

    pub(crate) fn finish_and_install(&self, state: BufferTextWindowFinishInstallState<'_>) {
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
        x: &'emit mut f32,
        col: &'emit mut usize,
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
            x,
            col,
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
        has_overlays: bool,
        active_face_state: &'request DisplayRowActiveFaceState,
        buffer: &B,
    ) -> bool
    where
        'surface: 'request,
    {
        self.loop_context
            .end_of_buffer_tail_request(
                byte_idx,
                charpos,
                has_overlays,
                self.overlay_context,
                active_face_state,
            )
            .render_and_apply(
                buffer,
                BufferEndOfBufferTailRenderState {
                    source_render: self.source_render.reborrow(),
                    x: self.x,
                    col: self.col,
                    row_geometry: self.row_geometry,
                    cursor_info: self.cursor_info,
                    hit_rows: self.hit_rows,
                    hit_row_range: self.hit_row_range,
                    row_y_positions: self.row_y_positions,
                    face_ids: self.face_ids,
                },
            )
            .point_is_visible_eob()
    }

    pub(crate) fn apply_tail_decorations(
        &self,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
    ) -> BufferTextWindowTailDecorationOutcome {
        tail_context
            .tail_decoration_request()
            .apply(BufferTextWindowTailDecorationState {
                x: *self.x,
                text_append_surface: self.text_append_surface,
                row_geometry: self.row_geometry,
                row_y_positions: self.row_y_positions,
                row_flags: self.row_flags,
                row_extend: self.row_extend,
                box_face: self.box_face,
            })
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
        has_overlays: bool,
        active_face_state: &'request DisplayRowActiveFaceState,
        buffer: &B,
        buf_access: &'rows RustBufferAccess<'buf, B>,
    ) -> BufferTextWindowPostLoopRenderOutcome
    where
        'surface: 'request,
    {
        let point_is_visible_eob = self.render_end_of_buffer_tail(
            byte_idx,
            charpos,
            has_overlays,
            active_face_state,
            buffer,
        );

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

pub(crate) struct BufferHscrollSkipRenderState<'a, 'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
}

pub(crate) struct BufferEndOfBufferTailRenderRequest<'a> {
    context: BufferEndOfBufferTailRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferEndOfBufferTailRenderContext<'a> {
    pub(crate) byte_idx: usize,
    pub(crate) charpos: i64,
    pub(crate) accessible_end: i64,
    pub(crate) point_charpos: i64,
    pub(crate) has_overlays: bool,
    pub(crate) overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
    pub(crate) active_face_state: &'a DisplayRowActiveFaceState,
    pub(crate) row_limit: DisplayRowLimit,
}

pub(crate) struct BufferEndOfBufferTailRenderState<'emit> {
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) row_y_positions: &'emit mut DisplayRowYPositions,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
}

pub(crate) struct BufferEndOfBufferTailRenderOutcome {
    point_is_visible_eob: bool,
}

impl BufferEndOfBufferTailRenderOutcome {
    pub(crate) fn point_is_visible_eob(self) -> bool {
        self.point_is_visible_eob
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferHscrollSkipAction {
    LineBreak {
        ch_start_byte_idx: usize,
        charpos: i64,
    },
    Text {
        ch_start_byte_idx: usize,
        charpos: i64,
        show_left_truncation: bool,
    },
}

impl BufferHscrollSkipAction {
    pub(crate) fn is_line_break(self) -> bool {
        matches!(self, Self::LineBreak { .. })
    }

    pub(crate) fn ch_start_byte_idx(self) -> usize {
        match self {
            Self::LineBreak {
                ch_start_byte_idx, ..
            }
            | Self::Text {
                ch_start_byte_idx, ..
            } => ch_start_byte_idx,
        }
    }

    pub(crate) fn charpos(self) -> i64 {
        match self {
            Self::LineBreak { charpos, .. } | Self::Text { charpos, .. } => charpos,
        }
    }

    pub(crate) fn should_show_left_truncation(self) -> bool {
        matches!(
            self,
            Self::Text {
                show_left_truncation: true,
                ..
            }
        )
    }

    pub(crate) fn apply_line_break_before_row_transition(
        self,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        output_emitter: &mut WindowOutputEmitter,
        x: &mut f32,
        content_x: f32,
    ) {
        if self.is_line_break() {
            *x = content_x;
            output_emitter.note_display_buffer_pos(LispCharPos1::new(self.charpos()));
            row_extend.clear();
        }
    }

    pub(crate) fn line_break_hit_range(
        self,
        hit_row_range: &mut HitRowRangeTracker,
    ) -> Option<DisplayRowHitRange> {
        if !self.is_line_break() {
            return None;
        }
        let hit_range = hit_row_range.range_to(self.charpos());
        hit_row_range.advance_to(self.charpos());
        Some(hit_range)
    }

    pub(crate) fn capture_line_break_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        x: f32,
        col: usize,
        char_h: f32,
    ) {
        if !target.is_missing() || point_charpos != self.charpos() {
            return;
        }
        capture_cursor_info(
            target,
            CapturedCursorInfo::line_break_from_active_face_state(
                active_face_state,
                CapturedCursorPlacement::from_row_text_position(
                    row_geometry.text_position(x, self.ch_start_byte_idx(), col),
                    CapturedCursorSlotWidth::FaceChar,
                    false,
                ),
                char_h,
            ),
        );
    }

    pub(crate) fn apply_after_line_break_row_transition(
        self,
        row_transition: TextMatrixRowTransition,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        x: f32,
        col: usize,
        char_h: f32,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        self.capture_line_break_cursor_if_point(
            target,
            active_face_state,
            row_geometry,
            point_charpos,
            x,
            col,
            char_h,
        );
        DisplayRowTransitionContinuation::Continue
    }

    pub(crate) fn capture_text_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        x: f32,
        col: usize,
    ) {
        if !target.is_missing() || point_charpos != self.charpos() {
            return;
        }
        capture_cursor_info(
            target,
            CapturedCursorInfo::from_active_face_state(
                active_face_state,
                CapturedCursorPlacement::from_row_text_position(
                    row_geometry.text_position(x, self.ch_start_byte_idx(), col),
                    CapturedCursorSlotWidth::FaceChar,
                    false,
                ),
            ),
        );
    }

    pub(crate) fn append_left_truncation_marker_to_text_row_and_apply<'ctx>(
        self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        state: &mut BufferSyntheticTextRenderState<'_>,
        content_x: f32,
    ) {
        if !self.should_show_left_truncation() {
            return;
        }
        state.append_hscroll_truncation_marker_to_text_row(render_context, row_geometry, content_x);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferEndOfBufferCursorAction {
    byte_idx: usize,
    charpos: i64,
    accessible_end: i64,
    point_charpos: i64,
}

impl BufferEndOfBufferCursorAction {
    pub(crate) fn new(
        byte_idx: usize,
        charpos: i64,
        accessible_end: i64,
        point_charpos: i64,
    ) -> Self {
        Self {
            byte_idx,
            charpos,
            accessible_end,
            point_charpos,
        }
    }

    fn point_is_visible_eob(self) -> bool {
        self.point_charpos == self.accessible_end && self.charpos == self.accessible_end
    }

    pub(crate) fn capture_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        x: f32,
        col: usize,
    ) {
        if !target.is_missing()
            || (self.charpos != self.point_charpos && !self.point_is_visible_eob())
        {
            return;
        }
        if self.point_is_visible_eob() {
            tracing::debug!(
                "layout_window_rust: capturing EOB cursor at x={:.1} y={:.1} point={} point-max={}",
                x,
                row_geometry.glyph_y(0.0),
                self.point_charpos,
                self.accessible_end
            );
        }
        capture_cursor_info(
            target,
            CapturedCursorInfo::from_active_face_state(
                active_face_state,
                CapturedCursorPlacement::from_row_text_position(
                    row_geometry.text_position(x, self.byte_idx, col),
                    CapturedCursorSlotWidth::FaceChar,
                    false,
                ),
            ),
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferEndOfBufferTailAction {
    cursor: BufferEndOfBufferCursorAction,
    has_overlays: bool,
}

impl BufferEndOfBufferTailAction {
    pub(crate) fn new(
        byte_idx: usize,
        charpos: i64,
        accessible_end: i64,
        point_charpos: i64,
        has_overlays: bool,
    ) -> Self {
        Self {
            cursor: BufferEndOfBufferCursorAction::new(
                byte_idx,
                charpos,
                accessible_end,
                point_charpos,
            ),
            has_overlays,
        }
    }

    pub(crate) fn point_is_visible_eob(self) -> bool {
        self.cursor.point_is_visible_eob()
    }

    pub(crate) fn capture_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        x: f32,
        col: usize,
    ) {
        self.cursor
            .capture_cursor_if_point(target, active_face_state, row_geometry, x, col);
    }

    pub(crate) fn should_render_overlay_strings(
        self,
        row_geometry: &DisplayRowGeometryState,
        row_limit: DisplayRowLimit,
    ) -> bool {
        self.has_overlays && row_geometry.is_within_row_limit(row_limit)
    }

    pub(crate) fn render_overlay_strings_at_eob<B: LayoutBufferView>(
        self,
        buffer: &B,
        render_context: BufferOverlayStringTextRowRenderContext<'_>,
        active_face_state: &DisplayRowActiveFaceState,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        render_context.render_at(buffer, self.cursor.charpos, active_face_state, state);
    }
}

impl<'a> BufferEndOfBufferTailRenderRequest<'a> {
    pub(crate) fn new(context: BufferEndOfBufferTailRenderContext<'a>) -> Self {
        Self { context }
    }

    pub(crate) fn render_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferEndOfBufferTailRenderState<'_>,
    ) -> BufferEndOfBufferTailRenderOutcome {
        let BufferEndOfBufferTailRenderState {
            source_render,
            x,
            col,
            row_geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
        } = state;
        let mut source_render = source_render;
        let context = self.context;

        let tail = BufferEndOfBufferTailAction::new(
            context.byte_idx,
            context.charpos,
            context.accessible_end,
            context.point_charpos,
            context.has_overlays,
        );
        let point_is_visible_eob = tail.point_is_visible_eob();
        tail.capture_cursor_if_point(
            cursor_info,
            context.active_face_state,
            row_geometry,
            *x,
            *col,
        );

        if tail.should_render_overlay_strings(row_geometry, context.row_limit) {
            let mut overlay_state = OverlayStringRenderState::from_source_render(
                source_render.reborrow(),
                x,
                col,
                row_geometry,
                cursor_info,
                hit_rows,
                hit_row_range,
                row_y_positions,
                face_ids,
            );
            tail.render_overlay_strings_at_eob(
                buffer,
                context.overlay_context,
                context.active_face_state,
                &mut overlay_state,
            );
        }

        BufferEndOfBufferTailRenderOutcome {
            point_is_visible_eob,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferHscrollSkipSourceStep {
    source_char: BufferTextSourceStepChar,
}

pub(crate) struct BufferHscrollSkipRenderRequest<'a> {
    context: BufferHscrollSkipRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferHscrollSkipRenderContext<'a> {
    pub(crate) text: &'a [u8],
    pub(crate) tab_width: i32,
    pub(crate) content_x: f32,
    pub(crate) append_surface: &'a DisplayRowAppendSurface,
    pub(crate) active_face_state: &'a DisplayRowActiveFaceState,
    pub(crate) default_face_ascent: f32,
    pub(crate) char_h: f32,
    pub(crate) char_w: f32,
    pub(crate) point_charpos: i64,
    pub(crate) has_prefix: bool,
    pub(crate) row_geometry_defaults: DisplayRowGeometryDefaults,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) max_rows: usize,
    pub(crate) row_limit: DisplayRowLimit,
}

impl BufferHscrollSkipSourceStep {
    fn new(source_char: BufferTextSourceStepChar) -> Self {
        Self { source_char }
    }

    pub(crate) fn consume_from_text(
        text: &[u8],
        byte_idx: &mut usize,
        charpos: &mut i64,
        hscroll_skip: &mut HorizontalScrollSkipState,
        tab_width: i32,
    ) -> Option<BufferHscrollSkipAction> {
        let source_char = BufferTextSourceStepChar::consume_from_text(text, byte_idx, charpos)?;
        Some(Self::new(source_char).consume_for_hscroll(hscroll_skip, tab_width))
    }

    fn consume_for_hscroll(
        self,
        hscroll_skip: &mut HorizontalScrollSkipState,
        tab_width: i32,
    ) -> BufferHscrollSkipAction {
        let source_char = self.source_char;
        let end_charpos = source_char.start_charpos() + 1;
        if source_char.ch() == '\n' {
            return BufferHscrollSkipAction::LineBreak {
                ch_start_byte_idx: source_char.start_byte_idx(),
                charpos: end_charpos,
            };
        }

        hscroll_skip.consume_columns(self.column_width(tab_width, hscroll_skip.consumed_columns()));
        BufferHscrollSkipAction::Text {
            ch_start_byte_idx: source_char.start_byte_idx(),
            charpos: end_charpos,
            show_left_truncation: !hscroll_skip.should_skip()
                && hscroll_skip.should_show_left_truncation(),
        }
    }

    fn column_width(self, tab_width: i32, consumed_columns: i32) -> i32 {
        if self.source_char.ch() == '\t' {
            let tab_width = tab_width.max(1);
            return ((consumed_columns / tab_width + 1) * tab_width) - consumed_columns;
        }

        if is_wide_char(self.source_char.ch()) {
            2
        } else {
            1
        }
    }
}

impl<'a> BufferHscrollSkipRenderRequest<'a> {
    pub(crate) fn new(context: BufferHscrollSkipRenderContext<'a>) -> Self {
        Self { context }
    }

    pub(crate) fn render_next_and_apply(
        self,
        state: BufferHscrollSkipRenderState<'_, '_>,
    ) -> DisplayRowTransitionContinuation {
        let BufferHscrollSkipRenderState {
            byte_idx,
            charpos,
            hscroll_skip,
            row_extend,
            source_render,
            x,
            col,
            prefix_request,
            line_numbers,
            word_wrap,
            trailing_whitespace,
            row_geometry,
            row_flags,
            hit_rows,
            hit_row_range,
            cursor_info,
            row_y_positions,
        } = state;
        let mut source_render = source_render;
        let context = self.context;

        let Some(hscroll_action) = BufferHscrollSkipSourceStep::consume_from_text(
            context.text,
            byte_idx,
            charpos,
            hscroll_skip,
            context.tab_width,
        ) else {
            return DisplayRowTransitionContinuation::Exhausted;
        };

        if hscroll_action.is_line_break() {
            hscroll_action.apply_line_break_before_row_transition(
                row_extend,
                source_render.output_emitter(),
                x,
                context.content_x,
            );
            let line_break_transition = DisplayRowLineBreakTransitionPlan::hscroll_line_break();
            let hit_range = hscroll_action
                .line_break_hit_range(hit_row_range)
                .expect("hscroll line break hit range");
            let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                context.row_geometry_defaults,
                context.text_matrix_row_base,
                row_y_positions,
                context.max_rows,
                row_geometry,
                row_flags,
                context.row_limit,
                hit_rows,
                &mut source_render,
            )
            .emit_line_break_then_row_start(
                line_break_transition,
                hit_range,
                DisplayRowPosition {
                    x_px: *x,
                    col: *col,
                },
                0.0,
                DisplayRowTransitionRenderState::new(
                    prefix_request,
                    context.has_prefix,
                    line_numbers,
                    hscroll_skip,
                    word_wrap,
                    trailing_whitespace,
                ),
                col,
            );
            return hscroll_action.apply_after_line_break_row_transition(
                row_transition,
                cursor_info,
                context.active_face_state,
                row_geometry,
                context.point_charpos,
                *x,
                *col,
                context.char_h,
            );
        }

        let mut synthetic_text_state =
            BufferSyntheticTextRenderState::new(source_render.reborrow(), x, col);
        hscroll_action.append_left_truncation_marker_to_text_row_and_apply(
            BufferSyntheticTextRenderContext::new(
                context.append_surface,
                context.active_face_state,
                0.0,
                context.char_h,
                context.default_face_ascent,
                context.char_w,
            ),
            row_geometry,
            &mut synthetic_text_state,
            context.content_x,
        );
        hscroll_action.capture_text_cursor_if_point(
            cursor_info,
            context.active_face_state,
            row_geometry,
            context.point_charpos,
            *x,
            *col,
        );
        DisplayRowTransitionContinuation::Continue
    }
}

pub(crate) struct BufferTextLineBreakRenderState<'a, 'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) box_face: &'emit mut BoxFaceRowState,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
}

pub(crate) struct BufferSelectiveDisplayTailRenderRequest<'a> {
    source_char: BufferTextSourceStepChar,
    context: BufferSelectiveDisplayTailRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferSelectiveDisplayTailRenderContext<'a> {
    pub(crate) text: &'a [u8],
    pub(crate) text_start_byte: usize,
    pub(crate) selective_display: i32,
    pub(crate) tab_width: i32,
    pub(crate) append_surface: &'a DisplayRowAppendSurface,
    pub(crate) active_face_state: &'a DisplayRowActiveFaceState,
    pub(crate) glyph_y_offset: f32,
    pub(crate) default_face_ascent: f32,
    pub(crate) char_h: f32,
    pub(crate) char_w: f32,
    pub(crate) content_x: f32,
    pub(crate) has_prefix: bool,
    pub(crate) row_geometry_defaults: DisplayRowGeometryDefaults,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) max_rows: usize,
    pub(crate) row_limit: DisplayRowLimit,
}

pub(crate) struct BufferSelectiveDisplayTailRenderState<'a, 'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) col: &'emit mut usize,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) box_face: &'emit mut BoxFaceRowState,
    pub(crate) x: &'emit mut f32,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
}

pub(crate) struct BufferInvisibleTextRenderRequest<'a> {
    context: BufferInvisibleTextRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferInvisibleTextRenderContext<'a> {
    pub(crate) text: &'a [u8],
    pub(crate) accessible_end: i64,
    pub(crate) point_charpos: i64,
    pub(crate) append_surface: &'a DisplayRowAppendSurface,
    pub(crate) overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
    pub(crate) active_face_state: &'a DisplayRowActiveFaceState,
    pub(crate) glyph_y_offset: f32,
    pub(crate) default_face_ascent: f32,
    pub(crate) char_h: f32,
    pub(crate) char_w: f32,
}

pub(crate) struct BufferInvisibleTextRenderRequestState<'a, 'emit> {
    pub(crate) checkpoints: &'emit mut TextPropertyScanCheckpoints,
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferSelectiveDisplayTailRenderOutcome {
    NotHidden,
    ContinueBufferWalk,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferInvisibleTextRenderOutcome {
    Visible,
    ContinueBufferWalk,
}

impl BufferSelectiveDisplayTailRenderOutcome {
    pub(crate) fn should_break(self) -> bool {
        matches!(self, Self::Stop)
    }

    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(self, Self::ContinueBufferWalk)
    }
}

impl BufferInvisibleTextRenderOutcome {
    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(self, Self::ContinueBufferWalk)
    }
}

impl<'a> BufferSelectiveDisplayTailRenderRequest<'a> {
    pub(crate) fn new(
        source_char: BufferTextSourceStepChar,
        context: BufferSelectiveDisplayTailRenderContext<'a>,
    ) -> Self {
        Self {
            source_char,
            context,
        }
    }

    pub(crate) fn render_if_needed_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferSelectiveDisplayTailRenderState<'_, '_>,
    ) -> BufferSelectiveDisplayTailRenderOutcome {
        let context = self.context;
        let selective_display = BufferSelectiveDisplayContext::new(
            context.text,
            context.selective_display,
            context.tab_width,
        );
        let Some(marker) = selective_display.carriage_return_tail_marker(self.source_char.ch())
        else {
            return BufferSelectiveDisplayTailRenderOutcome::NotHidden;
        };

        let BufferSelectiveDisplayTailRenderState {
            byte_idx,
            charpos,
            col,
            source_render,
            row_extend,
            box_face,
            x,
            line_numbers,
            row_geometry,
            row_flags,
            hit_rows,
            hit_row_range,
            prefix_request,
            hscroll_skip,
            word_wrap,
            trailing_whitespace,
            row_y_positions,
        } = state;
        let mut source_render = source_render;

        let mut synthetic_text_state =
            BufferSyntheticTextRenderState::new(source_render.reborrow(), x, col);
        marker.append_to_text_row_and_apply(
            BufferSyntheticTextRenderContext::new(
                context.append_surface,
                context.active_face_state,
                context.glyph_y_offset,
                context.char_h,
                context.default_face_ascent,
                context.char_w,
            ),
            row_geometry,
            &mut synthetic_text_state,
        );

        let tail_action =
            selective_display.skip_rest_of_line_after_carriage_return(byte_idx, charpos);
        if !tail_action.is_line_break() {
            return BufferSelectiveDisplayTailRenderOutcome::ContinueBufferWalk;
        }

        tail_action.apply_hidden_line_break_row_state(
            row_geometry,
            row_extend,
            box_face,
            context.content_x,
            x,
        );
        let line_break_transition = DisplayRowLineBreakTransitionPlan::hidden_line_break();
        let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
            context.row_geometry_defaults,
            context.text_matrix_row_base,
            row_y_positions,
            context.max_rows,
            row_geometry,
            row_flags,
            context.row_limit,
            hit_rows,
            &mut source_render,
        )
        .emit_line_break_then_row_start(
            line_break_transition,
            hit_row_range.range_to(*charpos),
            DisplayRowPosition {
                x_px: *x,
                col: *col,
            },
            0.0,
            DisplayRowTransitionRenderState::new(
                prefix_request,
                context.has_prefix,
                line_numbers,
                hscroll_skip,
                word_wrap,
                trailing_whitespace,
            ),
            col,
        );
        let synced_charpos = buffer
            .layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(
                context.text_start_byte + *byte_idx,
            ))
            .get() as i64;
        if tail_action
            .apply_after_hidden_line_break_transition(
                row_transition,
                synced_charpos,
                charpos,
                hit_row_range,
            )
            .should_break()
        {
            return BufferSelectiveDisplayTailRenderOutcome::Stop;
        }

        BufferSelectiveDisplayTailRenderOutcome::ContinueBufferWalk
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferInvisibleTextScanAction {
    Unchecked,
    Visible { next_visible: i64 },
    Hidden(BufferInvisibleTextSkip),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferInvisibleTextSkip {
    start_byte_idx: usize,
    start_charpos: i64,
    skip_to: i64,
    next_visible: i64,
    point_in_hidden_region: bool,
    ellipsis: bool,
}

impl BufferInvisibleTextSkip {
    pub(crate) fn new(
        start_byte_idx: usize,
        start_charpos: i64,
        skip_to: i64,
        next_visible: i64,
        point_in_hidden_region: bool,
        ellipsis: bool,
    ) -> Self {
        Self {
            start_byte_idx,
            start_charpos,
            skip_to,
            next_visible,
            point_in_hidden_region,
            ellipsis,
        }
    }

    #[cfg(test)]
    pub(crate) fn start_byte_idx(self) -> usize {
        self.start_byte_idx
    }

    #[cfg(test)]
    pub(crate) fn start_charpos(self) -> i64 {
        self.start_charpos
    }

    #[cfg(test)]
    pub(crate) fn skip_to(self) -> i64 {
        self.skip_to
    }

    #[cfg(test)]
    pub(crate) fn next_visible(self) -> i64 {
        self.next_visible
    }

    #[cfg(test)]
    pub(crate) fn point_in_hidden_region(self) -> bool {
        self.point_in_hidden_region
    }

    #[cfg(test)]
    pub(crate) fn ellipsis(self) -> bool {
        self.ellipsis
    }

    pub(crate) fn capture_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        x: f32,
        col: usize,
    ) {
        if !self.point_in_hidden_region {
            return;
        }
        capture_cursor_info(
            target,
            CapturedCursorInfo::from_active_face_state(
                active_face_state,
                CapturedCursorPlacement::from_row_text_position(
                    row_geometry.text_position(x, self.start_byte_idx, col),
                    CapturedCursorSlotWidth::FaceChar,
                    false,
                ),
            ),
        );
    }

    pub(crate) fn ellipsis_append_request(
        self,
        position: DisplayRowPosition,
    ) -> Option<SyntheticTextAppendRequest> {
        self.ellipsis.then(|| {
            SyntheticTextAppendRequest::active_marker(
                position,
                SyntheticTextMarker::InvisibleEllipsis,
            )
        })
    }

    pub(crate) fn append_to_text_row_and_apply<'ctx>(
        self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        state: &mut BufferInvisibleTextRenderState<'_>,
    ) {
        let position = state.synthetic_text.position();
        self.capture_cursor_if_point(
            state.cursor_info,
            render_context.active_face(),
            row_geometry,
            position.x_px,
            position.col,
        );

        let Some(request) = self.ellipsis_append_request(position) else {
            return;
        };
        state
            .synthetic_text
            .append_request_to_text_row(render_context, row_geometry, request);
    }
}

pub(crate) struct BufferInvisibleTextRenderState<'a> {
    synthetic_text: BufferSyntheticTextRenderState<'a>,
    cursor_info: &'a mut CursorCaptureState,
}

impl<'a> BufferInvisibleTextRenderState<'a> {
    pub(crate) fn new(
        source_render: TextRowSourceRenderState<'a>,
        cursor_info: &'a mut CursorCaptureState,
        x: &'a mut f32,
        col: &'a mut usize,
    ) -> Self {
        Self {
            synthetic_text: BufferSyntheticTextRenderState::new(source_render, x, col),
            cursor_info,
        }
    }
}

impl<'a> BufferInvisibleTextRenderRequest<'a> {
    pub(crate) fn new(context: BufferInvisibleTextRenderContext<'a>) -> Self {
        Self { context }
    }

    pub(crate) fn render_at_checkpoint_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferInvisibleTextRenderRequestState<'_, '_>,
    ) -> BufferInvisibleTextRenderOutcome {
        let BufferInvisibleTextRenderRequestState {
            checkpoints,
            byte_idx,
            charpos,
            source_render,
            x,
            col,
            row_geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
        } = state;
        let mut source_render = source_render;
        let context = self.context;

        let action = BufferInvisibleTextScanContext::new(
            context.text,
            context.accessible_end,
            context.point_charpos,
            cursor_info.is_missing(),
        )
        .consume_at_checkpoint(buffer, checkpoints, byte_idx, charpos);
        let BufferInvisibleTextScanAction::Hidden(hidden_text) = action else {
            return BufferInvisibleTextRenderOutcome::Visible;
        };

        let mut hidden_text_state =
            BufferInvisibleTextRenderState::new(source_render.reborrow(), cursor_info, x, col);
        hidden_text.append_to_text_row_and_apply(
            BufferSyntheticTextRenderContext::new(
                context.append_surface,
                context.active_face_state,
                context.glyph_y_offset,
                context.char_h,
                context.default_face_ascent,
                context.char_w,
            ),
            row_geometry,
            &mut hidden_text_state,
        );

        let mut overlay_state = OverlayStringRenderState::from_source_render(
            source_render.reborrow(),
            x,
            col,
            row_geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
        );
        context.overlay_context.render_at(
            buffer,
            *charpos,
            context.active_face_state,
            &mut overlay_state,
        );
        BufferInvisibleTextRenderOutcome::ContinueBufferWalk
    }
}

pub(crate) struct BufferInvisibleTextScanContext<'a> {
    text: &'a [u8],
    accessible_end: i64,
    point_charpos: i64,
    cursor_missing: bool,
}

impl<'a> BufferInvisibleTextScanContext<'a> {
    pub(crate) fn new(
        text: &'a [u8],
        accessible_end: i64,
        point_charpos: i64,
        cursor_missing: bool,
    ) -> Self {
        Self {
            text,
            accessible_end,
            point_charpos,
            cursor_missing,
        }
    }

    pub(crate) fn consume_at_checkpoint<B: LayoutBufferView>(
        &self,
        buffer: &B,
        checkpoints: &mut TextPropertyScanCheckpoints,
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) -> BufferInvisibleTextScanAction {
        if !checkpoints.should_check_invisible(*charpos) {
            return BufferInvisibleTextScanAction::Unchecked;
        }

        let start_byte_idx = *byte_idx;
        let start_charpos = *charpos;
        let text_props = RustTextPropAccess::new(buffer);
        let (invisible, next_visible) = text_props.check_invisible(start_charpos);
        checkpoints.record_invisible_next(next_visible);

        if !invisible.hidden {
            return BufferInvisibleTextScanAction::Visible { next_visible };
        }

        let skip_to = next_visible.min(self.accessible_end);
        let point_in_hidden_region = self.cursor_missing
            && self.point_charpos >= start_charpos
            && self.point_charpos < skip_to;
        skip_text_to_charpos(self.text, byte_idx, charpos, skip_to);

        BufferInvisibleTextScanAction::Hidden(BufferInvisibleTextSkip::new(
            start_byte_idx,
            start_charpos,
            skip_to,
            next_visible,
            point_in_hidden_region,
            invisible.ellipsis,
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferSelectiveDisplayLineTailAction {
    Exhausted,
    LineBreak { charpos: i64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSelectiveDisplayLineTailMarker;

impl BufferSelectiveDisplayLineTailMarker {
    pub(crate) fn ellipsis_append_request(
        self,
        position: DisplayRowPosition,
    ) -> SyntheticTextAppendRequest {
        SyntheticTextAppendRequest::active_marker(position, SyntheticTextMarker::SelectiveEllipsis)
    }

    pub(crate) fn append_to_text_row_and_apply<'ctx>(
        self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        state: &mut BufferSyntheticTextRenderState<'_>,
    ) {
        let request = self.ellipsis_append_request(state.position());
        state.append_request_to_text_row(render_context, row_geometry, request);
    }
}

impl BufferSelectiveDisplayLineTailAction {
    pub(crate) fn is_line_break(self) -> bool {
        matches!(self, Self::LineBreak { .. })
    }

    pub(crate) fn apply_hidden_line_break_row_state(
        self,
        row_geometry: &DisplayRowGeometryState,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &mut BoxFaceRowState,
        content_x: f32,
        x: &mut f32,
    ) {
        if self.is_line_break() {
            *x = content_x;
            row_extend.clear();
            box_face.continue_on_row(row_geometry.next_row_marker(), content_x);
        }
    }

    pub(crate) fn sync_after_hidden_line_break_transition(
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) {
        *charpos = synced_charpos;
        hit_row_range.advance_to(*charpos);
    }

    pub(crate) fn apply_after_hidden_line_break_transition(
        self,
        row_transition: TextMatrixRowTransition,
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        Self::sync_after_hidden_line_break_transition(synced_charpos, charpos, hit_row_range);
        DisplayRowTransitionContinuation::Continue
    }

    #[cfg(test)]
    pub(crate) fn charpos(self) -> Option<i64> {
        match self {
            Self::LineBreak { charpos } => Some(charpos),
            Self::Exhausted => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSelectiveDisplayHiddenLines {
    hidden_line_count: usize,
}

impl BufferSelectiveDisplayHiddenLines {
    fn new(hidden_line_count: usize) -> Self {
        Self { hidden_line_count }
    }

    #[cfg(test)]
    pub(crate) fn hidden_line_count(self) -> usize {
        self.hidden_line_count
    }

    pub(crate) fn apply_to_line_numbers(self, line_numbers: &mut LineNumberRenderState) {
        for _ in 0..self.hidden_line_count {
            line_numbers.advance_hidden_line();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSelectiveDisplayContext<'a> {
    text: &'a [u8],
    selective_display: i32,
    tab_width: i32,
}

impl<'a> BufferSelectiveDisplayContext<'a> {
    pub(crate) fn new(text: &'a [u8], selective_display: i32, tab_width: i32) -> Self {
        Self {
            text,
            selective_display,
            tab_width: tab_width.max(1),
        }
    }

    pub(crate) fn hides_carriage_return_tail(self, ch: char) -> bool {
        self.selective_display > 0 && ch == '\r'
    }

    pub(crate) fn carriage_return_tail_marker(
        self,
        ch: char,
    ) -> Option<BufferSelectiveDisplayLineTailMarker> {
        self.hides_carriage_return_tail(ch)
            .then_some(BufferSelectiveDisplayLineTailMarker)
    }

    pub(crate) fn hides_indented_lines_after_line_break(self, byte_idx: usize) -> bool {
        self.selective_display > 0
            && self.selective_display < i32::MAX
            && byte_idx < self.text.len()
    }

    pub(crate) fn skip_rest_of_line_after_carriage_return(
        self,
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) -> BufferSelectiveDisplayLineTailAction {
        *charpos += 1;
        while *byte_idx < self.text.len() {
            let Some(source_char) =
                BufferTextSourceStepChar::consume_from_text(self.text, byte_idx, charpos)
            else {
                break;
            };
            if source_char.ch() == '\n' {
                return BufferSelectiveDisplayLineTailAction::LineBreak { charpos: *charpos };
            }
        }

        BufferSelectiveDisplayLineTailAction::Exhausted
    }

    pub(crate) fn skip_hidden_indented_lines_after_line_break(
        self,
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) -> BufferSelectiveDisplayHiddenLines {
        let mut hidden_line_count = 0;
        while *byte_idx < self.text.len() {
            let Some(indent) = self.indentation_columns_at(*byte_idx) else {
                break;
            };
            if indent <= self.selective_display {
                break;
            }

            if self.skip_line(byte_idx, charpos) {
                hidden_line_count += 1;
            }
        }

        BufferSelectiveDisplayHiddenLines::new(hidden_line_count)
    }

    pub(crate) fn apply_hidden_indented_lines_after_line_break(
        self,
        byte_idx: &mut usize,
        charpos: &mut i64,
        line_numbers: &mut LineNumberRenderState,
    ) -> BufferSelectiveDisplayHiddenLines {
        if !self.hides_indented_lines_after_line_break(*byte_idx) {
            return BufferSelectiveDisplayHiddenLines::new(0);
        }
        let hidden_lines = self.skip_hidden_indented_lines_after_line_break(byte_idx, charpos);
        hidden_lines.apply_to_line_numbers(line_numbers);
        hidden_lines
    }

    fn indentation_columns_at(self, mut byte_idx: usize) -> Option<i32> {
        if byte_idx >= self.text.len() {
            return None;
        }

        let mut indent = 0i32;
        while byte_idx < self.text.len() {
            match self.text[byte_idx] {
                b' ' => {
                    indent += 1;
                    byte_idx += 1;
                }
                b'\t' => {
                    indent = ((indent / self.tab_width) + 1) * self.tab_width;
                    byte_idx += 1;
                }
                _ => break,
            }
        }
        Some(indent)
    }

    fn skip_line(self, byte_idx: &mut usize, charpos: &mut i64) -> bool {
        while *byte_idx < self.text.len() {
            let Some(source_char) =
                BufferTextSourceStepChar::consume_from_text(self.text, byte_idx, charpos)
            else {
                break;
            };
            if source_char.ch() == '\n' {
                return true;
            }
        }
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextLineBreakSourceAction {
    ch_start_byte_idx: usize,
    charpos: i64,
    next_charpos: i64,
    line_spacing: f32,
}

pub(crate) struct BufferTextLineBreakRenderRequest<'a> {
    source_char: BufferTextSourceStepChar,
    context: BufferTextLineBreakRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferTextLineBreakRenderContext<'a> {
    pub(crate) text: &'a [u8],
    pub(crate) text_start_byte: usize,
    pub(crate) selective_display: i32,
    pub(crate) tab_width: i32,
    pub(crate) active_face_state: &'a DisplayRowActiveFaceState,
    pub(crate) point_charpos: i64,
    pub(crate) char_h: f32,
    pub(crate) extra_line_spacing: f32,
    pub(crate) content_x: f32,
    pub(crate) has_prefix: bool,
    pub(crate) row_geometry_defaults: DisplayRowGeometryDefaults,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) max_rows: usize,
    pub(crate) row_limit: DisplayRowLimit,
    pub(crate) overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
}

impl<'a> BufferTextLineBreakRenderRequest<'a> {
    pub(crate) fn new(
        source_char: BufferTextSourceStepChar,
        context: BufferTextLineBreakRenderContext<'a>,
    ) -> Self {
        debug_assert_eq!(source_char.ch(), '\n');
        Self {
            source_char,
            context,
        }
    }

    pub(crate) fn render_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferTextLineBreakRenderState<'_, '_>,
    ) -> DisplayRowTransitionContinuation {
        let BufferTextLineBreakRenderState {
            byte_idx,
            charpos,
            cursor_info,
            row_geometry,
            trailing_whitespace,
            row_extend,
            box_face,
            source_render,
            x,
            col,
            prefix_request,
            line_numbers,
            hscroll_skip,
            word_wrap,
            row_flags,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
        } = state;
        let mut source_render = source_render;
        let context = self.context;

        let line_break_action = BufferTextLineBreakSourceAction::for_source_step_newline(
            buffer,
            self.source_char,
            context.char_h,
            context.extra_line_spacing,
        );
        {
            let mut overlay_state = OverlayStringRenderState::from_source_render(
                source_render.reborrow(),
                x,
                col,
                row_geometry,
                cursor_info,
                hit_rows,
                hit_row_range,
                row_y_positions,
                face_ids,
            );
            context.overlay_context.render_at(
                buffer,
                *charpos,
                context.active_face_state,
                &mut overlay_state,
            );
        }
        line_break_action.capture_cursor_if_point(
            cursor_info,
            context.active_face_state,
            row_geometry,
            context.point_charpos,
            *x,
            *col,
        );
        line_break_action.apply_before_row_transition(
            row_geometry,
            trailing_whitespace,
            row_extend,
            box_face,
            source_render.output_emitter(),
            context.content_x,
            x,
            charpos,
        );

        let line_break_transition = DisplayRowLineBreakTransitionPlan::line_break();
        let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
            context.row_geometry_defaults,
            context.text_matrix_row_base,
            row_y_positions,
            context.max_rows,
            row_geometry,
            row_flags,
            context.row_limit,
            hit_rows,
            &mut source_render,
        )
        .emit_line_break_then_row_start(
            line_break_transition,
            hit_row_range.range_to(*charpos),
            DisplayRowPosition {
                x_px: *x,
                col: *col,
            },
            line_break_action.line_spacing(),
            DisplayRowTransitionRenderState::new(
                prefix_request,
                context.has_prefix,
                line_numbers,
                hscroll_skip,
                word_wrap,
                trailing_whitespace,
            ),
            col,
        );

        let synced_charpos = buffer
            .layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(
                context.text_start_byte + *byte_idx,
            ))
            .get() as i64;
        let continuation = line_break_action.apply_after_line_break_row_transition(
            row_transition,
            synced_charpos,
            charpos,
            hit_row_range,
            row_geometry,
            box_face,
            context.content_x,
        );
        if continuation.should_break() {
            return continuation;
        }

        BufferSelectiveDisplayContext::new(
            context.text,
            context.selective_display,
            context.tab_width,
        )
        .apply_hidden_indented_lines_after_line_break(byte_idx, charpos, line_numbers);
        DisplayRowTransitionContinuation::Continue
    }
}

impl BufferTextLineBreakSourceAction {
    pub(crate) fn for_newline<B: LayoutBufferView>(
        buffer: &B,
        charpos: i64,
        ch_start_byte_idx: usize,
        char_h: f32,
        extra_line_spacing: f32,
    ) -> Self {
        let text_prop_spacing = RustTextPropAccess::new(buffer).check_line_spacing(charpos, char_h);
        let line_spacing = if text_prop_spacing > 0.0 {
            text_prop_spacing
        } else if extra_line_spacing > 0.0 {
            extra_line_spacing
        } else {
            0.0
        };
        Self {
            ch_start_byte_idx,
            charpos,
            next_charpos: charpos + 1,
            line_spacing,
        }
    }

    pub(crate) fn for_source_step_newline<B: LayoutBufferView>(
        buffer: &B,
        source_char: BufferTextSourceStepChar,
        char_h: f32,
        extra_line_spacing: f32,
    ) -> Self {
        Self::for_newline(
            buffer,
            source_char.start_charpos(),
            source_char.start_byte_idx(),
            char_h,
            extra_line_spacing,
        )
    }

    pub(crate) fn point_matches(self, point_charpos: i64) -> bool {
        point_charpos == self.charpos
    }

    pub(crate) fn next_charpos(self) -> i64 {
        self.next_charpos
    }

    pub(crate) fn line_spacing(self) -> f32 {
        self.line_spacing
    }

    pub(crate) fn apply_before_row_transition(
        self,
        row_geometry: &DisplayRowGeometryState,
        trailing_whitespace: &mut TrailingWhitespaceRenderState,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &mut BoxFaceRowState,
        output_emitter: &mut WindowOutputEmitter,
        content_x: f32,
        x: &mut f32,
        charpos: &mut i64,
    ) {
        trailing_whitespace.reset_after_row_transition();
        row_extend.clear();
        box_face.continue_on_row(row_geometry.current_row_marker(), content_x);
        *charpos = self.next_charpos();
        *x = content_x;
        output_emitter.note_display_buffer_pos(LispCharPos1::new(*charpos));
    }

    pub(crate) fn apply_after_row_transition(
        self,
        row_geometry: &DisplayRowGeometryState,
        box_face: &mut BoxFaceRowState,
        content_x: f32,
    ) {
        box_face.continue_on_row(row_geometry.current_row_marker(), content_x);
    }

    pub(crate) fn apply_after_line_break_row_transition(
        self,
        row_transition: TextMatrixRowTransition,
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        row_geometry: &DisplayRowGeometryState,
        box_face: &mut BoxFaceRowState,
        content_x: f32,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        Self::sync_after_row_transition(synced_charpos, charpos, hit_row_range);
        self.apply_after_row_transition(row_geometry, box_face, content_x);
        DisplayRowTransitionContinuation::Continue
    }

    pub(crate) fn sync_after_row_transition(
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) {
        *charpos = synced_charpos;
        hit_row_range.advance_to(*charpos);
    }

    pub(crate) fn cursor_info(
        self,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        x: f32,
        col: usize,
    ) -> CapturedCursorInfo {
        CapturedCursorInfo::from_active_face_state(
            active_face_state,
            CapturedCursorPlacement::from_row_text_position(
                row_geometry.text_position(x, self.ch_start_byte_idx, col),
                CapturedCursorSlotWidth::FaceChar,
                false,
            ),
        )
    }

    pub(crate) fn capture_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        x: f32,
        col: usize,
    ) {
        if !target.is_missing() || !self.point_matches(point_charpos) {
            return;
        }
        capture_cursor_info(
            target,
            self.cursor_info(active_face_state, row_geometry, x, col),
        );
    }
}

pub(crate) const SYNTHETIC_SOURCE_INVISIBLE_ELLIPSIS: u64 = 3;
pub(crate) const SYNTHETIC_SOURCE_HSCROLL_TRUNCATION: u64 = 4;
pub(crate) const SYNTHETIC_SOURCE_SELECTIVE_ELLIPSIS: u64 = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SyntheticTextSource {
    pub(crate) source_id: u64,
    pub(crate) text: Box<str>,
}

impl SyntheticTextSource {
    #[cfg(test)]
    pub(crate) fn new(source_id: u64, text: impl Into<Box<str>>) -> Self {
        Self {
            source_id,
            text: text.into(),
        }
    }

    fn marker(marker: SyntheticTextMarker) -> Self {
        Self {
            source_id: marker.source_id(),
            text: marker.text().into(),
        }
    }

    pub(crate) fn into_item_source(self, face_id: u32) -> SyntheticTextItemSource {
        SyntheticTextItemSource::new(self.source_id, self.text, RenderFaceRef::FaceId(face_id), 0)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SyntheticTextAppendRequest {
    position: DisplayRowPosition,
    source: SyntheticTextSource,
    face: SyntheticTextAppendFace,
}

#[derive(Clone, Debug)]
pub(crate) enum SyntheticTextAppendFace {
    ActiveFace,
    TextRowMetrics {
        face_id: u32,
        base_face: ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    },
}

impl SyntheticTextAppendRequest {
    #[cfg(test)]
    pub(crate) fn active_source(position: DisplayRowPosition, source: SyntheticTextSource) -> Self {
        Self {
            position,
            source,
            face: SyntheticTextAppendFace::ActiveFace,
        }
    }

    pub(crate) fn active_marker(position: DisplayRowPosition, marker: SyntheticTextMarker) -> Self {
        Self {
            position,
            source: SyntheticTextSource::marker(marker),
            face: SyntheticTextAppendFace::ActiveFace,
        }
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_row_metrics_source(
        position: DisplayRowPosition,
        source: SyntheticTextSource,
        face_id: u32,
        base_face: &ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    ) -> Self {
        Self {
            position,
            source,
            face: SyntheticTextAppendFace::TextRowMetrics {
                face_id,
                base_face: base_face.clone(),
                height_px,
                ascent_px,
                char_width_px,
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_row_metrics_marker(
        position: DisplayRowPosition,
        marker: SyntheticTextMarker,
        face_id: u32,
        base_face: &ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    ) -> Self {
        Self {
            position,
            source: SyntheticTextSource::marker(marker),
            face: SyntheticTextAppendFace::TextRowMetrics {
                face_id,
                base_face: base_face.clone(),
                height_px,
                ascent_px,
                char_width_px,
            },
        }
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        DisplayRowPosition,
        SyntheticTextSource,
        SyntheticTextAppendFace,
    ) {
        (self.position, self.source, self.face)
    }
}

#[derive(Clone)]
pub(crate) struct SyntheticTextAppendContext<'a> {
    face_id: u32,
    base_face: &'a ResolvedFace,
    frame: DisplayRowAppendFrame,
}

impl<'a> SyntheticTextAppendContext<'a> {
    pub(crate) fn new(
        face_id: u32,
        base_face: &'a ResolvedFace,
        frame: DisplayRowAppendFrame,
    ) -> Self {
        Self {
            face_id,
            base_face,
            frame,
        }
    }

    pub(crate) fn append_to_text_row_and_emit(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        position: DisplayRowPosition,
        source: SyntheticTextSource,
    ) -> Option<DisplayRowAppendProgress> {
        append_synthetic_text_to_display_row(
            state,
            self.base_face,
            self.frame.clone(),
            position,
            source,
            self.face_id,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SyntheticTextMarker {
    InvisibleEllipsis,
    HscrollTruncation,
    SelectiveEllipsis,
}

impl SyntheticTextMarker {
    pub(crate) fn source_id(self) -> u64 {
        match self {
            Self::InvisibleEllipsis => SYNTHETIC_SOURCE_INVISIBLE_ELLIPSIS,
            Self::HscrollTruncation => SYNTHETIC_SOURCE_HSCROLL_TRUNCATION,
            Self::SelectiveEllipsis => SYNTHETIC_SOURCE_SELECTIVE_ELLIPSIS,
        }
    }

    pub(crate) fn text(self) -> &'static str {
        match self {
            Self::InvisibleEllipsis | Self::SelectiveEllipsis => "...",
            Self::HscrollTruncation => "$",
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SyntheticTextRowAppendContext<'a> {
    active_face_context: DisplayRowActiveFaceAppendContext<'a, 'a>,
}

impl<'a> SyntheticTextRowAppendContext<'a> {
    pub(crate) fn new(
        append_surface: &'a DisplayRowAppendSurface,
        geometry: &'a DisplayRowGeometryState,
        active_face: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            active_face_context: DisplayRowActiveFaceAppendContext::new(
                append_surface,
                geometry,
                active_face,
                glyph_y_offset,
                default_row_height,
            ),
        }
    }

    fn active_face(
        self,
        face_id: u32,
        base_face: &'a ResolvedFace,
    ) -> SyntheticTextAppendContext<'a> {
        SyntheticTextAppendContext::new(
            face_id,
            base_face,
            self.active_face_context.active_face_frame(),
        )
    }

    fn text_row<'face>(
        self,
        face_id: u32,
        base_face: &'face ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    ) -> SyntheticTextAppendContext<'face> {
        SyntheticTextAppendContext::new(
            face_id,
            base_face,
            self.active_face_context
                .text_row_frame(height_px, ascent_px, char_width_px),
        )
    }

    pub(crate) fn append_request_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        request: SyntheticTextAppendRequest,
    ) -> Option<DisplayRowAppendProgress> {
        let (position, source, face) = request.into_parts();
        match face {
            SyntheticTextAppendFace::ActiveFace => {
                let active_face = self.active_face_context.active_face();
                self.active_face(active_face.face_id(), active_face.resolved_face())
                    .append_to_text_row_and_emit(state, position, source)
            }
            SyntheticTextAppendFace::TextRowMetrics {
                face_id,
                base_face,
                height_px,
                ascent_px,
                char_width_px,
            } => self
                .text_row(face_id, &base_face, height_px, ascent_px, char_width_px)
                .append_to_text_row_and_emit(state, position, source),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BufferSyntheticTextRenderContext<'a> {
    append_surface: &'a DisplayRowAppendSurface,
    active_face: &'a DisplayRowActiveFaceState,
    glyph_y_offset: f32,
    default_row_height: f32,
    default_row_ascent: f32,
    default_char_width: f32,
}

impl<'a> BufferSyntheticTextRenderContext<'a> {
    pub(crate) fn new(
        append_surface: &'a DisplayRowAppendSurface,
        active_face: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
        default_row_ascent: f32,
        default_char_width: f32,
    ) -> Self {
        Self {
            append_surface,
            active_face,
            glyph_y_offset,
            default_row_height,
            default_row_ascent,
            default_char_width,
        }
    }

    pub(crate) fn active_face(self) -> &'a DisplayRowActiveFaceState {
        self.active_face
    }

    fn row_context(
        self,
        geometry: &'a DisplayRowGeometryState,
    ) -> SyntheticTextRowAppendContext<'a> {
        SyntheticTextRowAppendContext::new(
            self.append_surface,
            geometry,
            self.active_face,
            self.glyph_y_offset,
            self.default_row_height,
        )
    }

    pub(crate) fn render_request_to_text_row<'face>(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        geometry: &'a DisplayRowGeometryState,
        request: SyntheticTextAppendRequest,
    ) -> Option<DisplayRowAppendProgress> {
        self.row_context(geometry)
            .append_request_to_text_row_and_emit(state, request)
    }

    #[cfg(test)]
    pub(crate) fn render_active_marker_to_text_row(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        geometry: &'a DisplayRowGeometryState,
        position: DisplayRowPosition,
        marker: SyntheticTextMarker,
    ) -> Option<DisplayRowPosition> {
        self.render_request_to_text_row(
            state,
            geometry,
            SyntheticTextAppendRequest::active_marker(position, marker),
        )
        .map(|progress| progress.end)
    }

    pub(crate) fn hscroll_truncation_request(
        self,
        base_face: ResolvedFace,
        content_x: f32,
    ) -> SyntheticTextAppendRequest {
        SyntheticTextAppendRequest::text_row_metrics_marker(
            DisplayRowPosition {
                x_px: content_x,
                col: 0,
            },
            SyntheticTextMarker::HscrollTruncation,
            BasicFaceId::Default.into(),
            &base_face,
            self.default_row_height,
            self.default_row_ascent,
            self.default_char_width,
        )
    }

    #[cfg(test)]
    pub(crate) fn render_hscroll_truncation_marker_to_text_row(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        geometry: &'a DisplayRowGeometryState,
        content_x: f32,
    ) -> Option<DisplayRowPosition> {
        let request = self.hscroll_truncation_request(state.default_face(), content_x);
        self.render_request_to_text_row(state, geometry, request)
            .map(|progress| progress.end)
    }
}

pub(crate) struct BufferSyntheticTextRenderState<'a> {
    source_render: TextRowSourceRenderState<'a>,
    x: &'a mut f32,
    col: &'a mut usize,
}

impl<'a> BufferSyntheticTextRenderState<'a> {
    pub(crate) fn new(
        source_render: TextRowSourceRenderState<'a>,
        x: &'a mut f32,
        col: &'a mut usize,
    ) -> Self {
        Self {
            source_render,
            x,
            col,
        }
    }

    pub(crate) fn position(&self) -> DisplayRowPosition {
        DisplayRowPosition {
            x_px: *self.x,
            col: *self.col,
        }
    }

    pub(crate) fn append_request_to_text_row<'ctx>(
        &mut self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        request: SyntheticTextAppendRequest,
    ) {
        let Some(progress) = render_context.render_request_to_text_row(
            &mut self.source_render,
            row_geometry,
            request,
        ) else {
            return;
        };
        *self.x = progress.end.x_px;
        *self.col = progress.end.col;
    }

    pub(crate) fn append_hscroll_truncation_marker_to_text_row<'ctx>(
        &mut self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        content_x: f32,
    ) {
        let request =
            render_context.hscroll_truncation_request(self.source_render.default_face(), content_x);
        self.append_request_to_text_row(render_context, row_geometry, request);
        self.source_render.mark_current_text_row_truncated_left();
    }
}

pub(crate) struct BufferLinePrefixRenderContext<'a> {
    values: DisplayRowPrefixValues,
    append_surface: &'a DisplayRowAppendSurface,
    row_geometry: &'a DisplayRowGeometryState,
    active_face_state: &'a DisplayRowActiveFaceState,
    glyph_y_offset: f32,
    default_row_height: f32,
}

pub(crate) struct BufferLinePrefixRenderRequest<'a> {
    context: BufferLinePrefixRenderContext<'a>,
    position: DisplayRowPosition,
}

impl<'a> BufferLinePrefixRenderContext<'a> {
    pub(crate) fn new(
        values: DisplayRowPrefixValues,
        append_surface: &'a DisplayRowAppendSurface,
        row_geometry: &'a DisplayRowGeometryState,
        active_face_state: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            values,
            append_surface,
            row_geometry,
            active_face_state,
            glyph_y_offset,
            default_row_height,
        }
    }

    pub(crate) fn render_requested_to_text_row_and_emit<B: LayoutBufferView>(
        self,
        request: &mut DisplayRowPrefixRequest,
        state: &mut TextRowSourceRenderState<'_>,
        buffer: &B,
        anchor_charpos: i64,
        face_ids: &mut FrameFaceIdAllocator,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        if !request.is_requested() {
            return position;
        }

        let text_props = RustTextPropAccess::new(buffer);
        let line_property = text_props.get_property(anchor_charpos, Value::symbol("line-prefix"));
        let wrap_property = text_props.get_property(anchor_charpos, Value::symbol("wrap-prefix"));
        let source = request.source_from_values(
            self.values.with_properties(line_property, wrap_property),
            CharPos0::new(anchor_charpos as usize),
        );
        request.clear();

        let Some(prefix_source) = source else {
            return position;
        };

        let prefix_base_face =
            state.default_display_string_base_face(buffer, prefix_source.origin(), face_ids);
        LispStringRowAppendContext::new(
            self.append_surface,
            self.row_geometry,
            self.active_face_state,
            self.glyph_y_offset,
            self.default_row_height,
        )
        .render_prefix_source_to_text_row_and_emit(
            state,
            face_ids,
            &prefix_base_face,
            prefix_source,
            position,
        )
    }
}

impl<'a> BufferLinePrefixRenderRequest<'a> {
    pub(crate) fn new(
        context: BufferLinePrefixRenderContext<'a>,
        position: DisplayRowPosition,
    ) -> Self {
        Self { context, position }
    }

    pub(crate) fn render_requested_with_source_state_and_apply<B: LayoutBufferView>(
        self,
        request: &mut DisplayRowPrefixRequest,
        source_render: &mut TextRowSourceRenderState<'_>,
        buffer: &B,
        anchor_charpos: i64,
        face_ids: &mut FrameFaceIdAllocator,
        x: &mut f32,
        col: &mut usize,
    ) {
        let position = self.context.render_requested_to_text_row_and_emit(
            request,
            source_render,
            buffer,
            anchor_charpos,
            face_ids,
            self.position,
        );
        *x = position.x_px;
        *col = position.col;
    }
}

pub(crate) struct BufferCurrentFaceResolutionContext<'a, B: LayoutBufferView> {
    buffer: &'a B,
    face_resolver: &'a FaceResolver,
    measurement_policy: DisplayRowMeasurementPolicy,
    default_resolved: &'a ResolvedFace,
    default_face_char_w: f32,
    default_face_ascent: f32,
    default_face_h: f32,
    char_w: f32,
    char_h: f32,
    font_ascent: f32,
    window_system: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferSourceItemLayoutResolutionContext<'a> {
    measurement_policy: DisplayRowMeasurementPolicy,
    default_resolved: &'a ResolvedFace,
    default_face_char_w: f32,
    default_face_ascent: f32,
    default_face_h: f32,
    char_w: f32,
    char_h: f32,
    font_ascent: f32,
    window_system: bool,
}

impl<'a, B: LayoutBufferView> Clone for BufferCurrentFaceResolutionContext<'a, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, B: LayoutBufferView> Copy for BufferCurrentFaceResolutionContext<'a, B> {}

impl<'a, B: LayoutBufferView> BufferCurrentFaceResolutionContext<'a, B> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        buffer: &'a B,
        face_resolver: &'a FaceResolver,
        measurement_policy: DisplayRowMeasurementPolicy,
        default_resolved: &'a ResolvedFace,
        default_face_char_w: f32,
        default_face_ascent: f32,
        default_face_h: f32,
        char_w: f32,
        char_h: f32,
        font_ascent: f32,
        window_system: bool,
    ) -> Self {
        Self {
            buffer,
            face_resolver,
            measurement_policy,
            default_resolved,
            default_face_char_w,
            default_face_ascent,
            default_face_h,
            char_w,
            char_h,
            font_ascent,
            window_system,
        }
    }

    pub(crate) fn resolve_at_checkpoint(
        &self,
        state: &mut BufferCurrentFaceResolutionState<'_, '_>,
        charpos: i64,
    ) -> bool {
        if !state.face_scan.should_resolve_at(charpos as usize) {
            return false;
        }

        let origin = DisplayOrigin::BufferText {
            charpos: neovm_core::buffer::CharPos0::new(charpos as usize),
        };
        let resolved = self.face_resolver.default_base_face_for_origin(
            Some(self.buffer),
            &origin,
            state.face_scan.next_check_mut(),
        );
        let face_id = state.face_ids.allocate();
        let resolved_extend = resolved.extend;
        let resolved_bg = resolved.bg;
        let resolved_box_type = resolved.box_type;
        *state.active_face_state = state.source_render.resolve_and_install_measured_face(
            self.measurement_policy,
            face_id,
            resolved,
            self.window_system,
            self.char_w,
            DisplayRowFallbackMetrics::from_default_face_extents(
                self.char_w,
                self.char_h,
                self.font_ascent,
            ),
        );
        let face_metrics = state.active_face_state.metrics();
        state
            .row_geometry
            .include_row_extents(face_metrics.row_height, face_metrics.ascent);

        if resolved_extend {
            let ext_bg = Color::from_pixel(resolved_bg);
            state
                .row_extend
                .activate(state.row_geometry.current_row_marker(), (ext_bg, face_id));
        }

        if state.box_face.is_active() && resolved_box_type == 0 {
            state.box_face.clear();
        }
        if resolved_box_type > 0 {
            state
                .box_face
                .activate(state.row_geometry.current_row_marker(), state.x);
        }
        true
    }

    pub(crate) fn source_item_layout_resolution_context(
        self,
    ) -> BufferSourceItemLayoutResolutionContext<'a> {
        BufferSourceItemLayoutResolutionContext {
            measurement_policy: self.measurement_policy,
            default_resolved: self.default_resolved,
            default_face_char_w: self.default_face_char_w,
            default_face_ascent: self.default_face_ascent,
            default_face_h: self.default_face_h,
            char_w: self.char_w,
            char_h: self.char_h,
            font_ascent: self.font_ascent,
            window_system: self.window_system,
        }
    }

    pub(crate) fn source_resolve_params(
        self,
        display_host: Option<&'a dyn DisplayHost>,
    ) -> DisplaySourceResolveParams<'a> {
        DisplaySourceResolveParams::new(
            DisplaySourceFaceBasis::new(
                self.face_resolver,
                u32::from(BasicFaceId::Default),
                self.default_resolved,
                DisplaySourceFallbackMetrics::new(
                    self.default_face_char_w,
                    self.default_face_ascent,
                    self.default_face_h,
                ),
            ),
            display_host,
        )
    }

    pub(crate) fn install_pending_source_faces(
        self,
        source_render: &mut TextRowSourceRenderState<'_>,
        row_geometry: &mut DisplayRowGeometryState,
        pending_faces: Vec<PendingDisplaySourceFace>,
    ) {
        let fallback_metrics = DisplayRowFallbackMetrics::from_default_face_extents(
            self.char_w,
            self.char_h,
            self.font_ascent,
        );
        for pending in pending_faces {
            let active_face = source_render.resolve_and_install_measured_face(
                self.measurement_policy,
                pending.face_id,
                pending.resolved,
                self.window_system,
                self.char_w,
                fallback_metrics,
            );
            let metrics = active_face.metrics();
            row_geometry.include_row_extents(metrics.row_height, metrics.ascent);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_at_checkpoint_with_source_state(
        &self,
        source_render: &mut TextRowSourceRenderState<'_>,
        face_scan: &mut FaceScanCheckpoint,
        face_ids: &mut FrameFaceIdAllocator,
        active_face_state: &mut DisplayRowActiveFaceState,
        row_geometry: &mut DisplayRowGeometryState,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &mut BoxFaceRowState,
        x: f32,
        charpos: i64,
    ) -> bool {
        self.resolve_at_checkpoint(
            &mut BufferCurrentFaceResolutionState::new(
                source_render,
                face_scan,
                face_ids,
                active_face_state,
                row_geometry,
                row_extend,
                box_face,
                x,
            ),
            charpos,
        )
    }
}

impl BufferSourceItemLayoutResolutionContext<'_> {
    pub(crate) fn resolve_source_item_layout_for_active_face(
        &self,
        source_render: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        row_geometry: &mut DisplayRowGeometryState,
        active_face_state: &DisplayRowActiveFaceState,
        item: &mut DisplayItem,
    ) -> DisplayRowActiveFaceState {
        if matches!(item.face, RenderFaceRef::Inherit) {
            item.face = RenderFaceRef::FaceId(active_face_state.face_id());
        }

        let Some(factor) = item
            .layout
            .height
            .filter(|factor| factor.is_finite() && *factor > 0.0)
        else {
            return active_face_state.clone();
        };

        item.layout.height = None;
        let Some(resolved) = height_adjusted_face(
            active_face_state.resolved_face(),
            DisplayHeightFaceBasis {
                canonical_face: self.default_resolved,
                base_face: self.default_resolved,
                fallback_char_width: self.default_face_char_w,
                fallback_ascent: self.default_face_ascent,
                fallback_row_height: self.default_face_h,
            },
            factor,
        ) else {
            return active_face_state.clone();
        };

        let face_id = face_ids.allocate();
        item.face = RenderFaceRef::FaceId(face_id);
        let resolved_active_face = source_render.resolve_and_install_measured_face(
            self.measurement_policy,
            face_id,
            resolved,
            self.window_system,
            self.char_w,
            DisplayRowFallbackMetrics::from_default_face_extents(
                self.char_w,
                self.char_h,
                self.font_ascent,
            ),
        );
        let metrics = resolved_active_face.metrics();
        row_geometry.include_row_extents(metrics.row_height, metrics.ascent);
        resolved_active_face
    }
}

pub(crate) struct BufferCurrentFaceResolutionState<'a, 'source> {
    source_render: &'a mut TextRowSourceRenderState<'source>,
    face_scan: &'a mut FaceScanCheckpoint,
    face_ids: &'a mut FrameFaceIdAllocator,
    active_face_state: &'a mut DisplayRowActiveFaceState,
    row_geometry: &'a mut DisplayRowGeometryState,
    row_extend: &'a mut DisplayRowScopedValue<(Color, u32)>,
    box_face: &'a mut BoxFaceRowState,
    x: f32,
}

impl<'a, 'source> BufferCurrentFaceResolutionState<'a, 'source> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        source_render: &'a mut TextRowSourceRenderState<'source>,
        face_scan: &'a mut FaceScanCheckpoint,
        face_ids: &'a mut FrameFaceIdAllocator,
        active_face_state: &'a mut DisplayRowActiveFaceState,
        row_geometry: &'a mut DisplayRowGeometryState,
        row_extend: &'a mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &'a mut BoxFaceRowState,
        x: f32,
    ) -> Self {
        Self {
            source_render,
            face_scan,
            face_ids,
            active_face_state,
            row_geometry,
            row_extend,
            box_face,
            x,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferDisplayPropertyTextWalkOutcome {
    Continue,
}

pub(crate) struct BufferDisplayPropertyCheckpointRenderRequest<'a, B: LayoutBufferView> {
    context: BufferDisplayPropertyCheckpointRenderContext<'a, B>,
}

pub(crate) struct BufferDisplayPropertyCheckpointRenderContext<'a, B: LayoutBufferView> {
    pub(crate) buffer: &'a B,
    pub(crate) charpos: i64,
}

pub(crate) struct BufferDisplayPropertyCheckpointRenderState<'emit> {
    checkpoints: &'emit mut TextPropertyScanCheckpoints,
}

impl<'emit> BufferDisplayPropertyCheckpointRenderState<'emit> {
    pub(crate) fn new(checkpoints: &'emit mut TextPropertyScanCheckpoints) -> Self {
        Self { checkpoints }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferDisplayPropertyTextReplacementOutcome {
    pub(crate) replacement: DisplayPropertyReplacementAppendOutcome,
    pub(crate) skip_to: i64,
}

pub(crate) enum BufferDisplayPropertyTextReplacementRenderOutcome {
    Continue,
    Fallback(BufferTextSourceItem),
    Stop,
}

pub(crate) struct BufferDisplayPropertyTextReplacementRenderRequest<'a> {
    replacement: BufferTextReplacementItem,
    text_start_byte: usize,
    text: &'a [u8],
    content_x: f32,
    params: &'a WindowParams,
    glyph_y_offset: f32,
    default_row_height: f32,
    active_face_state: &'a DisplayRowActiveFaceState,
    point_charpos: i64,
}

pub(crate) struct BufferDisplayPropertyTextReplacementRenderState<'emit> {
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
    pub(crate) append_surface: &'emit DisplayRowAppendSurface,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
}

impl BufferDisplayPropertyTextWalkOutcome {
    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        false
    }
}

impl<'a, B: LayoutBufferView> BufferDisplayPropertyCheckpointRenderRequest<'a, B> {
    pub(crate) fn new(context: BufferDisplayPropertyCheckpointRenderContext<'a, B>) -> Self {
        Self { context }
    }

    pub(crate) fn render_and_apply(
        self,
        state: BufferDisplayPropertyCheckpointRenderState<'_>,
    ) -> BufferDisplayPropertyTextWalkOutcome {
        let BufferDisplayPropertyCheckpointRenderState { checkpoints } = state;
        let context = self.context;
        if checkpoints.should_check_display(context.charpos) {
            let text_props = RustTextPropAccess::new(context.buffer);
            let (_, next_change) = text_props.check_display_prop(context.charpos);
            checkpoints.record_display_next(next_change);
        }
        BufferDisplayPropertyTextWalkOutcome::Continue
    }
}

impl<'a> BufferDisplayPropertyTextReplacementRenderRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        replacement: BufferTextReplacementItem,
        text_start_byte: usize,
        text: &'a [u8],
        content_x: f32,
        params: &'a WindowParams,
        glyph_y_offset: f32,
        default_row_height: f32,
        active_face_state: &'a DisplayRowActiveFaceState,
        point_charpos: i64,
    ) -> Self {
        Self {
            replacement,
            text_start_byte,
            text,
            content_x,
            params,
            glyph_y_offset,
            default_row_height,
            active_face_state,
            point_charpos,
        }
    }

    pub(crate) fn render_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        mut state: BufferDisplayPropertyTextReplacementRenderState<'_>,
    ) -> BufferDisplayPropertyTextReplacementRenderOutcome {
        let Some(source_text) = self
            .replacement
            .source_text(self.text_start_byte, self.text)
        else {
            return BufferDisplayPropertyTextReplacementRenderOutcome::Stop;
        };
        let display_property = classify_display_property(self.replacement.value());
        let append_request =
            state
                .source_render
                .with_font_metrics_and_display_host(|font_metrics, host| {
                    DisplayPropertyReplacementAppendRequestResolver::for_typed_replacement(
                        &display_property,
                        self.replacement.replacement_source(),
                        self.replacement.value(),
                        self.replacement.start_charpos0(),
                        source_text,
                        self.active_face_state,
                        *state.x,
                        self.content_x,
                        self.params,
                        self.glyph_y_offset,
                        self.default_row_height,
                        DisplayRowPosition {
                            x_px: *state.x,
                            col: *state.col,
                        },
                    )
                    .resolve(font_metrics, host)
                });
        let Some(request) = append_request else {
            let Some(source_item) = self.replacement.fallback_source_item(
                self.text_start_byte,
                self.text,
                RenderFaceRef::FaceId(self.active_face_state.face_id()),
            ) else {
                return BufferDisplayPropertyTextReplacementRenderOutcome::Stop;
            };
            return BufferDisplayPropertyTextReplacementRenderOutcome::Fallback(source_item);
        };
        let outcome = request.append_to_text_row(
            buffer,
            &mut state.source_render.reborrow(),
            state.face_ids,
            state.append_surface,
            state.row_geometry,
            self.active_face_state,
        );
        let replacement_outcome = BufferDisplayPropertyTextReplacementOutcome {
            replacement: outcome,
            skip_to: self.replacement.end_charpos(),
        };
        replacement_outcome.capture_cursor_info_if_point(
            state.cursor_info,
            self.active_face_state,
            state.row_geometry,
            self.point_charpos,
            self.replacement.start_charpos(),
            *state.byte_idx,
        );
        replacement_outcome.apply_to_walk_state(
            self.text,
            state.byte_idx,
            state.charpos,
            state.x,
            state.col,
        );
        BufferDisplayPropertyTextReplacementRenderOutcome::Continue
    }
}

impl BufferDisplayPropertyTextReplacementOutcome {
    pub(crate) fn point_in_replacement(self, point_charpos: i64, start_charpos: i64) -> bool {
        point_charpos >= start_charpos && point_charpos < self.skip_to
    }

    pub(crate) fn start_position(self) -> DisplayRowPosition {
        self.replacement.start_position()
    }

    pub(crate) fn end_position(self) -> DisplayRowPosition {
        self.replacement.end_position()
    }

    pub(crate) fn skip_covered_buffer_text(
        self,
        text: &[u8],
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) {
        skip_text_to_charpos(text, byte_idx, charpos, self.skip_to);
    }

    pub(crate) fn capture_cursor_info_if_point(
        self,
        cursor_info: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        start_charpos: i64,
        byte_idx: usize,
    ) {
        if cursor_info.is_missing() && self.point_in_replacement(point_charpos, start_charpos) {
            let start_position = self.start_position();
            capture_cursor_info(
                cursor_info,
                self.cursor_info(
                    active_face_state,
                    row_geometry.text_position(start_position.x_px, byte_idx, start_position.col),
                ),
            );
        }
    }

    pub(crate) fn apply_to_walk_state(
        self,
        text: &[u8],
        byte_idx: &mut usize,
        charpos: &mut i64,
        x: &mut f32,
        col: &mut usize,
    ) {
        let position = self.end_position();
        *x = position.x_px;
        *col = position.col;
        self.skip_covered_buffer_text(text, byte_idx, charpos);
    }

    #[cfg(test)]
    pub(crate) fn skip_to(self) -> i64 {
        self.skip_to
    }

    pub(crate) fn cursor_info(
        self,
        active_face_state: &DisplayRowActiveFaceState,
        position: DisplayRowTextPosition,
    ) -> CapturedCursorInfo {
        self.replacement.cursor_info(active_face_state, position)
    }
}

pub(crate) struct BufferTextOverflowRenderState<'a, 'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) col: &'emit mut usize,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) x: &'emit mut f32,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) face_scan: &'emit mut FaceScanCheckpoint,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
}

pub(crate) struct BufferTextSpecialOverflowRenderState<'a, 'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) col: &'emit mut usize,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) x: &'emit mut f32,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
}

pub(crate) struct BufferTextSourceCharRenderRequest<'a> {
    source_char: BufferTextSourceStepChar,
    source_item: DisplayItem,
    context: BufferTextSourceCharRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferTextSourceCharRenderContext<'a> {
    pub(crate) layout_resolution_context: BufferSourceItemLayoutResolutionContext<'a>,
    pub(crate) text: &'a [u8],
    pub(crate) text_start_byte: usize,
    pub(crate) buffer_id: BufferId,
    pub(crate) append_surface: &'a DisplayRowAppendSurface,
    pub(crate) overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
    pub(crate) active_face_state: &'a DisplayRowActiveFaceState,
    pub(crate) params: &'a WindowParams,
    pub(crate) glyph_y_offset: f32,
    pub(crate) char_h: f32,
    pub(crate) point_charpos: i64,
    pub(crate) row_visibility_limit: DisplayRowVisibilityLimit,
    pub(crate) content_x: f32,
    pub(crate) has_prefix: bool,
    pub(crate) row_geometry_defaults: DisplayRowGeometryDefaults,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) max_rows: usize,
    pub(crate) row_limit: DisplayRowLimit,
}

pub(crate) struct BufferTextSourceCharRenderRequestState<'a, 'emit> {
    pub(crate) append_state: &'emit mut BufferTextRowAppendState,
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) col: &'emit mut usize,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) x: &'emit mut f32,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) face_scan: &'emit mut FaceScanCheckpoint,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum BufferTextSourceCharOverflowAction {
    Fits,
    Truncate {
        transition: DisplayRowOverflowTransitionPlan,
    },
    WordWrap {
        break_candidate: WordWrapBreakCandidate,
        transition: DisplayRowOverflowTransitionPlan,
    },
    CharacterWrap {
        transition: DisplayRowOverflowTransitionPlan,
    },
}

impl BufferTextSourceCharOverflowAction {
    pub(crate) fn for_decision(decision: BufferTextRowOverflowDecision) -> Self {
        match decision {
            BufferTextRowOverflowDecision::Fits => Self::Fits,
            BufferTextRowOverflowDecision::Truncate => Self::Truncate {
                transition: DisplayRowOverflowTransitionPlan::truncation(
                    TextRowTransitionStatePolicy::truncation(),
                ),
            },
            BufferTextRowOverflowDecision::WordWrap { break_candidate } => Self::WordWrap {
                break_candidate,
                transition: DisplayRowOverflowTransitionPlan::visual_wrap(
                    TextRowTransitionStatePolicy::visual_wrap(),
                ),
            },
            BufferTextRowOverflowDecision::CharacterWrap => Self::CharacterWrap {
                transition: DisplayRowOverflowTransitionPlan::visual_wrap(
                    TextRowTransitionStatePolicy::character_wrap(),
                ),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextOverflowRenderRequest<'a> {
    prepared_append: &'a BufferTextSourceCharPreparedAppend,
    source_step_char: BufferTextSourceStepChar,
    context: BufferTextOverflowRenderContext,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextOverflowRenderContext {
    pub(crate) ch: char,
    pub(crate) right_edge_px: f32,
    pub(crate) wrap_mode: LineWrapMode,
    pub(crate) word_wrap: WordWrapRenderState,
    pub(crate) row_visibility_limit: DisplayRowVisibilityLimit,
    pub(crate) content_x: f32,
    pub(crate) has_prefix: bool,
    pub(crate) row_geometry_defaults: DisplayRowGeometryDefaults,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) max_rows: usize,
    pub(crate) row_limit: DisplayRowLimit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextOverflowRenderOutcome {
    Fits,
    Transition(DisplayRowTransitionContinuation),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextSourceAppendContinuation {
    Rendered,
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextSourceCharRenderOutcome {
    Rendered,
    ContinueBufferWalk,
    Stop,
}

pub(crate) struct BufferTextSourceCharRenderState<'a> {
    pub(crate) source_render: TextRowSourceRenderState<'a>,
    pub(crate) trailing_whitespace: &'a mut TrailingWhitespaceRenderState,
    pub(crate) word_wrap: &'a mut WordWrapRenderState,
    pub(crate) x: &'a mut f32,
    pub(crate) col: &'a mut usize,
    pub(crate) charpos: &'a mut i64,
}

impl<'a> BufferTextSourceCharRenderState<'a> {
    pub(crate) fn new(
        source_render: TextRowSourceRenderState<'a>,
        trailing_whitespace: &'a mut TrailingWhitespaceRenderState,
        word_wrap: &'a mut WordWrapRenderState,
        x: &'a mut f32,
        col: &'a mut usize,
        charpos: &'a mut i64,
    ) -> Self {
        Self {
            source_render,
            trailing_whitespace,
            word_wrap,
            x,
            col,
            charpos,
        }
    }
}

pub(crate) struct BufferTextSpecialSourceCharRenderState<'a> {
    pub(crate) face_ids: &'a mut FrameFaceIdAllocator,
    pub(crate) source_render: TextRowSourceRenderState<'a>,
    pub(crate) face_scan: &'a mut FaceScanCheckpoint,
    pub(crate) word_wrap: &'a mut WordWrapRenderState,
    pub(crate) x: &'a mut f32,
    pub(crate) col: &'a mut usize,
    pub(crate) charpos: &'a mut i64,
}

impl<'a> BufferTextSpecialSourceCharRenderState<'a> {
    pub(crate) fn new(
        face_ids: &'a mut FrameFaceIdAllocator,
        source_render: TextRowSourceRenderState<'a>,
        face_scan: &'a mut FaceScanCheckpoint,
        word_wrap: &'a mut WordWrapRenderState,
        x: &'a mut f32,
        col: &'a mut usize,
        charpos: &'a mut i64,
    ) -> Self {
        Self {
            face_ids,
            source_render,
            face_scan,
            word_wrap,
            x,
            col,
            charpos,
        }
    }
}

impl BufferTextSourceAppendContinuation {
    pub(crate) fn should_break(self) -> bool {
        matches!(self, Self::Stopped)
    }
}

impl BufferTextSourceCharRenderOutcome {
    pub(crate) fn should_break(self) -> bool {
        matches!(self, Self::Stop)
    }

    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(self, Self::ContinueBufferWalk)
    }
}

impl BufferTextOverflowRenderOutcome {
    pub(crate) fn should_break(self) -> bool {
        matches!(
            self,
            Self::Transition(
                DisplayRowTransitionContinuation::Exhausted
                    | DisplayRowTransitionContinuation::Hidden
            )
        )
    }

    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(
            self,
            Self::Transition(DisplayRowTransitionContinuation::Continue)
        )
    }
}

impl<'a> BufferTextSourceCharRenderRequest<'a> {
    pub(crate) fn new(
        source_step_char: BufferTextSourceStepChar,
        source_item: DisplayItem,
        context: BufferTextSourceCharRenderContext<'a>,
    ) -> Self {
        debug_assert_ne!(source_step_char.ch(), '\n');
        Self {
            source_char: source_step_char,
            source_item,
            context,
        }
    }

    pub(crate) fn render_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferTextSourceCharRenderRequestState<'_, '_>,
    ) -> BufferTextSourceCharRenderOutcome {
        let BufferTextSourceCharRenderRequestState {
            append_state,
            byte_idx,
            charpos,
            col,
            source_render,
            row_extend,
            x,
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
        } = state;
        let mut source_render = source_render;
        let context = self.context;
        let mut source_item = self.source_item;
        let active_face_state = context
            .layout_resolution_context
            .resolve_source_item_layout_for_active_face(
                &mut source_render,
                face_ids,
                row_geometry,
                context.active_face_state,
                &mut source_item,
            );
        let source_end_charpos = source_item
            .span
            .buffer_end_charpos()
            .map(|char_pos| char_pos.get() as i64);

        let source_step_char = self.source_char;
        let ch = source_step_char.ch();
        source_step_char.record_word_wrap_candidate(word_wrap, source_render.output_emitter());

        let buffer_source_char = source_step_char.source_char(context.params.nobreak_char_display);
        let buffer_row_append_context = BufferTextRowAppendContext::new(
            buffer,
            context.buffer_id,
            context.append_surface,
            &active_face_state,
            context.glyph_y_offset,
            context.char_h,
        );
        let append_position = DisplayRowPosition {
            x_px: *x,
            col: *col,
        };
        let append_geometry = *row_geometry;

        let prepared_append = {
            let mut preparation_state = BufferTextSourceCharPreparationState::from_source_render(
                append_state,
                &mut source_render,
            );
            buffer_row_append_context.prepare_source_item_for_current_text_row(
                BufferTextSourceDisplayItemPreparationRequest::new(
                    append_geometry,
                    &buffer_source_char,
                    context.text,
                    source_step_char.start_byte_idx(),
                    append_position,
                    &source_item,
                ),
                &mut preparation_state,
            )
        };

        let prepared_append = match prepared_append {
            BufferTextPreparedSourceCharAppend::Special(special_prepared_append) => {
                let special_overflow_outcome = BufferTextSpecialOverflowRenderRequest::new(
                    &special_prepared_append,
                    BufferTextSpecialOverflowRenderContext {
                        text: context.text,
                        text_start_byte: context.text_start_byte,
                        x_px: *x,
                        right_edge_px: context.append_surface.full_text_right_edge(),
                        wrap_mode: context.params.wrap_mode,
                        row_visibility_limit: context.row_visibility_limit,
                        content_x: context.content_x,
                        has_prefix: context.has_prefix,
                        row_geometry_defaults: context.row_geometry_defaults,
                        text_matrix_row_base: context.text_matrix_row_base,
                        max_rows: context.max_rows,
                        row_limit: context.row_limit,
                    },
                )
                .render_if_needed_and_apply(
                    buffer,
                    BufferTextSpecialOverflowRenderState {
                        byte_idx,
                        charpos,
                        col,
                        source_render: source_render.reborrow(),
                        row_extend,
                        x,
                        line_numbers,
                        row_geometry,
                        row_flags,
                        hit_rows,
                        hit_row_range,
                        prefix_request,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                        row_y_positions,
                    },
                );
                if special_overflow_outcome.should_break() {
                    return BufferTextSourceCharRenderOutcome::Stop;
                }
                if special_overflow_outcome.should_continue_buffer_walk() {
                    return BufferTextSourceCharRenderOutcome::ContinueBufferWalk;
                }

                if special_prepared_append
                    .append_to_text_row_and_apply(
                        &buffer_row_append_context,
                        row_geometry,
                        context.params,
                        &mut BufferTextSpecialSourceCharRenderState::new(
                            face_ids,
                            source_render.reborrow(),
                            face_scan,
                            word_wrap,
                            x,
                            col,
                            charpos,
                        ),
                    )
                    .should_break()
                {
                    return BufferTextSourceCharRenderOutcome::Stop;
                }
                return BufferTextSourceCharRenderOutcome::ContinueBufferWalk;
            }
            BufferTextPreparedSourceCharAppend::Text(prepared_append) => prepared_append,
        };

        prepared_append
            .update_cursor_info_for_main_char(cursor_info, source_step_char.start_byte_idx());
        let overflow_outcome = BufferTextOverflowRenderRequest::new(
            &prepared_append,
            source_step_char,
            BufferTextOverflowRenderContext {
                ch,
                right_edge_px: context.append_surface.right_edge(),
                wrap_mode: context.params.wrap_mode,
                word_wrap: *word_wrap,
                row_visibility_limit: context.row_visibility_limit,
                content_x: context.content_x,
                has_prefix: context.has_prefix,
                row_geometry_defaults: context.row_geometry_defaults,
                text_matrix_row_base: context.text_matrix_row_base,
                max_rows: context.max_rows,
                row_limit: context.row_limit,
            },
        )
        .render_if_needed_and_apply(
            context.text,
            BufferTextOverflowRenderState {
                byte_idx,
                charpos,
                col,
                source_render: source_render.reborrow(),
                row_extend,
                x,
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
            },
        );
        if overflow_outcome.should_break() {
            return BufferTextSourceCharRenderOutcome::Stop;
        }
        if overflow_outcome.should_continue_buffer_walk() {
            return BufferTextSourceCharRenderOutcome::ContinueBufferWalk;
        }

        {
            let mut overlay_state = OverlayStringRenderState::from_source_render(
                source_render.reborrow(),
                x,
                col,
                row_geometry,
                cursor_info,
                hit_rows,
                hit_row_range,
                row_y_positions,
                face_ids,
            );
            context.overlay_context.render_at(
                buffer,
                *charpos,
                &active_face_state,
                &mut overlay_state,
            );
        }

        prepared_append.capture_cursor_info_for_main_char_if_point(
            cursor_info,
            &active_face_state,
            row_geometry,
            *x,
            source_step_char.start_byte_idx(),
            *col,
            ch == '\t',
            *charpos,
            context.point_charpos,
        );
        if cursor_info.is_missing()
            && source_end_charpos
                .is_some_and(|end| context.point_charpos > *charpos && context.point_charpos < end)
        {
            capture_cursor_info(
                cursor_info,
                prepared_append.cursor_info_for_main_char(
                    &active_face_state,
                    row_geometry.text_position(*x, source_step_char.start_byte_idx(), *col),
                    ch == '\t',
                ),
            );
        }

        if prepared_append
            .append_to_text_row_and_apply(
                &buffer_row_append_context,
                &append_geometry,
                ch,
                &mut BufferTextSourceCharRenderState::new(
                    source_render.reborrow(),
                    trailing_whitespace,
                    word_wrap,
                    x,
                    col,
                    charpos,
                ),
            )
            .should_break()
        {
            return BufferTextSourceCharRenderOutcome::Stop;
        }
        if let Some(end_charpos) = source_end_charpos {
            *charpos = (*charpos).max(end_charpos);
        }

        BufferTextSourceCharRenderOutcome::Rendered
    }
}

impl<'a> BufferTextOverflowRenderRequest<'a> {
    pub(crate) fn new(
        prepared_append: &'a BufferTextSourceCharPreparedAppend,
        source_step_char: BufferTextSourceStepChar,
        context: BufferTextOverflowRenderContext,
    ) -> Self {
        Self {
            prepared_append,
            source_step_char,
            context,
        }
    }

    pub(crate) fn render_if_needed_and_apply(
        self,
        text: &[u8],
        state: BufferTextOverflowRenderState<'_, '_>,
    ) -> BufferTextOverflowRenderOutcome {
        let BufferTextOverflowRenderState {
            byte_idx,
            charpos,
            col,
            source_render,
            row_extend,
            x,
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
        } = state;
        let mut source_render = source_render;
        let context = self.context;

        match self.prepared_append.overflow_action(
            context.ch,
            context.right_edge_px,
            context.wrap_mode,
            context.word_wrap,
        ) {
            BufferTextSourceCharOverflowAction::Fits => BufferTextOverflowRenderOutcome::Fits,
            BufferTextSourceCharOverflowAction::Truncate { transition } => {
                let truncation_skip =
                    BufferTextTruncationSkipAction::consume_source_step_char_and_rest_of_line(
                        text, byte_idx, charpos,
                    );
                truncation_skip.apply_before_row_transition(
                    line_numbers,
                    row_extend,
                    x,
                    context.content_x,
                );
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    context.row_geometry_defaults,
                    context.text_matrix_row_base,
                    row_y_positions,
                    context.max_rows,
                    row_geometry,
                    row_flags,
                    context.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow_then_row_start(
                    transition,
                    hit_row_range.range_to(*charpos),
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
                    DisplayRowTransitionRenderState::new(
                        prefix_request,
                        context.has_prefix,
                        line_numbers,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                    ),
                    col,
                );
                BufferTextOverflowRenderOutcome::Transition(
                    truncation_skip.transition_continuation(row_transition),
                )
            }
            BufferTextSourceCharOverflowAction::WordWrap {
                break_candidate: wrap_break,
                transition,
            } => {
                let word_wrap_action = BufferTextWordWrapSourceAction::new(wrap_break);
                word_wrap_action.apply_before_row_transition(
                    source_render.output_emitter(),
                    byte_idx,
                    charpos,
                    col,
                    row_extend,
                    x,
                    context.content_x,
                );
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    context.row_geometry_defaults,
                    context.text_matrix_row_base,
                    row_y_positions,
                    context.max_rows,
                    row_geometry,
                    row_flags,
                    context.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow(
                    transition,
                    hit_row_range.range_to(*charpos),
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
                );
                BufferTextOverflowRenderOutcome::Transition(
                    word_wrap_action.apply_after_row_transition_and_prefix(
                        row_transition,
                        transition,
                        charpos,
                        hit_row_range,
                        face_scan,
                        row_geometry,
                        context.row_visibility_limit,
                        DisplayRowTransitionRenderState::new(
                            prefix_request,
                            context.has_prefix,
                            line_numbers,
                            hscroll_skip,
                            word_wrap,
                            trailing_whitespace,
                        ),
                    ),
                )
            }
            BufferTextSourceCharOverflowAction::CharacterWrap { transition } => {
                let character_wrap_action =
                    BufferTextCharacterWrapSourceAction::from_source_step_char(
                        self.source_step_char,
                    );
                character_wrap_action.apply_before_row_transition(row_extend, x, context.content_x);
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    context.row_geometry_defaults,
                    context.text_matrix_row_base,
                    row_y_positions,
                    context.max_rows,
                    row_geometry,
                    row_flags,
                    context.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow_then_row_start(
                    transition,
                    hit_row_range.range_to(*charpos),
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
                    DisplayRowTransitionRenderState::new(
                        prefix_request,
                        context.has_prefix,
                        line_numbers,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                    ),
                    col,
                );
                BufferTextOverflowRenderOutcome::Transition(
                    character_wrap_action.apply_after_visible_row_transition(
                        row_transition,
                        byte_idx,
                        charpos,
                        hit_row_range,
                        face_scan,
                        row_geometry,
                        context.row_visibility_limit,
                    ),
                )
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextTruncationSkipAction {
    pub(crate) charpos: i64,
    pub(crate) reached_line_break: bool,
}

impl BufferTextTruncationSkipAction {
    pub(crate) fn consume_source_step_char_and_rest_of_line(
        text: &[u8],
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) -> Self {
        *charpos += 1;
        let reached_line_break = skip_to_newline(text, byte_idx, charpos);
        Self {
            charpos: *charpos,
            reached_line_break,
        }
    }

    #[cfg(test)]
    pub(crate) fn charpos(self) -> i64 {
        self.charpos
    }

    pub(crate) fn reached_line_break(self) -> bool {
        self.reached_line_break
    }

    pub(crate) fn apply_before_row_transition(
        self,
        line_numbers: &mut LineNumberRenderState,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        x: &mut f32,
        content_x: f32,
    ) {
        if self.reached_line_break() {
            line_numbers.advance_line();
        }
        *x = content_x;
        row_extend.clear();
    }

    pub(crate) fn sync_after_row_transition(
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) {
        *charpos = synced_charpos;
        hit_row_range.advance_to(*charpos);
    }

    pub(crate) fn transition_continuation(
        self,
        row_transition: TextMatrixRowTransition,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            DisplayRowTransitionContinuation::Exhausted
        } else {
            DisplayRowTransitionContinuation::Continue
        }
    }

    pub(crate) fn sync_after_row_transition_if_visible(
        self,
        row_transition: TextMatrixRowTransition,
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        Self::sync_after_row_transition(synced_charpos, charpos, hit_row_range);
        DisplayRowTransitionContinuation::Continue
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWordWrapSourceAction {
    break_candidate: WordWrapBreakCandidate,
}

impl BufferTextWordWrapSourceAction {
    pub(crate) fn new(break_candidate: WordWrapBreakCandidate) -> Self {
        Self { break_candidate }
    }

    pub(crate) fn restore_row_output_progress(self, output_emitter: &mut WindowOutputEmitter) {
        output_emitter.truncate_display_points(self.break_candidate.display_point_count());
        let (row_first_display_pos, row_last_display_pos) =
            self.break_candidate.row_display_positions();
        output_emitter
            .restore_current_row_display_positions(row_first_display_pos, row_last_display_pos);
    }

    pub(crate) fn rewind_source_state(
        self,
        byte_idx: &mut usize,
        charpos: &mut i64,
        col: &mut usize,
    ) {
        *byte_idx = self.break_candidate.byte_idx();
        *charpos = self.break_candidate.charpos();
        *col = 0;
    }

    pub(crate) fn apply_before_row_transition(
        self,
        output_emitter: &mut WindowOutputEmitter,
        byte_idx: &mut usize,
        charpos: &mut i64,
        col: &mut usize,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        x: &mut f32,
        content_x: f32,
    ) {
        self.restore_row_output_progress(output_emitter);
        self.rewind_source_state(byte_idx, charpos, col);
        *x = content_x;
        row_extend.clear();
    }

    pub(crate) fn apply_after_row_transition(
        self,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
    ) {
        *charpos = self.charpos();
        hit_row_range.advance_to(*charpos);
        face_scan.invalidate();
    }

    pub(crate) fn apply_after_row_transition_and_prefix(
        self,
        row_transition: TextMatrixRowTransition,
        transition: DisplayRowOverflowTransitionPlan,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
        row_geometry: &DisplayRowGeometryState,
        row_visibility_limit: DisplayRowVisibilityLimit,
        render_state: DisplayRowTransitionRenderState<'_>,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        self.apply_after_row_transition(charpos, hit_row_range, face_scan);
        render_state.apply_overflow_prefix(transition);
        DisplayRowTransitionContinuation::after_visible_row_transition(
            row_transition,
            row_geometry,
            row_visibility_limit,
        )
    }

    pub(crate) fn charpos(self) -> i64 {
        self.break_candidate.charpos()
    }

    #[cfg(test)]
    pub(crate) fn byte_idx(self) -> usize {
        self.break_candidate.byte_idx()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextSpecialWrapSourceAction {
    charpos: i64,
}

impl BufferTextSpecialWrapSourceAction {
    pub(crate) fn new(charpos: i64) -> Self {
        Self { charpos }
    }

    pub(crate) fn apply_before_row_transition(
        self,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        x: &mut f32,
        content_x: f32,
    ) {
        *x = content_x;
        row_extend.clear();
    }

    pub(crate) fn hit_range_and_advance(
        self,
        hit_row_range: &mut HitRowRangeTracker,
    ) -> DisplayRowHitRange {
        let hit_range = hit_row_range.range_to(self.charpos);
        hit_row_range.advance_to(self.charpos);
        hit_range
    }

    pub(crate) fn transition_continuation(
        self,
        row_transition: TextMatrixRowTransition,
        row_geometry: &DisplayRowGeometryState,
        row_visibility_limit: DisplayRowVisibilityLimit,
    ) -> DisplayRowTransitionContinuation {
        DisplayRowTransitionContinuation::after_visible_row_transition(
            row_transition,
            row_geometry,
            row_visibility_limit,
        )
    }

    #[cfg(test)]
    pub(crate) fn charpos(self) -> i64 {
        self.charpos
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextCharacterWrapSourceAction {
    ch_start_byte_idx: usize,
    ch_start_charpos: i64,
}

impl BufferTextCharacterWrapSourceAction {
    pub(crate) fn new(ch_start_byte_idx: usize, ch_start_charpos: i64) -> Self {
        Self {
            ch_start_byte_idx,
            ch_start_charpos,
        }
    }

    pub(crate) fn from_source_step_char(source_char: BufferTextSourceStepChar) -> Self {
        Self::new(source_char.start_byte_idx(), source_char.start_charpos())
    }

    pub(crate) fn rewind_source_state(self, byte_idx: &mut usize, charpos: &mut i64) {
        *byte_idx = self.ch_start_byte_idx;
        *charpos = self.ch_start_charpos;
    }

    pub(crate) fn apply_before_row_transition(
        self,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        x: &mut f32,
        content_x: f32,
    ) {
        *x = content_x;
        row_extend.clear();
    }

    pub(crate) fn apply_after_row_transition(
        self,
        byte_idx: &mut usize,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
    ) {
        self.rewind_source_state(byte_idx, charpos);
        hit_row_range.advance_to(*charpos);
        face_scan.invalidate();
    }

    pub(crate) fn apply_after_visible_row_transition(
        self,
        row_transition: TextMatrixRowTransition,
        byte_idx: &mut usize,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
        row_geometry: &DisplayRowGeometryState,
        row_visibility_limit: DisplayRowVisibilityLimit,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        self.apply_after_row_transition(byte_idx, charpos, hit_row_range, face_scan);
        DisplayRowTransitionContinuation::after_visible_row_transition(
            row_transition,
            row_geometry,
            row_visibility_limit,
        )
    }
}

pub(crate) struct BufferTextSpecialOverflowRenderRequest<'a> {
    prepared_append: &'a BufferTextSpecialSourceCharPreparedAppend,
    context: BufferTextSpecialOverflowRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferTextSpecialOverflowRenderContext<'a> {
    pub(crate) text: &'a [u8],
    pub(crate) text_start_byte: usize,
    pub(crate) x_px: f32,
    pub(crate) right_edge_px: f32,
    pub(crate) wrap_mode: LineWrapMode,
    pub(crate) row_visibility_limit: DisplayRowVisibilityLimit,
    pub(crate) content_x: f32,
    pub(crate) has_prefix: bool,
    pub(crate) row_geometry_defaults: DisplayRowGeometryDefaults,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) max_rows: usize,
    pub(crate) row_limit: DisplayRowLimit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextSpecialOverflowRenderOutcome {
    Fits,
    AppendPrepared(DisplayRowTransitionContinuation),
    ContinueBufferWalk(DisplayRowTransitionContinuation),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum BufferTextSpecialSourceCharOverflowAction {
    Fits,
    Truncate {
        transition: DisplayRowOverflowTransitionPlan,
    },
    Wrap {
        transition: DisplayRowOverflowTransitionPlan,
    },
}

impl BufferTextSpecialSourceCharOverflowAction {
    pub(crate) fn for_decision(decision: SpecialTextRowOverflowDecision) -> Self {
        match decision {
            SpecialTextRowOverflowDecision::Fits => Self::Fits,
            SpecialTextRowOverflowDecision::Truncate => Self::Truncate {
                transition: DisplayRowOverflowTransitionPlan::truncation(
                    TextRowTransitionStatePolicy::special_truncation(),
                ),
            },
            SpecialTextRowOverflowDecision::Wrap => Self::Wrap {
                transition: DisplayRowOverflowTransitionPlan::visual_wrap(
                    TextRowTransitionStatePolicy::special_visual_wrap(),
                ),
            },
        }
    }
}

impl BufferTextSpecialOverflowRenderOutcome {
    pub(crate) fn should_break(self) -> bool {
        matches!(
            self,
            Self::AppendPrepared(
                DisplayRowTransitionContinuation::Exhausted
                    | DisplayRowTransitionContinuation::Hidden
            ) | Self::ContinueBufferWalk(
                DisplayRowTransitionContinuation::Exhausted
                    | DisplayRowTransitionContinuation::Hidden
            )
        )
    }

    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(
            self,
            Self::ContinueBufferWalk(DisplayRowTransitionContinuation::Continue)
        )
    }
}

impl<'a> BufferTextSpecialOverflowRenderRequest<'a> {
    pub(crate) fn new(
        prepared_append: &'a BufferTextSpecialSourceCharPreparedAppend,
        context: BufferTextSpecialOverflowRenderContext<'a>,
    ) -> Self {
        Self {
            prepared_append,
            context,
        }
    }

    pub(crate) fn render_if_needed_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferTextSpecialOverflowRenderState<'_, '_>,
    ) -> BufferTextSpecialOverflowRenderOutcome {
        let BufferTextSpecialOverflowRenderState {
            byte_idx,
            charpos,
            col,
            source_render,
            row_extend,
            x,
            line_numbers,
            row_geometry,
            row_flags,
            hit_rows,
            hit_row_range,
            prefix_request,
            hscroll_skip,
            word_wrap,
            trailing_whitespace,
            row_y_positions,
        } = state;
        let mut source_render = source_render;
        let context = self.context;

        match self.prepared_append.overflow_action(
            context.x_px,
            context.right_edge_px,
            context.wrap_mode,
        ) {
            None | Some(BufferTextSpecialSourceCharOverflowAction::Fits) => {
                BufferTextSpecialOverflowRenderOutcome::Fits
            }
            Some(BufferTextSpecialSourceCharOverflowAction::Truncate { transition }) => {
                let truncation_skip =
                    BufferTextTruncationSkipAction::consume_source_step_char_and_rest_of_line(
                        context.text,
                        byte_idx,
                        charpos,
                    );
                truncation_skip.apply_before_row_transition(
                    line_numbers,
                    row_extend,
                    x,
                    context.content_x,
                );
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    context.row_geometry_defaults,
                    context.text_matrix_row_base,
                    row_y_positions,
                    context.max_rows,
                    row_geometry,
                    row_flags,
                    context.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow_then_row_start(
                    transition,
                    hit_row_range.range_to(*charpos),
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
                    DisplayRowTransitionRenderState::new(
                        prefix_request,
                        context.has_prefix,
                        line_numbers,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                    ),
                    col,
                );
                let synced_charpos = buffer
                    .layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(
                        context.text_start_byte + *byte_idx,
                    ))
                    .get() as i64;
                BufferTextSpecialOverflowRenderOutcome::ContinueBufferWalk(
                    truncation_skip.sync_after_row_transition_if_visible(
                        row_transition,
                        synced_charpos,
                        charpos,
                        hit_row_range,
                    ),
                )
            }
            Some(BufferTextSpecialSourceCharOverflowAction::Wrap { transition }) => {
                let special_wrap_action = BufferTextSpecialWrapSourceAction::new(*charpos);
                special_wrap_action.apply_before_row_transition(row_extend, x, context.content_x);
                let hit_range = special_wrap_action.hit_range_and_advance(hit_row_range);
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    context.row_geometry_defaults,
                    context.text_matrix_row_base,
                    row_y_positions,
                    context.max_rows,
                    row_geometry,
                    row_flags,
                    context.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow_then_row_start(
                    transition,
                    hit_range,
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
                    DisplayRowTransitionRenderState::new(
                        prefix_request,
                        context.has_prefix,
                        line_numbers,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                    ),
                    col,
                );
                BufferTextSpecialOverflowRenderOutcome::AppendPrepared(
                    special_wrap_action.transition_continuation(
                        row_transition,
                        row_geometry,
                        context.row_visibility_limit,
                    ),
                )
            }
        }
    }
}
