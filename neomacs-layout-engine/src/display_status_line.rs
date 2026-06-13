//! Display-walker chrome row rendering.
//!
//! Mode-line, header-line, tab-line, tab-bar, and minibuffer echo rows share
//! the face realization helpers defined here. The generic display-row spec,
//! property harvester, and row renderer live in `display_row`; this module
//! retains the status-line filename because it grew from the older
//! mode-line-only path.
//!
//! History: this module started as a divergent
//! parallel implementation of display-line rendering that did not
//! process display properties and dropped doom-modeline's
//! (space :align-to ...) forms. Steps 3.3' through 3.6 of the
//! display-engine unification plan merged it into the backend
//! trait and renamed the file to reflect its new role.

use super::engine::LayoutEngine;
use super::neovm_bridge::{FaceResolver, ResolvedFace};
use super::window_output::{ChromeRowOutput, DisplayProgressSink, WindowOutputEmitter};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_item::RenderFaceRef;
use crate::display_row::{
    DisplayRowBoundsPolicy, DisplayRowGeometry, DisplayRowOwner, DisplayRowRenderContext,
    DisplayRowRenderStop, DisplayRowRenderer, DisplayRowSourceRenderRequest, DisplayRowSourceState,
    MeasuredDisplayRow, RenderedDisplayRow, install_measured_window_display_row,
};
pub(crate) use crate::display_row::{
    DisplayRowFace, DisplayRowFaceRealizer, DisplayRowOutputProgress,
};
use crate::display_row_builder::DisplayTabPolicy;
use crate::display_source::LispStringSourceCursor;
use crate::display_text::DisplayTextFragment;
use crate::matrix_builder::GlyphMatrixBuilder;
#[cfg(test)]
use neomacs_display_protocol::face::BoxType;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{Glyph, GlyphRow};
use neomacs_display_protocol::types::Rect;
use neovm_core::emacs_core::Context;
use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::eval::DisplayHost;

fn empty_minibuffer_echo_row(y: f32, ascent: f32, row_height: f32) -> Vec<RenderedDisplayRow> {
    let mut row = GlyphRow::new(GlyphRowRole::Minibuffer);
    row.enabled = true;
    row.height_px = row_height.max(1.0);
    row.ascent_px = ascent.max(0.0).min(row.height_px);
    vec![RenderedDisplayRow {
        row,
        progress: DisplayRowOutputProgress {
            end_x: 0.0,
            end_col: 0,
            y,
            height: row_height.max(1.0),
        },
        source_slots: Vec::new(),
        faces: Vec::new(),
        media: Vec::new(),
    }]
}

impl LayoutEngine {
    pub(crate) fn realize_display_row_face(
        &mut self,
        face_id: u32,
        face: &ResolvedFace,
        char_w: f32,
        ascent: f32,
        row_height: f32,
    ) -> DisplayRowFace {
        DisplayRowFaceRealizer::new(&mut self.font_metrics)
            .realize_face(face_id, face, char_w, ascent, row_height)
    }

    pub(crate) fn display_row_height_for_face(
        &mut self,
        face: &ResolvedFace,
        char_w: f32,
        fallback_ascent: f32,
        fallback_row_height: f32,
    ) -> f32 {
        DisplayRowFaceRealizer::new(&mut self.font_metrics).row_height_for_face(
            face,
            char_w,
            fallback_ascent,
            fallback_row_height,
        )
    }

    pub(crate) fn render_window_chrome_display_row(
        &mut self,
        evaluator: &mut Context,
        output_emitter: &mut WindowOutputEmitter,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        matrix_row: usize,
        output: ChromeRowOutput,
        owner: DisplayRowOwner,
        fallback_bounds: Rect,
        row_spec: DisplayRowSourceRenderRequest<'_>,
        rendered_text: DisplayTextFragment,
    ) -> Option<MeasuredDisplayRow> {
        let mut builder = std::mem::replace(&mut self.matrix_builder, GlyphMatrixBuilder::new());
        output_emitter.begin_chrome_progress(evaluator, output);
        let mut render_context = DisplayRowRenderContext::new(
            face_resolver,
            evaluator.display_host.as_deref(),
            face_ids,
        );
        let rendered_row = self.render_display_text_fragment_source_row_with_context(
            row_spec,
            rendered_text,
            &mut render_context,
        );
        let measured_row = rendered_row.map(|rendered| {
            MeasuredDisplayRow::new(
                owner,
                matrix_row.min(u32::MAX as usize) as u32,
                fallback_bounds,
                rendered,
                DisplayRowBoundsPolicy::PreserveAllocatedMinimum,
            )
        });
        if let Some(ref measured_row) = measured_row {
            install_measured_window_display_row(&mut builder, measured_row);
            output_emitter.emit_chrome_progress(evaluator, output, measured_row.output_progress());
        }
        self.matrix_builder = builder;
        if let Some(ref measured_row) = measured_row {
            output_emitter.finish_chrome_progress(measured_row.output_progress());
        }
        measured_row
    }

    /// Build minibuffer echo rows through the shared display-source path.
    ///
    /// The returned rows retain their realized faces and progress metadata so
    /// the caller can install them through the same path used by chrome rows.
    pub(crate) fn render_minibuffer_echo_rows(
        &mut self,
        y: f32,
        text_width: f32,
        char_w: f32,
        ascent: f32,
        row_height: f32,
        default_resolved: &ResolvedFace,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        echo_message: Value,
        max_rows: usize,
        truncate_lines: bool,
        reserve_right_special_col: bool,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> Vec<RenderedDisplayRow> {
        let mut base_face = default_resolved.clone();
        if base_face.font_char_width <= 0.0 {
            base_face.font_char_width = char_w.max(1.0);
        }
        if base_face.font_ascent <= 0.0 {
            base_face.font_ascent = ascent.max(row_height * 0.8);
        }
        let row_face = self.realize_display_row_face(0, &base_face, char_w, ascent, row_height);
        let base_render_face = row_face.render_face();
        let char_width = self.display_row_char_width(&row_face, char_w);
        let reserve_width = if reserve_right_special_col {
            char_width.max(1.0)
        } else {
            0.0
        };
        let wrap_width = if truncate_lines {
            text_width
        } else {
            (text_width - reserve_width).max(char_width.max(1.0))
        };
        let matrix_cols = (text_width / char_w.max(1.0)).ceil().max(1.0) as usize;
        let special_col = matrix_cols.saturating_sub(1);
        let base_face_id = if base_face.face_id != 0 {
            base_face.face_id
        } else {
            face_ids.allocate()
        };
        let Some(mut source) =
            LispStringSourceCursor::new(1, echo_message, RenderFaceRef::FaceId(base_face_id))
        else {
            return empty_minibuffer_echo_row(y, ascent, row_height);
        };
        let mut source_state = DisplayRowSourceState::default();
        let mut renderer = DisplayRowRenderer::new(&mut self.font_metrics);
        let mut render_context =
            DisplayRowRenderContext::new(face_resolver, display_host, face_ids);

        let mut rows = Vec::new();
        let max_rows = max_rows.max(1);
        while rows.len() < max_rows {
            let request = DisplayRowSourceRenderRequest::whole_row(
                DisplayRowGeometry {
                    y: y + rows.len() as f32 * row_height,
                    width: wrap_width,
                    height: row_height,
                    char_width: char_w,
                    ascent,
                    tab_policy: DisplayTabPolicy::every(8),
                },
                base_face_id,
                &base_face,
                GlyphRowRole::Minibuffer,
            );
            let Some(result) = renderer
                .render_display_item_source_row_step_from_request_with_context(
                    request,
                    &mut source,
                    &mut source_state,
                    &mut render_context,
                )
            else {
                break;
            };
            let stop = result.stop;
            let mut rendered = result.rendered;
            let special_face_id = rendered
                .faces
                .first()
                .map(|face| face.id)
                .unwrap_or(base_render_face.id);
            rendered.row.role = GlyphRowRole::Minibuffer;
            rendered.row.mode_line = false;
            if reserve_right_special_col && stop == DisplayRowRenderStop::Clipped {
                let ch = if truncate_lines { '$' } else { '\\' };
                while rendered.row.glyphs[1].len() < special_col {
                    rendered.row.glyphs[1].push(
                        Glyph::char(' ', special_face_id, 0).with_pixel_width(char_width.max(1.0)),
                    );
                }
                rendered.row.glyphs[1].push(
                    Glyph::char(ch, special_face_id, 0).with_pixel_width(char_width.max(1.0)),
                );
                rendered.progress.end_x = text_width.max(0.0);
                rendered.progress.end_col = matrix_cols as i64;
            }
            rows.push(rendered);
            match stop {
                DisplayRowRenderStop::SourceExhausted => break,
                DisplayRowRenderStop::RowBreak => {}
                DisplayRowRenderStop::Clipped => {
                    if truncate_lines {
                        break;
                    }
                }
            }
        }
        if rows.is_empty() {
            return empty_minibuffer_echo_row(y, ascent, row_height);
        }
        rows
    }
}

#[cfg(test)]
#[path = "display_status_line_test.rs"]
mod tests;
