//! Position types for distinguishing buffer coordinate spaces.
//!
//! GNU Emacs uses 1-based positions everywhere (BEG=1, first char is at
//! position 1).  NeoMacs stores positions 0-based internally but exposes
//! them 1-based to Lisp.  These wrappers make the distinction type-safe
//! at the boundary so the compiler catches accidental mixing.

/// 0-based internal character position (first character = 0).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct CharPos0(usize);

/// 0-based logical Emacs byte position.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct EmacsBytePos(usize);

/// Count of logical Emacs characters.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct CharLen(usize);

/// Count of logical Emacs bytes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct EmacsByteLen(usize);

/// Half-open internal character range `[start, end)`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CharRange {
    start: CharPos0,
    end: CharPos0,
}

/// 1-based Lisp character position (first character = 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct LispCharPos1(i64);

/// Display column coordinate. This is not a buffer character position.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct DisplayColumn(usize);

/// Half-open logical Emacs byte range `[start, end)`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EmacsByteRange {
    start: EmacsBytePos,
    end: EmacsBytePos,
}

/// Accessible logical Emacs byte range `[BEGV_BYTE, ZV_BYTE)`.
///
/// This is still an Emacs byte range, but carrying the narrowing meaning in
/// the type keeps higher-level motion/search code from reaching directly into
/// raw buffer fields for `begv_byte` and `zv_byte`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccessibleEmacsByteRange {
    range: EmacsByteRange,
}

/// Accessible internal character range `[BEGV, ZV)`.
///
/// GNU syntax and parse code carries both character and byte positions. This
/// companion to `AccessibleEmacsByteRange` keeps those character bounds
/// explicit at call sites that must not scan outside the narrowed region.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccessibleCharRange {
    range: CharRange,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextPositionAnchor {
    char_pos: CharPos0,
    emacs_byte_pos: EmacsBytePos,
}

/// Bracketing anchors used for char<->byte conversion.
///
/// GNU Emacs' `marker.c` conversion code keeps the nearest known `(char,
/// byte)` pair below and above the target, considering point, gap, narrowing,
/// the last conversion, and markers.  This type keeps that paired state
/// explicit so callers cannot update one coordinate without the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPositionBounds {
    below: TextPositionAnchor,
    above: TextPositionAnchor,
}

impl TextPositionAnchor {
    pub const fn new(char_pos: CharPos0, emacs_byte_pos: EmacsBytePos) -> Self {
        Self {
            char_pos,
            emacs_byte_pos,
        }
    }

    pub const fn from_usize(char_pos: usize, emacs_byte_pos: usize) -> Self {
        Self {
            char_pos: CharPos0::new(char_pos),
            emacs_byte_pos: EmacsBytePos::new(emacs_byte_pos),
        }
    }

    pub const fn char_pos(self) -> CharPos0 {
        self.char_pos
    }

    pub const fn emacs_byte_pos(self) -> EmacsBytePos {
        self.emacs_byte_pos
    }

    pub const fn char_pos_usize(self) -> usize {
        self.char_pos.get()
    }

    pub const fn emacs_byte_pos_usize(self) -> usize {
        self.emacs_byte_pos.get()
    }
}

impl TextPositionBounds {
    pub const fn new(above: TextPositionAnchor) -> Self {
        Self {
            below: TextPositionAnchor::new(CharPos0::ZERO, EmacsBytePos::ZERO),
            above,
        }
    }

    pub const fn below(self) -> TextPositionAnchor {
        self.below
    }

    pub const fn above(self) -> TextPositionAnchor {
        self.above
    }

    pub fn consider_char_anchor(&mut self, target: CharPos0, anchor: TextPositionAnchor) {
        if anchor.char_pos() <= target && anchor.char_pos() > self.below.char_pos() {
            self.below = anchor;
        }
        if anchor.char_pos() >= target && anchor.char_pos() < self.above.char_pos() {
            self.above = anchor;
        }
    }

    pub fn consider_byte_anchor(&mut self, target: EmacsBytePos, anchor: TextPositionAnchor) {
        if anchor.emacs_byte_pos() <= target
            && anchor.emacs_byte_pos() > self.below.emacs_byte_pos()
        {
            self.below = anchor;
        }
        if anchor.emacs_byte_pos() >= target
            && anchor.emacs_byte_pos() < self.above.emacs_byte_pos()
        {
            self.above = anchor;
        }
    }

    pub fn char_below_distance(self, target: CharPos0) -> usize {
        target.get().saturating_sub(self.below.char_pos_usize())
    }

    pub fn char_above_distance(self, target: CharPos0) -> usize {
        self.above.char_pos_usize().saturating_sub(target.get())
    }

    pub fn byte_below_distance(self, target: EmacsBytePos) -> usize {
        target
            .get()
            .saturating_sub(self.below.emacs_byte_pos_usize())
    }

    pub fn byte_above_distance(self, target: EmacsBytePos) -> usize {
        self.above
            .emacs_byte_pos_usize()
            .saturating_sub(target.get())
    }

    pub fn char_target_is_near(self, target: CharPos0, distance: usize) -> bool {
        self.char_above_distance(target) < distance || self.char_below_distance(target) < distance
    }

    pub fn byte_target_is_near(self, target: EmacsBytePos, distance: usize) -> bool {
        self.byte_above_distance(target) < distance || self.byte_below_distance(target) < distance
    }

    pub fn nearest_char_anchor(self, target: CharPos0) -> TextPositionAnchor {
        if self.char_below_distance(target) <= self.char_above_distance(target) {
            self.below
        } else {
            self.above
        }
    }

    pub fn nearest_byte_anchor(self, target: EmacsBytePos) -> TextPositionAnchor {
        if self.byte_below_distance(target) <= self.byte_above_distance(target) {
            self.below
        } else {
            self.above
        }
    }

    pub fn min_char_walk(self, target: CharPos0) -> usize {
        self.char_below_distance(target)
            .min(self.char_above_distance(target))
    }

    pub fn min_byte_walk(self, target: EmacsBytePos) -> usize {
        self.byte_below_distance(target)
            .min(self.byte_above_distance(target))
    }
}

impl CharPos0 {
    pub const ZERO: Self = Self(0);

    pub const fn new(pos: usize) -> Self {
        Self(pos)
    }

    pub const fn get(self) -> usize {
        self.0
    }

    /// Convert to a 1-based Lisp position.
    pub fn to_lisp(self) -> LispCharPos1 {
        LispCharPos1(self.0 as i64 + 1)
    }

    /// Convert from a 1-based Lisp position (clamped to 0).
    pub fn from_lisp(p: LispCharPos1) -> Self {
        Self((p.0 - 1).max(0) as usize)
    }

    /// Convert from a raw `usize` (for internal 0-based use).
    pub const fn from_usize(pos: usize) -> Self {
        Self(pos)
    }

    pub const fn add_len(self, len: CharLen) -> Self {
        Self(self.0 + len.get())
    }
}

impl EmacsBytePos {
    pub const ZERO: Self = Self(0);

    pub const fn new(pos: usize) -> Self {
        Self(pos)
    }

    pub const fn get(self) -> usize {
        self.0
    }

    /// Convert to a 1-based Lisp position.
    /// Requires the buffer for byte-to-char conversion.
    pub fn to_lisp(self, text: &super::BufferText) -> LispCharPos1 {
        text.emacs_byte_pos_to_char_pos(self).to_lisp()
    }

    /// Convert from a 1-based Lisp position.
    /// Requires the buffer for char-to-byte conversion.
    pub fn from_lisp(p: LispCharPos1, text: &super::BufferText) -> Self {
        text.char_pos_to_emacs_byte_pos(CharPos0::from_lisp(p))
    }

    pub const fn add_len(self, len: EmacsByteLen) -> Self {
        Self(self.0 + len.get())
    }
}

impl CharLen {
    pub const ZERO: Self = Self(0);

    pub const fn new(len: usize) -> Self {
        Self(len)
    }

    pub const fn get(self) -> usize {
        self.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl CharRange {
    pub const fn new(start: CharPos0, end: CharPos0) -> Self {
        Self { start, end }
    }

    pub const fn from_start_len(start: CharPos0, len: CharLen) -> Self {
        Self {
            start,
            end: start.add_len(len),
        }
    }

    pub const fn from_usize(start: usize, end: usize) -> Self {
        Self {
            start: CharPos0::new(start),
            end: CharPos0::new(end),
        }
    }

    pub const fn start(self) -> CharPos0 {
        self.start
    }

    pub const fn end(self) -> CharPos0 {
        self.end
    }

    pub const fn start_usize(self) -> usize {
        self.start.get()
    }

    pub const fn end_usize(self) -> usize {
        self.end.get()
    }

    pub const fn len(self) -> CharLen {
        CharLen::new(self.end.get().saturating_sub(self.start.get()))
    }

    pub const fn is_empty(self) -> bool {
        self.start.get() >= self.end.get()
    }
}

impl EmacsByteLen {
    pub const ZERO: Self = Self(0);

    pub const fn new(len: usize) -> Self {
        Self(len)
    }

    pub const fn get(self) -> usize {
        self.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl EmacsByteRange {
    pub const fn new(start: EmacsBytePos, end: EmacsBytePos) -> Self {
        Self { start, end }
    }

    pub const fn from_start_len(start: EmacsBytePos, len: EmacsByteLen) -> Self {
        Self {
            start,
            end: start.add_len(len),
        }
    }

    pub const fn from_usize(start: usize, end: usize) -> Self {
        Self {
            start: EmacsBytePos::new(start),
            end: EmacsBytePos::new(end),
        }
    }

    pub const fn start(self) -> EmacsBytePos {
        self.start
    }

    pub const fn end(self) -> EmacsBytePos {
        self.end
    }

    pub const fn start_usize(self) -> usize {
        self.start.get()
    }

    pub const fn end_usize(self) -> usize {
        self.end.get()
    }

    pub const fn len(self) -> EmacsByteLen {
        EmacsByteLen::new(self.end.get().saturating_sub(self.start.get()))
    }

    pub const fn is_empty(self) -> bool {
        self.start.get() >= self.end.get()
    }
}

impl AccessibleEmacsByteRange {
    pub const fn new(range: EmacsByteRange) -> Self {
        Self { range }
    }

    pub const fn range(self) -> EmacsByteRange {
        self.range
    }

    pub const fn start(self) -> EmacsBytePos {
        self.range.start()
    }

    pub const fn end(self) -> EmacsBytePos {
        self.range.end()
    }

    pub const fn start_usize(self) -> usize {
        self.range.start_usize()
    }

    pub const fn end_usize(self) -> usize {
        self.range.end_usize()
    }

    pub fn contains_usize(self, pos: usize) -> bool {
        self.start_usize() <= pos && pos < self.end_usize()
    }

    pub fn contains_preceding_char_boundary_usize(self, pos: usize) -> bool {
        self.start_usize() < pos && pos <= self.end_usize()
    }

    pub fn clamp_usize(self, pos: usize) -> usize {
        pos.clamp(self.start_usize(), self.end_usize())
    }
}

impl AccessibleCharRange {
    pub const fn new(range: CharRange) -> Self {
        Self { range }
    }

    pub const fn range(self) -> CharRange {
        self.range
    }

    pub const fn start(self) -> CharPos0 {
        self.range.start()
    }

    pub const fn end(self) -> CharPos0 {
        self.range.end()
    }

    pub const fn start_usize(self) -> usize {
        self.range.start_usize()
    }

    pub const fn end_usize(self) -> usize {
        self.range.end_usize()
    }

    pub const fn len(self) -> CharLen {
        self.range.len()
    }

    pub const fn is_empty(self) -> bool {
        self.range.is_empty()
    }

    pub fn contains_usize(self, pos: usize) -> bool {
        self.start_usize() <= pos && pos < self.end_usize()
    }

    pub fn contains_boundary_usize(self, pos: usize) -> bool {
        self.start_usize() <= pos && pos <= self.end_usize()
    }

    pub fn clamp_usize(self, pos: usize) -> usize {
        pos.clamp(self.start_usize(), self.end_usize())
    }
}

impl LispCharPos1 {
    pub const fn new(pos: i64) -> Self {
        Self(pos)
    }

    /// Convert to 0-based internal char position.
    pub fn to_char_pos(self) -> CharPos0 {
        CharPos0::from_lisp(self)
    }

    /// Convert to 0-based internal byte position.
    pub fn to_byte_pos(self, text: &super::BufferText) -> EmacsBytePos {
        EmacsBytePos::from_lisp(self, text)
    }

    /// The raw i64 value.
    pub fn as_i64(self) -> i64 {
        self.0
    }
}

impl DisplayColumn {
    pub const ZERO: Self = Self(0);

    pub const fn new(pos: usize) -> Self {
        Self(pos)
    }

    pub const fn get(self) -> usize {
        self.0
    }
}

impl From<usize> for CharPos0 {
    fn from(pos: usize) -> Self {
        Self(pos)
    }
}

impl From<CharPos0> for usize {
    fn from(p: CharPos0) -> Self {
        p.0
    }
}

impl From<usize> for CharLen {
    fn from(len: usize) -> Self {
        Self(len)
    }
}

impl From<CharLen> for usize {
    fn from(len: CharLen) -> Self {
        len.0
    }
}

impl From<usize> for EmacsBytePos {
    fn from(pos: usize) -> Self {
        Self(pos)
    }
}

impl From<EmacsBytePos> for usize {
    fn from(p: EmacsBytePos) -> Self {
        p.0
    }
}

impl From<usize> for EmacsByteLen {
    fn from(len: usize) -> Self {
        Self(len)
    }
}

impl From<EmacsByteLen> for usize {
    fn from(len: EmacsByteLen) -> Self {
        len.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_position_bounds_keep_char_and_byte_anchor_pairs_together() {
        let mut bounds = TextPositionBounds::new(TextPositionAnchor::from_usize(20, 40));

        bounds.consider_char_anchor(CharPos0::new(10), TextPositionAnchor::from_usize(5, 7));
        bounds.consider_char_anchor(CharPos0::new(10), TextPositionAnchor::from_usize(15, 27));
        bounds.consider_char_anchor(CharPos0::new(10), TextPositionAnchor::from_usize(12, 24));

        assert_eq!(bounds.below(), TextPositionAnchor::from_usize(5, 7));
        assert_eq!(bounds.above(), TextPositionAnchor::from_usize(12, 24));
        assert_eq!(
            bounds.nearest_char_anchor(CharPos0::new(10)),
            TextPositionAnchor::from_usize(12, 24)
        );

        let mut byte_bounds = TextPositionBounds::new(TextPositionAnchor::from_usize(20, 40));
        byte_bounds.consider_byte_anchor(
            EmacsBytePos::new(30),
            TextPositionAnchor::from_usize(11, 29),
        );
        byte_bounds.consider_byte_anchor(
            EmacsBytePos::new(30),
            TextPositionAnchor::from_usize(13, 33),
        );

        assert_eq!(byte_bounds.below(), TextPositionAnchor::from_usize(11, 29));
        assert_eq!(byte_bounds.above(), TextPositionAnchor::from_usize(13, 33));
        assert_eq!(byte_bounds.byte_below_distance(EmacsBytePos::new(30)), 1);
        assert_eq!(byte_bounds.byte_above_distance(EmacsBytePos::new(30)), 3);
        assert_eq!(
            byte_bounds.nearest_byte_anchor(EmacsBytePos::new(30)),
            TextPositionAnchor::from_usize(11, 29)
        );
    }
}

// Transitional aliases for older call sites. New code should use the explicit
// coordinate-space names above.
pub type CharPos = CharPos0;
pub type BytePos = EmacsBytePos;
pub type LispPos = LispCharPos1;
