use crate::coords::layout_i64_char_pos_to_lisp_char_pos;
use crate::display_cursor::{
    CapturedCursorInfo, CapturedCursorPlacement, CapturedCursorSlotWidth,
    CapturedCursorVisualState, CapturedTextWindowCursorPublishContext, CursorCaptureState,
    VisualTextWindowCursorPublishContext, VisualTextWindowCursorPublishSummary,
    capture_cursor_info, display_property_replacement_cursor_info,
    update_cursor_info_for_main_char,
};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_face_layout::{DisplayHeightFaceBasis, height_adjusted_face};
use crate::display_face_policy::BaseFacePolicy;
use crate::display_item::{
    DisplayGlyphless, DisplayItem, DisplayItemKind, DisplayMediaReplacement,
    DisplaySourceMappedText, DisplayTextRun, GlyphlessJoinerPolicy, GlyphlessMethod, RenderFaceRef,
    SourceSpan, glyphless_method_for_char,
};
use crate::display_origin::{DisplayOrigin, DisplayPropertySource, OverlayStringKind};
use crate::display_property::{
    DisplayMediaReplacementProperty, DisplayPropertyClassification, classify_display_property,
};
#[cfg(test)]
use crate::display_row::RenderedDisplayRow;
#[cfg(test)]
use crate::display_row::append_rendered_display_row_fragment_to_current_row;
use crate::display_row::{
    CurrentTextRowRenderOutcome, DisplayRowActiveFaceState, DisplayRowComplexTextRunAdvancePolicy,
    DisplayRowCurrentTextMeasureState, DisplayRowCurrentTextRenderState, DisplayRowFallbackMetrics,
    DisplayRowGeometry, DisplayRowMeasuredFaceMetrics, DisplayRowMeasurementPolicy,
    DisplayRowRenderBounds, DisplayRowRenderClipBehavior, DisplayRowRenderPolicy,
    DisplayRowResolvedMeasuredFace, DisplayRowSourceAppendRequest,
    DisplayRowSourceAppendRequestPolicy, DisplayRowSourceState, DisplaySourceAppendMeasurement,
    DisplaySourceAppendRenderPolicy, NaturalDisplayRowAppendRenderPolicy,
};
use crate::display_row::{DisplayRowRenderStop, insert_resolved_display_row_face};
use crate::display_row_builder::{
    DisplayRowAppendProgress, DisplayRowAppendStatus, DisplayRowGlyphSlot,
    DisplayRowItemMeasurement, DisplayRowPosition, DisplayTabPolicy,
};
use crate::display_row_geometry::{
    DisplayRowBoundaryTarget, DisplayRowFlagKind, DisplayRowFlags, DisplayRowGeometryDefaults,
    DisplayRowGeometryState, DisplayRowHitRange, DisplayRowLimit, DisplayRowScopedValue,
    DisplayRowTextPosition, DisplayRowVisibilityLimit, DisplayRowYPositions, DisplayRowYRecording,
};
use crate::display_row_walk_state::{
    ActiveDisplayPropertySpan, BoxFaceRowState, BufferTextRowOverflowDecision, FaceScanCheckpoint,
    HitRowRangeTracker, HorizontalScrollSkipState, LineNumberRenderState,
    SpecialTextRowOverflowDecision, TextPropertyScanCheckpoints, TextRowTransitionPrefixAction,
    TextRowTransitionStatePolicy, TrailingWhitespaceRenderState, WordWrapBreakCandidate,
    WordWrapRenderState, next_window_start_for_partially_visible_point_row,
    next_window_start_for_point_line_continuation, next_window_start_from_visible_rows,
    skip_text_to_charpos, skip_to_newline,
};
use crate::display_source::{
    BufferDisplayReplacementSource, BufferDisplayReplacementStringSource, BufferTextItemSource,
    DisplayItemSource, DisplayReplacementBox, LispStringSourceCursor,
};
#[cfg(test)]
use crate::display_source_resolver::PendingDisplaySourceFace;
use crate::display_source_resolver::{
    DisplayStringBaseFace, ResolvedDisplayReplacement, display_string_base_face,
    display_string_base_face_for_active_row, resolve_display_replacement,
};
use crate::display_space::{DisplaySpaceKey, display_space_positive_number};
use crate::display_text_run_measurement::ComplexTextRunAdvanceResolver;
use crate::font_metrics::FontMetricsService;
use crate::hit_test::{HitRow, WindowHitData};
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::neovm_bridge::{
    FaceResolver, LayoutBufferView, OverlayDisplayString, ResolvedFace, RustBufferAccess,
    RustTextPropAccess,
};
use crate::types::WindowParams;
use crate::unicode::{decode_utf8, is_wide_char};
#[cfg(test)]
use crate::window_output::TextRowOutput;
use crate::window_output::{
    TextMatrixRowBegin, TextMatrixRowGeometryTransition, TextMatrixRowMetrics,
    TextMatrixRowTransition, TextWindowBegin, TextWindowBodyOutputInstall, TextWindowCursorEffects,
    TextWindowLineNumberMargin, TextWindowPendingRowFinish, TextWindowRedisplayPositions,
    TextWindowRightBorder, TextWindowRightEdgeMarkers, WindowOutputEmitter,
    begin_text_window_output, close_text_window_output, current_text_window_cluster_tail,
    emit_text_matrix_row_transition, emit_text_matrix_row_transition_with_limit,
    emit_text_window_line_number_margin, finish_and_end_text_matrix_row_output,
    finish_pending_text_window_row, install_last_window_right_border,
    install_text_window_body_output, install_text_window_cursor_effects,
    mark_current_text_row_truncated_left,
};
use neomacs_display_protocol::effect_config::EffectsConfig;
use neomacs_display_protocol::face::BasicFaceId;
use neomacs_display_protocol::types::Color;
use neovm_core::buffer::{BufferId, CharLen, CharPos0, EmacsBytePos, EmacsByteRange, LispCharPos1};
use neovm_core::emacs_core::eval::DisplayHost;
use neovm_core::emacs_core::value::get_string_text_properties_table_for_value;
use neovm_core::emacs_core::{Context, Value};
use neovm_core::window::{DisplayRowSnapshot, FrameId, WindowDisplaySnapshot, WindowId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LispStringSourceId(u64);

impl LispStringSourceId {
    const OVERLAY_STRING: Self = Self(1);
    const PREFIX: Self = Self(2);

    fn display_replacement(source_id: u64) -> Self {
        Self(source_id)
    }

    fn raw(self) -> u64 {
        self.0
    }
}

const SYNTHETIC_SOURCE_INVISIBLE_ELLIPSIS: u64 = 3;
const SYNTHETIC_SOURCE_HSCROLL_TRUNCATION: u64 = 4;
const SYNTHETIC_SOURCE_SELECTIVE_ELLIPSIS: u64 = 5;

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

    #[cfg(test)]
    pub(crate) fn render_pending(
        self,
        line_numbers: &mut LineNumberRenderState,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        builder: &mut GlyphMatrixBuilder,
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
            face_resolver.resolve_named_face(line_number_request.face().face_name());
        let line_number_face_id = face_ids.allocate();
        insert_resolved_display_row_face(builder, line_number_face_id, &line_number_face, None);

        let text = line_number_request.text();
        emit_text_window_line_number_margin(
            builder,
            TextWindowLineNumberMargin {
                text: &text,
                cols: line_number_request.cols(),
                face_id: line_number_face_id,
                row_y: row_geometry.y(),
                row_height: row_geometry.height(),
                row_ascent: row_geometry.ascent(),
                char_width,
            },
        );

        face_scan.invalidate();
        line_numbers.consume_render_request();
        true
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
        emit_text_window_line_number_margin(
            source_render.output_render().builder,
            TextWindowLineNumberMargin {
                text: &text,
                cols: line_number_request.cols(),
                face_id: line_number_face_id,
                row_y: row_geometry.y(),
                row_height: row_geometry.height(),
                row_ascent: row_geometry.ascent(),
                char_width,
            },
        );

        face_scan.invalidate();
        line_numbers.consume_render_request();
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ResolvedBufferTextSourceAdvance {
    advance_px: f32,
    append_measurement: DisplaySourceAppendMeasurement,
}

impl ResolvedBufferTextSourceAdvance {
    fn natural(advance_px: f32) -> Self {
        Self {
            advance_px,
            append_measurement: DisplaySourceAppendMeasurement::Natural,
        }
    }

    fn resolved(advance_px: f32) -> Self {
        Self {
            advance_px,
            append_measurement: DisplaySourceAppendMeasurement::ResolvedAdvance { advance_px },
        }
    }

    fn advance_px(self) -> f32 {
        self.advance_px
    }

    fn append_measurement(self) -> DisplaySourceAppendMeasurement {
        self.append_measurement
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BufferTextSourceTextRequest {
    range: BufferTextSourceRange,
    resolved_advance: ResolvedBufferTextSourceAdvance,
}

impl BufferTextSourceTextRequest {
    fn new(
        range: BufferTextSourceRange,
        resolved_advance: ResolvedBufferTextSourceAdvance,
    ) -> Self {
        Self {
            range,
            resolved_advance,
        }
    }

    fn range(self) -> BufferTextSourceRange {
        self.range
    }

    fn append_measurement(self) -> DisplaySourceAppendMeasurement {
        self.resolved_advance.append_measurement()
    }

    fn advance_px(self) -> f32 {
        self.resolved_advance.advance_px()
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

    pub(crate) fn into_parts(
        self,
    ) -> (
        &'a mut GlyphMatrixBuilder,
        &'a mut WindowOutputEmitter,
        &'a mut Context,
    ) {
        (self.builder, self.output_emitter, self.evaluator)
    }

    fn finish_and_end_text_matrix_row_output(self, metrics: TextMatrixRowMetrics) {
        finish_and_end_text_matrix_row_output(
            self.builder,
            self.output_emitter,
            self.evaluator,
            metrics,
        );
    }

    fn emit_text_matrix_row_transition(self, transition: TextMatrixRowGeometryTransition) {
        emit_text_matrix_row_transition(
            self.builder,
            self.output_emitter,
            self.evaluator,
            transition,
        );
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
        display_string_base_face(
            buffer,
            self.face_resolver,
            origin,
            policy,
            face_ids,
            self.output_render.builder,
        )
    }

    pub(crate) fn display_string_base_face_for_active_row<B: LayoutBufferView>(
        &mut self,
        buffer: &B,
        origin: DisplayOrigin,
        policy: BaseFacePolicy,
        active_face_state: &DisplayRowActiveFaceState,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> DisplayStringBaseFace {
        display_string_base_face_for_active_row(
            buffer,
            self.face_resolver,
            origin,
            policy,
            active_face_state,
            face_ids,
            self.output_render.builder,
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

    pub(crate) fn mark_current_text_row_truncated_left(&mut self) {
        mark_current_text_row_truncated_left(self.output_render.builder);
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

    pub(crate) fn into_parts(
        self,
    ) -> (
        &'a mut GlyphMatrixBuilder,
        &'a mut WindowOutputEmitter,
        &'a mut Context,
        &'a mut Option<FontMetricsService>,
        &'a FaceResolver,
    ) {
        let (builder, output_emitter, evaluator) = self.output_render.into_parts();
        (
            builder,
            output_emitter,
            evaluator,
            self.font_metrics,
            self.face_resolver,
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
    let (builder, output_emitter, evaluator, font_metrics, face_resolver) =
        state.reborrow().into_parts();
    DisplayRowCurrentTextRenderState {
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        face_resolver,
        face_ids,
    }
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

    pub(crate) fn reborrow(&mut self) -> TextRowSourceMeasureState<'_> {
        TextRowSourceMeasureState {
            builder: self.builder,
            evaluator: self.evaluator,
            font_metrics: self.font_metrics,
            face_resolver: self.face_resolver,
        }
    }

    fn into_parts(
        self,
    ) -> (
        &'a mut GlyphMatrixBuilder,
        &'a mut Context,
        &'a mut Option<FontMetricsService>,
        &'a FaceResolver,
    ) {
        (
            self.builder,
            self.evaluator,
            self.font_metrics,
            self.face_resolver,
        )
    }

    fn font_metrics(&mut self) -> &mut Option<FontMetricsService> {
        self.font_metrics
    }
}

fn current_text_measure_state<'emit>(
    state: &'emit mut TextRowSourceMeasureState<'_>,
    face_ids: &'emit mut FrameFaceIdAllocator,
) -> DisplayRowCurrentTextMeasureState<'emit, 'emit> {
    let (builder, evaluator, font_metrics, face_resolver) = state.reborrow().into_parts();
    DisplayRowCurrentTextMeasureState {
        builder,
        evaluator,
        font_metrics,
        face_resolver,
        face_ids,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BufferTextSourceAdvancePath {
    NaturalRenderedSource,
    ResolvedComplexRun,
}

impl BufferTextSourceAdvancePath {
    fn for_cluster_state(cluster: BufferTextSourceClusterState) -> Self {
        if crate::composition::needs_complex_shaping(cluster.ch()) {
            Self::ResolvedComplexRun
        } else {
            Self::NaturalRenderedSource
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BufferTextSourceNaturalFallbackAdvance {
    Tab,
    ClusterContinuation,
    FaceColumns { columns: usize },
}

impl BufferTextSourceNaturalFallbackAdvance {
    fn for_cluster_state(cluster: BufferTextSourceClusterState) -> Self {
        let ch = cluster.ch();
        if ch == '\t' {
            Self::Tab
        } else if cluster.is_cluster_continuation() {
            Self::ClusterContinuation
        } else {
            Self::FaceColumns {
                columns: crate::composition::base_width_cols(ch) as usize,
            }
        }
    }

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

#[cfg(test)]
pub(crate) fn append_rendered_display_row_fragment_to_text_row_and_emit(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    rendered: &RenderedDisplayRow,
    output: TextRowOutput,
) -> DisplayRowPosition {
    let end = append_rendered_display_row_fragment_to_current_row(builder, rendered, output.row);
    output_emitter.emit_text_source_slots(evaluator, output, &rendered.source_slots, end);
    end
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

pub(crate) struct BufferHscrollSkipRenderState<'a, 'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
}

pub(crate) struct BufferTextLineBreakRenderState<'a, 'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) box_face: &'emit mut BoxFaceRowState,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
}

pub(crate) struct BufferTextOverflowRenderState<'a, 'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) col: &'emit mut usize,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) x: &'emit mut f32,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) face_scan: &'emit mut FaceScanCheckpoint,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
}

pub(crate) struct BufferTextSpecialOverflowRenderState<'a, 'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) col: &'emit mut usize,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) x: &'emit mut f32,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
}

pub(crate) struct BufferTextSourceCharRenderRequest<'a> {
    decoded_source_char: BufferTextDecodedSourceChar,
    text: &'a [u8],
    text_start_byte: usize,
    buffer_id: BufferId,
    append_surface: &'a DisplayRowAppendSurface,
    overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
    active_face_state: &'a DisplayRowActiveFaceState,
    params: &'a WindowParams,
    glyph_y_offset: f32,
    char_h: f32,
    point_charpos: i64,
    row_visibility_limit: DisplayRowVisibilityLimit,
    content_x: f32,
    has_prefix: bool,
    row_geometry_defaults: DisplayRowGeometryDefaults,
    text_matrix_row_base: usize,
    max_rows: usize,
    row_limit: DisplayRowLimit,
}

pub(crate) struct BufferTextSourceCharRenderRequestState<'a, 'emit> {
    pub(crate) append_state: &'emit mut BufferTextRowAppendState,
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) col: &'emit mut usize,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) x: &'emit mut f32,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) face_scan: &'emit mut FaceScanCheckpoint,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
    pub(crate) raise_span: &'emit mut ActiveDisplayPropertySpan<f32>,
}

pub(crate) struct BufferSelectiveDisplayTailRenderRequest<'a> {
    source_char: BufferTextDecodedSourceChar,
    text: &'a [u8],
    text_start_byte: usize,
    selective_display: i32,
    tab_width: i32,
    append_surface: &'a DisplayRowAppendSurface,
    active_face_state: &'a DisplayRowActiveFaceState,
    glyph_y_offset: f32,
    default_face_ascent: f32,
    char_h: f32,
    char_w: f32,
    content_x: f32,
    has_prefix: bool,
    row_geometry_defaults: DisplayRowGeometryDefaults,
    text_matrix_row_base: usize,
    max_rows: usize,
    row_limit: DisplayRowLimit,
}

pub(crate) struct BufferSelectiveDisplayTailRenderState<'a, 'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) col: &'emit mut usize,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) box_face: &'emit mut BoxFaceRowState,
    pub(crate) x: &'emit mut f32,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
}

pub(crate) struct BufferInvisibleTextRenderRequest<'a> {
    text: &'a [u8],
    accessible_end: i64,
    point_charpos: i64,
    append_surface: &'a DisplayRowAppendSurface,
    overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
    active_face_state: &'a DisplayRowActiveFaceState,
    glyph_y_offset: f32,
    default_face_ascent: f32,
    char_h: f32,
    char_w: f32,
}

pub(crate) struct BufferInvisibleTextRenderRequestState<'a, 'emit> {
    pub(crate) checkpoints: &'emit mut TextPropertyScanCheckpoints,
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
}

pub(crate) struct BufferEndOfBufferTailRenderRequest<'a> {
    byte_idx: usize,
    charpos: i64,
    accessible_end: i64,
    point_charpos: i64,
    has_overlays: bool,
    overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
    active_face_state: &'a DisplayRowActiveFaceState,
    row_limit: DisplayRowLimit,
}

pub(crate) struct BufferEndOfBufferTailRenderState<'emit> {
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) row_y_positions: &'emit mut DisplayRowYPositions,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
}

pub(crate) struct BufferTextWindowBeginRequest {
    frame_id: FrameId,
    window_id: WindowId,
    text_matrix_row_base: usize,
    text_area_left: f32,
    window_top: f32,
    matrix_window_id: u64,
    matrix_rows: usize,
    matrix_cols: usize,
    bounds: neomacs_display_protocol::types::Rect,
    text_bounds: neomacs_display_protocol::types::Rect,
    selected: bool,
    first_row: TextMatrixRowBegin,
}

pub(crate) struct BufferTextWindowBeginState<'a> {
    pub(crate) builder: &'a mut GlyphMatrixBuilder,
    pub(crate) evaluator: &'a mut Context,
}

pub(crate) struct BufferTextWindowCursorEffectsRequest {
    window_id: i64,
    effects: Option<EffectsConfig>,
}

pub(crate) struct BufferTextWindowTerminalRightBorderRequest {
    ch: char,
    face_name: &'static str,
    char_width: f32,
}

pub(crate) struct BufferTextWindowTailFinalizeRequest<'a> {
    params: &'a WindowParams,
    text: &'a [u8],
    text_matrix_row_base: usize,
    text_area_left: f32,
    window_top: f32,
    text_y: f32,
    text_height: f32,
    char_w: f32,
    char_h: f32,
    window_start: i64,
    point_charpos: i64,
    charpos: i64,
    point_is_visible_eob: bool,
    row_limit: DisplayRowLimit,
}

pub(crate) struct BufferTextWindowTailFinalizeState<'a, 'emit> {
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) row_geometry: &'a DisplayRowGeometryState,
    pub(crate) row_y_positions: &'a DisplayRowYPositions,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) output_render: TextRowOutputRenderState<'emit>,
}

pub(crate) struct BufferTextWindowBodyInstallRequest<'a> {
    window_id: u64,
    window_start: i64,
    text_start_byte: usize,
    byte_idx: usize,
    reserve_right_special_col: bool,
    reserve_right_border_col: bool,
    text_matrix_row_base: usize,
    matrix_cols: usize,
    row_flags: &'a DisplayRowFlags,
    right_edge_face_id: u32,
    char_w: f32,
}

pub(crate) struct BufferTextWindowBodyInstallState<'a, 'emit> {
    pub(crate) builder: &'emit mut GlyphMatrixBuilder,
    pub(crate) output_emitter: &'a WindowOutputEmitter,
}

pub(crate) struct BufferTextWindowVisibilityRetryRequest<'a, 'buf, B: LayoutBufferView> {
    rows: &'a [DisplayRowSnapshot],
    window_start: i64,
    accessible_start: i64,
    accessible_end: i64,
    point_charpos: i64,
    charpos: i64,
    point_is_visible_eob: bool,
    is_minibuffer: bool,
    text_area_top: i64,
    text_area_bottom: i64,
    buf_access: &'a RustBufferAccess<'buf, B>,
}

pub(crate) struct BufferTextWindowFinishRequest {
    window_id: i64,
    content_x: f32,
    char_w: f32,
    text_area_left_offset: i64,
    mode_line_height: i64,
    header_line_height: i64,
    tab_line_height: i64,
}

pub(crate) struct BufferTextWindowFinishState<'a> {
    pub(crate) builder: &'a mut GlyphMatrixBuilder,
    pub(crate) output_emitter: WindowOutputEmitter,
    pub(crate) evaluator: &'a mut Context,
    pub(crate) hit_rows: Vec<HitRow>,
}

pub(crate) struct BufferTextWindowFinishOutput {
    pub(crate) hit_data: WindowHitData,
    pub(crate) snapshot: WindowDisplaySnapshot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowVisibilityRetryOutcome {
    visible_end_lisp: Option<LispCharPos1>,
    visible_progress: i64,
    point_beyond_visible_span: bool,
    scroll_down_window_start: Option<i64>,
    point_row_window_start: Option<i64>,
    point_line_window_start: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowTailFinalizeOutcome {
    cursor_requested: bool,
    cursor_published: bool,
    visual_cursor_summary: VisualTextWindowCursorPublishSummary,
    pending_row_finished: bool,
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

    fn truncation(state_policy: TextRowTransitionStatePolicy) -> Self {
        Self::new(DisplayRowOverflowTransitionKind::Truncation, state_policy)
    }

    fn visual_wrap(state_policy: TextRowTransitionStatePolicy) -> Self {
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

#[derive(Clone, Copy)]
pub(crate) struct LispStringRowAppendContext<'row> {
    active_face_context: DisplayRowActiveFaceAppendContext<'row, 'row>,
}

impl<'row> LispStringRowAppendContext<'row> {
    pub(crate) fn new(
        append_surface: &'row DisplayRowAppendSurface,
        geometry: &'row DisplayRowGeometryState,
        active_face: &'row DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            active_face_context: DisplayRowActiveFaceAppendContext::new(
                append_surface,
                geometry,
                active_face,
                glyph_y_offset,
                default_row_height,
            ),
        }
    }

    fn render_active_face_source_request_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        base_face_id: u32,
        base_face: &'row ResolvedFace,
        request: LispStringSourceAppendRequest,
    ) -> DisplayRowPosition {
        let position = request.position();
        let Some(mut source_session) =
            LispStringSourceAppendSession::new(request, base_face_id, base_face)
        else {
            return position;
        };
        let frame = self.active_face_context.active_face_frame();
        source_session
            .render_to_text_row_and_emit(state, face_ids, frame, position)
            .map(|outcome| outcome.end_position())
            .unwrap_or(position)
    }

    pub(crate) fn render_prefix_source_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        base_face: &DisplayStringBaseFace,
        prefix_source: DisplayRowPrefixSource,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        self.render_active_face_source_request_to_text_row_and_emit(
            state,
            face_ids,
            base_face.face_id(),
            base_face.face(),
            prefix_source.append_request(position),
        )
    }
}

fn render_lisp_string_source_append_to_text_row_and_emit(
    state: &mut TextRowSourceRenderState<'_>,
    source: &mut LispStringSourceCursor,
    source_state: &mut DisplayRowSourceState,
    base_face: &ResolvedFace,
    base_face_id: u32,
    face_ids: &mut FrameFaceIdAllocator,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<CurrentTextRowRenderOutcome> {
    DisplayRowSourceAppendOperation::new(
        base_face,
        base_face_id,
        frame,
        position,
        DisplayRowAppendKind::SourceText,
    )
    .render_source_cursor_to_text_row_and_emit(state, source, source_state, face_ids)
}

pub(crate) struct LispStringSourceAppendContext<'a> {
    source: &'a mut LispStringSourceCursor,
    source_state: &'a mut DisplayRowSourceState,
    base_face_id: u32,
    base_face: &'a ResolvedFace,
}

impl<'a> LispStringSourceAppendContext<'a> {
    pub(crate) fn new(
        source: &'a mut LispStringSourceCursor,
        source_state: &'a mut DisplayRowSourceState,
        base_face_id: u32,
        base_face: &'a ResolvedFace,
    ) -> Self {
        Self {
            source,
            source_state,
            base_face_id,
            base_face,
        }
    }

    pub(crate) fn render_to_text_row_and_emit(
        &mut self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
    ) -> Option<CurrentTextRowRenderOutcome> {
        render_lisp_string_source_append_to_text_row_and_emit(
            state,
            self.source,
            self.source_state,
            self.base_face,
            self.base_face_id,
            face_ids,
            frame,
            position,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LispStringSourceAppendRequest {
    position: DisplayRowPosition,
    source_id: LispStringSourceId,
    value: Value,
}

impl LispStringSourceAppendRequest {
    fn new(position: DisplayRowPosition, source_id: LispStringSourceId, value: Value) -> Self {
        Self {
            position,
            source_id,
            value,
        }
    }

    fn position(self) -> DisplayRowPosition {
        self.position
    }

    fn into_source(self, base_face_id: u32) -> Option<LispStringSourceCursor> {
        LispStringSourceCursor::new(
            self.source_id.raw(),
            self.value,
            RenderFaceRef::FaceId(base_face_id),
        )
    }
}

pub(crate) struct LispStringSourceAppendSession<'a> {
    source: LispStringSourceCursor,
    source_state: DisplayRowSourceState,
    base_face_id: u32,
    base_face: &'a ResolvedFace,
}

impl<'a> LispStringSourceAppendSession<'a> {
    fn new(
        request: LispStringSourceAppendRequest,
        base_face_id: u32,
        base_face: &'a ResolvedFace,
    ) -> Option<Self> {
        let source = request.into_source(base_face_id)?;
        Some(Self {
            source,
            source_state: DisplayRowSourceState::default(),
            base_face_id,
            base_face,
        })
    }

    fn append_context(&mut self) -> LispStringSourceAppendContext<'_> {
        LispStringSourceAppendContext::new(
            &mut self.source,
            &mut self.source_state,
            self.base_face_id,
            self.base_face,
        )
    }

    fn render_to_text_row_and_emit(
        &mut self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
    ) -> Option<CurrentTextRowRenderOutcome> {
        self.append_context()
            .render_to_text_row_and_emit(state, face_ids, frame, position)
    }

    fn discard_pending_until_row_break(&mut self) -> bool {
        self.source_state.discard_pending_item();
        self.source.discard_until_row_break()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DisplayRowPrefixRequest {
    None,
    Line,
    Wrap,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowPrefixValues {
    line_property: Option<Value>,
    wrap_property: Option<Value>,
    line_default: Option<Value>,
    wrap_default: Option<Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DisplayRowPrefixKind {
    Line,
    Wrap,
}

#[derive(Clone, Copy)]
pub(crate) struct DisplayRowPrefixSource {
    value: Value,
    anchor_charpos: CharPos0,
    kind: DisplayRowPrefixKind,
}

pub(crate) struct BufferLinePrefixRenderContext<'a> {
    values: DisplayRowPrefixValues,
    append_surface: &'a DisplayRowAppendSurface,
    row_geometry: &'a DisplayRowGeometryState,
    active_face_state: &'a DisplayRowActiveFaceState,
    glyph_y_offset: f32,
    default_row_height: f32,
}

pub(crate) struct BufferLinePrefixRenderRequest<'a> {
    context: BufferLinePrefixRenderContext<'a>,
    position: DisplayRowPosition,
}

impl DisplayRowPrefixRequest {
    pub(crate) fn initial(has_prefix: bool, has_line_prefix: bool) -> Self {
        if has_prefix && has_line_prefix {
            Self::Line
        } else {
            Self::None
        }
    }

    pub(crate) fn request_line(&mut self) {
        *self = Self::Line;
    }

    pub(crate) fn request_wrap(&mut self) {
        *self = Self::Wrap;
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::None;
    }

    pub(crate) fn is_requested(self) -> bool {
        !matches!(self, Self::None)
    }

    pub(crate) fn apply_transition_prefix_action(
        &mut self,
        has_prefix: bool,
        action: TextRowTransitionPrefixAction,
    ) {
        if !has_prefix {
            return;
        }
        match action {
            TextRowTransitionPrefixAction::Line => self.request_line(),
            TextRowTransitionPrefixAction::Wrap => self.request_wrap(),
        }
    }

    pub(crate) fn source_for_value(
        self,
        value: Value,
        anchor_charpos: CharPos0,
    ) -> Option<DisplayRowPrefixSource> {
        let kind = match self {
            Self::Line => DisplayRowPrefixKind::Line,
            Self::Wrap => DisplayRowPrefixKind::Wrap,
            Self::None => return None,
        };
        Some(DisplayRowPrefixSource {
            value,
            anchor_charpos,
            kind,
        })
    }

    pub(crate) fn source_from_values(
        self,
        values: DisplayRowPrefixValues,
        anchor_charpos: CharPos0,
    ) -> Option<DisplayRowPrefixSource> {
        let value = match self {
            Self::Line => values.line_property.or(values.line_default),
            Self::Wrap => values.wrap_property.or(values.wrap_default),
            Self::None => None,
        }?;
        self.source_for_value(value, anchor_charpos)
    }
}

impl DisplayRowPrefixValues {
    pub(crate) fn new(
        line_property: Option<Value>,
        wrap_property: Option<Value>,
        line_default: Option<Value>,
        wrap_default: Option<Value>,
    ) -> Self {
        Self {
            line_property: Self::lisp_string_value(line_property),
            wrap_property: Self::lisp_string_value(wrap_property),
            line_default: Self::lisp_string_value(line_default),
            wrap_default: Self::lisp_string_value(wrap_default),
        }
    }

    fn lisp_string_value(value: Option<Value>) -> Option<Value> {
        value.filter(|value| value.as_lisp_string().is_some())
    }

    pub(crate) fn default_values(line_default: Option<Value>, wrap_default: Option<Value>) -> Self {
        Self::new(None, None, line_default, wrap_default)
    }

    pub(crate) fn with_properties(
        self,
        line_property: Option<Value>,
        wrap_property: Option<Value>,
    ) -> Self {
        Self::new(
            line_property,
            wrap_property,
            self.line_default,
            self.wrap_default,
        )
    }

    pub(crate) fn has_default_prefix(self) -> bool {
        self.line_default.is_some() || self.wrap_default.is_some()
    }

    pub(crate) fn has_line_default_prefix(self) -> bool {
        self.line_default.is_some()
    }
}

impl DisplayRowPrefixSource {
    #[cfg(test)]
    pub(crate) fn value(self) -> Value {
        self.value
    }

    pub(crate) fn origin(self) -> DisplayOrigin {
        match self.kind {
            DisplayRowPrefixKind::Line => DisplayOrigin::LinePrefix {
                anchor_charpos: self.anchor_charpos,
            },
            DisplayRowPrefixKind::Wrap => DisplayOrigin::WrapPrefix {
                anchor_charpos: self.anchor_charpos,
            },
        }
    }

    pub(crate) fn base_face_policy(self) -> BaseFacePolicy {
        BaseFacePolicy::DefaultFace
    }

    fn append_request(self, position: DisplayRowPosition) -> LispStringSourceAppendRequest {
        LispStringSourceAppendRequest::new(position, LispStringSourceId::PREFIX, self.value)
    }
}

impl<'a> BufferLinePrefixRenderContext<'a> {
    pub(crate) fn new(
        values: DisplayRowPrefixValues,
        append_surface: &'a DisplayRowAppendSurface,
        row_geometry: &'a DisplayRowGeometryState,
        active_face_state: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            values,
            append_surface,
            row_geometry,
            active_face_state,
            glyph_y_offset,
            default_row_height,
        }
    }

    pub(crate) fn render_requested_to_text_row_and_emit<B: LayoutBufferView>(
        self,
        request: &mut DisplayRowPrefixRequest,
        state: &mut TextRowSourceRenderState<'_>,
        buffer: &B,
        anchor_charpos: i64,
        face_ids: &mut FrameFaceIdAllocator,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        if !request.is_requested() {
            return position;
        }

        let text_props = RustTextPropAccess::new(buffer);
        let line_property = text_props.get_property(anchor_charpos, Value::symbol("line-prefix"));
        let wrap_property = text_props.get_property(anchor_charpos, Value::symbol("wrap-prefix"));
        let source = request.source_from_values(
            self.values.with_properties(line_property, wrap_property),
            CharPos0::new(anchor_charpos as usize),
        );
        request.clear();

        let Some(prefix_source) = source else {
            return position;
        };

        let prefix_base_face = state.display_string_base_face(
            buffer,
            prefix_source.origin(),
            prefix_source.base_face_policy(),
            face_ids,
        );
        LispStringRowAppendContext::new(
            self.append_surface,
            self.row_geometry,
            self.active_face_state,
            self.glyph_y_offset,
            self.default_row_height,
        )
        .render_prefix_source_to_text_row_and_emit(
            state,
            face_ids,
            &prefix_base_face,
            prefix_source,
            position,
        )
    }
}

impl<'a> BufferLinePrefixRenderRequest<'a> {
    pub(crate) fn new(
        context: BufferLinePrefixRenderContext<'a>,
        position: DisplayRowPosition,
    ) -> Self {
        Self { context, position }
    }

    #[allow(clippy::too_many_arguments)]
    #[cfg(test)]
    pub(crate) fn render_requested_to_text_row_and_apply<B: LayoutBufferView>(
        self,
        request: &mut DisplayRowPrefixRequest,
        evaluator: &mut Context,
        output_emitter: &mut WindowOutputEmitter,
        buffer: &B,
        anchor_charpos: i64,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        builder: &mut GlyphMatrixBuilder,
        x: &mut f32,
        col: &mut usize,
    ) {
        let position = self.context.render_requested_to_text_row_and_emit(
            request,
            &mut TextRowSourceRenderState::new(
                builder,
                output_emitter,
                evaluator,
                font_metrics,
                face_resolver,
            ),
            buffer,
            anchor_charpos,
            face_ids,
            self.position,
        );
        *x = position.x_px;
        *col = position.col;
    }

    pub(crate) fn render_requested_with_source_state_and_apply<B: LayoutBufferView>(
        self,
        request: &mut DisplayRowPrefixRequest,
        source_render: &mut TextRowSourceRenderState<'_>,
        buffer: &B,
        anchor_charpos: i64,
        face_ids: &mut FrameFaceIdAllocator,
        x: &mut f32,
        col: &mut usize,
    ) {
        let position = self.context.render_requested_to_text_row_and_emit(
            request,
            source_render,
            buffer,
            anchor_charpos,
            face_ids,
            self.position,
        );
        *x = position.x_px;
        *col = position.col;
    }
}

#[derive(Clone, Copy)]
pub(crate) struct OverlayStringRenderSource {
    string: Value,
    overlay_id: Value,
    anchor_charpos: CharPos0,
    kind: OverlayStringKind,
}

impl OverlayStringRenderSource {
    pub(crate) fn new(
        overlay_string: OverlayDisplayString,
        anchor_charpos: CharPos0,
        kind: OverlayStringKind,
    ) -> Self {
        Self {
            string: overlay_string.string,
            overlay_id: overlay_string.overlay_id,
            anchor_charpos,
            kind,
        }
    }

    pub(crate) fn anchor_i64(self) -> i64 {
        self.anchor_charpos.get() as i64
    }

    pub(crate) fn value(self) -> Value {
        self.string
    }

    pub(crate) fn origin(self) -> DisplayOrigin {
        DisplayOrigin::OverlayString {
            overlay_id: self.overlay_id,
            anchor_charpos: self.anchor_charpos,
            kind: self.kind,
        }
    }

    pub(crate) fn base_face_policy(self) -> BaseFacePolicy {
        BaseFacePolicy::OverlayStringAtAnchor
    }

    fn append_request(self, position: DisplayRowPosition) -> LispStringSourceAppendRequest {
        LispStringSourceAppendRequest::new(
            position,
            LispStringSourceId::OVERLAY_STRING,
            self.value(),
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) struct OverlayStringRenderBatchSource<'a> {
    overlay_strings: &'a [OverlayDisplayString],
    anchor_charpos: CharPos0,
    kind: OverlayStringKind,
}

impl<'a> OverlayStringRenderBatchSource<'a> {
    pub(crate) fn new(
        overlay_strings: &'a [OverlayDisplayString],
        anchor_charpos: CharPos0,
        kind: OverlayStringKind,
    ) -> Self {
        Self {
            overlay_strings,
            anchor_charpos,
            kind,
        }
    }

    pub(crate) fn is_empty(self) -> bool {
        self.overlay_strings.is_empty()
    }

    pub(crate) fn overlay_strings(self) -> &'a [OverlayDisplayString] {
        self.overlay_strings
    }

    pub(crate) fn source_for(
        self,
        overlay_string: OverlayDisplayString,
    ) -> OverlayStringRenderSource {
        OverlayStringRenderSource::new(overlay_string, self.anchor_charpos, self.kind)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct OverlayStringRenderRowContext<'a> {
    append_surface: &'a DisplayRowAppendSurface,
    face_char_w: f32,
    char_h: f32,
    default_row_ascent: f32,
    text_y: f32,
    row_base: usize,
    max_rows: usize,
}

impl<'a> OverlayStringRenderRowContext<'a> {
    pub(crate) fn new(
        append_surface: &'a DisplayRowAppendSurface,
        active_face_state: &DisplayRowActiveFaceState,
        char_h: f32,
        default_row_ascent: f32,
        text_y: f32,
        row_base: usize,
        max_rows: usize,
    ) -> Self {
        Self {
            append_surface,
            face_char_w: active_face_state.metrics().char_width,
            char_h,
            default_row_ascent,
            text_y,
            row_base,
            max_rows,
        }
    }

    fn content_x(self) -> f32 {
        self.append_surface.content_x()
    }

    fn right_edge(self) -> f32 {
        self.append_surface.right_edge()
    }

    fn geometry_defaults(self) -> DisplayRowGeometryDefaults {
        DisplayRowGeometryDefaults::new(self.text_y, self.char_h, self.default_row_ascent)
    }

    fn row_limit(self) -> DisplayRowLimit {
        DisplayRowLimit {
            max_rows: self.max_rows,
        }
    }

    fn cursor_visual_state(self, base_face: &ResolvedFace) -> CapturedCursorVisualState {
        CapturedCursorVisualState {
            face_width: self.face_char_w,
            face_height: self.char_h,
            face_ascent: self.default_row_ascent,
            background: neomacs_display_protocol::types::Color::from_pixel(base_face.bg),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BufferOverlayStringRenderContext<'a> {
    enabled: bool,
    window_id: u64,
    row_context: OverlayStringRenderRowContext<'a>,
}

pub(crate) struct OverlayStringRenderState<'a> {
    source_render: TextRowSourceRenderState<'a>,
    x: &'a mut f32,
    col: &'a mut usize,
    geometry: &'a mut DisplayRowGeometryState,
    cursor_info: &'a mut CursorCaptureState,
    hit_rows: &'a mut Vec<HitRow>,
    hit_row_range: &'a mut HitRowRangeTracker,
    row_y_positions: &'a mut DisplayRowYPositions,
    face_ids: &'a mut FrameFaceIdAllocator,
}

impl<'a> OverlayStringRenderState<'a> {
    #[allow(clippy::too_many_arguments)]
    #[cfg(test)]
    pub(crate) fn new(
        evaluator: &'a mut Context,
        output_emitter: &'a mut WindowOutputEmitter,
        font_metrics: &'a mut Option<FontMetricsService>,
        face_resolver: &'a FaceResolver,
        x: &'a mut f32,
        col: &'a mut usize,
        geometry: &'a mut DisplayRowGeometryState,
        cursor_info: &'a mut CursorCaptureState,
        hit_rows: &'a mut Vec<HitRow>,
        hit_row_range: &'a mut HitRowRangeTracker,
        row_y_positions: &'a mut DisplayRowYPositions,
        face_ids: &'a mut FrameFaceIdAllocator,
        builder: &'a mut GlyphMatrixBuilder,
    ) -> Self {
        Self {
            source_render: TextRowSourceRenderState::new(
                builder,
                output_emitter,
                evaluator,
                font_metrics,
                face_resolver,
            ),
            x,
            col,
            geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_source_render(
        source_render: TextRowSourceRenderState<'a>,
        x: &'a mut f32,
        col: &'a mut usize,
        geometry: &'a mut DisplayRowGeometryState,
        cursor_info: &'a mut CursorCaptureState,
        hit_rows: &'a mut Vec<HitRow>,
        hit_row_range: &'a mut HitRowRangeTracker,
        row_y_positions: &'a mut DisplayRowYPositions,
        face_ids: &'a mut FrameFaceIdAllocator,
    ) -> Self {
        Self {
            source_render,
            x,
            col,
            geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BufferOverlayStringTextRowRenderContext<'a> {
    enabled: bool,
    window_id: u64,
    append_surface: &'a DisplayRowAppendSurface,
    char_h: f32,
    default_row_ascent: f32,
    text_y: f32,
    row_base: usize,
    max_rows: usize,
}

impl<'a> BufferOverlayStringTextRowRenderContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        enabled: bool,
        window_id: u64,
        append_surface: &'a DisplayRowAppendSurface,
        char_h: f32,
        default_row_ascent: f32,
        text_y: f32,
        row_base: usize,
        max_rows: usize,
    ) -> Self {
        Self {
            enabled,
            window_id,
            append_surface,
            char_h,
            default_row_ascent,
            text_y,
            row_base,
            max_rows,
        }
    }

    fn overlay_context(
        self,
        active_face_state: &DisplayRowActiveFaceState,
    ) -> BufferOverlayStringRenderContext<'a> {
        BufferOverlayStringRenderContext::for_text_row(
            self.enabled,
            self.window_id,
            self.append_surface,
            active_face_state,
            self.char_h,
            self.default_row_ascent,
            self.text_y,
            self.row_base,
            self.max_rows,
        )
    }

    pub(crate) fn render_before_at<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        active_face_state: &DisplayRowActiveFaceState,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        self.overlay_context(active_face_state)
            .render_before_at(buffer, anchor_charpos, state);
    }

    pub(crate) fn render_after_at<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        active_face_state: &DisplayRowActiveFaceState,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        self.overlay_context(active_face_state)
            .render_after_at(buffer, anchor_charpos, state);
    }

    pub(crate) fn render_both_at<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        active_face_state: &DisplayRowActiveFaceState,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        self.overlay_context(active_face_state)
            .render_both_at(buffer, anchor_charpos, state);
    }
}

impl<'a> BufferOverlayStringRenderContext<'a> {
    pub(crate) fn new(
        enabled: bool,
        window_id: u64,
        row_context: OverlayStringRenderRowContext<'a>,
    ) -> Self {
        Self {
            enabled,
            window_id,
            row_context,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_text_row(
        enabled: bool,
        window_id: u64,
        append_surface: &'a DisplayRowAppendSurface,
        active_face_state: &DisplayRowActiveFaceState,
        char_h: f32,
        default_row_ascent: f32,
        text_y: f32,
        row_base: usize,
        max_rows: usize,
    ) -> Self {
        Self::new(
            enabled,
            window_id,
            OverlayStringRenderRowContext::new(
                append_surface,
                active_face_state,
                char_h,
                default_row_ascent,
                text_y,
                row_base,
                max_rows,
            ),
        )
    }

    pub(crate) fn render_before_at<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        self.render_at_kind(buffer, anchor_charpos, OverlayStringKind::Before, state);
    }

    pub(crate) fn render_after_at<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        self.render_at_kind(buffer, anchor_charpos, OverlayStringKind::After, state);
    }

    pub(crate) fn render_both_at<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        self.render_before_at(buffer, anchor_charpos, state);
        self.render_after_at(buffer, anchor_charpos, state);
    }

    fn render_at_kind<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        kind: OverlayStringKind,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        if !self.enabled {
            return;
        }
        let text_props = RustTextPropAccess::new_for_window(buffer, self.window_id);
        let (before_strings, after_strings) = text_props.overlay_strings_at(anchor_charpos);
        let overlay_strings = match kind {
            OverlayStringKind::Before => &before_strings,
            OverlayStringKind::After => &after_strings,
        };
        render_overlay_string_batch(
            buffer,
            OverlayStringRenderBatchSource::new(
                overlay_strings,
                CharPos0::new(anchor_charpos as usize),
                kind,
            ),
            self.row_context,
            state,
        );
    }
}

pub(crate) fn render_overlay_string_batch<B: LayoutBufferView>(
    buffer: &B,
    source_batch: OverlayStringRenderBatchSource<'_>,
    row_context: OverlayStringRenderRowContext<'_>,
    state: &mut OverlayStringRenderState<'_>,
) {
    if source_batch.is_empty() {
        return;
    }
    for overlay_string in source_batch.overlay_strings() {
        render_overlay_string(
            buffer,
            source_batch.source_for(*overlay_string),
            row_context,
            state,
        );
    }
}

#[derive(Clone, Copy)]
struct OverlayStringRowBreakRenderContext<'a> {
    anchor_charpos: i64,
    row_context: OverlayStringRenderRowContext<'a>,
}

impl<'a> OverlayStringRowBreakRenderContext<'a> {
    fn new(anchor_charpos: i64, row_context: OverlayStringRenderRowContext<'a>) -> Self {
        Self {
            anchor_charpos,
            row_context,
        }
    }

    fn finish_row(self, state: &mut OverlayStringRenderState<'_>) -> bool {
        let content_x = self.row_context.content_x();
        let geometry_transition = DisplayRowLineBreakTransitionRequest::new(
            state.hit_row_range.range_to(self.anchor_charpos),
            self.row_context.geometry_defaults(),
            self.row_context.row_base,
            0,
            content_x,
            0.0,
            DisplayRowYRecording::None,
            self.row_context.max_rows,
        )
        .finish_geometry(state.geometry, state.hit_rows);

        state.hit_row_range.advance_to(self.anchor_charpos);
        if !state
            .geometry
            .is_within_row_limit(self.row_context.row_limit())
        {
            state
                .source_render
                .output_render()
                .finish_and_end_text_matrix_row_output(geometry_transition.finished_row);
            return false;
        }

        state.geometry.record_current_row_y(state.row_y_positions);
        *state.x = content_x;
        *state.col = 0;
        state
            .source_render
            .output_render()
            .emit_text_matrix_row_transition(geometry_transition);
        true
    }
}

fn render_overlay_string<B: LayoutBufferView>(
    buffer: &B,
    source_request: OverlayStringRenderSource,
    row_context: OverlayStringRenderRowContext<'_>,
    state: &mut OverlayStringRenderState<'_>,
) {
    let anchor_charpos = source_request.anchor_i64();
    let text_value = source_request.value();
    if text_value.as_lisp_string().is_none() {
        return;
    }
    let text_props = get_string_text_properties_table_for_value(text_value);
    let base_face = state.source_render.display_string_base_face(
        buffer,
        source_request.origin(),
        source_request.base_face_policy(),
        state.face_ids,
    );
    let max_x = row_context.right_edge();
    let row_limit = row_context.row_limit();
    let row_break_context = OverlayStringRowBreakRenderContext::new(anchor_charpos, row_context);

    let append_request = source_request.append_request(DisplayRowPosition {
        x_px: *state.x,
        col: *state.col,
    });
    let Some(mut source_context) = LispStringSourceRowAppendSession::new(
        append_request,
        base_face.face_id(),
        base_face.face(),
        row_context.append_surface,
        0.0,
        row_context.char_h,
        row_context.default_row_ascent,
        row_context.face_char_w,
        row_context.char_h,
    ) else {
        return;
    };

    while state.geometry.is_within_row_limit(row_limit) {
        if *state.x >= max_x {
            break;
        }

        let Some(outcome) = source_context.render_to_text_row_and_emit(
            &mut state.source_render,
            state.face_ids,
            state.geometry,
            DisplayRowPosition {
                x_px: *state.x,
                col: *state.col,
            },
        ) else {
            break;
        };
        let stop = outcome.stop();
        outcome.include_vertical_metrics(state.geometry);
        let overlay_cursor_visual_state = row_context.cursor_visual_state(base_face.face());
        for slot in outcome.source_slots() {
            capture_overlay_string_cursor_at_slot(
                text_props.as_ref(),
                slot,
                state.cursor_info,
                state.geometry.y(),
                state.geometry.row(),
                overlay_cursor_visual_state,
            );
        }
        let end = outcome.end_position();
        *state.x = end.x_px;
        *state.col = end.col;

        if stop == DisplayRowRenderStop::RowBreak {
            if !row_break_context.finish_row(state) {
                break;
            }
            continue;
        }
        match stop {
            DisplayRowRenderStop::SourceExhausted => break,
            DisplayRowRenderStop::Clipped => {
                if source_context.discard_pending_until_row_break() {
                    if !row_break_context.finish_row(state) {
                        break;
                    }
                    continue;
                }
                break;
            }
            DisplayRowRenderStop::RowBreak => unreachable!("row break handled above"),
        }
    }
}

fn root_lisp_position_char(source: &crate::display_item::DisplaySourcePosition) -> Option<usize> {
    match source {
        crate::display_item::DisplaySourcePosition::LispString {
            source_id,
            char_index,
            ..
        } if source_id.get() == LispStringSourceId::OVERLAY_STRING.raw() => Some(*char_index),
        _ => None,
    }
}

fn capture_overlay_string_cursor_at_slot(
    text_props: Option<&neovm_core::buffer::text_props::TextPropertyTable>,
    slot: &DisplayRowGlyphSlot,
    cursor_info: &mut CursorCaptureState,
    y: f32,
    matrix_row: usize,
    visual_state: CapturedCursorVisualState,
) {
    let Some(char_idx) = root_lisp_position_char(&slot.source) else {
        return;
    };
    capture_overlay_string_cursor(
        text_props,
        char_idx,
        cursor_info,
        slot.x_px,
        y,
        slot.col,
        matrix_row,
        visual_state,
        CapturedCursorSlotWidth::Explicit(slot.width_px),
    );
}

#[allow(clippy::too_many_arguments)]
fn capture_overlay_string_cursor(
    text_props: Option<&neovm_core::buffer::text_props::TextPropertyTable>,
    char_idx: usize,
    cursor_info: &mut CursorCaptureState,
    x: f32,
    y: f32,
    col: usize,
    matrix_row: usize,
    visual_state: CapturedCursorVisualState,
    slot_width: CapturedCursorSlotWidth,
) {
    if cursor_info.is_captured() {
        return;
    }
    let Some(props) = text_props else {
        return;
    };
    let Some(cursor_prop) =
        props.get_property_at_char_pos(CharPos0::new(char_idx), Value::symbol("cursor"))
    else {
        return;
    };
    if cursor_prop.is_nil() {
        return;
    }

    cursor_info.capture_once(CapturedCursorInfo::from_visual_state(
        visual_state,
        CapturedCursorPlacement {
            x,
            y,
            byte_idx: 0,
            col,
            matrix_row,
            slot_width,
            stretch_like: false,
        },
    ));
}

pub(crate) struct LispStringSourceRowAppendContext<'a> {
    source_context: LispStringSourceAppendContext<'a>,
    append_surface: &'a DisplayRowAppendSurface,
    glyph_y_offset: f32,
    metrics: DisplayRowAppendMetrics,
}

impl<'a> LispStringSourceRowAppendContext<'a> {
    pub(crate) fn render_to_text_row_and_emit(
        &mut self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        geometry: &DisplayRowGeometryState,
        position: DisplayRowPosition,
    ) -> Option<CurrentTextRowRenderOutcome> {
        let frame = DisplayRowTextAppendContext::new(
            self.append_surface,
            geometry,
            self.glyph_y_offset,
            self.metrics.default_row_height,
        )
        .text_row_frame(
            self.metrics.height,
            self.metrics.ascent,
            self.metrics.char_width,
        );
        self.source_context
            .render_to_text_row_and_emit(state, face_ids, frame, position)
    }
}

struct LispStringSourceRowAppendSession<'a> {
    source_session: LispStringSourceAppendSession<'a>,
    append_surface: &'a DisplayRowAppendSurface,
    glyph_y_offset: f32,
    metrics: DisplayRowAppendMetrics,
}

impl<'a> LispStringSourceRowAppendSession<'a> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        request: LispStringSourceAppendRequest,
        base_face_id: u32,
        base_face: &'a ResolvedFace,
        append_surface: &'a DisplayRowAppendSurface,
        glyph_y_offset: f32,
        height: f32,
        ascent: f32,
        char_width: f32,
        default_row_height: f32,
    ) -> Option<Self> {
        let source_session = LispStringSourceAppendSession::new(request, base_face_id, base_face)?;
        Some(Self {
            source_session,
            append_surface,
            glyph_y_offset,
            metrics: DisplayRowAppendMetrics::text_row(
                height,
                ascent,
                char_width,
                default_row_height,
            ),
        })
    }

    fn append_context(&mut self) -> LispStringSourceRowAppendContext<'_> {
        LispStringSourceRowAppendContext {
            source_context: self.source_session.append_context(),
            append_surface: self.append_surface,
            glyph_y_offset: self.glyph_y_offset,
            metrics: self.metrics,
        }
    }

    pub(crate) fn render_to_text_row_and_emit(
        &mut self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        geometry: &DisplayRowGeometryState,
        position: DisplayRowPosition,
    ) -> Option<CurrentTextRowRenderOutcome> {
        self.append_context()
            .render_to_text_row_and_emit(state, face_ids, geometry, position)
    }

    pub(crate) fn discard_pending_until_row_break(&mut self) -> bool {
        self.source_session.discard_pending_until_row_break()
    }
}

pub(crate) fn synthetic_display_text_item(
    source: SyntheticTextSource,
    face_id: u32,
) -> DisplayItem {
    let source_id = source.source_id();
    let text = source.into_text();
    let char_len = text.chars().count();
    DisplayItem::new(
        SourceSpan::synthetic(source_id, 0, char_len),
        RenderFaceRef::FaceId(face_id),
        DisplayItemKind::TextRun(DisplayTextRun::new(text)),
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SyntheticTextSource {
    source_id: u64,
    text: Box<str>,
}

impl SyntheticTextSource {
    #[cfg(test)]
    pub(crate) fn new(source_id: u64, text: impl Into<Box<str>>) -> Self {
        Self {
            source_id,
            text: text.into(),
        }
    }

    fn marker(marker: SyntheticTextMarker) -> Self {
        Self {
            source_id: marker.source_id(),
            text: marker.text().into(),
        }
    }

    fn source_id(&self) -> u64 {
        self.source_id
    }

    fn into_text(self) -> Box<str> {
        self.text
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SyntheticTextAppendRequest {
    position: DisplayRowPosition,
    source: SyntheticTextSource,
    face: SyntheticTextAppendFace,
}

#[derive(Clone, Debug)]
enum SyntheticTextAppendFace {
    ActiveFace,
    TextRowMetrics {
        face_id: u32,
        base_face: ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    },
}

impl SyntheticTextAppendRequest {
    #[cfg(test)]
    pub(crate) fn active_source(position: DisplayRowPosition, source: SyntheticTextSource) -> Self {
        Self {
            position,
            source,
            face: SyntheticTextAppendFace::ActiveFace,
        }
    }

    pub(crate) fn active_marker(position: DisplayRowPosition, marker: SyntheticTextMarker) -> Self {
        Self {
            position,
            source: SyntheticTextSource::marker(marker),
            face: SyntheticTextAppendFace::ActiveFace,
        }
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_row_metrics_source(
        position: DisplayRowPosition,
        source: SyntheticTextSource,
        face_id: u32,
        base_face: &ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    ) -> Self {
        Self {
            position,
            source,
            face: SyntheticTextAppendFace::TextRowMetrics {
                face_id,
                base_face: base_face.clone(),
                height_px,
                ascent_px,
                char_width_px,
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_row_metrics_marker(
        position: DisplayRowPosition,
        marker: SyntheticTextMarker,
        face_id: u32,
        base_face: &ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    ) -> Self {
        Self {
            position,
            source: SyntheticTextSource::marker(marker),
            face: SyntheticTextAppendFace::TextRowMetrics {
                face_id,
                base_face: base_face.clone(),
                height_px,
                ascent_px,
                char_width_px,
            },
        }
    }

    fn into_parts(
        self,
    ) -> (
        DisplayRowPosition,
        SyntheticTextSource,
        SyntheticTextAppendFace,
    ) {
        (self.position, self.source, self.face)
    }
}

#[derive(Clone)]
pub(crate) struct SyntheticTextAppendContext<'a> {
    face_id: u32,
    base_face: &'a ResolvedFace,
    frame: DisplayRowAppendFrame,
}

impl<'a> SyntheticTextAppendContext<'a> {
    pub(crate) fn new(
        face_id: u32,
        base_face: &'a ResolvedFace,
        frame: DisplayRowAppendFrame,
    ) -> Self {
        Self {
            face_id,
            base_face,
            frame,
        }
    }

    pub(crate) fn append_to_text_row_and_emit(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        position: DisplayRowPosition,
        source: SyntheticTextSource,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        append_synthetic_text_to_display_row(
            state,
            self.base_face,
            self.frame.clone(),
            position,
            source,
            self.face_id,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SyntheticTextMarker {
    InvisibleEllipsis,
    HscrollTruncation,
    SelectiveEllipsis,
}

impl SyntheticTextMarker {
    fn source_id(self) -> u64 {
        match self {
            Self::InvisibleEllipsis => SYNTHETIC_SOURCE_INVISIBLE_ELLIPSIS,
            Self::HscrollTruncation => SYNTHETIC_SOURCE_HSCROLL_TRUNCATION,
            Self::SelectiveEllipsis => SYNTHETIC_SOURCE_SELECTIVE_ELLIPSIS,
        }
    }

    fn text(self) -> &'static str {
        match self {
            Self::InvisibleEllipsis | Self::SelectiveEllipsis => "...",
            Self::HscrollTruncation => "$",
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SyntheticTextRowAppendContext<'a> {
    active_face_context: DisplayRowActiveFaceAppendContext<'a, 'a>,
}

impl<'a> SyntheticTextRowAppendContext<'a> {
    pub(crate) fn new(
        append_surface: &'a DisplayRowAppendSurface,
        geometry: &'a DisplayRowGeometryState,
        active_face: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            active_face_context: DisplayRowActiveFaceAppendContext::new(
                append_surface,
                geometry,
                active_face,
                glyph_y_offset,
                default_row_height,
            ),
        }
    }

    fn active_face(
        self,
        face_id: u32,
        base_face: &'a ResolvedFace,
    ) -> SyntheticTextAppendContext<'a> {
        SyntheticTextAppendContext::new(
            face_id,
            base_face,
            self.active_face_context.active_face_frame(),
        )
    }

    fn text_row<'face>(
        self,
        face_id: u32,
        base_face: &'face ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    ) -> SyntheticTextAppendContext<'face> {
        SyntheticTextAppendContext::new(
            face_id,
            base_face,
            self.active_face_context
                .text_row_frame(height_px, ascent_px, char_width_px),
        )
    }

    pub(crate) fn append_request_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        request: SyntheticTextAppendRequest,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let (position, source, face) = request.into_parts();
        match face {
            SyntheticTextAppendFace::ActiveFace => {
                let active_face = self.active_face_context.active_face;
                self.active_face(active_face.face_id(), active_face.resolved_face())
                    .append_to_text_row_and_emit(state, position, source)
            }
            SyntheticTextAppendFace::TextRowMetrics {
                face_id,
                base_face,
                height_px,
                ascent_px,
                char_width_px,
            } => self
                .text_row(face_id, &base_face, height_px, ascent_px, char_width_px)
                .append_to_text_row_and_emit(state, position, source),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BufferSyntheticTextRenderContext<'a> {
    append_surface: &'a DisplayRowAppendSurface,
    active_face: &'a DisplayRowActiveFaceState,
    glyph_y_offset: f32,
    default_row_height: f32,
    default_row_ascent: f32,
    default_char_width: f32,
}

impl<'a> BufferSyntheticTextRenderContext<'a> {
    pub(crate) fn new(
        append_surface: &'a DisplayRowAppendSurface,
        active_face: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
        default_row_ascent: f32,
        default_char_width: f32,
    ) -> Self {
        Self {
            append_surface,
            active_face,
            glyph_y_offset,
            default_row_height,
            default_row_ascent,
            default_char_width,
        }
    }

    fn row_context(
        self,
        geometry: &'a DisplayRowGeometryState,
    ) -> SyntheticTextRowAppendContext<'a> {
        SyntheticTextRowAppendContext::new(
            self.append_surface,
            geometry,
            self.active_face,
            self.glyph_y_offset,
            self.default_row_height,
        )
    }

    pub(crate) fn render_request_to_text_row<'face>(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        geometry: &'a DisplayRowGeometryState,
        request: SyntheticTextAppendRequest,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.row_context(geometry)
            .append_request_to_text_row_and_emit(state, request)
    }

    #[cfg(test)]
    pub(crate) fn render_active_marker_to_text_row(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        geometry: &'a DisplayRowGeometryState,
        position: DisplayRowPosition,
        marker: SyntheticTextMarker,
    ) -> Option<DisplayRowPosition> {
        self.render_request_to_text_row(
            state,
            geometry,
            SyntheticTextAppendRequest::active_marker(position, marker),
        )
        .map(|(_progress, position)| position)
    }

    pub(crate) fn hscroll_truncation_request(
        self,
        base_face: ResolvedFace,
        content_x: f32,
    ) -> SyntheticTextAppendRequest {
        SyntheticTextAppendRequest::text_row_metrics_marker(
            DisplayRowPosition {
                x_px: content_x,
                col: 0,
            },
            SyntheticTextMarker::HscrollTruncation,
            BasicFaceId::Default.into(),
            &base_face,
            self.default_row_height,
            self.default_row_ascent,
            self.default_char_width,
        )
    }

    #[cfg(test)]
    pub(crate) fn render_hscroll_truncation_marker_to_text_row(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        geometry: &'a DisplayRowGeometryState,
        content_x: f32,
    ) -> Option<DisplayRowPosition> {
        let request = self.hscroll_truncation_request(state.default_face(), content_x);
        self.render_request_to_text_row(state, geometry, request)
            .map(|(_progress, position)| position)
    }
}

pub(crate) fn render_face_ref_id(face: RenderFaceRef, fallback: u32) -> u32 {
    match face {
        RenderFaceRef::FaceId(face_id) => face_id,
        RenderFaceRef::Inherit => fallback,
    }
}

#[cfg(test)]
pub(crate) fn apply_pending_display_source_faces(
    builder: &mut GlyphMatrixBuilder,
    pending_faces: &mut Vec<PendingDisplaySourceFace>,
) {
    for pending in pending_faces.drain(..) {
        crate::display_row::insert_resolved_display_row_face(
            builder,
            pending.face_id,
            &pending.resolved,
            None,
        );
    }
}

#[cfg(test)]
pub(crate) fn append_lisp_string_to_text_row(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    text_value: Value,
    source_id: u64,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    base_face_id: u32,
    face_ids: &mut FrameFaceIdAllocator,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> DisplayRowPosition {
    let request =
        LispStringSourceAppendRequest::new(position, LispStringSourceId(source_id), text_value);
    let Some(mut source_session) =
        LispStringSourceAppendSession::new(request, base_face_id, base_face)
    else {
        return position;
    };
    let mut font_metrics = None;
    source_session
        .render_to_text_row_and_emit(
            &mut TextRowSourceRenderState::new(
                builder,
                output_emitter,
                evaluator,
                &mut font_metrics,
                face_resolver,
            ),
            face_ids,
            frame,
            position,
        )
        .map(|outcome| outcome.end_position())
        .unwrap_or(position)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BufferTextSourceRange {
    start: CharPos0,
    end: CharPos0,
}

impl BufferTextSourceRange {
    pub(crate) fn new(start: CharPos0, end: CharPos0) -> Self {
        Self { start, end }
    }

    pub(crate) fn single_char(start: CharPos0) -> Self {
        Self::new(start, start.add_len(CharLen::new(1)))
    }

    fn start(self) -> CharPos0 {
        self.start
    }

    fn end(self) -> CharPos0 {
        self.end
    }

    fn is_single_char(self) -> bool {
        self.end == self.start.add_len(CharLen::new(1))
    }

    fn is_empty_or_reversed(self) -> bool {
        self.end <= self.start
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

struct BufferTextSourceAppendOperation<'face> {
    append_item: BufferTextSourceRangeItemAppendRequest,
    base_face: &'face ResolvedFace,
    face_id: u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
}

impl<'face> BufferTextSourceAppendOperation<'face> {
    fn new(
        append_item: BufferTextSourceRangeItemAppendRequest,
        base_face: &'face ResolvedFace,
        face_id: u32,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
    ) -> Self {
        Self {
            append_item,
            base_face,
            face_id,
            frame,
            position,
        }
    }

    fn for_buffer_text_range<B: LayoutBufferView + ?Sized>(
        range: BufferTextSourceRange,
        buffer_id: BufferId,
        buffer: &B,
        face_id: u32,
        base_face: &'face ResolvedFace,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
    ) -> Option<Self> {
        let append_item =
            buffer_text_source_range_append_request(range, buffer_id, buffer, face_id)?;
        Some(Self::new(append_item, base_face, face_id, frame, position))
    }

    fn for_buffer_text_request<B: LayoutBufferView + ?Sized>(
        source_text: BufferTextSourceTextRequest,
        buffer_id: BufferId,
        buffer: &B,
        face_id: u32,
        base_face: &'face ResolvedFace,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
    ) -> Option<Self> {
        Self::for_buffer_text_range(
            source_text.range(),
            buffer_id,
            buffer,
            face_id,
            base_face,
            frame,
            position,
        )
    }

    fn for_display_item_request<B: LayoutBufferView + ?Sized>(
        source_item: BufferTextSourceItemRequest,
        buffer_id: BufferId,
        buffer: &B,
        face_id: u32,
        base_face: &'face ResolvedFace,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
    ) -> Option<Self> {
        let append_item =
            buffer_text_source_item_append_request(source_item, buffer_id, buffer, face_id)?;
        Some(Self::new(append_item, base_face, face_id, frame, position))
    }

    fn request(&self) -> DisplayRowSourceAppendRequest<'face> {
        self.frame.source_append_request(
            self.position,
            self.face_id,
            self.base_face,
            self.append_item.append_kind(),
        )
    }

    fn render_to_text_row_and_emit<P: DisplayRowRenderPolicy>(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        render_policy: &mut P,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let mut face_ids = FrameFaceIdAllocator::new(self.face_id.saturating_add(1));
        let position = self.position;
        let request = self.request();
        let outcome = request.render_single_display_item_into_current_text_row_and_emit(
            &mut current_text_render_state(state, &mut face_ids),
            self.append_item.into_item(),
            render_policy,
        )?;
        Some(outcome.into_append_progress_and_position(position))
    }

    fn measure_to_text_row<P: DisplayRowRenderPolicy>(
        self,
        state: &mut TextRowSourceMeasureState<'_>,
        render_policy: &mut P,
    ) -> Option<DisplayRowAppendProgress> {
        let mut face_ids = FrameFaceIdAllocator::new(self.face_id.saturating_add(1));
        let position = self.position;
        let request = self
            .request()
            .with_measurement_bounds(DisplayRowRenderBounds::unbounded_from(position));
        let outcome = request.measure_single_display_item_against_current_text_row(
            &mut current_text_measure_state(state, &mut face_ids),
            self.append_item.into_item(),
            render_policy,
        )?;
        Some(outcome.into_append_progress(position))
    }
}

struct DisplayRowSingleItemAppendOperation<'face> {
    item: DisplayItem,
    base_face: &'face ResolvedFace,
    fallback_face_id: u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    kind: DisplayRowAppendKind,
}

impl<'face> DisplayRowSingleItemAppendOperation<'face> {
    fn new(
        item: DisplayItem,
        base_face: &'face ResolvedFace,
        fallback_face_id: u32,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
        kind: DisplayRowAppendKind,
    ) -> Self {
        Self {
            item,
            base_face,
            fallback_face_id,
            frame,
            position,
            kind,
        }
    }

    fn request_face_id(&self) -> u32 {
        render_face_ref_id(self.item.face, self.fallback_face_id)
    }

    fn request(&self) -> DisplayRowSourceAppendRequest<'face> {
        self.frame.source_append_request(
            self.position,
            self.request_face_id(),
            self.base_face,
            self.kind,
        )
    }

    fn render_to_text_row_and_emit<P: DisplayRowRenderPolicy>(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        render_policy: &mut P,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let request = self.request();
        let base_face_id = request.base_face_id();
        let mut item = self.item;
        item.face = RenderFaceRef::FaceId(base_face_id);
        let mut face_ids = FrameFaceIdAllocator::new(base_face_id.saturating_add(1));
        let start = request.start_position();
        let outcome = request.render_single_display_item_into_current_text_row_and_emit(
            &mut current_text_render_state(state, &mut face_ids),
            item,
            render_policy,
        )?;
        Some(outcome.into_append_progress_and_position(start))
    }
}

struct DisplayRowSourceAppendOperation<'face> {
    base_face: &'face ResolvedFace,
    base_face_id: u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    kind: DisplayRowAppendKind,
}

impl<'face> DisplayRowSourceAppendOperation<'face> {
    fn new(
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

    fn render_source_cursor_to_text_row_and_emit<S: DisplayItemSource>(
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

fn buffer_text_source_range_append_request<B: LayoutBufferView + ?Sized>(
    range: BufferTextSourceRange,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
) -> Option<BufferTextSourceRangeItemAppendRequest> {
    if !range.is_single_char() {
        return None;
    }

    let start = range.start();
    let end = range.end();
    let byte_start = buffer.layout_char_pos_to_emacs_byte_pos(start);
    let byte_end = buffer.layout_char_pos_to_emacs_byte_pos(end);
    let mut bytes = Vec::new();
    buffer.layout_copy_emacs_byte_range_to(EmacsByteRange::new(byte_start, byte_end), &mut bytes);
    let (ch, len) = decode_utf8(&bytes);
    if len == 0 {
        return None;
    }

    let item = BufferTextItemSource::single_char(buffer_id, start, byte_start, byte_end).item(
        RenderFaceRef::FaceId(face_id),
        DisplayItemKind::TextRun(DisplayTextRun::new(ch.to_string())),
    );
    let append_kind = if ch == '\t' {
        DisplayRowAppendKind::Tab
    } else {
        DisplayRowAppendKind::SourceText
    };
    Some(BufferTextSourceRangeItemAppendRequest::new(
        item,
        append_kind,
    ))
}

#[cfg(test)]
struct BufferTextSourceRangeAppendContext<'a, B: LayoutBufferView + ?Sized> {
    buffer: &'a B,
    buffer_id: BufferId,
    face_id: u32,
    base_face: &'a ResolvedFace,
    frame: DisplayRowAppendFrame,
}

#[cfg(test)]
impl<'a, B: LayoutBufferView + ?Sized> BufferTextSourceRangeAppendContext<'a, B> {
    fn new(
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

    #[cfg(test)]
    fn resolve_source_advance_request_to_text_row(
        &self,
        state: &mut BufferTextRowAppendState,
        measure_state: &mut TextRowSourceMeasureState<'_>,
        active_face_state: &DisplayRowActiveFaceState,
        request: BufferTextSourceAdvanceRequest<'_>,
    ) -> ResolvedBufferTextSourceAdvance {
        state
            .advance_resolver()
            .resolve_source_advance_request_to_text_row(
                measure_state,
                self.buffer_id,
                self.buffer,
                active_face_state,
                self.frame.clone(),
                request,
            )
    }

    #[cfg(test)]
    fn append_source_text_request_to_text_row(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        source_text: BufferTextSourceTextRequest,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let operation = BufferTextSourceAppendOperation::for_buffer_text_request(
            source_text,
            self.buffer_id,
            self.buffer,
            self.face_id,
            self.base_face,
            self.frame.clone(),
            position,
        )?;
        let mut render_policy =
            DisplaySourceAppendRenderPolicy::new(source_text.append_measurement());
        operation.render_to_text_row_and_emit(state, &mut render_policy)
    }

    #[cfg(test)]
    fn measure_source_range_natural_advance_to_text_row(
        &self,
        state: &mut TextRowSourceMeasureState<'_>,
        range: BufferTextSourceRange,
        position: DisplayRowPosition,
    ) -> Option<f32> {
        measure_buffer_text_source_range_natural_advance_to_text_row(
            state,
            range,
            self.base_face,
            self.buffer_id,
            self.buffer,
            self.face_id,
            self.frame.clone(),
            position,
        )
    }
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

    fn measure_item_source_request_width_or_active_face_fallback_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceMeasureState<'_>,
        source_item: BufferTextSourceItemRequest,
        position: DisplayRowPosition,
    ) -> f32 {
        self.item_active_face(geometry)
            .measure_source_request_width_or_active_face_fallback_to_text_row(
                state,
                source_item,
                position,
            )
    }

    fn measure_special_source_char_request_width_or_active_face_fallback_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceMeasureState<'_>,
        request: BufferTextSpecialSourceCharMeasureRequest,
    ) -> f32 {
        let position = request.position();
        self.measure_item_source_request_width_or_active_face_fallback_to_text_row(
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
            self.measure_special_source_char_request_width_or_active_face_fallback_to_text_row(
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
        request: BufferTextSourceAdvanceRequest<'_>,
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
        request: BufferTextSourceAdvanceRequest<'_>,
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
        let operation = BufferTextSourceAppendOperation::for_buffer_text_request(
            source_text,
            self.buffer_id,
            self.buffer,
            self.active_face.face_id(),
            self.active_face.resolved_face(),
            frame,
            position,
        )?;
        let mut render_policy =
            DisplaySourceAppendRenderPolicy::new(source_text.append_measurement());
        operation.render_to_text_row_and_emit(state, &mut render_policy)
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
    #[cfg(test)]
    pub(crate) fn new(
        append_state: &'a mut BufferTextRowAppendState,
        builder: &'a mut GlyphMatrixBuilder,
        evaluator: &'a mut Context,
        font_metrics: &'a mut Option<FontMetricsService>,
        face_resolver: &'a FaceResolver,
    ) -> Self {
        Self {
            append_state,
            measure: TextRowSourceMeasureState::new(
                builder,
                evaluator,
                font_metrics,
                face_resolver,
            ),
        }
    }

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
pub(crate) enum BufferTextSourceCharOverflowAction {
    Fits,
    Truncate {
        transition: DisplayRowOverflowTransitionPlan,
    },
    WordWrap {
        break_candidate: WordWrapBreakCandidate,
        transition: DisplayRowOverflowTransitionPlan,
    },
    CharacterWrap {
        transition: DisplayRowOverflowTransitionPlan,
    },
}

impl BufferTextSourceCharOverflowAction {
    fn for_decision(decision: BufferTextRowOverflowDecision) -> Self {
        match decision {
            BufferTextRowOverflowDecision::Fits => Self::Fits,
            BufferTextRowOverflowDecision::Truncate => Self::Truncate {
                transition: DisplayRowOverflowTransitionPlan::truncation(
                    TextRowTransitionStatePolicy::truncation(),
                ),
            },
            BufferTextRowOverflowDecision::WordWrap { break_candidate } => Self::WordWrap {
                break_candidate,
                transition: DisplayRowOverflowTransitionPlan::visual_wrap(
                    TextRowTransitionStatePolicy::visual_wrap(),
                ),
            },
            BufferTextRowOverflowDecision::CharacterWrap => Self::CharacterWrap {
                transition: DisplayRowOverflowTransitionPlan::visual_wrap(
                    TextRowTransitionStatePolicy::character_wrap(),
                ),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextSourceCharPreparedAppend {
    plan: BufferTextSourceCharAppendPlan,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextOverflowRenderRequest {
    prepared_append: BufferTextSourceCharPreparedAppend,
    decoded_source_char: BufferTextDecodedSourceChar,
    ch: char,
    right_edge_px: f32,
    truncate_lines: bool,
    word_wrap: WordWrapRenderState,
    row_visibility_limit: DisplayRowVisibilityLimit,
    content_x: f32,
    has_prefix: bool,
    row_geometry_defaults: DisplayRowGeometryDefaults,
    text_matrix_row_base: usize,
    max_rows: usize,
    row_limit: DisplayRowLimit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextOverflowRenderOutcome {
    Fits,
    Transition(DisplayRowTransitionContinuation),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextSourceAppendContinuation {
    Rendered,
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextSourceCharRenderOutcome {
    Rendered,
    ContinueBufferWalk,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferSelectiveDisplayTailRenderOutcome {
    NotHidden,
    ContinueBufferWalk,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferInvisibleTextRenderOutcome {
    Visible,
    ContinueBufferWalk,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferEndOfBufferTailRenderOutcome {
    point_is_visible_eob: bool,
}

pub(crate) struct BufferTextSourceCharRenderState<'a> {
    source_render: TextRowSourceRenderState<'a>,
    trailing_whitespace: &'a mut TrailingWhitespaceRenderState,
    word_wrap: &'a mut WordWrapRenderState,
    x: &'a mut f32,
    col: &'a mut usize,
    charpos: &'a mut i64,
}

impl<'a> BufferTextSourceCharRenderState<'a> {
    pub(crate) fn new(
        source_render: TextRowSourceRenderState<'a>,
        trailing_whitespace: &'a mut TrailingWhitespaceRenderState,
        word_wrap: &'a mut WordWrapRenderState,
        x: &'a mut f32,
        col: &'a mut usize,
        charpos: &'a mut i64,
    ) -> Self {
        Self {
            source_render,
            trailing_whitespace,
            word_wrap,
            x,
            col,
            charpos,
        }
    }
}

pub(crate) struct BufferTextSpecialSourceCharRenderState<'a> {
    face_ids: &'a mut FrameFaceIdAllocator,
    source_render: TextRowSourceRenderState<'a>,
    face_scan: &'a mut FaceScanCheckpoint,
    word_wrap: &'a mut WordWrapRenderState,
    x: &'a mut f32,
    col: &'a mut usize,
    charpos: &'a mut i64,
}

impl<'a> BufferTextSpecialSourceCharRenderState<'a> {
    pub(crate) fn new(
        face_ids: &'a mut FrameFaceIdAllocator,
        source_render: TextRowSourceRenderState<'a>,
        face_scan: &'a mut FaceScanCheckpoint,
        word_wrap: &'a mut WordWrapRenderState,
        x: &'a mut f32,
        col: &'a mut usize,
        charpos: &'a mut i64,
    ) -> Self {
        Self {
            face_ids,
            source_render,
            face_scan,
            word_wrap,
            x,
            col,
            charpos,
        }
    }
}

impl BufferTextSourceAppendContinuation {
    pub(crate) fn should_break(self) -> bool {
        matches!(self, Self::Stopped)
    }
}

impl BufferTextSourceCharRenderOutcome {
    pub(crate) fn should_break(self) -> bool {
        matches!(self, Self::Stop)
    }

    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(self, Self::ContinueBufferWalk)
    }
}

impl BufferSelectiveDisplayTailRenderOutcome {
    pub(crate) fn should_break(self) -> bool {
        matches!(self, Self::Stop)
    }

    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(self, Self::ContinueBufferWalk)
    }
}

impl BufferInvisibleTextRenderOutcome {
    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(self, Self::ContinueBufferWalk)
    }
}

impl BufferEndOfBufferTailRenderOutcome {
    pub(crate) fn point_is_visible_eob(self) -> bool {
        self.point_is_visible_eob
    }
}

impl BufferTextWindowTailFinalizeOutcome {
    #[cfg(test)]
    pub(crate) fn cursor_requested(self) -> bool {
        self.cursor_requested
    }

    #[cfg(test)]
    pub(crate) fn cursor_published(self) -> bool {
        self.cursor_published
    }

    #[cfg(test)]
    pub(crate) fn visual_cursor_summary(self) -> VisualTextWindowCursorPublishSummary {
        self.visual_cursor_summary
    }

    #[cfg(test)]
    pub(crate) fn pending_row_finished(self) -> bool {
        self.pending_row_finished
    }
}

impl BufferTextWindowBeginRequest {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        frame_id: FrameId,
        window_id: WindowId,
        text_matrix_row_base: usize,
        text_area_left: f32,
        window_top: f32,
        matrix_window_id: u64,
        matrix_rows: usize,
        matrix_cols: usize,
        bounds: neomacs_display_protocol::types::Rect,
        text_bounds: neomacs_display_protocol::types::Rect,
        selected: bool,
        first_row: TextMatrixRowBegin,
    ) -> Self {
        Self {
            frame_id,
            window_id,
            text_matrix_row_base,
            text_area_left,
            window_top,
            matrix_window_id,
            matrix_rows,
            matrix_cols,
            bounds,
            text_bounds,
            selected,
            first_row,
        }
    }

    pub(crate) fn begin_and_apply(
        self,
        state: BufferTextWindowBeginState<'_>,
    ) -> WindowOutputEmitter {
        let mut output_emitter = WindowOutputEmitter::new(
            self.frame_id,
            self.window_id,
            self.text_matrix_row_base,
            self.text_area_left,
            self.window_top,
        );
        output_emitter.begin_update(state.evaluator);
        begin_text_window_output(
            state.builder,
            &mut output_emitter,
            state.evaluator,
            TextWindowBegin {
                window_id: self.matrix_window_id,
                rows: self.matrix_rows,
                cols: self.matrix_cols,
                bounds: self.bounds,
                text_bounds: self.text_bounds,
                selected: self.selected,
                first_row: self.first_row,
            },
        );
        output_emitter
    }
}

impl BufferTextWindowCursorEffectsRequest {
    pub(crate) fn new(window_id: i64, effects: Option<EffectsConfig>) -> Self {
        Self { window_id, effects }
    }

    pub(crate) fn install_and_apply(self, builder: &mut GlyphMatrixBuilder) -> bool {
        let Some(effects) = self.effects else {
            return false;
        };
        install_text_window_cursor_effects(
            builder,
            TextWindowCursorEffects {
                window_id: self.window_id,
                effects,
            },
        );
        true
    }
}

impl BufferTextWindowTerminalRightBorderRequest {
    pub(crate) fn new(char_width: f32) -> Self {
        Self {
            ch: '|',
            face_name: "vertical-border",
            char_width,
        }
    }

    pub(crate) fn install_and_apply(
        self,
        builder: &mut GlyphMatrixBuilder,
        face_resolver: &FaceResolver,
    ) -> u32 {
        let border_face = face_resolver.resolve_named_face(self.face_name);
        let border_face_id = border_face.face_id;
        insert_resolved_display_row_face(builder, border_face_id, &border_face, None);
        install_last_window_right_border(
            builder,
            TextWindowRightBorder {
                ch: self.ch,
                face_id: border_face_id,
                char_width: self.char_width,
            },
        );
        border_face_id
    }
}

impl BufferTextOverflowRenderOutcome {
    pub(crate) fn should_break(self) -> bool {
        matches!(
            self,
            Self::Transition(
                DisplayRowTransitionContinuation::Exhausted
                    | DisplayRowTransitionContinuation::Hidden
            )
        )
    }

    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(
            self,
            Self::Transition(DisplayRowTransitionContinuation::Continue)
        )
    }
}

impl<'a> BufferSelectiveDisplayTailRenderRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        source_char: BufferTextDecodedSourceChar,
        text: &'a [u8],
        text_start_byte: usize,
        selective_display: i32,
        tab_width: i32,
        append_surface: &'a DisplayRowAppendSurface,
        active_face_state: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_face_ascent: f32,
        char_h: f32,
        char_w: f32,
        content_x: f32,
        has_prefix: bool,
        row_geometry_defaults: DisplayRowGeometryDefaults,
        text_matrix_row_base: usize,
        max_rows: usize,
        row_limit: DisplayRowLimit,
    ) -> Self {
        Self {
            source_char,
            text,
            text_start_byte,
            selective_display,
            tab_width,
            append_surface,
            active_face_state,
            glyph_y_offset,
            default_face_ascent,
            char_h,
            char_w,
            content_x,
            has_prefix,
            row_geometry_defaults,
            text_matrix_row_base,
            max_rows,
            row_limit,
        }
    }

    pub(crate) fn render_if_needed_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferSelectiveDisplayTailRenderState<'_, '_>,
    ) -> BufferSelectiveDisplayTailRenderOutcome {
        let context =
            BufferSelectiveDisplayContext::new(self.text, self.selective_display, self.tab_width);
        let Some(marker) = context.carriage_return_tail_marker(self.source_char.ch()) else {
            return BufferSelectiveDisplayTailRenderOutcome::NotHidden;
        };

        let BufferSelectiveDisplayTailRenderState {
            byte_idx,
            charpos,
            col,
            source_render,
            row_extend,
            box_face,
            x,
            line_numbers,
            row_geometry,
            row_flags,
            hit_rows,
            hit_row_range,
            prefix_request,
            hscroll_skip,
            word_wrap,
            trailing_whitespace,
            row_y_positions,
        } = state;
        let mut source_render = source_render;

        let mut synthetic_text_state =
            BufferSyntheticTextRenderState::new(source_render.reborrow(), x, col);
        marker.append_to_text_row_and_apply(
            BufferSyntheticTextRenderContext::new(
                self.append_surface,
                self.active_face_state,
                self.glyph_y_offset,
                self.char_h,
                self.default_face_ascent,
                self.char_w,
            ),
            row_geometry,
            &mut synthetic_text_state,
        );

        let tail_action = context.skip_rest_of_line_after_carriage_return(byte_idx, charpos);
        if !tail_action.is_line_break() {
            return BufferSelectiveDisplayTailRenderOutcome::ContinueBufferWalk;
        }

        tail_action.apply_hidden_line_break_row_state(
            row_geometry,
            row_extend,
            box_face,
            self.content_x,
            x,
        );
        let line_break_transition = DisplayRowLineBreakTransitionPlan::hidden_line_break();
        let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
            self.row_geometry_defaults,
            self.text_matrix_row_base,
            row_y_positions,
            self.max_rows,
            row_geometry,
            row_flags,
            self.row_limit,
            hit_rows,
            &mut source_render,
        )
        .emit_line_break_then_row_start(
            line_break_transition,
            hit_row_range.range_to(*charpos),
            DisplayRowPosition {
                x_px: *x,
                col: *col,
            },
            0.0,
            DisplayRowTransitionRenderState::new(
                prefix_request,
                self.has_prefix,
                line_numbers,
                hscroll_skip,
                word_wrap,
                trailing_whitespace,
            ),
            col,
        );
        let synced_charpos = buffer
            .layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(self.text_start_byte + *byte_idx))
            .get() as i64;
        if tail_action
            .apply_after_hidden_line_break_transition(
                row_transition,
                synced_charpos,
                charpos,
                hit_row_range,
            )
            .should_break()
        {
            return BufferSelectiveDisplayTailRenderOutcome::Stop;
        }

        BufferSelectiveDisplayTailRenderOutcome::ContinueBufferWalk
    }
}

impl<'a> BufferTextSourceCharRenderRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        decoded_source_char: BufferTextDecodedSourceChar,
        text: &'a [u8],
        text_start_byte: usize,
        buffer_id: BufferId,
        append_surface: &'a DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
        params: &'a WindowParams,
        glyph_y_offset: f32,
        char_h: f32,
        point_charpos: i64,
        row_visibility_limit: DisplayRowVisibilityLimit,
        content_x: f32,
        has_prefix: bool,
        row_geometry_defaults: DisplayRowGeometryDefaults,
        text_matrix_row_base: usize,
        max_rows: usize,
        row_limit: DisplayRowLimit,
    ) -> Self {
        Self {
            decoded_source_char,
            text,
            text_start_byte,
            buffer_id,
            append_surface,
            overlay_context,
            active_face_state,
            params,
            glyph_y_offset,
            char_h,
            point_charpos,
            row_visibility_limit,
            content_x,
            has_prefix,
            row_geometry_defaults,
            text_matrix_row_base,
            max_rows,
            row_limit,
        }
    }

    pub(crate) fn render_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferTextSourceCharRenderRequestState<'_, '_>,
    ) -> BufferTextSourceCharRenderOutcome {
        let BufferTextSourceCharRenderRequestState {
            append_state,
            byte_idx,
            charpos,
            col,
            source_render,
            row_extend,
            x,
            line_numbers,
            row_geometry,
            row_flags,
            hit_rows,
            hit_row_range,
            prefix_request,
            hscroll_skip,
            word_wrap,
            trailing_whitespace,
            face_scan,
            row_y_positions,
            cursor_info,
            face_ids,
            raise_span,
        } = state;
        let mut source_render = source_render;

        let ch = self.decoded_source_char.ch();
        self.decoded_source_char
            .record_word_wrap_candidate(word_wrap, source_render.output_emitter());

        let buffer_source_char = self
            .decoded_source_char
            .source_char(self.params.nobreak_char_display);
        let buffer_row_append_context = BufferTextRowAppendContext::new(
            buffer,
            self.buffer_id,
            self.append_surface,
            self.active_face_state,
            self.glyph_y_offset,
            self.char_h,
        );
        let append_position = DisplayRowPosition {
            x_px: *x,
            col: *col,
        };
        let append_geometry = *row_geometry;

        let prepared_append = {
            let mut preparation_state = BufferTextSourceCharPreparationState::from_source_render(
                append_state,
                &mut source_render,
            );
            buffer_row_append_context.prepare_source_char_for_current_text_row(
                BufferTextSourceCharPreparationRequest::new(
                    append_geometry,
                    &buffer_source_char,
                    self.text,
                    self.decoded_source_char.start_byte_idx(),
                    append_position,
                ),
                &mut preparation_state,
            )
        };

        let prepared_append = match prepared_append {
            BufferTextPreparedSourceCharAppend::Special(special_prepared_append) => {
                let special_overflow_outcome = BufferTextSpecialOverflowRenderRequest::new(
                    &special_prepared_append,
                    self.text,
                    self.text_start_byte,
                    *x,
                    self.append_surface.full_text_right_edge(),
                    self.params.truncate_lines,
                    self.row_visibility_limit,
                    self.content_x,
                    self.has_prefix,
                    self.row_geometry_defaults,
                    self.text_matrix_row_base,
                    self.max_rows,
                    self.row_limit,
                )
                .render_if_needed_and_apply(
                    buffer,
                    BufferTextSpecialOverflowRenderState {
                        byte_idx,
                        charpos,
                        col,
                        source_render: source_render.reborrow(),
                        row_extend,
                        x,
                        line_numbers,
                        row_geometry,
                        row_flags,
                        hit_rows,
                        hit_row_range,
                        prefix_request,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                        row_y_positions,
                    },
                );
                if special_overflow_outcome.should_break() {
                    return BufferTextSourceCharRenderOutcome::Stop;
                }
                if special_overflow_outcome.should_continue_buffer_walk() {
                    return BufferTextSourceCharRenderOutcome::ContinueBufferWalk;
                }

                if special_prepared_append
                    .append_to_text_row_and_apply(
                        &buffer_row_append_context,
                        row_geometry,
                        self.params,
                        &mut BufferTextSpecialSourceCharRenderState::new(
                            face_ids,
                            source_render.reborrow(),
                            face_scan,
                            word_wrap,
                            x,
                            col,
                            charpos,
                        ),
                    )
                    .should_break()
                {
                    return BufferTextSourceCharRenderOutcome::Stop;
                }
                return BufferTextSourceCharRenderOutcome::ContinueBufferWalk;
            }
            BufferTextPreparedSourceCharAppend::Text(prepared_append) => prepared_append,
        };

        prepared_append.update_cursor_info_for_main_char(
            cursor_info,
            self.decoded_source_char.start_byte_idx(),
        );
        let overflow_outcome = BufferTextOverflowRenderRequest::new(
            prepared_append,
            self.decoded_source_char,
            ch,
            self.append_surface.right_edge(),
            self.params.truncate_lines,
            *word_wrap,
            self.row_visibility_limit,
            self.content_x,
            self.has_prefix,
            self.row_geometry_defaults,
            self.text_matrix_row_base,
            self.max_rows,
            self.row_limit,
        )
        .render_if_needed_and_apply(
            self.text,
            BufferTextOverflowRenderState {
                byte_idx,
                charpos,
                col,
                source_render: source_render.reborrow(),
                row_extend,
                x,
                line_numbers,
                row_geometry,
                row_flags,
                hit_rows,
                hit_row_range,
                prefix_request,
                hscroll_skip,
                word_wrap,
                trailing_whitespace,
                face_scan,
                row_y_positions,
            },
        );
        if overflow_outcome.should_break() {
            return BufferTextSourceCharRenderOutcome::Stop;
        }
        if overflow_outcome.should_continue_buffer_walk() {
            return BufferTextSourceCharRenderOutcome::ContinueBufferWalk;
        }

        BufferDisplayPropertyTextModifierAction::clear_expired_raise_span(
            raise_span,
            *charpos,
            self.params.window_start,
        );

        prepared_append.capture_cursor_info_for_main_char_if_point(
            cursor_info,
            self.active_face_state,
            row_geometry,
            *x,
            self.decoded_source_char.start_byte_idx(),
            *col,
            ch == '\t',
            *charpos,
            self.point_charpos,
        );

        {
            let mut overlay_state = OverlayStringRenderState::from_source_render(
                source_render.reborrow(),
                x,
                col,
                row_geometry,
                cursor_info,
                hit_rows,
                hit_row_range,
                row_y_positions,
                face_ids,
            );
            self.overlay_context.render_before_at(
                buffer,
                *charpos,
                self.active_face_state,
                &mut overlay_state,
            );
        }

        if prepared_append
            .append_to_text_row_and_apply(
                &buffer_row_append_context,
                &append_geometry,
                ch,
                &mut BufferTextSourceCharRenderState::new(
                    source_render.reborrow(),
                    trailing_whitespace,
                    word_wrap,
                    x,
                    col,
                    charpos,
                ),
            )
            .should_break()
        {
            return BufferTextSourceCharRenderOutcome::Stop;
        }

        {
            let mut overlay_state = OverlayStringRenderState::from_source_render(
                source_render.reborrow(),
                x,
                col,
                row_geometry,
                cursor_info,
                hit_rows,
                hit_row_range,
                row_y_positions,
                face_ids,
            );
            self.overlay_context.render_after_at(
                buffer,
                *charpos,
                self.active_face_state,
                &mut overlay_state,
            );
        }

        BufferTextSourceCharRenderOutcome::Rendered
    }
}

impl BufferTextOverflowRenderRequest {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        prepared_append: BufferTextSourceCharPreparedAppend,
        decoded_source_char: BufferTextDecodedSourceChar,
        ch: char,
        right_edge_px: f32,
        truncate_lines: bool,
        word_wrap: WordWrapRenderState,
        row_visibility_limit: DisplayRowVisibilityLimit,
        content_x: f32,
        has_prefix: bool,
        row_geometry_defaults: DisplayRowGeometryDefaults,
        text_matrix_row_base: usize,
        max_rows: usize,
        row_limit: DisplayRowLimit,
    ) -> Self {
        Self {
            prepared_append,
            decoded_source_char,
            ch,
            right_edge_px,
            truncate_lines,
            word_wrap,
            row_visibility_limit,
            content_x,
            has_prefix,
            row_geometry_defaults,
            text_matrix_row_base,
            max_rows,
            row_limit,
        }
    }

    pub(crate) fn render_if_needed_and_apply(
        self,
        text: &[u8],
        state: BufferTextOverflowRenderState<'_, '_>,
    ) -> BufferTextOverflowRenderOutcome {
        let BufferTextOverflowRenderState {
            byte_idx,
            charpos,
            col,
            source_render,
            row_extend,
            x,
            line_numbers,
            row_geometry,
            row_flags,
            hit_rows,
            hit_row_range,
            prefix_request,
            hscroll_skip,
            word_wrap,
            trailing_whitespace,
            face_scan,
            row_y_positions,
        } = state;
        let mut source_render = source_render;

        match self.prepared_append.overflow_action(
            self.ch,
            self.right_edge_px,
            self.truncate_lines,
            self.word_wrap,
        ) {
            BufferTextSourceCharOverflowAction::Fits => BufferTextOverflowRenderOutcome::Fits,
            BufferTextSourceCharOverflowAction::Truncate { transition } => {
                let truncation_skip =
                    BufferTextTruncationSkipAction::consume_decoded_char_and_rest_of_line(
                        text, byte_idx, charpos,
                    );
                truncation_skip.apply_before_row_transition(
                    line_numbers,
                    row_extend,
                    x,
                    self.content_x,
                );
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    self.row_geometry_defaults,
                    self.text_matrix_row_base,
                    row_y_positions,
                    self.max_rows,
                    row_geometry,
                    row_flags,
                    self.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow_then_row_start(
                    transition,
                    hit_row_range.range_to(*charpos),
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
                    DisplayRowTransitionRenderState::new(
                        prefix_request,
                        self.has_prefix,
                        line_numbers,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                    ),
                    col,
                );
                BufferTextOverflowRenderOutcome::Transition(
                    truncation_skip.transition_continuation(row_transition),
                )
            }
            BufferTextSourceCharOverflowAction::WordWrap {
                break_candidate: wrap_break,
                transition,
            } => {
                let word_wrap_action = BufferTextWordWrapSourceAction::new(wrap_break);
                word_wrap_action.apply_before_row_transition(
                    source_render.output_emitter(),
                    byte_idx,
                    charpos,
                    col,
                    row_extend,
                    x,
                    self.content_x,
                );
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    self.row_geometry_defaults,
                    self.text_matrix_row_base,
                    row_y_positions,
                    self.max_rows,
                    row_geometry,
                    row_flags,
                    self.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow(
                    transition,
                    hit_row_range.range_to(*charpos),
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
                );
                BufferTextOverflowRenderOutcome::Transition(
                    word_wrap_action.apply_after_row_transition_and_prefix(
                        row_transition,
                        transition,
                        charpos,
                        hit_row_range,
                        face_scan,
                        row_geometry,
                        self.row_visibility_limit,
                        DisplayRowTransitionRenderState::new(
                            prefix_request,
                            self.has_prefix,
                            line_numbers,
                            hscroll_skip,
                            word_wrap,
                            trailing_whitespace,
                        ),
                    ),
                )
            }
            BufferTextSourceCharOverflowAction::CharacterWrap { transition } => {
                let character_wrap_action = BufferTextCharacterWrapSourceAction::from_decoded_char(
                    self.decoded_source_char,
                );
                character_wrap_action.apply_before_row_transition(row_extend, x, self.content_x);
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    self.row_geometry_defaults,
                    self.text_matrix_row_base,
                    row_y_positions,
                    self.max_rows,
                    row_geometry,
                    row_flags,
                    self.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow_then_row_start(
                    transition,
                    hit_row_range.range_to(*charpos),
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
                    DisplayRowTransitionRenderState::new(
                        prefix_request,
                        self.has_prefix,
                        line_numbers,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                    ),
                    col,
                );
                BufferTextOverflowRenderOutcome::Transition(
                    character_wrap_action.apply_after_visible_row_transition(
                        row_transition,
                        byte_idx,
                        charpos,
                        hit_row_range,
                        face_scan,
                        row_geometry,
                        self.row_visibility_limit,
                    ),
                )
            }
        }
    }
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
        truncate_lines: bool,
        word_wrap: WordWrapRenderState,
    ) -> BufferTextRowOverflowDecision {
        BufferTextRowOverflowDecision::for_char(
            ch,
            self.plan.position.x_px,
            self.advance_px(),
            right_edge_px,
            truncate_lines,
            word_wrap,
        )
    }

    pub(crate) fn overflow_action(
        self,
        ch: char,
        right_edge_px: f32,
        truncate_lines: bool,
        word_wrap: WordWrapRenderState,
    ) -> BufferTextSourceCharOverflowAction {
        BufferTextSourceCharOverflowAction::for_decision(self.overflow_decision(
            ch,
            right_edge_px,
            truncate_lines,
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

fn measure_buffer_text_source_range_natural_advance_to_text_row<B: LayoutBufferView + ?Sized>(
    state: &mut TextRowSourceMeasureState<'_>,
    range: BufferTextSourceRange,
    base_face: &ResolvedFace,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<f32> {
    let operation = BufferTextSourceAppendOperation::for_buffer_text_range(
        range, buffer_id, buffer, face_id, base_face, frame, position,
    )?;
    let mut render_policy =
        DisplaySourceAppendRenderPolicy::new(DisplaySourceAppendMeasurement::Natural);
    Some(
        operation
            .measure_to_text_row(state, &mut render_policy)?
            .metrics
            .width_px,
    )
}

#[allow(clippy::too_many_arguments)]
fn resolve_buffer_text_source_range_natural_advance_to_text_row<B: LayoutBufferView + ?Sized>(
    state: &mut TextRowSourceMeasureState<'_>,
    range: BufferTextSourceRange,
    buffer_id: BufferId,
    buffer: &B,
    active_face_state: &DisplayRowActiveFaceState,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    cluster: BufferTextSourceClusterState,
) -> f32 {
    if let Some(measured_width) = measure_buffer_text_source_range_natural_advance_to_text_row(
        state,
        range,
        active_face_state.resolved_face(),
        buffer_id,
        buffer,
        active_face_state.face_id(),
        frame.clone(),
        position,
    ) {
        return measured_width;
    }

    fallback_buffer_text_source_range_natural_advance_to_text_row(
        state.font_metrics(),
        active_face_state,
        &frame,
        position,
        cluster,
    )
}

fn fallback_buffer_text_source_range_natural_advance_to_text_row(
    font_metrics: &mut Option<FontMetricsService>,
    active_face_state: &DisplayRowActiveFaceState,
    frame: &DisplayRowAppendFrame,
    position: DisplayRowPosition,
    cluster: BufferTextSourceClusterState,
) -> f32 {
    BufferTextSourceNaturalFallbackAdvance::for_cluster_state(cluster).resolve_to_text_row(
        font_metrics,
        active_face_state,
        frame,
        position,
        cluster.ch(),
    )
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
        request: BufferTextSourceAdvanceRequest<'_>,
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
                let advance_px = resolve_buffer_text_source_range_natural_advance_to_text_row(
                    state,
                    request.range(),
                    buffer_id,
                    buffer,
                    active_face_state,
                    frame,
                    request.position(),
                    request.cluster(),
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
    let range = source_item.range();
    if range.is_empty_or_reversed() {
        return None;
    }

    let start = range.start();
    let end = range.end();
    let source = BufferTextItemSource::new(
        buffer_id,
        start,
        buffer.layout_char_pos_to_emacs_byte_pos(start),
        end,
        buffer.layout_char_pos_to_emacs_byte_pos(end),
    );

    let append_kind = source_item.append_kind();
    let item = source.item(
        RenderFaceRef::FaceId(face_id),
        source_item.into_display_item_kind(),
    );
    Some(BufferTextSourceRangeItemAppendRequest::new(
        item,
        append_kind,
    ))
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BufferTextSourceAppendItem {
    ControlChar { ch: char },
    SourceMappedText { text: Box<str> },
    Glyphless { ch: char, method: GlyphlessMethod },
}

#[derive(Clone, Debug, PartialEq)]
struct BufferTextSourceItemRequest {
    range: BufferTextSourceRange,
    item: BufferTextSourceAppendItem,
}

impl BufferTextSourceItemRequest {
    fn new(range: BufferTextSourceRange, item: BufferTextSourceAppendItem) -> Self {
        Self { range, item }
    }

    fn range(&self) -> BufferTextSourceRange {
        self.range
    }

    fn append_kind(&self) -> DisplayRowAppendKind {
        self.item.append_kind()
    }

    fn fallback_width_px(&self, fallback_char_width: f32) -> f32 {
        self.item
            .fallback_width_policy()
            .width_px(fallback_char_width)
    }

    fn into_display_item_kind(self) -> DisplayItemKind {
        self.item.into_display_item_kind()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BufferTextSourceSpecialDisplay {
    Control(BufferTextSourceAppendItem),
    Nobreak(BufferTextSourceAppendItem),
    Glyphless(BufferTextSourceAppendItem),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextDecodedSourceChar {
    ch: char,
    start_byte_idx: usize,
    start_charpos: i64,
}

impl BufferTextDecodedSourceChar {
    pub(crate) fn consume_from_text(
        text: &[u8],
        byte_idx: &mut usize,
        charpos: i64,
    ) -> Option<Self> {
        if *byte_idx >= text.len() {
            return None;
        }

        let start_byte_idx = *byte_idx;
        let (ch, ch_len) = decode_utf8(&text[*byte_idx..]);
        if ch_len == 0 {
            return None;
        }
        *byte_idx += ch_len;

        Some(Self {
            ch,
            start_byte_idx,
            start_charpos: charpos,
        })
    }

    pub(crate) fn ch(self) -> char {
        self.ch
    }

    pub(crate) fn start_byte_idx(self) -> usize {
        self.start_byte_idx
    }

    pub(crate) fn start_charpos(self) -> i64 {
        self.start_charpos
    }

    pub(crate) fn source_char(self, nobreak_display_policy: i32) -> BufferTextSourceChar {
        BufferTextSourceChar::new(
            self.ch,
            CharPos0::new(self.start_charpos as usize),
            nobreak_display_policy,
        )
    }

    pub(crate) fn record_word_wrap_candidate(
        self,
        word_wrap: &mut WordWrapRenderState,
        output_emitter: &WindowOutputEmitter,
    ) {
        if word_wrap.can_record_candidate(self.ch) {
            word_wrap.record_candidate(
                self.ch,
                self.start_byte_idx,
                self.start_charpos,
                output_emitter.display_point_len(),
                output_emitter.current_row_display_positions(),
            );
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSourceChar {
    ch: char,
    range: BufferTextSourceRange,
    precluster_special_display: Option<BufferTextSourceSpecialDisplay>,
}

impl BufferTextSourceChar {
    pub(crate) fn new(ch: char, start: CharPos0, nobreak_display_policy: i32) -> Self {
        Self {
            ch,
            range: BufferTextSourceRange::single_char(start),
            precluster_special_display: BufferTextSourceSpecialDisplay::for_precluster_char(
                ch,
                nobreak_display_policy,
            ),
        }
    }

    pub(crate) fn range(&self) -> BufferTextSourceRange {
        self.range
    }

    fn precluster_special_display(&self) -> Option<&BufferTextSourceSpecialDisplay> {
        self.precluster_special_display.as_ref()
    }

    fn special_request_for_display(
        &self,
        display: BufferTextSourceSpecialDisplay,
    ) -> BufferTextSpecialSourceCharRequest {
        BufferTextSpecialSourceCharRequest::new(self, display)
    }

    #[cfg(test)]
    pub(crate) fn control_special_request(&self) -> Option<BufferTextSpecialSourceCharRequest> {
        self.precluster_special_display()
            .filter(|display| display.is_control())
            .cloned()
            .map(|display| self.special_request_for_display(display))
    }

    #[cfg(test)]
    pub(crate) fn nobreak_special_request(&self) -> Option<BufferTextSpecialSourceCharRequest> {
        self.precluster_special_display()
            .filter(|display| display.is_nobreak())
            .cloned()
            .map(|display| self.special_request_for_display(display))
    }

    fn cluster_state(&self, tail: Option<(char, bool)>) -> BufferTextSourceClusterState {
        BufferTextSourceClusterState::for_char(self.ch, tail)
    }

    fn cluster_special_display(
        &self,
        tail: Option<(char, bool)>,
    ) -> Option<BufferTextSourceSpecialDisplay> {
        BufferTextSourceSpecialDisplay::for_cluster_state(self.cluster_state(tail))
    }

    pub(crate) fn cluster_special_request(
        &self,
        tail: Option<(char, bool)>,
    ) -> Option<BufferTextSpecialSourceCharRequest> {
        self.cluster_special_display(tail)
            .map(|display| self.special_request_for_display(display))
    }

    pub(crate) fn special_request(
        &self,
        tail: Option<(char, bool)>,
    ) -> Option<BufferTextSpecialSourceCharRequest> {
        self.precluster_special_display()
            .cloned()
            .map(|display| self.special_request_for_display(display))
            .or_else(|| self.cluster_special_request(tail))
    }

    fn advance_request_at<'text>(
        &self,
        text: &'text [u8],
        byte_idx: usize,
        position: DisplayRowPosition,
        tail: Option<(char, bool)>,
    ) -> BufferTextSourceAdvanceRequest<'text> {
        BufferTextSourceAdvanceRequest {
            text,
            byte_idx,
            range: self.range(),
            position,
            cluster: self.cluster_state(tail),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferHscrollSkipAction {
    LineBreak {
        ch_start_byte_idx: usize,
        charpos: i64,
    },
    Text {
        ch_start_byte_idx: usize,
        charpos: i64,
        show_left_truncation: bool,
    },
}

impl BufferHscrollSkipAction {
    pub(crate) fn is_line_break(self) -> bool {
        matches!(self, Self::LineBreak { .. })
    }

    pub(crate) fn ch_start_byte_idx(self) -> usize {
        match self {
            Self::LineBreak {
                ch_start_byte_idx, ..
            }
            | Self::Text {
                ch_start_byte_idx, ..
            } => ch_start_byte_idx,
        }
    }

    pub(crate) fn charpos(self) -> i64 {
        match self {
            Self::LineBreak { charpos, .. } | Self::Text { charpos, .. } => charpos,
        }
    }

    pub(crate) fn should_show_left_truncation(self) -> bool {
        matches!(
            self,
            Self::Text {
                show_left_truncation: true,
                ..
            }
        )
    }

    pub(crate) fn apply_line_break_before_row_transition(
        self,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        output_emitter: &mut WindowOutputEmitter,
        x: &mut f32,
        content_x: f32,
    ) {
        if self.is_line_break() {
            *x = content_x;
            output_emitter.note_display_buffer_pos(LispCharPos1::new(self.charpos()));
            row_extend.clear();
        }
    }

    pub(crate) fn line_break_hit_range(
        self,
        hit_row_range: &mut HitRowRangeTracker,
    ) -> Option<DisplayRowHitRange> {
        if !self.is_line_break() {
            return None;
        }
        let hit_range = hit_row_range.range_to(self.charpos());
        hit_row_range.advance_to(self.charpos());
        Some(hit_range)
    }

    pub(crate) fn capture_line_break_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        x: f32,
        col: usize,
        char_h: f32,
    ) {
        if !target.is_missing() || point_charpos != self.charpos() {
            return;
        }
        capture_cursor_info(
            target,
            CapturedCursorInfo::line_break_from_active_face_state(
                active_face_state,
                CapturedCursorPlacement::from_row_text_position(
                    row_geometry.text_position(x, self.ch_start_byte_idx(), col),
                    CapturedCursorSlotWidth::FaceChar,
                    false,
                ),
                char_h,
            ),
        );
    }

    pub(crate) fn apply_after_line_break_row_transition(
        self,
        row_transition: TextMatrixRowTransition,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        x: f32,
        col: usize,
        char_h: f32,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        self.capture_line_break_cursor_if_point(
            target,
            active_face_state,
            row_geometry,
            point_charpos,
            x,
            col,
            char_h,
        );
        DisplayRowTransitionContinuation::Continue
    }

    pub(crate) fn capture_text_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        x: f32,
        col: usize,
    ) {
        if !target.is_missing() || point_charpos != self.charpos() {
            return;
        }
        capture_cursor_info(
            target,
            CapturedCursorInfo::from_active_face_state(
                active_face_state,
                CapturedCursorPlacement::from_row_text_position(
                    row_geometry.text_position(x, self.ch_start_byte_idx(), col),
                    CapturedCursorSlotWidth::FaceChar,
                    false,
                ),
            ),
        );
    }

    pub(crate) fn append_left_truncation_marker_to_text_row_and_apply<'ctx>(
        self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        state: &mut BufferSyntheticTextRenderState<'_>,
        content_x: f32,
    ) {
        if !self.should_show_left_truncation() {
            return;
        }
        state.append_hscroll_truncation_marker_to_text_row(render_context, row_geometry, content_x);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferEndOfBufferCursorAction {
    byte_idx: usize,
    charpos: i64,
    accessible_end: i64,
    point_charpos: i64,
}

impl BufferEndOfBufferCursorAction {
    pub(crate) fn new(
        byte_idx: usize,
        charpos: i64,
        accessible_end: i64,
        point_charpos: i64,
    ) -> Self {
        Self {
            byte_idx,
            charpos,
            accessible_end,
            point_charpos,
        }
    }

    fn point_is_visible_eob(self) -> bool {
        self.point_charpos == self.accessible_end && self.charpos == self.accessible_end
    }

    pub(crate) fn capture_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        x: f32,
        col: usize,
    ) {
        if !target.is_missing()
            || (self.charpos != self.point_charpos && !self.point_is_visible_eob())
        {
            return;
        }
        if self.point_is_visible_eob() {
            tracing::debug!(
                "layout_window_rust: capturing EOB cursor at x={:.1} y={:.1} point={} point-max={}",
                x,
                row_geometry.glyph_y(0.0),
                self.point_charpos,
                self.accessible_end
            );
        }
        capture_cursor_info(
            target,
            CapturedCursorInfo::from_active_face_state(
                active_face_state,
                CapturedCursorPlacement::from_row_text_position(
                    row_geometry.text_position(x, self.byte_idx, col),
                    CapturedCursorSlotWidth::FaceChar,
                    false,
                ),
            ),
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferEndOfBufferTailAction {
    cursor: BufferEndOfBufferCursorAction,
    has_overlays: bool,
}

impl BufferEndOfBufferTailAction {
    pub(crate) fn new(
        byte_idx: usize,
        charpos: i64,
        accessible_end: i64,
        point_charpos: i64,
        has_overlays: bool,
    ) -> Self {
        Self {
            cursor: BufferEndOfBufferCursorAction::new(
                byte_idx,
                charpos,
                accessible_end,
                point_charpos,
            ),
            has_overlays,
        }
    }

    pub(crate) fn point_is_visible_eob(self) -> bool {
        self.cursor.point_is_visible_eob()
    }

    pub(crate) fn capture_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        x: f32,
        col: usize,
    ) {
        self.cursor
            .capture_cursor_if_point(target, active_face_state, row_geometry, x, col);
    }

    pub(crate) fn should_render_overlay_strings(
        self,
        row_geometry: &DisplayRowGeometryState,
        row_limit: DisplayRowLimit,
    ) -> bool {
        self.has_overlays && row_geometry.is_within_row_limit(row_limit)
    }

    pub(crate) fn render_overlay_strings_at_eob<B: LayoutBufferView>(
        self,
        buffer: &B,
        render_context: BufferOverlayStringTextRowRenderContext<'_>,
        active_face_state: &DisplayRowActiveFaceState,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        render_context.render_both_at(buffer, self.cursor.charpos, active_face_state, state);
    }
}

impl<'a> BufferEndOfBufferTailRenderRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        byte_idx: usize,
        charpos: i64,
        accessible_end: i64,
        point_charpos: i64,
        has_overlays: bool,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
        row_limit: DisplayRowLimit,
    ) -> Self {
        Self {
            byte_idx,
            charpos,
            accessible_end,
            point_charpos,
            has_overlays,
            overlay_context,
            active_face_state,
            row_limit,
        }
    }

    pub(crate) fn render_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferEndOfBufferTailRenderState<'_>,
    ) -> BufferEndOfBufferTailRenderOutcome {
        let BufferEndOfBufferTailRenderState {
            source_render,
            x,
            col,
            row_geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
        } = state;
        let mut source_render = source_render;

        let tail = BufferEndOfBufferTailAction::new(
            self.byte_idx,
            self.charpos,
            self.accessible_end,
            self.point_charpos,
            self.has_overlays,
        );
        let point_is_visible_eob = tail.point_is_visible_eob();
        tail.capture_cursor_if_point(cursor_info, self.active_face_state, row_geometry, *x, *col);

        if tail.should_render_overlay_strings(row_geometry, self.row_limit) {
            let mut overlay_state = OverlayStringRenderState::from_source_render(
                source_render.reborrow(),
                x,
                col,
                row_geometry,
                cursor_info,
                hit_rows,
                hit_row_range,
                row_y_positions,
                face_ids,
            );
            tail.render_overlay_strings_at_eob(
                buffer,
                self.overlay_context,
                self.active_face_state,
                &mut overlay_state,
            );
        }

        BufferEndOfBufferTailRenderOutcome {
            point_is_visible_eob,
        }
    }
}

impl<'a> BufferTextWindowTailFinalizeRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        params: &'a WindowParams,
        text: &'a [u8],
        text_matrix_row_base: usize,
        text_area_left: f32,
        window_top: f32,
        text_y: f32,
        text_height: f32,
        char_w: f32,
        char_h: f32,
        window_start: i64,
        point_charpos: i64,
        charpos: i64,
        point_is_visible_eob: bool,
        row_limit: DisplayRowLimit,
    ) -> Self {
        Self {
            params,
            text,
            text_matrix_row_base,
            text_area_left,
            window_top,
            text_y,
            text_height,
            char_w,
            char_h,
            window_start,
            point_charpos,
            charpos,
            point_is_visible_eob,
            row_limit,
        }
    }

    pub(crate) fn finalize_and_apply(
        self,
        state: BufferTextWindowTailFinalizeState<'_, '_>,
    ) -> BufferTextWindowTailFinalizeOutcome {
        let BufferTextWindowTailFinalizeState {
            cursor_info,
            row_geometry,
            row_y_positions,
            hit_row_range,
            hit_rows,
            output_render,
        } = state;
        let (builder, output_emitter, evaluator) = output_render.into_parts();

        let cursor_requested = self.point_charpos >= self.window_start
            && (self.point_charpos <= self.charpos || self.point_is_visible_eob);
        let mut cursor_published = false;

        if cursor_requested {
            if let Some(cursor) = cursor_info.captured() {
                let cursor_row_metrics = output_emitter.row_metrics().to_vec();
                CapturedTextWindowCursorPublishContext::new(
                    self.params,
                    self.text,
                    self.text_matrix_row_base,
                    self.text_area_left,
                    self.window_top,
                    self.text_y,
                    self.text_height,
                    self.char_w,
                    self.char_h,
                    self.point_charpos,
                    self.point_is_visible_eob,
                )
                .publish_captured_cursor(
                    cursor,
                    &cursor_row_metrics,
                    row_geometry.row_metrics_snapshot(self.text_matrix_row_base),
                    builder,
                    output_emitter,
                );
                cursor_published = true;
            } else {
                tracing::debug!(
                    "layout_window_rust: no explicit cursor capture for point={} window_start={} charpos_end={}",
                    self.point_charpos,
                    self.window_start,
                    self.charpos
                );
            }
        }

        let pending_row_finished = finish_pending_text_window_row(
            builder,
            output_emitter,
            evaluator,
            TextWindowPendingRowFinish {
                row_geometry,
                row_limit: self.row_limit,
                row_y_positions,
                text_y: self.text_y,
                char_height: self.char_h,
                charpos: self.charpos,
                hit_row_range,
                hit_rows,
            },
        );

        let visual_cursor_summary = VisualTextWindowCursorPublishContext::new(
            self.params,
            self.text_area_left,
            self.window_top,
            self.text_y,
            self.text_height,
            self.char_w,
        )
        .publish_visual_cursors(builder, output_emitter);

        BufferTextWindowTailFinalizeOutcome {
            cursor_requested,
            cursor_published,
            visual_cursor_summary,
            pending_row_finished,
        }
    }
}

impl<'a> BufferTextWindowBodyInstallRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        window_id: u64,
        window_start: i64,
        text_start_byte: usize,
        byte_idx: usize,
        reserve_right_special_col: bool,
        reserve_right_border_col: bool,
        text_matrix_row_base: usize,
        matrix_cols: usize,
        row_flags: &'a DisplayRowFlags,
        right_edge_face_id: u32,
        char_w: f32,
    ) -> Self {
        Self {
            window_id,
            window_start,
            text_start_byte,
            byte_idx,
            reserve_right_special_col,
            reserve_right_border_col,
            text_matrix_row_base,
            matrix_cols,
            row_flags,
            right_edge_face_id,
            char_w,
        }
    }

    pub(crate) fn install_and_apply(
        self,
        state: BufferTextWindowBodyInstallState<'_, '_>,
    ) -> TextWindowRedisplayPositions {
        let right_edge_markers = TextWindowRightEdgeMarkers::for_reserved_special_column(
            self.reserve_right_special_col,
            self.reserve_right_border_col,
            self.text_matrix_row_base,
            self.matrix_cols,
            self.row_flags,
            self.right_edge_face_id,
            self.char_w,
        );

        install_text_window_body_output(
            state.builder,
            state.output_emitter,
            TextWindowBodyOutputInstall {
                window_id: self.window_id,
                window_start: self.window_start,
                text_start_byte: self.text_start_byte,
                byte_idx: self.byte_idx,
                right_edge_markers,
            },
        )
    }
}

impl<'a, 'buf, B: LayoutBufferView> BufferTextWindowVisibilityRetryRequest<'a, 'buf, B> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        rows: &'a [DisplayRowSnapshot],
        window_start: i64,
        accessible_start: i64,
        accessible_end: i64,
        point_charpos: i64,
        charpos: i64,
        point_is_visible_eob: bool,
        is_minibuffer: bool,
        text_area_top: i64,
        text_area_bottom: i64,
        buf_access: &'a RustBufferAccess<'buf, B>,
    ) -> Self {
        Self {
            rows,
            window_start,
            accessible_start,
            accessible_end,
            point_charpos,
            charpos,
            point_is_visible_eob,
            is_minibuffer,
            text_area_top,
            text_area_bottom,
            buf_access,
        }
    }

    pub(crate) fn decide(self) -> BufferTextWindowVisibilityRetryOutcome {
        let point_lisp = layout_i64_char_pos_to_lisp_char_pos(self.point_charpos);
        let visible_end_lisp = self.rows.iter().rev().find_map(|row| row.end_buffer_pos);
        let visible_end_lisp = if self.point_is_visible_eob {
            Some(visible_end_lisp.unwrap_or(point_lisp).max(point_lisp))
        } else {
            visible_end_lisp
        };
        let visible_progress = visible_end_lisp
            .map(LispCharPos1::as_i64)
            .unwrap_or(self.charpos);
        let point_beyond_visible_span = visible_end_lisp
            .map(|end_lisp| point_lisp > end_lisp)
            .unwrap_or(self.point_charpos > self.charpos);

        let scroll_down_window_start = if point_beyond_visible_span
            && visible_progress > self.window_start
            && !self.is_minibuffer
        {
            next_window_start_from_visible_rows(self.rows, self.window_start)
                .map(|new_ws| new_ws.min(self.point_charpos.max(self.accessible_start)))
        } else {
            None
        };
        let point_row_window_start = next_window_start_for_partially_visible_point_row(
            self.rows,
            self.point_charpos,
            self.text_area_top,
            self.text_area_bottom,
            self.window_start,
        );
        let point_line_window_start = next_window_start_for_point_line_continuation(
            self.rows,
            self.point_charpos,
            self.window_start,
            self.buf_access,
            self.accessible_end,
        );

        BufferTextWindowVisibilityRetryOutcome {
            visible_end_lisp,
            visible_progress,
            point_beyond_visible_span,
            scroll_down_window_start,
            point_row_window_start,
            point_line_window_start,
        }
    }
}

impl BufferTextWindowFinishRequest {
    pub(crate) fn new(
        window_id: i64,
        content_x: f32,
        char_w: f32,
        text_area_left_offset: i64,
        mode_line_height: i64,
        header_line_height: i64,
        tab_line_height: i64,
    ) -> Self {
        Self {
            window_id,
            content_x,
            char_w,
            text_area_left_offset,
            mode_line_height,
            header_line_height,
            tab_line_height,
        }
    }

    pub(crate) fn finish_and_snapshot(
        self,
        state: BufferTextWindowFinishState<'_>,
    ) -> BufferTextWindowFinishOutput {
        close_text_window_output(state.builder);
        let hit_data = WindowHitData {
            window_id: self.window_id,
            content_x: self.content_x,
            char_w: self.char_w,
            rows: state.hit_rows,
        };
        let snapshot = state.output_emitter.finish_snapshot(
            state.evaluator,
            self.text_area_left_offset,
            self.mode_line_height,
            self.header_line_height,
            self.tab_line_height,
        );

        BufferTextWindowFinishOutput { hit_data, snapshot }
    }
}

impl BufferTextWindowVisibilityRetryOutcome {
    pub(crate) fn visible_end_lisp(self) -> Option<LispCharPos1> {
        self.visible_end_lisp
    }

    #[cfg(test)]
    pub(crate) fn point_beyond_visible_span(self) -> bool {
        self.point_beyond_visible_span
    }

    pub(crate) fn scroll_down_window_start(self) -> Option<i64> {
        self.scroll_down_window_start
    }

    pub(crate) fn point_row_window_start(self) -> Option<i64> {
        self.point_row_window_start
    }

    pub(crate) fn point_line_window_start(self) -> Option<i64> {
        self.point_line_window_start
    }

    pub(crate) fn retry_window_start(self) -> Option<i64> {
        self.scroll_down_window_start
            .or(self.point_row_window_start)
            .or(self.point_line_window_start)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferHscrollSkipSourceChar {
    ch_start_byte_idx: usize,
    ch: char,
    charpos: i64,
}

pub(crate) struct BufferHscrollSkipRenderRequest<'a> {
    text: &'a [u8],
    tab_width: i32,
    content_x: f32,
    append_surface: &'a DisplayRowAppendSurface,
    active_face_state: &'a DisplayRowActiveFaceState,
    default_face_ascent: f32,
    char_h: f32,
    char_w: f32,
    point_charpos: i64,
    has_prefix: bool,
    row_geometry_defaults: DisplayRowGeometryDefaults,
    text_matrix_row_base: usize,
    max_rows: usize,
    row_limit: DisplayRowLimit,
}

impl BufferHscrollSkipSourceChar {
    fn new(ch_start_byte_idx: usize, ch: char, charpos: i64) -> Self {
        Self {
            ch_start_byte_idx,
            ch,
            charpos,
        }
    }

    pub(crate) fn consume_from_text(
        text: &[u8],
        byte_idx: &mut usize,
        charpos: &mut i64,
        hscroll_skip: &mut HorizontalScrollSkipState,
        tab_width: i32,
    ) -> Option<BufferHscrollSkipAction> {
        if *byte_idx >= text.len() {
            return None;
        }

        let ch_start_byte_idx = *byte_idx;
        let (ch, ch_len) = decode_utf8(&text[*byte_idx..]);
        *byte_idx += ch_len;
        *charpos += 1;

        Some(
            Self::new(ch_start_byte_idx, ch, *charpos).consume_for_hscroll(hscroll_skip, tab_width),
        )
    }

    fn consume_for_hscroll(
        self,
        hscroll_skip: &mut HorizontalScrollSkipState,
        tab_width: i32,
    ) -> BufferHscrollSkipAction {
        if self.ch == '\n' {
            return BufferHscrollSkipAction::LineBreak {
                ch_start_byte_idx: self.ch_start_byte_idx,
                charpos: self.charpos,
            };
        }

        hscroll_skip.consume_columns(self.column_width(tab_width, hscroll_skip.consumed_columns()));
        BufferHscrollSkipAction::Text {
            ch_start_byte_idx: self.ch_start_byte_idx,
            charpos: self.charpos,
            show_left_truncation: !hscroll_skip.should_skip()
                && hscroll_skip.should_show_left_truncation(),
        }
    }

    fn column_width(self, tab_width: i32, consumed_columns: i32) -> i32 {
        if self.ch == '\t' {
            let tab_width = tab_width.max(1);
            return ((consumed_columns / tab_width + 1) * tab_width) - consumed_columns;
        }

        if is_wide_char(self.ch) { 2 } else { 1 }
    }
}

impl<'a> BufferHscrollSkipRenderRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        text: &'a [u8],
        tab_width: i32,
        content_x: f32,
        append_surface: &'a DisplayRowAppendSurface,
        active_face_state: &'a DisplayRowActiveFaceState,
        default_face_ascent: f32,
        char_h: f32,
        char_w: f32,
        point_charpos: i64,
        has_prefix: bool,
        row_geometry_defaults: DisplayRowGeometryDefaults,
        text_matrix_row_base: usize,
        max_rows: usize,
        row_limit: DisplayRowLimit,
    ) -> Self {
        Self {
            text,
            tab_width,
            content_x,
            append_surface,
            active_face_state,
            default_face_ascent,
            char_h,
            char_w,
            point_charpos,
            has_prefix,
            row_geometry_defaults,
            text_matrix_row_base,
            max_rows,
            row_limit,
        }
    }

    pub(crate) fn render_next_and_apply(
        self,
        state: BufferHscrollSkipRenderState<'_, '_>,
    ) -> DisplayRowTransitionContinuation {
        let BufferHscrollSkipRenderState {
            byte_idx,
            charpos,
            hscroll_skip,
            row_extend,
            source_render,
            x,
            col,
            prefix_request,
            line_numbers,
            word_wrap,
            trailing_whitespace,
            row_geometry,
            row_flags,
            hit_rows,
            hit_row_range,
            cursor_info,
            row_y_positions,
        } = state;
        let mut source_render = source_render;

        let Some(hscroll_action) = BufferHscrollSkipSourceChar::consume_from_text(
            self.text,
            byte_idx,
            charpos,
            hscroll_skip,
            self.tab_width,
        ) else {
            return DisplayRowTransitionContinuation::Exhausted;
        };

        if hscroll_action.is_line_break() {
            hscroll_action.apply_line_break_before_row_transition(
                row_extend,
                source_render.output_emitter(),
                x,
                self.content_x,
            );
            let line_break_transition = DisplayRowLineBreakTransitionPlan::hscroll_line_break();
            let hit_range = hscroll_action
                .line_break_hit_range(hit_row_range)
                .expect("hscroll line break hit range");
            let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                self.row_geometry_defaults,
                self.text_matrix_row_base,
                row_y_positions,
                self.max_rows,
                row_geometry,
                row_flags,
                self.row_limit,
                hit_rows,
                &mut source_render,
            )
            .emit_line_break_then_row_start(
                line_break_transition,
                hit_range,
                DisplayRowPosition {
                    x_px: *x,
                    col: *col,
                },
                0.0,
                DisplayRowTransitionRenderState::new(
                    prefix_request,
                    self.has_prefix,
                    line_numbers,
                    hscroll_skip,
                    word_wrap,
                    trailing_whitespace,
                ),
                col,
            );
            return hscroll_action.apply_after_line_break_row_transition(
                row_transition,
                cursor_info,
                self.active_face_state,
                row_geometry,
                self.point_charpos,
                *x,
                *col,
                self.char_h,
            );
        }

        let mut synthetic_text_state =
            BufferSyntheticTextRenderState::new(source_render.reborrow(), x, col);
        hscroll_action.append_left_truncation_marker_to_text_row_and_apply(
            BufferSyntheticTextRenderContext::new(
                self.append_surface,
                self.active_face_state,
                0.0,
                self.char_h,
                self.default_face_ascent,
                self.char_w,
            ),
            row_geometry,
            &mut synthetic_text_state,
            self.content_x,
        );
        hscroll_action.capture_text_cursor_if_point(
            cursor_info,
            self.active_face_state,
            row_geometry,
            self.point_charpos,
            *x,
            *col,
        );
        DisplayRowTransitionContinuation::Continue
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferInvisibleTextScanAction {
    Unchecked,
    Visible { next_visible: i64 },
    Hidden(BufferInvisibleTextSkip),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferInvisibleTextSkip {
    start_byte_idx: usize,
    start_charpos: i64,
    skip_to: i64,
    next_visible: i64,
    point_in_hidden_region: bool,
    ellipsis: bool,
}

impl BufferInvisibleTextSkip {
    fn new(
        start_byte_idx: usize,
        start_charpos: i64,
        skip_to: i64,
        next_visible: i64,
        point_in_hidden_region: bool,
        ellipsis: bool,
    ) -> Self {
        Self {
            start_byte_idx,
            start_charpos,
            skip_to,
            next_visible,
            point_in_hidden_region,
            ellipsis,
        }
    }

    #[cfg(test)]
    pub(crate) fn start_byte_idx(self) -> usize {
        self.start_byte_idx
    }

    #[cfg(test)]
    pub(crate) fn start_charpos(self) -> i64 {
        self.start_charpos
    }

    #[cfg(test)]
    pub(crate) fn skip_to(self) -> i64 {
        self.skip_to
    }

    #[cfg(test)]
    pub(crate) fn next_visible(self) -> i64 {
        self.next_visible
    }

    #[cfg(test)]
    pub(crate) fn point_in_hidden_region(self) -> bool {
        self.point_in_hidden_region
    }

    #[cfg(test)]
    pub(crate) fn ellipsis(self) -> bool {
        self.ellipsis
    }

    pub(crate) fn capture_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        x: f32,
        col: usize,
    ) {
        if !self.point_in_hidden_region {
            return;
        }
        capture_cursor_info(
            target,
            CapturedCursorInfo::from_active_face_state(
                active_face_state,
                CapturedCursorPlacement::from_row_text_position(
                    row_geometry.text_position(x, self.start_byte_idx, col),
                    CapturedCursorSlotWidth::FaceChar,
                    false,
                ),
            ),
        );
    }

    pub(crate) fn ellipsis_append_request(
        self,
        position: DisplayRowPosition,
    ) -> Option<SyntheticTextAppendRequest> {
        self.ellipsis.then(|| {
            SyntheticTextAppendRequest::active_marker(
                position,
                SyntheticTextMarker::InvisibleEllipsis,
            )
        })
    }

    pub(crate) fn append_to_text_row_and_apply<'ctx>(
        self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        state: &mut BufferInvisibleTextRenderState<'_>,
    ) {
        let position = state.synthetic_text.position();
        self.capture_cursor_if_point(
            state.cursor_info,
            render_context.active_face,
            row_geometry,
            position.x_px,
            position.col,
        );

        let Some(request) = self.ellipsis_append_request(position) else {
            return;
        };
        state
            .synthetic_text
            .append_request_to_text_row(render_context, row_geometry, request);
    }
}

pub(crate) struct BufferInvisibleTextRenderState<'a> {
    synthetic_text: BufferSyntheticTextRenderState<'a>,
    cursor_info: &'a mut CursorCaptureState,
}

impl<'a> BufferInvisibleTextRenderState<'a> {
    pub(crate) fn new(
        source_render: TextRowSourceRenderState<'a>,
        cursor_info: &'a mut CursorCaptureState,
        x: &'a mut f32,
        col: &'a mut usize,
    ) -> Self {
        Self {
            synthetic_text: BufferSyntheticTextRenderState::new(source_render, x, col),
            cursor_info,
        }
    }
}

impl<'a> BufferInvisibleTextRenderRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        text: &'a [u8],
        accessible_end: i64,
        point_charpos: i64,
        append_surface: &'a DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_face_ascent: f32,
        char_h: f32,
        char_w: f32,
    ) -> Self {
        Self {
            text,
            accessible_end,
            point_charpos,
            append_surface,
            overlay_context,
            active_face_state,
            glyph_y_offset,
            default_face_ascent,
            char_h,
            char_w,
        }
    }

    pub(crate) fn render_at_checkpoint_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferInvisibleTextRenderRequestState<'_, '_>,
    ) -> BufferInvisibleTextRenderOutcome {
        let BufferInvisibleTextRenderRequestState {
            checkpoints,
            byte_idx,
            charpos,
            source_render,
            x,
            col,
            row_geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
        } = state;
        let mut source_render = source_render;

        let action = BufferInvisibleTextScanContext::new(
            self.text,
            self.accessible_end,
            self.point_charpos,
            cursor_info.is_missing(),
        )
        .consume_at_checkpoint(buffer, checkpoints, byte_idx, charpos);
        let BufferInvisibleTextScanAction::Hidden(hidden_text) = action else {
            return BufferInvisibleTextRenderOutcome::Visible;
        };

        let mut hidden_text_state =
            BufferInvisibleTextRenderState::new(source_render.reborrow(), cursor_info, x, col);
        hidden_text.append_to_text_row_and_apply(
            BufferSyntheticTextRenderContext::new(
                self.append_surface,
                self.active_face_state,
                self.glyph_y_offset,
                self.char_h,
                self.default_face_ascent,
                self.char_w,
            ),
            row_geometry,
            &mut hidden_text_state,
        );

        let mut overlay_state = OverlayStringRenderState::from_source_render(
            source_render.reborrow(),
            x,
            col,
            row_geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
        );
        self.overlay_context.render_after_at(
            buffer,
            *charpos,
            self.active_face_state,
            &mut overlay_state,
        );
        BufferInvisibleTextRenderOutcome::ContinueBufferWalk
    }
}

pub(crate) struct BufferSyntheticTextRenderState<'a> {
    source_render: TextRowSourceRenderState<'a>,
    x: &'a mut f32,
    col: &'a mut usize,
}

impl<'a> BufferSyntheticTextRenderState<'a> {
    pub(crate) fn new(
        source_render: TextRowSourceRenderState<'a>,
        x: &'a mut f32,
        col: &'a mut usize,
    ) -> Self {
        Self {
            source_render,
            x,
            col,
        }
    }

    fn position(&self) -> DisplayRowPosition {
        DisplayRowPosition {
            x_px: *self.x,
            col: *self.col,
        }
    }

    pub(crate) fn append_request_to_text_row<'ctx>(
        &mut self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        request: SyntheticTextAppendRequest,
    ) {
        let Some((_progress, position)) = render_context.render_request_to_text_row(
            &mut self.source_render,
            row_geometry,
            request,
        ) else {
            return;
        };
        *self.x = position.x_px;
        *self.col = position.col;
    }

    pub(crate) fn append_hscroll_truncation_marker_to_text_row<'ctx>(
        &mut self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        content_x: f32,
    ) {
        let request =
            render_context.hscroll_truncation_request(self.source_render.default_face(), content_x);
        self.append_request_to_text_row(render_context, row_geometry, request);
        self.source_render.mark_current_text_row_truncated_left();
    }
}

pub(crate) struct BufferCurrentFaceResolutionContext<'a, B: LayoutBufferView> {
    buffer: &'a B,
    face_resolver: &'a FaceResolver,
    measurement_policy: DisplayRowMeasurementPolicy,
    default_resolved: &'a ResolvedFace,
    default_face_char_w: f32,
    default_face_ascent: f32,
    default_face_h: f32,
    char_w: f32,
    char_h: f32,
    font_ascent: f32,
    window_system: bool,
}

impl<'a, B: LayoutBufferView> Clone for BufferCurrentFaceResolutionContext<'a, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, B: LayoutBufferView> Copy for BufferCurrentFaceResolutionContext<'a, B> {}

impl<'a, B: LayoutBufferView> BufferCurrentFaceResolutionContext<'a, B> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        buffer: &'a B,
        face_resolver: &'a FaceResolver,
        measurement_policy: DisplayRowMeasurementPolicy,
        default_resolved: &'a ResolvedFace,
        default_face_char_w: f32,
        default_face_ascent: f32,
        default_face_h: f32,
        char_w: f32,
        char_h: f32,
        font_ascent: f32,
        window_system: bool,
    ) -> Self {
        Self {
            buffer,
            face_resolver,
            measurement_policy,
            default_resolved,
            default_face_char_w,
            default_face_ascent,
            default_face_h,
            char_w,
            char_h,
            font_ascent,
            window_system,
        }
    }

    pub(crate) fn resolve_at_checkpoint(
        &self,
        state: &mut BufferCurrentFaceResolutionState<'_, '_>,
        charpos: i64,
    ) -> bool {
        if !state.face_scan.should_resolve_at(charpos as usize) {
            return false;
        }

        let mut resolved = self.face_resolver.face_at_pos(
            self.buffer,
            charpos as usize,
            state.face_scan.next_check_mut(),
        );
        if let Some(factor) = state.height_span.value()
            && let Some(adjusted) = height_adjusted_face(
                &resolved,
                DisplayHeightFaceBasis {
                    canonical_face: self.default_resolved,
                    base_face: self.default_resolved,
                    fallback_char_width: self.default_face_char_w,
                    fallback_ascent: self.default_face_ascent,
                    fallback_row_height: self.default_face_h,
                },
                factor,
            )
        {
            resolved = adjusted;
        }

        let face_id = state.face_ids.allocate();
        let resolved_extend = resolved.extend;
        let resolved_bg = resolved.bg;
        let resolved_box_type = resolved.box_type;
        *state.active_face_state = state.source_render.resolve_and_install_measured_face(
            self.measurement_policy,
            face_id,
            resolved,
            self.window_system,
            self.char_w,
            DisplayRowFallbackMetrics::from_default_face_extents(
                self.char_w,
                self.char_h,
                self.font_ascent,
            ),
        );
        let face_metrics = state.active_face_state.metrics();
        state
            .row_geometry
            .include_row_extents(face_metrics.row_height, face_metrics.ascent);

        if resolved_extend {
            let ext_bg = Color::from_pixel(resolved_bg);
            state
                .row_extend
                .activate(state.row_geometry.current_row_marker(), (ext_bg, face_id));
        }

        if state.box_face.is_active() && resolved_box_type == 0 {
            state.box_face.clear();
        }
        if resolved_box_type > 0 {
            state
                .box_face
                .activate(state.row_geometry.current_row_marker(), state.x);
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_at_checkpoint_with_source_state(
        &self,
        source_render: &mut TextRowSourceRenderState<'_>,
        face_scan: &mut FaceScanCheckpoint,
        height_span: &mut ActiveDisplayPropertySpan<f32>,
        face_ids: &mut FrameFaceIdAllocator,
        active_face_state: &mut DisplayRowActiveFaceState,
        row_geometry: &mut DisplayRowGeometryState,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &mut BoxFaceRowState,
        x: f32,
        charpos: i64,
    ) -> bool {
        self.resolve_at_checkpoint(
            &mut BufferCurrentFaceResolutionState::new(
                source_render,
                face_scan,
                height_span,
                face_ids,
                active_face_state,
                row_geometry,
                row_extend,
                box_face,
                x,
            ),
            charpos,
        )
    }
}

pub(crate) struct BufferCurrentFaceResolutionState<'a, 'source> {
    source_render: &'a mut TextRowSourceRenderState<'source>,
    face_scan: &'a mut FaceScanCheckpoint,
    height_span: &'a ActiveDisplayPropertySpan<f32>,
    face_ids: &'a mut FrameFaceIdAllocator,
    active_face_state: &'a mut DisplayRowActiveFaceState,
    row_geometry: &'a mut DisplayRowGeometryState,
    row_extend: &'a mut DisplayRowScopedValue<(Color, u32)>,
    box_face: &'a mut BoxFaceRowState,
    x: f32,
}

impl<'a, 'source> BufferCurrentFaceResolutionState<'a, 'source> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        source_render: &'a mut TextRowSourceRenderState<'source>,
        face_scan: &'a mut FaceScanCheckpoint,
        height_span: &'a ActiveDisplayPropertySpan<f32>,
        face_ids: &'a mut FrameFaceIdAllocator,
        active_face_state: &'a mut DisplayRowActiveFaceState,
        row_geometry: &'a mut DisplayRowGeometryState,
        row_extend: &'a mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &'a mut BoxFaceRowState,
        x: f32,
    ) -> Self {
        Self {
            source_render,
            face_scan,
            height_span,
            face_ids,
            active_face_state,
            row_geometry,
            row_extend,
            box_face,
            x,
        }
    }
}

pub(crate) struct BufferInvisibleTextScanContext<'a> {
    text: &'a [u8],
    accessible_end: i64,
    point_charpos: i64,
    cursor_missing: bool,
}

impl<'a> BufferInvisibleTextScanContext<'a> {
    pub(crate) fn new(
        text: &'a [u8],
        accessible_end: i64,
        point_charpos: i64,
        cursor_missing: bool,
    ) -> Self {
        Self {
            text,
            accessible_end,
            point_charpos,
            cursor_missing,
        }
    }

    pub(crate) fn consume_at_checkpoint<B: LayoutBufferView>(
        &self,
        buffer: &B,
        checkpoints: &mut TextPropertyScanCheckpoints,
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) -> BufferInvisibleTextScanAction {
        if !checkpoints.should_check_invisible(*charpos) {
            return BufferInvisibleTextScanAction::Unchecked;
        }

        let start_byte_idx = *byte_idx;
        let start_charpos = *charpos;
        let text_props = RustTextPropAccess::new(buffer);
        let (invisible, next_visible) = text_props.check_invisible(start_charpos);
        checkpoints.record_invisible_next(next_visible);

        if !invisible.hidden {
            return BufferInvisibleTextScanAction::Visible { next_visible };
        }

        let skip_to = next_visible.min(self.accessible_end);
        let point_in_hidden_region = self.cursor_missing
            && self.point_charpos >= start_charpos
            && self.point_charpos < skip_to;
        skip_text_to_charpos(self.text, byte_idx, charpos, skip_to);

        BufferInvisibleTextScanAction::Hidden(BufferInvisibleTextSkip::new(
            start_byte_idx,
            start_charpos,
            skip_to,
            next_visible,
            point_in_hidden_region,
            invisible.ellipsis,
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferSelectiveDisplayLineTailAction {
    Exhausted,
    LineBreak { charpos: i64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSelectiveDisplayLineTailMarker;

impl BufferSelectiveDisplayLineTailMarker {
    pub(crate) fn ellipsis_append_request(
        self,
        position: DisplayRowPosition,
    ) -> SyntheticTextAppendRequest {
        SyntheticTextAppendRequest::active_marker(position, SyntheticTextMarker::SelectiveEllipsis)
    }

    pub(crate) fn append_to_text_row_and_apply<'ctx>(
        self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        state: &mut BufferSyntheticTextRenderState<'_>,
    ) {
        let request = self.ellipsis_append_request(state.position());
        state.append_request_to_text_row(render_context, row_geometry, request);
    }
}

impl BufferSelectiveDisplayLineTailAction {
    pub(crate) fn is_line_break(self) -> bool {
        matches!(self, Self::LineBreak { .. })
    }

    pub(crate) fn apply_hidden_line_break_row_state(
        self,
        row_geometry: &DisplayRowGeometryState,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &mut BoxFaceRowState,
        content_x: f32,
        x: &mut f32,
    ) {
        if self.is_line_break() {
            *x = content_x;
            row_extend.clear();
            box_face.continue_on_row(row_geometry.next_row_marker(), content_x);
        }
    }

    pub(crate) fn sync_after_hidden_line_break_transition(
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) {
        *charpos = synced_charpos;
        hit_row_range.advance_to(*charpos);
    }

    pub(crate) fn apply_after_hidden_line_break_transition(
        self,
        row_transition: TextMatrixRowTransition,
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        Self::sync_after_hidden_line_break_transition(synced_charpos, charpos, hit_row_range);
        DisplayRowTransitionContinuation::Continue
    }

    #[cfg(test)]
    pub(crate) fn charpos(self) -> Option<i64> {
        match self {
            Self::LineBreak { charpos } => Some(charpos),
            Self::Exhausted => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSelectiveDisplayHiddenLines {
    hidden_line_count: usize,
}

impl BufferSelectiveDisplayHiddenLines {
    fn new(hidden_line_count: usize) -> Self {
        Self { hidden_line_count }
    }

    #[cfg(test)]
    pub(crate) fn hidden_line_count(self) -> usize {
        self.hidden_line_count
    }

    pub(crate) fn apply_to_line_numbers(self, line_numbers: &mut LineNumberRenderState) {
        for _ in 0..self.hidden_line_count {
            line_numbers.advance_hidden_line();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSelectiveDisplayContext<'a> {
    text: &'a [u8],
    selective_display: i32,
    tab_width: i32,
}

impl<'a> BufferSelectiveDisplayContext<'a> {
    pub(crate) fn new(text: &'a [u8], selective_display: i32, tab_width: i32) -> Self {
        Self {
            text,
            selective_display,
            tab_width: tab_width.max(1),
        }
    }

    pub(crate) fn hides_carriage_return_tail(self, ch: char) -> bool {
        self.selective_display > 0 && ch == '\r'
    }

    pub(crate) fn carriage_return_tail_marker(
        self,
        ch: char,
    ) -> Option<BufferSelectiveDisplayLineTailMarker> {
        self.hides_carriage_return_tail(ch)
            .then_some(BufferSelectiveDisplayLineTailMarker)
    }

    pub(crate) fn hides_indented_lines_after_line_break(self, byte_idx: usize) -> bool {
        self.selective_display > 0
            && self.selective_display < i32::MAX
            && byte_idx < self.text.len()
    }

    pub(crate) fn skip_rest_of_line_after_carriage_return(
        self,
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) -> BufferSelectiveDisplayLineTailAction {
        *charpos += 1;
        while *byte_idx < self.text.len() {
            let (skip_ch, skip_len) = decode_utf8(&self.text[*byte_idx..]);
            if skip_len == 0 {
                break;
            }
            *byte_idx += skip_len;
            *charpos += 1;
            if skip_ch == '\n' {
                return BufferSelectiveDisplayLineTailAction::LineBreak { charpos: *charpos };
            }
        }

        BufferSelectiveDisplayLineTailAction::Exhausted
    }

    pub(crate) fn skip_hidden_indented_lines_after_line_break(
        self,
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) -> BufferSelectiveDisplayHiddenLines {
        let mut hidden_line_count = 0;
        while *byte_idx < self.text.len() {
            let Some(indent) = self.indentation_columns_at(*byte_idx) else {
                break;
            };
            if indent <= self.selective_display {
                break;
            }

            if self.skip_line(byte_idx, charpos) {
                hidden_line_count += 1;
            }
        }

        BufferSelectiveDisplayHiddenLines::new(hidden_line_count)
    }

    pub(crate) fn apply_hidden_indented_lines_after_line_break(
        self,
        byte_idx: &mut usize,
        charpos: &mut i64,
        line_numbers: &mut LineNumberRenderState,
    ) -> BufferSelectiveDisplayHiddenLines {
        if !self.hides_indented_lines_after_line_break(*byte_idx) {
            return BufferSelectiveDisplayHiddenLines::new(0);
        }
        let hidden_lines = self.skip_hidden_indented_lines_after_line_break(byte_idx, charpos);
        hidden_lines.apply_to_line_numbers(line_numbers);
        hidden_lines
    }

    fn indentation_columns_at(self, mut byte_idx: usize) -> Option<i32> {
        if byte_idx >= self.text.len() {
            return None;
        }

        let mut indent = 0i32;
        while byte_idx < self.text.len() {
            match self.text[byte_idx] {
                b' ' => {
                    indent += 1;
                    byte_idx += 1;
                }
                b'\t' => {
                    indent = ((indent / self.tab_width) + 1) * self.tab_width;
                    byte_idx += 1;
                }
                _ => break,
            }
        }
        Some(indent)
    }

    fn skip_line(self, byte_idx: &mut usize, charpos: &mut i64) -> bool {
        while *byte_idx < self.text.len() {
            let (skip_ch, skip_len) = decode_utf8(&self.text[*byte_idx..]);
            if skip_len == 0 {
                break;
            }
            *byte_idx += skip_len;
            *charpos += 1;
            if skip_ch == '\n' {
                return true;
            }
        }
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextLineBreakSourceAction {
    ch_start_byte_idx: usize,
    charpos: i64,
    next_charpos: i64,
    line_spacing: f32,
}

pub(crate) struct BufferTextLineBreakRenderRequest<'a> {
    source_char: BufferTextDecodedSourceChar,
    text: &'a [u8],
    text_start_byte: usize,
    selective_display: i32,
    tab_width: i32,
    active_face_state: &'a DisplayRowActiveFaceState,
    point_charpos: i64,
    char_h: f32,
    extra_line_spacing: f32,
    content_x: f32,
    has_prefix: bool,
    row_geometry_defaults: DisplayRowGeometryDefaults,
    text_matrix_row_base: usize,
    max_rows: usize,
    row_limit: DisplayRowLimit,
}

impl<'a> BufferTextLineBreakRenderRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        source_char: BufferTextDecodedSourceChar,
        text: &'a [u8],
        text_start_byte: usize,
        selective_display: i32,
        tab_width: i32,
        active_face_state: &'a DisplayRowActiveFaceState,
        point_charpos: i64,
        char_h: f32,
        extra_line_spacing: f32,
        content_x: f32,
        has_prefix: bool,
        row_geometry_defaults: DisplayRowGeometryDefaults,
        text_matrix_row_base: usize,
        max_rows: usize,
        row_limit: DisplayRowLimit,
    ) -> Self {
        Self {
            source_char,
            text,
            text_start_byte,
            selective_display,
            tab_width,
            active_face_state,
            point_charpos,
            char_h,
            extra_line_spacing,
            content_x,
            has_prefix,
            row_geometry_defaults,
            text_matrix_row_base,
            max_rows,
            row_limit,
        }
    }

    pub(crate) fn render_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferTextLineBreakRenderState<'_, '_>,
    ) -> DisplayRowTransitionContinuation {
        let BufferTextLineBreakRenderState {
            byte_idx,
            charpos,
            cursor_info,
            row_geometry,
            trailing_whitespace,
            row_extend,
            box_face,
            source_render,
            x,
            col,
            prefix_request,
            line_numbers,
            hscroll_skip,
            word_wrap,
            row_flags,
            hit_rows,
            hit_row_range,
            row_y_positions,
        } = state;
        let mut source_render = source_render;

        let line_break_action = BufferTextLineBreakSourceAction::for_decoded_newline(
            buffer,
            self.source_char,
            self.char_h,
            self.extra_line_spacing,
        );
        line_break_action.capture_cursor_if_point(
            cursor_info,
            self.active_face_state,
            row_geometry,
            self.point_charpos,
            *x,
            *col,
        );
        line_break_action.apply_before_row_transition(
            row_geometry,
            trailing_whitespace,
            row_extend,
            box_face,
            source_render.output_emitter(),
            self.content_x,
            x,
            charpos,
        );

        let line_break_transition = DisplayRowLineBreakTransitionPlan::line_break();
        let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
            self.row_geometry_defaults,
            self.text_matrix_row_base,
            row_y_positions,
            self.max_rows,
            row_geometry,
            row_flags,
            self.row_limit,
            hit_rows,
            &mut source_render,
        )
        .emit_line_break_then_row_start(
            line_break_transition,
            hit_row_range.range_to(*charpos),
            DisplayRowPosition {
                x_px: *x,
                col: *col,
            },
            line_break_action.line_spacing(),
            DisplayRowTransitionRenderState::new(
                prefix_request,
                self.has_prefix,
                line_numbers,
                hscroll_skip,
                word_wrap,
                trailing_whitespace,
            ),
            col,
        );

        let synced_charpos = buffer
            .layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(self.text_start_byte + *byte_idx))
            .get() as i64;
        let continuation = line_break_action.apply_after_line_break_row_transition(
            row_transition,
            synced_charpos,
            charpos,
            hit_row_range,
            row_geometry,
            box_face,
            self.content_x,
        );
        if continuation.should_break() {
            return continuation;
        }

        BufferSelectiveDisplayContext::new(self.text, self.selective_display, self.tab_width)
            .apply_hidden_indented_lines_after_line_break(byte_idx, charpos, line_numbers);
        DisplayRowTransitionContinuation::Continue
    }
}

impl BufferTextLineBreakSourceAction {
    pub(crate) fn for_newline<B: LayoutBufferView>(
        buffer: &B,
        charpos: i64,
        ch_start_byte_idx: usize,
        char_h: f32,
        extra_line_spacing: f32,
    ) -> Self {
        let text_prop_spacing = RustTextPropAccess::new(buffer).check_line_spacing(charpos, char_h);
        let line_spacing = if text_prop_spacing > 0.0 {
            text_prop_spacing
        } else if extra_line_spacing > 0.0 {
            extra_line_spacing
        } else {
            0.0
        };
        Self {
            ch_start_byte_idx,
            charpos,
            next_charpos: charpos + 1,
            line_spacing,
        }
    }

    pub(crate) fn for_decoded_newline<B: LayoutBufferView>(
        buffer: &B,
        source_char: BufferTextDecodedSourceChar,
        char_h: f32,
        extra_line_spacing: f32,
    ) -> Self {
        Self::for_newline(
            buffer,
            source_char.start_charpos(),
            source_char.start_byte_idx(),
            char_h,
            extra_line_spacing,
        )
    }

    pub(crate) fn point_matches(self, point_charpos: i64) -> bool {
        point_charpos == self.charpos
    }

    pub(crate) fn next_charpos(self) -> i64 {
        self.next_charpos
    }

    pub(crate) fn line_spacing(self) -> f32 {
        self.line_spacing
    }

    pub(crate) fn apply_before_row_transition(
        self,
        row_geometry: &DisplayRowGeometryState,
        trailing_whitespace: &mut TrailingWhitespaceRenderState,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &mut BoxFaceRowState,
        output_emitter: &mut WindowOutputEmitter,
        content_x: f32,
        x: &mut f32,
        charpos: &mut i64,
    ) {
        trailing_whitespace.reset_after_row_transition();
        row_extend.clear();
        box_face.continue_on_row(row_geometry.current_row_marker(), content_x);
        *charpos = self.next_charpos();
        *x = content_x;
        output_emitter.note_display_buffer_pos(LispCharPos1::new(*charpos));
    }

    pub(crate) fn apply_after_row_transition(
        self,
        row_geometry: &DisplayRowGeometryState,
        box_face: &mut BoxFaceRowState,
        content_x: f32,
    ) {
        box_face.continue_on_row(row_geometry.current_row_marker(), content_x);
    }

    pub(crate) fn apply_after_line_break_row_transition(
        self,
        row_transition: TextMatrixRowTransition,
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        row_geometry: &DisplayRowGeometryState,
        box_face: &mut BoxFaceRowState,
        content_x: f32,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        Self::sync_after_row_transition(synced_charpos, charpos, hit_row_range);
        self.apply_after_row_transition(row_geometry, box_face, content_x);
        DisplayRowTransitionContinuation::Continue
    }

    pub(crate) fn sync_after_row_transition(
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) {
        *charpos = synced_charpos;
        hit_row_range.advance_to(*charpos);
    }

    pub(crate) fn cursor_info(
        self,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        x: f32,
        col: usize,
    ) -> CapturedCursorInfo {
        CapturedCursorInfo::from_active_face_state(
            active_face_state,
            CapturedCursorPlacement::from_row_text_position(
                row_geometry.text_position(x, self.ch_start_byte_idx, col),
                CapturedCursorSlotWidth::FaceChar,
                false,
            ),
        )
    }

    pub(crate) fn capture_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        x: f32,
        col: usize,
    ) {
        if !target.is_missing() || !self.point_matches(point_charpos) {
            return;
        }
        capture_cursor_info(
            target,
            self.cursor_info(active_face_state, row_geometry, x, col),
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextTruncationSkipAction {
    charpos: i64,
    reached_line_break: bool,
}

impl BufferTextTruncationSkipAction {
    pub(crate) fn consume_decoded_char_and_rest_of_line(
        text: &[u8],
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) -> Self {
        *charpos += 1;
        let reached_line_break = skip_to_newline(text, byte_idx, charpos);
        Self {
            charpos: *charpos,
            reached_line_break,
        }
    }

    #[cfg(test)]
    pub(crate) fn charpos(self) -> i64 {
        self.charpos
    }

    pub(crate) fn reached_line_break(self) -> bool {
        self.reached_line_break
    }

    pub(crate) fn apply_before_row_transition(
        self,
        line_numbers: &mut LineNumberRenderState,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        x: &mut f32,
        content_x: f32,
    ) {
        if self.reached_line_break() {
            line_numbers.advance_line();
        }
        *x = content_x;
        row_extend.clear();
    }

    pub(crate) fn sync_after_row_transition(
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) {
        *charpos = synced_charpos;
        hit_row_range.advance_to(*charpos);
    }

    pub(crate) fn transition_continuation(
        self,
        row_transition: TextMatrixRowTransition,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            DisplayRowTransitionContinuation::Exhausted
        } else {
            DisplayRowTransitionContinuation::Continue
        }
    }

    pub(crate) fn sync_after_row_transition_if_visible(
        self,
        row_transition: TextMatrixRowTransition,
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        Self::sync_after_row_transition(synced_charpos, charpos, hit_row_range);
        DisplayRowTransitionContinuation::Continue
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWordWrapSourceAction {
    break_candidate: WordWrapBreakCandidate,
}

impl BufferTextWordWrapSourceAction {
    pub(crate) fn new(break_candidate: WordWrapBreakCandidate) -> Self {
        Self { break_candidate }
    }

    pub(crate) fn restore_row_output_progress(self, output_emitter: &mut WindowOutputEmitter) {
        output_emitter.truncate_display_points(self.break_candidate.display_point_count());
        let (row_first_display_pos, row_last_display_pos) =
            self.break_candidate.row_display_positions();
        output_emitter
            .restore_current_row_display_positions(row_first_display_pos, row_last_display_pos);
    }

    pub(crate) fn rewind_source_state(
        self,
        byte_idx: &mut usize,
        charpos: &mut i64,
        col: &mut usize,
    ) {
        *byte_idx = self.break_candidate.byte_idx();
        *charpos = self.break_candidate.charpos();
        *col = 0;
    }

    pub(crate) fn apply_before_row_transition(
        self,
        output_emitter: &mut WindowOutputEmitter,
        byte_idx: &mut usize,
        charpos: &mut i64,
        col: &mut usize,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        x: &mut f32,
        content_x: f32,
    ) {
        self.restore_row_output_progress(output_emitter);
        self.rewind_source_state(byte_idx, charpos, col);
        *x = content_x;
        row_extend.clear();
    }

    pub(crate) fn apply_after_row_transition(
        self,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
    ) {
        *charpos = self.charpos();
        hit_row_range.advance_to(*charpos);
        face_scan.invalidate();
    }

    pub(crate) fn apply_after_row_transition_and_prefix(
        self,
        row_transition: TextMatrixRowTransition,
        transition: DisplayRowOverflowTransitionPlan,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
        row_geometry: &DisplayRowGeometryState,
        row_visibility_limit: DisplayRowVisibilityLimit,
        render_state: DisplayRowTransitionRenderState<'_>,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        self.apply_after_row_transition(charpos, hit_row_range, face_scan);
        render_state.apply_overflow_prefix(transition);
        DisplayRowTransitionContinuation::after_visible_row_transition(
            row_transition,
            row_geometry,
            row_visibility_limit,
        )
    }

    pub(crate) fn charpos(self) -> i64 {
        self.break_candidate.charpos()
    }

    #[cfg(test)]
    pub(crate) fn byte_idx(self) -> usize {
        self.break_candidate.byte_idx()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextSpecialWrapSourceAction {
    charpos: i64,
}

impl BufferTextSpecialWrapSourceAction {
    pub(crate) fn new(charpos: i64) -> Self {
        Self { charpos }
    }

    pub(crate) fn apply_before_row_transition(
        self,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        x: &mut f32,
        content_x: f32,
    ) {
        *x = content_x;
        row_extend.clear();
    }

    pub(crate) fn hit_range_and_advance(
        self,
        hit_row_range: &mut HitRowRangeTracker,
    ) -> DisplayRowHitRange {
        let hit_range = hit_row_range.range_to(self.charpos);
        hit_row_range.advance_to(self.charpos);
        hit_range
    }

    pub(crate) fn transition_continuation(
        self,
        row_transition: TextMatrixRowTransition,
        row_geometry: &DisplayRowGeometryState,
        row_visibility_limit: DisplayRowVisibilityLimit,
    ) -> DisplayRowTransitionContinuation {
        DisplayRowTransitionContinuation::after_visible_row_transition(
            row_transition,
            row_geometry,
            row_visibility_limit,
        )
    }

    #[cfg(test)]
    pub(crate) fn charpos(self) -> i64 {
        self.charpos
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextCharacterWrapSourceAction {
    ch_start_byte_idx: usize,
    ch_start_charpos: i64,
}

impl BufferTextCharacterWrapSourceAction {
    pub(crate) fn new(ch_start_byte_idx: usize, ch_start_charpos: i64) -> Self {
        Self {
            ch_start_byte_idx,
            ch_start_charpos,
        }
    }

    pub(crate) fn from_decoded_char(source_char: BufferTextDecodedSourceChar) -> Self {
        Self::new(source_char.start_byte_idx(), source_char.start_charpos())
    }

    pub(crate) fn rewind_source_state(self, byte_idx: &mut usize, charpos: &mut i64) {
        *byte_idx = self.ch_start_byte_idx;
        *charpos = self.ch_start_charpos;
    }

    pub(crate) fn apply_before_row_transition(
        self,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        x: &mut f32,
        content_x: f32,
    ) {
        *x = content_x;
        row_extend.clear();
    }

    pub(crate) fn apply_after_row_transition(
        self,
        byte_idx: &mut usize,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
    ) {
        self.rewind_source_state(byte_idx, charpos);
        hit_row_range.advance_to(*charpos);
        face_scan.invalidate();
    }

    pub(crate) fn apply_after_visible_row_transition(
        self,
        row_transition: TextMatrixRowTransition,
        byte_idx: &mut usize,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
        row_geometry: &DisplayRowGeometryState,
        row_visibility_limit: DisplayRowVisibilityLimit,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        self.apply_after_row_transition(byte_idx, charpos, hit_row_range, face_scan);
        DisplayRowTransitionContinuation::after_visible_row_transition(
            row_transition,
            row_geometry,
            row_visibility_limit,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSpecialSourceCharRequest {
    range: BufferTextSourceRange,
    special_display: BufferTextSourceSpecialDisplay,
}

impl BufferTextSpecialSourceCharRequest {
    pub(crate) fn new(
        source_char: &BufferTextSourceChar,
        special_display: BufferTextSourceSpecialDisplay,
    ) -> Self {
        Self {
            range: source_char.range(),
            special_display,
        }
    }

    fn append_plan_at(
        &self,
        position: DisplayRowPosition,
    ) -> BufferTextSpecialSourceCharAppendPlan {
        BufferTextSpecialSourceCharAppendPlan {
            source_item: self.source_item_request(),
            position,
        }
    }

    pub(crate) fn kind(&self) -> BufferTextSourceSpecialDisplayKind {
        self.special_display.kind()
    }

    fn requires_overflow_measurement(&self) -> bool {
        self.special_display.is_control()
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

    fn source_item_request(&self) -> BufferTextSourceItemRequest {
        BufferTextSourceItemRequest::new(
            self.range,
            self.special_display.clone().into_append_item(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextSourceSpecialDisplayKind {
    Control,
    Nobreak,
    Glyphless,
}

impl BufferTextSourceSpecialDisplayKind {
    pub(crate) fn invalidates_face_after_append(self) -> bool {
        matches!(self, Self::Control | Self::Nobreak)
    }

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

pub(crate) struct BufferTextSpecialOverflowRenderRequest<'a> {
    prepared_append: &'a BufferTextSpecialSourceCharPreparedAppend,
    text: &'a [u8],
    text_start_byte: usize,
    x_px: f32,
    right_edge_px: f32,
    truncate_lines: bool,
    row_visibility_limit: DisplayRowVisibilityLimit,
    content_x: f32,
    has_prefix: bool,
    row_geometry_defaults: DisplayRowGeometryDefaults,
    text_matrix_row_base: usize,
    max_rows: usize,
    row_limit: DisplayRowLimit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextSpecialOverflowRenderOutcome {
    Fits,
    AppendPrepared(DisplayRowTransitionContinuation),
    ContinueBufferWalk(DisplayRowTransitionContinuation),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum BufferTextSpecialSourceCharOverflowAction {
    Fits,
    Truncate {
        transition: DisplayRowOverflowTransitionPlan,
    },
    Wrap {
        transition: DisplayRowOverflowTransitionPlan,
    },
}

impl BufferTextSpecialSourceCharOverflowAction {
    fn for_decision(decision: SpecialTextRowOverflowDecision) -> Self {
        match decision {
            SpecialTextRowOverflowDecision::Fits => Self::Fits,
            SpecialTextRowOverflowDecision::Truncate => Self::Truncate {
                transition: DisplayRowOverflowTransitionPlan::truncation(
                    TextRowTransitionStatePolicy::special_truncation(),
                ),
            },
            SpecialTextRowOverflowDecision::Wrap => Self::Wrap {
                transition: DisplayRowOverflowTransitionPlan::visual_wrap(
                    TextRowTransitionStatePolicy::special_visual_wrap(),
                ),
            },
        }
    }
}

impl BufferTextSpecialOverflowRenderOutcome {
    pub(crate) fn should_break(self) -> bool {
        matches!(
            self,
            Self::AppendPrepared(
                DisplayRowTransitionContinuation::Exhausted
                    | DisplayRowTransitionContinuation::Hidden
            ) | Self::ContinueBufferWalk(
                DisplayRowTransitionContinuation::Exhausted
                    | DisplayRowTransitionContinuation::Hidden
            )
        )
    }

    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(
            self,
            Self::ContinueBufferWalk(DisplayRowTransitionContinuation::Continue)
        )
    }
}

impl<'a> BufferTextSpecialOverflowRenderRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        prepared_append: &'a BufferTextSpecialSourceCharPreparedAppend,
        text: &'a [u8],
        text_start_byte: usize,
        x_px: f32,
        right_edge_px: f32,
        truncate_lines: bool,
        row_visibility_limit: DisplayRowVisibilityLimit,
        content_x: f32,
        has_prefix: bool,
        row_geometry_defaults: DisplayRowGeometryDefaults,
        text_matrix_row_base: usize,
        max_rows: usize,
        row_limit: DisplayRowLimit,
    ) -> Self {
        Self {
            prepared_append,
            text,
            text_start_byte,
            x_px,
            right_edge_px,
            truncate_lines,
            row_visibility_limit,
            content_x,
            has_prefix,
            row_geometry_defaults,
            text_matrix_row_base,
            max_rows,
            row_limit,
        }
    }

    pub(crate) fn render_if_needed_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferTextSpecialOverflowRenderState<'_, '_>,
    ) -> BufferTextSpecialOverflowRenderOutcome {
        let BufferTextSpecialOverflowRenderState {
            byte_idx,
            charpos,
            col,
            source_render,
            row_extend,
            x,
            line_numbers,
            row_geometry,
            row_flags,
            hit_rows,
            hit_row_range,
            prefix_request,
            hscroll_skip,
            word_wrap,
            trailing_whitespace,
            row_y_positions,
        } = state;
        let mut source_render = source_render;

        match self.prepared_append.overflow_action(
            self.x_px,
            self.right_edge_px,
            self.truncate_lines,
        ) {
            None | Some(BufferTextSpecialSourceCharOverflowAction::Fits) => {
                BufferTextSpecialOverflowRenderOutcome::Fits
            }
            Some(BufferTextSpecialSourceCharOverflowAction::Truncate { transition }) => {
                let truncation_skip =
                    BufferTextTruncationSkipAction::consume_decoded_char_and_rest_of_line(
                        self.text, byte_idx, charpos,
                    );
                truncation_skip.apply_before_row_transition(
                    line_numbers,
                    row_extend,
                    x,
                    self.content_x,
                );
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    self.row_geometry_defaults,
                    self.text_matrix_row_base,
                    row_y_positions,
                    self.max_rows,
                    row_geometry,
                    row_flags,
                    self.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow_then_row_start(
                    transition,
                    hit_row_range.range_to(*charpos),
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
                    DisplayRowTransitionRenderState::new(
                        prefix_request,
                        self.has_prefix,
                        line_numbers,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                    ),
                    col,
                );
                let synced_charpos = buffer
                    .layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(
                        self.text_start_byte + *byte_idx,
                    ))
                    .get() as i64;
                BufferTextSpecialOverflowRenderOutcome::ContinueBufferWalk(
                    truncation_skip.sync_after_row_transition_if_visible(
                        row_transition,
                        synced_charpos,
                        charpos,
                        hit_row_range,
                    ),
                )
            }
            Some(BufferTextSpecialSourceCharOverflowAction::Wrap { transition }) => {
                let special_wrap_action = BufferTextSpecialWrapSourceAction::new(*charpos);
                special_wrap_action.apply_before_row_transition(row_extend, x, self.content_x);
                let hit_range = special_wrap_action.hit_range_and_advance(hit_row_range);
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    self.row_geometry_defaults,
                    self.text_matrix_row_base,
                    row_y_positions,
                    self.max_rows,
                    row_geometry,
                    row_flags,
                    self.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow_then_row_start(
                    transition,
                    hit_range,
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
                    DisplayRowTransitionRenderState::new(
                        prefix_request,
                        self.has_prefix,
                        line_numbers,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                    ),
                    col,
                );
                BufferTextSpecialOverflowRenderOutcome::AppendPrepared(
                    special_wrap_action.transition_continuation(
                        row_transition,
                        row_geometry,
                        self.row_visibility_limit,
                    ),
                )
            }
        }
    }
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
        truncate_lines: bool,
    ) -> Option<SpecialTextRowOverflowDecision> {
        Some(SpecialTextRowOverflowDecision::for_width(
            x_px,
            self.measured_width_px()?,
            right_edge_px,
            truncate_lines,
        ))
    }

    pub(crate) fn overflow_action(
        &self,
        x_px: f32,
        right_edge_px: f32,
        truncate_lines: bool,
    ) -> Option<BufferTextSpecialSourceCharOverflowAction> {
        Some(BufferTextSpecialSourceCharOverflowAction::for_decision(
            self.overflow_decision(x_px, right_edge_px, truncate_lines)?,
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
struct BufferTextSourceAdvanceRequest<'text> {
    text: &'text [u8],
    byte_idx: usize,
    range: BufferTextSourceRange,
    position: DisplayRowPosition,
    cluster: BufferTextSourceClusterState,
}

impl<'text> BufferTextSourceAdvanceRequest<'text> {
    fn text(self) -> &'text [u8] {
        self.text
    }

    fn byte_idx(self) -> usize {
        self.byte_idx
    }

    fn range(self) -> BufferTextSourceRange {
        self.range
    }

    fn position(self) -> DisplayRowPosition {
        self.position
    }

    fn cluster(self) -> BufferTextSourceClusterState {
        self.cluster
    }

    fn append_plan(
        self,
        resolved_advance: ResolvedBufferTextSourceAdvance,
    ) -> BufferTextSourceCharAppendPlan {
        BufferTextSourceCharAppendPlan {
            source_text: BufferTextSourceTextRequest::new(self.range, resolved_advance),
            position: self.position,
        }
    }
}

impl BufferTextSourceSpecialDisplay {
    pub(crate) fn for_precluster_char(ch: char, nobreak_display_policy: i32) -> Option<Self> {
        if Self::is_control_char(ch) {
            Some(Self::Control(BufferTextSourceAppendItem::ControlChar {
                ch,
            }))
        } else {
            BufferTextSourceAppendItem::nobreak_display(ch, nobreak_display_policy)
                .map(Self::Nobreak)
        }
    }

    pub(crate) fn for_cluster_state(cluster: BufferTextSourceClusterState) -> Option<Self> {
        BufferTextSourceAppendItem::glyphless_display(cluster).map(Self::Glyphless)
    }

    pub(crate) fn into_append_item(self) -> BufferTextSourceAppendItem {
        match self {
            Self::Control(item) | Self::Nobreak(item) | Self::Glyphless(item) => item,
        }
    }

    fn is_control(&self) -> bool {
        matches!(self, Self::Control(_))
    }

    #[cfg(test)]
    fn is_nobreak(&self) -> bool {
        matches!(self, Self::Nobreak(_))
    }

    fn kind(&self) -> BufferTextSourceSpecialDisplayKind {
        match self {
            Self::Control(_) => BufferTextSourceSpecialDisplayKind::Control,
            Self::Nobreak(_) => BufferTextSourceSpecialDisplayKind::Nobreak,
            Self::Glyphless(_) => BufferTextSourceSpecialDisplayKind::Glyphless,
        }
    }

    fn is_control_char(ch: char) -> bool {
        (ch < ' ' && ch != '\n' && ch != '\t') || ch == '\x7F'
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BufferTextSourceFallbackWidthPolicy {
    Columns(usize),
}

impl BufferTextSourceFallbackWidthPolicy {
    fn for_append_item(item: &BufferTextSourceAppendItem) -> Self {
        match item {
            BufferTextSourceAppendItem::ControlChar { .. } => Self::Columns(2),
            BufferTextSourceAppendItem::SourceMappedText { text } => {
                Self::Columns(text.chars().count().max(1))
            }
            BufferTextSourceAppendItem::Glyphless { .. } => Self::Columns(1),
        }
    }

    fn width_px(self, fallback_char_width: f32) -> f32 {
        self.columns() as f32 * fallback_char_width.max(1.0)
    }

    fn columns(self) -> usize {
        match self {
            Self::Columns(columns) => columns,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextSourceClusterState {
    ch: char,
    tail: Option<(char, bool)>,
    is_cluster_continuation: bool,
}

impl BufferTextSourceClusterState {
    pub(crate) fn for_char(ch: char, tail: Option<(char, bool)>) -> Self {
        Self {
            ch,
            tail,
            is_cluster_continuation: crate::composition::continues_cluster(ch, tail),
        }
    }

    pub(crate) fn is_cluster_continuation(self) -> bool {
        self.is_cluster_continuation
    }

    fn ch(self) -> char {
        self.ch
    }

    fn has_tail(self) -> bool {
        self.tail.is_some()
    }
}

impl BufferTextSourceAppendItem {
    pub(crate) fn nobreak_display(ch: char, display_policy: i32) -> Option<Self> {
        let text = match (display_policy, ch) {
            (1, '\u{00A0}') => " ",
            (1, '\u{00AD}') => "-",
            (2, '\u{00A0}') => "\\ ",
            (2, '\u{00AD}') => "\\-",
            _ => return None,
        };
        Some(Self::SourceMappedText { text: text.into() })
    }

    pub(crate) fn glyphless_display(cluster: BufferTextSourceClusterState) -> Option<Self> {
        let ch = cluster.ch();
        if cluster.has_tail() && crate::composition::is_composition_joiner(ch) {
            return None;
        }
        let method = glyphless_method_for_char(ch, GlyphlessJoinerPolicy::ClassifyAsGlyphless)?;
        Some(Self::Glyphless { ch, method })
    }

    fn append_kind(&self) -> DisplayRowAppendKind {
        match self {
            Self::ControlChar { .. } => DisplayRowAppendKind::ControlChar,
            Self::SourceMappedText { .. } => DisplayRowAppendKind::SourceMappedText,
            Self::Glyphless { .. } => DisplayRowAppendKind::Glyphless,
        }
    }

    fn fallback_width_policy(&self) -> BufferTextSourceFallbackWidthPolicy {
        BufferTextSourceFallbackWidthPolicy::for_append_item(self)
    }

    fn into_display_item_kind(self) -> DisplayItemKind {
        match self {
            Self::ControlChar { ch } => DisplayItemKind::ControlChar { ch },
            Self::SourceMappedText { text } => {
                DisplayItemKind::SourceMappedText(DisplaySourceMappedText::new(text))
            }
            Self::Glyphless { ch, method } => {
                DisplayItemKind::Glyphless(DisplayGlyphless { ch, method })
            }
        }
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
        let operation = BufferTextSourceAppendOperation::for_display_item_request(
            source_item,
            self.buffer_id,
            self.buffer,
            self.face_id,
            self.base_face,
            self.frame.clone(),
            position,
        )?;
        let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
        operation.render_to_text_row_and_emit(state, &mut render_policy)
    }

    fn measure_source_request_width_to_text_row(
        &self,
        state: &mut TextRowSourceMeasureState<'_>,
        source_item: BufferTextSourceItemRequest,
        position: DisplayRowPosition,
    ) -> Option<f32> {
        let operation = BufferTextSourceAppendOperation::for_display_item_request(
            source_item,
            self.buffer_id,
            self.buffer,
            self.face_id,
            self.base_face,
            self.frame.clone(),
            position,
        )?;
        let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
        Some(
            operation
                .measure_to_text_row(state, &mut render_policy)?
                .metrics
                .width_px,
        )
    }

    fn measure_source_request_width_or_active_face_fallback_to_text_row(
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

#[derive(Clone)]
pub(crate) struct DisplayReplacementActiveFaceMeasurer {
    active_face_state: DisplayRowActiveFaceState,
}

impl DisplayReplacementActiveFaceMeasurer {
    pub(crate) fn from_active_face_state(active_face_state: &DisplayRowActiveFaceState) -> Self {
        Self {
            active_face_state: active_face_state.clone(),
        }
    }

    pub(crate) fn source_char_width_px(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        ch: char,
        fallback_advance_px: f32,
    ) -> f32 {
        self.active_face_state
            .advance_for_char(font_metrics, ch, fallback_advance_px)
    }

    fn replacement_string_cursor_slot_width_px(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        replacement: &str,
        fallback_char_width: f32,
    ) -> f32 {
        replacement
            .chars()
            .next()
            .map(|ch| self.source_char_width_px(font_metrics, ch, fallback_char_width))
            .unwrap_or_else(|| fallback_char_width.max(1.0))
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

#[derive(Clone)]
pub(crate) struct DisplayReplacementStringAppendItem {
    value: Value,
    origin: DisplayOrigin,
    base_face_policy: BaseFacePolicy,
    source_id: LispStringSourceId,
    active_face_state: DisplayRowActiveFaceState,
    cursor_slot_width_px: f32,
    is_empty: bool,
}

impl DisplayReplacementStringAppendItem {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn display_property_string(
        value: Value,
        anchor_charpos: CharPos0,
        source: DisplayPropertySource,
        source_id: u64,
        active_face_state: &DisplayRowActiveFaceState,
        font_metrics: &mut Option<FontMetricsService>,
        fallback_char_width: f32,
    ) -> Option<Self> {
        let replacement = value.as_utf8_str()?;
        let measurer =
            DisplayReplacementActiveFaceMeasurer::from_active_face_state(active_face_state);
        Some(Self {
            value,
            origin: DisplayOrigin::DisplayPropertyString {
                anchor_charpos,
                source,
            },
            base_face_policy: BaseFacePolicy::DisplayPropertyUnderlyingFace,
            source_id: LispStringSourceId::display_replacement(source_id),
            active_face_state: active_face_state.clone(),
            cursor_slot_width_px: measurer.replacement_string_cursor_slot_width_px(
                font_metrics,
                replacement,
                fallback_char_width,
            ),
            is_empty: replacement.is_empty(),
        })
    }

    pub(crate) fn cursor_slot_width_px(&self) -> f32 {
        self.cursor_slot_width_px
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.is_empty
    }

    pub(crate) fn origin(&self) -> DisplayOrigin {
        self.origin
    }

    pub(crate) fn base_face_policy(&self) -> BaseFacePolicy {
        self.base_face_policy
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
            self.source_id,
            self.value,
            replacement_source,
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct DisplayReplacementStringSourceAppendRequest {
    position: DisplayRowPosition,
    source_id: LispStringSourceId,
    value: Value,
    replacement_source: BufferDisplayReplacementSource,
}

impl DisplayReplacementStringSourceAppendRequest {
    fn new(
        position: DisplayRowPosition,
        source_id: LispStringSourceId,
        value: Value,
        replacement_source: BufferDisplayReplacementSource,
    ) -> Self {
        Self {
            position,
            source_id,
            value,
            replacement_source,
        }
    }

    fn position(self) -> DisplayRowPosition {
        self.position
    }

    fn into_source(
        self,
        fallback_face_id: u32,
    ) -> Option<BufferDisplayReplacementStringSource<LispStringSourceCursor>> {
        let string_source = LispStringSourceCursor::new(
            self.source_id.raw(),
            self.value,
            RenderFaceRef::FaceId(fallback_face_id),
        )?;
        Some(BufferDisplayReplacementStringSource::new(
            self.replacement_source,
            string_source,
        ))
    }

    fn render_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        append_context: &DisplayReplacementAppendContext<'_>,
        item_policy: &mut impl DisplayRowRenderPolicy,
    ) -> DisplayRowPosition {
        let position = self.position();
        let Some(source) = self.into_source(append_context.face_id) else {
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
    item: DisplayReplacementStringAppendItem,
    replacement_base_face: Option<DisplayStringBaseFace>,
}

impl DisplayReplacementStringAppendRequest {
    fn new(
        item: DisplayReplacementStringAppendItem,
        replacement_base_face: Option<DisplayStringBaseFace>,
    ) -> Self {
        Self {
            item,
            replacement_base_face,
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
        let Some(replacement_base_face) = self.replacement_base_face else {
            debug_assert!(false, "display string replacement missing base face");
            return position;
        };
        let source_request = self
            .item
            .source_append_request(replacement_append_context.replacement_source, position);
        let mut item_policy = self.item.string_item_measurer();
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

#[cfg(test)]
fn append_raw_display_replacement_item_to_text_row_and_emit(
    state: &mut TextRowSourceRenderState<'_>,
    item: DisplayItem,
    base_face: &ResolvedFace,
    fallback_face_id: u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    DisplayRowSingleItemAppendOperation::new(
        item,
        base_face,
        fallback_face_id,
        frame,
        position,
        DisplayRowAppendKind::DisplayReplacement,
    )
    .render_to_text_row_and_emit(state, &mut render_policy)
}

#[derive(Clone, Debug)]
enum DisplayReplacementAppendItem {
    Media(DisplayMediaReplacement),
    Stretch(DisplayReplacementBox),
    SourceMappedText(Box<str>),
}

impl DisplayReplacementAppendItem {
    fn stretch(item: DisplayReplacementStretchAppendItem) -> Self {
        Self::Stretch(item.geometry())
    }

    fn media(item: DisplayReplacementMediaAppendItem) -> Self {
        Self::Media(item.media())
    }

    fn source_mapped_text(item: DisplayReplacementSourceMappedTextAppendItem) -> Self {
        Self::SourceMappedText(item.text())
    }

    fn into_display_item(
        self,
        replacement_source: BufferDisplayReplacementSource,
        face_id: u32,
    ) -> DisplayItem {
        match self {
            Self::Media(media) => replacement_source.media_item(face_id, media),
            Self::Stretch(geometry) => replacement_source.stretch_item(face_id, geometry),
            Self::SourceMappedText(text) => {
                replacement_source.source_mapped_text_item(face_id, text)
            }
        }
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

#[derive(Clone, Copy, Debug)]
pub(crate) struct DisplayReplacementStretchAppendItem {
    geometry: DisplayReplacementBox,
    width_px: f32,
    height_px: f32,
    ascent_px: f32,
    cursor_slot_width_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayReplacementSpaceGeometry {
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) ascent: f32,
}

#[derive(Clone, Copy, Debug)]
enum DisplayReplacementSpaceWidthPolicy {
    Explicit(Value),
    Relative { factor: f32 },
    AlignTo(Value),
    Default,
}

impl DisplayReplacementSpaceWidthPolicy {
    fn from_items(items: &[Value]) -> Self {
        if let Some(prop) = display_space_plist_value(items, DisplaySpaceKey::Width)
            && !prop.is_nil()
        {
            Self::Explicit(prop)
        } else if let Some(prop) = display_space_plist_value(items, DisplaySpaceKey::RelativeWidth)
            && let Some(factor) = display_space_positive_number(prop)
        {
            Self::Relative { factor }
        } else if let Some(prop) = display_space_plist_value(items, DisplaySpaceKey::AlignTo)
            && !prop.is_nil()
        {
            Self::AlignTo(prop)
        } else {
            Self::Default
        }
    }

    fn zero_width_allowed(self) -> bool {
        matches!(self, Self::AlignTo(_))
    }

    fn resolve(
        self,
        pctx: &crate::display_pixel_calc::PixelCalcContext,
        current_x: f32,
        content_x: f32,
        display_char_width: f32,
        default_width: f32,
    ) -> f32 {
        use crate::display_pixel_calc::calc_pixel_width_or_height;

        match self {
            Self::Explicit(prop) => calc_pixel_width_or_height(pctx, &prop, true, None)
                .map(|pixels| pixels as f32)
                .unwrap_or(default_width),
            Self::Relative { factor } => factor * display_char_width.max(0.0),
            Self::AlignTo(prop) => {
                let mut align_to: i32 = -1;
                if let Some(pixels) =
                    calc_pixel_width_or_height(pctx, &prop, true, Some(&mut align_to))
                {
                    let target_x = if align_to >= 0 {
                        align_to as f32 + pixels as f32
                    } else {
                        content_x + pixels as f32
                    };
                    (target_x - current_x).max(0.0)
                } else {
                    default_width
                }
            }
            Self::Default => default_width,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum DisplayReplacementSpaceHeightPolicy {
    Explicit(Value),
    Relative { factor: f32 },
    Default,
}

impl DisplayReplacementSpaceHeightPolicy {
    fn from_items(items: &[Value]) -> Self {
        if let Some(prop) = display_space_plist_value(items, DisplaySpaceKey::Height)
            && !prop.is_nil()
        {
            Self::Explicit(prop)
        } else if let Some(prop) = display_space_plist_value(items, DisplaySpaceKey::RelativeHeight)
            && let Some(factor) = display_space_positive_number(prop)
        {
            Self::Relative { factor }
        } else {
            Self::Default
        }
    }

    fn zero_height_allowed(self) -> bool {
        matches!(self, Self::Explicit(_))
    }

    fn resolve(
        self,
        pctx: &crate::display_pixel_calc::PixelCalcContext,
        default_height: f32,
    ) -> f32 {
        use crate::display_pixel_calc::calc_pixel_width_or_height;

        match self {
            Self::Explicit(prop) => calc_pixel_width_or_height(pctx, &prop, false, None)
                .map(|pixels| pixels as f32)
                .unwrap_or(default_height),
            Self::Relative { factor } => default_height * factor,
            Self::Default => default_height,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum DisplayReplacementSpaceAscentPolicy {
    Percent { percent: f32 },
    Pixel(Value),
    Default,
}

impl DisplayReplacementSpaceAscentPolicy {
    fn from_items(items: &[Value]) -> Self {
        let Some(prop) = display_space_plist_value(items, DisplaySpaceKey::Ascent) else {
            return Self::Default;
        };
        if let Some(percent) = display_space_positive_number(prop)
            && percent <= 100.0
        {
            Self::Percent { percent }
        } else if !prop.is_nil() {
            Self::Pixel(prop)
        } else {
            Self::Default
        }
    }

    fn resolve(
        self,
        pctx: &crate::display_pixel_calc::PixelCalcContext,
        height: f32,
        default_ascent: f32,
        default_height: f32,
    ) -> f32 {
        use crate::display_pixel_calc::calc_pixel_width_or_height;

        match self {
            Self::Percent { percent } => height * percent / 100.0,
            Self::Pixel(prop) => calc_pixel_width_or_height(pctx, &prop, false, None)
                .map(|pixels| (pixels as f32).max(0.0).min(height))
                .unwrap_or_else(|| Self::default_ascent(height, default_ascent, default_height)),
            Self::Default => Self::default_ascent(height, default_ascent, default_height),
        }
    }

    fn default_ascent(height: f32, default_ascent: f32, default_height: f32) -> f32 {
        height * default_ascent / default_height
    }
}

fn display_space_plist_value(items: &[Value], wanted: DisplaySpaceKey) -> Option<Value> {
    let mut i = 1;
    while i + 1 < items.len() {
        if DisplaySpaceKey::from_lisp_value(items[i]) == Some(wanted) {
            return Some(items[i + 1]);
        }
        i += 2;
    }
    None
}

impl DisplayReplacementStretchAppendItem {
    pub(crate) fn from_extents(width_px: f32, height_px: f32, ascent_px: f32) -> Self {
        let width_px = width_px.max(0.0);
        let height_px = height_px.max(0.0);
        let ascent_px = ascent_px.max(0.0);
        Self {
            geometry: DisplayReplacementBox::new(width_px, height_px, ascent_px),
            width_px,
            height_px,
            ascent_px,
            cursor_slot_width_px: width_px,
        }
    }

    pub(crate) fn from_space_extents(
        width_px: f32,
        height_px: f32,
        ascent_px: f32,
        fallback_cursor_width_px: f32,
    ) -> Self {
        let mut item = Self::from_extents(width_px, height_px, ascent_px);
        item.cursor_slot_width_px = item.width_px.max(fallback_cursor_width_px);
        item
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn display_space_geometry(
        spec: &Value,
        current_x: f32,
        content_x: f32,
        face_char_w: f32,
        display_char_width: f32,
        default_height: f32,
        default_ascent: f32,
        params: &WindowParams,
    ) -> DisplayReplacementSpaceGeometry {
        use crate::display_pixel_calc::PixelCalcContext;

        let default_width = params.char_width.max(1.0);
        let default_height = if params.window_system {
            default_height.max(1.0)
        } else {
            params.char_height.max(1.0)
        };
        let default_ascent = if params.window_system {
            default_ascent.max(0.0).min(default_height)
        } else {
            default_height
        };
        let Some(items) = neovm_core::emacs_core::value::list_to_vec(spec) else {
            return DisplayReplacementSpaceGeometry {
                width: default_width,
                height: default_height,
                ascent: default_ascent,
            };
        };

        let pctx = PixelCalcContext {
            frame_column_width: params.char_width.max(1.0) as f64,
            frame_line_height: params.char_height.max(1.0) as f64,
            frame_res_x: 96.0,
            frame_res_y: 96.0,
            face_font_height: default_height as f64,
            face_font_width: face_char_w.round().max(1.0) as f64,
            text_area_left: params.text_bounds.x as f64,
            text_area_right: (params.text_bounds.x + params.text_bounds.width) as f64,
            text_area_width: params.text_bounds.width as f64,
            left_margin_left: (params.text_bounds.x
                - params.left_fringe_width
                - params.left_margin_width) as f64,
            left_margin_width: params.left_margin_width as f64,
            right_margin_left: (params.text_bounds.x
                + params.text_bounds.width
                + params.right_fringe_width) as f64,
            right_margin_width: params.right_margin_width as f64,
            left_fringe_width: params.left_fringe_width as f64,
            right_fringe_width: params.right_fringe_width as f64,
            fringes_outside_margins: false,
            scroll_bar_width: 0.0,
            scroll_bar_on_left: false,
            line_number_pixel_width: 0.0,
            symbol_values: std::collections::HashMap::new(),
        };

        let width_policy = DisplayReplacementSpaceWidthPolicy::from_items(&items);
        let mut width = width_policy.resolve(
            &pctx,
            current_x,
            content_x,
            display_char_width,
            default_width,
        );
        if width <= 0.0 && (width < 0.0 || !width_policy.zero_width_allowed()) {
            width = 1.0;
        }

        let (height, ascent) = if params.window_system {
            let height_policy = DisplayReplacementSpaceHeightPolicy::from_items(&items);
            let mut height = height_policy.resolve(&pctx, default_height);
            if height <= 0.0 && (height < 0.0 || !height_policy.zero_height_allowed()) {
                height = 1.0;
            }

            let ascent = DisplayReplacementSpaceAscentPolicy::from_items(&items).resolve(
                &pctx,
                height,
                default_ascent,
                default_height,
            );
            (height, ascent)
        } else {
            (1.0, 1.0)
        };

        DisplayReplacementSpaceGeometry {
            width,
            height,
            ascent: ascent.max(0.0).min(height),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_display_space_spec(
        spec: &Value,
        current_x: f32,
        content_x: f32,
        face_char_w: f32,
        display_char_width: f32,
        default_height: f32,
        default_ascent: f32,
        fallback_cursor_width_px: f32,
        params: &WindowParams,
    ) -> Self {
        let geometry = Self::display_space_geometry(
            spec,
            current_x,
            content_x,
            face_char_w,
            display_char_width,
            default_height,
            default_ascent,
            params,
        );
        Self::from_space_extents(
            geometry.width,
            geometry.height,
            geometry.ascent,
            fallback_cursor_width_px,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_display_space_property(
        spec: &Value,
        source_text: &[u8],
        active_face_state: &DisplayRowActiveFaceState,
        font_metrics: &mut Option<FontMetricsService>,
        current_x: f32,
        content_x: f32,
        default_char_width: f32,
        default_height: f32,
        default_ascent: f32,
        params: &WindowParams,
    ) -> Self {
        let (display_ch, _) = decode_utf8(source_text);
        let display_char_width = Self::source_char_width_px(
            active_face_state,
            font_metrics,
            display_ch,
            default_char_width,
        );
        Self::from_display_space_spec(
            spec,
            current_x,
            content_x,
            default_char_width,
            display_char_width,
            default_height,
            default_ascent,
            default_char_width,
            params,
        )
    }

    fn source_char_width_px(
        active_face_state: &DisplayRowActiveFaceState,
        font_metrics: &mut Option<FontMetricsService>,
        ch: char,
        fallback_advance_px: f32,
    ) -> f32 {
        DisplayReplacementActiveFaceMeasurer::from_active_face_state(active_face_state)
            .source_char_width_px(font_metrics, ch, fallback_advance_px)
    }

    pub(crate) fn width_px(self) -> f32 {
        self.width_px
    }

    pub(crate) fn height_px(self) -> f32 {
        self.height_px
    }

    pub(crate) fn ascent_px(self) -> f32 {
        self.ascent_px
    }

    pub(crate) fn cursor_slot_width_px(self) -> f32 {
        self.cursor_slot_width_px
    }

    fn geometry(self) -> DisplayReplacementBox {
        self.geometry
    }

    fn append_request(
        self,
        position: DisplayRowPosition,
    ) -> Option<DisplayReplacementItemAppendRequest> {
        (self.width_px() > 0.0).then(|| {
            DisplayReplacementItemAppendRequest::active_face(
                DisplayReplacementAppendItem::stretch(self),
                position,
            )
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayReplacementMediaAppendItem {
    media: DisplayMediaReplacement,
    cursor_face_height: f32,
    cursor_face_ascent: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DisplayReplacementMediaAppendResolution {
    Media(DisplayReplacementMediaAppendItem),
    Placeholder(DisplayReplacementSourceMappedTextAppendItem),
}

#[derive(Clone)]
pub(crate) enum DisplayPropertyReplacementAppendItem {
    String(DisplayReplacementStringAppendItem),
    Stretch(DisplayReplacementStretchAppendItem),
    Media(DisplayReplacementMediaAppendResolution),
}

pub(crate) enum BufferDisplayPropertyTextAppendAction {
    Replacement(BufferDisplayPropertyTextReplacementOutcome),
    Modifiers(BufferDisplayPropertyTextModifierAction),
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferDisplayPropertyTextWalkOutcome {
    Continue,
    ReplacementConsumed,
    FaceStateChanged,
}

pub(crate) struct BufferDisplayPropertyTextAppendRequest<'a> {
    value: Value,
    buffer_id: BufferId,
    anchor_charpos: CharPos0,
    anchor_bytepos: EmacsBytePos,
    source_text: &'a [u8],
    active_face_state: &'a DisplayRowActiveFaceState,
    current_x: f32,
    content_x: f32,
    params: &'a WindowParams,
    glyph_y_offset: f32,
    default_row_height: f32,
    start_position: DisplayRowPosition,
    next_change: i64,
    skip_to: i64,
}

pub(crate) struct BufferDisplayPropertyTextRenderContext<'a> {
    buffer_id: BufferId,
    text_start_byte: usize,
    text: &'a [u8],
    active_face_state: &'a DisplayRowActiveFaceState,
    current_x: f32,
    content_x: f32,
    params: &'a WindowParams,
    glyph_y_offset: f32,
    default_row_height: f32,
    start_position: DisplayRowPosition,
}

pub(crate) struct BufferDisplayPropertyCheckpointRenderRequest<'a, B: LayoutBufferView> {
    face_resolution_context: BufferCurrentFaceResolutionContext<'a, B>,
    buffer_id: BufferId,
    text_start_byte: usize,
    text: &'a [u8],
    current_x: f32,
    content_x: f32,
    params: &'a WindowParams,
    glyph_y_offset: f32,
    default_row_height: f32,
    start_position: DisplayRowPosition,
    charpos: i64,
    byte_idx: usize,
    accessible_end: i64,
}

pub(crate) struct BufferDisplayPropertyCheckpointRenderState<'a, 'emit> {
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
    pub(crate) append_surface: &'a DisplayRowAppendSurface,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) checkpoints: &'emit mut TextPropertyScanCheckpoints,
    pub(crate) face_scan: &'emit mut FaceScanCheckpoint,
    pub(crate) active_face_state: &'emit mut DisplayRowActiveFaceState,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) box_face: &'emit mut BoxFaceRowState,
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) raise_span: &'emit mut ActiveDisplayPropertySpan<f32>,
    pub(crate) height_span: &'emit mut ActiveDisplayPropertySpan<f32>,
    pub(crate) point_charpos: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferDisplayPropertyTextReplacementOutcome {
    replacement: DisplayPropertyReplacementAppendOutcome,
    skip_to: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferDisplayPropertyTextModifierAction {
    raise_offset_px: Option<f32>,
    height_factor: Option<f32>,
    next_change: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferDisplayPropertyTextModifierStateOutcome {
    height_face_changed: bool,
}

impl BufferDisplayPropertyTextModifierStateOutcome {
    fn new(height_face_changed: bool) -> Self {
        Self {
            height_face_changed,
        }
    }

    pub(crate) fn height_face_changed(self) -> bool {
        self.height_face_changed
    }
}

impl BufferDisplayPropertyTextWalkOutcome {
    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(self, Self::ReplacementConsumed)
    }

    pub(crate) fn should_resolve_face(self) -> bool {
        matches!(self, Self::FaceStateChanged)
    }
}

impl BufferDisplayPropertyTextAppendAction {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_to_buffer_walk_state(
        self,
        text: &[u8],
        byte_idx: &mut usize,
        charpos: &mut i64,
        x: &mut f32,
        col: &mut usize,
        cursor_info: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        raise_span: &mut ActiveDisplayPropertySpan<f32>,
        height_span: &mut ActiveDisplayPropertySpan<f32>,
        face_scan: &mut FaceScanCheckpoint,
    ) -> BufferDisplayPropertyTextWalkOutcome {
        match self {
            Self::Replacement(replacement_outcome) => {
                replacement_outcome.capture_cursor_info_if_point(
                    cursor_info,
                    active_face_state,
                    row_geometry,
                    point_charpos,
                    *charpos,
                    *byte_idx,
                );
                replacement_outcome.apply_to_walk_state(text, byte_idx, charpos, x, col);
                BufferDisplayPropertyTextWalkOutcome::ReplacementConsumed
            }
            Self::Modifiers(modifiers) => {
                if modifiers
                    .apply_to_walk_state(raise_span, height_span, face_scan)
                    .height_face_changed()
                {
                    BufferDisplayPropertyTextWalkOutcome::FaceStateChanged
                } else {
                    BufferDisplayPropertyTextWalkOutcome::Continue
                }
            }
            Self::None => BufferDisplayPropertyTextWalkOutcome::Continue,
        }
    }
}

impl<'a> BufferDisplayPropertyTextRenderContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        buffer_id: BufferId,
        text_start_byte: usize,
        text: &'a [u8],
        active_face_state: &'a DisplayRowActiveFaceState,
        current_x: f32,
        content_x: f32,
        params: &'a WindowParams,
        glyph_y_offset: f32,
        default_row_height: f32,
        start_position: DisplayRowPosition,
    ) -> Self {
        Self {
            buffer_id,
            text_start_byte,
            text,
            active_face_state,
            current_x,
            content_x,
            params,
            glyph_y_offset,
            default_row_height,
            start_position,
        }
    }

    pub(crate) fn resolve_and_append_at_checkpoint<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        append_surface: &DisplayRowAppendSurface,
        row_geometry: &mut DisplayRowGeometryState,
        checkpoints: &mut TextPropertyScanCheckpoints,
        charpos: i64,
        byte_idx: usize,
        accessible_end: i64,
    ) -> BufferDisplayPropertyTextAppendAction {
        if !checkpoints.should_check_display(charpos) {
            return BufferDisplayPropertyTextAppendAction::None;
        }

        let text_props = RustTextPropAccess::new(buffer);
        let (display_property, next_change) = text_props.check_display_prop(charpos);
        checkpoints.record_display_next(next_change);
        let Some(value) = display_property else {
            return BufferDisplayPropertyTextAppendAction::None;
        };

        BufferDisplayPropertyTextAppendRequest::for_text_property(
            value,
            self.buffer_id,
            CharPos0::new(charpos.max(0) as usize),
            EmacsBytePos::new(self.text_start_byte + byte_idx),
            self.text.get(byte_idx..).unwrap_or(&[]),
            self.active_face_state,
            self.current_x,
            self.content_x,
            self.params,
            self.glyph_y_offset,
            self.default_row_height,
            self.start_position,
            checkpoints.display_next(),
            checkpoints.display_skip_to(accessible_end),
        )
        .resolve_and_append_to_text_row(
            buffer,
            state,
            face_ids,
            append_surface,
            row_geometry,
        )
    }
}

impl<'a, B: LayoutBufferView> BufferDisplayPropertyCheckpointRenderRequest<'a, B> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        face_resolution_context: BufferCurrentFaceResolutionContext<'a, B>,
        buffer_id: BufferId,
        text_start_byte: usize,
        text: &'a [u8],
        current_x: f32,
        content_x: f32,
        params: &'a WindowParams,
        glyph_y_offset: f32,
        default_row_height: f32,
        start_position: DisplayRowPosition,
        charpos: i64,
        byte_idx: usize,
        accessible_end: i64,
    ) -> Self {
        Self {
            face_resolution_context,
            buffer_id,
            text_start_byte,
            text,
            current_x,
            content_x,
            params,
            glyph_y_offset,
            default_row_height,
            start_position,
            charpos,
            byte_idx,
            accessible_end,
        }
    }

    pub(crate) fn render_and_apply(
        self,
        state: BufferDisplayPropertyCheckpointRenderState<'_, '_>,
    ) -> BufferDisplayPropertyTextWalkOutcome {
        let BufferDisplayPropertyCheckpointRenderState {
            mut source_render,
            face_ids,
            append_surface,
            row_geometry,
            checkpoints,
            face_scan,
            active_face_state,
            row_extend,
            box_face,
            byte_idx,
            charpos,
            x,
            col,
            cursor_info,
            raise_span,
            height_span,
            point_charpos,
        } = state;

        BufferDisplayPropertyTextModifierAction::clear_expired_height_span(
            height_span,
            face_scan,
            self.charpos,
            self.params.window_start,
        );
        self.face_resolution_context
            .resolve_at_checkpoint_with_source_state(
                &mut source_render,
                face_scan,
                height_span,
                face_ids,
                active_face_state,
                row_geometry,
                row_extend,
                box_face,
                *x,
                self.charpos,
            );

        let action = BufferDisplayPropertyTextRenderContext::new(
            self.buffer_id,
            self.text_start_byte,
            self.text,
            active_face_state,
            self.current_x,
            self.content_x,
            self.params,
            self.glyph_y_offset,
            self.default_row_height,
            self.start_position,
        )
        .resolve_and_append_at_checkpoint(
            self.face_resolution_context.buffer,
            &mut source_render,
            face_ids,
            append_surface,
            row_geometry,
            checkpoints,
            self.charpos,
            self.byte_idx,
            self.accessible_end,
        );
        let outcome = action.apply_to_buffer_walk_state(
            self.text,
            byte_idx,
            charpos,
            x,
            col,
            cursor_info,
            active_face_state,
            row_geometry,
            point_charpos,
            raise_span,
            height_span,
            face_scan,
        );
        if outcome.should_resolve_face() {
            self.face_resolution_context
                .resolve_at_checkpoint_with_source_state(
                    &mut source_render,
                    face_scan,
                    height_span,
                    face_ids,
                    active_face_state,
                    row_geometry,
                    row_extend,
                    box_face,
                    *x,
                    *charpos,
                );
        }
        outcome
    }
}

impl<'a> BufferDisplayPropertyTextAppendRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_text_property(
        value: Value,
        buffer_id: BufferId,
        anchor_charpos: CharPos0,
        anchor_bytepos: EmacsBytePos,
        source_text: &'a [u8],
        active_face_state: &'a DisplayRowActiveFaceState,
        current_x: f32,
        content_x: f32,
        params: &'a WindowParams,
        glyph_y_offset: f32,
        default_row_height: f32,
        start_position: DisplayRowPosition,
        next_change: i64,
        skip_to: i64,
    ) -> Self {
        Self {
            value,
            buffer_id,
            anchor_charpos,
            anchor_bytepos,
            source_text,
            active_face_state,
            current_x,
            content_x,
            params,
            glyph_y_offset,
            default_row_height,
            start_position,
            next_change,
            skip_to,
        }
    }

    pub(crate) fn resolve_and_append_to_text_row<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        append_surface: &DisplayRowAppendSurface,
        row_geometry: &mut DisplayRowGeometryState,
    ) -> BufferDisplayPropertyTextAppendAction {
        let display_property = classify_display_property(self.value);
        let replacement_item = state.with_font_metrics_and_display_host(|font_metrics, host| {
            DisplayPropertyReplacementAppendItem::resolve(
                &display_property,
                self.value,
                self.anchor_charpos,
                self.source_text,
                self.active_face_state,
                font_metrics,
                self.current_x,
                self.content_x,
                self.params,
                host,
            )
        });
        if let Some(item) = replacement_item {
            let replacement = DisplayPropertyReplacementAppendRequest::new(
                BufferDisplayReplacementSource::new(
                    self.buffer_id,
                    self.anchor_charpos,
                    self.anchor_bytepos,
                ),
                item,
                self.glyph_y_offset,
                self.default_row_height,
                self.start_position,
            )
            .append_to_text_row(
                buffer,
                state,
                face_ids,
                append_surface,
                row_geometry,
                self.active_face_state,
            );
            return BufferDisplayPropertyTextAppendAction::Replacement(
                BufferDisplayPropertyTextReplacementOutcome {
                    replacement,
                    skip_to: self.skip_to,
                },
            );
        }

        BufferDisplayPropertyTextModifierAction::for_display_property(
            &display_property,
            self.default_row_height,
            self.next_change,
        )
        .map(BufferDisplayPropertyTextAppendAction::Modifiers)
        .unwrap_or(BufferDisplayPropertyTextAppendAction::None)
    }
}

impl BufferDisplayPropertyTextReplacementOutcome {
    pub(crate) fn point_in_replacement(self, point_charpos: i64, start_charpos: i64) -> bool {
        point_charpos >= start_charpos && point_charpos < self.skip_to
    }

    pub(crate) fn start_position(self) -> DisplayRowPosition {
        self.replacement.start_position()
    }

    pub(crate) fn end_position(self) -> DisplayRowPosition {
        self.replacement.end_position()
    }

    pub(crate) fn skip_covered_buffer_text(
        self,
        text: &[u8],
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) {
        skip_text_to_charpos(text, byte_idx, charpos, self.skip_to);
    }

    pub(crate) fn capture_cursor_info_if_point(
        self,
        cursor_info: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        start_charpos: i64,
        byte_idx: usize,
    ) {
        if cursor_info.is_missing() && self.point_in_replacement(point_charpos, start_charpos) {
            let start_position = self.start_position();
            capture_cursor_info(
                cursor_info,
                self.cursor_info(
                    active_face_state,
                    row_geometry.text_position(start_position.x_px, byte_idx, start_position.col),
                ),
            );
        }
    }

    pub(crate) fn apply_to_walk_state(
        self,
        text: &[u8],
        byte_idx: &mut usize,
        charpos: &mut i64,
        x: &mut f32,
        col: &mut usize,
    ) {
        let position = self.end_position();
        *x = position.x_px;
        *col = position.col;
        self.skip_covered_buffer_text(text, byte_idx, charpos);
    }

    #[cfg(test)]
    pub(crate) fn skip_to(self) -> i64 {
        self.skip_to
    }

    pub(crate) fn cursor_info(
        self,
        active_face_state: &DisplayRowActiveFaceState,
        position: DisplayRowTextPosition,
    ) -> CapturedCursorInfo {
        self.replacement.cursor_info(active_face_state, position)
    }
}

impl BufferDisplayPropertyTextModifierAction {
    #[cfg(test)]
    pub(crate) fn new_for_test(
        raise_offset_px: Option<f32>,
        height_factor: Option<f32>,
        next_change: i64,
    ) -> Self {
        Self {
            raise_offset_px,
            height_factor,
            next_change,
        }
    }

    pub(crate) fn clear_expired_raise_span(
        raise_span: &mut ActiveDisplayPropertySpan<f32>,
        charpos: i64,
        inactive_end_charpos: i64,
    ) {
        let _ = raise_span.clear_if_expired(charpos, inactive_end_charpos);
    }

    pub(crate) fn clear_expired_height_span(
        height_span: &mut ActiveDisplayPropertySpan<f32>,
        face_scan: &mut FaceScanCheckpoint,
        charpos: i64,
        inactive_end_charpos: i64,
    ) -> BufferDisplayPropertyTextModifierStateOutcome {
        let height_face_changed = height_span.clear_if_expired(charpos, inactive_end_charpos);
        if height_face_changed {
            face_scan.invalidate();
        }
        BufferDisplayPropertyTextModifierStateOutcome::new(height_face_changed)
    }

    fn for_display_property(
        display_property: &DisplayPropertyClassification,
        row_height: f32,
        next_change: i64,
    ) -> Option<Self> {
        let raise_offset_px = display_property
            .modifiers
            .raise
            .map(|factor| -(factor * row_height));
        let height_factor = display_property
            .modifiers
            .height
            .filter(|factor| factor.is_finite() && *factor > 0.0);
        (raise_offset_px.is_some() || height_factor.is_some()).then_some(Self {
            raise_offset_px,
            height_factor,
            next_change,
        })
    }

    pub(crate) fn apply_to_walk_state(
        self,
        raise_span: &mut ActiveDisplayPropertySpan<f32>,
        height_span: &mut ActiveDisplayPropertySpan<f32>,
        face_scan: &mut FaceScanCheckpoint,
    ) -> BufferDisplayPropertyTextModifierStateOutcome {
        if let Some(raise_offset_px) = self.raise_offset_px {
            raise_span.set(raise_offset_px, self.next_change);
        }
        let height_face_changed = if let Some(factor) = self.height_factor {
            height_span.set(factor, self.next_change);
            face_scan.invalidate();
            true
        } else {
            false
        };
        BufferDisplayPropertyTextModifierStateOutcome::new(height_face_changed)
    }

    #[cfg(test)]
    pub(crate) fn raise_offset_px(self) -> Option<f32> {
        self.raise_offset_px
    }

    #[cfg(test)]
    pub(crate) fn height_factor(self) -> Option<f32> {
        self.height_factor
    }

    #[cfg(test)]
    pub(crate) fn next_change(self) -> i64 {
        self.next_change
    }
}

#[cfg(test)]
pub(crate) struct DisplayPropertyReplacementAppendResolveRequest<'a> {
    display_property: &'a DisplayPropertyClassification,
    value: Value,
    replacement_source: BufferDisplayReplacementSource,
    anchor_charpos: CharPos0,
    source_text: &'a [u8],
    active_face_state: &'a DisplayRowActiveFaceState,
    current_x: f32,
    content_x: f32,
    params: &'a WindowParams,
    glyph_y_offset: f32,
    default_row_height: f32,
    start_position: DisplayRowPosition,
}

#[cfg(test)]
impl<'a> DisplayPropertyReplacementAppendResolveRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        display_property: &'a DisplayPropertyClassification,
        value: Value,
        replacement_source: BufferDisplayReplacementSource,
        anchor_charpos: CharPos0,
        source_text: &'a [u8],
        active_face_state: &'a DisplayRowActiveFaceState,
        current_x: f32,
        content_x: f32,
        params: &'a WindowParams,
        glyph_y_offset: f32,
        default_row_height: f32,
        start_position: DisplayRowPosition,
    ) -> Self {
        Self {
            display_property,
            value,
            replacement_source,
            anchor_charpos,
            source_text,
            active_face_state,
            current_x,
            content_x,
            params,
            glyph_y_offset,
            default_row_height,
            start_position,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_text_property(
        display_property: &'a DisplayPropertyClassification,
        value: Value,
        buffer_id: BufferId,
        anchor_charpos: CharPos0,
        anchor_bytepos: EmacsBytePos,
        source_text: &'a [u8],
        active_face_state: &'a DisplayRowActiveFaceState,
        current_x: f32,
        content_x: f32,
        params: &'a WindowParams,
        glyph_y_offset: f32,
        default_row_height: f32,
        start_position: DisplayRowPosition,
    ) -> Self {
        Self::new(
            display_property,
            value,
            BufferDisplayReplacementSource::new(buffer_id, anchor_charpos, anchor_bytepos),
            anchor_charpos,
            source_text,
            active_face_state,
            current_x,
            content_x,
            params,
            glyph_y_offset,
            default_row_height,
            start_position,
        )
    }

    pub(crate) fn resolve(
        self,
        font_metrics: &mut Option<FontMetricsService>,
        display_host: Option<&dyn DisplayHost>,
    ) -> Option<DisplayPropertyReplacementAppendRequest> {
        let item = DisplayPropertyReplacementAppendItem::resolve(
            self.display_property,
            self.value,
            self.anchor_charpos,
            self.source_text,
            self.active_face_state,
            font_metrics,
            self.current_x,
            self.content_x,
            self.params,
            display_host,
        )?;
        Some(DisplayPropertyReplacementAppendRequest::new(
            self.replacement_source,
            item,
            self.glyph_y_offset,
            self.default_row_height,
            self.start_position,
        ))
    }

    pub(crate) fn resolve_and_append_to_text_row<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        append_surface: &DisplayRowAppendSurface,
        row_geometry: &mut DisplayRowGeometryState,
    ) -> Option<DisplayPropertyReplacementAppendOutcome> {
        let active_face_state = self.active_face_state;
        let request = state.with_font_metrics_and_display_host(|font_metrics, host| {
            self.resolve(font_metrics, host)
        })?;
        Some(request.append_to_text_row(
            buffer,
            state,
            face_ids,
            append_surface,
            row_geometry,
            active_face_state,
        ))
    }
}

#[derive(Clone)]
pub(crate) struct DisplayPropertyReplacementAppendRequest {
    replacement_source: BufferDisplayReplacementSource,
    item: DisplayPropertyReplacementAppendItem,
    glyph_y_offset: f32,
    default_row_height: f32,
    start_position: DisplayRowPosition,
}

impl DisplayPropertyReplacementAppendRequest {
    pub(crate) fn new(
        replacement_source: BufferDisplayReplacementSource,
        item: DisplayPropertyReplacementAppendItem,
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
        let item = self
            .item
            .into_plan_item(buffer, state, active_face_state, face_ids);
        DisplayPropertyReplacementAppendPlan {
            replacement_source: self.replacement_source,
            item,
            glyph_y_offset: self.glyph_y_offset,
            default_row_height: self.default_row_height,
            start_position: self.start_position,
        }
    }

    #[cfg(test)]
    pub(crate) fn into_item(self) -> DisplayPropertyReplacementAppendItem {
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
    Stretch(DisplayReplacementStretchAppendItem),
    Media(DisplayReplacementMediaAppendResolution),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum DisplayPropertyReplacementCursorPolicy {
    TextSlot {
        width_px: f32,
        stretch_like: bool,
    },
    DisplayBox {
        width_px: f32,
        cursor_face_height_px: f32,
        cursor_face_ascent_px: f32,
    },
    FaceChar,
}

impl DisplayPropertyReplacementAppendItem {
    #[allow(clippy::too_many_arguments)]
    fn into_plan_item<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: &mut TextRowSourceRenderState<'_>,
        active_face_state: &DisplayRowActiveFaceState,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> DisplayPropertyReplacementAppendPlanItem {
        match self {
            Self::String(item) => {
                let replacement_base_face = (!item.is_empty()).then(|| {
                    state.display_string_base_face_for_active_row(
                        buffer,
                        item.origin(),
                        item.base_face_policy(),
                        active_face_state,
                        face_ids,
                    )
                });
                DisplayPropertyReplacementAppendPlanItem::String(
                    DisplayReplacementStringAppendRequest::new(item, replacement_base_face),
                )
            }
            Self::Stretch(item) => DisplayPropertyReplacementAppendPlanItem::Stretch(item),
            Self::Media(item) => DisplayPropertyReplacementAppendPlanItem::Media(item),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve(
        display_property: &DisplayPropertyClassification,
        value: Value,
        anchor_charpos: CharPos0,
        source_text: &[u8],
        active_face_state: &DisplayRowActiveFaceState,
        font_metrics: &mut Option<FontMetricsService>,
        current_x: f32,
        content_x: f32,
        params: &WindowParams,
        display_host: Option<&dyn DisplayHost>,
    ) -> Option<Self> {
        let face_metrics = active_face_state.metrics();
        if display_property.is_string_replacement() {
            DisplayReplacementStringAppendItem::display_property_string(
                value,
                anchor_charpos,
                DisplayPropertySource::TextProperty,
                1,
                active_face_state,
                font_metrics,
                face_metrics.char_width,
            )
            .map(Self::String)
        } else if display_property.stretch_replacement().is_some() {
            Some(Self::Stretch(
                DisplayReplacementStretchAppendItem::from_display_space_property(
                    &value,
                    source_text,
                    active_face_state,
                    font_metrics,
                    current_x,
                    content_x,
                    face_metrics.char_width,
                    face_metrics.row_height,
                    face_metrics.ascent,
                    params,
                ),
            ))
        } else {
            let media_replacement = display_property.media_replacement()?;
            DisplayReplacementMediaAppendItem::resolve_display_property(
                value,
                media_replacement,
                display_host,
                active_face_state,
                face_metrics.char_width,
                face_metrics.row_height,
            )
            .map(Self::Media)
        }
    }

    pub(crate) fn cursor_policy(&self) -> DisplayPropertyReplacementCursorPolicy {
        match self {
            Self::String(item) => DisplayPropertyReplacementCursorPolicy::TextSlot {
                width_px: item.cursor_slot_width_px(),
                stretch_like: false,
            },
            Self::Stretch(item) => DisplayPropertyReplacementCursorPolicy::TextSlot {
                width_px: item.cursor_slot_width_px(),
                stretch_like: true,
            },
            Self::Media(DisplayReplacementMediaAppendResolution::Media(item)) => {
                DisplayPropertyReplacementCursorPolicy::DisplayBox {
                    width_px: item.width_px(),
                    cursor_face_height_px: item.cursor_face_height_px(),
                    cursor_face_ascent_px: item.cursor_face_ascent_px(),
                }
            }
            Self::Media(DisplayReplacementMediaAppendResolution::Placeholder(_)) => {
                DisplayPropertyReplacementCursorPolicy::FaceChar
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

impl DisplayReplacementStretchAppendItem {
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

impl DisplayReplacementMediaAppendResolution {
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

impl DisplayReplacementMediaAppendItem {
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

impl DisplayReplacementSourceMappedTextAppendItem {
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

impl DisplayReplacementMediaAppendItem {
    pub(crate) fn new(
        media: DisplayMediaReplacement,
        active_face_state: &DisplayRowActiveFaceState,
        uses_xwidget_cursor_extents: bool,
    ) -> Self {
        let metrics = active_face_state.metrics();
        let (cursor_face_height, cursor_face_ascent) = if uses_xwidget_cursor_extents {
            (
                media.height.max(metrics.row_height),
                media.height.max(metrics.ascent),
            )
        } else {
            (media.height, media.height)
        };
        Self {
            media,
            cursor_face_height,
            cursor_face_ascent,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_display_property(
        display_prop: Value,
        replacement: &DisplayMediaReplacementProperty,
        display_host: Option<&dyn DisplayHost>,
        active_face_state: &DisplayRowActiveFaceState,
        fallback_char_width: f32,
        fallback_row_height: f32,
    ) -> Option<DisplayReplacementMediaAppendResolution> {
        match resolve_display_replacement(
            display_prop,
            replacement,
            display_host,
            active_face_state.resolved_face(),
            fallback_char_width,
            fallback_row_height,
        )? {
            ResolvedDisplayReplacement::Media(media) => {
                Some(DisplayReplacementMediaAppendResolution::Media(Self::new(
                    media,
                    active_face_state,
                    replacement.uses_xwidget_cursor_extents(),
                )))
            }
            ResolvedDisplayReplacement::Placeholder(placeholder) => {
                Some(DisplayReplacementMediaAppendResolution::Placeholder(
                    DisplayReplacementSourceMappedTextAppendItem::new(placeholder),
                ))
            }
        }
    }

    pub(crate) fn media(self) -> DisplayMediaReplacement {
        self.media
    }

    pub(crate) fn width_px(self) -> f32 {
        self.media.width
    }

    pub(crate) fn display_height_px(self) -> f32 {
        self.media.height
    }

    pub(crate) fn display_ascent_px(self) -> f32 {
        self.media.height
    }

    pub(crate) fn cursor_face_height_px(self) -> f32 {
        self.cursor_face_height
    }

    pub(crate) fn cursor_face_ascent_px(self) -> f32 {
        self.cursor_face_ascent
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
            DisplayReplacementAppendItem::media(self),
            self.display_height_px(),
            self.display_ascent_px(),
            position,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DisplayReplacementSourceMappedTextAppendItem {
    text: Box<str>,
}

impl DisplayReplacementSourceMappedTextAppendItem {
    pub(crate) fn new(text: impl Into<Box<str>>) -> Self {
        Self { text: text.into() }
    }

    fn text(self) -> Box<str> {
        self.text
    }

    fn append_request(self, position: DisplayRowPosition) -> DisplayReplacementItemAppendRequest {
        DisplayReplacementItemAppendRequest::active_face(
            DisplayReplacementAppendItem::source_mapped_text(self),
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
        DisplayRowSingleItemAppendOperation::new(
            item,
            self.base_face,
            self.face_id,
            self.frame.clone(),
            position,
            DisplayRowAppendKind::DisplayReplacement,
        )
        .render_to_text_row_and_emit(state, &mut render_policy)
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextWindowAppendSurfaceRequest<'a> {
    content_x: f32,
    text_width: f32,
    line_number_width: f32,
    reserve_right_border_col: bool,
    reserve_right_special_col: bool,
    char_width: f32,
    tab_width: i32,
    tab_stop_list: &'a [i32],
}

impl<'a> TextWindowAppendSurfaceRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        content_x: f32,
        text_width: f32,
        line_number_width: f32,
        reserve_right_border_col: bool,
        reserve_right_special_col: bool,
        char_width: f32,
        tab_width: i32,
        tab_stop_list: &'a [i32],
    ) -> Self {
        Self {
            content_x,
            text_width,
            line_number_width,
            reserve_right_border_col,
            reserve_right_special_col,
            char_width,
            tab_width,
            tab_stop_list,
        }
    }

    fn reserved_width(self) -> f32 {
        let right_border = if self.reserve_right_border_col {
            self.char_width
        } else {
            0.0
        };
        let right_special = if self.reserve_right_special_col {
            self.char_width
        } else {
            0.0
        };
        right_border + right_special
    }

    fn append_width(self) -> f32 {
        (self.text_width - self.line_number_width - self.reserved_width()).max(self.char_width)
    }

    pub(crate) fn into_surface(self) -> DisplayRowAppendSurface {
        DisplayRowAppendSurface::new(
            DisplayRowAppendArea::new(
                self.content_x,
                self.append_width(),
                self.text_width,
                self.line_number_width,
            ),
            DisplayTabPolicy::from_tab_width_and_stops(
                self.content_x,
                self.tab_width,
                self.tab_stop_list,
            ),
        )
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
    height: f32,
    ascent: f32,
    char_width: f32,
    space_width: f32,
    default_row_height: f32,
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
enum DisplayRowAppendKind {
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

    fn max_x(self, frame: &DisplayRowAppendFrame) -> f32 {
        match self {
            Self::Tab => f32::INFINITY,
            Self::ControlChar => frame.content_x + (frame.text_width - frame.line_number_width),
            Self::SourceText
            | Self::SourceMappedText
            | Self::Glyphless
            | Self::DisplayReplacement
            | Self::DisplayReplacementString => frame.content_x + frame.geometry.width,
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

fn append_synthetic_text_to_display_row(
    state: &mut TextRowSourceRenderState<'_>,
    base_face: &ResolvedFace,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    source: SyntheticTextSource,
    face_id: u32,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let item = synthetic_display_text_item(source, face_id);
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    DisplayRowSingleItemAppendOperation::new(
        item,
        base_face,
        face_id,
        frame,
        position,
        DisplayRowAppendKind::SourceText,
    )
    .render_to_text_row_and_emit(state, &mut render_policy)
}

#[cfg(test)]
#[path = "display_row_append_test.rs"]
mod tests;
