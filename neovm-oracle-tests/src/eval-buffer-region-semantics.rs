//! Oracle parity tests for `eval-buffer` and `eval-region`.
//!
//! GNU implements `eval-buffer` and `eval-region` in `src/lread.c`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_eval_buffer_returns_nil_for_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(progn
  (switch-to-buffer (get-buffer-create "*neovm-test-evalbuf*"))
  (erase-buffer)
  (eval-buffer))"#,
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_eval_buffer_evaluates_content() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(progn
  (switch-to-buffer (get-buffer-create "*neovm-test-evalbuf2*"))
  (erase-buffer)
  (insert "(setq neovm--test-evalbuf-result 42)")
  (eval-buffer)
  neovm--test-evalbuf-result)"#,
    );
    assert_ok_eq("42", &oracle, &neovm);
}

#[test]
fn oracle_eval_region_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(progn
  (switch-to-buffer (get-buffer-create "*neovm-test-evalreg*"))
  (erase-buffer)
  (insert "99")
  (eval-region (point-min) (point-max)))"#,
    );
    assert_ok_eq("99", &oracle, &neovm);
}

#[test]
fn oracle_eval_buffer_wrong_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(eval-buffer 42)");
    assert_err_kind(&oracle, &neovm, "wrong-type-argument");
}
