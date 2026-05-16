//! Oracle parity for throw/catch + unwind-protect strict edges.
//! GNU src/eval.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_catch_returns_body() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(catch 'x 42)"#);
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_catch_catches_throw() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(catch 'tag (throw 'tag 'val))"#);
    assert_ok_eq("val", &o, &n);
}

#[test]
fn oracle_unwind_protect_runs_cleanup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (defvar neovm--test-uwp-log '()) (unwind-protect (progn (setq neovm--test-uwp-log (cons 'body neovm--test-uwp-log)) 42) (setq neovm--test-uwp-log (cons 'cleanup neovm--test-uwp-log))) (nreverse neovm--test-uwp-log))"#,
    );
    assert_ok_eq("(body cleanup)", &o, &n);
}

#[test]
fn oracle_unwind_protect_cleanup_after_throw() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (defvar neovm--test-ut-log '()) (catch 'exit (unwind-protect (throw 'exit 'result) (setq neovm--test-ut-log 'cleaned))) neovm--test-ut-log)"#,
    );
    assert_ok_eq("cleaned", &o, &n);
}

#[test]
fn oracle_throw_value_is_evaluated() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(catch 't (+ 1 2 3))"#);
    assert_ok_eq("6", &o, &n);
}

#[test]
fn oracle_nested_catch_inner_first() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) =
        eval_oracle_and_neovm(r#"(catch 'outer (list (catch 'inner (throw 'inner 'i)) 'after))"#);
    assert_ok_eq("(i after)", &o, &n);
}

#[test]
fn oracle_catch_tag_quote_is_caught() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(catch 'tag (throw 'tag 'caught))"#);
    assert_ok_eq("caught", &o, &n);
}
