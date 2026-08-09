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
    fn get_string(&mut self, cap: &str) -> Option<Vec<u8>> {
        self.strings.get(cap).map(|value| value.as_bytes().to_vec())
    }

    fn get_number(&mut self, cap: &str) -> Option<i32> {
        self.numbers.get(cap).copied()
    }

    fn get_flag(&mut self, cap: &str) -> bool {
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

use neomacs_display_runtime::backend::tty::rif::RegionScrollMethod;

#[test]
fn xterm_shaped_entry_resolves_every_planner_capability() {
    let mut database = FakeCapabilityDatabase::xterm_like().with_flag("ut");
    let caps = resolve_term_caps(&mut database);
    assert_eq!(caps.scroll_region, Some(RegionScrollMethod::SuSd));
    assert!(caps.back_color_erase);
    assert!(caps.insert_delete_char);
    assert!(caps.erase_to_eol);
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
    assert!(!caps.erase_to_eol, "tvi955 ce is not ESC[K");
}

#[test]
fn back_color_erase_comes_from_the_ut_flag_alone() {
    let mut with_ut = FakeCapabilityDatabase::xterm_like().with_flag("ut");
    let mut without_ut = FakeCapabilityDatabase::xterm_like();
    assert!(resolve_term_caps(&mut with_ut).back_color_erase);
    assert!(!resolve_term_caps(&mut without_ut).back_color_erase);
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
