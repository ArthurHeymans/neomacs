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

const FRAME_CHROME_WINDOW_ID: i64 = 0;

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

#[derive(Clone, Copy)]
struct CursorVisualColumnResolutionContext<'a> {
    current_window_id: u64,
    current_pixel_bounds: Rect,
    matrix: Option<&'a GlyphMatrix>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CursorVisualColumnResolutionRequest {
    window_id: i64,
    row: usize,
    charpos: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ResolvedPhysCursorPlacement {
    col: u16,
    x: Option<f32>,
}

impl CursorVisualColumnResolutionRequest {
    fn new(window_id: i64, row: usize, charpos: usize) -> Self {
        Self {
            window_id,
            row,
            charpos,
        }
    }

    fn from_cursor(cursor: &PhysCursor) -> Self {
        Self::new(cursor.window_id, cursor.row, cursor.charpos)
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
    fn resolve(self, context: CursorVisualColumnResolutionContext<'_>) -> Option<u16> {
        if !cursor_window_matches_current(self.window_id, context.current_window_id) {
            return None;
        }
        let matrix = context.matrix?;
        let row = matrix.rows.get(self.row)?;

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
            if glyph.charpos == self.charpos {
                return Some(col_acc);
            }
            if glyph.charpos > self.charpos
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

    fn resolve_phys_cursor_placement(
        self,
        context: CursorVisualColumnResolutionContext<'_>,
    ) -> Option<ResolvedPhysCursorPlacement> {
        let col = self.resolve(context)?;
        let x = context.matrix.and_then(|matrix| {
            (matrix.ncols > 0).then(|| {
                let char_w = context.current_pixel_bounds.width / matrix.ncols as f32;
                context.current_pixel_bounds.x + col as f32 * char_w
            })
        });
        Some(ResolvedPhysCursorPlacement { col, x })
    }
}

impl ResolvedPhysCursorPlacement {
    fn apply_to(self, cursor: &mut PhysCursor) {
        if self.col != cursor.col {
            cursor.col = self.col;
            cursor.slot_id.col = self.col;
            if let Some(x) = self.x {
                cursor.x = x;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum MatrixMediaInstallKind {
    Image {
        image_id: u32,
    },
    Video {
        video_id: u32,
        loop_count: i32,
        autoplay: bool,
    },
    Xwidget {
        xwidget_id: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum MatrixCurrentWindowMediaClip {
    TextBounds,
    Explicit(Rect),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum MatrixMediaInstallTarget {
    CurrentWindow {
        role: GlyphRowRole,
        row: u32,
        col: u16,
        clip: MatrixCurrentWindowMediaClip,
    },
    FrameChrome {
        role: GlyphRowRole,
        row: u32,
        col: u16,
        clip: Rect,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MatrixMediaInstallRequest {
    target: MatrixMediaInstallTarget,
    kind: MatrixMediaInstallKind,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ResolvedMatrixMediaTarget {
    window_id: i64,
    role: GlyphRowRole,
    clip: Option<Rect>,
    slot_id: DisplaySlotId,
}

impl MatrixMediaInstallRequest {
    pub(crate) fn new(
        target: MatrixMediaInstallTarget,
        kind: MatrixMediaInstallKind,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            target,
            kind,
            x,
            y,
            width,
            height,
        }
    }

    fn install(self, builder: &mut GlyphMatrixBuilder) {
        let target = self.target.resolve(builder);
        match self.kind {
            MatrixMediaInstallKind::Image { image_id } => builder.images.push(ImageItem {
                window_id: target.window_id,
                row_role: target.role,
                clip_rect: target.clip,
                slot_id: Some(target.slot_id),
                image_id,
                x: self.x,
                y: self.y,
                width: self.width,
                height: self.height,
            }),
            MatrixMediaInstallKind::Video {
                video_id,
                loop_count,
                autoplay,
            } => builder.videos.push(VideoItem {
                window_id: target.window_id,
                row_role: target.role,
                clip_rect: target.clip,
                slot_id: Some(target.slot_id),
                video_id,
                x: self.x,
                y: self.y,
                width: self.width,
                height: self.height,
                loop_count,
                autoplay,
            }),
            MatrixMediaInstallKind::Xwidget { xwidget_id } => builder.xwidgets.push(XwidgetItem {
                window_id: target.window_id,
                row_role: target.role,
                clip_rect: target.clip,
                slot_id: Some(target.slot_id),
                xwidget_id,
                x: self.x,
                y: self.y,
                width: self.width,
                height: self.height,
            }),
        }
    }
}

impl MatrixMediaInstallTarget {
    fn resolve(self, builder: &GlyphMatrixBuilder) -> ResolvedMatrixMediaTarget {
        match self {
            Self::CurrentWindow {
                role,
                row,
                col,
                clip,
            } => {
                let window_id = builder.current_window_id as i64;
                let clip = match clip {
                    MatrixCurrentWindowMediaClip::TextBounds => builder.current_text_pixel_bounds,
                    MatrixCurrentWindowMediaClip::Explicit(clip) => clip,
                };
                ResolvedMatrixMediaTarget {
                    window_id,
                    role,
                    clip: Some(clip),
                    slot_id: DisplaySlotId {
                        window_id,
                        row,
                        col,
                    },
                }
            }
            Self::FrameChrome {
                role,
                row,
                col,
                clip,
            } => ResolvedMatrixMediaTarget {
                window_id: FRAME_CHROME_WINDOW_ID,
                role,
                clip: Some(clip),
                slot_id: DisplaySlotId {
                    window_id: FRAME_CHROME_WINDOW_ID,
                    row,
                    col,
                },
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MatrixCursorInstallRequest {
    pub(crate) window_id: i64,
    pub(crate) slot_id: DisplaySlotId,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) style: CursorStyle,
    pub(crate) color: Color,
}

impl MatrixCursorInstallRequest {
    fn cursor_item(self) -> CursorItem {
        CursorItem {
            window_id: self.window_id,
            slot_id: self.slot_id,
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            style: self.style,
            color: self.color,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum MatrixFrameArtifactInstallRequest {
    Background {
        bounds: Rect,
        color: Color,
    },
    Border {
        window_id: i64,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    },
    ScrollBar(ScrollBarItem),
    WindowInfo(WindowInfo),
    TransitionHint(WindowTransitionHint),
    EffectHint(WindowEffectHint),
}

impl MatrixFrameArtifactInstallRequest {
    fn install(self, builder: &mut GlyphMatrixBuilder) {
        match self {
            Self::Background { bounds, color } => {
                builder.backgrounds.push(BackgroundItem { bounds, color });
            }
            Self::Border {
                window_id,
                x,
                y,
                width,
                height,
                color,
            } => {
                builder.borders.push(BorderItem {
                    window_id,
                    x,
                    y,
                    width,
                    height,
                    color,
                });
            }
            Self::ScrollBar(item) => builder.scroll_bars.push(item),
            Self::WindowInfo(info) => builder.window_infos.push(info),
            Self::TransitionHint(hint) => builder.transition_hints.push(hint),
            Self::EffectHint(hint) => builder.effect_hints.push(hint),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MatrixFrameIdentityInstallRequest {
    pub(crate) frame_id: u64,
    pub(crate) parent_id: u64,
    pub(crate) parent_x: f32,
    pub(crate) parent_y: f32,
    pub(crate) z_order: i32,
    pub(crate) undecorated: bool,
    pub(crate) border_width: f32,
    pub(crate) border_color: Color,
    pub(crate) background_alpha: f32,
    pub(crate) no_accept_focus: bool,
}

#[derive(Clone, Debug)]
pub(crate) enum MatrixFrameStateInstallRequest {
    Identity(MatrixFrameIdentityInstallRequest),
    BackgroundColor(Color),
    FontPixelSize(f32),
    Faces(HashMap<u32, Face>),
    Face {
        id: u32,
        face: Face,
    },
    CursorEffects {
        window_id: i64,
        effects: EffectsConfig,
    },
}

impl MatrixFrameStateInstallRequest {
    fn install(self, builder: &mut GlyphMatrixBuilder) {
        match self {
            Self::Identity(identity) => {
                builder.frame_id = identity.frame_id;
                builder.parent_id = identity.parent_id;
                builder.parent_x = identity.parent_x;
                builder.parent_y = identity.parent_y;
                builder.z_order = identity.z_order;
                builder.undecorated = identity.undecorated;
                builder.border_width = identity.border_width;
                builder.border_color = identity.border_color;
                builder.background_alpha = identity.background_alpha;
                builder.no_accept_focus = identity.no_accept_focus;
            }
            Self::BackgroundColor(color) => builder.background_color = color,
            Self::FontPixelSize(size) => builder.font_pixel_size = size,
            Self::Faces(faces) => builder.faces = faces,
            Self::Face { id, face } => {
                builder.faces.insert(id, face);
            }
            Self::CursorEffects { window_id, effects } => {
                builder.cursor_effects_by_window.insert(window_id, effects);
            }
        }
    }
}

pub(crate) struct GlyphMatrixBuilder {
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
    fn write_row_metrics(row: &mut GlyphRow, pixel_y_rel: f32, height_px: f32, ascent_px: f32) {
        row.pixel_y = pixel_y_rel;
        row.height_px = height_px.max(0.0);
        row.ascent_px = ascent_px.max(0.0).min(row.height_px.max(0.0));
    }

    pub(crate) fn new() -> Self {
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

    pub(crate) fn reset(&mut self) {
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

    #[cfg(test)]
    pub(crate) fn begin_window(
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

    pub(crate) fn begin_window_with_text_bounds(
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

    pub(crate) fn end_window(&mut self) {
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

    pub(crate) fn begin_row(&mut self, row: usize, role: GlyphRowRole) {
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

    pub(crate) fn end_row(&mut self) {
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
    pub(crate) fn end_prebuilt_row(&mut self) {
        self.in_row = false;
    }

    /// Record authoritative geometry for the currently open row.
    ///
    /// `pixel_y` is frame-absolute; the builder stores rows
    /// window-relative to match GNU `struct glyph_row::y`.
    pub(crate) fn set_current_row_metrics(&mut self, pixel_y: f32, height_px: f32, ascent_px: f32) {
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
    pub(crate) fn set_row_metrics(
        &mut self,
        row: usize,
        pixel_y: f32,
        height_px: f32,
        ascent_px: f32,
    ) {
        if let Some(ref mut matrix) = self.current_matrix
            && row < matrix.rows.len()
        {
            let pixel_y_rel = pixel_y - self.current_pixel_bounds.y;
            Self::write_row_metrics(&mut matrix.rows[row], pixel_y_rel, height_px, ascent_px);
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
    pub(crate) fn install_prebuilt_current_row(&mut self, source: &GlyphRow) {
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

    #[cfg(test)]
    fn install_prebuilt_row(&mut self, row: usize, source: &GlyphRow) {
        self.begin_row(row, source.role);
        self.install_prebuilt_current_row(source);
        self.end_prebuilt_row();
    }

    pub(crate) fn last_text_cluster_tail_in_row(row: &GlyphRow) -> Option<(char, bool)> {
        crate::composition::last_text_cluster_tail_in_row(row)
    }

    pub(crate) fn last_text_cluster_tail(&self) -> Option<(char, bool)> {
        let matrix = self.current_matrix.as_ref()?;
        let row = matrix.rows.get(self.current_row)?;
        Self::last_text_cluster_tail_in_row(row)
    }

    pub(crate) fn set_cursor_at_row(
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

    // -----------------------------------------------------------------------
    // Non-grid item installation
    // -----------------------------------------------------------------------

    pub(crate) fn install_frame_artifact(&mut self, request: MatrixFrameArtifactInstallRequest) {
        request.install(self);
    }

    pub(crate) fn install_cursor(&mut self, request: MatrixCursorInstallRequest) {
        self.cursors.push(request.cursor_item());
    }

    pub(crate) fn install_media(&mut self, request: MatrixMediaInstallRequest) {
        request.install(self);
    }

    fn cursor_visual_column_context(&self) -> CursorVisualColumnResolutionContext<'_> {
        CursorVisualColumnResolutionContext {
            current_window_id: self.current_window_id,
            current_pixel_bounds: self.current_pixel_bounds,
            matrix: self.current_matrix.as_ref(),
        }
    }

    pub(crate) fn set_phys_cursor(&mut self, cursor: PhysCursor) {
        let mut cursor = cursor;
        let placement = CursorVisualColumnResolutionRequest::from_cursor(&cursor)
            .resolve_phys_cursor_placement(self.cursor_visual_column_context());

        if let Some(placement) = placement {
            placement.apply_to(&mut cursor);
        }

        if let Some(placement) = placement
            && let Some(ref mut matrix) = self.current_matrix
            && cursor.row < matrix.rows.len()
        {
            matrix.rows[cursor.row].cursor_col = Some(placement.col);
            matrix.rows[cursor.row].cursor_type = Some(cursor.style);
        }

        // The selected window is represented solely by the phys cursor: the
        // window output no longer installs a redundant per-window CursorItem
        // for it (see the `!cursor.selected` guard around install_cursor), so
        // there is nothing to keep in sync here.
        self.phys_cursor = Some(cursor);
    }

    pub(crate) fn install_frame_state(&mut self, request: MatrixFrameStateInstallRequest) {
        request.install(self);
    }

    pub(crate) fn windows(&self) -> &[WindowMatrixEntry] {
        &self.windows
    }

    pub(crate) fn window_infos(&self) -> &[WindowInfo] {
        &self.window_infos
    }

    pub(crate) fn window_infos_last_mut(&mut self) -> Option<&mut WindowInfo> {
        self.window_infos.last_mut()
    }

    pub(crate) fn transition_hints(&self) -> &[WindowTransitionHint] {
        &self.transition_hints
    }

    pub(crate) fn effect_hints(&self) -> &[WindowEffectHint] {
        &self.effect_hints
    }

    pub(crate) fn truncate_transition_hints(&mut self, len: usize) {
        self.transition_hints.truncate(len);
    }

    pub(crate) fn truncate_effect_hints(&mut self, len: usize) {
        self.effect_hints.truncate(len);
    }

    pub(crate) fn background_color(&self) -> &Color {
        &self.background_color
    }

    #[cfg(test)]
    pub(crate) fn faces(&self) -> &HashMap<u32, Face> {
        &self.faces
    }

    pub(crate) fn cursors(&self) -> &[CursorItem] {
        &self.cursors
    }

    pub(crate) fn phys_cursor(&self) -> Option<&PhysCursor> {
        self.phys_cursor.as_ref()
    }

    pub(crate) fn with_current_window_row_mut<R>(
        &mut self,
        row_idx: usize,
        f: impl FnOnce(&mut GlyphRow, usize) -> R,
    ) -> Option<R> {
        let Some(matrix) = self.current_matrix.as_mut() else {
            return None;
        };
        let ncols = matrix.ncols;
        let Some(row) = matrix.rows.get_mut(row_idx) else {
            return None;
        };
        Some(f(row, ncols))
    }

    pub(crate) fn with_last_window_rows_mut<R>(
        &mut self,
        f: impl FnOnce(&mut [GlyphRow], usize) -> R,
    ) -> Option<R> {
        let entry = self.windows.last_mut()?;
        Some(f(&mut entry.matrix.rows, entry.matrix.ncols))
    }

    pub(crate) fn finish(
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

    pub(crate) fn finish_with_pixel_size(
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
