use crate::buffer::position::{CharPos0, EmacsBytePos};

/// Backend-neutral text extent in GNU Emacs coordinate spaces.
///
/// `chars` is a 0-based character count/end position. `emacs_bytes` is a
/// logical Emacs byte count/end position. Concrete backends may have a
/// different physical storage byte coordinate, but that must not leak through
/// this type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextMetrics {
    chars: CharPos0,
    emacs_bytes: EmacsBytePos,
}

impl TextMetrics {
    pub const ZERO: Self = Self {
        chars: CharPos0::ZERO,
        emacs_bytes: EmacsBytePos::ZERO,
    };

    pub const fn new(chars: usize, emacs_bytes: usize) -> Self {
        Self {
            chars: CharPos0::new(chars),
            emacs_bytes: EmacsBytePos::new(emacs_bytes),
        }
    }

    pub const fn from_positions(chars: CharPos0, emacs_bytes: EmacsBytePos) -> Self {
        Self { chars, emacs_bytes }
    }

    pub const fn chars(self) -> usize {
        self.chars.get()
    }

    pub const fn emacs_bytes(self) -> usize {
        self.emacs_bytes.get()
    }

    pub const fn char_end(self) -> CharPos0 {
        self.chars
    }

    pub const fn emacs_byte_end(self) -> EmacsBytePos {
        self.emacs_bytes
    }

    pub const fn is_empty(self) -> bool {
        self.chars.get() == 0 && self.emacs_bytes.get() == 0
    }
}
