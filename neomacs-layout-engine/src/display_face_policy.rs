use neomacs_display_protocol::face::BasicFaceId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BaseFacePolicy {
    BufferFaceIncludingOverlays,
    OverlayStringAtAnchor,
    DisplayPropertyUnderlyingFace,
    DefaultFace,
    FixedBasicFace(BasicFaceId),
}
