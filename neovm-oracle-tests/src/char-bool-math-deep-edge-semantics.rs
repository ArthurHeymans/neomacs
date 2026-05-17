//! Oracle parity for char-equal, bool-vector, min/max, and bitwise edge cases.
//! GNU src/fns.c, src/data.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- char-equal (case-insensitive) ---

#[test]
fn oracle_char_equal_case_insensitive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(char-equal ?a ?A)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_char_equal_same_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(char-equal ?a ?a)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_char_equal_different() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(char-equal ?a ?b)"#);
    assert_ok_eq("nil", &o, &n);
}

// --- bool-vector-p ---

#[test]
fn oracle_bool_vector_p_on_bool_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(bool-vector-p (bool-vector t nil))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_bool_vector_p_on_regular_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(bool-vector-p [t nil])"#);
    assert_ok_eq("nil", &o, &n);
}

// --- min / max identity on single arg ---

#[test]
fn oracle_min_single_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(min -5)"#);
    assert_ok_eq("-5", &o, &n);
}

#[test]
fn oracle_max_single_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(max -5)"#);
    assert_ok_eq("-5", &o, &n);
}

#[test]
fn oracle_min_multiple_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(min 3 1 4 1 5)"#);
    assert_ok_eq("1", &o, &n);
}

#[test]
fn oracle_max_multiple_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(max 3 1 4 1 5)"#);
    assert_ok_eq("5", &o, &n);
}

// --- bitwise operations ---

#[test]
fn oracle_logand() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(logand 6 3)"#);
    assert_ok_eq("2", &o, &n);
}

#[test]
fn oracle_logior_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(logior 1 2 4)"#);
    assert_ok_eq("7", &o, &n);
}

#[test]
fn oracle_logxor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(logxor 5 3)"#);
    assert_ok_eq("6", &o, &n);
}

#[test]
fn oracle_lognot() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(lognot 0)"#);
    assert_ok_eq("-1", &o, &n);
}

// --- string-to-char ---

#[test]
fn oracle_string_to_char_first_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-to-char "ABC")"#);
    assert_ok_eq("65", &o, &n);
}

#[test]
fn oracle_string_to_char_empty_returns_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-to-char "")"#);
    assert_ok_eq("0", &o, &n);
}
