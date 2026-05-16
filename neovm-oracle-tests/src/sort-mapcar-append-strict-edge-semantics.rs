//! Oracle parity tests for `sort`, `mapcar`, `mapc`, `append` —
//! strict edge cases.
//!
//! GNU src/fns.c: `sort` destructively sorts a list; `mapcar`/`mapc`
//! apply a function to list elements; `append` concatenates lists.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

// ---------------------------------------------------------------------------
// sort
// ---------------------------------------------------------------------------

#[test]
fn oracle_sort_numeric() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(sort '(3 1 4 1 5) '<)"#);
    assert_ok_eq("(1 1 3 4 5)", &o, &n);
}

#[test]
fn oracle_sort_should_preserve_length() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(= (length (sort '(3 2 1) '<)) 3)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_sort_singleton_unchanged() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(sort '(42) '<)"#);
    assert_ok_eq("(42)", &o, &n);
}

#[test]
fn oracle_sort_nil_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(sort nil '<)"#);
    assert_ok_eq("nil", &o, &n);
}

// ---------------------------------------------------------------------------
// mapcar / mapc
// ---------------------------------------------------------------------------

#[test]
fn oracle_mapcar_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(mapcar '1+ '(1 2 3))"#);
    assert_ok_eq("(2 3 4)", &o, &n);
}

#[test]
fn oracle_mapc_returns_first_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(mapc '1+ '(1 2 3))"#);
    assert_ok_eq("(1 2 3)", &o, &n);
}

// ---------------------------------------------------------------------------
// append
// ---------------------------------------------------------------------------

#[test]
fn oracle_append_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(append '(1 2) '(3 4))"#);
    assert_ok_eq("(1 2 3 4)", &o, &n);
}

#[test]
fn oracle_append_last_arg_not_list_makes_dotted() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // GNU: append with a non-list final arg produces a dotted list.
    let (o, n) = eval_oracle_and_neovm(r#"(append '(1) 42)"#);
    assert_ok_eq("(1 . 42)", &o, &n);
}

#[test]
fn oracle_append_nil_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(append nil nil nil)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_append_no_args_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(append)"#);
    assert_ok_eq("nil", &o, &n);
}
