use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_face_layout::{DisplayHeightFaceBasis, height_adjusted_face};
use crate::display_item::{DisplayItem, DisplayItemKind, RenderFaceRef};
use crate::display_media::{DisplayMediaResolveParams, resolve_display_media_property};
use crate::display_source::{DisplayItemFaceResolver, DisplayItemSource, DisplaySourceContext};
use crate::neovm_bridge::{FaceResolver, ResolvedFace};
use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::eval::DisplayHost;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplaySourceFallbackMetrics {
    char_width: f32,
    ascent: f32,
    row_height: f32,
}

impl DisplaySourceFallbackMetrics {
    pub(crate) fn new(char_width: f32, ascent: f32, row_height: f32) -> Self {
        Self {
            char_width,
            ascent,
            row_height,
        }
    }

    pub(crate) fn char_width(self) -> f32 {
        self.char_width
    }

    pub(crate) fn ascent(self) -> f32 {
        self.ascent
    }

    pub(crate) fn row_height(self) -> f32 {
        self.row_height
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DisplaySourceFaceBasis<'a> {
    face_resolver: &'a FaceResolver,
    base_face_id: u32,
    base_face: &'a ResolvedFace,
    canonical_face: &'a ResolvedFace,
    fallback_metrics: DisplaySourceFallbackMetrics,
}

impl<'a> DisplaySourceFaceBasis<'a> {
    pub(crate) fn new(
        face_resolver: &'a FaceResolver,
        base_face_id: u32,
        base_face: &'a ResolvedFace,
        fallback_metrics: DisplaySourceFallbackMetrics,
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

    pub(crate) fn fallback_metrics(self) -> DisplaySourceFallbackMetrics {
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

#[derive(Clone, Debug)]
pub(crate) struct ResolvedDisplaySourceItem {
    pub(crate) item: Option<DisplayItem>,
    pub(crate) pending_faces: Vec<PendingDisplaySourceFace>,
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

    fn resolve_display_property(
        &mut self,
        display_prop: Value,
        face: RenderFaceRef,
    ) -> Option<DisplayItemKind> {
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
) -> Option<DisplayItemKind> {
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
