//! Divergence tests: edebug, trace, elp, and debugging facilities.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_trace_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'trace-function)
  (fboundp 'trace-function-foreground)
  (fboundp 'untrace-function)
  (fboundp 'untrace-all))"#,
        expect_test::expect![[r#""OK (t t nil nil)""#]],
    );
}

#[test]
fn divergence_backtrace_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'backtrace)
  (fboundp 'backtrace-frame)
  (fboundp 'mapbacktrace))"#,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}

#[test]
fn divergence_test_cover() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'testcover-start)
  (fboundp 'testcover-end)
  (featurep 'testcover))"#,
        expect_test::expect![[r#""OK (t nil nil)""#]],
    );
}

#[test]
fn divergence_ert_framework() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'ert-deftest)
  (fboundp 'ert-run-tests-interactively)
  (featurep 'ert))"#,
        expect_test::expect![[r#""OK (t t nil)""#]],
    );
}

#[test]
fn divergence_checkdoc() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'checkdoc)
  (fboundp 'checkdoc-current-buffer)
  (featurep 'checkdoc))"#,
        expect_test::expect![[r#""OK (t t nil)""#]],
    );
}

#[test]
fn divergence_lisp_interaction() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'eval-last-sexp)
  (fboundp 'eval-print-last-sexp)
  (fboundp 'eval-expression)
  (fboundp 'ielm))"#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_elisp_bytecomp_warnings() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (boundp 'byte-compile-warnings)
  (listp byte-compile-warnings)
  (boundp 'byte-compile-verbose)
  (booleanp byte-compile-verbose))"#,
        expect_test::expect![[r#""ERR (void-variable byte-compile-warnings)""#]],
    );
}

#[test]
fn divergence_subr_depth() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (>= (max-specpdl-size) 100)
  (>= (max-lisp-eval-depth) 100)
  (integerp max-specpdl-size)
  (integerp max-lisp-eval-depth))"#,
        expect_test::expect![[r#""ERR (void-function max-specpdl-size)""#]],
    );
}

#[test]
fn divergence_lread_variables() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (integerp read-circle)
  (integerp load-read-function)
  (booleanp load-dangerously-install-links))"#,
        expect_test::expect![[r#""ERR (void-variable load-dangerously-install-links)""#]],
    );
}
