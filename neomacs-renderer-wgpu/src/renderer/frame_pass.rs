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

use neomacs_display_protocol::face::Face;
use neomacs_display_protocol::frame_glyphs::{FrameGlyphBuffer, GlyphRowRole};
use neomacs_display_protocol::types::{AnimatedCursor, Color, Rect};

use super::super::vertex::RectVertex;
use super::pointer_override::PointerOverrideResolver;

/// Immutable per-frame inputs shared by every render phase.
pub(super) struct FrameParams<'a> {
    pub(super) frame_glyphs: &'a FrameGlyphBuffer,
    pub(super) pointer_override: PointerOverrideResolver,
    pub(super) faces: &'a HashMap<FaceId, Face>,
    pub(super) cursor_visible: bool,
    pub(super) animated_cursor: &'a Option<AnimatedCursor>,
    pub(super) mouse_pos: (f32, f32),
    // RGB-pair gradient endpoints; a dedicated type alias would add little here.
    #[allow(clippy::type_complexity)]
    pub(super) background_gradient: Option<((f32, f32, f32), (f32, f32, f32))>,
    /// Logical frame size from `prepare_frame_uniforms`.
    pub(super) logical_w: f32,
    pub(super) logical_h: f32,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BoxPaintPolicy {
    Sharp,
    Rounded,
}

/// Insert a semantic box contribution independent of primitive traversal
/// order. Complement paints may appear between two alternate-face entries,
/// so adjacency is searched within the full matching group rather than only
/// against the last emitted entry.
pub(super) fn push_box_span(spans: &mut Vec<BoxSpan>, mut candidate: BoxSpan) {
    while let Some(index) = spans.iter().position(|span| {
        let same_group = span.face_id == candidate.face_id
            && span.row_role == candidate.row_role
            && span.clip == candidate.clip
            && span.policy == candidate.policy
            && (span.y - candidate.y).abs() < 0.5
            && (span.height - candidate.height).abs() < 0.5;
        let span_right = span.x + span.width;
        let candidate_right = candidate.x + candidate.width;
        same_group && candidate.x <= span_right + 1.0 && span.x <= candidate_right + 1.0
    }) {
        let existing = spans.remove(index);
        let right = (candidate.x + candidate.width).max(existing.x + existing.width);
        candidate.x = candidate.x.min(existing.x);
        candidate.width = right - candidate.x;
    }
    spans.push(candidate);
}

/// All merged box spans of a frame.
pub(super) struct BoxSpanSet {
    pub(super) spans: Vec<BoxSpan>,
}

/// Cursor, window border, and scroll bar vertex sets collected before the
/// render pass. `cursor_bg` and `behind_text_cursor` draw before text
/// (inverse-video filled box cursor); `cursors` draws after text.
pub(super) struct ChromeLayerVertices {
    pub(super) cursor_bg: Vec<RectVertex>,
    pub(super) behind_text_cursor: Vec<RectVertex>,
    pub(super) cursors: Vec<RectVertex>,
    /// Scroll bar thumbs as (x, y, w, h, corner_radius, color).
    pub(super) scroll_bar_thumbs: Vec<(f32, f32, f32, f32, f32, Color)>,
}
