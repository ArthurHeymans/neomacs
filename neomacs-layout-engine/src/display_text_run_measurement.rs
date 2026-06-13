use crate::font_metrics::ShapedGlyph;
use crate::glyph_advance::GlyphAdvanceQuantization;
use crate::unicode::{decode_utf8, is_cluster_extender};

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

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ComplexTextRunAdvanceCache {
    start_byte_idx: usize,
    end_byte_idx: usize,
    advances: Vec<DisplayTextRunByteAdvance>,
}

impl ComplexTextRunAdvanceCache {
    pub(crate) fn record(
        &mut self,
        start_byte_idx: usize,
        end_byte_idx: usize,
        advances: Vec<DisplayTextRunByteAdvance>,
    ) {
        self.start_byte_idx = start_byte_idx;
        self.end_byte_idx = end_byte_idx;
        self.advances = advances;
    }

    pub(crate) fn contains(&self, byte_idx: usize) -> bool {
        self.start_byte_idx <= byte_idx && byte_idx < self.end_byte_idx
    }

    pub(crate) fn advance_for(&self, byte_idx: usize) -> Option<f32> {
        self.advances
            .iter()
            .find(|advance| advance.byte_offset == byte_idx)
            .map(|advance| advance.advance_px)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ComplexTextRunSpan {
    text: String,
    end_byte_idx: usize,
}

impl ComplexTextRunSpan {
    pub(crate) fn from_text_at(text: &[u8], start_byte_idx: usize, first_char: char) -> Self {
        let script = crate::composition::complex_script(first_char);
        let mut end_byte_idx = start_byte_idx;
        let mut run_text = String::new();
        while end_byte_idx < text.len() {
            let (ch, ch_len) = decode_utf8(&text[end_byte_idx..]);
            if crate::composition::complex_script(ch) == script
                || (end_byte_idx > start_byte_idx && is_cluster_extender(ch))
            {
                run_text.push(ch);
                end_byte_idx += ch_len;
            } else {
                break;
            }
        }

        Self {
            text: run_text,
            end_byte_idx,
        }
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn end_byte_idx(&self) -> usize {
        self.end_byte_idx
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DisplayTextRunMeasurement {
    PerChar,
    Measured(Vec<DisplayTextRunAdvance>),
}

impl DisplayTextRunMeasurement {
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
    pub(crate) fn from_resolved_fragment_advance(
        text: &str,
        advance_px: f32,
    ) -> DisplayTextRunMeasurement {
        if text.is_empty() {
            return DisplayTextRunMeasurement::PerChar;
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
        DisplayTextRunMeasurement::Measured(advances)
    }

    #[cfg(test)]
    pub(crate) fn uniform_for_text(text: &str, advance_px: f32) -> DisplayTextRunMeasurement {
        if text.is_empty() {
            return DisplayTextRunMeasurement::PerChar;
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
        DisplayTextRunMeasurement::Measured(advances)
    }

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

    #[test]
    fn measurement_plan_builds_uniform_advances_for_text() {
        let measurement = DisplayTextRunMeasurementPlan::uniform_for_text("aé中", 5.0);

        let DisplayTextRunMeasurement::Measured(advances) = measurement else {
            panic!("non-empty text should produce uniform measured advances");
        };
        assert_eq!(
            advances
                .iter()
                .map(|advance| (advance.char_offset, advance.byte_offset, advance.advance_px))
                .collect::<Vec<_>>(),
            vec![(0, 0, 5.0), (1, 1, 5.0), (2, 3, 5.0)]
        );
    }

    #[test]
    fn complex_run_advance_cache_records_byte_scoped_advances() {
        let mut cache = ComplexTextRunAdvanceCache::default();

        cache.record(
            10,
            18,
            vec![
                DisplayTextRunByteAdvance::new(10, 7.0),
                DisplayTextRunByteAdvance::new(14, 11.0),
            ],
        );

        assert!(!cache.contains(9));
        assert!(cache.contains(10));
        assert!(cache.contains(17));
        assert!(!cache.contains(18));
        assert_eq!(cache.advance_for(10), Some(7.0));
        assert_eq!(cache.advance_for(14), Some(11.0));
        assert_eq!(cache.advance_for(12), None);
    }

    #[test]
    fn complex_text_run_span_keeps_same_script_text() {
        let text = "abc\u{0633}\u{0644}\u{0627}def".as_bytes();
        let start = "abc".len();

        let span = ComplexTextRunSpan::from_text_at(text, start, '\u{0633}');

        assert_eq!(span.text(), "\u{0633}\u{0644}\u{0627}");
        assert_eq!(span.end_byte_idx(), "abc\u{0633}\u{0644}\u{0627}".len());
    }

    #[test]
    fn complex_text_run_span_keeps_following_cluster_extenders() {
        let text = "\u{0915}\u{093C}x".as_bytes();

        let span = ComplexTextRunSpan::from_text_at(text, 0, '\u{0915}');

        assert_eq!(span.text(), "\u{0915}\u{093C}");
        assert_eq!(span.end_byte_idx(), "\u{0915}\u{093C}".len());
    }
}
