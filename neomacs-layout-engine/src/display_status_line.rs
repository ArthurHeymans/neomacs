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
    FrameChromeKind, MeasuredDisplayRow, RenderedDisplayRow, WindowChromeKind,
    install_measured_frame_chrome_row, install_measured_window_display_row,
    install_rendered_display_row,
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
use neomacs_display_protocol::glyph_matrix::{Glyph, GlyphArea, GlyphRow};
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

pub(crate) enum FrameTabBarDisplayRowRender {
    Empty,
    Measured(MeasuredDisplayRow),
}

pub(crate) struct WindowChromeDisplayRowRequest<'face> {
    pub(crate) window_id: u64,
    pub(crate) kind: WindowChromeKind,
    pub(crate) matrix_row: usize,
    pub(crate) output: ChromeRowOutput,
    pub(crate) bounds: Rect,
    pub(crate) char_width: f32,
    pub(crate) ascent: f32,
    pub(crate) tab_policy: DisplayTabPolicy,
    pub(crate) base_face: &'face ResolvedFace,
    pub(crate) symbol_values: std::collections::HashMap<String, Value>,
    pub(crate) text: DisplayTextFragment,
}

pub(crate) struct InactiveMinibufferDisplayRowRequest<'face> {
    pub(crate) window_id: u64,
    pub(crate) window_bounds: Rect,
    pub(crate) text_bounds: Rect,
    pub(crate) selected: bool,
    pub(crate) text_width: f32,
    pub(crate) row_height: f32,
    pub(crate) char_width: f32,
    pub(crate) ascent: f32,
    pub(crate) base_face: &'face ResolvedFace,
}

pub(crate) struct EchoMinibufferDisplayRowsRequest<'face> {
    pub(crate) window_id: u64,
    pub(crate) window_bounds: Rect,
    pub(crate) text_bounds: Rect,
    pub(crate) selected: bool,
    pub(crate) text_width: f32,
    pub(crate) char_width: f32,
    pub(crate) ascent: f32,
    pub(crate) row_height: f32,
    pub(crate) base_face: &'face ResolvedFace,
    pub(crate) message: Value,
    pub(crate) max_rows: usize,
    pub(crate) truncate_lines: bool,
    pub(crate) reserve_right_special_col: bool,
}

fn window_chrome_glyph_row_role(kind: WindowChromeKind) -> GlyphRowRole {
    match kind {
        WindowChromeKind::TabLine => GlyphRowRole::TabLine,
        WindowChromeKind::HeaderLine => GlyphRowRole::HeaderLine,
        WindowChromeKind::ModeLine => GlyphRowRole::ModeLine,
    }
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
        request: WindowChromeDisplayRowRequest<'_>,
    ) -> Option<MeasuredDisplayRow> {
        let output = request.output;
        let owner = DisplayRowOwner::WindowChrome {
            window_id: request.window_id,
            kind: request.kind,
        };
        let row_spec = DisplayRowSourceRenderRequest::from_base_face(
            DisplayRowGeometry {
                y: request.bounds.y,
                width: request.bounds.width,
                height: request.bounds.height,
                char_width: request.char_width,
                ascent: request.ascent,
                tab_policy: request.tab_policy,
            },
            face_ids,
            request.base_face,
            window_chrome_glyph_row_role(request.kind),
            request.symbol_values,
        );
        let mut builder = std::mem::replace(&mut self.matrix_builder, GlyphMatrixBuilder::new());
        output_emitter.begin_chrome_progress(evaluator, output);
        let mut render_context = DisplayRowRenderContext::new(
            face_resolver,
            evaluator.display_host.as_deref(),
            face_ids,
        );
        let rendered_row = self.render_display_text_fragment_source_row_with_context(
            row_spec,
            request.text,
            &mut render_context,
        );
        let measured_row = rendered_row.map(|rendered| {
            MeasuredDisplayRow::new(
                owner,
                request.matrix_row.min(u32::MAX as usize) as u32,
                request.bounds,
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

    pub(crate) fn render_frame_tab_bar_display_row(
        &mut self,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        face_ids: &mut FrameFaceIdAllocator,
        row_index: u32,
        y: f32,
        width: f32,
        height: f32,
        char_width: f32,
        tab_bar_face: &ResolvedFace,
        rendered_text: DisplayTextFragment,
    ) -> Option<FrameTabBarDisplayRowRender> {
        let row_spec = DisplayRowSourceRenderRequest::from_base_face(
            DisplayRowGeometry {
                y,
                width,
                height,
                char_width,
                ascent: tab_bar_face.font_ascent,
                tab_policy: DisplayTabPolicy::every(8),
            },
            face_ids,
            tab_bar_face,
            GlyphRowRole::TabBar,
            std::collections::HashMap::new(),
        );
        let mut render_context =
            DisplayRowRenderContext::new(face_resolver, display_host, face_ids);
        let rendered = self.render_display_text_fragment_source_row_with_context(
            row_spec,
            rendered_text,
            &mut render_context,
        )?;
        if rendered.row.glyphs[GlyphArea::Text.index()].is_empty() {
            return Some(FrameTabBarDisplayRowRender::Empty);
        }
        let measured = MeasuredDisplayRow::new(
            DisplayRowOwner::FrameChrome {
                kind: FrameChromeKind::TabBar,
            },
            row_index,
            Rect::new(0.0, y, width, height),
            rendered,
            DisplayRowBoundsPolicy::MeasureContent,
        );
        install_measured_frame_chrome_row(
            &mut self.matrix_builder,
            &mut self.pending_frame_chrome_rows,
            &measured,
        );
        Some(FrameTabBarDisplayRowRender::Measured(measured))
    }

    pub(crate) fn render_inactive_minibuffer_window(
        &mut self,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        face_ids: &mut FrameFaceIdAllocator,
        request: InactiveMinibufferDisplayRowRequest<'_>,
    ) {
        let cols = (request.text_width / request.char_width.max(1.0))
            .ceil()
            .max(1.0) as usize;
        self.matrix_builder.begin_window_with_text_bounds(
            request.window_id,
            1,
            cols,
            request.window_bounds,
            request.text_bounds,
            request.selected,
        );
        let row_spec = DisplayRowSourceRenderRequest::from_base_face(
            DisplayRowGeometry {
                y: request.window_bounds.y,
                width: request.text_width,
                height: request.row_height,
                char_width: request.char_width,
                ascent: request.ascent,
                tab_policy: DisplayTabPolicy::every(8),
            },
            face_ids,
            request.base_face,
            GlyphRowRole::Minibuffer,
            std::collections::HashMap::new(),
        );
        let mut render_context =
            DisplayRowRenderContext::new(face_resolver, display_host, face_ids);
        let rendered = self
            .render_lisp_string_source_row_with_context(
                row_spec,
                Value::string(""),
                &mut render_context,
            )
            .expect("empty Lisp string should render an inactive minibuffer row");
        install_rendered_display_row(&mut self.matrix_builder, &rendered, 0);
        self.matrix_builder.end_window();
    }

    pub(crate) fn render_echo_minibuffer_window(
        &mut self,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        face_ids: &mut FrameFaceIdAllocator,
        request: EchoMinibufferDisplayRowsRequest<'_>,
    ) {
        let rows = self.render_minibuffer_echo_rows(
            request.window_bounds.y,
            request.text_width,
            request.char_width,
            request.ascent,
            request.row_height,
            request.base_face,
            face_resolver,
            display_host,
            request.message,
            request.max_rows,
            request.truncate_lines,
            request.reserve_right_special_col,
            face_ids,
        );
        let max_rows = rows.len().clamp(1, request.max_rows.max(1));
        let cols = (request.text_width / request.char_width.max(1.0))
            .ceil()
            .max(1.0) as usize;
        self.matrix_builder.begin_window_with_text_bounds(
            request.window_id,
            max_rows,
            cols,
            request.window_bounds,
            request.text_bounds,
            request.selected,
        );
        for (row_index, rendered) in rows.iter().enumerate() {
            install_rendered_display_row(&mut self.matrix_builder, rendered, row_index);
        }
        self.matrix_builder.end_window();
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
