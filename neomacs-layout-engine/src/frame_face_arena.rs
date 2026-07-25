//! Frame-scoped ownership for face identity across speculative layout attempts.

use neomacs_display_protocol::face::{BasicFaceId, Face};
use neomacs_display_protocol::types::FaceId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FrameFaceGeneration(u64);

impl Default for FrameFaceGeneration {
    fn default() -> Self {
        Self(1)
    }
}

impl FrameFaceGeneration {
    fn next(self) -> Self {
        Self(
            self.0
                .checked_add(1)
                .expect("frame face generation exhausted"),
        )
    }
}

#[derive(Clone, Debug)]
pub(crate) struct FrameFaceArena {
    generation: FrameFaceGeneration,
    faces: Arc<HashMap<FaceId, Face>>,
}

#[derive(Clone, Debug)]
pub(crate) struct FrameFaceAttempt {
    state: Rc<RefCell<FrameFaceAttemptState>>,
}

#[derive(Debug)]
struct FrameFaceAttemptState {
    generation: FrameFaceGeneration,
    next_face_id: u32,
    faces: HashMap<FaceId, Face>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FrameFaceConflict {
    pub(crate) face_id: FaceId,
    pub(crate) existing: Face,
    pub(crate) replacement: Face,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FrameFaceReuseError {
    StaleGeneration {
        retained: FrameFaceGeneration,
        current: FrameFaceGeneration,
    },
    MissingFace(FaceId),
    ConflictingFace(FaceId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FrameFaceSealError {
    FaceSetChanged {
        published: Vec<FaceId>,
        finalized: Vec<FaceId>,
    },
    MismatchedFaceId {
        table_id: FaceId,
        face_id: FaceId,
    },
}

impl Default for FrameFaceArena {
    fn default() -> Self {
        Self {
            generation: FrameFaceGeneration(1),
            faces: Arc::new(HashMap::new()),
        }
    }
}

impl FrameFaceArena {
    pub(crate) fn generation(&self) -> FrameFaceGeneration {
        self.generation
    }

    pub(crate) fn begin_attempt(&self) -> FrameFaceAttempt {
        FrameFaceAttempt {
            state: Rc::new(RefCell::new(FrameFaceAttemptState {
                generation: self.generation,
                next_face_id: BasicFaceId::SENTINEL,
                faces: HashMap::new(),
            })),
        }
    }

    #[cfg(test)]
    pub(crate) fn invalidate(&self) -> Self {
        Self {
            generation: self.generation.next(),
            faces: Arc::new(HashMap::new()),
        }
    }
}

impl FrameFaceAttempt {
    #[cfg(test)]
    pub(crate) fn for_test_with_next_id(next_face_id: u32) -> Self {
        Self {
            state: Rc::new(RefCell::new(FrameFaceAttemptState {
                generation: FrameFaceGeneration(1),
                next_face_id: next_face_id.max(BasicFaceId::SENTINEL),
                faces: HashMap::new(),
            })),
        }
    }

    pub(crate) fn reserve_dynamic_face(&mut self) -> FaceId {
        let mut state = self.state.borrow_mut();
        while state.faces.contains_key(&FaceId::new(state.next_face_id)) {
            state.next_face_id = state.next_face_id.saturating_add(1);
        }
        let face_id = FaceId::new(state.next_face_id);
        state.next_face_id = state.next_face_id.saturating_add(1);
        face_id
    }

    pub(crate) fn reserve_after(&mut self, face_id: FaceId) {
        let mut state = self.state.borrow_mut();
        state.next_face_id = state.next_face_id.max(face_id.get().saturating_add(1));
    }

    #[cfg(test)]
    pub(crate) fn next_face_id_for_test(&self) -> u32 {
        self.state.borrow().next_face_id
    }

    pub(crate) fn admit_retained(
        &mut self,
        generation: FrameFaceGeneration,
        face_ids: impl IntoIterator<Item = FaceId>,
        arena: &FrameFaceArena,
    ) -> Result<(), FrameFaceReuseError> {
        if generation != arena.generation {
            return Err(FrameFaceReuseError::StaleGeneration {
                retained: generation,
                current: arena.generation,
            });
        }
        let face_ids: Vec<FaceId> = face_ids.into_iter().collect();
        for face_id in &face_ids {
            if !arena.faces.contains_key(face_id) {
                return Err(FrameFaceReuseError::MissingFace(*face_id));
            }
        }
        {
            let state = self.state.borrow();
            for face_id in &face_ids {
                if state
                    .faces
                    .get(face_id)
                    .is_some_and(|existing| existing != &arena.faces[face_id])
                {
                    return Err(FrameFaceReuseError::ConflictingFace(*face_id));
                }
            }
        }
        let mut state = self.state.borrow_mut();
        for face_id in face_ids {
            state.faces.insert(face_id, arena.faces[&face_id].clone());
        }
        Ok(())
    }

    pub(crate) fn publish(&mut self, face: Face) -> Result<FaceId, FrameFaceConflict> {
        let mut state = self.state.borrow_mut();
        let face_id = face.id;
        match state.faces.entry(face_id) {
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(face);
            }
            std::collections::hash_map::Entry::Occupied(mut slot) if slot.get() != &face => {
                if !merge_compatible_realization(slot.get_mut(), &face) {
                    return Err(FrameFaceConflict {
                        face_id,
                        existing: slot.get().clone(),
                        replacement: face,
                    });
                }
            }
            std::collections::hash_map::Entry::Occupied(_) => {}
        }
        Ok(face_id)
    }

    pub(crate) fn faces(&self) -> HashMap<FaceId, Face> {
        self.state.borrow().faces.clone()
    }

    #[cfg(test)]
    pub(crate) fn face(&self, face_id: FaceId) -> Option<Face> {
        self.state.borrow().faces.get(&face_id).cloned()
    }

    #[cfg(test)]
    pub(crate) fn commit(&self) -> FrameFaceArena {
        let state = self.state.borrow();
        FrameFaceArena {
            generation: state.generation.next(),
            faces: Arc::new(state.faces.clone()),
        }
    }

    /// Seal the exact renderer-facing table produced by the layout transaction.
    ///
    /// Font realization may enrich a published face with an exact font file or
    /// resolved-font handle after row construction. It may not add, remove, or
    /// re-key face identities.
    pub(crate) fn seal(
        &self,
        finalized_faces: HashMap<FaceId, Face>,
    ) -> Result<FrameFaceArena, FrameFaceSealError> {
        let state = self.state.borrow();
        let mut published: Vec<FaceId> = state.faces.keys().copied().collect();
        let mut finalized: Vec<FaceId> = finalized_faces.keys().copied().collect();
        published.sort_unstable();
        finalized.sort_unstable();
        if published != finalized {
            return Err(FrameFaceSealError::FaceSetChanged {
                published,
                finalized,
            });
        }
        if let Some((table_id, face_id)) = finalized_faces
            .iter()
            .find_map(|(table_id, face)| (*table_id != face.id).then_some((*table_id, face.id)))
        {
            return Err(FrameFaceSealError::MismatchedFaceId { table_id, face_id });
        }
        Ok(FrameFaceArena {
            generation: state.generation.next(),
            faces: Arc::new(finalized_faces),
        })
    }
}

fn merge_compatible_realization(existing: &mut Face, replacement: &Face) -> bool {
    let mut existing_identity = existing.clone();
    existing_identity.font_ascent = 0;
    existing_identity.font_descent = 0;
    existing_identity.font_file_path = None;
    existing_identity.default_resolved_font_id = None;
    let mut replacement_identity = replacement.clone();
    replacement_identity.font_ascent = 0;
    replacement_identity.font_descent = 0;
    replacement_identity.font_file_path = None;
    replacement_identity.default_resolved_font_id = None;
    if existing_identity != replacement_identity {
        return false;
    }

    if existing
        .font_file_path
        .as_ref()
        .zip(replacement.font_file_path.as_ref())
        .is_some_and(|(existing, replacement)| existing != replacement)
        || existing
            .default_resolved_font_id
            .as_ref()
            .zip(replacement.default_resolved_font_id.as_ref())
            .is_some_and(|(existing, replacement)| existing != replacement)
    {
        return false;
    }

    if replacement.font_ascent != 0 {
        existing.font_ascent = replacement.font_ascent;
    }
    if replacement.font_descent != 0 {
        existing.font_descent = replacement.font_descent;
    }
    if replacement.font_file_path.is_some() {
        existing
            .font_file_path
            .clone_from(&replacement.font_file_path);
    }
    if replacement.default_resolved_font_id.is_some() {
        existing
            .default_resolved_font_id
            .clone_from(&replacement.default_resolved_font_id);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use neomacs_display_protocol::types::Color;

    #[test]
    fn one_attempt_cannot_rebind_a_face_id_to_different_rendering() {
        let arena = FrameFaceArena::default();
        let mut attempt = arena.begin_attempt();
        let face_id = attempt.reserve_dynamic_face();

        let mut original = Face::new(face_id);
        original.foreground = Color::from_pixel(0x00112233);
        attempt
            .publish(original.clone())
            .expect("first publication");

        let mut replacement = Face::new(face_id);
        replacement.foreground = Color::from_pixel(0x00445566);
        assert!(
            attempt.publish(replacement).is_err(),
            "a frame face id is immutable once published"
        );
        assert_eq!(
            attempt.faces().get(&face_id),
            Some(&original),
            "rejected publication must preserve the original face"
        );
    }

    #[test]
    fn one_attempt_can_complete_missing_metrics_for_the_same_face() {
        let arena = FrameFaceArena::default();
        let mut attempt = arena.begin_attempt();
        let face_id = attempt.reserve_dynamic_face();
        let incomplete = Face::new(face_id);
        attempt
            .publish(incomplete)
            .expect("publish semantic face before measurement");

        let mut measured = Face::new(face_id);
        measured.font_ascent = 13;
        measured.font_descent = 5;
        attempt
            .publish(measured.clone())
            .expect("measurement may complete missing metrics");
        assert_eq!(attempt.face(face_id), Some(measured));
    }

    #[test]
    fn later_realization_replaces_metrics_without_clearing_exact_font_identity() {
        let arena = FrameFaceArena::default();
        let mut attempt = arena.begin_attempt();
        let face_id = attempt.reserve_dynamic_face();
        let mut earlier = Face::new(face_id);
        earlier.font_ascent = 7;
        earlier.font_descent = 3;
        earlier.font_file_path = Some("/fonts/exact.ttf".to_owned());
        attempt
            .publish(earlier)
            .expect("publish earlier realization");

        let mut later = Face::new(face_id);
        later.font_ascent = 4;
        later.font_descent = 2;
        attempt
            .publish(later)
            .expect("publish later realization of the same face");

        let realized = attempt.face(face_id).expect("realized face");
        assert_eq!((realized.font_ascent, realized.font_descent), (4, 2));
        assert_eq!(realized.font_file_path.as_deref(), Some("/fonts/exact.ttf"));
    }

    #[test]
    fn retained_faces_occupy_their_slots_before_fresh_allocation() {
        let arena = FrameFaceArena::default();
        let mut first = arena.begin_attempt();
        let retained_id = first.reserve_dynamic_face();
        let mut retained_face = Face::new(retained_id);
        retained_face.foreground = Color::from_pixel(0x00112233);
        first
            .publish(retained_face.clone())
            .expect("publish retained face");
        let committed = first.commit();

        let mut next = committed.begin_attempt();
        next.admit_retained(committed.generation, [retained_id], &committed)
            .expect("admit retained face");

        let fresh_id = next.reserve_dynamic_face();
        assert_ne!(
            fresh_id, retained_id,
            "fresh allocation must not alias an admitted retained face"
        );
        assert_eq!(next.faces().get(&retained_id), Some(&retained_face));
    }

    #[test]
    fn invalidated_arena_rejects_stale_retained_handles_before_admission() {
        let arena = FrameFaceArena::default();
        let mut first = arena.begin_attempt();
        let retained_id = first.reserve_dynamic_face();
        first
            .publish(Face::new(retained_id))
            .expect("publish retained face");
        let committed = first.commit();
        let stale_generation = committed.generation();
        let invalidated = committed.invalidate();
        let mut next = invalidated.begin_attempt();

        assert_eq!(
            next.admit_retained(stale_generation, [retained_id], &invalidated),
            Err(FrameFaceReuseError::StaleGeneration {
                retained: stale_generation,
                current: invalidated.generation(),
            })
        );
        assert!(
            next.faces().is_empty(),
            "failed admission must not partially publish retained faces"
        );
    }

    #[test]
    fn retained_admission_cannot_overwrite_an_attempt_publication() {
        let arena = FrameFaceArena::default();
        let mut first = arena.begin_attempt();
        let face_id = first.reserve_dynamic_face();
        let mut retained = Face::new(face_id);
        retained.foreground = Color::from_pixel(0x00112233);
        first.publish(retained).expect("publish retained face");
        let committed = first.commit();

        let mut next = committed.begin_attempt();
        let mut fresh = Face::new(face_id);
        fresh.foreground = Color::from_pixel(0x00445566);
        next.publish(fresh.clone()).expect("publish fresh face");
        assert_eq!(
            next.admit_retained(committed.generation(), [face_id], &committed),
            Err(FrameFaceReuseError::ConflictingFace(face_id))
        );
        assert_eq!(
            next.face(face_id),
            Some(fresh),
            "failed retained admission must preserve the attempt publication"
        );
    }

    #[test]
    fn sealing_commits_the_finalized_face_table_for_future_replay() {
        let arena = FrameFaceArena::default();
        let mut attempt = arena.begin_attempt();
        let face_id = attempt.reserve_dynamic_face();
        attempt
            .publish(Face::new(face_id))
            .expect("publish semantic face");

        let mut finalized_faces = attempt.faces();
        finalized_faces
            .get_mut(&face_id)
            .expect("published face")
            .font_file_path = Some("/fonts/exact.ttf".to_owned());
        let sealed = attempt
            .seal(finalized_faces)
            .expect("sealing may enrich a published face");

        let mut replay = sealed.begin_attempt();
        replay
            .admit_retained(sealed.generation(), [face_id], &sealed)
            .expect("admit face from sealed arena");
        assert_eq!(
            replay.face(face_id).and_then(|face| face.font_file_path),
            Some("/fonts/exact.ttf".to_owned())
        );
    }

    #[test]
    fn sealing_advances_the_generation() {
        let arena = FrameFaceArena::default();
        let attempt = arena.begin_attempt();

        let sealed = attempt.seal(HashMap::new()).expect("seal empty attempt");

        assert_ne!(
            sealed.generation(),
            arena.generation(),
            "each accepted presentation needs a distinct retained-face generation"
        );
    }
}
