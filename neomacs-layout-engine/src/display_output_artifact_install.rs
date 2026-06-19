use crate::display_output_builder::DisplayOutputBuilder;
use crate::font_metrics::FontMetrics;
use crate::neovm_bridge::ResolvedFace;
use neomacs_display_protocol::face::Face;
use neomacs_display_protocol::frame_glyphs::{WindowEffectHint, WindowInfo, WindowTransitionHint};
use neomacs_display_protocol::glyph_matrix::ScrollBarItem;
use neomacs_display_protocol::types::{Color, Rect};

pub(crate) struct OutputArtifactInstallSurface<'output> {
    output_builder: &'output mut DisplayOutputBuilder,
}

impl<'output> OutputArtifactInstallSurface<'output> {
    pub(crate) fn from_output_builder(output_builder: &'output mut DisplayOutputBuilder) -> Self {
        Self { output_builder }
    }

    pub(crate) fn set_frame_identity(
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
        self.output_builder.set_output_frame_identity(
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

    pub(crate) fn add_background(&mut self, bounds: Rect, color: Color) {
        self.output_builder.add_output_background(bounds, color);
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

    pub(crate) fn add_window_info(&mut self, info: WindowInfo) {
        self.output_builder.add_output_window_info(info);
    }

    pub(crate) fn add_transition_hint(&mut self, hint: WindowTransitionHint) {
        self.output_builder.add_output_transition_hint(hint);
    }

    pub(crate) fn add_effect_hint(&mut self, hint: WindowEffectHint) {
        self.output_builder.add_output_effect_hint(hint);
    }
}
