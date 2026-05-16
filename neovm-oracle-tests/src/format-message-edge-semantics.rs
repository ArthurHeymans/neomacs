//! Oracle parity tests for `format` and `message` — strict edge cases.
//!
//! GNU src/editfns.c: `format` has many format specifiers (%s, %d, %S,
//! %c, %%, etc.) each with subtle semantics around escaping and types.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_format_s_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(format "%s" "hello")"#);
    assert_ok_eq("\"hello\"", &oracle, &neovm);
}

#[test]
fn oracle_format_d_integer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(format "%d" 42)"#);
    assert_ok_eq("\"42\"", &oracle, &neovm);
}

#[test]
fn oracle_format_S_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(format "%S" 'hello)"#);
    assert_ok_eq("\"hello\"", &oracle, &neovm);
}

#[test]
fn oracle_format_percent_literal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(format "%%")"#);
    assert_ok_eq("\"%\"", &oracle, &neovm);
}

#[test]
fn oracle_format_multiple_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(format "%s-%d" "item" 42)"#);
    assert_ok_eq("\"item-42\"", &oracle, &neovm);
}

#[test]
fn oracle_format_too_few_args_is_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) =
        eval_oracle_and_neovm(r#"(condition-case err (format "%s %s" "one") (error 'caught))"#);
    assert_ok_eq("caught", &oracle, &neovm);
}

#[test]
fn oracle_message_returns_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(stringp (message "test-%d" 42))"#);
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_format_wrong_type_first_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(format 42 "hello")"#);
    assert_err_kind(&oracle, &neovm, "wrong-type-argument");
}
