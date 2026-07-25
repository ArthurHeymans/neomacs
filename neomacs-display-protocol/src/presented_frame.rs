//! Immutable frame ancestry and parent-relative child placement.

use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use crate::{DisplayFrameId, PresentationId, Rect};

/// A frame rectangle in placement coordinates.
///
/// Unlike [`crate::FrameRect`], whose origin is inside a frame's own nonnegative
/// chrome coordinate space, a child frame's origin is relative to its parent
/// and may be negative when the child is clipped at the parent's top or left
/// edge.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[serde(transparent)]
pub struct ParentFrameRect(Rect);

impl ParentFrameRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Result<Self, FramePlacementError> {
        if !x.is_finite() || !y.is_finite() || !valid_extent(width) || !valid_extent(height) {
            return Err(FramePlacementError::InvalidRect);
        }
        Ok(Self(Rect::new(x, y, width, height)))
    }

    pub const fn x(self) -> f32 {
        self.0.x
    }

    pub const fn y(self) -> f32 {
        self.0.y
    }

    pub const fn width(self) -> f32 {
        self.0.width
    }

    pub const fn height(self) -> f32 {
        self.0.height
    }

    pub const fn raw(self) -> Rect {
        self.0
    }
}

impl<'de> serde::Deserialize<'de> for ParentFrameRect {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let rect = <Rect as serde::Deserialize>::deserialize(deserializer)?;
        Self::new(rect.x, rect.y, rect.width, rect.height).map_err(serde::de::Error::custom)
    }
}

/// A derived frame rectangle in root-surface coordinates.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[serde(transparent)]
pub struct RootSurfaceRect(Rect);

impl RootSurfaceRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Result<Self, FramePlacementError> {
        if !x.is_finite() || !y.is_finite() || !valid_extent(width) || !valid_extent(height) {
            return Err(FramePlacementError::InvalidRect);
        }
        Ok(Self(Rect::new(x, y, width, height)))
    }

    pub const fn x(self) -> f32 {
        self.0.x
    }

    pub const fn y(self) -> f32 {
        self.0.y
    }

    pub const fn width(self) -> f32 {
        self.0.width
    }

    pub const fn height(self) -> f32 {
        self.0.height
    }

    pub const fn raw(self) -> Rect {
        self.0
    }
}

impl<'de> serde::Deserialize<'de> for RootSurfaceRect {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let rect = <Rect as serde::Deserialize>::deserialize(deserializer)?;
        Self::new(rect.x, rect.y, rect.width, rect.height).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FramePlacementError {
    InvalidRect,
}

impl fmt::Display for FramePlacementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid frame placement rectangle")
    }
}

impl std::error::Error for FramePlacementError {}

fn valid_extent(value: f32) -> bool {
    value.is_finite() && value >= 0.0
}

/// Explicit placement of one presented frame in its immediate parent's content
/// coordinate space. Root placement is always `(0, 0)` and never includes
/// desktop/window-manager offsets or frame chrome adjustments.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PresentedFramePlacement {
    frame: DisplayFrameId,
    presentation: PresentationId,
    parent: Option<DisplayFrameId>,
    outer_in_parent: ParentFrameRect,
    z_order: i32,
}

impl Default for PresentedFramePlacement {
    fn default() -> Self {
        Self::new(
            DisplayFrameId::new(0),
            PresentationId::default(),
            None,
            ParentFrameRect::new(0.0, 0.0, 0.0, 0.0).expect("zero placement is valid"),
            0,
        )
    }
}

impl PresentedFramePlacement {
    #[must_use]
    pub const fn new(
        frame: DisplayFrameId,
        presentation: PresentationId,
        parent: Option<DisplayFrameId>,
        outer_in_parent: ParentFrameRect,
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
    pub const fn outer_in_parent(self) -> ParentFrameRect {
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
    parent_relative: ParentFrameRect,
    root_relative: RootSurfaceRect,
    clip_in_root: PresentedClip,
    z_path: Vec<i32>,
}

impl PlacedFrame {
    pub const fn frame(&self) -> DisplayFrameId {
        self.frame
    }
    pub const fn root(&self) -> DisplayFrameId {
        self.root
    }
    pub const fn parent_relative(&self) -> ParentFrameRect {
        self.parent_relative
    }
    pub const fn root_relative(&self) -> RootSurfaceRect {
        self.root_relative
    }
    pub const fn clip_in_root(&self) -> PresentedClip {
        self.clip_in_root
    }
    pub fn z_path(&self) -> &[i32] {
        &self.z_path
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PresentedClip {
    Empty,
    Rect(RootSurfaceRect),
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
    InvalidRootOrigin(DisplayFrameId),
    InvalidDerivedPlacement(DisplayFrameId),
}

impl PresentedFrameScene {
    pub fn from_placements(
        placements: impl IntoIterator<Item = PresentedFramePlacement>,
    ) -> Result<Self, PlaceChildError> {
        let mut frames = HashMap::new();
        for placement in placements {
            if placement.parent.is_none()
                && (placement.outer_in_parent.x() != 0.0 || placement.outer_in_parent.y() != 0.0)
            {
                return Err(PlaceChildError::InvalidRootOrigin(placement.frame));
            }
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
            let outer = RootSurfaceRect::new(
                root_x,
                root_y,
                ancestor.outer_in_parent.width(),
                ancestor.outer_in_parent.height(),
            )
            .map_err(|_| PlaceChildError::InvalidDerivedPlacement(ancestor.frame))?;
            clip = Some(match clip {
                None => PresentedClip::Rect(outer),
                Some(PresentedClip::Empty) => PresentedClip::Empty,
                Some(PresentedClip::Rect(current)) => intersect(current, outer),
            });
            z_path.push(ancestor.z_order);
        }
        let root_relative = RootSurfaceRect::new(
            root_x,
            root_y,
            placement.outer_in_parent.width(),
            placement.outer_in_parent.height(),
        )
        .map_err(|_| PlaceChildError::InvalidDerivedPlacement(placement.frame))?;
        Ok(PlacedFrame {
            frame: placement.frame,
            root: ancestry.last().expect("ancestry is nonempty").frame,
            parent_relative: placement.outer_in_parent,
            root_relative,
            clip_in_root: clip.expect("ancestry is nonempty"),
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

fn intersect(current: RootSurfaceRect, next: RootSurfaceRect) -> PresentedClip {
    let left = current.x().max(next.x());
    let top = current.y().max(next.y());
    let right = (current.x() + current.width()).min(next.x() + next.width());
    let bottom = (current.y() + current.height()).min(next.y() + next.height());
    if right > left && bottom > top {
        PresentedClip::Rect(
            RootSurfaceRect::new(left, top, right - left, bottom - top)
                .expect("intersection of valid root-surface rectangles is valid"),
        )
    } else {
        PresentedClip::Empty
    }
}

#[cfg(test)]
#[path = "presented_frame_test.rs"]
mod tests;
