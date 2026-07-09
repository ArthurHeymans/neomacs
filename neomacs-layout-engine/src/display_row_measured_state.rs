use crate::display_row_render_state::{DisplayRowOutputProgress, RenderedDisplayRow};
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
    PreserveAllocatedMinimum,
    MeasureContent,
}

pub(crate) struct MeasuredDisplayRow {
    owner: DisplayRowOwner,
    row_index: u32,
    bounds: Rect,
    rendered: RenderedDisplayRow,
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
    let mut height = 1.0_f32;
    for glyph in rendered.row().glyphs.iter().flatten() {
        if let Some(face) = rendered
            .faces()
            .iter()
            .find(|face| face.id == glyph.face_id)
        {
            let face_height = (face.font_ascent + face.font_descent).max(1) as f32;
            height = height.max(face_height + glyph.vertical_offset_px.abs());
        }
    }
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
    let allocated_height = stable_pixel_ceil(
        fallback_height
            .max(rendered.row().height_px)
            .max(rendered.progress().height())
            .max(content_height),
    );
    match bounds_policy {
        DisplayRowBoundsPolicy::PreserveAllocatedMinimum => allocated_height,
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

    pub(crate) fn absolute_output_row(&self) -> GlyphRow {
        self.rendered
            .materialize_output_row(self.bounds.y, self.row_height(), self.row_ascent())
    }

    pub(crate) fn window_relative_output_row(&self, window_bounds: Rect) -> GlyphRow {
        self.rendered.materialize_output_row(
            self.bounds.y - window_bounds.y,
            self.row_height(),
            self.row_ascent(),
        )
    }
}
