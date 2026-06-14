use crate::display_row::DisplayRowActiveFaceState;
use crate::display_row_append::DisplayPropertyReplacementCursorPolicy;
use crate::display_row_geometry::DisplayRowTextPosition;
use crate::types::{VisualCursorSpec, WindowParams};
use crate::unicode::{decode_utf8, is_cluster_extender, is_wide_char};
use crate::window_output::RowMetricsSnapshot;
use neomacs_display_protocol::frame_glyphs::{CursorStyle, DisplaySlotId};
use neomacs_display_protocol::types::Color;
use neovm_core::window::{DisplayPointSnapshot, WindowCursorPos};

#[derive(Clone, Copy, Debug)]
pub(crate) struct CapturedCursorInfo {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) face_w: f32,
    pub(crate) face_h: f32,
    pub(crate) face_ascent: f32,
    pub(crate) bg: Color,
    pub(crate) byte_idx: usize,
    pub(crate) col: usize,
    pub(crate) matrix_row: usize,
    pub(crate) slot_width: Option<f32>,
    pub(crate) stretch_like: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CursorCaptureState {
    captured: Option<CapturedCursorInfo>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum CapturedCursorSlotWidth {
    FaceChar,
    Explicit(f32),
}

impl CapturedCursorSlotWidth {
    fn resolve(self, face_char_width: f32) -> f32 {
        match self {
            Self::FaceChar => face_char_width,
            Self::Explicit(width) => width,
        }
        .max(1.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CapturedCursorPlacement {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) byte_idx: usize,
    pub(crate) col: usize,
    pub(crate) matrix_row: usize,
    pub(crate) slot_width: CapturedCursorSlotWidth,
    pub(crate) stretch_like: bool,
}

impl CapturedCursorPlacement {
    pub(crate) fn from_row_text_position(
        position: DisplayRowTextPosition,
        slot_width: CapturedCursorSlotWidth,
        stretch_like: bool,
    ) -> Self {
        Self {
            x: position.x,
            y: position.y,
            byte_idx: position.byte_idx,
            col: position.col,
            matrix_row: position.row,
            slot_width,
            stretch_like,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CapturedCursorVisualState {
    pub(crate) face_width: f32,
    pub(crate) face_height: f32,
    pub(crate) face_ascent: f32,
    pub(crate) background: Color,
}

impl CapturedCursorVisualState {
    pub(crate) fn from_active_face_state(active_face_state: &DisplayRowActiveFaceState) -> Self {
        let metrics = active_face_state.metrics();
        Self {
            face_width: metrics.char_width,
            face_height: metrics.row_height,
            face_ascent: metrics.ascent,
            background: active_face_state.background(),
        }
    }

    pub(crate) fn display_box_from_active_face_state(
        active_face_state: &DisplayRowActiveFaceState,
        face_height: f32,
        face_ascent: f32,
    ) -> Self {
        let metrics = active_face_state.metrics();
        Self {
            face_width: metrics.char_width,
            face_height,
            face_ascent,
            background: active_face_state.background(),
        }
    }

    fn line_break_from_active_face_state(
        active_face_state: &DisplayRowActiveFaceState,
        line_height: f32,
    ) -> Self {
        let metrics = active_face_state.metrics();
        Self::display_box_from_active_face_state(active_face_state, line_height, metrics.ascent)
    }
}

impl CapturedCursorInfo {
    pub(crate) fn logical_cursor_position(
        &self,
        row_metric: RowMetricsSnapshot,
        text_matrix_row_base: usize,
        text_area_left: f32,
        window_top: f32,
    ) -> WindowCursorPos {
        WindowCursorPos {
            x: (self.x - text_area_left).round() as i64,
            y: (row_metric.pixel_y - window_top).round() as i64,
            row: text_matrix_row_base as i64 + self.matrix_row as i64,
            col: self.col as i64,
        }
    }

    pub(crate) fn resolved_slot_width(
        &self,
        style: CursorStyle,
        text: &[u8],
        params: &WindowParams,
    ) -> f32 {
        if let Some(slot_width) = self.slot_width {
            slot_width.max(1.0)
        } else {
            CursorSlotWidthRequest::from_window_params(
                style,
                text,
                self.byte_idx,
                self.col as i32,
                params,
            )
            .width_px(self.face_w)
            .max(1.0)
        }
    }

    pub(crate) fn from_visual_state(
        visual_state: CapturedCursorVisualState,
        placement: CapturedCursorPlacement,
    ) -> Self {
        Self {
            x: placement.x,
            y: placement.y,
            face_w: visual_state.face_width,
            face_h: visual_state.face_height,
            face_ascent: visual_state.face_ascent,
            bg: visual_state.background,
            byte_idx: placement.byte_idx,
            col: placement.col,
            matrix_row: placement.matrix_row,
            slot_width: Some(placement.slot_width.resolve(visual_state.face_width)),
            stretch_like: placement.stretch_like,
        }
    }

    pub(crate) fn from_active_face_state(
        active_face_state: &DisplayRowActiveFaceState,
        placement: CapturedCursorPlacement,
    ) -> Self {
        Self::from_visual_state(
            CapturedCursorVisualState::from_active_face_state(active_face_state),
            placement,
        )
    }

    pub(crate) fn display_box_from_active_face_state(
        active_face_state: &DisplayRowActiveFaceState,
        placement: CapturedCursorPlacement,
        face_height: f32,
        face_ascent: f32,
    ) -> Self {
        Self::from_visual_state(
            CapturedCursorVisualState::display_box_from_active_face_state(
                active_face_state,
                face_height,
                face_ascent,
            ),
            placement,
        )
    }

    pub(crate) fn line_break_from_active_face_state(
        active_face_state: &DisplayRowActiveFaceState,
        placement: CapturedCursorPlacement,
        line_height: f32,
    ) -> Self {
        Self::from_visual_state(
            CapturedCursorVisualState::line_break_from_active_face_state(
                active_face_state,
                line_height,
            ),
            placement,
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ResolvedCursorGeometry {
    pub(crate) slot_id: DisplaySlotId,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) ascent: f32,
    pub(crate) style: CursorStyle,
    pub(crate) color: Color,
    pub(crate) cursor_fg: Color,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CursorGeometrySource {
    pub(crate) slot_id: DisplaySlotId,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) slot_width: f32,
    pub(crate) face_height: f32,
    pub(crate) face_ascent: f32,
    pub(crate) row_height: f32,
    pub(crate) row_ascent: f32,
    pub(crate) default_line_height: f32,
    pub(crate) stretch_like: bool,
    pub(crate) ends_at_visible_eob: bool,
    pub(crate) cursor_fg: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CursorGeometryContext {
    pub(crate) window_id: i64,
    pub(crate) slot_width: f32,
    pub(crate) default_line_height: f32,
    pub(crate) ends_at_visible_eob: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct VisualCursorGeometryContext {
    pub(crate) window_id: i64,
    pub(crate) text_area_left: f32,
    pub(crate) window_top: f32,
}

impl CursorGeometrySource {
    pub(crate) fn from_captured_cursor(
        cursor: &CapturedCursorInfo,
        row_metric: RowMetricsSnapshot,
        context: CursorGeometryContext,
    ) -> Self {
        Self {
            slot_id: DisplaySlotId {
                window_id: context.window_id,
                row: row_metric.row as u32,
                col: cursor.col as u16,
            },
            x: cursor.x,
            y: cursor.y,
            slot_width: context.slot_width.max(1.0),
            face_height: cursor.face_h,
            face_ascent: cursor.face_ascent,
            row_height: row_metric.height,
            row_ascent: row_metric.ascent,
            default_line_height: context.default_line_height,
            stretch_like: cursor.stretch_like,
            ends_at_visible_eob: context.ends_at_visible_eob,
            cursor_fg: cursor.bg,
        }
    }

    pub(crate) fn from_display_point(
        point: &DisplayPointSnapshot,
        context: VisualCursorGeometryContext,
    ) -> Self {
        let point_h = (point.height as f32).max(1.0);
        Self {
            slot_id: DisplaySlotId {
                window_id: context.window_id,
                row: point.row.max(0) as u32,
                col: point.col.max(0) as u16,
            },
            x: context.text_area_left + point.x as f32,
            y: context.window_top + point.y as f32,
            slot_width: (point.width as f32).max(1.0),
            face_height: point_h,
            face_ascent: point_h,
            row_height: point_h,
            row_ascent: point_h,
            default_line_height: point_h,
            stretch_like: false,
            ends_at_visible_eob: false,
            cursor_fg: Color::BLACK,
        }
    }
}

impl ResolvedCursorGeometry {
    pub(crate) fn window_id(&self) -> i64 {
        self.slot_id.window_id
    }
}

impl CursorCaptureState {
    pub(crate) fn new() -> Self {
        Self { captured: None }
    }

    pub(crate) fn is_missing(self) -> bool {
        self.captured.is_none()
    }

    pub(crate) fn is_captured(self) -> bool {
        self.captured.is_some()
    }

    pub(crate) fn capture_once(&mut self, info: CapturedCursorInfo) {
        if self.captured.is_none() {
            self.captured = Some(info);
        }
    }

    pub(crate) fn update_for_main_char(&mut self, byte_idx: usize, advance: f32) {
        let Some(cursor) = self.captured.as_mut() else {
            return;
        };
        if cursor.byte_idx != byte_idx {
            return;
        }
        cursor.slot_width = Some(advance.max(1.0));
    }

    #[cfg(test)]
    pub(crate) fn as_ref(&self) -> Option<&CapturedCursorInfo> {
        self.captured.as_ref()
    }

    pub(crate) fn captured(self) -> Option<CapturedCursorInfo> {
        self.captured
    }
}

pub(crate) fn capture_cursor_info(target: &mut CursorCaptureState, info: CapturedCursorInfo) {
    target.capture_once(info);
}

pub(crate) fn update_cursor_info_for_main_char(
    target: &mut CursorCaptureState,
    byte_idx: usize,
    advance: f32,
) {
    target.update_for_main_char(byte_idx, advance);
}

pub(crate) fn display_property_replacement_cursor_info(
    policy: DisplayPropertyReplacementCursorPolicy,
    active_face_state: &DisplayRowActiveFaceState,
    position: DisplayRowTextPosition,
) -> CapturedCursorInfo {
    match policy {
        DisplayPropertyReplacementCursorPolicy::TextSlot {
            width_px,
            stretch_like,
        } => CapturedCursorInfo::from_active_face_state(
            active_face_state,
            CapturedCursorPlacement::from_row_text_position(
                position,
                CapturedCursorSlotWidth::Explicit(width_px),
                stretch_like,
            ),
        ),
        DisplayPropertyReplacementCursorPolicy::DisplayBox {
            width_px,
            cursor_face_height_px,
            cursor_face_ascent_px,
        } => CapturedCursorInfo::display_box_from_active_face_state(
            active_face_state,
            CapturedCursorPlacement::from_row_text_position(
                position,
                CapturedCursorSlotWidth::Explicit(width_px),
                false,
            ),
            cursor_face_height_px,
            cursor_face_ascent_px,
        ),
        DisplayPropertyReplacementCursorPolicy::FaceChar => {
            CapturedCursorInfo::from_active_face_state(
                active_face_state,
                CapturedCursorPlacement::from_row_text_position(
                    position,
                    CapturedCursorSlotWidth::FaceChar,
                    false,
                ),
            )
        }
    }
}

pub(crate) fn row_metrics_for_cursor(
    row_metrics: &[RowMetricsSnapshot],
    cursor_row: usize,
    current_row_fallback: RowMetricsSnapshot,
) -> RowMetricsSnapshot {
    row_metrics
        .iter()
        .find(|metric| metric.row == cursor_row)
        .copied()
        .unwrap_or(current_row_fallback)
}

#[inline]
pub(crate) fn cursor_style_for_window(params: &WindowParams) -> Option<CursorStyle> {
    use neomacs_display_protocol::frame_glyphs::CursorKind;

    if params.cursor_kind == CursorKind::NoCursor {
        return None;
    }

    CursorStyle::from_kind(params.cursor_kind, params.cursor_bar_width)
}

pub(crate) fn cursor_style_for_visual(spec: &VisualCursorSpec) -> Option<CursorStyle> {
    use neomacs_display_protocol::frame_glyphs::CursorKind;

    if spec.cursor_kind == CursorKind::NoCursor {
        return None;
    }

    CursorStyle::from_kind(spec.cursor_kind, spec.cursor_bar_width)
}

pub(crate) fn resolve_cursor_vertical_metrics(
    cursor_y: f32,
    face_h: f32,
    face_ascent: f32,
    row_height: f32,
    row_ascent: f32,
    default_line_height: f32,
    ends_at_visible_eob: bool,
) -> (f32, f32, f32) {
    let row_height = row_height.max(1.0);
    let glyph_ascent = face_ascent.max(0.0).min(face_h.max(1.0));
    let glyph_descent = (face_h - glyph_ascent).max(0.0);
    let mut y = cursor_y;
    let mut ascent = row_ascent.max(0.0).min(row_height);

    if !ends_at_visible_eob && ascent < glyph_ascent {
        y -= glyph_ascent - ascent;
        ascent = glyph_ascent.min(row_height);
    }

    let minimum_height = default_line_height.max(1.0).min(row_height);
    let height = (ascent + glyph_descent).max(minimum_height).min(row_height);
    (y, height, ascent.min(height))
}

pub(crate) fn resolve_cursor_geometry(
    style: CursorStyle,
    source: CursorGeometrySource,
    x_stretch_cursor: bool,
    fallback_char_width: f32,
    color: Color,
) -> ResolvedCursorGeometry {
    let actual_slot_width = match style {
        CursorStyle::Bar(width) => width.max(1.0),
        CursorStyle::Hbar(_) | CursorStyle::FilledBox | CursorStyle::Hollow => {
            source.slot_width.max(1.0)
        }
    };
    let width = if source.stretch_like && !x_stretch_cursor && !matches!(style, CursorStyle::Bar(_))
    {
        fallback_char_width.max(1.0)
    } else {
        actual_slot_width
    };
    let (y, height, ascent) = resolve_cursor_vertical_metrics(
        source.y,
        source.face_height,
        source.face_ascent,
        source.row_height,
        source.row_ascent,
        source.default_line_height,
        source.ends_at_visible_eob,
    );

    ResolvedCursorGeometry {
        slot_id: source.slot_id,
        x: source.x,
        y,
        width,
        height,
        ascent,
        style,
        color,
        cursor_fg: source.cursor_fg,
    }
}

pub(crate) fn visual_cursor_source_from_point(
    point: &DisplayPointSnapshot,
    window_id: i64,
    text_area_left: f32,
    window_top: f32,
) -> CursorGeometrySource {
    CursorGeometrySource::from_display_point(
        point,
        VisualCursorGeometryContext {
            window_id,
            text_area_left,
            window_top,
        },
    )
}

#[inline]
pub(crate) fn next_tab_stop_col(
    current_col: usize,
    tab_width: i32,
    tab_stop_list: &[i32],
) -> usize {
    if !tab_stop_list.is_empty() {
        if let Some(&stop) = tab_stop_list
            .iter()
            .find(|&&stop| (stop as usize) > current_col)
        {
            return stop as usize;
        }
        let last = *tab_stop_list.last().unwrap() as usize;
        let tab_w = tab_width.max(1) as usize;
        if current_col >= last {
            return last + ((current_col - last) / tab_w + 1) * tab_w;
        }
        return last;
    }

    let tab_w = tab_width.max(1) as usize;
    ((current_col / tab_w) + 1) * tab_w
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum CursorSlotWidthPolicy {
    ExplicitPixels(f32),
    GlyphColumns(usize),
    TabClamp { frame_char_width: f32 },
}

pub(crate) struct CursorSlotWidthRequest<'a> {
    style: CursorStyle,
    text: &'a [u8],
    byte_idx: usize,
    col: i32,
    tab_width: i32,
    tab_stop_list: &'a [i32],
    x_stretch_cursor: bool,
    frame_char_width: f32,
}

impl<'a> CursorSlotWidthRequest<'a> {
    pub(crate) fn from_window_params(
        style: CursorStyle,
        text: &'a [u8],
        byte_idx: usize,
        col: i32,
        params: &'a WindowParams,
    ) -> Self {
        Self {
            style,
            text,
            byte_idx,
            col,
            tab_width: params.tab_width,
            tab_stop_list: &params.tab_stop_list,
            x_stretch_cursor: params.x_stretch_cursor,
            frame_char_width: params.char_width,
        }
    }

    pub(crate) fn point_columns(&self) -> usize {
        if self.byte_idx >= self.text.len() {
            return 1;
        }

        let (ch, _) = decode_utf8(&self.text[self.byte_idx..]);
        match ch {
            '\t' => {
                let col_usize = self.col.max(0) as usize;
                let next_tab = next_tab_stop_col(col_usize, self.tab_width, self.tab_stop_list)
                    .max(col_usize + 1);
                next_tab - col_usize
            }
            '\n' | '\r' => 1,
            _ if is_cluster_extender(ch) => 0,
            _ if is_wide_char(ch) => 2,
            _ => 1,
        }
    }

    pub(crate) fn width_policy(&self) -> CursorSlotWidthPolicy {
        match self.style {
            CursorStyle::Bar(width) => CursorSlotWidthPolicy::ExplicitPixels(width),
            CursorStyle::Hbar(_) => CursorSlotWidthPolicy::GlyphColumns(self.point_columns()),
            CursorStyle::FilledBox | CursorStyle::Hollow => {
                if !self.x_stretch_cursor && self.byte_idx < self.text.len() {
                    let (ch, _) = decode_utf8(&self.text[self.byte_idx..]);
                    if ch == '\t' {
                        return CursorSlotWidthPolicy::TabClamp {
                            frame_char_width: self.frame_char_width,
                        };
                    }
                }
                CursorSlotWidthPolicy::GlyphColumns(self.point_columns())
            }
        }
    }

    pub(crate) fn width_px(&self, face_char_w: f32) -> f32 {
        self.width_policy().width_px(face_char_w)
    }
}

impl CursorSlotWidthPolicy {
    pub(crate) fn width_px(self, face_char_w: f32) -> f32 {
        match self {
            Self::ExplicitPixels(width) => width,
            Self::GlyphColumns(columns) => columns as f32 * face_char_w,
            Self::TabClamp { frame_char_width } => frame_char_width.max(1.0),
        }
    }
}
