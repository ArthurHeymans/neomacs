#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowCharWidthPolicy {
    fallback_char_width: f32,
}

impl DisplayRowCharWidthPolicy {
    pub(crate) fn new(fallback_char_width: f32) -> Self {
        Self {
            fallback_char_width: positive_row_width(fallback_char_width).unwrap_or(1.0),
        }
    }

    pub(crate) fn fallback(self) -> f32 {
        self.fallback_char_width
    }

    pub(crate) fn has_width(self, width: f32) -> bool {
        positive_row_width(width).is_some()
    }

    pub(crate) fn width(self, width: f32) -> f32 {
        positive_row_width(width)
            .unwrap_or(self.fallback_char_width)
            .max(self.fallback_char_width)
    }

    pub(crate) fn width_or_measured(self, width: f32, measured_width: Option<f32>) -> f32 {
        positive_row_width(width)
            .map(|width| width.max(self.fallback_char_width))
            .or_else(|| measured_width.map(|width| self.width(width)))
            .unwrap_or(self.fallback_char_width)
    }

    pub(crate) fn advance_for_columns(self, columns: u8) -> f32 {
        self.fallback_char_width * f32::from(columns)
    }
}

fn positive_row_width(value: f32) -> Option<f32> {
    (value.is_finite() && value > 0.0).then_some(value)
}
