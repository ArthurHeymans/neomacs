//! DisplayOutputBuilder — records authoritative window matrices during layout.
//!
//! The builder observes layout emissions and writes them into the per-window
//! `GlyphMatrix` grids published through `FrameDisplayState`. Renderers then
//! materialize that immutable snapshot into runtime glyph buffers on the
//! consumer side; layout no longer treats `FrameGlyphBuffer` as the primary
//! output contract.

use crate::display_cursor::CursorVisualColumnResolutionContext;
#[cfg(test)]
use crate::display_cursor::CursorVisualColumnResolutionRequest;
use crate::display_output_install_request::{
    OutputCursorInstallRequest, OutputFrameArtifactInstallRequest,
    OutputFrameIdentityInstallRequest, OutputFrameStateInstallRequest, OutputMediaInstallKind,
    OutputMediaInstallRequest, OutputWindowMetadataInstallRequest,
};
#[cfg(test)]
use crate::display_output_row_request::OutputCurrentRowDecorationRequest;
use crate::display_output_row_request::{
    DisplayCurrentRowMutation, DisplayWindowRowMutation, DisplayWindowRowsMutation,
    OutputRowLifecycleRequest,
};
use crate::display_output_window_request::OutputWindowLifecycleRequest;
use crate::display_output_window_state::OutputWindowBuildState;
#[cfg(test)]
use crate::display_row::resolved_display_row_face;
#[cfg(test)]
use crate::font_metrics::FontMetrics;
#[cfg(test)]
use crate::neovm_bridge::ResolvedFace;
use neomacs_display_protocol::effect_config::EffectsConfig;
use neomacs_display_protocol::face::Face;
#[cfg(test)]
use neomacs_display_protocol::frame_glyphs::DisplaySlotId;
#[cfg(test)]
use neomacs_display_protocol::frame_glyphs::{CursorStyle, GlyphRowRole};
use neomacs_display_protocol::frame_glyphs::{
    PhysCursor, StipplePattern, WindowEffectHint, WindowInfo, WindowTransitionHint,
};
use neomacs_display_protocol::glyph_matrix::*;
use neomacs_display_protocol::types::{Color, Rect};
use std::collections::HashMap;

pub(crate) const FRAME_CHROME_WINDOW_ID: i64 = 0;

pub(crate) struct DisplayOutputBuilder {
    window_state: OutputWindowBuildState,

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
    pub(crate) fn new() -> Self {
        Self {
            window_state: OutputWindowBuildState::new(),
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
        self.window_state.reset();
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

    pub(crate) fn install_output_window_lifecycle(
        &mut self,
        request: OutputWindowLifecycleRequest,
    ) {
        self.window_state.install_window_lifecycle(request);
    }

    #[cfg(test)]
    pub(crate) fn begin_output_window(
        &mut self,
        window_id: u64,
        nrows: usize,
        ncols: usize,
        pixel_bounds: Rect,
        text_pixel_bounds: Rect,
        selected: bool,
    ) {
        self.install_output_window_lifecycle(OutputWindowLifecycleRequest::begin(
            window_id,
            nrows,
            ncols,
            pixel_bounds,
            text_pixel_bounds,
            selected,
        ));
    }

    #[cfg(test)]
    pub(crate) fn end_output_window(&mut self) {
        self.install_output_window_lifecycle(OutputWindowLifecycleRequest::end());
    }

    pub(crate) fn install_window_metadata(
        &mut self,
        request: impl Into<OutputWindowMetadataInstallRequest>,
    ) {
        self.install_output_window_metadata(request.into());
    }

    fn install_output_window_metadata(&mut self, request: OutputWindowMetadataInstallRequest) {
        match request {
            OutputWindowMetadataInstallRequest::TextDisplayRange(range) => {
                if let Some(info) = self.window_infos.last_mut()
                    && info.window_id == range.window_id
                {
                    info.window_start = range.window_start;
                    info.window_end = range.window_end;
                }
            }
            OutputWindowMetadataInstallRequest::RestoreRetryCheckpoint(checkpoint) => {
                self.transition_hints
                    .truncate(checkpoint.transition_hints_len);
                self.effect_hints.truncate(checkpoint.effect_hints_len);
            }
        }
    }

    pub(crate) fn install_output_row_lifecycle(&mut self, request: OutputRowLifecycleRequest) {
        self.window_state
            .install_row_lifecycle(request, self.phys_cursor.as_mut());
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

    pub(crate) fn apply_current_output_row_mutation<M>(&mut self, mutation: M) -> Option<M::Output>
    where
        M: DisplayCurrentRowMutation,
    {
        self.window_state
            .edit_current_row(|row| mutation.apply(row))
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

    pub(crate) fn apply_current_window_row_mutation<M>(
        &mut self,
        row_idx: usize,
        mutation: M,
    ) -> Option<M::Output>
    where
        M: DisplayWindowRowMutation,
    {
        self.window_state
            .edit_current_window_row_with_matrix_cols(row_idx, |row, matrix_cols| {
                mutation.apply(row, matrix_cols)
            })
    }

    pub(crate) fn apply_last_window_rows_mutation<M>(&mut self, mut mutation: M)
    where
        M: DisplayWindowRowsMutation,
    {
        self.window_state
            .edit_last_window_rows_with_matrix_cols(|row, matrix_cols| {
                mutation.apply(row, matrix_cols);
            });
    }

    #[cfg(test)]
    pub(crate) fn begin_row(&mut self, row: usize, role: GlyphRowRole) {
        self.begin_output_row(row, role, matches!(role, GlyphRowRole::ModeLine));
    }

    #[cfg(test)]
    pub(crate) fn end_row(&mut self) {
        let current_row = self.window_state.current_row_index();
        self.finalize_output_row_index(current_row);
    }

    /// Record stored geometry for the currently open row.
    #[cfg(test)]
    pub(crate) fn set_current_row_metrics(&mut self, pixel_y: f32, height_px: f32, ascent_px: f32) {
        let current_row = self.window_state.current_row_index();
        self.set_output_row_metrics(current_row, pixel_y, height_px, ascent_px);
    }

    #[cfg(test)]
    pub(crate) fn edit_current_row_for_test<R>(
        &mut self,
        f: impl FnOnce(&mut GlyphRow) -> R,
    ) -> Option<R> {
        self.window_state.edit_current_row(f)
    }

    pub(crate) fn current_row_for_render(&self) -> Option<&GlyphRow> {
        self.window_state.current_row_for_render()
    }

    #[cfg(test)]
    pub(crate) fn current_row_for_test(&self) -> Option<&GlyphRow> {
        self.current_row_for_render()
    }

    #[cfg(test)]
    fn install_display_row(&mut self, row_index: usize, source: &GlyphRow) {
        self.install_output_row_lifecycle(OutputRowLifecycleRequest::complete_window_absolute_row(
            row_index,
            source,
            self.current_window_pixel_bounds(),
        ));
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
        match request {
            OutputFrameArtifactInstallRequest::Background { bounds, color } => {
                self.backgrounds.push(BackgroundItem { bounds, color });
            }
            OutputFrameArtifactInstallRequest::Border {
                window_id,
                x,
                y,
                width,
                height,
                color,
            } => {
                self.borders.push(BorderItem {
                    window_id,
                    x,
                    y,
                    width,
                    height,
                    color,
                });
            }
            OutputFrameArtifactInstallRequest::ScrollBar(item) => self.scroll_bars.push(item),
            OutputFrameArtifactInstallRequest::WindowInfo(info) => self.window_infos.push(info),
            OutputFrameArtifactInstallRequest::TransitionHint(hint) => {
                self.transition_hints.push(hint);
            }
            OutputFrameArtifactInstallRequest::EffectHint(hint) => self.effect_hints.push(hint),
            OutputFrameArtifactInstallRequest::PhysCursor(cursor) => {
                self.phys_cursor = Some(cursor)
            }
        }
    }

    pub(crate) fn install_output_cursor(&mut self, request: OutputCursorInstallRequest) {
        self.cursors.push(request.cursor_item());
    }

    pub(crate) fn install_output_media(&mut self, request: OutputMediaInstallRequest) {
        let target = request.target;
        match request.kind {
            OutputMediaInstallKind::Image { image_id } => self.images.push(ImageItem {
                window_id: target.window_id,
                row_role: target.role,
                clip_rect: target.clip,
                slot_id: Some(target.slot_id),
                image_id,
                x: request.x,
                y: request.y,
                width: request.width,
                height: request.height,
            }),
            OutputMediaInstallKind::Video {
                video_id,
                loop_count,
                autoplay,
            } => self.videos.push(VideoItem {
                window_id: target.window_id,
                row_role: target.role,
                clip_rect: target.clip,
                slot_id: Some(target.slot_id),
                video_id,
                x: request.x,
                y: request.y,
                width: request.width,
                height: request.height,
                loop_count,
                autoplay,
            }),
            OutputMediaInstallKind::Xwidget { xwidget_id } => self.xwidgets.push(XwidgetItem {
                window_id: target.window_id,
                row_role: target.role,
                clip_rect: target.clip,
                slot_id: Some(target.slot_id),
                xwidget_id,
                x: request.x,
                y: request.y,
                width: request.width,
                height: request.height,
            }),
        }
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
        self.window_state.current_window_id_i64()
    }

    pub(crate) fn current_window_pixel_bounds(&self) -> Rect {
        self.window_state.current_window_pixel_bounds()
    }

    pub(crate) fn current_window_text_pixel_bounds(&self) -> Rect {
        self.window_state.current_window_text_pixel_bounds()
    }

    pub(crate) fn cursor_visual_column_context(&self) -> CursorVisualColumnResolutionContext<'_> {
        self.window_state.cursor_visual_column_context()
    }

    #[cfg(test)]
    pub(crate) fn set_phys_cursor(&mut self, cursor: PhysCursor) {
        let mut cursor = cursor;
        let placement = CursorVisualColumnResolutionRequest::from_cursor(&cursor)
            .resolve_phys_cursor_placement(self.cursor_visual_column_context());

        if let Some(placement) = placement {
            placement.apply_to(&mut cursor);
        }

        if let Some(placement) = placement {
            self.window_state
                .write_row_cursor(cursor.row, placement.col(), cursor.style);
        }

        // The selected window is represented solely by the phys cursor: the
        // window output no longer installs a redundant per-window CursorItem
        // for it (see the `!cursor.selected` guard around install_cursor), so
        // there is nothing to keep in sync here.
        self.install_output_frame_artifact(OutputFrameArtifactInstallRequest::phys_cursor(cursor));
    }

    #[cfg(test)]
    pub(crate) fn set_glyph_row_resolved_phys_cursor(&mut self, cursor: PhysCursor) {
        self.window_state
            .write_row_cursor(cursor.row, cursor.col, cursor.style);

        self.phys_cursor = Some(cursor);
    }

    pub(crate) fn install_output_frame_state(&mut self, request: OutputFrameStateInstallRequest) {
        match request {
            OutputFrameStateInstallRequest::Identity(identity) => {
                self.frame_id = identity.frame_id;
                self.parent_id = identity.parent_id;
                self.parent_x = identity.parent_x;
                self.parent_y = identity.parent_y;
                self.z_order = identity.z_order;
                self.undecorated = identity.undecorated;
                self.border_width = identity.border_width;
                self.border_color = identity.border_color;
                self.background_alpha = identity.background_alpha;
                self.no_accept_focus = identity.no_accept_focus;
            }
            OutputFrameStateInstallRequest::BackgroundColor(color) => self.background_color = color,
            OutputFrameStateInstallRequest::FontPixelSize(size) => self.font_pixel_size = size,
            OutputFrameStateInstallRequest::Face { id, face } => {
                self.faces.insert(id, face);
            }
            OutputFrameStateInstallRequest::CursorEffects { window_id, effects } => {
                self.cursor_effects_by_window.insert(window_id, effects);
            }
        }
    }

    pub(crate) fn latest_window_enabled_rows(&self) -> Option<usize> {
        self.window_state.latest_window_enabled_rows()
    }

    #[cfg(test)]
    pub(crate) fn completed_window_count(&self) -> usize {
        self.window_state.completed_window_count()
    }

    #[cfg(test)]
    pub(crate) fn completed_window_id(&self, index: usize) -> Option<u64> {
        self.window_state.completed_window_id(index)
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
        state.window_matrices = self.window_state.into_window_matrix_entries();
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
