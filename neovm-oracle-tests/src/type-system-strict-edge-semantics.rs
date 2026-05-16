//! Oracle parity for type-system: type-of, max-char, bool-vector-p.
//! GNU src/data.c, src/character.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_type_of_integer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(type-of 42)"#);
    assert_ok_eq("integer", &o, &n);
}

#[test]
fn oracle_type_of_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(type-of "hello")"#);
    assert_ok_eq("string", &o, &n);
}

#[test]
fn oracle_type_of_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(type-of 'sym)"#);
    assert_ok_eq("symbol", &o, &n);
}

#[test]
fn oracle_type_of_cons() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(type-of '(a . b))"#);
    assert_ok_eq("cons", &o, &n);
}

#[test]
fn oracle_type_of_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(type-of [1 2 3])"#);
    assert_ok_eq("vector", &o, &n);
}

#[test]
fn oracle_type_of_float() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(type-of 3.14)"#);
    assert_ok_eq("float", &o, &n);
}

#[test]
fn oracle_max_char_positive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(> (max-char) 0)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_bool_vector_p_on_bool_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(bool-vector-p (bool-vector t nil))"#);
    assert_ok_eq("t", &o, &n);
}
