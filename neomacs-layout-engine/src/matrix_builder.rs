//! GlyphMatrixBuilder — records authoritative window matrices during layout.
//!
//! The builder observes layout emissions and writes them into the per-window
//! `GlyphMatrix` grids published through `FrameDisplayState`. Renderers then
//! materialize that immutable snapshot into runtime glyph buffers on the
//! consumer side; layout no longer treats `FrameGlyphBuffer` as the primary
//! output contract.

use crate::display_cursor::CursorVisualColumnResolutionContext;
#[cfg(test)]
use crate::display_cursor::CursorVisualColumnResolutionRequest;
use crate::display_row_finalizer::GlyphRowFinalizationContext;
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
enum MatrixMediaInstallKind {
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
struct MatrixMediaInstallRequest {
    target: ResolvedMatrixMediaInstallTarget,
    kind: MatrixMediaInstallKind,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResolvedMatrixMediaInstallTarget {
    pub(crate) window_id: i64,
    pub(crate) role: GlyphRowRole,
    pub(crate) clip: Option<Rect>,
    pub(crate) slot_id: DisplaySlotId,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MatrixCurrentWindowMediaInstallContext {
    pub(crate) window_id: i64,
    pub(crate) text_pixel_bounds: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MatrixCurrentWindowRowInstallContext {
    pub(crate) pixel_bounds: Rect,
}

impl MatrixMediaInstallRequest {
    fn new(
        target: ResolvedMatrixMediaInstallTarget,
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
        let target = self.target;
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

#[derive(Clone, Copy, Debug, PartialEq)]
struct MatrixCursorInstallRequest {
    window_id: i64,
    slot_id: DisplaySlotId,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    style: CursorStyle,
    color: Color,
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
enum MatrixFrameArtifactInstallRequest {
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
enum MatrixFrameStateInstallRequest {
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MatrixWindowBeginRequest {
    pub(crate) window_id: u64,
    pub(crate) nrows: usize,
    pub(crate) ncols: usize,
    pub(crate) pixel_bounds: Rect,
    pub(crate) text_pixel_bounds: Rect,
    pub(crate) selected: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum MatrixWindowLifecycleRequest {
    Begin(MatrixWindowBeginRequest),
    End,
}

impl MatrixWindowLifecycleRequest {
    fn install(self, builder: &mut GlyphMatrixBuilder) {
        match self {
            Self::Begin(begin) => {
                builder.current_matrix = Some(GlyphMatrix::new(begin.nrows, begin.ncols));
                builder.current_window_id = begin.window_id;
                builder.current_pixel_bounds = begin.pixel_bounds;
                builder.current_text_pixel_bounds = begin.text_pixel_bounds;
                builder.current_selected = begin.selected;
                builder.current_row = 0;
            }
            Self::End => {
                if let Some(matrix) = builder.current_matrix.take() {
                    builder.windows.push(WindowMatrixEntry {
                        window_id: builder.current_window_id,
                        matrix,
                        pixel_bounds: builder.current_pixel_bounds,
                        text_pixel_bounds: builder.current_text_pixel_bounds,
                        selected: builder.current_selected,
                    });
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MatrixRowBeginRequest {
    pub(crate) row: usize,
    pub(crate) role: GlyphRowRole,
    pub(crate) mode_line: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MatrixRowMetricsRequest {
    /// Stored row Y, relative to the window matrix origin.
    pub(crate) pixel_y: f32,
    pub(crate) height_px: f32,
    pub(crate) ascent_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatrixCurrentRowDecorationRequest {
    MarkTruncatedLeft,
}

#[derive(Clone, Debug)]
enum MatrixRowLifecycleRequest {
    Begin(MatrixRowBeginRequest),
    ReplaceCurrent {
        row: GlyphRow,
    },
    Metrics {
        row: usize,
        metrics: MatrixRowMetricsRequest,
    },
    Finalize {
        row: usize,
    },
    Cursor {
        row: usize,
        col: u16,
        style: CursorStyle,
    },
    CurrentDecoration(MatrixCurrentRowDecorationRequest),
}

impl MatrixRowLifecycleRequest {
    fn install(self, builder: &mut GlyphMatrixBuilder) {
        match self {
            Self::Begin(begin) => builder.begin_current_row(begin),
            Self::ReplaceCurrent { row } => builder.replace_current_row(row),
            Self::Metrics { row, metrics } => builder.write_row_metrics_at(row, metrics),
            Self::Finalize { row } => builder.finalize_matrix_row(row),
            Self::Cursor { row, col, style } => builder.write_row_cursor(row, col, style),
            Self::CurrentDecoration(decoration) => builder.decorate_current_row(decoration),
        }
    }
}

pub(crate) trait MatrixRowDecorator {
    fn decorate_row(&mut self, row: &mut GlyphRow, matrix_cols: usize);
}

pub(crate) struct MatrixCurrentWindowRowDecorationRequest<D> {
    pub(crate) row_idx: usize,
    pub(crate) decorator: D,
}

impl<D> MatrixCurrentWindowRowDecorationRequest<D> {
    pub(crate) fn new(row_idx: usize, decorator: D) -> Self {
        Self { row_idx, decorator }
    }
}

pub(crate) struct MatrixLastWindowRowsDecorationRequest<D> {
    pub(crate) decorator: D,
}

impl<D> MatrixLastWindowRowsDecorationRequest<D> {
    pub(crate) fn new(decorator: D) -> Self {
        Self { decorator }
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

pub(crate) struct MatrixRowInstaller<'a> {
    builder: &'a mut GlyphMatrixBuilder,
}

impl MatrixRowInstaller<'_> {
    fn install(&mut self, request: MatrixRowLifecycleRequest) {
        self.builder.install_row_lifecycle(request);
    }

    pub(crate) fn begin(&mut self, begin: MatrixRowBeginRequest) {
        self.install(MatrixRowLifecycleRequest::Begin(begin));
    }

    pub(crate) fn install_complete_row(&mut self, begin: MatrixRowBeginRequest, row: GlyphRow) {
        self.begin(begin);
        self.replace_current_row(row);
        self.finalize(self.builder.current_row);
    }

    pub(crate) fn replace_current_row(&mut self, row: GlyphRow) {
        self.install(MatrixRowLifecycleRequest::ReplaceCurrent { row });
    }

    pub(crate) fn edit_current_row<R>(&mut self, f: impl FnOnce(&mut GlyphRow) -> R) -> Option<R> {
        self.builder.with_current_row_mut(f)
    }

    pub(crate) fn set_metrics(&mut self, row: usize, metrics: MatrixRowMetricsRequest) {
        self.install(MatrixRowLifecycleRequest::Metrics { row, metrics });
    }

    pub(crate) fn finalize(&mut self, row: usize) {
        self.install(MatrixRowLifecycleRequest::Finalize { row });
    }

    pub(crate) fn set_cursor(&mut self, row: usize, col: u16, style: CursorStyle) {
        self.install(MatrixRowLifecycleRequest::Cursor { row, col, style });
    }

    pub(crate) fn mark_current_truncated_left(&mut self) {
        self.install(MatrixRowLifecycleRequest::CurrentDecoration(
            MatrixCurrentRowDecorationRequest::MarkTruncatedLeft,
        ));
    }
}

pub(crate) struct MatrixWindowInstaller<'a> {
    builder: &'a mut GlyphMatrixBuilder,
}

impl MatrixWindowInstaller<'_> {
    fn install(&mut self, request: MatrixWindowLifecycleRequest) {
        self.builder.install_window_lifecycle(request);
    }

    pub(crate) fn begin(&mut self, begin: MatrixWindowBeginRequest) {
        self.install(MatrixWindowLifecycleRequest::Begin(begin));
    }

    pub(crate) fn end(&mut self) {
        self.install(MatrixWindowLifecycleRequest::End);
    }
}

pub(crate) struct MatrixArtifactInstaller<'a> {
    builder: &'a mut GlyphMatrixBuilder,
}

impl MatrixArtifactInstaller<'_> {
    fn install_frame_state(&mut self, request: MatrixFrameStateInstallRequest) {
        self.builder.install_frame_state(request);
    }

    pub(crate) fn set_frame_identity(&mut self, identity: MatrixFrameIdentityInstallRequest) {
        self.install_frame_state(MatrixFrameStateInstallRequest::Identity(identity));
    }

    pub(crate) fn set_background_color(&mut self, color: Color) {
        self.install_frame_state(MatrixFrameStateInstallRequest::BackgroundColor(color));
    }

    pub(crate) fn set_font_pixel_size(&mut self, size: f32) {
        self.install_frame_state(MatrixFrameStateInstallRequest::FontPixelSize(size));
    }

    pub(crate) fn set_faces(&mut self, faces: HashMap<u32, Face>) {
        self.install_frame_state(MatrixFrameStateInstallRequest::Faces(faces));
    }

    pub(crate) fn set_face(&mut self, id: u32, face: Face) {
        self.install_frame_state(MatrixFrameStateInstallRequest::Face { id, face });
    }

    pub(crate) fn set_cursor_effects(&mut self, window_id: i64, effects: EffectsConfig) {
        self.install_frame_state(MatrixFrameStateInstallRequest::CursorEffects {
            window_id,
            effects,
        });
    }

    fn install_frame_artifact(&mut self, request: MatrixFrameArtifactInstallRequest) {
        self.builder.install_frame_artifact(request);
    }

    pub(crate) fn add_background(&mut self, bounds: Rect, color: Color) {
        self.install_frame_artifact(MatrixFrameArtifactInstallRequest::Background {
            bounds,
            color,
        });
    }

    pub(crate) fn add_border(
        &mut self,
        window_id: i64,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    ) {
        self.install_frame_artifact(MatrixFrameArtifactInstallRequest::Border {
            window_id,
            x,
            y,
            width,
            height,
            color,
        });
    }

    pub(crate) fn add_scroll_bar(&mut self, item: ScrollBarItem) {
        self.install_frame_artifact(MatrixFrameArtifactInstallRequest::ScrollBar(item));
    }

    pub(crate) fn add_window_info(&mut self, info: WindowInfo) {
        self.install_frame_artifact(MatrixFrameArtifactInstallRequest::WindowInfo(info));
    }

    pub(crate) fn add_transition_hint(&mut self, hint: WindowTransitionHint) {
        self.install_frame_artifact(MatrixFrameArtifactInstallRequest::TransitionHint(hint));
    }

    pub(crate) fn add_effect_hint(&mut self, hint: WindowEffectHint) {
        self.install_frame_artifact(MatrixFrameArtifactInstallRequest::EffectHint(hint));
    }

    fn install_cursor(&mut self, request: MatrixCursorInstallRequest) {
        self.builder.install_cursor(request);
    }

    pub(crate) fn add_cursor(
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
        self.install_cursor(MatrixCursorInstallRequest {
            window_id,
            slot_id,
            x,
            y,
            width,
            height,
            style,
            color,
        });
    }

    fn install_media(&mut self, request: MatrixMediaInstallRequest) {
        self.builder.install_media(request);
    }

    fn add_media(
        &mut self,
        target: ResolvedMatrixMediaInstallTarget,
        kind: MatrixMediaInstallKind,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        self.install_media(MatrixMediaInstallRequest::new(
            target, kind, x, y, width, height,
        ));
    }

    pub(crate) fn add_image_media(
        &mut self,
        target: ResolvedMatrixMediaInstallTarget,
        image_id: u32,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        self.add_media(
            target,
            MatrixMediaInstallKind::Image { image_id },
            x,
            y,
            width,
            height,
        );
    }

    pub(crate) fn add_video_media(
        &mut self,
        target: ResolvedMatrixMediaInstallTarget,
        video_id: u32,
        loop_count: i32,
        autoplay: bool,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        self.add_media(
            target,
            MatrixMediaInstallKind::Video {
                video_id,
                loop_count,
                autoplay,
            },
            x,
            y,
            width,
            height,
        );
    }

    pub(crate) fn add_xwidget_media(
        &mut self,
        target: ResolvedMatrixMediaInstallTarget,
        xwidget_id: u32,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        self.add_media(
            target,
            MatrixMediaInstallKind::Xwidget { xwidget_id },
            x,
            y,
            width,
            height,
        );
    }

    pub(crate) fn store_phys_cursor(&mut self, cursor: PhysCursor) {
        self.builder.store_phys_cursor(cursor);
    }
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
        self.window_installer().begin(MatrixWindowBeginRequest {
            window_id,
            nrows,
            ncols,
            pixel_bounds,
            text_pixel_bounds,
            selected,
        });
    }

    #[cfg(test)]
    pub(crate) fn end_window(&mut self) {
        self.window_installer().end();
    }

    fn install_window_lifecycle(&mut self, request: MatrixWindowLifecycleRequest) {
        request.install(self);
    }

    fn install_row_lifecycle(&mut self, request: MatrixRowLifecycleRequest) {
        request.install(self);
    }

    pub(crate) fn row_installer(&mut self) -> MatrixRowInstaller<'_> {
        MatrixRowInstaller { builder: self }
    }

    pub(crate) fn window_installer(&mut self) -> MatrixWindowInstaller<'_> {
        MatrixWindowInstaller { builder: self }
    }

    pub(crate) fn artifact_installer(&mut self) -> MatrixArtifactInstaller<'_> {
        MatrixArtifactInstaller { builder: self }
    }

    #[cfg(test)]
    pub(crate) fn begin_row(&mut self, row: usize, role: GlyphRowRole) {
        self.row_installer().begin(MatrixRowBeginRequest {
            row,
            role,
            mode_line: matches!(role, GlyphRowRole::ModeLine),
        });
    }

    #[cfg(test)]
    pub(crate) fn end_row(&mut self) {
        let current_row = self.current_row;
        self.row_installer().finalize(current_row);
    }

    /// Record stored geometry for the currently open row.
    #[cfg(test)]
    pub(crate) fn set_current_row_metrics(&mut self, pixel_y: f32, height_px: f32, ascent_px: f32) {
        let current_row = self.current_row;
        self.row_installer().set_metrics(
            current_row,
            MatrixRowMetricsRequest {
                pixel_y,
                height_px,
                ascent_px,
            },
        );
    }

    fn with_current_row_mut<R>(&mut self, f: impl FnOnce(&mut GlyphRow) -> R) -> Option<R> {
        let matrix = self.current_matrix.as_mut()?;
        let row = matrix.rows.get_mut(self.current_row)?;
        Some(f(row))
    }

    #[cfg(test)]
    pub(crate) fn edit_current_row_for_test<R>(
        &mut self,
        f: impl FnOnce(&mut GlyphRow) -> R,
    ) -> Option<R> {
        self.with_current_row_mut(f)
    }

    fn decorate_current_row(&mut self, decoration: MatrixCurrentRowDecorationRequest) {
        let _ = self.with_current_row_mut(|row| match decoration {
            MatrixCurrentRowDecorationRequest::MarkTruncatedLeft => {
                row.truncated_left = true;
            }
        });
    }

    pub(crate) fn current_row_for_render(&self) -> Option<&GlyphRow> {
        let matrix = self.current_matrix.as_ref()?;
        matrix.rows.get(self.current_row)
    }

    #[cfg(test)]
    pub(crate) fn current_row_for_test(&self) -> Option<&GlyphRow> {
        self.current_row_for_render()
    }

    fn write_row_metrics_at(&mut self, row: usize, metrics: MatrixRowMetricsRequest) {
        if let Some(ref mut matrix) = self.current_matrix
            && row < matrix.rows.len()
        {
            Self::write_row_metrics(
                &mut matrix.rows[row],
                metrics.pixel_y,
                metrics.height_px,
                metrics.ascent_px,
            );
        }
    }

    fn write_row_cursor(
        &mut self,
        row: usize,
        col: u16,
        style: neomacs_display_protocol::frame_glyphs::CursorStyle,
    ) {
        if let Some(ref mut matrix) = self.current_matrix
            && row < matrix.rows.len()
        {
            matrix.rows[row].cursor_col = Some(col);
            matrix.rows[row].cursor_type = Some(style);
        }
    }

    fn replace_current_row(&mut self, source: GlyphRow) {
        let current_row = self.current_row;
        if let Some(ref mut matrix) = self.current_matrix
            && current_row < matrix.rows.len()
        {
            matrix.rows[current_row] = source;
        }
    }

    #[cfg(test)]
    fn install_display_row(&mut self, row_index: usize, source: &GlyphRow) {
        let context = self.current_window_row_install_context();
        let mut row = source.clone();
        row.pixel_y -= context.pixel_bounds.y;
        self.row_installer().install_complete_row(
            MatrixRowBeginRequest {
                row: row_index,
                role: row.role,
                mode_line: row.mode_line,
            },
            row,
        );
    }

    fn begin_current_row(&mut self, begin: MatrixRowBeginRequest) {
        self.current_row = begin.row;
        if let Some(ref mut matrix) = self.current_matrix
            && begin.row < matrix.rows.len()
        {
            matrix.rows[begin.row].role = begin.role;
            matrix.rows[begin.row].enabled = true;
            matrix.rows[begin.row].mode_line = begin.mode_line;
        }
    }

    fn finalize_matrix_row(&mut self, row: usize) {
        if let Some(ref mut matrix) = self.current_matrix {
            let matrix_ncols = matrix.ncols;
            let Some(matrix_row) = matrix.rows.get_mut(row) else {
                return;
            };
            GlyphRowFinalizationContext::new(
                self.current_window_id,
                row,
                self.current_pixel_bounds,
            )
            .finalize_row(matrix_row, matrix_ncols, self.phys_cursor.as_mut());
        }
    }

    #[cfg(test)]
    pub(crate) fn set_cursor_at_row(
        &mut self,
        row: usize,
        col: u16,
        style: neomacs_display_protocol::frame_glyphs::CursorStyle,
    ) {
        self.row_installer().set_cursor(row, col, style);
    }

    // -----------------------------------------------------------------------
    // Non-grid item installation
    // -----------------------------------------------------------------------

    fn install_frame_artifact(&mut self, request: MatrixFrameArtifactInstallRequest) {
        request.install(self);
    }

    fn install_cursor(&mut self, request: MatrixCursorInstallRequest) {
        self.cursors.push(request.cursor_item());
    }

    fn install_media(&mut self, request: MatrixMediaInstallRequest) {
        request.install(self);
    }

    pub(crate) fn current_window_media_install_context(
        &self,
    ) -> MatrixCurrentWindowMediaInstallContext {
        MatrixCurrentWindowMediaInstallContext {
            window_id: self.current_window_id as i64,
            text_pixel_bounds: self.current_text_pixel_bounds,
        }
    }

    pub(crate) fn current_window_row_install_context(
        &self,
    ) -> MatrixCurrentWindowRowInstallContext {
        MatrixCurrentWindowRowInstallContext {
            pixel_bounds: self.current_pixel_bounds,
        }
    }

    pub(crate) fn cursor_visual_column_context(&self) -> CursorVisualColumnResolutionContext<'_> {
        CursorVisualColumnResolutionContext::new(
            self.current_window_id,
            self.current_pixel_bounds,
            self.current_matrix.as_ref(),
        )
    }

    fn store_phys_cursor(&mut self, cursor: PhysCursor) {
        self.phys_cursor = Some(cursor);
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
            && let Some(ref mut matrix) = self.current_matrix
            && cursor.row < matrix.rows.len()
        {
            matrix.rows[cursor.row].cursor_col = Some(placement.col());
            matrix.rows[cursor.row].cursor_type = Some(cursor.style);
        }

        // The selected window is represented solely by the phys cursor: the
        // window output no longer installs a redundant per-window CursorItem
        // for it (see the `!cursor.selected` guard around install_cursor), so
        // there is nothing to keep in sync here.
        self.store_phys_cursor(cursor);
    }

    #[cfg(test)]
    pub(crate) fn set_glyph_row_resolved_phys_cursor(&mut self, cursor: PhysCursor) {
        if let Some(ref mut matrix) = self.current_matrix
            && cursor.row < matrix.rows.len()
        {
            matrix.rows[cursor.row].cursor_col = Some(cursor.col);
            matrix.rows[cursor.row].cursor_type = Some(cursor.style);
        }

        self.phys_cursor = Some(cursor);
    }

    fn install_frame_state(&mut self, request: MatrixFrameStateInstallRequest) {
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

    pub(crate) fn decorate_current_window_row<D>(
        &mut self,
        mut request: MatrixCurrentWindowRowDecorationRequest<D>,
    ) where
        D: MatrixRowDecorator,
    {
        let Some(matrix) = self.current_matrix.as_mut() else {
            return;
        };
        let Some(row) = matrix.rows.get_mut(request.row_idx) else {
            return;
        };
        request.decorator.decorate_row(row, matrix.ncols);
    }

    pub(crate) fn decorate_last_window_rows<D>(
        &mut self,
        mut request: MatrixLastWindowRowsDecorationRequest<D>,
    ) where
        D: MatrixRowDecorator,
    {
        let Some(entry) = self.windows.last_mut() else {
            return;
        };
        let matrix_cols = entry.matrix.ncols;
        for row in &mut entry.matrix.rows {
            request.decorator.decorate_row(row, matrix_cols);
        }
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
}

#[cfg(test)]
#[path = "matrix_builder_test.rs"]
mod tests;
