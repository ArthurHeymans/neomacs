//! Divergence tests: ERT testing framework, assertions deep.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_ert_core() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'ert-deftest)
  (fboundp 'should)
  (fboundp 'should-not)
  (fboundp 'should-error)
  (fboundp 'ert-run-tests-interactively)
  (fboundp 'ert-run-tests-batch)
  (featurep 'ert)) "#,
    );
}

#[test]
fn divergence_ert_selectors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'ert-select-tests)
  (fboundp 'ert-test-result-type-p)
  (fboundp 'ert-pass)
  (fboundp 'ert-fail)
  (fboundp 'ert--stats)) "#,
    );
}

#[test]
fn divergence_buttercup() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'buttercup-define-matcher)
  (fboundp 'buttercup-run)
  (featurep 'buttercup)) "#,
    );
}

#[test]
fn divergence_ert_mock() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'ert-with-test-buffer)
  (fboundp 'ert-with-global-buffer)
  (fboundp 'ert--explain)) "#,
    );
}

#[test]
fn divergence_debugger() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'debug)
  (fboundp 'debug-on-entry)
  (fboundp 'cancel-debug-on-entry)
  (boundp 'debug-on-error)
  (boundp 'debug-on-quit)
  (boundp 'debugger)
  (fboundp debugger)) "#,
    );
}

#[test]
fn divergence_backtrace() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'backtrace)
  (fboundp 'backtrace-frame)
  (fboundp 'backtrace-debug)
  (fboundp 'mapbacktrace)) "#,
    );
}

#[test]
fn divergence_trace() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'trace-function)
  (fboundp 'trace-function-foreground)
  (fboundp 'untrace-function)
  (fboundp 'untrace-all)
  (featurep 'trace)) "#,
    );
}

#[test]
fn divergence_elisp_demos() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'shortdoc-display-groups)
  (fboundp 'shortdoc-display-function)
  (featurep 'shortdoc)) "#,
    );
}

#[test]
fn divergence_finder() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'finder-commentary)
  (fboundp 'finder-by-keyword)
  (featurep 'finder)) "#,
    );
}

#[test]
fn divergence_package_tests() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'ert-test)
  (fboundp 'ert-make-test)
  (fboundp 'ert-get-test)
  (fboundp 'ert-test-boundp)) "#,
    );
}
