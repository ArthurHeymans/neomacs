use neomacs_display_protocol::face::BasicFaceId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub(crate) enum BaseFacePolicy {
    BufferFaceIncludingOverlays,
    OverlayStringAtAnchor,
    DisplayPropertyUnderlyingFace,
    DefaultFace,
    FixedBasicFace(BasicFaceId),
}
