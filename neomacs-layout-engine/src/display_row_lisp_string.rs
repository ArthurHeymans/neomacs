use crate::display_face_id::FrameFaceIdAllocator;
#[cfg(test)]
use crate::display_face_policy::BaseFacePolicy;
use crate::display_item::RenderFaceRef;
use crate::display_origin::DisplayOrigin;
#[cfg(test)]
use crate::display_output_builder::DisplayOutputBuilder;
use crate::display_row::{
    CurrentTextRowRenderOutcome, DisplayRowActiveFaceState, DisplayRowSourceState,
    NaturalDisplayRowAppendRenderPolicy,
};
use crate::display_row_append_context::{
    DisplayRowActiveFaceAppendContext, DisplayRowAppendFrame, DisplayRowAppendKind,
    DisplayRowAppendMetrics, DisplayRowAppendSurface, DisplayRowTextAppendContext,
};
use crate::display_row_builder::DisplayRowPosition;
use crate::display_row_geometry::DisplayRowGeometryState;
#[cfg(test)]
use crate::display_row_output_install::install_output_resolved_face;
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_row_walk_state::TextRowTransitionPrefixAction;
use crate::display_source::LispStringSourceCursor;
use crate::display_source_resolver::DisplayStringBaseFace;
#[cfg(test)]
use crate::display_source_resolver::PendingDisplaySourceFace;
use crate::neovm_bridge::ResolvedFace;
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
    base_face_id: u32,
    face_ids: &mut FrameFaceIdAllocator,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<CurrentTextRowRenderOutcome> {
    let (request, output) = frame.source_render_parts(
        position,
        base_face_id,
        base_face,
        DisplayRowAppendKind::SourceText,
    );
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    state.render_display_item_source_into_current_text_row_and_emit(
        face_ids,
        source,
        source_state,
        request,
        output,
        &mut render_policy,
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
pub(crate) struct LispStringSourceAppendRequest {
    pub(crate) position: DisplayRowPosition,
    pub(crate) source_id: LispStringSourceId,
    pub(crate) value: Value,
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
        }
    }

    fn into_source(self, base_face_id: u32) -> Option<LispStringSourceCursor> {
        LispStringSourceCursor::new(
            self.source_id.raw(),
            self.value,
            RenderFaceRef::FaceId(base_face_id),
        )
    }
}

pub(crate) struct LispStringSourceAppendSessionRequest<'a> {
    append_request: LispStringSourceAppendRequest,
    base_face_id: u32,
    base_face: &'a ResolvedFace,
}

impl<'a> LispStringSourceAppendSessionRequest<'a> {
    pub(crate) fn new(
        append_request: LispStringSourceAppendRequest,
        base_face_id: u32,
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
    base_face_id: u32,
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

pub(crate) struct LispStringSourceRowAppendSession<'a> {
    source_session: LispStringSourceAppendSession<'a>,
    append_surface: &'a DisplayRowAppendSurface,
    glyph_y_offset: f32,
    metrics: DisplayRowAppendMetrics,
}

impl<'a> LispStringSourceRowAppendSession<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        request: LispStringSourceAppendSessionRequest<'a>,
        append_surface: &'a DisplayRowAppendSurface,
        glyph_y_offset: f32,
        height: f32,
        ascent: f32,
        char_width: f32,
        default_row_height: f32,
    ) -> Option<Self> {
        let source_session = LispStringSourceAppendSession::new(request)?;
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

#[cfg(test)]
pub(crate) fn render_face_ref_id(face: RenderFaceRef, fallback: u32) -> u32 {
    match face {
        RenderFaceRef::FaceId(face_id) => face_id,
        RenderFaceRef::Inherit => fallback,
    }
}

#[cfg(test)]
pub(crate) fn apply_pending_display_source_faces(
    builder: &mut DisplayOutputBuilder,
    pending_faces: &mut Vec<PendingDisplaySourceFace>,
) {
    for pending in pending_faces.drain(..) {
        install_output_resolved_face(builder, pending.face_id, &pending.resolved, None);
    }
}

#[cfg(test)]
pub(crate) fn append_lisp_string_to_text_row(
    state: &mut TextRowSourceRenderState<'_>,
    text_value: Value,
    source_id: u64,
    base_face: &ResolvedFace,
    base_face_id: u32,
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
