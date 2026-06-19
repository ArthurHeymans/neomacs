//! Source-render state facade.
//!
//! This module holds the state types that bridge the typed display-source
//! layer (`DisplayItemSource`) to the row renderer and matrix builder.  It
//! lives between `display_source.rs` / `display_row.rs` and the append-layer
//! helpers in `display_row_append.rs`, so that the append module does not
//! need to own the render-state facade.

use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_face_policy::BaseFacePolicy;
use crate::display_origin::DisplayOrigin;
use crate::display_row::{
    CurrentTextRowRenderOutcome, DisplayRowActiveFaceState,
    DisplayRowCurrentSourceFragmentRenderState, DisplayRowCurrentTextMeasureState,
    DisplayRowCurrentTextRenderState, DisplayRowFallbackMetrics, DisplayRowMeasurementPolicy,
    DisplayRowRenderIntoRowResult, DisplayRowRenderPolicy, DisplayRowResolvedMeasuredFace,
    DisplayRowSourceFragmentRenderRequest, DisplayRowSourceRenderRequest, DisplayRowSourceState,
    insert_resolved_display_row_face, measure_display_item_source_against_current_text_row,
    render_display_item_source_into_current_text_row_and_emit,
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
use neovm_core::emacs_core::Context;
use neovm_core::emacs_core::eval::DisplayHost;
use neovm_core::window::DisplayRowSnapshot;

pub(crate) struct TextRowOutputRenderState<'a> {
    builder: &'a mut GlyphMatrixBuilder,
    output_emitter: &'a mut WindowOutputEmitter,
    evaluator: &'a mut Context,
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
}

pub(crate) struct TextRowSourceRenderState<'a> {
    output_render: TextRowOutputRenderState<'a>,
    font_metrics: &'a mut Option<FontMetricsService>,
    face_resolver: &'a FaceResolver,
}

impl<'a> TextRowSourceRenderState<'a> {
    pub(crate) fn new(
        builder: &'a mut GlyphMatrixBuilder,
        output_emitter: &'a mut WindowOutputEmitter,
        evaluator: &'a mut Context,
        font_metrics: &'a mut Option<FontMetricsService>,
        face_resolver: &'a FaceResolver,
    ) -> Self {
        Self {
            output_render: TextRowOutputRenderState::new(builder, output_emitter, evaluator),
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
        TextRowSourceMeasureState {
            builder: self.output_render.builder,
            evaluator: self.output_render.evaluator,
            font_metrics: self.font_metrics,
            face_resolver: self.face_resolver,
        }
    }

    pub(crate) fn insert_resolved_face(&mut self, face_id: u32, face: &ResolvedFace) {
        insert_resolved_display_row_face(self.output_render.builder, face_id, face, None);
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
        resolved_face.install_into(self.output_render.builder);
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
        request.render_natural_fragment_into_current_row(
            &mut DisplayRowCurrentSourceFragmentRenderState::new(
                self.output_render.builder,
                self.font_metrics,
                self.face_resolver,
                self.output_render.evaluator.display_host.as_deref(),
                face_ids,
            ),
            source,
            source_state,
        )
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
        render_display_item_source_into_current_text_row_and_emit(
            &mut current_text_render_state(self, face_ids),
            source,
            source_state,
            request,
            output,
            render_policy,
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
        f(
            self.font_metrics,
            self.output_render.evaluator.display_host.as_deref(),
        )
    }

    pub(crate) fn output_emitter(&mut self) -> &mut WindowOutputEmitter {
        self.output_render.output_emitter
    }

    pub(crate) fn output_rows(&self) -> &[DisplayRowSnapshot] {
        self.output_render.output_emitter.rows()
    }

    pub(crate) fn output_rows_len(&self) -> usize {
        self.output_render.output_emitter.rows().len()
    }
}

fn current_text_render_state<'emit>(
    state: &'emit mut TextRowSourceRenderState<'_>,
    face_ids: &'emit mut FrameFaceIdAllocator,
) -> DisplayRowCurrentTextRenderState<'emit, 'emit> {
    DisplayRowCurrentTextRenderState::new(
        state.output_render.builder,
        state.output_render.output_emitter,
        state.output_render.evaluator,
        state.font_metrics,
        state.face_resolver,
        face_ids,
    )
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
        crate::display_row::current_display_row_cluster_tail(self.builder)
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
