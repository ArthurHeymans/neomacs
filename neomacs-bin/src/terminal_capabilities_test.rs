//! Tests for terminfo → [`TtyAttributeCapabilities`] resolution.

use super::*;
use neomacs_display_protocol::tty_capabilities::{TtyCapability, TtyItalicRendition};
use std::collections::HashMap;

/// A capability database standing in for terminfo, so the resolution can be
/// tested against known entries without a real terminal.
struct FakeCapabilityDatabase {
    strings: HashMap<&'static str, &'static str>,
    numbers: HashMap<&'static str, i32>,
}

impl FakeCapabilityDatabase {
    /// `screen-256color`: standout, underline, bold and dim, but NO `sitm`
    /// (italics) and no `smxx` (strike-through) — the entry that made GNU render
    /// `:slant italic` as dim while neomacs emitted an italic escape.
    fn screen_256color() -> Self {
        Self {
            strings: HashMap::from([
                ("so", "\x1b[7m"),
                ("us", "\x1b[4m"),
                ("md", "\x1b[1m"),
                ("mh", "\x1b[2m"),
            ]),
            numbers: HashMap::from([("Co", 256), ("NC", -1)]),
        }
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
    assert!(caps.inverse);
    assert!(!caps.strike_through, "screen has no smxx");
    assert!(!caps.underline_styled, "screen has no Smulx");
    assert_eq!(caps.color_cells, 256);
    // GNU: `if (TN_no_color_video == -1) TN_no_color_video = 0'.
    assert_eq!(caps.no_color_video, TtyNoColorVideo::NONE);
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
    let mut database = FakeCapabilityDatabase {
        strings: HashMap::new(),
        numbers: HashMap::from([("Co", 0)]),
    };
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
    let mut database = FakeCapabilityDatabase {
        strings: HashMap::from([("md", "\x1b[1m")]),
        numbers: HashMap::from([("Co", -1), ("NC", 32)]),
    };
    let caps = resolve_tty_attribute_capabilities(&mut database);

    assert_eq!(caps.color_cells, 0);
    assert!(
        caps.supports(TtyCapability::Bold),
        "a monochrome terminal ignores ncv"
    );
}
