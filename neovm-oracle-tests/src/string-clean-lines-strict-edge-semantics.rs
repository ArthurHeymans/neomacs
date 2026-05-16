//! Oracle parity for string-clean-whitespace, string-lines,
//! string-pad, string-limit, string-replace.
//! GNU src/fns.c, src/editfns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_string_clean_whitespace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-clean-whitespace "  a  b  ")"#);
    assert_ok_eq("\"a b\"", &o, &n);
}

#[test]
fn oracle_string_lines_split() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(length (string-lines "a\nb\nc"))"#);
    assert_ok_eq("3", &o, &n);
}

#[test]
fn oracle_string_pad_right() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(length (string-pad "hi" 5))"#);
    assert_ok_eq("5", &o, &n);
}

#[test]
fn oracle_string_limit_shorten() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-limit "hello world" 5)"#);
    assert_ok_eq("\"hello\"", &o, &n);
}

#[test]
fn oracle_string_replace_literal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-replace "hello" "world" "hello world")"#);
    assert_ok_eq("\"world world\"", &o, &n);
}

#[test]
fn oracle_string_fill_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(stringp (string-fill "hi" 10))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_string_remove_prefix_no_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-remove-prefix "xyz" "abc")"#);
    assert_ok_eq("\"abc\"", &o, &n);
}
