//! Oracle parity for string compare: `string-lessp`, `string-version-lessp`.
//! GNU src/fns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_string_lessp_true() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-lessp "abc" "abd")"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_string_lessp_false() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-lessp "zzz" "aaa")"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_string_lessp_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-lessp "abc" "abc")"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_string_lessp_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-lessp "" "a")"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_string_lessp_empty_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-lessp "" "")"#);
    assert_ok_eq("nil", &o, &n);
}
