//! Buffer source face and source-item layout resolution.
//!
//! This module resolves buffer source faces at scan checkpoints and prepares
//! display source items whose layout changes require derived measured faces.

use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_face_layout::{DisplayHeightFaceBasis, height_adjusted_face};
use crate::display_face_ref::render_face_ref_with_fallback;
use crate::display_item::{DisplayItem, RenderFaceRef};
use crate::display_origin::DisplayOrigin;
use crate::display_row_face_state::{DisplayRowActiveFaceState, DisplayRowMeasurementPolicy};
use crate::display_row_geometry::{DisplayRowGeometryState, DisplayRowScopedValue};
use crate::display_row_metrics::DisplayRowFallbackMetrics;
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_row_walk_state::{BoxFaceRowState, FaceScanCheckpoint};
use crate::display_source_resolver::{
    DisplaySourceFaceBasis, DisplaySourceResolveParams, PendingDisplaySourceFace,
};
use crate::neovm_bridge::{FaceResolver, LayoutBufferView, ResolvedFace};
use neomacs_display_protocol::face::BasicFaceId;
use neomacs_display_protocol::types::Color;
use neovm_core::emacs_core::eval::DisplayHost;

pub(crate) struct BufferSourceFaceResolutionContext<'a, B: LayoutBufferView> {
    buffer: &'a B,
    face_resolver: &'a FaceResolver,
    measurement_policy: DisplayRowMeasurementPolicy,
    default_resolved: &'a ResolvedFace,
    default_face_metrics: DisplayRowFallbackMetrics,
    window_metrics: DisplayRowFallbackMetrics,
    window_system: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferSourceItemLayoutResolutionContext<'a> {
    measurement_policy: DisplayRowMeasurementPolicy,
    default_resolved: &'a ResolvedFace,
    default_face_metrics: DisplayRowFallbackMetrics,
    window_metrics: DisplayRowFallbackMetrics,
    window_system: bool,
}

impl<'a> BufferSourceItemLayoutResolutionContext<'a> {
    pub(crate) fn new(
        measurement_policy: DisplayRowMeasurementPolicy,
        default_resolved: &'a ResolvedFace,
        default_face_metrics: DisplayRowFallbackMetrics,
        window_metrics: DisplayRowFallbackMetrics,
        window_system: bool,
    ) -> Self {
        Self {
            measurement_policy,
            default_resolved,
            default_face_metrics,
            window_metrics,
            window_system,
        }
    }
}

impl<'a, B: LayoutBufferView> Clone for BufferSourceFaceResolutionContext<'a, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, B: LayoutBufferView> Copy for BufferSourceFaceResolutionContext<'a, B> {}

impl<'a, B: LayoutBufferView> BufferSourceFaceResolutionContext<'a, B> {
    pub(crate) fn new(
        buffer: &'a B,
        face_resolver: &'a FaceResolver,
        measurement_policy: DisplayRowMeasurementPolicy,
        default_resolved: &'a ResolvedFace,
        default_face_metrics: DisplayRowFallbackMetrics,
        window_metrics: DisplayRowFallbackMetrics,
        window_system: bool,
    ) -> Self {
        Self {
            buffer,
            face_resolver,
            measurement_policy,
            default_resolved,
            default_face_metrics,
            window_metrics,
            window_system,
        }
    }

    pub(crate) fn resolve_at_checkpoint(
        &self,
        state: &mut BufferSourceFaceResolutionState<'_, '_>,
        charpos: i64,
    ) -> bool {
        if !state.face_scan.should_resolve_at(charpos as usize) {
            return false;
        }

        let origin = DisplayOrigin::BufferText {
            charpos: neovm_core::buffer::CharPos0::new(charpos as usize),
        };
        let resolved = self.face_resolver.default_base_face_for_origin(
            Some(self.buffer),
            &origin,
            state.face_scan.next_check_mut(),
        );
        let face_id = state.face_ids.allocate();
        let resolved_extend = resolved.extend;
        let resolved_bg = resolved.bg;
        let resolved_box_type = resolved.box_type;
        *state.active_face_state = state.source_render.resolve_and_install_measured_face(
            self.measurement_policy,
            face_id,
            resolved,
            self.window_system,
            self.window_metrics.char_width(),
            self.window_metrics,
        );
        let face_metrics = state.active_face_state.metrics();
        state
            .row_geometry
            .include_row_extents(face_metrics.row_height(), face_metrics.ascent());

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

    pub(crate) fn source_item_layout_resolution_context(
        self,
    ) -> BufferSourceItemLayoutResolutionContext<'a> {
        BufferSourceItemLayoutResolutionContext::new(
            self.measurement_policy,
            self.default_resolved,
            self.default_face_metrics,
            self.window_metrics,
            self.window_system,
        )
    }

    pub(crate) fn source_resolve_params(
        self,
        display_host: Option<&'a dyn DisplayHost>,
    ) -> DisplaySourceResolveParams<'a> {
        DisplaySourceResolveParams::new(
            DisplaySourceFaceBasis::new(
                self.face_resolver,
                u32::from(BasicFaceId::Default),
                self.default_resolved,
                self.default_face_metrics,
            ),
            display_host,
        )
    }

    pub(crate) fn install_pending_source_faces(
        self,
        source_render: &mut TextRowSourceRenderState<'_>,
        row_geometry: &mut DisplayRowGeometryState,
        pending_faces: Vec<PendingDisplaySourceFace>,
    ) {
        for pending in pending_faces {
            let (face_id, resolved) = pending.into_parts();
            let active_face = source_render.resolve_and_install_measured_face(
                self.measurement_policy,
                face_id,
                resolved,
                self.window_system,
                self.window_metrics.char_width(),
                self.window_metrics,
            );
            let metrics = active_face.metrics();
            row_geometry.include_row_extents(metrics.row_height(), metrics.ascent());
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_at_checkpoint_with_source_state(
        &self,
        source_render: &mut TextRowSourceRenderState<'_>,
        face_scan: &mut FaceScanCheckpoint,
        face_ids: &mut FrameFaceIdAllocator,
        active_face_state: &mut DisplayRowActiveFaceState,
        row_geometry: &mut DisplayRowGeometryState,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &mut BoxFaceRowState,
        x: f32,
        charpos: i64,
    ) -> bool {
        self.resolve_at_checkpoint(
            &mut BufferSourceFaceResolutionState::new(
                source_render,
                face_scan,
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

impl BufferSourceItemLayoutResolutionContext<'_> {
    pub(crate) fn resolve_source_item_layout_for_active_face(
        &self,
        source_render: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        row_geometry: &mut DisplayRowGeometryState,
        active_face_state: &DisplayRowActiveFaceState,
        item: &mut DisplayItem,
    ) -> DisplayRowActiveFaceState {
        item.face = render_face_ref_with_fallback(item.face, active_face_state.face_id());

        let Some(factor) = item
            .layout
            .height
            .filter(|factor| factor.is_finite() && *factor > 0.0)
        else {
            return active_face_state.clone();
        };

        item.layout.height = None;
        let Some(resolved) = height_adjusted_face(
            active_face_state.resolved_face(),
            DisplayHeightFaceBasis {
                canonical_face: self.default_resolved,
                base_face: self.default_resolved,
                fallback_metrics: self.default_face_metrics,
            },
            factor,
        ) else {
            return active_face_state.clone();
        };

        let face_id = face_ids.allocate();
        item.face = RenderFaceRef::FaceId(face_id);
        let resolved_active_face = source_render.resolve_and_install_measured_face(
            self.measurement_policy,
            face_id,
            resolved,
            self.window_system,
            self.window_metrics.char_width(),
            self.window_metrics,
        );
        let metrics = resolved_active_face.metrics();
        row_geometry.include_row_extents(metrics.row_height(), metrics.ascent());
        resolved_active_face
    }
}

pub(crate) struct BufferSourceFaceResolutionState<'a, 'source> {
    source_render: &'a mut TextRowSourceRenderState<'source>,
    face_scan: &'a mut FaceScanCheckpoint,
    face_ids: &'a mut FrameFaceIdAllocator,
    active_face_state: &'a mut DisplayRowActiveFaceState,
    row_geometry: &'a mut DisplayRowGeometryState,
    row_extend: &'a mut DisplayRowScopedValue<(Color, u32)>,
    box_face: &'a mut BoxFaceRowState,
    x: f32,
}

impl<'a, 'source> BufferSourceFaceResolutionState<'a, 'source> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        source_render: &'a mut TextRowSourceRenderState<'source>,
        face_scan: &'a mut FaceScanCheckpoint,
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
            face_ids,
            active_face_state,
            row_geometry,
            row_extend,
            box_face,
            x,
        }
    }
}
