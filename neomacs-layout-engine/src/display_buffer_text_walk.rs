use crate::display_cursor::CursorCaptureState;
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_row::{DisplayRowActiveFaceState, DisplayRowMeasurementPolicy};
use crate::display_row_append::{
    BufferCurrentFaceResolutionContext, BufferDisplayPropertyCheckpointRenderRequest,
    BufferDisplayPropertyCheckpointRenderState, BufferDisplayPropertyTextWalkOutcome,
    BufferEndOfBufferTailRenderRequest, BufferEndOfBufferTailRenderState,
    BufferHscrollSkipRenderRequest, BufferHscrollSkipRenderState, BufferInvisibleTextRenderOutcome,
    BufferInvisibleTextRenderRequest, BufferInvisibleTextRenderRequestState,
    BufferLineNumberMarginRenderRequest, BufferLinePrefixRenderContext,
    BufferLinePrefixRenderRequest, BufferOverlayStringTextRowRenderContext,
    BufferSelectiveDisplayTailRenderOutcome, BufferSelectiveDisplayTailRenderRequest,
    BufferSelectiveDisplayTailRenderState, BufferTextDecodedSourceChar,
    BufferTextLineBreakRenderRequest, BufferTextLineBreakRenderState, BufferTextRowAppendState,
    BufferTextSourceCharRenderOutcome, BufferTextSourceCharRenderRequest,
    BufferTextSourceCharRenderRequestState, BufferTextWindowBeginRequest,
    BufferTextWindowBodyInstallRequest, BufferTextWindowBodyInstallState,
    BufferTextWindowFinishRequest, BufferTextWindowFinishState,
    BufferTextWindowTailFinalizeOutcome, BufferTextWindowTailFinalizeRequest,
    BufferTextWindowTailFinalizeState, BufferTextWindowVisibilityRetryOutcome,
    BufferTextWindowVisibilityRetryRequest, DisplayRowAppendSurface, DisplayRowPrefixRequest,
    DisplayRowPrefixValues, DisplayRowTransitionContinuation, TextRowSourceRenderState,
    TextWindowAppendSurfaceRequest,
};
use crate::display_row_builder::DisplayRowPosition;
use crate::display_row_geometry::{
    DisplayRowFlagKind, DisplayRowFlags, DisplayRowGeometryDefaults, DisplayRowGeometryState,
    DisplayRowLimit, DisplayRowScopedValue, DisplayRowVisibilityLimit, DisplayRowYPositions,
};
use crate::display_row_walk_state::FaceScanCheckpoint;
use crate::display_row_walk_state::{
    ActiveDisplayPropertySpan, BoxFaceRowState, HitRowRangeTracker, HorizontalScrollSkipState,
    LineNumberRenderState, TextPropertyScanCheckpoints, TrailingWhitespaceRenderState,
    WordWrapRenderState,
};
use crate::hit_test::{HitRow, WindowHitData};
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::neovm_bridge::{
    FaceResolver, LayoutBufferView, ResolvedFace, RustBufferAccess,
    buffer_display_line_numbers_mode, buffer_local_bool, buffer_local_int, buffer_local_value,
};
use crate::types::WindowParams;
use crate::window_output::{TextWindowRedisplayPositions, WindowOutputEmitter};
use neomacs_display_protocol::types::{Color, Rect};
use neovm_core::buffer::BufferId;
use neovm_core::emacs_core::Context;
use neovm_core::window::{DisplayRowSnapshot, FrameId, WindowDisplaySnapshot, WindowId};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextWindowGeometryRequest {
    text_x: f32,
    text_y: f32,
    text_width: f32,
    text_height: f32,
    vscroll: f32,
    is_minibuffer: bool,
    top_chrome_rows: usize,
    bottom_chrome_rows: usize,
    char_width: f32,
    char_height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextWindowGeometry {
    pub(crate) text_x: f32,
    pub(crate) text_y: f32,
    pub(crate) text_width: f32,
    pub(crate) text_height: f32,
    pub(crate) max_rows: usize,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) text_matrix_rows: usize,
    pub(crate) bottom_chrome_rows: usize,
    pub(crate) mode_line_matrix_row: usize,
    pub(crate) cols: usize,
    pub(crate) line_number_pixel_width: f32,
    pub(crate) content_x: f32,
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
    truncate_lines: bool,
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
    pub(crate) raise_span: ActiveDisplayPropertySpan<f32>,
    pub(crate) height_span: ActiveDisplayPropertySpan<f32>,
    pub(crate) row_flags: DisplayRowFlags,
    pub(crate) hscroll_skip: HorizontalScrollSkipState,
    pub(crate) word_wrap: WordWrapRenderState,
    pub(crate) prefix_request: DisplayRowPrefixRequest,
    pub(crate) text_append_surface: crate::display_row_append::DisplayRowAppendSurface,
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
}

pub(crate) struct BufferTextWindowOutputSetup {
    pub(crate) begin_request: BufferTextWindowBeginRequest,
    pub(crate) row_visibility_limit: DisplayRowVisibilityLimit,
    pub(crate) row_limit: DisplayRowLimit,
    pub(crate) body_install_context: BufferTextWindowBodyInstallContext,
    pub(crate) retry_bounds: BufferTextWindowRetryBounds,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextWindowBodyInstallContext {
    matrix_window_id: u64,
    text_matrix_row_base: usize,
    matrix_cols: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowRetryBounds {
    pub(crate) text_area_top: i64,
    pub(crate) text_area_bottom: i64,
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

pub(crate) struct BufferTextWindowTailRequestContext<'a> {
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
}

pub(crate) struct BufferTextWindowPostLoopState<'rows, 'emit> {
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
}

pub(crate) struct BufferTextWindowFinishInstallState<'a> {
    pub(crate) builder: &'a mut GlyphMatrixBuilder,
    pub(crate) output_emitter: WindowOutputEmitter,
    pub(crate) evaluator: &'a mut Context,
    pub(crate) hit_rows: Vec<HitRow>,
    pub(crate) hit_data: &'a mut Vec<WindowHitData>,
    pub(crate) display_snapshots: &'a mut Vec<WindowDisplaySnapshot>,
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

pub(crate) struct BufferTextWindowBodyInstallRenderState<'emit> {
    pub(crate) builder: &'emit mut GlyphMatrixBuilder,
    pub(crate) output_emitter: &'emit mut WindowOutputEmitter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowPostLoopRenderOutcome {
    retry: BufferTextWindowVisibilityRetryOutcome,
    rendered_rows_len: usize,
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

pub(crate) struct BufferTextWindowLoopRenderState<'rows, 'emit> {
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
    raise_span: &'emit mut ActiveDisplayPropertySpan<f32>,
    height_span: &'emit mut ActiveDisplayPropertySpan<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextWindowLoopStepOutcome {
    ContinueBufferWalk,
    StopBufferWalk,
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
            is_minibuffer: params.is_minibuffer,
            top_chrome_rows: usize::from(tab_line_height > 0.0)
                + usize::from(header_line_height > 0.0),
            bottom_chrome_rows: usize::from(mode_line_height > 0.0),
            char_width,
            char_height,
        }
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

    pub(crate) fn into_geometry(
        self,
        line_number_columns: i32,
        minibuffer_content_rows: Option<usize>,
    ) -> BufferTextWindowGeometry {
        let max_rows = minibuffer_content_rows.unwrap_or_else(|| self.visible_max_rows());
        let line_number_pixel_width = line_number_columns as f32 * self.char_width;
        let text_matrix_row_base = self.top_chrome_rows;
        let text_matrix_rows = max_rows.max(1);
        let mode_line_matrix_row = text_matrix_row_base + text_matrix_rows;
        let cols = ((self.text_width - line_number_pixel_width) / self.char_width).floor() as usize;
        let content_x = self.text_x + line_number_pixel_width;

        BufferTextWindowGeometry {
            text_x: self.text_x,
            text_y: self.text_y,
            text_width: self.text_width,
            text_height: self.text_height,
            max_rows,
            text_matrix_row_base,
            text_matrix_rows,
            bottom_chrome_rows: self.bottom_chrome_rows,
            mode_line_matrix_row,
            cols,
            line_number_pixel_width,
            content_x,
        }
    }

    fn base_max_rows(self) -> usize {
        (self.text_height / self.char_height).floor() as usize
    }

    fn visible_max_rows(self) -> usize {
        let max_rows = self.base_max_rows();
        // The minibuffer must always render at least 1 row.  Its pixel
        // height may be fractionally smaller than char_height (e.g. 24px vs
        // 24.15 with line-spacing) causing floor() to yield 0.  Exception:
        // when vscroll is active, don't force 1 row -- vscroll is used (e.g.
        // by vertico-posframe) to intentionally hide content.
        if self.is_minibuffer && max_rows == 0 && self.text_height > 0.0 && self.vscroll == 0.0 {
            1
        } else {
            max_rows
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
        truncate_lines: bool,
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
            truncate_lines,
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
            raise_span: ActiveDisplayPropertySpan::inactive(),
            height_span: ActiveDisplayPropertySpan::inactive(),
            row_flags: DisplayRowFlags::new(self.max_rows),
            hscroll_skip: HorizontalScrollSkipState::new(self.truncate_lines, self.hscroll),
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
        BufferTextWindowLoopRenderState::new(
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
            &mut self.raise_span,
            &mut self.height_span,
        )
        .render_visible_steps(
            row_prelude_context,
            loop_context,
            face_resolution_context,
            text,
            params,
            &self.text_append_surface,
            overlay_text_row_context,
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
        )
        .render_tail_and_decide_retry(
            loop_context,
            tail_context,
            text,
            &self.text_append_surface,
            self.byte_idx,
            self.charpos,
            has_overlays,
            overlay_context,
            active_face_state,
            buffer,
            buf_access,
        )
    }

    pub(crate) fn install_body(
        &mut self,
        state: BufferTextWindowBodyInstallRenderState<'_>,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
    ) -> TextWindowRedisplayPositions {
        tail_context
            .body_install_request(self.byte_idx, &self.row_flags)
            .install_and_apply(BufferTextWindowBodyInstallState {
                builder: state.builder,
                output_emitter: state.output_emitter,
            })
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
        }
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
                bottom_y: self.text_y + self.text_height,
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
        BufferTextWindowBodyInstallRequest::new(
            self.matrix_window_id,
            window_start,
            text_start_byte,
            byte_idx,
            reserve_right_special_col,
            reserve_right_border_col,
            self.text_matrix_row_base,
            self.matrix_cols,
            row_flags,
            0,
            char_width,
        )
    }

    #[cfg(test)]
    pub(crate) fn matrix_cols(self) -> usize {
        self.matrix_cols
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
        BufferTextWindowTailFinalizeRequest::new(
            self.params,
            text,
            self.text_matrix_row_base,
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
        )
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
            self.params.is_minibuffer,
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
        let finished_window =
            self.finish_request()
                .finish_and_snapshot(BufferTextWindowFinishState {
                    builder: state.builder,
                    output_emitter: state.output_emitter,
                    evaluator: state.evaluator,
                    hit_rows: state.hit_rows,
                });
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

impl<'rows, 'emit> BufferTextWindowPostLoopState<'rows, 'emit> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
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
    ) -> Self {
        Self {
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
        }
    }

    pub(crate) fn render_end_of_buffer_tail<'request, B: LayoutBufferView>(
        &mut self,
        loop_context: BufferTextWindowLoopRequestContext,
        byte_idx: usize,
        charpos: i64,
        has_overlays: bool,
        overlay_context: BufferOverlayStringTextRowRenderContext<'request>,
        active_face_state: &'request DisplayRowActiveFaceState,
        buffer: &B,
    ) -> bool {
        loop_context
            .end_of_buffer_tail_request(
                byte_idx,
                charpos,
                has_overlays,
                overlay_context,
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
        text_append_surface: &DisplayRowAppendSurface,
    ) -> BufferTextWindowTailDecorationOutcome {
        tail_context
            .tail_decoration_request()
            .apply(BufferTextWindowTailDecorationState {
                x: *self.x,
                text_append_surface,
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
            .finalize_and_apply(BufferTextWindowTailFinalizeState {
                cursor_info: self.cursor_info,
                row_geometry: self.row_geometry,
                row_y_positions: self.row_y_positions,
                hit_row_range: self.hit_row_range,
                hit_rows: self.hit_rows,
                output_render: self.source_render.output_render(),
            })
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
        loop_context: BufferTextWindowLoopRequestContext,
        tail_context: &BufferTextWindowTailRequestContext<'_>,
        text: &'request [u8],
        text_append_surface: &DisplayRowAppendSurface,
        byte_idx: usize,
        charpos: i64,
        has_overlays: bool,
        overlay_context: BufferOverlayStringTextRowRenderContext<'request>,
        active_face_state: &'request DisplayRowActiveFaceState,
        buffer: &B,
        buf_access: &'rows RustBufferAccess<'buf, B>,
    ) -> BufferTextWindowPostLoopRenderOutcome {
        let point_is_visible_eob = self.render_end_of_buffer_tail(
            loop_context,
            byte_idx,
            charpos,
            has_overlays,
            overlay_context,
            active_face_state,
            buffer,
        );

        self.apply_tail_decorations(tail_context, text_append_surface);
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
        BufferInvisibleTextRenderRequest::new(
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
        )
    }

    pub(crate) fn hscroll_skip_request<'a>(
        self,
        text: &'a [u8],
        append_surface: &'a DisplayRowAppendSurface,
        active_face_state: &'a DisplayRowActiveFaceState,
        face_resolver: &'a FaceResolver,
    ) -> BufferHscrollSkipRenderRequest<'a> {
        BufferHscrollSkipRenderRequest::new(
            text,
            self.tab_width,
            self.content_x,
            append_surface,
            active_face_state,
            self.default_face_ascent,
            self.char_height,
            self.char_width,
            face_resolver,
            self.point_charpos,
            self.has_prefix,
            self.row_geometry_defaults,
            self.text_matrix_row_base,
            self.max_rows,
            self.row_limit,
        )
    }

    pub(crate) fn display_property_checkpoint_request<'a, B>(
        self,
        face_resolution_context: BufferCurrentFaceResolutionContext<'a, B>,
        text: &'a [u8],
        params: &'a WindowParams,
        x: f32,
        col: usize,
        charpos: i64,
        byte_idx: usize,
        glyph_y_offset: f32,
    ) -> BufferDisplayPropertyCheckpointRenderRequest<'a, B>
    where
        B: LayoutBufferView,
    {
        BufferDisplayPropertyCheckpointRenderRequest::new(
            face_resolution_context,
            self.buffer_id,
            self.text_start_byte,
            text,
            x,
            self.content_x,
            params,
            glyph_y_offset,
            self.char_height,
            DisplayRowPosition { x_px: x, col },
            charpos,
            byte_idx,
            self.accessible_end,
        )
    }

    pub(crate) fn selective_display_tail_request<'a>(
        self,
        decoded_source_char: BufferTextDecodedSourceChar,
        text: &'a [u8],
        append_surface: &'a DisplayRowAppendSurface,
        active_face_state: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
    ) -> BufferSelectiveDisplayTailRenderRequest<'a> {
        BufferSelectiveDisplayTailRenderRequest::new(
            decoded_source_char,
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
            self.text_matrix_row_base,
            self.max_rows,
            self.row_limit,
        )
    }

    pub(crate) fn line_break_request<'a>(
        self,
        decoded_source_char: BufferTextDecodedSourceChar,
        text: &'a [u8],
        active_face_state: &'a DisplayRowActiveFaceState,
    ) -> BufferTextLineBreakRenderRequest<'a> {
        BufferTextLineBreakRenderRequest::new(
            decoded_source_char,
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
            self.text_matrix_row_base,
            self.max_rows,
            self.row_limit,
        )
    }

    pub(crate) fn source_char_request<'a>(
        self,
        decoded_source_char: BufferTextDecodedSourceChar,
        text: &'a [u8],
        append_surface: &'a DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
        params: &'a WindowParams,
        glyph_y_offset: f32,
    ) -> BufferTextSourceCharRenderRequest<'a> {
        BufferTextSourceCharRenderRequest::new(
            decoded_source_char,
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
            self.text_matrix_row_base,
            self.max_rows,
            self.row_limit,
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
        BufferEndOfBufferTailRenderRequest::new(
            byte_idx,
            charpos,
            self.accessible_end,
            self.point_charpos,
            has_overlays,
            overlay_context,
            active_face_state,
            self.row_limit,
        )
    }

    #[cfg(test)]
    pub(crate) fn buffer_id(self) -> BufferId {
        self.buffer_id
    }

    #[cfg(test)]
    pub(crate) fn text_start_byte(self) -> usize {
        self.text_start_byte
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

impl<'rows, 'emit> BufferTextWindowLoopRenderState<'rows, 'emit> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
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
        raise_span: &'emit mut ActiveDisplayPropertySpan<f32>,
        height_span: &'emit mut ActiveDisplayPropertySpan<f32>,
    ) -> Self {
        Self {
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
            raise_span,
            height_span,
        }
    }

    pub(crate) fn hscroll_should_skip(&self) -> bool {
        self.hscroll_skip.should_skip()
    }

    pub(crate) fn render_visible_steps<'request, B: LayoutBufferView>(
        &mut self,
        row_prelude_context: BufferTextWindowRowPreludeRequestContext,
        loop_context: BufferTextWindowLoopRequestContext,
        face_resolution_context: BufferCurrentFaceResolutionContext<'request, B>,
        text: &'request [u8],
        params: &'request WindowParams,
        append_surface: &'request DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'request>,
        active_face_state: &mut DisplayRowActiveFaceState,
        buffer: &B,
    ) {
        while *self.byte_idx < text.len()
            && self
                .row_geometry
                .current_row_is_visible(loop_context.row_visibility_limit)
        {
            if matches!(
                self.render_next_step(
                    row_prelude_context,
                    loop_context,
                    face_resolution_context.clone(),
                    text,
                    params,
                    append_surface,
                    overlay_context,
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
        row_prelude_context: BufferTextWindowRowPreludeRequestContext,
        loop_context: BufferTextWindowLoopRequestContext,
        face_resolution_context: BufferCurrentFaceResolutionContext<'request, B>,
        text: &'request [u8],
        params: &'request WindowParams,
        append_surface: &'request DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'request>,
        active_face_state: &mut DisplayRowActiveFaceState,
        buffer: &B,
    ) -> BufferTextWindowLoopStepOutcome {
        self.render_row_prelude(
            row_prelude_context,
            append_surface,
            active_face_state,
            buffer,
        );

        if self
            .render_invisible_text_for_context(
                loop_context,
                text,
                append_surface,
                overlay_context,
                active_face_state,
                buffer,
            )
            .should_continue_buffer_walk()
        {
            return BufferTextWindowLoopStepOutcome::ContinueBufferWalk;
        }

        if self.hscroll_should_skip() {
            if self
                .render_hscroll_skip_for_context(
                    loop_context,
                    text,
                    append_surface,
                    active_face_state,
                )
                .should_break()
            {
                return BufferTextWindowLoopStepOutcome::StopBufferWalk;
            }
            return BufferTextWindowLoopStepOutcome::ContinueBufferWalk;
        }

        let display_property_walk = self.render_display_property_checkpoint_for_context(
            loop_context,
            face_resolution_context,
            text,
            params,
            append_surface,
            active_face_state,
        );
        if display_property_walk.should_continue_buffer_walk() {
            return BufferTextWindowLoopStepOutcome::ContinueBufferWalk;
        }

        let Some(decoded_source_char) = self.consume_source_char(text) else {
            return BufferTextWindowLoopStepOutcome::StopBufferWalk;
        };

        let selective_display_outcome = self.render_selective_display_tail_for_context(
            loop_context,
            decoded_source_char,
            text,
            append_surface,
            active_face_state,
            buffer,
        );
        if selective_display_outcome.should_break() {
            return BufferTextWindowLoopStepOutcome::StopBufferWalk;
        }
        if selective_display_outcome.should_continue_buffer_walk() {
            return BufferTextWindowLoopStepOutcome::ContinueBufferWalk;
        }

        if decoded_source_char.ch() == '\n' {
            if self
                .render_line_break_for_context(
                    loop_context,
                    decoded_source_char,
                    text,
                    active_face_state,
                    buffer,
                )
                .should_break()
            {
                return BufferTextWindowLoopStepOutcome::StopBufferWalk;
            }
            return BufferTextWindowLoopStepOutcome::ContinueBufferWalk;
        }

        let char_render_outcome = self.render_source_char_for_context(
            loop_context,
            decoded_source_char,
            text,
            append_surface,
            overlay_context,
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

        BufferTextWindowLoopStepOutcome::ContinueBufferWalk
    }

    pub(crate) fn render_row_prelude<B: LayoutBufferView>(
        &mut self,
        context: BufferTextWindowRowPreludeRequestContext,
        append_surface: &DisplayRowAppendSurface,
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
                append_surface,
                self.row_geometry,
                active_face_state,
                self.raise_span.value_or(0.0),
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

    pub(crate) fn consume_source_char(
        &mut self,
        text: &[u8],
    ) -> Option<BufferTextDecodedSourceChar> {
        BufferTextDecodedSourceChar::consume_from_text(text, self.byte_idx, *self.charpos)
    }

    pub(crate) fn render_invisible_text_for_context<'request, B: LayoutBufferView>(
        &mut self,
        context: BufferTextWindowLoopRequestContext,
        text: &'request [u8],
        append_surface: &'request DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'request>,
        active_face_state: &'request DisplayRowActiveFaceState,
        buffer: &B,
    ) -> BufferInvisibleTextRenderOutcome {
        let request = context.invisible_text_request(
            text,
            append_surface,
            overlay_context,
            active_face_state,
            self.raise_span.value_or(0.0),
        );
        self.render_invisible_text_at_checkpoint(request, buffer)
    }

    pub(crate) fn render_hscroll_skip_for_context<'request>(
        &mut self,
        context: BufferTextWindowLoopRequestContext,
        text: &'request [u8],
        append_surface: &'request DisplayRowAppendSurface,
        active_face_state: &'request DisplayRowActiveFaceState,
    ) -> DisplayRowTransitionContinuation {
        let request = context.hscroll_skip_request(
            text,
            append_surface,
            active_face_state,
            self.source_render.face_resolver(),
        );
        self.render_hscroll_skip(request)
    }

    pub(crate) fn render_display_property_checkpoint_for_context<'request, B: LayoutBufferView>(
        &mut self,
        context: BufferTextWindowLoopRequestContext,
        face_resolution_context: BufferCurrentFaceResolutionContext<'request, B>,
        text: &'request [u8],
        params: &'request WindowParams,
        append_surface: &'request DisplayRowAppendSurface,
        active_face_state: &mut DisplayRowActiveFaceState,
    ) -> BufferDisplayPropertyTextWalkOutcome {
        let request = context.display_property_checkpoint_request(
            face_resolution_context,
            text,
            params,
            *self.x,
            *self.col,
            *self.charpos,
            *self.byte_idx,
            self.raise_span.value_or(0.0),
        );
        self.render_display_property_checkpoint(
            request,
            append_surface,
            active_face_state,
            context.point_charpos,
        )
    }

    pub(crate) fn render_selective_display_tail_for_context<'request, B: LayoutBufferView>(
        &mut self,
        context: BufferTextWindowLoopRequestContext,
        decoded_source_char: BufferTextDecodedSourceChar,
        text: &'request [u8],
        append_surface: &'request DisplayRowAppendSurface,
        active_face_state: &'request DisplayRowActiveFaceState,
        buffer: &B,
    ) -> BufferSelectiveDisplayTailRenderOutcome {
        let request = context.selective_display_tail_request(
            decoded_source_char,
            text,
            append_surface,
            active_face_state,
            self.raise_span.value_or(0.0),
        );
        self.render_selective_display_tail(request, buffer)
    }

    pub(crate) fn render_line_break_for_context<'request, B: LayoutBufferView>(
        &mut self,
        context: BufferTextWindowLoopRequestContext,
        decoded_source_char: BufferTextDecodedSourceChar,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
        buffer: &B,
    ) -> DisplayRowTransitionContinuation {
        let request = context.line_break_request(decoded_source_char, text, active_face_state);
        self.render_line_break(request, buffer)
    }

    pub(crate) fn render_source_char_for_context<'request, B: LayoutBufferView>(
        &mut self,
        context: BufferTextWindowLoopRequestContext,
        decoded_source_char: BufferTextDecodedSourceChar,
        text: &'request [u8],
        append_surface: &'request DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'request>,
        active_face_state: &'request DisplayRowActiveFaceState,
        params: &'request WindowParams,
        buffer: &B,
    ) -> BufferTextSourceCharRenderOutcome {
        let request = context.source_char_request(
            decoded_source_char,
            text,
            append_surface,
            overlay_context,
            active_face_state,
            params,
            self.raise_span.value_or(0.0),
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
        append_surface: &DisplayRowAppendSurface,
        active_face_state: &mut DisplayRowActiveFaceState,
        point_charpos: i64,
    ) -> BufferDisplayPropertyTextWalkOutcome {
        request.render_and_apply(BufferDisplayPropertyCheckpointRenderState {
            source_render: self.source_render.reborrow(),
            face_ids: self.face_ids,
            append_surface,
            row_geometry: self.row_geometry,
            checkpoints: self.text_property_checkpoints,
            face_scan: self.face_scan,
            active_face_state,
            row_extend: self.row_extend,
            box_face: self.box_face,
            byte_idx: self.byte_idx,
            charpos: self.charpos,
            x: self.x,
            col: self.col,
            cursor_info: self.cursor_info,
            raise_span: self.raise_span,
            height_span: self.height_span,
            point_charpos,
        })
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
                raise_span: self.raise_span,
            },
        )
    }
}

impl BufferTextWindowPostLoopRenderOutcome {
    pub(crate) fn retry(self) -> BufferTextWindowVisibilityRetryOutcome {
        self.retry
    }

    pub(crate) fn rendered_rows_len(self) -> usize {
        self.rendered_rows_len
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

#[cfg(test)]
#[path = "display_buffer_text_walk_test.rs"]
mod tests;
