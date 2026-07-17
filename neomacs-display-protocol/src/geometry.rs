//! Typed logical coordinate spaces and their explicit conversion boundaries.
//!
//! Layout stays in [`LayoutUnit`] coordinates.  A translation is branded with
//! its source and destination spaces, so row-local geometry cannot be passed as
//! frame geometry without the transform that owns that conversion.  Fractional
//! display scale is applied only when a sealed frame is adapted to device space.

use std::marker::PhantomData;

use crate::types::LayoutUnit;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RowSpace {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WindowSpace {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FrameSpace {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BandSpace {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LayoutPoint<Space> {
    x: LayoutUnit,
    y: LayoutUnit,
    space: PhantomData<fn() -> Space>,
}

impl<Space> LayoutPoint<Space> {
    #[must_use]
    pub const fn new(x: LayoutUnit, y: LayoutUnit) -> Self {
        Self {
            x,
            y,
            space: PhantomData,
        }
    }

    #[must_use]
    pub fn from_px(x: f32, y: f32) -> Self {
        Self::new(LayoutUnit::from_px(x), LayoutUnit::from_px(y))
    }

    #[must_use]
    pub const fn x(&self) -> LayoutUnit {
        self.x
    }

    #[must_use]
    pub const fn y(&self) -> LayoutUnit {
        self.y
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LayoutSize<Space> {
    width: LayoutUnit,
    height: LayoutUnit,
    space: PhantomData<fn() -> Space>,
}

impl<Space> LayoutSize<Space> {
    #[must_use]
    pub const fn new(width: LayoutUnit, height: LayoutUnit) -> Self {
        Self {
            width,
            height,
            space: PhantomData,
        }
    }

    #[must_use]
    pub fn from_px(width: f32, height: f32) -> Self {
        Self::new(
            LayoutUnit::from_px(width).max(LayoutUnit::ZERO),
            LayoutUnit::from_px(height).max(LayoutUnit::ZERO),
        )
    }

    #[must_use]
    pub const fn width(&self) -> LayoutUnit {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> LayoutUnit {
        self.height
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LayoutRect<Space> {
    origin: LayoutPoint<Space>,
    size: LayoutSize<Space>,
}

impl<Space> LayoutRect<Space> {
    #[must_use]
    pub const fn new(origin: LayoutPoint<Space>, size: LayoutSize<Space>) -> Self {
        Self { origin, size }
    }

    #[must_use]
    pub fn from_px(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self::new(
            LayoutPoint::from_px(x, y),
            LayoutSize::from_px(width, height),
        )
    }

    #[must_use]
    pub const fn x(&self) -> LayoutUnit {
        self.origin.x()
    }

    #[must_use]
    pub const fn y(&self) -> LayoutUnit {
        self.origin.y()
    }

    #[must_use]
    pub const fn width(&self) -> LayoutUnit {
        self.size.width()
    }

    #[must_use]
    pub const fn height(&self) -> LayoutUnit {
        self.size.height()
    }
}

/// A translation that is only applicable from `From` coordinates to `To`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SpaceTranslation<From, To> {
    dx: LayoutUnit,
    dy: LayoutUnit,
    spaces: PhantomData<fn(From) -> To>,
}

impl<From, To> SpaceTranslation<From, To> {
    #[must_use]
    pub const fn new(dx: LayoutUnit, dy: LayoutUnit) -> Self {
        Self {
            dx,
            dy,
            spaces: PhantomData,
        }
    }

    #[must_use]
    pub fn from_px(dx: f32, dy: f32) -> Self {
        Self::new(LayoutUnit::from_px(dx), LayoutUnit::from_px(dy))
    }

    #[must_use]
    pub fn map_point(self, point: LayoutPoint<From>) -> LayoutPoint<To> {
        LayoutPoint::new(point.x() + self.dx, point.y() + self.dy)
    }

    #[must_use]
    pub fn map_rect(self, rect: LayoutRect<From>) -> LayoutRect<To> {
        LayoutRect::new(
            self.map_point(LayoutPoint::new(rect.x(), rect.y())),
            LayoutSize::new(rect.width(), rect.height()),
        )
    }

    #[must_use]
    pub fn then<Next>(self, next: SpaceTranslation<To, Next>) -> SpaceTranslation<From, Next> {
        SpaceTranslation::new(self.dx + next.dx, self.dy + next.dy)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeviceRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl DeviceRect {
    #[must_use]
    pub const fn x(self) -> f32 {
        self.x
    }

    #[must_use]
    pub const fn y(self) -> f32 {
        self.y
    }

    #[must_use]
    pub const fn width(self) -> f32 {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> f32 {
        self.height
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidDeviceScale;

/// Fractional logical-to-device scale, validated once at the adapter boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeviceScale(f32);

impl DeviceScale {
    pub fn new(scale: f32) -> Result<Self, InvalidDeviceScale> {
        if scale.is_finite() && scale > 0.0 {
            Ok(Self(scale))
        } else {
            Err(InvalidDeviceScale)
        }
    }

    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }

    #[must_use]
    pub fn map_frame_rect(self, rect: LayoutRect<FrameSpace>) -> DeviceRect {
        DeviceRect {
            x: rect.x().to_px() * self.0,
            y: rect.y().to_px() * self.0,
            width: rect.width().to_px() * self.0,
            height: rect.height().to_px() * self.0,
        }
    }
}
