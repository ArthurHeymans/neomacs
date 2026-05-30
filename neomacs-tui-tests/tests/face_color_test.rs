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
    gnu.read_until(Duration::from_secs(15), startup);
    neo.read_until(Duration::from_secs(30), startup);
    // Settle
    gnu.read(Duration::from_millis(500));
    neo.read(Duration::from_millis(500));

    // Open Doom help: SPC h d h
    gnu.send_key("SPC");
    neo.send_key("SPC");
    std::thread::sleep(Duration::from_millis(50));
    gnu.send(b"hdh");
    neo.send(b"hdh");
    gnu.read(Duration::from_secs(8));
    neo.read(Duration::from_secs(8));

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
    let ratio = neo_fg as f64 / gnu_fg.max(1) as f64;
    eprintln!("Neomacs/GNU colour ratio: {ratio:.2}");
    assert!(
        ratio > 0.3,
        "Neomacs should have at least 30% of GNU's coloured cells (got {ratio:.2})"
    );
}
