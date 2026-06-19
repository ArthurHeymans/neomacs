use crate::coords::layout_i64_char_pos_to_lisp_char_pos;
use crate::display_buffer_text_append::{
    BufferTextWindowBeginRequest, BufferTextWindowBeginState, BufferTextWindowBodyInstallState,
    BufferTextWindowFinishState, BufferTextWindowVisibilityRetryOutcome,
    TextWindowAppendSurfaceRequest,
};
use crate::display_buffer_text_item_append::BufferTextRowAppendState;
use crate::display_buffer_text_render::{
    BufferCurrentFaceResolutionContext, BufferDisplayPropertyCheckpointRenderRequest,
    BufferDisplayPropertyCheckpointRenderState, BufferDisplayPropertyTextReplacementRenderOutcome,
    BufferDisplayPropertyTextReplacementRenderRequest,
    BufferDisplayPropertyTextReplacementRenderState, BufferDisplayPropertyTextWalkOutcome,
    BufferHscrollSkipRenderRequest, BufferHscrollSkipRenderState, BufferInvisibleTextRenderOutcome,
    BufferInvisibleTextRenderRequest, BufferInvisibleTextRenderRequestState,
    BufferSelectiveDisplayTailRenderOutcome, BufferSelectiveDisplayTailRenderRequest,
    BufferSelectiveDisplayTailRenderState, BufferSourceItemLayoutResolutionContext,
    BufferTextLineBreakRenderRequest, BufferTextLineBreakRenderState,
    BufferTextSourceCharRenderOutcome, BufferTextSourceCharRenderRequest,
    BufferTextSourceCharRenderRequestState, BufferTextWindowBodyInstallContext,
    BufferTextWindowFinishInstallState, BufferTextWindowLoopRequestContext,
    BufferTextWindowPostLoopRenderOutcome, BufferTextWindowPostLoopState,
    BufferTextWindowRetryBounds, BufferTextWindowRowPreludeRequestContext,
    BufferTextWindowTailRequestContext,
};
use crate::display_buffer_text_source::BufferTextWindowSource;
use crate::display_buffer_text_source::{
    BufferTextConsumedSourceItem, BufferTextReplacementItem, BufferTextSourceCursor,
    BufferTextSourceItem, BufferTextSourceItemStep, BufferTextSourceItemStepper,
    BufferTextSourceStepChar,
};
use crate::display_cursor::CursorCaptureState;
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_item::{DisplayItem, RenderFaceRef};
use crate::display_row::{
    DisplayRowActiveFaceState, DisplayRowFallbackMetrics, DisplayRowMeasurementPolicy,
};
use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_builder::DisplayRowPosition;
use crate::display_row_geometry::{
    DisplayRowFlags, DisplayRowGeometryDefaults, DisplayRowGeometryState, DisplayRowLimit,
    DisplayRowScopedValue, DisplayRowVisibilityLimit, DisplayRowYPositions,
};
use crate::display_row_lisp_string::{DisplayRowPrefixRequest, DisplayRowPrefixValues};
use crate::display_row_overlay_string::BufferOverlayStringTextRowRenderContext;
use crate::display_row_source_render::{TextRowOutputRenderState, TextRowSourceRenderState};
use crate::display_row_transition::DisplayRowTransitionContinuation;
use crate::display_row_walk_state::FaceScanCheckpoint;
use crate::display_row_walk_state::{
    BoxFaceRowState, HitRowRangeTracker, HorizontalScrollSkipState, LineNumberRenderState,
    TextPropertyScanCheckpoints, TrailingWhitespaceRenderState, WordWrapRenderState,
};
use crate::display_source::DisplaySourceContext;
use crate::display_source_resolver::{DisplaySourcePropertyResolver, DisplaySourceResolveState};
use crate::display_status_line::{ChromeRowRenderServices, WindowChromeRowsRenderRequest};
use crate::font_metrics::FontMetricsService;
use crate::hit_test::{HitRow, WindowHitData};
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::neovm_bridge::{
    FaceResolver, LayoutBufferView, ResolvedFace, RustBufferAccess,
    buffer_display_line_numbers_mode, buffer_local_bool, buffer_local_int, buffer_local_value,
};
use crate::types::{LineWrapMode, WindowKind, WindowParams};
use crate::window_output::{
    TextWindowOutputRenderState, TextWindowOutputRetryCheckpoint, TextWindowRedisplayPositions,
    WindowOutputEmitter,
};
use neomacs_display_protocol::types::{Color, Rect};
use neovm_core::buffer::{BufferId, CharPos0, EmacsBytePos, LispCharPos1};
use neovm_core::emacs_core::Context;
use neovm_core::window::{FrameId, WindowDisplaySnapshot, WindowId};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextWindowGeometryRequest {
    text_x: f32,
    text_y: f32,
    text_width: f32,
    text_height: f32,
    vscroll: f32,
    kind: WindowKind,
    top_chrome_rows: usize,
    bottom_chrome_rows: usize,
    char_width: f32,
    char_height: f32,
    /// Ceiling (in display rows) from `max-mini-window-height` for a
    /// minibuffer window.  `None` for ordinary windows; set via
    /// `with_max_mini_window_rows`.  GNU `resize_mini_window` measures the
    /// mini-window's content height with `move_it_to(ZV)` UNCLAMPED and only
    /// clips the result to this ceiling, so the walk must be allowed to emit
    /// up to this many rows even when the window is currently one row tall.
    max_mini_window_rows: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextWindowGeometry {
    pub(crate) text_x: f32,
    pub(crate) text_y: f32,
    pub(crate) text_width: f32,
    pub(crate) text_height: f32,
    pub(crate) char_width: f32,
    pub(crate) char_height: f32,
    pub(crate) max_rows: usize,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) text_matrix_rows: usize,
    pub(crate) bottom_chrome_rows: usize,
    pub(crate) mode_line_matrix_row: usize,
    pub(crate) cols: usize,
    pub(crate) line_number_pixel_width: f32,
    pub(crate) content_x: f32,
    /// Y at which a row stops being "visible" during the walk.  For ordinary
    /// windows this is `text_y + text_height` (the physical text area).  For a
    /// minibuffer it is lifted to span `max_rows` rows so the unclamped GNU
    /// `resize_mini_window` measurement can emit content rows beyond the
    /// window's current (often one-row) physical height; `max_rows` still hard
    /// caps the row count at the `max-mini-window-height` ceiling.
    pub(crate) visibility_bottom_y: f32,
}

pub(crate) struct BufferTextWindowGeometryPlan {
    pub(crate) geometry: BufferTextWindowGeometry,
    pub(crate) line_number_columns: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextWindowChromeHeights {
    pub(crate) mode_line: f32,
    pub(crate) header_line: f32,
    pub(crate) tab_line: f32,
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
    pub(crate) text_append_surface: crate::display_row_append_context::DisplayRowAppendSurface,
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextWindowLocalDisplayPolicy {
    line_number_mode: u8,
    line_number_offset: i64,
    line_number_major_tick: i32,
    line_number_current_absolute: bool,
    line_number_widen: bool,
    line_number_min_width: i32,
    prefix_values: DisplayRowPrefixValues,
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
    output: BufferTextWindowBodyOutputState<'emit>,
    face_ids: &'emit mut FrameFaceIdAllocator,
}

struct BufferTextWindowBodyOutputState<'emit> {
    builder: &'emit mut GlyphMatrixBuilder,
    evaluator: &'emit mut Context,
    font_metrics: &'emit mut Option<FontMetricsService>,
    face_resolver: &'emit FaceResolver,
}

impl<'emit> BufferTextWindowBodyOutputState<'emit> {
    fn begin_text_window_output(
        &mut self,
        begin_request: BufferTextWindowBeginRequest,
    ) -> WindowOutputEmitter {
        begin_request.begin_and_apply(BufferTextWindowBeginState::new(
            self.builder,
            self.evaluator,
        ))
    }

    fn source_render_state<'output>(
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

pub(crate) struct BufferTextWindowOutputSession<'emit> {
    builder: &'emit mut GlyphMatrixBuilder,
    evaluator: &'emit mut Context,
    font_metrics: &'emit mut Option<FontMetricsService>,
    face_resolver: &'emit FaceResolver,
    face_ids: &'emit mut FrameFaceIdAllocator,
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

    fn body_pass_state(&mut self) -> BufferTextWindowBodyPassState<'_> {
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

    fn rendered_body_complete_state<'hit>(
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

    fn initial_active_face_state(
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

pub(crate) struct BufferTextWindowBodyInstallRenderState<'emit, 'output, 'face> {
    output: TextWindowOutputRenderState<'emit, 'output>,
    render_services: ChromeRowRenderServices<'emit, 'face>,
}

pub(crate) struct BufferTextWindowBodyInstallPublishState<'emit, 'output, 'face> {
    output: TextWindowOutputRenderState<'emit, 'output>,
    evaluator: &'emit mut Context,
    render_services: ChromeRowRenderServices<'emit, 'face>,
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

impl<'emit, 'face> BufferTextWindowRenderedBodyCompleteState<'emit, 'face> {
    fn finish_state(self) -> BufferTextWindowRenderedBodyFinishState<'emit> {
        BufferTextWindowRenderedBodyFinishState {
            builder: self.builder,
            evaluator: self.evaluator,
            hit_data: self.hit_data,
            display_snapshots: self.display_snapshots,
        }
    }

    fn install_body_and_publish_redisplay(
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

    fn render_chrome_rows(
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
    fn finish_install_state(
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

pub(crate) struct BufferTextWindowLoopRenderState<'rows, 'emit, 'surface> {
    loop_context: BufferTextWindowLoopRequestContext,
    append_state: &'emit mut BufferTextRowAppendState,
    text_property_checkpoints: &'emit mut TextPropertyScanCheckpoints,
    byte_idx: &'emit mut usize,
    charpos: &'emit mut i64,
    col: &'emit mut usize,
    source_render: TextRowSourceRenderState<'emit>,
    row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    box_face: &'emit mut BoxFaceRowState,
    x: &'emit mut f32,
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

impl BufferTextWindowGeometryRequest {
    pub(crate) fn new(
        params: &WindowParams,
        char_width: f32,
        char_height: f32,
        mode_line_height: f32,
        header_line_height: f32,
        tab_line_height: f32,
    ) -> Self {
        let text_x = params.text_bounds.x;
        let text_y = params.text_bounds.y + header_line_height + tab_line_height;
        let text_width = params.text_bounds.width;
        let text_height =
            params.bounds.height - mode_line_height - header_line_height - tab_line_height;

        // In Emacs, w->vscroll is negative when content is shifted up.
        let vscroll = (-params.vscroll).max(0) as f32;
        let text_height = (text_height - vscroll).max(0.0);

        Self {
            text_x,
            text_y,
            text_width,
            text_height,
            vscroll,
            kind: params.kind,
            top_chrome_rows: usize::from(tab_line_height > 0.0)
                + usize::from(header_line_height > 0.0),
            bottom_chrome_rows: usize::from(mode_line_height > 0.0),
            char_width,
            char_height,
            max_mini_window_rows: None,
        }
    }

    /// Record the `max-mini-window-height` ceiling (in display rows) so the
    /// minibuffer walk measures its full content height up to that ceiling,
    /// the way GNU `resize_mini_window` does.  No-op for non-minibuffer
    /// windows.
    pub(crate) fn with_max_mini_window_rows(mut self, max_mini_window_rows: usize) -> Self {
        if self.kind.is_minibuffer() {
            self.max_mini_window_rows = Some(max_mini_window_rows.max(1));
        }
        self
    }

    pub(crate) fn line_number_row_capacity(self) -> usize {
        // GNU's `maybe_produce_line_number' reserves `lnum_width + 2`
        // columns: the right-aligned number plus one blank on each side.
        // `lnum_width` is wide enough for the largest line number that can
        // appear in the current window, so a tiny buffer in a tall window
        // still gets the same two-digit gutter GNU displays for visible rows
        // 1..N.
        self.base_max_rows()
    }

    pub(crate) fn into_geometry(self, line_number_columns: i32) -> BufferTextWindowGeometry {
        let max_rows = self.visible_max_rows();
        let line_number_pixel_width = line_number_columns as f32 * self.char_width;
        let text_matrix_row_base = self.top_chrome_rows;
        let text_matrix_rows = max_rows.max(1);
        let mode_line_matrix_row = text_matrix_row_base + text_matrix_rows;
        let cols = ((self.text_width - line_number_pixel_width) / self.char_width).floor() as usize;
        let content_x = self.text_x + line_number_pixel_width;

        // For a minibuffer measured with the GNU `move_it_to(ZV)` policy, lift
        // the visibility bottom to span `max_rows` so the walk can emit content
        // rows past the window's current physical height; `max_rows` (the
        // ceiling) still hard caps the count.  Ordinary windows keep the
        // physical text-area bottom.
        let physical_bottom_y = self.text_y + self.text_height;
        let visibility_bottom_y = if self.kind.is_minibuffer() {
            physical_bottom_y.max(self.text_y + max_rows as f32 * self.char_height)
        } else {
            physical_bottom_y
        };

        BufferTextWindowGeometry {
            text_x: self.text_x,
            text_y: self.text_y,
            text_width: self.text_width,
            text_height: self.text_height,
            char_width: self.char_width,
            char_height: self.char_height,
            max_rows,
            text_matrix_row_base,
            text_matrix_rows,
            bottom_chrome_rows: self.bottom_chrome_rows,
            mode_line_matrix_row,
            cols,
            line_number_pixel_width,
            content_x,
            visibility_bottom_y,
        }
    }

    fn base_max_rows(self) -> usize {
        (self.text_height / self.char_height).floor() as usize
    }

    fn visible_max_rows(self) -> usize {
        // GNU `resize_mini_window` measures the mini-window's full content
        // height with an unclamped `move_it_to(ZV)` and clips it only to the
        // `max-mini-window-height` ceiling.  Mirror that: when the ceiling is
        // known, let the walk emit up to that many rows (not just the rows that
        // physically fit the current, often one-row, window).  vscroll != 0
        // means content is intentionally hidden (e.g. vertico-posframe), so
        // fall back to the physical row count there.
        if self.kind.is_minibuffer() && self.vscroll == 0.0 && self.text_height > 0.0 {
            if let Some(ceiling) = self.max_mini_window_rows {
                return ceiling.max(1);
            }
        }

        let max_rows = self.base_max_rows();
        // The minibuffer must always render at least 1 row.  Its pixel
        // height may be fractionally smaller than char_height (e.g. 24px vs
        // 24.15 with line-spacing) causing floor() to yield 0.  Exception:
        // when vscroll is active, don't force 1 row -- vscroll is used (e.g.
        // by vertico-posframe) to intentionally hide content.
        if self.kind.is_minibuffer()
            && max_rows == 0
            && self.text_height > 0.0
            && self.vscroll == 0.0
        {
            1
        } else {
            max_rows
        }
    }

    pub(crate) fn into_window_plan<B: LayoutBufferView>(
        self,
        local_display_policy: &BufferTextWindowLocalDisplayPolicy,
        buffer_access: &RustBufferAccess<'_, B>,
    ) -> BufferTextWindowGeometryPlan {
        let line_number_columns = local_display_policy
            .line_number_columns(buffer_access, self.line_number_row_capacity());
        let geometry = self.into_geometry(line_number_columns);
        BufferTextWindowGeometryPlan {
            geometry,
            line_number_columns,
        }
    }
}

impl BufferTextWindowChromeHeights {
    pub(crate) const fn new(mode_line: f32, header_line: f32, tab_line: f32) -> Self {
        Self {
            mode_line,
            header_line,
            tab_line,
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
            &mut self.byte_idx,
            &mut self.charpos,
            &mut self.col,
            state.source_render.reborrow(),
            &mut self.row_extend,
            &mut self.box_face,
            &mut self.x,
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

impl BufferTextWindowLocalDisplayPolicy {
    pub(crate) fn from_buffer(buffer: &impl LayoutBufferView) -> Self {
        Self {
            line_number_mode: buffer_display_line_numbers_mode(buffer).engine_code(),
            line_number_offset: buffer_local_int(buffer, "display-line-numbers-offset", 0),
            line_number_major_tick: buffer_local_int(buffer, "display-line-numbers-major-tick", 0)
                as i32,
            line_number_current_absolute: buffer_local_bool(
                buffer,
                "display-line-numbers-current-absolute",
            ),
            line_number_widen: buffer_local_bool(buffer, "display-line-numbers-widen"),
            line_number_min_width: buffer_local_int(buffer, "display-line-numbers-width", 0) as i32,
            prefix_values: DisplayRowPrefixValues::default_values(
                buffer_local_value(buffer, "line-prefix"),
                buffer_local_value(buffer, "wrap-prefix"),
            ),
        }
    }

    pub(crate) fn line_number_columns<B: LayoutBufferView>(
        self,
        access: &RustBufferAccess<'_, B>,
        max_rows: usize,
    ) -> i32 {
        if !self.line_numbers_enabled() {
            return 0;
        }
        let total_lines = access.count_lines(0, access.zv()) + 1;
        let visible_lines = max_rows.max(1) as i64;
        let digit_count = total_lines.max(visible_lines).max(1).to_string().len() as i32;
        let min = self.line_number_min_width.max(1);
        digit_count.max(min) + 2
    }

    pub(crate) fn initial_line_numbers<B: LayoutBufferView>(
        self,
        access: &RustBufferAccess<'_, B>,
        window_start: i64,
        point_charpos: i64,
    ) -> LineNumberRenderState {
        let window_start_byte = access.charpos_to_bytepos(window_start);
        let begin_byte = if self.line_number_widen {
            0
        } else {
            access.begv()
        };
        let current_line = if self.line_numbers_enabled() {
            access.count_lines(begin_byte, window_start_byte) + 1
        } else {
            1
        };
        let point_line = if self.line_numbers_enabled() && self.line_number_mode >= 2 {
            let pt_byte = access.charpos_to_bytepos(point_charpos);
            access.count_lines(begin_byte, pt_byte) + 1
        } else {
            0
        };
        LineNumberRenderState::new(self.line_numbers_enabled(), current_line, point_line)
    }

    pub(crate) fn row_prelude_context(
        self,
        line_number_cols: i32,
        char_width: f32,
        char_height: f32,
    ) -> BufferTextWindowRowPreludeRequestContext {
        BufferTextWindowRowPreludeRequestContext::new(
            self.line_number_mode,
            self.line_number_current_absolute,
            self.line_number_offset,
            self.line_number_major_tick,
            line_number_cols,
            self.prefix_values,
            char_width,
            char_height,
        )
    }

    pub(crate) fn has_prefix(self) -> bool {
        self.prefix_values.has_default_prefix()
    }

    pub(crate) fn has_line_default_prefix(self) -> bool {
        self.prefix_values.has_line_default_prefix()
    }

    fn line_numbers_enabled(self) -> bool {
        self.line_number_mode > 0
    }

    #[cfg(test)]
    pub(crate) fn from_parts(
        line_number_mode: u8,
        line_number_widen: bool,
        line_number_min_width: i32,
        prefix_values: DisplayRowPrefixValues,
    ) -> Self {
        Self {
            line_number_mode,
            line_number_offset: 0,
            line_number_major_tick: 0,
            line_number_current_absolute: false,
            line_number_widen,
            line_number_min_width,
            prefix_values,
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

impl<'rows, 'emit, 'surface> BufferTextWindowLoopRenderState<'rows, 'emit, 'surface> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        loop_context: BufferTextWindowLoopRequestContext,
        append_state: &'emit mut BufferTextRowAppendState,
        text_property_checkpoints: &'emit mut TextPropertyScanCheckpoints,
        byte_idx: &'emit mut usize,
        charpos: &'emit mut i64,
        col: &'emit mut usize,
        source_render: TextRowSourceRenderState<'emit>,
        row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &'emit mut BoxFaceRowState,
        x: &'emit mut f32,
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
        while *self.byte_idx < text.len()
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
            *self.charpos = (*self.charpos).max(end_charpos);
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

        let Some(source_step) =
            item_stepper.item_step_from_source_item(source_item, self.byte_idx, *self.charpos)
        else {
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
            self.loop_context.buffer_id(),
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
                byte_idx: self.byte_idx,
                charpos: self.charpos,
                x: self.x,
                col: self.col,
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
                DisplayRowPosition {
                    x_px: *self.x,
                    col: *self.col,
                },
            )
            .render_requested_with_source_state_and_apply(
                self.prefix_request,
                &mut self.source_render,
                buffer,
                *self.charpos,
                self.face_ids,
                self.x,
                self.col,
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
                self.byte_idx,
                *self.charpos,
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
            *self.x,
            *self.charpos,
        );
    }

    pub(crate) fn render_display_property_checkpoint_for_context<'request, B: LayoutBufferView>(
        &mut self,
        buffer: &'request B,
    ) -> BufferDisplayPropertyTextWalkOutcome {
        let request = self
            .loop_context
            .display_property_checkpoint_request(buffer, *self.charpos);
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
                byte_idx: self.byte_idx,
                charpos: self.charpos,
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
    }

    fn render_hscroll_skip(
        &mut self,
        request: BufferHscrollSkipRenderRequest<'_>,
    ) -> DisplayRowTransitionContinuation {
        request.render_next_and_apply(BufferHscrollSkipRenderState {
            byte_idx: self.byte_idx,
            charpos: self.charpos,
            hscroll_skip: self.hscroll_skip,
            row_extend: self.row_extend,
            source_render: self.source_render.reborrow(),
            x: self.x,
            col: self.col,
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
                byte_idx: self.byte_idx,
                charpos: self.charpos,
                col: self.col,
                source_render: self.source_render.reborrow(),
                row_extend: self.row_extend,
                box_face: self.box_face,
                x: self.x,
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
                byte_idx: self.byte_idx,
                charpos: self.charpos,
                cursor_info: self.cursor_info,
                row_geometry: self.row_geometry,
                trailing_whitespace: self.trailing_whitespace,
                row_extend: self.row_extend,
                box_face: self.box_face,
                source_render: self.source_render.reborrow(),
                x: self.x,
                col: self.col,
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
                byte_idx: self.byte_idx,
                charpos: self.charpos,
                col: self.col,
                source_render: self.source_render.reborrow(),
                row_extend: self.row_extend,
                x: self.x,
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

#[cfg(test)]
#[path = "display_buffer_text_walk_test.rs"]
mod tests;
