use super::*;
use neomacs_display_protocol::{
    DisplayFrameId, FrameDisplayState, FrameRect, PresentationId, PresentedFramePlacement,
};

fn resolved_state(revision: u64) -> FrameDisplayState {
    let mut state = FrameDisplayState::new(80, 24, 8.0, 16.0);
    state.presentation_id = PresentationId::new(revision);
    state.frame_placement = PresentedFramePlacement::new(
        DisplayFrameId::new(7),
        state.presentation_id,
        None,
        FrameRect::new(0.0, 0.0, 640.0, 384.0).unwrap(),
        0,
    );
    state
}

#[test]
fn composer_seals_all_spatial_products_under_one_revision() {
    let resolved = ResolvedFrame::new(resolved_state(41)).expect("coherent resolved frame");

    let sealed = PresentationComposer::compose(resolved, &[]).expect("valid presentation");

    assert_eq!(sealed.revision().presentation(), PresentationId::new(41));
    assert_eq!(sealed.transport().presentation_id, PresentationId::new(41));
    assert_eq!(
        sealed.transport().presented_hit_index.presentation(),
        PresentationId::new(41)
    );
}

#[test]
fn resolved_frame_rejects_mismatched_placement_revision() {
    let mut state = resolved_state(41);
    state.frame_placement = PresentedFramePlacement::new(
        DisplayFrameId::new(7),
        PresentationId::new(40),
        None,
        FrameRect::new(0.0, 0.0, 640.0, 384.0).unwrap(),
        0,
    );

    assert_eq!(
        ResolvedFrame::new(state).unwrap_err(),
        PresentationComposeError::StaleFramePlacement {
            frame: DisplayFrameId::new(7),
            expected: PresentationId::new(41),
            available: PresentationId::new(40),
        }
    );
}

#[test]
fn resolved_frame_requires_a_real_revision() {
    let state = FrameDisplayState::new(80, 24, 8.0, 16.0);

    assert_eq!(
        ResolvedFrame::new(state).unwrap_err(),
        PresentationComposeError::MissingRevision
    );
}
