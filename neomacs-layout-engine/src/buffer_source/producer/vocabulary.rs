//! The typed producer vocabulary: what an element is, where it came from, and
//! what its glyphs get stamped with.
//!
//! GNU keeps glyph provenance as a `(charpos, object)` PAIR (`struct glyph`,
//! dispextern.h:460-483) and two position tracks on the iterator —
//! `it->current.pos`, the honest buffer position that feeds row min/max, versus
//! `it->position`, what actually lands on the glyph (a string index while a
//! string is being displayed, xdisp.c:9609-9613). This engine collapses both
//! into one `usize` per glyph with `NO_BUFFER_POSITION_CHARPOS` standing in for
//! every non-buffer case, so a truncation mark, a line-end space and a
//! `display`-string glyph are indistinguishable once written. The types here
//! restore the distinction ahead of the consumers that will need it, without
//! changing a single stamped VALUE.
//!
//! Scope freeze (design section 4.7): phase 4 makes provenance TYPED, not
//! GNU-VALUED. String glyphs keep this engine's covered/anchor buffer stamps,
//! which is what lets every rung shadow-prove glyph-for-glyph equality.

use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayRowBreakReason, DisplaySourceId, DisplaySourcePosition,
    DisplayStretchWidth, RenderFaceRef,
};
use crate::display_source::{DisplaySourceStepItem, DisplaySourceTextPosition};
use neomacs_display_protocol::glyph_matrix::NO_BUFFER_POSITION_CHARPOS;
use neovm_core::buffer::CharPos0;

/// Identifies the Lisp string a [`GlyphProvenance::Str`] index is relative to.
/// The engine's existing source-id handle; GNU stores the string object itself.
pub(crate) type ProducedStringId = DisplaySourceId;

/// The producer's scan track: charpos plus byte index, ALWAYS through buffer
/// text (GNU `it->current.pos`). The walk's existing position type — the
/// producer does not get a second, drifting copy.
pub(crate) type BufferScanPos = DisplaySourceTextPosition;

/// What a produced element, and every glyph it makes, is attributed to. GNU's
/// `(charpos, object)` pair as one value, so a charpos can never be read in the
/// wrong coordinate space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GlyphProvenance {
    /// Buffer text, and the stretch exception (design section 4.3): `charpos`
    /// is an honest buffer position. Today's covered-charpos replacement
    /// glyphs are `Buffer(covered_start)` — representable unchanged.
    Buffer { charpos: CharPos0 },
    /// A Lisp string element: `index` is relative to THAT string.
    ///
    /// UNPOPULATED in phase 4 by design (section 4.7). Nothing the producer
    /// yields and nothing the append path stamps carries this arm; the only
    /// way to reach it is [`GlyphProvenance::from_source_position`] on a raw
    /// `LispString` item position, because replacement strings are rewritten to
    /// covered buffer provenance before they are appended (the
    /// `DisplayTextSourceMapping::SourceMapped` arm in display_row/builder.rs).
    /// Adopting GNU's string-index stamps means migrating the cursor and mouse
    /// two-step lookups with it, which is a post-phase parity project — doing
    /// it inside phase 4 would change stamp VALUES and break glyph-for-glyph
    /// shadowing by definition.
    Str {
        string: ProducedStringId,
        index: usize,
    },
    /// Redisplay's own glyph: truncation and continuation marks, the appended
    /// newline space, extend fill, prefix glyphs. Carries the GNU sentinel
    /// semantics explicitly instead of one magic charpos.
    Redisplay(RedisplaySentinel),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RedisplaySentinel {
    /// GNU charpos 0: the `append_space_for_newline` glyph at a real line end,
    /// extend-to-end-of-line stretches, the fill-column indicator.
    LineEnd,
    /// GNU charpos -1: truncation and continuation marks, line numbers, the
    /// TTY overlay arrow.
    Mark,
    /// GNU's empty-line patch (xdisp.c:26535-26537): redisplay's own glyph, but
    /// stamped with the newline's REAL buffer position. This engine does the
    /// same at row level — an empty line's row reports its newline's charpos
    /// rather than a positionless (0,0), which is what closed the scroll and
    /// edit-reuse corruption class (e772f82ed).
    EmptyLineNewline { charpos: CharPos0 },
}

/// Mirrors the renderer's private `DisplayTextSourceMapping` (builder.rs): do a
/// run's glyphs advance the stamp per character, or does every glyph carry the
/// run's start?
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RunStamping {
    /// Buffer text: glyph N carries start + N.
    NaturalText,
    /// A replacement's covered range: every glyph carries the covered start,
    /// however many glyphs the replacement text produced.
    Covered,
}

impl GlyphProvenance {
    pub(crate) const fn buffer(charpos: CharPos0) -> Self {
        Self::Buffer { charpos }
    }

    pub(crate) const fn line_end() -> Self {
        Self::Redisplay(RedisplaySentinel::LineEnd)
    }

    pub(crate) const fn mark() -> Self {
        Self::Redisplay(RedisplaySentinel::Mark)
    }

    pub(crate) const fn empty_line_newline(charpos: CharPos0) -> Self {
        Self::Redisplay(RedisplaySentinel::EmptyLineNewline { charpos })
    }

    /// The legacy bridge from an item's span position. Synthetic spans are
    /// redisplay's own glyph sources (special_glyphs.rs, the hscroll marker),
    /// so they carry the `Mark` sentinel rather than their synthetic offset.
    pub(crate) fn from_source_position(position: &DisplaySourcePosition) -> Self {
        match position {
            DisplaySourcePosition::Buffer { char_pos, .. } => Self::buffer(*char_pos),
            DisplaySourcePosition::LispString {
                source_id,
                char_index,
                ..
            } => Self::Str {
                string: *source_id,
                index: *char_index,
            },
            DisplaySourcePosition::Synthetic { .. } => Self::mark(),
        }
    }

    /// Advance a stamp by whole characters. Redisplay sentinels do not advance:
    /// every glyph of a mark or a line-end fill carries the same sentinel.
    pub(crate) fn advanced_by(self, char_offset: usize) -> Self {
        match self {
            Self::Buffer { charpos } => Self::buffer(CharPos0::new(charpos.get() + char_offset)),
            Self::Str { string, index } => Self::Str {
                string,
                index: index + char_offset,
            },
            Self::Redisplay(sentinel) => Self::Redisplay(sentinel),
        }
    }

    /// Provenance of glyph `char_offset` of an ordinary buffer-text run.
    pub(crate) fn natural_text_glyph(
        span_start: &DisplaySourcePosition,
        char_offset: usize,
    ) -> Self {
        Self::from_source_position(span_start).advanced_by(char_offset)
    }

    /// Provenance of EVERY glyph of a covered replacement run.
    pub(crate) fn covered_text_glyph(span_start: &DisplaySourcePosition) -> Self {
        Self::from_source_position(span_start)
    }

    /// The value this provenance writes into a glyph's single `charpos` field.
    /// Lossy on purpose — it is the field the vocabulary exists to disambiguate.
    pub(crate) fn glyph_charpos(self) -> usize {
        match self {
            Self::Buffer { charpos } => charpos.get(),
            Self::Redisplay(RedisplaySentinel::EmptyLineNewline { charpos }) => charpos.get(),
            Self::Redisplay(RedisplaySentinel::LineEnd | RedisplaySentinel::Mark) => {
                NO_BUFFER_POSITION_CHARPOS
            }
            Self::Str { .. } => {
                debug_assert!(
                    false,
                    "phase 4 never stamps Str provenance (design section 4.7); \
                     replacement strings are rewritten to covered buffer provenance"
                );
                NO_BUFFER_POSITION_CHARPOS
            }
        }
    }

    /// The buffer position this element is attributed to, for consumers that
    /// need an honest one (cursor placement, row min/max). Redisplay's own
    /// glyphs have none even when they carry a real charpos for row bounds.
    pub(crate) fn buffer_charpos(self) -> Option<CharPos0> {
        match self {
            Self::Buffer { charpos } => Some(charpos),
            Self::Str { .. } | Self::Redisplay(_) => None,
        }
    }
}

/// GNU `it->current.pos` and `it->position` as one struct, so they can never be
/// advanced independently by accident. `scan` always walks buffer text; `stamp`
/// is what the next element's glyphs carry, and differs from `scan` exactly
/// while a producer frame is active.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProducerPosition {
    scan: BufferScanPos,
    stamp: GlyphProvenance,
}

impl ProducerPosition {
    /// Producing buffer text: the stamp IS the scan position.
    pub(crate) fn buffer_at(scan: BufferScanPos) -> Self {
        Self {
            scan,
            stamp: GlyphProvenance::buffer(CharPos0::new(scan.charpos().max(0) as usize)),
        }
    }

    pub(crate) const fn with_stamp(scan: BufferScanPos, stamp: GlyphProvenance) -> Self {
        Self { scan, stamp }
    }

    pub(crate) const fn scan(self) -> BufferScanPos {
        self.scan
    }

    pub(crate) const fn stamp(self) -> GlyphProvenance {
        self.stamp
    }

    /// The scan and stamp today's pipeline item carries.
    pub(crate) fn from_step_item(item: &DisplaySourceStepItem) -> Self {
        let step_char = item.source_step_char();
        let scan = BufferScanPos::new(step_char.start_byte_idx(), step_char.start_charpos());
        Self::with_stamp(
            scan,
            GlyphProvenance::from_source_position(&item.item().span.start),
        )
    }
}

/// One display element: the typed output of GNU's `get_next_display_element`.
/// Payloads start at what the legacy bridge below can fill and grow per rung.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ProducedElement {
    /// One character, buffer- or string-stamped per its position.
    Char(ProducedChar),
    /// A homogeneous run — a pure batching optimization over `Char`, never a
    /// different meaning: consuming k characters and asking again must yield
    /// the rest.
    Run(ProducedRun),
    /// A stretch glyph (`(space ...)` specs). Buffer-stamped, design 4.3.
    Stretch(ProducedStretch),
    /// A line end. Provenance distinguishes a buffer newline from a
    /// string-supplied one (GNU `ends_in_newline_from_string_p`).
    RowBreak(ProducedRowBreak),
    /// End of the visible text window.
    EndOfText,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProducedChar {
    position: ProducerPosition,
    ch: char,
    face: RenderFaceRef,
    avoid_cursor: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProducedRun {
    position: ProducerPosition,
    text: Box<str>,
    face: RenderFaceRef,
    stamping: RunStamping,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProducedStretch {
    position: ProducerPosition,
    width: DisplayStretchWidth,
    face: RenderFaceRef,
    avoid_cursor: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProducedRowBreak {
    position: ProducerPosition,
    reason: DisplayRowBreakReason,
}

impl ProducedChar {
    pub(crate) const fn position(&self) -> ProducerPosition {
        self.position
    }

    pub(crate) const fn ch(&self) -> char {
        self.ch
    }

    pub(crate) const fn face(&self) -> RenderFaceRef {
        self.face
    }

    /// GNU `avoid_cursor_p` (xdisp.c:32693): the cursor never lands here.
    pub(crate) const fn avoid_cursor(&self) -> bool {
        self.avoid_cursor
    }
}

impl ProducedRun {
    pub(crate) const fn position(&self) -> ProducerPosition {
        self.position
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) const fn face(&self) -> RenderFaceRef {
        self.face
    }

    pub(crate) fn is_covered_provenance(&self) -> bool {
        self.stamping == RunStamping::Covered
    }

    /// Provenance of the run's `char_offset`-th glyph, following the same rule
    /// the append path uses.
    pub(crate) fn glyph_provenance(&self, char_offset: usize) -> GlyphProvenance {
        match self.stamping {
            RunStamping::NaturalText => self.position.stamp().advanced_by(char_offset),
            RunStamping::Covered => self.position.stamp(),
        }
    }
}

impl ProducedStretch {
    pub(crate) const fn position(&self) -> ProducerPosition {
        self.position
    }

    pub(crate) fn width(&self) -> &DisplayStretchWidth {
        &self.width
    }

    pub(crate) const fn face(&self) -> RenderFaceRef {
        self.face
    }

    pub(crate) const fn avoid_cursor(&self) -> bool {
        self.avoid_cursor
    }
}

impl ProducedRowBreak {
    pub(crate) const fn position(&self) -> ProducerPosition {
        self.position
    }

    pub(crate) const fn reason(&self) -> DisplayRowBreakReason {
        self.reason
    }
}

impl ProducedElement {
    /// Bridge from today's item vocabulary at a known scan position. Kinds the
    /// element vocabulary does not model yet (glyphless, media replacements)
    /// return `None` and keep flowing through the legacy item path until their
    /// rung, rather than being given an invented element shape.
    pub(crate) fn from_item(item: &DisplayItem, scan: BufferScanPos) -> Option<Self> {
        let stamp = GlyphProvenance::from_source_position(&item.span.start);
        let position = ProducerPosition::with_stamp(scan, stamp);
        match &item.kind {
            DisplayItemKind::TextRun(run) => Some(Self::Run(ProducedRun {
                position,
                text: run.text.clone(),
                face: item.face,
                stamping: RunStamping::NaturalText,
            })),
            DisplayItemKind::SourceMappedText(text) => Some(Self::Run(ProducedRun {
                position,
                text: text.text.clone(),
                face: item.face,
                stamping: RunStamping::Covered,
            })),
            DisplayItemKind::ControlChar { ch } => Some(Self::Char(ProducedChar {
                position,
                ch: *ch,
                face: item.face,
                avoid_cursor: false,
            })),
            DisplayItemKind::Stretch(stretch) => Some(Self::Stretch(ProducedStretch {
                position,
                width: stretch.width.clone(),
                face: item.face,
                avoid_cursor: false,
            })),
            DisplayItemKind::RowBreak(row_break) => Some(Self::RowBreak(ProducedRowBreak {
                position,
                reason: row_break.reason,
            })),
            DisplayItemKind::Glyphless(_) | DisplayItemKind::MediaReplacement(_) => None,
        }
    }

    /// Bridge from today's pipeline step item, whose step char supplies the
    /// scan position.
    pub(crate) fn from_step_item(item: &DisplaySourceStepItem) -> Option<Self> {
        Self::from_item(item.item(), ProducerPosition::from_step_item(item).scan())
    }
}

#[cfg(test)]
#[path = "vocabulary_test.rs"]
mod tests;
