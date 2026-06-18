//! Display-line-number left-margin rendering — GNU `maybe_produce_line_number` (xdisp.c:25447). Relocated out of display_row_append.rs (pure move, no behavior change).

use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_row::{
    DisplayRowCurrentSourceFragmentRenderState, DisplayRowGeometry,
    DisplayRowItemSourceRenderRequest, DisplayRowRenderBounds, DisplayRowSourceRequestPolicy,
    DisplayRowSourceState,
};
use crate::display_row_builder::DisplayTabPolicy;
use crate::display_row_geometry::DisplayRowGeometryState;
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_row_walk_state::{FaceScanCheckpoint, LineNumberRenderState};
use crate::window_output::{LineNumberMarginItemSource, TextWindowLineNumberMargin};
use neomacs_display_protocol::glyph_matrix::GlyphArea;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferLineNumberMarginRenderRequest {
    mode: u8,
    current_absolute: bool,
    offset: i64,
    major_tick: i32,
    cols: i32,
}

impl BufferLineNumberMarginRenderRequest {
    pub(crate) fn new(
        mode: u8,
        current_absolute: bool,
        offset: i64,
        major_tick: i32,
        cols: i32,
    ) -> Self {
        Self {
            mode,
            current_absolute,
            offset,
            major_tick,
            cols,
        }
    }

    pub(crate) fn render_pending_with_source_state(
        self,
        line_numbers: &mut LineNumberRenderState,
        source_render: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        row_geometry: &DisplayRowGeometryState,
        face_scan: &mut FaceScanCheckpoint,
        char_width: f32,
    ) -> bool {
        let Some(line_number_request) = line_numbers.margin_render_request(
            self.mode,
            self.current_absolute,
            self.offset,
            self.major_tick,
            self.cols,
        ) else {
            return false;
        };

        let line_number_face =
            source_render.resolve_named_face(line_number_request.face().face_name());
        let line_number_face_id = face_ids.allocate();
        source_render.insert_resolved_face(line_number_face_id, &line_number_face);

        let text = line_number_request.text();
        let margin_request = TextWindowLineNumberMargin {
            text: &text,
            cols: line_number_request.cols(),
            face_id: line_number_face_id,
            row_y: row_geometry.y(),
            row_height: row_geometry.height(),
            row_ascent: row_geometry.ascent(),
            char_width,
        };
        let mut source = LineNumberMarginItemSource::new(&margin_request);
        let mut source_state = DisplayRowSourceState::default();
        let request =
            DisplayRowItemSourceRenderRequest::from_base_face_id_policy_with_render_bounds(
                DisplayRowSourceRequestPolicy::from_display_row_geometry(
                    DisplayRowGeometry {
                        y: margin_request.row_y,
                        width: (margin_request.cols.max(1) as f32 + 1.0) * char_width.max(1.0),
                        height: margin_request.row_height,
                        char_width,
                        ascent: margin_request.row_ascent,
                        tab_policy: DisplayTabPolicy::every(8),
                    },
                    neomacs_display_protocol::frame_glyphs::GlyphRowRole::Text,
                ),
                line_number_face_id,
                &line_number_face,
                DisplayRowRenderBounds::whole_row(
                    (margin_request.cols.max(1) as f32 + 1.0) * char_width.max(1.0),
                ),
            )
            .with_glyph_area(GlyphArea::LeftMargin);
        request.render_natural_fragment_into_current_row(
            &mut DisplayRowCurrentSourceFragmentRenderState {
                builder: source_render.output_render.builder,
                font_metrics: source_render.font_metrics,
                face_resolver: source_render.face_resolver,
                display_host: source_render
                    .output_render
                    .evaluator
                    .display_host
                    .as_deref(),
                face_ids,
            },
            &mut source,
            &mut source_state,
        );

        face_scan.invalidate();
        line_numbers.consume_render_request();
        true
    }
}
