//! Position types for distinguishing 0-based internal positions from
//! 1-based Lisp positions.
//!
//! GNU Emacs uses 1-based positions everywhere (BEG=1, first char is at
//! position 1).  NeoMacs stores positions 0-based internally but exposes
//! them 1-based to Lisp.  These wrappers make the distinction type-safe
//! at the boundary so the compiler catches accidental mixing.

/// 0-based internal character position (first character = 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct CharPos(pub usize);

/// 0-based internal byte position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct BytePos(pub usize);

/// 1-based Lisp character position (first character = 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LispPos(pub i64);

impl CharPos {
    /// Convert to a 1-based Lisp position.
    pub fn to_lisp(self) -> LispPos {
        LispPos(self.0 as i64 + 1)
    }

    /// Convert from a 1-based Lisp position (clamped to 0).
    pub fn from_lisp(p: LispPos) -> Self {
        CharPos((p.0 - 1).max(0) as usize)
    }

    /// Convert from a raw `usize` (for internal 0-based use).
    pub const fn from_usize(pos: usize) -> Self {
        CharPos(pos)
    }
}

impl BytePos {
    /// Convert to a 1-based Lisp position.
    /// Requires the buffer for byte-to-char conversion.
    pub fn to_lisp(self, text: &super::BufferText) -> LispPos {
        LispPos(text.byte_to_char(self.0) as i64 + 1)
    }

    /// Convert from a 1-based Lisp position.
    /// Requires the buffer for char-to-byte conversion.
    pub fn from_lisp(p: LispPos, text: &super::BufferText) -> Self {
        let char_pos = (p.0 - 1).max(0) as usize;
        BytePos(text.char_to_byte(char_pos))
    }
}

impl LispPos {
    /// Convert to 0-based internal char position.
    pub fn to_char_pos(self) -> CharPos {
        CharPos::from_lisp(self)
    }

    /// Convert to 0-based internal byte position.
    pub fn to_byte_pos(self, text: &super::BufferText) -> BytePos {
        BytePos::from_lisp(self, text)
    }

    /// The raw i64 value.
    pub fn as_i64(self) -> i64 {
        self.0
    }
}

// Convenience: usize -> CharPos
impl From<usize> for CharPos {
    fn from(pos: usize) -> Self { CharPos(pos) }
}

// Convenience: CharPos -> usize
impl From<CharPos> for usize {
    fn from(p: CharPos) -> Self { p.0 }
}

// Convenience: BytePos -> usize
impl From<BytePos> for usize {
    fn from(p: BytePos) -> Self { p.0 }
}

// Convenience: usize -> BytePos
impl From<usize> for BytePos {
    fn from(pos: usize) -> Self { BytePos(pos) }
}
