//! Builds renderer-ready pointer metadata for a completed frame presentation.
//!
//! Producer integration follows in later pointer-map tasks; keep this seam
//! crate-private until those call sites submit resolved rendered runs.

#![allow(dead_code)]

use neomacs_display_protocol::{
    FrameGlyphBuffer, FrameRect, InteractionId, PointerAppearanceId, PresentedPointerAppearance,
    PresentedPointerMapError, PresentedPointerRegion,
};

/// One fully resolved rendered run observed by pointer-map construction.
pub(crate) struct RenderedPointerRun {
    hit_bounds: FrameRect,
    interaction: Option<InteractionId>,
    appearance: Option<PresentedPointerAppearance>,
}

impl RenderedPointerRun {
    pub(crate) fn new(
        hit_bounds: FrameRect,
        interaction: Option<InteractionId>,
        appearance: Option<PresentedPointerAppearance>,
    ) -> Self {
        Self {
            hit_bounds,
            interaction,
            appearance,
        }
    }
}

struct PendingRegion {
    bounds: FrameRect,
    interaction: Option<InteractionId>,
    appearance: Option<PresentedPointerAppearance>,
}

/// Collects resolved layout observations and publishes validated pointer data.
pub(crate) struct PresentedPointerMapBuilder {
    regions: Vec<PendingRegion>,
}

impl PresentedPointerMapBuilder {
    pub(crate) fn new() -> Self {
        Self {
            regions: Vec::new(),
        }
    }

    pub(crate) fn observe_rendered_run(&mut self, run: RenderedPointerRun) {
        if run.interaction.is_none() && run.appearance.is_none() {
            return;
        }

        if let Some(previous) = self.regions.last_mut()
            && previous.interaction == run.interaction
            && previous.appearance == run.appearance
            && previous.bounds.y() == run.hit_bounds.y()
            && previous.bounds.height() == run.hit_bounds.height()
            && previous.bounds.x() + previous.bounds.width() == run.hit_bounds.x()
            && let Ok(combined) = FrameRect::new(
                previous.bounds.x(),
                previous.bounds.y(),
                previous.bounds.width() + run.hit_bounds.width(),
                previous.bounds.height(),
            )
        {
            previous.bounds = combined;
            return;
        }

        self.regions.push(PendingRegion {
            bounds: run.hit_bounds,
            interaction: run.interaction,
            appearance: run.appearance,
        });
    }

    pub(crate) fn finish_into(
        self,
        frame: &mut FrameGlyphBuffer,
    ) -> Result<(), PresentedPointerMapError> {
        let mut appearances = Vec::new();
        let mut regions = Vec::with_capacity(self.regions.len());

        for region in self.regions {
            let appearance = if let Some(appearance) = region.appearance {
                let index = appearances
                    .iter()
                    .position(|existing| existing == &appearance)
                    .unwrap_or_else(|| {
                        appearances.push(appearance);
                        appearances.len() - 1
                    });
                Some(
                    PointerAppearanceId::try_from(index)
                        .map_err(|_| PresentedPointerMapError::TooManyAppearances)?,
                )
            } else {
                None
            };
            regions.push(PresentedPointerRegion::new(
                region.bounds,
                region.interaction,
                appearance,
            ));
        }

        frame.install_presented_pointer(regions, appearances)
    }
}

#[cfg(test)]
#[path = "presented_pointer_map_test.rs"]
mod tests;
