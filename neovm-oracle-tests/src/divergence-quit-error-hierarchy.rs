//! Divergence tests: quit/error hierarchy.
//!
//! GNU Emacs: `quit` has error-conditions = (quit), NOT inheriting from `error`.
//! Neomacs bug: `quit` is registered as a child of `error`, so error handlers
//! incorrectly catch quit signals.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_quit_not_caught_by_error_handler() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(condition-case err
    (progn (signal 'quit nil) 'not-reached)
  (error 'error-caught)
  (quit 'quit-caught))"#;
    let (oracle, neovm) = eval_oracle_and_neovm(form);
    assert_ok_eq("quit-caught", &oracle, &neovm);
}

#[test]
fn oracle_quit_error_conditions_list_excludes_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(member 'error (get 'quit 'error-conditions))"#;
    let (oracle, neovm) = eval_oracle_and_neovm(form);
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_quit_error_conditions_includes_quit() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(member 'quit (get 'quit 'error-conditions))"#;
    let (oracle, neovm) = eval_oracle_and_neovm(form);
    assert_ok_eq("(quit)", &oracle, &neovm);
}

#[test]
fn oracle_quit_not_matched_by_t_condition() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(condition-case err
    (signal 'quit nil)
  (t (list 'caught-by-t (car err))))"#;
    let (oracle, neovm) = eval_oracle_and_neovm(form);
    assert_ok_eq("(caught-by-t quit)", &oracle, &neovm);
}

#[test]
fn oracle_quit_vs_error_different_handler_branches() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(list
  (condition-case err
      (signal 'error "test")
    (error 'got-error)
    (quit 'got-quit))
  (condition-case err
      (signal 'quit nil)
    (error 'got-error)
    (quit 'got-quit)))"#;
    let (oracle, neovm) = eval_oracle_and_neovm(form);
    assert_ok_eq("(got-error got-quit)", &oracle, &neovm);
}

#[test]
fn oracle_minibuffer_quit_hierarchy() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(get 'minibuffer-quit 'error-conditions)"#;
    let (oracle, neovm) = eval_oracle_and_neovm(form);
    assert_ok_eq("(minibuffer-quit quit)", &oracle, &neovm);
}

#[test]
fn oracle_minibuffer_quit_not_inheriting_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(member 'error (get 'minibuffer-quit 'error-conditions))"#;
    let (oracle, neovm) = eval_oracle_and_neovm(form);
    assert_ok_eq("nil", &oracle, &neovm);
}
