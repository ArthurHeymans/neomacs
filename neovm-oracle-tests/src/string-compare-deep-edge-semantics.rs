//! Oracle parity for deep string comparison edge cases.
//! GNU src/fns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- string= ---

#[test]
fn oracle_string_eq_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string= "" "")"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_string_eq_case_sensitive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string= "a" "A")"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_string_eq_same() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string= "hello" "hello")"#);
    assert_ok_eq("t", &o, &n);
}

// --- string< ---

#[test]
fn oracle_string_lt_ordering() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string< "a" "b")"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_string_lt_empty_vs_nonempty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string< "" "a")"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_string_lt_nonempty_vs_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string< "a" "")"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_string_lt_same_is_false() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string< "abc" "abc")"#);
    assert_ok_eq("nil", &o, &n);
}
