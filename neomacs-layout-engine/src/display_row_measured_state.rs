use crate::display_row_render_state::{
    DisplayRowOutputProgress, RenderedDisplayRow, RenderedDisplayRowMedia,
};
use neomacs_display_protocol::glyph_matrix::GlyphRow;
use neomacs_display_protocol::types::Rect;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
// The shared `Line` suffix is domain-meaningful (tab/header/mode LINES); renaming
// the variants would obscure intent, so the naming lint is allowed here.
#[allow(clippy::enum_variant_names)]
pub(crate) enum WindowChromeKind {
    TabLine,
    HeaderLine,
    ModeLine,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FrameChromeKind {
    TabBar,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayRowOwner {
    WindowChrome {
        window_id: u64,
        kind: WindowChromeKind,
    },
    FrameChrome {
        kind: FrameChromeKind,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayRowBoundsPolicy {
    /// Legacy allocation behavior retained only by lifecycle unit fixtures.
    #[cfg(test)]
    PreserveAllocatedMinimum,
    /// Use the row produced by shaping, including its face metrics, display
    /// properties, and media.  A previous window allocation is not intrinsic
    /// content and must not prevent a chrome row from shrinking.
    MeasureIntrinsic,
    /// Ignore row layout metrics and fit only the materialized glyph/media
    /// content.  Frame chrome uses this while converging its reserved band.
    MeasureContent,
}

pub(crate) struct MeasuredDisplayRow {
    owner: DisplayRowOwner,
    row_index: u32,
    bounds: Rect,
    rendered: RenderedDisplayRow,
}

/// The two coordinate-space views of media embedded in a measured display row.
///
/// Display-row shaping starts at a row-local horizontal origin, while its
/// vertical baseline is expressed in the render attempt's coordinate space.
/// Row owners can subsequently move the measured row (most notably a tall,
/// bottom-anchored mode line).  Keeping this conversion here makes the measured
/// row the single authority for placing both its glyph row and its media.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MeasuredDisplayRowMediaPlacement {
    row_local_bounds: Rect,
    frame_bounds: Rect,
}

impl MeasuredDisplayRowMediaPlacement {
    pub(crate) fn row_local_bounds(self) -> Rect {
        self.row_local_bounds
    }

    pub(crate) fn frame_bounds(self) -> Rect {
        self.frame_bounds
    }
}

fn stable_pixel_ceil(px: f32) -> f32 {
    if !px.is_finite() {
        return 1.0;
    }
    let px = px.max(1.0);
    let rounded = px.round();
    if (px - rounded).abs() <= 0.01 {
        rounded.max(1.0)
    } else {
        px.ceil().max(1.0)
    }
}

fn rendered_display_row_content_height(rendered: &RenderedDisplayRow) -> f32 {
    let mut max_ascent = 0.0_f32;
    let mut max_descent = 0.0_f32;
    for glyph in rendered.row().glyphs.iter().flatten() {
        if glyph.padding {
            continue;
        }
        let glyph_metrics = (glyph.pixel_height > 0.0).then_some((
            glyph.pixel_ascent.max(0.0).min(glyph.pixel_height),
            (glyph.pixel_height - glyph.pixel_ascent).max(0.0),
        ));
        let face_metrics = rendered
            .faces()
            .iter()
            .find(|face| face.id == glyph.face_id)
            .map(|face| {
                (
                    face.font_ascent.max(0) as f32,
                    face.font_descent.max(0) as f32,
                )
            });
        if let Some((ascent, descent)) = glyph_metrics.or(face_metrics) {
            // GNU records a raised/lowered glyph as baseline-relative ascent
            // and descent, then takes the maxima independently.  Adding the
            // absolute offset to a scalar height over-counts the side that
            // moved back inside the row.
            max_ascent = max_ascent.max((ascent - glyph.vertical_offset_px).max(0.0));
            max_descent = max_descent.max((descent + glyph.vertical_offset_px).max(0.0));
        }
    }
    let mut height = (max_ascent + max_descent).max(1.0);
    for media in rendered.media() {
        height = height.max(media.height);
    }
    height
}

/// Compute the row height a `MeasuredDisplayRow` would resolve for the given
/// rendered row, fallback height, and bounds policy — without consuming the
/// rendered row. The chrome render uses this to learn a row's real height the
/// moment it is built (before wrapping it for install), so the bottom-anchored
/// mode line can be pinned to the window bottom and its measured height reported
/// as `window-mode-line-height`. Sharing this helper with `MeasuredDisplayRow`
/// guarantees the reported height equals the height the installed row renders
/// at.
pub(crate) fn measured_display_row_height(
    rendered: &RenderedDisplayRow,
    fallback_height: f32,
    bounds_policy: DisplayRowBoundsPolicy,
) -> f32 {
    let content_height = stable_pixel_ceil(rendered_display_row_content_height(rendered));
    #[cfg(test)]
    let allocated_height = stable_pixel_ceil(
        fallback_height
            .max(rendered.row().height_px)
            .max(rendered.progress().height())
            .max(content_height),
    );
    #[cfg(not(test))]
    let _ = fallback_height;
    match bounds_policy {
        #[cfg(test)]
        DisplayRowBoundsPolicy::PreserveAllocatedMinimum => allocated_height,
        DisplayRowBoundsPolicy::MeasureIntrinsic => stable_pixel_ceil(
            rendered
                .row()
                .height_px
                .max(rendered.progress().height())
                .max(content_height),
        ),
        DisplayRowBoundsPolicy::MeasureContent => content_height,
    }
    .max(1.0)
}

impl MeasuredDisplayRow {
    pub(crate) fn new(
        owner: DisplayRowOwner,
        row_index: u32,
        fallback_bounds: Rect,
        rendered: RenderedDisplayRow,
        bounds_policy: DisplayRowBoundsPolicy,
    ) -> Self {
        let height = measured_display_row_height(&rendered, fallback_bounds.height, bounds_policy);
        Self {
            owner,
            row_index,
            bounds: Rect::new(
                fallback_bounds.x,
                fallback_bounds.y,
                fallback_bounds.width,
                height,
            ),
            rendered,
        }
    }

    pub(crate) fn owner(&self) -> DisplayRowOwner {
        self.owner
    }

    pub(crate) fn row_index(&self) -> u32 {
        self.row_index
    }

    pub(crate) fn bounds(&self) -> Rect {
        self.bounds
    }

    pub(crate) fn rendered(&self) -> &RenderedDisplayRow {
        &self.rendered
    }

    /// Place rendered media using this row's final measured bounds.
    ///
    /// `RenderedDisplayRowMedia::x` is relative to the display-row start.
    /// Its `y` is based on the provisional render baseline, so first normalize
    /// it against the rendered row's origin and then translate both axes by the
    /// final measured-row origin.  This also follows a mode line when intrinsic
    /// measurement re-anchors it at the bottom of its window.
    pub(crate) fn place_media(
        &self,
        medium: &RenderedDisplayRowMedia,
    ) -> MeasuredDisplayRowMediaPlacement {
        let row_local_bounds = Rect::new(
            medium.x,
            medium.y - self.rendered.progress().y(),
            medium.width,
            medium.height,
        );
        let frame_bounds = Rect::new(
            self.bounds.x + row_local_bounds.x,
            self.bounds.y + row_local_bounds.y,
            row_local_bounds.width,
            row_local_bounds.height,
        );
        MeasuredDisplayRowMediaPlacement {
            row_local_bounds,
            frame_bounds,
        }
    }

    pub(crate) fn row_height(&self) -> f32 {
        self.bounds.height.max(1.0)
    }

    pub(crate) fn row_ascent(&self) -> f32 {
        self.rendered
            .row()
            .ascent_px
            .max(0.0)
            .min(self.row_height())
    }

    pub(crate) fn output_progress(&self) -> DisplayRowOutputProgress {
        self.rendered
            .progress()
            .with_y(self.bounds.y)
            .with_height(self.bounds.height)
    }

    pub(crate) fn frame_chrome_output_row(&self) -> GlyphRow {
        self.rendered
            .materialize_output_row(0.0, self.row_height(), self.row_ascent())
    }

    pub(crate) fn window_relative_output_row(&self, window_bounds: Rect) -> GlyphRow {
        self.rendered.materialize_output_row(
            self.bounds.y - window_bounds.y,
            self.row_height(),
            self.row_ascent(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display_row_render_state::{RenderedDisplayRowMedia, RenderedDisplayRowMediaKind};
    use neomacs_display_protocol::face::Face;
    use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
    use neomacs_display_protocol::glyph_matrix::{Glyph, GlyphArea};
    use neomacs_display_protocol::types::FaceId;

    #[test]
    fn content_height_uses_the_font_metrics_stamped_on_the_glyph() {
        let face_id = FaceId::new(31);
        let mut row = GlyphRow::new(GlyphRowRole::TabLine);
        row.enabled = true;
        row.height_px = 12.0;
        row.ascent_px = 9.0;
        row.glyphs[GlyphArea::Text.index()].push(
            Glyph::char('\u{e632}', face_id, 0)
                .with_pixel_geometry(10.0, 12.0, 9.0)
                .with_vertical_offset(-1.0),
        );

        // The face's primary font is taller than the concrete fontset fallback
        // that contains this Nerd Font character.  GNU sizes the row from the
        // font selected for the glyph, not from the face recipe.
        let mut face = Face::new(face_id);
        face.font_ascent = 11;
        face.font_descent = 3;
        let rendered = RenderedDisplayRow::new(
            row,
            DisplayRowOutputProgress::new(10.0, 1, 0.0, 12.0),
            Vec::new(),
            vec![face],
            Vec::new(),
        );

        assert_eq!(rendered_display_row_content_height(&rendered), 12.0);
    }

    #[test]
    fn media_placement_uses_final_row_origin_on_both_axes() {
        let mut row = GlyphRow::new(GlyphRowRole::ModeLine);
        row.enabled = true;
        row.height_px = 24.0;
        row.ascent_px = 18.0;
        let medium = RenderedDisplayRowMedia {
            kind: RenderedDisplayRowMediaKind::Xwidget { xwidget_id: 91 },
            x: 8.0,
            y: 45.0,
            col: 1,
            width: 32.0,
            height: 18.0,
        };
        let measured = MeasuredDisplayRow::new(
            DisplayRowOwner::WindowChrome {
                window_id: 7,
                kind: WindowChromeKind::ModeLine,
            },
            9,
            // The owner moved the final row away from both the horizontal
            // origin and the provisional render Y (40).
            Rect::new(320.0, 100.0, 280.0, 24.0),
            RenderedDisplayRow::new(
                row,
                DisplayRowOutputProgress::new(40.0, 2, 40.0, 24.0),
                Vec::new(),
                Vec::new(),
                vec![medium.clone()],
            ),
            DisplayRowBoundsPolicy::PreserveAllocatedMinimum,
        );

        let placement = measured.place_media(&medium);

        assert_eq!(
            placement.row_local_bounds(),
            Rect::new(8.0, 5.0, 32.0, 18.0)
        );
        assert_eq!(
            placement.frame_bounds(),
            Rect::new(328.0, 105.0, 32.0, 18.0)
        );
    }
}
