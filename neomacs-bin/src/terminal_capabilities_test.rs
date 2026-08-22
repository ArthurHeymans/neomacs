//! Tests for terminfo → [`TtyAttributeCapabilities`] resolution.

use super::*;
use neomacs_display_protocol::face::UnderlineStyle;
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

    assert!(caps.italic_sequence.is_none(), "screen has no sitm");
    assert_eq!(
        caps.dim_sequence.as_deref(),
        Some(b"\x1b[2m".as_slice()),
        "screen has mh, so italics fall back to dim"
    );
    assert_eq!(
        caps.italic_rendition(),
        TtyItalicRendition::Dim(b"\x1b[2m")
    );
    assert_eq!(caps.bold(), Some(b"\x1b[1m".as_slice()));
    assert_eq!(caps.underline(), Some(b"\x1b[4m".as_slice()));
    assert_eq!(
        caps.standout_sequence.as_deref(),
        Some(b"\x1b[3m".as_slice())
    );
    assert!(
        caps.strike_through_sequence.is_none(),
        "screen has no smxx"
    );
    assert!(caps.styled_underline.is_none(), "screen has no Smulx");
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

    assert_eq!(
        caps.italic_sequence.as_deref(),
        Some(b"\x1b[3m".as_slice())
    );
    assert_eq!(
        caps.italic_rendition(),
        TtyItalicRendition::Italic(b"\x1b[3m")
    );
    assert_eq!(
        caps.strike_through_sequence.as_deref(),
        Some(b"\x1b[9m".as_slice())
    );
    assert!(caps.styled_underline.is_some());
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
        resolve_tty_attribute_capabilities(&mut su_only)
            .styled_underline
            .is_some(),
        "Su without Smulx is GNU's kitty-sequence fallback (term.c:4700-4703)"
    );
    assert!(
        resolve_tty_attribute_capabilities(&mut su_only).supports(TtyCapability::UnderlineStyled)
    );

    let mut both = FakeCapabilityDatabase::screen_256color()
        .with_string("Smulx", "\x1b[4:%p1%dm")
        .with_flag("Su");
    assert!(
        resolve_tty_attribute_capabilities(&mut both)
            .styled_underline
            .is_some(),
        "Smulx alone already answers; the flag is not consulted"
    );

    let mut neither = FakeCapabilityDatabase::screen_256color();
    assert!(
        resolve_tty_attribute_capabilities(&mut neither)
            .styled_underline
            .is_none(),
        "neither source: no styled underline, and so no underline colour"
    );
}

/// The capability record carries the entry's BYTES, not a flag, because that
/// is what GNU emits: `OUTPUT1_IF (tty, tty->TS_enter_bold_mode)`
/// (src/term.c:2061) is one field answering both "does this terminal have
/// bold?" and "what is bold spelled as here?".
///
/// Terminfo padding is dropped and nothing else is: `OUTPUT1` is `tputs`,
/// which turns `$<..>` into a DELAY and does no parameter expansion at all.
/// That is a different rule from [`canonical_cap`], which also strips `%pN` so
/// the update planner can compare a terminfo spelling with its termcap
/// translation -- a normalization that would corrupt a string being EMITTED.
#[test]
fn every_rendition_capability_carries_the_entrys_own_bytes() {
    let mut database = FakeCapabilityDatabase::bare()
        .with_string("so", "\x1b[7;31m")
        .with_string("us", "\x1bG8$<10>")
        .with_string("md", "\x1b[1;43m")
        .with_string("mh", "\x1bGp")
        .with_string("ZH", "\x1b[3;44m")
        .with_string("smxx", "\x1bG@")
        .with_number("Co", 8);
    let caps = resolve_tty_attribute_capabilities(&mut database);

    assert_eq!(
        caps.standout_sequence.as_deref(),
        Some(b"\x1b[7;31m".as_slice())
    );
    assert_eq!(
        caps.underline_sequence.as_deref(),
        Some(b"\x1bG8".as_slice()),
        "padding is a delay, not bytes"
    );
    assert_eq!(
        caps.bold_sequence.as_deref(),
        Some(b"\x1b[1;43m".as_slice())
    );
    assert_eq!(caps.dim_sequence.as_deref(), Some(b"\x1bGp".as_slice()));
    assert_eq!(
        caps.italic_sequence.as_deref(),
        Some(b"\x1b[3;44m".as_slice())
    );
    assert_eq!(
        caps.strike_through_sequence.as_deref(),
        Some(b"\x1bG@".as_slice())
    );
}

/// GNU runs `Smulx` through `tparam` (src/term.c:2083), which in a terminfo
/// build IS ncurses' `tparm` (src/terminfo.c:43-55), so a terminal whose
/// `Smulx` is not the kitty spelling gets its own sequence.  This port emitted
/// a fixed `ESC [ 4 : N m`.
///
/// Ledger 158 recorded this as invisible and ledger 186 measured it: of the
/// 1,862 unique entries `toe -a` lists here, 25 carry `Smulx` and all 25 spell
/// it `\E[4:%p1%dm`.  So the divergence is only observable against an entry
/// built for the purpose -- `tic -x tmp/pw186/ti/pw186.src`, whose
/// `pw186-smulx-semicolon` answers `tparm ("\E[4;%p1%dm", 3)` = `\E[4;3m`
/// (`tmp/pw186/smulx_probe.c`).  The expansion below runs through the SAME
/// ncurses `tparm`, so what is pinned is the expander GNU uses and not a
/// re-implementation of terminfo's format language.
#[test]
fn the_styled_underline_is_smulx_expanded_by_ncurses_tparm() {
    let mut kitty =
        FakeCapabilityDatabase::screen_256color().with_string("Smulx", "\x1b[4:%p1%dm");
    let styled = resolve_tty_attribute_capabilities(&mut kitty)
        .styled_underline
        .expect("Smulx present");
    assert_eq!(
        styled.sequence(UnderlineStyle::Wave),
        Some(b"\x1b[4:3m".as_slice())
    );

    let mut semicolon =
        FakeCapabilityDatabase::screen_256color().with_string("Smulx", "\x1b[4;%p1%dm");
    let styled = resolve_tty_attribute_capabilities(&mut semicolon)
        .styled_underline
        .expect("Smulx present");
    for (style, expected) in [
        (UnderlineStyle::Double, b"\x1b[4;2m".as_slice()),
        (UnderlineStyle::Wave, b"\x1b[4;3m".as_slice()),
        (UnderlineStyle::Dotted, b"\x1b[4;4m".as_slice()),
        (UnderlineStyle::Dashed, b"\x1b[4;5m".as_slice()),
    ] {
        assert_eq!(styled.sequence(style), Some(expected), "{style:?}");
    }
    // The two styles that never reach `Smulx` in GNU have no expansion here.
    assert_eq!(styled.sequence(UnderlineStyle::Line), None);
    assert_eq!(styled.sequence(UnderlineStyle::None), None);

    // A private-mode spelling that shares no bytes at all with the rule this
    // port used to emit (`pw186-smulx-private`).
    let mut private =
        FakeCapabilityDatabase::screen_256color().with_string("Smulx", "\x1b[>4%p1%dw");
    let styled = resolve_tty_attribute_capabilities(&mut private)
        .styled_underline
        .expect("Smulx present");
    assert_eq!(
        styled.sequence(UnderlineStyle::Wave),
        Some(b"\x1b[>43w".as_slice())
    );

    // GNU's `Su` fallback installs its own literal and expands THAT
    // (src/term.c:4700-4703), so the kitty spelling comes back.
    let mut su_only = FakeCapabilityDatabase::screen_256color().with_flag("Su");
    let styled = resolve_tty_attribute_capabilities(&mut su_only)
        .styled_underline
        .expect("Su is the second source");
    assert_eq!(
        styled.sequence(UnderlineStyle::Dotted),
        Some(b"\x1b[4:4m".as_slice())
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
    let styled = tmux
        .styled_underline
        .as_ref()
        .expect("tmux-256color has Smulx; tgetstr cannot see it and tigetstr can");
    assert_eq!(
        styled.sequence(UnderlineStyle::Wave),
        Some(b"\x1b[4:3m".as_slice()),
        "and its own spelling, expanded through ncurses' tparm"
    );
    assert_eq!(
        tmux.strike_through_sequence.as_deref(),
        Some(b"\x1b[9m".as_slice()),
        "tmux-256color has smxx; tgetstr cannot see it and tigetstr can"
    );

    let Some(xterm) = tty_attribute_capabilities_for_term("xterm-256color") else {
        return;
    };
    assert_eq!(
        xterm.strike_through_sequence.as_deref(),
        Some(b"\x1b[9m".as_slice()),
        "xterm-256color has smxx even though it has no Smulx"
    );
    assert!(
        xterm.styled_underline.is_none(),
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
    // The bytes, not just the presence: this is the one test in the file that
    // reads a REAL terminfo entry, so it is where the entry's own spelling can
    // be pinned against something other than a fake table.
    assert_eq!(
        xterm.underline_sequence.as_deref(),
        Some(b"\x1b[4m".as_slice()),
        "xterm-256color has us"
    );
    assert_eq!(
        xterm.bold_sequence.as_deref(),
        Some(b"\x1b[1m".as_slice()),
        "xterm-256color has md"
    );
    assert_eq!(
        xterm.italic_sequence.as_deref(),
        Some(b"\x1b[3m".as_slice()),
        "xterm-256color has ZH"
    );
    assert_eq!(
        xterm.standout_sequence.as_deref(),
        Some(b"\x1b[7m".as_slice()),
        "xterm-256color has so, and the writer needs its bytes"
    );
    assert_eq!(xterm.color_cells, 256, "xterm-256color has Co#256");
}
