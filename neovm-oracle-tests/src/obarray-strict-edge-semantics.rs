//! Oracle parity tests for obarray strict edge cases.
//!
//! GNU src/lread.c: `intern`, `intern-soft`, `unintern`, `obarrayp`,
//! `mapatoms` operate on obarrays.  Symbol interning and obarray
//! manipulation has subtle semantics.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_intern_creates_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(symbolp (intern "neovm--test-intern-abc"))"#);
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_intern_same_name_returns_same_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(eq (intern "neovm--test-same")
             (intern "neovm--test-same"))"#,
    );
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_intern_soft_returns_nil_for_unknown() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) =
        eval_oracle_and_neovm(r#"(intern-soft "neovm--test-never-interned-xyz")"#);
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_intern_soft_returns_symbol_when_present() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(progn
  (intern "neovm--test-is-there")
  (symbolp (intern-soft "neovm--test-is-there")))"#,
    );
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_make_symbol_creates_uninterned() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(progn
  (let ((s (make-symbol "neovm--test-uninterned")))
    (list (symbolp s)
          (eq s (intern-soft "neovm--test-uninterned")))))"#,
    );
    assert_ok_eq("(t nil)", &oracle, &neovm);
}

#[test]
fn oracle_intern_empty_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(symbolp (intern ""))"#);
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_obarrayp_for_standard_obarray() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(obarrayp obarray)");
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_intern_wrong_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(intern 42)");
    assert_err_kind(&oracle, &neovm, "wrong-type-argument");
}
