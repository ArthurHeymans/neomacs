//! Oracle parity tests for `string-match-p`.
//!
//! GNU implements `string-match-p` in `src/search.c` — like `string-match`
//! but faster since it doesn't modify match data.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_string_match_p_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm(r#"(string-match-p "foo" "foobar")"#);
    assert_ok_eq("0", &oracle, &neovm);
}

#[test]
fn oracle_string_match_p_no_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm(r#"(string-match-p "xyz" "foobar")"#);
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_string_match_p_with_start() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm(r#"(string-match-p "bar" "foobarbar" 6)"#);
    assert_ok_eq("6", &oracle, &neovm);
}

#[test]
fn oracle_string_match_p_wrong_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm(r#"(string-match-p 42 "foo")"#);
    assert_err_kind(&oracle, &neovm, "wrong-type-argument");
}

#[test]
fn oracle_string_match_p_does_not_modify_match_data() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(progn
  (string-match "xxx" "---xxx---")
  (let ((before (match-data)))
    (string-match-p "yyy" "---xxx---")
    (equal before (match-data))))"#,
    );
    assert_ok_eq("t", &oracle, &neovm);
}
