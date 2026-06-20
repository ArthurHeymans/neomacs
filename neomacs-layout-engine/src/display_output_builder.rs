//! DisplayOutputBuilder — records authoritative window matrices during layout.
//!
//! The builder observes layout emissions and writes them into the per-window
//! `GlyphMatrix` grids published through `FrameDisplayState`. Renderers then
//! materialize that immutable snapshot into runtime glyph buffers on the
//! consumer side; layout no longer treats `FrameGlyphBuffer` as the primary
//! output contract.

#[cfg(test)]
use crate::display_cursor::CursorVisualColumnResolutionRequest;
use crate::display_cursor::{CursorVisualColumnResolutionContext, CursorVisualColumnRows};
#[cfg(test)]
use crate::display_row::resolved_display_row_face;
use crate::display_row_finalizer::GlyphRowFinalizationContext;
#[cfg(test)]
use crate::font_metrics::FontMetrics;
#[cfg(test)]
use crate::neovm_bridge::ResolvedFace;
use neomacs_display_protocol::effect_config::EffectsConfig;
use neomacs_display_protocol::face::Face;
use neomacs_display_protocol::frame_glyphs::{
    CursorStyle, DisplaySlotId, GlyphRowRole, PhysCursor, StipplePattern, WindowEffectHint,
    WindowInfo, WindowTransitionHint,
};
use neomacs_display_protocol::glyph_matrix::*;
use neomacs_display_protocol::types::{Color, Rect};
use std::collections::HashMap;

pub(crate) const FRAME_CHROME_WINDOW_ID: i64 = 0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum OutputMediaInstallKind {
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
pub(crate) struct OutputMediaInstallRequest {
    target: ResolvedOutputMediaInstallTarget,
    kind: OutputMediaInstallKind,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResolvedOutputMediaInstallTarget {
    window_id: i64,
    role: GlyphRowRole,
    clip: Option<Rect>,
    slot_id: DisplaySlotId,
}

impl OutputMediaInstallRequest {
    pub(crate) fn new(
        target: ResolvedOutputMediaInstallTarget,
        kind: OutputMediaInstallKind,
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

    pub(crate) fn image(
        target: ResolvedOutputMediaInstallTarget,
        image_id: u32,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Self {
        Self::new(
            target,
            OutputMediaInstallKind::Image { image_id },
            x,
            y,
            width,
            height,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn video(
        target: ResolvedOutputMediaInstallTarget,
        video_id: u32,
        loop_count: i32,
        autoplay: bool,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Self {
        Self::new(
            target,
            OutputMediaInstallKind::Video {
                video_id,
                loop_count,
                autoplay,
            },
            x,
            y,
            width,
            height,
        )
    }

    pub(crate) fn xwidget(
        target: ResolvedOutputMediaInstallTarget,
        xwidget_id: u32,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Self {
        Self::new(
            target,
            OutputMediaInstallKind::Xwidget { xwidget_id },
            x,
            y,
            width,
            height,
        )
    }

    fn install(self, builder: &mut DisplayOutputBuilder) {
        let target = self.target;
        match self.kind {
            OutputMediaInstallKind::Image { image_id } => builder.images.push(ImageItem {
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
            OutputMediaInstallKind::Video {
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
            OutputMediaInstallKind::Xwidget { xwidget_id } => builder.xwidgets.push(XwidgetItem {
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

impl ResolvedOutputMediaInstallTarget {
    pub(crate) fn new(
        window_id: i64,
        role: GlyphRowRole,
        clip: Option<Rect>,
        slot_id: DisplaySlotId,
    ) -> Self {
        Self {
            window_id,
            role,
            clip,
            slot_id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OutputCursorInstallRequest {
    window_id: i64,
    slot_id: DisplaySlotId,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    style: CursorStyle,
    color: Color,
}

impl OutputCursorInstallRequest {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        window_id: i64,
        slot_id: DisplaySlotId,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: CursorStyle,
        color: Color,
    ) -> Self {
        Self {
            window_id,
            slot_id,
            x,
            y,
            width,
            height,
            style,
            color,
        }
    }

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
pub(crate) enum OutputFrameArtifactInstallRequest {
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
    PhysCursor(PhysCursor),
}

impl OutputFrameArtifactInstallRequest {
    pub(crate) fn phys_cursor(cursor: PhysCursor) -> Self {
        Self::PhysCursor(cursor)
    }

    fn install(self, builder: &mut DisplayOutputBuilder) {
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
            Self::PhysCursor(cursor) => builder.phys_cursor = Some(cursor),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct OutputFrameIdentityInstallRequest {
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

#[derive(Clone, Debug)]
pub(crate) enum OutputFrameStateInstallRequest {
    Identity(OutputFrameIdentityInstallRequest),
    BackgroundColor(Color),
    FontPixelSize(f32),
    Face {
        id: u32,
        face: Face,
    },
    CursorEffects {
        window_id: i64,
        effects: EffectsConfig,
    },
}

impl OutputFrameStateInstallRequest {
    pub(crate) fn face(id: u32, face: Face) -> Self {
        Self::Face { id, face }
    }

    pub(crate) fn cursor_effects(window_id: i64, effects: EffectsConfig) -> Self {
        Self::CursorEffects { window_id, effects }
    }

    fn install(self, builder: &mut DisplayOutputBuilder) {
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
            Self::Face { id, face } => {
                builder.faces.insert(id, face);
            }
            Self::CursorEffects { window_id, effects } => {
                builder.cursor_effects_by_window.insert(window_id, effects);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct OutputWindowBeginRequest {
    window_id: u64,
    nrows: usize,
    ncols: usize,
    pixel_bounds: Rect,
    text_pixel_bounds: Rect,
    selected: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum OutputWindowLifecycleRequest {
    Begin(OutputWindowBeginRequest),
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OutputTextWindowDisplayRangeInstallRequest {
    window_id: i64,
    window_start: i64,
    window_end: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OutputRetryCheckpointRestoreRequest {
    transition_hints_len: usize,
    effect_hints_len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OutputWindowMetadataInstallRequest {
    TextDisplayRange(OutputTextWindowDisplayRangeInstallRequest),
    RestoreRetryCheckpoint(OutputRetryCheckpointRestoreRequest),
}

impl OutputWindowLifecycleRequest {
    fn install(self, builder: &mut DisplayOutputBuilder) {
        match self {
            Self::Begin(begin) => {
                builder.current_row_grid = Some(OutputWindowRowGrid::new(begin.nrows, begin.ncols));
                builder.current_window_id = begin.window_id;
                builder.current_pixel_bounds = begin.pixel_bounds;
                builder.current_text_pixel_bounds = begin.text_pixel_bounds;
                builder.current_selected = begin.selected;
                builder.current_row = 0;
            }
            Self::End => {
                if let Some(grid) = builder.current_row_grid.take() {
                    builder.windows.push(OutputWindowGridEntry::new(
                        builder.current_window_id,
                        grid,
                        builder.current_pixel_bounds,
                        builder.current_text_pixel_bounds,
                        builder.current_selected,
                    ));
                }
            }
        }
    }
}

impl OutputTextWindowDisplayRangeInstallRequest {
    pub(crate) fn new(window_id: i64, window_start: i64, window_end: i64) -> Self {
        Self {
            window_id,
            window_start,
            window_end,
        }
    }
}

impl OutputRetryCheckpointRestoreRequest {
    pub(crate) fn new(transition_hints_len: usize, effect_hints_len: usize) -> Self {
        Self {
            transition_hints_len,
            effect_hints_len,
        }
    }
}

impl From<OutputTextWindowDisplayRangeInstallRequest> for OutputWindowMetadataInstallRequest {
    fn from(request: OutputTextWindowDisplayRangeInstallRequest) -> Self {
        Self::TextDisplayRange(request)
    }
}

impl From<OutputRetryCheckpointRestoreRequest> for OutputWindowMetadataInstallRequest {
    fn from(request: OutputRetryCheckpointRestoreRequest) -> Self {
        Self::RestoreRetryCheckpoint(request)
    }
}

impl OutputWindowMetadataInstallRequest {
    fn install(self, builder: &mut DisplayOutputBuilder) {
        match self {
            Self::TextDisplayRange(range) => {
                if let Some(info) = builder.window_infos.last_mut()
                    && info.window_id == range.window_id
                {
                    info.window_start = range.window_start;
                    info.window_end = range.window_end;
                }
            }
            Self::RestoreRetryCheckpoint(checkpoint) => {
                builder
                    .transition_hints
                    .truncate(checkpoint.transition_hints_len);
                builder.effect_hints.truncate(checkpoint.effect_hints_len);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OutputRowBeginRequest {
    row: usize,
    role: GlyphRowRole,
    mode_line: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct OutputCompleteRowInstallRequest {
    row: usize,
    role: GlyphRowRole,
    mode_line: bool,
    glyph_row: GlyphRow,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OutputRowMetricsRequest {
    /// Stored row Y, relative to the window matrix origin.
    pixel_y: f32,
    height_px: f32,
    ascent_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OutputCurrentRowDecorationRequest {
    MarkTruncatedLeft,
}

#[derive(Clone, Debug)]
pub(crate) enum OutputRowLifecycleRequest {
    Begin(OutputRowBeginRequest),
    Complete(OutputCompleteRowInstallRequest),
    Metrics {
        row: usize,
        metrics: OutputRowMetricsRequest,
    },
    Finalize {
        row: usize,
    },
    Cursor {
        row: usize,
        col: u16,
        style: CursorStyle,
    },
    CurrentDecoration(OutputCurrentRowDecorationRequest),
}

impl OutputRowBeginRequest {
    pub(crate) fn new(row: usize, role: GlyphRowRole, mode_line: bool) -> Self {
        Self {
            row,
            role,
            mode_line,
        }
    }
}

impl OutputCompleteRowInstallRequest {
    pub(crate) fn new(
        row: usize,
        role: GlyphRowRole,
        mode_line: bool,
        glyph_row: GlyphRow,
    ) -> Self {
        Self {
            row,
            role,
            mode_line,
            glyph_row,
        }
    }
}

impl OutputRowMetricsRequest {
    pub(crate) fn new(pixel_y: f32, height_px: f32, ascent_px: f32) -> Self {
        Self {
            pixel_y,
            height_px,
            ascent_px,
        }
    }
}

impl OutputRowLifecycleRequest {
    pub(crate) fn begin(row: usize, role: GlyphRowRole, mode_line: bool) -> Self {
        Self::Begin(OutputRowBeginRequest::new(row, role, mode_line))
    }

    pub(crate) fn complete(
        row: usize,
        role: GlyphRowRole,
        mode_line: bool,
        glyph_row: GlyphRow,
    ) -> Self {
        Self::Complete(OutputCompleteRowInstallRequest::new(
            row, role, mode_line, glyph_row,
        ))
    }

    pub(crate) fn metrics(row: usize, pixel_y: f32, height_px: f32, ascent_px: f32) -> Self {
        Self::Metrics {
            row,
            metrics: OutputRowMetricsRequest::new(pixel_y, height_px, ascent_px),
        }
    }

    pub(crate) fn finalize(row: usize) -> Self {
        Self::Finalize { row }
    }

    pub(crate) fn cursor(row: usize, col: u16, style: CursorStyle) -> Self {
        Self::Cursor { row, col, style }
    }

    pub(crate) fn current_decoration(decoration: OutputCurrentRowDecorationRequest) -> Self {
        Self::CurrentDecoration(decoration)
    }

    fn install(self, builder: &mut DisplayOutputBuilder) {
        match self {
            Self::Begin(begin) => builder.begin_current_row(begin),
            Self::Complete(complete) => builder.install_complete_row(complete),
            Self::Metrics { row, metrics } => builder.write_row_metrics_at(row, metrics),
            Self::Finalize { row } => builder.finalize_output_row(row),
            Self::Cursor { row, col, style } => builder.write_row_cursor(row, col, style),
            Self::CurrentDecoration(decoration) => builder.decorate_current_row(decoration),
        }
    }
}

struct OutputWindowRowGrid {
    matrix: GlyphMatrix,
}

struct OutputWindowGridEntry {
    window_id: u64,
    grid: OutputWindowRowGrid,
    pixel_bounds: Rect,
    text_pixel_bounds: Rect,
    selected: bool,
}

impl OutputWindowRowGrid {
    fn new(nrows: usize, ncols: usize) -> Self {
        Self {
            matrix: GlyphMatrix::new(nrows, ncols),
        }
    }

    fn into_matrix(self) -> GlyphMatrix {
        self.matrix
    }

    fn ensure_hashes(&mut self) {
        self.matrix.ensure_hashes();
    }

    fn enabled_row_count(&self) -> usize {
        self.matrix.rows.iter().filter(|row| row.enabled).count()
    }

    fn cursor_rows(&self) -> CursorVisualColumnRows<'_> {
        CursorVisualColumnRows::new(&self.matrix.rows, self.matrix.ncols)
    }

    fn row(&self, row: usize) -> Option<&GlyphRow> {
        self.matrix.rows.get(row)
    }

    fn row_mut(&mut self, row: usize) -> Option<&mut GlyphRow> {
        self.matrix.rows.get_mut(row)
    }

    fn edit_row_with_matrix_cols<R>(
        &mut self,
        row: usize,
        f: impl FnOnce(&mut GlyphRow, usize) -> R,
    ) -> Option<R> {
        let ncols = self.matrix.ncols;
        let row = self.row_mut(row)?;
        Some(f(row, ncols))
    }

    fn edit_rows_with_matrix_cols(&mut self, mut f: impl FnMut(&mut GlyphRow, usize)) {
        let ncols = self.matrix.ncols;
        for row in &mut self.matrix.rows {
            f(row, ncols);
        }
    }

    fn write_row_metrics(&mut self, row: usize, metrics: OutputRowMetricsRequest) {
        let Some(row) = self.row_mut(row) else {
            return;
        };
        DisplayOutputBuilder::write_row_metrics(
            row,
            metrics.pixel_y,
            metrics.height_px,
            metrics.ascent_px,
        );
    }

    fn write_row_cursor(&mut self, row: usize, col: u16, style: CursorStyle) {
        let Some(row) = self.row_mut(row) else {
            return;
        };
        row.cursor_col = Some(col);
        row.cursor_type = Some(style);
    }

    fn replace_row(&mut self, row: usize, source: GlyphRow) {
        let Some(row) = self.row_mut(row) else {
            return;
        };
        *row = source;
    }

    fn begin_row(&mut self, begin: OutputRowBeginRequest) {
        let Some(row) = self.row_mut(begin.row) else {
            return;
        };
        row.role = begin.role;
        row.enabled = true;
        row.mode_line = begin.mode_line;
    }

    fn finalize_row(
        &mut self,
        window_id: u64,
        row: usize,
        pixel_bounds: Rect,
        phys_cursor: Option<&mut PhysCursor>,
    ) {
        let matrix_ncols = self.matrix.ncols;
        let Some(matrix_row) = self.row_mut(row) else {
            return;
        };
        GlyphRowFinalizationContext::new(window_id, row, pixel_bounds).finalize_row(
            matrix_row,
            matrix_ncols,
            phys_cursor,
        );
    }
}

impl OutputWindowGridEntry {
    fn new(
        window_id: u64,
        grid: OutputWindowRowGrid,
        pixel_bounds: Rect,
        text_pixel_bounds: Rect,
        selected: bool,
    ) -> Self {
        Self {
            window_id,
            grid,
            pixel_bounds,
            text_pixel_bounds,
            selected,
        }
    }

    fn edit_rows_with_matrix_cols(&mut self, f: impl FnMut(&mut GlyphRow, usize)) {
        self.grid.edit_rows_with_matrix_cols(f);
    }

    fn enabled_row_count(&self) -> usize {
        self.grid.enabled_row_count()
    }

    #[cfg(test)]
    fn window_id(&self) -> u64 {
        self.window_id
    }

    fn into_window_matrix_entry(mut self) -> WindowMatrixEntry {
        self.grid.ensure_hashes();
        WindowMatrixEntry {
            window_id: self.window_id,
            matrix: self.grid.into_matrix(),
            pixel_bounds: self.pixel_bounds,
            text_pixel_bounds: self.text_pixel_bounds,
            selected: self.selected,
        }
    }
}

pub(crate) struct DisplayOutputBuilder {
    windows: Vec<OutputWindowGridEntry>,
    current_row_grid: Option<OutputWindowRowGrid>,
    current_window_id: u64,
    current_pixel_bounds: Rect,
    current_text_pixel_bounds: Rect,
    /// Whether the window currently open in the builder is the
    /// selected window. Copied into the protocol window entry
    /// by `end_window`. Mirrors GNU's per-frame
    /// `w == XWINDOW (selected_window)` check in
    /// `src/xdisp.c::update_window`.
    current_selected: bool,
    current_row: usize,

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

impl DisplayOutputBuilder {
    fn write_row_metrics(row: &mut GlyphRow, pixel_y_rel: f32, height_px: f32, ascent_px: f32) {
        row.pixel_y = pixel_y_rel;
        row.height_px = height_px.max(0.0);
        row.ascent_px = ascent_px.max(0.0).min(row.height_px.max(0.0));
    }

    pub(crate) fn new() -> Self {
        Self {
            windows: Vec::new(),
            current_row_grid: None,
            current_window_id: 0,
            current_pixel_bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            current_text_pixel_bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            current_selected: false,
            current_row: 0,
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
        self.current_row_grid = None;
        self.current_window_id = 0;
        self.current_selected = false;
        self.current_row = 0;
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

    #[cfg(test)]
    pub(crate) fn begin_window_with_text_bounds(
        &mut self,
        window_id: u64,
        nrows: usize,
        ncols: usize,
        pixel_bounds: Rect,
        text_pixel_bounds: Rect,
        selected: bool,
    ) {
        self.begin_output_window(
            window_id,
            nrows,
            ncols,
            pixel_bounds,
            text_pixel_bounds,
            selected,
        );
    }

    #[cfg(test)]
    pub(crate) fn end_window(&mut self) {
        self.end_output_window();
    }

    fn install_window_lifecycle(&mut self, request: OutputWindowLifecycleRequest) {
        request.install(self);
    }

    pub(crate) fn begin_output_window(
        &mut self,
        window_id: u64,
        nrows: usize,
        ncols: usize,
        pixel_bounds: Rect,
        text_pixel_bounds: Rect,
        selected: bool,
    ) {
        self.install_window_lifecycle(OutputWindowLifecycleRequest::Begin(
            OutputWindowBeginRequest {
                window_id,
                nrows,
                ncols,
                pixel_bounds,
                text_pixel_bounds,
                selected,
            },
        ));
    }

    pub(crate) fn end_output_window(&mut self) {
        self.install_window_lifecycle(OutputWindowLifecycleRequest::End);
    }

    pub(crate) fn install_window_metadata(
        &mut self,
        request: impl Into<OutputWindowMetadataInstallRequest>,
    ) {
        request.into().install(self);
    }

    pub(crate) fn install_output_row_lifecycle(&mut self, request: OutputRowLifecycleRequest) {
        request.install(self);
    }

    #[cfg(test)]
    pub(crate) fn begin_output_row(&mut self, row: usize, role: GlyphRowRole, mode_line: bool) {
        self.install_output_row_lifecycle(OutputRowLifecycleRequest::begin(row, role, mode_line));
    }

    #[cfg(test)]
    pub(crate) fn install_complete_output_row(
        &mut self,
        matrix_row: usize,
        role: GlyphRowRole,
        mode_line: bool,
        glyph_row: GlyphRow,
    ) {
        self.install_output_row_lifecycle(OutputRowLifecycleRequest::complete(
            matrix_row, role, mode_line, glyph_row,
        ));
    }

    pub(crate) fn edit_current_output_row<R>(
        &mut self,
        f: impl FnOnce(&mut GlyphRow) -> R,
    ) -> Option<R> {
        self.with_current_row_mut(f)
    }

    #[cfg(test)]
    pub(crate) fn set_output_row_metrics(
        &mut self,
        row: usize,
        pixel_y: f32,
        height_px: f32,
        ascent_px: f32,
    ) {
        self.install_output_row_lifecycle(OutputRowLifecycleRequest::metrics(
            row, pixel_y, height_px, ascent_px,
        ));
    }

    #[cfg(test)]
    pub(crate) fn finalize_output_row_index(&mut self, row: usize) {
        self.install_output_row_lifecycle(OutputRowLifecycleRequest::finalize(row));
    }

    #[cfg(test)]
    pub(crate) fn set_output_row_cursor(&mut self, row: usize, col: u16, style: CursorStyle) {
        self.install_output_row_lifecycle(OutputRowLifecycleRequest::cursor(row, col, style));
    }

    #[cfg(test)]
    pub(crate) fn mark_current_output_row_truncated_left(&mut self) {
        self.install_output_row_lifecycle(OutputRowLifecycleRequest::current_decoration(
            OutputCurrentRowDecorationRequest::MarkTruncatedLeft,
        ));
    }

    pub(crate) fn edit_current_window_row_with_matrix_cols<R>(
        &mut self,
        row_idx: usize,
        f: impl FnOnce(&mut GlyphRow, usize) -> R,
    ) -> Option<R> {
        self.current_row_grid
            .as_mut()?
            .edit_row_with_matrix_cols(row_idx, f)
    }

    pub(crate) fn edit_last_window_rows_with_matrix_cols(
        &mut self,
        f: impl FnMut(&mut GlyphRow, usize),
    ) {
        if let Some(entry) = self.windows.last_mut() {
            entry.edit_rows_with_matrix_cols(f);
        }
    }

    #[cfg(test)]
    pub(crate) fn begin_row(&mut self, row: usize, role: GlyphRowRole) {
        self.begin_output_row(row, role, matches!(role, GlyphRowRole::ModeLine));
    }

    #[cfg(test)]
    pub(crate) fn end_row(&mut self) {
        let current_row = self.current_row;
        self.finalize_output_row_index(current_row);
    }

    /// Record stored geometry for the currently open row.
    #[cfg(test)]
    pub(crate) fn set_current_row_metrics(&mut self, pixel_y: f32, height_px: f32, ascent_px: f32) {
        let current_row = self.current_row;
        self.set_output_row_metrics(current_row, pixel_y, height_px, ascent_px);
    }

    fn with_current_row_mut<R>(&mut self, f: impl FnOnce(&mut GlyphRow) -> R) -> Option<R> {
        let grid = self.current_row_grid.as_mut()?;
        let row = grid.row_mut(self.current_row)?;
        Some(f(row))
    }

    #[cfg(test)]
    pub(crate) fn edit_current_row_for_test<R>(
        &mut self,
        f: impl FnOnce(&mut GlyphRow) -> R,
    ) -> Option<R> {
        self.with_current_row_mut(f)
    }

    fn decorate_current_row(&mut self, decoration: OutputCurrentRowDecorationRequest) {
        let _ = self.with_current_row_mut(|row| match decoration {
            OutputCurrentRowDecorationRequest::MarkTruncatedLeft => {
                row.truncated_left = true;
            }
        });
    }

    pub(crate) fn current_row_for_render(&self) -> Option<&GlyphRow> {
        self.current_row_grid.as_ref()?.row(self.current_row)
    }

    #[cfg(test)]
    pub(crate) fn current_row_for_test(&self) -> Option<&GlyphRow> {
        self.current_row_for_render()
    }

    fn write_row_metrics_at(&mut self, row: usize, metrics: OutputRowMetricsRequest) {
        if let Some(grid) = self.current_row_grid.as_mut() {
            grid.write_row_metrics(row, metrics);
        }
    }

    fn write_row_cursor(
        &mut self,
        row: usize,
        col: u16,
        style: neomacs_display_protocol::frame_glyphs::CursorStyle,
    ) {
        if let Some(grid) = self.current_row_grid.as_mut() {
            grid.write_row_cursor(row, col, style);
        }
    }

    fn replace_current_row(&mut self, source: GlyphRow) {
        let current_row = self.current_row;
        if let Some(grid) = self.current_row_grid.as_mut() {
            grid.replace_row(current_row, source);
        }
    }

    #[cfg(test)]
    fn install_display_row(&mut self, row_index: usize, source: &GlyphRow) {
        let pixel_bounds = self.current_window_pixel_bounds();
        let mut row = source.clone();
        row.pixel_y -= pixel_bounds.y;
        self.install_complete_output_row(row_index, row.role, row.mode_line, row);
    }

    fn begin_current_row(&mut self, begin: OutputRowBeginRequest) {
        self.current_row = begin.row;
        if let Some(grid) = self.current_row_grid.as_mut() {
            grid.begin_row(begin);
        }
    }

    fn install_complete_row(&mut self, request: OutputCompleteRowInstallRequest) {
        self.begin_current_row(OutputRowBeginRequest::new(
            request.row,
            request.role,
            request.mode_line,
        ));
        self.replace_current_row(request.glyph_row);
        self.finalize_output_row(request.row);
    }

    fn finalize_output_row(&mut self, row: usize) {
        if let Some(grid) = self.current_row_grid.as_mut() {
            grid.finalize_row(
                self.current_window_id,
                row,
                self.current_pixel_bounds,
                self.phys_cursor.as_mut(),
            );
        }
    }

    #[cfg(test)]
    pub(crate) fn set_cursor_at_row(
        &mut self,
        row: usize,
        col: u16,
        style: neomacs_display_protocol::frame_glyphs::CursorStyle,
    ) {
        self.set_output_row_cursor(row, col, style);
    }

    // -----------------------------------------------------------------------
    // Non-grid item installation
    // -----------------------------------------------------------------------

    pub(crate) fn install_output_frame_artifact(
        &mut self,
        request: OutputFrameArtifactInstallRequest,
    ) {
        request.install(self);
    }

    pub(crate) fn install_output_cursor(&mut self, request: OutputCursorInstallRequest) {
        self.cursors.push(request.cursor_item());
    }

    pub(crate) fn install_output_media(&mut self, request: OutputMediaInstallRequest) {
        request.install(self);
    }

    pub(crate) fn set_output_frame_identity(
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
        self.install_output_frame_state(OutputFrameStateInstallRequest::Identity(
            OutputFrameIdentityInstallRequest {
                frame_id,
                parent_id,
                parent_x,
                parent_y,
                z_order,
                undecorated,
                border_width,
                border_color,
                background_alpha,
                no_accept_focus,
            },
        ));
    }

    pub(crate) fn set_output_background_color(&mut self, color: Color) {
        self.install_output_frame_state(OutputFrameStateInstallRequest::BackgroundColor(color));
    }

    pub(crate) fn set_output_font_pixel_size(&mut self, size: f32) {
        self.install_output_frame_state(OutputFrameStateInstallRequest::FontPixelSize(size));
    }

    #[cfg(test)]
    pub(crate) fn install_output_face(&mut self, id: u32, face: Face) {
        self.install_output_frame_state(OutputFrameStateInstallRequest::face(id, face));
    }

    #[cfg(test)]
    pub(crate) fn install_output_resolved_display_row_face(
        &mut self,
        face_id: u32,
        face: &ResolvedFace,
        metrics: Option<FontMetrics>,
    ) {
        let render_face = resolved_display_row_face(face_id, face, metrics);
        self.install_output_frame_state(OutputFrameStateInstallRequest::face(
            render_face.face_id,
            render_face.render_face(),
        ));
    }

    pub(crate) fn add_output_background(&mut self, bounds: Rect, color: Color) {
        self.install_output_frame_artifact(OutputFrameArtifactInstallRequest::Background {
            bounds,
            color,
        });
    }

    pub(crate) fn add_output_border(
        &mut self,
        window_id: i64,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    ) {
        self.install_output_frame_artifact(OutputFrameArtifactInstallRequest::Border {
            window_id,
            x,
            y,
            width,
            height,
            color,
        });
    }

    pub(crate) fn add_output_scroll_bar(&mut self, item: ScrollBarItem) {
        self.install_output_frame_artifact(OutputFrameArtifactInstallRequest::ScrollBar(item));
    }

    pub(crate) fn add_output_window_info(&mut self, info: WindowInfo) {
        self.install_output_frame_artifact(OutputFrameArtifactInstallRequest::WindowInfo(info));
    }

    pub(crate) fn add_output_transition_hint(&mut self, hint: WindowTransitionHint) {
        self.install_output_frame_artifact(OutputFrameArtifactInstallRequest::TransitionHint(hint));
    }

    pub(crate) fn add_output_effect_hint(&mut self, hint: WindowEffectHint) {
        self.install_output_frame_artifact(OutputFrameArtifactInstallRequest::EffectHint(hint));
    }

    #[cfg(test)]
    pub(crate) fn add_output_cursor(
        &mut self,
        window_id: i64,
        slot_id: DisplaySlotId,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: CursorStyle,
        color: Color,
    ) {
        self.install_output_cursor(OutputCursorInstallRequest::new(
            window_id, slot_id, x, y, width, height, style, color,
        ));
    }

    pub(crate) fn current_window_id_i64(&self) -> i64 {
        self.current_window_id as i64
    }

    pub(crate) fn current_window_pixel_bounds(&self) -> Rect {
        self.current_pixel_bounds
    }

    pub(crate) fn current_window_text_pixel_bounds(&self) -> Rect {
        self.current_text_pixel_bounds
    }

    pub(crate) fn cursor_visual_column_context(&self) -> CursorVisualColumnResolutionContext<'_> {
        CursorVisualColumnResolutionContext::new(
            self.current_window_id,
            self.current_pixel_bounds,
            self.current_row_grid
                .as_ref()
                .map(OutputWindowRowGrid::cursor_rows),
        )
    }

    #[cfg(test)]
    pub(crate) fn set_phys_cursor(&mut self, cursor: PhysCursor) {
        let mut cursor = cursor;
        let placement = CursorVisualColumnResolutionRequest::from_cursor(&cursor)
            .resolve_phys_cursor_placement(self.cursor_visual_column_context());

        if let Some(placement) = placement {
            placement.apply_to(&mut cursor);
        }

        if let Some(placement) = placement
            && let Some(grid) = self.current_row_grid.as_mut()
        {
            grid.write_row_cursor(cursor.row, placement.col(), cursor.style);
        }

        // The selected window is represented solely by the phys cursor: the
        // window output no longer installs a redundant per-window CursorItem
        // for it (see the `!cursor.selected` guard around install_cursor), so
        // there is nothing to keep in sync here.
        self.install_output_frame_artifact(OutputFrameArtifactInstallRequest::phys_cursor(cursor));
    }

    #[cfg(test)]
    pub(crate) fn set_glyph_row_resolved_phys_cursor(&mut self, cursor: PhysCursor) {
        if let Some(grid) = self.current_row_grid.as_mut() {
            grid.write_row_cursor(cursor.row, cursor.col, cursor.style);
        }

        self.phys_cursor = Some(cursor);
    }

    pub(crate) fn install_output_frame_state(&mut self, request: OutputFrameStateInstallRequest) {
        request.install(self);
    }

    pub(crate) fn latest_window_enabled_rows(&self) -> Option<usize> {
        self.windows
            .last()
            .map(OutputWindowGridEntry::enabled_row_count)
    }

    #[cfg(test)]
    pub(crate) fn completed_window_count(&self) -> usize {
        self.windows.len()
    }

    #[cfg(test)]
    pub(crate) fn completed_window_id(&self, index: usize) -> Option<u64> {
        self.windows
            .get(index)
            .map(OutputWindowGridEntry::window_id)
    }

    pub(crate) fn window_infos(&self) -> &[WindowInfo] {
        &self.window_infos
    }

    pub(crate) fn transition_hints(&self) -> &[WindowTransitionHint] {
        &self.transition_hints
    }

    pub(crate) fn effect_hints(&self) -> &[WindowEffectHint] {
        &self.effect_hints
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

    pub(crate) fn finish(
        self,
        frame_cols: usize,
        frame_rows: usize,
        char_width: f32,
        char_height: f32,
    ) -> FrameDisplayState {
        let mut state = FrameDisplayState::new(frame_cols, frame_rows, char_width, char_height);
        state.window_matrices = self
            .windows
            .into_iter()
            .map(OutputWindowGridEntry::into_window_matrix_entry)
            .collect();
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
}

#[cfg(test)]
#[path = "display_output_builder_test.rs"]
mod tests;
