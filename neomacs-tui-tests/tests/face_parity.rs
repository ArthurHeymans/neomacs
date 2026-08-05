//! Attribute-level face parity: same glyph, same paint.
//!
//! Every other comparison in this crate is text-only; `diff_screens`'s
//! color channel had no callers, so the suite was structurally blind to
//! face divergence -- exactly the class of TTY bug still on the backlog.
//! These tests compare cell colors (via `color_diffs_in`, which only
//! looks at cells whose characters already match) for the classic
//! face-sensitive scenarios: font-lock, region, isearch, mode-line,
//! and the minibuffer prompt.
//!
//! Comparison discipline: both editors run with TERM=screen-256color
//! and -Q, so any color mismatch on a char-identical cell is a real
//! face-pipeline divergence, not terminal-capability noise. Each
//! assertion retries until the two screens agree or a deadline passes,
//! because face painting can land a frame later than the text.

mod support;
use neomacs_tui_tests::*;
use std::time::{Duration, Instant};
use support::*;

/// Retry until the color diffs in `rows`/`cols` drain to empty or the
/// deadline passes; panic with a cell report if they never do.
fn assert_color_parity(
    label: &str,
    gnu: &mut TuiSession,
    neo: &mut TuiSession,
    rows: std::ops::Range<u16>,
    cols: std::ops::Range<u16>,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut diffs;
    loop {
        diffs = color_diffs_in(gnu.screen(), neo.screen(), rows.clone(), cols.clone());
        if diffs.is_empty() {
            return;
        }
        if Instant::now() >= deadline {
            break;
        }
        read_both(gnu, neo, Duration::from_millis(300));
    }
    dump_pair_grids(label, gnu, neo);
    panic!(
        "{label}: {} char-identical cells differ in color:\n{}",
        diffs.len(),
        format_color_diffs(&diffs, 30),
    );
}

/// Rows of the text area (everything above mode line and echo area).
const TEXT_ROWS: std::ops::Range<u16> = 0..(ROWS - 2);
/// The mode-line row under the default single-window layout.
const MODE_LINE_ROW: u16 = ROWS - 2;
/// The echo-area/minibuffer row.
const ECHO_ROW: u16 = ROWS - 1;

/// Font-lock over an Emacs Lisp buffer: keyword, function name, doc
/// string, and comment faces are the highest-traffic faces there are.
#[test]
fn font_lock_elisp_faces_match_gnu() {
    let (mut gnu, mut neo) = boot_pair("");
    wait_for_both(&mut gnu, &mut neo, Duration::from_secs(30), scratch_ready);

    eval_expression(
        &mut gnu,
        &mut neo,
        "(progn (switch-to-buffer (get-buffer-create \"fl.el\")) \
         (erase-buffer) \
         (insert \";; leading comment\\n\
                  (defun face-parity-probe (x)\\n\
                  \\\"Doc string face.\\\"\\n\
                  (let ((y (+ x 1))) (if (> y 0) 'positive nil)))\\n\") \
         (emacs-lisp-mode) (font-lock-ensure) (goto-char (point-min)) nil)",
    );

    // Wait until GNU has visibly fontified (the defun keyword row exists).
    let fontified = |grid: &[String]| grid.iter().any(|row| row.contains("face-parity-probe"));
    wait_for_both(&mut gnu, &mut neo, Duration::from_secs(10), fontified);

    assert_color_parity("elisp font-lock", &mut gnu, &mut neo, TEXT_ROWS, 0..COLS);
}

/// The active region highlight after C-SPC + motion.
#[test]
fn region_highlight_matches_gnu() {
    let (mut gnu, mut neo) = boot_pair("");
    wait_for_both(&mut gnu, &mut neo, Duration::from_secs(30), scratch_ready);

    eval_expression(
        &mut gnu,
        &mut neo,
        "(progn (switch-to-buffer (get-buffer-create \"region\")) \
         (erase-buffer) (insert \"alpha beta gamma\\ndelta epsilon zeta\\n\") \
         (goto-char (point-min)) nil)",
    );
    let ready = |grid: &[String]| grid.iter().any(|row| row.contains("alpha beta gamma"));
    wait_for_both(&mut gnu, &mut neo, Duration::from_secs(10), ready);

    // Activate the mark and extend the region across a line boundary.
    send_both(&mut gnu, &mut neo, "C-SPC");
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));
    send_both(&mut gnu, &mut neo, "C-n");
    send_both(&mut gnu, &mut neo, "C-e");
    read_both(&mut gnu, &mut neo, Duration::from_millis(500));

    assert_color_parity("region highlight", &mut gnu, &mut neo, TEXT_ROWS, 0..COLS);
}

/// Isearch: current-match face plus lazy highlight on other matches.
#[test]
fn isearch_highlight_matches_gnu() {
    let (mut gnu, mut neo) = boot_pair("");
    wait_for_both(&mut gnu, &mut neo, Duration::from_secs(30), scratch_ready);

    eval_expression(
        &mut gnu,
        &mut neo,
        "(progn (switch-to-buffer (get-buffer-create \"search\")) \
         (erase-buffer) \
         (insert \"needle in a haystack\\nanother needle here\\nlast needle line\\n\") \
         (goto-char (point-min)) nil)",
    );
    let ready = |grid: &[String]| grid.iter().any(|row| row.contains("needle in a haystack"));
    wait_for_both(&mut gnu, &mut neo, Duration::from_secs(10), ready);

    send_both(&mut gnu, &mut neo, "C-s");
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));
    for b in b"needle" {
        send_both_raw(&mut gnu, &mut neo, &[*b]);
    }
    // Wait for the echo area to show the search prompt in both, then let
    // lazy-highlight settle.
    let searching = |grid: &[String]| grid.iter().any(|row| row.contains("I-search: needle"));
    wait_for_both(&mut gnu, &mut neo, Duration::from_secs(5), searching);

    assert_color_parity("isearch highlight", &mut gnu, &mut neo, TEXT_ROWS, 0..COLS);
}

/// The mode line: on a TTY this is the highest-visibility face of all.
/// Only char-identical cells are compared, so the product-name segment
/// (GNU Emacs vs Neomacs) is excluded automatically.
///
/// Was RED when written (2026-08-05): neomacs painted X11 white
/// (255,255,255) where GNU paints the xterm palette entry (229,229,229)
/// that xterm-register-default-colors installs, because TTY face
/// realization resolved color names through the build-time X11 table and
/// never consulted the tty color table. Fixed by routing TTY-frame face
/// colors through tty-color-desc at face-sync time, mirroring GNU
/// realize_tty_face / map_tty_color (xfaces.c:6620).
#[test]
fn mode_line_face_matches_gnu() {
    let (mut gnu, mut neo) = boot_pair("");
    wait_for_both(&mut gnu, &mut neo, Duration::from_secs(30), scratch_ready);

    assert_color_parity(
        "mode line",
        &mut gnu,
        &mut neo,
        MODE_LINE_ROW..MODE_LINE_ROW + 1,
        0..COLS,
    );
}

/// The minibuffer prompt face during M-x.
///
/// Was RED when written (2026-08-05), same root cause and fix as
/// mode_line_face_matches_gnu: xterm palette "cyan" (0,205,205) vs X11
/// cyan (0,255,255).
#[test]
fn minibuffer_prompt_face_matches_gnu() {
    let (mut gnu, mut neo) = boot_pair("");
    wait_for_both(&mut gnu, &mut neo, Duration::from_secs(30), scratch_ready);

    send_both(&mut gnu, &mut neo, "M-x");
    let prompting = |grid: &[String]| grid.iter().any(|row| row.trim_start().starts_with("M-x"));
    wait_for_both(&mut gnu, &mut neo, Duration::from_secs(5), prompting);

    assert_color_parity(
        "minibuffer prompt",
        &mut gnu,
        &mut neo,
        ECHO_ROW..ECHO_ROW + 1,
        0..COLS,
    );

    // Leave the minibuffer so teardown is uniform.
    send_both(&mut gnu, &mut neo, "C-g");
}
