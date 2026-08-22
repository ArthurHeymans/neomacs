//! What THIS terminal can render — GNU's `struct tty_display_info` capability
//! fields, resolved from terminfo once per terminal.
//!
//! GNU asks terminfo what the terminal can do (`term.c:init_tty`), stores the
//! answer on the terminal, and then consults it from exactly two places:
//! `turn_on_face`, which emits the capability's sequence (with documented
//! fallbacks — italics become dim, a styled underline becomes a plain one), and
//! `tty_capable_p`, which answers `display-supports-face-attributes-p`.
//!
//! neomacs had neither: the renderer hardcoded SGR sequences, so it emitted
//! `ESC [ 3 m` for `:slant italic` on a terminal whose terminfo has no `sitm`
//! (GNU emits its dim fallback there), and the Lisp predicate had no tty branch
//! at all, answering nil for bold and underline that GNU reports as supported.
//! Those are the same fact answered two different wrong ways, so this type is
//! the single answer both paths read.
//!
//! What each capability is SPELLED as is part of that same fact.  GNU's fields
//! are `const char *` and `turn_on_face`'s guard IS the pointer
//! (`OUTPUT1_IF (tty, tty->TS_enter_bold_mode)`, src/term.c:2061), so presence
//! and bytes are one answer.  Carrying a `bool` here and spelling the sequence
//! in the writer made them two, and the two disagree on the database ncurses
//! ships: of its 1,862 unique entries, 448 of the 1,303 that have `us` spell it
//! something other than `ESC [ 4 m`, 234 of 996 spell `md` something other than
//! `ESC [ 1 m`, and 281 of 616 spell `mh` something other than `ESC [ 2 m`
//! (ledger 186).  `so` was already carried as bytes, which is why inverse video
//! was the one attribute this port got right on `screen`, whose standout is
//! `ESC [ 3 m`.

use crate::face::UnderlineStyle;

/// Terminfo `ncv` (`NC`): attributes that CANNOT be combined with colors on this
/// terminal. Bit values are GNU's own `NC_*` enum (src/term.c), not ncurses'.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TtyNoColorVideo(pub u16);

impl TtyNoColorVideo {
    pub const NONE: Self = Self(0);
    pub const STANDOUT: Self = Self(1 << 0);
    pub const UNDERLINE: Self = Self(1 << 1);
    pub const REVERSE: Self = Self(1 << 2);
    pub const ITALIC: Self = Self(1 << 3);
    pub const DIM: Self = Self(1 << 4);
    pub const BOLD: Self = Self(1 << 5);
    pub const STRIKE_THROUGH: Self = Self(1 << 6);
    pub const PROTECT: Self = Self(1 << 7);

    pub const fn contains(self, bit: Self) -> bool {
        self.0 & bit.0 != 0
    }
}

/// One renderable attribute, as GNU's `TTY_CAP_*` flags name them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TtyCapability {
    /// `so` — GNU pairs inverse video with the standout capability.
    Inverse,
    /// `us`
    Underline,
    /// `Smulx` — the parameterized styled underline (wave, dotted, …).
    UnderlineStyled,
    /// `md`
    Bold,
    /// `mh`
    Dim,
    /// `ZH`
    Italic,
    /// `smxx`
    StrikeThrough,
}

impl TtyCapability {
    /// The `ncv` bit that disables this attribute on a color terminal, per
    /// GNU's `tty_capable_p` pairings.
    const fn no_color_video_bit(self) -> TtyNoColorVideo {
        match self {
            Self::Inverse => TtyNoColorVideo::REVERSE,
            // GNU tests NC_UNDERLINE for both the plain and the styled form.
            Self::Underline | Self::UnderlineStyled => TtyNoColorVideo::UNDERLINE,
            Self::Bold => TtyNoColorVideo::BOLD,
            Self::Dim => TtyNoColorVideo::DIM,
            Self::Italic => TtyNoColorVideo::ITALIC,
            Self::StrikeThrough => TtyNoColorVideo::STRIKE_THROUGH,
        }
    }
}

/// GNU `TF_set_underline_style` (`Smulx`), already expanded.
///
/// GNU expands at emit time — `tparam (tty->TF_set_underline_style, NULL, 0,
/// face->underline, 0, 0, 0)` (src/term.c:2083), and in a terminfo build
/// `tparam` IS ncurses' `tparm` (src/terminfo.c:43-55).  The parameter is an
/// `enum face_underline_type` and its domain is CLOSED: `turn_on_face` reaches
/// this call only when `face->underline != FACE_UNDERLINE_SINGLE`
/// (src/term.c:2076-2085), so exactly four values can arrive.  Expanding all
/// four when the terminal is resolved is therefore the same answer GNU
/// computes lazily, and it keeps the terminfo expander in the one crate that
/// links ncurses.
///
/// There is no field-wise constructor: [`TtyStyledUnderline::expand_all`] is
/// the only way to build one, so a half-filled set cannot exist.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TtyStyledUnderline {
    double_line: Vec<u8>,
    wave: Vec<u8>,
    dots: Vec<u8>,
    dashes: Vec<u8>,
}

impl TtyStyledUnderline {
    /// Expand `Smulx` for every style that can reach it, through `expand` —
    /// which is GNU's `tparam` at the call site that has one.  `None` when any
    /// expansion fails, because a terminal that can spell three of the four is
    /// not a terminal GNU would emit the fourth on.
    pub fn expand_all(mut expand: impl FnMut(u8) -> Option<Vec<u8>>) -> Option<Self> {
        Some(Self {
            double_line: expand(UnderlineStyle::Double.gnu_code())?,
            wave: expand(UnderlineStyle::Wave.gnu_code())?,
            dots: expand(UnderlineStyle::Dotted.gnu_code())?,
            dashes: expand(UnderlineStyle::Dashed.gnu_code())?,
        })
    }

    /// The sequence for `style`, or `None` for the two styles that never reach
    /// `Smulx`: `None` emits nothing and `Line` takes the `smul` arm above.
    pub fn sequence(&self, style: UnderlineStyle) -> Option<&[u8]> {
        match style {
            UnderlineStyle::None | UnderlineStyle::Line => None,
            UnderlineStyle::Double => Some(&self.double_line),
            UnderlineStyle::Wave => Some(&self.wave),
            UnderlineStyle::Dotted => Some(&self.dots),
            UnderlineStyle::Dashed => Some(&self.dashes),
        }
    }
}

/// How a `:slant italic` face is rendered on this terminal, and with which
/// bytes.
///
/// GNU `turn_on_face` (src/term.c:2063-2072): the whole arm is gated on
/// `MAY_USE_WITH_COLORS_P (tty, NC_ITALIC)`, and INSIDE it the choice is the
/// pointer — `sitm` when the terminal has it, otherwise `dim`, "Italics mode is
/// unavailable on many terminals.  In that case, map slant to dimmed text; we
/// want italic text to appear different and dimming is not otherwise used."
/// The fallback is emitted with `OUTPUT1`, not `OUTPUT1_IF`, and no second
/// `MAY_USE_WITH_COLORS_P` — so a terminal whose `ncv` forbids DIM on a colour
/// frame still gets the dim fallback for an italic face.  78 of the entries
/// ncurses ships are in exactly that state (ledger 186).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TtyItalicRendition<'a> {
    Italic(&'a [u8]),
    Dim(&'a [u8]),
    None,
}

/// The capabilities of one terminal.
///
/// Fields mirror the terminfo capabilities GNU reads in `init_tty`: `so`, `us`,
/// `Smulx`, `md`, `mh`, `ZH`, `smxx`, `Co` and `NC`.  Each string capability is
/// carried as its own bytes, terminfo padding removed, because that is what
/// GNU emits and because presence is not separable from spelling.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TtyAttributeCapabilities {
    /// `so` — GNU `TS_standout_mode`.
    pub standout_sequence: Option<Vec<u8>>,
    /// `us` — GNU `TS_enter_underline_mode`.
    pub underline_sequence: Option<Vec<u8>>,
    /// `md` — GNU `TS_enter_bold_mode`.
    pub bold_sequence: Option<Vec<u8>>,
    /// `mh` — GNU `TS_enter_dim_mode`.
    pub dim_sequence: Option<Vec<u8>>,
    /// `ZH` (`sitm`) — GNU `TS_enter_italic_mode`.
    pub italic_sequence: Option<Vec<u8>>,
    /// `smxx` — GNU `TS_enter_strike_through_mode`.
    pub strike_through_sequence: Option<Vec<u8>>,
    /// `Smulx` (or GNU's `Su` fallback literal) — GNU
    /// `TF_set_underline_style`, expanded.
    pub styled_underline: Option<TtyStyledUnderline>,
    /// `Co` — GNU `TN_max_colors`, the color-cell count.
    pub color_cells: i64,
    /// `NC` — GNU `TN_no_color_video`.
    pub no_color_video: TtyNoColorVideo,
}

impl TtyAttributeCapabilities {
    /// Every attribute available, no `ncv` restrictions, 24-bit color.
    ///
    /// This is the assumption neomacs shipped with before capabilities existed,
    /// so it stays the default for a terminal whose terminfo entry cannot be
    /// read: a missing entry should not silently strip highlighting.  The
    /// spellings are the xterm-family ones, and the styled underline is the
    /// literal GNU installs itself when a terminal claims `Su` without `Smulx`
    /// (src/term.c:4703).
    pub fn full() -> Self {
        Self {
            standout_sequence: Some(b"\x1b[7m".to_vec()),
            underline_sequence: Some(b"\x1b[4m".to_vec()),
            bold_sequence: Some(b"\x1b[1m".to_vec()),
            dim_sequence: Some(b"\x1b[2m".to_vec()),
            italic_sequence: Some(b"\x1b[3m".to_vec()),
            strike_through_sequence: Some(b"\x1b[9m".to_vec()),
            styled_underline: TtyStyledUnderline::expand_all(|style| {
                Some(format!("\x1b[4:{style}m").into_bytes())
            }),
            color_cells: 16_777_216,
            no_color_video: TtyNoColorVideo::NONE,
        }
    }

    /// A terminal that can render no attributes at all (a `dumb`-style entry).
    pub fn none() -> Self {
        Self {
            standout_sequence: None,
            underline_sequence: None,
            bold_sequence: None,
            dim_sequence: None,
            italic_sequence: None,
            strike_through_sequence: None,
            styled_underline: None,
            color_cells: 0,
            no_color_video: TtyNoColorVideo::NONE,
        }
    }

    /// GNU's presence question — `if (tty->TS_enter_bold_mode)` and its six
    /// neighbours.  Exhaustive on purpose: a capability added to
    /// [`TtyCapability`] without a field to answer from is a compile error.
    fn has_capability_string(&self, capability: TtyCapability) -> bool {
        match capability {
            TtyCapability::Inverse => self.standout_sequence.is_some(),
            TtyCapability::Underline => self.underline_sequence.is_some(),
            TtyCapability::UnderlineStyled => self.styled_underline.is_some(),
            TtyCapability::Bold => self.bold_sequence.is_some(),
            TtyCapability::Dim => self.dim_sequence.is_some(),
            TtyCapability::Italic => self.italic_sequence.is_some(),
            TtyCapability::StrikeThrough => self.strike_through_sequence.is_some(),
        }
    }

    /// GNU `tty_capable_p`: the capability's terminfo string must exist, and on a
    /// terminal that has colors its `ncv` bit must be clear
    /// (`MAY_USE_WITH_COLORS_P`). A monochrome terminal ignores `ncv` entirely.
    pub fn supports(&self, capability: TtyCapability) -> bool {
        self.has_capability_string(capability)
            && self.may_use_with_colors(capability.no_color_video_bit())
    }

    /// GNU `MAY_USE_WITH_COLORS_P` (term.c).
    fn may_use_with_colors(&self, bit: TtyNoColorVideo) -> bool {
        !self.supports_color() || !self.no_color_video.contains(bit)
    }

    /// Whether the terminal has colors at all — GNU `TN_max_colors > 0`.
    pub fn supports_color(&self) -> bool {
        self.color_cells > 0
    }

    // `turn_on_face` names each field literally rather than through a lookup,
    // and so does this: one accessor per GNU field, each carrying that field's
    // own `MAY_USE_WITH_COLORS_P` term, so an emission site cannot pick up the
    // bytes of a capability whose guard it did not check.

    /// GNU `TS_standout_mode` under `MAY_USE_WITH_COLORS_P (tty, NC_REVERSE)`.
    pub fn standout(&self) -> Option<&[u8]> {
        self.supports(TtyCapability::Inverse)
            .then(|| self.standout_sequence.as_deref())
            .flatten()
    }

    /// GNU `TS_enter_underline_mode` under `NC_UNDERLINE`.
    pub fn underline(&self) -> Option<&[u8]> {
        self.supports(TtyCapability::Underline)
            .then(|| self.underline_sequence.as_deref())
            .flatten()
    }

    /// GNU `TS_enter_bold_mode` under `NC_BOLD`.
    pub fn bold(&self) -> Option<&[u8]> {
        self.supports(TtyCapability::Bold)
            .then(|| self.bold_sequence.as_deref())
            .flatten()
    }

    /// GNU `TS_enter_strike_through_mode` under `NC_STRIKE_THROUGH`.
    pub fn strike_through(&self) -> Option<&[u8]> {
        self.supports(TtyCapability::StrikeThrough)
            .then(|| self.strike_through_sequence.as_deref())
            .flatten()
    }

    /// GNU `turn_on_face`'s slant decision, resolved once.  See
    /// [`TtyItalicRendition`] for why the dim fallback carries no `ncv` term.
    pub fn italic_rendition(&self) -> TtyItalicRendition<'_> {
        if !self.may_use_with_colors(TtyNoColorVideo::ITALIC) {
            return TtyItalicRendition::None;
        }
        match (
            self.italic_sequence.as_deref(),
            self.dim_sequence.as_deref(),
        ) {
            (Some(italic), _) => TtyItalicRendition::Italic(italic),
            (None, Some(dim)) => TtyItalicRendition::Dim(dim),
            (None, None) => TtyItalicRendition::None,
        }
    }

    /// The styled-underline sequence for `style`, or `None` when GNU takes the
    /// plain `smul` arm instead: no `Smulx`, a `Line` style, or an `ncv` that
    /// forbids underline on this colour terminal.
    pub fn styled_underline_sequence(&self, style: UnderlineStyle) -> Option<&[u8]> {
        self.supports(TtyCapability::UnderlineStyled)
            .then(|| {
                self.styled_underline
                    .as_ref()
                    .and_then(|styled| styled.sequence(style))
            })
            .flatten()
    }
}

impl Default for TtyAttributeCapabilities {
    fn default() -> Self {
        Self::full()
    }
}
