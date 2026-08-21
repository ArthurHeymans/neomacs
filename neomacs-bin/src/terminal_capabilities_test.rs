//! Tests for terminfo → [`TtyAttributeCapabilities`] resolution.

use super::*;
use neomacs_display_protocol::tty_capabilities::{TtyCapability, TtyItalicRendition};
use std::collections::HashMap;

/// A capability database standing in for terminfo, so the resolution can be
/// tested against known entries without a real terminal.
struct FakeCapabilityDatabase {
    strings: HashMap<&'static str, &'static str>,
    numbers: HashMap<&'static str, i32>,
    flags: std::collections::HashSet<&'static str>,
}

impl FakeCapabilityDatabase {
    /// `screen-256color`: standout, underline, bold and dim, but NO `sitm`
    /// (italics) and no `smxx` (strike-through) — the entry that made GNU render
    /// `:slant italic` as dim while neomacs emitted an italic escape.
    fn screen_256color() -> Self {
        Self {
            strings: HashMap::from([
                ("so", "\x1b[3m"),
                ("us", "\x1b[4m"),
                ("md", "\x1b[1m"),
                ("mh", "\x1b[2m"),
            ]),
            numbers: HashMap::from([("Co", 256), ("NC", -1)]),
            flags: std::collections::HashSet::new(),
        }
    }

    /// An empty entry to build capability shapes on.
    fn bare() -> Self {
        Self {
            strings: HashMap::new(),
            numbers: HashMap::new(),
            flags: std::collections::HashSet::new(),
        }
    }

    /// The ANSI update capabilities of an xterm-shaped entry, in the termcap
    /// spellings tgetstr returns (terminfo %p markers already translated).
    fn xterm_like() -> Self {
        Self::bare()
            .with_string("cm", "\x1b[%i%d;%dH")
            .with_string("cs", "\x1b[%i%d;%dr")
            .with_string("SF", "\x1b[%dS")
            .with_string("SR", "\x1b[%dT")
            .with_string("sf", "\n")
            .with_string("sr", "\x1bM")
            .with_string("IC", "\x1b[%d@")
            .with_string("DC", "\x1b[%dP")
            .with_string("ce", "\x1b[K")
    }

    /// vt220 shape: DECSTBM + cursor addressing but NO indn/rin — CSI S/T
    /// is not implemented by this terminal class.
    fn vt220_like() -> Self {
        Self::bare()
            .with_string("cm", "\x1b[%i%d;%dH")
            .with_string("cs", "\x1b[%i%d;%dr")
            .with_string("sf", "\n")
            .with_string("sr", "\x1bM")
            .with_string("ce", "\x1b[K$<3>")
    }

    /// tvi955 shape: insert/delete strings EXIST but are not ANSI.
    fn tvi955_like() -> Self {
        Self::bare()
            .with_string("cm", "\x1b[%i%d;%dH")
            .with_string("IC", "\x1bQ")
            .with_string("DC", "\x1bW")
            .with_string("ce", "\x1bt")
    }

    fn with_flag(mut self, cap: &'static str) -> Self {
        self.flags.insert(cap);
        self
    }

    fn without_string(mut self, cap: &'static str) -> Self {
        self.strings.remove(cap);
        self
    }

    fn with_string(mut self, cap: &'static str, value: &'static str) -> Self {
        self.strings.insert(cap, value);
        self
    }

    fn with_number(mut self, cap: &'static str, value: i32) -> Self {
        self.numbers.insert(cap, value);
        self
    }
}

impl TerminalCapabilityDatabase for FakeCapabilityDatabase {
    /// Both namespaces are one map here, keyed by the capability's own name.
    /// A fake will answer any key, which is exactly why it cannot attest that
    /// the REAL database can find `Smulx` and `smxx`; see
    /// `styled_underline_and_strike_through_come_from_the_terminfo_database`.
    fn get_string(&mut self, cap: StringCapability) -> Option<Vec<u8>> {
        let name = match cap {
            StringCapability::Termcap(name) | StringCapability::Terminfo(name) => name,
        };
        self.strings
            .get(name)
            .map(|value| value.as_bytes().to_vec())
    }

    fn get_termcap_number(&mut self, cap: &str) -> Option<i32> {
        self.numbers.get(cap).copied()
    }

    fn get_termcap_flag(&mut self, cap: &str) -> bool {
        self.flags.contains(cap)
    }
}

#[test]
fn screen_terminfo_reports_no_italics_but_keeps_bold_and_underline() {
    let mut database = FakeCapabilityDatabase::screen_256color();
    let caps = resolve_tty_attribute_capabilities(&mut database);

    assert!(!caps.italic, "screen has no sitm");
    assert!(caps.dim, "screen has mh, so italics fall back to dim");
    assert_eq!(caps.italic_rendition(), TtyItalicRendition::Dim);
    assert!(caps.bold);
    assert!(caps.underline);
    assert_eq!(
        caps.standout_sequence.as_deref(),
        Some(b"\x1b[3m".as_slice())
    );
    assert!(!caps.strike_through, "screen has no smxx");
    assert!(!caps.underline_styled, "screen has no Smulx");
    assert_eq!(caps.color_cells, 256);
    // GNU: `if (TN_no_color_video == -1) TN_no_color_video = 0'.
    assert_eq!(caps.no_color_video, TtyNoColorVideo::NONE);
}

#[test]
fn complete_standout_sequence_is_preserved() {
    let mut database = FakeCapabilityDatabase::bare()
        .with_string("so", "\x1b[0;1;3m$<2>")
        .with_number("Co", 256);

    let caps = resolve_tty_attribute_capabilities(&mut database);

    assert_eq!(
        caps.standout_sequence.as_deref(),
        Some(b"\x1b[0;1;3m".as_slice()),
    );
    assert!(caps.supports(TtyCapability::Inverse));
}

#[test]
fn capability_names_match_the_ones_gnu_reads_in_init_tty() {
    // GNU term.c: so / us / md / mh / ZH / smxx / Smulx, and tgetnum Co / NC.
    let mut database = FakeCapabilityDatabase::screen_256color()
        .with_string("ZH", "\x1b[3m")
        .with_string("smxx", "\x1b[9m")
        .with_string("Smulx", "\x1b[4:%p1%dm")
        .with_number("NC", 32);
    let caps = resolve_tty_attribute_capabilities(&mut database);

    assert!(caps.italic);
    assert_eq!(caps.italic_rendition(), TtyItalicRendition::Italic);
    assert!(caps.strike_through);
    assert!(caps.underline_styled);
    // ncv bit 1<<5 is GNU's NC_BOLD: bold cannot be combined with colors here.
    assert_eq!(caps.no_color_video, TtyNoColorVideo::BOLD);
    assert!(!caps.supports(TtyCapability::Bold));
    assert!(caps.supports(TtyCapability::Underline));
}

/// GNU has a SECOND source for styled underlines, immediately below the
/// `Smulx` lookup: `if (!tty->TF_set_underline_style && tgetflag ("Su"))
/// tty->TF_set_underline_style = "\x1b[4:%p1%dm";` (src/term.c:4700-4703) --
/// the kitty default its own comment calls "not recommended".  Because
/// `TF_set_underline_style` also gates `TF_set_underline_color` (:4705-4708),
/// the flag turns on both, which is why one boolean models it here.
///
/// Ledger 158 recorded this as LATENT: of the 3,697 entries `toe -a` lists on
/// this machine exactly one carries `Su` -- `xterm-kitty` -- and it ships
/// `Smulx` too, so the `!TF_set_underline_style` guard is false and the
/// fallback fires zero times.  That made it the SECOND capability in this area
/// whose absence from the shipped database proves nothing, the first being the
/// `tgetstr`-vs-`tigetstr` namespace trap 158 fixed for `Smulx` itself.  The
/// prerequisite that trap raises -- can `tgetflag` even SEE an extended
/// terminfo boolean? -- is settled with `tic`, not with a search for a real
/// terminal (`tmp/pw175/pw175.src`, `tmp/pw175/su_probe.c`):
///
/// ```text
/// pw175-su-only        tgetflag(Su)=1  tigetstr(Smulx)=null  => GNU: yes (Su fallback)
/// pw175-su-and-smulx   tgetflag(Su)=1  tigetstr(Smulx)=FOUND => GNU: yes (Smulx)
/// pw175-neither        tgetflag(Su)=0  tigetstr(Smulx)=null  => GNU: no
/// ```
///
/// So `Su` is NOT a second dead lookup: unlike `tgetstr ("Smulx")`, which
/// answers null on every entry that has it, `tgetflag ("Su")` resolves the
/// extended boolean and GNU's fallback works.  Ledger 175.
#[test]
fn the_su_flag_is_a_styled_underline_where_smulx_is_absent_like_gnu() {
    let mut su_only = FakeCapabilityDatabase::screen_256color().with_flag("Su");
    assert!(
        resolve_tty_attribute_capabilities(&mut su_only).underline_styled,
        "Su without Smulx is GNU's kitty-sequence fallback (term.c:4700-4703)"
    );
    assert!(
        resolve_tty_attribute_capabilities(&mut su_only).supports(TtyCapability::UnderlineStyled)
    );

    let mut both = FakeCapabilityDatabase::screen_256color()
        .with_string("Smulx", "\x1b[4:%p1%dm")
        .with_flag("Su");
    assert!(
        resolve_tty_attribute_capabilities(&mut both).underline_styled,
        "Smulx alone already answers; the flag is not consulted"
    );

    let mut neither = FakeCapabilityDatabase::screen_256color();
    assert!(
        !resolve_tty_attribute_capabilities(&mut neither).underline_styled,
        "neither source: no styled underline, and so no underline colour"
    );
}

#[test]
fn a_terminal_with_no_attribute_strings_supports_nothing() {
    let mut database = FakeCapabilityDatabase::bare().with_number("Co", 0);
    let caps = resolve_tty_attribute_capabilities(&mut database);

    assert_eq!(caps.italic_rendition(), TtyItalicRendition::None);
    for capability in [
        TtyCapability::Bold,
        TtyCapability::Dim,
        TtyCapability::Italic,
        TtyCapability::Underline,
        TtyCapability::UnderlineStyled,
        TtyCapability::Inverse,
        TtyCapability::StrikeThrough,
    ] {
        assert!(!caps.supports(capability), "{capability:?} must be absent");
    }
}

#[test]
fn an_absent_color_count_is_monochrome_like_gnu() {
    // GNU only sets up colors when `op' (TS_orig_pair) exists; a terminfo entry
    // without `Co' has no colors, and then `ncv' never applies.
    let mut database = FakeCapabilityDatabase::bare()
        .with_string("md", "\x1b[1m")
        .with_number("Co", -1)
        .with_number("NC", 32);
    let caps = resolve_tty_attribute_capabilities(&mut database);

    assert_eq!(caps.color_cells, 0);
    assert!(
        caps.supports(TtyCapability::Bold),
        "a monochrome terminal ignores ncv"
    );
}

// ---------------------------------------------------------------------------
// Update-planner capabilities (TermCaps): the gate must attest the exact
// bytes the encoder emits, not mere capability presence.
// ---------------------------------------------------------------------------

use neomacs_display_runtime::backend::tty::rif::{BlankTailMethod, RegionScrollMethod};

#[test]
fn xterm_shaped_entry_resolves_every_planner_capability() {
    let mut database = FakeCapabilityDatabase::xterm_like().with_flag("ut");
    let caps = resolve_term_caps(&mut database);
    assert_eq!(caps.scroll_region, Some(RegionScrollMethod::SuSd));
    assert!(caps.insert_delete_char);
    assert_eq!(
        caps.blank_tail,
        BlankTailMethod::EraseToEol {
            back_color_erase: true,
        }
    );
    assert!(caps.synchronized_output);
}

#[test]
fn vt220_shaped_entry_scrolls_by_index_never_su_sd() {
    // The SU/SD-on-vt220 trap: cs attests DECSTBM, but CSI S/T is VT420+.
    // The entry's own sf/sr (LF and ESC M) attest the index form instead.
    let mut database = FakeCapabilityDatabase::vt220_like();
    let caps = resolve_term_caps(&mut database);
    assert_eq!(caps.scroll_region, Some(RegionScrollMethod::Index));
}

#[test]
fn decstbm_without_reverse_index_refuses_region_scrolls() {
    let mut database = FakeCapabilityDatabase::vt220_like().without_string("sr");
    let caps = resolve_term_caps(&mut database);
    assert_eq!(caps.scroll_region, None);
}

#[test]
fn missing_cursor_addressing_refuses_region_scrolls() {
    let mut database = FakeCapabilityDatabase::xterm_like().without_string("cm");
    let caps = resolve_term_caps(&mut database);
    assert_eq!(caps.scroll_region, None);
}

#[test]
fn non_ansi_insert_delete_strings_refuse_ich_dch() {
    // tvi955 HAS insert/delete-char capabilities; they are just not the
    // ANSI bytes the encoder hardcodes. Presence-gating would corrupt it.
    let mut database = FakeCapabilityDatabase::tvi955_like();
    let caps = resolve_term_caps(&mut database);
    assert!(!caps.insert_delete_char);
    assert_eq!(
        caps.blank_tail,
        BlankTailMethod::WriteSpaces,
        "tvi955 ce is not ESC[K"
    );
}

#[test]
fn back_color_erase_comes_from_the_ut_flag_alone() {
    let mut with_ut = FakeCapabilityDatabase::xterm_like().with_flag("ut");
    let mut without_ut = FakeCapabilityDatabase::xterm_like();
    assert_eq!(
        resolve_term_caps(&mut with_ut).blank_tail,
        BlankTailMethod::EraseToEol {
            back_color_erase: true,
        }
    );
    assert_eq!(
        resolve_term_caps(&mut without_ut).blank_tail,
        BlankTailMethod::EraseToEol {
            back_color_erase: false,
        }
    );
}

#[test]
fn insert_null_glitch_requires_written_blank_tails() {
    let mut database = FakeCapabilityDatabase::xterm_like().with_flag("in");

    assert_eq!(
        resolve_term_caps(&mut database).blank_tail,
        BlankTailMethod::WriteSpaces,
        "GNU's must_write_spaces comes directly from termcap `in`"
    );
}

#[test]
fn padding_and_parameter_markers_do_not_defeat_recognition() {
    // vt100-style entries carry delay padding ($<5>) on csr and terminfo
    // %p markers survive in some spellings; both canonicalize away.
    let mut database = FakeCapabilityDatabase::xterm_like()
        .with_string("cs", "\x1b[%i%p1%d;%p2%dr$<5>")
        .with_string("cm", "\x1b[%i%p1%d;%p2%dH");
    let caps = resolve_term_caps(&mut database);
    assert_eq!(caps.scroll_region, Some(RegionScrollMethod::SuSd));
}

/// The two long capability names GNU reads, read from the REAL database.
///
/// Every other test in this file feeds a `FakeCapabilityDatabase` keyed by
/// plain strings, which answers `Smulx` and `smxx` because a `HashMap` will
/// answer any key.  The real database will not: `tgetstr` resolves two-letter
/// TERMCAP names and nothing else, so it answers NULL for both of these on
/// every entry in existence.  GNU reads them with `tigetstr`
/// (src/term.c:4587 and :4694) for exactly that reason.
///
/// These two entries are ncurses' own and their contents are stable:
/// `infocmp -x tmux-256color` carries `Smulx` and `smxx`, and
/// `infocmp -x xterm-256color` carries `smxx` but not `Smulx`.  An entry that
/// cannot be opened at all (no terminfo database on the machine) makes the
/// assertion vacuous rather than red, since that is the one condition under
/// which neomacs is right to answer "absent".
#[test]
fn styled_underline_and_strike_through_come_from_the_terminfo_database() {
    let Some(tmux) = tty_attribute_capabilities_for_term("tmux-256color") else {
        return;
    };
    assert!(
        tmux.underline_styled,
        "tmux-256color has Smulx; tgetstr cannot see it and tigetstr can"
    );
    assert!(
        tmux.strike_through,
        "tmux-256color has smxx; tgetstr cannot see it and tigetstr can"
    );

    let Some(xterm) = tty_attribute_capabilities_for_term("xterm-256color") else {
        return;
    };
    assert!(
        xterm.strike_through,
        "xterm-256color has smxx even though it has no Smulx"
    );
    assert!(
        !xterm.underline_styled,
        "xterm-256color has no Smulx, so a styled underline must fall back"
    );
}

/// The two-letter names must keep going to termcap.
///
/// `tigetstr` is the mirror image of `tgetstr`: it resolves TERMINFO names and
/// answers NULL for `us`, `so` and `ZH`, whose terminfo spellings are `smul`,
/// `smso` and `sitm`.  Moving every lookup to terminfo would break the other
/// direction just as silently.
#[test]
fn two_letter_capability_names_still_come_from_termcap() {
    let Some(xterm) = tty_attribute_capabilities_for_term("xterm-256color") else {
        return;
    };
    assert!(xterm.underline, "xterm-256color has us");
    assert!(xterm.bold, "xterm-256color has md");
    assert!(xterm.italic, "xterm-256color has ZH");
    assert!(
        xterm.standout_sequence.is_some(),
        "xterm-256color has so, and the writer needs its bytes"
    );
    assert_eq!(xterm.color_cells, 256, "xterm-256color has Co#256");
}
