//! Oracle parity for deep edge cases: string-to-number, substring,
//! safe-length, proper-list-p, number-to-string.
//! GNU src/editfns.c, src/fns.c, src/data.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- string-to-number deep edges ---

#[test]
fn oracle_string_to_number_octal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-to-number "077" 8)"#);
    assert_ok_eq("63", &o, &n);
}

#[test]
fn oracle_string_to_number_hex_base_16() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // With explicit base 16, string content should be hex digits
    let (o, n) = eval_oracle_and_neovm(r#"(string-to-number "ff" 16)"#);
    assert_ok_eq("255", &o, &n);
}

#[test]
fn oracle_string_to_number_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-to-number "1010" 2)"#);
    assert_ok_eq("10", &o, &n);
}

#[test]
fn oracle_string_to_number_whitespace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-to-number "  -42  ")"#);
    assert_ok_eq("-42", &o, &n);
}

#[test]
fn oracle_string_to_number_non_numeric_returns_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-to-number "abc")"#);
    assert_ok_eq("0", &o, &n);
}

// --- substring deep edges ---

#[test]
fn oracle_substring_mid_range() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(substring "hello" 1 4)"#);
    assert_ok_eq("\"ell\"", &o, &n);
}

#[test]
fn oracle_substring_from_mid_to_end() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(substring "hello" 2)"#);
    assert_ok_eq("\"llo\"", &o, &n);
}

#[test]
fn oracle_substring_negative_from() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // 0-indexed start, -1 means up to last character exclusive
    let (o, n) = eval_oracle_and_neovm(r#"(substring "hello" 0 -1)"#);
    assert_ok_eq("\"hell\"", &o, &n);
}

// --- safe-length ---

#[test]
fn oracle_safe_length_proper_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(safe-length '(a b c))"#);
    assert_ok_eq("3", &o, &n);
}

#[test]
fn oracle_safe_length_dotted_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(safe-length '(a . b))"#);
    assert_ok_eq("1", &o, &n);
}

#[test]
fn oracle_safe_length_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(safe-length nil)"#);
    assert_ok_eq("0", &o, &n);
}

// --- number-to-string ---

#[test]
fn oracle_number_to_string_positive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(number-to-string 255)"#);
    assert_ok_eq("\"255\"", &o, &n);
}

#[test]
fn oracle_number_to_string_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(number-to-string -10)"#);
    assert_ok_eq("\"-10\"", &o, &n);
}

#[test]
fn oracle_number_to_string_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(number-to-string 0)"#);
    assert_ok_eq("\"0\"", &o, &n);
}
