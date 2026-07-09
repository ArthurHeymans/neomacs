use neomacs_display_protocol::face::BasicFaceId;
use neomacs_display_protocol::types::FaceId;

/// Mints frame-scoped dynamic face ids (GNU's single `face_cache->used`
/// counter). The raw `u32` counter is allocator-internal seed state; ids
/// leave this type only as [`FaceId`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FrameFaceIdAllocator {
    next_face_id: u32,
}

impl FrameFaceIdAllocator {
    pub(crate) fn new(next_face_id: u32) -> Self {
        Self {
            next_face_id: next_face_id.max(BasicFaceId::SENTINEL),
        }
    }

    pub(crate) fn allocate(&mut self) -> FaceId {
        let face_id = self.next_face_id;
        self.next_face_id += 1;
        FaceId::new(face_id)
    }

    pub(crate) fn reserve_after(&mut self, face_id: FaceId) {
        self.next_face_id = self.next_face_id.max(face_id.get().saturating_add(1));
    }

    pub(crate) fn finish(self) -> u32 {
        self.next_face_id
    }

    pub(crate) fn finish_into(self, frame_counter: &mut u32) {
        *frame_counter = self.finish();
    }
}
