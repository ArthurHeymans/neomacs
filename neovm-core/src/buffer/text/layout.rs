use crate::buffer::position::{CharPos0, EmacsBytePos};
use crate::buffer::text::TextMetrics;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GapCompatState {
    pos: CharPos0,
    size: usize,
}

impl GapCompatState {
    pub const fn new(pos: CharPos0, size: usize) -> Self {
        Self { pos, size }
    }

    pub const fn pos(self) -> CharPos0 {
        self.pos
    }

    pub const fn size(self) -> usize {
        self.size
    }

    pub fn lisp_position(self) -> i64 {
        self.pos.to_lisp().as_i64()
    }

    pub const fn with_pos(self, pos: CharPos0) -> Self {
        Self {
            pos,
            size: self.size,
        }
    }

    pub const fn with_size(self, size: usize) -> Self {
        Self {
            pos: self.pos,
            size,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GapDebugLayout {
    pub gpt: CharPos0,
    pub z: CharPos0,
    pub gpt_byte: EmacsBytePos,
    pub z_byte: EmacsBytePos,
    pub gap_size: usize,
}

impl GapDebugLayout {
    pub const fn compat_state(self) -> GapCompatState {
        GapCompatState::new(self.gpt, self.gap_size)
    }
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
