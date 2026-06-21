use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_item::DisplayItem;
use crate::display_source::DisplayItemSource;
#[cfg(test)]
use crate::display_source_resolver::PendingDisplaySourceFace;
use crate::display_source_resolver::{
    DisplaySourceResolveParams, DisplaySourceResolveState, ResolvedDisplaySourceItem,
    resolve_next_display_source_item,
};
use crate::neovm_bridge::ResolvedFace;
#[cfg(test)]
use crate::{
    display_row_metrics::DisplayRowFallbackMetrics,
    display_source_resolver::DisplaySourceFaceBasis, neovm_bridge::FaceResolver,
};
#[cfg(test)]
use neovm_core::emacs_core::eval::DisplayHost;

#[derive(Default)]
pub(crate) struct DisplayRowSourceState {
    resolve_state: DisplaySourceResolveState,
    pending_item: Option<DisplayItem>,
    exhausted: bool,
}

impl DisplayRowSourceState {
    pub(crate) fn next_resolved_item(
        &mut self,
        source: &mut impl DisplayItemSource,
        params: DisplaySourceResolveParams<'_>,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> ResolvedDisplaySourceItem {
        if self.is_finished() {
            return ResolvedDisplaySourceItem::empty();
        }
        if let Some(item) = self.take_pending_item() {
            return ResolvedDisplaySourceItem::new(Some(item), Vec::new());
        }
        let resolved =
            resolve_next_display_source_item(source, params, &mut self.resolve_state, face_ids);
        if resolved.item().is_none() {
            self.mark_exhausted();
        }
        resolved
    }

    pub(crate) fn resolved_face(&self, face_id: u32) -> Option<&ResolvedFace> {
        self.resolve_state.resolved_face(face_id)
    }

    fn take_pending_item(&mut self) -> Option<DisplayItem> {
        self.pending_item.take()
    }

    pub(crate) fn remember_pending_item(&mut self, item: Option<DisplayItem>) {
        self.pending_item = item;
    }

    pub(crate) fn discard_pending_item(&mut self) {
        self.pending_item = None;
    }

    fn mark_exhausted(&mut self) {
        self.exhausted = true;
    }

    pub(crate) fn is_finished(&self) -> bool {
        self.exhausted && self.pending_item.is_none()
    }
}

#[cfg(test)]
pub(crate) struct DisplayRowSourceStep {
    item: DisplayItem,
    pending_faces: Vec<PendingDisplaySourceFace>,
}

#[cfg(test)]
impl DisplayRowSourceStep {
    pub(crate) fn into_parts(self) -> (DisplayItem, Vec<PendingDisplaySourceFace>) {
        (self.item, self.pending_faces)
    }
}

#[cfg(test)]
pub(crate) struct DisplayRowSourceWalker<S> {
    source: S,
    state: DisplayRowSourceState,
}

#[cfg(test)]
impl<S> DisplayRowSourceWalker<S> {
    pub(crate) fn new(source: S) -> Self {
        Self {
            source,
            state: DisplayRowSourceState::default(),
        }
    }
}

#[cfg(test)]
impl<S: DisplayItemSource> DisplayRowSourceWalker<S> {
    pub(crate) fn next_step(
        &mut self,
        face_resolver: &FaceResolver,
        base_face: &ResolvedFace,
        base_face_id: u32,
        face_ids: &mut FrameFaceIdAllocator,
        display_host: Option<&dyn DisplayHost>,
        fallback_char_width: f32,
        fallback_ascent: f32,
        fallback_row_height: f32,
    ) -> Option<DisplayRowSourceStep> {
        let face_basis = DisplaySourceFaceBasis::new(
            face_resolver,
            base_face_id,
            base_face,
            DisplayRowFallbackMetrics::from_default_face_extents(
                fallback_char_width,
                fallback_row_height,
                fallback_ascent,
            ),
        );
        let resolved = self.state.next_resolved_item(
            &mut self.source,
            DisplaySourceResolveParams::new(face_basis, display_host),
            face_ids,
        );
        let (item, pending_faces) = resolved.into_parts();
        item.map(|item| DisplayRowSourceStep {
            item,
            pending_faces,
        })
    }
}
