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
pub(crate) use crate::display_row::{
    DisplayRowFace, DisplayRowFaceRealizer, DisplayRowOutputProgress,
};
use crate::display_row::{DisplayRowGeometry, DisplayRowSpec, install_rendered_display_source_row};
#[cfg(test)]
pub(crate) use crate::display_row::{
    OverlayFaceRun, apply_overlay_face_run, parse_overlay_face_runs,
};
use crate::matrix_builder::GlyphMatrixBuilder;
#[cfg(test)]
use neomacs_display_protocol::face::BoxType;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neovm_core::emacs_core::{Context, Value};
use std::collections::HashMap;

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

    pub(crate) fn render_window_chrome_display_source_row(
        &mut self,
        evaluator: &mut Context,
        output_emitter: &mut WindowOutputEmitter,
        face_resolver: &FaceResolver,
        next_face_id: &mut u32,
        request: WindowChromeDisplayRowRequest<'_>,
        rendered_text: Value,
    ) {
        let mut builder = std::mem::replace(&mut self.matrix_builder, GlyphMatrixBuilder::new());
        output_emitter.begin_chrome_progress(evaluator, request.output);
        let rendered_row = self.render_display_source_row(
            request.display_row_spec(next_face_id),
            rendered_text,
            face_resolver,
            next_face_id,
        );
        if let Some(ref rendered_row) = rendered_row {
            install_rendered_display_source_row(&mut builder, rendered_row, request.matrix_row);
            output_emitter.emit_chrome_progress(evaluator, request.output, rendered_row.progress);
        }
        self.matrix_builder = builder;
        if let Some(rendered_row) = rendered_row {
            output_emitter.finish_chrome_progress(rendered_row.progress);
        }
    }
}

pub(crate) struct WindowChromeDisplayRowRequest<'a> {
    pub(crate) matrix_row: usize,
    pub(crate) output: ChromeRowOutput,
    pub(crate) geometry: DisplayRowGeometry,
    pub(crate) base_face: &'a ResolvedFace,
    pub(crate) role: GlyphRowRole,
    pub(crate) symbol_values: HashMap<String, Value>,
}

impl<'a> WindowChromeDisplayRowRequest<'a> {
    pub(crate) fn display_row_spec(&self, next_face_id: &mut u32) -> DisplayRowSpec<'a> {
        DisplayRowSpec::from_base_face(
            self.geometry.clone(),
            next_face_id,
            self.base_face,
            self.role,
            self.symbol_values.clone(),
        )
    }
}

#[cfg(test)]
#[path = "display_status_line_test.rs"]
mod tests;
