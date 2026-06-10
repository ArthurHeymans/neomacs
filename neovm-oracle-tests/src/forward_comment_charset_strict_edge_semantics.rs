//! Oracle parity for charset operations.
//! GNU src/charset.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_char_charset_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(symbolp (char-charset ?a))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_char_equal_same() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(char-equal ?a ?A)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_char_equal_diff() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(char-equal ?a ?b)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_characterp_on_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(characterp ?x)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_characterp_on_non_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(characterp 999999999)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_charsetp_on_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(charsetp 'ascii)"#);
    assert_ok_eq("t", &o, &n);
}
