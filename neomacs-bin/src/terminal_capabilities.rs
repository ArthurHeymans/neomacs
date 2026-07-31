//! The terminfo/termcap access point for this terminal.
//!
//! GNU reads every terminal capability it needs in one place — `term.c:init_tty`
//! — and stores the answers on the terminal: function-key sequences for
//! `input-decode-map`, attribute sequences for `turn_on_face`, and the color
//! numbers for `tty_capable_p`. neomacs had only the input half (see
//! `super::termcap_input`) while output attributes were hardcoded in the
//! renderer, so `:slant italic` was emitted as an italic escape even on a
//! terminal whose terminfo has no `sitm` — where GNU emits its dim fallback.
//!
//! This module owns the database handle so both halves ask the same terminfo
//! entry the same way.

#[cfg(not(windows))]
use std::ffi::{CStr, CString};
#[cfg(not(windows))]
use std::os::raw::c_char;
#[cfg(not(windows))]
use std::os::raw::c_int;

use neomacs_display_protocol::tty_capabilities::{TtyAttributeCapabilities, TtyNoColorVideo};

#[cfg(not(windows))]
#[cfg_attr(target_os = "linux", link(name = "ncursesw"))]
#[cfg_attr(target_os = "macos", link(name = "ncurses"))]
unsafe extern "C" {
    fn tgetent(buffer: *mut c_char, term: *const c_char) -> c_int;
    fn tgetstr(capability: *const c_char, area: *mut *mut c_char) -> *mut c_char;
    fn tgetnum(capability: *const c_char) -> c_int;
    fn tgetflag(capability: *const c_char) -> c_int;
}

/// A source of terminal capabilities — terminfo in production, a table in tests.
pub(crate) trait TerminalCapabilityDatabase {
    /// A string capability (GNU `tgetstr`). `None` when the entry lacks it.
    fn get_string(&mut self, cap: &str) -> Option<Vec<u8>>;

    /// A numeric capability (GNU `tgetnum`). `None`, like GNU's `-1`, when the
    /// entry lacks it.
    fn get_number(&mut self, cap: &str) -> Option<i32>;

    /// A boolean capability (GNU `tgetflag`). Termcap two-letter names, e.g.
    /// `ut` for back-color-erase (terminfo `bce`).
    fn get_flag(&mut self, cap: &str) -> bool;
}

pub(crate) fn open_terminal_capability_database(
    term: &str,
) -> Option<Box<dyn TerminalCapabilityDatabase>> {
    open_platform_terminal_capability_database(term)
}

/// Resolve what this terminal can render, reading the same capability names GNU
/// reads in `init_tty`:
///
/// | capability | GNU field | meaning |
/// |---|---|---|
/// | `so` | `TS_standout_mode` | inverse video |
/// | `us` | `TS_enter_underline_mode` | underline |
/// | `Smulx` | `TF_set_underline_style` | styled underline |
/// | `md` | `TS_enter_bold_mode` | bold |
/// | `mh` | `TS_enter_dim_mode` | dim (and GNU's italic fallback) |
/// | `ZH` | `TS_enter_italic_mode` | italic (`sitm`) |
/// | `smxx` | `TS_enter_strike_through_mode` | strike-through |
/// | `Co` | `TN_max_colors` | color cells |
/// | `NC` | `TN_no_color_video` | attributes unusable with colors |
pub(crate) fn resolve_tty_attribute_capabilities(
    database: &mut dyn TerminalCapabilityDatabase,
) -> TtyAttributeCapabilities {
    let has = |database: &mut dyn TerminalCapabilityDatabase, cap: &str| {
        database
            .get_string(cap)
            .is_some_and(|value| !value.is_empty())
    };
    let color_cells = database
        .get_number("Co")
        .filter(|colors| *colors > 0)
        .unwrap_or(0);
    // GNU: `TN_no_color_video = tgetnum ("NC"); if (== -1) TN_no_color_video = 0'.
    let no_color_video = database
        .get_number("NC")
        .filter(|ncv| *ncv > 0)
        .map_or(TtyNoColorVideo::NONE, |ncv| TtyNoColorVideo(ncv as u16));

    TtyAttributeCapabilities {
        inverse: has(database, "so"),
        underline: has(database, "us"),
        underline_styled: has(database, "Smulx"),
        bold: has(database, "md"),
        dim: has(database, "mh"),
        italic: has(database, "ZH"),
        strike_through: has(database, "smxx"),
        color_cells: i64::from(color_cells),
        no_color_video,
    }
}

/// Resolve the capabilities of the terminal named by `TERM`, or `None` when the
/// entry cannot be read (GNU then falls back to a `dumb`-terminal default; the
/// caller keeps the previous full-capability assumption instead, so a missing
/// terminfo database does not silently strip highlighting).
pub(crate) fn tty_attribute_capabilities_for_term(term: &str) -> Option<TtyAttributeCapabilities> {
    let mut database = open_terminal_capability_database(term)?;
    Some(resolve_tty_attribute_capabilities(database.as_mut()))
}

/// Resolve the update-planner capabilities ([`TermCaps`]) the same way GNU
/// gates its terminal update: `term.c:4908` sets `scroll_region_ok` from the
/// presence of `cs` (set_scroll_region) with absolute cursor motion, and
/// `TF_bce` from the `ut` flag. Synchronized output (DECSET 2026) has no
/// terminfo name; it is spec-safe to emit everywhere (an unknown DEC private
/// mode is ignored), so it stays enabled unconditionally.
pub(crate) fn resolve_term_caps(
    database: &mut dyn TerminalCapabilityDatabase,
) -> neomacs_display_protocol::tty_rif::TermCaps {
    let has_string = |database: &mut dyn TerminalCapabilityDatabase, cap: &str| {
        database
            .get_string(cap)
            .is_some_and(|value| !value.is_empty())
    };
    neomacs_display_protocol::tty_rif::TermCaps {
        scroll_region: has_string(database, "cs") && has_string(database, "cm"),
        back_color_erase: database.get_flag("ut"),
        synchronized_output: true,
    }
}

/// [`resolve_term_caps`] for the terminal named by `TERM`; `None` when the
/// terminfo entry cannot be read (the caller then assumes a modern terminal,
/// mirroring the attribute-capability fallback above).
pub(crate) fn term_caps_for_term(
    term: &str,
) -> Option<neomacs_display_protocol::tty_rif::TermCaps> {
    let mut database = open_terminal_capability_database(term)?;
    Some(resolve_term_caps(database.as_mut()))
}

#[cfg(not(windows))]
struct UnixTermcapDatabase {
    _termcap_buffer: Vec<c_char>,
    _string_area: Vec<c_char>,
    string_area_ptr: *mut c_char,
}

#[cfg(not(windows))]
impl UnixTermcapDatabase {
    fn open(term: &str) -> Option<Self> {
        let term = CString::new(term).ok()?;
        let mut termcap_buffer = vec![0 as c_char; 16384];
        let ok = unsafe { tgetent(termcap_buffer.as_mut_ptr(), term.as_ptr()) };
        if ok <= 0 {
            return None;
        }
        let mut string_area = vec![0 as c_char; 32768];
        let string_area_ptr = string_area.as_mut_ptr();
        Some(Self {
            _termcap_buffer: termcap_buffer,
            _string_area: string_area,
            string_area_ptr,
        })
    }
}

#[cfg(not(windows))]
impl TerminalCapabilityDatabase for UnixTermcapDatabase {
    fn get_string(&mut self, cap: &str) -> Option<Vec<u8>> {
        let cap = CString::new(cap).ok()?;
        let raw = unsafe { tgetstr(cap.as_ptr(), &mut self.string_area_ptr) };
        if raw.is_null() {
            return None;
        }
        let bytes = unsafe { CStr::from_ptr(raw) }.to_bytes().to_vec();
        (!bytes.is_empty()).then_some(bytes)
    }

    fn get_number(&mut self, cap: &str) -> Option<i32> {
        let cap = CString::new(cap).ok()?;
        let value = unsafe { tgetnum(cap.as_ptr()) };
        (value != -1).then_some(value)
    }

    fn get_flag(&mut self, cap: &str) -> bool {
        let Ok(cap) = CString::new(cap) else {
            return false;
        };
        unsafe { tgetflag(cap.as_ptr()) != 0 }
    }
}

#[cfg(not(windows))]
fn open_platform_terminal_capability_database(
    term: &str,
) -> Option<Box<dyn TerminalCapabilityDatabase>> {
    UnixTermcapDatabase::open(term)
        .map(|database| Box::new(database) as Box<dyn TerminalCapabilityDatabase>)
}

#[cfg(windows)]
fn open_platform_terminal_capability_database(
    _term: &str,
) -> Option<Box<dyn TerminalCapabilityDatabase>> {
    // GNU Emacs' native Windows console backend does not link termcap.
    // nt/inc/ms-w32.h redirects tgetstr to sys_tgetstr, and
    // src/w32console.c implements that hook as a NULL capability lookup.
    None
}

#[cfg(test)]
#[path = "terminal_capabilities_test.rs"]
mod tests;
