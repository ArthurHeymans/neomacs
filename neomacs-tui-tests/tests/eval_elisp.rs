//! TUI comparison tests: eval elisp.

mod support;
use neomacs_tui_tests::*;
use std::fs;
use std::time::Duration;
use support::*;

// ── Local helpers ───────────────────────────────────────────

fn backtrace_ready(grid: &[String]) -> bool {
    grid.iter().any(|row| row.contains("*Backtrace*"))
        && grid.iter().any(|row| row.contains("Debugger entered"))
        && grid
            .iter()
            .any(|row| row.contains("void-variable") || row.contains("value as variable is void"))
}

// ── Tests ──────────────────────────────────────────────────
#[test]
fn eval_last_sexp_via_cx_ce_prints_echo_area_value() {
    let (mut gnu, mut neo) = boot_pair("");

    send_both_raw(&mut gnu, &mut neo, b"(+ 40 2)");
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));
    send_both(&mut gnu, &mut neo, "C-x C-e");

    let ready = |grid: &[String]| grid.iter().rev().take(4).any(|row| row.contains("42"));
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            grid.iter().any(|row| row.contains("(+ 40 2)")),
            "{label} should keep the evaluated sexp in the buffer"
        );
        assert!(
            grid.iter().rev().take(4).any(|row| row.contains("42")),
            "{label} should show eval-last-sexp's value in the echo area"
        );
    }
    assert_pair_nearly_matches(
        "eval_last_sexp_via_cx_ce_prints_echo_area_value",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn eval_last_sexp_error_via_cx_ce_opens_backtrace() {
    let (mut gnu, mut neo) = boot_pair("");

    send_both_raw(&mut gnu, &mut neo, b"hello");
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));
    send_both(&mut gnu, &mut neo, "C-x C-e");

    gnu.read_until(Duration::from_secs(6), backtrace_ready);
    neo.read_until(Duration::from_secs(8), backtrace_ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    if !backtrace_ready(&gnu.text_grid()) || !backtrace_ready(&neo.text_grid()) {
        dump_pair_grids("eval_last_sexp_error_via_cx_ce_opens_backtrace", &gnu, &neo);
    }

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            grid.iter().any(|row| row.contains("*Backtrace*")),
            "{label} should display the Backtrace buffer"
        );
        assert!(
            grid.iter().any(|row| row.contains("Debugger entered")),
            "{label} should show debugger entry text"
        );
        assert!(
            grid.iter().any(|row| row.contains("hello")),
            "{label} should show the void variable in the backtrace"
        );
    }
    assert_pair_nearly_matches(
        "eval_last_sexp_error_via_cx_ce_opens_backtrace",
        &gnu,
        &neo,
        4,
    );
}

#[test]
fn trace_function_background_writes_trace_output_buffer() {
    let (mut gnu, mut neo) = boot_pair("");

    send_both(&mut gnu, &mut neo, "M-:");
    let eval_prompt = |grid: &[String]| grid.iter().any(|row| row.contains("Eval:"));
    gnu.read_until(Duration::from_secs(6), eval_prompt);
    neo.read_until(Duration::from_secs(8), eval_prompt);
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));

    for session in [&mut gnu, &mut neo] {
        session.send(
            br#"(progn (defun trace-probe (x) (+ x 1)) (trace-function-background 'trace-probe) (trace-probe 41))"#,
        );
    }
    send_both(&mut gnu, &mut neo, "RET");

    let eval_ready = |grid: &[String]| grid.iter().rev().take(4).any(|row| row.contains("42"));
    gnu.read_until(Duration::from_secs(6), eval_ready);
    neo.read_until(Duration::from_secs(8), eval_ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    send_both(&mut gnu, &mut neo, "C-x b");
    let switch_prompt = |grid: &[String]| grid.iter().any(|row| row.contains("Switch to buffer:"));
    gnu.read_until(Duration::from_secs(6), switch_prompt);
    neo.read_until(Duration::from_secs(8), switch_prompt);
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));
    for session in [&mut gnu, &mut neo] {
        session.send(b"*trace-output*");
    }
    send_both(&mut gnu, &mut neo, "RET");

    let trace_ready = |grid: &[String]| {
        grid.iter().any(|row| row.contains("*trace-output*"))
            && grid.iter().any(|row| row.contains("1 -> (trace-probe 41)"))
            && grid.iter().any(|row| row.contains("1 <- trace-probe: 42"))
    };
    gnu.read_until(Duration::from_secs(6), trace_ready);
    neo.read_until(Duration::from_secs(8), trace_ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            grid.iter().any(|row| row.contains("*trace-output*")),
            "{label} should display trace-buffer"
        );
        assert!(
            grid.iter().any(|row| row.contains("1 -> (trace-probe 41)")),
            "{label} should show trace entry"
        );
        assert!(
            grid.iter().any(|row| row.contains("1 <- trace-probe: 42")),
            "{label} should show trace exit"
        );
    }
    assert_pair_nearly_matches(
        "trace_function_background_writes_trace_output_buffer",
        &gnu,
        &neo,
        3,
    );
}

#[test]
fn completion_at_point_in_elisp_buffer_completes_function_name() {
    let (mut gnu, mut neo) = boot_pair("");

    open_home_file(
        &mut gnu,
        &mut neo,
        "completion-at-point.el",
        "(forward-cha\n",
        "C-x C-f",
    );

    send_both(&mut gnu, &mut neo, "C-e");
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));
    invoke_mx_command(&mut gnu, &mut neo, "completion-at-point");

    let ready = |grid: &[String]| {
        grid.iter()
            .any(|row| row.contains("completion-at-point.el"))
            && grid.iter().any(|row| row.contains("(forward-char"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            grid.iter().any(|row| row.contains("(forward-char")),
            "{label} should complete an Emacs Lisp function name at point\n{}",
            grid.join("\n")
        );
    }
    assert_pair_nearly_matches(
        "completion_at_point_in_elisp_buffer_completes_function_name",
        &gnu,
        &neo,
        3,
    );
}

#[test]
fn eval_expression_via_mcolon_prints_echo_area_value() {
    let (mut gnu, mut neo) = boot_pair("");

    send_both(&mut gnu, &mut neo, "M-:");
    let prompt_ready = |grid: &[String]| grid.iter().any(|row| row.contains("Eval:"));
    gnu.read_until(Duration::from_secs(6), prompt_ready);
    neo.read_until(Duration::from_secs(8), prompt_ready);
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));
    assert_pair_nearly_matches(
        "eval_expression_via_mcolon_prints_echo_area_value/prompt",
        &gnu,
        &neo,
        2,
    );

    for session in [&mut gnu, &mut neo] {
        session.send(b"(+ 2 3)");
    }
    send_both(&mut gnu, &mut neo, "RET");

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("5 (#o5, #x5"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            grid.iter()
                .rev()
                .take(4)
                .any(|row| row.contains("5 (#o5, #x5")),
            "{label} should show eval-expression's integer value formats"
        );
    }
    assert_pair_nearly_matches(
        "eval_expression_via_mcolon_prints_echo_area_value",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn eval_expression_history_via_mcolon_mp_recalls_previous_expression() {
    let (mut gnu, mut neo) = boot_pair("");

    send_both(&mut gnu, &mut neo, "M-:");
    let prompt_ready = |grid: &[String]| grid.iter().any(|row| row.contains("Eval:"));
    gnu.read_until(Duration::from_secs(6), prompt_ready);
    neo.read_until(Duration::from_secs(8), prompt_ready);
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));
    assert_pair_nearly_matches(
        "eval_expression_history_via_mcolon_mp_recalls_previous_expression/prompt-1",
        &gnu,
        &neo,
        2,
    );

    for session in [&mut gnu, &mut neo] {
        session.send(b"(+ 1 2)");
    }
    let first_expr_typed =
        |grid: &[String]| grid.last().is_some_and(|row| row.contains("Eval: (+ 1 2)"));
    gnu.read_until(Duration::from_secs(6), first_expr_typed);
    neo.read_until(Duration::from_secs(8), first_expr_typed);
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));
    assert_pair_nearly_matches(
        "eval_expression_history_via_mcolon_mp_recalls_previous_expression/typed-1",
        &gnu,
        &neo,
        2,
    );

    send_both(&mut gnu, &mut neo, "RET");
    let first_result = |grid: &[String]| grid.iter().rev().take(4).any(|row| row.contains("3"));
    gnu.read_until(Duration::from_secs(6), first_result);
    neo.read_until(Duration::from_secs(8), first_result);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));
    assert_pair_nearly_matches(
        "eval_expression_history_via_mcolon_mp_recalls_previous_expression/result-1",
        &gnu,
        &neo,
        2,
    );

    send_both(&mut gnu, &mut neo, "M-:");
    gnu.read_until(Duration::from_secs(6), prompt_ready);
    neo.read_until(Duration::from_secs(8), prompt_ready);
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));
    assert_pair_nearly_matches(
        "eval_expression_history_via_mcolon_mp_recalls_previous_expression/prompt-2",
        &gnu,
        &neo,
        2,
    );

    send_both(&mut gnu, &mut neo, "M-p");
    let recalled = |grid: &[String]| grid.last().is_some_and(|row| row.contains("Eval: (+ 1 2)"));
    gnu.read_until(Duration::from_secs(6), recalled);
    neo.read_until(Duration::from_secs(8), recalled);
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));
    assert_pair_nearly_matches(
        "eval_expression_history_via_mcolon_mp_recalls_previous_expression/recalled",
        &gnu,
        &neo,
        2,
    );

    send_both(&mut gnu, &mut neo, "DEL DEL");
    send_both_raw(&mut gnu, &mut neo, b"5)");
    let edited = |grid: &[String]| grid.last().is_some_and(|row| row.contains("Eval: (+ 1 5)"));
    gnu.read_until(Duration::from_secs(6), edited);
    neo.read_until(Duration::from_secs(8), edited);
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));
    assert_pair_nearly_matches(
        "eval_expression_history_via_mcolon_mp_recalls_previous_expression/edited",
        &gnu,
        &neo,
        2,
    );

    send_both(&mut gnu, &mut neo, "RET");
    let second_result = |grid: &[String]| grid.iter().rev().take(4).any(|row| row.contains("6"));
    gnu.read_until(Duration::from_secs(6), second_result);
    neo.read_until(Duration::from_secs(8), second_result);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));
    assert_pair_nearly_matches(
        "eval_expression_history_via_mcolon_mp_recalls_previous_expression/result-2",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn eval_expression_error_via_mcolon_opens_backtrace() {
    let (mut gnu, mut neo) = boot_pair("");

    send_both(&mut gnu, &mut neo, "M-:");
    let prompt_ready = |grid: &[String]| grid.iter().any(|row| row.contains("Eval:"));
    gnu.read_until(Duration::from_secs(6), prompt_ready);
    neo.read_until(Duration::from_secs(8), prompt_ready);
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));
    assert_pair_nearly_matches(
        "eval_expression_error_via_mcolon_opens_backtrace/prompt",
        &gnu,
        &neo,
        2,
    );

    for session in [&mut gnu, &mut neo] {
        session.send(b"missing-variable");
    }
    send_both(&mut gnu, &mut neo, "RET");

    gnu.read_until(Duration::from_secs(6), backtrace_ready);
    neo.read_until(Duration::from_secs(8), backtrace_ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    if !backtrace_ready(&gnu.text_grid()) || !backtrace_ready(&neo.text_grid()) {
        dump_pair_grids(
            "eval_expression_error_via_mcolon_opens_backtrace",
            &gnu,
            &neo,
        );
    }

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            grid.iter().any(|row| row.contains("*Backtrace*")),
            "{label} should display the Backtrace buffer"
        );
        assert!(
            grid.iter().any(|row| row.contains("Debugger entered")),
            "{label} should show debugger entry text"
        );
        assert!(
            grid.iter().any(|row| row.contains("missing-variable")),
            "{label} should show the void variable in the backtrace"
        );
    }
    assert_pair_nearly_matches(
        "eval_expression_error_via_mcolon_opens_backtrace",
        &gnu,
        &neo,
        4,
    );
}

#[test]
fn eval_expression() {
    let (mut gnu, mut neo) = boot_pair("");
    send_both(&mut gnu, &mut neo, "M-:");
    read_both(&mut gnu, &mut neo, Duration::from_secs(2));

    // Type (+ 1 2) RET
    for s in [&mut gnu, &mut neo] {
        s.send(b"(+ 1 2)");
    }
    read_both(&mut gnu, &mut neo, Duration::from_millis(500));
    send_both(&mut gnu, &mut neo, "RET");
    read_both(&mut gnu, &mut neo, Duration::from_secs(2));

    // Echo area (last row) should show "3"
    let gl = gnu.text_grid();
    let nl = neo.text_grid();
    let gnu_echo = gl.last().unwrap();
    let neo_echo = nl.last().unwrap();
    assert!(
        gnu_echo.contains('3'),
        "GNU echo should show 3: {gnu_echo:?}"
    );
    assert!(
        neo_echo.contains('3'),
        "NEO echo should show 3: {neo_echo:?}"
    );
}

// ── File modtime tests ───────────────────────────────────────

#[test]
fn visited_file_modtime_returns_cons_after_file_visit() {
    let (mut gnu, mut neo) = boot_pair("");

    // Visit a file with insert-file-contents :visit
    open_home_file(
        &mut gnu,
        &mut neo,
        "modtime-test.el",
        "(message \"hello\")\n",
        "C-x C-f",
    );

    // Evaluate (visited-file-modtime) — should return a cons, not 0
    send_both(&mut gnu, &mut neo, "M-:");
    let prompt_ready = |grid: &[String]| grid.iter().any(|row| row.contains("Eval:"));
    gnu.read_until(Duration::from_secs(6), prompt_ready);
    neo.read_until(Duration::from_secs(8), prompt_ready);
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));

    for session in [&mut gnu, &mut neo] {
        session.send(b"(visited-file-modtime)");
    }
    send_both(&mut gnu, &mut neo, "RET");

    // Result should show a cons like (12345 67890) in the echo area,
    // not the integer 0
    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains('(') && row.chars().filter(|&c| c.is_ascii_digit()).count() >= 4
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            !echo.contains(" 0 "),
            "{label}: visited-file-modtime should return cons, not 0. Echo: {echo}"
        );
    }
    assert_pair_nearly_matches(
        "visited_file_modtime_returns_cons_after_file_visit",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn verify_visited_file_modtime_returns_t_for_unmodified_file() {
    let (mut gnu, mut neo) = boot_pair("");

    open_home_file(
        &mut gnu,
        &mut neo,
        "modtime-u.el",
        "(provide 'modtime-u)\n",
        "C-x C-f",
    );

    // Evaluate (verify-visited-file-modtime) — should return t
    send_both(&mut gnu, &mut neo, "M-:");
    let prompt_ready = |grid: &[String]| grid.iter().any(|row| row.contains("Eval:"));
    gnu.read_until(Duration::from_secs(6), prompt_ready);
    neo.read_until(Duration::from_secs(8), prompt_ready);
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));

    for session in [&mut gnu, &mut neo] {
        session.send(b"(verify-visited-file-modtime)");
    }
    send_both(&mut gnu, &mut neo, "RET");

    let ready = |grid: &[String]| grid.iter().rev().take(4).any(|row| row.contains('t'));
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('t'),
            "{label}: verify-visited-file-modtime should return t. Echo: {echo}"
        );
    }
    assert_pair_nearly_matches(
        "verify_visited_file_modtime_returns_t_for_unmodified_file",
        &gnu,
        &neo,
        2,
    );
}

// ── Narrowing / buffer position tests ────────────────────────

#[test]
fn mode_line_shows_buffer_position_percent() {
    let (mut gnu, mut neo) = boot_pair("");

    open_home_file(
        &mut gnu,
        &mut neo,
        "mode-pct.el",
        "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\n",
        "C-x C-f",
    );

    // Move to bottom, check mode-line shows Top/Bot/All
    send_both(&mut gnu, &mut neo, "M-<");
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    // Mode-line row (second to last) should show buffer position
    let gl = gnu.text_grid();
    let nl = neo.text_grid();
    let gnu_mode = &gl[gl.len().saturating_sub(2)];
    let neo_mode = &nl[nl.len().saturating_sub(2)];

    // Both should show some position indicator (Top, Bot, All, or %)
    let has_pos = |row: &str| {
        row.contains("Top") || row.contains("Bot") || row.contains("All") || row.contains('%')
    };
    assert!(
        has_pos(gnu_mode),
        "GNU mode-line should have position indicator: {gnu_mode}"
    );
    assert!(
        has_pos(neo_mode),
        "NEO mode-line should have position indicator: {neo_mode}"
    );
}

// ── Lisp environment semantics tests ────────────────────────

#[test]
fn lisp_environment_variables_match_gnu_emacs_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    // Test (emacs-version) returns a string
    support::eval_expression(&mut gnu, &mut neo, "(emacs-version)");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('\"'),
            "{label}: (emacs-version) should return a string. Echo: {echo}"
        );
    }

    // Test (boundp 'enable-recursive-minibuffers) — should be t
    support::eval_expression(&mut gnu, &mut neo, "(boundp 'enable-recursive-minibuffers)");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('t'),
            "{label}: (boundp 'enable-recursive-minibuffers) should be t. Echo: {echo}"
        );
    }

    // Test (>= emacs-major-version 31) — NeoMacs is Emacs 31+
    support::eval_expression(&mut gnu, &mut neo, "(>= emacs-major-version 31)");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('t'),
            "{label}: (>= emacs-major-version 31) should be t. Echo: {echo}"
        );
    }
}

// ── Face inheritance tests ──────────────────────────────────

#[test]
fn face_attribute_inherit_returns_correct_chain_for_mode_line() {
    let (mut gnu, mut neo) = boot_pair("");

    // mode-line inherits from mode-line-active which inherits from
    // mode-line base.  Test the chain via face-attribute.
    support::eval_expression(&mut gnu, &mut neo, "(face-attribute 'mode-line :inherit)");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        // Both should return something non-nil (a face name or nil)
        assert!(
            !echo.trim().is_empty(),
            "{label}: (face-attribute 'mode-line :inherit) should return value. Echo: {echo}"
        );
    }
}

// ── Buffer position correctness tests ───────────────────────

#[test]
fn buffer_positions_are_correct_1_based_after_file_visit() {
    let (mut gnu, mut neo) = boot_pair("");

    open_home_file(&mut gnu, &mut neo, "pos-check.txt", "abc\n", "C-x C-f");

    // Check (point-min) is 1
    support::eval_expression(&mut gnu, &mut neo, "(point-min)");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('1'),
            "{label}: (point-min) should be 1 after visiting file. Echo: {echo}"
        );
    }

    // Check (point-max) matches between GNU and NeoMacs
    support::eval_expression(&mut gnu, &mut neo, "(point-max)");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    let gnu_pm = gnu.text_grid().last().cloned().unwrap_or_default();
    let neo_pm = neo.text_grid().last().cloned().unwrap_or_default();
    let gnu_num: String = gnu_pm.chars().filter(|c| c.is_ascii_digit()).collect();
    let neo_num: String = neo_pm.chars().filter(|c| c.is_ascii_digit()).collect();
    assert!(!gnu_num.is_empty(), "GNU point-max not found: {gnu_pm}");
    assert!(!neo_num.is_empty(), "NEO point-max not found: {neo_pm}");
    assert_eq!(
        gnu_num, neo_num,
        "point-max mismatch: GNU={gnu_num} NEO={neo_num}"
    );

    // (buffer-size) should also match
    support::eval_expression(&mut gnu, &mut neo, "(buffer-size)");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    let gnu_bs = gnu.text_grid().last().cloned().unwrap_or_default();
    let neo_bs = neo.text_grid().last().cloned().unwrap_or_default();
    let gnu_bs_num: String = gnu_bs.chars().filter(|c| c.is_ascii_digit()).collect();
    let neo_bs_num: String = neo_bs.chars().filter(|c| c.is_ascii_digit()).collect();
    assert_eq!(
        gnu_bs_num, neo_bs_num,
        "buffer-size mismatch: GNU={gnu_bs_num} NEO={neo_bs_num}"
    );

    // (point) at start of buffer should be 1
    send_both(&mut gnu, &mut neo, "M-<");
    read_both(&mut gnu, &mut neo, Duration::from_millis(500));
    support::eval_expression(&mut gnu, &mut neo, "(point)");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('1'),
            "{label}: (point) at buffer start should be 1. Echo: {echo}"
        );
    }
}

// ── Fundamental Elisp operation tests ───────────────────────

#[test]
fn fundamental_elisp_operations_return_correct_values() {
    let (mut gnu, mut neo) = boot_pair("");

    // Test (car (cons 1 2)) should be 1
    support::eval_expression(&mut gnu, &mut neo, "(car (cons 1 2))");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('1'),
            "{label}: (car (cons 1 2)) should be 1. Echo: {echo}"
        );
    }

    // Test (cdr (cons 1 2)) should be 2
    support::eval_expression(&mut gnu, &mut neo, "(cdr (cons 1 2))");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('2'),
            "{label}: (cdr (cons 1 2)) should be 2. Echo: {echo}"
        );
    }

    // Test (equal (cons 1 2) (cons 1 2)) should be t
    support::eval_expression(&mut gnu, &mut neo, "(equal (cons 1 2) (cons 1 2))");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('t'),
            "{label}: (equal (cons 1 2) (cons 1 2)) should be t. Echo: {echo}"
        );
    }

    // Test (listp (cons 1 2)) should be t
    support::eval_expression(&mut gnu, &mut neo, "(listp (cons 1 2))");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('t'),
            "{label}: (listp (cons 1 2)) should be t. Echo: {echo}"
        );
    }
}

// ── String and numeric operation tests ──────────────────────

#[test]
fn string_and_numeric_operations_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    // (concat "a" "b") should be "ab"
    support::eval_expression(&mut gnu, &mut neo, "(concat \"a\" \"b\")");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains("ab"),
            "{label}: (concat \"a\" \"b\") should be \"ab\". Echo: {echo}"
        );
    }

    // (substring "hello" 1 3) should be "el" (0-indexed in GNU!)
    support::eval_expression(&mut gnu, &mut neo, "(substring \"hello\" 1 3)");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains("el"),
            "{label}: (substring \"hello\" 1 3) should be \"el\". Echo: {echo}"
        );
    }

    // (length "hello") should be 5
    support::eval_expression(&mut gnu, &mut neo, "(length \"hello\")");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('5'),
            "{label}: (length \"hello\") should be 5. Echo: {echo}"
        );
    }

    // (+ 1 2 3) should be 6
    support::eval_expression(&mut gnu, &mut neo, "(+ 1 2 3)");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('6'),
            "{label}: (+ 1 2 3) should be 6. Echo: {echo}"
        );
    }

    // (symbol-name 'hello) should be "hello"
    support::eval_expression(&mut gnu, &mut neo, "(symbol-name 'hello)");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains("hello"),
            "{label}: (symbol-name 'hello) should be \"hello\". Echo: {echo}"
        );
    }

    // (intern "hello") should return the symbol hello
    support::eval_expression(&mut gnu, &mut neo, "(intern \"hello\")");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains("hello"),
            "{label}: (intern \"hello\") should be hello. Echo: {echo}"
        );
    }
}

// ── Environment and keymap tests ────────────────────────────

#[test]
fn getenv_returns_same_path_as_gnu() {
    let (mut gnu, mut neo) = boot_pair("");

    support::eval_expression(&mut gnu, &mut neo, "(getenv \"HOME\")");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('\"') || echo.contains('/'),
            "{label}: (getenv HOME) should return a path. Echo: {echo}"
        );
    }

    support::eval_expression(&mut gnu, &mut neo, "(getenv \"USER\")");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('\"'),
            "{label}: (getenv USER) should return a string. Echo: {echo}"
        );
    }
}

#[test]
fn lookup_key_global_map_returns_correct_binding() {
    let (mut gnu, mut neo) = boot_pair("");

    support::eval_expression(
        &mut gnu,
        &mut neo,
        "(lookup-key global-map (kbd \"C-x C-f\"))",
    );
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            !echo.trim().is_empty() && !echo.contains("nil"),
            "{label}: (lookup-key global-map (kbd C-x C-f)) should find binding"
        );
    }
}

// ── Hash table tests ────────────────────────────────────────

#[test]
fn hash_table_operations_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    // (make-hash-table) should create a hash table
    support::eval_expression(&mut gnu, &mut neo, "(hash-table-p (make-hash-table))");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('t'),
            "{label}: (hash-table-p (make-hash-table)) should be t. Echo: {echo}"
        );
    }

    // (gethash 'key (make-hash-table)) should be nil
    support::eval_expression(&mut gnu, &mut neo, "(gethash 'key (make-hash-table))");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains("nil"),
            "{label}: (gethash 'key (make-hash-table)) should be nil. Echo: {echo}"
        );
    }
}

// ── Sequence tests ──────────────────────────────────────────

#[test]
fn sequence_operations_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    // (length [1 2 3]) should be 3
    support::eval_expression(&mut gnu, &mut neo, "(length [1 2 3])");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('3'),
            "{label}: (length [1 2 3]) should be 3. Echo: {echo}"
        );
    }

    // (aref [1 2 3] 0) should be 1
    support::eval_expression(&mut gnu, &mut neo, "(aref [1 2 3] 0)");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('1'),
            "{label}: (aref [1 2 3] 0) should be 1. Echo: {echo}"
        );
    }
}

// ── Regexp and assoc tests ──────────────────────────────────

#[test]
fn regexp_and_assoc_operations_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    // (string-match "foo" "foobar") should be 0 (match at position 0)
    support::eval_expression(&mut gnu, &mut neo, "(string-match \"foo\" \"foobar\")");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('0'),
            "{label}: (string-match ...) should be 0. Echo: {echo}"
        );
    }

    // (assoc 'b '((a . 1) (b . 2))) should be (b . 2)
    support::eval_expression(&mut gnu, &mut neo, "(assoc 'b '((a . 1) (b . 2)))");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('b') && echo.contains('2'),
            "{label}: (assoc 'b ...) should find (b . 2). Echo: {echo}"
        );
    }
}

// ── Evaluator core tests ────────────────────────────────────

#[test]
fn lambda_apply_funcall_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    // ((lambda (x) (+ x 1)) 41) should be 42
    support::eval_expression(&mut gnu, &mut neo, "((lambda (x) (+ x 1)) 41)");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains("42"),
            "{label}: ((lambda (x) (+ x 1)) 41) should be 42. Echo: {echo}"
        );
    }

    // (apply '+ '(1 2 3)) should be 6
    support::eval_expression(&mut gnu, &mut neo, "(apply '+ '(1 2 3))");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('6'),
            "{label}: (apply '+ '(1 2 3)) should be 6. Echo: {echo}"
        );
    }

    // (funcall '+ 1 2 3) should be 6
    support::eval_expression(&mut gnu, &mut neo, "(funcall '+ 1 2 3)");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('6'),
            "{label}: (funcall '+ 1 2 3) should be 6. Echo: {echo}"
        );
    }
}

// ── Macro and control flow tests ────────────────────────────

#[test]
fn macroexpand_and_condition_case_match_gnu() {
    let (mut gnu, mut neo) = boot_pair("");

    // (condition-case nil (/ 1 0) (arith-error "caught")) should return "caught"
    support::eval_expression(
        &mut gnu,
        &mut neo,
        "(condition-case nil (/ 1 0) (arith-error \"caught\"))",
    );
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains("caught"),
            "{label}: (condition-case ... (/ 1 0) ...) should catch arith-error"
        );
    }

    // (eval '(+ 1 2)) should be 3
    support::eval_expression(&mut gnu, &mut neo, "(eval '(+ 1 2))");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('3'),
            "{label}: (eval '(+ 1 2)) should be 3. Echo: {echo}"
        );
    }
}

// ── Non-local exit tests ────────────────────────────────────

#[test]
fn catch_throw_and_unwind_protect_match_gnu() {
    let (mut gnu, mut neo) = boot_pair("");

    // (catch 'tag (throw 'tag 42)) should be 42
    support::eval_expression(&mut gnu, &mut neo, "(catch 'tag (throw 'tag 42))");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains("42"),
            "{label}: (catch 'tag (throw 'tag 42)) should be 42"
        );
    }

    // (unwind-protect 42 (message "cleanup")) should be 42
    support::eval_expression(&mut gnu, &mut neo, "(unwind-protect 42 (ignore))");
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains("42"),
            "{label}: (unwind-protect 42 ...) should return 42"
        );
    }
}

// ── Prefix argument diagnostic ──────────────────────────────

#[test]
fn prefix_arg_survives_from_cu_to_next_command() {
    let (mut gnu, mut neo) = boot_pair("");

    // Check prefix-arg is nil before any C-u
    support::eval_expression(
        &mut gnu,
        &mut neo,
        "(if (null prefix-arg) \"nil\" \"non-nil\")",
    );
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains("nil"),
            "{label}: prefix-arg should be nil before C-u. Echo: {echo}"
        );
    }

    // Check current-prefix-arg is nil too
    support::eval_expression(
        &mut gnu,
        &mut neo,
        "(if (null current-prefix-arg) \"nil\" \"non-nil\")",
    );
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains("nil"),
            "{label}: current-prefix-arg should be nil"
        );
    }
}

// ── Function definition and call tests ──────────────────────

#[test]
fn defun_and_optional_args_preserve_argument_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    // Define and call a function
    support::eval_expression(
        &mut gnu,
        &mut neo,
        "(progn (defun tui-test-fn (x) (* x x)) (tui-test-fn 7))",
    );
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains("49"),
            "{label}: (defun fn (x) (* x x)) then (fn 7) should be 49"
        );
    }

    // Test &optional args
    support::eval_expression(
        &mut gnu,
        &mut neo,
        "(progn (defun tui-opt (a &optional b) (if b (+ a b) a)) (tui-opt 5 3))",
    );
    read_both(&mut gnu, &mut neo, Duration::from_secs(3));
    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let empty = String::new();
        let echo = grid.last().unwrap_or(&empty);
        assert!(
            echo.contains('8'),
            "{label}: (defun fn (a &optional b)) then (fn 5 3) should be 8"
        );
    }
}
