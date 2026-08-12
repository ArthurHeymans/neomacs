use crate::coords::layout_i64_char_pos_to_lisp_char_pos;
use crate::display_row::face_state::DisplayRowActiveFaceState;
use crate::display_row::geometry::DisplayRowTextPosition;
use crate::display_row::width::DisplayRowCharWidthPolicy;
use crate::display_source::DisplayPropertyReplacementCursorPolicy;
use crate::types::{VisualCursorSpec, WindowParams};
use crate::unicode::{decode_utf8, is_cluster_extender, is_wide_char};
use crate::window_output::{
    RowMetricsSnapshot, TextWindowCursor, TextWindowDecorativeCursor, TextWindowOutputTarget,
    WindowOutputEmitter, publish_text_window_cursor, publish_text_window_decorative_cursor,
};
use neomacs_display_protocol::frame_glyphs::{CursorStyle, DisplaySlotId};
use neomacs_display_protocol::glyph_matrix::{
    GlyphArea, GlyphProvenance, GlyphRow, RedisplayGlyphProvenance,
};
use neomacs_display_protocol::types::{Color, DisplayWindowId, Rect};
use neovm_core::window::{DisplayPointSnapshot, WindowCursorPos};

/// The ordinary face colors of the glyph underneath a cursor.
///
/// Keeping this pair typed prevents cursor reconstruction paths from confusing
/// a glyph foreground with the cursor-box background.  GNU's GUI ports need
/// both values when they merge the cursor GC with a non-default glyph face.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CursorGlyphFaceColors {
    pub(crate) foreground: Color,
    pub(crate) background: Color,
}

impl CursorGlyphFaceColors {
    pub(crate) const fn new(foreground: Color, background: Color) -> Self {
        Self {
            foreground,
            background,
        }
    }
}

/// Fully resolved box-cursor paint, ready for the display protocol.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResolvedBoxCursorPaint {
    pub(crate) background: Color,
    pub(crate) glyph_foreground: Color,
}

impl ResolvedBoxCursorPaint {
    /// Port of GNU xterm.c `x_set_cursor_gc` / pgtkterm.c
    /// `pgtk_set_cursor_gc`.
    pub(crate) fn resolve_gnu(
        cursor_background: Color,
        glyph_face: CursorGlyphFaceColors,
        frame_cursor_foreground: Color,
    ) -> Self {
        let mut paint = Self {
            background: cursor_background,
            glyph_foreground: glyph_face.background,
        };

        if paint.glyph_foreground == paint.background {
            paint.glyph_foreground = glyph_face.foreground;
        }
        if paint.glyph_foreground == paint.background {
            paint.glyph_foreground = frame_cursor_foreground;
        }
        if paint.glyph_foreground == paint.background {
            paint.glyph_foreground = glyph_face.foreground;
        }

        // GNU also keeps the cursor visually distinct when the proposed paint
        // would reproduce the ordinary glyph face unchanged.
        if paint.background == glyph_face.background
            && paint.glyph_foreground == glyph_face.foreground
        {
            paint.background = glyph_face.foreground;
            paint.glyph_foreground = glyph_face.background;
        }

        paint
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CapturedCursorInfo {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) face_w: f32,
    pub(crate) face_h: f32,
    pub(crate) face_ascent: f32,
    pub(crate) fg: Color,
    pub(crate) bg: Color,
    pub(crate) byte_idx: usize,
    pub(crate) col: usize,
    pub(crate) display_row_offset: usize,
    pub(crate) slot_width: Option<f32>,
    pub(crate) stretch_like: bool,
    pub(crate) glyph_row_resolved: bool,
    /// For a cursor sitting at a `display`-property replacement slot: the 1-based
    /// buffer position of the real glyph immediately preceding the slot (the
    /// replaced region's start minus one). The cursor's integer/grid x is derived
    /// from that glyph's already-rounded display point (`x + width`) rather than
    /// re-rounding the accumulated sub-pixel slot start, so it stays byte-identical
    /// to the glyph edge for every font size. `None` for ordinary cursors and for a
    /// replacement at the very start of the buffer (no preceding glyph).
    pub(crate) display_replacement_anchor_charpos: Option<i64>,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CursorCaptureState {
    captured: Option<CapturedCursorInfo>,
    string_cursor_property_captured: bool,
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
    pub(crate) display_row_offset: usize,
    pub(crate) slot_width: CapturedCursorSlotWidth,
    pub(crate) stretch_like: bool,
}

pub(crate) fn cursor_window_matches_current(cursor_window_id: i64, current_window_id: u64) -> bool {
    cursor_window_id >= 0 && cursor_window_id as u64 == current_window_id
}

#[derive(Clone, Copy)]
pub(crate) struct CursorVisualColumnRows<'a> {
    rows: &'a [neomacs_display_protocol::glyph_matrix::MatrixRow],
    ncols: usize,
}

impl<'a> CursorVisualColumnRows<'a> {
    pub(crate) fn new(
        rows: &'a [neomacs_display_protocol::glyph_matrix::MatrixRow],
        ncols: usize,
    ) -> Self {
        Self { rows, ncols }
    }

    fn row(self, row: usize) -> Option<&'a GlyphRow> {
        self.rows.get(row).map(|row| row.as_ref())
    }

    fn ncols(self) -> usize {
        self.ncols
    }
}

#[derive(Clone, Copy)]
pub(crate) struct CursorVisualColumnResolutionContext<'a> {
    current_window_id: u64,
    current_pixel_bounds: Rect,
    rows: Option<CursorVisualColumnRows<'a>>,
}

impl<'a> CursorVisualColumnResolutionContext<'a> {
    pub(crate) fn new(
        current_window_id: u64,
        current_pixel_bounds: Rect,
        rows: Option<CursorVisualColumnRows<'a>>,
    ) -> Self {
        Self {
            current_window_id,
            current_pixel_bounds,
            rows,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CursorVisualColumnResolutionRequest {
    window_id: i64,
    row: usize,
    charpos: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResolvedPhysCursorPlacement {
    col: u16,
    x: Option<f32>,
}

impl CursorVisualColumnResolutionRequest {
    pub(crate) fn new(window_id: i64, row: usize, charpos: usize) -> Self {
        Self {
            window_id,
            row,
            charpos,
        }
    }

    pub(crate) fn from_cursor(cursor: &neomacs_display_protocol::frame_glyphs::PhysCursor) -> Self {
        Self::new(cursor.window_id.get(), cursor.row, cursor.charpos)
    }

    /// Resolve the materialize-grid column the cursor at `charpos` on `row`
    /// actually occupies, or `None` when the cursor is not on the current
    /// window's matrix.
    ///
    /// The column must equal the one `FrameDisplayState::materialize_grid_row`
    /// assigns to point's glyph: a single running counter over the LeftMargin
    /// (line numbers, fringe) area and then the Text area, skipping padding
    /// cells and weighting each glyph by its cell span. Counting only the
    /// Text-area index drops the line-number gutter, so the renderer would snap
    /// the cursor to a glyph `lnum_cols` cells to the left (or into the gutter
    /// on short lines), drawing a stray second cursor. GNU accounts for the
    /// same gutter in `set_cursor_from_row`, where the line-number glyphs live
    /// at the start of TEXT_AREA (src/xdisp.c).
    ///
    /// An exact charpos match places the cursor on point's own glyph. When point
    /// sits on invisible/hidden text (e.g. an org heading's collapsed `#+title:`
    /// or leading stars produce no glyph for that charpos), GNU's
    /// set_cursor_from_row instead places the cursor on the first visible glyph
    /// that follows point. We track the glyph with the smallest charpos greater
    /// than point as that fallback, so the cursor never reverts to the captured
    /// column (which would land on the line-number gutter and draw a stray
    /// second cursor).
    pub(crate) fn resolve(self, context: CursorVisualColumnResolutionContext<'_>) -> Option<u16> {
        if !cursor_window_matches_current(self.window_id, context.current_window_id) {
            return None;
        }
        let rows = context.rows?;
        let row = rows.row(self.row)?;

        let mut col_acc: u16 = 0;
        for glyph in &row.glyphs[GlyphArea::LeftMargin.index()] {
            if glyph.padding {
                continue;
            }
            col_acc = col_acc.saturating_add(glyph.materialized_slot_span());
        }

        // Trim the trailing redisplay-owned LINE-END suffix from the Text
        // area. GNU set_cursor_from_row walks `end` backwards while object is
        // nil and charpos <= 0 (xdisp.c), but first advances over leading
        // redisplay glyphs such as a line-number prefix. Typed provenance lets
        // us identify exactly the two line-end products we emit:
        //
        // 1. the same-face `:extend` suffix ending in a stretch glyph;
        // 2. append_space_for_newline's one terminal space on a row that
        //    actually spans buffer text (unlike the blank EOB gutter row).
        let text_glyphs = &row.glyphs[GlyphArea::Text.index()];
        let mut text_end = text_glyphs.len();
        if let Some(fill) = text_glyphs.last()
            && matches!(
                fill.provenance,
                GlyphProvenance::Redisplay(RedisplayGlyphProvenance::LineEnd)
            )
            && matches!(
                fill.glyph_type,
                neomacs_display_protocol::glyph_matrix::GlyphType::Stretch { .. }
            )
        {
            let fill_face = fill.face_id;
            while text_end > 0
                && matches!(
                    text_glyphs[text_end - 1].provenance,
                    GlyphProvenance::Redisplay(RedisplayGlyphProvenance::LineEnd)
                )
                && text_glyphs[text_end - 1].face_id == fill_face
            {
                text_end -= 1;
            }
        }
        if row.end_charpos > row.start_charpos
            && text_end > 0
            && matches!(
                text_glyphs[text_end - 1].provenance,
                GlyphProvenance::Redisplay(RedisplayGlyphProvenance::LineEnd)
            )
            && matches!(
                text_glyphs[text_end - 1].glyph_type,
                neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch: ' ' }
            )
        {
            text_end -= 1;
        }

        let mut nearest_after: Option<(usize, u16)> = None;
        let mut replacement_candidate: Option<(usize, u16)> = None;
        for glyph in &text_glyphs[..text_end] {
            if glyph.padding {
                continue;
            }
            match glyph.provenance {
                GlyphProvenance::Buffer { charpos } => {
                    if charpos == self.charpos {
                        return Some(col_acc);
                    }
                    if charpos > self.charpos
                        && nearest_after.is_none_or(|(after, _)| charpos < after)
                    {
                        nearest_after = Some((charpos, col_acc));
                    }
                }
                GlyphProvenance::Str { index, .. }
                    if row.glyph_covers_buffer_charpos(glyph, self.charpos) =>
                {
                    // GNU set_cursor_from_row Step 2 chooses the smallest
                    // string index, not the first glyph in visual order (bidi
                    // can reorder the string).  The exact covered range is
                    // carried once by the row's source occurrence instead of
                    // recovered heuristically or duplicated on every glyph.
                    if replacement_candidate
                        .is_none_or(|(candidate_index, _)| index < candidate_index)
                    {
                        replacement_candidate = Some((index, col_acc));
                    }
                }
                GlyphProvenance::Str { .. } | GlyphProvenance::Redisplay(_) => {}
            }
            col_acc = col_acc.saturating_add(glyph.materialized_slot_span());
        }
        // No glyph carries point's charpos. Point is either before the first
        // visible glyph (a hidden prefix -- use the first following glyph's
        // column, tracked in nearest_after) or past the row's last glyph (end
        // of line, or a blank line that has only gutter glyphs -- use col_acc,
        // the first cell after all the gutter and text). Returning col_acc
        // rather than None keeps a blank/EOL cursor out of the line-number
        // gutter (where the captured Text-index 0 would land it), matching GNU
        // set_cursor_from_row placing the cursor in the empty area after a row.
        Some(
            replacement_candidate
                .or(nearest_after)
                .map_or(col_acc, |(_, col)| col),
        )
    }

    pub(crate) fn resolve_phys_cursor_placement(
        self,
        context: CursorVisualColumnResolutionContext<'_>,
    ) -> Option<ResolvedPhysCursorPlacement> {
        let col = self.resolve(context)?;
        let x = context.rows.and_then(|rows| {
            (rows.ncols() > 0).then(|| {
                let char_w = context.current_pixel_bounds.width / rows.ncols() as f32;
                context.current_pixel_bounds.x + col as f32 * char_w
            })
        });
        Some(ResolvedPhysCursorPlacement { col, x })
    }
}

impl ResolvedPhysCursorPlacement {
    #[cfg(test)]
    pub(crate) fn col(self) -> u16 {
        self.col
    }

    pub(crate) fn apply_to(self, cursor: &mut neomacs_display_protocol::frame_glyphs::PhysCursor) {
        if self.col != cursor.col {
            cursor.col = self.col;
            cursor.slot_id.col = self.col;
            if let Some(x) = self.x {
                cursor.x = x;
            }
        }
    }
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
            display_row_offset: position.row,
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
    pub(crate) foreground: Color,
    pub(crate) background: Color,
}

impl CapturedCursorVisualState {
    pub(crate) fn from_active_face_state(active_face_state: &DisplayRowActiveFaceState) -> Self {
        let metrics = active_face_state.metrics();
        Self {
            face_width: metrics.char_width(),
            face_height: metrics.row_height(),
            face_ascent: metrics.ascent(),
            foreground: Color::from_pixel(active_face_state.resolved_face().fg),
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
            face_width: metrics.char_width(),
            face_height,
            face_ascent,
            foreground: Color::from_pixel(active_face_state.resolved_face().fg),
            background: active_face_state.background(),
        }
    }

    fn line_break_from_active_face_state(
        active_face_state: &DisplayRowActiveFaceState,
        line_height: f32,
    ) -> Self {
        let metrics = active_face_state.metrics();
        Self::display_box_from_active_face_state(active_face_state, line_height, metrics.ascent())
    }
}

impl CapturedCursorInfo {
    pub(crate) fn logical_cursor_position(
        &self,
        row_metric: RowMetricsSnapshot,
        display_text_row_base: usize,
        text_area_left: f32,
        window_top: f32,
    ) -> WindowCursorPos {
        WindowCursorPos {
            x: (self.x - text_area_left).round() as i64,
            y: (row_metric.pixel_y() - window_top).round() as i64,
            row: display_text_row_base as i64 + self.display_row_offset as i64,
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
            fg: visual_state.foreground,
            bg: visual_state.background,
            byte_idx: placement.byte_idx,
            col: placement.col,
            display_row_offset: placement.display_row_offset,
            slot_width: Some(placement.slot_width.resolve(visual_state.face_width)),
            stretch_like: placement.stretch_like,
            glyph_row_resolved: false,
            display_replacement_anchor_charpos: None,
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
    pub(crate) glyph_face: CursorGlyphFaceColors,
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
    pub(crate) glyph_face: CursorGlyphFaceColors,
}

impl CursorGeometrySource {
    pub(crate) fn from_captured_cursor(
        cursor: &CapturedCursorInfo,
        row_metric: RowMetricsSnapshot,
        context: CursorGeometryContext,
    ) -> Self {
        Self {
            slot_id: DisplaySlotId {
                window_id: DisplayWindowId::new(context.window_id),
                row: row_metric.row() as u32,
                col: cursor.col as u16,
            },
            x: cursor.x,
            y: cursor.y,
            slot_width: context.slot_width.max(1.0),
            face_height: cursor.face_h,
            face_ascent: cursor.face_ascent,
            row_height: row_metric.height(),
            row_ascent: row_metric.ascent(),
            default_line_height: context.default_line_height,
            stretch_like: cursor.stretch_like,
            ends_at_visible_eob: context.ends_at_visible_eob,
            glyph_face: CursorGlyphFaceColors::new(cursor.fg, cursor.bg),
        }
    }

    pub(crate) fn from_display_point(
        point: &DisplayPointSnapshot,
        context: VisualCursorGeometryContext,
    ) -> Self {
        let point_h = (point.height as f32).max(1.0);
        Self {
            slot_id: DisplaySlotId {
                window_id: DisplayWindowId::new(context.window_id),
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
            glyph_face: context.glyph_face,
        }
    }
}

impl ResolvedCursorGeometry {
    pub(crate) fn window_id(&self) -> i64 {
        self.slot_id.window_id.get()
    }
}

impl CursorCaptureState {
    pub(crate) fn new() -> Self {
        Self {
            captured: None,
            string_cursor_property_captured: false,
        }
    }

    pub(crate) fn is_missing(self) -> bool {
        self.captured.is_none()
    }

    pub(crate) fn capture_once(&mut self, info: CapturedCursorInfo) {
        if self.captured.is_none() {
            self.captured = Some(info);
        }
    }

    pub(crate) fn capture_string_cursor_property(&mut self, mut info: CapturedCursorInfo) {
        if !self.string_cursor_property_captured {
            info.glyph_row_resolved = true;
            self.captured = Some(info);
            self.string_cursor_property_captured = true;
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
    preceding_charpos: Option<i64>,
) -> CapturedCursorInfo {
    let mut info = match policy {
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
    };
    info.display_replacement_anchor_charpos = preceding_charpos;
    info
}

pub(crate) fn row_metrics_for_cursor(
    row_metrics: &[RowMetricsSnapshot],
    cursor_row: usize,
    current_row_fallback: RowMetricsSnapshot,
) -> RowMetricsSnapshot {
    row_metrics
        .iter()
        .find(|metric| metric.row() == cursor_row)
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
    frame_cursor_foreground: Color,
) -> ResolvedCursorGeometry {
    let actual_slot_width = match style {
        CursorStyle::Bar(width) => width.max(1.0),
        CursorStyle::Hbar(_) | CursorStyle::FilledBox | CursorStyle::Hollow => {
            source.slot_width.max(1.0)
        }
    };
    let width = if source.stretch_like && !x_stretch_cursor && !matches!(style, CursorStyle::Bar(_))
    {
        DisplayRowCharWidthPolicy::new(fallback_char_width).fallback()
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

    let paint =
        ResolvedBoxCursorPaint::resolve_gnu(color, source.glyph_face, frame_cursor_foreground);

    ResolvedCursorGeometry {
        slot_id: source.slot_id,
        x: source.x,
        y,
        width,
        height,
        ascent,
        style,
        color: paint.background,
        cursor_fg: paint.glyph_foreground,
    }
}

pub(crate) fn visual_cursor_source_from_point(
    point: &DisplayPointSnapshot,
    window_id: i64,
    text_area_left: f32,
    window_top: f32,
    glyph_face: CursorGlyphFaceColors,
) -> CursorGeometrySource {
    CursorGeometrySource::from_display_point(
        point,
        VisualCursorGeometryContext {
            window_id,
            text_area_left,
            window_top,
            glyph_face,
        },
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CapturedTextWindowCursorPublishOutcome {
    NoWindowCursor,
    Clipped,
    Published,
}

#[derive(Clone, Copy)]
pub(crate) struct CapturedTextWindowCursorPublishContext<'a> {
    params: &'a WindowParams,
    text: &'a [u8],
    display_text_row_base: usize,
    text_area_left: f32,
    window_top: f32,
    text_y: f32,
    text_height: f32,
    char_w: f32,
    char_h: f32,
    point_charpos: i64,
    ends_at_visible_eob: bool,
}

impl<'a> CapturedTextWindowCursorPublishContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        params: &'a WindowParams,
        text: &'a [u8],
        display_text_row_base: usize,
        text_area_left: f32,
        window_top: f32,
        text_y: f32,
        text_height: f32,
        char_w: f32,
        char_h: f32,
        point_charpos: i64,
        ends_at_visible_eob: bool,
    ) -> Self {
        Self {
            params,
            text,
            display_text_row_base,
            text_area_left,
            window_top,
            text_y,
            text_height,
            char_w,
            char_h,
            point_charpos,
            ends_at_visible_eob,
        }
    }

    pub(crate) fn publish_captured_cursor(
        self,
        cursor: CapturedCursorInfo,
        row_metrics: &[RowMetricsSnapshot],
        fallback_row_metric: RowMetricsSnapshot,
        output: TextWindowOutputTarget<'_>,
        output_emitter: &mut WindowOutputEmitter,
    ) -> CapturedTextWindowCursorPublishOutcome {
        let row_metric = row_metrics_for_cursor(
            row_metrics,
            self.display_text_row_base + cursor.display_row_offset,
            fallback_row_metric,
        );

        // For a cursor at a `display`-property replacement slot, derive its
        // integer grid x from the already-rounded display point of the glyph
        // immediately before the slot (`x + width`, both text-area-relative)
        // rather than re-rounding the accumulated sub-pixel slot start. GNU's
        // `set_cursor_from_row` reads the glyph's accumulated integer x from the
        // glyph matrix, so the caret aligns to the glyph edge by construction;
        // `round(x) + round(w)` (the glyph point) and `round(x + w)` (the raw
        // slot start) otherwise disagree by ±1px for ~27% of font sizes. Only the
        // integer snapshot/logical position is affected — the GUI renderer keeps
        // drawing the caret at the sub-pixel `resolved_cursor.x`.
        let cursor_display_row =
            self.display_text_row_base as i64 + cursor.display_row_offset as i64;
        let grid_x_override = cursor
            .display_replacement_anchor_charpos
            .and_then(|anchor| {
                let point = output_emitter
                    .point_for_lisp_buffer_pos(layout_i64_char_pos_to_lisp_char_pos(anchor))?;
                (point.row == cursor_display_row).then_some(point.x + point.width)
            });

        let mut logical_cursor = cursor.logical_cursor_position(
            row_metric,
            self.display_text_row_base,
            self.text_area_left,
            self.window_top,
        );
        if let Some(grid_x) = grid_x_override {
            logical_cursor.x = grid_x;
        }
        output_emitter.set_logical_cursor(logical_cursor);

        let Some(style) = cursor_style_for_window(self.params) else {
            return CapturedTextWindowCursorPublishOutcome::NoWindowCursor;
        };
        let source = CursorGeometrySource::from_captured_cursor(
            &cursor,
            row_metric,
            CursorGeometryContext {
                window_id: self.params.window_id,
                slot_width: cursor.resolved_slot_width(style, self.text, self.params),
                default_line_height: self.char_h,
                ends_at_visible_eob: self.ends_at_visible_eob,
            },
        );
        let resolved_cursor = resolve_cursor_geometry(
            style,
            source,
            self.params.x_stretch_cursor,
            self.char_w,
            Color::from_pixel(self.params.cursor_color),
            Color::from_pixel(self.params.cursor_foreground),
        );
        if resolved_cursor.y < self.text_y
            || resolved_cursor.y + resolved_cursor.height > self.text_y + self.text_height
        {
            return CapturedTextWindowCursorPublishOutcome::Clipped;
        }

        publish_text_window_cursor(
            output,
            output_emitter,
            TextWindowCursor {
                selected: self.params.selected,
                window_id: resolved_cursor.window_id(),
                charpos: self.point_charpos.max(0) as usize,
                slot_id: resolved_cursor.slot_id,
                x: resolved_cursor.x,
                y: resolved_cursor.y,
                width: resolved_cursor.width,
                height: resolved_cursor.height,
                ascent: resolved_cursor.ascent,
                style: resolved_cursor.style,
                color: resolved_cursor.color,
                cursor_fg: resolved_cursor.cursor_fg,
                text_area_left: self.text_area_left,
                window_top: self.window_top,
                glyph_row_resolved: cursor.glyph_row_resolved,
                grid_x_override,
            },
        );

        if self.ends_at_visible_eob {
            tracing::debug!(
                "layout_window_rust: emitting EOB cursor at x={:.1} y={:.1} w={:.1} h={:.1}",
                resolved_cursor.x,
                resolved_cursor.y,
                resolved_cursor.width,
                resolved_cursor.height
            );
        }

        CapturedTextWindowCursorPublishOutcome::Published
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct VisualTextWindowCursorPublishSummary {
    pub(crate) requested: usize,
    pub(crate) no_cursor: usize,
    pub(crate) missing_point: usize,
    pub(crate) clipped: usize,
    pub(crate) published: usize,
}

#[derive(Clone, Copy)]
pub(crate) struct VisualTextWindowCursorPublishContext<'a> {
    params: &'a WindowParams,
    text_area_left: f32,
    window_top: f32,
    text_y: f32,
    text_height: f32,
    char_w: f32,
}

impl<'a> VisualTextWindowCursorPublishContext<'a> {
    pub(crate) fn new(
        params: &'a WindowParams,
        text_area_left: f32,
        window_top: f32,
        text_y: f32,
        text_height: f32,
        char_w: f32,
    ) -> Self {
        Self {
            params,
            text_area_left,
            window_top,
            text_y,
            text_height,
            char_w,
        }
    }

    pub(crate) fn publish_visual_cursors(
        self,
        mut output: TextWindowOutputTarget<'_>,
        output_emitter: &WindowOutputEmitter,
    ) -> VisualTextWindowCursorPublishSummary {
        let mut summary = VisualTextWindowCursorPublishSummary::default();

        for spec in &self.params.visual_cursors {
            summary.requested += 1;
            let Some(style) = cursor_style_for_visual(spec) else {
                summary.no_cursor += 1;
                continue;
            };
            let Some(point) = output_emitter
                .point_for_lisp_buffer_pos(layout_i64_char_pos_to_lisp_char_pos(spec.charpos))
            else {
                summary.missing_point += 1;
                continue;
            };
            let source = visual_cursor_source_from_point(
                point,
                spec.id as i64,
                self.text_area_left,
                self.window_top,
                CursorGlyphFaceColors::new(
                    Color::from_pixel(self.params.default_fg),
                    Color::from_pixel(self.params.default_bg),
                ),
            );
            let resolved_cursor = resolve_cursor_geometry(
                style,
                source,
                self.params.x_stretch_cursor,
                self.char_w,
                Color::from_pixel(spec.color),
                Color::from_pixel(self.params.cursor_foreground),
            );
            if resolved_cursor.y < self.text_y
                || resolved_cursor.y + resolved_cursor.height > self.text_y + self.text_height
            {
                summary.clipped += 1;
                continue;
            }
            publish_text_window_decorative_cursor(
                output.reborrow(),
                TextWindowDecorativeCursor {
                    window_id: resolved_cursor.window_id(),
                    slot_id: resolved_cursor.slot_id,
                    x: resolved_cursor.x,
                    y: resolved_cursor.y,
                    width: resolved_cursor.width,
                    height: resolved_cursor.height,
                    style: resolved_cursor.style,
                    color: resolved_cursor.color,
                    cursor_fg: resolved_cursor.cursor_fg,
                    effects: spec.effects.clone(),
                },
            );
            summary.published += 1;
        }

        summary
    }
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
