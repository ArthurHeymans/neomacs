// Face colour comparison test via PTY.
//!
// Boots neomacs and GNU Emacs side-by-side, opens the Doom help
// index.org, then compares the rendered screen for coloured cells.
// Reports whether neomacs has non-default face colours matching GNU.
#![allow(dead_code)]

mod support;
use neomacs_tui_tests::*;
use std::time::Duration;
use support::*;

#[test]
fn index_org_has_face_colours() {
    // Start both editors WITH init (Doom config)
    let mut gnu = TuiSession::gnu_emacs_with_init();
    let mut neo = TuiSession::neomacs_with_init("");
    // Wait for Doom startup on both
    let startup = |grid: &[String]| grid.iter().any(|r| r.contains("*doom*"));
    gnu.read_until(Duration::from_secs(20), startup);
    neo.read_until(Duration::from_secs(35), startup);
    // Settle — let Doom's async init complete
    gnu.read(Duration::from_millis(2000));
    neo.read(Duration::from_millis(2000));

    // Open Doom help: SPC h d h — send as one burst
    gnu.send(b" hdh");
    neo.send(b" hdh");
    // Wait for index.org content to appear (SPC h d h takes ~3s)
    let has_help = |grid: &[String]| {
        grid.iter()
            .any(|r| r.contains("FAQ") || r.contains("Doom Docs"))
    };
    gnu.read_until(Duration::from_secs(10), has_help);
    neo.read_until(Duration::from_secs(10), has_help);
    // Navigate to top of buffer, then to column 0 — ensures both
    // editors are scrolled to the same position for comparison.
    // gg = evil-goto-first-line, 0 = evil-first-non-blank / beginning-of-line
    gnu.send(b"gg0");
    neo.send(b"gg0");
    // Final settle
    gnu.read(Duration::from_secs(2));
    neo.read(Duration::from_secs(2));

    // Count cells with non-default foreground
    let count_colours = |sess: &TuiSession| -> (usize, usize) {
        let screen = sess.screen();
        let (rows, cols) = screen.size();
        let mut colour_fg = 0usize;
        let mut colour_any = 0usize;
        for r in 0..rows {
            for c in 0..cols {
                if let Some(cell) = screen.cell(r, c) {
                    if cell.fgcolor() != vt100::Color::Default {
                        colour_fg += 1;
                    }
                    if cell.fgcolor() != vt100::Color::Default
                        || cell.bgcolor() != vt100::Color::Default
                    {
                        colour_any += 1;
                    }
                }
            }
        }
        (colour_fg, colour_any)
    };

    let (gnu_fg, gnu_any) = count_colours(&gnu);
    let (neo_fg, neo_any) = count_colours(&neo);

    let total = gnu.screen().size().0 as usize * gnu.screen().size().1 as usize;
    eprintln!("GNU: {gnu_fg} fg-coloured / {gnu_any} any-coloured / {total} total cells");
    eprintln!("NEO: {neo_fg} fg-coloured / {neo_any} any-coloured / {total} total cells");

    // After the fix, neomacs must have coloured cells on the index.org page.
    assert!(
        neo_fg > 0,
        "Neomacs should have at least some cells with non-default foreground on index.org"
    );

    // Neomacs should be within a reasonable range of GNU
    // Cell-by-cell comparison where text content matches but color differs
    let gnu_s = gnu.screen();
    let neo_s = neo.screen();
    let (rows, cols) = gnu_s.size();
    let mut same_text_diff_fg = 0u64;
    let mut same_text_same_fg = 0u64;
    let mut shown = 0usize;
    for r in 0..rows {
        for c in 0..cols {
            if let (Some(gc), Some(nc)) = (gnu_s.cell(r, c), neo_s.cell(r, c)) {
                let gt = gc.contents();
                let nt = nc.contents();
                if gt == nt && !gt.trim().is_empty() {
                    if gc.fgcolor() == nc.fgcolor() {
                        same_text_same_fg += 1;
                    } else {
                        same_text_diff_fg += 1;
                        if shown < 15 {
                            eprintln!(
                                "  color-only [{r},{c}] txt='{gt}' gnu-fg={:?} neo-fg={:?}",
                                gc.fgcolor(),
                                nc.fgcolor()
                            );
                            shown += 1;
                        }
                    }
                }
            }
        }
    }
    eprintln!("same-text same-fg: {same_text_same_fg}, same-text diff-fg: {same_text_diff_fg}");
    let ratio = if same_text_same_fg + same_text_diff_fg > 0 {
        same_text_same_fg as f64 / (same_text_same_fg + same_text_diff_fg) as f64 * 100.0
    } else {
        100.0
    };
    eprintln!("Same-text fg match: {ratio:.1}%");
    assert!(
        ratio >= 80.0,
        "Neomacs should match GNU fg on >= 80% of same-text cells (got {ratio:.1}%)"
    );
}
