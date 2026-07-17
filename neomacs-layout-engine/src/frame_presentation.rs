//! One-way frame presentation pipeline.
//!
//! Layout may freely assemble a [`FrameDisplayState`] until it is wrapped as a
//! [`ResolvedFrame`].  Composition then consumes that value, derives every
//! spatial projection from the same window snapshots, validates the result,
//! and returns [`SealedFramePresentation`].  The sealed wrapper exposes no
//! mutable transport access, so renderer and TTY adapters observe one revision.

use neomacs_display_protocol::{
    DisplayFrameId, FrameDisplayState, PresentationId, PresentedHitError,
};
use neovm_core::window::WindowDisplaySnapshot;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct FrameRevision(PresentationId);

impl FrameRevision {
    pub(crate) const fn presentation(self) -> PresentationId {
        self.0
    }
}

#[derive(Debug)]
pub(crate) struct ResolvedFrame {
    revision: FrameRevision,
    transport: FrameDisplayState,
}

impl ResolvedFrame {
    pub(crate) fn new(transport: FrameDisplayState) -> Result<Self, PresentationComposeError> {
        let expected = transport.presentation_id;
        if expected == PresentationId::default() {
            return Err(PresentationComposeError::MissingRevision);
        }
        let placement = transport.frame_placement;
        if placement.presentation() != expected {
            return Err(PresentationComposeError::StaleFramePlacement {
                frame: placement.frame(),
                expected,
                available: placement.presentation(),
            });
        }
        Ok(Self {
            revision: FrameRevision(expected),
            transport,
        })
    }
}

#[derive(Debug)]
pub(crate) struct SealedFramePresentation {
    revision: FrameRevision,
    transport: FrameDisplayState,
}

impl SealedFramePresentation {
    pub(crate) const fn revision(&self) -> FrameRevision {
        self.revision
    }

    pub(crate) const fn transport(&self) -> &FrameDisplayState {
        &self.transport
    }

    pub(crate) fn into_transport(self) -> FrameDisplayState {
        self.transport
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PresentationComposeError {
    MissingRevision,
    StaleFramePlacement {
        frame: DisplayFrameId,
        expected: PresentationId,
        available: PresentationId,
    },
    Spatial(PresentedHitError),
}

impl From<PresentedHitError> for PresentationComposeError {
    fn from(error: PresentedHitError) -> Self {
        Self::Spatial(error)
    }
}

pub(crate) struct PresentationComposer;

impl PresentationComposer {
    pub(crate) fn compose(
        resolved: ResolvedFrame,
        snapshots: &[WindowDisplaySnapshot],
    ) -> Result<SealedFramePresentation, PresentationComposeError> {
        let ResolvedFrame {
            revision,
            mut transport,
        } = resolved;
        let spatial =
            crate::presentation_spatial::PresentationSpatialPlan::compile(&transport, snapshots)?;
        spatial.seal(&mut transport)?;
        Ok(SealedFramePresentation {
            revision,
            transport,
        })
    }
}

#[cfg(test)]
#[path = "frame_presentation_test.rs"]
mod tests;
