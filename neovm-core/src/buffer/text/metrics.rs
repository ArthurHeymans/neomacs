use crate::buffer::position::{CharLen, CharPos0, EmacsByteLen, EmacsBytePos};

use super::TextExtent;

/// Backend-neutral text extent in GNU Emacs coordinate spaces.
///
/// `chars` and `emacs_bytes` are lengths. Concrete backends may have a
/// different physical storage byte coordinate, but that must not leak through
/// this type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextMetrics {
    chars: CharLen,
    emacs_bytes: EmacsByteLen,
}

impl TextMetrics {
    pub const ZERO: Self = Self {
        chars: CharLen::ZERO,
        emacs_bytes: EmacsByteLen::ZERO,
    };

    pub const fn new(chars: usize, emacs_bytes: usize) -> Self {
        Self {
            chars: CharLen::new(chars),
            emacs_bytes: EmacsByteLen::new(emacs_bytes),
        }
    }

    pub const fn from_lengths(chars: CharLen, emacs_bytes: EmacsByteLen) -> Self {
        Self { chars, emacs_bytes }
    }

    pub const fn from_extent(extent: TextExtent) -> Self {
        Self {
            chars: extent.chars(),
            emacs_bytes: extent.emacs_bytes(),
        }
    }

    pub const fn chars(self) -> usize {
        self.chars.get()
    }

    pub const fn emacs_bytes(self) -> usize {
        self.emacs_bytes.get()
    }

    pub const fn char_end(self) -> CharPos0 {
        CharPos0::new(self.chars.get())
    }

    pub const fn emacs_byte_end(self) -> EmacsBytePos {
        EmacsBytePos::new(self.emacs_bytes.get())
    }

    pub const fn is_empty(self) -> bool {
        self.chars.get() == 0 && self.emacs_bytes.get() == 0
    }
}
