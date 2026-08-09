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

/// How a `:slant italic` face is rendered on this terminal.
///
/// GNU `turn_on_face`: emit `sitm` when the terminal has it, otherwise fall back
/// to `dim` — "Italics not supported, use dim instead ... we want italic text to
/// appear different and dimming is not otherwise used" — and emit nothing when
/// the terminal has neither.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TtyItalicRendition {
    Italic,
    Dim,
    None,
}

/// The capabilities of one terminal.
///
/// Fields mirror the terminfo capabilities GNU reads in `init_tty`: `so`, `us`,
/// `Smulx`, `md`, `mh`, `ZH`, `smxx`, `Co` and `NC`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TtyAttributeCapabilities {
    /// Complete `so` (standout) control sequence after terminfo padding is
    /// removed. GNU emits this capability itself; preserving its bytes handles
    /// combined and non-SGR renditions without guessing their meaning.
    pub standout_sequence: Option<Vec<u8>>,
    /// `us`.
    pub underline: bool,
    /// `Smulx`.
    pub underline_styled: bool,
    /// `md`.
    pub bold: bool,
    /// `mh`.
    pub dim: bool,
    /// `ZH` (`sitm`).
    pub italic: bool,
    /// `smxx`.
    pub strike_through: bool,
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
    /// read: a missing entry should not silently strip highlighting.
    pub fn full() -> Self {
        Self {
            standout_sequence: Some(b"\x1b[7m".to_vec()),
            underline: true,
            underline_styled: true,
            bold: true,
            dim: true,
            italic: true,
            strike_through: true,
            color_cells: 16_777_216,
            no_color_video: TtyNoColorVideo::NONE,
        }
    }

    /// A terminal that can render no attributes at all (a `dumb`-style entry).
    pub fn none() -> Self {
        Self {
            standout_sequence: None,
            underline: false,
            underline_styled: false,
            bold: false,
            dim: false,
            italic: false,
            strike_through: false,
            color_cells: 0,
            no_color_video: TtyNoColorVideo::NONE,
        }
    }

    /// GNU `tty_capable_p`: the capability's terminfo string must exist, and on a
    /// terminal that has colors its `ncv` bit must be clear
    /// (`MAY_USE_WITH_COLORS_P`). A monochrome terminal ignores `ncv` entirely.
    pub fn supports(&self, capability: TtyCapability) -> bool {
        let present = match capability {
            TtyCapability::Inverse => self.standout_sequence.is_some(),
            TtyCapability::Underline => self.underline,
            TtyCapability::UnderlineStyled => self.underline_styled,
            TtyCapability::Bold => self.bold,
            TtyCapability::Dim => self.dim,
            TtyCapability::Italic => self.italic,
            TtyCapability::StrikeThrough => self.strike_through,
        };
        present && self.may_use_with_colors(capability.no_color_video_bit())
    }

    /// GNU `MAY_USE_WITH_COLORS_P` (term.c).
    fn may_use_with_colors(&self, bit: TtyNoColorVideo) -> bool {
        !self.supports_color() || !self.no_color_video.contains(bit)
    }

    /// Whether the terminal has colors at all — GNU `TN_max_colors > 0`.
    pub fn supports_color(&self) -> bool {
        self.color_cells > 0
    }

    /// GNU `turn_on_face`'s slant decision, resolved once.
    pub fn italic_rendition(&self) -> TtyItalicRendition {
        if self.supports(TtyCapability::Italic) {
            TtyItalicRendition::Italic
        } else if self.supports(TtyCapability::Dim) {
            TtyItalicRendition::Dim
        } else {
            TtyItalicRendition::None
        }
    }
}

impl Default for TtyAttributeCapabilities {
    fn default() -> Self {
        Self::full()
    }
}
