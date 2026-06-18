use crate::display_cursor::{
    CapturedCursorInfo, CapturedCursorPlacement, CapturedCursorSlotWidth, CursorCaptureState,
    capture_cursor_info, display_property_replacement_cursor_info,
    update_cursor_info_for_main_char,
};
use crate::display_face_id::FrameFaceIdAllocator;

use crate::display_buffer_text_render::{
    BufferTextSourceAppendContinuation, BufferTextSourceCharOverflowAction,
    BufferTextSourceCharRenderState, BufferTextSpecialSourceCharOverflowAction,
    BufferTextSpecialSourceCharRenderState, SyntheticTextSource,
};
use crate::display_buffer_text_source::BufferTextDecodedSourceChar;
#[cfg(test)]
use crate::display_face_policy::BaseFacePolicy;
use crate::display_item::{DisplayItem, DisplayItemKind, RenderFaceRef};
#[cfg(test)]
use crate::display_origin::DisplayOrigin;
use crate::display_property::DisplayMediaReplacementProperty;
#[cfg(test)]
use crate::display_row::DisplayRowRenderStop;
#[cfg(test)]
use crate::display_row::append_rendered_display_row_fragment_to_text_row_and_emit;
use crate::display_row::{
    CurrentTextRowRenderOutcome, DisplayRowActiveFaceState, DisplayRowComplexTextRunAdvancePolicy,
    DisplayRowGeometry, DisplayRowMeasuredFaceMetrics, DisplayRowRenderBounds,
    DisplayRowRenderClipBehavior, DisplayRowRenderPolicy, DisplayRowSourceAppendRequest,
    DisplayRowSourceAppendRequestPolicy, DisplayRowSourceState, DisplaySourceAppendRenderPolicy,
    NaturalDisplayRowAppendRenderPolicy,
};
use crate::display_row_builder::{
    DisplayRowAppendProgress, DisplayRowAppendStatus, DisplayRowItemMeasurement,
    DisplayRowPosition, DisplayTabPolicy,
};
use crate::display_row_geometry::{
    DisplayRowBoundaryTarget, DisplayRowFlagKind, DisplayRowFlags, DisplayRowGeometryDefaults,
    DisplayRowGeometryState, DisplayRowHitRange, DisplayRowLimit, DisplayRowMaxX,
    DisplayRowTextPosition, DisplayRowVisibilityLimit, DisplayRowYPositions, DisplayRowYRecording,
};
#[cfg(test)]
use crate::display_row_lisp_string::LispStringSourceId;
use crate::display_row_lisp_string::{DisplayRowPrefixRequest, render_face_ref_id};
use crate::display_row_source_render::{
    TextRowOutputRenderState, TextRowSourceMeasureState, TextRowSourceRenderState,
    current_text_measure_state, current_text_render_state,
};
use crate::display_row_walk_state::{
    BufferTextRowOverflowDecision, FaceScanCheckpoint, HorizontalScrollSkipState,
    LineNumberRenderState, SpecialTextRowOverflowDecision, TextRowTransitionStatePolicy,
    TrailingWhitespaceRenderState, WordWrapRenderState,
};
use crate::display_source::{
    BufferDisplayReplacementSource, BufferDisplayReplacementStringRequest,
    BufferTextSourceAdvancePath, BufferTextSourceAdvanceRequest, BufferTextSourceAppendItem,
    BufferTextSourceChar, BufferTextSourceClusterState, BufferTextSourceItemRequest,
    BufferTextSourceNaturalAdvanceRequest, BufferTextSourceNaturalFallbackAdvance,
    BufferTextSourceRange, BufferTextSourceSpecialDisplayKind, BufferTextSourceTextItemRequest,
    BufferTextSourceTextRequest, BufferTextSpecialSourceCharRequest, DisplayItemSource,
    DisplayPropertyReplacementCursorPolicy, DisplayPropertyReplacementSourceItem,
    DisplayReplacementAppendItem, DisplayReplacementMediaSourceItem,
    DisplayReplacementMediaSourceResolution, DisplayReplacementSourceMappedTextItem,
    DisplayReplacementStretchSourceItem, DisplayReplacementStringSourceItem,
    ResolvedBufferTextSourceAdvance,
};
use crate::display_source_resolver::{
    DisplayStringBaseFace, ResolvedDisplayReplacement, resolve_display_replacement,
};
use crate::display_text_run_measurement::ComplexTextRunAdvanceResolver;
use crate::font_metrics::FontMetricsService;
use crate::hit_test::HitRow;
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::neovm_bridge::{LayoutBufferView, ResolvedFace};
use crate::types::LineWrapMode;
use crate::types::WindowParams;
use crate::window_output::{
    TextMatrixRowGeometryTransition, TextMatrixRowTransition, WindowOutputEmitter,
    current_text_window_cluster_tail, emit_text_matrix_row_transition_with_limit,
};
use neovm_core::buffer::BufferId;
use neovm_core::emacs_core::eval::DisplayHost;
use neovm_core::emacs_core::{Context, Value};

impl ResolvedBufferTextSourceAdvance {
    fn append_render_policy(self) -> DisplaySourceAppendRenderPolicy {
        match self {
            Self::Natural { .. } => DisplaySourceAppendRenderPolicy::natural(),
            Self::Resolved { advance_px } => {
                DisplaySourceAppendRenderPolicy::resolved_advance(advance_px)
            }
        }
    }
}

impl BufferTextSourceTextRequest {
    fn append_render_policy(self) -> DisplaySourceAppendRenderPolicy {
        self.resolved_advance().append_render_policy()
    }

    fn append_request<B: LayoutBufferView + ?Sized>(
        self,
        buffer_id: BufferId,
        buffer: &B,
        face_id: u32,
    ) -> Option<BufferTextSourceRangeItemAppendRequest> {
        buffer_text_source_text_item_append_request(self.source_item(), buffer_id, buffer, face_id)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct BufferTextSourceAdvanceResolver {
    complex_run: ComplexTextRunAdvanceResolver,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct BufferTextRowAppendState {
    advance_resolver: BufferTextSourceAdvanceResolver,
}

impl BufferTextRowAppendState {
    fn advance_resolver(&mut self) -> &mut BufferTextSourceAdvanceResolver {
        &mut self.advance_resolver
    }
}

impl BufferTextSourceNaturalFallbackAdvance {
    fn resolve_to_text_row(
        self,
        font_metrics: &mut Option<FontMetricsService>,
        active_face_state: &DisplayRowActiveFaceState,
        frame: &DisplayRowAppendFrame,
        position: DisplayRowPosition,
        ch: char,
    ) -> f32 {
        match self {
            Self::Tab => {
                frame
                    .geometry
                    .tab_policy
                    .advance_from(position, frame.face_space_width)
                    .pixel_width
            }
            Self::ClusterContinuation => 0.0,
            Self::FaceColumns { columns } => {
                active_face_state.advance_for_columns(font_metrics, ch, columns)
            }
        }
    }
}

impl BufferTextSourceNaturalAdvanceRequest {
    #[allow(clippy::too_many_arguments)]
    fn measure_to_text_row<B: LayoutBufferView + ?Sized>(
        self,
        state: &mut TextRowSourceMeasureState<'_>,
        base_face: &ResolvedFace,
        buffer_id: BufferId,
        buffer: &B,
        face_id: u32,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
    ) -> Option<f32> {
        let append_item = buffer_text_source_text_item_append_request(
            self.source_item(),
            buffer_id,
            buffer,
            face_id,
        )?;
        let kind = append_item.append_kind();
        let item = append_item.into_item();
        let mut render_policy = DisplaySourceAppendRenderPolicy::natural();
        Some(
            DisplayRowSourceAppendOperation::for_single_item(
                &item, base_face, face_id, frame, position, kind,
            )
            .measure_single_item_to_text_row(state, item, &mut render_policy)?
            .metrics
            .width_px,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_to_text_row<B: LayoutBufferView + ?Sized>(
        self,
        state: &mut TextRowSourceMeasureState<'_>,
        buffer_id: BufferId,
        buffer: &B,
        active_face_state: &DisplayRowActiveFaceState,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
    ) -> f32 {
        if let Some(measured_width) = self.measure_to_text_row(
            state,
            active_face_state.resolved_face(),
            buffer_id,
            buffer,
            active_face_state.face_id(),
            frame.clone(),
            position,
        ) {
            return measured_width;
        }

        self.fallback().resolve_to_text_row(
            state.font_metrics(),
            active_face_state,
            &frame,
            position,
            self.source_item().source_char(),
        )
    }
}

pub(crate) struct DisplayRowBoundaryTransitionRequest<'a> {
    target: DisplayRowBoundaryTarget<'a>,
    max_rows: usize,
}

pub(crate) struct DisplayRowLineBreakTransitionRequest<'a> {
    hit_range: DisplayRowHitRange,
    defaults: DisplayRowGeometryDefaults,
    row_base: usize,
    col: usize,
    x: f32,
    line_spacing: f32,
    row_y_recording: DisplayRowYRecording<'a>,
    max_rows: usize,
}

pub(crate) struct DisplayRowTransitionRequestContext<'a> {
    defaults: DisplayRowGeometryDefaults,
    row_base: usize,
    row_y_recording: DisplayRowYRecording<'a>,
    max_rows: usize,
}

pub(crate) struct DisplayRowTextWindowTransitionContext<'a> {
    request_context: DisplayRowTransitionRequestContext<'a>,
}

pub(crate) struct DisplayRowTextWindowEmitContext<'a, 'emit> {
    defaults: DisplayRowGeometryDefaults,
    row_base: usize,
    row_y_positions: &'a mut DisplayRowYPositions,
    max_rows: usize,
    row_geometry: &'emit mut DisplayRowGeometryState,
    row_flags: &'emit mut DisplayRowFlags,
    row_limit: DisplayRowLimit,
    hit_rows: &'emit mut Vec<HitRow>,
    output_render: TextRowOutputRenderState<'emit>,
}

pub(crate) struct DisplayRowTransitionRenderState<'a> {
    prefix_request: &'a mut DisplayRowPrefixRequest,
    has_prefix: bool,
    line_numbers: &'a mut LineNumberRenderState,
    hscroll_skip: &'a mut HorizontalScrollSkipState,
    word_wrap: &'a mut WordWrapRenderState,
    trailing_whitespace: &'a mut TrailingWhitespaceRenderState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DisplayRowTransitionContinuation {
    Continue,
    Exhausted,
    Hidden,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowLineBreakTransitionPlan {
    state_policy: TextRowTransitionStatePolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DisplayRowOverflowTransitionKind {
    Truncation,
    VisualWrap,
}

pub(crate) struct DisplayRowOverflowTransitionRequest<'a> {
    kind: DisplayRowOverflowTransitionKind,
    hit_range: DisplayRowHitRange,
    defaults: DisplayRowGeometryDefaults,
    row_base: usize,
    col: usize,
    x: f32,
    row_y_recording: DisplayRowYRecording<'a>,
    max_rows: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowOverflowTransitionPlan {
    kind: DisplayRowOverflowTransitionKind,
    state_policy: TextRowTransitionStatePolicy,
}

impl<'a> DisplayRowBoundaryTransitionRequest<'a> {
    pub(crate) fn new(target: DisplayRowBoundaryTarget<'a>, max_rows: usize) -> Self {
        Self { target, max_rows }
    }

    pub(crate) fn emit(
        self,
        row_geometry: &mut DisplayRowGeometryState,
        hit_rows: &mut Vec<HitRow>,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
    ) -> TextMatrixRowTransition {
        let geometry_transition =
            row_geometry.finish_boundary_and_record_hit(self.target, hit_rows);
        emit_text_matrix_row_transition_with_limit(
            builder,
            output_emitter,
            evaluator,
            geometry_transition,
            self.max_rows,
        )
    }
}

impl<'a> DisplayRowTransitionRequestContext<'a> {
    pub(crate) fn new(
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        row_y_recording: DisplayRowYRecording<'a>,
        max_rows: usize,
    ) -> Self {
        Self {
            defaults,
            row_base,
            row_y_recording,
            max_rows,
        }
    }

    pub(crate) fn line_break(
        self,
        plan: DisplayRowLineBreakTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
        line_spacing: f32,
    ) -> DisplayRowLineBreakTransitionRequest<'a> {
        plan.request(
            hit_range,
            self.defaults,
            self.row_base,
            position,
            line_spacing,
            self.row_y_recording,
            self.max_rows,
        )
    }

    pub(crate) fn overflow(
        self,
        plan: DisplayRowOverflowTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
    ) -> DisplayRowOverflowTransitionRequest<'a> {
        plan.request(
            hit_range,
            self.defaults,
            self.row_base,
            position,
            self.row_y_recording,
            self.max_rows,
        )
    }
}

impl<'a> DisplayRowTextWindowTransitionContext<'a> {
    pub(crate) fn new(
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        row_y_positions: &'a mut DisplayRowYPositions,
        max_rows: usize,
    ) -> Self {
        Self {
            request_context: DisplayRowTransitionRequestContext::new(
                defaults,
                row_base,
                row_y_positions.recording(),
                max_rows,
            ),
        }
    }

    pub(crate) fn line_break(
        self,
        plan: DisplayRowLineBreakTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
        line_spacing: f32,
    ) -> DisplayRowLineBreakTransitionRequest<'a> {
        self.request_context
            .line_break(plan, hit_range, position, line_spacing)
    }

    pub(crate) fn emit_line_break(
        self,
        plan: DisplayRowLineBreakTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
        line_spacing: f32,
        row_geometry: &mut DisplayRowGeometryState,
        hit_rows: &mut Vec<HitRow>,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
    ) -> TextMatrixRowTransition {
        self.line_break(plan, hit_range, position, line_spacing)
            .emit(row_geometry, hit_rows, builder, output_emitter, evaluator)
    }

    pub(crate) fn overflow(
        self,
        plan: DisplayRowOverflowTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
    ) -> DisplayRowOverflowTransitionRequest<'a> {
        self.request_context.overflow(plan, hit_range, position)
    }

    pub(crate) fn emit_overflow(
        self,
        plan: DisplayRowOverflowTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
        row_geometry: &mut DisplayRowGeometryState,
        row_flags: &mut DisplayRowFlags,
        row_limit: DisplayRowLimit,
        hit_rows: &mut Vec<HitRow>,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
    ) -> TextMatrixRowTransition {
        self.overflow(plan, hit_range, position).emit(
            row_geometry,
            row_flags,
            row_limit,
            hit_rows,
            builder,
            output_emitter,
            evaluator,
        )
    }
}

impl<'a, 'emit> DisplayRowTextWindowEmitContext<'a, 'emit> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        row_y_positions: &'a mut DisplayRowYPositions,
        max_rows: usize,
        row_geometry: &'emit mut DisplayRowGeometryState,
        row_flags: &'emit mut DisplayRowFlags,
        row_limit: DisplayRowLimit,
        hit_rows: &'emit mut Vec<HitRow>,
        output_render: TextRowOutputRenderState<'emit>,
    ) -> Self {
        Self {
            defaults,
            row_base,
            row_y_positions,
            max_rows,
            row_geometry,
            row_flags,
            row_limit,
            hit_rows,
            output_render,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_source_render(
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        row_y_positions: &'a mut DisplayRowYPositions,
        max_rows: usize,
        row_geometry: &'emit mut DisplayRowGeometryState,
        row_flags: &'emit mut DisplayRowFlags,
        row_limit: DisplayRowLimit,
        hit_rows: &'emit mut Vec<HitRow>,
        source_render: &'emit mut TextRowSourceRenderState<'emit>,
    ) -> Self {
        Self::new(
            defaults,
            row_base,
            row_y_positions,
            max_rows,
            row_geometry,
            row_flags,
            row_limit,
            hit_rows,
            source_render.output_render(),
        )
    }

    pub(crate) fn emit_line_break(
        self,
        plan: DisplayRowLineBreakTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
        line_spacing: f32,
    ) -> TextMatrixRowTransition {
        let (builder, output_emitter, evaluator) = self.output_render.into_parts();
        DisplayRowTextWindowTransitionContext::new(
            self.defaults,
            self.row_base,
            self.row_y_positions,
            self.max_rows,
        )
        .emit_line_break(
            plan,
            hit_range,
            position,
            line_spacing,
            self.row_geometry,
            self.hit_rows,
            builder,
            output_emitter,
            evaluator,
        )
    }

    pub(crate) fn emit_line_break_then_row_start(
        self,
        plan: DisplayRowLineBreakTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
        line_spacing: f32,
        render_state: DisplayRowTransitionRenderState<'_>,
        col: &mut usize,
    ) -> TextMatrixRowTransition {
        let transition = self.emit_line_break(plan, hit_range, position, line_spacing);
        if !transition.is_exhausted() {
            render_state.apply_line_break_row_start(plan, col);
        }
        transition
    }

    pub(crate) fn emit_overflow(
        self,
        plan: DisplayRowOverflowTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
    ) -> TextMatrixRowTransition {
        let (builder, output_emitter, evaluator) = self.output_render.into_parts();
        DisplayRowTextWindowTransitionContext::new(
            self.defaults,
            self.row_base,
            self.row_y_positions,
            self.max_rows,
        )
        .emit_overflow(
            plan,
            hit_range,
            position,
            self.row_geometry,
            self.row_flags,
            self.row_limit,
            self.hit_rows,
            builder,
            output_emitter,
            evaluator,
        )
    }

    pub(crate) fn emit_overflow_then_row_start(
        self,
        plan: DisplayRowOverflowTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
        render_state: DisplayRowTransitionRenderState<'_>,
        col: &mut usize,
    ) -> TextMatrixRowTransition {
        let transition = self.emit_overflow(plan, hit_range, position);
        if !transition.is_exhausted() {
            render_state.apply_overflow_row_start(plan, col);
        }
        transition
    }
}

impl<'a> DisplayRowTransitionRenderState<'a> {
    pub(crate) fn new(
        prefix_request: &'a mut DisplayRowPrefixRequest,
        has_prefix: bool,
        line_numbers: &'a mut LineNumberRenderState,
        hscroll_skip: &'a mut HorizontalScrollSkipState,
        word_wrap: &'a mut WordWrapRenderState,
        trailing_whitespace: &'a mut TrailingWhitespaceRenderState,
    ) -> Self {
        Self {
            prefix_request,
            has_prefix,
            line_numbers,
            hscroll_skip,
            word_wrap,
            trailing_whitespace,
        }
    }

    fn apply_state_policy(&mut self, policy: TextRowTransitionStatePolicy) {
        let prefix_action = policy.apply(
            self.line_numbers,
            self.hscroll_skip,
            self.word_wrap,
            self.trailing_whitespace,
        );
        self.prefix_request
            .apply_transition_prefix_action(self.has_prefix, prefix_action);
    }

    pub(crate) fn apply_line_break_row_start(
        self,
        plan: DisplayRowLineBreakTransitionPlan,
        col: &mut usize,
    ) {
        plan.apply_row_start_prefix_state(col, self);
    }

    pub(crate) fn apply_overflow_prefix(self, plan: DisplayRowOverflowTransitionPlan) {
        plan.apply_prefix_state(self);
    }

    pub(crate) fn apply_overflow_row_start(
        self,
        plan: DisplayRowOverflowTransitionPlan,
        col: &mut usize,
    ) {
        plan.apply_row_start_prefix_state(col, self);
    }
}

impl DisplayRowTransitionContinuation {
    pub(crate) fn after_visible_row_transition(
        row_transition: TextMatrixRowTransition,
        row_geometry: &DisplayRowGeometryState,
        row_visibility_limit: DisplayRowVisibilityLimit,
    ) -> Self {
        if row_transition.is_exhausted() {
            Self::Exhausted
        } else if row_geometry.current_row_is_visible(row_visibility_limit) {
            Self::Continue
        } else {
            Self::Hidden
        }
    }

    pub(crate) fn should_break(self) -> bool {
        !matches!(self, Self::Continue)
    }
}

impl DisplayRowLineBreakTransitionPlan {
    fn new(state_policy: TextRowTransitionStatePolicy) -> Self {
        Self { state_policy }
    }

    pub(crate) fn hscroll_line_break() -> Self {
        Self::new(TextRowTransitionStatePolicy::hscroll_line_break())
    }

    pub(crate) fn hidden_line_break() -> Self {
        Self::new(TextRowTransitionStatePolicy::hidden_line_break())
    }

    pub(crate) fn line_break() -> Self {
        Self::new(TextRowTransitionStatePolicy::line_break())
    }

    pub(crate) fn apply_prefix_state(self, mut state: DisplayRowTransitionRenderState<'_>) {
        state.apply_state_policy(self.state_policy);
    }

    pub(crate) fn apply_row_start_prefix_state(
        self,
        col: &mut usize,
        state: DisplayRowTransitionRenderState<'_>,
    ) {
        *col = 0;
        self.apply_prefix_state(state);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn request<'a>(
        self,
        hit_range: DisplayRowHitRange,
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        position: DisplayRowPosition,
        line_spacing: f32,
        row_y_recording: DisplayRowYRecording<'a>,
        max_rows: usize,
    ) -> DisplayRowLineBreakTransitionRequest<'a> {
        DisplayRowLineBreakTransitionRequest::new(
            hit_range,
            defaults,
            row_base,
            position.col,
            position.x_px,
            line_spacing,
            row_y_recording,
            max_rows,
        )
    }
}

impl<'a> DisplayRowLineBreakTransitionRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        hit_range: DisplayRowHitRange,
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        col: usize,
        x: f32,
        line_spacing: f32,
        row_y_recording: DisplayRowYRecording<'a>,
        max_rows: usize,
    ) -> Self {
        Self {
            hit_range,
            defaults,
            row_base,
            col,
            x,
            line_spacing,
            row_y_recording,
            max_rows,
        }
    }

    fn boundary_target(self) -> DisplayRowBoundaryTarget<'a> {
        DisplayRowBoundaryTarget::line_break(
            self.hit_range,
            self.defaults,
            self.row_base,
            self.col,
            self.x,
            self.line_spacing,
            self.row_y_recording,
        )
    }

    pub(crate) fn finish_geometry(
        self,
        row_geometry: &mut DisplayRowGeometryState,
        hit_rows: &mut Vec<HitRow>,
    ) -> TextMatrixRowGeometryTransition {
        row_geometry.finish_boundary_and_record_hit(self.boundary_target(), hit_rows)
    }

    pub(crate) fn emit(
        self,
        row_geometry: &mut DisplayRowGeometryState,
        hit_rows: &mut Vec<HitRow>,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
    ) -> TextMatrixRowTransition {
        let max_rows = self.max_rows;
        DisplayRowBoundaryTransitionRequest::new(self.boundary_target(), max_rows).emit(
            row_geometry,
            hit_rows,
            builder,
            output_emitter,
            evaluator,
        )
    }
}

impl DisplayRowOverflowTransitionPlan {
    fn new(
        kind: DisplayRowOverflowTransitionKind,
        state_policy: TextRowTransitionStatePolicy,
    ) -> Self {
        Self { kind, state_policy }
    }

    pub(crate) fn truncation(state_policy: TextRowTransitionStatePolicy) -> Self {
        Self::new(DisplayRowOverflowTransitionKind::Truncation, state_policy)
    }

    pub(crate) fn visual_wrap(state_policy: TextRowTransitionStatePolicy) -> Self {
        Self::new(DisplayRowOverflowTransitionKind::VisualWrap, state_policy)
    }

    pub(crate) fn apply_prefix_state(self, mut state: DisplayRowTransitionRenderState<'_>) {
        state.apply_state_policy(self.state_policy);
    }

    pub(crate) fn apply_row_start_prefix_state(
        self,
        col: &mut usize,
        state: DisplayRowTransitionRenderState<'_>,
    ) {
        *col = 0;
        self.apply_prefix_state(state);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn request<'a>(
        self,
        hit_range: DisplayRowHitRange,
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        position: DisplayRowPosition,
        row_y_recording: DisplayRowYRecording<'a>,
        max_rows: usize,
    ) -> DisplayRowOverflowTransitionRequest<'a> {
        match self.kind {
            DisplayRowOverflowTransitionKind::Truncation => {
                DisplayRowOverflowTransitionRequest::truncation(
                    hit_range,
                    defaults,
                    row_base,
                    position.col,
                    position.x_px,
                    row_y_recording,
                    max_rows,
                )
            }
            DisplayRowOverflowTransitionKind::VisualWrap => {
                DisplayRowOverflowTransitionRequest::visual_wrap(
                    hit_range,
                    defaults,
                    row_base,
                    position.col,
                    position.x_px,
                    row_y_recording,
                    max_rows,
                )
            }
        }
    }
}

impl<'a> DisplayRowOverflowTransitionRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn truncation(
        hit_range: DisplayRowHitRange,
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        col: usize,
        x: f32,
        row_y_recording: DisplayRowYRecording<'a>,
        max_rows: usize,
    ) -> Self {
        Self {
            kind: DisplayRowOverflowTransitionKind::Truncation,
            hit_range,
            defaults,
            row_base,
            col,
            x,
            row_y_recording,
            max_rows,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn visual_wrap(
        hit_range: DisplayRowHitRange,
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        col: usize,
        x: f32,
        row_y_recording: DisplayRowYRecording<'a>,
        max_rows: usize,
    ) -> Self {
        Self {
            kind: DisplayRowOverflowTransitionKind::VisualWrap,
            hit_range,
            defaults,
            row_base,
            col,
            x,
            row_y_recording,
            max_rows,
        }
    }

    fn boundary_target(self) -> DisplayRowBoundaryTarget<'a> {
        match self.kind {
            DisplayRowOverflowTransitionKind::Truncation => DisplayRowBoundaryTarget::truncation(
                self.hit_range,
                self.defaults,
                self.row_base,
                self.col,
                self.x,
                self.row_y_recording,
            ),
            DisplayRowOverflowTransitionKind::VisualWrap => DisplayRowBoundaryTarget::visual_wrap(
                self.hit_range,
                self.defaults,
                self.row_base,
                self.col,
                self.x,
                self.row_y_recording,
            ),
        }
    }

    pub(crate) fn emit(
        self,
        row_geometry: &mut DisplayRowGeometryState,
        row_flags: &mut DisplayRowFlags,
        row_limit: DisplayRowLimit,
        hit_rows: &mut Vec<HitRow>,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
    ) -> TextMatrixRowTransition {
        match self.kind {
            DisplayRowOverflowTransitionKind::Truncation => {
                row_geometry.mark_current_row_flag_kind(
                    row_flags,
                    DisplayRowFlagKind::Truncated,
                    row_limit,
                );
            }
            DisplayRowOverflowTransitionKind::VisualWrap => {
                row_geometry.mark_current_row_flag_kind(
                    row_flags,
                    DisplayRowFlagKind::Continued,
                    row_limit,
                );
            }
        }
        let kind = self.kind;
        let max_rows = self.max_rows;
        let transition = DisplayRowBoundaryTransitionRequest::new(self.boundary_target(), max_rows)
            .emit(row_geometry, hit_rows, builder, output_emitter, evaluator);
        if kind == DisplayRowOverflowTransitionKind::VisualWrap && !transition.is_exhausted() {
            row_geometry.mark_current_row_flag_kind(
                row_flags,
                DisplayRowFlagKind::Continuation,
                row_limit,
            );
        }
        transition
    }
}

#[derive(Clone, Debug, PartialEq)]
struct BufferTextSourceRangeItemAppendRequest {
    item: DisplayItem,
    append_kind: DisplayRowAppendKind,
}

impl BufferTextSourceRangeItemAppendRequest {
    fn new(item: DisplayItem, append_kind: DisplayRowAppendKind) -> Self {
        Self { item, append_kind }
    }

    fn append_kind(&self) -> DisplayRowAppendKind {
        self.append_kind
    }

    fn into_item(self) -> DisplayItem {
        self.item
    }
}

pub(crate) struct DisplayRowSourceAppendOperation<'face> {
    base_face: &'face ResolvedFace,
    base_face_id: u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    kind: DisplayRowAppendKind,
}

impl<'face> DisplayRowSourceAppendOperation<'face> {
    pub(crate) fn new(
        base_face: &'face ResolvedFace,
        base_face_id: u32,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
        kind: DisplayRowAppendKind,
    ) -> Self {
        Self {
            base_face,
            base_face_id,
            frame,
            position,
            kind,
        }
    }

    fn request(&self) -> DisplayRowSourceAppendRequest<'face> {
        self.frame.source_append_request(
            self.position,
            self.base_face_id,
            self.base_face,
            self.kind,
        )
    }

    fn for_single_item(
        item: &DisplayItem,
        base_face: &'face ResolvedFace,
        fallback_face_id: u32,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
        kind: DisplayRowAppendKind,
    ) -> Self {
        Self::new(
            base_face,
            render_face_ref_id(item.face, fallback_face_id),
            frame,
            position,
            kind,
        )
    }

    fn render_single_item_to_text_row_and_emit<P: DisplayRowRenderPolicy>(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        item: DisplayItem,
        render_policy: &mut P,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let request = self.request();
        let start = request.start_position();
        let mut face_ids = FrameFaceIdAllocator::new(self.base_face_id.saturating_add(1));
        let outcome = request.render_display_item_into_current_text_row_and_emit(
            &mut current_text_render_state(state, &mut face_ids),
            item,
            render_policy,
        )?;
        Some(outcome.into_append_progress_and_position(start))
    }

    fn measure_single_item_to_text_row<P: DisplayRowRenderPolicy>(
        self,
        state: &mut TextRowSourceMeasureState<'_>,
        item: DisplayItem,
        render_policy: &mut P,
    ) -> Option<DisplayRowAppendProgress> {
        let request = self
            .request()
            .with_measurement_bounds(DisplayRowRenderBounds::unbounded_from(self.position));
        let start = request.start_position();
        let mut face_ids = FrameFaceIdAllocator::new(self.base_face_id.saturating_add(1));
        let outcome = request.measure_display_item_against_current_text_row(
            &mut current_text_measure_state(state, &mut face_ids),
            item,
            render_policy,
        )?;
        Some(outcome.into_append_progress(start))
    }

    fn render_source_to_text_row_and_emit<S: DisplayItemSource, P: DisplayRowRenderPolicy>(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        source: S,
        face_ids: &mut FrameFaceIdAllocator,
        render_policy: &mut P,
    ) -> Option<CurrentTextRowRenderOutcome> {
        self.request()
            .render_owned_display_source_into_current_text_row_and_emit(
                &mut current_text_render_state(state, face_ids),
                source,
                render_policy,
            )
    }

    pub(crate) fn render_source_cursor_to_text_row_and_emit<S: DisplayItemSource>(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> Option<CurrentTextRowRenderOutcome> {
        self.request()
            .render_natural_display_source_into_current_text_row_and_emit(
                &mut current_text_render_state(state, face_ids),
                source,
                source_state,
            )
    }
}

fn buffer_text_source_text_item_append_request<B: LayoutBufferView + ?Sized>(
    source_item: BufferTextSourceTextItemRequest,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
) -> Option<BufferTextSourceRangeItemAppendRequest> {
    let append_kind = source_item.append_kind();
    let item = source_item.into_display_item(buffer_id, buffer, RenderFaceRef::FaceId(face_id))?;
    Some(BufferTextSourceRangeItemAppendRequest::new(
        item,
        append_kind,
    ))
}

#[derive(Clone, Copy)]
pub(crate) struct BufferTextRowAppendContext<'source, 'surface, B: LayoutBufferView + ?Sized> {
    buffer: &'source B,
    buffer_id: BufferId,
    append_surface: &'surface DisplayRowAppendSurface,
    active_face: &'source DisplayRowActiveFaceState,
    glyph_y_offset: f32,
    default_row_height: f32,
}

impl<'source, 'surface, B: LayoutBufferView + ?Sized>
    BufferTextRowAppendContext<'source, 'surface, B>
{
    pub(crate) fn new(
        buffer: &'source B,
        buffer_id: BufferId,
        append_surface: &'surface DisplayRowAppendSurface,
        active_face: &'source DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            buffer,
            buffer_id,
            append_surface,
            active_face,
            glyph_y_offset,
            default_row_height,
        }
    }

    fn active_face_context<'row>(
        &self,
        geometry: &'row DisplayRowGeometryState,
    ) -> DisplayRowActiveFaceAppendContext<'row, 'source>
    where
        'surface: 'row,
    {
        DisplayRowActiveFaceAppendContext::new(
            self.append_surface,
            geometry,
            self.active_face,
            self.glyph_y_offset,
            self.default_row_height,
        )
    }

    fn item_active_face(
        &self,
        geometry: &DisplayRowGeometryState,
    ) -> BufferTextItemAppendContext<'source, B> {
        let frame = self.active_face_context(geometry).active_face_frame();
        BufferTextItemAppendContext::new(
            self.buffer,
            self.buffer_id,
            self.active_face.face_id(),
            self.active_face.resolved_face(),
            frame,
        )
    }

    fn measure_item_source_request_width_or_item_fallback_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceMeasureState<'_>,
        source_item: BufferTextSourceItemRequest,
        position: DisplayRowPosition,
    ) -> f32 {
        self.item_active_face(geometry)
            .measure_source_request_width_or_item_fallback_to_text_row(state, source_item, position)
    }

    fn measure_special_source_char_request_width_or_item_fallback_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceMeasureState<'_>,
        request: BufferTextSpecialSourceCharMeasureRequest,
    ) -> f32 {
        let position = request.position();
        self.measure_item_source_request_width_or_item_fallback_to_text_row(
            geometry,
            state,
            request.source_item(),
            position,
        )
    }

    pub(crate) fn prepare_special_source_char_at(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceMeasureState<'_>,
        request: BufferTextSpecialSourceCharRequest,
        position: DisplayRowPosition,
    ) -> BufferTextSpecialSourceCharPreparedAppend {
        let measured_width_px = request.requires_overflow_measurement().then(|| {
            self.measure_special_source_char_request_width_or_item_fallback_to_text_row(
                geometry,
                state,
                request.measure_at(position),
            )
        });
        request.prepared_append_at(position, measured_width_px)
    }

    fn append_item_source_request_to_text_row_and_emit(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        source_item: BufferTextSourceItemRequest,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.item_active_face(geometry)
            .append_source_request_to_text_row_and_emit(state, source_item, position)
    }

    fn append_special_source_char_plan_to_text_row_and_emit(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        plan: BufferTextSpecialSourceCharAppendPlan,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let position = plan.position();
        self.append_item_source_request_to_text_row_and_emit(
            geometry,
            state,
            plan.source_item(),
            position,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_source_advance_request_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        append_state: &mut BufferTextRowAppendState,
        measure_state: &mut TextRowSourceMeasureState<'_>,
        request: BufferTextSourcePositionedAdvanceRequest<'_>,
    ) -> ResolvedBufferTextSourceAdvance {
        let frame = self.active_face_context(geometry).active_face_frame();
        append_state
            .advance_resolver()
            .resolve_source_advance_request_to_text_row(
                measure_state,
                self.buffer_id,
                self.buffer,
                self.active_face,
                frame,
                request,
            )
    }

    fn prepare_source_char_append_plan(
        &self,
        geometry: &DisplayRowGeometryState,
        append_state: &mut BufferTextRowAppendState,
        measure_state: &mut TextRowSourceMeasureState<'_>,
        request: BufferTextSourcePositionedAdvanceRequest<'_>,
    ) -> BufferTextSourceCharAppendPlan {
        let resolved_advance = self.resolve_source_advance_request_to_text_row(
            geometry,
            append_state,
            measure_state,
            request,
        );
        request.append_plan(resolved_advance)
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_text_source_char_at(
        &self,
        geometry: &DisplayRowGeometryState,
        append_state: &mut BufferTextRowAppendState,
        measure_state: &mut TextRowSourceMeasureState<'_>,
        source_char: &BufferTextSourceChar,
        text: &[u8],
        byte_idx: usize,
        position: DisplayRowPosition,
        cluster_tail: Option<(char, bool)>,
    ) -> BufferTextSourceCharPreparedAppend {
        let request = source_char.advance_request_at(text, byte_idx, position, cluster_tail);
        BufferTextSourceCharPreparedAppend {
            plan: self.prepare_source_char_append_plan(
                geometry,
                append_state,
                measure_state,
                request,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_source_char_at(
        &self,
        geometry: &DisplayRowGeometryState,
        append_state: &mut BufferTextRowAppendState,
        measure_state: &mut TextRowSourceMeasureState<'_>,
        source_char: &BufferTextSourceChar,
        text: &[u8],
        byte_idx: usize,
        position: DisplayRowPosition,
        cluster_tail: Option<(char, bool)>,
    ) -> BufferTextPreparedSourceCharAppend {
        if let Some(request) = source_char.special_request(cluster_tail) {
            return BufferTextPreparedSourceCharAppend::Special(
                self.prepare_special_source_char_at(geometry, measure_state, request, position),
            );
        }
        BufferTextPreparedSourceCharAppend::Text(self.prepare_text_source_char_at(
            geometry,
            append_state,
            measure_state,
            source_char,
            text,
            byte_idx,
            position,
            cluster_tail,
        ))
    }

    pub(crate) fn prepare_source_char_for_current_text_row(
        &self,
        request: BufferTextSourceCharPreparationRequest<'_>,
        state: &mut BufferTextSourceCharPreparationState<'_>,
    ) -> BufferTextPreparedSourceCharAppend {
        let cluster_tail = current_text_window_cluster_tail(state.measure.builder);
        self.prepare_source_char_at(
            &request.geometry,
            state.append_state,
            &mut state.measure,
            request.source_char,
            request.text,
            request.byte_idx,
            request.position,
            cluster_tail,
        )
    }

    fn append_source_text_request_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        source_text: BufferTextSourceTextRequest,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let frame = self.active_face_context(geometry).active_face_frame();
        let face_id = self.active_face.face_id();
        let append_item = source_text.append_request(self.buffer_id, self.buffer, face_id)?;
        let kind = append_item.append_kind();
        let item = append_item.into_item();
        let mut render_policy = source_text.append_render_policy();
        DisplayRowSourceAppendOperation::for_single_item(
            &item,
            self.active_face.resolved_face(),
            face_id,
            frame,
            position,
            kind,
        )
        .render_single_item_to_text_row_and_emit(state, item, &mut render_policy)
    }

    fn append_source_char_plan_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        plan: BufferTextSourceCharAppendPlan,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.append_source_text_request_to_text_row(
            geometry,
            state,
            plan.source_text(),
            plan.position(),
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BufferTextSourceCharPreparationRequest<'a> {
    geometry: DisplayRowGeometryState,
    source_char: &'a BufferTextSourceChar,
    text: &'a [u8],
    byte_idx: usize,
    position: DisplayRowPosition,
}

impl<'a> BufferTextSourceCharPreparationRequest<'a> {
    pub(crate) fn new(
        geometry: DisplayRowGeometryState,
        source_char: &'a BufferTextSourceChar,
        text: &'a [u8],
        byte_idx: usize,
        position: DisplayRowPosition,
    ) -> Self {
        Self {
            geometry,
            source_char,
            text,
            byte_idx,
            position,
        }
    }
}

pub(crate) struct BufferTextSourceCharPreparationState<'a> {
    append_state: &'a mut BufferTextRowAppendState,
    measure: TextRowSourceMeasureState<'a>,
}

impl<'a> BufferTextSourceCharPreparationState<'a> {
    pub(crate) fn from_source_render(
        append_state: &'a mut BufferTextRowAppendState,
        source_render: &'a mut TextRowSourceRenderState<'_>,
    ) -> Self {
        Self {
            append_state,
            measure: source_render.measure_state(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BufferTextPreparedSourceCharAppend {
    Special(BufferTextSpecialSourceCharPreparedAppend),
    Text(BufferTextSourceCharPreparedAppend),
}

impl BufferTextPreparedSourceCharAppend {
    #[cfg(test)]
    pub(crate) fn into_text(self) -> Option<BufferTextSourceCharPreparedAppend> {
        match self {
            Self::Text(prepared_append) => Some(prepared_append),
            Self::Special(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextSourceCharPreparedAppend {
    plan: BufferTextSourceCharAppendPlan,
}

impl BufferTextSourceCharPreparedAppend {
    fn advance_px(self) -> f32 {
        self.plan.advance_px()
    }

    pub(crate) fn update_cursor_info_for_main_char(
        self,
        target: &mut CursorCaptureState,
        byte_idx: usize,
    ) {
        update_cursor_info_for_main_char(target, byte_idx, self.advance_px());
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn capture_cursor_info_for_main_char_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        geometry: &DisplayRowGeometryState,
        x_px: f32,
        byte_idx: usize,
        col: usize,
        is_tab: bool,
        charpos: i64,
        point_charpos: i64,
    ) {
        if target.is_missing() && charpos == point_charpos {
            capture_cursor_info(
                target,
                self.cursor_info_for_main_char(
                    active_face_state,
                    geometry.text_position(x_px, byte_idx, col),
                    is_tab,
                ),
            );
        }
    }

    pub(crate) fn overflow_decision(
        self,
        ch: char,
        right_edge_px: f32,
        wrap_mode: LineWrapMode,
        word_wrap: WordWrapRenderState,
    ) -> BufferTextRowOverflowDecision {
        BufferTextRowOverflowDecision::for_char(
            ch,
            self.plan.position.x_px,
            self.advance_px(),
            right_edge_px,
            wrap_mode,
            word_wrap,
        )
    }

    pub(crate) fn overflow_action(
        self,
        ch: char,
        right_edge_px: f32,
        wrap_mode: LineWrapMode,
        word_wrap: WordWrapRenderState,
    ) -> BufferTextSourceCharOverflowAction {
        BufferTextSourceCharOverflowAction::for_decision(self.overflow_decision(
            ch,
            right_edge_px,
            wrap_mode,
            word_wrap,
        ))
    }

    fn cursor_slot_width(self) -> CapturedCursorSlotWidth {
        CapturedCursorSlotWidth::Explicit(self.advance_px())
    }

    pub(crate) fn cursor_info_for_main_char(
        self,
        active_face_state: &DisplayRowActiveFaceState,
        position: DisplayRowTextPosition,
        is_tab: bool,
    ) -> CapturedCursorInfo {
        CapturedCursorInfo::from_active_face_state(
            active_face_state,
            CapturedCursorPlacement::from_row_text_position(
                position,
                self.cursor_slot_width(),
                is_tab,
            ),
        )
    }

    pub(crate) fn append_to_text_row<B: LayoutBufferView + ?Sized>(
        self,
        context: &BufferTextRowAppendContext<'_, '_, B>,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
    ) -> Option<BufferTextSourceCharAppendOutcome> {
        let advance_px = self.advance_px();
        let (progress, position) =
            context.append_source_char_plan_to_text_row(geometry, state, self.plan)?;
        Some(BufferTextSourceCharAppendOutcome {
            progress,
            position,
            advance_px,
        })
    }

    pub(crate) fn append_to_text_row_and_apply<B: LayoutBufferView + ?Sized>(
        self,
        context: &BufferTextRowAppendContext<'_, '_, B>,
        geometry: &DisplayRowGeometryState,
        ch: char,
        state: &mut BufferTextSourceCharRenderState<'_>,
    ) -> BufferTextSourceAppendContinuation {
        let Some(outcome) = self.append_to_text_row(context, geometry, &mut state.source_render)
        else {
            return BufferTextSourceAppendContinuation::Stopped;
        };
        outcome.apply_rendered_char_to_walk_state(
            state.trailing_whitespace,
            state.word_wrap,
            ch,
            geometry,
            state.x,
            state.col,
            state.charpos,
        );
        BufferTextSourceAppendContinuation::Rendered
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSourceCharAppendOutcome {
    progress: DisplayRowAppendProgress,
    position: DisplayRowPosition,
    advance_px: f32,
}

impl BufferTextSourceCharAppendOutcome {
    pub(crate) fn apply_to_text_row_state(
        &self,
        trailing_whitespace: &mut TrailingWhitespaceRenderState,
        ch: char,
        geometry: &DisplayRowGeometryState,
        x: &mut f32,
        col: &mut usize,
    ) {
        trailing_whitespace.track_rendered_char(
            ch,
            geometry.start_marker_at_x(self.position.x_px - self.advance_px),
        );
        *x = self.position.x_px;
        *col = self.position.col;
    }

    pub(crate) fn apply_rendered_char_to_walk_state(
        &self,
        trailing_whitespace: &mut TrailingWhitespaceRenderState,
        word_wrap: &mut WordWrapRenderState,
        ch: char,
        geometry: &DisplayRowGeometryState,
        x: &mut f32,
        col: &mut usize,
        charpos: &mut i64,
    ) {
        self.apply_to_text_row_state(trailing_whitespace, ch, geometry, x, col);
        *charpos += 1;
        word_wrap.allow_after_current_char(ch);
    }
}

impl BufferTextSourceAdvanceResolver {
    #[allow(clippy::too_many_arguments)]
    fn resolve_source_advance_request_to_text_row<B: LayoutBufferView + ?Sized>(
        &mut self,
        state: &mut TextRowSourceMeasureState<'_>,
        buffer_id: BufferId,
        buffer: &B,
        active_face_state: &DisplayRowActiveFaceState,
        frame: DisplayRowAppendFrame,
        request: BufferTextSourcePositionedAdvanceRequest<'_>,
    ) -> ResolvedBufferTextSourceAdvance {
        let ch = request.cluster().ch();
        match BufferTextSourceAdvancePath::for_cluster_state(request.cluster()) {
            BufferTextSourceAdvancePath::ResolvedComplexRun => {
                let mut policy = DisplayRowComplexTextRunAdvancePolicy::new(
                    active_face_state,
                    state.font_metrics(),
                );
                let advance_px = self.complex_run.advance_for_char(
                    request.text(),
                    request.byte_idx(),
                    ch,
                    request.cluster().is_cluster_continuation(),
                    &mut policy,
                );
                ResolvedBufferTextSourceAdvance::resolved(advance_px)
            }
            BufferTextSourceAdvancePath::NaturalRenderedSource => {
                let advance_px = BufferTextSourceNaturalAdvanceRequest::for_range_and_cluster(
                    request.range(),
                    request.cluster(),
                )
                .resolve_to_text_row(
                    state,
                    buffer_id,
                    buffer,
                    active_face_state,
                    frame,
                    request.position(),
                );
                ResolvedBufferTextSourceAdvance::natural(advance_px)
            }
        }
    }
}

fn buffer_text_source_item_append_request<B: LayoutBufferView + ?Sized>(
    source_item: BufferTextSourceItemRequest,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
) -> Option<BufferTextSourceRangeItemAppendRequest> {
    let append_kind = source_item.append_kind();
    let item = source_item.into_display_item(buffer_id, buffer, RenderFaceRef::FaceId(face_id))?;
    Some(BufferTextSourceRangeItemAppendRequest::new(
        item,
        append_kind,
    ))
}

impl BufferTextDecodedSourceChar {
    pub(crate) fn record_word_wrap_candidate(
        self,
        word_wrap: &mut WordWrapRenderState,
        output_emitter: &WindowOutputEmitter,
    ) {
        if word_wrap.can_record_candidate(self.ch()) {
            word_wrap.record_candidate(
                self.ch(),
                self.start_byte_idx(),
                self.start_charpos(),
                output_emitter.display_point_len(),
                output_emitter.current_row_display_positions(),
            );
        }
    }
}

impl BufferTextSourceChar {
    fn advance_request_at<'text>(
        &self,
        text: &'text [u8],
        byte_idx: usize,
        position: DisplayRowPosition,
        tail: Option<(char, bool)>,
    ) -> BufferTextSourcePositionedAdvanceRequest<'text> {
        BufferTextSourcePositionedAdvanceRequest::new(
            self.advance_request(text, byte_idx, tail),
            position,
        )
    }
}

impl BufferTextSpecialSourceCharRequest {
    fn append_plan_at(
        &self,
        position: DisplayRowPosition,
    ) -> BufferTextSpecialSourceCharAppendPlan {
        BufferTextSpecialSourceCharAppendPlan {
            source_item: self.source_item_request(),
            position,
        }
    }

    fn prepared_append_at(
        self,
        position: DisplayRowPosition,
        measured_width_px: Option<f32>,
    ) -> BufferTextSpecialSourceCharPreparedAppend {
        BufferTextSpecialSourceCharPreparedAppend {
            kind: self.kind(),
            append_plan: self.append_plan_at(position),
            measured_width_px,
        }
    }

    fn measure_at(
        &self,
        position: DisplayRowPosition,
    ) -> BufferTextSpecialSourceCharMeasureRequest {
        BufferTextSpecialSourceCharMeasureRequest {
            source_item: self.source_item_request(),
            position,
        }
    }
}

impl BufferTextSourceSpecialDisplayKind {
    fn should_allocate_policy_face(self, params: &WindowParams) -> bool {
        match self {
            Self::Control => params.escape_glyph_fg != 0,
            Self::Nobreak => params.nobreak_char_fg != 0,
            Self::Glyphless => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSpecialSourceCharPreparedAppend {
    kind: BufferTextSourceSpecialDisplayKind,
    append_plan: BufferTextSpecialSourceCharAppendPlan,
    measured_width_px: Option<f32>,
}

impl BufferTextSpecialSourceCharPreparedAppend {
    #[cfg(test)]
    pub(crate) fn kind(&self) -> BufferTextSourceSpecialDisplayKind {
        self.kind
    }

    fn prepare_append_policy(
        &self,
        params: &WindowParams,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> BufferTextSpecialSourceCharAppendPolicy {
        if self.kind.should_allocate_policy_face(params) {
            let _ = face_ids.allocate();
        }
        BufferTextSpecialSourceCharAppendPolicy {
            invalidate_face_after_append: self.kind.invalidates_face_after_append(),
        }
    }

    fn measured_width_px(&self) -> Option<f32> {
        self.measured_width_px
    }

    pub(crate) fn overflow_decision(
        &self,
        x_px: f32,
        right_edge_px: f32,
        wrap_mode: LineWrapMode,
    ) -> Option<SpecialTextRowOverflowDecision> {
        Some(SpecialTextRowOverflowDecision::for_width(
            x_px,
            self.measured_width_px()?,
            right_edge_px,
            wrap_mode,
        ))
    }

    pub(crate) fn overflow_action(
        &self,
        x_px: f32,
        right_edge_px: f32,
        wrap_mode: LineWrapMode,
    ) -> Option<BufferTextSpecialSourceCharOverflowAction> {
        Some(BufferTextSpecialSourceCharOverflowAction::for_decision(
            self.overflow_decision(x_px, right_edge_px, wrap_mode)?,
        ))
    }

    pub(crate) fn append_to_text_row<B: LayoutBufferView + ?Sized>(
        self,
        context: &BufferTextRowAppendContext<'_, '_, B>,
        geometry: &DisplayRowGeometryState,
        params: &WindowParams,
        face_ids: &mut FrameFaceIdAllocator,
        state: &mut TextRowSourceRenderState<'_>,
    ) -> Option<BufferTextSpecialSourceCharAppendOutcome> {
        let append_policy = self.prepare_append_policy(params, face_ids);
        let (progress, position) = context.append_special_source_char_plan_to_text_row_and_emit(
            geometry,
            state,
            self.append_plan,
        )?;
        Some(BufferTextSpecialSourceCharAppendOutcome {
            progress,
            position,
            append_policy,
        })
    }

    pub(crate) fn append_to_text_row_and_apply<B: LayoutBufferView + ?Sized>(
        self,
        context: &BufferTextRowAppendContext<'_, '_, B>,
        geometry: &DisplayRowGeometryState,
        params: &WindowParams,
        state: &mut BufferTextSpecialSourceCharRenderState<'_>,
    ) -> BufferTextSourceAppendContinuation {
        let Some(outcome) = self.append_to_text_row(
            context,
            geometry,
            params,
            state.face_ids,
            &mut state.source_render,
        ) else {
            return BufferTextSourceAppendContinuation::Stopped;
        };
        outcome.apply_rendered_special_char_to_walk_state(
            state.face_scan,
            state.word_wrap,
            state.x,
            state.col,
            state.charpos,
        );
        BufferTextSourceAppendContinuation::Rendered
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BufferTextSpecialSourceCharAppendPolicy {
    invalidate_face_after_append: bool,
}

impl BufferTextSpecialSourceCharAppendPolicy {
    fn invalidates_face_after_append(self) -> bool {
        self.invalidate_face_after_append
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSpecialSourceCharAppendOutcome {
    progress: DisplayRowAppendProgress,
    position: DisplayRowPosition,
    append_policy: BufferTextSpecialSourceCharAppendPolicy,
}

impl BufferTextSpecialSourceCharAppendOutcome {
    pub(crate) fn apply_to_text_row_state(
        &self,
        face_scan: &mut FaceScanCheckpoint,
        x: &mut f32,
        col: &mut usize,
    ) {
        if self.append_policy.invalidates_face_after_append() {
            face_scan.invalidate();
        }
        *x = self.position.x_px;
        *col = self.position.col;
    }

    pub(crate) fn apply_rendered_special_char_to_walk_state(
        &self,
        face_scan: &mut FaceScanCheckpoint,
        word_wrap: &mut WordWrapRenderState,
        x: &mut f32,
        col: &mut usize,
        charpos: &mut i64,
    ) {
        self.apply_to_text_row_state(face_scan, x, col);
        *charpos += 1;
        word_wrap.disallow_after_current_char();
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSpecialSourceCharAppendPlan {
    source_item: BufferTextSourceItemRequest,
    position: DisplayRowPosition,
}

impl BufferTextSpecialSourceCharAppendPlan {
    fn position(&self) -> DisplayRowPosition {
        self.position
    }

    fn source_item(&self) -> BufferTextSourceItemRequest {
        self.source_item.clone()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSpecialSourceCharMeasureRequest {
    source_item: BufferTextSourceItemRequest,
    position: DisplayRowPosition,
}

impl BufferTextSpecialSourceCharMeasureRequest {
    fn position(&self) -> DisplayRowPosition {
        self.position
    }

    fn source_item(&self) -> BufferTextSourceItemRequest {
        self.source_item.clone()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BufferTextSourceCharAppendPlan {
    source_text: BufferTextSourceTextRequest,
    position: DisplayRowPosition,
}

impl BufferTextSourceCharAppendPlan {
    fn source_text(self) -> BufferTextSourceTextRequest {
        self.source_text
    }

    fn position(self) -> DisplayRowPosition {
        self.position
    }

    fn advance_px(self) -> f32 {
        self.source_text.advance_px()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BufferTextSourcePositionedAdvanceRequest<'text> {
    source: BufferTextSourceAdvanceRequest<'text>,
    position: DisplayRowPosition,
}

impl<'text> BufferTextSourcePositionedAdvanceRequest<'text> {
    fn new(source: BufferTextSourceAdvanceRequest<'text>, position: DisplayRowPosition) -> Self {
        Self { source, position }
    }

    fn text(self) -> &'text [u8] {
        self.source.text()
    }

    fn byte_idx(self) -> usize {
        self.source.byte_idx()
    }

    fn range(self) -> BufferTextSourceRange {
        self.source.range()
    }

    fn position(self) -> DisplayRowPosition {
        self.position
    }

    fn cluster(self) -> BufferTextSourceClusterState {
        self.source.cluster()
    }

    fn append_plan(
        self,
        resolved_advance: ResolvedBufferTextSourceAdvance,
    ) -> BufferTextSourceCharAppendPlan {
        BufferTextSourceCharAppendPlan {
            source_text: self.source.into_text_request(resolved_advance),
            position: self.position,
        }
    }
}

impl BufferTextSourceAppendItem {
    fn append_kind(&self) -> DisplayRowAppendKind {
        match self {
            Self::ControlChar { .. } => DisplayRowAppendKind::ControlChar,
            Self::SourceMappedText { .. } => DisplayRowAppendKind::SourceMappedText,
            Self::Glyphless { .. } => DisplayRowAppendKind::Glyphless,
        }
    }
}

impl BufferTextSourceTextItemRequest {
    fn append_kind(self) -> DisplayRowAppendKind {
        if self.source_char() == '\t' {
            DisplayRowAppendKind::Tab
        } else {
            DisplayRowAppendKind::SourceText
        }
    }
}

impl BufferTextSourceItemRequest {
    fn append_kind(&self) -> DisplayRowAppendKind {
        self.item().append_kind()
    }
}

pub(crate) struct BufferTextItemAppendContext<'a, B: LayoutBufferView + ?Sized> {
    buffer: &'a B,
    buffer_id: BufferId,
    face_id: u32,
    base_face: &'a ResolvedFace,
    frame: DisplayRowAppendFrame,
}

impl<'a, B: LayoutBufferView + ?Sized> BufferTextItemAppendContext<'a, B> {
    pub(crate) fn new(
        buffer: &'a B,
        buffer_id: BufferId,
        face_id: u32,
        base_face: &'a ResolvedFace,
        frame: DisplayRowAppendFrame,
    ) -> Self {
        Self {
            buffer,
            buffer_id,
            face_id,
            base_face,
            frame,
        }
    }

    fn append_source_request_to_text_row_and_emit(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        source_item: BufferTextSourceItemRequest,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let append_item = buffer_text_source_item_append_request(
            source_item,
            self.buffer_id,
            self.buffer,
            self.face_id,
        )?;
        let kind = append_item.append_kind();
        let item = append_item.into_item();
        let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
        DisplayRowSourceAppendOperation::for_single_item(
            &item,
            self.base_face,
            self.face_id,
            self.frame.clone(),
            position,
            kind,
        )
        .render_single_item_to_text_row_and_emit(state, item, &mut render_policy)
    }

    fn measure_source_request_width_to_text_row(
        &self,
        state: &mut TextRowSourceMeasureState<'_>,
        source_item: BufferTextSourceItemRequest,
        position: DisplayRowPosition,
    ) -> Option<f32> {
        let append_item = buffer_text_source_item_append_request(
            source_item,
            self.buffer_id,
            self.buffer,
            self.face_id,
        )?;
        let kind = append_item.append_kind();
        let item = append_item.into_item();
        let mut render_policy = DisplaySourceAppendRenderPolicy::natural();
        Some(
            DisplayRowSourceAppendOperation::for_single_item(
                &item,
                self.base_face,
                self.face_id,
                self.frame.clone(),
                position,
                kind,
            )
            .measure_single_item_to_text_row(state, item, &mut render_policy)?
            .metrics
            .width_px,
        )
    }

    fn measure_source_request_width_or_item_fallback_to_text_row(
        &self,
        state: &mut TextRowSourceMeasureState<'_>,
        source_item: BufferTextSourceItemRequest,
        position: DisplayRowPosition,
    ) -> f32 {
        let fallback_width = source_item.fallback_width_px(self.frame.geometry.char_width);
        self.measure_source_request_width_to_text_row(state, source_item, position)
            .unwrap_or(fallback_width)
    }
}

pub(crate) struct DisplayReplacementStringItemMeasurer {
    active_face_state: DisplayRowActiveFaceState,
}

impl DisplayRowRenderPolicy for DisplayReplacementStringItemMeasurer {
    fn measurement_for(
        &mut self,
        item: &DisplayItem,
        _face_id: u32,
        font_metrics: &mut Option<FontMetricsService>,
    ) -> DisplayRowItemMeasurement {
        let DisplayItemKind::SourceMappedText(text) = &item.kind else {
            return DisplayRowItemMeasurement::Default;
        };
        DisplayRowItemMeasurement::TextRun(
            self.active_face_state
                .text_run_measurement(font_metrics, text.text.as_ref()),
        )
    }
}

struct DisplayReplacementStringRenderPolicy<'a, M> {
    item_policy: &'a mut M,
}

impl<M: DisplayRowRenderPolicy> DisplayRowRenderPolicy
    for DisplayReplacementStringRenderPolicy<'_, M>
{
    fn stop_before_item(&mut self, item: &DisplayItem) -> bool {
        matches!(item.kind, DisplayItemKind::RowBreak(_))
    }

    fn measurement_for(
        &mut self,
        item: &DisplayItem,
        face_id: u32,
        font_metrics: &mut Option<FontMetricsService>,
    ) -> DisplayRowItemMeasurement {
        self.item_policy
            .measurement_for(item, face_id, font_metrics)
    }

    fn clipped_behavior(&mut self, item: &DisplayItem) -> DisplayRowRenderClipBehavior {
        if matches!(item.kind, DisplayItemKind::SourceMappedText(_)) {
            DisplayRowRenderClipBehavior::Stop
        } else {
            DisplayRowRenderClipBehavior::Continue
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct DisplayReplacementStringSourceAppendRequest {
    position: DisplayRowPosition,
    source: BufferDisplayReplacementStringRequest,
}

impl DisplayReplacementStringSourceAppendRequest {
    fn new(position: DisplayRowPosition, source: BufferDisplayReplacementStringRequest) -> Self {
        Self { position, source }
    }

    fn position(self) -> DisplayRowPosition {
        self.position
    }

    #[cfg(test)]
    fn source_id(self) -> LispStringSourceId {
        LispStringSourceId(self.source.source_id())
    }

    #[cfg(test)]
    fn value(self) -> Value {
        self.source.value()
    }

    fn render_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        append_context: &DisplayReplacementAppendContext<'_>,
        item_policy: &mut impl DisplayRowRenderPolicy,
    ) -> DisplayRowPosition {
        let position = self.position();
        let Some(source) = self.source.into_source(append_context.face_id) else {
            return position;
        };
        let mut render_policy = DisplayReplacementStringRenderPolicy { item_policy };
        let Some(outcome) = DisplayRowSourceAppendOperation::new(
            append_context.base_face,
            append_context.face_id,
            append_context.frame.clone(),
            position,
            DisplayRowAppendKind::DisplayReplacementString,
        )
        .render_source_to_text_row_and_emit(state, source, face_ids, &mut render_policy) else {
            return position;
        };
        outcome.end_position()
    }
}

#[derive(Clone)]
pub(crate) struct DisplayReplacementStringAppendRequest {
    item: DisplayReplacementStringSourceItem,
    replacement_base_face: Option<DisplayStringBaseFace>,
    active_face_state: DisplayRowActiveFaceState,
}

impl DisplayReplacementStringAppendRequest {
    fn new(
        item: DisplayReplacementStringSourceItem,
        replacement_base_face: Option<DisplayStringBaseFace>,
        active_face_state: DisplayRowActiveFaceState,
    ) -> Self {
        Self {
            item,
            replacement_base_face,
            active_face_state,
        }
    }

    #[cfg(test)]
    fn origin(&self) -> DisplayOrigin {
        self.item.origin()
    }

    #[cfg(test)]
    fn base_face_policy(&self) -> BaseFacePolicy {
        self.item.base_face_policy()
    }

    fn string_item_measurer(&self) -> DisplayReplacementStringItemMeasurer {
        DisplayReplacementStringItemMeasurer {
            active_face_state: self.active_face_state.clone(),
        }
    }

    fn source_append_request(
        &self,
        replacement_source: BufferDisplayReplacementSource,
        position: DisplayRowPosition,
    ) -> DisplayReplacementStringSourceAppendRequest {
        DisplayReplacementStringSourceAppendRequest::new(
            position,
            BufferDisplayReplacementStringRequest::new(
                self.item.source_id(),
                self.item.value(),
                replacement_source,
            ),
        )
    }

    fn append_to_text_row(
        self,
        replacement_append_context: DisplayReplacementRowAppendContext<'_>,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        if self.item.is_empty() {
            return position;
        }
        let Some(ref replacement_base_face) = self.replacement_base_face else {
            debug_assert!(false, "display string replacement missing base face");
            return position;
        };
        let source_request =
            self.source_append_request(replacement_append_context.replacement_source, position);
        let mut item_policy = self.string_item_measurer();
        let append_context = replacement_append_context.full_text_width_active_face(
            replacement_base_face.face_id(),
            replacement_base_face.face(),
        );
        source_request.render_to_text_row_and_emit(
            state,
            face_ids,
            &append_context,
            &mut item_policy,
        )
    }
}

#[derive(Clone, Debug)]
struct DisplayReplacementItemAppendRequest {
    item: DisplayReplacementAppendItem,
    frame: DisplayReplacementItemAppendFrame,
    position: DisplayRowPosition,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DisplayReplacementItemAppendFrame {
    ActiveFace,
    DisplayBox { height_px: f32, ascent_px: f32 },
}

impl DisplayReplacementItemAppendRequest {
    fn active_face(item: DisplayReplacementAppendItem, position: DisplayRowPosition) -> Self {
        Self {
            item,
            frame: DisplayReplacementItemAppendFrame::ActiveFace,
            position,
        }
    }

    fn display_box(
        item: DisplayReplacementAppendItem,
        height_px: f32,
        ascent_px: f32,
        position: DisplayRowPosition,
    ) -> Self {
        Self {
            item,
            frame: DisplayReplacementItemAppendFrame::DisplayBox {
                height_px,
                ascent_px,
            },
            position,
        }
    }
}

impl DisplayReplacementStretchSourceItem {
    fn append_request(
        self,
        position: DisplayRowPosition,
    ) -> Option<DisplayReplacementItemAppendRequest> {
        (self.width_px() > 0.0).then(|| {
            DisplayReplacementItemAppendRequest::active_face(
                DisplayReplacementAppendItem::stretch(self.geometry()),
                position,
            )
        })
    }
}

#[derive(Clone)]
pub(crate) struct DisplayPropertyReplacementAppendRequest {
    replacement_source: BufferDisplayReplacementSource,
    item: DisplayPropertyReplacementSourceItem,
    glyph_y_offset: f32,
    default_row_height: f32,
    start_position: DisplayRowPosition,
}

impl DisplayPropertyReplacementAppendRequest {
    pub(crate) fn new(
        replacement_source: BufferDisplayReplacementSource,
        item: DisplayPropertyReplacementSourceItem,
        glyph_y_offset: f32,
        default_row_height: f32,
        start_position: DisplayRowPosition,
    ) -> Self {
        Self {
            replacement_source,
            item,
            glyph_y_offset,
            default_row_height,
            start_position,
        }
    }

    pub(crate) fn cursor_policy(&self) -> DisplayPropertyReplacementCursorPolicy {
        self.item.cursor_policy()
    }

    pub(crate) fn start_position(&self) -> DisplayRowPosition {
        self.start_position
    }

    pub(crate) fn into_plan<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: &mut TextRowSourceRenderState<'_>,
        active_face_state: &DisplayRowActiveFaceState,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> DisplayPropertyReplacementAppendPlan {
        let item = DisplayPropertyReplacementAppendPlanItemRequest::new(self.item).resolve(
            buffer,
            state,
            active_face_state,
            face_ids,
        );
        DisplayPropertyReplacementAppendPlan {
            replacement_source: self.replacement_source,
            item,
            glyph_y_offset: self.glyph_y_offset,
            default_row_height: self.default_row_height,
            start_position: self.start_position,
        }
    }

    #[cfg(test)]
    pub(crate) fn into_item(self) -> DisplayPropertyReplacementSourceItem {
        self.item
    }

    pub(crate) fn append_to_text_row<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        append_surface: &DisplayRowAppendSurface,
        row_geometry: &mut DisplayRowGeometryState,
        active_face_state: &DisplayRowActiveFaceState,
    ) -> DisplayPropertyReplacementAppendOutcome {
        let start_position = self.start_position();
        let cursor_policy = self.cursor_policy();
        let plan = state.display_property_replacement_append_plan(
            self,
            buffer,
            active_face_state,
            face_ids,
        );
        let end_position = plan.append_to_text_row(
            state,
            face_ids,
            append_surface,
            row_geometry,
            active_face_state,
        );
        DisplayPropertyReplacementAppendOutcome {
            start_position,
            end_position,
            cursor_policy,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayPropertyReplacementAppendOutcome {
    start_position: DisplayRowPosition,
    end_position: DisplayRowPosition,
    cursor_policy: DisplayPropertyReplacementCursorPolicy,
}

impl DisplayPropertyReplacementAppendOutcome {
    pub(crate) fn start_position(self) -> DisplayRowPosition {
        self.start_position
    }

    pub(crate) fn end_position(self) -> DisplayRowPosition {
        self.end_position
    }

    pub(crate) fn cursor_info(
        self,
        active_face_state: &DisplayRowActiveFaceState,
        position: DisplayRowTextPosition,
    ) -> CapturedCursorInfo {
        display_property_replacement_cursor_info(self.cursor_policy, active_face_state, position)
    }
}

pub(crate) struct DisplayPropertyReplacementAppendPlan {
    replacement_source: BufferDisplayReplacementSource,
    item: DisplayPropertyReplacementAppendPlanItem,
    glyph_y_offset: f32,
    default_row_height: f32,
    start_position: DisplayRowPosition,
}

impl DisplayPropertyReplacementAppendPlan {
    #[cfg(test)]
    pub(crate) fn string_append_request(&self) -> Option<&DisplayReplacementStringAppendRequest> {
        match &self.item {
            DisplayPropertyReplacementAppendPlanItem::String(request) => Some(request),
            _ => None,
        }
    }

    pub(crate) fn append_to_text_row(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        append_surface: &DisplayRowAppendSurface,
        row_geometry: &mut DisplayRowGeometryState,
        active_face_state: &DisplayRowActiveFaceState,
    ) -> DisplayRowPosition {
        let position = self.start_position;
        let replacement_append_context = DisplayReplacementRowAppendContext::new(
            self.replacement_source,
            append_surface,
            row_geometry,
            active_face_state,
            self.glyph_y_offset,
            self.default_row_height,
        );
        self.item.append_to_text_row(
            replacement_append_context,
            row_geometry,
            state,
            face_ids,
            position,
        )
    }
}

#[derive(Clone)]
enum DisplayPropertyReplacementAppendPlanItem {
    String(DisplayReplacementStringAppendRequest),
    Stretch(DisplayReplacementStretchSourceItem),
    Media(DisplayReplacementMediaSourceResolution),
}

struct DisplayPropertyReplacementAppendPlanItemRequest {
    item: DisplayPropertyReplacementSourceItem,
}

impl DisplayPropertyReplacementAppendPlanItemRequest {
    fn new(item: DisplayPropertyReplacementSourceItem) -> Self {
        Self { item }
    }

    fn resolve<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: &mut TextRowSourceRenderState<'_>,
        active_face_state: &DisplayRowActiveFaceState,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> DisplayPropertyReplacementAppendPlanItem {
        match self.item {
            DisplayPropertyReplacementSourceItem::String(item) => {
                let replacement_base_face = (!item.is_empty()).then(|| {
                    state.default_display_string_base_face_for_active_row(
                        buffer,
                        item.origin(),
                        active_face_state,
                        face_ids,
                    )
                });
                DisplayPropertyReplacementAppendPlanItem::String(
                    DisplayReplacementStringAppendRequest::new(
                        item,
                        replacement_base_face,
                        active_face_state.clone(),
                    ),
                )
            }
            DisplayPropertyReplacementSourceItem::Stretch(item) => {
                DisplayPropertyReplacementAppendPlanItem::Stretch(item)
            }
            DisplayPropertyReplacementSourceItem::Media(item) => {
                DisplayPropertyReplacementAppendPlanItem::Media(item)
            }
        }
    }
}

impl DisplayPropertyReplacementAppendPlanItem {
    fn append_to_text_row(
        self,
        replacement_append_context: DisplayReplacementRowAppendContext<'_>,
        row_geometry: &mut DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        match self {
            Self::String(request) => {
                request.append_to_text_row(replacement_append_context, state, face_ids, position)
            }
            Self::Stretch(stretch_item) => stretch_item.append_to_text_row(
                replacement_append_context,
                row_geometry,
                state,
                position,
            ),
            Self::Media(media_item) => media_item.append_to_text_row(
                replacement_append_context,
                row_geometry,
                state,
                position,
            ),
        }
    }
}

impl DisplayReplacementStretchSourceItem {
    fn append_to_text_row(
        self,
        replacement_append_context: DisplayReplacementRowAppendContext<'_>,
        row_geometry: &mut DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        let Some(request) = self.append_request(position) else {
            return position;
        };
        row_geometry.include_glyph_vertical_metrics(self.height_px(), self.ascent_px());
        replacement_append_context
            .append_item_request_to_text_row_and_emit(state, request)
            .map(|(_progress, position)| position)
            .unwrap_or(position)
    }
}

impl DisplayReplacementMediaSourceResolution {
    fn append_to_text_row(
        self,
        replacement_append_context: DisplayReplacementRowAppendContext<'_>,
        row_geometry: &mut DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        match self {
            Self::Media(media_item) => media_item.append_to_text_row(
                replacement_append_context,
                row_geometry,
                state,
                position,
            ),
            Self::Placeholder(placeholder_item) => {
                placeholder_item.append_to_text_row(replacement_append_context, state, position)
            }
        }
    }
}

impl DisplayReplacementMediaSourceItem {
    fn append_to_text_row(
        self,
        replacement_append_context: DisplayReplacementRowAppendContext<'_>,
        row_geometry: &mut DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        if let Some((progress, appended_position)) = replacement_append_context
            .append_item_request_to_text_row_and_emit(state, self.append_request(position))
            && let Some((height, ascent)) = self.row_extents_after_append(&progress)
        {
            row_geometry.include_row_extents(height, ascent);
            appended_position
        } else {
            position
        }
    }
}

impl DisplayReplacementSourceMappedTextItem {
    fn append_to_text_row(
        self,
        replacement_append_context: DisplayReplacementRowAppendContext<'_>,
        state: &mut TextRowSourceRenderState<'_>,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        replacement_append_context
            .append_item_request_to_text_row_and_emit(state, self.append_request(position))
            .map(|(_progress, position)| position)
            .unwrap_or(position)
    }
}

impl DisplayReplacementMediaSourceItem {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_display_property(
        display_prop: Value,
        replacement: &DisplayMediaReplacementProperty,
        display_host: Option<&dyn DisplayHost>,
        active_face_state: &DisplayRowActiveFaceState,
        fallback_char_width: f32,
        fallback_row_height: f32,
    ) -> Option<DisplayReplacementMediaSourceResolution> {
        match resolve_display_replacement(
            display_prop,
            replacement,
            display_host,
            active_face_state.resolved_face(),
            fallback_char_width,
            fallback_row_height,
        )? {
            ResolvedDisplayReplacement::Media(media) => {
                Some(DisplayReplacementMediaSourceResolution::Media(Self::new(
                    media,
                    active_face_state.metrics().row_height,
                    active_face_state.metrics().ascent,
                    replacement.uses_xwidget_cursor_extents(),
                )))
            }
            ResolvedDisplayReplacement::Placeholder(placeholder) => {
                Some(DisplayReplacementMediaSourceResolution::Placeholder(
                    DisplayReplacementSourceMappedTextItem::new(placeholder),
                ))
            }
        }
    }

    pub(crate) fn row_extents_after_append(
        self,
        progress: &DisplayRowAppendProgress,
    ) -> Option<(f32, f32)> {
        if progress.status == DisplayRowAppendStatus::Complete && progress.metrics.width_px > 0.0 {
            Some((self.display_height_px(), self.display_ascent_px()))
        } else {
            None
        }
    }

    fn append_request(self, position: DisplayRowPosition) -> DisplayReplacementItemAppendRequest {
        DisplayReplacementItemAppendRequest::display_box(
            DisplayReplacementAppendItem::media(self.media()),
            self.display_height_px(),
            self.display_ascent_px(),
            position,
        )
    }
}

impl DisplayReplacementSourceMappedTextItem {
    fn append_request(self, position: DisplayRowPosition) -> DisplayReplacementItemAppendRequest {
        DisplayReplacementItemAppendRequest::active_face(
            DisplayReplacementAppendItem::source_mapped_text(self.into_text()),
            position,
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DisplayReplacementRowAppendContext<'a> {
    replacement_source: BufferDisplayReplacementSource,
    append_surface: &'a DisplayRowAppendSurface,
    placement: DisplayRowAppendPlacement,
    active_face: &'a DisplayRowActiveFaceState,
    default_row_height: f32,
}

impl<'a> DisplayReplacementRowAppendContext<'a> {
    pub(crate) fn new(
        replacement_source: BufferDisplayReplacementSource,
        append_surface: &'a DisplayRowAppendSurface,
        geometry: &DisplayRowGeometryState,
        active_face: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            replacement_source,
            append_surface,
            placement: DisplayRowAppendPlacement::from_geometry_state(geometry, glyph_y_offset),
            active_face,
            default_row_height,
        }
    }

    fn active_face_frame(self) -> DisplayRowAppendFrame {
        self.append_surface.frame(
            self.placement,
            DisplayRowAppendMetrics::from_active_face_state(
                self.active_face,
                self.default_row_height,
            ),
        )
    }

    fn full_text_width_active_face_frame(self) -> DisplayRowAppendFrame {
        self.append_surface.full_text_width_surface().frame(
            self.placement,
            DisplayRowAppendMetrics::from_active_face_state(
                self.active_face,
                self.default_row_height,
            ),
        )
    }

    fn display_box_frame(self, height_px: f32, ascent_px: f32) -> DisplayRowAppendFrame {
        self.append_surface.frame(
            self.placement,
            DisplayRowAppendMetrics::display_box_from_active_face_state(
                self.active_face,
                height_px,
                ascent_px,
                self.default_row_height,
            ),
        )
    }

    fn active_face(
        self,
        face_id: u32,
        base_face: &'a ResolvedFace,
    ) -> DisplayReplacementAppendContext<'a> {
        DisplayReplacementAppendContext::new(
            self.replacement_source,
            face_id,
            base_face,
            self.active_face_frame(),
        )
    }

    fn full_text_width_active_face(
        self,
        face_id: u32,
        base_face: &'a ResolvedFace,
    ) -> DisplayReplacementAppendContext<'a> {
        DisplayReplacementAppendContext::new(
            self.replacement_source,
            face_id,
            base_face,
            self.full_text_width_active_face_frame(),
        )
    }

    fn display_box(
        self,
        face_id: u32,
        base_face: &'a ResolvedFace,
        height_px: f32,
        ascent_px: f32,
    ) -> DisplayReplacementAppendContext<'a> {
        DisplayReplacementAppendContext::new(
            self.replacement_source,
            face_id,
            base_face,
            self.display_box_frame(height_px, ascent_px),
        )
    }

    fn append_item_request_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        request: DisplayReplacementItemAppendRequest,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let DisplayReplacementItemAppendRequest {
            item,
            frame,
            position,
        } = request;
        let append_context = match frame {
            DisplayReplacementItemAppendFrame::ActiveFace => {
                self.active_face(self.active_face.face_id(), self.active_face.resolved_face())
            }
            DisplayReplacementItemAppendFrame::DisplayBox {
                height_px,
                ascent_px,
            } => self.display_box(
                self.active_face.face_id(),
                self.active_face.resolved_face(),
                height_px,
                ascent_px,
            ),
        };
        append_context.append_replacement_item_to_text_row_and_emit(state, item, position)
    }
}

#[derive(Clone)]
pub(crate) struct DisplayReplacementAppendContext<'a> {
    replacement_source: BufferDisplayReplacementSource,
    face_id: u32,
    base_face: &'a ResolvedFace,
    frame: DisplayRowAppendFrame,
}

impl<'a> DisplayReplacementAppendContext<'a> {
    pub(crate) fn new(
        replacement_source: BufferDisplayReplacementSource,
        face_id: u32,
        base_face: &'a ResolvedFace,
        frame: DisplayRowAppendFrame,
    ) -> Self {
        Self {
            replacement_source,
            face_id,
            base_face,
            frame,
        }
    }

    fn append_replacement_item_to_text_row_and_emit(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        item: DisplayReplacementAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let item = item.into_display_item(self.replacement_source, self.face_id);
        let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
        DisplayRowSourceAppendOperation::for_single_item(
            &item,
            self.base_face,
            self.face_id,
            self.frame.clone(),
            position,
            DisplayRowAppendKind::DisplayReplacement,
        )
        .render_single_item_to_text_row_and_emit(state, item, &mut render_policy)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DisplayRowAppendPlacement {
    row: usize,
    y: f32,
    glyph_y: f32,
}

impl DisplayRowAppendPlacement {
    fn new(row: usize, y: f32, glyph_y: f32) -> Self {
        Self { row, y, glyph_y }
    }

    fn from_geometry_state(geometry: &DisplayRowGeometryState, glyph_y_offset: f32) -> Self {
        Self::new(
            geometry.row(),
            geometry.y(),
            geometry.glyph_y(glyph_y_offset),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowAppendArea {
    content_x: f32,
    width: f32,
    text_width: f32,
    line_number_width: f32,
}

impl DisplayRowAppendArea {
    pub(crate) fn new(content_x: f32, width: f32, text_width: f32, line_number_width: f32) -> Self {
        Self {
            content_x,
            width,
            text_width,
            line_number_width,
        }
    }

    pub(crate) fn content_x(self) -> f32 {
        self.content_x
    }

    pub(crate) fn right_edge(self) -> f32 {
        self.content_x + self.width
    }

    fn full_text_width(self) -> Self {
        Self {
            width: (self.text_width - self.line_number_width).max(0.0),
            ..self
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayRowAppendSurface {
    area: DisplayRowAppendArea,
    tab_policy: DisplayTabPolicy,
}

impl DisplayRowAppendSurface {
    pub(crate) fn new(area: DisplayRowAppendArea, tab_policy: DisplayTabPolicy) -> Self {
        Self { area, tab_policy }
    }

    pub(crate) fn content_x(&self) -> f32 {
        self.area.content_x()
    }

    pub(crate) fn right_edge(&self) -> f32 {
        self.area.right_edge()
    }

    pub(crate) fn full_text_right_edge(&self) -> f32 {
        self.area.full_text_width().right_edge()
    }

    pub(crate) fn full_text_width_surface(&self) -> Self {
        Self {
            area: self.area.full_text_width(),
            tab_policy: self.tab_policy.clone(),
        }
    }

    fn frame(
        &self,
        placement: DisplayRowAppendPlacement,
        metrics: DisplayRowAppendMetrics,
    ) -> DisplayRowAppendFrame {
        DisplayRowAppendFrame::from_parts(placement, self.area, metrics, self.tab_policy.clone())
    }

    fn frame_from_geometry_state(
        &self,
        geometry: &DisplayRowGeometryState,
        glyph_y_offset: f32,
        metrics: DisplayRowAppendMetrics,
    ) -> DisplayRowAppendFrame {
        self.frame(
            DisplayRowAppendPlacement::from_geometry_state(geometry, glyph_y_offset),
            metrics,
        )
    }

    fn text_row_frame_from_geometry_state(
        &self,
        geometry: &DisplayRowGeometryState,
        glyph_y_offset: f32,
        height: f32,
        ascent: f32,
        char_width: f32,
        default_row_height: f32,
    ) -> DisplayRowAppendFrame {
        self.frame_from_geometry_state(
            geometry,
            glyph_y_offset,
            DisplayRowAppendMetrics::text_row(height, ascent, char_width, default_row_height),
        )
    }

    fn frame_for_active_face_from_geometry_state(
        &self,
        geometry: &DisplayRowGeometryState,
        glyph_y_offset: f32,
        active_face: &DisplayRowActiveFaceState,
        default_row_height: f32,
    ) -> DisplayRowAppendFrame {
        self.frame_from_geometry_state(
            geometry,
            glyph_y_offset,
            DisplayRowAppendMetrics::from_active_face_state(active_face, default_row_height),
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DisplayRowTextAppendContext<'a> {
    append_surface: &'a DisplayRowAppendSurface,
    geometry: &'a DisplayRowGeometryState,
    glyph_y_offset: f32,
    default_row_height: f32,
}

impl<'a> DisplayRowTextAppendContext<'a> {
    pub(crate) fn new(
        append_surface: &'a DisplayRowAppendSurface,
        geometry: &'a DisplayRowGeometryState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            append_surface,
            geometry,
            glyph_y_offset,
            default_row_height,
        }
    }

    pub(crate) fn text_row_frame(
        self,
        height: f32,
        ascent: f32,
        char_width: f32,
    ) -> DisplayRowAppendFrame {
        self.append_surface.text_row_frame_from_geometry_state(
            self.geometry,
            self.glyph_y_offset,
            height,
            ascent,
            char_width,
            self.default_row_height,
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DisplayRowActiveFaceAppendContext<'row, 'face> {
    text_context: DisplayRowTextAppendContext<'row>,
    active_face: &'face DisplayRowActiveFaceState,
}

impl<'row, 'face> DisplayRowActiveFaceAppendContext<'row, 'face> {
    pub(crate) fn new(
        append_surface: &'row DisplayRowAppendSurface,
        geometry: &'row DisplayRowGeometryState,
        active_face: &'face DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            text_context: DisplayRowTextAppendContext::new(
                append_surface,
                geometry,
                glyph_y_offset,
                default_row_height,
            ),
            active_face,
        }
    }

    pub(crate) fn active_face_frame(self) -> DisplayRowAppendFrame {
        self.text_context
            .append_surface
            .frame_for_active_face_from_geometry_state(
                self.text_context.geometry,
                self.text_context.glyph_y_offset,
                self.active_face,
                self.text_context.default_row_height,
            )
    }

    pub(crate) fn active_face(self) -> &'face DisplayRowActiveFaceState {
        self.active_face
    }

    #[cfg(test)]
    pub(crate) fn full_text_width_active_face_frame(self) -> DisplayRowAppendFrame {
        self.text_context
            .append_surface
            .full_text_width_surface()
            .frame_for_active_face_from_geometry_state(
                self.text_context.geometry,
                self.text_context.glyph_y_offset,
                self.active_face,
                self.text_context.default_row_height,
            )
    }

    pub(crate) fn text_row_frame(
        self,
        height: f32,
        ascent: f32,
        char_width: f32,
    ) -> DisplayRowAppendFrame {
        self.text_context.text_row_frame(height, ascent, char_width)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowAppendMetrics {
    pub(crate) height: f32,
    pub(crate) ascent: f32,
    pub(crate) char_width: f32,
    space_width: f32,
    pub(crate) default_row_height: f32,
}

impl DisplayRowAppendMetrics {
    fn new(
        height: f32,
        ascent: f32,
        char_width: f32,
        space_width: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            height,
            ascent,
            char_width,
            space_width,
            default_row_height,
        }
    }

    pub(crate) fn text_row(
        height: f32,
        ascent: f32,
        char_width: f32,
        default_row_height: f32,
    ) -> Self {
        Self::new(height, ascent, char_width, char_width, default_row_height)
    }

    pub(crate) fn from_active_face_state(
        active_face: &DisplayRowActiveFaceState,
        default_row_height: f32,
    ) -> Self {
        Self::from_measured_face_metrics(active_face.metrics(), default_row_height)
    }

    pub(crate) fn display_box_from_active_face_state(
        active_face: &DisplayRowActiveFaceState,
        height: f32,
        ascent: f32,
        default_row_height: f32,
    ) -> Self {
        let metrics = active_face.metrics();
        Self::new(
            height,
            ascent,
            metrics.char_width,
            metrics.space_width,
            default_row_height,
        )
    }

    pub(crate) fn from_measured_face_metrics(
        metrics: DisplayRowMeasuredFaceMetrics,
        default_row_height: f32,
    ) -> Self {
        Self::new(
            metrics.row_height,
            metrics.ascent,
            metrics.char_width,
            metrics.space_width,
            default_row_height,
        )
    }
}

#[derive(Clone)]
pub(crate) struct DisplayRowAppendFrame {
    row: usize,
    glyph_y: f32,
    geometry: DisplayRowGeometry,
    default_row_height: f32,
    content_x: f32,
    text_width: f32,
    line_number_width: f32,
    face_space_width: f32,
}

impl DisplayRowAppendFrame {
    fn right_edge(&self) -> f32 {
        self.content_x + self.geometry.width
    }

    fn text_right_edge_excluding_line_number(&self) -> f32 {
        self.content_x + (self.text_width - self.line_number_width).max(0.0)
    }

    fn from_parts(
        placement: DisplayRowAppendPlacement,
        area: DisplayRowAppendArea,
        metrics: DisplayRowAppendMetrics,
        tab_policy: DisplayTabPolicy,
    ) -> Self {
        Self {
            row: placement.row,
            glyph_y: placement.glyph_y,
            geometry: DisplayRowGeometry {
                y: placement.y,
                width: area.width,
                height: metrics.height,
                char_width: metrics.char_width,
                ascent: metrics.ascent,
                tab_policy,
            },
            default_row_height: metrics.default_row_height,
            content_x: area.content_x,
            text_width: area.text_width,
            line_number_width: area.line_number_width,
            face_space_width: metrics.space_width,
        }
    }

    fn source_append_request<'face>(
        &self,
        position: DisplayRowPosition,
        face_id: u32,
        base_face: &'face ResolvedFace,
        kind: DisplayRowAppendKind,
    ) -> DisplayRowSourceAppendRequest<'face> {
        let geometry = DisplayRowGeometry {
            char_width: kind.char_width(self),
            ..self.geometry.clone()
        };
        DisplayRowSourceAppendRequest::from_text_row_policy(
            position,
            face_id,
            base_face,
            DisplayRowSourceAppendRequestPolicy::new(
                self.row,
                self.geometry.y,
                self.glyph_y,
                kind.output_height(self),
                geometry,
                kind.max_x(self),
            ),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayRowAppendKind {
    SourceText,
    Tab,
    ControlChar,
    SourceMappedText,
    Glyphless,
    DisplayReplacement,
    DisplayReplacementString,
}

impl DisplayRowAppendKind {
    fn char_width(self, frame: &DisplayRowAppendFrame) -> f32 {
        match self {
            Self::Tab | Self::DisplayReplacementString => frame.face_space_width,
            Self::SourceText
            | Self::ControlChar
            | Self::SourceMappedText
            | Self::Glyphless
            | Self::DisplayReplacement => frame.geometry.char_width,
        }
    }

    fn max_x(self, frame: &DisplayRowAppendFrame) -> DisplayRowMaxX {
        match self {
            Self::Tab => DisplayRowMaxX::Unbounded,
            Self::ControlChar => {
                DisplayRowMaxX::Bounded(frame.text_right_edge_excluding_line_number())
            }
            Self::SourceText
            | Self::SourceMappedText
            | Self::Glyphless
            | Self::DisplayReplacement
            | Self::DisplayReplacementString => DisplayRowMaxX::Bounded(frame.right_edge()),
        }
    }

    fn output_height(self, frame: &DisplayRowAppendFrame) -> f32 {
        match self {
            Self::SourceText
            | Self::Glyphless
            | Self::DisplayReplacement
            | Self::DisplayReplacementString => frame.geometry.height,
            Self::Tab | Self::ControlChar | Self::SourceMappedText => frame.default_row_height,
        }
    }
}

pub(crate) fn append_synthetic_text_to_display_row(
    state: &mut TextRowSourceRenderState<'_>,
    base_face: &ResolvedFace,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    source: SyntheticTextSource,
    face_id: u32,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let source = source.into_item_source(face_id);
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    let start = position;
    let mut face_ids = FrameFaceIdAllocator::new(face_id.saturating_add(1));
    let outcome = DisplayRowSourceAppendOperation::new(
        base_face,
        face_id,
        frame,
        position,
        DisplayRowAppendKind::SourceText,
    )
    .render_source_to_text_row_and_emit(state, source, &mut face_ids, &mut render_policy)?;
    Some(outcome.into_append_progress_and_position(start))
}

#[cfg(test)]
#[path = "display_row_append_test.rs"]
mod tests;
