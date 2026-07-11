//! Builds renderer-ready pointer metadata for a completed frame presentation.
//!
//! Producer integration follows in later pointer-map tasks; keep this seam
//! crate-private until those call sites submit resolved rendered runs.

use std::collections::HashMap;

use neomacs_display_protocol::{
    FrameGlyphBuffer, FrameRect, InteractionId, PointerAppearanceId, PointerDrawMode,
    PresentedPaintSpan, PresentedPointerAppearance, PresentedPointerMapError,
    PresentedPointerRegion,
};

/// Stable identity of one semantic source range's transient appearance.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[allow(dead_code)] // Producer wiring lands in later pointer-map tasks.
pub(crate) struct PointerAppearanceRangeId(u64);

#[allow(dead_code)]
impl PointerAppearanceRangeId {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }
}

/// Failure to aggregate or install layout-side pointer observations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub(crate) enum PresentedPointerMapBuildError {
    ConflictingAppearanceModes(PointerAppearanceRangeId),
    TooManyAppearances,
    Protocol(PresentedPointerMapError),
}

impl std::fmt::Display for PresentedPointerMapBuildError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "failed to build presented pointer map: {self:?}")
    }
}

impl std::error::Error for PresentedPointerMapBuildError {}

impl From<PresentedPointerMapError> for PresentedPointerMapBuildError {
    fn from(error: PresentedPointerMapError) -> Self {
        Self::Protocol(error)
    }
}

/// One renderer-ready paint contribution to a semantic appearance range.
#[allow(dead_code)]
pub(crate) struct RenderedPointerAppearance {
    identity: PointerAppearanceRangeId,
    paint_span: PresentedPaintSpan,
    hover: PointerDrawMode,
    pressed: PointerDrawMode,
}

#[allow(dead_code)]
impl RenderedPointerAppearance {
    pub(crate) fn new(
        identity: PointerAppearanceRangeId,
        paint_span: PresentedPaintSpan,
        hover: PointerDrawMode,
        pressed: PointerDrawMode,
    ) -> Self {
        Self {
            identity,
            paint_span,
            hover,
            pressed,
        }
    }
}

/// One fully resolved rendered run observed by pointer-map construction.
#[allow(dead_code)]
pub(crate) struct RenderedPointerRun {
    hit_bounds: FrameRect,
    interaction: Option<InteractionId>,
    appearance_identity: Option<PointerAppearanceRangeId>,
    appearance: Option<RenderedPointerAppearance>,
}

#[allow(dead_code)]
impl RenderedPointerRun {
    pub(crate) fn new(
        hit_bounds: FrameRect,
        interaction: Option<InteractionId>,
        appearance: Option<RenderedPointerAppearance>,
    ) -> Self {
        let appearance_identity = appearance.as_ref().map(|appearance| appearance.identity);
        Self {
            hit_bounds,
            interaction,
            appearance_identity,
            appearance,
        }
    }

    pub(crate) fn referencing_appearance(
        hit_bounds: FrameRect,
        interaction: Option<InteractionId>,
        appearance_identity: PointerAppearanceRangeId,
    ) -> Self {
        Self {
            hit_bounds,
            interaction,
            appearance_identity: Some(appearance_identity),
            appearance: None,
        }
    }
}

#[allow(dead_code)]
struct PendingRegion {
    bounds: FrameRect,
    interaction: Option<InteractionId>,
    appearance: Option<PointerAppearanceRangeId>,
}

#[allow(dead_code)]
struct AppearanceAggregate {
    paint_spans: Vec<PresentedPaintSpan>,
    hover: PointerDrawMode,
    pressed: PointerDrawMode,
}

/// Collects resolved layout observations and publishes validated pointer data.
#[allow(dead_code)]
pub(crate) struct PresentedPointerMapBuilder {
    regions: Vec<PendingRegion>,
    appearance_positions: HashMap<PointerAppearanceRangeId, usize>,
    appearances: Vec<AppearanceAggregate>,
    error: Option<PresentedPointerMapBuildError>,
}

#[allow(dead_code)]
impl PresentedPointerMapBuilder {
    pub(crate) fn new() -> Self {
        Self {
            regions: Vec::new(),
            appearance_positions: HashMap::new(),
            appearances: Vec::new(),
            error: None,
        }
    }

    pub(crate) fn observe_rendered_run(&mut self, run: RenderedPointerRun) {
        if run.interaction.is_none() && run.appearance_identity.is_none() {
            return;
        }

        let appearance_identity = run.appearance_identity;
        if let Some(appearance) = run.appearance {
            if let Some(&index) = self.appearance_positions.get(&appearance.identity) {
                let aggregate = &mut self.appearances[index];
                if aggregate.hover != appearance.hover || aggregate.pressed != appearance.pressed {
                    self.error.get_or_insert(
                        PresentedPointerMapBuildError::ConflictingAppearanceModes(
                            appearance.identity,
                        ),
                    );
                } else {
                    aggregate.paint_spans.push(appearance.paint_span);
                }
            } else {
                let index = self.appearances.len();
                self.appearance_positions.insert(appearance.identity, index);
                self.appearances.push(AppearanceAggregate {
                    paint_spans: vec![appearance.paint_span],
                    hover: appearance.hover,
                    pressed: appearance.pressed,
                });
            }
        }

        if let Some(previous) = self.regions.last_mut()
            && previous.interaction == run.interaction
            && previous.appearance == appearance_identity
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
            appearance: appearance_identity,
        });
    }

    pub(crate) fn finish_into(
        self,
        frame: &mut FrameGlyphBuffer,
    ) -> Result<(), PresentedPointerMapBuildError> {
        if let Some(error) = self.error {
            return Err(error);
        }

        let appearances = self
            .appearances
            .into_iter()
            .map(|appearance| {
                PresentedPointerAppearance::new(
                    appearance.paint_spans,
                    appearance.hover,
                    appearance.pressed,
                )
            })
            .collect();
        let mut regions = Vec::with_capacity(self.regions.len());

        for region in self.regions {
            let appearance = if let Some(identity) = region.appearance {
                let index = self.appearance_positions[&identity];
                Some(
                    PointerAppearanceId::try_from(index)
                        .map_err(|_| PresentedPointerMapBuildError::TooManyAppearances)?,
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

        frame
            .install_presented_pointer(regions, appearances)
            .map_err(Into::into)
    }
}

#[cfg(test)]
#[path = "presented_pointer_map_test.rs"]
mod tests;
