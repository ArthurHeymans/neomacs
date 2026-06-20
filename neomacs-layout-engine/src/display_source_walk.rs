use crate::display_source::{DisplaySourceItem, DisplaySourceTextPosition};
use crate::display_source_progress::DisplaySourceProgressState;
use crate::display_source_resolver::PendingDisplaySourceFace;

pub(crate) struct DisplaySourceWalkConsumption {
    source_item: Option<DisplaySourceItem>,
    source_position: DisplaySourceTextPosition,
    pending_faces: Vec<PendingDisplaySourceFace>,
}

impl DisplaySourceWalkConsumption {
    pub(crate) fn new(
        source_item: Option<DisplaySourceItem>,
        source_position: DisplaySourceTextPosition,
        pending_faces: Vec<PendingDisplaySourceFace>,
    ) -> Self {
        Self {
            source_item,
            source_position,
            pending_faces,
        }
    }

    pub(crate) fn apply_to_progress(
        self,
        progress: &mut DisplaySourceProgressState<'_>,
    ) -> (Option<DisplaySourceItem>, Vec<PendingDisplaySourceFace>) {
        if self.source_item.is_none() {
            progress.apply_source_position(self.source_position);
        }
        (self.source_item, self.pending_faces)
    }
}

pub(crate) struct DisplaySourcePositionConsumption<T> {
    value: T,
    source_position: DisplaySourceTextPosition,
}

impl<T> DisplaySourcePositionConsumption<T> {
    pub(crate) fn new(value: T, source_position: DisplaySourceTextPosition) -> Self {
        Self {
            value,
            source_position,
        }
    }

    pub(crate) fn apply_to_progress(self, progress: &mut DisplaySourceProgressState<'_>) -> T {
        progress.apply_source_position(self.source_position);
        self.value
    }
}
