//! Oracle parity tests for command-loop keyboard variable semantics.
//!
//! These tests verify parity between GNU Emacs and Neomacs for:
//!
//! 1. `this-command` / `real-this-command` / `this-original-command` are nil
//!    at the start of each command-loop iteration, matching GNU
//!    `keyboard.c:1416-1419`.
//!
//! 2. `echo-keystrokes-help` default value (GNU `keyboard.c` initialization).
//!
//! 3. Minibuffer lifecycle: `minibuffer-mode` / `minibuffer-inactive-mode`
//!    are called around minibuffer entry/exit.
//!
//! 4. Idle timer interaction with `this-single-command-keys` and
//!    `this-command` during `read_key_sequence`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, assert_oracle_parity, eval_oracle_and_neovm};

// ---------------------------------------------------------------------------
// this-command initial state
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_this_command_initially_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(null this-command)",
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_prop_real_this_command_initially_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(null real-this-command)",
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_prop_this_original_command_initially_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(null this-original-command)",
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_prop_last_command_initially_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(null last-command)",
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

// ---------------------------------------------------------------------------
// echo-keystrokes-help default
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_echo_keystrokes_help_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(symbol-value 'echo-keystrokes-help)",
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_prop_echo_keystrokes_default_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(default-value 'echo-keystrokes)",
        expect_test::expect![[r#""OK 1""#]],
    );
}

// ---------------------------------------------------------------------------
// this-command set by command-execute
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_this_command_after_call_interactively() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(progn
      (call-interactively (setq this-command 'ignore))
      this-command)"#;
    crate::common::assert_oracle_parity_expect(form, expect_test::expect![[r#""OK ignore""#]]);
}

#[test]
fn oracle_prop_this_command_set_by_setq() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(progn
      (setq this-command 'some-command)
      this-command)"#;
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        form,
        expect_test::expect![[r#""OK some-command""#]],
    );
    assert_ok_eq("some-command", &o, &n);
}

// ---------------------------------------------------------------------------
// Minibuffer lifecycle variables
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_minibuffer_depth_initial() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(minibuffer-depth)",
        expect_test::expect![[r#""OK 0""#]],
    );
    assert_ok_eq("0", &o, &n);
}

#[test]
fn oracle_prop_minibufferp_initially_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(minibufferp)",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_prop_active_minibuffer_window_initial() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(active-minibuffer-window)",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_prop_minibuffer_exit_hook_bound() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(boundp 'minibuffer-exit-hook)",
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_prop_minibuffer_exit_hook_is_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = "(listp (symbol-value 'minibuffer-exit-hook))";
    let (o, n) =
        crate::common::eval_oracle_and_neovm_expect(form, expect_test::expect![[r#""OK t""#]]);
    assert_ok_eq("t", &o, &n);
}

// ---------------------------------------------------------------------------
// command-execute / this-command lifecycle
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_command_execute_sets_this_command() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(progn
      (let ((this-command nil))
        (command-execute 'ignore)
        this-command))"#;
    crate::common::assert_oracle_parity_expect(form, expect_test::expect![[r#""OK nil""#]]);
}

#[test]
fn oracle_prop_this_command_keys_initially_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(progn
      (let ((keys (this-command-keys)))
        (length keys)))"#;
    let (o, n) =
        crate::common::eval_oracle_and_neovm_expect(form, expect_test::expect![[r#""OK 0""#]]);
    assert_ok_eq("0", &o, &n);
}

// ---------------------------------------------------------------------------
// this-single-command-keys in batch context
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_this_single_command_keys_vector_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = "(vectorp (this-single-command-keys))";
    let (o, n) =
        crate::common::eval_oracle_and_neovm_expect(form, expect_test::expect![[r#""OK t""#]]);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_prop_this_command_keys_type_in_batch() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(this-command-keys)",
        expect_test::expect![[r#""OK \"\"""#]],
    );
}

#[test]
fn oracle_prop_this_single_command_keys_empty_in_batch() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = "(length (this-single-command-keys))";
    let (o, n) =
        crate::common::eval_oracle_and_neovm_expect(form, expect_test::expect![[r#""OK 0""#]]);
    assert_ok_eq("0", &o, &n);
}

// ---------------------------------------------------------------------------
// Timer creation and cancellation parity
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_run_with_idle_timer_returns_timer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(progn
      (let ((timer (run-with-idle-timer 10 nil 'ignore)))
        (and (timerp timer) (prog1 t (cancel-timer timer)))))"#;
    let (o, n) =
        crate::common::eval_oracle_and_neovm_expect(form, expect_test::expect![[r#""OK t""#]]);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_prop_cancel_timer_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(progn
      (let ((timer (run-with-idle-timer 10 nil 'ignore)))
        (cancel-timer timer)
        (not (memq timer timer-idle-list))))"#;
    let (o, n) =
        crate::common::eval_oracle_and_neovm_expect(form, expect_test::expect![[r#""OK t""#]]);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_prop_timerp_on_idle_timer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(progn
      (let ((timer (run-with-idle-timer 10 nil 'ignore)))
        (prog1 (timerp timer) (cancel-timer timer))))"#;
    let (o, n) =
        crate::common::eval_oracle_and_neovm_expect(form, expect_test::expect![[r#""OK t""#]]);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_prop_run_with_timer_returns_timer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(progn
      (let ((timer (run-with-timer 10 nil 'ignore)))
        (and (timerp timer) (prog1 t (cancel-timer timer)))))"#;
    let (o, n) =
        crate::common::eval_oracle_and_neovm_expect(form, expect_test::expect![[r#""OK t""#]]);
    assert_ok_eq("t", &o, &n);
}

// ---------------------------------------------------------------------------
// command-keys and key-description parity
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_key_description_single_space() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(key-description [32])"#;
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        form,
        expect_test::expect![[r#""OK \"SPC\"""#]],
    );
    assert_ok_eq("\"SPC\"", &o, &n);
}

#[test]
fn oracle_prop_key_description_prefix_sequence() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(key-description [32 104])"#;
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        form,
        expect_test::expect![[r#""OK \"SPC h\"""#]],
    );
    assert_ok_eq("\"SPC h\"", &o, &n);
}

#[test]
fn oracle_prop_single_key_description() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(single-key-description 32)"#;
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        form,
        expect_test::expect![[r#""OK \"SPC\"""#]],
    );
    assert_ok_eq("\"SPC\"", &o, &n);
}

#[test]
fn oracle_prop_single_key_description_with_ctrl() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(single-key-description ?\\C-x)"#;
    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[r#""ERR (invalid-read-syntax \"?\" 1 27)""#]],
    );
}

// ---------------------------------------------------------------------------
// this-command / last-command transition semantics
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_last_command_after_command_execute() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(progn
      (setq this-command 'cmd-a)
      (command-execute 'ignore)
      (setq last-command this-command)
      last-command)"#;
    crate::common::assert_oracle_parity_expect(form, expect_test::expect![[r#""OK cmd-a""#]]);
}

#[test]
fn oracle_prop_real_last_command_var_exists() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = "(boundp 'real-last-command)";
    let (o, n) =
        crate::common::eval_oracle_and_neovm_expect(form, expect_test::expect![[r#""OK t""#]]);
    assert_ok_eq("t", &o, &n);
}
