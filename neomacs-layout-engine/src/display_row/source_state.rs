use crate::display_item::DisplayItem;
use crate::display_source::DisplayItemSource;
use crate::display_source_resolver::{
    DisplaySourceResolveParams, DisplaySourceResolveState, ResolvedDisplaySourceItem,
    resolve_next_display_source_item,
};
use crate::frame_face_arena::FrameFaceAttempt;
use crate::neovm_bridge::ResolvedFace;
use neomacs_display_protocol::types::FaceId;

#[derive(Default)]
pub(crate) struct DisplayRowSourceState {
    resolve_state: DisplaySourceResolveState,
    pending_item: Option<DisplayItem>,
    exhausted: bool,
    /// `(left-fringe …)` / `(right-fringe …)` specs collected while resolving
    /// this source's items, drained by the render path onto the output row.
    pending_fringes: Vec<crate::display_spec::DisplayFringeLayout>,
}

impl DisplayRowSourceState {
    pub(crate) fn next_resolved_item(
        &mut self,
        source: &mut impl DisplayItemSource,
        params: DisplaySourceResolveParams<'_>,
        face_ids: &mut FrameFaceAttempt,
    ) -> ResolvedDisplaySourceItem {
        if self.is_finished() {
            return ResolvedDisplaySourceItem::empty();
        }
        if let Some(item) = self.take_pending_item() {
            return ResolvedDisplaySourceItem::new(Some(item), Vec::new());
        }
        let mut resolved =
            resolve_next_display_source_item(source, params, &mut self.resolve_state, face_ids);
        self.pending_fringes.extend(resolved.take_pending_fringes());
        if resolved.item().is_none() {
            self.mark_exhausted();
        }
        resolved
    }

    /// Drain the fringe layouts collected while resolving this source.
    pub(crate) fn take_pending_fringes(&mut self) -> Vec<crate::display_spec::DisplayFringeLayout> {
        std::mem::take(&mut self.pending_fringes)
    }

    pub(crate) fn resolved_face(&self, face_id: FaceId) -> Option<&ResolvedFace> {
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
