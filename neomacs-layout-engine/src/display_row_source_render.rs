//! Source-render state facade.
//!
//! This module holds the state types that bridge the typed display-source
//! layer (`DisplayItemSource`) to the row renderer and matrix builder.  It
//! lives between `display_source.rs` / `display_row.rs` and the append-layer
//! helpers in `display_row_append.rs`, so that the append module does not
//! need to own the render-state facade.

use crate::composition::last_text_cluster_tail_in_row;
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_face_policy::BaseFacePolicy;
use crate::display_origin::DisplayOrigin;
use crate::display_row::{
    CurrentTextRowRenderOutcome, DisplayRowActiveFaceState, DisplayRowFallbackMetrics,
    DisplayRowMeasurementPolicy, DisplayRowRenderContext, DisplayRowRenderExecutor,
    DisplayRowRenderIntoRowResult, DisplayRowRenderPolicy, DisplayRowRenderer,
    DisplayRowResolvedMeasuredFace, DisplayRowSourceFragmentRenderRequest,
    DisplayRowSourceRenderRequest, DisplayRowSourceState, display_row_output_end_position,
};
use crate::display_row_builder::merge_display_row_source_slot_bounds;
use crate::display_row_matrix_install::{
    RenderedDisplayRowAssetsInstall, install_resolved_display_row_face,
};
use crate::display_row_replacement::{
    DisplayPropertyReplacementAppendPlan, DisplayPropertyReplacementAppendRequest,
};
use crate::display_source::DisplayItemSource;
use crate::display_source_resolver::{
    ActiveDisplayStringBaseFace, DisplayDefaultFaceInstallPolicy, DisplayStringBaseFace,
    resolve_display_string_base_face,
};
use crate::font_metrics::FontMetricsService;
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::neovm_bridge::{FaceResolver, LayoutBufferView, ResolvedFace};
use crate::window_output::{
    TextMatrixRowGeometryTransition, TextMatrixRowMetrics, TextMatrixRowTransition, TextRowOutput,
    TextWindowOutputRenderState, TextWindowRowDecorationRequest, TextWindowRowLifecycleInstaller,
    WindowOutputEmitter,
};
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neovm_core::emacs_core::Context;
use neovm_core::emacs_core::eval::DisplayHost;
use neovm_core::window::DisplayRowSnapshot;

pub(crate) struct TextRowOutputRenderState<'a> {
    builder: &'a mut GlyphMatrixBuilder,
    output_emitter: &'a mut WindowOutputEmitter,
    evaluator: &'a mut Context,
}

struct DisplayRowCurrentRowSurface<'a> {
    builder: &'a mut GlyphMatrixBuilder,
}

struct DisplayRowCurrentTextRenderState<'face, 'emit> {
    row_surface: DisplayRowCurrentRowSurface<'emit>,
    evaluator: &'emit mut Context,
    font_metrics: &'emit mut Option<FontMetricsService>,
    face_resolver: &'face FaceResolver,
    face_ids: &'emit mut FrameFaceIdAllocator,
}

struct DisplayRowCurrentTextMeasureState<'face, 'emit> {
    row_surface: DisplayRowCurrentRowSurface<'emit>,
    evaluator: &'emit mut Context,
    font_metrics: &'emit mut Option<FontMetricsService>,
    face_resolver: &'face FaceResolver,
    face_ids: &'emit mut FrameFaceIdAllocator,
}

struct DisplayRowCurrentSourceFragmentRenderState<'face, 'emit> {
    builder: &'emit mut GlyphMatrixBuilder,
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

pub(crate) fn current_display_row_cluster_tail(
    builder: &GlyphMatrixBuilder,
) -> Option<(char, bool)> {
    builder
        .current_row_for_render()
        .and_then(last_text_cluster_tail_in_row)
}

impl<'a> DisplayRowCurrentRowSurface<'a> {
    fn new(builder: &'a mut GlyphMatrixBuilder) -> Self {
        Self { builder }
    }

    fn edit_current_row<R>(
        &mut self,
        f: impl FnOnce(&mut neomacs_display_protocol::glyph_matrix::GlyphRow) -> R,
    ) -> Option<R> {
        self.builder.row_installer().edit_current_row(f)
    }

    fn current_row_snapshot(&self) -> Option<neomacs_display_protocol::glyph_matrix::GlyphRow> {
        self.builder.current_row_for_render().cloned()
    }

    fn merge_source_slot_bounds(
        &mut self,
        slots: &[crate::display_row_builder::DisplayRowGlyphSlot],
    ) {
        let _ = self.edit_current_row(|row| {
            merge_display_row_source_slot_bounds(row, slots);
        });
    }

    fn render_source_step_with_policy<S, P>(
        &mut self,
        row_request: DisplayRowSourceRenderRequest<'_>,
        renderer: &mut DisplayRowRenderer<'_>,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        context: &mut DisplayRowRenderContext<'_, '_>,
        render_policy: &mut P,
    ) -> Option<(DisplayRowRenderIntoRowResult, f32, f32)>
    where
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    {
        self.edit_current_row(|row| {
            let result = row_request.render_fragment_step_into_row_with_policy(
                renderer,
                row,
                source,
                source_state,
                context,
                render_policy,
            )?;
            Some((result, row.height_px, row.ascent_px))
        })?
    }

    fn measure_source_step_with_policy<S, P>(
        &mut self,
        row_request: DisplayRowSourceRenderRequest<'_>,
        renderer: &mut DisplayRowRenderer<'_>,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        context: &mut DisplayRowRenderContext<'_, '_>,
        render_policy: &mut P,
    ) -> Option<(DisplayRowRenderIntoRowResult, f32, f32)>
    where
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    {
        let mut scratch_row = self.current_row_snapshot()?;
        let result = row_request.render_fragment_step_into_row_with_policy(
            renderer,
            &mut scratch_row,
            source,
            source_state,
            context,
            render_policy,
        )?;
        Some((result, scratch_row.height_px, scratch_row.ascent_px))
    }

    fn render_natural_source_fragment_step<S>(
        &mut self,
        request: DisplayRowSourceFragmentRenderRequest<'_>,
        render_executor: &mut DisplayRowRenderExecutor<'_, '_, '_>,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
    ) -> Option<DisplayRowRenderIntoRowResult>
    where
        S: DisplayItemSource,
    {
        self.edit_current_row(|row| {
            render_executor.render_item_source_fragment_into_row(request, row, source, source_state)
        })?
    }
}

impl<'face, 'emit> DisplayRowCurrentTextRenderState<'face, 'emit> {
    fn new(
        builder: &'emit mut GlyphMatrixBuilder,
        evaluator: &'emit mut Context,
        font_metrics: &'emit mut Option<FontMetricsService>,
        face_resolver: &'face FaceResolver,
        face_ids: &'emit mut FrameFaceIdAllocator,
    ) -> Self {
        Self {
            row_surface: DisplayRowCurrentRowSurface::new(builder),
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
        let mut renderer = DisplayRowRenderer::new(self.font_metrics);
        let mut context = DisplayRowRenderContext::new(
            self.face_resolver,
            self.evaluator.display_host.as_deref(),
            self.face_ids,
        );
        let (result, row_height_px, row_ascent_px) =
            self.row_surface.render_source_step_with_policy(
                row_request,
                &mut renderer,
                source,
                source_state,
                &mut context,
                render_policy,
            )?;
        self.row_surface
            .merge_source_slot_bounds(&result.source_slots);
        Some(DisplayRowCurrentTextSourceStepResult {
            role,
            result,
            row_height_px,
            row_ascent_px,
        })
    }
}

impl<'face, 'emit> DisplayRowCurrentTextMeasureState<'face, 'emit> {
    fn new(
        builder: &'emit mut GlyphMatrixBuilder,
        evaluator: &'emit mut Context,
        font_metrics: &'emit mut Option<FontMetricsService>,
        face_resolver: &'face FaceResolver,
        face_ids: &'emit mut FrameFaceIdAllocator,
    ) -> Self {
        Self {
            row_surface: DisplayRowCurrentRowSurface::new(builder),
            evaluator,
            font_metrics,
            face_resolver,
            face_ids,
        }
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
        let mut renderer = DisplayRowRenderer::new(self.font_metrics);
        let mut context = DisplayRowRenderContext::new(
            self.face_resolver,
            self.evaluator.display_host.as_deref(),
            self.face_ids,
        );
        let (result, row_height_px, row_ascent_px) =
            self.row_surface.measure_source_step_with_policy(
                row_request,
                &mut renderer,
                source,
                source_state,
                &mut context,
                render_policy,
            )?;
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
        builder: &'emit mut GlyphMatrixBuilder,
        font_metrics: &'emit mut Option<FontMetricsService>,
        face_resolver: &'face FaceResolver,
        display_host: Option<&'emit dyn DisplayHost>,
        face_ids: &'emit mut FrameFaceIdAllocator,
    ) -> Self {
        Self {
            builder,
            font_metrics,
            face_resolver,
            display_host,
            face_ids,
        }
    }

    fn render_natural_fragment_into_current_row<S: DisplayItemSource>(
        &mut self,
        request: DisplayRowSourceFragmentRenderRequest<'_>,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        let mut render_executor = DisplayRowRenderExecutor::new(
            self.font_metrics,
            self.face_resolver,
            self.display_host,
            self.face_ids,
        );
        let mut surface = DisplayRowCurrentRowSurface::new(self.builder);
        let result = surface.render_natural_source_fragment_step(
            request,
            &mut render_executor,
            source,
            source_state,
        )?;
        surface.merge_source_slot_bounds(&result.source_slots);
        Some(result)
    }
}

impl DisplayRowCurrentTextSourceStepResult {
    fn into_measure_outcome(self) -> CurrentTextRowRenderOutcome {
        let end = display_row_output_end_position(self.result.progress);
        let source_slots = self.result.source_slots;
        CurrentTextRowRenderOutcome {
            stop: self.result.stop,
            source_slots,
            end,
            row_height_px: self.row_height_px,
            row_ascent_px: self.row_ascent_px,
        }
    }
}

fn render_display_item_source_into_current_text_row<S, P>(
    state: &mut DisplayRowCurrentTextRenderState<'_, '_>,
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
    state: &mut DisplayRowCurrentTextMeasureState<'_, '_>,
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
    pub(crate) fn new(
        builder: &'a mut GlyphMatrixBuilder,
        output_emitter: &'a mut WindowOutputEmitter,
        evaluator: &'a mut Context,
    ) -> Self {
        Self {
            builder,
            output_emitter,
            evaluator,
        }
    }

    pub(crate) fn reborrow(&mut self) -> TextRowOutputRenderState<'_> {
        TextRowOutputRenderState {
            builder: self.builder,
            output_emitter: self.output_emitter,
            evaluator: self.evaluator,
        }
    }

    pub(crate) fn with_text_window_output<R>(
        self,
        f: impl FnOnce(&mut TextWindowOutputRenderState<'_, '_>, &mut Context) -> R,
    ) -> R {
        let mut output = TextWindowOutputRenderState::new(self.builder, self.output_emitter);
        f(&mut output, self.evaluator)
    }

    pub(crate) fn finish_and_end_text_matrix_row_output(self, metrics: TextMatrixRowMetrics) {
        TextWindowRowLifecycleInstaller::new(self.builder, self.output_emitter, self.evaluator)
            .finish_and_end_row(metrics);
    }

    pub(crate) fn emit_text_matrix_row_transition(
        self,
        transition: TextMatrixRowGeometryTransition,
    ) {
        TextWindowRowLifecycleInstaller::new(self.builder, self.output_emitter, self.evaluator)
            .transition(transition);
    }

    pub(crate) fn emit_text_matrix_row_transition_with_limit(
        self,
        transition: TextMatrixRowGeometryTransition,
        max_rows: usize,
    ) -> TextMatrixRowTransition {
        TextWindowRowLifecycleInstaller::new(self.builder, self.output_emitter, self.evaluator)
            .transition_with_limit(transition, max_rows)
    }

    pub(crate) fn install_row_decoration(self, request: TextWindowRowDecorationRequest) {
        TextWindowRowLifecycleInstaller::new(self.builder, self.output_emitter, self.evaluator)
            .install_row_decoration(request);
    }

    fn insert_resolved_face(&mut self, face_id: u32, face: &ResolvedFace) {
        install_resolved_display_row_face(self.builder, face_id, face, None);
    }

    fn install_resolved_measured_face(&mut self, face: &DisplayRowResolvedMeasuredFace) {
        install_resolved_display_row_face(
            self.builder,
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
            builder: self.builder,
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
    ) -> DisplayRowCurrentTextRenderState<'emit, 'emit> {
        DisplayRowCurrentTextRenderState::new(
            self.builder,
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
            self.builder,
            font_metrics,
            face_resolver,
            self.evaluator.display_host.as_deref(),
            face_ids,
        )
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
        let end = display_row_output_end_position(result.progress);
        RenderedDisplayRowAssetsInstall::fragment(role, output.row, &result.faces, &result.media)
            .install(self.builder);
        let source_slots = result.source_slots;
        self.output_emitter
            .emit_text_source_slots(self.evaluator, output, &source_slots, end);
        CurrentTextRowRenderOutcome {
            stop: result.stop,
            source_slots,
            end,
            row_height_px,
            row_ascent_px,
        }
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

    #[cfg(test)]
    pub(crate) fn new(
        builder: &'a mut GlyphMatrixBuilder,
        output_emitter: &'a mut WindowOutputEmitter,
        evaluator: &'a mut Context,
        font_metrics: &'a mut Option<FontMetricsService>,
        face_resolver: &'a FaceResolver,
    ) -> Self {
        Self::from_output_render(
            TextRowOutputRenderState::new(builder, output_emitter, evaluator),
            font_metrics,
            face_resolver,
        )
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
            self.insert_resolved_face(pending_face.face_id, &pending_face.resolved);
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

    pub(crate) fn display_property_replacement_append_plan<B: LayoutBufferView>(
        &mut self,
        request: DisplayPropertyReplacementAppendRequest,
        buffer: &B,
        active_face_state: &DisplayRowActiveFaceState,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> DisplayPropertyReplacementAppendPlan {
        request.into_plan(buffer, self, active_face_state, face_ids)
    }

    pub(crate) fn render_natural_fragment_into_current_row<S: DisplayItemSource>(
        &mut self,
        request: DisplayRowSourceFragmentRenderRequest<'_>,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        self.output_render
            .current_source_fragment_render_state(self.font_metrics, self.face_resolver, face_ids)
            .render_natural_fragment_into_current_row(request, source, source_state)
    }

    pub(crate) fn render_display_item_source_into_current_text_row_and_emit<
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    >(
        &mut self,
        face_ids: &mut FrameFaceIdAllocator,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        request: DisplayRowSourceRenderRequest<'_>,
        output: TextRowOutput,
        render_policy: &mut P,
    ) -> Option<CurrentTextRowRenderOutcome> {
        let result = render_display_item_source_into_current_text_row(
            &mut current_text_render_state(self, face_ids),
            source,
            source_state,
            request,
            render_policy,
        )?;
        Some(
            self.output_render
                .finish_current_text_row_render(output, result),
        )
    }

    pub(crate) fn mark_current_text_row_truncated_left(&mut self) {
        self.output_render()
            .install_row_decoration(TextWindowRowDecorationRequest::MarkCurrentTruncatedLeft);
    }

    pub(crate) fn with_font_metrics_and_display_host<R>(
        &mut self,
        f: impl FnOnce(&mut Option<FontMetricsService>, Option<&dyn DisplayHost>) -> R,
    ) -> R {
        f(self.font_metrics, self.output_render.display_host())
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
) -> DisplayRowCurrentTextRenderState<'emit, 'emit> {
    state
        .output_render
        .current_text_render_state(state.font_metrics, state.face_resolver, face_ids)
}

pub(crate) struct TextRowSourceMeasureState<'a> {
    builder: &'a mut GlyphMatrixBuilder,
    evaluator: &'a mut Context,
    font_metrics: &'a mut Option<FontMetricsService>,
    face_resolver: &'a FaceResolver,
}

impl<'a> TextRowSourceMeasureState<'a> {
    #[cfg(test)]
    pub(crate) fn new(
        builder: &'a mut GlyphMatrixBuilder,
        evaluator: &'a mut Context,
        font_metrics: &'a mut Option<FontMetricsService>,
        face_resolver: &'a FaceResolver,
    ) -> Self {
        Self {
            builder,
            evaluator,
            font_metrics,
            face_resolver,
        }
    }

    pub(crate) fn font_metrics(&mut self) -> &mut Option<FontMetricsService> {
        self.font_metrics
    }

    pub(crate) fn current_cluster_tail(&self) -> Option<(char, bool)> {
        current_display_row_cluster_tail(self.builder)
    }

    pub(crate) fn measure_display_item_source_against_current_text_row<
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    >(
        &mut self,
        face_ids: &mut FrameFaceIdAllocator,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        request: DisplayRowSourceRenderRequest<'_>,
        render_policy: &mut P,
    ) -> Option<CurrentTextRowRenderOutcome> {
        measure_display_item_source_against_current_text_row(
            &mut current_text_measure_state(self, face_ids),
            source,
            source_state,
            request,
            render_policy,
        )
    }
}

fn current_text_measure_state<'emit>(
    state: &'emit mut TextRowSourceMeasureState<'_>,
    face_ids: &'emit mut FrameFaceIdAllocator,
) -> DisplayRowCurrentTextMeasureState<'emit, 'emit> {
    DisplayRowCurrentTextMeasureState::new(
        state.builder,
        state.evaluator,
        state.font_metrics,
        state.face_resolver,
        face_ids,
    )
}
