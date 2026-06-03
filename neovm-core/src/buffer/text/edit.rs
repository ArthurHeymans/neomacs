use crate::buffer::position::{
    CharLen, CharPos0, CharRange, EmacsByteLen, EmacsBytePos, EmacsByteRange,
};

/// Logical size of inserted or deleted buffer text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextExtent {
    chars: CharLen,
    emacs_bytes: EmacsByteLen,
}

/// Insertion point plus the text size known by the caller.
///
/// GNU `insert_1_both` receives character and byte lengths separately.  This
/// type keeps that contract explicit at the Rust boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextInsertion {
    byte_pos: EmacsBytePos,
    char_pos: CharPos0,
    extent: TextExtent,
}

/// Half-open edit range with both byte and character coordinates.
///
/// GNU `del_range_both` carries `from`, `from_byte`, `to`, and `to_byte`.
/// Keeping the same shape here avoids recomputing one coordinate space from the
/// other and makes byte/character mixups visible in the type signature.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextEditRange {
    byte_range: EmacsByteRange,
    char_start: CharPos0,
    char_end: CharPos0,
}

impl TextExtent {
    pub const ZERO: Self = Self {
        chars: CharLen::ZERO,
        emacs_bytes: EmacsByteLen::ZERO,
    };

    pub const fn new(chars: CharLen, emacs_bytes: EmacsByteLen) -> Self {
        Self { chars, emacs_bytes }
    }

    pub const fn from_usize(chars: usize, emacs_bytes: usize) -> Self {
        Self {
            chars: CharLen::new(chars),
            emacs_bytes: EmacsByteLen::new(emacs_bytes),
        }
    }

    pub const fn chars(self) -> CharLen {
        self.chars
    }

    pub const fn emacs_bytes(self) -> EmacsByteLen {
        self.emacs_bytes
    }

    pub const fn is_empty(self) -> bool {
        self.chars.is_empty() && self.emacs_bytes.is_empty()
    }
}

impl TextInsertion {
    pub const fn new(byte_pos: EmacsBytePos, char_pos: CharPos0, extent: TextExtent) -> Self {
        Self {
            byte_pos,
            char_pos,
            extent,
        }
    }

    pub const fn from_usize(
        byte_pos: usize,
        char_pos: usize,
        chars: usize,
        emacs_bytes: usize,
    ) -> Self {
        Self {
            byte_pos: EmacsBytePos::new(byte_pos),
            char_pos: CharPos0::new(char_pos),
            extent: TextExtent::from_usize(chars, emacs_bytes),
        }
    }

    pub const fn byte_pos(self) -> EmacsBytePos {
        self.byte_pos
    }

    pub const fn char_pos(self) -> CharPos0 {
        self.char_pos
    }

    pub const fn extent(self) -> TextExtent {
        self.extent
    }

    pub const fn byte_pos_usize(self) -> usize {
        self.byte_pos.get()
    }

    pub const fn char_pos_usize(self) -> usize {
        self.char_pos.get()
    }
}

impl TextEditRange {
    pub const fn new(byte_range: EmacsByteRange, char_start: CharPos0, char_end: CharPos0) -> Self {
        Self {
            byte_range,
            char_start,
            char_end,
        }
    }

    pub const fn from_usize(
        byte_start: usize,
        byte_end: usize,
        char_start: usize,
        char_end: usize,
    ) -> Self {
        Self {
            byte_range: EmacsByteRange::from_usize(byte_start, byte_end),
            char_start: CharPos0::new(char_start),
            char_end: CharPos0::new(char_end),
        }
    }

    pub const fn byte_range(self) -> EmacsByteRange {
        self.byte_range
    }

    pub const fn byte_start(self) -> EmacsBytePos {
        self.byte_range.start()
    }

    pub const fn byte_end(self) -> EmacsBytePos {
        self.byte_range.end()
    }

    pub const fn char_start(self) -> CharPos0 {
        self.char_start
    }

    pub const fn char_end(self) -> CharPos0 {
        self.char_end
    }

    pub const fn char_range(self) -> CharRange {
        CharRange::new(self.char_start, self.char_end)
    }

    pub const fn byte_start_usize(self) -> usize {
        self.byte_range.start_usize()
    }

    pub const fn byte_end_usize(self) -> usize {
        self.byte_range.end_usize()
    }

    pub const fn char_start_usize(self) -> usize {
        self.char_start.get()
    }

    pub const fn char_end_usize(self) -> usize {
        self.char_end.get()
    }

    pub const fn byte_len(self) -> EmacsByteLen {
        self.byte_range.len()
    }

    pub const fn char_len(self) -> CharLen {
        CharLen::new(self.char_end.get().saturating_sub(self.char_start.get()))
    }

    pub const fn extent(self) -> TextExtent {
        TextExtent::new(self.char_len(), self.byte_len())
    }

    pub const fn is_empty(self) -> bool {
        self.byte_range.is_empty()
    }
}
