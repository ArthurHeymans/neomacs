use neomacs_display_protocol::frame_chrome::FrameChromeKind;
use neomacs_display_protocol::frame_glyphs::{PresentedCellOrigin, PresentedWindowGeometry};
use neomacs_display_protocol::glyph_matrix::FrameDisplayState;
use neomacs_display_protocol::{
    DisplayWindowId, FrameRect, PresentedHitError, PresentedHitIndex, PresentedHitRegion,
    PresentedRegionKind, PresentedTextPosition,
};
use neovm_core::window::WindowDisplaySnapshot;

/// All spatial products compiled from one completed redisplay snapshot.
///
/// The plan is deliberately not a transport object: it can only be consumed
/// by [`Self::seal`], which installs window metadata and hit-test geometry as
/// one validated operation.
pub(crate) struct PresentationSpatialPlan {
    windows: Vec<(DisplayWindowId, PresentedWindowGeometry)>,
    hit_index: PresentedHitIndex,
}

impl PresentationSpatialPlan {
    pub(crate) fn compile(
        state: &FrameDisplayState,
        snapshots: &[WindowDisplaySnapshot],
    ) -> Result<Self, PresentedHitError> {
        let mut windows = Vec::new();
        let mut regions = Vec::new();
        let mut positions = Vec::new();

        for (window_z, info) in state.window_infos.iter().enumerate() {
            let Some(snapshot) = snapshots
                .iter()
                .find(|snapshot| snapshot.window_id.0 as i64 == info.window_id.get())
            else {
                continue;
            };
            let cell_origin = PresentedCellOrigin {
                column: snapshot.cell_origin.column().get(),
                line: snapshot.cell_origin.line().get(),
            };
            let geometry = if snapshot.regions_materialized {
                PresentedWindowGeometry::Complete {
                    cell_origin,
                    regions: snapshot.regions,
                }
            } else {
                PresentedWindowGeometry::Skipped {
                    cell_origin,
                    outer: snapshot.regions.outer,
                }
            };
            windows.push((info.window_id, geometry));

            if !snapshot.regions_materialized {
                continue;
            }
            let window_regions = snapshot.regions;
            let base_z = i32::try_from(window_z)
                .unwrap_or(i32::MAX)
                .saturating_mul(100);
            let window = Some(info.window_id);
            push_region(
                &mut regions,
                window,
                PresentedRegionKind::TextBody,
                window_regions.text_body,
                base_z,
            )?;
            for (kind, rect, priority) in [
                (
                    PresentedRegionKind::LeftMargin,
                    window_regions.left_margin,
                    10,
                ),
                (
                    PresentedRegionKind::RightMargin,
                    window_regions.right_margin,
                    10,
                ),
                (
                    PresentedRegionKind::LeftFringe,
                    window_regions.left_fringe,
                    10,
                ),
                (
                    PresentedRegionKind::RightFringe,
                    window_regions.right_fringe,
                    10,
                ),
                (
                    PresentedRegionKind::LeftScrollBar,
                    window_regions.left_scroll_bar,
                    20,
                ),
                (
                    PresentedRegionKind::RightScrollBar,
                    window_regions.right_scroll_bar,
                    20,
                ),
                (
                    PresentedRegionKind::HorizontalScrollBar,
                    window_regions.horizontal_scroll_bar,
                    20,
                ),
                (PresentedRegionKind::TabLine, window_regions.tab_line, 20),
                (
                    PresentedRegionKind::HeaderLine,
                    window_regions.header_line,
                    20,
                ),
                (PresentedRegionKind::ModeLine, window_regions.mode_line, 20),
                (
                    PresentedRegionKind::RightDivider,
                    window_regions.right_divider,
                    30,
                ),
                (
                    PresentedRegionKind::BottomDivider,
                    window_regions.bottom_divider,
                    30,
                ),
            ] {
                if let Some(rect) = rect {
                    push_region(&mut regions, window, kind, rect, base_z + priority)?;
                }
            }

            for point in &snapshot.points {
                let body_row = snapshot
                    .body_rows
                    .iter()
                    .find(|row| row.output_row == point.row)
                    .ok_or(PresentedHitError::MissingBodyRow {
                        window: info.window_id,
                        output_row: point.row,
                    })?;
                let raw_x = window_regions.text_body.x + point.x as f32;
                let raw_y = window_regions.text_body.y + body_row.body_y as f32;
                let left = raw_x.max(window_regions.text_body.x);
                let top = raw_y.max(window_regions.text_body.y);
                let right = (raw_x + point.width.max(1) as f32)
                    .min(window_regions.text_body.x + window_regions.text_body.width);
                let bottom = (raw_y + point.height.max(1) as f32)
                    .min(window_regions.text_body.y + window_regions.text_body.height);
                if right <= left || bottom <= top {
                    continue;
                }
                let bounds = FrameRect::new(left, top, right - left, bottom - top)
                    .map_err(|_| PresentedHitError::InvalidTextPositionGeometry)?;
                positions.push(PresentedTextPosition::new(
                    info.window_id,
                    bounds,
                    point.buffer_pos.as_i64(),
                    body_row.body_row,
                    point.col,
                ));
            }
            push_row_fallback_positions(
                &mut positions,
                info.window_id,
                snapshot,
                window_regions.text_body,
            )?;
        }

        for band in state.frame_chrome.bands() {
            let kind = match band.kind() {
                FrameChromeKind::MenuBar => PresentedRegionKind::MenuBar,
                FrameChromeKind::ToolBar => PresentedRegionKind::ToolBar,
                FrameChromeKind::CompactBar => PresentedRegionKind::CompactBar,
                FrameChromeKind::TabBar => PresentedRegionKind::TabBar,
            };
            regions.push(PresentedHitRegion::new(None, kind, band.bounds(), i32::MAX));
        }

        Ok(Self {
            windows,
            hit_index: PresentedHitIndex::from_parts(state.presentation_id, regions, positions)?,
        })
    }

    #[cfg(test)]
    pub(crate) fn hit_index(&self) -> &PresentedHitIndex {
        &self.hit_index
    }

    pub(crate) fn seal(self, state: &mut FrameDisplayState) -> Result<(), PresentedHitError> {
        for (window, geometry) in self.windows {
            let Some(info) = state
                .window_infos
                .iter_mut()
                .find(|info| info.window_id == window)
            else {
                continue;
            };
            info.geometry = geometry;
            match geometry {
                PresentedWindowGeometry::Complete { regions, .. } => {
                    info.bounds = regions.outer;
                    info.tab_line_height = regions.tab_line.map_or(0.0, |rect| rect.height);
                    info.header_line_height = regions.header_line.map_or(0.0, |rect| rect.height);
                    info.mode_line_height = regions.mode_line.map_or(0.0, |rect| rect.height);
                }
                PresentedWindowGeometry::Skipped { outer, .. } => {
                    info.bounds = outer;
                    info.tab_line_height = 0.0;
                    info.header_line_height = 0.0;
                    info.mode_line_height = 0.0;
                }
            }
        }
        state.presented_hit_index = self.hit_index;
        state.validate_spatial_projections()
    }
}

/// Fill the source-position gaps that have no glyph rectangle of their own.
///
/// Newlines and empty display rows do not produce glyphs, while the area after
/// the last glyph on a row is still clickable.  These spans make the sealed
/// presentation's hit map total over every visible body row.  Exact glyph
/// positions are installed first and therefore win if a font's ink/advance
/// geometry overlaps one of these fallback spans.
fn push_row_fallback_positions(
    positions: &mut Vec<PresentedTextPosition>,
    window: DisplayWindowId,
    snapshot: &WindowDisplaySnapshot,
    text_body: neomacs_display_protocol::Rect,
) -> Result<(), PresentedHitError> {
    if text_body.width <= 0.0 || text_body.height <= 0.0 {
        return Ok(());
    }
    let body_left = text_body.x;
    let body_right = text_body.x + text_body.width;

    for row in &snapshot.rows {
        let Some(row_anchor) = row.start_buffer_pos.or(row.end_buffer_pos) else {
            continue;
        };
        let (body_row, body_y) = snapshot.text_body_position(row.row, row.y);
        let top = (text_body.y + body_y as f32).max(text_body.y);
        let bottom = (text_body.y + body_y as f32 + row.height.max(1) as f32)
            .min(text_body.y + text_body.height);
        if bottom <= top {
            continue;
        }

        let mut row_points = snapshot
            .points
            .iter()
            .filter(|point| point.row == row.row)
            .collect::<Vec<_>>();
        row_points.sort_by_key(|point| (point.x, point.col, point.buffer_pos));

        let mut covered_right = body_left;
        let mut preceding = None;
        for point in row_points {
            let point_left = (text_body.x + point.x as f32).clamp(body_left, body_right);
            let point_right = (text_body.x + point.x as f32 + point.width.max(1) as f32)
                .clamp(body_left, body_right);
            if point_left > covered_right {
                let (buffer_position, column) = preceding.map_or(
                    (point.buffer_pos.as_i64(), point.col),
                    |previous: &neovm_core::window::DisplayPointSnapshot| {
                        (previous.buffer_pos.as_i64(), previous.col)
                    },
                );
                push_text_position_span(
                    positions,
                    window,
                    covered_right,
                    top,
                    point_left - covered_right,
                    bottom - top,
                    buffer_position,
                    body_row,
                    column,
                )?;
            }
            covered_right = covered_right.max(point_right);
            preceding = Some(point);
        }

        if covered_right < body_right {
            let (buffer_position, column) = preceding
                .map_or((row_anchor.as_i64(), row.start_col), |point| {
                    (point.buffer_pos.as_i64(), point.col)
                });
            push_text_position_span(
                positions,
                window,
                covered_right,
                top,
                body_right - covered_right,
                bottom - top,
                buffer_position,
                body_row,
                column,
            )?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn push_text_position_span(
    positions: &mut Vec<PresentedTextPosition>,
    window: DisplayWindowId,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    buffer_position: i64,
    row: i64,
    column: i64,
) -> Result<(), PresentedHitError> {
    if width <= 0.0 || height <= 0.0 {
        return Ok(());
    }
    let bounds = FrameRect::new(x, y, width, height)
        .map_err(|_| PresentedHitError::InvalidTextPositionGeometry)?;
    positions.push(PresentedTextPosition::new(
        window,
        bounds,
        buffer_position,
        row,
        column,
    ));
    Ok(())
}

fn push_region(
    regions: &mut Vec<PresentedHitRegion>,
    window: Option<DisplayWindowId>,
    kind: PresentedRegionKind,
    rect: neomacs_display_protocol::Rect,
    z_order: i32,
) -> Result<(), PresentedHitError> {
    if rect.width == 0.0 || rect.height == 0.0 {
        return Ok(());
    }
    let bounds = FrameRect::new(rect.x, rect.y, rect.width, rect.height)
        .map_err(|_| PresentedHitError::InvalidRegionGeometry)?;
    regions.push(PresentedHitRegion::new(window, kind, bounds, z_order));
    Ok(())
}
