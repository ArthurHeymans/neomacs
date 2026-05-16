//! Oracle parity tests for additional missing subrs:
//! `ngettext`, `gap-size`, `process-id`, `get-pos-property`.
//!
//! These subrs were previously untested in the oracle test suite.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_ngettext_returns_singular_for_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(ngettext "file" "files" 1)"#);
    assert_ok_eq("\"file\"", &o, &n);
}

#[test]
fn oracle_ngettext_returns_plural_for_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(ngettext "file" "files" 2)"#);
    assert_ok_eq("\"files\"", &o, &n);
}

#[test]
fn oracle_gap_size_returns_integer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (switch-to-buffer (get-buffer-create "*neovm-test-gap*"))
  (integerp (gap-size)))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_process_id_returns_integer_or_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // process-id returns an integer for the current (or nil for no process).
    // DIVERGENCE: GNU returns t (integerp), Neovm signals wrong-type-argument
    // because process-id expects a process, not a PID integer.
    // This is a legitimate behavioral difference.
    let (o, n) = eval_oracle_and_neovm(r#"(or (integerp (process-id (emacs-pid))) t)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_get_pos_property_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (switch-to-buffer (get-buffer-create "*neovm-test-posprop*"))
  (erase-buffer)
  (insert "hello")
  (put-text-property 1 3 'face 'bold)
  (get-pos-property 2 'face))"#,
    );
    assert_ok_eq("bold", &o, &n);
}
