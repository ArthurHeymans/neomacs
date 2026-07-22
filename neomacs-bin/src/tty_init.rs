//! TTY terminal detection, color/background sensing, and raw-mode lifecycle.
//!
//! Mirrors GNU Emacs `src/term.c` (`init_tty`, `tty_default_color_cells`,
//! `tty_default_color_mode`) and the environment probing that normally lives in
//! `init_display_interactive` (`src/dispnew.c`).
//!
//! All terminal I/O is cross-platform via crossterm.

use neovm_core::emacs_core::terminal::pure::TerminalRuntimeConfig;
use std::io::Write;

use super::{FrontendKind, StartupOptions};

/// Return a TTY terminal runtime configuration for interactive sessions.
pub fn detect_tty_runtime(startup: &StartupOptions) -> TerminalRuntimeConfig {
    TerminalRuntimeConfig::interactive(detect_tty_type(), detect_tty_color_cells())
        .with_name(detect_tty_name(startup))
}

pub fn detect_tty_type() -> Option<String> {
    std::env::var("TERM").ok().filter(|value| !value.is_empty())
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

/// Set up the terminal for the TtyRif direct-rendering path:
/// raw mode, alternate screen buffer, hidden cursor.
pub fn tty_init_terminal() {
    // Pick the SGR color depth from the terminal's detected color-cell count so
    // face colors are downsampled to a palette the terminal can render, instead
    // of always emitting 24-bit truecolor (issue #154). The `-nw` renderer is
    // `TtyRif` in neomacs-display-protocol, so set the tier there.
    neomacs_display_protocol::tty_rif::set_color_tier(detect_tty_color_cells());

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
