//! Issue #140: with `truncate-lines` non-nil, `C-e` to the end of a line that
//! extends past the window's right edge must auto-hscroll so the cursor stays
//! visible — matching GNU's `hscroll_window_tree` exactly. Before the fix
//! neomacs left `window-hscroll` at 0 and dropped the cursor off-screen.
//!
//! Two branches of GNU's centering target are exercised (the difference is
//! GNU `ITERATOR_AT_END_OF_LINE_P` = the char AT point is a newline):
//!   * point at EOB, no trailing newline  -> centering, `text_cols/2`  (hscroll 220)
//!   * point before a trailing newline    -> end-of-line, `text_cols-4` (hscroll 144)
//! Both must equal live GNU's `window-hscroll`.
mod support;

use neomacs_tui_tests::TuiSession;
use std::time::Duration;
use support::*;

/// Read `(window-hscroll)` out of one editor via a uniquely-marked message.
fn window_hscroll(s: &mut TuiSession) -> i64 {
    eval_expression_one(s, "(message \"HSXX%dXX\" (window-hscroll))");
    s.read(Duration::from_millis(500));
    let (rows, _) = s.screen_size();
    for r in (0..rows).rev() {
        let t = s.row_text(r);
        if let Some(i) = t.find("HSXX") {
            let n: String = t[i + 4..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(v) = n.parse::<i64>() {
                return v;
            }
        }
    }
    -1
}

/// Run `setup` (which inserts a long line + sets truncate-lines + goes to BOL),
/// press C-e on both editors, and return (gnu_hscroll, neo_hscroll, gnu_cursor,
/// neo_cursor).
fn ctrl_e_scenario(setup: &str) -> (i64, i64, (u16, u16), (u16, u16)) {
    let (mut gnu, mut neo) = boot_pair("");
    resize_both(&mut gnu, &mut neo, 40, 160);
    read_both(&mut gnu, &mut neo, Duration::from_millis(700));
    eval_expression(&mut gnu, &mut neo, setup);
    read_both(&mut gnu, &mut neo, Duration::from_millis(900));
    send_both(&mut gnu, &mut neo, "C-e");
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));
    let gc = gnu.screen().cursor_position();
    let nc = neo.screen().cursor_position();
    let gh = window_hscroll(&mut gnu);
    let nh = window_hscroll(&mut neo);
    (gh, nh, gc, nc)
}

fn assert_matches_gnu(label: &str, gh: i64, nh: i64, gc: (u16, u16), nc: (u16, u16)) {
    eprintln!(
        "issue#140 [{label}]: GNU hscroll={gh} cursor={gc:?}  NEO hscroll={nh} cursor={nc:?}"
    );
    assert!(
        gh > 0,
        "[{label}] precondition: GNU must auto-hscroll (got {gh})"
    );
    assert_eq!(nh, gh, "[{label}] neomacs window-hscroll must equal GNU's");
    assert_eq!(
        nc.0, gc.0,
        "[{label}] cursor must be on the same (long-line) row as GNU, not dropped off",
    );
    assert!(
        (nc.1 as i64 - gc.1 as i64).abs() <= 1,
        "[{label}] cursor column {} must be within 1 of GNU's {}",
        nc.1,
        gc.1,
    );
}

#[test]
fn issue_140_ce_at_eob_no_newline_centers() {
    // 300 x's, NO trailing newline -> point at EOB -> GNU centers (text_cols/2).
    let (gh, nh, gc, nc) = ctrl_e_scenario(
        "(progn (erase-buffer) (setq truncate-lines t) \
         (insert (make-string 300 ?x)) (goto-char (point-min)) nil)",
    );
    assert_matches_gnu("EOB/centering", gh, nh, gc, nc);
}

#[test]
fn issue_140_ce_before_newline_targets_eol() {
    // 300 x's + a newline -> point before the newline -> GNU end-of-line (text_cols-4).
    let (gh, nh, gc, nc) = ctrl_e_scenario(
        "(progn (erase-buffer) (setq truncate-lines t) \
         (insert (make-string 300 ?x) 10) (goto-char (point-min)) nil)",
    );
    assert_matches_gnu("before-newline/EOL", gh, nh, gc, nc);
}
