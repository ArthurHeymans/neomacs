//! Oracle parity tests for `substring` — strict edge cases.
//!
//! GNU src/fns.c `Fsubstring`: FROM and TO can be negative (counting
//! from end).  nil TO means end-of-string.  Out-of-range indices signal
//! `args-out-of-range`.  These edges are historically bug-prone.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_substring_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(substring "hello" 1 3)"#);
    assert_ok_eq("\"el\"", &o, &n);
}

#[test]
fn oracle_substring_from_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(substring "hello" 0 2)"#);
    assert_ok_eq("\"he\"", &o, &n);
}

#[test]
fn oracle_substring_omit_to() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(substring "hello" 2)"#);
    assert_ok_eq("\"llo\"", &o, &n);
}

#[test]
fn oracle_substring_nil_to_means_end() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(substring "hello" 2 nil)"#);
    assert_ok_eq("\"llo\"", &o, &n);
}

#[test]
fn oracle_substring_negative_from() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // -1 = last char, -2 = second to last, etc.
    let (o, n) = eval_oracle_and_neovm(r#"(substring "hello" -2)"#);
    assert_ok_eq("\"lo\"", &o, &n);
}

#[test]
fn oracle_substring_negative_to() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // -1 = last char position (exclusive end)
    let (o, n) = eval_oracle_and_neovm(r#"(substring "hello" 0 -1)"#);
    assert_ok_eq("\"hell\"", &o, &n);
}

#[test]
fn oracle_substring_both_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(substring "hello" -3 -1)"#);
    assert_ok_eq("\"ll\"", &o, &n);
}

#[test]
fn oracle_substring_from_equals_to_returns_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(substring "hello" 2 2)"#);
    assert_ok_eq("\"\"", &o, &n);
}

#[test]
fn oracle_substring_from_equals_length_returns_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(substring "abc" 3 3)"#);
    assert_ok_eq("\"\"", &o, &n);
}

#[test]
fn oracle_substring_from_greater_than_to_is_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(substring "abc" 2 1)"#);
    assert_err_kind(&o, &n, "args-out-of-range");
}

#[test]
fn oracle_substring_from_negative_out_of_range() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(substring "abc" -10)"#);
    assert_err_kind(&o, &n, "args-out-of-range");
}

#[test]
fn oracle_substring_wrong_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(substring 42 0)"#);
    assert_err_kind(&o, &n, "wrong-type-argument");
}
