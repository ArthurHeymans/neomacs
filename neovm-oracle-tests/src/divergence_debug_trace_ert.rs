//! Divergence tests: edebug, trace, elp, and debugging facilities.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_trace_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'trace-function)
  (fboundp 'trace-function-foreground)
  (fboundp 'untrace-function)
  (fboundp 'untrace-all))"#,
    );
}

#[test]
fn divergence_backtrace_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'backtrace)
  (fboundp 'backtrace-frame)
  (fboundp 'mapbacktrace))"#,
    );
}

#[test]
fn divergence_test_cover() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'testcover-start)
  (fboundp 'testcover-end)
  (featurep 'testcover))"#,
    );
}

#[test]
fn divergence_ert_framework() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'ert-deftest)
  (fboundp 'ert-run-tests-interactively)
  (featurep 'ert))"#,
    );
}

#[test]
fn divergence_checkdoc() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'checkdoc)
  (fboundp 'checkdoc-current-buffer)
  (featurep 'checkdoc))"#,
    );
}

#[test]
fn divergence_lisp_interaction() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'eval-last-sexp)
  (fboundp 'eval-print-last-sexp)
  (fboundp 'eval-expression)
  (fboundp 'ielm))"#,
    );
}

#[test]
fn divergence_elisp_bytecomp_warnings() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'byte-compile-warnings)
  (listp byte-compile-warnings)
  (boundp 'byte-compile-verbose)
  (booleanp byte-compile-verbose))"#,
    );
}

#[test]
fn divergence_subr_depth() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (>= (max-specpdl-size) 100)
  (>= (max-lisp-eval-depth) 100)
  (integerp max-specpdl-size)
  (integerp max-lisp-eval-depth))"#,
    );
}

#[test]
fn divergence_lread_variables() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (integerp read-circle)
  (integerp load-read-function)
  (booleanp load-dangerously-install-links))"#,
    );
}
