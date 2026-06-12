use crate::font_metrics::ShapedGlyph;
use crate::glyph_advance::GlyphAdvanceQuantization;
use crate::unicode::is_cluster_extender;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayTextRunAdvance {
    pub(crate) char_offset: usize,
    pub(crate) byte_offset: usize,
    pub(crate) advance_px: f32,
}

impl DisplayTextRunAdvance {
    pub(crate) fn new(char_offset: usize, byte_offset: usize, advance_px: f32) -> Self {
        Self {
            char_offset,
            byte_offset,
            advance_px,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayTextRunByteAdvance {
    pub(crate) byte_offset: usize,
    pub(crate) advance_px: f32,
}

impl DisplayTextRunByteAdvance {
    pub(crate) fn new(byte_offset: usize, advance_px: f32) -> Self {
        Self {
            byte_offset,
            advance_px,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DisplayTextRunMeasurement {
    PerChar,
    Measured(Vec<DisplayTextRunAdvance>),
}

impl DisplayTextRunMeasurement {
    pub(crate) fn uniform_for_text(text: &str, advance_px: f32) -> Self {
        if text.is_empty() {
            return Self::PerChar;
        }
        let advance_px = if advance_px.is_finite() && advance_px >= 0.0 {
            advance_px
        } else {
            0.0
        };
        let advances = text
            .char_indices()
            .enumerate()
            .map(|(char_offset, (byte_offset, _))| {
                DisplayTextRunAdvance::new(char_offset, byte_offset, advance_px)
            })
            .collect();
        Self::Measured(advances)
    }

    pub(crate) fn measured_advances(&self) -> Option<&[DisplayTextRunAdvance]> {
        match self {
            Self::PerChar => None,
            Self::Measured(advances) => Some(advances),
        }
    }

    pub(crate) fn base_char_byte_advances(
        &self,
        text: &str,
        base_byte_offset: usize,
    ) -> Vec<DisplayTextRunByteAdvance> {
        let Self::Measured(advances) = self else {
            return Vec::new();
        };

        advances
            .iter()
            .filter_map(|advance| {
                let c = text.get(advance.byte_offset..)?.chars().next()?;
                (!is_cluster_extender(c)).then_some(DisplayTextRunByteAdvance::new(
                    base_byte_offset + advance.byte_offset,
                    advance.advance_px,
                ))
            })
            .collect()
    }

    pub(crate) fn advance_for(&self, char_offset: usize, byte_offset: usize) -> Option<f32> {
        match self {
            Self::PerChar => None,
            Self::Measured(advances) => advances
                .iter()
                .find(|advance| {
                    advance.char_offset == char_offset && advance.byte_offset == byte_offset
                })
                .and_then(|advance| {
                    (advance.advance_px.is_finite() && advance.advance_px >= 0.0)
                        .then_some(advance.advance_px)
                }),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct DisplayTextRunClusterAdvance {
    byte_offset: usize,
    advance_px: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct DisplayTextRunClusterAdvances {
    advances: Vec<DisplayTextRunClusterAdvance>,
}

impl DisplayTextRunClusterAdvances {
    fn from_shaped_glyphs(text_len: usize, glyphs: impl IntoIterator<Item = ShapedGlyph>) -> Self {
        let mut advances: Vec<DisplayTextRunClusterAdvance> = Vec::new();
        for glyph in glyphs {
            if glyph.cluster_start > text_len {
                continue;
            }
            if let Some(advance) = advances
                .iter_mut()
                .find(|advance| advance.byte_offset == glyph.cluster_start)
            {
                advance.advance_px += glyph.x_advance;
            } else {
                advances.push(DisplayTextRunClusterAdvance {
                    byte_offset: glyph.cluster_start,
                    advance_px: glyph.x_advance,
                });
            }
        }
        Self { advances }
    }

    fn advance_at(&self, byte_offset: usize) -> Option<f32> {
        self.advances
            .iter()
            .find(|advance| advance.byte_offset == byte_offset)
            .map(|advance| advance.advance_px)
    }
}

pub(crate) struct DisplayTextRunMeasurementPlan;

impl DisplayTextRunMeasurementPlan {
    pub(crate) fn from_shaped_glyphs(
        text: &str,
        glyphs: impl IntoIterator<Item = ShapedGlyph>,
        face_char_width_px: f32,
        fallback_char_width_px: f32,
        quantization: GlyphAdvanceQuantization,
    ) -> DisplayTextRunMeasurement {
        let cluster_advances =
            DisplayTextRunClusterAdvances::from_shaped_glyphs(text.len(), glyphs);
        let face_char_width_px = face_char_width_px.max(fallback_char_width_px).max(1.0);
        let fallback_char_width_px = fallback_char_width_px.max(face_char_width_px).max(1.0);
        let advances = text
            .char_indices()
            .enumerate()
            .filter_map(|(char_offset, (byte_offset, ch))| {
                let measured = cluster_advances.advance_at(byte_offset)?;
                let columns = crate::composition::base_width_cols(ch);
                let minimum = f32::from(columns.max(1)) * face_char_width_px;
                let fallback = f32::from(columns.max(1)) * fallback_char_width_px;
                Some(DisplayTextRunAdvance::new(
                    char_offset,
                    byte_offset,
                    quantization.resolve(Some(measured), fallback, minimum),
                ))
            })
            .collect::<Vec<_>>();

        if advances.is_empty() {
            DisplayTextRunMeasurement::PerChar
        } else {
            DisplayTextRunMeasurement::Measured(advances)
        }
    }

    pub(crate) fn from_char_advances(
        text: &str,
        fallback_char_width_px: f32,
        mut advance_for_char: impl FnMut(char, f32) -> f32,
    ) -> DisplayTextRunMeasurement {
        let advances = text
            .char_indices()
            .enumerate()
            .map(|(char_offset, (byte_offset, ch))| {
                let columns = crate::composition::base_width_cols(ch).max(1);
                let fallback_advance_px = fallback_char_width_px * f32::from(columns);
                DisplayTextRunAdvance::new(
                    char_offset,
                    byte_offset,
                    advance_for_char(ch, fallback_advance_px),
                )
            })
            .collect();
        DisplayTextRunMeasurement::Measured(advances)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shaped(cluster_start: usize, x_advance: f32) -> ShapedGlyph {
        ShapedGlyph {
            font_id: fontdb::ID::dummy(),
            glyph_id: 1,
            x: 0.0,
            y: 0.0,
            x_advance,
            cluster_start,
            cluster_end: cluster_start + 1,
        }
    }

    #[test]
    fn cluster_advances_group_shaped_glyphs_by_cluster_start() {
        let advances = DisplayTextRunClusterAdvances::from_shaped_glyphs(
            "aéb".len(),
            [
                shaped(0, 3.0),
                shaped(0, 4.5),
                shaped(3, 5.0),
                shaped(99, 10.0),
            ],
        );

        assert_eq!(advances.advance_at(0), Some(7.5));
        assert_eq!(advances.advance_at(3), Some(5.0));
        assert_eq!(advances.advance_at(1), None);
        assert_eq!(advances.advance_at(99), None);
    }
}
