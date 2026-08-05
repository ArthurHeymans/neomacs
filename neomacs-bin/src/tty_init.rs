//! TTY terminal detection, color/background sensing, and raw-mode lifecycle.
//!
//! Mirrors GNU Emacs `src/term.c` (`init_tty`, `tty_default_color_cells`,
//! `tty_default_color_mode`) and the environment probing that normally lives in
//! `init_display_interactive` (`src/dispnew.c`).
//!
//! All terminal I/O is cross-platform via crossterm.

use neomacs_display_protocol::tty_capabilities::TtyAttributeCapabilities;
use neovm_core::emacs_core::terminal::pure::TerminalRuntimeConfig;
use std::io::Write;

use super::{FrontendKind, StartupOptions};

/// Return a TTY terminal runtime configuration for interactive sessions.
pub fn detect_tty_runtime(startup: &StartupOptions) -> TerminalRuntimeConfig {
    TerminalRuntimeConfig::interactive(detect_tty_type(), detect_tty_color_cells())
        .with_name(detect_tty_name(startup))
        .with_attribute_capabilities(detect_tty_attribute_capabilities())
}

/// What this terminal can render, from its terminfo entry -- the capabilities GNU
/// reads in `init_tty`.
///
/// ONE resolution point: the answer goes both to the terminal runtime, where
/// `display-supports-face-attributes-p` reads it (GNU `tty_capable_p`), and to the
/// renderer, which emits from it (GNU `turn_on_face`). A terminfo entry that
/// cannot be read is assumed fully capable rather than incapable -- neomacs'
/// choice, so a missing terminfo database does not silently strip every
/// highlight -- and the color-cell count is detected separately (COLORTERM, which
/// terminfo does not describe).
pub fn detect_tty_attribute_capabilities() -> TtyAttributeCapabilities {
    let mut caps = detect_tty_type()
        .and_then(|term| super::terminal_capabilities::tty_attribute_capabilities_for_term(&term))
        .unwrap_or_else(|| {
            tracing::debug!("no terminfo entry for TERM; assuming full capabilities");
            TtyAttributeCapabilities::full()
        });
    caps.color_cells = detect_tty_color_cells();
    caps
}

pub fn detect_tty_type() -> Option<String> {
    std::env::var("TERM").ok().filter(|value| !value.is_empty())
}

/// Detect the update-planner capabilities for the connected terminal.
///
/// Unlike the attribute fallback above (where over-claiming merely styles
/// text a dumb terminal ignores), over-claiming a planner capability emits
/// scroll/shift bytes an unknown terminal may not implement while the
/// screen model assumes it did — permanent corruption. An unset TERM or
/// unreadable terminfo therefore falls to the conservative floor.
pub fn detect_term_caps() -> neomacs_display_runtime::backend::tty::rif::TermCaps {
    detect_tty_type()
        .and_then(|term| super::terminal_capabilities::term_caps_for_term(&term))
        .unwrap_or_else(|| {
            tracing::debug!("no terminfo entry for TERM; refusing planner optimizations");
            neomacs_display_runtime::backend::tty::rif::TermCaps::unknown_terminal()
        })
}

pub(crate) fn default_controlling_tty_name() -> &'static str {
    #[cfg(windows)]
    {
        "CONOUT$"
    }
    #[cfg(not(windows))]
    {
        "/dev/tty"
    }
}

pub fn detect_tty_name(_startup: &StartupOptions) -> String {
    default_controlling_tty_name().to_string()
}

pub fn detect_tty_color_cells() -> i64 {
    let colorterm = std::env::var("COLORTERM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if colorterm.contains("truecolor") || colorterm.contains("24bit") {
        return 16777216;
    }

    let term = std::env::var("TERM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if term.is_empty() || term == "dumb" {
        return 0;
    }
    if term.contains("256color") {
        return 256;
    }
    8
}

pub fn detect_tty_background_mode() -> &'static str {
    let Some(colorfgbg) = std::env::var("COLORFGBG").ok() else {
        return "dark";
    };
    let Some(background) = colorfgbg
        .split(';')
        .next_back()
        .and_then(|value| value.parse::<i32>().ok())
    else {
        return "dark";
    };

    if (7..=15).contains(&background) {
        "light"
    } else {
        "dark"
    }
}

// ── Terminal size ─────────────────────────────────────────────────────────

fn parse_positive_u32(value: Option<String>) -> Option<u32> {
    value
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
}

fn terminal_size_from_env_values(
    columns: Option<String>,
    lines: Option<String>,
) -> Option<(u32, u32)> {
    let cols = parse_positive_u32(columns)?;
    let rows = parse_positive_u32(lines)?;
    Some((cols, rows))
}

fn terminal_size_from_env() -> Option<(u32, u32)> {
    terminal_size_from_env_values(std::env::var("COLUMNS").ok(), std::env::var("LINES").ok())
}

/// Query the terminal size in character cells via crossterm.
/// Falls back to `COLUMNS`/`LINES` environment variables.
pub fn query_terminal_size_cells() -> Option<(u32, u32)> {
    crossterm::terminal::size()
        .ok()
        .map(|(cols, rows)| (cols as u32, rows as u32))
        .or_else(terminal_size_from_env)
}

// ── TTY terminal lifecycle (raw mode, alt screen) ────────────────────────

/// GNU's init_tty refusal (term.c:4881): before touching terminal modes,
/// verify the terminal can position the cursor. Returns the diagnostic to
/// print (and exit with) when it cannot; checked here, while stdout is
/// still cooked and no alternate-screen byte has been written.
pub fn tty_check_terminal_powerful_enough() -> Result<(), String> {
    match detect_tty_type() {
        Some(term) => super::terminal_capabilities::check_terminal_powerful_enough(&term),
        None => Ok(()),
    }
}

/// Set up the terminal for the TtyRif direct-rendering path:
/// raw mode, alternate screen buffer, hidden cursor.
pub fn tty_init_terminal() {
    // Register what this terminal can render, as GNU does in `init_tty`: the
    // terminfo attribute strings (`sitm`, `smul`, `Smulx`, `bold`, `dim`, `smxx`,
    // `ncv`) plus the color depth. The renderer then emits an attribute only when
    // the terminal has it, with GNU's fallbacks -- and the `-nw` face colors are
    // downsampled to a palette the terminal can render instead of always emitting
    // 24-bit truecolor (issue #154).
    //
    // Same record the terminal runtime carries (see
    // `detect_tty_attribute_capabilities`), so the predicate and the renderer
    // cannot disagree about what this terminal can do.
    neomacs_display_runtime::backend::tty::rif::set_capabilities(
        detect_tty_attribute_capabilities(),
    );

    if let Err(e) = crossterm::terminal::enable_raw_mode() {
        tracing::error!("tty_init_terminal: enable_raw_mode failed: {}", e);
        return;
    }

    // Enter alternate screen, hide cursor, clear
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(b"\x1b[?1049h\x1b[?25l\x1b[2J");
    let _ = stdout.flush();
    tracing::info!("TTY terminal initialized (raw mode + alt screen)");
}

/// Restore the terminal to its original state: show cursor, leave alt screen,
/// reset SGR, disable raw mode.
pub fn tty_shutdown_terminal() {
    // Show cursor, reset SGR, leave alternate screen
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(b"\x1b[0m\x1b[?25h\x1b[?1049l");
    let _ = stdout.flush();

    // Restore raw mode
    if let Err(e) = crossterm::terminal::disable_raw_mode() {
        tracing::warn!("tty_shutdown_terminal: disable_raw_mode failed: {}", e);
    } else {
        tracing::info!("TTY terminal restored");
    }
}

/// Returns `true` when the session is an interactive TTY (not batch and
/// not a GUI frontend).
pub fn should_enable_live_tty_io(startup: &StartupOptions) -> bool {
    startup.frontend == FrontendKind::Tty && !startup.noninteractive
}

#[cfg(test)]
mod tests {
    use super::terminal_size_from_env_values;

    #[test]
    fn terminal_size_from_env_values_uses_positive_columns_and_lines() {
        assert_eq!(
            terminal_size_from_env_values(Some("160".to_string()), Some("50".to_string())),
            Some((160, 50))
        );
    }

    #[test]
    fn terminal_size_from_env_values_rejects_missing_zero_or_invalid_values() {
        assert_eq!(
            terminal_size_from_env_values(None, Some("50".to_string())),
            None
        );
        assert_eq!(
            terminal_size_from_env_values(Some("160".to_string()), Some("0".to_string())),
            None
        );
        assert_eq!(
            terminal_size_from_env_values(Some("wide".to_string()), Some("50".to_string())),
            None
        );
    }
}
