//! Typed output install requests consumed by `DisplayOutputBuilder`.

use neomacs_display_protocol::effect_config::EffectsConfig;
use neomacs_display_protocol::face::Face;
use neomacs_display_protocol::frame_glyphs::{
    CursorStyle, DisplaySlotId, GlyphRowRole, PhysCursor, PresentedCellOrigin,
    PresentedWindowRegions, WindowEffectHint, WindowInfo, WindowTransitionHint,
};
use neomacs_display_protocol::glyph_matrix::{CursorItem, FaceFillItem, ScrollBarItem};
use neomacs_display_protocol::types::FaceId;
use neomacs_display_protocol::types::{
    Color, DisplayFrameId, DisplayWindowId, ImageId, Rect, VideoId, XwidgetId,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum OutputMediaInstallKind {
    Image {
        image_id: ImageId,
    },
    Video {
        video_id: VideoId,
        loop_count: i32,
        autoplay: bool,
    },
    Xwidget {
        xwidget_id: XwidgetId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OutputMediaInstallRequest {
    pub(crate) target: ResolvedOutputMediaInstallTarget,
    pub(crate) kind: OutputMediaInstallKind,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResolvedOutputMediaInstallTarget {
    pub(crate) window_id: DisplayWindowId,
    pub(crate) role: GlyphRowRole,
    pub(crate) clip: Option<Rect>,
    pub(crate) slot_id: DisplaySlotId,
}

impl OutputMediaInstallRequest {
    pub(crate) fn new(
        target: ResolvedOutputMediaInstallTarget,
        kind: OutputMediaInstallKind,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            target,
            kind,
            x,
            y,
            width,
            height,
        }
    }

    pub(crate) fn image(
        target: ResolvedOutputMediaInstallTarget,
        image_id: ImageId,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Self {
        Self::new(
            target,
            OutputMediaInstallKind::Image { image_id },
            x,
            y,
            width,
            height,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn video(
        target: ResolvedOutputMediaInstallTarget,
        video_id: VideoId,
        loop_count: i32,
        autoplay: bool,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Self {
        Self::new(
            target,
            OutputMediaInstallKind::Video {
                video_id,
                loop_count,
                autoplay,
            },
            x,
            y,
            width,
            height,
        )
    }

    pub(crate) fn xwidget(
        target: ResolvedOutputMediaInstallTarget,
        xwidget_id: XwidgetId,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Self {
        Self::new(
            target,
            OutputMediaInstallKind::Xwidget { xwidget_id },
            x,
            y,
            width,
            height,
        )
    }
}

impl ResolvedOutputMediaInstallTarget {
    pub(crate) fn new(
        window_id: DisplayWindowId,
        role: GlyphRowRole,
        clip: Option<Rect>,
        slot_id: DisplaySlotId,
    ) -> Self {
        Self {
            window_id,
            role,
            clip,
            slot_id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OutputCursorInstallRequest {
    window_id: DisplayWindowId,
    slot_id: DisplaySlotId,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    style: CursorStyle,
    color: Color,
}

impl OutputCursorInstallRequest {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        window_id: DisplayWindowId,
        slot_id: DisplaySlotId,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: CursorStyle,
        color: Color,
    ) -> Self {
        Self {
            window_id,
            slot_id,
            x,
            y,
            width,
            height,
            style,
            color,
        }
    }

    pub(crate) fn cursor_item(self) -> CursorItem {
        CursorItem {
            window_id: self.window_id,
            slot_id: self.slot_id,
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            style: self.style,
            color: self.color,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum OutputFrameArtifactInstallRequest {
    Background {
        bounds: Rect,
        color: Color,
    },
    FaceFill(FaceFillItem),
    Border {
        window_id: DisplayWindowId,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    },
    ScrollBar(ScrollBarItem),
    WindowInfo(WindowInfo),
    TransitionHint(WindowTransitionHint),
    EffectHint(WindowEffectHint),
    PhysCursor(PhysCursor),
}

impl OutputFrameArtifactInstallRequest {
    pub(crate) fn phys_cursor(cursor: PhysCursor) -> Self {
        Self::PhysCursor(cursor)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct OutputFrameIdentityInstallRequest {
    pub(crate) frame_id: DisplayFrameId,
    pub(crate) parent_id: DisplayFrameId,
    pub(crate) parent_x: f32,
    pub(crate) parent_y: f32,
    pub(crate) z_order: i32,
    pub(crate) undecorated: bool,
    pub(crate) border_width: f32,
    pub(crate) border_color: Color,
    pub(crate) background_alpha: f32,
    pub(crate) no_accept_focus: bool,
}

#[derive(Clone, Debug)]
// Large payload variant; boxing is a perf hint deferred out of the lint gate.
#[allow(clippy::large_enum_variant)]
pub(crate) enum OutputFrameStateInstallRequest {
    Identity(OutputFrameIdentityInstallRequest),
    BackgroundColor(Color),
    FontPixelSize(f32),
    Face {
        id: FaceId,
        face: Face,
    },
    CursorEffects {
        window_id: DisplayWindowId,
        effects: EffectsConfig,
    },
}

impl OutputFrameStateInstallRequest {
    pub(crate) fn face(id: FaceId, face: Face) -> Self {
        Self::Face { id, face }
    }

    pub(crate) fn cursor_effects(window_id: DisplayWindowId, effects: EffectsConfig) -> Self {
        Self::CursorEffects { window_id, effects }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OutputTextWindowDisplayRangeInstallRequest {
    pub(crate) window_id: DisplayWindowId,
    pub(crate) window_start: i64,
    pub(crate) window_end: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OutputRetryCheckpointRestoreRequest {
    pub(crate) transition_hints_len: usize,
    pub(crate) effect_hints_len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OutputPresentedWindowGeometryInstallRequest {
    pub(crate) window_id: DisplayWindowId,
    pub(crate) cell_origin: PresentedCellOrigin,
    pub(crate) regions: PresentedWindowRegions,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum OutputWindowMetadataInstallRequest {
    TextDisplayRange(OutputTextWindowDisplayRangeInstallRequest),
    PresentedGeometry(OutputPresentedWindowGeometryInstallRequest),
    RestoreRetryCheckpoint(OutputRetryCheckpointRestoreRequest),
}

impl From<OutputPresentedWindowGeometryInstallRequest> for OutputWindowMetadataInstallRequest {
    fn from(request: OutputPresentedWindowGeometryInstallRequest) -> Self {
        Self::PresentedGeometry(request)
    }
}

impl OutputTextWindowDisplayRangeInstallRequest {
    pub(crate) fn new(window_id: DisplayWindowId, window_start: i64, window_end: i64) -> Self {
        Self {
            window_id,
            window_start,
            window_end,
        }
    }
}

impl OutputRetryCheckpointRestoreRequest {
    pub(crate) fn new(transition_hints_len: usize, effect_hints_len: usize) -> Self {
        Self {
            transition_hints_len,
            effect_hints_len,
        }
    }
}

impl From<OutputTextWindowDisplayRangeInstallRequest> for OutputWindowMetadataInstallRequest {
    fn from(request: OutputTextWindowDisplayRangeInstallRequest) -> Self {
        Self::TextDisplayRange(request)
    }
}

impl From<OutputRetryCheckpointRestoreRequest> for OutputWindowMetadataInstallRequest {
    fn from(request: OutputRetryCheckpointRestoreRequest) -> Self {
        Self::RestoreRetryCheckpoint(request)
    }
}
