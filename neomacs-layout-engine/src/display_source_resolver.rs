use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_face_layout::{DisplayHeightFaceBasis, height_adjusted_face};
use crate::display_face_policy::BaseFacePolicy;
use crate::display_item::{DisplayItem, DisplayMediaReplacement, RenderFaceRef};
use crate::display_media::{DisplayMediaResolveParams, resolve_display_media_property};
use crate::display_origin::DisplayOrigin;
use crate::display_property::{
    DisplayMediaReplacementProperty, DisplayPropertyClassification, DisplayReplacementProperty,
};
use crate::display_row::{DisplayRowActiveFaceState, DisplayRowFallbackMetrics};
use crate::display_row_builder::DisplayRowPosition;
use crate::display_row_replacement::DisplayPropertyReplacementAppendRequest;
use crate::display_source::{
    BufferDisplayReplacementSource, DisplayPropertyReplacementSourceInputs,
    DisplayPropertyReplacementSourceItem, DisplayPropertyReplacementSourceMetrics,
    DisplayReplacementMediaSourceItem, DisplayReplacementMediaSourceResolution,
    DisplayReplacementSourceMappedTextItem,
};
use crate::display_source::{DisplayItemFaceResolver, DisplayItemSource, DisplaySourceContext};
use crate::font_metrics::FontMetricsService;
use crate::neovm_bridge::{FaceResolver, LayoutBufferView, ResolvedFace};
use crate::types::WindowParams;
use crate::unicode::decode_utf8;
use neomacs_display_protocol::face::BasicFaceId;
use neovm_core::buffer::CharPos0;
use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::eval::DisplayHost;
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub(crate) struct DisplaySourceFaceBasis<'a> {
    face_resolver: &'a FaceResolver,
    base_face_id: u32,
    base_face: &'a ResolvedFace,
    canonical_face: &'a ResolvedFace,
    fallback_metrics: DisplayRowFallbackMetrics,
}

impl<'a> DisplaySourceFaceBasis<'a> {
    pub(crate) fn new(
        face_resolver: &'a FaceResolver,
        base_face_id: u32,
        base_face: &'a ResolvedFace,
        fallback_metrics: DisplayRowFallbackMetrics,
    ) -> Self {
        Self {
            face_resolver,
            base_face_id,
            base_face,
            canonical_face: face_resolver.default_face(),
            fallback_metrics,
        }
    }

    pub(crate) fn face_resolver(self) -> &'a FaceResolver {
        self.face_resolver
    }

    pub(crate) fn base_face_id(self) -> u32 {
        self.base_face_id
    }

    pub(crate) fn base_face(self) -> &'a ResolvedFace {
        self.base_face
    }

    pub(crate) fn canonical_face(self) -> &'a ResolvedFace {
        self.canonical_face
    }

    pub(crate) fn fallback_metrics(self) -> DisplayRowFallbackMetrics {
        self.fallback_metrics
    }

    fn height_basis(self) -> DisplayHeightFaceBasis<'a> {
        let fallback = self.fallback_metrics();
        DisplayHeightFaceBasis {
            canonical_face: self.canonical_face(),
            base_face: self.base_face(),
            fallback_char_width: fallback.char_width(),
            fallback_ascent: fallback.ascent(),
            fallback_row_height: fallback.row_height(),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DisplaySourceResolveParams<'a> {
    face_basis: DisplaySourceFaceBasis<'a>,
    display_host: Option<&'a dyn DisplayHost>,
}

impl<'a> DisplaySourceResolveParams<'a> {
    pub(crate) fn new(
        face_basis: DisplaySourceFaceBasis<'a>,
        display_host: Option<&'a dyn DisplayHost>,
    ) -> Self {
        Self {
            face_basis,
            display_host,
        }
    }

    pub(crate) fn face_basis(self) -> DisplaySourceFaceBasis<'a> {
        self.face_basis
    }

    fn display_host(self) -> Option<&'a dyn DisplayHost> {
        self.display_host
    }
}

#[derive(Default)]
pub(crate) struct DisplaySourceResolveState {
    face_cache: HashMap<Value, u32>,
    height_face_cache: HashMap<DisplayHeightFaceKey, u32>,
    resolved_faces: HashMap<u32, ResolvedFace>,
}

impl DisplaySourceResolveState {
    pub(crate) fn remember_face(&mut self, face_id: u32, face: &ResolvedFace) {
        self.resolved_faces.insert(face_id, face.clone());
    }

    pub(crate) fn resolved_face(&self, face_id: u32) -> Option<&ResolvedFace> {
        self.resolved_faces.get(&face_id)
    }

    fn cached_face(&self, face_value: &Value) -> Option<RenderFaceRef> {
        self.face_cache
            .get(face_value)
            .copied()
            .map(RenderFaceRef::FaceId)
    }

    fn cache_face(&mut self, face_value: Value, face_id: u32, resolved: &ResolvedFace) {
        self.face_cache.insert(face_value, face_id);
        self.remember_face(face_id, resolved);
    }

    fn resolved_face_for(&self, face: RenderFaceRef, base_face: &ResolvedFace) -> ResolvedFace {
        let RenderFaceRef::FaceId(face_id) = face else {
            return base_face.clone();
        };
        self.resolved_faces
            .get(&face_id)
            .cloned()
            .unwrap_or_else(|| base_face.clone())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct DisplayHeightFaceKey {
    base_face_id: u32,
    factor_bits: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct PendingDisplaySourceFace {
    pub(crate) face_id: u32,
    pub(crate) resolved: ResolvedFace,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayDefaultFaceInstallPolicy {
    InstallDefaultFace,
    ReuseInstalledDefaultFace,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ActiveDisplayStringBaseFace<'a> {
    face_id: u32,
    resolved: &'a ResolvedFace,
}

impl<'a> ActiveDisplayStringBaseFace<'a> {
    pub(crate) fn new(face_id: u32, resolved: &'a ResolvedFace) -> Self {
        Self { face_id, resolved }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DisplayStringBaseFace {
    face: ResolvedFace,
    face_id: u32,
    pending_face: Option<PendingDisplaySourceFace>,
}

impl DisplayStringBaseFace {
    pub(crate) fn face(&self) -> &ResolvedFace {
        &self.face
    }

    pub(crate) fn face_id(&self) -> u32 {
        self.face_id
    }

    pub(crate) fn pending_face(&self) -> Option<&PendingDisplaySourceFace> {
        self.pending_face.as_ref()
    }
}

pub(crate) fn resolve_display_string_base_face<B: LayoutBufferView>(
    buffer: &B,
    face_resolver: &FaceResolver,
    origin: DisplayOrigin,
    policy: BaseFacePolicy,
    active_base_face: Option<ActiveDisplayStringBaseFace<'_>>,
    default_install_policy: DisplayDefaultFaceInstallPolicy,
    face_ids: &mut FrameFaceIdAllocator,
) -> DisplayStringBaseFace {
    let mut next_check = buffer.layout_point_max_char_pos().get();
    let face = face_resolver.base_face_for_origin(Some(buffer), &origin, policy, &mut next_check);

    let (face_id, pending_face) = if let Some(active_base_face) = active_base_face
        && same_resolved_face(&face, active_base_face.resolved)
    {
        (active_base_face.face_id, None)
    } else if same_resolved_face(&face, face_resolver.default_face()) {
        let face_id = u32::from(BasicFaceId::Default);
        let pending_face = match default_install_policy {
            DisplayDefaultFaceInstallPolicy::InstallDefaultFace => Some(PendingDisplaySourceFace {
                face_id,
                resolved: face.clone(),
            }),
            DisplayDefaultFaceInstallPolicy::ReuseInstalledDefaultFace => None,
        };
        (face_id, pending_face)
    } else {
        let face_id = face_ids.allocate();
        let pending_face = Some(PendingDisplaySourceFace {
            face_id,
            resolved: face.clone(),
        });
        (face_id, pending_face)
    };

    DisplayStringBaseFace {
        face,
        face_id,
        pending_face,
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedDisplaySourceItem {
    pub(crate) item: Option<DisplayItem>,
    pub(crate) pending_faces: Vec<PendingDisplaySourceFace>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ResolvedDisplayReplacement {
    Media(DisplayMediaReplacement),
    Placeholder(&'static str),
}

fn resolved_media_replacement(geometry: DisplayMediaReplacement) -> ResolvedDisplayReplacement {
    ResolvedDisplayReplacement::Media(geometry)
}

pub(crate) fn resolve_display_replacement(
    display_prop: Value,
    replacement: &DisplayMediaReplacementProperty,
    display_host: Option<&dyn DisplayHost>,
    resolved_face: &ResolvedFace,
    fallback_char_width: f32,
    fallback_row_height: f32,
) -> Option<ResolvedDisplayReplacement> {
    if let Some(media) = replacement.direct_replacement() {
        return Some(resolved_media_replacement(media));
    }

    if let Some(media) = resolve_display_property_media(
        &display_prop,
        display_host,
        resolved_face,
        fallback_char_width,
        fallback_row_height,
    )
    .filter(|media| replacement.accepts_media_replacement(media))
    {
        return Some(resolved_media_replacement(media));
    }

    replacement
        .media_fallback_placeholder()
        .map(ResolvedDisplayReplacement::Placeholder)
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
}

pub(crate) struct DisplayPropertyReplacementSourceResolveRequest<'a, 'source> {
    display_property: &'a DisplayPropertyClassification,
    replacement_value: Value,
    anchor_charpos: CharPos0,
    source_text: &'source [u8],
    active_face_state: &'a DisplayRowActiveFaceState,
    font_metrics: &'a mut Option<FontMetricsService>,
    current_x: f32,
    content_x: f32,
    params: &'a WindowParams,
    display_host: Option<&'a dyn DisplayHost>,
}

impl<'a, 'source> DisplayPropertyReplacementSourceResolveRequest<'a, 'source> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_typed_replacement(
        display_property: &'a DisplayPropertyClassification,
        replacement_value: Value,
        anchor_charpos: CharPos0,
        source_text: &'source [u8],
        active_face_state: &'a DisplayRowActiveFaceState,
        font_metrics: &'a mut Option<FontMetricsService>,
        current_x: f32,
        content_x: f32,
        params: &'a WindowParams,
        display_host: Option<&'a dyn DisplayHost>,
    ) -> Self {
        Self {
            display_property,
            replacement_value,
            anchor_charpos,
            source_text,
            active_face_state,
            font_metrics,
            current_x,
            content_x,
            params,
            display_host,
        }
    }

    fn face_metrics(&self) -> crate::display_row::DisplayRowMeasuredFaceMetrics {
        self.active_face_state.metrics()
    }

    pub(crate) fn resolve(self) -> Option<DisplayPropertyReplacementSourceItem> {
        let display_property = self.display_property;
        let replacement_value = self.replacement_value;
        let anchor_charpos = self.anchor_charpos;
        let source_text = self.source_text;
        let face_metrics = self.face_metrics();
        let source_metrics = DisplayPropertyReplacementSourceMetrics::new(
            face_metrics.char_width,
            face_metrics.row_height,
            face_metrics.ascent,
        );
        let source_inputs = match display_property.replacement()? {
            DisplayReplacementProperty::String => {
                let replacement = replacement_value.as_utf8_str()?;
                let cursor_slot_width_px = replacement
                    .chars()
                    .next()
                    .map(|ch| {
                        self.active_face_state.advance_for_char(
                            self.font_metrics,
                            ch,
                            face_metrics.char_width,
                        )
                    })
                    .unwrap_or_else(|| face_metrics.char_width.max(1.0));
                DisplayPropertyReplacementSourceInputs::empty()
                    .with_string_cursor_slot_width_px(cursor_slot_width_px)
            }
            DisplayReplacementProperty::Stretch(_) => {
                let (display_ch, _) = decode_utf8(source_text);
                let display_char_width = self.active_face_state.advance_for_char(
                    self.font_metrics,
                    display_ch,
                    face_metrics.char_width,
                );
                DisplayPropertyReplacementSourceInputs::empty()
                    .with_stretch_display_char_width_px(display_char_width)
            }
            DisplayReplacementProperty::Media(media_replacement) => {
                let media = DisplayReplacementMediaSourceItem::resolve_display_property(
                    replacement_value,
                    media_replacement,
                    self.display_host,
                    self.active_face_state,
                    face_metrics.char_width,
                    face_metrics.row_height,
                )?;
                DisplayPropertyReplacementSourceInputs::empty().with_media(media)
            }
        };
        DisplayPropertyReplacementSourceItem::from_display_property_parts(
            display_property,
            replacement_value,
            anchor_charpos,
            self.current_x,
            self.content_x,
            self.params,
            source_metrics,
            source_inputs,
        )
    }
}

pub(crate) struct DisplayPropertyReplacementAppendRequestResolver<'a, 'source> {
    display_property: &'a DisplayPropertyClassification,
    replacement_source: BufferDisplayReplacementSource,
    replacement_value: Value,
    anchor_charpos: CharPos0,
    source_text: &'source [u8],
    active_face_state: &'a DisplayRowActiveFaceState,
    current_x: f32,
    content_x: f32,
    params: &'a WindowParams,
    glyph_y_offset: f32,
    default_row_height: f32,
    start_position: DisplayRowPosition,
}

impl<'a, 'source> DisplayPropertyReplacementAppendRequestResolver<'a, 'source> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_typed_replacement(
        display_property: &'a DisplayPropertyClassification,
        replacement_source: BufferDisplayReplacementSource,
        replacement_value: Value,
        anchor_charpos: CharPos0,
        source_text: &'source [u8],
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
            replacement_source,
            replacement_value,
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

    pub(crate) fn resolve(
        self,
        font_metrics: &mut Option<FontMetricsService>,
        display_host: Option<&dyn DisplayHost>,
    ) -> Option<DisplayPropertyReplacementAppendRequest> {
        let item = DisplayPropertyReplacementSourceResolveRequest::from_typed_replacement(
            self.display_property,
            self.replacement_value,
            self.anchor_charpos,
            self.source_text,
            self.active_face_state,
            font_metrics,
            self.current_x,
            self.content_x,
            self.params,
            display_host,
        )
        .resolve()?;
        Some(DisplayPropertyReplacementAppendRequest::new(
            self.replacement_source,
            item,
            self.glyph_y_offset,
            self.default_row_height,
            self.start_position,
        ))
    }
}

pub(crate) struct DisplaySourcePropertyResolver<'a> {
    params: DisplaySourceResolveParams<'a>,
    state: &'a mut DisplaySourceResolveState,
    face_ids: &'a mut FrameFaceIdAllocator,
    pending_faces: &'a mut Vec<PendingDisplaySourceFace>,
}

impl<'a> DisplaySourcePropertyResolver<'a> {
    pub(crate) fn new(
        params: DisplaySourceResolveParams<'a>,
        state: &'a mut DisplaySourceResolveState,
        face_ids: &'a mut FrameFaceIdAllocator,
        pending_faces: &'a mut Vec<PendingDisplaySourceFace>,
    ) -> Self {
        let face_basis = params.face_basis();
        state.remember_face(face_basis.base_face_id(), face_basis.base_face());
        Self {
            params,
            state,
            face_ids,
            pending_faces,
        }
    }

    fn resolve_item_layout(&mut self, mut item: DisplayItem) -> DisplayItem {
        if let Some(factor) = item
            .layout
            .height
            .filter(|factor| factor.is_finite() && *factor > 0.0)
        {
            item.face = self.resolve_height_face_ref(item.face, factor);
        }
        item
    }

    fn resolve_height_face_ref(&mut self, face: RenderFaceRef, factor: f32) -> RenderFaceRef {
        let face_basis = self.params.face_basis();
        let fallback = face_basis.fallback_metrics();
        if fallback.char_width() <= 1.0 && fallback.row_height() <= 1.0 {
            return face;
        }

        let base_face_id = match face {
            RenderFaceRef::FaceId(face_id) => face_id,
            RenderFaceRef::Inherit => face_basis.base_face_id(),
        };
        let key = DisplayHeightFaceKey {
            base_face_id,
            factor_bits: factor.to_bits(),
        };
        if let Some(face_id) = self.state.height_face_cache.get(&key).copied() {
            return RenderFaceRef::FaceId(face_id);
        }

        let source = self.state.resolved_face_for(face, face_basis.base_face());
        let Some(resolved) = height_adjusted_face(&source, face_basis.height_basis(), factor)
        else {
            return face;
        };
        if same_resolved_face(&resolved, &source) {
            return face;
        }

        let face_id = self.face_ids.allocate();
        self.state.height_face_cache.insert(key, face_id);
        self.state.remember_face(face_id, &resolved);
        self.pending_faces
            .push(PendingDisplaySourceFace { face_id, resolved });
        RenderFaceRef::FaceId(face_id)
    }
}

impl DisplayItemFaceResolver for DisplaySourcePropertyResolver<'_> {
    fn resolve_face_ref(&mut self, base: RenderFaceRef, face_value: Value) -> RenderFaceRef {
        let face_basis = self.params.face_basis();
        if let Some(cached) = self.state.cached_face(&face_value) {
            return cached;
        }
        let Some(resolved) = face_basis
            .face_resolver()
            .resolve_face_value_over(face_basis.base_face(), &face_value)
        else {
            return base;
        };

        if same_resolved_face(&resolved, face_basis.base_face()) {
            self.state.cache_face(
                face_value,
                face_basis.base_face_id(),
                face_basis.base_face(),
            );
            return RenderFaceRef::FaceId(face_basis.base_face_id());
        }

        let face_id = self.face_ids.allocate();
        self.state.cache_face(face_value, face_id, &resolved);
        self.pending_faces
            .push(PendingDisplaySourceFace { face_id, resolved });
        RenderFaceRef::FaceId(face_id)
    }

    fn resolve_display_media_replacement(
        &mut self,
        display_prop: Value,
        face: RenderFaceRef,
    ) -> Option<DisplayMediaReplacement> {
        let face_basis = self.params.face_basis();
        let fallback = face_basis.fallback_metrics();
        let resolved_face = self.state.resolved_face_for(face, face_basis.base_face());
        resolve_display_property_media(
            &display_prop,
            self.params.display_host(),
            &resolved_face,
            fallback.char_width(),
            fallback.row_height(),
        )
    }
}

pub(crate) fn resolve_next_display_source_item(
    source: &mut impl DisplayItemSource,
    params: DisplaySourceResolveParams<'_>,
    state: &mut DisplaySourceResolveState,
    face_ids: &mut FrameFaceIdAllocator,
) -> ResolvedDisplaySourceItem {
    let mut pending_faces = Vec::new();
    let item = {
        let mut resolver =
            DisplaySourcePropertyResolver::new(params, state, face_ids, &mut pending_faces);
        let mut context = DisplaySourceContext::with_face_resolver(&mut resolver);
        source
            .next_item(&mut context)
            .map(|item| resolver.resolve_item_layout(item))
    };
    ResolvedDisplaySourceItem {
        item,
        pending_faces,
    }
}

pub(crate) fn resolve_display_property_media(
    display_prop: &Value,
    display_host: Option<&dyn DisplayHost>,
    resolved_face: &ResolvedFace,
    fallback_char_width: f32,
    fallback_row_height: f32,
) -> Option<DisplayMediaReplacement> {
    resolve_display_media_property(
        display_prop,
        DisplayMediaResolveParams {
            display_host: display_host?,
            default_fg: resolved_face.fg,
            default_bg: resolved_face.bg,
            fallback_char_width,
            fallback_row_height,
        },
    )
}

pub(crate) fn same_resolved_face(lhs: &ResolvedFace, rhs: &ResolvedFace) -> bool {
    lhs.fg == rhs.fg
        && lhs.bg == rhs.bg
        && lhs.font_family == rhs.font_family
        && lhs.font_weight == rhs.font_weight
        && lhs.italic == rhs.italic
        && (lhs.font_size - rhs.font_size).abs() <= f32::EPSILON
        && lhs.underline_style == rhs.underline_style
        && lhs.underline_color == rhs.underline_color
        && lhs.strike_through == rhs.strike_through
        && lhs.strike_through_color == rhs.strike_through_color
        && lhs.overline == rhs.overline
        && lhs.overline_color == rhs.overline_color
        && lhs.box_type == rhs.box_type
        && lhs.box_color == rhs.box_color
        && lhs.box_line_width == rhs.box_line_width
        && lhs.extend == rhs.extend
        && lhs.terminal_inverse_video == rhs.terminal_inverse_video
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display_item::DisplayXwidgetItem;
    use crate::neovm_bridge::LayoutBufferSnapshot;
    use neovm_core::buffer::CharPos0;
    use neovm_core::emacs_core::Context;
    use neovm_core::face::FaceTable;

    fn test_buffer_snapshot() -> LayoutBufferSnapshot {
        let mut context = Context::new();
        let buf_id = context
            .buffer_manager()
            .current_buffer()
            .expect("current buffer")
            .id();
        {
            let buffer = context
                .buffer_manager_mut()
                .get_mut(buf_id)
                .expect("current buffer");
            buffer.insert("abc");
            buffer.widen();
        }
        let buffer = context
            .buffer_manager()
            .get(buf_id)
            .expect("current buffer");
        LayoutBufferSnapshot::from_buffer(buffer)
    }

    fn test_face_resolver(table: &FaceTable) -> FaceResolver {
        FaceResolver::new(table, 0x00ffffff, 0x000000, 14.0, None)
    }

    #[test]
    fn display_string_base_face_reuses_active_face_before_default_policy() {
        let buffer = test_buffer_snapshot();
        let table = FaceTable::new();
        let resolver = test_face_resolver(&table);
        let mut face_ids = FrameFaceIdAllocator::new(BasicFaceId::SENTINEL);

        let base_face = resolve_display_string_base_face(
            &buffer,
            &resolver,
            DisplayOrigin::LinePrefix {
                anchor_charpos: CharPos0::new(0),
            },
            BaseFacePolicy::DefaultFace,
            Some(ActiveDisplayStringBaseFace::new(
                500,
                resolver.default_face(),
            )),
            DisplayDefaultFaceInstallPolicy::InstallDefaultFace,
            &mut face_ids,
        );

        assert_eq!(base_face.face_id(), 500);
        assert!(base_face.pending_face().is_none());
        assert!(same_resolved_face(
            base_face.face(),
            resolver.default_face()
        ));
    }

    #[test]
    fn display_string_base_face_default_policy_controls_pending_face() {
        let buffer = test_buffer_snapshot();
        let table = FaceTable::new();
        let resolver = test_face_resolver(&table);
        let mut install_face_ids = FrameFaceIdAllocator::new(BasicFaceId::SENTINEL);
        let mut reuse_face_ids = FrameFaceIdAllocator::new(BasicFaceId::SENTINEL);

        let installed = resolve_display_string_base_face(
            &buffer,
            &resolver,
            DisplayOrigin::LinePrefix {
                anchor_charpos: CharPos0::new(0),
            },
            BaseFacePolicy::DefaultFace,
            None,
            DisplayDefaultFaceInstallPolicy::InstallDefaultFace,
            &mut install_face_ids,
        );
        let reused = resolve_display_string_base_face(
            &buffer,
            &resolver,
            DisplayOrigin::LinePrefix {
                anchor_charpos: CharPos0::new(0),
            },
            BaseFacePolicy::DefaultFace,
            None,
            DisplayDefaultFaceInstallPolicy::ReuseInstalledDefaultFace,
            &mut reuse_face_ids,
        );

        assert_eq!(installed.face_id(), u32::from(BasicFaceId::Default));
        assert!(installed.pending_face().is_some());
        assert_eq!(reused.face_id(), u32::from(BasicFaceId::Default));
        assert!(reused.pending_face().is_none());
    }

    #[test]
    fn display_string_base_face_allocates_pending_face_for_dynamic_source_face() {
        let buffer = test_buffer_snapshot();
        let table = FaceTable::new();
        let resolver = test_face_resolver(&table);
        let mut face_ids = FrameFaceIdAllocator::new(500);

        let base_face = resolve_display_string_base_face(
            &buffer,
            &resolver,
            DisplayOrigin::ModeLine { selected: true },
            BaseFacePolicy::FixedBasicFace(BasicFaceId::ModeLineActive),
            None,
            DisplayDefaultFaceInstallPolicy::ReuseInstalledDefaultFace,
            &mut face_ids,
        );

        assert_eq!(base_face.face_id(), 500);
        let pending_face = base_face.pending_face().expect("pending face");
        assert_eq!(pending_face.face_id, 500);
        assert!(same_resolved_face(&pending_face.resolved, base_face.face()));
        assert_eq!(face_ids.finish(), 501);
    }

    #[test]
    fn resolve_display_replacement_returns_direct_xwidget_media() {
        let table = FaceTable::new();
        let resolver = test_face_resolver(&table);
        let xwidget = DisplayXwidgetItem {
            xwidget_id: 42,
            width: 120.0,
            height: 36.0,
        };
        let media = DisplayMediaReplacement::xwidget(xwidget);

        let resolved = resolve_display_replacement(
            Value::NIL,
            &DisplayMediaReplacementProperty::Xwidget(media),
            None,
            resolver.default_face(),
            8.0,
            16.0,
        );

        assert_eq!(resolved, Some(ResolvedDisplayReplacement::Media(media)));
    }

    #[test]
    fn resolve_display_replacement_uses_media_placeholder_without_host() {
        let table = FaceTable::new();
        let resolver = test_face_resolver(&table);

        let resolved = resolve_display_replacement(
            Value::NIL,
            &DisplayMediaReplacementProperty::Image,
            None,
            resolver.default_face(),
            8.0,
            16.0,
        );

        assert_eq!(
            resolved,
            Some(ResolvedDisplayReplacement::Placeholder("[img]"))
        );
    }
}
