use crate::display_cursor::CursorCaptureState;
use crate::display_row::{DisplayRowActiveFaceState, DisplayRowMeasurementPolicy};
use crate::display_row_append::{
    BufferCurrentFaceResolutionContext, BufferDisplayPropertyCheckpointRenderRequest,
    BufferEndOfBufferTailRenderRequest, BufferHscrollSkipRenderRequest,
    BufferInvisibleTextRenderRequest, BufferLineNumberMarginRenderRequest,
    BufferLinePrefixRenderContext, BufferLinePrefixRenderRequest,
    BufferOverlayStringTextRowRenderContext, BufferSelectiveDisplayTailRenderRequest,
    BufferTextDecodedSourceChar, BufferTextLineBreakRenderRequest, BufferTextRowAppendState,
    BufferTextSourceCharRenderRequest, BufferTextWindowBeginRequest,
    BufferTextWindowBodyInstallRequest, BufferTextWindowFinishRequest,
    BufferTextWindowTailFinalizeRequest, BufferTextWindowVisibilityRetryRequest,
    DisplayRowAppendSurface, DisplayRowPrefixRequest, DisplayRowPrefixValues,
    TextWindowAppendSurfaceRequest,
};
use crate::display_row_builder::DisplayRowPosition;
use crate::display_row_geometry::{
    DisplayRowFlagKind, DisplayRowFlags, DisplayRowGeometryDefaults, DisplayRowGeometryState,
    DisplayRowLimit, DisplayRowScopedValue, DisplayRowVisibilityLimit, DisplayRowYPositions,
};
use crate::display_row_walk_state::{
    ActiveDisplayPropertySpan, BoxFaceRowState, HitRowRangeTracker, HorizontalScrollSkipState,
    LineNumberRenderState, TextPropertyScanCheckpoints, TrailingWhitespaceRenderState,
    WordWrapRenderState,
};
use crate::hit_test::HitRow;
use crate::neovm_bridge::{
    FaceResolver, LayoutBufferView, ResolvedFace, RustBufferAccess,
    buffer_display_line_numbers_mode, buffer_local_bool, buffer_local_int, buffer_local_value,
};
use crate::types::WindowParams;
use neomacs_display_protocol::types::{Color, Rect};
use neovm_core::buffer::BufferId;
use neovm_core::window::{DisplayRowSnapshot, FrameId, WindowId};

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

    #[cfg(test)]
    pub(crate) fn window_start(&self) -> i64 {
        self.window_start
    }

    #[cfg(test)]
    pub(crate) fn accessible_range(&self) -> (i64, i64) {
        (self.accessible_start, self.accessible_end)
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
