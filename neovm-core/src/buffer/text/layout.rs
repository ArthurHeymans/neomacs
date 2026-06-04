use crate::buffer::position::{CharPos0, EmacsBytePos};
use crate::buffer::text::TextMetrics;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GapDebugLayout {
    pub gpt: CharPos0,
    pub z: CharPos0,
    pub gpt_byte: EmacsBytePos,
    pub z_byte: EmacsBytePos,
    pub gap_size: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextBackendDebugLayout {
    Gap(GapDebugLayout),
    PieceTree(TextMetrics),
    Rope(TextMetrics),
}

impl TextBackendDebugLayout {
    pub fn metrics(self) -> TextMetrics {
        match self {
            Self::Gap(layout) => TextMetrics::from_positions(layout.z, layout.z_byte),
            Self::PieceTree(metrics) => metrics,
            Self::Rope(metrics) => metrics,
        }
    }

    pub fn gap(self) -> Option<GapDebugLayout> {
        match self {
            Self::Gap(layout) => Some(layout),
            Self::PieceTree(_) | Self::Rope(_) => None,
        }
    }
}

impl Default for TextBackendDebugLayout {
    fn default() -> Self {
        Self::Gap(GapDebugLayout::default())
    }
}
