//! Shared context types for the `render_frame_glyphs` phase pipeline.
//!
//! A frame renders in two stages:
//! 1. CPU collection phases build per-layer vertex sets (`layer_backgrounds`,
//!    `layer_chrome`) from the frame glyph buffer.
//! 2. A single render pass draws the layers in the documented z-order
//!    (backgrounds -> text -> decorations -> borders -> media -> cursors);
//!    the pass-side phases live in `layer_text`, `layer_media`, and
//!    `layer_chrome`.
//!
//! [`FrameParams`] carries the immutable per-frame inputs through both
//! stages; [`FramePassCtx`] bundles the active render pass with those
//! params for the draw phases.

use neomacs_display_protocol::types::FaceId;
use std::collections::HashMap;

use neomacs_display_protocol::DeviceScale;
use neomacs_display_protocol::face::Face;
use neomacs_display_protocol::frame_glyphs::{FrameGlyphBuffer, GlyphRowRole};
use neomacs_display_protocol::types::{AnimatedCursor, Color, Rect};

use super::super::vertex::RectVertex;
use super::cursor_presentation::InverseVideoCell;
use super::pointer_override::PointerOverrideResolver;

/// Immutable per-frame inputs shared by every render phase.
pub(super) struct FrameParams<'a> {
    pub(super) frame_glyphs: &'a FrameGlyphBuffer,
    pub(super) pointer_override: PointerOverrideResolver,
    pub(super) faces: &'a HashMap<FaceId, Face>,
    pub(super) cursor_visible: bool,
    pub(super) animated_cursor: &'a Option<AnimatedCursor>,
    /// Present-time inverse-video contract for the active filled-box cursor.
    /// `None` while its visual box is in flight, so text cannot be recolored
    /// at a destination the box has not reached.
    pub(super) cursor_inverse_video: Option<InverseVideoCell>,
    pub(super) mouse_pos: (f32, f32),
    // RGB-pair gradient endpoints; a dedicated type alias would add little here.
    #[allow(clippy::type_complexity)]
    pub(super) background_gradient: Option<((f32, f32, f32), (f32, f32, f32))>,
    /// Logical frame size from `prepare_frame_uniforms`.
    pub(super) logical_w: f32,
    pub(super) logical_h: f32,
    /// Native pixels per logical pixel for device-defined decoration widths.
    pub(super) device_scale: DeviceScale,
    pub(super) face_debug_call_id: u64,
    /// Whether line/scroll-spacing animations are active this frame
    /// (glyph Y positions then go through `line_y_offset`).
    pub(super) has_line_anims: bool,
    /// Layout row damage for this frame (built by display-runtime from the
    /// same FrameDisplayState the glyph buffer was materialized from).
    pub(super) row_damage: Option<&'a super::row_reuse::FrameRowDamage>,
}

/// The active render pass plus the per-frame params, handed to each draw
/// phase in z-order.
pub(super) struct FramePassCtx<'e, 'a> {
    pub(super) pass: wgpu::RenderPass<'e>,
    pub(super) params: &'e FrameParams<'a>,
}

/// A merged span of adjacent boxed glyphs on the same row.
pub(super) struct BoxSpan {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) face_id: FaceId,
    pub(super) row_role: GlyphRowRole,
    pub(super) bg: Option<Color>,
    pub(super) clip: Option<Rect>,
    pub(super) policy: BoxPaintPolicy,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) enum BoxPaintPolicy {
    Rounded,
    SharpSameFace,
    SharpContinuousChrome,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct BoxGroupKey {
    row_role: GlyphRowRole,
    y: u32,
    height: u32,
    clip: Option<(u32, u32, u32, u32)>,
    policy: BoxPaintPolicy,
    face: Option<FaceId>,
}

impl BoxGroupKey {
    fn new(span: &BoxSpan) -> Self {
        let clip = span.clip.map(|clip| {
            (
                clip.x.to_bits(),
                clip.y.to_bits(),
                clip.width.to_bits(),
                clip.height.to_bits(),
            )
        });
        let face = (span.policy != BoxPaintPolicy::SharpContinuousChrome).then_some(span.face_id);
        Self {
            row_role: span.row_role,
            y: span.y.to_bits(),
            height: span.height.to_bits(),
            clip,
            policy: span.policy,
            face,
        }
    }
}

/// O(1)-average semantic grouping while retaining first-contribution output
/// order. Continuous sharp chrome deliberately ignores face identity and
/// keeps the first span's face as the selected border material.
#[derive(Default)]
pub(super) struct BoxSpanAccumulator {
    spans: Vec<BoxSpan>,
    open_group: HashMap<BoxGroupKey, usize>,
}

impl BoxSpanAccumulator {
    pub(super) fn push(&mut self, candidate: BoxSpan) {
        let key = BoxGroupKey::new(&candidate);
        if let Some(&index) = self.open_group.get(&key) {
            let existing = &mut self.spans[index];
            if ((existing.x + existing.width) - candidate.x).abs() < 1.0 {
                existing.width = candidate.x + candidate.width - existing.x;
                return;
            }
        }
        let index = self.spans.len();
        self.spans.push(candidate);
        self.open_group.insert(key, index);
    }

    pub(super) fn finish(self) -> Vec<BoxSpan> {
        self.spans
    }

    #[cfg(test)]
    pub(super) fn group_count(&self) -> usize {
        self.open_group.len()
    }
}

/// All merged box spans of a frame.
pub(super) struct BoxSpanSet {
    pub(super) spans: Vec<BoxSpan>,
}

/// Cursor and window-border vertex sets collected before the render pass.
/// `cursor_bg` and `behind_text_cursor` draw before text (inverse-video filled
/// box cursor); `cursors` draws after text and also carries the scroll bar
/// track + thumb rects, which all go through the rect pipeline.
pub(super) struct ChromeLayerVertices {
    pub(super) cursor_bg: Vec<RectVertex>,
    pub(super) behind_text_cursor: Vec<RectVertex>,
    pub(super) cursors: Vec<RectVertex>,
}
