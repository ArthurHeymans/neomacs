use crate::buffer::position::{CharLen, CharPos0, EmacsByteLen, EmacsBytePos};
use crate::buffer::text::TextMetrics;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GapCompatState {
    pos: CharPos0,
    byte_len: EmacsByteLen,
}

impl GapCompatState {
    pub const fn new(pos: CharPos0, byte_len: EmacsByteLen) -> Self {
        Self { pos, byte_len }
    }

    pub const fn pos(self) -> CharPos0 {
        self.pos
    }

    pub const fn byte_len(self) -> EmacsByteLen {
        self.byte_len
    }

    pub fn lisp_position(self) -> i64 {
        self.pos.to_lisp().as_i64()
    }

    pub fn lisp_size(self) -> i64 {
        self.byte_len.get() as i64
    }

    pub const fn with_pos(self, pos: CharPos0) -> Self {
        Self {
            pos,
            byte_len: self.byte_len,
        }
    }

    pub const fn with_byte_len(self, byte_len: EmacsByteLen) -> Self {
        Self {
            pos: self.pos,
            byte_len,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GapDebugLayout {
    pub gpt: CharPos0,
    pub z: CharPos0,
    pub gpt_byte: EmacsBytePos,
    pub z_byte: EmacsBytePos,
    pub gap_byte_len: EmacsByteLen,
}

impl GapDebugLayout {
    pub const fn compat_state(self) -> GapCompatState {
        GapCompatState::new(self.gpt, self.gap_byte_len)
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
            Self::Gap(layout) => TextMetrics::from_lengths(
                CharLen::new(layout.z.get()),
                EmacsByteLen::new(layout.z_byte.get()),
            ),
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
