//! Immutable lookup of transient paint overrides for one presented frame.

use super::super::vertex::{GlyphVertex, RectVertex, RoundedRectVertex, SubpixelGlyphVertex};
use neomacs_display_protocol::Color;
use neomacs_display_protocol::types::Rect;
use neomacs_display_protocol::{
    FaceId, FrameGlyphBuffer, FrameRect, PointerAppearancePhase, PointerAppearanceSelection,
    PointerDrawMode, PresentedPrimitiveKind,
};
#[cfg(test)]
use neomacs_display_protocol::{FrameGlyph, MaterializedFaceData};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PrimitivePointerOverride {
    mode: PointerDrawMode,
    clip: FrameRect,
}

impl PrimitivePointerOverride {
    pub(super) const fn mode(self) -> PointerDrawMode {
        self.mode
    }

    pub(super) const fn clip(self) -> FrameRect {
        self.clip
    }
}

#[cfg(test)]
pub(super) struct ResolvedGlyphPaint<'a> {
    primitive: &'a FrameGlyph,
    face_id: Option<FaceId>,
    materialized_face: MaterializedFaceData,
    clip: FrameRect,
}

#[cfg(test)]
impl<'a> ResolvedGlyphPaint<'a> {
    pub(super) const fn primitive(&self) -> &'a FrameGlyph {
        self.primitive
    }

    pub(super) const fn face_id(&self) -> Option<FaceId> {
        self.face_id
    }

    pub(super) const fn materialized_face(&self) -> MaterializedFaceData {
        self.materialized_face
    }

    pub(super) const fn clip(&self) -> FrameRect {
        self.clip
    }
}

/// Deep, immutable resolver shared by every renderer layer.
///
/// It addresses the already-presented primitive table; it cannot modify a
/// glyph, its source slot, or any geometry used by layout.
pub(super) struct PointerOverrideResolver {
    overrides: Vec<Option<(PresentedPrimitiveKind, PrimitivePointerOverride)>>,
    active: bool,
    frame_bounds: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct FacePaint {
    face_id: FaceId,
    clip: Option<Rect>,
}

impl FacePaint {
    pub(super) const fn face_id(self) -> FaceId {
        self.face_id
    }

    pub(super) const fn clip(self) -> Option<Rect> {
        self.clip
    }
}

impl PointerOverrideResolver {
    pub(super) fn new(
        frame: &FrameGlyphBuffer,
        selection: Option<PointerAppearanceSelection>,
    ) -> Self {
        let mut overrides = Vec::new();
        let mut active = false;
        if let Some(selection) = selection
            && let Some(appearance) = frame.presented_pointer().appearance(selection.appearance())
        {
            active = true;
            overrides.resize(frame.glyphs.len(), None);
            let mode = match selection.phase() {
                PointerAppearancePhase::Hover => appearance.hover(),
                PointerAppearancePhase::Pressed => appearance.pressed(),
            };
            for span in appearance.paint_spans() {
                let Ok(first) = usize::try_from(span.first()) else {
                    continue;
                };
                let Ok(len) = usize::try_from(span.len()) else {
                    continue;
                };
                let Some(end) = first.checked_add(len) else {
                    continue;
                };
                for slot in overrides.get_mut(first..end).into_iter().flatten() {
                    *slot = Some((
                        span.kind(),
                        PrimitivePointerOverride {
                            mode,
                            clip: span.clip(),
                        },
                    ));
                }
            }
        }
        Self {
            overrides,
            active,
            frame_bounds: Rect::new(0.0, 0.0, frame.width, frame.height),
        }
    }

    pub(super) fn glyph_override(&self, index: usize) -> Option<PrimitivePointerOverride> {
        self.primitive_override(PresentedPrimitiveKind::Glyph, index)
    }

    pub(super) fn image_override(&self, index: usize) -> Option<PrimitivePointerOverride> {
        self.primitive_override(PresentedPrimitiveKind::Image, index)
    }

    #[cfg(test)]
    pub(super) fn face_id(&self, index: usize, base: FaceId) -> FaceId {
        match self
            .glyph_override(index)
            .map(PrimitivePointerOverride::mode)
        {
            Some(PointerDrawMode::Face(face)) => face,
            _ => base,
        }
    }

    /// Replacement paint plan for a face-backed primitive. Base paint covers
    /// only the complement of the transient override; the alternate face is
    /// last and covers exactly the effective override clip.
    pub(super) fn face_paints(
        &self,
        index: usize,
        base_face: FaceId,
        original_clip: Option<&Rect>,
    ) -> Vec<FacePaint> {
        let Some(override_paint) = self.glyph_override(index) else {
            return vec![FacePaint {
                face_id: base_face,
                clip: original_clip.cloned(),
            }];
        };
        let PointerDrawMode::Face(override_face) = override_paint.mode() else {
            return vec![FacePaint {
                face_id: base_face,
                clip: original_clip.cloned(),
            }];
        };
        let domain = original_clip
            .cloned()
            .unwrap_or_else(|| self.frame_bounds.clone());
        let raw = override_paint.clip();
        let raw = Rect::new(raw.x(), raw.y(), raw.width(), raw.height());
        let Some(cut) = intersect_rect(&domain, &raw) else {
            return vec![FacePaint {
                face_id: base_face,
                clip: original_clip.cloned(),
            }];
        };
        let mut paints = rect_complement(&domain, &cut)
            .into_iter()
            .map(|clip| FacePaint {
                face_id: base_face,
                clip: Some(clip),
            })
            .collect::<Vec<_>>();
        paints.push(FacePaint {
            face_id: override_face,
            clip: Some(cut),
        });
        paints
    }

    #[cfg(test)]
    pub(super) fn resolve_glyph<'a>(
        &self,
        frame: &'a FrameGlyphBuffer,
        index: usize,
    ) -> Option<ResolvedGlyphPaint<'a>> {
        let primitive = frame.glyphs.get(index)?;
        let base_face = primitive.face_id()?;
        let face_id = self.face_id(index, base_face);
        let clip = self
            .glyph_clip(index, primitive.clip_rect().as_ref())
            .and_then(|clip| FrameRect::new(clip.x, clip.y, clip.width, clip.height).ok())
            .or_else(|| {
                primitive
                    .cell_rect()
                    .and_then(|(x, y, width, height)| FrameRect::new(x, y, width, height).ok())
            })?;
        Some(ResolvedGlyphPaint {
            primitive,
            face_id: Some(face_id),
            materialized_face: frame.resolved_face(face_id),
            clip,
        })
    }

    pub(super) const fn is_active(&self) -> bool {
        self.active
    }

    #[cfg(test)]
    pub(super) fn glyph_clip(&self, index: usize, base: Option<&Rect>) -> Option<Rect> {
        self.effective_clip(PresentedPrimitiveKind::Glyph, index, base)
    }

    pub(super) fn image_clip(&self, index: usize, base: Option<&Rect>) -> Option<Rect> {
        self.effective_clip(PresentedPrimitiveKind::Image, index, base)
    }

    fn effective_clip(
        &self,
        kind: PresentedPrimitiveKind,
        index: usize,
        base: Option<&Rect>,
    ) -> Option<Rect> {
        let Some(override_paint) = self.primitive_override(kind, index) else {
            return base.cloned();
        };
        let clip = override_paint.clip();
        let override_rect = Rect {
            x: clip.x(),
            y: clip.y(),
            width: clip.width(),
            height: clip.height(),
        };
        base.map_or(Some(override_rect.clone()), |base| {
            intersect_rect(base, &override_rect)
        })
    }

    fn primitive_override(
        &self,
        kind: PresentedPrimitiveKind,
        index: usize,
    ) -> Option<PrimitivePointerOverride> {
        self.overrides
            .get(index)
            .and_then(|entry| *entry)
            .and_then(|(actual_kind, value)| (actual_kind == kind).then_some(value))
    }
}

fn intersect_rect(left: &Rect, right: &Rect) -> Option<Rect> {
    let x = left.x.max(right.x);
    let y = left.y.max(right.y);
    let right_edge = (left.x + left.width).min(right.x + right.width);
    let bottom = (left.y + left.height).min(right.y + right.height);
    (right_edge > x && bottom > y).then(|| Rect {
        x,
        y,
        width: right_edge - x,
        height: bottom - y,
    })
}

fn rect_complement(domain: &Rect, cut: &Rect) -> Vec<Rect> {
    let mut out = Vec::with_capacity(4);
    let domain_right = domain.x + domain.width;
    let domain_bottom = domain.y + domain.height;
    let cut_right = cut.x + cut.width;
    let cut_bottom = cut.y + cut.height;
    if cut.y > domain.y {
        out.push(Rect::new(
            domain.x,
            domain.y,
            domain.width,
            cut.y - domain.y,
        ));
    }
    if cut_bottom < domain_bottom {
        out.push(Rect::new(
            domain.x,
            cut_bottom,
            domain.width,
            domain_bottom - cut_bottom,
        ));
    }
    if cut.x > domain.x {
        out.push(Rect::new(domain.x, cut.y, cut.x - domain.x, cut.height));
    }
    if cut_right < domain_right {
        out.push(Rect::new(
            cut_right,
            cut.y,
            domain_right - cut_right,
            cut.height,
        ));
    }
    out
}

pub(super) fn clip_geometry(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    clip: Option<&Rect>,
) -> Option<(f32, f32, f32, f32)> {
    let Some(clip) = clip else {
        return Some((x, y, width, height));
    };
    let draw_x = x.max(clip.x);
    let draw_y = y.max(clip.y);
    let right = (x + width).min(clip.x + clip.width);
    let bottom = (y + height).min(clip.y + clip.height);
    (right > draw_x && bottom > draw_y).then_some((draw_x, draw_y, right - draw_x, bottom - draw_y))
}

/// Clip rect primitives appended since `start` without changing their layer
/// order. Every renderer rect is encoded as six vertices.
pub(super) fn clip_new_rect_vertices(
    vertices: &mut Vec<RectVertex>,
    start: usize,
    clip: Option<&Rect>,
) {
    let Some(clip) = clip else { return };
    let generated: Vec<[RectVertex; 6]> = vertices[start..]
        .chunks_exact(6)
        .filter_map(|chunk| chunk.try_into().ok())
        .collect();
    vertices.truncate(start);
    for rect in generated {
        let min_x = rect
            .iter()
            .map(|v| v.position[0])
            .fold(f32::INFINITY, f32::min);
        let min_y = rect
            .iter()
            .map(|v| v.position[1])
            .fold(f32::INFINITY, f32::min);
        let max_x = rect
            .iter()
            .map(|v| v.position[0])
            .fold(f32::NEG_INFINITY, f32::max);
        let max_y = rect
            .iter()
            .map(|v| v.position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        let Some((x, y, width, height)) =
            clip_geometry(min_x, min_y, max_x - min_x, max_y - min_y, Some(clip))
        else {
            continue;
        };
        let color = rect[0].color;
        vertices.extend_from_slice(&[
            RectVertex {
                position: [x, y],
                color,
            },
            RectVertex {
                position: [x + width, y],
                color,
            },
            RectVertex {
                position: [x + width, y + height],
                color,
            },
            RectVertex {
                position: [x, y],
                color,
            },
            RectVertex {
                position: [x + width, y + height],
                color,
            },
            RectVertex {
                position: [x, y + height],
                color,
            },
        ]);
    }
}

pub(super) fn clip_new_rounded_vertices(
    vertices: &mut Vec<RoundedRectVertex>,
    start: usize,
    clip: Option<&Rect>,
) {
    let Some(clip) = clip else { return };
    let generated: Vec<[RoundedRectVertex; 6]> = vertices[start..]
        .chunks_exact(6)
        .filter_map(|chunk| chunk.try_into().ok())
        .collect();
    vertices.truncate(start);
    for quad in generated {
        let min_x = quad
            .iter()
            .map(|v| v.position[0])
            .fold(f32::INFINITY, f32::min);
        let min_y = quad
            .iter()
            .map(|v| v.position[1])
            .fold(f32::INFINITY, f32::min);
        let max_x = quad
            .iter()
            .map(|v| v.position[0])
            .fold(f32::NEG_INFINITY, f32::max);
        let max_y = quad
            .iter()
            .map(|v| v.position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        let Some((x, y, width, height)) =
            clip_geometry(min_x, min_y, max_x - min_x, max_y - min_y, Some(clip))
        else {
            continue;
        };
        let template = quad[0];
        vertices.extend_from_slice(
            &[
                [x, y],
                [x + width, y],
                [x + width, y + height],
                [x, y],
                [x + width, y + height],
                [x, y + height],
            ]
            .map(|position| RoundedRectVertex {
                position,
                ..template
            }),
        );
    }
}

pub(super) fn clip_glyph_quad(
    quad: [GlyphVertex; 6],
    clip: Option<&Rect>,
) -> Option<[GlyphVertex; 6]> {
    let Some(clip) = clip else { return Some(quad) };
    let min_x = quad
        .iter()
        .map(|v| v.position[0])
        .fold(f32::INFINITY, f32::min);
    let min_y = quad
        .iter()
        .map(|v| v.position[1])
        .fold(f32::INFINITY, f32::min);
    let max_x = quad
        .iter()
        .map(|v| v.position[0])
        .fold(f32::NEG_INFINITY, f32::max);
    let max_y = quad
        .iter()
        .map(|v| v.position[1])
        .fold(f32::NEG_INFINITY, f32::max);
    let width = max_x - min_x;
    let height = max_y - min_y;
    let (x, y, draw_width, draw_height) = clip_geometry(min_x, min_y, width, height, Some(clip))?;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let min_u = quad
        .iter()
        .map(|v| v.tex_coords[0])
        .fold(f32::INFINITY, f32::min);
    let max_u = quad
        .iter()
        .map(|v| v.tex_coords[0])
        .fold(f32::NEG_INFINITY, f32::max);
    let min_v = quad
        .iter()
        .map(|v| v.tex_coords[1])
        .fold(f32::INFINITY, f32::min);
    let max_v = quad
        .iter()
        .map(|v| v.tex_coords[1])
        .fold(f32::NEG_INFINITY, f32::max);
    let u0 = min_u + (max_u - min_u) * ((x - min_x) / width);
    let u1 = min_u + (max_u - min_u) * ((x + draw_width - min_x) / width);
    let v0 = min_v + (max_v - min_v) * ((y - min_y) / height);
    let v1 = min_v + (max_v - min_v) * ((y + draw_height - min_y) / height);
    let color = quad[0].color;
    Some([
        GlyphVertex {
            position: [x, y],
            tex_coords: [u0, v0],
            color,
        },
        GlyphVertex {
            position: [x + draw_width, y],
            tex_coords: [u1, v0],
            color,
        },
        GlyphVertex {
            position: [x + draw_width, y + draw_height],
            tex_coords: [u1, v1],
            color,
        },
        GlyphVertex {
            position: [x, y],
            tex_coords: [u0, v0],
            color,
        },
        GlyphVertex {
            position: [x + draw_width, y + draw_height],
            tex_coords: [u1, v1],
            color,
        },
        GlyphVertex {
            position: [x, y + draw_height],
            tex_coords: [u0, v1],
            color,
        },
    ])
}

pub(super) fn clip_subpixel_quad(
    quad: [SubpixelGlyphVertex; 6],
    clip: Option<&Rect>,
) -> Option<[SubpixelGlyphVertex; 6]> {
    let glyph_quad = quad.map(|vertex| GlyphVertex {
        position: vertex.position,
        tex_coords: vertex.tex_coords,
        color: vertex.fg_color,
    });
    let clipped = clip_glyph_quad(glyph_quad, clip)?;
    let fg_color = quad[0].fg_color;
    let bg_color = quad[0].bg_color;
    Some(clipped.map(|vertex| SubpixelGlyphVertex {
        position: vertex.position,
        tex_coords: vertex.tex_coords,
        fg_color,
        bg_color,
    }))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ReliefEdge {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Color,
}

impl ReliefEdge {
    pub(super) const fn bounds(self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.width, self.height)
    }

    pub(super) const fn color(self) -> Color {
        self.color
    }
}

/// GNU-style one-pixel relief entirely inside the existing image rectangle.
pub(super) fn relief_edges(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    mode: PointerDrawMode,
) -> Option<[ReliefEdge; 4]> {
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let light = Color::new(0.85, 0.85, 0.85, 1.0);
    let dark = Color::new(0.25, 0.25, 0.25, 1.0);
    let (top_left, bottom_right) = match mode {
        PointerDrawMode::ImageRaised => (light, dark),
        PointerDrawMode::ImageSunken => (dark, light),
        PointerDrawMode::Face(_) => return None,
    };
    let edge = 1.0_f32.min(width).min(height);
    Some([
        ReliefEdge {
            x,
            y,
            width,
            height: edge,
            color: top_left,
        },
        ReliefEdge {
            x,
            y,
            width: edge,
            height,
            color: top_left,
        },
        ReliefEdge {
            x,
            y: y + height - edge,
            width,
            height: edge,
            color: bottom_right,
        },
        ReliefEdge {
            x: x + width - edge,
            y,
            width: edge,
            height,
            color: bottom_right,
        },
    ])
}

#[cfg(test)]
#[path = "pointer_override_test.rs"]
mod tests;
