use crate::display_item::RenderFaceRef;

pub(crate) fn render_face_ref_id(face: RenderFaceRef, fallback: u32) -> u32 {
    match face {
        RenderFaceRef::FaceId(face_id) => face_id,
        RenderFaceRef::Inherit => fallback,
    }
}

pub(crate) fn render_face_ref_with_fallback(face: RenderFaceRef, fallback: u32) -> RenderFaceRef {
    RenderFaceRef::FaceId(render_face_ref_id(face, fallback))
}
