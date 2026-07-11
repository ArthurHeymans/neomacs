use crate::display_face_id::FrameFaceIdAllocator;
#[cfg(test)]
use crate::display_face_policy::BaseFacePolicy;
use crate::display_item::RenderFaceRef;
use crate::display_origin::DisplayOrigin;
#[cfg(test)]
use crate::display_output_builder::DisplayOutputBuilder;
use crate::display_row_append_context::{
    DisplayRowActiveFaceAppendContext, DisplayRowAppendFrame, DisplayRowAppendKind,
    DisplayRowAppendMetrics, DisplayRowAppendSurface,
};
use crate::display_row_builder::DisplayRowPosition;
use crate::display_row_face_state::DisplayRowActiveFaceState;
use crate::display_row_geometry::DisplayRowGeometryState;
use crate::display_row_metrics::DisplayRowFallbackMetrics;
use crate::display_row_render_state::CurrentTextRowRenderOutcome;
use crate::display_row_source_append::SingleDisplayItemAppendContext;
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_row_source_state::DisplayRowSourceState;
use crate::display_row_walk_state::TextRowTransitionPrefixAction;
use crate::display_source::{LispStringSourceCursor, LispStringSourceOrigin};
use crate::display_source_append_plan::NaturalDisplayRowAppendRenderPolicy;
use crate::display_source_resolver::DisplayStringBaseFace;
#[cfg(test)]
use crate::display_source_resolver::PendingDisplaySourceFace;
#[cfg(test)]
use crate::display_text_output_install::install_output_resolved_face;
use crate::neovm_bridge::{LayoutBufferView, ResolvedFace, RustTextPropAccess};
use neomacs_display_protocol::types::FaceId;
use neovm_core::buffer::CharPos0;
use neovm_core::emacs_core::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LispStringSourceId(pub(crate) u64);

impl LispStringSourceId {
    pub(crate) const OVERLAY_STRING: Self = Self(1);
    pub(crate) const PREFIX: Self = Self(2);

    #[cfg(test)]
    pub(crate) fn display_replacement(source_id: u64) -> Self {
        Self(source_id)
    }

    pub(crate) fn raw(self) -> u64 {
        self.0
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
        fallback_metrics: DisplayRowFallbackMetrics,
    ) -> Self {
        Self {
            active_face_context: DisplayRowActiveFaceAppendContext::new(
                append_surface,
                geometry,
                active_face,
                glyph_y_offset,
                fallback_metrics,
            ),
        }
    }

    pub(crate) fn render_active_face_source_request_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        request: LispStringSourceAppendSessionRequest<'row>,
    ) -> DisplayRowPosition {
        let position = request.position();
        let Some(mut source_session) = LispStringSourceAppendSession::new(request) else {
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
            LispStringSourceAppendSessionRequest::new(
                prefix_source.append_request(position),
                base_face.face_id(),
                base_face.face(),
            ),
        )
    }
}

fn render_lisp_string_source_append_to_text_row_and_emit(
    state: &mut TextRowSourceRenderState<'_>,
    source: &mut LispStringSourceCursor,
    source_state: &mut DisplayRowSourceState,
    base_face: &ResolvedFace,
    base_face_id: FaceId,
    face_ids: &mut FrameFaceIdAllocator,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<CurrentTextRowRenderOutcome> {
    let append_context = SingleDisplayItemAppendContext::new(base_face, base_face_id, frame);
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    append_context.render_source_with_policy(
        state,
        face_ids,
        source,
        source_state,
        position,
        DisplayRowAppendKind::SourceText,
        &mut render_policy,
        base_face_id,
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LispStringSourceAppendRequest {
    pub(crate) position: DisplayRowPosition,
    pub(crate) source_id: LispStringSourceId,
    pub(crate) value: Value,
    source_origin: LispStringSourceOrigin,
}

impl LispStringSourceAppendRequest {
    pub(crate) fn new(
        position: DisplayRowPosition,
        source_id: LispStringSourceId,
        value: Value,
    ) -> Self {
        Self {
            position,
            source_id,
            value,
            source_origin: LispStringSourceOrigin::Normal,
        }
    }

    pub(crate) fn with_source_origin(mut self, source_origin: LispStringSourceOrigin) -> Self {
        self.source_origin = source_origin;
        self
    }

    fn into_source(self, base_face_id: FaceId) -> Option<LispStringSourceCursor> {
        LispStringSourceCursor::new(
            self.source_id.raw(),
            self.value,
            RenderFaceRef::FaceId(base_face_id),
            self.source_origin,
        )
    }
}

pub(crate) struct LispStringSourceAppendSessionRequest<'a> {
    append_request: LispStringSourceAppendRequest,
    base_face_id: FaceId,
    base_face: &'a ResolvedFace,
}

impl<'a> LispStringSourceAppendSessionRequest<'a> {
    pub(crate) fn new(
        append_request: LispStringSourceAppendRequest,
        base_face_id: FaceId,
        base_face: &'a ResolvedFace,
    ) -> Self {
        Self {
            append_request,
            base_face_id,
            base_face,
        }
    }

    fn position(&self) -> DisplayRowPosition {
        self.append_request.position
    }
}

pub(crate) struct LispStringSourceAppendSession<'a> {
    source: LispStringSourceCursor,
    source_state: DisplayRowSourceState,
    base_face_id: FaceId,
    base_face: &'a ResolvedFace,
}

impl<'a> LispStringSourceAppendSession<'a> {
    fn new(request: LispStringSourceAppendSessionRequest<'a>) -> Option<Self> {
        let source = request.append_request.into_source(request.base_face_id)?;
        Some(Self {
            source,
            source_state: DisplayRowSourceState::default(),
            base_face_id: request.base_face_id,
            base_face: request.base_face,
        })
    }

    fn render_to_text_row_and_emit(
        &mut self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
    ) -> Option<CurrentTextRowRenderOutcome> {
        render_lisp_string_source_append_to_text_row_and_emit(
            state,
            &mut self.source,
            &mut self.source_state,
            self.base_face,
            self.base_face_id,
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
enum BufferAnchoredLispStringSourceKind {
    Prefix(DisplayRowPrefixKind),
}

#[derive(Clone, Copy)]
struct BufferAnchoredLispStringSource {
    value: Value,
    anchor_charpos: CharPos0,
    kind: BufferAnchoredLispStringSourceKind,
}

#[derive(Clone, Copy)]
pub(crate) struct DisplayRowPrefixSource {
    source: BufferAnchoredLispStringSource,
}

impl DisplayRowPrefixRequest {
    pub(crate) fn initial(_has_prefix: bool, _has_line_prefix: bool) -> Self {
        // The first visible row is a line start: request the line prefix so its
        // per-row `line-prefix` TEXT PROPERTY is consulted even when the buffer
        // sets no `line-prefix` variable (e.g. org-indent virtual indentation).
        // The text-property read returns None for unprefixed lines (cheap).
        Self::Line
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
        _has_prefix: bool,
        action: TextRowTransitionPrefixAction,
    ) {
        // Always request the prefix at a row transition so the per-row
        // `line-prefix`/`wrap-prefix` TEXT PROPERTY is consulted (GNU checks it
        // per line). The variable (`_has_prefix`) is only a default; properties
        // like org-indent's virtual indentation set the property without setting
        // the variable. The text-property read returns None for unprefixed lines
        // (cheap) and `source_from_values` falls back to the variable default.
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
            source: BufferAnchoredLispStringSource::prefix(value, anchor_charpos, kind),
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

impl BufferAnchoredLispStringSource {
    fn prefix(value: Value, anchor_charpos: CharPos0, kind: DisplayRowPrefixKind) -> Self {
        Self {
            value,
            anchor_charpos,
            kind: BufferAnchoredLispStringSourceKind::Prefix(kind),
        }
    }

    #[cfg(test)]
    fn value(self) -> Value {
        self.value
    }

    fn origin(self) -> DisplayOrigin {
        match self.kind {
            BufferAnchoredLispStringSourceKind::Prefix(DisplayRowPrefixKind::Line) => {
                DisplayOrigin::LinePrefix {
                    anchor_charpos: self.anchor_charpos,
                }
            }
            BufferAnchoredLispStringSourceKind::Prefix(DisplayRowPrefixKind::Wrap) => {
                DisplayOrigin::WrapPrefix {
                    anchor_charpos: self.anchor_charpos,
                }
            }
        }
    }

    #[cfg(test)]
    fn base_face_policy(self) -> BaseFacePolicy {
        self.origin().default_base_face_policy()
    }

    fn source_id(self) -> LispStringSourceId {
        match self.kind {
            BufferAnchoredLispStringSourceKind::Prefix(_) => LispStringSourceId::PREFIX,
        }
    }

    fn append_request(self, position: DisplayRowPosition) -> LispStringSourceAppendRequest {
        LispStringSourceAppendRequest::new(position, self.source_id(), self.value)
    }
}

impl DisplayRowPrefixSource {
    #[cfg(test)]
    pub(crate) fn value(self) -> Value {
        self.source.value()
    }

    pub(crate) fn origin(self) -> DisplayOrigin {
        self.source.origin()
    }

    #[cfg(test)]
    pub(crate) fn base_face_policy(self) -> BaseFacePolicy {
        self.source.base_face_policy()
    }

    pub(crate) fn append_request(
        self,
        position: DisplayRowPosition,
    ) -> LispStringSourceAppendRequest {
        self.source.append_request(position)
    }
}

pub(crate) struct BufferLinePrefixRenderRequest<'a> {
    values: DisplayRowPrefixValues,
    append_surface: &'a DisplayRowAppendSurface,
    row_geometry: &'a DisplayRowGeometryState,
    active_face_state: &'a DisplayRowActiveFaceState,
    glyph_y_offset: f32,
    fallback_metrics: DisplayRowFallbackMetrics,
    position: DisplayRowPosition,
}

impl<'a> BufferLinePrefixRenderRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        values: DisplayRowPrefixValues,
        append_surface: &'a DisplayRowAppendSurface,
        row_geometry: &'a DisplayRowGeometryState,
        active_face_state: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
        position: DisplayRowPosition,
    ) -> Self {
        Self {
            values,
            append_surface,
            row_geometry,
            active_face_state,
            glyph_y_offset,
            fallback_metrics,
            position,
        }
    }

    pub(crate) fn render_requested_to_text_row_and_emit<B: LayoutBufferView>(
        self,
        request: &mut DisplayRowPrefixRequest,
        state: &mut TextRowSourceRenderState<'_>,
        buffer: &B,
        anchor_charpos: i64,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> DisplayRowPosition {
        let position = self.position;
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

        let prefix_base_face =
            state.default_display_string_base_face(buffer, prefix_source.origin(), face_ids);
        LispStringRowAppendContext::new(
            self.append_surface,
            self.row_geometry,
            self.active_face_state,
            self.glyph_y_offset,
            self.fallback_metrics,
        )
        .render_prefix_source_to_text_row_and_emit(
            state,
            face_ids,
            &prefix_base_face,
            prefix_source,
            position,
        )
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
        let position = self.render_requested_to_text_row_and_emit(
            request,
            source_render,
            buffer,
            anchor_charpos,
            face_ids,
        );
        *x = position.x_px();
        *col = position.col();
    }
}

pub(crate) struct LispStringSourceRowAppendSession<'a> {
    source_session: LispStringSourceAppendSession<'a>,
    append_surface: &'a DisplayRowAppendSurface,
    glyph_y_offset: f32,
    metrics: DisplayRowAppendMetrics,
}

pub(crate) struct LispStringSourceRowAppendSessionRequest<'a> {
    source_request: LispStringSourceAppendSessionRequest<'a>,
    append_surface: &'a DisplayRowAppendSurface,
    glyph_y_offset: f32,
    metrics: DisplayRowAppendMetrics,
}

impl<'a> LispStringSourceRowAppendSessionRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        source_request: LispStringSourceAppendSessionRequest<'a>,
        append_surface: &'a DisplayRowAppendSurface,
        glyph_y_offset: f32,
        height: f32,
        ascent: f32,
        char_width: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
    ) -> Self {
        Self {
            source_request,
            append_surface,
            glyph_y_offset,
            metrics: DisplayRowAppendMetrics::text_row(
                height,
                ascent,
                char_width,
                fallback_metrics,
            ),
        }
    }
}

impl<'a> LispStringSourceRowAppendSession<'a> {
    pub(crate) fn new(request: LispStringSourceRowAppendSessionRequest<'a>) -> Option<Self> {
        let source_session = LispStringSourceAppendSession::new(request.source_request)?;
        Some(Self {
            source_session,
            append_surface: request.append_surface,
            glyph_y_offset: request.glyph_y_offset,
            metrics: request.metrics,
        })
    }

    pub(crate) fn render_to_text_row_and_emit(
        &mut self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        geometry: &DisplayRowGeometryState,
        position: DisplayRowPosition,
    ) -> Option<CurrentTextRowRenderOutcome> {
        let frame = self
            .metrics
            .text_row_frame(self.append_surface, geometry, self.glyph_y_offset);
        self.source_session
            .render_to_text_row_and_emit(state, face_ids, frame, position)
    }

    pub(crate) fn discard_pending_until_row_break(&mut self) -> bool {
        self.source_session.discard_pending_until_row_break()
    }
}

#[cfg(test)]
pub(crate) fn apply_pending_display_source_faces(
    builder: &mut DisplayOutputBuilder,
    pending_faces: &mut Vec<PendingDisplaySourceFace>,
) {
    for pending in pending_faces.drain(..) {
        let (face_id, resolved) = pending.into_parts();
        install_output_resolved_face(builder, face_id, &resolved, None);
    }
}

#[cfg(test)]
pub(crate) fn append_lisp_string_to_text_row(
    state: &mut TextRowSourceRenderState<'_>,
    text_value: Value,
    source_id: u64,
    base_face: &ResolvedFace,
    base_face_id: FaceId,
    face_ids: &mut FrameFaceIdAllocator,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> DisplayRowPosition {
    let request =
        LispStringSourceAppendRequest::new(position, LispStringSourceId(source_id), text_value);
    let session_request =
        LispStringSourceAppendSessionRequest::new(request, base_face_id, base_face);
    let Some(mut source_session) = LispStringSourceAppendSession::new(session_request) else {
        return position;
    };
    source_session
        .render_to_text_row_and_emit(state, face_ids, frame, position)
        .map(|outcome| outcome.end_position())
        .unwrap_or(position)
}
