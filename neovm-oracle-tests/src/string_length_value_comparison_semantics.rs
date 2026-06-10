//! Oracle parity tests for `string<`, `string=`, `value<`, `length<`,
//! `length=`, and `length>` comparison predicates.
//!
//! GNU implements `string<`/`string=` in `src/fns.c` (string-lessp/string-equal
//! with symbol arguments), `value<` in `src/data.c`, and
//! `length<`/`length=`/`length>` in `src/fns.c`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

// ---------------------------------------------------------------------------
// string< / string=
// ---------------------------------------------------------------------------

#[test]
fn oracle_string_lt_true() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(string< "a" "b")"#);
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_string_lt_false_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(string< "a" "a")"#);
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_string_eq_true() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(string= "hello" "hello")"#);
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_string_eq_false() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(string= "hello" "world")"#);
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_string_eq_wrong_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(string< 42 "foo")"#);
    assert_err_kind(&oracle, &neovm, "wrong-type-argument");
}

// ---------------------------------------------------------------------------
// value<
// ---------------------------------------------------------------------------

#[test]
fn oracle_value_lt_numbers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(value< 1 2)"#);
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_value_lt_strings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(value< "a" "b")"#);
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_value_lt_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(value< 1 1)"#);
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_value_lt_wrong_number() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(value< 1)"#);
    assert_err_kind(&oracle, &neovm, "wrong-number-of-arguments");
}

// ---------------------------------------------------------------------------
// length< / length= / length>
// ---------------------------------------------------------------------------

#[test]
fn oracle_length_lt_true() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(length< '(a) 2)"#);
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_length_eq_true() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(length= '(a b) 2)"#);
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_length_gt_true() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(length> '(a b c) 1)"#);
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_length_eq_wrong_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(length< 42 "foo")"#);
    assert_err_kind(&oracle, &neovm, "wrong-type-argument");
}

#[test]
fn oracle_length_predicates_reject_non_fixnum_threshold() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    for form in [
        r#"(length< '(a) '(a b))"#,
        r#"(length= '(a b) "ab")"#,
        r#"(length> '(a b c) '(a))"#,
    ] {
        let (oracle, neovm) = eval_oracle_and_neovm(form);
        assert_err_kind(&oracle, &neovm, "wrong-type-argument");
    }
}
