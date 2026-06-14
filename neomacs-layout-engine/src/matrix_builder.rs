//! GlyphMatrixBuilder — records authoritative window matrices during layout.
//!
//! The builder observes layout emissions and writes them into the per-window
//! `GlyphMatrix` grids published through `FrameDisplayState`. Renderers then
//! materialize that immutable snapshot into runtime glyph buffers on the
//! consumer side; layout no longer treats `FrameGlyphBuffer` as the primary
//! output contract.

use neomacs_display_protocol::effect_config::EffectsConfig;
use neomacs_display_protocol::face::Face;
use neomacs_display_protocol::frame_glyphs::{
    CursorStyle, DisplaySlotId, GlyphRowRole, PhysCursor, StipplePattern, WindowEffectHint,
    WindowInfo, WindowTransitionHint,
};
use neomacs_display_protocol::glyph_matrix::*;
use neomacs_display_protocol::types::{Color, Rect};
use std::collections::HashMap;

fn cursor_window_matches_current(cursor_window_id: i64, current_window_id: u64) -> bool {
    cursor_window_id >= 0 && cursor_window_id as u64 == current_window_id
}

/// Number of grid columns `glyph` advances the materialize column counter.
///
/// Must stay in lock-step with `FrameDisplayState::materialize_grid_row`'s
/// `col +=` rule (glyph_matrix.rs): a stretch advances by its `width_cols`, a
/// double-width glyph by 2, everything else by 1. Used to place the physical
/// cursor on the same column index materialize assigns to the glyph at point.
fn glyph_cell_span(glyph: &Glyph) -> u16 {
    match glyph.glyph_type {
        GlyphType::Stretch { width_cols } => width_cols,
        _ => {
            if glyph.wide {
                2
            } else {
                1
            }
        }
    }
}

pub struct GlyphMatrixBuilder {
    windows: Vec<WindowMatrixEntry>,
    current_matrix: Option<GlyphMatrix>,
    current_window_id: u64,
    current_pixel_bounds: Rect,
    current_text_pixel_bounds: Rect,
    /// Whether the window currently open in the builder is the
    /// selected window. Copied into `WindowMatrixEntry.selected`
    /// by `end_window`. Mirrors GNU's per-frame
    /// `w == XWINDOW (selected_window)` check in
    /// `src/xdisp.c::update_window`.
    current_selected: bool,
    current_row: usize,
    in_row: bool,

    // Non-grid items
    backgrounds: Vec<BackgroundItem>,
    borders: Vec<BorderItem>,
    cursors: Vec<CursorItem>,
    images: Vec<ImageItem>,
    videos: Vec<VideoItem>,
    xwidgets: Vec<XwidgetItem>,
    scroll_bars: Vec<ScrollBarItem>,
    phys_cursor: Option<PhysCursor>,
    cursor_effects_by_window: HashMap<i64, EffectsConfig>,
    faces: HashMap<u32, Face>,
    stipple_patterns: HashMap<i32, StipplePattern>,
    window_infos: Vec<WindowInfo>,
    transition_hints: Vec<WindowTransitionHint>,
    effect_hints: Vec<WindowEffectHint>,
    background_color: Color,
    font_pixel_size: f32,
    frame_id: u64,
    parent_id: u64,
    parent_x: f32,
    parent_y: f32,
    z_order: i32,
    undecorated: bool,
    border_width: f32,
    border_color: Color,
    background_alpha: f32,
    no_accept_focus: bool,
}

impl GlyphMatrixBuilder {
    fn pad_row_and_write_glyph(
        row: &mut GlyphRow,
        target_col: usize,
        ch: char,
        face_id: u32,
        area: GlyphArea,
    ) {
        row.enabled = true;

        // Count existing glyphs across the three areas
        // (LeftMargin, Text, RightMargin). We treat every
        // glyph as one column advance — matching the TTY
        // RIF's `col += 1` in rasterize.
        let current_total = |row: &GlyphRow| -> usize {
            row.glyphs[GlyphArea::LeftMargin.index()].len()
                + row.glyphs[GlyphArea::Text.index()].len()
                + row.glyphs[GlyphArea::RightMargin.index()].len()
        };

        let preserve_trailing_truncation_marker = matches!(area, GlyphArea::RightMargin)
            && row.glyphs[GlyphArea::Text.index()]
                .last()
                .is_some_and(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '$' }));
        let preserved_trailing = if preserve_trailing_truncation_marker {
            row.glyphs[GlyphArea::Text.index()].pop()
        } else {
            None
        };
        let preserved_cols = usize::from(preserved_trailing.is_some());

        // Truncate anything in the text area that pushes
        // the glyph count past `target_col`. Left/right
        // margin columns belong to the caller — we only
        // touch the text area.
        let before_final_cols = target_col.saturating_sub(preserved_cols);
        while current_total(row) > before_final_cols {
            let text_area = &mut row.glyphs[GlyphArea::Text.index()];
            if text_area.is_empty() {
                break;
            }
            text_area.pop();
        }

        // Pad the text area with spaces until the combined
        // count reaches `target_col`.
        while current_total(row) < before_final_cols {
            row.glyphs[GlyphArea::Text.index()].push(Glyph::char(' ', face_id, 0));
        }

        if let Some(glyph) = preserved_trailing {
            row.glyphs[GlyphArea::Text.index()].push(glyph);
        }

        while current_total(row) < target_col {
            row.glyphs[GlyphArea::Text.index()].push(Glyph::char(' ', face_id, 0));
        }

        // Push the replacement glyph as the final glyph so it lands
        // at absolute column `target_col`.
        row.glyphs[area.index()].push(Glyph::char(ch, face_id, 0));
    }

    fn write_row_metrics(row: &mut GlyphRow, pixel_y_rel: f32, height_px: f32, ascent_px: f32) {
        row.pixel_y = pixel_y_rel;
        row.height_px = height_px.max(0.0);
        row.ascent_px = ascent_px.max(0.0).min(row.height_px.max(0.0));
    }

    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            current_matrix: None,
            current_window_id: 0,
            current_pixel_bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            current_text_pixel_bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            current_selected: false,
            current_row: 0,
            in_row: false,
            backgrounds: Vec::new(),
            borders: Vec::new(),
            cursors: Vec::new(),
            images: Vec::new(),
            videos: Vec::new(),
            xwidgets: Vec::new(),
            scroll_bars: Vec::new(),
            phys_cursor: None,
            cursor_effects_by_window: HashMap::new(),
            faces: HashMap::new(),
            stipple_patterns: HashMap::new(),
            window_infos: Vec::new(),
            transition_hints: Vec::new(),
            effect_hints: Vec::new(),
            background_color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            font_pixel_size: 0.0,
            frame_id: 0,
            parent_id: 0,
            parent_x: 0.0,
            parent_y: 0.0,
            z_order: 0,
            undecorated: false,
            border_width: 0.0,
            border_color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            background_alpha: 1.0,
            no_accept_focus: false,
        }
    }

    pub fn reset(&mut self) {
        self.windows.clear();
        self.current_matrix = None;
        self.current_window_id = 0;
        self.current_selected = false;
        self.current_row = 0;
        self.in_row = false;
        self.backgrounds.clear();
        self.borders.clear();
        self.cursors.clear();
        self.images.clear();
        self.videos.clear();
        self.xwidgets.clear();
        self.scroll_bars.clear();
        self.phys_cursor = None;
        self.cursor_effects_by_window.clear();
        self.faces.clear();
        self.stipple_patterns.clear();
        self.window_infos.clear();
        self.transition_hints.clear();
        self.effect_hints.clear();
        self.background_color = Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };
        self.font_pixel_size = 0.0;
        self.frame_id = 0;
        self.parent_id = 0;
        self.parent_x = 0.0;
        self.parent_y = 0.0;
        self.z_order = 0;
        self.undecorated = false;
        self.border_width = 0.0;
        self.border_color = Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };
        self.background_alpha = 1.0;
        self.no_accept_focus = false;
    }

    pub fn begin_window(
        &mut self,
        window_id: u64,
        nrows: usize,
        ncols: usize,
        pixel_bounds: Rect,
        selected: bool,
    ) {
        self.begin_window_with_text_bounds(
            window_id,
            nrows,
            ncols,
            pixel_bounds,
            pixel_bounds,
            selected,
        );
    }

    pub fn begin_window_with_text_bounds(
        &mut self,
        window_id: u64,
        nrows: usize,
        ncols: usize,
        pixel_bounds: Rect,
        text_pixel_bounds: Rect,
        selected: bool,
    ) {
        self.current_matrix = Some(GlyphMatrix::new(nrows, ncols));
        self.current_window_id = window_id;
        self.current_pixel_bounds = pixel_bounds;
        self.current_text_pixel_bounds = text_pixel_bounds;
        self.current_selected = selected;
        self.current_row = 0;
        self.in_row = false;
    }

    pub fn end_window(&mut self) {
        if let Some(matrix) = self.current_matrix.take() {
            self.windows.push(WindowMatrixEntry {
                window_id: self.current_window_id,
                matrix,
                pixel_bounds: self.current_pixel_bounds,
                text_pixel_bounds: self.current_text_pixel_bounds,
                selected: self.current_selected,
            });
        }
    }

    pub fn begin_row(&mut self, row: usize, role: GlyphRowRole) {
        self.current_row = row;
        self.in_row = true;
        if let Some(ref mut matrix) = self.current_matrix {
            if row < matrix.rows.len() {
                matrix.rows[row].role = role;
                matrix.rows[row].enabled = true;
                matrix.rows[row].mode_line = matches!(role, GlyphRowRole::ModeLine);
            }
        }
    }

    pub fn end_row(&mut self) {
        self.reorder_current_row_bidi();
        self.in_row = false;
    }

    /// Close a row whose glyph ordering was already finalized before
    /// installation.
    ///
    /// `end_row` owns bidi normalization for rows built incrementally through
    /// the matrix builder. Rows produced by `DisplayRowBuilder` have already
    /// gone through the same normalization, so closing them must only update
    /// builder state.
    pub fn end_prebuilt_row(&mut self) {
        self.in_row = false;
    }

    /// Record authoritative geometry for the currently open row.
    ///
    /// `pixel_y` is frame-absolute; the builder stores rows
    /// window-relative to match GNU `struct glyph_row::y`.
    pub fn set_current_row_metrics(&mut self, pixel_y: f32, height_px: f32, ascent_px: f32) {
        if let Some(ref mut matrix) = self.current_matrix
            && self.current_row < matrix.rows.len()
        {
            let pixel_y_rel = pixel_y - self.current_pixel_bounds.y;
            Self::write_row_metrics(
                &mut matrix.rows[self.current_row],
                pixel_y_rel,
                height_px,
                ascent_px,
            );
        }
    }

    /// Record authoritative geometry for an explicit row in the current window.
    ///
    /// `pixel_y` is frame-absolute; the stored row value is window-relative.
    pub fn set_row_metrics(&mut self, row: usize, pixel_y: f32, height_px: f32, ascent_px: f32) {
        if let Some(ref mut matrix) = self.current_matrix
            && row < matrix.rows.len()
        {
            let pixel_y_rel = pixel_y - self.current_pixel_bounds.y;
            Self::write_row_metrics(&mut matrix.rows[row], pixel_y_rel, height_px, ascent_px);
        }
    }

    /// Install a complete set of text-area glyphs into the currently open row.
    ///
    /// Used by walkers that render directly into the active window matrix
    /// instead of appending a post-window chrome row.
    pub fn install_current_row_glyphs(&mut self, glyphs: Vec<Glyph>) {
        if let Some(ref mut matrix) = self.current_matrix {
            if self.current_row < matrix.rows.len() {
                let row = &mut matrix.rows[self.current_row];
                row.displays_text = !glyphs.is_empty();
                row.glyphs[GlyphArea::Text.index()] = glyphs;
            }
        }
    }

    pub(crate) fn with_current_row_mut<R>(
        &mut self,
        f: impl FnOnce(&mut GlyphRow) -> R,
    ) -> Option<R> {
        let matrix = self.current_matrix.as_mut()?;
        let row = matrix.rows.get_mut(self.current_row)?;
        Some(f(row))
    }

    /// Install a complete row whose glyph order and row-level metadata were
    /// produced outside the matrix builder.
    ///
    /// The source row's `pixel_y` is frame-absolute; rows stored in a window
    /// matrix use window-relative Y, matching GNU `struct glyph_row::y`.
    pub fn install_prebuilt_current_row(&mut self, source: &GlyphRow) {
        if let Some(ref mut matrix) = self.current_matrix {
            if self.current_row < matrix.rows.len() {
                let row = &mut matrix.rows[self.current_row];
                row.glyphs = source.glyphs.clone();
                row.hash = source.hash;
                row.enabled = source.enabled;
                row.role = source.role;
                row.cursor_col = source.cursor_col;
                row.cursor_type = source.cursor_type;
                row.truncated_left = source.truncated_left;
                row.continued = source.continued;
                row.reversed_p = source.reversed_p;
                row.displays_text = source.displays_text;
                row.ends_at_zv = source.ends_at_zv;
                row.mode_line = source.mode_line;
                row.start_charpos = source.start_charpos;
                row.end_charpos = source.end_charpos;
                let pixel_y_rel = source.pixel_y - self.current_pixel_bounds.y;
                Self::write_row_metrics(row, pixel_y_rel, source.height_px, source.ascent_px);
            }
        }
    }

    pub fn push_left_margin_char(&mut self, ch: char, face_id: u32) {
        if let Some(ref mut matrix) = self.current_matrix {
            if self.current_row < matrix.rows.len() {
                matrix.rows[self.current_row].glyphs[GlyphArea::LeftMargin.index()]
                    .push(Glyph::char(ch, face_id, 0));
            }
        }
    }

    pub fn push_left_margin_stretch(&mut self, width_cols: u16, face_id: u32) {
        if let Some(ref mut matrix) = self.current_matrix {
            if self.current_row < matrix.rows.len() {
                matrix.rows[self.current_row].glyphs[GlyphArea::LeftMargin.index()]
                    .push(Glyph::stretch(width_cols, face_id));
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn push_char(&mut self, ch: char, face_id: u32, charpos: usize) {
        self.push_char_with_pixel_width(ch, face_id, charpos, 0.0);
    }

    #[cfg(test)]
    pub(crate) fn push_char_to_row(
        row: &mut GlyphRow,
        ch: char,
        face_id: u32,
        charpos: usize,
        pixel_width: f32,
    ) {
        crate::glyph_row_writer::push_char_to_row(row, ch, face_id, charpos, pixel_width);
    }

    #[cfg(test)]
    pub(crate) fn push_char_with_pixel_width(
        &mut self,
        ch: char,
        face_id: u32,
        charpos: usize,
        pixel_width: f32,
    ) {
        if let Some(ref mut matrix) = self.current_matrix {
            if self.current_row < matrix.rows.len() {
                Self::push_char_to_row(
                    &mut matrix.rows[self.current_row],
                    ch,
                    face_id,
                    charpos,
                    pixel_width,
                );
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn push_wide_char(&mut self, ch: char, face_id: u32, charpos: usize) {
        self.push_wide_char_with_pixel_width(ch, face_id, charpos, 0.0);
    }

    #[cfg(test)]
    pub(crate) fn push_wide_char_to_row(
        row: &mut GlyphRow,
        ch: char,
        face_id: u32,
        charpos: usize,
        pixel_width: f32,
    ) {
        crate::glyph_row_writer::push_wide_char_to_row(row, ch, face_id, charpos, pixel_width);
    }

    #[cfg(test)]
    pub(crate) fn push_wide_char_with_pixel_width(
        &mut self,
        ch: char,
        face_id: u32,
        charpos: usize,
        pixel_width: f32,
    ) {
        if let Some(ref mut matrix) = self.current_matrix {
            if self.current_row < matrix.rows.len() {
                Self::push_wide_char_to_row(
                    &mut matrix.rows[self.current_row],
                    ch,
                    face_id,
                    charpos,
                    pixel_width,
                );
            }
        }
    }

    /// Append a grapheme-cluster continuation character — a ZWJ-joined
    /// emoji, the second regional indicator of a flag, a combining mark,
    /// a variation selector, etc. — to the last emitted text glyph,
    /// upgrading it to a `Composite` so the renderer shapes the whole
    /// cluster as one unit. Falls back to a standalone glyph when there
    /// is no mergeable base (e.g. a stray ZWJ at the start of a row).
    ///
    /// Mirrors GNU's automatic composition, which collapses a grapheme
    /// cluster into a single `COMPOSITE_GLYPH` (see `src/composite.c` and
    /// `produce_composite_glyph` in `src/term.c`).
    pub(crate) fn push_cluster_continuation_to_row(
        row: &mut GlyphRow,
        ch: char,
        face_id: u32,
        charpos: usize,
    ) {
        crate::glyph_row_writer::push_cluster_continuation_to_row(row, ch, face_id, charpos);
    }

    pub fn push_cluster_continuation(&mut self, ch: char, face_id: u32, charpos: usize) {
        if let Some(ref mut matrix) = self.current_matrix {
            if self.current_row < matrix.rows.len() {
                Self::push_cluster_continuation_to_row(
                    &mut matrix.rows[self.current_row],
                    ch,
                    face_id,
                    charpos,
                );
            }
        }
    }

    pub(crate) fn last_text_cluster_tail_in_row(row: &GlyphRow) -> Option<(char, bool)> {
        crate::composition::last_text_cluster_tail_in_row(row)
    }

    pub fn last_text_cluster_tail(&self) -> Option<(char, bool)> {
        let matrix = self.current_matrix.as_ref()?;
        let row = matrix.rows.get(self.current_row)?;
        Self::last_text_cluster_tail_in_row(row)
    }

    /// Grow a contextual-shaping run (Arabic, Indic) by appending `ch` to the
    /// last text glyph's composed cluster AND pushing a padding cell carrying
    /// `ch`'s own buffer position. The run becomes one `Composite` glyph that
    /// the renderer shapes as a unit (joining / reordering), occupying one
    /// column per character; the per-char padding keeps per-letter cursor
    /// positions. Mirrors GNU emitting a multi-character composition while
    /// each character keeps a distinct buffer position. Falls back to a
    /// standalone glyph when there is no base to merge into (run start).
    #[cfg(test)]
    pub(crate) fn push_run_member_to_row(
        row: &mut GlyphRow,
        ch: char,
        face_id: u32,
        charpos: usize,
        pixel_width: f32,
    ) {
        crate::glyph_row_writer::push_run_member_to_row(row, ch, face_id, charpos, pixel_width);
    }

    #[cfg(test)]
    pub(crate) fn push_run_member(
        &mut self,
        ch: char,
        face_id: u32,
        charpos: usize,
        pixel_width: f32,
    ) {
        if let Some(ref mut matrix) = self.current_matrix {
            if self.current_row < matrix.rows.len() {
                Self::push_run_member_to_row(
                    &mut matrix.rows[self.current_row],
                    ch,
                    face_id,
                    charpos,
                    pixel_width,
                );
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn push_stretch(&mut self, width_cols: u16, face_id: u32) {
        self.push_stretch_with_pixel_width(width_cols, face_id, 0.0);
    }

    #[cfg(test)]
    pub(crate) fn push_stretch_to_row(
        row: &mut GlyphRow,
        width_cols: u16,
        face_id: u32,
        pixel_width: f32,
        pixel_height: f32,
        pixel_ascent: f32,
    ) {
        crate::glyph_row_writer::push_stretch_to_row(
            row,
            width_cols,
            face_id,
            pixel_width,
            pixel_height,
            pixel_ascent,
        );
    }

    #[cfg(test)]
    pub(crate) fn push_stretch_with_pixel_width(
        &mut self,
        width_cols: u16,
        face_id: u32,
        pixel_width: f32,
    ) {
        self.push_stretch_with_pixel_geometry(width_cols, face_id, pixel_width, 0.0, 0.0);
    }

    #[cfg(test)]
    pub(crate) fn push_stretch_with_pixel_geometry(
        &mut self,
        width_cols: u16,
        face_id: u32,
        pixel_width: f32,
        pixel_height: f32,
        pixel_ascent: f32,
    ) {
        if let Some(ref mut matrix) = self.current_matrix {
            if self.current_row < matrix.rows.len() {
                Self::push_stretch_to_row(
                    &mut matrix.rows[self.current_row],
                    width_cols,
                    face_id,
                    pixel_width,
                    pixel_height,
                    pixel_ascent,
                );
            }
        }
    }

    pub fn set_cursor(
        &mut self,
        col: u16,
        style: neomacs_display_protocol::frame_glyphs::CursorStyle,
    ) {
        self.set_cursor_at_row(self.current_row, col, style);
    }

    pub fn set_cursor_at_row(
        &mut self,
        row: usize,
        col: u16,
        style: neomacs_display_protocol::frame_glyphs::CursorStyle,
    ) {
        if let Some(ref mut matrix) = self.current_matrix {
            if row < matrix.rows.len() {
                matrix.rows[row].cursor_col = Some(col);
                matrix.rows[row].cursor_type = Some(style);
            }
        }
    }

    pub fn set_row_charpos(&mut self, start: usize, end: usize) {
        if let Some(ref mut matrix) = self.current_matrix {
            if self.current_row < matrix.rows.len() {
                matrix.rows[self.current_row].start_charpos = start;
                matrix.rows[self.current_row].end_charpos = end;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Non-grid item push methods
    // -----------------------------------------------------------------------

    pub fn push_background(&mut self, bounds: Rect, color: Color) {
        self.backgrounds.push(BackgroundItem { bounds, color });
    }

    pub fn push_border(&mut self, window_id: i64, x: f32, y: f32, w: f32, h: f32, color: Color) {
        self.borders.push(BorderItem {
            window_id,
            x,
            y,
            width: w,
            height: h,
            color,
        });
    }

    pub fn push_scroll_bar(&mut self, item: neomacs_display_protocol::glyph_matrix::ScrollBarItem) {
        self.scroll_bars.push(item);
    }

    pub fn push_cursor(
        &mut self,
        window_id: i64,
        slot_id: DisplaySlotId,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        style: CursorStyle,
        color: Color,
    ) {
        self.cursors.push(CursorItem {
            window_id,
            slot_id,
            x,
            y,
            width: w,
            height: h,
            style,
            color,
        });
    }

    fn push_image_with_slot_id(
        &mut self,
        window_id: i64,
        role: GlyphRowRole,
        clip: Option<Rect>,
        slot_id: DisplaySlotId,
        image_id: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) {
        self.images.push(ImageItem {
            window_id,
            row_role: role,
            clip_rect: clip,
            slot_id: Some(slot_id),
            image_id,
            x,
            y,
            width: w,
            height: h,
        });
    }

    pub(crate) fn push_current_window_image(
        &mut self,
        role: GlyphRowRole,
        row: u32,
        col: u16,
        image_id: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) {
        let window_id = self.current_window_id as i64;
        self.push_image_with_slot_id(
            window_id,
            role,
            Some(self.current_text_pixel_bounds),
            DisplaySlotId {
                window_id,
                row,
                col,
            },
            image_id,
            x,
            y,
            w,
            h,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn push_current_window_image_with_clip(
        &mut self,
        role: GlyphRowRole,
        row: u32,
        col: u16,
        clip: Rect,
        image_id: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) {
        let window_id = self.current_window_id as i64;
        self.push_image_with_slot_id(
            window_id,
            role,
            Some(clip),
            DisplaySlotId {
                window_id,
                row,
                col,
            },
            image_id,
            x,
            y,
            w,
            h,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn push_frame_chrome_image(
        &mut self,
        role: GlyphRowRole,
        row: u32,
        col: u16,
        clip: Rect,
        image_id: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) {
        const FRAME_CHROME_WINDOW_ID: i64 = 0;
        self.push_image_with_slot_id(
            FRAME_CHROME_WINDOW_ID,
            role,
            Some(clip),
            DisplaySlotId {
                window_id: FRAME_CHROME_WINDOW_ID,
                row,
                col,
            },
            image_id,
            x,
            y,
            w,
            h,
        );
    }

    fn push_video_with_slot_id(
        &mut self,
        window_id: i64,
        role: GlyphRowRole,
        clip: Option<Rect>,
        slot_id: DisplaySlotId,
        video_id: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        loop_count: i32,
        autoplay: bool,
    ) {
        self.videos.push(VideoItem {
            window_id,
            row_role: role,
            clip_rect: clip,
            slot_id: Some(slot_id),
            video_id,
            x,
            y,
            width: w,
            height: h,
            loop_count,
            autoplay,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn push_current_window_video(
        &mut self,
        role: GlyphRowRole,
        row: u32,
        col: u16,
        video_id: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        loop_count: i32,
        autoplay: bool,
    ) {
        let window_id = self.current_window_id as i64;
        self.push_video_with_slot_id(
            window_id,
            role,
            Some(self.current_text_pixel_bounds),
            DisplaySlotId {
                window_id,
                row,
                col,
            },
            video_id,
            x,
            y,
            w,
            h,
            loop_count,
            autoplay,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn push_current_window_video_with_clip(
        &mut self,
        role: GlyphRowRole,
        row: u32,
        col: u16,
        clip: Rect,
        video_id: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        loop_count: i32,
        autoplay: bool,
    ) {
        let window_id = self.current_window_id as i64;
        self.push_video_with_slot_id(
            window_id,
            role,
            Some(clip),
            DisplaySlotId {
                window_id,
                row,
                col,
            },
            video_id,
            x,
            y,
            w,
            h,
            loop_count,
            autoplay,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn push_frame_chrome_video(
        &mut self,
        role: GlyphRowRole,
        row: u32,
        col: u16,
        clip: Rect,
        video_id: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        loop_count: i32,
        autoplay: bool,
    ) {
        const FRAME_CHROME_WINDOW_ID: i64 = 0;
        self.push_video_with_slot_id(
            FRAME_CHROME_WINDOW_ID,
            role,
            Some(clip),
            DisplaySlotId {
                window_id: FRAME_CHROME_WINDOW_ID,
                row,
                col,
            },
            video_id,
            x,
            y,
            w,
            h,
            loop_count,
            autoplay,
        );
    }

    fn push_xwidget_with_slot_id(
        &mut self,
        window_id: i64,
        role: GlyphRowRole,
        clip: Option<Rect>,
        slot_id: DisplaySlotId,
        xwidget_id: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) {
        self.xwidgets.push(XwidgetItem {
            window_id,
            row_role: role,
            clip_rect: clip,
            slot_id: Some(slot_id),
            xwidget_id,
            x,
            y,
            width: w,
            height: h,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn push_frame_chrome_xwidget(
        &mut self,
        role: GlyphRowRole,
        row: u32,
        col: u16,
        clip: Rect,
        xwidget_id: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) {
        const FRAME_CHROME_WINDOW_ID: i64 = 0;
        self.push_xwidget_with_slot_id(
            FRAME_CHROME_WINDOW_ID,
            role,
            Some(clip),
            DisplaySlotId {
                window_id: FRAME_CHROME_WINDOW_ID,
                row,
                col,
            },
            xwidget_id,
            x,
            y,
            w,
            h,
        );
    }

    pub(crate) fn push_current_window_xwidget(
        &mut self,
        role: GlyphRowRole,
        row: u32,
        col: u16,
        xwidget_id: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) {
        let window_id = self.current_window_id as i64;
        self.push_xwidget_with_slot_id(
            window_id,
            role,
            Some(self.current_text_pixel_bounds),
            DisplaySlotId {
                window_id,
                row,
                col,
            },
            xwidget_id,
            x,
            y,
            w,
            h,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn push_current_window_xwidget_with_clip(
        &mut self,
        role: GlyphRowRole,
        row: u32,
        col: u16,
        clip: Rect,
        xwidget_id: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) {
        let window_id = self.current_window_id as i64;
        self.push_xwidget_with_slot_id(
            window_id,
            role,
            Some(clip),
            DisplaySlotId {
                window_id,
                row,
                col,
            },
            xwidget_id,
            x,
            y,
            w,
            h,
        );
    }

    /// Resolve the materialize-grid column the cursor at `cursor.charpos` on
    /// `cursor.row` actually occupies, or `None` when the cursor is not on the
    /// current window's matrix. This is the single authority for "which display
    /// column is point on"; `set_phys_cursor` applies it to the phys cursor.
    ///
    /// The column must equal the one `FrameDisplayState::materialize_grid_row`
    /// assigns to point's glyph: a single running counter over the LeftMargin
    /// (line numbers, fringe) area and then the Text area, skipping padding
    /// cells and weighting each glyph by its cell span. Counting only the
    /// Text-area index drops the line-number gutter, so the renderer would snap
    /// the cursor to a glyph `lnum_cols` cells to the left (or into the gutter
    /// on short lines), drawing a stray second cursor. GNU accounts for the same
    /// gutter in `set_cursor_from_row`, where the line-number glyphs live at the
    /// start of TEXT_AREA (src/xdisp.c).
    ///
    /// An exact charpos match places the cursor on point's own glyph. When point
    /// sits on invisible/hidden text (e.g. an org heading's collapsed `#+title:`
    /// or leading stars produce no glyph for that charpos), GNU's
    /// set_cursor_from_row instead places the cursor on the first visible glyph
    /// that follows point. We track the glyph with the smallest charpos greater
    /// than point as that fallback, so the cursor never reverts to the captured
    /// column (which would land on the line-number gutter and draw a stray
    /// second cursor).
    fn resolve_cursor_visual_col(&self, window_id: i64, row: usize, charpos: usize) -> Option<u16> {
        if !cursor_window_matches_current(window_id, self.current_window_id) {
            return None;
        }
        let matrix = self.current_matrix.as_ref()?;
        let row = matrix.rows.get(row)?;

        let mut col_acc: u16 = 0;
        for glyph in &row.glyphs[GlyphArea::LeftMargin.index()] {
            if glyph.padding {
                continue;
            }
            col_acc = col_acc.saturating_add(glyph_cell_span(glyph));
        }

        let mut nearest_after: Option<(usize, u16)> = None;
        for glyph in &row.glyphs[GlyphArea::Text.index()] {
            if glyph.padding {
                continue;
            }
            if glyph.charpos == charpos {
                return Some(col_acc);
            }
            if glyph.charpos > charpos
                && nearest_after.is_none_or(|(after, _)| glyph.charpos < after)
            {
                nearest_after = Some((glyph.charpos, col_acc));
            }
            col_acc = col_acc.saturating_add(glyph_cell_span(glyph));
        }
        // No glyph carries point's charpos. Point is either before the first
        // visible glyph (a hidden prefix -- use the first following glyph's
        // column, tracked in nearest_after) or past the row's last glyph (end
        // of line, or a blank line that has only gutter glyphs -- use col_acc,
        // the first cell after all the gutter and text). Returning col_acc
        // rather than None keeps a blank/EOL cursor out of the line-number
        // gutter (where the captured Text-index 0 would land it), matching GNU
        // set_cursor_from_row placing the cursor in the empty area after a row.
        Some(nearest_after.map_or(col_acc, |(_, col)| col))
    }

    pub fn set_phys_cursor(&mut self, cursor: PhysCursor) {
        let mut cursor = cursor;
        let visual_col =
            self.resolve_cursor_visual_col(cursor.window_id, cursor.row, cursor.charpos);

        if let Some(col) = visual_col
            && col != cursor.col
        {
            cursor.col = col;
            cursor.slot_id.col = col;
            if let Some(matrix) = self.current_matrix.as_ref()
                && matrix.ncols > 0
            {
                let char_w = self.current_pixel_bounds.width / matrix.ncols as f32;
                cursor.x = self.current_pixel_bounds.x + col as f32 * char_w;
            }
        }

        if let Some(col) = visual_col
            && let Some(ref mut matrix) = self.current_matrix
            && cursor.row < matrix.rows.len()
        {
            matrix.rows[cursor.row].cursor_col = Some(col);
            matrix.rows[cursor.row].cursor_type = Some(cursor.style);
        }

        // The selected window is represented solely by the phys cursor: the
        // engine no longer pushes a redundant per-window CursorItem for it (see
        // the `!params.selected` guard around push_cursor in engine.rs), so
        // there is nothing to keep in sync here.
        self.phys_cursor = Some(cursor);
    }

    pub fn set_window_cursor_effects(&mut self, window_id: i64, effects: EffectsConfig) {
        self.cursor_effects_by_window.insert(window_id, effects);
    }

    pub fn set_faces(&mut self, faces: HashMap<u32, Face>) {
        self.faces = faces;
    }

    pub fn insert_face(&mut self, id: u32, face: Face) {
        self.faces.insert(id, face);
    }

    pub fn set_stipple_patterns(&mut self, patterns: HashMap<i32, StipplePattern>) {
        self.stipple_patterns = patterns;
    }

    pub fn push_window_info(&mut self, info: WindowInfo) {
        self.window_infos.push(info);
    }

    pub fn push_transition_hint(&mut self, hint: WindowTransitionHint) {
        self.transition_hints.push(hint);
    }

    pub fn push_effect_hint(&mut self, hint: WindowEffectHint) {
        self.effect_hints.push(hint);
    }

    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
    }

    pub fn set_font_pixel_size(&mut self, size: f32) {
        self.font_pixel_size = size;
    }

    pub fn windows(&self) -> &[WindowMatrixEntry] {
        &self.windows
    }

    pub fn window_infos(&self) -> &[WindowInfo] {
        &self.window_infos
    }

    pub fn window_infos_last_mut(&mut self) -> Option<&mut WindowInfo> {
        self.window_infos.last_mut()
    }

    pub fn transition_hints(&self) -> &[WindowTransitionHint] {
        &self.transition_hints
    }

    pub fn effect_hints(&self) -> &[WindowEffectHint] {
        &self.effect_hints
    }

    pub fn truncate_transition_hints(&mut self, len: usize) {
        self.transition_hints.truncate(len);
    }

    pub fn truncate_effect_hints(&mut self, len: usize) {
        self.effect_hints.truncate(len);
    }

    pub fn background_color(&self) -> &Color {
        &self.background_color
    }

    pub fn faces(&self) -> &HashMap<u32, Face> {
        &self.faces
    }

    pub fn cursors(&self) -> &[CursorItem] {
        &self.cursors
    }

    pub fn phys_cursor(&self) -> Option<&PhysCursor> {
        self.phys_cursor.as_ref()
    }

    pub fn set_frame_identity(
        &mut self,
        frame_id: u64,
        parent_id: u64,
        parent_x: f32,
        parent_y: f32,
        z_order: i32,
        undecorated: bool,
        border_width: f32,
        border_color: Color,
        background_alpha: f32,
        no_accept_focus: bool,
    ) {
        self.frame_id = frame_id;
        self.parent_id = parent_id;
        self.parent_x = parent_x;
        self.parent_y = parent_y;
        self.z_order = z_order;
        self.undecorated = undecorated;
        self.border_width = border_width;
        self.border_color = border_color;
        self.background_alpha = background_alpha;
        self.no_accept_focus = no_accept_focus;
    }

    /// Begin a new status-line row in the currently open window.
    ///
    /// Call this BEFORE `end_window()`.  Pushes a new enabled, mode-line
    /// row to the current window's matrix and returns `true` on success.
    /// Returns `false` when no window is currently open (`current_matrix`
    /// is None).
    pub fn begin_status_line_row(&mut self, role: GlyphRowRole) -> bool {
        let Some(ref mut matrix) = self.current_matrix else {
            return false;
        };
        let mut row = GlyphRow::new(role);
        row.enabled = true;
        row.mode_line = true;
        self.current_row = matrix.rows.len();
        matrix.rows.push(row);
        matrix.nrows += 1;
        true
    }

    /// Record authoritative geometry for the last row in the currently
    /// open window.
    ///
    /// `pixel_y` is frame-absolute; the stored row value is window-relative.
    pub fn set_current_window_last_row_metrics(
        &mut self,
        pixel_y: f32,
        height_px: f32,
        ascent_px: f32,
    ) {
        let window_y = self.current_pixel_bounds.y;
        let Some(ref mut matrix) = self.current_matrix else {
            return;
        };
        let Some(row) = matrix.rows.last_mut() else {
            return;
        };
        let pixel_y_rel = pixel_y - window_y;
        Self::write_row_metrics(row, pixel_y_rel, height_px, ascent_px);
    }

    /// Install a complete set of text-area glyphs into the current
    /// status-line row of the currently open window.
    ///
    /// Must be called after `begin_status_line_row`.
    pub fn install_status_line_row_glyphs(&mut self, glyphs: Vec<Glyph>) {
        let Some(ref mut matrix) = self.current_matrix else {
            return;
        };
        if self.current_row < matrix.rows.len() {
            let row = &mut matrix.rows[self.current_row];
            row.displays_text = !glyphs.is_empty();
            row.glyphs[GlyphArea::Text.index()] = glyphs;
            let _ = crate::glyph_row_writer::reorder_row_bidi(row, None);
        }
    }

    /// Normalize a standalone row built outside the window-matrix walker.
    ///
    /// Used for frame-level chrome rows such as the tab bar, which are
    /// produced before any leaf window exists but still need the same bidi
    /// reordering and row bookkeeping as status-line rows.
    pub fn normalize_external_row(row: &mut GlyphRow) {
        crate::glyph_row_writer::normalize_external_row(row);
    }

    /// Patch the last-closed window matrix so its rightmost
    /// column shows a vertical-border glyph on every enabled row.
    ///
    /// Mirrors GNU `src/dispnew.c::build_frame_matrix_from_leaf_window`
    /// (2568-2697), which — for every window that is not the
    /// rightmost in the frame — takes the window's row slice and
    /// overwrites its last glyph with `right_border_glyph`
    /// (default `|`, face `VERTICAL_BORDER_FACE_ID`):
    ///
    ///   if (!WINDOW_RIGHTMOST_P (w))
    ///     SET_GLYPH_FROM_CHAR (right_border_glyph, '|');
    ///   ...
    ///   if (GLYPH_CHAR (right_border_glyph) != 0) {
    ///     struct glyph *border = window_row->glyphs[LAST_AREA] - 1;
    ///     SET_CHAR_GLYPH_FROM_GLYPH (f, *border, right_border_glyph);
    ///   }
    ///
    /// The window's text has already been laid out to fill all
    /// `ncols` columns; the last glyph position is then replaced
    /// with the border character. On TTY, the column corresponds
    /// to one character cell.
    ///
    /// This helper operates on the LAST window pushed into
    /// `self.windows`, which is the window most recently closed
    /// by `end_window`. Callers (`engine.rs::layout_frame_rust`)
    /// invoke this after `layout_window_rust` returns for a
    /// non-rightmost window.
    pub fn overwrite_last_window_right_border(&mut self, ch: char, face_id: u32) {
        let Some(entry) = self.windows.last_mut() else {
            return;
        };
        let ncols = entry.matrix.ncols;
        if ncols == 0 {
            return;
        }
        let target_col = ncols - 1;

        for row in &mut entry.matrix.rows {
            Self::pad_row_and_write_glyph(row, target_col, ch, face_id, GlyphArea::RightMargin);
        }
    }

    pub fn overwrite_current_window_row_last_glyph(
        &mut self,
        row_idx: usize,
        ch: char,
        face_id: u32,
    ) {
        let Some(matrix) = self.current_matrix.as_ref() else {
            return;
        };
        let ncols = matrix.ncols;
        if ncols == 0 {
            return;
        }
        self.overwrite_current_window_row_glyph_at_col(row_idx, ncols - 1, ch, face_id);
    }

    pub fn overwrite_current_window_row_glyph_at_col(
        &mut self,
        row_idx: usize,
        target_col: usize,
        ch: char,
        face_id: u32,
    ) {
        let Some(matrix) = self.current_matrix.as_mut() else {
            return;
        };
        let ncols = matrix.ncols;
        if ncols == 0 {
            return;
        }
        let Some(row) = matrix.rows.get_mut(row_idx) else {
            return;
        };
        let clamped_col = target_col.min(ncols - 1);
        Self::pad_row_and_write_glyph(row, clamped_col, ch, face_id, GlyphArea::Text);
    }

    pub fn current_window_row_enabled(&self, row_idx: usize) -> bool {
        self.current_matrix
            .as_ref()
            .and_then(|matrix| matrix.rows.get(row_idx))
            .is_some_and(|row| row.enabled)
    }

    pub fn enable_current_window_row(&mut self, row_idx: usize) {
        let Some(matrix) = self.current_matrix.as_mut() else {
            return;
        };
        let Some(row) = matrix.rows.get_mut(row_idx) else {
            return;
        };
        row.enabled = true;
    }

    pub fn set_current_window_row_role(&mut self, row_idx: usize, role: GlyphRowRole) {
        let Some(matrix) = self.current_matrix.as_mut() else {
            return;
        };
        let Some(row) = matrix.rows.get_mut(row_idx) else {
            return;
        };
        row.role = role;
    }

    pub fn set_current_window_row_metrics(
        &mut self,
        row_idx: usize,
        pixel_y: f32,
        height_px: f32,
        ascent_px: f32,
    ) {
        let Some(matrix) = self.current_matrix.as_mut() else {
            return;
        };
        let Some(row) = matrix.rows.get_mut(row_idx) else {
            return;
        };
        let pixel_y_rel = pixel_y - self.current_pixel_bounds.y;
        Self::write_row_metrics(row, pixel_y_rel, height_px, ascent_px);
    }

    pub fn finish(
        mut self,
        frame_cols: usize,
        frame_rows: usize,
        char_width: f32,
        char_height: f32,
    ) -> FrameDisplayState {
        for entry in &mut self.windows {
            entry.matrix.ensure_hashes();
        }
        let mut state = FrameDisplayState::new(frame_cols, frame_rows, char_width, char_height);
        state.window_matrices = self.windows;
        state.backgrounds = self.backgrounds;
        state.borders = self.borders;
        state.cursors = self.cursors;
        state.images = self.images;
        state.videos = self.videos;
        state.xwidgets = self.xwidgets;
        state.scroll_bars = self.scroll_bars;
        state.phys_cursor = self.phys_cursor;
        state.cursor_effects_by_window = self.cursor_effects_by_window;
        state.faces = self.faces;
        state.stipple_patterns = self.stipple_patterns;
        state.window_infos = self.window_infos;
        state.transition_hints = self.transition_hints;
        state.effect_hints = self.effect_hints;
        state.background = self.background_color;
        state.font_pixel_size = self.font_pixel_size;
        state.frame_id = self.frame_id;
        state.parent_id = self.parent_id;
        state.parent_x = self.parent_x;
        state.parent_y = self.parent_y;
        state.z_order = self.z_order;
        state.undecorated = self.undecorated;
        state.border_width = self.border_width;
        state.border_color = self.border_color;
        state.background_alpha = self.background_alpha;
        state.no_accept_focus = self.no_accept_focus;
        state
    }

    pub fn finish_with_pixel_size(
        self,
        frame_cols: usize,
        frame_rows: usize,
        char_width: f32,
        char_height: f32,
        frame_pixel_width: f32,
        frame_pixel_height: f32,
    ) -> FrameDisplayState {
        let mut state = self.finish(frame_cols, frame_rows, char_width, char_height);
        state.frame_pixel_width = frame_pixel_width;
        state.frame_pixel_height = frame_pixel_height;
        state
    }

    #[cfg(test)]
    pub(crate) fn reorder_row_bidi(
        row: &mut GlyphRow,
        phys_cursor_col: Option<u16>,
    ) -> Option<u16> {
        crate::glyph_row_writer::reorder_row_bidi(row, phys_cursor_col)
    }

    fn reorder_current_row_bidi(&mut self) {
        let remapped_cursor_col = if let Some(ref mut matrix) = self.current_matrix {
            if self.current_row >= matrix.rows.len() {
                return;
            }

            let phys_cursor_col = self
                .phys_cursor
                .as_ref()
                .filter(|cursor| {
                    cursor_window_matches_current(cursor.window_id, self.current_window_id)
                        && cursor.row == self.current_row
                })
                .map(|cursor| cursor.col);

            crate::glyph_row_writer::reorder_row_bidi(
                &mut matrix.rows[self.current_row],
                phys_cursor_col,
            )
        } else {
            None
        };

        if let Some(col) = remapped_cursor_col
            && let Some(ref mut cursor) = self.phys_cursor
            && cursor_window_matches_current(cursor.window_id, self.current_window_id)
            && cursor.row == self.current_row
        {
            cursor.col = col;
            cursor.slot_id.col = col;
            if let Some(ref matrix) = self.current_matrix
                && matrix.ncols > 0
            {
                let char_w = self.current_pixel_bounds.width / matrix.ncols as f32;
                cursor.x = self.current_pixel_bounds.x + col as f32 * char_w;
            }
        }
    }
}

#[cfg(test)]
#[path = "matrix_builder_test.rs"]
mod tests;
