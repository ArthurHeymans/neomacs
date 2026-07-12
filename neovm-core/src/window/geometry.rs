//! Coordinate-safe views over redisplay's existing window snapshots.
//!
//! This module does not own a second copy of geometry.  It gives the live
//! `Window` and its last `WindowDisplaySnapshot` typed pixel and cell views so
//! consumers cannot silently combine body-local, window-local, frame-local,
//! and cell-grid values.

use super::{DisplayPointSnapshot, FrameId, LispCharPos1, Rect, WindowDisplaySnapshot, WindowId};
use std::collections::HashMap;
use std::marker::PhantomData;

/// Evaluator-owned identity of one immutable displayed geometry publication.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PresentationId(u64);

impl PresentationId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// One immutable, presentation-owned publication of all evaluator window geometry.
#[derive(Clone, Debug)]
pub struct PresentedGeometry {
    presentation: PresentationId,
    windows: HashMap<WindowId, WindowDisplaySnapshot>,
}

impl PresentedGeometry {
    pub(crate) fn new(
        presentation: PresentationId,
        snapshots: impl IntoIterator<Item = WindowDisplaySnapshot>,
    ) -> Self {
        Self {
            presentation,
            windows: snapshots
                .into_iter()
                .map(|snapshot| (snapshot.window_id, snapshot))
                .collect(),
        }
    }

    pub const fn presentation(&self) -> PresentationId {
        self.presentation
    }

    pub fn window(&self, window: WindowId) -> Option<&WindowDisplaySnapshot> {
        self.windows.get(&window)
    }

    pub(crate) fn without_window(&self, window: WindowId) -> Self {
        let mut windows = self.windows.clone();
        windows.remove(&window);
        Self {
            presentation: self.presentation,
            windows,
        }
    }
}

/// A logical-pixel coordinate or extent.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct LogicalPx(f32);

impl LogicalPx {
    pub const fn get(self) -> f32 {
        self.0
    }

    fn from_i64(value: i64) -> Self {
        Self(value as f32)
    }
}

/// A stored character-column coordinate, distinct from pixels.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Column(i64);

impl Column {
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> i64 {
        self.0
    }
}

/// A stored character-line coordinate, distinct from pixels.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Line(i64);

impl Line {
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> i64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FrameLogicalSpace;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WindowLocalSpace;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WindowBodySpace;

/// A logical-pixel point whose coordinate space is part of its type.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PixelPoint<Space> {
    x: LogicalPx,
    y: LogicalPx,
    space: PhantomData<Space>,
}

impl<Space> PixelPoint<Space> {
    fn new(x: f32, y: f32) -> Result<Self, GeometryError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(GeometryError::NonFiniteCoordinate);
        }
        Ok(Self {
            x: LogicalPx(x),
            y: LogicalPx(y),
            space: PhantomData,
        })
    }

    pub const fn x(self) -> LogicalPx {
        self.x
    }

    pub const fn y(self) -> LogicalPx {
        self.y
    }
}

/// A finite, nonnegative-extent logical-pixel rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PixelRect<Space> {
    origin: PixelPoint<Space>,
    width: LogicalPx,
    height: LogicalPx,
}

impl<Space> PixelRect<Space> {
    fn from_raw(rect: &Rect) -> Result<Self, GeometryError> {
        if rect.x < 0.0
            || rect.y < 0.0
            || !rect.width.is_finite()
            || !rect.height.is_finite()
            || rect.width < 0.0
            || rect.height < 0.0
        {
            return Err(GeometryError::InvalidExtent);
        }
        Ok(Self {
            origin: PixelPoint::new(rect.x, rect.y)?,
            width: LogicalPx(rect.width),
            height: LogicalPx(rect.height),
        })
    }

    pub const fn origin(self) -> PixelPoint<Space> {
        self.origin
    }

    pub const fn width(self) -> LogicalPx {
        self.width
    }

    pub const fn height(self) -> LogicalPx {
        self.height
    }
}

/// Independent stored cell-grid origin for one window.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CellOrigin {
    column: Column,
    line: Line,
}

impl CellOrigin {
    pub const fn new(column: i64, line: i64) -> Self {
        Self {
            column: Column::new(column),
            line: Line::new(line),
        }
    }

    pub const fn column(self) -> Column {
        self.column
    }

    pub const fn line(self) -> Line {
        self.line
    }
}

/// A frame-owned point.  The owner prevents points from different frames from
/// being treated as interchangeable merely because both are frame-relative.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FramePoint {
    frame: FrameId,
    point: PixelPoint<FrameLogicalSpace>,
}

impl FramePoint {
    pub const fn frame(self) -> FrameId {
        self.frame
    }

    pub const fn x(self) -> LogicalPx {
        self.point.x()
    }

    pub const fn y(self) -> LogicalPx {
        self.point.y()
    }
}

/// A window-owned point in a statically named window coordinate space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowPoint<Space> {
    window: WindowId,
    point: PixelPoint<Space>,
}

impl<Space> WindowPoint<Space> {
    pub const fn window(self) -> WindowId {
        self.window
    }

    pub const fn x(self) -> LogicalPx {
        self.point.x()
    }

    pub const fn y(self) -> LogicalPx {
        self.point.y()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeometryError {
    SnapshotWindowMismatch,
    NonFiniteCoordinate,
    InvalidExtent,
}

/// Typed geometry for one visible source position in a window snapshot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapshotPointGeometry {
    buffer_pos: LispCharPos1,
    body_point: WindowPoint<WindowBodySpace>,
    frame_point: FramePoint,
    width: LogicalPx,
    height: LogicalPx,
    row: i64,
    column: i64,
}

impl SnapshotPointGeometry {
    pub const fn buffer_pos(self) -> LispCharPos1 {
        self.buffer_pos
    }

    pub const fn in_text_body(self) -> WindowPoint<WindowBodySpace> {
        self.body_point
    }

    pub const fn in_frame(self) -> FramePoint {
        self.frame_point
    }

    pub const fn width(self) -> LogicalPx {
        self.width
    }

    pub const fn height(self) -> LogicalPx {
        self.height
    }

    pub const fn row(self) -> i64 {
        self.row
    }

    pub const fn column(self) -> i64 {
        self.column
    }
}

/// A borrowed, coordinate-safe view over one live window and its last
/// redisplay snapshot.
///
/// It deliberately carries no `PresentationId` yet: the current evaluator
/// snapshot storage does not publish one.  Adding that identity is the next
/// migration step; naming this a snapshot view avoids inventing authority.
pub struct SnapshotWindowGeometry<'a> {
    presentation: PresentationId,
    frame: FrameId,
    window: WindowId,
    snapshot: &'a WindowDisplaySnapshot,
    outer: PixelRect<FrameLogicalSpace>,
}

impl<'a> SnapshotWindowGeometry<'a> {
    pub fn new(
        presentation: PresentationId,
        frame: FrameId,
        window: WindowId,
        snapshot: &'a WindowDisplaySnapshot,
    ) -> Result<Self, GeometryError> {
        if window != snapshot.window_id {
            return Err(GeometryError::SnapshotWindowMismatch);
        }
        Ok(Self {
            presentation,
            frame,
            window,
            snapshot,
            outer: PixelRect::from_raw(&snapshot.regions.outer)?,
        })
    }

    pub const fn presentation(&self) -> PresentationId {
        self.presentation
    }

    pub const fn frame(&self) -> FrameId {
        self.frame
    }

    pub fn window(&self) -> WindowId {
        self.window
    }

    pub const fn outer_in_frame(&self) -> PixelRect<FrameLogicalSpace> {
        self.outer
    }

    pub fn cell_origin(&self) -> CellOrigin {
        self.snapshot.cell_origin
    }

    pub fn text_body_origin_in_window(&self) -> WindowPoint<WindowLocalSpace> {
        WindowPoint {
            window: self.window,
            point: PixelPoint {
                x: LogicalPx::from_i64(self.snapshot.text_area_left_offset),
                y: LogicalPx::from_i64(self.snapshot.top_chrome_height()),
                space: PhantomData,
            },
        }
    }

    pub fn text_body_origin_in_frame(&self) -> Result<FramePoint, GeometryError> {
        let local = self.text_body_origin_in_window();
        Ok(FramePoint {
            frame: self.frame,
            point: PixelPoint::new(
                self.outer.origin().x().get() + local.x().get(),
                self.outer.origin().y().get() + local.y().get(),
            )?,
        })
    }

    pub fn point_for_buffer_pos(
        &self,
        buffer_pos: LispCharPos1,
    ) -> Result<Option<SnapshotPointGeometry>, GeometryError> {
        self.snapshot
            .point_for_buffer_pos(buffer_pos)
            .map(|point| self.materialize_point(point))
            .transpose()
    }

    /// Resolve coordinates in GNU's current snapshot convention: X is
    /// text-body-local while Y is window-local.
    pub fn point_at_window_coords(
        &self,
        body_x: i64,
        window_y: i64,
    ) -> Result<Option<SnapshotPointGeometry>, GeometryError> {
        self.snapshot
            .point_at_coords(body_x, window_y)
            .as_ref()
            .map(|point| self.materialize_point(point))
            .transpose()
    }

    fn materialize_point(
        &self,
        point: &DisplayPointSnapshot,
    ) -> Result<SnapshotPointGeometry, GeometryError> {
        let body_point = WindowPoint {
            window: self.window,
            point: PixelPoint::new(
                LogicalPx::from_i64(point.x).get(),
                LogicalPx::from_i64(self.snapshot.text_area_relative_y(point.y)).get(),
            )?,
        };
        let body_origin = self.text_body_origin_in_frame()?;
        let frame_point = FramePoint {
            frame: self.frame,
            point: PixelPoint::new(
                body_origin.x().get() + body_point.x().get(),
                body_origin.y().get() + body_point.y().get(),
            )?,
        };
        Ok(SnapshotPointGeometry {
            buffer_pos: point.buffer_pos,
            body_point,
            frame_point,
            width: LogicalPx::from_i64(point.width),
            height: LogicalPx::from_i64(point.height),
            row: self.snapshot.text_area_relative_row(point.row),
            column: point.col,
        })
    }
}
