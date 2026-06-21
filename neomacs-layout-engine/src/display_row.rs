use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_face_ref::render_face_ref_id;
use crate::display_item::RenderFaceRef;
use crate::display_origin::DisplayOrigin;
use crate::display_property::parse_display_length_expr;
use crate::display_row_builder::{
    DisplayRowAppendStatus, DisplayRowItemMeasurement, DisplayRowLayout, DisplayRowPosition,
    DisplayRowProgressWriter, DisplayTabPolicy, new_display_row_for_role,
};
use crate::display_row_geometry::DisplayRowGeometryState;
pub(crate) use crate::display_row_geometry::{DisplayRowGeometry, DisplayRowMaxX};
#[cfg(test)]
pub(crate) use crate::display_row_measured_state::{
    DisplayRowBoundsPolicy, DisplayRowOwner, FrameChromeKind, MeasuredDisplayRow, WindowChromeKind,
};
pub(crate) use crate::display_row_metrics::DisplayRowFallbackMetrics;
#[cfg(test)]
pub(crate) use crate::display_row_metrics::DisplayRowMeasuredFaceMetrics;
use crate::display_row_render_item::DisplayRowRenderItem;
use crate::display_row_render_policy::NaturalDisplayRowRenderPolicy;
pub(crate) use crate::display_row_render_policy::{
    DisplayRowRenderClipBehavior, DisplayRowRenderPolicy,
};
#[cfg(test)]
pub(crate) use crate::display_row_render_state::RenderedDisplayRowMedia;
#[cfg(test)]
pub(crate) use crate::display_row_render_state::{
    CurrentTextRowRenderOutcome, DisplayRowOutputProgress, RenderedDisplayRowMediaKind,
};
pub(crate) use crate::display_row_render_state::{
    DisplayRowRenderBounds, DisplayRowRenderIntoRowResult, DisplayRowRenderResult,
    DisplayRowRenderStop, RenderedDisplayRow, display_row_progress,
};
pub(crate) use crate::display_row_source_state::DisplayRowSourceState;
#[cfg(test)]
pub(crate) use crate::display_row_source_state::DisplayRowSourceWalker;
use crate::display_source::{DisplayItemSource, LispStringSourceCursor};
use crate::display_source_resolver::{DisplaySourceFaceBasis, DisplaySourceResolveParams};
use crate::font_metrics::FontMetricsService;
use crate::neovm_bridge::FaceResolver;
use crate::neovm_bridge::ResolvedFace;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{GlyphArea, GlyphRow};
use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::eval::DisplayHost;

#[cfg(test)]
pub(crate) use crate::display_row_face_state::{
    DisplayRowActiveFaceState, DisplayRowGlyphMeasurementFace, DisplayRowMeasurementMode,
    DisplayRowMeasurementPolicy,
};
pub(crate) use crate::display_row_face_state::{
    DisplayRowFace, DisplayRowFaceRealizer, DisplayRowGlyphMeasurer,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DisplayRowLispStringSourceId(u64);

impl DisplayRowLispStringSourceId {
    const ROOT: Self = Self(1);

    fn raw(self) -> u64 {
        self.0
    }
}

pub(crate) struct DisplayRowItemSourceRenderRequest<'a> {
    row_request: DisplayRowSourceRenderRequest<'a>,
}

pub(crate) struct DisplayRowSourceFragmentRenderRequest<'a> {
    item_request: DisplayRowItemSourceRenderRequest<'a>,
}

#[derive(Clone)]
pub(crate) struct DisplayRowSourceFragmentFrame<'face> {
    policy: DisplayRowSourceRequestPolicy,
    base_face_id: u32,
    base_face: &'face ResolvedFace,
}

impl<'face> DisplayRowSourceFragmentFrame<'face> {
    pub(crate) fn new(
        geometry: DisplayRowGeometry,
        role: GlyphRowRole,
        base_face_id: u32,
        base_face: &'face ResolvedFace,
    ) -> Self {
        Self {
            policy: DisplayRowSourceRequestPolicy::from_display_row_geometry(geometry, role),
            base_face_id,
            base_face,
        }
    }

    pub(crate) fn render_request(
        self,
        render_bounds: DisplayRowRenderBounds,
    ) -> DisplayRowSourceFragmentRenderRequest<'face> {
        DisplayRowSourceFragmentRenderRequest::from_base_face_id_policy_with_render_bounds(
            self.policy,
            self.base_face_id,
            self.base_face,
            render_bounds,
        )
    }

    pub(crate) fn render_request_for_area(
        self,
        render_bounds: DisplayRowRenderBounds,
        area: GlyphArea,
    ) -> DisplayRowSourceFragmentRenderRequest<'face> {
        self.render_request(render_bounds).with_glyph_area(area)
    }

    pub(crate) fn from_glyph_row_columns(
        row: &GlyphRow,
        matrix_cols: usize,
        char_width: f32,
        role: GlyphRowRole,
        base_face_id: u32,
        base_face: &'face ResolvedFace,
    ) -> Self {
        let char_width = char_width.max(1.0);
        let height = row.height_px.max(1.0);
        Self::new(
            DisplayRowGeometry::new(
                row.pixel_y,
                matrix_cols.max(1) as f32 * char_width,
                height,
                char_width,
                row.ascent_px.max(0.0).min(height),
                DisplayTabPolicy::every(8),
            ),
            role,
            base_face_id,
            base_face,
        )
    }

    pub(crate) fn from_row_geometry_columns(
        row_geometry: &DisplayRowGeometryState,
        columns: usize,
        char_width: f32,
        role: GlyphRowRole,
        base_face_id: u32,
        base_face: &'face ResolvedFace,
    ) -> Self {
        let char_width = char_width.max(1.0);
        Self::new(
            DisplayRowGeometry::new(
                row_geometry.y(),
                columns.max(1) as f32 * char_width,
                row_geometry.height(),
                char_width,
                row_geometry.ascent(),
                DisplayTabPolicy::every(8),
            ),
            role,
            base_face_id,
            base_face,
        )
    }

    pub(crate) fn render_request_from_column(
        self,
        start_col: usize,
        max_col: usize,
    ) -> DisplayRowSourceFragmentRenderRequest<'face> {
        let char_width = self.policy.geometry.char_width;
        self.render_request(DisplayRowRenderBounds::new(
            DisplayRowPosition::new(start_col as f32 * char_width, start_col),
            DisplayRowMaxX::Bounded(max_col as f32 * char_width),
        ))
    }

    pub(crate) fn render_request_from_column_for_area(
        self,
        start_col: usize,
        max_col: usize,
        area: GlyphArea,
    ) -> DisplayRowSourceFragmentRenderRequest<'face> {
        self.render_request_from_column(start_col, max_col)
            .with_glyph_area(area)
    }
}

pub(crate) struct DisplayRowLispStringSourceSessionRequest {
    source_id: DisplayRowLispStringSourceId,
    value: Value,
    base_face_id: u32,
}

pub(crate) struct DisplayRowLispStringSourceRenderRequest<'a> {
    row_request: DisplayRowSourceRenderRequest<'a>,
    session_request: DisplayRowLispStringSourceSessionRequest,
}

impl<'a> DisplayRowLispStringSourceRenderRequest<'a> {
    pub(crate) fn from_value(row_request: DisplayRowSourceRenderRequest<'a>, value: Value) -> Self {
        let session_request = DisplayRowLispStringSourceSessionRequest::for_base_face_id(
            value,
            row_request.base_face_id(),
        );
        Self {
            row_request,
            session_request,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_origin_value(
        y: f32,
        width: f32,
        height: f32,
        char_width: f32,
        ascent: f32,
        tab_policy: DisplayTabPolicy,
        origin: DisplayOrigin,
        face_ids: &mut FrameFaceIdAllocator,
        base_face: &'a ResolvedFace,
        value: Value,
        symbol_values: std::collections::HashMap<String, Value>,
    ) -> Self {
        let row_request = DisplayRowSourceRequestPolicy::from_origin(
            y, width, height, char_width, ascent, tab_policy, origin,
        )
        .with_symbol_values(symbol_values)
        .source_request_from_base_face(face_ids, base_face);
        Self::from_value(row_request, value)
    }

    fn into_render_parts(
        self,
    ) -> (
        DisplayRowRenderPlan<'a>,
        DisplayRowLispStringSourceSessionRequest,
    ) {
        (self.row_request.into_render_plan(), self.session_request)
    }
}

impl<'a> DisplayRowItemSourceRenderRequest<'a> {
    fn new(row_request: DisplayRowSourceRenderRequest<'a>) -> Self {
        Self { row_request }
    }

    fn from_base_face_id_policy_with_render_bounds(
        policy: DisplayRowSourceRequestPolicy,
        base_face_id: u32,
        base_face: &'a ResolvedFace,
        render_bounds: DisplayRowRenderBounds,
    ) -> Self {
        Self::new(
            policy
                .source_request_for_base_face_id(base_face_id, base_face)
                .with_render_bounds(render_bounds),
        )
    }

    fn with_glyph_area(mut self, area: GlyphArea) -> Self {
        self.row_request = self.row_request.with_glyph_area(area);
        self
    }

    fn into_render_plan(self) -> DisplayRowRenderPlan<'a> {
        self.row_request.into_render_plan()
    }

    #[cfg(test)]
    pub(crate) fn render<S: DisplayItemSource>(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        source: &mut S,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> Option<RenderedDisplayRow> {
        let mut context = DisplayRowRenderContext::new(face_resolver, None, face_ids);
        self.render_with_context(renderer, source, &mut context)
    }

    #[cfg(test)]
    fn render_with_context<S: DisplayItemSource>(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        source: &mut S,
        context: &mut DisplayRowRenderContext<'_, '_>,
    ) -> Option<RenderedDisplayRow> {
        let mut state = DisplayRowSourceState::default();
        self.render_step_with_context(renderer, source, &mut state, context)
            .map(DisplayRowRenderResult::into_rendered)
    }

    #[cfg(test)]
    pub(crate) fn render_step_with_context<S: DisplayItemSource>(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        source: &mut S,
        state: &mut DisplayRowSourceState,
        context: &mut DisplayRowRenderContext<'_, '_>,
    ) -> Option<DisplayRowRenderResult> {
        renderer.render_display_item_source_row_step_with_context(
            self.into_render_plan(),
            source,
            state,
            context,
        )
    }

    #[cfg(test)]
    pub(crate) fn render_fragment_step_with_display_host<S: DisplayItemSource>(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        source: &mut S,
        state: &mut DisplayRowSourceState,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> Option<DisplayRowRenderResult> {
        let mut context = DisplayRowRenderContext::new(face_resolver, display_host, face_ids);
        renderer.render_display_item_source_row_fragment_step_with_context(
            self.into_render_plan(),
            source,
            state,
            &mut context,
        )
    }

    #[cfg(test)]
    pub(crate) fn render_fragment_step_into_row_with_display_host<S: DisplayItemSource>(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        row: &mut GlyphRow,
        source: &mut S,
        state: &mut DisplayRowSourceState,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        let mut context = DisplayRowRenderContext::new(face_resolver, display_host, face_ids);
        renderer.render_display_item_source_row_fragment_step_into_row_with_context(
            self.into_render_plan(),
            row,
            source,
            state,
            &mut context,
        )
    }

    pub(crate) fn render_fragment_step_into_row_with_policy<
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    >(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        row: &mut GlyphRow,
        source: &mut S,
        state: &mut DisplayRowSourceState,
        context: &mut DisplayRowRenderContext<'_, '_>,
        policy: &mut P,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        renderer.render_display_item_source_row_fragment_step_into_row_with_policy(
            self.into_render_plan(),
            row,
            source,
            state,
            context,
            policy,
        )
    }
}

impl<'a> DisplayRowSourceFragmentRenderRequest<'a> {
    fn from_base_face_id_policy_with_render_bounds(
        policy: DisplayRowSourceRequestPolicy,
        base_face_id: u32,
        base_face: &'a ResolvedFace,
        render_bounds: DisplayRowRenderBounds,
    ) -> Self {
        Self {
            item_request:
                DisplayRowItemSourceRenderRequest::from_base_face_id_policy_with_render_bounds(
                    policy,
                    base_face_id,
                    base_face,
                    render_bounds,
                ),
        }
    }

    pub(crate) fn with_glyph_area(mut self, area: GlyphArea) -> Self {
        self.item_request = self.item_request.with_glyph_area(area);
        self
    }

    fn into_item_request(self) -> DisplayRowItemSourceRenderRequest<'a> {
        self.item_request
    }

    #[cfg(test)]
    pub(crate) fn geometry(&self) -> &DisplayRowGeometry {
        self.item_request.row_request.geometry()
    }

    #[cfg(test)]
    pub(crate) fn render_bounds(&self) -> DisplayRowRenderBounds {
        self.item_request.row_request.render_bounds()
    }

    #[cfg(test)]
    pub(crate) fn glyph_area(&self) -> GlyphArea {
        self.item_request.row_request.area
    }

    #[cfg(test)]
    pub(crate) fn render<S: DisplayItemSource>(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        source: &mut S,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> Option<RenderedDisplayRow> {
        self.into_item_request()
            .render(renderer, source, face_resolver, face_ids)
    }

    #[cfg(test)]
    pub(crate) fn render_fragment_step_with_display_host<S: DisplayItemSource>(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        source: &mut S,
        state: &mut DisplayRowSourceState,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> Option<DisplayRowRenderResult> {
        self.into_item_request()
            .render_fragment_step_with_display_host(
                renderer,
                source,
                state,
                face_resolver,
                display_host,
                face_ids,
            )
    }
}

impl DisplayRowLispStringSourceSessionRequest {
    fn for_base_face_id(value: Value, base_face_id: u32) -> Self {
        Self {
            source_id: DisplayRowLispStringSourceId::ROOT,
            value,
            base_face_id,
        }
    }
}

pub(crate) struct DisplayRowLispStringSourceSession {
    source: LispStringSourceCursor,
    state: DisplayRowSourceState,
}

impl DisplayRowLispStringSourceSession {
    pub(crate) fn new(request: DisplayRowLispStringSourceSessionRequest) -> Option<Self> {
        let source = LispStringSourceCursor::new(
            request.source_id.raw(),
            request.value,
            RenderFaceRef::FaceId(request.base_face_id),
        )?;
        Some(Self {
            source,
            state: DisplayRowSourceState::default(),
        })
    }

    fn render_next_row_plan_with_context(
        &mut self,
        renderer: &mut DisplayRowRenderer<'_>,
        plan: DisplayRowRenderPlan<'_>,
        context: &mut DisplayRowRenderContext<'_, '_>,
    ) -> Option<DisplayRowRenderResult> {
        renderer.render_display_item_source_row_step_with_context(
            plan,
            &mut self.source,
            &mut self.state,
            context,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
struct DisplayRowSourceGeometry {
    y: f32,
    width: f32,
    height: f32,
    char_width: f32,
    ascent: f32,
    tab_policy: DisplayTabPolicy,
}

impl DisplayRowSourceGeometry {
    fn new(
        y: f32,
        width: f32,
        height: f32,
        char_width: f32,
        ascent: f32,
        tab_policy: DisplayTabPolicy,
    ) -> Self {
        Self {
            y,
            width,
            height,
            char_width,
            ascent,
            tab_policy,
        }
    }

    fn from_display_row_geometry(geometry: DisplayRowGeometry) -> Self {
        Self::new(
            geometry.y(),
            geometry.width(),
            geometry.height(),
            geometry.char_width(),
            geometry.ascent(),
            geometry.tab_policy().clone(),
        )
    }

    fn into_geometry(self) -> DisplayRowGeometry {
        DisplayRowGeometry::new(
            self.y,
            self.width,
            self.height,
            self.char_width,
            self.ascent,
            self.tab_policy,
        )
    }

    pub(crate) fn source_request_from_base_face<'face>(
        self,
        face_ids: &mut FrameFaceIdAllocator,
        base_face: &'face ResolvedFace,
        role: GlyphRowRole,
        symbol_values: std::collections::HashMap<String, Value>,
    ) -> DisplayRowSourceRenderRequest<'face> {
        DisplayRowSourceRenderRequest::from_base_face(
            self.into_geometry(),
            face_ids,
            base_face,
            role,
            symbol_values,
        )
    }

    pub(crate) fn source_request_for_base_face_id<'face>(
        self,
        base_face_id: u32,
        base_face: &'face ResolvedFace,
        role: GlyphRowRole,
    ) -> DisplayRowSourceRenderRequest<'face> {
        DisplayRowSourceRenderRequest::whole_row(
            self.into_geometry(),
            base_face_id,
            base_face,
            role,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
struct DisplayRowSourceRequestPolicy {
    geometry: DisplayRowSourceGeometry,
    role: GlyphRowRole,
    symbol_values: std::collections::HashMap<String, Value>,
}

impl DisplayRowSourceRequestPolicy {
    fn new(
        y: f32,
        width: f32,
        height: f32,
        char_width: f32,
        ascent: f32,
        tab_policy: DisplayTabPolicy,
        role: GlyphRowRole,
    ) -> Self {
        Self {
            geometry: DisplayRowSourceGeometry::new(
                y, width, height, char_width, ascent, tab_policy,
            ),
            role,
            symbol_values: std::collections::HashMap::new(),
        }
    }

    fn from_display_row_geometry(geometry: DisplayRowGeometry, role: GlyphRowRole) -> Self {
        Self {
            geometry: DisplayRowSourceGeometry::from_display_row_geometry(geometry),
            role,
            symbol_values: std::collections::HashMap::new(),
        }
    }

    pub(crate) fn from_origin(
        y: f32,
        width: f32,
        height: f32,
        char_width: f32,
        ascent: f32,
        tab_policy: DisplayTabPolicy,
        origin: DisplayOrigin,
    ) -> Self {
        let role = origin
            .glyph_row_role()
            .expect("display row source origin must map to a glyph row role");
        Self::new(y, width, height, char_width, ascent, tab_policy, role)
    }

    pub(crate) fn with_symbol_values(
        mut self,
        symbol_values: std::collections::HashMap<String, Value>,
    ) -> Self {
        self.symbol_values = symbol_values;
        self
    }

    pub(crate) fn source_request_from_base_face<'face>(
        self,
        face_ids: &mut FrameFaceIdAllocator,
        base_face: &'face ResolvedFace,
    ) -> DisplayRowSourceRenderRequest<'face> {
        self.geometry.source_request_from_base_face(
            face_ids,
            base_face,
            self.role,
            self.symbol_values,
        )
    }

    pub(crate) fn source_request_for_base_face_id<'face>(
        self,
        base_face_id: u32,
        base_face: &'face ResolvedFace,
    ) -> DisplayRowSourceRenderRequest<'face> {
        debug_assert!(self.symbol_values.is_empty());
        self.geometry
            .source_request_for_base_face_id(base_face_id, base_face, self.role)
    }
}

struct DisplayRowRenderPlan<'a> {
    geometry: DisplayRowGeometry,
    render_bounds: DisplayRowRenderBounds,
    area: GlyphArea,
    base_face_id: u32,
    base_face: &'a ResolvedFace,
    role: GlyphRowRole,
    symbol_values: std::collections::HashMap<String, Value>,
}

pub(crate) struct DisplayRowSourceRenderRequest<'a> {
    geometry: DisplayRowGeometry,
    render_bounds: DisplayRowRenderBounds,
    area: GlyphArea,
    base_face_id: u32,
    base_face: &'a ResolvedFace,
    role: GlyphRowRole,
    symbol_values: std::collections::HashMap<String, Value>,
}

impl<'a> DisplayRowSourceRenderRequest<'a> {
    fn whole_row(
        geometry: DisplayRowGeometry,
        base_face_id: u32,
        base_face: &'a ResolvedFace,
        role: GlyphRowRole,
    ) -> Self {
        let render_bounds = DisplayRowRenderBounds::whole_row(geometry.width());
        Self {
            geometry,
            render_bounds,
            area: GlyphArea::Text,
            base_face_id,
            base_face,
            role,
            symbol_values: std::collections::HashMap::new(),
        }
    }

    fn from_base_face(
        geometry: DisplayRowGeometry,
        face_ids: &mut FrameFaceIdAllocator,
        base_face: &'a ResolvedFace,
        role: GlyphRowRole,
        symbol_values: std::collections::HashMap<String, Value>,
    ) -> Self {
        let base_face_id = if base_face.face_id != 0 {
            base_face.face_id
        } else {
            face_ids.allocate()
        };
        let render_bounds = DisplayRowRenderBounds::whole_row(geometry.width());
        Self {
            geometry,
            render_bounds,
            area: GlyphArea::Text,
            base_face_id,
            base_face,
            role,
            symbol_values,
        }
    }

    pub(crate) fn from_display_row_geometry_for_base_face_id(
        geometry: DisplayRowGeometry,
        base_face_id: u32,
        base_face: &'a ResolvedFace,
        role: GlyphRowRole,
    ) -> Self {
        DisplayRowSourceRequestPolicy::from_display_row_geometry(geometry, role)
            .source_request_for_base_face_id(base_face_id, base_face)
    }

    #[cfg(test)]
    pub(crate) fn from_display_row_geometry(
        geometry: DisplayRowGeometry,
        face_ids: &mut FrameFaceIdAllocator,
        base_face: &'a ResolvedFace,
        role: GlyphRowRole,
        symbol_values: std::collections::HashMap<String, Value>,
    ) -> Self {
        DisplayRowSourceRequestPolicy::from_display_row_geometry(geometry, role)
            .with_symbol_values(symbol_values)
            .source_request_from_base_face(face_ids, base_face)
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_origin(
        y: f32,
        width: f32,
        height: f32,
        char_width: f32,
        ascent: f32,
        tab_policy: DisplayTabPolicy,
        origin: DisplayOrigin,
        face_ids: &mut FrameFaceIdAllocator,
        base_face: &'a ResolvedFace,
        symbol_values: std::collections::HashMap<String, Value>,
    ) -> Self {
        DisplayRowSourceRequestPolicy::from_origin(
            y, width, height, char_width, ascent, tab_policy, origin,
        )
        .with_symbol_values(symbol_values)
        .source_request_from_base_face(face_ids, base_face)
    }

    pub(crate) fn with_render_bounds(mut self, render_bounds: DisplayRowRenderBounds) -> Self {
        self.render_bounds = render_bounds;
        self
    }

    fn with_glyph_area(mut self, area: GlyphArea) -> Self {
        self.area = area;
        self
    }

    #[cfg(test)]
    pub(crate) fn base_face_ref(&self) -> RenderFaceRef {
        RenderFaceRef::FaceId(self.base_face_id)
    }

    pub(crate) fn base_face_id(&self) -> u32 {
        self.base_face_id
    }

    #[cfg(test)]
    pub(crate) fn base_face(&self) -> &'a ResolvedFace {
        self.base_face
    }

    #[cfg(test)]
    pub(crate) fn geometry(&self) -> &DisplayRowGeometry {
        &self.geometry
    }

    #[cfg(test)]
    pub(crate) fn render_bounds(&self) -> DisplayRowRenderBounds {
        self.render_bounds
    }

    pub(crate) fn role(&self) -> GlyphRowRole {
        self.role
    }

    pub(crate) fn render_fragment_step_into_row_with_policy<
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    >(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        row: &mut GlyphRow,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        context: &mut DisplayRowRenderContext<'_, '_>,
        render_policy: &mut P,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        DisplayRowItemSourceRenderRequest::new(self).render_fragment_step_into_row_with_policy(
            renderer,
            row,
            source,
            source_state,
            context,
            render_policy,
        )
    }

    #[cfg(test)]
    pub(crate) fn symbol_values(&self) -> &std::collections::HashMap<String, Value> {
        &self.symbol_values
    }

    fn into_render_plan(self) -> DisplayRowRenderPlan<'a> {
        DisplayRowRenderPlan {
            geometry: self.geometry,
            render_bounds: self.render_bounds,
            area: self.area,
            base_face_id: self.base_face_id,
            base_face: self.base_face,
            role: self.role,
            symbol_values: self.symbol_values,
        }
    }
}

fn include_display_row_face_metrics(layout: &mut DisplayRowLayout, face: &DisplayRowFace) {
    face.metrics.include_in_layout(layout);
}

pub(crate) struct DisplayRowRenderContext<'a, 'ids> {
    face_resolver: &'a FaceResolver,
    display_host: Option<&'a dyn DisplayHost>,
    face_ids: &'ids mut FrameFaceIdAllocator,
}

impl<'a, 'ids> DisplayRowRenderContext<'a, 'ids> {
    pub(crate) fn new(
        face_resolver: &'a FaceResolver,
        display_host: Option<&'a dyn DisplayHost>,
        face_ids: &'ids mut FrameFaceIdAllocator,
    ) -> Self {
        Self {
            face_resolver,
            display_host,
            face_ids,
        }
    }

    pub(crate) fn source_resolve_params<'b>(
        &self,
        base_face_id: u32,
        base_face: &'b ResolvedFace,
        fallback: DisplayRowFallbackMetrics,
    ) -> DisplaySourceResolveParams<'b>
    where
        'a: 'b,
    {
        DisplaySourceResolveParams::new(
            DisplaySourceFaceBasis::new(self.face_resolver, base_face_id, base_face, fallback),
            self.display_host.map(|host| host as &'b dyn DisplayHost),
        )
    }

    fn face_ids(&mut self) -> &mut FrameFaceIdAllocator {
        self.face_ids
    }
}

pub(crate) struct DisplayRowRenderer<'metrics> {
    font_metrics: &'metrics mut Option<FontMetricsService>,
}

pub(crate) struct DisplayRowRenderExecutor<'metrics, 'context, 'ids> {
    renderer: DisplayRowRenderer<'metrics>,
    context: DisplayRowRenderContext<'context, 'ids>,
}

impl<'metrics, 'context, 'ids> DisplayRowRenderExecutor<'metrics, 'context, 'ids> {
    pub(crate) fn new(
        font_metrics: &'metrics mut Option<FontMetricsService>,
        face_resolver: &'context FaceResolver,
        display_host: Option<&'context dyn DisplayHost>,
        face_ids: &'ids mut FrameFaceIdAllocator,
    ) -> Self {
        Self {
            renderer: DisplayRowRenderer::new(font_metrics),
            context: DisplayRowRenderContext::new(face_resolver, display_host, face_ids),
        }
    }

    pub(crate) fn render_lisp_string_source_request(
        &mut self,
        request: DisplayRowLispStringSourceRenderRequest<'_>,
    ) -> Option<RenderedDisplayRow> {
        let (plan, session_request) = request.into_render_parts();
        self.renderer
            .render_lisp_string_plan_with_context(plan, session_request, &mut self.context)
    }

    pub(crate) fn render_item_source_fragment_into_row<S: DisplayItemSource>(
        &mut self,
        request: DisplayRowSourceFragmentRenderRequest<'_>,
        row: &mut GlyphRow,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        request
            .into_item_request()
            .render_fragment_step_into_row_with_policy(
                &mut self.renderer,
                row,
                source,
                source_state,
                &mut self.context,
                &mut NaturalDisplayRowRenderPolicy,
            )
    }
}

impl<'metrics> DisplayRowRenderer<'metrics> {
    pub(crate) fn new(font_metrics: &'metrics mut Option<FontMetricsService>) -> Self {
        Self { font_metrics }
    }

    fn render_lisp_string_plan_with_context(
        &mut self,
        plan: DisplayRowRenderPlan<'_>,
        session_request: DisplayRowLispStringSourceSessionRequest,
        context: &mut DisplayRowRenderContext<'_, '_>,
    ) -> Option<RenderedDisplayRow> {
        let mut session = DisplayRowLispStringSourceSession::new(session_request)?;
        session
            .render_next_row_plan_with_context(self, plan, context)
            .map(DisplayRowRenderResult::into_rendered)
    }

    fn render_display_item_source_row_step_with_context(
        &mut self,
        plan: DisplayRowRenderPlan<'_>,
        source: &mut impl DisplayItemSource,
        state: &mut DisplayRowSourceState,
        context: &mut DisplayRowRenderContext<'_, '_>,
    ) -> Option<DisplayRowRenderResult> {
        let mut result = self.render_display_item_source_row_fragment_step_with_context(
            plan, source, state, context,
        )?;
        result.finalize_external_row();
        Some(result)
    }

    fn render_display_item_source_row_fragment_step_with_context(
        &mut self,
        plan: DisplayRowRenderPlan<'_>,
        source: &mut impl DisplayItemSource,
        state: &mut DisplayRowSourceState,
        context: &mut DisplayRowRenderContext<'_, '_>,
    ) -> Option<DisplayRowRenderResult> {
        let mut row = new_display_row_for_role(plan.role);
        let result = self.render_display_item_source_row_fragment_step_into_row_with_context(
            plan, &mut row, source, state, context,
        )?;
        Some(result.with_row(row))
    }

    fn render_display_item_source_row_fragment_step_into_row_with_context(
        &mut self,
        plan: DisplayRowRenderPlan<'_>,
        row: &mut GlyphRow,
        source: &mut impl DisplayItemSource,
        state: &mut DisplayRowSourceState,
        context: &mut DisplayRowRenderContext<'_, '_>,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        let mut policy = NaturalDisplayRowRenderPolicy;
        self.render_display_item_source_row_fragment_step_into_row_with_policy(
            plan,
            row,
            source,
            state,
            context,
            &mut policy,
        )
    }

    fn render_display_item_source_row_fragment_step_into_row_with_policy<
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    >(
        &mut self,
        plan: DisplayRowRenderPlan<'_>,
        row: &mut GlyphRow,
        source: &mut S,
        state: &mut DisplayRowSourceState,
        context: &mut DisplayRowRenderContext<'_, '_>,
        policy: &mut P,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        if state.is_finished() {
            return None;
        }

        let DisplayRowRenderPlan {
            geometry,
            render_bounds,
            area,
            base_face_id,
            base_face,
            role,
            symbol_values,
        } = plan;
        context.face_ids().reserve_after(base_face_id);
        let mut face_realizer = DisplayRowFaceRealizer::new(&mut *self.font_metrics);
        let row_face = face_realizer.realize_face(
            base_face_id,
            base_face,
            geometry.char_width(),
            geometry.ascent(),
            geometry.height(),
        );
        let char_width = face_realizer
            .char_width(&row_face, geometry.char_width())
            .max(1.0);
        let mut row_faces = vec![row_face.clone()];

        let parsed_symbol_values = symbol_values
            .into_iter()
            .filter_map(|(name, value)| parse_display_length_expr(value).map(|expr| (name, expr)))
            .collect();
        let row_ascent = row_face
            .metrics
            .ascent_px()
            .max(geometry.ascent())
            .min(geometry.height().max(1.0));
        let mut row_layout = geometry.to_layout(
            role,
            char_width,
            row_ascent,
            RenderFaceRef::FaceId(row_face.face_id),
            parsed_symbol_values,
        );
        let mut position = render_bounds.start();
        let mut source_slots = Vec::new();
        let mut media = Vec::new();
        let fallback_metrics = DisplayRowFallbackMetrics::from_default_face_extents(
            char_width,
            geometry.height(),
            geometry.ascent(),
        );
        let stop = loop {
            let params =
                context.source_resolve_params(row_face.face_id, base_face, fallback_metrics);
            let resolved = state.next_resolved_item(source, params, context.face_ids());
            let (item, pending_faces) = resolved.into_parts();
            for pending in pending_faces {
                let (face_id, resolved) = pending.into_parts();
                let row_face = face_realizer.realize_face(
                    face_id,
                    &resolved,
                    char_width,
                    geometry.ascent(),
                    geometry.height(),
                );
                include_display_row_face_metrics(&mut row_layout, &row_face);
                row_faces.push(row_face);
            }
            let Some(item) = item else {
                break DisplayRowRenderStop::SourceExhausted;
            };
            if policy.stop_before_item(&item) {
                break DisplayRowRenderStop::SourceExhausted;
            }
            if let RenderFaceRef::FaceId(face_id) = item.face {
                if face_id != row_face.face_id
                    && !row_faces.iter().any(|face| face.face_id == face_id)
                    && let Some(resolved) = state.resolved_face(face_id).cloned()
                {
                    let realized = face_realizer.realize_face(
                        face_id,
                        &resolved,
                        char_width,
                        geometry.ascent(),
                        geometry.height(),
                    );
                    include_display_row_face_metrics(&mut row_layout, &realized);
                    row_faces.push(realized);
                }
            }
            let render_item = DisplayRowRenderItem::from_source_item(item);
            let item_face_id = render_face_ref_id(render_item.row_face(), row_face.face_id);
            let measurement = policy.measurement_for(
                render_item.row_item(),
                item_face_id,
                face_realizer.font_metrics_mut(),
            );
            let progress = match measurement {
                DisplayRowItemMeasurement::Default => {
                    let mut glyph_measurer = DisplayRowGlyphMeasurer::new(
                        &row_faces,
                        face_realizer.font_metrics_service_mut(),
                        char_width,
                    );
                    let mut row_writer = DisplayRowProgressWriter::with_glyph_measurer_for_area(
                        &row_layout,
                        &mut *row,
                        &mut glyph_measurer,
                        position,
                        render_bounds.max_x().to_f32(),
                        area,
                    );
                    row_writer.push_item(render_item.row_item_for_write())
                }
                DisplayRowItemMeasurement::TextRun(measurement) => {
                    let mut row_writer =
                        DisplayRowProgressWriter::with_text_run_measurement_for_area(
                            &row_layout,
                            &mut *row,
                            measurement,
                            position,
                            render_bounds.max_x().to_f32(),
                            area,
                        );
                    row_writer.push_item(render_item.row_item_for_write())
                }
            };
            position = progress.end();
            source_slots.extend(progress.slots().iter().cloned());
            if let Some(rendered) =
                render_item.rendered_media_for_progress(&progress, row_layout.y_px)
            {
                media.push(rendered);
            }
            match progress.status() {
                DisplayRowAppendStatus::Complete => {}
                DisplayRowAppendStatus::Clipped => {
                    match policy.clipped_behavior(render_item.source_item()) {
                        DisplayRowRenderClipBehavior::PreserveRemainderAndStop => {
                            state.remember_pending_item(render_item.clipped_remainder(&progress));
                            break DisplayRowRenderStop::Clipped;
                        }
                        DisplayRowRenderClipBehavior::Stop => {
                            break DisplayRowRenderStop::Clipped;
                        }
                        DisplayRowRenderClipBehavior::Continue => {}
                    }
                }
                DisplayRowAppendStatus::RowBreak => {
                    break DisplayRowRenderStop::RowBreak;
                }
            }
        };
        let progress_height = if row.height_px > 0.0 {
            row.height_px
        } else {
            row_layout.height_px
        };
        let progress = display_row_progress(position, geometry.y(), progress_height);
        let faces = row_faces
            .into_iter()
            .map(|face| face.render_face())
            .collect();
        Some(DisplayRowRenderIntoRowResult::new(
            progress,
            source_slots,
            faces,
            media,
            stop,
        ))
    }
}

#[cfg(test)]
#[path = "display_row_test.rs"]
mod tests;
