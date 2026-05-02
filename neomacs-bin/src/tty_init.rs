//! TTY terminal detection, color/background sensing, and raw-mode lifecycle.
//!
//! Mirrors GNU Emacs `src/term.c` (`init_tty`, `tty_default_color_cells`,
//! `tty_default_color_mode`) and the environment probing that normally lives in
//! `init_display_interactive` (`src/dispnew.c`).

use neovm_core::emacs_core::terminal::pure::TerminalRuntimeConfig;

use super::{FrontendKind, StartupOptions};

/// Return a TTY terminal runtime configuration for interactive sessions.
pub fn detect_tty_runtime(startup: &StartupOptions) -> TerminalRuntimeConfig {
    TerminalRuntimeConfig::interactive(detect_tty_type(), detect_tty_color_cells())
        .with_name(detect_tty_name(startup))
}

pub fn detect_tty_type() -> Option<String> {
    std::env::var("TERM").ok().filter(|value| !value.is_empty())
}

fn default_controlling_tty_name() -> &'static str {
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

/// Query the terminal size in character cells.  Returns `None` if the
/// ioctl fails or the dimensions are zero.
///
/// This is the canonical copy; `tty_frontend` shared this function, and
/// the TTY redisplay callback also uses it to drive resize detection.
#[cfg(unix)]
pub fn query_terminal_size_cells() -> Option<(u32, u32)> {
    use std::mem::MaybeUninit;

    unsafe {
        let mut winsize = MaybeUninit::<libc::winsize>::uninit();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, winsize.as_mut_ptr()) == 0 {
            let winsize = winsize.assume_init();
            if winsize.ws_col > 0 && winsize.ws_row > 0 {
                return Some((u32::from(winsize.ws_col), u32::from(winsize.ws_row)));
            }
        }
    }
    None
}

#[cfg(not(unix))]
pub fn query_terminal_size_cells() -> Option<(u32, u32)> {
    None
}

// ── TTY terminal lifecycle (raw mode, alt screen) ────────────────────────

/// Saved original termios for the TtyRif path. Stored globally so
/// `tty_shutdown_terminal` can restore it even from a panic handler.
#[cfg(unix)]
static TTY_SAVED_TERMIOS: std::sync::Mutex<Option<libc::termios>> = std::sync::Mutex::new(None);

/// Set up the terminal for the TtyRif direct-rendering path:
/// raw mode, alternate screen buffer, hidden cursor.
#[cfg(unix)]
pub fn tty_init_terminal() {
    use std::io::Write;
    use std::mem::MaybeUninit;

    unsafe {
        let mut original = MaybeUninit::<libc::termios>::uninit();
        if libc::tcgetattr(libc::STDIN_FILENO, original.as_mut_ptr()) != 0 {
            tracing::error!("tty_init_terminal: tcgetattr failed");
            return;
        }
        let original = original.assume_init();

        // Save for later restore
        if let Ok(mut guard) = TTY_SAVED_TERMIOS.lock() {
            *guard = Some(original);
        }

        let mut raw = original;
        // Input: no break, no CR->NL, no parity, no strip, no start/stop
        raw.c_iflag &= !(libc::BRKINT | libc::ICRNL | libc::INPCK | libc::ISTRIP | libc::IXON);
        // Output: disable post-processing
        raw.c_oflag &= !libc::OPOST;
        // Control: 8-bit chars
        raw.c_cflag |= libc::CS8;
        // Local: no echo, no canonical, no signals, no extended
        raw.c_lflag &= !(libc::ECHO | libc::ICANON | libc::ISIG | libc::IEXTEN);
        // Non-blocking reads
        raw.c_cc[libc::VMIN] = 0;
        raw.c_cc[libc::VTIME] = 0;

        if libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &raw) != 0 {
            tracing::error!("tty_init_terminal: tcsetattr failed");
            return;
        }
    }

    // Enter alternate screen, hide cursor, clear
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(b"\x1b[?1049h\x1b[?25l\x1b[2J");
    let _ = stdout.flush();
    tracing::info!("TTY terminal initialized (raw mode + alt screen)");
}

#[cfg(not(unix))]
pub fn tty_init_terminal() {
    tracing::warn!("tty_init_terminal: not implemented on this platform");
}

/// Restore the terminal to its original state: show cursor, leave alt screen,
/// reset SGR, restore saved termios.
#[cfg(unix)]
pub fn tty_shutdown_terminal() {
    use std::io::Write;

    // Show cursor, reset SGR, leave alternate screen
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(b"\x1b[0m\x1b[?25h\x1b[?1049l");
    let _ = stdout.flush();

    // Restore termios
    if let Ok(guard) = TTY_SAVED_TERMIOS.lock() {
        if let Some(ref original) = *guard {
            unsafe {
                let _ = libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, original);
            }
        }
    }
    tracing::info!("TTY terminal restored");
}

#[cfg(not(unix))]
pub fn tty_shutdown_terminal() {
    tracing::warn!("tty_shutdown_terminal: not implemented on this platform");
}

/// Returns `true` when the session is an interactive TTY (not batch and
/// not a GUI frontend).
pub fn should_enable_live_tty_io(startup: &StartupOptions) -> bool {
    startup.frontend == FrontendKind::Tty && !startup.noninteractive
}
