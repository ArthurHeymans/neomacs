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
    // Wait for index.org content to appear
    let has_help = |grid: &[String]| {
        grid.iter()
            .any(|r| r.contains("FAQ") || r.contains("Doom Docs"))
    };
    gnu.read_until(Duration::from_secs(15), has_help);
    neo.read_until(Duration::from_secs(15), has_help);
    // Wait 3s for buffer colours to fully render after document loads
    gnu.read(Duration::from_secs(3));
    neo.read(Duration::from_secs(3));
    // Navigate to top of buffer, then to column 0 for aligned comparison
    gnu.send(b"gg0");
    neo.send(b"gg0");
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

    // ── Identify specific face colour mismatches ──
    // Match known text patterns against their fg colours
    let gnu_s = gnu.screen();
    let neo_s = neo.screen();
    let (rows, cols) = gnu_s.size();

    // Helper: find first cell containing substring, return its fg color
    let find_fg = |screen: &vt100::Screen, needle: &str| -> Option<vt100::Color> {
        for r in 0..rows {
            let mut buf = String::new();
            for c in 0..cols {
                if let Some(cell) = screen.cell(r, c) {
                    buf.push_str(cell.contents());
                }
            }
            if buf.contains(needle) {
                // Find first column where needle starts
                let pos = buf.find(needle).unwrap_or(0);
                let cell = screen.cell(r, pos as u16);
                return cell.map(|c| c.fgcolor());
            }
        }
        None
    };

    // Check specific text patterns
    for (label, needle) in [
        ("heading + Emacs", "+ Emacs & Emacs Lisp"),
        ("link example.com", "example.com"),
        ("link gnu.org", "gnu.org"),
        ("link github", "github.com"),
        ("heading + Doom", "+ Doom Emacs"),
    ] {
        let gnu_c = find_fg(gnu_s, needle);
        let neo_c = find_fg(neo_s, needle);
        eprintln!("  {label}: gnu-fg={gnu_c:?} neo-fg={neo_c:?}");
    }

    // Check overall colour diversity: how many distinct fg colours?
    let distinct_fgs = |screen: &vt100::Screen| -> usize {
        let mut set = std::collections::HashSet::new();
        for r in 0..rows {
            for c in 0..cols {
                if let Some(cell) = screen.cell(r, c) {
                    if !cell.contents().trim().is_empty() {
                        set.insert(format!("{:?}", cell.fgcolor()));
                    }
                }
            }
        }
        set.len()
    };
    let gnu_d = distinct_fgs(gnu_s);
    let neo_d = distinct_fgs(neo_s);
    eprintln!("Distinct fg colours: GNU={gnu_d} NEO={neo_d} (target: NEO >= GNU)");
    assert!(
        neo_d >= gnu_d,
        "Neomacs should have at least as many distinct fg colours as GNU (NEO={neo_d} vs GNU={gnu_d})"
    );
}
