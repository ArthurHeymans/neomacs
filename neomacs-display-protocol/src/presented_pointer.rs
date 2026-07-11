//! Renderer-safe pointer interactions and transient paint overrides.
//!
//! A map is validated against the presentation that owns its geometry and
//! primitive tables before it can be published. The validation context is not
//! retained: render-time consumers only receive coherent immutable records.

use crate::{FaceId, FrameGlyph, FrameGlyphBuffer, FrameRect, FrameSize, InteractionId};

/// Presentation-local index of one transient pointer appearance.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub struct PointerAppearanceId(u32);

impl PointerAppearanceId {
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl TryFrom<usize> for PointerAppearanceId {
    type Error = PointerAppearanceIdOverflow;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        u32::try_from(value)
            .map(Self)
            .map_err(|_| PointerAppearanceIdOverflow)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PointerAppearanceIdOverflow;

impl std::fmt::Display for PointerAppearanceIdOverflow {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("pointer appearance index exceeds u32")
    }
}

impl std::error::Error for PointerAppearanceIdOverflow {}

/// Existing presentation primitive table addressed by a paint span.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PresentedPrimitiveKind {
    Glyph,
    Image,
}

/// Contiguous primitives redrawn with a transient pointer override.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PresentedPaintSpan {
    kind: PresentedPrimitiveKind,
    first: u32,
    len: u32,
    clip: FrameRect,
}

impl PresentedPaintSpan {
    #[must_use]
    pub const fn new(kind: PresentedPrimitiveKind, first: u32, len: u32, clip: FrameRect) -> Self {
        Self {
            kind,
            first,
            len,
            clip,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> PresentedPrimitiveKind {
        self.kind
    }

    #[must_use]
    pub const fn first(&self) -> u32 {
        self.first
    }

    #[must_use]
    pub const fn len(&self) -> u32 {
        self.len
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[must_use]
    pub const fn clip(&self) -> FrameRect {
        self.clip
    }
}

/// Renderer operation selected for a hovered or pressed appearance.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PointerDrawMode {
    Face(FaceId),
    ImageRaised,
    ImageSunken,
}

/// Paint behavior shared by one or more independent interaction regions.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PresentedPointerAppearance {
    paint_spans: Vec<PresentedPaintSpan>,
    hover: PointerDrawMode,
    pressed: PointerDrawMode,
}

impl PresentedPointerAppearance {
    #[must_use]
    pub fn new(
        paint_spans: Vec<PresentedPaintSpan>,
        hover: PointerDrawMode,
        pressed: PointerDrawMode,
    ) -> Self {
        Self {
            paint_spans,
            hover,
            pressed,
        }
    }

    #[must_use]
    pub fn paint_spans(&self) -> &[PresentedPaintSpan] {
        &self.paint_spans
    }

    #[must_use]
    pub const fn hover(&self) -> PointerDrawMode {
        self.hover
    }

    #[must_use]
    pub const fn pressed(&self) -> PointerDrawMode {
        self.pressed
    }
}

/// Hit geometry, evaluator-owned click meaning, and renderer-owned appearance.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PresentedPointerRegion {
    bounds: FrameRect,
    interaction: Option<InteractionId>,
    appearance: Option<PointerAppearanceId>,
}

impl PresentedPointerRegion {
    #[must_use]
    pub const fn new(
        bounds: FrameRect,
        interaction: Option<InteractionId>,
        appearance: Option<PointerAppearanceId>,
    ) -> Self {
        Self {
            bounds,
            interaction,
            appearance,
        }
    }

    #[must_use]
    pub const fn bounds(&self) -> FrameRect {
        self.bounds
    }

    #[must_use]
    pub const fn interaction(&self) -> Option<InteractionId> {
        self.interaction
    }

    #[must_use]
    pub const fn appearance(&self) -> Option<PointerAppearanceId> {
        self.appearance
    }
}

/// Cross-field limits supplied by the completed presentation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PointerMapValidationContext<'a> {
    frame_buffer: &'a FrameGlyphBuffer,
}

impl<'a> PointerMapValidationContext<'a> {
    pub(crate) fn from_frame_buffer(
        frame_buffer: &'a FrameGlyphBuffer,
    ) -> Result<Self, PresentedPointerMapError> {
        FrameSize::new(frame_buffer.width, frame_buffer.height)
            .map_err(|_| PresentedPointerMapError::InvalidFrameGeometry)?;
        Ok(Self { frame_buffer })
    }

    fn frame(self) -> FrameSize {
        FrameSize::new(self.frame_buffer.width, self.frame_buffer.height)
            .expect("validation context checked frame dimensions")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresentedPointerMapError {
    UnknownAppearance(PointerAppearanceId),
    MissingRegionBehavior,
    EmptyAppearance,
    EmptyPaintSpan,
    PaintSpanOutOfRange,
    InvalidRegionGeometry,
    InvalidClipGeometry,
    InvalidFrameGeometry,
    RegionOutsideFrame,
    ClipOutsideFrame,
    PrimitiveKindMismatch,
    UnknownFace(FaceId),
}

impl std::fmt::Display for PresentedPointerMapError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "invalid presented pointer map: {self:?}")
    }
}

impl std::error::Error for PresentedPointerMapError {}

/// Intrinsically valid pointer metadata for one immutable presentation.
///
/// Deserialization validates internal geometry, indices, and references only.
/// Renderer-safe contextual validity is established atomically when the map is
/// installed through [`FrameGlyphBuffer::install_presented_pointer_map`].
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct PresentedPointerMap {
    regions: Vec<PresentedPointerRegion>,
    appearances: Vec<PresentedPointerAppearance>,
    #[serde(skip)]
    row_buckets: Vec<PointerRowBucket>,
}

#[derive(Clone, Debug, PartialEq)]
struct PointerRowBucket {
    top: f32,
    bottom: f32,
    prefix_max_bottom: f32,
    candidates: Vec<usize>,
    prefix_max_right: Vec<f32>,
}

impl PresentedPointerMap {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            regions: Vec::new(),
            appearances: Vec::new(),
            row_buckets: Vec::new(),
        }
    }

    pub(crate) fn from_parts(
        regions: Vec<PresentedPointerRegion>,
        appearances: Vec<PresentedPointerAppearance>,
    ) -> Result<Self, PresentedPointerMapError> {
        let mut map = Self {
            regions,
            appearances,
            row_buckets: Vec::new(),
        };
        map.validate_intrinsic()?;
        map.rebuild_hit_index();
        Ok(map)
    }

    /// Revalidates snapshot-dependent references after transport.
    pub(crate) fn validate_against(
        &self,
        context: PointerMapValidationContext<'_>,
    ) -> Result<(), PresentedPointerMapError> {
        let frame = context.frame();
        for region in &self.regions {
            if !rect_is_within_frame(region.bounds, frame) {
                return Err(PresentedPointerMapError::RegionOutsideFrame);
            }
        }

        for appearance in &self.appearances {
            validate_mode(appearance.hover, context.frame_buffer)?;
            validate_mode(appearance.pressed, context.frame_buffer)?;
            for span in &appearance.paint_spans {
                if !rect_is_within_frame(span.clip, frame) {
                    return Err(PresentedPointerMapError::ClipOutsideFrame);
                }
                let end = span
                    .first
                    .checked_add(span.len)
                    .ok_or(PresentedPointerMapError::PaintSpanOutOfRange)?;
                let (Ok(first), Ok(end)) = (usize::try_from(span.first), usize::try_from(end))
                else {
                    return Err(PresentedPointerMapError::PaintSpanOutOfRange);
                };
                let Some(primitives) = context.frame_buffer.glyphs.get(first..end) else {
                    return Err(PresentedPointerMapError::PaintSpanOutOfRange);
                };
                let matches_kind = match span.kind {
                    PresentedPrimitiveKind::Glyph => primitives.iter().all(|primitive| {
                        matches!(
                            primitive,
                            FrameGlyph::Char { .. } | FrameGlyph::Stretch { .. }
                        )
                    }),
                    PresentedPrimitiveKind::Image => primitives
                        .iter()
                        .all(|primitive| matches!(primitive, FrameGlyph::Image { .. })),
                };
                if !matches_kind {
                    return Err(PresentedPointerMapError::PrimitiveKindMismatch);
                }
            }
        }

        Ok(())
    }

    fn validate_intrinsic(&self) -> Result<(), PresentedPointerMapError> {
        for region in &self.regions {
            if !rect_has_valid_geometry(region.bounds) {
                return Err(PresentedPointerMapError::InvalidRegionGeometry);
            }
            if region.interaction.is_none() && region.appearance.is_none() {
                return Err(PresentedPointerMapError::MissingRegionBehavior);
            }
            if let Some(appearance) = region.appearance
                && usize::try_from(appearance.get())
                    .map_or(true, |index| index >= self.appearances.len())
            {
                return Err(PresentedPointerMapError::UnknownAppearance(appearance));
            }
        }

        for appearance in &self.appearances {
            if appearance.paint_spans.is_empty() {
                return Err(PresentedPointerMapError::EmptyAppearance);
            }
            for span in &appearance.paint_spans {
                if span.len == 0 {
                    return Err(PresentedPointerMapError::EmptyPaintSpan);
                }
                if !rect_has_valid_geometry(span.clip) {
                    return Err(PresentedPointerMapError::InvalidClipGeometry);
                }
                if span.first.checked_add(span.len).is_none() {
                    return Err(PresentedPointerMapError::PaintSpanOutOfRange);
                }
            }
        }

        Ok(())
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty() && self.appearances.is_empty()
    }

    #[must_use]
    pub fn regions(&self) -> &[PresentedPointerRegion] {
        &self.regions
    }

    #[must_use]
    pub fn appearances(&self) -> &[PresentedPointerAppearance] {
        &self.appearances
    }

    #[must_use]
    pub fn appearance(&self, id: PointerAppearanceId) -> Option<&PresentedPointerAppearance> {
        usize::try_from(id.get())
            .ok()
            .and_then(|index| self.appearances.get(index))
    }

    /// Returns the first published region containing `(x, y)`.
    ///
    /// Rectangle edges are half-open, matching frame chrome hit testing. Input
    /// order defines stable priority if producers publish overlapping regions.
    #[must_use]
    pub fn hit_test(&self, x: f32, y: f32) -> Option<&PresentedPointerRegion> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        let mut bucket_end = self.row_buckets.partition_point(|bucket| bucket.top <= y);
        let mut best = None;
        while bucket_end > 0 {
            let bucket_index = bucket_end - 1;
            let bucket = &self.row_buckets[bucket_index];
            if bucket.prefix_max_bottom <= y {
                break;
            }
            if y < bucket.bottom {
                let mut candidate_end = bucket
                    .candidates
                    .partition_point(|&index| self.regions[index].bounds.x() <= x);
                while candidate_end > 0 {
                    let candidate_position = candidate_end - 1;
                    if bucket.prefix_max_right[candidate_position] <= x {
                        break;
                    }
                    let region_index = bucket.candidates[candidate_position];
                    let bounds = self.regions[region_index].bounds;
                    if x < bounds.x() + bounds.width() {
                        best = Some(
                            best.map_or(region_index, |current: usize| current.min(region_index)),
                        );
                    }
                    candidate_end -= 1;
                }
            }
            bucket_end -= 1;
        }
        best.map(|index| &self.regions[index])
    }

    fn rebuild_hit_index(&mut self) {
        let mut entries: Vec<_> = self
            .regions
            .iter()
            .enumerate()
            .map(|(index, region)| {
                let bounds = region.bounds;
                (bounds.y(), bounds.y() + bounds.height(), index)
            })
            .collect();
        entries.sort_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then(left.1.total_cmp(&right.1))
                .then(left.2.cmp(&right.2))
        });

        self.row_buckets.clear();
        for (top, bottom, region_index) in entries {
            let starts_new_bucket = self.row_buckets.last().is_none_or(|bucket| {
                bucket.top.total_cmp(&top).is_ne() || bucket.bottom.total_cmp(&bottom).is_ne()
            });
            if starts_new_bucket {
                self.row_buckets.push(PointerRowBucket {
                    top,
                    bottom,
                    prefix_max_bottom: bottom,
                    candidates: Vec::new(),
                    prefix_max_right: Vec::new(),
                });
            }
            self.row_buckets
                .last_mut()
                .expect("bucket was just created")
                .candidates
                .push(region_index);
        }

        let mut prefix_max_bottom = 0.0_f32;
        for bucket in &mut self.row_buckets {
            prefix_max_bottom = prefix_max_bottom.max(bucket.bottom);
            bucket.prefix_max_bottom = prefix_max_bottom;
            bucket.candidates.sort_by(|&left, &right| {
                self.regions[left]
                    .bounds
                    .x()
                    .total_cmp(&self.regions[right].bounds.x())
                    .then(left.cmp(&right))
            });
            let mut prefix_max_right = 0.0_f32;
            bucket.prefix_max_right = bucket
                .candidates
                .iter()
                .map(|&index| {
                    let bounds = self.regions[index].bounds;
                    prefix_max_right = prefix_max_right.max(bounds.x() + bounds.width());
                    prefix_max_right
                })
                .collect();
        }
    }

    #[cfg(test)]
    pub(crate) fn hit_test_candidate_count(&self, y: f32) -> usize {
        self.row_buckets
            .iter()
            .filter(|bucket| bucket.top <= y && y < bucket.bottom)
            .map(|bucket| bucket.candidates.len())
            .sum()
    }

    #[cfg(test)]
    pub(crate) fn hit_index_entry_count(&self) -> usize {
        self.row_buckets
            .iter()
            .map(|bucket| bucket.candidates.len())
            .sum()
    }
}

#[derive(serde::Deserialize)]
struct RawPresentedPointerMap {
    regions: Vec<PresentedPointerRegion>,
    appearances: Vec<PresentedPointerAppearance>,
}

impl<'de> serde::Deserialize<'de> for PresentedPointerMap {
    fn deserialize<Deserializer>(deserializer: Deserializer) -> Result<Self, Deserializer::Error>
    where
        Deserializer: serde::Deserializer<'de>,
    {
        let raw = <RawPresentedPointerMap as serde::Deserialize>::deserialize(deserializer)?;
        let mut map = Self {
            regions: raw.regions,
            appearances: raw.appearances,
            row_buckets: Vec::new(),
        };
        map.validate_intrinsic().map_err(serde::de::Error::custom)?;
        map.rebuild_hit_index();
        Ok(map)
    }
}

fn validate_mode(
    mode: PointerDrawMode,
    frame_buffer: &FrameGlyphBuffer,
) -> Result<(), PresentedPointerMapError> {
    if let PointerDrawMode::Face(face_id) = mode
        && !frame_buffer.faces.contains_key(&face_id)
    {
        return Err(PresentedPointerMapError::UnknownFace(face_id));
    }
    Ok(())
}

fn rect_is_within_frame(rect: FrameRect, frame: FrameSize) -> bool {
    rect_has_valid_geometry(rect)
        && rect.x() + rect.width() <= frame.width()
        && rect.y() + rect.height() <= frame.height()
}

fn rect_has_valid_geometry(rect: FrameRect) -> bool {
    rect.x().is_finite()
        && rect.y().is_finite()
        && rect.width().is_finite()
        && rect.height().is_finite()
        && rect.x() >= 0.0
        && rect.y() >= 0.0
        && rect.width() >= 0.0
        && rect.height() >= 0.0
        && (rect.x() + rect.width()).is_finite()
        && (rect.y() + rect.height()).is_finite()
}
