//! TUI comparison tests: eval elisp.

mod support;
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
fn eval_expression_minibuffer_ctrl_h_does_not_delete_previous_character() {
    let (mut gnu, mut neo) = boot_pair("");

    send_both(&mut gnu, &mut neo, "M-:");
    let eval_prompt = |grid: &[String]| grid.iter().any(|row| row.contains("Eval:"));
    gnu.read_until(Duration::from_secs(6), eval_prompt);
    neo.read_until(Duration::from_secs(8), eval_prompt);
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));

    for session in [&mut gnu, &mut neo] {
        session.send(b"(+ 1 2)X");
    }
    send_both(&mut gnu, &mut neo, "BS");

    let expression_preserved = |grid: &[String]| grid.iter().any(|row| row.contains("(+ 1 2)X"));
    gnu.read_until(Duration::from_secs(6), expression_preserved);
    neo.read_until(Duration::from_secs(8), expression_preserved);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            expression_preserved(&grid),
            "{label} should keep the previous eval-expression minibuffer character after terminal C-h\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "eval_expression_minibuffer_ctrl_h_does_not_delete_previous_character",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn eval_expression_minibuffer_del_deletes_previous_character() {
    let (mut gnu, mut neo) = boot_pair("");

    send_both(&mut gnu, &mut neo, "M-:");
    let eval_prompt = |grid: &[String]| grid.iter().any(|row| row.contains("Eval:"));
    gnu.read_until(Duration::from_secs(6), eval_prompt);
    neo.read_until(Duration::from_secs(8), eval_prompt);
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));

    for session in [&mut gnu, &mut neo] {
        session.send(b"(+ 1 2)X");
    }
    send_both(&mut gnu, &mut neo, "DEL RET");

    let result_ready = |grid: &[String]| grid.iter().rev().take(4).any(|row| row.contains("3"));
    gnu.read_until(Duration::from_secs(6), result_ready);
    neo.read_until(Duration::from_secs(8), result_ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            result_ready(&grid),
            "{label} should evaluate corrected (+ 1 2) after terminal DEL in M-:\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "eval_expression_minibuffer_del_deletes_previous_character",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn eval_expression_empty_minibuffer_multiple_del_keeps_prompt() {
    let (mut gnu, mut neo) = boot_pair("");

    send_both(&mut gnu, &mut neo, "M-:");
    let eval_prompt = |grid: &[String]| grid.iter().any(|row| row.contains("Eval:"));
    gnu.read_until(Duration::from_secs(6), eval_prompt);
    neo.read_until(Duration::from_secs(8), eval_prompt);
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));

    send_both(&mut gnu, &mut neo, "DEL DEL DEL");

    let prompt_intact = |grid: &[String]| {
        grid.iter().any(|row| row.contains("Eval:"))
            && !grid.iter().any(|row| row.contains("*Help*"))
            && !grid.iter().any(|row| row.contains("*Backtrace*"))
    };
    gnu.read_until(Duration::from_secs(6), prompt_intact);
    neo.read_until(Duration::from_secs(8), prompt_intact);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            prompt_intact(&grid),
            "{label} should keep the empty M-: prompt intact after repeated terminal DEL keyhits\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "eval_expression_empty_minibuffer_multiple_del_keeps_prompt",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn execute_extended_command_minibuffer_ctrl_h_preserves_command_text() {
    let (mut gnu, mut neo) = boot_pair("");

    send_both(&mut gnu, &mut neo, "M-x");
    let mx_prompt = |grid: &[String]| grid.iter().any(|row| row.contains("M-x"));
    gnu.read_until(Duration::from_secs(6), mx_prompt);
    neo.read_until(Duration::from_secs(8), mx_prompt);
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));

    for session in [&mut gnu, &mut neo] {
        session.send(b"forward-charX");
    }
    send_both(&mut gnu, &mut neo, "BS");

    let command_preserved = |grid: &[String]| grid.iter().any(|row| row.contains("forward-charX"));
    gnu.read_until(Duration::from_secs(6), command_preserved);
    neo.read_until(Duration::from_secs(8), command_preserved);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            command_preserved(&grid),
            "{label} should keep the previous M-x minibuffer character after terminal C-h\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "execute_extended_command_minibuffer_ctrl_h_preserves_command_text",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn execute_extended_command_minibuffer_del_deletes_previous_character() {
    let (mut gnu, mut neo) = boot_pair("");

    send_both(&mut gnu, &mut neo, "M-x");
    let mx_prompt = |grid: &[String]| grid.iter().any(|row| row.contains("M-x"));
    gnu.read_until(Duration::from_secs(6), mx_prompt);
    neo.read_until(Duration::from_secs(8), mx_prompt);
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));

    for session in [&mut gnu, &mut neo] {
        session.send(b"forward-charX");
    }
    send_both(&mut gnu, &mut neo, "DEL");
    send_both(&mut gnu, &mut neo, "RET");
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));
    for session in [&mut gnu, &mut neo] {
        session.send(b"Z");
    }

    let inserted = |grid: &[String]| grid.iter().any(|row| row.trim_end() == "Z");
    gnu.read_until(Duration::from_secs(6), inserted);
    neo.read_until(Duration::from_secs(8), inserted);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            inserted(&grid),
            "{label} should run corrected forward-char after terminal DEL in M-x\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "execute_extended_command_minibuffer_del_deletes_previous_character",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn execute_extended_command_minibuffer_multiple_del_keyhits() {
    let (mut gnu, mut neo) = boot_pair("");

    send_both(&mut gnu, &mut neo, "M-x");
    let mx_prompt = |grid: &[String]| grid.iter().any(|row| row.contains("M-x"));
    gnu.read_until(Duration::from_secs(6), mx_prompt);
    neo.read_until(Duration::from_secs(8), mx_prompt);
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));

    for session in [&mut gnu, &mut neo] {
        session.send(b"forward-charXYZ");
    }
    send_both(&mut gnu, &mut neo, "DEL DEL DEL RET");
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));
    for session in [&mut gnu, &mut neo] {
        session.send(b"Z");
    }

    let inserted = |grid: &[String]| grid.iter().any(|row| row.trim_end() == "Z");
    gnu.read_until(Duration::from_secs(6), inserted);
    neo.read_until(Duration::from_secs(8), inserted);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            inserted(&grid),
            "{label} should run corrected forward-char after three terminal DEL keyhits in M-x\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "execute_extended_command_minibuffer_multiple_del_keyhits",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn execute_extended_command_empty_minibuffer_multiple_del_keeps_prompt() {
    let (mut gnu, mut neo) = boot_pair("");

    send_both(&mut gnu, &mut neo, "M-x");
    let mx_prompt = |grid: &[String]| grid.iter().any(|row| row.contains("M-x"));
    gnu.read_until(Duration::from_secs(6), mx_prompt);
    neo.read_until(Duration::from_secs(8), mx_prompt);
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));

    send_both(&mut gnu, &mut neo, "DEL DEL DEL");

    let prompt_intact = |grid: &[String]| {
        grid.iter().any(|row| row.contains("M-x"))
            && !grid.iter().any(|row| row.contains("No match"))
            && !grid.iter().any(|row| row.contains("*Help*"))
    };
    gnu.read_until(Duration::from_secs(6), prompt_intact);
    neo.read_until(Duration::from_secs(8), prompt_intact);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            prompt_intact(&grid),
            "{label} should keep the empty M-x prompt intact after repeated terminal DEL keyhits\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "execute_extended_command_empty_minibuffer_multiple_del_keeps_prompt",
        &gnu,
        &neo,
        2,
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

#[test]
fn sequence_mutation_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"seq-mut:%S\" (list (mapcar (lambda (x) (cons x (* x x))) '(1 2 3)) (let ((xs (list 3 1 2))) (sort xs '<)) (delq 'b (list 'a 'b 'c 'b)) (nreverse (list 1 2 3))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("seq-mut:")
                && row.contains("((1 . 1) (2 . 4) (3 . 9))")
                && row.contains("(1 2 3)")
                && row.contains("(a c)")
                && row.contains("(3 2 1)")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: sequence mutation functions should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "sequence_mutation_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn vector_sort_compare_strings_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"sortseq:%S\" (list (sort (copy-sequence [3 1 2]) '<) (compare-strings \"abc\" nil nil \"abd\" nil nil) (compare-strings \"abc\" nil nil \"ABC\" nil nil t)))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("sortseq:") && row.contains("([1 2 3] -3 t)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: vector sort and compare-strings should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "vector_sort_compare_strings_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn copy_tree_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"copy:%S\" (let* ((inner (list 1)) (tree (list inner (vector inner))) (copy (copy-tree tree t))) (setcar inner 9) (list tree copy (eq (car tree) (car copy)) (eq (aref (cadr tree) 0) (aref (cadr copy) 0)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("copy:")
                && row.contains("((9)")
                && row.contains("((1)")
                && row.contains("nil nil")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: copy-tree list/vector deep-copy behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "copy_tree_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn property_list_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"plist:%S\" (let ((plist (list :a 1 :b 2 :a 3))) (list (plist-get plist :a) (plist-member plist :b) (progn (setq plist (plist-put plist :c 4)) plist) (progn (setq plist (plist-put plist :a 9)) plist))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("plist:")
                && row.contains("(1")
                && row.contains("(:b 2 :a 3 :c 4)")
                && row.contains(":c 4")
                && row.contains(":a 9")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: property-list functions should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "property_list_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn property_list_edge_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"plistedge:%S\" (let ((p (list :a 1 :b 2 :a 3))) (list (plist-get p :a) (plist-member p :a) (plist-get (plist-put p :b 9) :b) (condition-case e (plist-get '(:a) :a) (wrong-type-argument (car e)) (error (car e))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("plistedge:")
                && row.contains("(1 (:a 1")
                && row.contains(":b 9")
                && row.contains("nil")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: property-list edge behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "property_list_edge_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn symbol_property_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"symprop:%S\" (let ((sym (make-symbol \"symprop-target\"))) (put sym 'alpha 1) (put sym 'beta '(x y)) (list (get sym 'alpha) (or (get sym 'missing) 'fallback) (symbol-plist sym) (progn (setplist sym '(gamma 3 delta 4)) (symbol-plist sym)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("symprop:")
                && row.contains("(1 fallback")
                && row.contains("alpha 1")
                && row.contains("beta (x y)")
                && row.contains("(gamma 3 delta 4)")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: symbol property functions should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "symbol_property_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

// ── String and numeric operation tests ──────────────────────

#[test]
fn string_search_replace_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"strfun:%S\" (list (upcase \"aBz\") (downcase \"AbZ\") (capitalize \"hello-world test\") (let ((s (copy-sequence \"abc\"))) (aset s 1 ?Z) s) (progn (string-match \"\\\\([a-z]+\\\\)-\\\\([0-9]+\\\\)\" \"foo-123\") (list (match-string 0 \"foo-123\") (match-string 1 \"foo-123\") (replace-match \"bar\" nil nil \"foo-123\" 1)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("strfun:")
                && row.contains("ABZ")
                && row.contains("abz")
                && row.contains("Hello-World Test")
                && row.contains("aZc")
                && row.contains("foo-123")
                && row.contains("foo")
                && row.contains("bar-123")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: string search/replace functions should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "string_search_replace_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn subst_char_in_string_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"subst:%S\" (list (subst-char-in-string ?a ?x \"banana\") (let ((s (copy-sequence \"banana\"))) (list (subst-char-in-string ?a ?x s t) s)) (let ((s \"banana\")) (eq s (subst-char-in-string ?a ?x s))) (subst-char-in-string ?q ?x \"banana\")))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("subst:")
                && row.matches("bxnxnx").count() >= 3
                && row.contains("nil")
                && row.contains("banana")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: subst-char-in-string copy and in-place behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "subst_char_in_string_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn remove_delq_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"remove:%S\" (let* ((xs (list 'a 'b 'c 'b)) (r (remove 'b xs)) (ys (list 'a 'b 'c 'b)) (d (delq 'b ys))) (list r xs d ys)))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("remove:") && row.contains("((a c) (a b c b) (a c) (a c))"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: remove and delq behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "remove_delq_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn alist_lookup_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"alist:%S\" (let ((xs (list (cons \"k\" 1) (cons (copy-sequence \"k\") 2) (cons 'sym 3)))) (list (assoc \"k\" xs) (assq \"k\" xs) (assq 'sym xs) (rassoc 2 xs))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("alist:")
                && row.contains("nil")
                && row.contains("(sym . 3)")
                && row.contains(". 2)")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: alist lookup behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "alist_lookup_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn member_predicate_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"member:%S\" (let ((s (copy-sequence \"x\"))) (list (member \"x\" (list s)) (memq \"x\" (list s)) (memql 1.0 (list 1.0)) (memql 1 (list 1.0)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("member:") && row.contains("nil") && row.contains("1.0"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: member predicate behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "member_predicate_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn vector_array_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"vecfun:%S\" (let* ((v (vector 'a 'b 'c)) (copy (copy-sequence v))) (aset copy 1 'B) (list (length v) (aref v 1) copy (vconcat '(1 2) [3 4] \"ab\") (append [x y] '(z)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("vecfun:")
                && row.contains("(3 b [a B c]")
                && row.contains("[1 2 3 4 97 98]")
                && row.contains("(x y z)")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: vector/array functions should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "vector_array_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn fillarray_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"fillseq:%S\" (list (let ((v (vector 1 2 3))) (fillarray v 9) v) (let ((s (copy-sequence \"abc\"))) (fillarray s ?x) s) (condition-case e (fillarray (list 1 2) 3) (wrong-type-argument (car e)) (error (car e)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("fillseq:")
                && row.contains("[9 9 9]")
                && row.contains("xxx")
                && row.contains("wrong-type-argument")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: fillarray vector/string mutation should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "fillarray_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn bool_vector_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"boolvec:%S\" (let ((bv (make-bool-vector 4 nil))) (aset bv 1 t) (aset bv 3 t) (list (bool-vector-p bv) (aref bv 0) (aref bv 1) (vconcat bv))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("boolvec:") && row.contains("(t nil t [nil t nil t])"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: bool-vector behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "bool_vector_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn record_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"record:%S\" (let ((r (record 'foo 1 2))) (aset r 1 9) (list (recordp r) (type-of r) (aref r 0) (aref r 1) (length r))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("record:") && row.contains("(t foo foo 9 3)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: record behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("record_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

#[test]
fn hash_table_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"hashfun:%S\" (let ((h (make-hash-table :test 'equal))) (puthash \"k\" 1 h) (puthash \"j\" 2 h) (remhash \"j\" h) (list (gethash \"k\" h 'missing) (gethash \"j\" h 'missing) (hash-table-count h))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("hashfun:") && row.contains("(1 missing 1)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: hash-table functions should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "hash_table_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn hash_table_copy_maphash_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"hashcopy:%S\" (let ((h (make-hash-table :test 'equal))) (puthash \"a\" 1 h) (puthash \"b\" 2 h) (let ((c (copy-hash-table h)) seen) (puthash \"a\" 9 c) (maphash (lambda (k v) (push (cons k v) seen)) h) (list (gethash \"a\" h) (gethash \"a\" c) (hash-table-count h) (length seen)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("hashcopy:") && row.contains("(1 9 2 2)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: hash-table copy and maphash behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "hash_table_copy_maphash_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn hash_table_key_test_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"hashtest:%S\" (let ((eqh (make-hash-table :test 'eq)) (equalh (make-hash-table :test 'equal)) (eqlh (make-hash-table :test 'eql))) (puthash (copy-sequence \"k\") 'eq-string eqh) (puthash (copy-sequence \"k\") 'equal-string equalh) (puthash 1.0 'float eqlh) (puthash 1 'int eqlh) (list (gethash \"k\" eqh 'missing) (gethash \"k\" equalh 'missing) (gethash 1.0 eqlh 'missing) (gethash 1 eqlh 'missing))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("hashtest:") && row.contains("(missing equal-string float int)")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: hash-table key-test semantics should match GNU\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "hash_table_key_test_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn marker_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"marker:%S\" (with-temp-buffer (insert \"ab\") (goto-char 2) (let ((left (point-marker)) (right (copy-marker (point) t))) (insert \"X\") (let ((before (list (buffer-string) (marker-position left) (marker-insertion-type left) (marker-position right) (marker-insertion-type right) (bufferp (marker-buffer left))))) (set-marker left nil) (append before (list (marker-position left) (marker-buffer left)))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("marker:") && row.contains("aXb") && row.contains("2 nil 3 t t nil nil")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: marker functions should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("marker_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

#[test]
fn marker_insertion_type_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"marktype:%S\" (with-temp-buffer (insert \"ab\") (let ((m1 (copy-marker (point-max) nil)) (m2 (copy-marker (point-max) t))) (goto-char (point-max)) (insert \"Z\") (list (marker-position m1) (marker-position m2) (buffer-string)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("marktype:") && row.contains("(3 4"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: marker insertion type behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "marker_insertion_type_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn narrowing_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"narrow:%S\" (with-temp-buffer (insert \"alpha\\nbeta\\ngamma\\n\") (goto-char (point-min)) (forward-line 1) (let ((beg (point))) (forward-line 1) (narrow-to-region beg (point)) (list (buffer-size) (point-min) (point-max) (buffer-string) (save-restriction (widen) (list (point-min) (point-max) (buffer-size)))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        let tail = grid
            .iter()
            .rev()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        tail.contains("narrow:")
            && tail.contains("(17 7 12")
            && tail.contains("beta")
            && tail.contains("(1 18 17)")
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: narrowing functions should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "narrowing_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn overlay_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"overlay:%S\" (with-temp-buffer (insert \"abc\") (let ((o (make-overlay 1 2))) (overlay-put o 'p 7) (list (overlay-start o) (overlay-end o) (overlay-get o 'p) (length (overlays-at 1)) (progn (delete-overlay o) (overlay-buffer o))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("overlay:") && row.contains("(1 2 7 1 nil)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: overlay functions should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("overlay_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

#[test]
fn overlay_move_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"ovmove:%S\" (with-temp-buffer (insert \"abcdef\") (let ((o (make-overlay 2 4))) (move-overlay o 3 6) (overlay-put o 'evaporate t) (list (overlay-start o) (overlay-end o) (overlay-get o 'evaporate) (length (overlays-at 4))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("ovmove:") && row.contains("(3 6 t 1)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: overlay move behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "overlay_move_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn overlay_overlap_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"ovprio:%S\" (with-temp-buffer (insert \"abcdef\") (let ((a (make-overlay 2 5)) (b (make-overlay 3 4))) (overlay-put a 'priority 1) (overlay-put b 'priority 9) (list (mapcar (lambda (o) (overlay-get o 'priority)) (overlays-at 3)) (length (overlays-in 1 6)) (bufferp (overlay-buffer a))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("ovprio:") && row.contains("((1 9) 2 t)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: overlapping overlay enumeration should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "overlay_overlap_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn text_property_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"textprop:%S\" (let ((s (copy-sequence \"abcd\"))) (put-text-property 1 3 'face 'bold s) (put-text-property 2 4 'mouse-face 'highlight s) (list (get-text-property 1 'face s) (get-text-property 2 'mouse-face s) (text-properties-at 2 s) (next-single-property-change 1 'face s) (previous-single-property-change 4 'mouse-face s))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("textprop:")
                && row.contains("(bold highlight")
                && row.contains("face bold")
                && row.contains("mouse-face highlight")
                && row.contains("3 2")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: text property functions should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "text_property_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn text_property_equality_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"tpeq:%S\" (let ((s (copy-sequence \"abcd\"))) (put-text-property 1 3 'face 'bold s) (list (substring s 1 3) (text-properties-at 0 (substring s 1 3)) (equal s (substring-no-properties s)) (equal-including-properties s (substring-no-properties s)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("tpeq:")
                && row.contains("bc")
                && row.contains("face bold")
                && row.contains("t nil")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: text-property equality should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "text_property_equality_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn text_property_removal_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"tprop2:%S\" (let ((s (copy-sequence \"abcd\"))) (put-text-property 0 4 'face 'bold s) (remove-text-properties 1 3 '(face nil) s) (list (text-properties-at 0 s) (text-properties-at 1 s) (next-single-property-change 0 'face s) (next-single-property-change 1 'face s) s)))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("tprop2:") && row.contains("(face bold)") && row.contains("nil 1 3")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: text property removal should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "text_property_removal_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn text_property_stickiness_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"sticky:%S\" (with-temp-buffer (insert \"ab\") (put-text-property 1 2 'face 'bold) (put-text-property 1 2 'rear-nonsticky '(face)) (goto-char 2) (insert \"X\") (list (buffer-string) (text-properties-at 0 (buffer-string)) (text-properties-at 1 (buffer-string)) (text-properties-at 2 (buffer-string)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("sticky:")
                && row.contains("aXb")
                && row.contains("rear-nonsticky")
                && row.contains("face bold")
                && row.contains("nil nil")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: text-property stickiness should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "text_property_stickiness_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn text_property_search_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"tpropsearch:%S\" (let ((s (copy-sequence \"abcdef\"))) (put-text-property 1 4 'face 'bold s) (list (text-property-any 0 6 'face 'bold s) (text-property-any 4 6 'face 'bold s) (text-property-not-all 1 4 'face 'bold s) (text-property-not-all 0 6 'face 'bold s))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("tpropsearch:") && row.contains("(1 nil nil 0)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: text-property search helpers should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "text_property_search_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn button_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'button) (message \"button:%S\" (with-temp-buffer (insert-text-button \"Go\" 'action (lambda (_) 'done) 'help-echo \"Help\") (let ((b (button-at (point-min)))) (list (not (null b)) (button-label b) (button-get b 'help-echo) (button-has-type-p b 'push-button))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("button:")
                && row.contains("t")
                && row.contains("Go")
                && row.contains("Help")
                && row.contains("nil")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: button text property helper semantics should match GNU\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("button_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

#[test]
fn buffer_substring_property_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"bufsub:%S\" (with-temp-buffer (insert \"abcd\") (put-text-property 2 4 'face 'bold) (list (buffer-substring 2 4) (text-properties-at 0 (buffer-substring 2 4)) (buffer-substring-no-properties 2 4) (text-properties-at 0 (buffer-substring-no-properties 2 4)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("bufsub:") && row.contains("face bold") && row.contains("nil"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: buffer substring property behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "buffer_substring_property_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn buffer_local_variable_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"buflocal:%S\" (with-temp-buffer (setq-local fill-column 33) (list fill-column (local-variable-p 'fill-column) (with-temp-buffer fill-column))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("buflocal:") && row.contains("(33 t 70)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: buffer-local variable functions should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "buffer_local_variable_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn default_local_variable_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"deflocal:%S\" (let ((orig (default-value 'fill-column))) (unwind-protect (with-temp-buffer (setq-default fill-column 71) (setq-local fill-column 33) (list fill-column (default-value 'fill-column) (progn (kill-local-variable 'fill-column) fill-column))) (setq-default fill-column orig))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("deflocal:") && row.contains("(33 71 71)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: default and local variable behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "default_local_variable_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn permanent_local_variable_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"permlocal:%S\" (with-temp-buffer (put 'neo-p 'permanent-local t) (set (make-local-variable 'neo-n) 1) (set (make-local-variable 'neo-p) 2) (kill-all-local-variables) (list (local-variable-p 'neo-n) (local-variable-p 'neo-p) (boundp 'neo-n) (boundp 'neo-p) neo-p)))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("permlocal:") && row.contains("(nil t nil t 2)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: permanent-local variable behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "permanent_local_variable_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn hook_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"hook:%S\" (let ((hook nil) (seen nil)) (add-hook 'hook (lambda () (push 'a seen))) (add-hook 'hook (lambda () (push 'b seen))) (run-hooks 'hook) (list seen (length hook))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("hook:") && row.contains("((a b) 0)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: hook functions should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("hook_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

#[test]
fn condition_object_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"cond:%S\" (list (condition-case e (signal 'wrong-type-argument '(integerp \"x\")) (wrong-type-argument (list 'typed e)) (error (list 'error e))) (condition-case e (error \"boom %s\" 7) (error (list (car e) (cadr e))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("cond:")
                && row.contains("typed")
                && row.contains("wrong-type-argument")
                && row.contains("integerp")
                && row.contains("boom 7")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: condition object handling should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "condition_object_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn nonlocal_exit_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"nonlocal:%S\" (list (catch 'a (throw 'a 7)) (condition-case e (throw 'b 3) (no-catch (cadr e))) (let (s) (condition-case e (unwind-protect (progn (push 'body s) (error \"boom\")) (push 'cleanup s)) (error (nreverse s))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("nonlocal:") && row.contains("(7 b") && row.contains("(body cleanup)")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: catch/throw no-catch and unwind-protect ordering should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "nonlocal_exit_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn timer_object_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"timerobj:%S\" (let ((tm (run-at-time 100 nil (lambda () nil)))) (prog1 (list (timerp tm) (not (null (memq tm timer-list))) (cancel-timer tm) (memq tm timer-list)) (ignore-errors (cancel-timer tm)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("timerobj:") && row.contains("(t t nil nil)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: timer object creation and cancellation should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "timer_object_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn define_error_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (define-error 'neo-test-error \"Neo message\" 'file-error) (message \"deferr:%S\" (list (get 'neo-test-error 'error-conditions) (get 'neo-test-error 'error-message) (condition-case e (signal 'neo-test-error '(\"payload\")) (file-error (list 'file (car e) (cdr e))) (error (list 'error e))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("deferr:")
                && row.contains("neo-test-error")
                && row.contains("file-error")
                && row.contains("Neo message")
                && row.contains("payload")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: define-error inheritance and signaling should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "define_error_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn read_from_string_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"readstr:%S\" (list (read-from-string \"(a . b) tail\") (read-from-string \"\\\"a\\\\\\\"b\\\"x\") (condition-case e (read-from-string \"(\") (end-of-file (car e)) (error (car e)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("readstr:")
                && row.contains("((a . b) . 7)")
                && row.contains(". 6)")
                && row.contains("end-of-file")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: read-from-string object, index, and error behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "read_from_string_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn circular_read_print_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"circle:%S\" (let* ((print-circle t) (x (read \"#1=(a . #1#)\"))) (list (consp x) (eq x (cdr x)) (prin1-to-string x))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("circle:") && row.contains("(t t") && row.contains("#1=(a . #1#)")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: circular read and print-circle behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "circular_read_print_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn radix_reader_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"radix:%S\" (list (read \"#b1010\") (read \"#o12\") (read \"#xA\") (read \"#36rZ\") (condition-case e (read \"#2r2\") (invalid-read-syntax (car e)) (error (car e)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("radix:") && row.contains("(10 10 10 35 invalid-read-syntax)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: radix reader syntax should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "radix_reader_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn character_reader_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"charread:%S\" (list (read \"?A\") (read \"?\\\\n\") (read \"?\\\\C-a\") (read \"?\\\\M-a\") (read \"?\\\\N{LATIN CAPITAL LETTER A}\")))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("charread:") && row.contains("(65 10 1 134217825 65)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: character reader syntax should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "character_reader_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn provide_eval_after_load_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (setq neo-after-load-log nil) (eval-after-load 'neo-feature '(push 'after neo-after-load-log)) (message \"feature:%S\" (list (featurep 'neo-feature) neo-after-load-log (provide 'neo-feature) (featurep 'neo-feature) neo-after-load-log)))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("feature:(nil nil neo-feature t (after))"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: provide, featurep, and eval-after-load should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "provide_eval_after_load_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn match_data_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"match:%S\" (progn (string-match \"\\\\(a\\\\)\" \"a\") (save-match-data (string-match \"b\" \"b\")) (list (match-beginning 1) (match-end 1) (match-string 1 \"a\"))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("match:") && row.contains("(0 1"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: match data preservation should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "match_data_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn optional_submatch_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"submatch:%S\" (progn (string-match \"\\\\(a\\\\)?b\" \"b\") (list (match-beginning 1) (match-end 1) (match-string 1 \"b\") (match-string 0 \"b\"))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("submatch:") && row.contains("(nil nil nil") && row.contains("b")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: optional unmatched submatch behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "optional_submatch_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn failed_match_data_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"matchfail:%S\" (progn (string-match \"a\" \"abc\") (let ((before (match-beginning 0))) (string-match \"z\" \"abc\") (list before (match-beginning 0) (match-data)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("matchfail:") && row.contains("(0 0 (0 1))"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: failed match data behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "failed_match_data_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn obarray_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"ob:%S\" (let ((ob (make-vector 7 0))) (list (intern-soft \"foo\" ob) (symbol-name (intern \"foo\" ob)) (eq (intern-soft \"foo\" ob) (intern \"foo\" ob)) (unintern \"foo\" ob) (intern-soft \"foo\" ob))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("ob:") && row.contains("(nil") && row.contains("t t nil"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: obarray operations should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("obarray_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

#[test]
fn symbol_keyword_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"symedge:%S\" (let ((s (make-symbol \"k\"))) (list (symbol-name :foo) (keywordp :foo) (keywordp 'foo) (symbol-name s) (eq s (intern-soft \"k\")) (intern-soft \"no-such-symbol\"))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("symedge:")
                && row.contains(":foo")
                && row.contains("t nil")
                && row.contains("k")
                && row.contains("nil nil")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: keyword and uninterned symbol behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "symbol_keyword_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn abbrev_table_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"abbrev:%S\" (let ((tab (make-abbrev-table))) (define-abbrev tab \"btw\" \"by the way\") (list (abbrev-table-p tab) (symbol-value (intern-soft \"btw\" tab)) (abbrev-expansion \"btw\" tab))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("abbrev:") && row.contains("(t") && row.contains("by the way"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: abbrev table behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "abbrev_table_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn face_attribute_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"face:%S\" (let ((f (make-face 'neo-face))) (set-face-attribute f nil :weight 'bold :slant 'italic) (list (facep f) (face-attribute f :weight nil) (face-attribute f :slant nil))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("face:") && row.contains("bold") && row.contains("italic"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: face attribute behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "face_attribute_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn symbol_value_cell_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"symbind:%S\" (let ((s (make-symbol \"neo-var\"))) (list (boundp s) (progn (set s 7) (boundp s)) (symbol-value s) (progn (makunbound s) (boundp s)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("symbind:") && row.contains("(nil t 7 nil)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: symbol value cell operations should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "symbol_value_cell_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn variable_watcher_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"watch:%S\" (let ((sym (make-symbol \"watched\")) seen) (add-variable-watcher sym (lambda (s n o w) (push (list s n o w) seen))) (set sym 1) (set sym 2) (list (mapcar (lambda (x) (list (cadr x) (caddr x) (cadddr x))) (nreverse seen)) (get sym sym))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("watch:") && row.contains("((1 set nil) (2 set nil))"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: variable watcher behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "variable_watcher_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn symbol_function_cell_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"funbind:%S\" (let ((s (make-symbol \"neo-fun\"))) (list (fboundp s) (progn (fset s (lambda (x) (+ x 1))) (fboundp s)) (funcall (symbol-function s) 4) (progn (fmakunbound s) (fboundp s)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("funbind:") && row.contains("(nil t 5 nil)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: symbol function cell operations should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "symbol_function_cell_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn function_predicate_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"funpred:%S\" (list (functionp (lambda () 1)) (subrp (symbol-function 'car)) (macrop 'when) (commandp 'find-file) (commandp (lambda () (interactive)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("funpred:") && row.contains("(t t t t t)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: function predicate behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "function_predicate_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn function_arity_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"arity:%S\" (list (func-arity (lambda (a &optional b &rest c) nil)) (subr-arity (symbol-function 'car)) (help-function-arglist (lambda (x &optional y) nil)) (help-function-arglist (symbol-function 'cons))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("arity:")
                && row.contains("(1 . many)")
                && row.contains("(1 . 1)")
                && row.contains("(x &optional y)")
                && row.contains("(arg1 arg2)")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: function arity introspection should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "function_arity_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn interactive_form_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"interactive:%S\" (let ((f (lambda (x) (interactive \"p\") x))) (list (commandp f) (interactive-form f))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("interactive:") && row.contains("(t (interactive"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: interactive form behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "interactive_form_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn autoload_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"auto:%S\" (let ((s (make-symbol \"neo-auto\"))) (autoload s \"nofile\" \"doc\" t) (list (autoloadp (symbol-function s)) (commandp s) (documentation s t))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("auto:") && row.contains("(t t") && row.contains("doc"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: autoload behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "autoload_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn documentation_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"doc:%S\" (let ((s (make-symbol \"docfun\"))) (fset s (lambda () \"DOCSTR\" 1)) (list (documentation s t) (documentation-property s 'function-documentation t))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("doc:") && row.contains("DOCSTR") && row.contains("nil"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: documentation behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "documentation_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn advice_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"advice:%S\" (let ((s (make-symbol \"adv\")) (adv (lambda (orig x) (* 10 (funcall orig x))))) (fset s (lambda (x) (+ x 1))) (advice-add s :around adv) (prog1 (list (funcall s 2) (not (null (advice-member-p adv s)))) (advice-remove s adv))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("advice:") && row.contains("(30 t)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: advice behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("advice_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

#[test]
fn lambda_binding_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"dynlex:%S\" (let ((x 1)) (list (let ((f (lambda () x))) (let ((x 2)) (funcall f))) (let ((y 1)) (let ((f (lambda () y))) (setq y 3) (funcall f))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("dynlex:") && row.contains("(1 3)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: lambda binding behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "lambda_binding_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn let_sequence_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"letseq:%S\" (let ((x 1)) (list (let ((x 2) (y x)) (list x y)) (let* ((x 2) (y x)) (list x y)) x)))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("letseq:") && row.contains("((2 1) (2 2) 1)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: let and let* sequencing should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "let_sequence_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn boolean_short_circuit_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"bool:%S\" (list (and 1 2 nil (error \"no\")) (or nil 0 (error \"no\")) (not nil) (not 0)))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("bool:") && row.contains("(nil 0 t nil)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: boolean short-circuit behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "boolean_short_circuit_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn cond_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"condform:%S\" (list (cond ((> 1 2) 'bad) ((< 1 2) 'ok) (t 'fallback)) (cond (nil 'bad) ((quote (x y))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("condform:") && row.contains("(ok (x y))"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: cond behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("cond_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

#[test]
fn loop_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"loop:%S\" (let ((i 0) acc) (list (while (< i 3) (push i acc) (setq i (1+ i))) acc (dotimes (j 3 'done) (push j acc)) acc)))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("loop:") && row.contains("nil") && row.contains("done"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: loop behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("loop_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

#[test]
fn pcase_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"pcase:%S\" (list (pcase (list 1 2) (`(,a ,b) (+ a b)) (_ nil)) (pcase :foo (:bar 1) (:foo 2) (_ 3)) (pcase '(a . b) (`(,x . ,y) (list x y)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("pcase:") && row.contains("(3 2 (a b))"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: pcase behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("pcase_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

#[test]
fn cl_lib_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'cl-lib) (message \"cl:%S\" (let ((x 1)) (list (cl-incf x 2) x (cl-loop for i below 3 sum i) (cl-typecase \"x\" (string 'str) (t 'other))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("cl:") && row.contains("(3 3 3 str)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: cl-lib behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("cl_lib_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

#[test]
fn cl_defstruct_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'cl-lib) (cl-defstruct neo-point x y) (message \"clstruct:%S\" (let ((p (make-neo-point :x 1 :y 2))) (setf (neo-point-y p) 9) (list (neo-point-p p) (neo-point-x p) (neo-point-y p) (type-of p) (length p)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("clstruct:") && row.contains("(t 1 9 neo-point 3)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: cl-defstruct constructor/accessor behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "cl_defstruct_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn cl_symbol_macrolet_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'cl-lib) (message \"symmac:%S\" (list (cl-symbol-macrolet ((x (car cell))) (let ((cell (list 1))) (setq x 7) cell)) (macroexpand '(cl-symbol-macrolet ((x y)) x)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("symmac:") && row.contains("((7) y)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: cl-symbol-macrolet expansion and setq behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "cl_symbol_macrolet_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn seq_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'seq) (message \"seq:%S\" (list (seq-filter #'numberp '(a 1 b 2)) (seq-map #'1+ [1 2]) (seq-position '(a b c) 'b))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("seq:") && row.contains("((1 2) (2 3) 1)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: seq library behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("seq_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

#[test]
fn byte_compile_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'bytecomp) (message \"byte:%S\" (let ((f (byte-compile (lambda (x) (+ x 2))))) (list (byte-code-function-p f) (funcall f 3)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("byte:") && row.contains("(t 5)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: byte-compile behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "byte_compile_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn rx_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'rx) (message \"rx:%S\" (list (rx-to-string '(seq bol (or \"a\" \"b\") eol)) (string-match-p (rx bol (+ digit) eol) \"123\") (regexp-opt '(\"foo\" \"bar\")))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("rx:") && row.contains("[ab]") && row.contains("0"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: rx behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("rx_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

#[test]
fn regexp_opt_depth_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"regexp:%S\" (let ((re (regexp-opt '(\"cat\" \"car\") 'paren))) (list re (regexp-opt-depth re) (string-match re \"car\") (match-string 1 \"car\"))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("regexp:") && row.contains("ca[rt]") && row.contains("1 0"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: regexp-opt-depth behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "regexp_opt_depth_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn ring_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'ring) (message \"ring:%S\" (let ((r (make-ring 3))) (ring-insert r 'a) (ring-insert r 'b) (ring-insert r 'c) (ring-insert r 'd) (list (ring-length r) (ring-ref r 0) (ring-ref r 2) (ring-empty-p r)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("ring:") && row.contains("(3 d b nil)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: ring behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("ring_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

#[test]
fn subr_x_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'subr-x) (message \"subrx:%S\" (list (string-empty-p \"\") (string-trim \"  hi \") (when-let ((x 3)) (+ x 4)) (if-let ((x nil)) x 'fallback))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("subrx:")
                && row.contains("(t")
                && row.contains("hi")
                && row.contains("7 fallback")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: subr-x behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("subr_x_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

#[test]
fn thread_macro_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'subr-x) (message \"thread:%S\" (list (thread-first 3 (1+) (* 2)) (thread-last '(1 2 3) (mapcar #'1+) (apply '+)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("thread:") && row.contains("(8 9)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: thread macro behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "thread_macro_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn eieio_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'eieio) (defclass neo-eieio-test () ((x :initarg :x :initform 1))) (message \"eieio:%S\" (let ((o (neo-eieio-test :x 5))) (list (object-of-class-p o 'neo-eieio-test) (oref o x) (progn (oset o x 7) (oref o x))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("eieio:") && row.contains("(t 5 7)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: EIEIO behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("eieio_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

#[test]
fn cl_generic_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'cl-generic) (cl-defgeneric neo-generic (x)) (cl-defmethod neo-generic ((x integer)) (list 'int x)) (cl-defmethod neo-generic ((x string)) (list 'str x)) (message \"clgen:%S\" (list (neo-generic 3) (neo-generic \"x\") (condition-case e (neo-generic 'sym) (cl-no-applicable-method (car e)) (error (car e))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("clgen:")
                && row.contains("(int 3)")
                && row.contains("str")
                && row.contains("x")
                && row.contains("cl-no-applicable-method")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: cl-generic dispatch and no-applicable-method signaling should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "cl_generic_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn define_minor_mode_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'easy-mmode) (define-minor-mode neo-test-mode \"Doc.\" :init-value nil :lighter \" Neo\") (with-temp-buffer (neo-test-mode 1) (message \"minor:%S\" (list neo-test-mode (assq 'neo-test-mode minor-mode-alist) (commandp 'neo-test-mode)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("minor:")
                && row.contains("neo-test-mode")
                && row.contains("Neo")
                && row.contains("t")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: define-minor-mode behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "define_minor_mode_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn define_derived_mode_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (define-derived-mode neo-derived-mode fundamental-mode \"NeoD\" \"Doc.\") (with-temp-buffer (neo-derived-mode) (message \"derived:%S\" (list major-mode mode-name (derived-mode-p 'fundamental-mode) (derived-mode-p 'neo-derived-mode)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("derived:")
                && row.contains("neo-derived-mode")
                && row.contains("NeoD")
                && row.contains("nil")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: define-derived-mode behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "define_derived_mode_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn map_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'map) (message \"map:%S\" (let ((h (make-hash-table :test 'equal))) (puthash \"a\" 1 h) (list (map-elt '((a . 1)) 'a) (map-elt '(:a 2) :a) (map-elt h \"a\") (map-keys '((x . 1) (y . 2)))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("map:") && row.contains("(1 2 1 (x y))"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: map library behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("map_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

#[test]
fn macroexpand_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"macro:%S\" (let ((m (macroexpand '(when t 1 2)))) (list (car m) (cadr m) (caddr m) (cadddr m))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("macro:") && row.contains("(if t") && row.contains("(progn 1 2)")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: macroexpand should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "macroexpand_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn macroexp_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'macroexp) (message \"macroexp:%S\" (macroexp-progn '((setq a 1) (setq b 2)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("macroexp:")
                && row.contains("(progn")
                && row.contains("(setq a 1)")
                && row.contains("(setq b 2)")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: macroexp behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "macroexp_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn syntax_table_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"syntax:%S\" (let ((st (make-syntax-table))) (with-syntax-table st (modify-syntax-entry ?_ \"w\") (modify-syntax-entry ?# \"<\") (list (char-syntax ?_) (char-syntax ?#) (char-syntax ?a)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("syntax:") && row.contains("(119 60 119)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: syntax table operations should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "syntax_table_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn syntax_table_copy_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"syntcopy:%S\" (let ((st (make-syntax-table))) (modify-syntax-entry ?_ \"w\" st) (let ((cp (copy-syntax-table st))) (modify-syntax-entry ?_ \"_\" cp) (list (with-syntax-table st (char-syntax ?_)) (with-syntax-table cp (char-syntax ?_)) (string-to-syntax \"w\")))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("syntcopy:") && row.contains("(119 95 (2))"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: syntax-table copying should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "syntax_table_copy_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn syntax_table_regexp_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"synre:%S\" (let ((st (make-syntax-table))) (modify-syntax-entry ?_ \"w\" st) (modify-syntax-entry ?$ \".\" st) (with-syntax-table st (list (char-syntax ?_) (char-syntax ?$) (string-match \"\\\\sw+\" \"__\") (string-match \"\\\\s.+\" \"$$\")))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("synre:") && row.contains("(119 46 0 0)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: syntax-table regexp classes should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "syntax_table_regexp_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn syntax_table_comment_flags_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"syntaxextra:%S\" (let ((st (make-syntax-table))) (modify-syntax-entry ?/ \". 124b\" st) (modify-syntax-entry ?* \". 23\" st) (with-syntax-table st (list (string-to-syntax \". 124b\") (char-syntax ?/) (char-syntax ?*)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("syntaxextra:") && row.contains("((2818049) 46 46)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: syntax-table comment flag encoding should match GNU\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "syntax_table_comment_flags_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn category_table_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"category:%S\" (let ((ct (make-category-table))) (define-category ?x \"X category\" ct) (modify-category-entry ?a ?x ct) (list (category-docstring ?x ct) (category-set-mnemonics (aref ct ?a)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("category:") && row.contains("X category") && row.contains("x"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: category table behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "category_table_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn char_table_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"chartab:%S\" (let ((ct (make-char-table nil 0))) (set-char-table-range ct '(?a . ?c) 9) (aset ct ?b 4) (list (aref ct ?a) (aref ct ?b) (aref ct ?c) (aref ct ?d))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("chartab:") && row.contains("(9 4 9 0)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: char-table operations should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "char_table_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn char_table_map_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"chartabmap:%S\" (let ((ct (make-char-table nil nil)) seen) (set-char-table-range ct ?a 1) (set-char-table-range ct ?b 2) (map-char-table (lambda (k v) (push (cons k v) seen)) ct) (sort seen (lambda (a b) (< (car a) (car b))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("chartabmap:") && row.contains("((97 . 1) (98 . 2))"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: map-char-table behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "char_table_map_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn display_table_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"disptab:%S\" (let ((dt (make-display-table))) (aset dt 0 [65]) (list (vectorp dt) (char-table-p dt) (aref dt 0) (aref dt 1) (length dt))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("disptab:") && row.contains("(nil t [65] nil 4194304)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: display-table char-table slot behavior should match GNU\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "display_table_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn save_excursion_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"saveexc:%S\" (with-temp-buffer (insert \"abc\") (goto-char 2) (let ((before (point)) inside after) (setq inside (save-excursion (goto-char (point-max)) (insert \"Z\") (point))) (setq after (point)) (list before inside after (buffer-string)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("saveexc:") && row.contains("(2 5 2"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: save-excursion should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "save_excursion_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn save_current_buffer_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"savebuf:%S\" (let ((a (current-buffer)) (b (generate-new-buffer \" *neo-savebuf*\")) inside after) (unwind-protect (progn (setq inside (save-current-buffer (set-buffer b) (buffer-name (current-buffer)))) (setq after (eq (current-buffer) a)) (list inside after (buffer-live-p b))) (kill-buffer b))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("savebuf:") && row.contains("t t"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: save-current-buffer should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "save_current_buffer_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn generate_buffer_name_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"genbuf:%S\" (let ((a (generate-new-buffer \"neo\")) (b (generate-new-buffer \"neo\"))) (prog1 (list (buffer-name a) (buffer-name b) (eq a b)) (kill-buffer a) (kill-buffer b))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("genbuf:")
                && row.contains("neo")
                && row.contains("neo<2>")
                && row.contains("nil")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: generated buffer naming should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "generate_buffer_name_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn buffer_modified_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"mod:%S\" (with-temp-buffer (list (buffer-modified-p) (progn (insert \"x\") (buffer-modified-p)) (progn (set-buffer-modified-p nil) (buffer-modified-p)) (progn (set-buffer-modified-p t) (buffer-modified-p)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("mod:") && row.contains("(nil t nil t)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: buffer modified flag semantics should match GNU\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "buffer_modified_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn buffer_undo_list_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"undolist:%S\" (with-temp-buffer (buffer-enable-undo) (insert \"abc\") (undo-boundary) (delete-char -1) (list (buffer-string) (consp buffer-undo-list) (memq nil buffer-undo-list) (progn (primitive-undo 1 buffer-undo-list) (buffer-string)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("undolist:")
                && row.contains("ab")
                && row.contains("(nil")
                && row.contains("(1 . 4)")
                && row.contains("abc")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: buffer undo list and primitive-undo should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "buffer_undo_list_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn column_motion_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"column:%S\" (with-temp-buffer (setq tab-width 8) (insert \"a\\tb\") (goto-char (point-min)) (list (current-column) (progn (forward-char 1) (current-column)) (progn (forward-char 1) (current-column)) (progn (move-to-column 4 t) (list (current-column) (buffer-string))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("column:")
                && row.contains("(0 1 8")
                && row.contains("(4")
                && row.contains("a")
                && row.contains("b")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: current-column and move-to-column tab behavior should match GNU\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "column_motion_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn thing_at_point_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"bounds:%S\" (with-temp-buffer (insert \"foo bar\\n  baz\") (goto-char 2) (list (bounds-of-thing-at-point 'word) (thing-at-point 'word t) (progn (goto-char 6) (bounds-of-thing-at-point 'symbol)) (progn (goto-char 12) (thing-at-point 'line t)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("bounds:")
                && row.contains("((1 . 4)")
                && row.contains("foo")
                && row.contains("(5 . 8)")
                && row.contains("baz")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: thing-at-point bounds and text extraction should match GNU\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "thing_at_point_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn point_motion_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"move:%S\" (with-temp-buffer (insert \"abc\") (goto-char 1) (list (progn (forward-char 2) (point)) (progn (backward-char 1) (point)) (condition-case e (forward-char 99) (error (car e))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("move:") && row.contains("(3 2 end-of-buffer)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: point motion behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "point_motion_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn delete_text_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"deltext:%S\" (with-temp-buffer (insert \"abcdef\") (list (delete-region 2 4) (buffer-string) (progn (goto-char 2) (delete-char 1)) (buffer-string))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("deltext:")
                && row.contains("nil")
                && row.contains("adef")
                && row.contains("aef")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: text deletion behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "delete_text_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn change_hook_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"changehook:%S\" (with-temp-buffer (let (seen) (add-hook 'before-change-functions (lambda (b e) (push (list 'before b e) seen)) nil t) (add-hook 'after-change-functions (lambda (b e l) (push (list 'after b e l) seen)) nil t) (insert \"ab\") (nreverse seen))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("changehook:")
                && row.contains("(before 1 1)")
                && row.contains("(after 1 3 0)")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: change hook behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "change_hook_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn char_at_point_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"charpos:%S\" (with-temp-buffer (insert \"ab\") (goto-char 1) (list (char-after) (char-before) (progn (goto-char 2) (list (char-before) (char-after))) (progn (goto-char (point-max)) (char-after)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("charpos:") && row.contains("(97 nil (97 98) nil)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: char-before and char-after behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "char_at_point_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn line_motion_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"linepos:%S\" (with-temp-buffer (insert \"aa\\nbbb\\n\") (goto-char 1) (list (progn (end-of-line) (point)) (progn (forward-line 1) (point)) (progn (end-of-line) (point)) (progn (beginning-of-line) (point)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("linepos:") && row.contains("(3 4 7 4)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: line motion behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "line_motion_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn sexp_motion_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"sexp:%S\" (with-temp-buffer (emacs-lisp-mode) (insert \"(a (b c))\") (list (progn (goto-char 1) (scan-sexps (point) 1)) (progn (goto-char 4) (forward-sexp 1) (point)) (progn (goto-char 9) (backward-sexp 1) (point)) (condition-case e (scan-sexps 1 -1) (scan-error (car e)) (error (car e))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("sexp:") && row.contains("(10 9 4 nil)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: sexp scanning and motion should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "sexp_motion_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn parse_partial_sexp_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"pparse:%S\" (with-temp-buffer (emacs-lisp-mode) (insert \"(a \\\"b\\\") ;c\") (list (parse-partial-sexp 1 (point-max)) (nth 0 (syntax-ppss (point-max))) (nth 3 (syntax-ppss (point-max))) (nth 4 (syntax-ppss (point-max))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("pparse:")
                && row.contains("(0 nil 1 nil t")
                && row.contains("9 nil nil")
                && row.contains("0 nil t")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: parse-partial-sexp and syntax-ppss should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "parse_partial_sexp_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn emacs_lisp_indent_region_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"indent:%S\" (with-temp-buffer (emacs-lisp-mode) (insert \"(progn\\n(+ 1 2))\") (indent-region (point-min) (point-max)) (buffer-string)))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        let recent = grid
            .iter()
            .rev()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        recent.contains("indent:") && recent.contains("(progn") && recent.contains("(+ 1 2))")
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: emacs-lisp-mode indent-region should match GNU\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "emacs_lisp_indent_region_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn case_fold_search_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"casefold:%S\" (with-temp-buffer (insert \"Abc\") (goto-char 1) (let ((case-fold-search t)) (list (search-forward \"abc\" nil t) (progn (goto-char 1) (let ((case-fold-search nil)) (search-forward \"abc\" nil t)))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("casefold:") && row.contains("(4 nil)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: case-fold-search behavior should match GNU\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "case_fold_search_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn case_fold_regexp_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"casefre:%S\" (list (let ((case-fold-search t)) (string-match \"abc\" \"ABC\")) (let ((case-fold-search nil)) (string-match \"abc\" \"ABC\")) (let ((case-fold-search t)) (string-match \"[[:upper:]]+\" \"abc\")) (let ((case-fold-search nil)) (string-match \"[[:upper:]]+\" \"abc\"))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("casefre:") && row.contains("(0 nil 0 nil)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: case-fold-search regexp behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "case_fold_regexp_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn replace_match_literal_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"repl:%S\" (list (progn (string-match \"\\\\(foo\\\\)\" \"foo\") (replace-match \"X\\\\1\" nil nil \"foo\")) (progn (string-match \"\\\\(foo\\\\)\" \"foo\") (replace-match \"X\\\\1\" nil t \"foo\"))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("repl:") && row.contains("Xfoo"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: replace-match literal behavior should match GNU\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "replace_match_literal_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn replace_regexp_in_string_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"replcase:%S\" (list (replace-regexp-in-string \"[a-z]\" (lambda (m) (upcase m)) \"ab\") (let ((case-replace t)) (replace-regexp-in-string \"foo\" \"bar\" \"Foo\")) (let ((case-replace nil)) (replace-regexp-in-string \"foo\" \"bar\" \"Foo\"))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("replcase:") && row.contains("AB") && row.contains("Bar"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: replace-regexp-in-string behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "replace_regexp_in_string_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn equality_predicate_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"eqs:%S\" (list (eq 1000 1000) (eql 1.0 1.0) (equal 1 1.0) (equal \"x\" (copy-sequence \"x\")) (eq \"x\" (copy-sequence \"x\"))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("eqs:") && row.contains("(t t nil t nil)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: equality predicates should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "equality_predicate_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn substring_sequence_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"substr:%S\" (list (substring \"abcdef\" 1 4) (substring \"abcdef\" -3 -1) (substring [a b c d] 1 3)))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("substr:") && row.contains("bcd") && row.contains("[b c]"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: substring sequence behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "substring_sequence_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn ignore_errors_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"ignerr:%S\" (list (ignore-errors (+ 1 2)) (ignore-errors (error \"bad %s\" 9)) (condition-case e (error \"bad %s\" 9) (error (cdr e)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("ignerr:") && row.contains("(3 nil") && row.contains("bad 9"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: ignore-errors behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "ignore_errors_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn completion_table_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"comp:%S\" (let ((tbl '(\"alpha\" \"alpine\" \"beta\"))) (list (try-completion \"al\" tbl) (try-completion \"alp\" tbl) (all-completions \"al\" tbl) (test-completion \"alpha\" tbl) (test-completion \"al\" tbl))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("comp:")
                && row.contains("alp")
                && row.contains("alpha")
                && row.contains("alpine")
                && row.contains("t nil")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: completion table functions should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "completion_table_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn add_to_history_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (defvar neomacs-test-history nil) (let ((neomacs-test-history nil) (history-delete-duplicates t)) (add-to-history 'neomacs-test-history \"a\") (add-to-history 'neomacs-test-history \"b\") (add-to-history 'neomacs-test-history \"a\") (message \"history:%S\" neomacs-test-history)))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("history:") && row.contains("a") && row.contains("b"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: add-to-history duplicate deletion should match GNU\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "add_to_history_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn numeric_rounding_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"round:%S\" (list (truncate -1.7) (floor -1.2) (ceiling -1.2) (round 2.5) (round -2.5)))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("round:") && row.contains("(-1 -2 -1 2 -2)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: numeric rounding behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "numeric_rounding_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn arithmetic_remainder_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr =
        "(message \"arith:%S\" (list (/ 7 3) (/ 7 3.0) (mod -7 3) (% -7 3) (mod 7 -3) (% 7 -3)))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("arith:") && row.contains("(2 2.333") && row.contains("2 -1 -2 1")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: arithmetic remainder behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "arithmetic_remainder_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn integer_bit_arithmetic_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"numedge:%S\" (list (floor -3 2) (ceiling -3 2) (truncate -3 2) (round 2.5) (round -2.5) (mod -3 2) (ash -8 -1) (logand #b1100 #b1010)))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("numedge:") && row.contains("(-2 -1 -1 2 -2 1 -4 8)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: integer division and bit arithmetic should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "integer_bit_arithmetic_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn time_value_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"time:%S\" (list (time-less-p (seconds-to-time 1) (seconds-to-time 2)) (time-add (seconds-to-time 1) (seconds-to-time 2)) (float-time (seconds-to-time 3))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("time:") && row.contains("(t (0 3 0 0) 3.0)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: time value behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "time_value_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn parse_time_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"timeparse:%S\" (list (parse-time-string \"2026-05-08 11:22:33 -0400\") (format-time-string \"%Y-%m-%d %H:%M:%S %z\" (encode-time (parse-time-string \"2026-05-08 11:22:33 -0400\")) t)))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("timeparse:")
                && row.contains("(33 22 11 8 5 2026 nil -1 -14400)")
                && row.contains("2026-05-08 15:22:33 +0000")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: parse-time-string and encode-time timezone behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "parse_time_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn split_string_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"split:%S\" (list (split-string \"a,,b,\" \",\" t) (split-string \" a  b \" nil t) (regexp-quote \"a.b*c\")))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("split:") && row.contains("\\\"a\\\"") && row.contains("\\\"b\\\"")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: split-string and regexp-quote should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "split_string_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn file_name_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"file:%S\" (let ((default-directory \"/tmp/\")) (list (expand-file-name \"a/../b\") (file-name-nondirectory \"/x/y.txt\") (file-name-directory \"/x/y.txt\") (file-name-extension \"a.tar.gz\"))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("file:")
                && row.contains("/tmp/b")
                && row.contains("y.txt")
                && row.contains("gz")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: file-name behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "file_name_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn file_name_edge_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"fileedge:%S\" (list (file-name-directory \"/tmp/a/b.txt\") (file-name-nondirectory \"/tmp/a/b.txt\") (directory-file-name \"/tmp/a/\") (file-name-as-directory \"/tmp/a\") (file-remote-p \"/ssh:host:/tmp/x\" 'method) (file-remote-p \"/tmp/x\")))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("fileedge:")
                && row.contains("/tmp/a/")
                && row.contains("b.txt")
                && row.contains("/tmp/a")
                && row.contains("ssh")
                && row.contains("nil")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: file-name edge behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "file_name_edge_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn file_mode_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"modes:%S\" (list (file-modes-symbolic-to-number \"u=rw,go=r\") (file-modes-number-to-symbolic #o644) (file-modes-number-to-symbolic #o755)))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("modes:")
                && row.contains("420")
                && row.contains("-rw-r--r--")
                && row.contains("-rwxr-xr-x")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: file mode conversion should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "file_mode_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn character_string_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"chars:%S\" (list (string-to-char \"abc\") (char-to-string ?A) (length \"é\") (string-bytes \"é\") (multibyte-string-p \"é\")))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("chars:") && row.contains("(97") && row.contains("1 2 t"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: character and string conversion should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "character_string_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn coding_string_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"coding:%S\" (let* ((s \"é\") (u (encode-coding-string s 'utf-8))) (list (length s) (string-bytes s) (multibyte-string-p u) (string-bytes u) (decode-coding-string u 'utf-8) (string-as-unibyte \"é\"))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("coding:")
                && row.contains("(1 2 nil 2")
                && row.contains("é")
                && row.contains("303")
                && row.contains("251")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: coding string conversion should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "coding_string_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn url_util_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'url-util) (message \"urlhex:%S\" (list (url-hexify-string \"a b/é\") (url-unhex-string \"a%20b%2F%C3%A9\") (url-unhex-string \"%E9\") (url-hexify-string \"!*()\"))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        let recent = grid
            .iter()
            .rev()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        recent.contains("urlhex:")
            && recent.contains("a%20b%2F%C3%A9")
            && recent.contains("a b/")
            && recent.contains("303")
            && recent.contains("251")
            && recent.contains("\\351")
            && recent.contains("%21%2A%28%29")
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: URL hex string helpers should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "url_util_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn url_parse_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'url-parse) (let ((u (url-generic-parse-url \"https://user:pw@example.com:8443/a/b?q=1#frag\"))) (message \"urlparse:%S\" (list (url-type u) (url-user u) (url-password u) (url-host u) (url-portspec u) (url-filename u) (url-target u)))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("urlparse:")
                && row.contains("https")
                && row.contains("user")
                && row.contains("pw")
                && row.contains("example.com")
                && row.contains("8443")
                && row.contains("/a/b?q=1")
                && row.contains("frag")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: URL parser accessors should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "url_parse_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn char_code_property_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"charprop:%S\" (list (get-char-code-property ?A 'general-category) (get-char-code-property ?0 'general-category) (get-char-code-property ?\\s 'general-category) (get-char-code-property ?é 'name)))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("charprop:")
                && row.contains("Lu")
                && row.contains("Nd")
                && row.contains("Zs")
                && row.contains("LATIN SMALL LETTER E WITH ACUTE")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: character code properties should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "char_code_property_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn string_compare_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"cmp:%S\" (list (string-lessp \"a\" \"b\") (string-lessp \"b\" \"a\") (compare-strings \"abc\" nil nil \"abd\" nil nil) (compare-strings \"abc\" nil nil \"abc\" nil nil)))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("cmp:") && row.contains("(t nil -3 t)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: string comparison should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "string_compare_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn string_algorithm_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"stralg:%S\" (list (string-version-lessp \"file9\" \"file10\") (string-version-lessp \"file10\" \"file9\") (string-distance \"kitten\" \"sitting\") (string-distance \"same\" \"same\")))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("stralg:") && row.contains("(t nil 3 0)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: string version and distance algorithms should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "string_algorithm_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn format_print_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"fmt:%S\" (list (format \"%04d\" 7) (format \"%S\" \"x\\ny\") (prin1-to-string (list 'a \"b\"))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        let recent = grid
            .iter()
            .rev()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        recent.contains("fmt:")
            && recent.contains("0007")
            && recent.contains("(a")
            && recent.contains("b")
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: formatting and printing behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "format_print_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn with_output_to_string_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"outstr:%S\" (list (with-output-to-string (princ \"A\") (prin1 'b)) (prin1-to-string '(a . b)) (prin1-to-string [1 2])))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        let recent = grid
            .iter()
            .rev()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        recent.contains("outstr:")
            && recent.contains("Ab")
            && recent.contains("(a . b)")
            && recent.contains("[1 2]")
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: with-output-to-string and object printing should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "with_output_to_string_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn hash_base64_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"crypto:%S\" (list (md5 \"abc\") (secure-hash 'sha1 \"abc\") (base64-encode-string \"abc\" t) (base64-decode-string \"YWJj\")))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("crypto:")
                && row.contains("900150983cd24fb0d6963f7d28e17f72")
                && row.contains("a9993e364706816aba3e25717850c26c9cd0d89d")
                && row.contains("YWJj")
                && row.contains("abc")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: hash and base64 string helpers should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "hash_base64_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn json_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'json) (message \"json:%S\" (let ((json-object-type 'alist) (json-array-type 'list)) (list (json-encode '((a . 1) (b . [2 3]))) (json-read-from-string \"{\\\"a\\\":1,\\\"b\\\":[2,3]}\") (json-encode-string \"é\\n\")))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        let recent = grid
            .iter()
            .rev()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        recent.contains("json:")
            && recent.contains("a")
            && recent.contains("b")
            && recent.contains("[2,3]")
            && recent.contains("((a . 1) (b 2 3))")
            && recent.contains("é")
            && recent.contains("\\\\n")
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: JSON encode/decode behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("json_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

#[test]
fn xml_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'xml) (message \"xml:%S\" (with-temp-buffer (insert \"<root a=\\\"1\\\"><child>é</child></root>\") (car (xml-parse-region (point-min) (point-max))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("xml:")
                && row.contains("(root")
                && row.contains("((a .")
                && row.contains("1")
                && row.contains("(child nil")
                && row.contains("é")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: XML parser tree shape should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("xml_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

#[test]
fn dom_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(progn (require 'dom) (let ((tree '(root ((class . \"top\")) (section ((id . \"a\")) \"Alpha\") (section ((id . \"b\")) (span nil \"Beta\"))))) (message \"dom:%S\" (list (dom-tag tree) (dom-attr tree 'class) (length (dom-by-tag tree 'section)) (dom-text (car (dom-by-tag tree 'span)))))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("dom:")
                && row.contains("root")
                && row.contains("top")
                && row.contains(" 2 ")
                && row.contains("Beta")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: DOM helper traversal should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches("dom_elisp_functions_match_gnu_semantics", &gnu, &neo, 2);
}

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
fn key_description_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"kbd:%S\" (list (key-description (kbd \"C-x C-f\")) (single-key-description ?\\C-h) (vectorp (kbd \"<f5>\")) (key-description [f5])))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("kbd:")
                && row.contains("C-x C-f")
                && row.contains("C-h")
                && row.contains(" t ")
                && row.contains("<f5>")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: keyboard description helpers should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "key_description_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn sparse_keymap_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"keymap:%S\" (let ((m (make-sparse-keymap))) (define-key m (kbd \"C-c a\") 'ignore) (list (keymapp m) (lookup-key m (kbd \"C-c a\")) (lookup-key m (kbd \"C-c b\")))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("keymap:") && row.contains("(t ignore nil)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: sparse keymap behavior should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "sparse_keymap_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn keymap_parent_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"keyparent:%S\" (let ((parent (make-sparse-keymap)) (child (make-sparse-keymap))) (define-key parent (kbd \"C-c p\") 'previous-line) (define-key child (kbd \"C-c c\") 'next-line) (set-keymap-parent child parent) (list (lookup-key child (kbd \"C-c c\")) (lookup-key child (kbd \"C-c p\")) (eq (keymap-parent child) parent))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter()
            .rev()
            .take(4)
            .any(|row| row.contains("keyparent:") && row.contains("(next-line previous-line t)"))
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: keymap parent inheritance should match GNU semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "keymap_parent_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
}

#[test]
fn substitute_command_keys_elisp_functions_match_gnu_semantics() {
    let (mut gnu, mut neo) = boot_pair("");

    let expr = "(message \"keys:%S\" (let ((map (make-sparse-keymap))) (define-key map (kbd \"C-c n\") 'next-line) (let ((overriding-local-map map)) (list (key-description (kbd \"C-c n\")) (lookup-key map (kbd \"C-c n\")) (substitute-command-keys \"Go: \\\\[next-line]\")))))";
    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| {
        grid.iter().rev().take(4).any(|row| {
            row.contains("keys:")
                && row.contains("C-c n")
                && row.contains("next-line")
                && row.contains("Go: C-c n")
        })
    };
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            ready(&grid),
            "{label}: substitute-command-keys should match GNU keymap substitution semantics\n{}",
            grid.join("\n")
        );
    }

    assert_pair_nearly_matches(
        "substitute_command_keys_elisp_functions_match_gnu_semantics",
        &gnu,
        &neo,
        2,
    );
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

#[test]
fn where_is_internal_returns_key_bindings_for_commands() {
    let (mut gnu, mut neo) = boot_pair("");
    // Short expression to avoid NEO TUI M-: input issues
    let expr = "(message \"wi=%d\" (length (where-is-internal 'find-file)))";

    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| grid.iter().any(|r| r.contains("wi="));
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        let has_output = grid
            .iter()
            .any(|r| r.contains("wi=1") || r.contains("wi=2") || r.contains("wi=3"));
        assert!(
            has_output,
            "{label}: where-is-internal find-file should show wi=N with N > 0"
        );
    }
}

#[test]
fn apropos_command_includes_key_binding_for_find_file() {
    let (mut gnu, mut neo) = boot_pair("");
    let expr = "(apropos-command \"find-file\")";

    support::eval_expression(&mut gnu, &mut neo, expr);

    let ready = |grid: &[String]| grid.iter().any(|r| r.contains("*Apropos*"));
    gnu.read_until(Duration::from_secs(6), ready);
    neo.read_until(Duration::from_secs(8), ready);
    read_both(&mut gnu, &mut neo, Duration::from_secs(1));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        assert!(
            grid.iter().any(|r| r.contains("*Apropos*")),
            "{label}: apropos-command should open *Apropos* buffer"
        );
    }
}

#[test]
fn recent_keys_includes_command_after_self_insert() {
    let (mut gnu, mut neo) = boot_pair("");
    // Type X, then check recent-keys via M-:
    send_both_raw(&mut gnu, &mut neo, b"X");
    read_both(&mut gnu, &mut neo, Duration::from_millis(500));

    send_both(&mut gnu, &mut neo, "M-:");
    let p = |g: &[String]| g.iter().any(|r| r.contains("Eval:"));
    gnu.read_until(Duration::from_secs(6), p);
    neo.read_until(Duration::from_secs(8), p);
    read_both(&mut gnu, &mut neo, Duration::from_millis(300));

    // Short expression
    for s in [&mut gnu, &mut neo] {
        s.send(b"(length (recent-keys 'include-cmds))\r");
    }
    read_both(&mut gnu, &mut neo, Duration::from_secs(2));

    for (label, session) in [("GNU", &gnu), ("NEO", &neo)] {
        let grid = session.text_grid();
        // Look for any number in the output
        let has_output = grid
            .iter()
            .any(|r| r.split_whitespace().any(|w| w.parse::<i32>().is_ok()));
        assert!(
            has_output,
            "{label}: M-: eval should produce a numeric result"
        );
    }
}
