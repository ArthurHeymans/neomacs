use neomacs_display_protocol::face::BasicFaceId;

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

    pub(crate) fn allocate(&mut self) -> u32 {
        let face_id = self.next_face_id;
        self.next_face_id += 1;
        face_id
    }

    pub(crate) fn reserve_after(&mut self, face_id: u32) {
        self.next_face_id = self.next_face_id.max(face_id.saturating_add(1));
    }

    pub(crate) fn finish(self) -> u32 {
        self.next_face_id
    }

    pub(crate) fn finish_into(self, frame_counter: &mut u32) {
        *frame_counter = self.finish();
    }
}
