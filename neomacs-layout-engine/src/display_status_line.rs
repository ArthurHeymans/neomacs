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
use crate::display_row::{
    DisplayRowBoundsPolicy, DisplayRowOwner, DisplayRowSpec, FrameFaceIdAllocator,
    MeasuredDisplayRow, install_measured_window_display_row,
};
pub(crate) use crate::display_row::{
    DisplayRowFace, DisplayRowFaceRealizer, DisplayRowOutputProgress,
};
use crate::display_text::DisplayTextFragment;
use crate::matrix_builder::GlyphMatrixBuilder;
#[cfg(test)]
use neomacs_display_protocol::face::BoxType;
use neomacs_display_protocol::types::Rect;
use neovm_core::emacs_core::Context;

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
        row_spec: DisplayRowSpec<'_>,
        rendered_text: DisplayTextFragment,
    ) -> Option<MeasuredDisplayRow> {
        let mut builder = std::mem::replace(&mut self.matrix_builder, GlyphMatrixBuilder::new());
        output_emitter.begin_chrome_progress(evaluator, output);
        let rendered_row = self.render_display_text_fragment_row_with_display_host(
            row_spec,
            rendered_text,
            face_resolver,
            evaluator.display_host.as_deref(),
            face_ids,
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
}

#[cfg(test)]
#[path = "display_status_line_test.rs"]
mod tests;
