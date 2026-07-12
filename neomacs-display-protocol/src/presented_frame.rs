//! Immutable frame ancestry and parent-relative child placement.

use std::collections::{HashMap, HashSet};

use crate::{DisplayFrameId, FrameRect, PresentationId};

/// Explicit placement of one presented frame in its immediate parent's content
/// coordinate space. Root placement is always `(0, 0)` and never includes
/// desktop/window-manager offsets or frame chrome adjustments.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PresentedFramePlacement {
    frame: DisplayFrameId,
    presentation: PresentationId,
    parent: Option<DisplayFrameId>,
    outer_in_parent: FrameRect,
    z_order: i32,
}

impl PresentedFramePlacement {
    #[must_use]
    pub const fn new(
        frame: DisplayFrameId,
        presentation: PresentationId,
        parent: Option<DisplayFrameId>,
        outer_in_parent: FrameRect,
        z_order: i32,
    ) -> Self {
        Self {
            frame,
            presentation,
            parent,
            outer_in_parent,
            z_order,
        }
    }

    pub const fn frame(self) -> DisplayFrameId {
        self.frame
    }
    pub const fn presentation(self) -> PresentationId {
        self.presentation
    }
    pub const fn parent(self) -> Option<DisplayFrameId> {
        self.parent
    }
    pub const fn outer_in_parent(self) -> FrameRect {
        self.outer_in_parent
    }
    pub const fn z_order(self) -> i32 {
        self.z_order
    }
}

#[derive(Clone, Debug, Default)]
pub struct PresentedFrameScene {
    frames: HashMap<DisplayFrameId, PresentedFramePlacement>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaceChildQuery {
    frame: DisplayFrameId,
    presentation: PresentationId,
}

impl PlaceChildQuery {
    pub const fn new(frame: DisplayFrameId, presentation: PresentationId) -> Self {
        Self {
            frame,
            presentation,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlacedFrame {
    frame: DisplayFrameId,
    root: DisplayFrameId,
    parent_relative: FrameRect,
    root_relative: FrameRect,
    clip_in_root: Option<FrameRect>,
    z_path: Vec<i32>,
}

impl PlacedFrame {
    pub const fn frame(&self) -> DisplayFrameId {
        self.frame
    }
    pub const fn root(&self) -> DisplayFrameId {
        self.root
    }
    pub const fn parent_relative(&self) -> FrameRect {
        self.parent_relative
    }
    pub const fn root_relative(&self) -> FrameRect {
        self.root_relative
    }
    pub const fn clip_in_root(&self) -> Option<FrameRect> {
        self.clip_in_root
    }
    pub fn z_path(&self) -> &[i32] {
        &self.z_path
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlaceChildError {
    MissingFrame(DisplayFrameId),
    MissingParent {
        frame: DisplayFrameId,
        parent: DisplayFrameId,
    },
    StalePresentation {
        frame: DisplayFrameId,
        requested: PresentationId,
        available: PresentationId,
    },
    AncestryCycle(DisplayFrameId),
}

impl PresentedFrameScene {
    pub fn from_placements(
        placements: impl IntoIterator<Item = PresentedFramePlacement>,
    ) -> Result<Self, PlaceChildError> {
        let mut frames = HashMap::new();
        for placement in placements {
            if frames.insert(placement.frame, placement).is_some() {
                return Err(PlaceChildError::AncestryCycle(placement.frame));
            }
        }
        let scene = Self { frames };
        for placement in scene.frames.values() {
            scene.ancestry(*placement)?;
        }
        Ok(scene)
    }

    pub fn place(&self, query: PlaceChildQuery) -> Result<PlacedFrame, PlaceChildError> {
        let placement = *self
            .frames
            .get(&query.frame)
            .ok_or(PlaceChildError::MissingFrame(query.frame))?;
        if placement.presentation != query.presentation {
            return Err(PlaceChildError::StalePresentation {
                frame: query.frame,
                requested: query.presentation,
                available: placement.presentation,
            });
        }
        let ancestry = self.ancestry(placement)?;
        let mut root_x = 0.0;
        let mut root_y = 0.0;
        let mut clip = None;
        let mut z_path = Vec::with_capacity(ancestry.len());
        for ancestor in ancestry.iter().rev() {
            root_x += ancestor.outer_in_parent.x();
            root_y += ancestor.outer_in_parent.y();
            let outer = FrameRect::new(
                root_x,
                root_y,
                ancestor.outer_in_parent.width(),
                ancestor.outer_in_parent.height(),
            )
            .expect("translated valid frame rectangle remains valid");
            clip = intersect_optional(clip, outer);
            z_path.push(ancestor.z_order);
        }
        let root_relative = FrameRect::new(
            root_x,
            root_y,
            placement.outer_in_parent.width(),
            placement.outer_in_parent.height(),
        )
        .expect("translated valid frame rectangle remains valid");
        Ok(PlacedFrame {
            frame: placement.frame,
            root: ancestry.last().expect("ancestry is nonempty").frame,
            parent_relative: placement.outer_in_parent,
            root_relative,
            clip_in_root: clip,
            z_path,
        })
    }

    fn ancestry(
        &self,
        placement: PresentedFramePlacement,
    ) -> Result<Vec<PresentedFramePlacement>, PlaceChildError> {
        let mut result = Vec::new();
        let mut current = placement;
        let mut seen = HashSet::new();
        loop {
            if !seen.insert(current.frame) {
                return Err(PlaceChildError::AncestryCycle(current.frame));
            }
            result.push(current);
            let Some(parent) = current.parent else { break };
            current = *self
                .frames
                .get(&parent)
                .ok_or(PlaceChildError::MissingParent {
                    frame: current.frame,
                    parent,
                })?;
        }
        Ok(result)
    }
}

fn intersect_optional(current: Option<FrameRect>, next: FrameRect) -> Option<FrameRect> {
    let Some(current) = current else {
        return Some(next);
    };
    let left = current.x().max(next.x());
    let top = current.y().max(next.y());
    let right = (current.x() + current.width()).min(next.x() + next.width());
    let bottom = (current.y() + current.height()).min(next.y() + next.height());
    (right > left && bottom > top)
        .then(|| FrameRect::new(left, top, right - left, bottom - top).unwrap())
}

#[cfg(test)]
#[path = "presented_frame_test.rs"]
mod tests;
