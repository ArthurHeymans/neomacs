use crate::display_buffer_text_append::BufferTextWindowTerminalRightBorderRequest;
use crate::display_frame_output::FrameOutputIdentity;
use crate::display_output_builder::DisplayOutputBuilder;
use crate::display_status_line::ChromeRowRenderServices;
use crate::font_metrics::FontMetrics;
use crate::neovm_bridge::ResolvedFace;
use crate::window_output::TextWindowArtifactOutputSurface;
use neomacs_display_protocol::face::Face;
use neomacs_display_protocol::frame_glyphs::{
    PhysCursor, WindowEffectHint, WindowInfo, WindowTransitionHint,
};
use neomacs_display_protocol::glyph_matrix::{CursorItem, ScrollBarItem, WindowMatrixEntry};
use neomacs_display_protocol::types::{Color, Rect};

pub(crate) struct FrameOutputInstallSurface<'output> {
    output_builder: &'output mut DisplayOutputBuilder,
}

pub(crate) struct FrameOutputReadSurface<'output> {
    output_builder: &'output DisplayOutputBuilder,
}

impl<'output> FrameOutputInstallSurface<'output> {
    pub(crate) fn from_output_builder(output_builder: &'output mut DisplayOutputBuilder) -> Self {
        Self { output_builder }
    }

    pub(crate) fn set_frame_identity(&mut self, identity: FrameOutputIdentity) {
        self.output_builder.set_output_frame_identity(
            identity.frame_id,
            identity.parent_id,
            identity.parent_x,
            identity.parent_y,
            identity.z_order,
            identity.undecorated,
            identity.border_width,
            identity.border_color,
            identity.background_alpha,
            identity.no_accept_focus,
        );
    }

    pub(crate) fn set_background_color(&mut self, color: Color) {
        self.output_builder.set_output_background_color(color);
    }

    pub(crate) fn set_font_pixel_size(&mut self, size: f32) {
        self.output_builder.set_output_font_pixel_size(size);
    }

    pub(crate) fn install_face(&mut self, face: &Face) {
        self.output_builder
            .install_output_face(face.id, face.clone());
    }

    pub(crate) fn install_resolved_face(
        &mut self,
        face_id: u32,
        face: &ResolvedFace,
        metrics: Option<FontMetrics>,
    ) {
        self.output_builder
            .install_output_resolved_display_row_face(face_id, face, metrics);
    }

    pub(crate) fn install_terminal_right_border(
        &mut self,
        request: BufferTextWindowTerminalRightBorderRequest,
        render_services: ChromeRowRenderServices<'_, '_>,
    ) -> u32 {
        request.install_and_apply(
            &mut TextWindowArtifactOutputSurface::from_output_builder(self.output_builder),
            render_services,
        )
    }

    pub(crate) fn add_background(&mut self, bounds: Rect, color: Color) {
        self.output_builder.add_output_background(bounds, color);
    }

    pub(crate) fn add_window_info(&mut self, info: WindowInfo) {
        self.output_builder.add_output_window_info(info);
    }

    pub(crate) fn add_transition_hint(&mut self, hint: WindowTransitionHint) {
        self.output_builder.add_output_transition_hint(hint);
    }

    pub(crate) fn add_effect_hint(&mut self, hint: WindowEffectHint) {
        self.output_builder.add_output_effect_hint(hint);
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
        self.output_builder
            .add_output_border(window_id, x, y, width, height, color);
    }

    pub(crate) fn add_scroll_bar(&mut self, item: ScrollBarItem) {
        self.output_builder.add_output_scroll_bar(item);
    }
}

impl<'output> FrameOutputReadSurface<'output> {
    pub(crate) fn from_output_builder(output_builder: &'output DisplayOutputBuilder) -> Self {
        Self { output_builder }
    }

    pub(crate) fn window_infos(&self) -> &'output [WindowInfo] {
        self.output_builder.window_infos()
    }

    pub(crate) fn transition_hints(&self) -> &'output [WindowTransitionHint] {
        self.output_builder.transition_hints()
    }

    pub(crate) fn background_color(&self) -> Color {
        *self.output_builder.background_color()
    }

    pub(crate) fn phys_cursor(&self) -> Option<&'output PhysCursor> {
        self.output_builder.phys_cursor()
    }

    pub(crate) fn cursors(&self) -> &'output [CursorItem] {
        self.output_builder.cursors()
    }

    pub(crate) fn latest_window_info(&self, window_id: i64) -> Option<WindowInfo> {
        self.window_infos()
            .iter()
            .rev()
            .find(|info| info.window_id == window_id)
            .cloned()
    }

    pub(crate) fn latest_window_enabled_rows(&self) -> Option<usize> {
        self.windows()
            .last()
            .map(|entry| entry.matrix.rows.iter().filter(|row| row.enabled).count())
    }

    fn windows(&self) -> &[WindowMatrixEntry] {
        self.output_builder.windows()
    }
}
