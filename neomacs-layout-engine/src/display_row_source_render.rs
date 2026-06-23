//! Source-render state facade.
//!
//! This module holds the state types that bridge the typed display-source
//! layer (`DisplayItemSource`) to the row renderer and output builder.  It
//! lives between `display_source.rs` / `display_row.rs` and the append-layer
//! helpers in `display_row_append.rs`, so that the append module does not
//! need to own the render-state facade.

use crate::display_current_row_output::{DisplayCurrentRowMutation, DisplayRowCurrentRowOutput};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_face_policy::BaseFacePolicy;
use crate::display_item::DisplayPropertyReplacementDescriptor;
use crate::display_origin::DisplayOrigin;
use crate::display_row::{
    DisplayRowRenderContext, DisplayRowRenderExecutor, DisplayRowRenderer,
    DisplayRowSourceFragmentFrame, DisplayRowSourceRenderRequest,
};
use crate::display_row_append_context::DisplayRowAppendSourceRenderRequest;
use crate::display_row_builder::DisplayRowPosition;
use crate::display_row_face_state::{
    DisplayRowActiveFaceState, DisplayRowMeasurementPolicy, DisplayRowResolvedMeasuredFace,
};
use crate::display_row_geometry::{DisplayRowGeometryState, DisplayRowScopedValue};
use crate::display_row_metrics::DisplayRowFallbackMetrics;
use crate::display_row_render_policy::DisplayRowRenderPolicy;
use crate::display_row_render_state::{
    CurrentTextRowRenderOutcome, DisplayRowRenderIntoRowResult, display_row_output_end_position,
};
use crate::display_row_replacement::DisplayPropertyReplacementRowRenderRequest;
use crate::display_row_source_state::DisplayRowSourceState;
use crate::display_row_text_output::TextRowOutput;
use crate::display_source::DisplayItemSource;
use crate::display_source_resolver::{
    ActiveDisplayStringBaseFace, DisplayDefaultFaceInstallPolicy, DisplayStringBaseFace,
    resolve_display_string_base_face,
};
use crate::display_text_output_install::TextWindowRowDecorationRequest;
use crate::font_metrics::FontMetricsService;
use crate::glyph_row_writer::push_stretch_to_area;
use crate::neovm_bridge::{FaceResolver, LayoutBufferView, ResolvedFace};
use crate::types::WindowParams;
use crate::window_output::{
    DisplayTextRowGeometryTransition, DisplayTextRowMetrics, DisplayTextRowTransition,
    TextWindowOutputTarget, WindowOutputEmitter, finish_and_end_text_window_row,
    install_text_window_row_decoration_request, transition_text_window_row,
    transition_text_window_row_with_limit,
};
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{
    Glyph, GlyphArea, GlyphRow, NO_BUFFER_POSITION_CHARPOS,
};
use neomacs_display_protocol::types::Color;
use neovm_core::emacs_core::Context;
use neovm_core::emacs_core::eval::DisplayHost;
use neovm_core::window::DisplayRowSnapshot;

pub(crate) struct TextRowOutputRenderState<'a> {
    output: TextWindowOutputTarget<'a>,
    output_emitter: &'a mut WindowOutputEmitter,
    evaluator: &'a mut Context,
}

struct DisplayRowCurrentTextSourceState<'face, 'emit> {
    row_output: DisplayRowCurrentRowOutput<'emit>,
    evaluator: &'emit mut Context,
    font_metrics: &'emit mut Option<FontMetricsService>,
    face_resolver: &'face FaceResolver,
    face_ids: &'emit mut FrameFaceIdAllocator,
}

struct DisplayRowCurrentSourceFragmentRenderState<'face, 'emit> {
    row_output: DisplayRowCurrentRowOutput<'emit>,
    font_metrics: &'emit mut Option<FontMetricsService>,
    face_resolver: &'face FaceResolver,
    display_host: Option<&'emit dyn DisplayHost>,
    face_ids: &'emit mut FrameFaceIdAllocator,
}

struct DisplayRowCurrentTextSourceStepResult {
    role: GlyphRowRole,
    result: DisplayRowRenderIntoRowResult,
    row_height_px: f32,
    row_ascent_px: f32,
}

struct DisplayRowCurrentSourceStepMutation<'a, 'request, 'renderer, 'face, 'host, S, P> {
    row_request: DisplayRowSourceRenderRequest<'request>,
    renderer: DisplayRowRenderer<'renderer>,
    source: &'a mut S,
    source_state: &'a mut DisplayRowSourceState,
    context: DisplayRowRenderContext<'face, 'host>,
    render_policy: &'a mut P,
}

struct DisplayRowNaturalSourceFragmentMutation<'a, 'request, 'metrics, 'face, 'host, S> {
    request: DisplayRowSourceRenderRequest<'request>,
    render_executor: &'a mut DisplayRowRenderExecutor<'metrics, 'face, 'host>,
    source: &'a mut S,
    source_state: &'a mut DisplayRowSourceState,
}

/// Geometry + face payload for one trailing `:extend` fill (GNU
/// `extend_face_to_end_of_line`). `face_id` is the extend face installed on the
/// row; `bg` is its background pixel; `(width_px, height_px, ascent_px)` are the
/// stretch geometry; `char_width` is the face's column advance used to size the
/// stretch column count.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RowExtendFill {
    bg: Color,
    face_id: u32,
    width_px: f32,
    height_px: f32,
    ascent_px: f32,
    char_width: f32,
}

impl RowExtendFill {
    pub(crate) fn new(
        bg: Color,
        face_id: u32,
        width_px: f32,
        height_px: f32,
        ascent_px: f32,
        char_width: f32,
    ) -> Self {
        Self {
            bg,
            face_id,
            width_px,
            height_px,
            ascent_px,
            char_width,
        }
    }

    /// Number of stretch columns (>=1) covering `width_px` at the face advance.
    fn width_cols(self) -> u16 {
        let cw = self.char_width.max(1.0);
        ((self.width_px / cw).ceil() as i64).clamp(1, u16::MAX as i64) as u16
    }
}

/// Mutation that appends the trailing extend-face stretch to the current row's
/// TEXT area, without emitting any output span. Mirrors GNU
/// `extend_face_to_end_of_line`: an empty row first gets a leading space glyph
/// (xdisp.c:24420) so the row `displays_text` and carries a face anchor; the
/// fill stretch is then pushed to the text-area right edge. Returns `true` when
/// a fill was applied.
struct RowExtendFillMutation {
    fill: RowExtendFill,
}

impl DisplayCurrentRowMutation for RowExtendFillMutation {
    type Output = bool;

    fn apply(self, row: &mut GlyphRow) -> Self::Output {
        // R2L safety: never fill a reversed row (the stretch would reorder to
        // the visual left). The caller already guards on reversed_p, but the
        // row is the authoritative source so we re-check here.
        if row.reversed_p {
            return false;
        }
        let text_index = GlyphArea::Text.index();
        // Empty row: push a leading space carrying the extend face so the row
        // displays text and has a face anchor (GNU xdisp.c:24420). It covers no
        // buffer position, so stamp the no-position sentinel (see GNU's
        // `glyph->charpos = -1` for the same anchor, src/xdisp.c:26021) — this
        // keeps the blank-line cursor from latching onto the fill.
        if row.glyphs[text_index].is_empty() {
            row.glyphs[text_index].push(
                Glyph::char(' ', self.fill.face_id, NO_BUFFER_POSITION_CHARPOS)
                    .with_pixel_width(self.fill.char_width.max(1.0)),
            );
            row.displays_text = true;
        }
        push_stretch_to_area(
            row,
            text_index,
            self.fill.width_cols(),
            self.fill.face_id,
            self.fill.width_px,
            self.fill.height_px,
            self.fill.ascent_px,
        );
        // The trailing fill stretch likewise maps to no buffer position; mark it
        // so cursor placement excludes it (GNU set_cursor_from_row, xdisp.c:18648).
        if let Some(last) = row.glyphs[text_index].last_mut() {
            last.charpos = NO_BUFFER_POSITION_CHARPOS;
        }
        true
    }
}

impl<S, P> DisplayCurrentRowMutation
    for DisplayRowCurrentSourceStepMutation<'_, '_, '_, '_, '_, S, P>
where
    S: DisplayItemSource,
    P: DisplayRowRenderPolicy,
{
    type Output = Option<(DisplayRowRenderIntoRowResult, f32, f32)>;

    fn apply(self, row: &mut neomacs_display_protocol::glyph_matrix::GlyphRow) -> Self::Output {
        let mut renderer = self.renderer;
        let mut context = self.context;
        let result = self.row_request.render_fragment_step_into_row_with_policy(
            &mut renderer,
            row,
            self.source,
            self.source_state,
            &mut context,
            self.render_policy,
        )?;
        result.apply_current_row_effects_to(row);
        Some((result, row.height_px, row.ascent_px))
    }
}

impl<S> DisplayCurrentRowMutation for DisplayRowNaturalSourceFragmentMutation<'_, '_, '_, '_, '_, S>
where
    S: DisplayItemSource,
{
    type Output = Option<DisplayRowRenderIntoRowResult>;

    fn apply(self, row: &mut neomacs_display_protocol::glyph_matrix::GlyphRow) -> Self::Output {
        let result = self.render_executor.render_item_source_fragment_into_row(
            self.request,
            row,
            self.source,
            self.source_state,
        )?;
        result.apply_current_row_effects_to(row);
        Some(result)
    }
}

impl<'face, 'emit> DisplayRowCurrentTextSourceState<'face, 'emit> {
    fn new(
        row_output: DisplayRowCurrentRowOutput<'emit>,
        evaluator: &'emit mut Context,
        font_metrics: &'emit mut Option<FontMetricsService>,
        face_resolver: &'face FaceResolver,
        face_ids: &'emit mut FrameFaceIdAllocator,
    ) -> Self {
        Self {
            row_output,
            evaluator,
            font_metrics,
            face_resolver,
            face_ids,
        }
    }

    fn render_source_with_policy<S, P>(
        &mut self,
        row_request: DisplayRowSourceRenderRequest<'_>,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        render_policy: &mut P,
    ) -> Option<DisplayRowCurrentTextSourceStepResult>
    where
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    {
        let role = row_request.role();
        let mutation = DisplayRowCurrentSourceStepMutation {
            row_request,
            renderer: DisplayRowRenderer::new(self.font_metrics),
            source,
            source_state,
            context: DisplayRowRenderContext::new(
                self.face_resolver,
                self.evaluator.display_host.as_deref(),
                self.face_ids,
            ),
            render_policy,
        };
        let (result, row_height_px, row_ascent_px) =
            self.row_output.apply_current_row_mutation(mutation)??;
        Some(DisplayRowCurrentTextSourceStepResult {
            role,
            result,
            row_height_px,
            row_ascent_px,
        })
    }
    fn measure_source_with_policy<S, P>(
        &mut self,
        row_request: DisplayRowSourceRenderRequest<'_>,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        render_policy: &mut P,
    ) -> Option<DisplayRowCurrentTextSourceStepResult>
    where
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    {
        let role = row_request.role();
        let mutation = DisplayRowCurrentSourceStepMutation {
            row_request,
            renderer: DisplayRowRenderer::new(self.font_metrics),
            source,
            source_state,
            context: DisplayRowRenderContext::new(
                self.face_resolver,
                self.evaluator.display_host.as_deref(),
                self.face_ids,
            ),
            render_policy,
        };
        let (result, row_height_px, row_ascent_px) = self
            .row_output
            .apply_current_row_scratch_mutation(mutation)??;
        Some(DisplayRowCurrentTextSourceStepResult {
            role,
            result,
            row_height_px,
            row_ascent_px,
        })
    }
}

impl<'face, 'emit> DisplayRowCurrentSourceFragmentRenderState<'face, 'emit> {
    fn new(
        row_output: DisplayRowCurrentRowOutput<'emit>,
        font_metrics: &'emit mut Option<FontMetricsService>,
        face_resolver: &'face FaceResolver,
        display_host: Option<&'emit dyn DisplayHost>,
        face_ids: &'emit mut FrameFaceIdAllocator,
    ) -> Self {
        Self {
            row_output,
            font_metrics,
            face_resolver,
            display_host,
            face_ids,
        }
    }

    fn render_natural_fragment_into_current_row<S: DisplayItemSource>(
        &mut self,
        request: DisplayRowSourceRenderRequest<'_>,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        let mut render_executor = DisplayRowRenderExecutor::new(
            self.font_metrics,
            self.face_resolver,
            self.display_host,
            self.face_ids,
        );
        let result = self.row_output.apply_current_row_mutation(
            DisplayRowNaturalSourceFragmentMutation {
                request,
                render_executor: &mut render_executor,
                source,
                source_state,
            },
        )??;
        Some(result)
    }
}

impl DisplayRowCurrentTextSourceStepResult {
    fn into_measure_outcome(self) -> CurrentTextRowRenderOutcome {
        let (progress, source_slots, _faces, _media, stop) = self.result.into_current_row_parts();
        let end = display_row_output_end_position(progress);
        CurrentTextRowRenderOutcome::new(
            stop,
            source_slots,
            end,
            self.row_height_px,
            self.row_ascent_px,
        )
    }
}

fn render_display_item_source_into_current_text_row<S, P>(
    state: &mut DisplayRowCurrentTextSourceState<'_, '_>,
    source: &mut S,
    source_state: &mut DisplayRowSourceState,
    request: DisplayRowSourceRenderRequest<'_>,
    render_policy: &mut P,
) -> Option<DisplayRowCurrentTextSourceStepResult>
where
    S: DisplayItemSource,
    P: DisplayRowRenderPolicy,
{
    state.render_source_with_policy(request, source, source_state, render_policy)
}

fn measure_display_item_source_against_current_text_row<S, P>(
    state: &mut DisplayRowCurrentTextSourceState<'_, '_>,
    source: &mut S,
    source_state: &mut DisplayRowSourceState,
    request: DisplayRowSourceRenderRequest<'_>,
    render_policy: &mut P,
) -> Option<CurrentTextRowRenderOutcome>
where
    S: DisplayItemSource,
    P: DisplayRowRenderPolicy,
{
    state
        .measure_source_with_policy(request, source, source_state, render_policy)
        .map(DisplayRowCurrentTextSourceStepResult::into_measure_outcome)
}

impl<'a> TextRowOutputRenderState<'a> {
    pub(crate) fn from_parts(
        output: TextWindowOutputTarget<'a>,
        output_emitter: &'a mut WindowOutputEmitter,
        evaluator: &'a mut Context,
    ) -> Self {
        Self {
            output,
            output_emitter,
            evaluator,
        }
    }

    pub(crate) fn reborrow(&mut self) -> TextRowOutputRenderState<'_> {
        TextRowOutputRenderState {
            output: self.output.reborrow(),
            output_emitter: self.output_emitter,
            evaluator: self.evaluator,
        }
    }

    pub(crate) fn with_output_target_parts<R>(
        self,
        f: impl FnOnce(TextWindowOutputTarget<'_>, &mut WindowOutputEmitter, &mut Context) -> R,
    ) -> R {
        f(self.output, self.output_emitter, self.evaluator)
    }

    pub(crate) fn finish_and_end_text_row(self, metrics: DisplayTextRowMetrics) {
        finish_and_end_text_window_row(self.output, self.output_emitter, metrics);
    }

    pub(crate) fn transition_text_row(self, transition: DisplayTextRowGeometryTransition) {
        transition_text_window_row(self.output, self.output_emitter, self.evaluator, transition);
    }

    pub(crate) fn transition_text_row_with_limit(
        self,
        transition: DisplayTextRowGeometryTransition,
        max_rows: usize,
    ) -> DisplayTextRowTransition {
        transition_text_window_row_with_limit(
            self.output,
            self.output_emitter,
            self.evaluator,
            transition,
            max_rows,
        )
    }

    pub(crate) fn install_row_decoration(self, request: TextWindowRowDecorationRequest) {
        install_text_window_row_decoration_request(self.output, request);
    }

    fn insert_resolved_face(&mut self, face_id: u32, face: &ResolvedFace) {
        self.output.install_resolved_face(face_id, face, None);
    }

    fn install_resolved_measured_face(&mut self, face: &DisplayRowResolvedMeasuredFace) {
        self.output.install_resolved_face(
            face.face_id(),
            face.resolved_face(),
            face.font_metrics(),
        );
    }

    fn display_host(&self) -> Option<&dyn DisplayHost> {
        self.evaluator.display_host.as_deref()
    }

    fn output_emitter(&mut self) -> &mut WindowOutputEmitter {
        self.output_emitter
    }

    fn output_rows(&self) -> &[DisplayRowSnapshot] {
        self.output_emitter.rows()
    }

    fn output_rows_len(&self) -> usize {
        self.output_emitter.rows().len()
    }

    fn measure_state<'emit>(
        &'emit mut self,
        font_metrics: &'emit mut Option<FontMetricsService>,
        face_resolver: &'emit FaceResolver,
    ) -> TextRowSourceMeasureState<'emit> {
        TextRowSourceMeasureState {
            row_output: self.output.current_row_output(),
            evaluator: self.evaluator,
            font_metrics,
            face_resolver,
        }
    }

    fn current_text_render_state<'emit>(
        &'emit mut self,
        font_metrics: &'emit mut Option<FontMetricsService>,
        face_resolver: &'emit FaceResolver,
        face_ids: &'emit mut FrameFaceIdAllocator,
    ) -> DisplayRowCurrentTextSourceState<'emit, 'emit> {
        DisplayRowCurrentTextSourceState::new(
            self.output.current_row_output(),
            self.evaluator,
            font_metrics,
            face_resolver,
            face_ids,
        )
    }

    fn current_source_fragment_render_state<'emit>(
        &'emit mut self,
        font_metrics: &'emit mut Option<FontMetricsService>,
        face_resolver: &'emit FaceResolver,
        face_ids: &'emit mut FrameFaceIdAllocator,
    ) -> DisplayRowCurrentSourceFragmentRenderState<'emit, 'emit> {
        DisplayRowCurrentSourceFragmentRenderState::new(
            self.output.current_row_output(),
            font_metrics,
            face_resolver,
            self.evaluator.display_host.as_deref(),
            face_ids,
        )
    }

    /// Append a trailing `:extend` fill stretch to the current row's TEXT area
    /// without emitting an output span. Returns `true` when a fill was applied.
    fn extend_current_row_face_to_end_of_line(&mut self, fill: RowExtendFill) -> bool {
        self.output
            .current_row_output()
            .apply_current_row_mutation(RowExtendFillMutation { fill })
            .unwrap_or(false)
    }

    fn finish_current_text_row_render(
        &mut self,
        output: TextRowOutput,
        result: DisplayRowCurrentTextSourceStepResult,
    ) -> CurrentTextRowRenderOutcome {
        let DisplayRowCurrentTextSourceStepResult {
            role,
            result,
            row_height_px,
            row_ascent_px,
        } = result;
        let (progress, source_slots, faces, media, stop) = result.into_current_row_parts();
        let end = display_row_output_end_position(progress);
        self.output
            .install_rendered_fragment_assets(role, output.row(), &faces, &media);
        let output_spans = output.spans_for_source_slots(&source_slots);
        self.output_emitter
            .emit_text_output_spans(self.evaluator, output, output_spans, end);
        CurrentTextRowRenderOutcome::new(stop, source_slots, end, row_height_px, row_ascent_px)
    }
}

pub(crate) struct TextRowSourceRenderState<'a> {
    output_render: TextRowOutputRenderState<'a>,
    font_metrics: &'a mut Option<FontMetricsService>,
    face_resolver: &'a FaceResolver,
}

impl<'a> TextRowSourceRenderState<'a> {
    pub(crate) fn from_output_render(
        output_render: TextRowOutputRenderState<'a>,
        font_metrics: &'a mut Option<FontMetricsService>,
        face_resolver: &'a FaceResolver,
    ) -> Self {
        Self {
            output_render,
            font_metrics,
            face_resolver,
        }
    }

    pub(crate) fn reborrow(&mut self) -> TextRowSourceRenderState<'_> {
        TextRowSourceRenderState {
            output_render: self.output_render.reborrow(),
            font_metrics: self.font_metrics,
            face_resolver: self.face_resolver,
        }
    }

    pub(crate) fn output_render(&mut self) -> TextRowOutputRenderState<'_> {
        self.output_render.reborrow()
    }

    pub(crate) fn measure_state(&mut self) -> TextRowSourceMeasureState<'_> {
        self.output_render
            .measure_state(self.font_metrics, self.face_resolver)
    }

    pub(crate) fn insert_resolved_face(&mut self, face_id: u32, face: &ResolvedFace) {
        self.output_render.insert_resolved_face(face_id, face);
    }

    fn install_pending_display_string_base_face(&mut self, base_face: &DisplayStringBaseFace) {
        if let Some(pending_face) = base_face.pending_face() {
            self.insert_resolved_face(pending_face.face_id(), pending_face.resolved());
        }
    }

    fn resolved_measured_face(
        &mut self,
        measurement_policy: DisplayRowMeasurementPolicy,
        face_id: u32,
        face: ResolvedFace,
        window_system: bool,
        fallback_char_width: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
    ) -> DisplayRowResolvedMeasuredFace {
        let metrics = if window_system {
            self.font_metrics.as_mut().map(|svc| {
                svc.font_metrics(
                    &face.font_family,
                    face.font_weight,
                    face.italic,
                    face.font_size,
                )
            })
        } else {
            None
        };
        measurement_policy.resolved_measured_face(
            face_id,
            face,
            metrics,
            fallback_char_width,
            fallback_metrics,
            self.font_metrics,
        )
    }

    pub(crate) fn resolve_and_install_measured_face(
        &mut self,
        measurement_policy: DisplayRowMeasurementPolicy,
        face_id: u32,
        face: ResolvedFace,
        window_system: bool,
        fallback_char_width: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
    ) -> DisplayRowActiveFaceState {
        let resolved_face = self.resolved_measured_face(
            measurement_policy,
            face_id,
            face,
            window_system,
            fallback_char_width,
            fallback_metrics,
        );
        self.output_render
            .install_resolved_measured_face(&resolved_face);
        resolved_face.into_active_face_state()
    }

    pub(crate) fn resolve_named_face(&self, face_name: &str) -> ResolvedFace {
        self.face_resolver.resolve_named_face(face_name)
    }

    pub(crate) fn default_face(&self) -> ResolvedFace {
        self.face_resolver.default_face().clone()
    }

    pub(crate) fn display_string_base_face<B: LayoutBufferView>(
        &mut self,
        buffer: &B,
        origin: DisplayOrigin,
        policy: BaseFacePolicy,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> DisplayStringBaseFace {
        let base_face = resolve_display_string_base_face(
            buffer,
            self.face_resolver,
            origin,
            policy,
            None,
            DisplayDefaultFaceInstallPolicy::InstallDefaultFace,
            face_ids,
        );
        self.install_pending_display_string_base_face(&base_face);
        base_face
    }

    pub(crate) fn default_display_string_base_face<B: LayoutBufferView>(
        &mut self,
        buffer: &B,
        origin: DisplayOrigin,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> DisplayStringBaseFace {
        self.display_string_base_face(buffer, origin, origin.default_base_face_policy(), face_ids)
    }

    pub(crate) fn display_string_base_face_for_active_row<B: LayoutBufferView>(
        &mut self,
        buffer: &B,
        origin: DisplayOrigin,
        policy: BaseFacePolicy,
        active_face_state: &DisplayRowActiveFaceState,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> DisplayStringBaseFace {
        let base_face = resolve_display_string_base_face(
            buffer,
            self.face_resolver,
            origin,
            policy,
            Some(ActiveDisplayStringBaseFace::new(
                active_face_state.face_id(),
                active_face_state.resolved_face(),
            )),
            DisplayDefaultFaceInstallPolicy::ReuseInstalledDefaultFace,
            face_ids,
        );
        self.install_pending_display_string_base_face(&base_face);
        base_face
    }

    pub(crate) fn default_display_string_base_face_for_active_row<B: LayoutBufferView>(
        &mut self,
        buffer: &B,
        origin: DisplayOrigin,
        active_face_state: &DisplayRowActiveFaceState,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> DisplayStringBaseFace {
        self.display_string_base_face_for_active_row(
            buffer,
            origin,
            origin.default_base_face_policy(),
            active_face_state,
            face_ids,
        )
    }

    pub(crate) fn render_natural_fragment_into_current_row<S: DisplayItemSource>(
        &mut self,
        request: DisplayRowSourceRenderRequest<'_>,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        self.output_render
            .current_source_fragment_render_state(self.font_metrics, self.face_resolver, face_ids)
            .render_natural_fragment_into_current_row(request, source, source_state)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_natural_fragment_from_row_geometry_columns<S: DisplayItemSource>(
        &mut self,
        row_geometry: &DisplayRowGeometryState,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        cols: usize,
        char_width: f32,
        role: neomacs_display_protocol::frame_glyphs::GlyphRowRole,
        face_id: u32,
        base_face: &ResolvedFace,
        start_col: usize,
        max_col: usize,
        area: neomacs_display_protocol::glyph_matrix::GlyphArea,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        let request = DisplayRowSourceFragmentFrame::from_row_geometry_columns(
            row_geometry,
            cols,
            char_width,
            role,
            face_id,
            base_face,
        )
        .render_request_from_column_for_area(start_col, max_col, area);
        self.render_natural_fragment_into_current_row(request, source, source_state, face_ids)
    }

    pub(crate) fn render_display_item_source_into_current_text_row_and_emit<
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    >(
        &mut self,
        face_ids: &mut FrameFaceIdAllocator,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        request: DisplayRowAppendSourceRenderRequest<'_>,
        render_policy: &mut P,
    ) -> Option<CurrentTextRowRenderOutcome> {
        let mut state = current_text_render_state(self, face_ids);
        let (result, output) = request.render_with_row_request(|row_request| {
            render_display_item_source_into_current_text_row(
                &mut state,
                source,
                source_state,
                row_request,
                render_policy,
            )
        });
        let result = result?;
        Some(
            self.output_render
                .finish_current_text_row_render(output, result),
        )
    }

    pub(crate) fn mark_current_text_row_truncated_left(&mut self) {
        self.output_render()
            .install_row_decoration(TextWindowRowDecorationRequest::MarkCurrentTruncatedLeft);
    }

    /// Fill the current row's background from the current pen `x` to the
    /// text-area `right_edge` with the active `:extend` face (GNU
    /// `extend_face_to_end_of_line`). No-op (returns `false`) when the row is
    /// reversed (R2L), when the fill width is non-positive, or when the extend
    /// background equals the frame background.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn extend_face_to_end_of_line(
        &mut self,
        row_extend: &DisplayRowScopedValue<(Color, u32)>,
        row_geometry: &DisplayRowGeometryState,
        current_x: f32,
        right_edge: f32,
        frame_background: Color,
        reversed_p: bool,
        height_px: f32,
        ascent_px: f32,
        char_width: f32,
    ) -> bool {
        if reversed_p {
            return false;
        }
        let fill_px = right_edge - current_x;
        if fill_px <= 0.0 {
            return false;
        }
        let Some(&(bg, face_id)) = row_extend.value_on(row_geometry) else {
            return false;
        };
        if bg == frame_background {
            return false;
        }
        self.output_render
            .extend_current_row_face_to_end_of_line(RowExtendFill::new(
                bg, face_id, fill_px, height_px, ascent_px, char_width,
            ))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_display_property_replacement_row_request(
        &mut self,
        descriptor: &DisplayPropertyReplacementDescriptor,
        source_text: &[u8],
        active_face_state: &DisplayRowActiveFaceState,
        current_x: f32,
        content_x: f32,
        params: &WindowParams,
        glyph_y_offset: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
        start_position: DisplayRowPosition,
    ) -> Option<DisplayPropertyReplacementRowRenderRequest> {
        DisplayPropertyReplacementRowRenderRequest::from_typed_replacement_descriptor(
            descriptor,
            source_text,
            active_face_state,
            self.font_metrics,
            current_x,
            content_x,
            params,
            self.output_render.display_host(),
            glyph_y_offset,
            fallback_metrics,
            start_position,
        )
    }

    pub(crate) fn output_emitter(&mut self) -> &mut WindowOutputEmitter {
        self.output_render.output_emitter()
    }

    pub(crate) fn output_rows(&self) -> &[DisplayRowSnapshot] {
        self.output_render.output_rows()
    }

    pub(crate) fn output_rows_len(&self) -> usize {
        self.output_render.output_rows_len()
    }
}

fn current_text_render_state<'emit>(
    state: &'emit mut TextRowSourceRenderState<'_>,
    face_ids: &'emit mut FrameFaceIdAllocator,
) -> DisplayRowCurrentTextSourceState<'emit, 'emit> {
    state
        .output_render
        .current_text_render_state(state.font_metrics, state.face_resolver, face_ids)
}

pub(crate) struct TextRowSourceMeasureState<'a> {
    row_output: DisplayRowCurrentRowOutput<'a>,
    evaluator: &'a mut Context,
    font_metrics: &'a mut Option<FontMetricsService>,
    face_resolver: &'a FaceResolver,
}

impl<'a> TextRowSourceMeasureState<'a> {
    #[cfg(test)]
    pub(crate) fn from_current_row(
        row_output: DisplayRowCurrentRowOutput<'a>,
        evaluator: &'a mut Context,
        font_metrics: &'a mut Option<FontMetricsService>,
        face_resolver: &'a FaceResolver,
    ) -> Self {
        Self {
            row_output,
            evaluator,
            font_metrics,
            face_resolver,
        }
    }

    pub(crate) fn font_metrics(&mut self) -> &mut Option<FontMetricsService> {
        self.font_metrics
    }

    pub(crate) fn current_cluster_tail(&self) -> Option<(char, bool)> {
        self.row_output.cluster_tail()
    }

    pub(crate) fn measure_display_item_source_against_current_text_row<
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    >(
        &mut self,
        face_ids: &mut FrameFaceIdAllocator,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        row_request: DisplayRowSourceRenderRequest<'_>,
        render_policy: &mut P,
    ) -> Option<CurrentTextRowRenderOutcome> {
        let mut state = current_text_measure_state(self, face_ids);
        measure_display_item_source_against_current_text_row(
            &mut state,
            source,
            source_state,
            row_request,
            render_policy,
        )
    }
}

fn current_text_measure_state<'emit>(
    state: &'emit mut TextRowSourceMeasureState<'_>,
    face_ids: &'emit mut FrameFaceIdAllocator,
) -> DisplayRowCurrentTextSourceState<'emit, 'emit> {
    DisplayRowCurrentTextSourceState::new(
        state.row_output.reborrow(),
        state.evaluator,
        state.font_metrics,
        state.face_resolver,
        face_ids,
    )
}

#[cfg(test)]
#[path = "display_row_extend_fill_test.rs"]
mod extend_fill_tests;
