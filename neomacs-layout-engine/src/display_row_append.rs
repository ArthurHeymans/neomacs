use crate::display_cursor::{
    CapturedCursorInfo, CapturedCursorPlacement, CapturedCursorSlotWidth,
    CapturedCursorVisualState, CursorCaptureState, display_property_replacement_cursor_info,
    update_cursor_info_for_main_char,
};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_face_policy::BaseFacePolicy;
use crate::display_item::{
    DisplayGlyphless, DisplayItem, DisplayItemKind, DisplayMediaReplacement,
    DisplaySourceMappedText, DisplayTextRun, GlyphlessJoinerPolicy, GlyphlessMethod, RenderFaceRef,
    SourceSpan, glyphless_method_for_char,
};
use crate::display_origin::{DisplayOrigin, DisplayPropertySource, OverlayStringKind};
use crate::display_property::{DisplayMediaReplacementProperty, DisplayPropertyClassification};
use crate::display_row::DisplayRowRenderStop;
#[cfg(test)]
use crate::display_row::RenderedDisplayRow;
#[cfg(test)]
use crate::display_row::append_rendered_display_row_fragment_to_current_row;
use crate::display_row::{
    CurrentTextRowRenderOutcome, DisplayRowActiveFaceState, DisplayRowComplexTextRunAdvancePolicy,
    DisplayRowGeometry, DisplayRowMeasuredFaceMetrics, DisplayRowRenderBounds,
    DisplayRowRenderClipBehavior, DisplayRowRenderPolicy, DisplayRowSourceAppendRequest,
    DisplayRowSourceAppendRequestPolicy, DisplayRowSourceState, DisplaySourceAppendMeasurement,
    DisplaySourceAppendRenderPolicy, NaturalDisplayRowAppendRenderPolicy,
    append_single_display_item_fragment_to_text_row_and_emit,
};
use crate::display_row_builder::{
    DisplayRowAppendProgress, DisplayRowAppendStatus, DisplayRowGlyphSlot,
    DisplayRowItemMeasurement, DisplayRowPosition, DisplayTabPolicy,
};
use crate::display_row_geometry::{
    DisplayRowBoundaryTarget, DisplayRowFlagKind, DisplayRowFlags, DisplayRowGeometryDefaults,
    DisplayRowGeometryState, DisplayRowHitRange, DisplayRowLimit, DisplayRowTextPosition,
    DisplayRowYPositions, DisplayRowYRecording,
};
use crate::display_row_walk_state::{
    BufferTextRowOverflowDecision, HitRowRangeTracker, TextRowTransitionPrefixAction,
    TrailingWhitespaceRenderState, WordWrapRenderState,
};
use crate::display_source::{
    BufferDisplayReplacementSource, BufferDisplayReplacementStringSource, BufferTextItemSource,
    DisplayItemSource, DisplayReplacementBox, LispStringSourceCursor,
};
#[cfg(test)]
use crate::display_source_resolver::PendingDisplaySourceFace;
use crate::display_source_resolver::{
    DisplayDefaultFaceInstallPolicy, DisplayStringBaseFace, ResolvedDisplayReplacement,
    display_string_base_face_for_active_row, resolve_and_install_display_string_base_face,
    resolve_display_replacement,
};
use crate::display_space::{DisplaySpaceKey, display_space_positive_number};
use crate::display_text_run_measurement::ComplexTextRunAdvanceResolver;
use crate::font_metrics::FontMetricsService;
use crate::hit_test::HitRow;
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::neovm_bridge::{FaceResolver, LayoutBufferView, OverlayDisplayString, ResolvedFace};
use crate::types::WindowParams;
use crate::unicode::decode_utf8;
#[cfg(test)]
use crate::window_output::TextRowOutput;
use crate::window_output::{
    TextMatrixRowGeometryTransition, TextMatrixRowTransition, WindowOutputEmitter,
    emit_text_matrix_row_transition, emit_text_matrix_row_transition_with_limit,
    finish_and_end_text_matrix_row_output,
};
use neovm_core::buffer::{BufferId, CharLen, CharPos0, EmacsBytePos, EmacsByteRange};
use neovm_core::emacs_core::eval::DisplayHost;
use neovm_core::emacs_core::value::get_string_text_properties_table_for_value;
use neovm_core::emacs_core::{Context, Value};

const LISP_STRING_SOURCE_OVERLAY_STRING: u64 = 1;

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

    #[allow(clippy::too_many_arguments)]
    fn render_active_face_source_request_to_text_row_and_emit(
        self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
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
            .render_to_text_row_and_emit(
                builder,
                output_emitter,
                evaluator,
                font_metrics,
                face_resolver,
                face_ids,
                frame,
                position,
            )
            .map(|outcome| outcome.end_position())
            .unwrap_or(position)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_prefix_source_to_text_row_and_emit(
        self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        base_face: &DisplayStringBaseFace,
        prefix_source: DisplayRowPrefixSource,
        position: DisplayRowPosition,
        source_id: u64,
    ) -> DisplayRowPosition {
        self.render_active_face_source_request_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            face_ids,
            base_face.face_id(),
            base_face.face(),
            prefix_source.append_request(position, source_id),
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn render_lisp_string_source_append_to_text_row_and_emit(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    source: &mut LispStringSourceCursor,
    source_state: &mut DisplayRowSourceState,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    base_face_id: u32,
    face_ids: &mut FrameFaceIdAllocator,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<CurrentTextRowRenderOutcome> {
    let request = frame.source_append_request(
        position,
        base_face_id,
        base_face,
        DisplayRowAppendKind::SourceText,
    );
    request.render_natural_display_source_into_current_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        source,
        source_state,
        face_resolver,
        face_ids,
    )
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_to_text_row_and_emit(
        &mut self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
    ) -> Option<CurrentTextRowRenderOutcome> {
        render_lisp_string_source_append_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            self.source,
            self.source_state,
            face_resolver,
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
    source_id: u64,
    value: Value,
}

impl LispStringSourceAppendRequest {
    fn new(position: DisplayRowPosition, source_id: u64, value: Value) -> Self {
        Self {
            position,
            source_id,
            value,
        }
    }

    fn into_parts(self) -> LispStringSourceAppendRequestParts {
        LispStringSourceAppendRequestParts {
            position: self.position,
            source_id: self.source_id,
            value: self.value,
        }
    }

    fn position(self) -> DisplayRowPosition {
        self.position
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LispStringSourceAppendRequestParts {
    position: DisplayRowPosition,
    source_id: u64,
    value: Value,
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
        let parts = request.into_parts();
        let source = LispStringSourceCursor::new(
            parts.source_id,
            parts.value,
            RenderFaceRef::FaceId(base_face_id),
        )?;
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

    #[allow(clippy::too_many_arguments)]
    fn render_to_text_row_and_emit(
        &mut self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
    ) -> Option<CurrentTextRowRenderOutcome> {
        self.append_context().render_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            face_ids,
            frame,
            position,
        )
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

    fn append_request(
        self,
        position: DisplayRowPosition,
        source_id: u64,
    ) -> LispStringSourceAppendRequest {
        LispStringSourceAppendRequest::new(position, source_id, self.value)
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_overlay_string_batch<B: LayoutBufferView>(
    evaluator: &mut Context,
    output_emitter: &mut WindowOutputEmitter,
    buffer: &B,
    source_batch: OverlayStringRenderBatchSource<'_>,
    font_metrics: &mut Option<FontMetricsService>,
    face_resolver: &FaceResolver,
    x: &mut f32,
    col: &mut usize,
    geometry: &mut DisplayRowGeometryState,
    cursor_info: &mut CursorCaptureState,
    hit_rows: &mut Vec<HitRow>,
    hit_row_range: &mut HitRowRangeTracker,
    row_y_positions: &mut DisplayRowYPositions,
    row_context: OverlayStringRenderRowContext<'_>,
    face_ids: &mut FrameFaceIdAllocator,
    builder: &mut GlyphMatrixBuilder,
) {
    if source_batch.is_empty() {
        return;
    }
    for overlay_string in source_batch.overlay_strings() {
        render_overlay_string(
            evaluator,
            output_emitter,
            buffer,
            source_batch.source_for(*overlay_string),
            font_metrics,
            face_resolver,
            x,
            col,
            geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            row_context,
            face_ids,
            builder,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn render_overlay_string<B: LayoutBufferView>(
    evaluator: &mut Context,
    output_emitter: &mut WindowOutputEmitter,
    buffer: &B,
    source_request: OverlayStringRenderSource,
    font_metrics: &mut Option<FontMetricsService>,
    face_resolver: &FaceResolver,
    x: &mut f32,
    col: &mut usize,
    geometry: &mut DisplayRowGeometryState,
    cursor_info: &mut CursorCaptureState,
    hit_rows: &mut Vec<HitRow>,
    hit_row_range: &mut HitRowRangeTracker,
    row_y_positions: &mut DisplayRowYPositions,
    row_context: OverlayStringRenderRowContext<'_>,
    face_ids: &mut FrameFaceIdAllocator,
    builder: &mut GlyphMatrixBuilder,
) {
    let anchor_charpos = source_request.anchor_i64();
    let text_value = source_request.value();
    if text_value.as_lisp_string().is_none() {
        return;
    }
    let text_props = get_string_text_properties_table_for_value(text_value);
    let base_face = resolve_and_install_display_string_base_face(
        buffer,
        face_resolver,
        source_request.origin(),
        source_request.base_face_policy(),
        None,
        DisplayDefaultFaceInstallPolicy::InstallDefaultFace,
        face_ids,
        builder,
    );
    let content_x = row_context.content_x();
    let max_x = row_context.right_edge();
    let row_geometry_defaults = row_context.geometry_defaults();
    let row_limit = row_context.row_limit();

    macro_rules! finish_overlay_string_row {
        () => {{
            let geometry_transition = DisplayRowLineBreakTransitionRequest::new(
                hit_row_range.range_to(anchor_charpos),
                row_geometry_defaults,
                row_context.row_base,
                0,
                content_x,
                0.0,
                DisplayRowYRecording::None,
                row_context.max_rows,
            )
            .finish_geometry(geometry, hit_rows);
            hit_row_range.advance_to(anchor_charpos);
            if !geometry.is_within_row_limit(row_limit) {
                finish_and_end_text_matrix_row_output(
                    builder,
                    output_emitter,
                    evaluator,
                    geometry_transition.finished_row,
                );
                false
            } else {
                geometry.record_current_row_y(row_y_positions);
                *x = content_x;
                *col = 0;
                emit_text_matrix_row_transition(
                    builder,
                    output_emitter,
                    evaluator,
                    geometry_transition,
                );
                true
            }
        }};
    }

    let append_request = LispStringSourceAppendRequest::new(
        DisplayRowPosition {
            x_px: *x,
            col: *col,
        },
        LISP_STRING_SOURCE_OVERLAY_STRING,
        text_value,
    );
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

    while geometry.is_within_row_limit(row_limit) {
        if *x >= max_x {
            break;
        }

        let Some(outcome) = source_context.render_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            face_ids,
            geometry,
            DisplayRowPosition {
                x_px: *x,
                col: *col,
            },
        ) else {
            break;
        };
        let stop = outcome.stop();
        outcome.include_vertical_metrics(geometry);
        let overlay_cursor_visual_state = row_context.cursor_visual_state(base_face.face());
        for slot in outcome.source_slots() {
            capture_overlay_string_cursor_at_slot(
                text_props.as_ref(),
                slot,
                cursor_info,
                geometry.y(),
                geometry.row(),
                overlay_cursor_visual_state,
            );
        }
        let end = outcome.end_position();
        *x = end.x_px;
        *col = end.col;

        if stop == DisplayRowRenderStop::RowBreak {
            if !finish_overlay_string_row!() {
                break;
            }
            continue;
        }
        match stop {
            DisplayRowRenderStop::SourceExhausted => break,
            DisplayRowRenderStop::Clipped => {
                if source_context.discard_pending_until_row_break() {
                    if !finish_overlay_string_row!() {
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
        } if source_id.get() == LISP_STRING_SOURCE_OVERLAY_STRING => Some(*char_index),
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
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_to_text_row_and_emit(
        &mut self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
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
        self.source_context.render_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            face_ids,
            frame,
            position,
        )
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_to_text_row_and_emit(
        &mut self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        geometry: &DisplayRowGeometryState,
        position: DisplayRowPosition,
    ) -> Option<CurrentTextRowRenderOutcome> {
        self.append_context().render_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            face_ids,
            geometry,
            position,
        )
    }

    pub(crate) fn discard_pending_until_row_break(&mut self) -> bool {
        self.source_session.discard_pending_until_row_break()
    }
}

pub(crate) fn synthetic_display_text_item(
    source_id: u64,
    text: impl Into<Box<str>>,
    face_id: u32,
) -> DisplayItem {
    let text = text.into();
    let char_len = text.chars().count();
    DisplayItem::new(
        SourceSpan::synthetic(source_id, 0, char_len),
        RenderFaceRef::FaceId(face_id),
        DisplayItemKind::TextRun(DisplayTextRun::new(text)),
    )
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_to_text_row_and_emit(
        &self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        position: DisplayRowPosition,
        source_id: u64,
        text: impl Into<Box<str>>,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        append_synthetic_text_to_display_row(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            self.base_face,
            self.frame.clone(),
            position,
            source_id,
            text,
            self.face_id,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SyntheticTextAppendRequest {
    position: DisplayRowPosition,
    source_id: u64,
    text: Box<str>,
}

impl SyntheticTextAppendRequest {
    pub(crate) fn new(
        position: DisplayRowPosition,
        source_id: u64,
        text: impl Into<Box<str>>,
    ) -> Self {
        Self {
            position,
            source_id,
            text: text.into(),
        }
    }

    pub(crate) fn position(&self) -> DisplayRowPosition {
        self.position
    }

    fn into_source_text(self) -> (u64, Box<str>) {
        (self.source_id, self.text)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SyntheticTextMetricsAppendRequest<'face> {
    position: DisplayRowPosition,
    source_id: u64,
    text: Box<str>,
    face_id: u32,
    base_face: &'face ResolvedFace,
    height_px: f32,
    ascent_px: f32,
    char_width_px: f32,
}

impl<'face> SyntheticTextMetricsAppendRequest<'face> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        position: DisplayRowPosition,
        source_id: u64,
        text: impl Into<Box<str>>,
        face_id: u32,
        base_face: &'face ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    ) -> Self {
        Self {
            position,
            source_id,
            text: text.into(),
            face_id,
            base_face,
            height_px,
            ascent_px,
            char_width_px,
        }
    }

    fn into_parts(self) -> SyntheticTextMetricsAppendRequestParts<'face> {
        SyntheticTextMetricsAppendRequestParts {
            position: self.position,
            source_id: self.source_id,
            text: self.text,
            face_id: self.face_id,
            base_face: self.base_face,
            height_px: self.height_px,
            ascent_px: self.ascent_px,
            char_width_px: self.char_width_px,
        }
    }
}

struct SyntheticTextMetricsAppendRequestParts<'face> {
    position: DisplayRowPosition,
    source_id: u64,
    text: Box<str>,
    face_id: u32,
    base_face: &'face ResolvedFace,
    height_px: f32,
    ascent_px: f32,
    char_width_px: f32,
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

    fn text_row(
        self,
        face_id: u32,
        base_face: &'a ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    ) -> SyntheticTextAppendContext<'a> {
        SyntheticTextAppendContext::new(
            face_id,
            base_face,
            self.active_face_context
                .text_row_frame(height_px, ascent_px, char_width_px),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_active_face_to_text_row_and_emit(
        self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        position: DisplayRowPosition,
        source_id: u64,
        text: impl Into<Box<str>>,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let active_face = self.active_face_context.active_face;
        self.active_face(active_face.face_id(), active_face.resolved_face())
            .append_to_text_row_and_emit(
                builder,
                output_emitter,
                evaluator,
                font_metrics,
                face_resolver,
                position,
                source_id,
                text,
            )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_active_face_request_to_text_row_and_emit(
        self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        request: SyntheticTextAppendRequest,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let position = request.position();
        let (source_id, text) = request.into_source_text();
        self.append_active_face_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            position,
            source_id,
            text,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_text_row_metrics_to_text_row_and_emit(
        self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        position: DisplayRowPosition,
        source_id: u64,
        text: impl Into<Box<str>>,
        face_id: u32,
        base_face: &'a ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.text_row(face_id, base_face, height_px, ascent_px, char_width_px)
            .append_to_text_row_and_emit(
                builder,
                output_emitter,
                evaluator,
                font_metrics,
                face_resolver,
                position,
                source_id,
                text,
            )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_text_row_metrics_request_to_text_row_and_emit(
        self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        request: SyntheticTextMetricsAppendRequest<'a>,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let parts = request.into_parts();
        self.append_text_row_metrics_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            parts.position,
            parts.source_id,
            parts.text,
            parts.face_id,
            parts.base_face,
            parts.height_px,
            parts.ascent_px,
            parts.char_width_px,
        )
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
    let Some(mut source) = crate::display_source::LispStringSourceCursor::new(
        source_id,
        text_value,
        RenderFaceRef::FaceId(base_face_id),
    ) else {
        return position;
    };
    let request = frame.source_append_request(
        position,
        base_face_id,
        base_face,
        DisplayRowAppendKind::SourceText,
    );
    let mut source_state = DisplayRowSourceState::default();
    let mut font_metrics = None;
    let Some(outcome) = request.render_natural_display_source_into_current_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        &mut font_metrics,
        &mut source,
        &mut source_state,
        face_resolver,
        face_ids,
    ) else {
        return position;
    };
    outcome.end_position()
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

fn buffer_text_source_range_item<B: LayoutBufferView + ?Sized>(
    range: BufferTextSourceRange,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
) -> Option<(DisplayItem, DisplayRowAppendKind)> {
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
    Some((item, append_kind))
}

#[allow(clippy::too_many_arguments)]
fn append_resolved_buffer_text_source_range_to_text_row<B: LayoutBufferView + ?Sized>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    range: BufferTextSourceRange,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
    resolved_advance: ResolvedBufferTextSourceAdvance,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let (item, append_kind) = buffer_text_source_range_item(range, buffer_id, buffer, face_id)?;
    let request = frame.source_append_request(position, face_id, base_face, append_kind);
    let mut face_ids = FrameFaceIdAllocator::new(face_id.saturating_add(1));
    let mut render_policy =
        DisplaySourceAppendRenderPolicy::new(resolved_advance.append_measurement());
    let outcome = request.render_single_display_item_into_current_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        item,
        face_resolver,
        &mut face_ids,
        &mut render_policy,
    )?;
    Some(outcome.into_append_progress_and_position(position))
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

    #[allow(clippy::too_many_arguments)]
    #[cfg(test)]
    fn resolve_source_range_advance_to_text_row(
        &self,
        state: &mut BufferTextRowAppendState,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        text: &[u8],
        byte_idx: usize,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        active_face_state: &DisplayRowActiveFaceState,
        position: DisplayRowPosition,
        cluster: BufferTextSourceClusterState,
    ) -> ResolvedBufferTextSourceAdvance {
        state.advance_resolver().resolve_source_range_to_text_row(
            builder,
            evaluator,
            font_metrics,
            text,
            byte_idx,
            range,
            face_resolver,
            self.buffer_id,
            self.buffer,
            active_face_state,
            self.frame.clone(),
            position,
            cluster,
        )
    }

    #[allow(clippy::too_many_arguments)]
    #[cfg(test)]
    fn append_resolved_source_range_to_text_row(
        &self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        resolved_advance: ResolvedBufferTextSourceAdvance,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        append_resolved_buffer_text_source_range_to_text_row(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            range,
            face_resolver,
            self.base_face,
            self.buffer_id,
            self.buffer,
            self.face_id,
            resolved_advance,
            self.frame.clone(),
            position,
        )
    }

    #[cfg(test)]
    fn measure_source_range_natural_advance_to_text_row(
        &self,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        position: DisplayRowPosition,
    ) -> Option<f32> {
        measure_buffer_text_source_range_natural_advance_to_text_row(
            builder,
            evaluator,
            font_metrics,
            range,
            face_resolver,
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

    #[allow(clippy::too_many_arguments)]
    fn measure_item_source_range_width_or_active_face_fallback_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        item: BufferTextSourceAppendItem,
        position: DisplayRowPosition,
    ) -> f32 {
        self.item_active_face(geometry)
            .measure_source_range_width_or_active_face_fallback_to_text_row(
                builder,
                evaluator,
                font_metrics,
                range,
                face_resolver,
                item,
                position,
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn measure_special_source_char_request_width_or_active_face_fallback_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        request: BufferTextSpecialSourceCharMeasureRequest,
    ) -> f32 {
        let parts = request.into_parts();
        self.measure_item_source_range_width_or_active_face_fallback_to_text_row(
            geometry,
            builder,
            evaluator,
            font_metrics,
            parts.range,
            face_resolver,
            parts.special_display.into_append_item(),
            parts.position,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_special_source_char_layout_plan(
        &self,
        geometry: &DisplayRowGeometryState,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        request: BufferTextSpecialSourceCharRequest,
        position: DisplayRowPosition,
    ) -> BufferTextSpecialSourceCharLayoutPlan {
        let measured_width_px = self
            .measure_special_source_char_request_width_or_active_face_fallback_to_text_row(
                geometry,
                builder,
                evaluator,
                font_metrics,
                face_resolver,
                request.measure_at(position),
            );
        request.layout_plan(measured_width_px)
    }

    #[allow(clippy::too_many_arguments)]
    fn append_item_source_range_to_text_row_and_emit(
        &self,
        geometry: &DisplayRowGeometryState,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        item: BufferTextSourceAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.item_active_face(geometry)
            .append_source_range_to_text_row_and_emit(
                builder,
                output_emitter,
                evaluator,
                font_metrics,
                range,
                face_resolver,
                item,
                position,
            )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_special_source_char_plan_to_text_row_and_emit(
        &self,
        geometry: &DisplayRowGeometryState,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        plan: BufferTextSpecialSourceCharAppendPlan,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let parts = plan.into_parts();
        self.append_item_source_range_to_text_row_and_emit(
            geometry,
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            parts.range,
            face_resolver,
            parts.special_display.into_append_item(),
            parts.position,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_source_range_advance_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut BufferTextRowAppendState,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        text: &[u8],
        byte_idx: usize,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        position: DisplayRowPosition,
        cluster: BufferTextSourceClusterState,
    ) -> ResolvedBufferTextSourceAdvance {
        let frame = self.active_face_context(geometry).active_face_frame();
        state.advance_resolver().resolve_source_range_to_text_row(
            builder,
            evaluator,
            font_metrics,
            text,
            byte_idx,
            range,
            face_resolver,
            self.buffer_id,
            self.buffer,
            self.active_face,
            frame,
            position,
            cluster,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_source_char_append_request_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut BufferTextRowAppendState,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        request: BufferTextSourceCharAppendRequest<'_>,
    ) -> ResolvedBufferTextSourceAdvance {
        let parts = request.into_parts();
        self.resolve_source_range_advance_to_text_row(
            geometry,
            state,
            builder,
            evaluator,
            font_metrics,
            parts.text,
            parts.byte_idx,
            parts.range,
            face_resolver,
            parts.position,
            parts.cluster,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_source_char_append_plan(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut BufferTextRowAppendState,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        request: BufferTextSourceCharAppendRequest<'_>,
    ) -> BufferTextSourceCharAppendPlan {
        let resolved_advance = self.resolve_source_char_append_request_to_text_row(
            geometry,
            state,
            builder,
            evaluator,
            font_metrics,
            face_resolver,
            request,
        );
        request.append_plan(resolved_advance)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_source_char_at(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut BufferTextRowAppendState,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        source_char: &BufferTextSourceChar,
        text: &[u8],
        byte_idx: usize,
        position: DisplayRowPosition,
        cluster_tail: Option<(char, bool)>,
    ) -> BufferTextSourceCharPreparedAppend {
        let request = source_char.append_request_at(text, byte_idx, position, cluster_tail);
        BufferTextSourceCharPreparedAppend {
            plan: self.prepare_source_char_append_plan(
                geometry,
                state,
                builder,
                evaluator,
                font_metrics,
                face_resolver,
                request,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn append_resolved_source_range_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        resolved_advance: ResolvedBufferTextSourceAdvance,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let frame = self.active_face_context(geometry).active_face_frame();
        append_resolved_buffer_text_source_range_to_text_row(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            range,
            face_resolver,
            self.active_face.resolved_face(),
            self.buffer_id,
            self.buffer,
            self.active_face.face_id(),
            resolved_advance,
            frame,
            position,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn append_source_char_plan_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        plan: BufferTextSourceCharAppendPlan,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let parts = plan.into_parts();
        self.append_resolved_source_range_to_text_row(
            geometry,
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            parts.range,
            face_resolver,
            parts.resolved_advance,
            parts.position,
        )
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

    pub(crate) fn cursor_slot_width(self) -> CapturedCursorSlotWidth {
        CapturedCursorSlotWidth::Explicit(self.advance_px())
    }

    pub(crate) fn track_trailing_whitespace_rendered_char(
        self,
        trailing_whitespace: &mut TrailingWhitespaceRenderState,
        ch: char,
        geometry: &DisplayRowGeometryState,
        current_x_px: f32,
    ) {
        trailing_whitespace.track_rendered_char(
            ch,
            geometry.start_marker_at_x(current_x_px - self.advance_px()),
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_to_text_row<B: LayoutBufferView + ?Sized>(
        self,
        context: &BufferTextRowAppendContext<'_, '_, B>,
        geometry: &DisplayRowGeometryState,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        context.append_source_char_plan_to_text_row(
            geometry,
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            self.plan,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn measure_buffer_text_source_range_append_progress_to_text_row<B: LayoutBufferView + ?Sized>(
    builder: &mut GlyphMatrixBuilder,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    range: BufferTextSourceRange,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<DisplayRowAppendProgress> {
    let (item, append_kind) = buffer_text_source_range_item(range, buffer_id, buffer, face_id)?;
    let request = frame
        .source_append_request(position, face_id, base_face, append_kind)
        .with_measurement_bounds(DisplayRowRenderBounds::unbounded_from(position));
    let mut face_ids = FrameFaceIdAllocator::new(face_id.saturating_add(1));
    let mut render_policy =
        DisplaySourceAppendRenderPolicy::new(DisplaySourceAppendMeasurement::Natural);
    let outcome = request.measure_single_display_item_against_current_text_row(
        builder,
        evaluator,
        font_metrics,
        item,
        face_resolver,
        &mut face_ids,
        &mut render_policy,
    )?;
    Some(outcome.into_append_progress(position))
}

#[allow(clippy::too_many_arguments)]
fn measure_buffer_text_source_range_natural_advance_to_text_row<B: LayoutBufferView + ?Sized>(
    builder: &mut GlyphMatrixBuilder,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    range: BufferTextSourceRange,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<f32> {
    Some(
        measure_buffer_text_source_range_append_progress_to_text_row(
            builder,
            evaluator,
            font_metrics,
            range,
            face_resolver,
            base_face,
            buffer_id,
            buffer,
            face_id,
            frame,
            position,
        )?
        .metrics
        .width_px,
    )
}

#[allow(clippy::too_many_arguments)]
fn resolve_buffer_text_source_range_natural_advance_to_text_row<B: LayoutBufferView + ?Sized>(
    builder: &mut GlyphMatrixBuilder,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    range: BufferTextSourceRange,
    face_resolver: &FaceResolver,
    buffer_id: BufferId,
    buffer: &B,
    active_face_state: &DisplayRowActiveFaceState,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    cluster: BufferTextSourceClusterState,
) -> f32 {
    if let Some(measured_width) = measure_buffer_text_source_range_natural_advance_to_text_row(
        builder,
        evaluator,
        font_metrics,
        range,
        face_resolver,
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
        font_metrics,
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
    fn resolve_source_range_to_text_row<B: LayoutBufferView + ?Sized>(
        &mut self,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        text: &[u8],
        byte_idx: usize,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        buffer_id: BufferId,
        buffer: &B,
        active_face_state: &DisplayRowActiveFaceState,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
        cluster: BufferTextSourceClusterState,
    ) -> ResolvedBufferTextSourceAdvance {
        let ch = cluster.ch();
        match BufferTextSourceAdvancePath::for_cluster_state(cluster) {
            BufferTextSourceAdvancePath::ResolvedComplexRun => {
                let mut policy =
                    DisplayRowComplexTextRunAdvancePolicy::new(active_face_state, font_metrics);
                let advance_px = self.complex_run.advance_for_char(
                    text,
                    byte_idx,
                    ch,
                    cluster.is_cluster_continuation(),
                    &mut policy,
                );
                ResolvedBufferTextSourceAdvance::resolved(advance_px)
            }
            BufferTextSourceAdvancePath::NaturalRenderedSource => {
                let advance_px = resolve_buffer_text_source_range_natural_advance_to_text_row(
                    builder,
                    evaluator,
                    font_metrics,
                    range,
                    face_resolver,
                    buffer_id,
                    buffer,
                    active_face_state,
                    frame,
                    position,
                    cluster,
                );
                ResolvedBufferTextSourceAdvance::natural(advance_px)
            }
        }
    }
}

fn buffer_display_item_source_range_item<B: LayoutBufferView + ?Sized>(
    range: BufferTextSourceRange,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
    item: BufferTextSourceAppendItem,
) -> Option<(DisplayItem, DisplayRowAppendKind)> {
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

    let append_kind = item.append_kind();
    let item = source.item(
        RenderFaceRef::FaceId(face_id),
        item.into_display_item_kind(),
    );
    Some((item, append_kind))
}

#[allow(clippy::too_many_arguments)]
fn append_buffer_display_item_source_range_to_text_row_and_emit<B: LayoutBufferView + ?Sized>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    range: BufferTextSourceRange,
    buffer: &B,
    buffer_id: BufferId,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    face_id: u32,
    item: BufferTextSourceAppendItem,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let (item, append_kind) =
        buffer_display_item_source_range_item(range, buffer_id, buffer, face_id, item)?;
    let request = frame.source_append_request(position, face_id, base_face, append_kind);
    let mut face_ids = FrameFaceIdAllocator::new(face_id.saturating_add(1));
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    let outcome = request.render_single_display_item_into_current_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        item,
        face_resolver,
        &mut face_ids,
        &mut render_policy,
    )?;
    Some(outcome.into_append_progress_and_position(position))
}

#[allow(clippy::too_many_arguments)]
fn measure_buffer_display_item_source_range_append_progress_to_text_row<
    B: LayoutBufferView + ?Sized,
>(
    builder: &mut GlyphMatrixBuilder,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    range: BufferTextSourceRange,
    buffer: &B,
    buffer_id: BufferId,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    face_id: u32,
    item: BufferTextSourceAppendItem,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<DisplayRowAppendProgress> {
    let (item, append_kind) =
        buffer_display_item_source_range_item(range, buffer_id, buffer, face_id, item)?;
    let request = frame
        .source_append_request(position, face_id, base_face, append_kind)
        .with_measurement_bounds(DisplayRowRenderBounds::unbounded_from(position));
    let mut face_ids = FrameFaceIdAllocator::new(face_id.saturating_add(1));
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    let outcome = request.measure_single_display_item_against_current_text_row(
        builder,
        evaluator,
        font_metrics,
        item,
        face_resolver,
        &mut face_ids,
        &mut render_policy,
    )?;
    Some(outcome.into_append_progress(position))
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BufferTextSourceAppendItem {
    ControlChar { ch: char },
    SourceMappedText { text: Box<str> },
    Glyphless { ch: char, method: GlyphlessMethod },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BufferTextSourceSpecialDisplay {
    Control(BufferTextSourceAppendItem),
    Nobreak(BufferTextSourceAppendItem),
    Glyphless(BufferTextSourceAppendItem),
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

    pub(crate) fn control_special_request(&self) -> Option<BufferTextSpecialSourceCharRequest> {
        self.precluster_special_display()
            .filter(|display| display.is_control())
            .cloned()
            .map(|display| self.special_request_for_display(display))
    }

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

    fn append_request_at<'text>(
        &self,
        text: &'text [u8],
        byte_idx: usize,
        position: DisplayRowPosition,
        tail: Option<(char, bool)>,
    ) -> BufferTextSourceCharAppendRequest<'text> {
        BufferTextSourceCharAppendRequest {
            text,
            byte_idx,
            range: self.range(),
            position,
            cluster: self.cluster_state(tail),
        }
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

    pub(crate) fn append_plan_at(
        &self,
        position: DisplayRowPosition,
    ) -> BufferTextSpecialSourceCharAppendPlan {
        BufferTextSpecialSourceCharAppendPlan {
            range: self.range,
            special_display: self.special_display.clone(),
            position,
        }
    }

    fn layout_plan(self, measured_width_px: f32) -> BufferTextSpecialSourceCharLayoutPlan {
        BufferTextSpecialSourceCharLayoutPlan {
            source_request: self,
            measured_width_px,
        }
    }

    fn measure_at(
        &self,
        position: DisplayRowPosition,
    ) -> BufferTextSpecialSourceCharMeasureRequest {
        BufferTextSpecialSourceCharMeasureRequest {
            range: self.range,
            special_display: self.special_display.clone(),
            position,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSpecialSourceCharLayoutPlan {
    source_request: BufferTextSpecialSourceCharRequest,
    measured_width_px: f32,
}

impl BufferTextSpecialSourceCharLayoutPlan {
    pub(crate) fn measured_width_px(&self) -> f32 {
        self.measured_width_px
    }

    pub(crate) fn append_plan_at(
        &self,
        position: DisplayRowPosition,
    ) -> BufferTextSpecialSourceCharAppendPlan {
        self.source_request.append_plan_at(position)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSpecialSourceCharAppendPlan {
    range: BufferTextSourceRange,
    special_display: BufferTextSourceSpecialDisplay,
    position: DisplayRowPosition,
}

impl BufferTextSpecialSourceCharAppendPlan {
    fn into_parts(self) -> BufferTextSpecialSourceCharAppendPlanParts {
        BufferTextSpecialSourceCharAppendPlanParts {
            range: self.range,
            special_display: self.special_display,
            position: self.position,
        }
    }
}

struct BufferTextSpecialSourceCharAppendPlanParts {
    range: BufferTextSourceRange,
    special_display: BufferTextSourceSpecialDisplay,
    position: DisplayRowPosition,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSpecialSourceCharMeasureRequest {
    range: BufferTextSourceRange,
    special_display: BufferTextSourceSpecialDisplay,
    position: DisplayRowPosition,
}

impl BufferTextSpecialSourceCharMeasureRequest {
    fn into_parts(self) -> BufferTextSpecialSourceCharMeasureRequestParts {
        BufferTextSpecialSourceCharMeasureRequestParts {
            range: self.range,
            special_display: self.special_display,
            position: self.position,
        }
    }
}

struct BufferTextSpecialSourceCharMeasureRequestParts {
    range: BufferTextSourceRange,
    special_display: BufferTextSourceSpecialDisplay,
    position: DisplayRowPosition,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BufferTextSourceCharAppendPlan {
    range: BufferTextSourceRange,
    resolved_advance: ResolvedBufferTextSourceAdvance,
    position: DisplayRowPosition,
}

impl BufferTextSourceCharAppendPlan {
    fn into_parts(self) -> BufferTextSourceCharAppendPlanParts {
        BufferTextSourceCharAppendPlanParts {
            range: self.range,
            resolved_advance: self.resolved_advance,
            position: self.position,
        }
    }

    fn advance_px(self) -> f32 {
        self.resolved_advance.advance_px()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BufferTextSourceCharAppendPlanParts {
    range: BufferTextSourceRange,
    resolved_advance: ResolvedBufferTextSourceAdvance,
    position: DisplayRowPosition,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BufferTextSourceCharAppendRequest<'text> {
    text: &'text [u8],
    byte_idx: usize,
    range: BufferTextSourceRange,
    position: DisplayRowPosition,
    cluster: BufferTextSourceClusterState,
}

impl<'text> BufferTextSourceCharAppendRequest<'text> {
    fn into_parts(self) -> BufferTextSourceCharAppendRequestParts<'text> {
        BufferTextSourceCharAppendRequestParts {
            text: self.text,
            byte_idx: self.byte_idx,
            range: self.range,
            position: self.position,
            cluster: self.cluster,
        }
    }

    fn append_plan(
        self,
        resolved_advance: ResolvedBufferTextSourceAdvance,
    ) -> BufferTextSourceCharAppendPlan {
        BufferTextSourceCharAppendPlan {
            range: self.range,
            resolved_advance,
            position: self.position,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BufferTextSourceCharAppendRequestParts<'text> {
    text: &'text [u8],
    byte_idx: usize,
    range: BufferTextSourceRange,
    position: DisplayRowPosition,
    cluster: BufferTextSourceClusterState,
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

    fn is_nobreak(&self) -> bool {
        matches!(self, Self::Nobreak(_))
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_source_range_to_text_row_and_emit(
        &self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        item: BufferTextSourceAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        append_buffer_display_item_source_range_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            range,
            self.buffer,
            self.buffer_id,
            face_resolver,
            self.base_face,
            self.face_id,
            item,
            self.frame.clone(),
            position,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn measure_source_range_width_to_text_row(
        &self,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        item: BufferTextSourceAppendItem,
        position: DisplayRowPosition,
    ) -> Option<f32> {
        Some(
            measure_buffer_display_item_source_range_append_progress_to_text_row(
                builder,
                evaluator,
                font_metrics,
                range,
                self.buffer,
                self.buffer_id,
                face_resolver,
                self.base_face,
                self.face_id,
                item,
                self.frame.clone(),
                position,
            )?
            .metrics
            .width_px,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn measure_source_range_width_or_active_face_fallback_to_text_row(
        &self,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        item: BufferTextSourceAppendItem,
        position: DisplayRowPosition,
    ) -> f32 {
        let fallback_width = item
            .fallback_width_policy()
            .width_px(self.frame.geometry.char_width);
        self.measure_source_range_width_to_text_row(
            builder,
            evaluator,
            font_metrics,
            range,
            face_resolver,
            item,
            position,
        )
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
    source_id: u64,
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
            source_id,
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

    fn source_append_request(&self, position: DisplayRowPosition) -> LispStringSourceAppendRequest {
        LispStringSourceAppendRequest::new(position, self.source_id, self.value)
    }
}

fn append_raw_display_replacement_item_to_text_row_and_emit(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    item: DisplayItem,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    fallback_face_id: u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let face_id = render_face_ref_id(item.face, fallback_face_id);
    let request = frame.source_append_request(
        position,
        face_id,
        base_face,
        DisplayRowAppendKind::DisplayReplacement,
    );
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    append_single_display_item_fragment_to_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        item,
        face_resolver,
        request,
        &mut render_policy,
    )
}

#[derive(Clone, Debug)]
enum DisplayReplacementAppendItem {
    Media(DisplayMediaReplacement),
    Stretch(DisplayReplacementBox),
    SourceMappedText(Box<str>),
}

impl DisplayReplacementAppendItem {
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_and_append_to_text_row<B: LayoutBufferView>(
        self,
        buffer: &B,
        evaluator: &mut Context,
        output_emitter: &mut WindowOutputEmitter,
        builder: &mut GlyphMatrixBuilder,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        append_surface: &DisplayRowAppendSurface,
        row_geometry: &mut DisplayRowGeometryState,
    ) -> Option<DisplayPropertyReplacementAppendOutcome> {
        let active_face_state = self.active_face_state;
        let request = self.resolve(font_metrics, evaluator.display_host.as_deref())?;
        Some(request.append_to_text_row(
            buffer,
            evaluator,
            output_emitter,
            builder,
            font_metrics,
            face_resolver,
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

    pub(crate) fn row_append_context<'a>(
        &self,
        append_surface: &'a DisplayRowAppendSurface,
        geometry: &DisplayRowGeometryState,
        active_face: &'a DisplayRowActiveFaceState,
    ) -> DisplayReplacementRowAppendContext<'a> {
        DisplayReplacementRowAppendContext::new(
            self.replacement_source,
            append_surface,
            geometry,
            active_face,
            self.glyph_y_offset,
            self.default_row_height,
        )
    }

    pub(crate) fn string_base_face_request(
        &self,
    ) -> Option<DisplayPropertyReplacementStringBaseFaceRequest> {
        match &self.item {
            DisplayPropertyReplacementAppendItem::String(item) if !item.is_empty() => {
                Some(DisplayPropertyReplacementStringBaseFaceRequest {
                    origin: item.origin(),
                    base_face_policy: item.base_face_policy(),
                })
            }
            _ => None,
        }
    }

    pub(crate) fn into_plan(
        self,
        string_base_face: Option<DisplayStringBaseFace>,
    ) -> DisplayPropertyReplacementAppendPlan {
        DisplayPropertyReplacementAppendPlan {
            request: self,
            string_base_face,
        }
    }

    pub(crate) fn into_item(self) -> DisplayPropertyReplacementAppendItem {
        self.item
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_to_text_row<B: LayoutBufferView>(
        self,
        buffer: &B,
        evaluator: &mut Context,
        output_emitter: &mut WindowOutputEmitter,
        builder: &mut GlyphMatrixBuilder,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        append_surface: &DisplayRowAppendSurface,
        row_geometry: &mut DisplayRowGeometryState,
        active_face_state: &DisplayRowActiveFaceState,
    ) -> DisplayPropertyReplacementAppendOutcome {
        let start_position = self.start_position();
        let cursor_policy = self.cursor_policy();
        let string_base_face = self.string_base_face_request().map(|request| {
            display_string_base_face_for_active_row(
                buffer,
                face_resolver,
                request.origin(),
                request.base_face_policy(),
                active_face_state,
                face_ids,
                builder,
            )
        });
        let end_position = self.into_plan(string_base_face).append_to_text_row(
            evaluator,
            output_emitter,
            builder,
            font_metrics,
            face_resolver,
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayPropertyReplacementStringBaseFaceRequest {
    origin: DisplayOrigin,
    base_face_policy: BaseFacePolicy,
}

impl DisplayPropertyReplacementStringBaseFaceRequest {
    pub(crate) fn origin(self) -> DisplayOrigin {
        self.origin
    }

    pub(crate) fn base_face_policy(self) -> BaseFacePolicy {
        self.base_face_policy
    }
}

pub(crate) struct DisplayPropertyReplacementAppendPlan {
    request: DisplayPropertyReplacementAppendRequest,
    string_base_face: Option<DisplayStringBaseFace>,
}

impl DisplayPropertyReplacementAppendPlan {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_to_text_row(
        self,
        evaluator: &mut Context,
        output_emitter: &mut WindowOutputEmitter,
        builder: &mut GlyphMatrixBuilder,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        append_surface: &DisplayRowAppendSurface,
        row_geometry: &mut DisplayRowGeometryState,
        active_face_state: &DisplayRowActiveFaceState,
    ) -> DisplayRowPosition {
        let position = self.request.start_position();
        let replacement_append_context =
            self.request
                .row_append_context(append_surface, row_geometry, active_face_state);
        match self.request.into_item() {
            DisplayPropertyReplacementAppendItem::String(replacement_item) => {
                if replacement_item.is_empty() {
                    return position;
                }
                let Some(replacement_base_face) = self.string_base_face else {
                    debug_assert!(false, "display string replacement missing base face");
                    return position;
                };
                replacement_append_context.append_full_text_width_string_item_to_text_row(
                    builder,
                    output_emitter,
                    evaluator,
                    font_metrics,
                    replacement_item,
                    face_resolver,
                    face_ids,
                    replacement_base_face.face_id(),
                    replacement_base_face.face(),
                    position,
                )
            }
            DisplayPropertyReplacementAppendItem::Stretch(stretch_item) => {
                if stretch_item.width_px() <= 0.0 {
                    return position;
                }
                row_geometry.include_glyph_vertical_metrics(
                    stretch_item.height_px(),
                    stretch_item.ascent_px(),
                );
                replacement_append_context
                    .append_active_face_stretch_to_text_row_and_emit(
                        builder,
                        output_emitter,
                        evaluator,
                        font_metrics,
                        face_resolver,
                        stretch_item,
                        position,
                    )
                    .map(|(_progress, position)| position)
                    .unwrap_or(position)
            }
            DisplayPropertyReplacementAppendItem::Media(
                DisplayReplacementMediaAppendResolution::Media(media_item),
            ) => {
                if let Some((progress, appended_position)) = replacement_append_context
                    .append_display_box_media_to_text_row_and_emit(
                        builder,
                        output_emitter,
                        evaluator,
                        font_metrics,
                        face_resolver,
                        media_item,
                        position,
                    )
                    && let Some((height, ascent)) = media_item.row_extents_after_append(&progress)
                {
                    row_geometry.include_row_extents(height, ascent);
                    appended_position
                } else {
                    position
                }
            }
            DisplayPropertyReplacementAppendItem::Media(
                DisplayReplacementMediaAppendResolution::Placeholder(placeholder_item),
            ) => replacement_append_context
                .append_active_face_source_mapped_text_to_text_row_and_emit(
                    builder,
                    output_emitter,
                    evaluator,
                    font_metrics,
                    face_resolver,
                    placeholder_item,
                    position,
                )
                .map(|(_progress, position)| position)
                .unwrap_or(position),
        }
    }
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_full_text_width_string_item_to_text_row(
        self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        item: DisplayReplacementStringAppendItem,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        face_id: u32,
        base_face: &'a ResolvedFace,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        self.full_text_width_active_face(face_id, base_face)
            .append_string_item_to_text_row(
                builder,
                output_emitter,
                evaluator,
                font_metrics,
                item,
                face_resolver,
                face_ids,
                position,
            )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_active_face_stretch_to_text_row_and_emit(
        self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        item: DisplayReplacementStretchAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.active_face(self.active_face.face_id(), self.active_face.resolved_face())
            .append_stretch_to_text_row_and_emit(
                builder,
                output_emitter,
                evaluator,
                font_metrics,
                face_resolver,
                item,
                position,
            )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_display_box_media_to_text_row_and_emit(
        self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        item: DisplayReplacementMediaAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.display_box(
            self.active_face.face_id(),
            self.active_face.resolved_face(),
            item.display_height_px(),
            item.display_ascent_px(),
        )
        .append_media_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            item,
            position,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_active_face_source_mapped_text_to_text_row_and_emit(
        self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        item: DisplayReplacementSourceMappedTextAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.active_face(self.active_face.face_id(), self.active_face.resolved_face())
            .append_source_mapped_text_to_text_row_and_emit(
                builder,
                output_emitter,
                evaluator,
                font_metrics,
                face_resolver,
                item,
                position,
            )
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

    #[allow(clippy::too_many_arguments)]
    fn append_item_to_text_row_and_emit(
        &self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        item: DisplayReplacementAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        append_display_replacement_item_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            self.replacement_source,
            self.face_id,
            item,
            face_resolver,
            self.base_face,
            self.frame.clone(),
            position,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_stretch_to_text_row_and_emit(
        &self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        item: DisplayReplacementStretchAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.append_item_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            DisplayReplacementAppendItem::Stretch(item.geometry()),
            position,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_media_to_text_row_and_emit(
        &self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        item: DisplayReplacementMediaAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.append_item_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            DisplayReplacementAppendItem::Media(item.media()),
            position,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_source_mapped_text_to_text_row_and_emit(
        &self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        item: DisplayReplacementSourceMappedTextAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.append_item_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            DisplayReplacementAppendItem::SourceMappedText(item.text()),
            position,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_string_item_to_text_row(
        &self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        item: DisplayReplacementStringAppendItem,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        let request = item.source_append_request(position);
        let mut item_policy = item.string_item_measurer();
        self.append_string_source_request_to_text_row(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            face_ids,
            request,
            &mut item_policy,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn append_string_source_request_to_text_row(
        &self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        request: LispStringSourceAppendRequest,
        item_policy: &mut impl DisplayRowRenderPolicy,
    ) -> DisplayRowPosition {
        let parts = request.into_parts();
        append_display_replacement_string_value_to_text_row(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            parts.value,
            self.replacement_source,
            parts.source_id,
            face_resolver,
            self.base_face,
            self.face_id,
            face_ids,
            self.frame.clone(),
            parts.position,
            item_policy,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn append_display_replacement_item_to_text_row_and_emit(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    replacement_source: BufferDisplayReplacementSource,
    face_id: u32,
    item: DisplayReplacementAppendItem,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let item = item.into_display_item(replacement_source, face_id);
    append_raw_display_replacement_item_to_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        item,
        face_resolver,
        base_face,
        face_id,
        frame,
        position,
    )
}

#[allow(clippy::too_many_arguments)]
fn append_display_replacement_string_source_to_text_row<S: DisplayItemSource>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    source: S,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    fallback_face_id: u32,
    face_ids: &mut FrameFaceIdAllocator,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    item_policy: &mut impl DisplayRowRenderPolicy,
) -> DisplayRowPosition {
    let request = frame.source_append_request(
        position,
        fallback_face_id,
        base_face,
        DisplayRowAppendKind::DisplayReplacementString,
    );
    let mut render_policy = DisplayReplacementStringRenderPolicy { item_policy };
    let Some(outcome) = request.render_owned_display_source_into_current_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        source,
        face_resolver,
        face_ids,
        &mut render_policy,
    ) else {
        return position;
    };
    outcome.end_position()
}

#[allow(clippy::too_many_arguments)]
fn append_display_replacement_string_value_to_text_row(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    text_value: Value,
    replacement_source: BufferDisplayReplacementSource,
    source_id: u64,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    fallback_face_id: u32,
    face_ids: &mut FrameFaceIdAllocator,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    item_policy: &mut impl DisplayRowRenderPolicy,
) -> DisplayRowPosition {
    let Some(string_source) = LispStringSourceCursor::new(
        source_id,
        text_value,
        RenderFaceRef::FaceId(fallback_face_id),
    ) else {
        return position;
    };
    let source = BufferDisplayReplacementStringSource::new(replacement_source, string_source);
    append_display_replacement_string_source_to_text_row(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        source,
        face_resolver,
        base_face,
        fallback_face_id,
        face_ids,
        frame,
        position,
        item_policy,
    )
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
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    source_id: u64,
    text: impl Into<Box<str>>,
    face_id: u32,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let item = synthetic_display_text_item(source_id, text, face_id);
    let request = frame.source_append_request(
        position,
        face_id,
        base_face,
        DisplayRowAppendKind::SourceText,
    );
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    append_single_display_item_fragment_to_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        item,
        face_resolver,
        request,
        &mut render_policy,
    )
}

#[cfg(test)]
#[path = "display_row_append_test.rs"]
mod tests;
