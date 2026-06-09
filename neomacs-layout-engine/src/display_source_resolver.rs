use crate::display_item::{DisplayItemKind, RenderFaceRef};
use crate::display_media::{DisplayMediaResolveParams, resolve_display_media_property};
use crate::display_source::DisplayItemFaceResolver;
use crate::neovm_bridge::{FaceResolver, ResolvedFace};
use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::eval::DisplayHost;
use std::collections::HashMap;

pub(crate) struct DisplaySourceResolveParams<'a> {
    pub(crate) face_resolver: &'a FaceResolver,
    pub(crate) display_host: Option<&'a dyn DisplayHost>,
    pub(crate) base_face: &'a ResolvedFace,
    pub(crate) base_face_id: u32,
    pub(crate) fallback_char_width: f32,
    pub(crate) fallback_row_height: f32,
}

#[derive(Default)]
pub(crate) struct DisplaySourceResolveState {
    face_cache: HashMap<Value, u32>,
    resolved_faces: HashMap<u32, ResolvedFace>,
}

impl DisplaySourceResolveState {
    pub(crate) fn remember_face(&mut self, face_id: u32, face: &ResolvedFace) {
        self.resolved_faces.insert(face_id, face.clone());
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

#[derive(Clone, Debug)]
pub(crate) struct PendingDisplaySourceFace {
    pub(crate) face_id: u32,
    pub(crate) resolved: ResolvedFace,
}

pub(crate) struct DisplaySourcePropertyResolver<'a> {
    params: DisplaySourceResolveParams<'a>,
    state: &'a mut DisplaySourceResolveState,
    next_face_id: &'a mut u32,
    pending_faces: &'a mut Vec<PendingDisplaySourceFace>,
}

impl<'a> DisplaySourcePropertyResolver<'a> {
    pub(crate) fn new(
        params: DisplaySourceResolveParams<'a>,
        state: &'a mut DisplaySourceResolveState,
        next_face_id: &'a mut u32,
        pending_faces: &'a mut Vec<PendingDisplaySourceFace>,
    ) -> Self {
        state.remember_face(params.base_face_id, params.base_face);
        Self {
            params,
            state,
            next_face_id,
            pending_faces,
        }
    }
}

impl DisplayItemFaceResolver for DisplaySourcePropertyResolver<'_> {
    fn resolve_face_ref(&mut self, base: RenderFaceRef, face_value: Value) -> RenderFaceRef {
        if let Some(cached) = self.state.cached_face(&face_value) {
            return cached;
        }
        let Some(resolved) = self
            .params
            .face_resolver
            .resolve_face_value_over(self.params.base_face, &face_value)
        else {
            return base;
        };

        if same_resolved_face(&resolved, self.params.base_face) {
            self.state
                .cache_face(face_value, self.params.base_face_id, self.params.base_face);
            return RenderFaceRef::FaceId(self.params.base_face_id);
        }

        let face_id = *self.next_face_id;
        *self.next_face_id += 1;
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
        let display_host = self.params.display_host?;
        let resolved_face = self.state.resolved_face_for(face, self.params.base_face);
        resolve_display_media_property(
            &display_prop,
            DisplayMediaResolveParams {
                display_host,
                default_fg: resolved_face.fg,
                default_bg: resolved_face.bg,
                fallback_char_width: self.params.fallback_char_width,
                fallback_row_height: self.params.fallback_row_height,
            },
        )
    }
}

fn same_resolved_face(lhs: &ResolvedFace, rhs: &ResolvedFace) -> bool {
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
